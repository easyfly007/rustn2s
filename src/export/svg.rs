use crate::model::{Schematic, Point, PowerType, SymbolGraphic, builtin_symbols};
use std::collections::HashMap;
use std::fmt::Write;

pub struct SvgTheme {
    pub background: String,
    pub wire_color: String,
    pub label_color: String,
    pub junction_color: String,
    pub gnd_color: String,
    pub vdd_color: String,
    pub pin_color: String,
    pub text_color: String,
    pub subtext_color: String,
    pub grid_color: String,
    pub component_stroke: String,
    pub component_fill: String,
    pub label_bg: String,
}

impl Default for SvgTheme {
    fn default() -> Self {
        Self {
            background: "#1a1a2e".into(),
            wire_color: "#4cc9f0".into(),
            label_color: "#e8c547".into(),
            junction_color: "#4cc9f0".into(),
            gnd_color: "#4cc9f0".into(),
            vdd_color: "#e76f51".into(),
            pin_color: "#f4a261".into(),
            text_color: "#e0e0e0".into(),
            subtext_color: "#888888".into(),
            grid_color: "#333333".into(),
            component_stroke: "#9a8c98".into(),
            component_fill: "#4a4e69".into(),
            label_bg: "#2a2a4a".into(),
        }
    }
}

pub struct SvgOptions {
    pub scale: f64,
    pub show_grid: bool,
    pub grid_spacing: f64,
    pub show_pin_names: bool,
    pub show_instance_names: bool,
    pub show_symbol_names: bool,
    pub show_legend: bool,
    pub margin: f64,
    pub theme: SvgTheme,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            show_grid: true,
            grid_spacing: 40.0,
            show_pin_names: true,
            show_instance_names: true,
            show_symbol_names: true,
            show_legend: true,
            margin: 80.0,
            theme: SvgTheme::default(),
        }
    }
}

pub fn render_to_svg(schematic: &Schematic, opts: &SvgOptions) -> String {
    let symbols = builtin_symbols::all();
    let bounds = compute_bounds(schematic, &symbols);
    let margin = opts.margin;
    let w = (bounds.2 + margin * 2.0) * opts.scale;
    let h = (bounds.3 + margin * 2.0) * opts.scale;
    let ox = (-bounds.0 + margin) * opts.scale;
    let oy = (-bounds.1 + margin) * opts.scale;

    let mut svg = String::with_capacity(8192);
    let t = &opts.theme;

    // Header
    write!(svg, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <svg xmlns=\"http://www.w3.org/2000/svg\" \
        width=\"{w}\" height=\"{h}\" \
        viewBox=\"0 0 {w} {h}\" \
        style=\"background:{bg}\">\n",
        bg = t.background).unwrap();

    // CSS
    write!(svg, "<defs><style>\n\
  .wire {{ stroke:{wc}; stroke-width:2; fill:none; }}\n\
  .comp-stroke {{ stroke:{cs}; stroke-width:1.5; fill:none; }}\n\
  .comp-fill {{ stroke:{cs}; stroke-width:1.5; fill:{cf}; }}\n\
  .pin {{ fill:{pc}; }}\n\
  .pin-name {{ fill:{pc}; font-family:monospace; font-size:8px; dominant-baseline:central; }}\n\
  .name {{ fill:{tc}; font-family:monospace; font-size:11px; text-anchor:middle; dominant-baseline:central; }}\n\
  .sub {{ fill:{sc}; font-family:monospace; font-size:9px; text-anchor:middle; dominant-baseline:central; }}\n\
  .lbl {{ fill:{lc}; font-family:monospace; font-size:10px; text-anchor:middle; dominant-baseline:central; }}\n\
  .junc {{ fill:{jc}; }}\n\
</style></defs>\n",
        wc = t.wire_color, cs = t.component_stroke, cf = t.component_fill,
        pc = t.pin_color, tc = t.text_color, sc = t.subtext_color,
        lc = t.label_color, jc = t.junction_color).unwrap();

    // Grid
    if opts.show_grid {
        render_grid(&mut svg, w, h, opts);
    }

    // Wires
    render_wires(&mut svg, schematic, ox, oy, opts);

    // Components
    render_components(&mut svg, schematic, ox, oy, opts, &symbols);

    // Power symbols
    render_power_symbols(&mut svg, schematic, ox, oy, opts);

    // Labels
    render_labels(&mut svg, schematic, ox, oy, opts);

    // Junctions
    render_junctions(&mut svg, schematic, ox, oy, opts);

    // Legend
    if opts.show_legend {
        render_legend(&mut svg, w, h, opts);
    }

    svg.push_str("</svg>\n");
    svg
}

