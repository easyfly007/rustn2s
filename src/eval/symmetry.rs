use std::collections::HashMap;
use serde::Serialize;
use crate::model::Schematic;

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
    // Group components by (symbol_name, key properties like W/L/model)
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, comp) in schematic.components.iter().enumerate() {
        let mut key_parts = vec![comp.symbol_name.clone()];
        // Sort properties for stable key
        let mut props: Vec<(&str, &str)> = comp.properties.iter()
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

            // Symmetry: perfect if same y, reasonable x separation
            // Score: 1.0 when y_diff == 0, decays with y_diff
            let max_dim = x_diff.max(y_diff).max(1.0);
            let score = 1.0 - (y_diff / max_dim).min(1.0);

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
