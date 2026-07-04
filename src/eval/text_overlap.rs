//! Text-collision metric: does any rendered text sit on top of something
//! else? The geometric `overlap` metric sees only symbol bodies, so a
//! schematic could score a perfect quality while every net label printed
//! over a component box (test case 27 scored 1.000 while visually
//! illegible; case 34's label-on-box defect was invisible to every score).
//! This check mirrors the SVG renderer's text geometry:
//!
//! - net label boxes: 50x16, centered on `label.position` (svg.rs
//!   render_labels);
//! - instance-name captions: 11px monospace anchored just off the
//!   symbol's top-right corner (svg.rs render_components).
//!
//! Not modeled (v1): pin-name texts (8px, tiny and tied to their own
//! symbol) and power-symbol rail texts.

use crate::model::{builtin_symbols, Rect, Schematic, SymbolDef};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct TextOverlapReport {
    /// Number of text elements considered (labels + captions).
    pub total_texts: usize,
    /// Text elements that collide with at least one other text or a
    /// component body.
    pub dirty_texts: usize,
    /// Sample of colliding pairs (capped), for diagnosis.
    pub collisions: Vec<(String, String)>,
    /// clean / total, 1.0 when nothing collides (or there is no text).
    pub score: f64,
}

const MAX_REPORTED_PAIRS: usize = 20;

/// Caption text metrics matching the SVG `.name` class (11px monospace).
const CAPTION_CHAR_W: f64 = 6.6;
const CAPTION_H: f64 = 11.0;

