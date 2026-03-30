use std::collections::HashMap;
use serde::Serialize;
use crate::model::Schematic;

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