pub fn render_to_file(schematic: &Schematic, path: &str, opts: &SvgOptions) -> Result<(), String> {
    let svg = render_to_svg(schematic, opts);
    std::fs::write(path, &svg).map_err(|e| format!("Cannot write {}: {}", path, e))
}

// ============================================================================
// Bounds
// ============================================================================

fn compute_bounds(sch: &Schematic, symbols: &HashMap<String, crate::model::SymbolDef>) -> (f64, f64, f64, f64) {
    let mut min_x = 1e9f64;
    let mut min_y = 1e9f64;
    let mut max_x = -1e9f64;
    let mut max_y = -1e9f64;
    let mut has_any = false;

    let mut expand = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        has_any = true;
    };

    for comp in &sch.components {
        if let Some(sym) = symbols.get(&comp.symbol_name) {
            let br = sym.bounding_rect();
            for &(bx, by) in &[
                (br.left(), br.top()), (br.right(), br.top()),
                (br.left(), br.bottom()), (br.right(), br.bottom()),
            ] {
                let p = Point::new(bx, by).transform(comp.rotation, comp.mirrored);
                expand(comp.position.x + p.x, comp.position.y + p.y);
            }
            for pin in &sym.pins {
                let wp = pin.offset.transform(comp.rotation, comp.mirrored);
                expand(comp.position.x + wp.x, comp.position.y + wp.y);
            }
        } else {
            expand(comp.position.x - 20.0, comp.position.y - 15.0);
            expand(comp.position.x + 20.0, comp.position.y + 15.0);
        }
    }
    for wire in &sch.wires {
        for pt in &wire.points { expand(pt.x, pt.y); }
    }
    for label in &sch.labels {
        expand(label.position.x - 30.0, label.position.y - 10.0);
        expand(label.position.x + 30.0, label.position.y + 10.0);
    }
    for ps in &sch.power_symbols {
        expand(ps.position.x - 15.0, ps.position.y - 15.0);
        expand(ps.position.x + 15.0, ps.position.y + 15.0);
    }
    for j in &sch.junctions {
        expand(j.position.x - 5.0, j.position.y - 5.0);
        expand(j.position.x + 5.0, j.position.y + 5.0);
    }

    if !has_any { return (0.0, 0.0, 200.0, 200.0); }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

// ============================================================================
// Render helpers
// ============================================================================

fn render_grid(svg: &mut String, w: f64, h: f64, opts: &SvgOptions) {
    let spacing = opts.grid_spacing * opts.scale;
    let mut gx = 0.0;
    while gx < w {
        let mut gy = 0.0;
        while gy < h {
            write!(svg, "  <circle cx=\"{gx}\" cy=\"{gy}\" r=\"0.5\" fill=\"{}\"/>\n",
                opts.theme.grid_color).unwrap();
            gy += spacing;
        }
        gx += spacing;
    }
}

fn render_wires(svg: &mut String, sch: &Schematic, ox: f64, oy: f64, opts: &SvgOptions) {
    let sc = opts.scale;
    for wire in &sch.wires {
        if wire.points.len() < 2 { continue; }
        svg.push_str("  <polyline class=\"wire\" points=\"");
        for (i, pt) in wire.points.iter().enumerate() {
            if i > 0 { svg.push(' '); }
            write!(svg, "{},{}", pt.x * sc + ox, pt.y * sc + oy).unwrap();
        }
        svg.push_str("\"/>\n");
    }
}

