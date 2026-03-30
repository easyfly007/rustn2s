use serde::Serialize;
use crate::model::Schematic;

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
