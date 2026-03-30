use std::collections::HashMap;
use serde::Serialize;
use crate::model::{Schematic, Point, builtin_symbols};
use crate::parser::{ParseResult, pin_names_for_symbol};
use crate::placer::SchematicPlacer;

#[derive(Debug, Serialize)]
pub struct ConnectivityReport {
    pub expected_net_count: usize,
    pub found_net_count: usize,
    pub all_nets_connected: bool,
    pub missing_connections: Vec<MissingConnection>,
    pub orphan_labels: Vec<String>,
    pub duplicate_label_positions: usize,
}

#[derive(Debug, Serialize)]
pub struct MissingConnection {
    pub net_name: String,
    pub expected_pins: usize,
    pub found_pins: usize,
}

pub fn check(parse_result: &ParseResult, schematic: &Schematic) -> ConnectivityReport {
    let symbols = builtin_symbols::all();

    // Build expected net map from netlist
    let devices = if !parse_result.subcircuits.is_empty() {
        &parse_result.subcircuits[0].devices
    } else {
        &parse_result.devices
    };

    let mut expected_nets: HashMap<String, usize> = HashMap::new();
    for device in devices {
        let sym_name = SchematicPlacer::symbol_for_device(device);
        let pin_names = pin_names_for_symbol(&sym_name);
        for (i, node) in device.nodes.iter().enumerate() {
            if i >= pin_names.len() { break; }
            *expected_nets.entry(node.clone()).or_insert(0) += 1;
        }
    }

    // Build actual net map from schematic
    let mut actual_nets: HashMap<String, usize> = HashMap::new();

    // Count connections from wires by tracing pin positions
    for comp in &schematic.components {
        if let Some(sym) = symbols.get(&comp.symbol_name) {
            for pin in &sym.pins {
                let world_pos = comp.position + pin.offset.transform(comp.rotation, comp.mirrored);
                // Check if this pin connects via wire
                for wire in &schematic.wires {
                    if let Some(first) = wire.points.first() {
                        if close_enough(first, &world_pos) {
                            // Wire connects at this pin — but we track via labels/power
                            break;
                        }
                    }
                    if let Some(last) = wire.points.last() {
                        if close_enough(last, &world_pos) {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Count from labels
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for label in &schematic.labels {
        *label_counts.entry(label.name.clone()).or_insert(0) += 1;
    }
    for (name, count) in &label_counts {
        *actual_nets.entry(name.clone()).or_insert(0) += count;
    }

    // Count from power symbols
    let mut power_counts: HashMap<String, usize> = HashMap::new();
    for ps in &schematic.power_symbols {
        *power_counts.entry(ps.net_name.clone()).or_insert(0) += 1;
    }
    for (name, count) in &power_counts {
        *actual_nets.entry(name.clone()).or_insert(0) += count;
    }

    // Count wire endpoints as connections
    *actual_nets.entry("__wires__".into()).or_insert(0) += schematic.wires.len();

    // Identify orphan labels (appear only once)
    let orphan_labels: Vec<String> = label_counts.iter()
        .filter(|(_, &count)| count == 1)
        .map(|(name, _)| name.clone())
        .collect();

    // Count duplicate label positions (same name, same position)
    let mut dup_count = 0;
    for label_name in label_counts.keys() {
        let positions: Vec<&Point> = schematic.labels.iter()
            .filter(|l| l.name == *label_name)
            .map(|l| &l.position)
            .collect();
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                if close_enough(positions[i], positions[j]) {
                    dup_count += 1;
                }
            }
        }
    }

    // Check missing connections
    let mut missing = Vec::new();
    for (net_name, &expected_count) in &expected_nets {
        let actual = label_counts.get(net_name).copied().unwrap_or(0)
            + power_counts.get(net_name).copied().unwrap_or(0);
        // A net with N pins should have some representation
        if expected_count >= 2 && actual == 0 {
            // Check if connected by direct wires (can't fully verify without pin mapping)
            // but at least flag completely unrepresented nets
            let has_wire_connection = schematic.components.iter().any(|c| {
                if let Some(sym) = symbols.get(&c.symbol_name) {
                    sym.pins.iter().enumerate().any(|(_, pin)| {
                        let wp = c.position + pin.offset.transform(c.rotation, c.mirrored);
                        schematic.wires.iter().any(|w| {
                            w.points.first().map_or(false, |p| close_enough(p, &wp))
                                || w.points.last().map_or(false, |p| close_enough(p, &wp))
                        })
                    })
                } else {
                    false
                }
            });
            if !has_wire_connection {
                missing.push(MissingConnection {
                    net_name: net_name.clone(),
                    expected_pins: expected_count,
                    found_pins: 0,
                });
            }
        }
    }

    let found_net_count = label_counts.len() + power_counts.len();

    ConnectivityReport {
        expected_net_count: expected_nets.len(),
        found_net_count,
        all_nets_connected: missing.is_empty() && orphan_labels.is_empty(),
        missing_connections: missing,
        orphan_labels,
        duplicate_label_positions: dup_count,
    }
}

fn close_enough(a: &Point, b: &Point) -> bool {
    (a.x - b.x).abs() < 2.0 && (a.y - b.y).abs() < 2.0
}
