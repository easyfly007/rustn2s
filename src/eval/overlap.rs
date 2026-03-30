use serde::Serialize;
use crate::model::{Schematic, Rect, builtin_symbols};

#[derive(Debug, Serialize)]
pub struct OverlapReport {
    pub overlap_count: usize,
    pub overlapping_pairs: Vec<(String, String)>,
}

pub fn check(schematic: &Schematic) -> OverlapReport {
    let symbols = builtin_symbols::all();
    let mut rects: Vec<(String, Rect)> = Vec::new();

    for comp in &schematic.components {
        if let Some(sym) = symbols.get(&comp.symbol_name) {
            let base = sym.bounding_rect();
            // Transform bounding rect corners
            let corners = [
                crate::model::Point::new(base.left(), base.top()),
                crate::model::Point::new(base.right(), base.top()),
                crate::model::Point::new(base.left(), base.bottom()),
                crate::model::Point::new(base.right(), base.bottom()),
            ];
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            for c in &corners {
                let t = c.transform(comp.rotation, comp.mirrored);
                let world = comp.position + t;
                min_x = min_x.min(world.x);
                min_y = min_y.min(world.y);
                max_x = max_x.max(world.x);
                max_y = max_y.max(world.y);
            }
            rects.push((
                comp.instance_name.clone(),
                Rect::new(min_x, min_y, max_x - min_x, max_y - min_y),
            ));
        }
    }

    let mut overlapping_pairs = Vec::new();
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if rects_overlap(&rects[i].1, &rects[j].1) {
                overlapping_pairs.push((rects[i].0.clone(), rects[j].0.clone()));
            }
        }
    }

    OverlapReport {
        overlap_count: overlapping_pairs.len(),
        overlapping_pairs,
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    // Use a small margin to avoid false positives from touching edges
    let margin = 1.0;
    a.left() + margin < b.right()
        && b.left() + margin < a.right()
        && a.top() + margin < b.bottom()
        && b.top() + margin < a.bottom()
}
