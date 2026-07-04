use crate::model::Schematic;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct SymmetryReport {
    pub matched_pairs: Vec<MatchedPair>,
    pub overall_score: f64,
}

#[derive(Debug, Serialize)]
pub struct MatchedPair {
    pub device_a: String,
    pub device_b: String,
    pub y_diff: f64,
    pub symmetry_score: f64,
}

pub fn check(schematic: &Schematic) -> SymmetryReport {
    // Group components by (symbol_name, key properties like W/L/model).
    // Skip independent voltage and current sources: two V sources or two
    // I sources happen to share an identical match key (no W/L, model
    // field is the value string which is often the same across instances)
    // even though they represent unrelated parts of the circuit. Counting
    // them as a "pair" produced misleading symmetry scores — see Bug 2 in
    // the 2026-05-01 test-set-expansion findings.
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, comp) in schematic.components.iter().enumerate() {
        if matches!(comp.symbol_name.as_str(), "vsource" | "isource") {
            continue;
        }
        // Box components that are not FET primitives — synthetic gates,
        // library cells, hierarchical blocks — are unrelated logic, not
        // analog mirror pairs; same reasoning as the V/I exclusion above.
        // (Case 36: its only two NOR2 gates scored 0.0 as a "pair" across
        // fold bands; case 46: two same-model stdcell buffers scattered
        // across a 1188-instance canvas failed Tier 1.) X-FET boxes
        // (sky130_fd_pr__*fet) stay in — analog mirrors are real there.
        if let Some(model) = comp.symbol_name.strip_prefix("subckt_") {
            if crate::parser::SpiceParser::fet_model_polarity(model).is_none() {
                continue;
            }
        }
        let mut key_parts = vec![comp.symbol_name.clone()];
        // Sort properties for stable key
        let mut props: Vec<(&str, &str)> = comp
            .properties
            .iter()
            .filter(|(k, _)| matches!(k.as_str(), "W" | "L" | "model"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        props.sort();
        for (k, v) in props {
            key_parts.push(format!("{}={}", k, v));
        }
        let key = key_parts.join("|");
        groups.entry(key).or_default().push(i);
    }

    let mut matched_pairs = Vec::new();

    // For groups with exactly 2 members, compute symmetry
    for indices in groups.values() {
        if indices.len() == 2 {
            let a = &schematic.components[indices[0]];
            let b = &schematic.components[indices[1]];

            let y_diff = (a.position.y - b.position.y).abs();
            let x_diff = (a.position.x - b.position.x).abs();

            // Two clean arrangements score 1.0:
            //  - mirror pair: same row (y_diff ≈ 0), side by side;
            //  - same-column arrangement: x_diff within one symbol width
            //    (60, the same column semantics as power_convention) —
            //    covers exact vertical stacks (cascode), the tail-below-
            //    center diff-pair template (tail sits half a pitch from
            //    each pair member; case 51's q1/q4 false pair), and the
            //    only safe layout when a same-column pair cannot be
            //    y-aligned without overlap (cases 30/34).
            // Only a diagonal offset — misaligned in BOTH axes — is sloppy.
            let max_dim = x_diff.max(y_diff).max(1.0);
            let score = if x_diff < 60.0 {
                1.0
            } else {
                1.0 - (y_diff / max_dim).min(1.0)
            };

            matched_pairs.push(MatchedPair {
                device_a: a.instance_name.clone(),
                device_b: b.instance_name.clone(),
                y_diff: round2(y_diff),
                symmetry_score: round2(score),
            });
        }
    }

    let overall = if matched_pairs.is_empty() {
        1.0
    } else {
        let sum: f64 = matched_pairs.iter().map(|p| p.symmetry_score).sum();
        round2(sum / matched_pairs.len() as f64)
    };

    SymmetryReport {
        matched_pairs,
        overall_score: overall,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Point};

    fn comp(name: &str, sym: &str, x: f64, y: f64, props: &[(&str, &str)]) -> Component {
        Component {
            instance_name: name.into(),
            symbol_name: sym.into(),
            position: Point::new(x, y),
            rotation: 0,
            mirrored: false,
            properties: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn no_components_perfect_score() {
        let r = check(&Schematic::new(""));
        assert!(r.matched_pairs.is_empty());
        assert_eq!(r.overall_score, 1.0);
    }

    #[test]
    fn singletons_dont_form_pairs() {
        // One nmos with W=1u, one pmos with W=1u → different symbol → not a pair.
        let mut s = Schematic::new("");
        s.components
            .push(comp("M1", "nmos4", 0.0, 0.0, &[("W", "1u"), ("L", "1u")]));
        s.components
            .push(comp("M2", "pmos4", 100.0, 0.0, &[("W", "1u"), ("L", "1u")]));
        let r = check(&s);
        assert!(r.matched_pairs.is_empty());
    }

    #[test]
    fn aligned_pair_scores_one() {
        // Two identical nmos at same y → perfect symmetry.
        let mut s = Schematic::new("");
        s.components.push(comp(
            "M1",
            "nmos4",
            -50.0,
            0.0,
            &[("W", "10u"), ("L", "1u")],
        ));
        s.components
            .push(comp("M2", "nmos4", 50.0, 0.0, &[("W", "10u"), ("L", "1u")]));
        let r = check(&s);
        assert_eq!(r.matched_pairs.len(), 1);
        assert_eq!(r.matched_pairs[0].y_diff, 0.0);
        assert_eq!(r.matched_pairs[0].symmetry_score, 1.0);
        assert_eq!(r.overall_score, 1.0);
    }

    #[test]
    fn misaligned_pair_scores_lower() {
        // Same x_diff = 100 and y_diff = 100 → max_dim = 100, score = 1 - 100/100 = 0.0
        let mut s = Schematic::new("");
        s.components
            .push(comp("M1", "nmos4", 0.0, 0.0, &[("W", "10u"), ("L", "1u")]));
        s.components.push(comp(
            "M2",
            "nmos4",
            100.0,
            100.0,
            &[("W", "10u"), ("L", "1u")],
        ));
        let r = check(&s);
        assert_eq!(r.matched_pairs.len(), 1);
        assert_eq!(r.matched_pairs[0].y_diff, 100.0);
        assert!(r.matched_pairs[0].symmetry_score <= 0.01);
    }

    #[test]
    fn vertical_stack_is_a_clean_arrangement() {
        // Same column (x_diff = 0), different rows: a deliberate series /
        // cascode-style stack, not a failed mirror — scores 1.0.
        let mut s = Schematic::new("");
        s.components
            .push(comp("M1", "nmos4", 0.0, 0.0, &[("W", "10u"), ("L", "1u")]));
        s.components.push(comp(
            "M2",
            "nmos4",
            0.0,
            160.0,
            &[("W", "10u"), ("L", "1u")],
        ));
        let r = check(&s);
        assert_eq!(r.matched_pairs.len(), 1);
        assert_eq!(r.matched_pairs[0].symmetry_score, 1.0);
        assert_eq!(r.overall_score, 1.0);
    }

    #[test]
    fn property_differences_break_pairing() {
        // Two nmos with different W → not the same key, so no pair.
        let mut s = Schematic::new("");
        s.components
            .push(comp("M1", "nmos4", 0.0, 0.0, &[("W", "10u"), ("L", "1u")]));
        s.components.push(comp(
            "M2",
            "nmos4",
            100.0,
            0.0,
            &[("W", "20u"), ("L", "1u")],
        ));
        let r = check(&s);
        assert!(r.matched_pairs.is_empty());
    }

    #[test]
    fn group_of_three_is_not_a_pair() {
        // 3 identical devices → group size != 2 → not scored.
        let mut s = Schematic::new("");
        for (i, x) in [-100.0, 0.0, 100.0].iter().enumerate() {
            s.components.push(comp(
                &format!("M{}", i + 1),
                "nmos4",
                *x,
                0.0,
                &[("W", "10u"), ("L", "1u")],
            ));
        }
        let r = check(&s);
        assert!(r.matched_pairs.is_empty());
        assert_eq!(r.overall_score, 1.0);
    }
}
