use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::model::{
    Point, Schematic, Component, PowerSymbol, PowerType,
    SymbolDef, SymbolPin, SymbolGraphic, PinDirection, builtin_symbols,
};

const SCALE: f64 = 0.254;
const A4_WIDTH: f64 = 297.0;
const A4_HEIGHT: f64 = 210.0;

struct UuidGen {
    counter: u64,
}

impl UuidGen {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn next(&mut self) -> String {
        self.counter += 1;
        format!("00000000-0000-0000-0000-{:012x}", self.counter)
    }
}

fn sc(v: f64) -> f64 {
    v * SCALE
}

fn fmt2(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else {
        format!("{:.4}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn n2s_rotation_to_kicad(rotation: i32) -> i32 {
    ((360 - rotation) % 360 + 360) % 360
}

fn pin_dir_to_kicad_angle(dir: &PinDirection) -> i32 {
    match dir {
        PinDirection::Right => 0,
        PinDirection::Up => 90,
        PinDirection::Left => 180,
        PinDirection::Down => 270,
    }
}

fn ref_prefix(symbol_name: &str) -> &'static str {
    match symbol_name {
        "nmos4" | "pmos4" => "M",
        "resistor" => "R",
        "capacitor" => "C",
        "inductor" => "L",
        "diode" => "D",
        "npn" | "pnp" => "Q",
        "vsource" => "V",
        "isource" => "I",
        "vcvs" => "E",
        "vccs" => "G",
        "ccvs" => "H",
        "cccs" => "F",
        s if s.starts_with("subckt_") => "X",
        _ => "U",
    }
}

fn compute_offset(schematic: &Schematic, symbols: &HashMap<String, SymbolDef>) -> Point {
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

    for c in &schematic.components {
        expand(c.position.x, c.position.y);
        if let Some(sym) = symbols.get(&c.symbol_name) {
            for pin in &sym.pins {
                let wp = pin.offset.transform(c.rotation, c.mirrored);
                expand(c.position.x + wp.x, c.position.y + wp.y);
            }
        }
    }
    for w in &schematic.wires {
        for p in &w.points {
            expand(p.x, p.y);
        }
    }
    for l in &schematic.labels {
        expand(l.position.x, l.position.y);
    }
    for ps in &schematic.power_symbols {
        expand(ps.position.x, ps.position.y);
    }
    for j in &schematic.junctions {
        expand(j.position.x, j.position.y);
    }

    if min_x > max_x {
        return Point::new(A4_WIDTH / 2.0, A4_HEIGHT / 2.0);
    }

    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    Point::new(
        A4_WIDTH / 2.0 - sc(cx),
        A4_HEIGHT / 2.0 - sc(cy),
    )
}

fn xy(p: Point, off: Point) -> (String, String) {
    (fmt2(sc(p.x) + off.x), fmt2(sc(p.y) + off.y))
}

// ---------------------------------------------------------------------------
// lib_symbols: component symbols
// ---------------------------------------------------------------------------

fn emit_graphic(g: &SymbolGraphic, out: &mut String) {
    match g {
        SymbolGraphic::Line { x1, y1, x2, y2 } => {
            let _ = writeln!(out,
                "      (polyline (pts (xy {} {}) (xy {} {})) (stroke (width 0) (type default)) (fill (type none)))",
                fmt2(sc(*x1)), fmt2(sc(*y1)), fmt2(sc(*x2)), fmt2(sc(*y2)));
        }
        SymbolGraphic::Rect { x, y, width, height, filled } => {
            let fill = if *filled { "background" } else { "none" };
            let _ = writeln!(out,
                "      (rectangle (start {} {}) (end {} {}) (stroke (width 0) (type default)) (fill (type {})))",
                fmt2(sc(*x)), fmt2(sc(*y)),
                fmt2(sc(x + width)), fmt2(sc(y + height)),
                fill);
        }
        SymbolGraphic::Circle { cx, cy, radius, filled } => {
            let fill = if *filled { "background" } else { "none" };
            let _ = writeln!(out,
                "      (circle (center {} {}) (radius {}) (stroke (width 0) (type default)) (fill (type {})))",
                fmt2(sc(*cx)), fmt2(sc(*cy)), fmt2(sc(*radius)), fill);
        }
        SymbolGraphic::Arc { cx, cy, radius, start_angle, span_angle } => {
            let r = *radius;
            let sa = start_angle.to_radians();
            let ea = (start_angle + span_angle).to_radians();
            let ma = (start_angle + span_angle / 2.0).to_radians();
            let sx = cx + r * sa.cos();
            let sy = cy - r * sa.sin();
            let ex = cx + r * ea.cos();
            let ey = cy - r * ea.sin();
            let mx = cx + r * ma.cos();
            let my = cy - r * ma.sin();
            let _ = writeln!(out,
                "      (arc (start {} {}) (mid {} {}) (end {} {}) (stroke (width 0) (type default)) (fill (type none)))",
                fmt2(sc(sx)), fmt2(sc(sy)),
                fmt2(sc(mx)), fmt2(sc(my)),
                fmt2(sc(ex)), fmt2(sc(ey)));
        }
        SymbolGraphic::Polyline { points, filled } => {
            let fill = if *filled { "background" } else { "none" };
            let mut pts = String::new();
            for p in points {
                let _ = write!(pts, "(xy {} {}) ", fmt2(sc(p.x)), fmt2(sc(p.y)));
            }
            let _ = writeln!(out,
                "      (polyline (pts {}) (stroke (width 0) (type default)) (fill (type {})))",
                pts.trim(), fill);
        }
        SymbolGraphic::Text { x, y, text, font_size } => {
            let sz = sc(*font_size).max(1.0);
            let _ = writeln!(out,
                "      (text \"{}\" (at {} {}) (effects (font (size {} {}))))",
                text, fmt2(sc(*x)), fmt2(sc(*y)), fmt2(sz), fmt2(sz));
        }
    }
}

fn emit_pin(pin: &SymbolPin, out: &mut String) {
    let angle = pin_dir_to_kicad_angle(&pin.direction);
    let _ = writeln!(out,
        "      (pin passive line (at {} {} {}) (length 0) (name \"{}\" (effects (font (size 1.27 1.27)))) (number \"{}\" (effects (font (size 1.27 1.27)))))",
        fmt2(sc(pin.offset.x)), fmt2(sc(pin.offset.y)), angle,
        pin.name, pin.pin_number);
}

fn emit_lib_symbol(sym: &SymbolDef, out: &mut String) {
    let lib_id = format!("n2s:{}", sym.name);
    let prefix = ref_prefix(&sym.name);
    let _ = writeln!(out, "    (symbol \"{}\"", lib_id);
    let _ = writeln!(out, "      (pin_names (offset 0))");
    let _ = writeln!(out, "      (in_bom yes)");
    let _ = writeln!(out, "      (on_board yes)");
    let _ = writeln!(out, "      (property \"Reference\" \"{}\" (at 0 {} 0) (effects (font (size 1.27 1.27))))",
        prefix, fmt2(sc(-25.0)));
    let _ = writeln!(out, "      (property \"Value\" \"{}\" (at 0 {} 0) (effects (font (size 1.27 1.27))))",
        sym.name, fmt2(sc(25.0)));
    let _ = writeln!(out, "      (property \"Footprint\" \"\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "      (property \"Datasheet\" \"~\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");

    // Graphics sub-symbol
    let _ = writeln!(out, "      (symbol \"{}_0_1\"", lib_id);
    for g in &sym.graphics {
        emit_graphic(g, out);
    }
    let _ = writeln!(out, "      )");

    // Pins sub-symbol
    let _ = writeln!(out, "      (symbol \"{}_1_1\"", lib_id);
    for pin in &sym.pins {
        emit_pin(pin, out);
    }
    let _ = writeln!(out, "      )");

    let _ = writeln!(out, "    )");
}

// ---------------------------------------------------------------------------
// lib_symbols: power symbols
// ---------------------------------------------------------------------------

fn emit_power_lib_symbol(net_name: &str, power_type: PowerType, out: &mut String) {
    let lib_id = format!("power:{}", net_name);
    let _ = writeln!(out, "    (symbol \"{}\"", lib_id);
    let _ = writeln!(out, "      (power)");
    let _ = writeln!(out, "      (pin_names (offset 0))");
    let _ = writeln!(out, "      (in_bom yes)");
    let _ = writeln!(out, "      (on_board yes)");
    let _ = writeln!(out, "      (property \"Reference\" \"#PWR\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "      (property \"Value\" \"{}\" (at 0 {} 0) (effects (font (size 1.27 1.27))))",
        net_name,
        match power_type { PowerType::GND => "3.81", _ => "-3.81" });
    let _ = writeln!(out, "      (property \"Footprint\" \"\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "      (property \"Datasheet\" \"~\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");

    let _ = writeln!(out, "      (symbol \"{}_0_1\"", lib_id);
    match power_type {
        PowerType::GND => {
            let _ = writeln!(out, "      (polyline (pts (xy 0 0) (xy 0 1.27)) (stroke (width 0) (type default)) (fill (type none)))");
            let _ = writeln!(out, "      (polyline (pts (xy -1.27 1.27) (xy 1.27 1.27)) (stroke (width 0.254) (type default)) (fill (type none)))");
            let _ = writeln!(out, "      (polyline (pts (xy -0.762 1.778) (xy 0.762 1.778)) (stroke (width 0.254) (type default)) (fill (type none)))");
            let _ = writeln!(out, "      (polyline (pts (xy -0.254 2.286) (xy 0.254 2.286)) (stroke (width 0.254) (type default)) (fill (type none)))");
        }
        PowerType::VDD | PowerType::Custom => {
            let _ = writeln!(out, "      (polyline (pts (xy 0 0) (xy 0 -1.27)) (stroke (width 0) (type default)) (fill (type none)))");
            let _ = writeln!(out, "      (polyline (pts (xy -1.27 -1.27) (xy 1.27 -1.27)) (stroke (width 0.254) (type default)) (fill (type none)))");
        }
    }
    let _ = writeln!(out, "      )");

    let _ = writeln!(out, "      (symbol \"{}_1_1\"", lib_id);
    match power_type {
        PowerType::GND => {
            let _ = writeln!(out, "      (pin power_in line (at 0 0 90) (length 0) (name \"{}\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))",
                net_name);
        }
        PowerType::VDD | PowerType::Custom => {
            let _ = writeln!(out, "      (pin power_in line (at 0 0 270) (length 0) (name \"{}\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))",
                net_name);
        }
    }
    let _ = writeln!(out, "      )");

    let _ = writeln!(out, "    )");
}

// ---------------------------------------------------------------------------
// Schematic body
// ---------------------------------------------------------------------------

fn emit_symbol_instance(
    comp: &Component,
    off: Point,
    uuid: &mut UuidGen,
    out: &mut String,
) {
    let lib_id = format!("n2s:{}", comp.symbol_name);
    let angle = n2s_rotation_to_kicad(comp.rotation);
    let (cx, cy) = xy(comp.position, off);

    let _ = writeln!(out, "  (symbol");
    let _ = writeln!(out, "    (lib_id \"{}\")", lib_id);
    let _ = write!(out, "    (at {} {} {})", cx, cy, angle);
    if comp.mirrored {
        let _ = write!(out, " (mirror y)");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "    (unit 1)");
    let _ = writeln!(out, "    (uuid \"{}\")", uuid.next());

    // Reference property
    let ref_y = sc(comp.position.y) + off.y - 5.0;
    let _ = writeln!(out,
        "    (property \"Reference\" \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27))))",
        comp.instance_name, cx, fmt2(ref_y));

    // Value property
    let value = extract_value(comp);
    let val_y = sc(comp.position.y) + off.y + 5.0;
    let _ = writeln!(out,
        "    (property \"Value\" \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27))))",
        value, cx, fmt2(val_y));

    let _ = writeln!(out, "    (property \"Footprint\" \"\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "    (property \"Datasheet\" \"~\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "  )");
}

fn extract_value(comp: &Component) -> String {
    if let Some((_, v)) = comp.properties.iter().find(|(k, _)| k == "model") {
        return v.clone();
    }
    if comp.properties.is_empty() {
        return comp.symbol_name.clone();
    }
    comp.properties
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ")
}

fn emit_power_instance(
    ps: &PowerSymbol,
    off: Point,
    pwr_idx: &mut usize,
    uuid: &mut UuidGen,
    out: &mut String,
) {
    let lib_id = format!("power:{}", ps.net_name);
    let (px, py) = xy(ps.position, off);

    *pwr_idx += 1;
    let _ = writeln!(out, "  (symbol");
    let _ = writeln!(out, "    (lib_id \"{}\")", lib_id);
    let _ = writeln!(out, "    (at {} {} 0)", px, py);
    let _ = writeln!(out, "    (unit 1)");
    let _ = writeln!(out, "    (uuid \"{}\")", uuid.next());
    let _ = writeln!(out,
        "    (property \"Reference\" \"#PWR{:02}\" (at {} {} 0) (effects (font (size 1.27 1.27)) hide))",
        pwr_idx, px, py);

    let vy = match ps.power_type {
        PowerType::GND => sc(ps.position.y) + off.y + 3.81,
        _ => sc(ps.position.y) + off.y - 3.81,
    };
    let _ = writeln!(out,
        "    (property \"Value\" \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27))))",
        ps.net_name, px, fmt2(vy));
    let _ = writeln!(out, "    (property \"Footprint\" \"\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "    (property \"Datasheet\" \"~\" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))");
    let _ = writeln!(out, "  )");
}

fn emit_wires(schematic: &Schematic, off: Point, uuid: &mut UuidGen, out: &mut String) {
    for wire in &schematic.wires {
        for seg in wire.points.windows(2) {
            let (x1, y1) = xy(seg[0], off);
            let (x2, y2) = xy(seg[1], off);
            let _ = writeln!(out,
                "  (wire (pts (xy {} {}) (xy {} {})) (stroke (width 0) (type default)) (uuid \"{}\"))",
                x1, y1, x2, y2, uuid.next());
        }
    }
}

fn emit_labels(schematic: &Schematic, off: Point, uuid: &mut UuidGen, out: &mut String) {
    for label in &schematic.labels {
        let (lx, ly) = xy(label.position, off);
        let _ = writeln!(out,
            "  (label \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27))) (uuid \"{}\"))",
            label.name, lx, ly, uuid.next());
    }
}

fn emit_junctions(schematic: &Schematic, off: Point, uuid: &mut UuidGen, out: &mut String) {
    for junc in &schematic.junctions {
        let (jx, jy) = xy(junc.position, off);
        let _ = writeln!(out,
            "  (junction (at {} {}) (diameter 0) (color 0 0 0 0) (uuid \"{}\"))",
            jx, jy, uuid.next());
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn render_to_kicad_sch(
    schematic: &Schematic,
    extra_symbols: &HashMap<String, SymbolDef>,
) -> String {
    let all_builtin = builtin_symbols::all();
    let mut symbols: HashMap<String, &SymbolDef> = HashMap::new();
    for (k, v) in &all_builtin {
        symbols.insert(k.clone(), v);
    }
    for (k, v) in extra_symbols {
        symbols.insert(k.clone(), v);
    }

    let sym_map: HashMap<String, SymbolDef> = symbols
        .iter()
        .map(|(k, v)| (k.clone(), (*v).clone()))
        .collect();
    let off = compute_offset(schematic, &sym_map);

    let mut uuid = UuidGen::new();
    let mut out = String::with_capacity(8192);

    // Header
    let _ = writeln!(out, "(kicad_sch");
    let _ = writeln!(out, "  (version 20231120)");
    let _ = writeln!(out, "  (generator \"n2s\")");
    let _ = writeln!(out, "  (generator_version \"1.0\")");
    let _ = writeln!(out, "  (uuid \"{}\")", uuid.next());
    let _ = writeln!(out, "  (paper \"A4\")");
    let _ = writeln!(out);

    // lib_symbols
    let _ = writeln!(out, "  (lib_symbols");

    let used_symbols: HashSet<&str> = schematic
        .components
        .iter()
        .map(|c| c.symbol_name.as_str())
        .collect();
    for name in &used_symbols {
        if let Some(sym) = symbols.get(*name) {
            emit_lib_symbol(sym, &mut out);
        }
    }

    let mut power_nets: HashMap<String, PowerType> = HashMap::new();
    for ps in &schematic.power_symbols {
        power_nets.entry(ps.net_name.clone()).or_insert(ps.power_type);
    }
    for (net_name, ptype) in &power_nets {
        emit_power_lib_symbol(net_name, *ptype, &mut out);
    }

    let _ = writeln!(out, "  )");
    let _ = writeln!(out);

    // Symbol instances (components)
    for comp in &schematic.components {
        emit_symbol_instance(comp, off, &mut uuid, &mut out);
        let _ = writeln!(out);
    }

    // Power symbol instances
    let mut pwr_idx = 0usize;
    for ps in &schematic.power_symbols {
        emit_power_instance(ps, off, &mut pwr_idx, &mut uuid, &mut out);
        let _ = writeln!(out);
    }

    // Wires
    emit_wires(schematic, off, &mut uuid, &mut out);
    let _ = writeln!(out);

    // Labels
    emit_labels(schematic, off, &mut uuid, &mut out);
    let _ = writeln!(out);

    // Junctions
    emit_junctions(schematic, off, &mut uuid, &mut out);
    let _ = writeln!(out);

    // Footer
    let _ = writeln!(out, "  (sheet_instances");
    let _ = writeln!(out, "    (path \"/\" (page \"1\"))");
    let _ = writeln!(out, "  )");
    let _ = writeln!(out, ")");

    out
}

pub fn render_to_file(
    schematic: &Schematic,
    path: &str,
    extra_symbols: &HashMap<String, SymbolDef>,
) -> Result<(), String> {
    let content = render_to_kicad_sch(schematic, extra_symbols);
    std::fs::write(path, content).map_err(|e| format!("Cannot write {}: {}", path, e))
}