pub fn check(
    schematic: &Schematic,
    subckt_symbols: &HashMap<String, SymbolDef>,
) -> TextOverlapReport {
    let builtin = builtin_symbols::all();

    // Component body world rects — same transform logic as eval/overlap.
    let mut bodies: Vec<(String, Rect)> = Vec::new();
    // Symbol-local bounding rects, reused for caption anchoring below.
    let mut local_bounds: Vec<Option<Rect>> = Vec::new();
    for comp in &schematic.components {
        let base = builtin
            .get(&comp.symbol_name)
            .or_else(|| subckt_symbols.get(&comp.symbol_name))
            .map(|s| s.bounding_rect());
        local_bounds.push(base);
        if let Some(base) = base {
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
                min_x = min_x.min(comp.position.x + t.x);
                min_y = min_y.min(comp.position.y + t.y);
                max_x = max_x.max(comp.position.x + t.x);
                max_y = max_y.max(comp.position.y + t.y);
            }
            bodies.push((
                comp.instance_name.clone(),
                Rect::new(min_x, min_y, max_x - min_x, max_y - min_y),
            ));
        } else {
            bodies.push((
                comp.instance_name.clone(),
                Rect::new(comp.position.x - 20.0, comp.position.y - 15.0, 40.0, 30.0),
            ));
        }
    }

    // Text elements: (description, rect, owner-component index or None).
    let mut texts: Vec<(String, Rect, Option<usize>)> = Vec::new();

    // Net label boxes (svg.rs: text-adaptive width, 16 tall).
    for label in &schematic.labels {
        let w = crate::model::label_box_width(&label.name);
        texts.push((
            format!("label \"{}\"", label.name),
            Rect::new(label.position.x - w / 2.0, label.position.y - 8.0, w, 16.0),
            None,
        ));
    }

    // Instance-name captions (svg.rs: start-anchored at right+4, top-2,
    // dominant-baseline central → the text box straddles that y).
    for (ci, comp) in schematic.components.iter().enumerate() {
        if comp.instance_name.is_empty() {
            continue;
        }
        let b = local_bounds[ci].unwrap_or(Rect::new(-15.0, -15.0, 30.0, 30.0));
        let ax = comp.position.x + b.right() + 4.0;
        let ay = comp.position.y + b.top() - 2.0;
        texts.push((
            format!("caption \"{}\"", comp.instance_name),
            Rect::new(
                ax,
                ay - CAPTION_H / 2.0,
                comp.instance_name.len() as f64 * CAPTION_CHAR_W,
                CAPTION_H,
            ),
            Some(ci),
        ));
    }

    let mut dirty = vec![false; texts.len()];
    let mut collisions: Vec<(String, String)> = Vec::new();
    let record = |collisions: &mut Vec<(String, String)>, a: String, b: String| {
        if collisions.len() < MAX_REPORTED_PAIRS {
            collisions.push((a, b));
        }
    };

    // Text vs text.
    for i in 0..texts.len() {
        for j in (i + 1)..texts.len() {
            if rects_overlap(&texts[i].1, &texts[j].1) {
                dirty[i] = true;
                dirty[j] = true;
                record(&mut collisions, texts[i].0.clone(), texts[j].0.clone());
            }
        }
    }

    // Text vs component body. A caption may not collide with its OWN
    // symbol (it is anchored at that symbol's corner by construction and
    // small rounding there is not a readability problem).
    for (ti, (desc, rect, owner)) in texts.iter().enumerate() {
        for (bi, (bname, brect)) in bodies.iter().enumerate() {
            if *owner == Some(bi) {
                continue;
            }
            if rects_overlap(rect, brect) {
                dirty[ti] = true;
                record(
                    &mut collisions,
                    desc.clone(),
                    format!("component {}", bname),
                );
            }
        }
    }

    let total = texts.len();
    let dirty_count = dirty.iter().filter(|&&d| d).count();
    let score = if total == 0 {
        1.0
    } else {
        round2((total - dirty_count) as f64 / total as f64)
    };

    TextOverlapReport {
        total_texts: total,
        dirty_texts: dirty_count,
        collisions,
        score,
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    // Same 1px touching margin as the component overlap metric.
    let margin = 1.0;
    a.left() + margin < b.right()
        && b.left() + margin < a.right()
        && a.top() + margin < b.bottom()
        && b.top() + margin < a.bottom()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Label, Point};

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

    fn label(name: &str, x: f64, y: f64) -> Label {
        Label {
            name: name.into(),
            position: Point::new(x, y),
        }
    }

    #[test]
    fn empty_schematic_scores_one() {
        let r = check(&Schematic::new(""), &HashMap::new());
        assert_eq!(r.total_texts, 0);
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn label_on_component_body_is_dirty() {
        // The case-34 defect: a net label printed dead-center on a box.
        let ports: Vec<String> = (1..=4).map(|i| i.to_string()).collect();
        let mut subckt = HashMap::new();
        subckt.insert(
            "subckt_nfet_01v8".to_string(),
            builtin_symbols::create_subcircuit_symbol("nfet_01v8", &ports),
        );
        let mut s = Schematic::new("");
        s.components.push(comp("X1", "subckt_nfet_01v8", 0.0, 0.0));
        s.labels.push(label("out", 0.0, 0.0));
        let r = check(&s, &subckt);
        assert!(r.dirty_texts >= 1);
        assert!(r.score < 1.0);
    }

    #[test]
    fn label_beside_component_is_clean() {
        let ports: Vec<String> = (1..=4).map(|i| i.to_string()).collect();
        let mut subckt = HashMap::new();
        subckt.insert(
            "subckt_nfet_01v8".to_string(),
            builtin_symbols::create_subcircuit_symbol("nfet_01v8", &ports),
        );
        let mut s = Schematic::new("");
        s.components.push(comp("X1", "subckt_nfet_01v8", 0.0, 0.0));
        // Box half-width ≈ 37 + 15 stub; label half-width 25 → x=150 clears
        // both the body and the caption at the top-right corner.
        s.labels.push(label("out", 150.0, 0.0));
        let r = check(&s, &subckt);
        assert_eq!(r.dirty_texts, 0, "collisions: {:?}", r.collisions);
        assert_eq!(r.score, 1.0);
    }

    #[test]
    fn overlapping_labels_are_dirty() {
        let mut s = Schematic::new("");
        s.labels.push(label("a", 0.0, 0.0));
        s.labels.push(label("b", 20.0, 0.0)); // 50-wide boxes at dx=20 overlap
        let r = check(&s, &HashMap::new());
        assert_eq!(r.dirty_texts, 2);
        assert_eq!(r.score, 0.0);
    }

    #[test]
    fn caption_does_not_collide_with_own_symbol() {
        let mut s = Schematic::new("");
        s.components.push(comp("R1", "resistor", 0.0, 0.0));
        let r = check(&s, &HashMap::new());
        assert_eq!(r.dirty_texts, 0, "collisions: {:?}", r.collisions);
    }
}
