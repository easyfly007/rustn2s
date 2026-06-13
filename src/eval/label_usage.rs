use crate::model::Schematic;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct LabelUsageReport {
    pub total_labels: usize,
    pub unique_label_names: usize,
    pub label_pairs: usize,
    pub direct_wires: usize,
    pub label_to_wire_ratio: f64,
}

pub fn check(schematic: &Schematic) -> LabelUsageReport {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for label in &schematic.labels {
        *counts.entry(&label.name).or_insert(0) += 1;
    }

    let unique = counts.len();
    let pairs = counts.values().filter(|&&c| c >= 2).count();
    let wires = schematic.wires.len();
    let total = schematic.labels.len();

    let ratio = if wires > 0 {
        pairs as f64 / wires as f64
    } else if pairs > 0 {
        f64::INFINITY
    } else {
        0.0
    };

    LabelUsageReport {
        total_labels: total,
        unique_label_names: unique,
        label_pairs: pairs,
        direct_wires: wires,
        label_to_wire_ratio: round2(ratio),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Label, Point, Wire};

    fn schematic_of(labels: &[(&str, f64, f64)], wire_count: usize) -> Schematic {
        let mut s = Schematic::new("");
        for &(name, x, y) in labels {
            s.labels.push(Label {
                name: name.into(),
                position: Point::new(x, y),
            });
        }
        for _ in 0..wire_count {
            s.wires.push(Wire {
                points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            });
        }
        s
    }

    #[test]
    fn no_labels_no_wires_yields_zero_ratio() {
        let r = check(&Schematic::new(""));
        assert_eq!(r.total_labels, 0);
        assert_eq!(r.unique_label_names, 0);
        assert_eq!(r.label_pairs, 0);
        assert_eq!(r.direct_wires, 0);
        assert_eq!(r.label_to_wire_ratio, 0.0);
    }

    #[test]
    fn label_pairs_counts_names_with_at_least_two_occurrences() {
        // "out" appears 2x → counts as a pair; "lone" appears 1x → not a pair.
        let s = schematic_of(
            &[("out", 0.0, 0.0), ("out", 50.0, 0.0), ("lone", 0.0, 50.0)],
            2,
        );
        let r = check(&s);
        assert_eq!(r.total_labels, 3);
        assert_eq!(r.unique_label_names, 2);
        assert_eq!(r.label_pairs, 1);
        assert_eq!(r.direct_wires, 2);
        assert_eq!(r.label_to_wire_ratio, 0.5); // 1 pair / 2 wires
    }

    #[test]
    fn three_copies_still_only_one_pair() {
        // Even with 3 copies of the same label, it's still ≥2 → counted as one pair.
        let s = schematic_of(&[("n", 0.0, 0.0), ("n", 1.0, 0.0), ("n", 2.0, 0.0)], 0);
        let r = check(&s);
        assert_eq!(r.label_pairs, 1);
    }
}