fn render_components(
    svg: &mut String, sch: &Schematic, ox: f64, oy: f64,
    opts: &SvgOptions, symbols: &HashMap<String, crate::model::SymbolDef>,
) {
    let sc = opts.scale;
    for comp in &sch.components {
        let cx = comp.position.x * sc + ox;
        let cy = comp.position.y * sc + oy;

        if let Some(sym) = symbols.get(&comp.symbol_name) {
            // Render symbol graphics
            for graphic in &sym.graphics {
                render_graphic(svg, graphic, cx, cy, comp.rotation, comp.mirrored, opts);
            }
            // Render pin dots
            for pin in &sym.pins {
                let wp = pin.offset.transform(comp.rotation, comp.mirrored);
                let px = cx + wp.x * sc;
                let py = cy + wp.y * sc;
                write!(svg, "  <circle cx=\"{px}\" cy=\"{py}\" r=\"2.5\" class=\"pin\"/>\n").unwrap();

                if opts.show_pin_names {
                    let (name_x, anchor) = match pin.direction {
                        crate::model::PinDirection::Left => (px - 6.0, "end"),
                        crate::model::PinDirection::Right => (px + 6.0, "start"),
                        _ => (px, "middle"),
                    };
                    let name_y = match pin.direction {
                        crate::model::PinDirection::Up => py - 6.0,
                        crate::model::PinDirection::Down => py + 10.0,
                        _ => py,
                    };
                    write!(svg, "  <text x=\"{name_x}\" y=\"{name_y}\" class=\"pin-name\" \
                        text-anchor=\"{anchor}\">{}</text>\n", pin.name).unwrap();
                }
            }
        } else {
            // Fallback rectangle
            let rw = 20.0 * sc;
            let rh = 15.0 * sc;
            write!(svg, "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" class=\"comp-fill\"/>\n",
                cx - rw, cy - rh, rw * 2.0, rh * 2.0).unwrap();
        }

        if opts.show_instance_names && !comp.instance_name.is_empty() {
            write!(svg, "  <text x=\"{cx}\" y=\"{}\" class=\"name\">{}</text>\n",
                cy - 3.0 * sc, comp.instance_name).unwrap();
        }
        if opts.show_symbol_names {
            write!(svg, "  <text x=\"{cx}\" y=\"{}\" class=\"sub\">{}</text>\n",
                cy + 10.0 * sc, comp.symbol_name.to_uppercase()).unwrap();
        }
    }
}

