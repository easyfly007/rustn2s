use crate::model::Schematic;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PowerConventionReport {
    pub pmos_count: usize,
    pub nmos_count: usize,
    pub violations: Vec<ConventionViolation>,
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct ConventionViolation {
    pub pmos_device: String,
    pub pmos_y: f64,
    pub nmos_device: String,
    pub nmos_y: f64,
}

/// Check that PMOS devices are placed above (smaller y) NMOS devices.
/// Only compares devices that are horizontally close (likely in the same column/block).
pub fn check(schematic: &Schematic) -> PowerConventionReport {
    let mut pmos: Vec<(String, f64, f64)> = Vec::new(); // (name, x, y)
    let mut nmos: Vec<(String, f64, f64)> = Vec::new();

    for comp in &schematic.components {
        match comp.symbol_name.as_str() {
            "pmos4" => pmos.push((comp.instance_name.clone(), comp.position.x, comp.position.y)),
            "nmos4" => nmos.push((comp.instance_name.clone(), comp.position.x, comp.position.y)),
            _ => {}
        }
    }

    // Each PMOS is compared only against the NEAREST NMOS in its column
    // (nearest by |Δy|). Comparing against every NMOS in the column would
    // flag legitimate multi-row layouts: two CMOS stages stacked vertically
    // always put the lower stage's PMOS below the upper stage's NMOS, yet
    // each stage is internally P-above-N (see test case 31, the TG DFF).
    let mut violations = Vec::new();
    // Compare only devices whose horizontal symbol extents overlap — a
    // MOSFET symbol is ~60px wide, so anything offset by a full symbol
    // width sits in a NEIGHBORING column, and P-above-N is a per-column
    // convention. The previous 100px window flagged cross-column pairs
    // (case 40: MPASS below the unrelated MLN one column over).
    let x_threshold = 60.0;
    let mut compared = 0usize;
    let mut valid_pairs = 0usize;

    for (pname, px, py) in &pmos {
        let nearest = nmos
            .iter()
            .filter(|(_, nx, _)| (px - nx).abs() < x_threshold)
            .min_by(|(_, _, ay), (_, _, by)| (py - ay).abs().total_cmp(&(py - by).abs()));
        if let Some((nname, _, ny)) = nearest {
            compared += 1;
            // PMOS should have smaller y (higher on page) than its stage's NMOS
            if py > ny {
                violations.push(ConventionViolation {
                    pmos_device: pname.clone(),
                    pmos_y: *py,
                    nmos_device: nname.clone(),
                    nmos_y: *ny,
                });
            } else {
                valid_pairs += 1;
            }
        }
    }

    let score = if compared > 0 {
        round2(valid_pairs as f64 / compared as f64)
    } else {
        1.0
    };

    PowerConventionReport {
        pmos_count: pmos.len(),
        nmos_count: nmos.len(),
        violations,
        score,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Point};

    fn comp(name: &str, sym: &str, x: f64, y: f64) -> Component {
        Component {
            instance_name: name.into(),
            symbol_name: sym.into(),
            position: Point::new(x, y),
            rotation: 0,
            mirrored: false,
            properties: vec![],
        }
    }

    #[test]
    fn no_mosfets_perfect_score() {
        let r = check(&Schematic::new(""));
        assert_eq!(r.pmos_count, 0);
        assert_eq!(r.nmos_count, 0);
        assert_eq!(r.score, 1.0);
        assert!(r.violations.is_empty());
    }

    #[test]
    fn pmos_above_nmos_in_same_column_is_valid() {
        // Smaller y means higher on the page. PMOS at y=0, NMOS at y=100.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4", 0.0, 0.0));
        s.components.push(comp("M2", "nmos4", 0.0, 100.0));
        let r = check(&s);
        assert_eq!(r.pmos_count, 1);
        assert_eq!(r.nmos_count, 1);
        assert!(r.violations.is_empty());
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn pmos_below_nmos_in_same_column_violates() {
        // PMOS at y=100 (lower), NMOS at y=0 (higher) → violation.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4", 0.0, 100.0));
        s.components.push(comp("M2", "nmos4", 0.0, 0.0));
        let r = check(&s);
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.score, 0.0);
    }

    #[test]
    fn devices_in_different_columns_are_not_compared() {
        // x_threshold = 60 (symbol width). Pair separated by 200 → no comparison.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4", 0.0, 100.0));
        s.components.push(comp("M2", "nmos4", 200.0, 0.0));
        let r = check(&s);
        assert!(r.violations.is_empty());
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn mixed_compliance_yields_partial_score() {
        // M1 (pmos, y=0): nearest NMOS is M2 (y=0, Δ=0) → valid (0 ≤ 0).
        // M4 (pmos, y=100): nearest NMOS is M3 (y=50, Δ=50) → violation.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4", 0.0, 0.0));
        s.components.push(comp("M4", "pmos4", 0.0, 100.0));
        s.components.push(comp("M2", "nmos4", 0.0, 0.0));
        s.components.push(comp("M3", "nmos4", 0.0, 50.0));
        let r = check(&s);
        assert_eq!(r.violations.len(), 1);
        // 1 valid out of 2 compared
        assert_eq!(r.score, 0.5);
    }

    #[test]
    fn stacked_cmos_stages_in_one_column_are_valid() {
        // Two CMOS stages stacked vertically, each internally P-above-N
        // (the case-31 TG-DFF pattern). The lower stage's PMOS (y=220) sits
        // below the upper stage's NMOS (y=80) but must NOT be flagged: each
        // PMOS is checked only against its nearest NMOS.
        let mut s = Schematic::new("");
        s.components.push(comp("MP1", "pmos4", 0.0, 0.0));
        s.components.push(comp("MN1", "nmos4", 0.0, 80.0));
        s.components.push(comp("MP3", "pmos4", 0.0, 220.0));
        s.components.push(comp("MN3", "nmos4", 0.0, 300.0));
        let r = check(&s);
        assert!(r.violations.is_empty());
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn inverted_stage_below_valid_stage_is_caught() {
        // Upper stage correct (P above N); lower stage upside down
        // (N at y=220 above its P at y=300). Nearest-pairing still
        // catches the inverted stage.
        let mut s = Schematic::new("");
        s.components.push(comp("MP1", "pmos4", 0.0, 0.0));
        s.components.push(comp("MN1", "nmos4", 0.0, 80.0));
        s.components.push(comp("MN3", "nmos4", 0.0, 220.0));
        s.components.push(comp("MP3", "pmos4", 0.0, 300.0));
        let r = check(&s);
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.violations[0].pmos_device, "MP3");
        assert_eq!(r.score, 0.5);
    }
}
