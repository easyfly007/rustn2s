use serde::Serialize;
use crate::model::Schematic;

#[derive(Debug, Serialize)]
pub struct WireLengthReport {
    pub total_length: f64,
    pub wire_count: usize,
    pub avg_length: f64,
    pub max_length: f64,
    pub min_length: f64,
}

pub fn check(schematic: &Schematic) -> WireLengthReport {
    let mut total = 0.0;
    let mut max_len = 0.0f64;
    let mut min_len = f64::MAX;

    for wire in &schematic.wires {
        let mut wire_len = 0.0;
        for k in 0..wire.points.len().saturating_sub(1) {
            wire_len += wire.points[k].distance_to(&wire.points[k + 1]);
        }
        total += wire_len;
        max_len = max_len.max(wire_len);
        min_len = min_len.min(wire_len);
    }

    let count = schematic.wires.len();
    if count == 0 {
        min_len = 0.0;
    }

    WireLengthReport {
        total_length: round2(total),
        wire_count: count,
        avg_length: round2(if count > 0 { total / count as f64 } else { 0.0 }),
        max_length: round2(max_len),
        min_length: round2(min_len),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
