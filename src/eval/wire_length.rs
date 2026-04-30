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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Point, Wire};

    fn schematic_with_wires(wires: Vec<Wire>) -> Schematic {
        let mut s = Schematic::new("");
        s.wires = wires;
        s
    }

    #[test]
    fn empty_schematic_yields_zeros() {
        let r = check(&schematic_with_wires(vec![]));
        assert_eq!(r.wire_count, 0);
        assert_eq!(r.total_length, 0.0);
        assert_eq!(r.avg_length, 0.0);
        assert_eq!(r.min_length, 0.0);
    }

    #[test]
    fn single_segment_length() {
        let w = Wire { points: vec![Point::new(0.0, 0.0), Point::new(3.0, 4.0)] };
        let r = check(&schematic_with_wires(vec![w]));
        assert_eq!(r.wire_count, 1);
        assert_eq!(r.total_length, 5.0);
        assert_eq!(r.max_length, 5.0);
        assert_eq!(r.min_length, 5.0);
        assert_eq!(r.avg_length, 5.0);
    }

    #[test]
    fn multi_wire_min_max_avg() {
        let short = Wire { points: vec![Point::new(0.0, 0.0), Point::new(0.0, 10.0)] };
        let long = Wire { points: vec![
            Point::new(0.0, 0.0), Point::new(50.0, 0.0), Point::new(50.0, 50.0),
        ] };
        let r = check(&schematic_with_wires(vec![short, long]));
        assert_eq!(r.wire_count, 2);
        assert_eq!(r.min_length, 10.0);
        assert_eq!(r.max_length, 100.0);
        assert_eq!(r.total_length, 110.0);
        assert_eq!(r.avg_length, 55.0);
    }
}
