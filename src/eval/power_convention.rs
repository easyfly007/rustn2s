use serde::Serialize;
use crate::model::Schematic;

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

    let mut violations = Vec::new();
    let x_threshold = 100.0; // Only compare devices in similar columns

    for (pname, px, py) in &pmos {
        for (nname, nx, ny) in &nmos {
            if (px - nx).abs() <= x_threshold {
                // PMOS should have smaller y (higher on page) than NMOS
                if py > ny {
                    violations.push(ConventionViolation {
                        pmos_device: pname.clone(),
                        pmos_y: *py,
                        nmos_device: nname.clone(),
                        nmos_y: *ny,
                    });
                }
            }
        }
    }

    let total_pairs = pmos.len() * nmos.len().max(1);
    let score = if total_pairs == 0 {
        1.0
    } else {
        let valid_pairs = pmos.iter().flat_map(|(_, px, py)| {
            nmos.iter().filter(move |(_, nx, _)| (px - nx).abs() <= x_threshold)
                .map(move |(_, _, ny)| if py <= ny { 1.0 } else { 0.0 })
        }).sum::<f64>();
        let compared = pmos.iter().flat_map(|(_, px, _)| {
            nmos.iter().filter(move |(_, nx, _)| (px - nx).abs() <= x_threshold)
        }).count();
        if compared > 0 { round2(valid_pairs / compared as f64) } else { 1.0 }
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