fn render_graphic(
    svg: &mut String, g: &SymbolGraphic, cx: f64, cy: f64,
    rot: i32, mir: bool, opts: &SvgOptions,
) {
    let sc = opts.scale;
    let stroke = &opts.theme.component_stroke;
    let fill = &opts.theme.component_fill;

    match g {
        SymbolGraphic::Line { x1, y1, x2, y2 } => {
            let p1 = Point::new(*x1, *y1).transform(rot, mir);
            let p2 = Point::new(*x2, *y2).transform(rot, mir);
            write!(svg, "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" \
                stroke=\"{stroke}\" stroke-width=\"1.5\"/>\n",
                cx + p1.x * sc, cy + p1.y * sc, cx + p2.x * sc, cy + p2.y * sc).unwrap();
        }
        SymbolGraphic::Rect { x, y, width, height, filled } => {
            let c0 = Point::new(*x, *y).transform(rot, mir);
            let c1 = Point::new(x + width, *y).transform(rot, mir);
            let c2 = Point::new(x + width, y + height).transform(rot, mir);
            let c3 = Point::new(*x, y + height).transform(rot, mir);
            let f = if *filled { fill.as_str() } else { "none" };
            write!(svg, "  <polygon points=\"{},{} {},{} {},{} {},{}\" \
                stroke=\"{stroke}\" stroke-width=\"1.5\" fill=\"{f}\"/>\n",
                cx + c0.x * sc, cy + c0.y * sc,
                cx + c1.x * sc, cy + c1.y * sc,
                cx + c2.x * sc, cy + c2.y * sc,
                cx + c3.x * sc, cy + c3.y * sc).unwrap();
        }
        SymbolGraphic::Circle { cx: gcx, cy: gcy, radius, filled } => {
            let center = Point::new(*gcx, *gcy).transform(rot, mir);
            let f = if *filled { fill.as_str() } else { "none" };
            write!(svg, "  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" \
                stroke=\"{stroke}\" stroke-width=\"1.5\" fill=\"{f}\"/>\n",
                cx + center.x * sc, cy + center.y * sc, radius * sc).unwrap();
        }
        SymbolGraphic::Arc { cx: gcx, cy: gcy, radius, start_angle, span_angle } => {
            let center = Point::new(*gcx, *gcy).transform(rot, mir);
            let mut sa = *start_angle;
            let sp = *span_angle;
            if mir { sa = 180.0 - sa - sp; }
            sa += rot as f64;
            let sr = sa * std::f64::consts::PI / 180.0;
            let er = (sa + sp) * std::f64::consts::PI / 180.0;
            let sx = (cx + center.x * sc) + radius * sc * sr.cos();
            let sy = (cy + center.y * sc) - radius * sc * sr.sin();
            let ex = (cx + center.x * sc) + radius * sc * er.cos();
            let ey = (cy + center.y * sc) - radius * sc * er.sin();
            let la = if sp.abs() > 180.0 { 1 } else { 0 };
            let sw = if sp > 0.0 { 0 } else { 1 };
            let r = radius * sc;
            write!(svg, "  <path d=\"M {sx} {sy} A {r} {r} 0 {la} {sw} {ex} {ey}\" \
                stroke=\"{stroke}\" stroke-width=\"1.5\" fill=\"none\"/>\n").unwrap();
        }
        SymbolGraphic::Polyline { points, filled } => {
            if points.is_empty() { return; }
            let tag = if *filled { "polygon" } else { "polyline" };
            let f = if *filled { fill.as_str() } else { "none" };
            write!(svg, "  <{tag} points=\"").unwrap();
            for (i, pt) in points.iter().enumerate() {
                if i > 0 { svg.push(' '); }
                let tp = pt.transform(rot, mir);
                write!(svg, "{},{}", cx + tp.x * sc, cy + tp.y * sc).unwrap();
            }
            write!(svg, "\" stroke=\"{stroke}\" stroke-width=\"1.5\" fill=\"{f}\"/>\n").unwrap();
        }
        SymbolGraphic::Text { x, y, text, font_size } => {
            let tp = Point::new(*x, *y).transform(rot, mir);
            write!(svg, "  <text x=\"{}\" y=\"{}\" font-family=\"monospace\" \
                fill=\"{}\" font-size=\"{font_size}px\" dominant-baseline=\"central\">{text}</text>\n",
                cx + tp.x * sc, cy + tp.y * sc, opts.theme.text_color).unwrap();
        }
    }
}

