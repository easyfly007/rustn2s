use serde::Serialize;
use crate::model::{Schematic, builtin_symbols};

#[derive(Debug, Serialize)]
pub struct BoundingBoxReport {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub width: f64,
    pub height: f64,
    pub area: f64,
    pub aspect_ratio: f64,
    pub component_count: usize,
}

pub fn check(schematic: &Schematic) -> BoundingBoxReport {
    let symbols = builtin_symbols::all();
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    let mut expand = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };

    // Components (with symbol bounds)
    for comp in &schematic.components {
        if let Some(sym) = symbols.get(&comp.symbol_name) {
            let base = sym.bounding_rect();
            let corners = [
                crate::model::Point::new(base.left(), base.top()),
                crate::model::Point::new(base.right(), base.top()),
                crate::model::Point::new(base.left(), base.bottom()),
                crate::model::Point::new(base.right(), base.bottom()),
            ];
            for c in &corners {
                let t = c.transform(comp.rotation, comp.mirrored);
                let world = comp.position + t;
                expand(world.x, world.y);
            }
        } else {
            expand(comp.position.x, comp.position.y);
        }
    }

    // Wire points
    for wire in &schematic.wires {
        for p in &wire.points {
            expand(p.x, p.y);
        }
    }

    // Labels
    for label in &schematic.labels {
        expand(label.position.x, label.position.y);
    }

    // Power symbols
    for ps in &schematic.power_symbols {
        expand(ps.position.x, ps.position.y);
    }

    if min_x > max_x {
        return BoundingBoxReport {
            min_x: 0.0, min_y: 0.0, max_x: 0.0, max_y: 0.0,
            width: 0.0, height: 0.0, area: 0.0, aspect_ratio: 1.0,
            component_count: 0,
        };
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let area = width * height;
    let aspect_ratio = if height > 0.0 && width > 0.0 {
        let r = width / height;
        if r >= 1.0 { r } else { 1.0 / r }
    } else {
        1.0
    };

    BoundingBoxReport {
        min_x: round2(min_x),
        min_y: round2(min_y),
        max_x: round2(max_x),
        max_y: round2(max_y),
        width: round2(width),
        height: round2(height),
        area: round2(area),
        aspect_ratio: round2(aspect_ratio),
        component_count: schematic.components.len(),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
