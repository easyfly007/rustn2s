use std::collections::{HashMap, HashSet};
use crate::parser::SpiceDevice;
use crate::placer::{PlacementResult, SchematicPlacer};
use crate::model::{
    Schematic, Component, Wire, Label, PowerSymbol, Junction, PowerType, Point,
    SymbolDef, builtin_symbols,
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
        self.route_with_subcircuits(placement, devices, power_nets, opts, &HashMap::new())
    }

    /// Build the final Schematic, with additional subcircuit symbols for X instances.
    pub fn route_with_subcircuits(
        &self,
        placement: PlacementResult,
        devices: &[SpiceDevice],
        power_nets: &HashSet<String>,
        opts: &RouterOptions,
        subckt_symbols: &HashMap<String, SymbolDef>,
    ) -> Schematic {
        let builtin = builtin_symbols::all();
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
            // For X devices, use the subcircuit symbol's pin definitions
            let sym_def = subckt_symbols.get(&sym_name)
                .or_else(|| builtin.get(&sym_name));

            // For subcircuit instances, nodes map directly to ports by position
            if let Some(sym) = sym_def {
                for (i, node) in device.nodes.iter().enumerate() {
                    if i >= sym.pins.len() { break; }
                    let pin = &sym.pins[i];
                    let offset = pin.offset.transform(dp.rotation, dp.mirrored);
                    let pin_pos = dp.position + offset;
                    net_connections.entry(node.clone()).or_default().push(pin_pos);
                }
            } else {
                // Fallback: place all nodes at component center
                for node in &device.nodes {
                    net_connections.entry(node.clone()).or_default().push(dp.position);
                }
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

        // Build MST edges for this net instead of star topology
        let edges = minimum_spanning_tree(pins);

        // Track which pins need labels (long-distance connections)
        let mut label_pins: HashSet<usize> = HashSet::new();

        for &(i, j) in &edges {
            let from = pins[i];
            let to = pins[j];
            let dist = from.distance_to(&to);

            if dist >= opts.long_net_threshold {
                // Long edge: mark both endpoints for labeling
                label_pins.insert(i);
                label_pins.insert(j);
            } else {
                // Short edge: L-route wire, trying both orientations
                let wire_pts = l_route_best(from, to, &schematic.wires);
                let clean: Vec<Point> = snap_and_dedup(&wire_pts, opts.grid_size);
                if clean.len() >= 2 {
                    schematic.wires.push(Wire { points: clean });
                }
            }
        }

        // Emit one label per pin that needs labeling (deduplicated)
        let mut labeled_positions: Vec<Point> = Vec::new();
        for &pi in &label_pins {
            let pos = pins[pi].snap_to_grid(opts.grid_size);
            // Skip if we already have a label at this position
            if labeled_positions.iter().any(|p| close(p, &pos)) {
                continue;
            }
            labeled_positions.push(pos);
            schematic.labels.push(Label {
                name: net_name.into(),
                position: pos,
            });
        }

        // Junction at any pin connected by more than one MST edge
        let mut edge_count = vec![0usize; pins.len()];
        for &(i, j) in &edges {
            edge_count[i] += 1;
            edge_count[j] += 1;
        }
        for (pi, &count) in edge_count.iter().enumerate() {
            if count > 1 {
                let pos = pins[pi].snap_to_grid(opts.grid_size);
                schematic.junctions.push(Junction { position: pos });
            }
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

fn close(a: &Point, b: &Point) -> bool {
    (a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() < 1.0
}

/// Try both L-route orientations and pick the one with fewer crossings
/// against existing wires.
fn l_route_best(from: Point, to: Point, existing_wires: &[Wire]) -> Vec<Point> {
    if (from.x - to.x).abs() < 0.001 || (from.y - to.y).abs() < 0.001 {
        // Already aligned: straight line, no choice needed
        return vec![from, to];
    }

    // Option A: horizontal first, then vertical
    let route_a = vec![from, Point::new(to.x, from.y), to];
    // Option B: vertical first, then horizontal
    let route_b = vec![from, Point::new(from.x, to.y), to];

    let crossings_a = count_crossings_with(&route_a, existing_wires);
    let crossings_b = count_crossings_with(&route_b, existing_wires);

    if crossings_b < crossings_a {
        route_b
    } else {
        route_a // Default to horizontal-first on tie
    }
}

/// Count how many times a candidate route crosses existing wire segments.
fn count_crossings_with(route: &[Point], existing_wires: &[Wire]) -> usize {
    let mut count = 0;
    for k in 0..route.len().saturating_sub(1) {
        let p1 = &route[k];
        let p2 = &route[k + 1];
        for wire in existing_wires {
            for s in 0..wire.points.len().saturating_sub(1) {
                let p3 = &wire.points[s];
                let p4 = &wire.points[s + 1];
                if segments_cross(p1, p2, p3, p4) {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Test if two line segments have a strict interior crossing.
fn segments_cross(p1: &Point, p2: &Point, p3: &Point, p4: &Point) -> bool {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-10 {
        return false; // Parallel or collinear
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;
    let u = ((p3.x - p1.x) * d1y - (p3.y - p1.y) * d1x) / denom;

    let eps = 0.001;
    t > eps && t < 1.0 - eps && u > eps && u < 1.0 - eps
}

/// Compute the minimum spanning tree of a set of points using Prim's algorithm.
/// Returns edges as pairs of point indices.
fn minimum_spanning_tree(pins: &[Point]) -> Vec<(usize, usize)> {
    let n = pins.len();
    if n <= 1 {
        return Vec::new();
    }
    if n == 2 {
        return vec![(0, 1)];
    }

    let mut in_tree = vec![false; n];
    let mut min_cost = vec![f64::MAX; n];
    let mut min_edge = vec![0usize; n]; // which tree node gives the min cost

    let mut edges = Vec::with_capacity(n - 1);

    // Start from node 0
    in_tree[0] = true;
    for j in 1..n {
        min_cost[j] = pins[0].distance_to(&pins[j]);
        min_edge[j] = 0;
    }

    for _ in 0..n - 1 {
        // Find the closest non-tree node
        let mut best = usize::MAX;
        let mut best_cost = f64::MAX;
        for j in 0..n {
            if !in_tree[j] && min_cost[j] < best_cost {
                best_cost = min_cost[j];
                best = j;
            }
        }
        if best == usize::MAX { break; }

        in_tree[best] = true;
        edges.push((min_edge[best], best));

        // Update costs
        for j in 0..n {
            if !in_tree[j] {
                let d = pins[best].distance_to(&pins[j]);
                if d < min_cost[j] {
                    min_cost[j] = d;
                    min_edge[j] = best;
                }
            }
        }
    }

    edges
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