fn render_power_symbols(svg: &mut String, sch: &Schematic, ox: f64, oy: f64, opts: &SvgOptions) {
    let sc = opts.scale;
    for ps in &sch.power_symbols {
        let px = ps.position.x * sc + ox;
        let py = ps.position.y * sc + oy;

        match ps.power_type {
            PowerType::GND => {
                let c = &opts.theme.gnd_color;
                write!(svg, "  <line x1=\"{px}\" y1=\"{}\" x2=\"{px}\" y2=\"{py}\" \
                    stroke=\"{c}\" stroke-width=\"1.5\"/>\n", py - 8.0 * sc).unwrap();
                write!(svg, "  <line x1=\"{}\" y1=\"{py}\" x2=\"{}\" y2=\"{py}\" \
                    stroke=\"{c}\" stroke-width=\"2\"/>\n", px - 8.0 * sc, px + 8.0 * sc).unwrap();
                write!(svg, "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" \
                    stroke=\"{c}\" stroke-width=\"1.5\"/>\n",
                    px - 5.0 * sc, py + 3.0 * sc, px + 5.0 * sc, py + 3.0 * sc).unwrap();
                write!(svg, "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" \
                    stroke=\"{c}\" stroke-width=\"1\"/>\n",
                    px - 2.0 * sc, py + 6.0 * sc, px + 2.0 * sc, py + 6.0 * sc).unwrap();
                write!(svg, "  <text x=\"{px}\" y=\"{}\" class=\"lbl\">{}</text>\n",
                    py + 16.0 * sc, ps.net_name).unwrap();
            }
            _ => {
                let c = &opts.theme.vdd_color;
                write!(svg, "  <line x1=\"{px}\" y1=\"{}\" x2=\"{px}\" y2=\"{py}\" \
                    stroke=\"{c}\" stroke-width=\"1.5\"/>\n", py + 8.0 * sc).unwrap();
                write!(svg, "  <line x1=\"{}\" y1=\"{py}\" x2=\"{}\" y2=\"{py}\" \
                    stroke=\"{c}\" stroke-width=\"2\"/>\n", px - 8.0 * sc, px + 8.0 * sc).unwrap();
                write!(svg, "  <text x=\"{px}\" y=\"{}\" class=\"lbl\" fill=\"{c}\">{}</text>\n",
                    py - 8.0 * sc, ps.net_name).unwrap();
            }
        }
    }
}

fn render_labels(svg: &mut String, sch: &Schematic, ox: f64, oy: f64, opts: &SvgOptions) {
    let sc = opts.scale;
    for label in &sch.labels {
        let lx = label.position.x * sc + ox;
        let ly = label.position.y * sc + oy;
        write!(svg, "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" \
            fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>\n",
            lx - 25.0 * sc, ly - 8.0 * sc, 50.0 * sc, 16.0 * sc,
            opts.theme.label_bg, opts.theme.label_color).unwrap();
        write!(svg, "  <text x=\"{lx}\" y=\"{ly}\" class=\"lbl\">{}</text>\n", label.name).unwrap();
    }
}

fn render_junctions(svg: &mut String, sch: &Schematic, ox: f64, oy: f64, opts: &SvgOptions) {
    let sc = opts.scale;
    for j in &sch.junctions {
        let jx = j.position.x * sc + ox;
        let jy = j.position.y * sc + oy;
        write!(svg, "  <circle cx=\"{jx}\" cy=\"{jy}\" r=\"{}\" class=\"junc\"/>\n", 4.0 * sc).unwrap();
    }
}

fn render_legend(svg: &mut String, _w: f64, h: f64, opts: &SvgOptions) {
    let t = &opts.theme;
    let lx = 15.0;
    let ly = h - 50.0;
    write!(svg, "  <rect x=\"{lx}\" y=\"{ly}\" width=\"12\" height=\"12\" \
        fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>\n", t.component_fill, t.component_stroke).unwrap();
    write!(svg, "  <text x=\"{}\" y=\"{}\" class=\"sub\" text-anchor=\"start\">Component</text>\n",
        lx + 16.0, ly + 10.0).unwrap();
    write!(svg, "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"wire\"/>\n",
        lx + 80.0, ly + 6.0, lx + 92.0, ly + 6.0).unwrap();
    write!(svg, "  <text x=\"{}\" y=\"{}\" class=\"sub\" text-anchor=\"start\">Wire</text>\n",
        lx + 96.0, ly + 10.0).unwrap();
    write!(svg, "  <circle cx=\"{}\" cy=\"{}\" r=\"3\" class=\"pin\"/>\n",
        lx + 140.0, ly + 6.0).unwrap();
    write!(svg, "  <text x=\"{}\" y=\"{}\" class=\"sub\" text-anchor=\"start\">Pin</text>\n",
        lx + 147.0, ly + 10.0).unwrap();
}
