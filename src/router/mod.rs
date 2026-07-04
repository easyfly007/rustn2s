use crate::model::{
    builtin_symbols, Component, Junction, Label, PinDirection, Point, PowerSymbol, PowerType,
    Schematic, SymbolDef, Wire,
};
use crate::parser::SpiceDevice;
use crate::placer::{PlacementResult, SchematicPlacer};
use std::collections::{HashMap, HashSet};

mod astar;

/// A pin's world-space position plus the outward offset to apply when placing
/// a label at that pin so the label does not overlap the component graphic.
#[derive(Clone, Copy)]
struct PinInfo {
    position: Point,
    /// Offset vector added to `position` to get a label center clear of the
    /// component body. `(0, 0)` means the pin has no known direction; labels
    /// fall back to the raw pin position (old behavior).
    label_offset: Point,
}

pub struct RouterOptions {
    /// Absolute floor on the wire-vs-label decision. Edges shorter than
    /// this always become wires; edges longer fall through to the
    /// adaptive check below.
    pub long_net_threshold: f64,
    pub grid_size: f64,
    /// When true, route short edges via grid A* with component obstacles.
    /// On A* failure (no path), falls back to the original L-route.
    pub avoid_obstacles: bool,
    /// Penalty added to A* g_cost each time the path direction changes.
    /// Higher values prefer straighter wires.
    pub bend_penalty: f64,
    /// Penalty added to A* g_cost when stepping perpendicular to an
    /// already-routed wire (i.e. the step would create a visible crossing).
    /// Higher values prefer detours over crossings.
    pub crossing_penalty: f64,
    /// Fraction of the placement's bounding-box diagonal used as a soft
    /// lower bound on the effective label threshold. The router uses
    /// `max(long_net_threshold, bbox_diagonal × adaptive_label_ratio)` so
    /// large schematics tolerate longer wires before switching to labels.
    /// Set to `0.0` to disable adaptive behavior (use the absolute
    /// `long_net_threshold` only).
    pub adaptive_label_ratio: f64,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self {
            long_net_threshold: 300.0,
            grid_size: 10.0,
            // Opt-in: A* avoids walking wires through component bodies, but
            // it can lengthen routes and create new crossings between
            // detoured wires, so eval-driven scores are not strictly better.
            // Enable via --obstacle-avoidance (or set this directly).
            avoid_obstacles: false,
            bend_penalty: 0.5,
            crossing_penalty: 20.0,
            adaptive_label_ratio: 0.3,
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

        // Build components and collect net→pin (position + outward label offset)
        let mut net_connections: HashMap<String, Vec<PinInfo>> = HashMap::new();

        for dp in &placement.placements {
            let device = &devices[dp.device_index];
            let sym_name = if dp.symbol_name.is_empty() {
                SchematicPlacer::symbol_for_device(device)
            } else {
                dp.symbol_name.clone()
            };

            let mut props: Vec<(String, String)> = device
                .parameters
                .iter()
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
            let sym_def = subckt_symbols
                .get(&sym_name)
                .or_else(|| builtin.get(&sym_name));

            // For subcircuit instances, nodes map directly to ports by position
            if let Some(sym) = sym_def {
                for (i, node) in device.nodes.iter().enumerate() {
                    if i >= sym.pins.len() {
                        break;
                    }
                    let pin = &sym.pins[i];
                    let offset = pin.offset.transform(dp.rotation, dp.mirrored);
                    let pin_pos = dp.position + offset;
                    let label_offset =
                        label_offset_for_pin(pin.direction, dp.rotation, dp.mirrored);
                    net_connections
                        .entry(node.clone())
                        .or_default()
                        .push(PinInfo {
                            position: pin_pos,
                            label_offset,
                        });
                }
            } else {
                // Fallback: place all nodes at component center (no direction)
                for node in &device.nodes {
                    net_connections
                        .entry(node.clone())
                        .or_default()
                        .push(PinInfo {
                            position: dp.position,
                            label_offset: Point::new(0.0, 0.0),
                        });
                }
            }
        }

        // Build the obstacle grid once for the whole schematic. All nets
        // share it so paths from earlier nets don't get re-checked against
        // obstacles each time. The grid is only built when obstacle
        // avoidance is enabled.
        let mut symbol_map: HashMap<String, SymbolDef> = builtin.clone();
        for (k, v) in subckt_symbols {
            symbol_map.insert(k.clone(), v.clone());
        }
        let mut obstacle_grid = if opts.avoid_obstacles {
            Some(astar::build_grid(
                &schematic.components,
                &symbol_map,
                placement.bounding_rect,
                opts.grid_size,
            ))
        } else {
            None
        };

        // Adaptive label threshold: scale up the wire-vs-label cutoff for
        // larger schematics so a fixed user threshold doesn't prematurely
        // force medium-distance nets onto labels. Acts as an additional
        // floor — never goes below the user-supplied absolute threshold.
        let (bb_min, bb_max) = placement.bounding_rect;
        let bbox_diag = ((bb_max.x - bb_min.x).powi(2) + (bb_max.y - bb_min.y).powi(2)).sqrt();
        let effective_threshold = opts
            .long_net_threshold
            .max(bbox_diag * opts.adaptive_label_ratio);

        // Route each net
        for (net_name, pins) in &net_connections {
            if pins.len() < 2 {
                continue;
            }

            if power_nets.contains(&net_name.to_lowercase()) || power_nets.contains(net_name) {
                self.route_power_net(&mut schematic, net_name, pins, opts);
            } else {
                self.route_signal_net(
                    &mut schematic,
                    net_name,
                    pins,
                    opts,
                    effective_threshold,
                    obstacle_grid.as_mut(),
                );
            }
        }

        schematic
    }

