use crate::model::Schematic;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WireBendReport {
    pub total_bends: usize,
    pub wires_with_bends: usize,
    pub max_bends_per_wire: usize,
}

pub fn check(schematic: &Schematic) -> WireBendReport {
    let mut total = 0;
    let mut with_bends = 0;
    let mut max_bends = 0;

    for wire in &schematic.wires {
        let bends = wire.points.len().saturating_sub(2);
        total += bends;
        if bends > 0 {
            with_bends += 1;
        }
        max_bends = max_bends.max(bends);
    }

    WireBendReport {
        total_bends: total,
        wires_with_bends: with_bends,
        max_bends_per_wire: max_bends,
    }
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
    fn straight_wire_has_no_bends() {
        // 2 points = 1 segment = 0 bends
        let w = Wire {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        };
        let r = check(&schematic_with_wires(vec![w]));
        assert_eq!(r.total_bends, 0);
        assert_eq!(r.wires_with_bends, 0);
    }

    #[test]
    fn l_shape_has_one_bend() {
        // 3 points = 1 bend
        let w = Wire {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
            ],
        };
        let r = check(&schematic_with_wires(vec![w]));
        assert_eq!(r.total_bends, 1);
        assert_eq!(r.wires_with_bends, 1);
        assert_eq!(r.max_bends_per_wire, 1);
    }

    #[test]
    fn aggregates_across_wires_and_tracks_max() {
        let straight = Wire {
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        };
        let z_shape = Wire {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(5.0, 0.0),
                Point::new(5.0, 5.0),
                Point::new(10.0, 5.0),
            ],
        }; // 2 bends
        let r = check(&schematic_with_wires(vec![straight, z_shape]));
        assert_eq!(r.total_bends, 2);
        assert_eq!(r.wires_with_bends, 1);
        assert_eq!(r.max_bends_per_wire, 2);
    }
}
