use serde::{Serialize, Deserialize};
use super::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PinDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolPin {
    pub name: String,
    pub pin_number: i32,
    pub offset: Point,
    pub direction: PinDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolGraphic {
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Rect { x: f64, y: f64, width: f64, height: f64, filled: bool },
    Circle { cx: f64, cy: f64, radius: f64, filled: bool },
    Arc { cx: f64, cy: f64, radius: f64, start_angle: f64, span_angle: f64 },
    Polyline { points: Vec<Point>, filled: bool },
    Text { x: f64, y: f64, text: String, font_size: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDef {
    pub name: String,
    pub pins: Vec<SymbolPin>,
    pub graphics: Vec<SymbolGraphic>,
}

impl SymbolDef {
    pub fn bounding_rect(&self) -> super::geometry::Rect {
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

        for pin in &self.pins {
            expand(pin.offset.x, pin.offset.y);
        }

        for g in &self.graphics {
            match g {
                SymbolGraphic::Line { x1, y1, x2, y2 } => {
                    expand(*x1, *y1);
                    expand(*x2, *y2);
                }
                SymbolGraphic::Rect { x, y, width, height, .. } => {
                    expand(*x, *y);
                    expand(x + width, y + height);
                }
                SymbolGraphic::Circle { cx, cy, radius, .. } => {
                    expand(cx - radius, cy - radius);
                    expand(cx + radius, cy + radius);
                }
                SymbolGraphic::Arc { cx, cy, radius, .. } => {
                    expand(cx - radius, cy - radius);
                    expand(cx + radius, cy + radius);
                }
                SymbolGraphic::Polyline { points, .. } => {
                    for p in points {
                        expand(p.x, p.y);
                    }
                }
                SymbolGraphic::Text { x, y, .. } => {
                    expand(*x, *y);
                }
            }
        }

        if min_x > max_x {
            return super::geometry::Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        super::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

// ============================================================================
// Builtin analog symbols — matching C++ BuiltinSymbols
// ============================================================================

pub mod builtin_symbols {
    use super::*;
    use std::collections::HashMap;

    fn pin(name: &str, num: i32, x: f64, y: f64, dir: PinDirection) -> SymbolPin {
        SymbolPin { name: name.into(), pin_number: num, offset: Point::new(x, y), direction: dir }
    }

    fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> SymbolGraphic {
        SymbolGraphic::Line { x1, y1, x2, y2 }
    }

    fn polyline(pts: &[(f64, f64)], filled: bool) -> SymbolGraphic {
        SymbolGraphic::Polyline {
            points: pts.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            filled,
        }
    }

    fn circle(cx: f64, cy: f64, r: f64, filled: bool) -> SymbolGraphic {
        SymbolGraphic::Circle { cx, cy, radius: r, filled }
    }

    fn arc(cx: f64, cy: f64, r: f64, start: f64, span: f64) -> SymbolGraphic {
        SymbolGraphic::Arc { cx, cy, radius: r, start_angle: start, span_angle: span }
    }

    #[allow(dead_code)]
    fn text(x: f64, y: f64, t: &str, size: f64) -> SymbolGraphic {
        SymbolGraphic::Text { x, y, text: t.into(), font_size: size }
    }

    pub fn create_nmos4() -> SymbolDef {
        SymbolDef {
            name: "nmos4".into(),
            pins: vec![
                pin("G", 1, -30.0, 0.0, PinDirection::Left),
                pin("D", 2, 0.0, -20.0, PinDirection::Up),
                pin("S", 3, 0.0, 20.0, PinDirection::Down),
                pin("B", 4, 10.0, 0.0, PinDirection::Right),
            ],
            graphics: vec![
                line(-30.0, 0.0, -10.0, 0.0),
                line(-10.0, -15.0, -10.0, 15.0),
                line(-5.0, -15.0, -5.0, -5.0),
                line(-5.0, -10.0, 0.0, -10.0),
                line(0.0, -20.0, 0.0, -10.0),
                line(-5.0, 5.0, -5.0, 15.0),
                line(-5.0, 10.0, 0.0, 10.0),
                line(0.0, 10.0, 0.0, 20.0),
                line(-5.0, 0.0, 0.0, 0.0),
                line(0.0, 0.0, 10.0, 0.0),
                polyline(&[(-5.0, -2.0), (-2.0, 0.0), (-5.0, 2.0)], true),
            ],
        }
    }

    pub fn create_pmos4() -> SymbolDef {
        SymbolDef {
            name: "pmos4".into(),
            pins: vec![
                pin("G", 1, -30.0, 0.0, PinDirection::Left),
                pin("D", 2, 0.0, -20.0, PinDirection::Up),
                pin("S", 3, 0.0, 20.0, PinDirection::Down),
                pin("B", 4, 10.0, 0.0, PinDirection::Right),
            ],
            graphics: vec![
                line(-30.0, 0.0, -10.0, 0.0),
                line(-10.0, -15.0, -10.0, 15.0),
                line(-5.0, -15.0, -5.0, -5.0),
                line(-5.0, -10.0, 0.0, -10.0),
                line(0.0, -20.0, 0.0, -10.0),
                line(-5.0, 5.0, -5.0, 15.0),
                line(-5.0, 10.0, 0.0, 10.0),
                line(0.0, 10.0, 0.0, 20.0),
                line(-5.0, 0.0, 0.0, 0.0),
                line(0.0, 0.0, 10.0, 0.0),
                polyline(&[(-2.0, -2.0), (-5.0, 0.0), (-2.0, 2.0)], true),
                circle(-8.0, 0.0, 2.0, false),
            ],
        }
    }

    pub fn create_resistor() -> SymbolDef {
        SymbolDef {
            name: "resistor".into(),
            pins: vec![
                pin("P", 1, 0.0, -20.0, PinDirection::Up),
                pin("N", 2, 0.0, 20.0, PinDirection::Down),
            ],
            graphics: vec![
                line(0.0, -20.0, 0.0, -15.0),
                line(0.0, -15.0, 5.0, -12.0),
                line(5.0, -12.0, -5.0, -6.0),
                line(-5.0, -6.0, 5.0, 0.0),
                line(5.0, 0.0, -5.0, 6.0),
                line(-5.0, 6.0, 5.0, 12.0),
                line(5.0, 12.0, 0.0, 15.0),
                line(0.0, 15.0, 0.0, 20.0),
            ],
        }
    }

    pub fn create_capacitor() -> SymbolDef {
        SymbolDef {
            name: "capacitor".into(),
            pins: vec![
                pin("P", 1, 0.0, -15.0, PinDirection::Up),
                pin("N", 2, 0.0, 15.0, PinDirection::Down),
            ],
            graphics: vec![
                line(0.0, -15.0, 0.0, -3.0),
                line(-8.0, -3.0, 8.0, -3.0),
                line(-8.0, 3.0, 8.0, 3.0),
                line(0.0, 3.0, 0.0, 15.0),
            ],
        }
    }

    pub fn create_inductor() -> SymbolDef {
        SymbolDef {
            name: "inductor".into(),
            pins: vec![
                pin("P", 1, 0.0, -20.0, PinDirection::Up),
                pin("N", 2, 0.0, 20.0, PinDirection::Down),
            ],
            graphics: vec![
                line(0.0, -20.0, 0.0, -12.0),
                arc(0.0, -8.0, 4.0, 90.0, 180.0),
                arc(0.0, 0.0, 4.0, 90.0, 180.0),
                arc(0.0, 8.0, 4.0, 90.0, 180.0),
                line(0.0, 12.0, 0.0, 20.0),
            ],
        }
    }

    pub fn create_diode() -> SymbolDef {
        SymbolDef {
            name: "diode".into(),
            pins: vec![
                pin("A", 1, 0.0, -15.0, PinDirection::Up),
                pin("K", 2, 0.0, 15.0, PinDirection::Down),
            ],
            graphics: vec![
                line(0.0, -15.0, 0.0, -5.0),
                polyline(&[(-7.0, -5.0), (7.0, -5.0), (0.0, 5.0)], true),
                line(-7.0, 5.0, 7.0, 5.0),
                line(0.0, 5.0, 0.0, 15.0),
            ],
        }
    }

    pub fn create_npn() -> SymbolDef {
        SymbolDef {
            name: "npn".into(),
            pins: vec![
                pin("B", 1, -20.0, 0.0, PinDirection::Left),
                pin("C", 2, 10.0, -20.0, PinDirection::Up),
                pin("E", 3, 10.0, 20.0, PinDirection::Down),
            ],
            graphics: vec![
                line(-20.0, 0.0, -5.0, 0.0),
                line(-5.0, -12.0, -5.0, 12.0),
                line(-5.0, -7.0, 10.0, -17.0),
                line(10.0, -20.0, 10.0, -17.0),
                line(-5.0, 7.0, 10.0, 17.0),
                line(10.0, 17.0, 10.0, 20.0),
                polyline(&[(6.0, 14.0), (10.0, 17.0), (7.0, 11.0)], true),
            ],
        }
    }

    pub fn create_pnp() -> SymbolDef {
        SymbolDef {
            name: "pnp".into(),
            pins: vec![
                pin("B", 1, -20.0, 0.0, PinDirection::Left),
                pin("C", 2, 10.0, -20.0, PinDirection::Up),
                pin("E", 3, 10.0, 20.0, PinDirection::Down),
            ],
            graphics: vec![
                line(-20.0, 0.0, -5.0, 0.0),
                line(-5.0, -12.0, -5.0, 12.0),
                line(-5.0, -7.0, 10.0, -17.0),
                line(10.0, -20.0, 10.0, -17.0),
                line(-5.0, 7.0, 10.0, 17.0),
                line(10.0, 17.0, 10.0, 20.0),
                polyline(&[(-2.0, 4.0), (-5.0, 7.0), (-1.0, 10.0)], true),
            ],
        }
    }

    pub fn create_vsource() -> SymbolDef {
        SymbolDef {
            name: "vsource".into(),
            pins: vec![
                pin("P", 1, 0.0, -25.0, PinDirection::Up),
                pin("N", 2, 0.0, 25.0, PinDirection::Down),
            ],
            graphics: vec![
                circle(0.0, 0.0, 15.0, false),
                line(0.0, -25.0, 0.0, -15.0),
                line(0.0, 15.0, 0.0, 25.0),
                line(0.0, -9.0, 0.0, -3.0),
                line(-3.0, -6.0, 3.0, -6.0),
                line(-3.0, 6.0, 3.0, 6.0),
            ],
        }
    }

    pub fn create_isource() -> SymbolDef {
        SymbolDef {
            name: "isource".into(),
            pins: vec![
                pin("P", 1, 0.0, -25.0, PinDirection::Up),
                pin("N", 2, 0.0, 25.0, PinDirection::Down),
            ],
            graphics: vec![
                circle(0.0, 0.0, 15.0, false),
                line(0.0, -25.0, 0.0, -15.0),
                line(0.0, 15.0, 0.0, 25.0),
                line(0.0, 8.0, 0.0, -8.0),
                polyline(&[(-3.0, -4.0), (0.0, -8.0), (3.0, -4.0)], true),
            ],
        }
    }

    fn create_controlled_source(name: &str) -> SymbolDef {
        SymbolDef {
            name: name.into(),
            pins: vec![
                pin("NP", 1, 15.0, -10.0, PinDirection::Right),
                pin("NN", 2, 15.0, 10.0, PinDirection::Right),
                pin("CP", 3, -15.0, -10.0, PinDirection::Left),
                pin("CN", 4, -15.0, 10.0, PinDirection::Left),
            ],
            graphics: vec![
                polyline(&[(0.0, -15.0), (15.0, 0.0), (0.0, 15.0), (-15.0, 0.0), (0.0, -15.0)], false),
            ],
        }
    }

    pub fn create_vcvs() -> SymbolDef { create_controlled_source("vcvs") }
    pub fn create_vccs() -> SymbolDef { create_controlled_source("vccs") }
    pub fn create_ccvs() -> SymbolDef { create_controlled_source("ccvs") }
    pub fn create_cccs() -> SymbolDef { create_controlled_source("cccs") }

    /// Get all builtin symbols as a HashMap keyed by name.
    pub fn all() -> HashMap<String, SymbolDef> {
        let syms = vec![
            create_nmos4(), create_pmos4(),
            create_resistor(), create_capacitor(), create_inductor(),
            create_diode(), create_npn(), create_pnp(),
            create_vsource(), create_isource(),
            create_vcvs(), create_vccs(), create_ccvs(), create_cccs(),
        ];
        syms.into_iter().map(|s| (s.name.clone(), s)).collect()
    }
}