    fn route_power_net(
        &self,
        schematic: &mut Schematic,
        net_name: &str,
        pins: &[PinInfo],
        opts: &RouterOptions,
    ) {
        let ptype = power_type_from_name(net_name);
        for pin in pins {
            let mut sym_pos = pin.position.snap_to_grid(opts.grid_size);
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
        &self,
        schematic: &mut Schematic,
        net_name: &str,
        pins: &[PinInfo],
        opts: &RouterOptions,
        long_net_threshold: f64,
        grid: Option<&mut astar::ObstacleGrid>,
    ) {
        if pins.len() < 2 {
            return;
        }

        // MST operates on pin positions only
        let positions: Vec<Point> = pins.iter().map(|p| p.position).collect();
        let mut edges = minimum_spanning_tree(&positions);
        // Route shorter edges first: they have less flexibility (any detour
        // is a large fraction of the original distance), so getting them
        // down first leaves longer edges free to take roundabout paths.
        edges.sort_by(|&(a1, b1), &(a2, b2)| {
            let d1 = positions[a1].distance_to(&positions[b1]);
            let d2 = positions[a2].distance_to(&positions[b2]);
            d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Track which pins need labels (long-distance connections)
        let mut label_pins: HashSet<usize> = HashSet::new();

        let mut grid_ref = grid;

        for &(i, j) in &edges {
            let from = positions[i];
            let to = positions[j];
            let dist = from.distance_to(&to);

            if dist >= long_net_threshold {
                // Long edge: mark both endpoints for labeling
                label_pins.insert(i);
                label_pins.insert(j);
            } else {
                // Short edge: prefer A* through the obstacle grid; on no-path
                // fall back to L-route. When the grid isn't available
                // (avoid_obstacles=false), use L-route directly.
                // Try L-route first (it's optimal in clear space). Only fall
                // through to A* if the L-route would pass through a blocked
                // cell — i.e., walk through a component body. This keeps
                // simple circuits at L-route quality and only invokes the
                // detour-prone A* when we actually need to dodge something.
                let l_route = l_route_best(from, to, &schematic.wires);
                let wire_pts = match grid_ref.as_deref() {
                    Some(g) if !g.polyline_clear(&l_route) => {
                        astar::find_path(g, from, to, opts.bend_penalty, opts.crossing_penalty)
                            .unwrap_or(l_route)
                    }
                    _ => l_route,
                };
                let clean: Vec<Point> = snap_and_dedup(&wire_pts, opts.grid_size);
                if clean.len() >= 2 {
                    schematic.wires.push(Wire {
                        points: clean.clone(),
                    });
                    // Mark this wire's cells with their orientation so
                    // subsequent A* searches can avoid creating crossings.
                    if let Some(g) = grid_ref.as_deref_mut() {
                        g.mark_wire_orientation(&clean);
                    }
                }
            }
        }

        // Emit one label per pin that needs labeling (deduplicated).
        // The label sits at pin_pos + label_offset so it clears the component
        // graphic, and a short stub wire connects the pin to the label anchor.
        let mut labeled_positions: Vec<Point> = Vec::new();
        for &pi in &label_pins {
            let pin = &pins[pi];
            let label_pos = (pin.position + pin.label_offset).snap_to_grid(opts.grid_size);

            // Skip duplicate labels at the same anchor point
            if labeled_positions.iter().any(|p| close(p, &label_pos)) {
                continue;
            }
            labeled_positions.push(label_pos);
            schematic.labels.push(Label {
                name: net_name.into(),
                position: label_pos,
            });

            // Draw a stub wire from the pin to the label anchor so the
            // schematic stays electrically readable. Skip if the offset is
            // zero (fallback path — label coincides with pin).
            let pin_snapped = pin.position.snap_to_grid(opts.grid_size);
            if !close(&pin_snapped, &label_pos) {
                schematic.wires.push(Wire {
                    points: vec![pin_snapped, label_pos],
                });
            }
        }

        // Junction at any pin connected by more than one MST edge
        let mut edge_count = vec![0usize; pins.len()];
        for &(i, j) in &edges {
            edge_count[i] += 1;
            edge_count[j] += 1;
        }
        for (pi, &count) in edge_count.iter().enumerate() {
            if count > 1 {
                let pos = positions[pi].snap_to_grid(opts.grid_size);
                schematic.junctions.push(Junction { position: pos });
            }
        }
    }
}

/// Compute the outward offset vector to shift a label away from its pin so
/// the label rectangle clears the component body. Horizontal pins get a
/// larger offset because the label is wider than it is tall.
fn label_offset_for_pin(dir: PinDirection, rotation: i32, mirrored: bool) -> Point {
    // Magnitude: label rect is 50 wide × 16 tall, so 30 along the horizontal
    // axis and 15 along the vertical axis clear the pin stub comfortably.
    let raw = match dir {
        PinDirection::Left => Point::new(-30.0, 0.0),
        PinDirection::Right => Point::new(30.0, 0.0),
        PinDirection::Up => Point::new(0.0, -15.0),
        PinDirection::Down => Point::new(0.0, 15.0),
    };
    raw.transform(rotation, mirrored)
}

fn power_type_from_name(name: &str) -> PowerType {
    let lower = name.to_lowercase();
    if matches!(
        lower.as_str(),
        "0" | "gnd" | "gnd!" | "vss" | "vss!" | "avss" | "vgnd" | "vnb"
    ) {
        PowerType::GND
    } else if matches!(
        lower.as_str(),
        "vdd" | "vdd!" | "vcc" | "vcc!" | "avdd" | "vpwr" | "vpb"
    ) {
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
        if best == usize::MAX {
            break;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    // ---- power_type_from_name ----

    #[test]
    fn power_type_classifies_gnd_aliases() {
        for name in ["0", "GND", "gnd", "vss", "VSS!", "AVSS"] {
            assert_eq!(power_type_from_name(name), PowerType::GND, "{}", name);
        }
    }

    #[test]
    fn power_type_classifies_vdd_aliases() {
        for name in ["VDD", "vdd", "VCC", "vcc!", "AVDD"] {
            assert_eq!(power_type_from_name(name), PowerType::VDD, "{}", name);
        }
    }

    #[test]
    fn power_type_other_is_custom() {
        for name in ["vout", "n1", "tail", "bias_p"] {
            assert_eq!(power_type_from_name(name), PowerType::Custom, "{}", name);
        }
    }

    // ---- label_offset_for_pin ----

    #[test]
    fn label_offset_left_pin_no_transform() {
        let off = label_offset_for_pin(PinDirection::Left, 0, false);
        assert!((off.x - (-30.0)).abs() < 1e-6 && off.y.abs() < 1e-6);
    }

    #[test]
    fn label_offset_rotation_90_swaps_axes() {
        // A "Right" pin (offset (30, 0)) rotated +90° → (0, 30) approximately.
        let off = label_offset_for_pin(PinDirection::Right, 90, false);
        assert!(off.x.abs() < 1e-6, "x should be ~0, got {}", off.x);
        assert!(
            (off.y - 30.0).abs() < 1e-6,
            "y should be ~30, got {}",
            off.y
        );
    }

    #[test]
    fn label_offset_mirror_flips_horizontal_axis() {
        // Right pin mirrored: 30 → -30
        let off = label_offset_for_pin(PinDirection::Right, 0, true);
        assert!((off.x - (-30.0)).abs() < 1e-6);
    }

    // ---- snap_and_dedup ----

    #[test]
    fn snap_and_dedup_removes_collinear_repeats() {
        // After snapping to grid 10, all three points are (0,0), (10,0), (10,0) — middle dup'd.
        let pts = vec![p(0.0, 0.0), p(10.0, 0.0), p(11.0, 0.0)];
        let cleaned = snap_and_dedup(&pts, 10.0);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(cleaned[0], p(0.0, 0.0));
        assert_eq!(cleaned[1], p(10.0, 0.0));
    }

    #[test]
    fn snap_and_dedup_preserves_distinct_points() {
        let pts = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)];
        let cleaned = snap_and_dedup(&pts, 10.0);
        assert_eq!(cleaned.len(), 3);
    }

    // ---- segments_cross ----

    #[test]
    fn segments_cross_interior_only() {
        // Cross at (5,5): both interior
        assert!(segments_cross(
            &p(0.0, 5.0),
            &p(10.0, 5.0),
            &p(5.0, 0.0),
            &p(5.0, 10.0)
        ));
        // Touch at endpoint: not a cross
        assert!(!segments_cross(
            &p(0.0, 0.0),
            &p(5.0, 0.0),
            &p(5.0, 0.0),
            &p(5.0, 10.0)
        ));
        // Parallel: no cross
        assert!(!segments_cross(
            &p(0.0, 0.0),
            &p(10.0, 0.0),
            &p(0.0, 5.0),
            &p(10.0, 5.0)
        ));
        // Disjoint: no cross
        assert!(!segments_cross(
            &p(0.0, 0.0),
            &p(1.0, 0.0),
            &p(5.0, 5.0),
            &p(6.0, 5.0)
        ));
    }

    // ---- l_route_best ----

    #[test]
    fn l_route_aligned_points_yields_straight_line() {
        let route = l_route_best(p(0.0, 0.0), p(10.0, 0.0), &[]);
        assert_eq!(route, vec![p(0.0, 0.0), p(10.0, 0.0)]);
    }

    #[test]
    fn l_route_default_is_horizontal_first() {
        let route = l_route_best(p(0.0, 0.0), p(10.0, 10.0), &[]);
        // H-first: corner at (10, 0)
        assert_eq!(route, vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)]);
    }

    #[test]
    fn l_route_picks_v_first_when_h_first_crosses() {
        // Place an existing horizontal wire that the H-first route would cross at (5,0)
        // but the V-first route avoids.
        // From (0,0) to (10,10).
        // H-first: (0,0) → (10,0) → (10,10). Segment (0,0)-(10,0) along y=0.
        // V-first: (0,0) → (0,10) → (10,10). Segment (0,10)-(10,10) along y=10.
        // Existing wire at y=0 from x=2 to x=8 will cross the H-first first segment
        // ... but it would only touch endpoints. Let's make the obstacle vertical:
        // existing vertical wire from (5, -5) to (5, 5) crosses the H-first
        // horizontal segment (0,0)-(10,0) at (5, 0) — interior of both.
        let obstacle = Wire {
            points: vec![p(5.0, -5.0), p(5.0, 5.0)],
        };
        let route = l_route_best(p(0.0, 0.0), p(10.0, 10.0), &[obstacle]);
        // Should have picked V-first: (0,0) → (0,10) → (10,10)
        assert_eq!(route, vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0)]);
    }

    // ---- minimum_spanning_tree ----

    #[test]
    fn mst_empty_and_singleton() {
        assert!(minimum_spanning_tree(&[]).is_empty());
        assert!(minimum_spanning_tree(&[p(0.0, 0.0)]).is_empty());
    }

    #[test]
    fn mst_two_points_is_one_edge() {
        let edges = minimum_spanning_tree(&[p(0.0, 0.0), p(10.0, 0.0)]);
        assert_eq!(edges, vec![(0, 1)]);
    }

    #[test]
    fn mst_chain_picks_shortest_edges() {
        // Three colinear points: 0 — 10 — 110
        // MST should be {(0,1), (1,2)} not {(0,1), (0,2)}.
        let pts = [p(0.0, 0.0), p(10.0, 0.0), p(110.0, 0.0)];
        let edges = minimum_spanning_tree(&pts);
        assert_eq!(edges.len(), 2);
        let mut s: Vec<(usize, usize)> = edges
            .iter()
            .map(|&(a, b)| if a < b { (a, b) } else { (b, a) })
            .collect();
        s.sort();
        assert_eq!(s, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn mst_total_length_optimal_on_square() {
        // 4 corners of a 10x10 square. MST total length = 30 (3 sides), not the
        // 30 + diagonal that a star from corner 0 would produce (10 + 10 + ~14.14).
        let pts = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
        let edges = minimum_spanning_tree(&pts);
        assert_eq!(edges.len(), 3);
        let total: f64 = edges
            .iter()
            .map(|&(a, b)| pts[a].distance_to(&pts[b]))
            .sum();
        assert!((total - 30.0).abs() < 1e-6, "MST total = {}", total);
    }
}
