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
        // x_threshold = 100. Pair separated by 200 → no comparison, no violation.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4",   0.0, 100.0));
        s.components.push(comp("M2", "nmos4", 200.0,   0.0));
        let r = check(&s);
        assert!(r.violations.is_empty());
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn mixed_compliance_yields_partial_score() {
        // M1 (pmos top) vs M3 (nmos bottom): valid.
        // M1 (pmos top) vs M2 (nmos top, in same column): valid (py=0 ≤ ny=0).
        // M4 (pmos bottom) vs M3 (nmos top): violation (py=100 > ny=0).
        // → 2 valid + 1 violation across 3 compared pairs in column 0.
        let mut s = Schematic::new("");
        s.components.push(comp("M1", "pmos4", 0.0,   0.0));
        s.components.push(comp("M4", "pmos4", 0.0, 100.0));
        s.components.push(comp("M2", "nmos4", 0.0,   0.0));
        s.components.push(comp("M3", "nmos4", 0.0,  50.0));
        let r = check(&s);
        // M1 vs M2 (0 ≤ 0): valid
        // M1 vs M3 (0 ≤ 50): valid
        // M4 vs M2 (100 > 0): violation
        // M4 vs M3 (100 > 50): violation
        assert_eq!(r.violations.len(), 2);
        // 2 valid out of 4 compared
        assert_eq!(r.score, 0.5);
    }
}
