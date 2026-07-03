use crate::model::{builtin_symbols, Rect, Schematic, SymbolDef};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct OverlapReport {
    pub overlap_count: usize,
    pub overlapping_pairs: Vec<(String, String)>,
}

/// `subckt_symbols` sizes the `subckt_*` box components that builtin lookup
/// can't. Without it, X-instance boxes had no bounding rect and were
/// invisible to this check — overlapping boxes sailed through no_overlap
/// (found on test case 34's cross-coupled pairs, caught by eye only).
pub fn check(schematic: &Schematic, subckt_symbols: &HashMap<String, SymbolDef>) -> OverlapReport {
    let symbols = builtin_symbols::all();
    let mut rects: Vec<(String, Rect)> = Vec::new();

    for comp in &schematic.components {
        if let Some(sym) = symbols
            .get(&comp.symbol_name)
            .or_else(|| subckt_symbols.get(&comp.symbol_name))
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Point as MPoint};

    fn comp(name: &str, sym: &str, x: f64, y: f64) -> Component {
        Component {
            instance_name: name.into(),
            symbol_name: sym.into(),
            position: MPoint::new(x, y),
            rotation: 0,
            mirrored: false,
            properties: vec![],
        }
    }

    #[test]
    fn rects_overlap_geometry() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        // Strict overlap
        assert!(rects_overlap(&a, &Rect::new(5.0, 5.0, 10.0, 10.0)));
        // Disjoint
        assert!(!rects_overlap(&a, &Rect::new(20.0, 0.0, 10.0, 10.0)));
        // Just touching at edge — within 1.0 margin → not flagged
        assert!(!rects_overlap(&a, &Rect::new(10.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn no_components_yields_no_overlap() {
        let r = check(&Schematic::new(""), &HashMap::new());
        assert_eq!(r.overlap_count, 0);
        assert!(r.overlapping_pairs.is_empty());
    }

    #[test]
    fn far_apart_components_dont_overlap() {
        let mut s = Schematic::new("");
        s.components.push(comp("R1", "resistor", 0.0, 0.0));
        s.components.push(comp("R2", "resistor", 1000.0, 0.0));
        let r = check(&s, &HashMap::new());
        assert_eq!(r.overlap_count, 0);
    }

    #[test]
    fn coincident_components_overlap() {
        let mut s = Schematic::new("");
        s.components.push(comp("R1", "resistor", 0.0, 0.0));
        s.components.push(comp("R2", "resistor", 0.0, 0.0));
        let r = check(&s, &HashMap::new());
        assert_eq!(r.overlap_count, 1);
        let (a, b) = &r.overlapping_pairs[0];
        assert!((a == "R1" && b == "R2") || (a == "R2" && b == "R1"));
    }

    #[test]
    fn unknown_symbol_components_are_skipped() {
        // Components with unknown symbol_name don't get a bounding rect → no overlap.
        let mut s = Schematic::new("");
        s.components.push(comp("X1", "no_such_symbol", 0.0, 0.0));
        s.components.push(comp("X2", "no_such_symbol", 0.0, 0.0));
        let r = check(&s, &HashMap::new());
        assert_eq!(r.overlap_count, 0);
    }

    #[test]
    fn overlapping_subckt_boxes_are_flagged() {
        // Two X-instance boxes at the same position, sized via the provided
        // subckt symbol table (the case-34 blind spot).
        let ports: Vec<String> = (1..=4).map(|i| i.to_string()).collect();
        let mut subckt = HashMap::new();
        subckt.insert(
            "subckt_nfet_01v8".to_string(),
            builtin_symbols::create_subcircuit_symbol("nfet_01v8", &ports),
        );
        let mut s = Schematic::new("");
        s.components.push(comp("X1", "subckt_nfet_01v8", 0.0, 0.0));
        s.components.push(comp("X2", "subckt_nfet_01v8", 40.0, 0.0));
        let r = check(&s, &subckt);
        assert_eq!(r.overlap_count, 1);

        // Same pair far enough apart → clean.
        let mut s2 = Schematic::new("");
        s2.components.push(comp("X1", "subckt_nfet_01v8", 0.0, 0.0));
        s2.components
            .push(comp("X2", "subckt_nfet_01v8", 300.0, 0.0));
        let r2 = check(&s2, &subckt);
        assert_eq!(r2.overlap_count, 0);
    }
}
