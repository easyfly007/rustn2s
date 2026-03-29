use std::collections::{HashMap, HashSet};
use crate::parser::{SpiceDevice, pin_names_for_symbol};
use crate::placer::{PlacementResult, SchematicPlacer};
use crate::model::{
    Schematic, Component, Wire, Label, PowerSymbol, Junction, PowerType, Point,
    builtin_symbols,
};

pub struct RouterOptions {
    pub long_net_threshold: f64,
    pub grid_size: f64,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self {
            long_net_threshold: 300.0,
            grid_size: 10.0,
        }
    }
}

pub struct SchematicRouter;

impl SchematicRouter {
    /// Build the final Schematic from placement result + routing.
    pub fn route(
        &self,
        placement: PlacementResult,
        devices: &[SpiceDevice],
        power_nets: &HashSet<String>,
        opts: &RouterOptions,
    ) -> Schematic {
        let symbols = builtin_symbols::all();
        let mut schematic = Schematic::new("");

        // Build components and collect net→pin positions
        let mut net_connections: HashMap<String, Vec<Point>> = HashMap::new();

        for dp in &placement.placements {
            let device = &devices[dp.device_index];
            let sym_name = if dp.symbol_name.is_empty() {
                SchematicPlacer::symbol_for_device(device)
            } else {
                dp.symbol_name.clone()
            };

            let mut props: Vec<(String, String)> = device.parameters.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if !device.model_or_value.is_empty() {
                props.push(("model".into(), device.model_or_value.clone()));
            }

            schematic.components.push(Component {
                instance_name: device.instance_name.clone(),
                symbol_name: sym_name.clone(),
                position: dp.position,
                rotation: dp.rotation,
                mirrored: dp.mirrored,
                properties: props,
            });

            // Map SPICE nodes to pin world positions
            let pin_names = pin_names_for_symbol(&sym_name);
            let sym_def = symbols.get(&sym_name);

            for (i, node) in device.nodes.iter().enumerate() {
                if i >= pin_names.len() { break; }
                let pin_pos = if let Some(sym) = sym_def {
                    if let Some(sp) = sym.pins.iter().find(|p| p.name == pin_names[i]) {
                        let offset = sp.offset.transform(dp.rotation, dp.mirrored);
                        dp.position + offset
                    } else {
                        dp.position
                    }
                } else {
                    dp.position
                };
                net_connections.entry(node.clone()).or_default().push(pin_pos);
            }
        }

        // Route each net
        for (net_name, pins) in &net_connections {
            if pins.len() < 2 { continue; }

            if power_nets.contains(&net_name.to_lowercase()) || power_nets.contains(net_name) {
                self.route_power_net(&mut schematic, net_name, pins, opts);
            } else {
                self.route_signal_net(&mut schematic, net_name, pins, opts);
            }
        }

        schematic
    }

    fn route_power_net(
        &self, schematic: &mut Schematic, net_name: &str, pins: &[Point], opts: &RouterOptions,
    ) {
        let ptype = power_type_from_name(net_name);
        for &pin_pos in pins {
            let mut sym_pos = pin_pos.snap_to_grid(opts.grid_size);
            match ptype {
                PowerType::GND => sym_pos.y += 10.0,
                _ => sym_pos.y -= 10.0,
            }
            schematic.power_symbols.push(PowerSymbol {
                power_type: ptype,
                net_name: net_name.into(),
                position: sym_pos,
            });
        }
    }

    fn route_signal_net(
        &self, schematic: &mut Schematic, net_name: &str, pins: &[Point], opts: &RouterOptions,
    ) {
        if pins.len() < 2 { return; }

        let anchor = pins[0];
        for &target in &pins[1..] {
            let dist = anchor.distance_to(&target);

            if dist >= opts.long_net_threshold {
                // Long net: use labels
                schematic.labels.push(Label {
                    name: net_name.into(),
                    position: anchor.snap_to_grid(opts.grid_size),
                });
                schematic.labels.push(Label {
                    name: net_name.into(),
                    position: target.snap_to_grid(opts.grid_size),
                });
            } else {
                // Short net: L-route wire
                let pts = l_route(anchor, target);
                let clean: Vec<Point> = snap_and_dedup(&pts, opts.grid_size);
                if clean.len() >= 2 {
                    schematic.wires.push(Wire { points: clean });
                }
            }
        }

        // Junction at anchor if >2 pins
        if pins.len() > 2 {
            schematic.junctions.push(Junction {
                position: anchor.snap_to_grid(opts.grid_size),
            });
        }
    }
}

fn power_type_from_name(name: &str) -> PowerType {
    let lower = name.to_lowercase();
    if matches!(lower.as_str(), "0" | "gnd" | "gnd!" | "vss" | "vss!" | "avss") {
        PowerType::GND
    } else if matches!(lower.as_str(), "vdd" | "vdd!" | "vcc" | "vcc!" | "avdd") {
        PowerType::VDD
    } else {
        PowerType::Custom
    }
}

fn l_route(from: Point, to: Point) -> Vec<Point> {
    if (from.x - to.x).abs() < 0.001 || (from.y - to.y).abs() < 0.001 {
        vec![from, to]
    } else {
        // Horizontal first, then vertical
        vec![from, Point::new(to.x, from.y), to]
    }
}

fn snap_and_dedup(pts: &[Point], grid: f64) -> Vec<Point> {
    let mut clean: Vec<Point> = Vec::new();
    for pt in pts {
        let snapped = pt.snap_to_grid(grid);
        if let Some(last) = clean.last() {
            if (snapped.x - last.x).abs() < 0.001 && (snapped.y - last.y).abs() < 0.001 {
                continue;
            }
        }
        clean.push(snapped);
    }
    clean
}
