use crate::analyzer::{BlockType, FunctionalBlock};
use crate::model::{builtin_symbols, Point};
use crate::parser::{SpiceDevice, SpiceParser};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct DevicePlacement {
    pub device_index: usize,
    pub symbol_name: String,
    pub position: Point,
    pub rotation: i32,
    pub mirrored: bool,
}

pub struct PlacementResult {
    pub placements: Vec<DevicePlacement>,
    pub bounding_rect: (Point, Point), // (min, max)
}

pub struct PlacerOptions {
    pub layer_spacing: f64,
    pub inter_block_spacing: f64,
    pub intra_block_spacing: f64,
    pub grid_size: f64,
}

impl Default for PlacerOptions {
    fn default() -> Self {
        Self {
            layer_spacing: 200.0,
            inter_block_spacing: 100.0,
            intra_block_spacing: 80.0,
            grid_size: 10.0,
        }
    }
}

struct BlockGraph {
    node_count: usize,
    adj: Vec<Vec<usize>>,
    radj: Vec<Vec<usize>>,
    edges: Vec<(usize, usize)>,
}

struct InternalLayout {
    placements: Vec<(usize, String, Point, i32, bool)>, // (dev_idx, sym, offset, rot, mir)
    #[allow(dead_code)]
    width: f64,
    height: f64,
}

pub struct SchematicPlacer;

impl SchematicPlacer {
    pub fn symbol_for_device(device: &SpiceDevice) -> String {
        match device.device_type {
            'M' => SpiceParser::infer_mos_type(device).to_string(),
            'R' => "resistor".into(),
            'C' => "capacitor".into(),
            'L' => "inductor".into(),
            'D' => "diode".into(),
            'Q' => SpiceParser::infer_bjt_type(device).to_string(),
            'V' => "vsource".into(),
            'I' => "isource".into(),
            'E' => "vcvs".into(),
            'G' => "vccs".into(),
            'H' => "ccvs".into(),
            'F' => "cccs".into(),
            'X' => format!("subckt_{}", device.model_or_value),
            _ => "resistor".into(),
        }
    }

    /// Polarity class of a block for PMOS-top / NMOS-bottom ordering:
    /// 0 = PMOS-only (top), 1 = mixed/neutral, 2 = NMOS-only (bottom).
    fn block_polarity_class(block: &FunctionalBlock, devices: &[SpiceDevice]) -> u8 {
        let mut has_pmos = false;
        let mut has_nmos = false;
        for &di in &block.device_indices {
            if di < devices.len() {
                match Self::polarity_symbol(&devices[di]).as_str() {
                    "pmos4" | "pnp" => has_pmos = true,
                    "nmos4" | "npn" => has_nmos = true,
                    _ => {}
                }
            }
        }
        match (has_pmos, has_nmos) {
            (true, false) => 0,
            (false, true) => 2,
            _ => 1,
        }
    }

    /// Symbol name used for polarity (PMOS-top / NMOS-bottom) sorting.
    /// X instances of PDK FET primitives resolve to nmos4/pmos4 via the
    /// model name so they sort like M devices; everything else keeps its
    /// rendering symbol.
    fn polarity_symbol(dev: &SpiceDevice) -> String {
        SpiceParser::infer_x_transistor_type(dev)
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::symbol_for_device(dev))
    }

    pub fn place(
        &self,
        blocks: &[FunctionalBlock],
        power_nets: &HashSet<String>,
        opts: &PlacerOptions,
    ) -> PlacementResult {
        self.place_with_devices(blocks, power_nets, opts, &[])
    }

    pub fn place_with_devices(
        &self,
        blocks: &[FunctionalBlock],
        power_nets: &HashSet<String>,
        opts: &PlacerOptions,
        devices: &[SpiceDevice],
    ) -> PlacementResult {
        if blocks.is_empty() {
            return PlacementResult {
                placements: Vec::new(),
                bounding_rect: (Point::new(0.0, 0.0), Point::new(0.0, 0.0)),
            };
        }

        // 1. Build DAG
        let graph = Self::build_dag(blocks, power_nets);

        // 2. Assign layers (with source proximity fix)
        let mut layer_assignment = Self::assign_layers(&graph);
        Self::fix_isolated_source_layers(&mut layer_assignment, blocks, &graph);
        Self::enforce_signal_flow(&mut layer_assignment, &graph);

        let max_layer = *layer_assignment.iter().max().unwrap_or(&0);
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
        for (i, &l) in layer_assignment.iter().enumerate() {
            layers[l].push(i);
        }

        // Diagnostic stats for scale work (docs/scale_placement.md).
        // Opt-in via env var; goes to stderr so it never pollutes output.
        if std::env::var("N2S_DEBUG_STATS").is_ok() {
            let mut sizes: Vec<usize> = blocks.iter().map(|b| b.device_indices.len()).collect();
            sizes.sort_unstable();
            let singletons = sizes.iter().filter(|&&s| s == 1).count();
            eprintln!(
                "STATS blocks={} singletons={} sizes(min/med/max)={}/{}/{}",
                blocks.len(),
                singletons,
                sizes.first().unwrap_or(&0),
                sizes.get(sizes.len() / 2).unwrap_or(&0),
                sizes.last().unwrap_or(&0)
            );
            let mut widths: Vec<usize> = layers.iter().map(|l| l.len()).collect();
            let max_w = widths.iter().max().copied().unwrap_or(0);
            widths.sort_unstable();
            eprintln!(
                "STATS dag_edges={} layers={} layer_width(med/max)={}/{} last_layer={}",
                graph.edges.len(),
                layers.len(),
                widths.get(widths.len() / 2).unwrap_or(&0),
                max_w,
                layers.last().map(|l| l.len()).unwrap_or(0)
            );
        }

        // 3. Crossing minimization
        Self::minimize_crossings(&mut layers, &graph, 4);

        // 3.5. Sort blocks within each layer: PMOS-containing above NMOS-containing
        if !devices.is_empty() {
            Self::sort_blocks_by_polarity(&mut layers, blocks, devices);
        }

        // 4. Block-internal layouts
        let block_layouts: Vec<InternalLayout> = blocks
            .iter()
            .map(|b| Self::layout_block(b, devices, opts))
            .collect();

        // 5. Absolute coordinates
        //    When a layer has many blocks, arrange them in a grid to avoid
        //    extreme vertical aspect ratios. The target is roughly square.
        let mut placements = Vec::new();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        // Per-layer strip footprint: total width of the layer's grid and
        // the tallest of its columns.
        let mut strip_w: Vec<f64> = Vec::new();
        let mut strip_h: Vec<f64> = Vec::new();
        for layer in &layers {
            let cols = Self::compute_grid_columns(layer, &block_layouts, opts, blocks, devices);
            let mut w = 0.0f64;
            let mut h = 0.0f64;
            for col_blocks in &cols {
                let col_w = col_blocks
                    .iter()
                    .map(|&bi| block_layouts[bi].width)
                    .fold(0.0f64, f64::max);
                w += col_w + opts.layer_spacing;
                let col_h: f64 = col_blocks
                    .iter()
                    .map(|&bi| block_layouts[bi].height + opts.inter_block_spacing)
                    .sum();
                h = h.max(col_h);
            }
            strip_w.push(w.max(opts.layer_spacing));
            strip_h.push(h);
        }

        // Scale Phase 2 — depth folding (docs/scale_placement.md). A deep
        // design laid out as one horizontal strip per layer becomes an
        // unreadable ribbon (case 36: 67 layers, 28k px wide). When the
        // design is deep enough, fold the layer sequence into horizontal
        // bands sized so the canvas comes out roughly square: with total
        // strip width S and mean strip height H, b = sqrt(S/H) bands of
        // width ~S/b give width ≈ height. Signal flow reads left→right
        // within a band, bands top→down (newspaper order). Shallow designs
        // (every current case except 36) are untouched.
        const FOLD_MIN_LAYERS: usize = 12;
        let total_w: f64 = strip_w.iter().sum();
        let mut layer_x_start: Vec<f64> = Vec::with_capacity(layers.len());
        let mut layer_y_start: Vec<f64> = Vec::with_capacity(layers.len());
        if layers.len() >= FOLD_MIN_LAYERS {
            let h_bar = (strip_h.iter().sum::<f64>() / strip_h.len() as f64).max(1.0);
            let bands = (total_w / h_bar).sqrt().round().max(1.0);
            let band_target_w = total_w / bands;
            let mut x_cursor = 0.0;
            let mut band_y = 0.0;
            let mut band_h = 0.0f64;
            for l in 0..layers.len() {
                if x_cursor > 0.0 && x_cursor + strip_w[l] > band_target_w {
                    band_y += band_h + opts.layer_spacing;
                    x_cursor = 0.0;
                    band_h = 0.0;
                }
                layer_x_start.push(x_cursor);
                layer_y_start.push(band_y);
                x_cursor += strip_w[l];
                band_h = band_h.max(strip_h[l]);
            }
        } else {
            let mut x_cursor = 0.0;
            for w in strip_w.iter().take(layers.len()) {
                layer_x_start.push(x_cursor);
                layer_y_start.push(0.0);
                x_cursor += w;
            }
        }

        for (l, layer) in layers.iter().enumerate() {
            let base_x = layer_x_start[l];
            let cols = Self::compute_grid_columns(layer, &block_layouts, opts, blocks, devices);

            let mut col_x = base_x;
            for col_blocks in &cols {
                let mut y_cursor = layer_y_start[l];
                let col_width = col_blocks
                    .iter()
                    .map(|&bi| block_layouts[bi].width)
                    .fold(0.0f64, f64::max);

                for &block_idx in col_blocks {
                    let layout = &block_layouts[block_idx];
                    let anchor = Point::new(col_x, y_cursor);

                    for &(dev_idx, ref sym, offset, rot, mir) in &layout.placements {
                        let pos = (anchor + offset).snap_to_grid(opts.grid_size);
                        placements.push(DevicePlacement {
                            device_index: dev_idx,
                            symbol_name: sym.clone(),
                            position: pos,
                            rotation: rot,
                            mirrored: mir,
                        });
                        min_x = min_x.min(pos.x - 30.0);
                        min_y = min_y.min(pos.y - 25.0);
                        max_x = max_x.max(pos.x + 30.0);
                        max_y = max_y.max(pos.y + 25.0);
                    }

                    y_cursor += layout.height + opts.inter_block_spacing;
                }

                col_x += col_width + opts.layer_spacing;
            }
        }

        // 6. Align matched device pairs (symmetry improvement) and pull
        //    isolated sources beside the devices they drive.
        if !devices.is_empty() {
            Self::align_matched_pairs(&mut placements, blocks, devices, opts);
            Self::align_isolated_sources(&mut placements, blocks, devices, &graph, opts);
            // Recompute bounding rect after alignment
            min_x = f64::MAX;
            min_y = f64::MAX;
            max_x = f64::MIN;
            max_y = f64::MIN;
            for p in &placements {
                min_x = min_x.min(p.position.x - 30.0);
                min_y = min_y.min(p.position.y - 25.0);
                max_x = max_x.max(p.position.x + 30.0);
                max_y = max_y.max(p.position.y + 25.0);
            }
        }

        PlacementResult {
            placements,
            bounding_rect: (Point::new(min_x, min_y), Point::new(max_x, max_y)),
        }
    }

    /// Canonical key for identifying devices that should be laid out as a
    /// matched pair — same symbol, same geometric parameters (W/L), same
    /// model. Two devices with identical keys are considered electrically
    /// and visually interchangeable, so the placer can put them at the same
    /// y-coordinate for symmetric schematics.
    fn device_match_key(dev: &SpiceDevice) -> String {
        let sym_name = Self::symbol_for_device(dev);
        let mut key_parts = vec![sym_name];
        let mut props: Vec<(&str, &str)> = dev
            .parameters
            .iter()
            .filter(|(k, _)| matches!(k.as_str(), "W" | "L"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        props.push(("model", &dev.model_or_value));
        props.sort();
        for (k, v) in props {
            if !v.is_empty() {
                key_parts.push(format!("{}={}", k, v));
            }
        }
        key_parts.join("|")
    }

    // ========================================================================
    // Cross-block symmetry alignment
    // ========================================================================

    /// Post-processing pass: identify matched device pairs across different
    /// blocks and align their y-coordinates for visual symmetry.
    ///
    /// A "matched pair" is two devices with the same symbol type and identical
    /// key properties (W, L, model) — e.g., the two load resistors in a diff
    /// pair, or two mirror transistors in separate blocks.
    fn align_matched_pairs(
        placements: &mut [DevicePlacement],
        blocks: &[FunctionalBlock],
        devices: &[SpiceDevice],
        opts: &PlacerOptions,
    ) {
        // Build device_index → (placement_index, block_index) maps
        let mut dev_to_placement: HashMap<usize, usize> = HashMap::new();
        for (pi, p) in placements.iter().enumerate() {
            dev_to_placement.insert(p.device_index, pi);
        }

        let mut dev_to_block: HashMap<usize, usize> = HashMap::new();
        for (bi, block) in blocks.iter().enumerate() {
            for &di in &block.device_indices {
                dev_to_block.insert(di, bi);
            }
        }

        let device_rects = Self::device_bounding_rects(devices);
        let rects_collide = |di_a: usize, pa: Point, di_b: usize, pb: Point| -> bool {
            Self::rects_collide(&device_rects, di_a, pa, di_b, pb)
        };

        // Group devices by matching key: (symbol_name, sorted W/L/model).
        // Skip V/I sources: two independent voltage sources happen to have
        // identical match keys (no W/L parameters, model_or_value parsed
        // from "AC=1" or similar), and forcing their y-coordinates to
        // align collapses disconnected sub-circuits onto the same row —
        // Bug 2 in the 2026-05-01 test-set-expansion findings (circuit
        // 14_disconnected_filters: V1 and V2 ended up at identical
        // (x, y)). Pure independent sources should not be treated as a
        // symmetry pair.
        let mut match_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (di, dev) in devices.iter().enumerate() {
            if matches!(dev.device_type, 'V' | 'I') {
                continue;
            }
            // X-instance boxes that are not FET primitives (synthetic
            // gates, library cells, hierarchical blocks) are not analog
            // mirror pairs; aligning two same-model cells across a digital
            // design is meaningless churn (mirrors the symmetry metric's
            // exclusion).
            if dev.device_type == 'X' && SpiceParser::infer_x_transistor_type(dev).is_none() {
                continue;
            }
            let key = Self::device_match_key(dev);
            match_groups.entry(key).or_default().push(di);
        }

        // For groups of exactly 2, align y-coordinates if in different blocks.
        // Iterate in sorted-key order: HashMap order is random per run, and
        // pairs processed earlier change the collision landscape for later
        // ones — unsorted iteration made Tier 1 symmetry flaky (case 34
        // passed or failed depending on the run).
        let mut sorted_groups: Vec<(&String, &Vec<usize>)> = match_groups.iter().collect();
        sorted_groups.sort_by_key(|(k, _)| k.as_str().to_string());
        // Iterate to a fixpoint (bounded): an early pair's move can be
        // blocked by a device that a LATER pair's move frees up (case 34:
        // CL1 could not drop onto CF1's spot until the CF pair moved CF1
        // away). One extra sweep lets those pairs align; the bound keeps
        // pathological block-sharing from oscillating forever.
        for _pass in 0..3 {
            let mut moved_any = false;
            for (_, group) in &sorted_groups {
                if group.len() != 2 {
                    continue;
                }

                let di_a = group[0];
                let di_b = group[1];

                let block_a = match dev_to_block.get(&di_a) {
                    Some(&b) => b,
                    None => continue,
                };
                let block_b = match dev_to_block.get(&di_b) {
                    Some(&b) => b,
                    None => continue,
                };

                // Skip if same block (already handled by block template)
                if block_a == block_b {
                    continue;
                }

                let pi_a = match dev_to_placement.get(&di_a) {
                    Some(&p) => p,
                    None => continue,
                };
                let pi_b = match dev_to_placement.get(&di_b) {
                    Some(&p) => p,
                    None => continue,
                };

                let y_a = placements[pi_a].position.y;
                let y_b = placements[pi_b].position.y;
                let y_diff = (y_a - y_b).abs();

                // Only align if they're at meaningfully different y positions
                if y_diff < opts.grid_size {
                    continue;
                }

                // A symmetric pair sits side-by-side; if the two devices are in
                // the same column, aligning y would stack them onto the same
                // point (case 34: XNR1/XNR2 collapsed to identical coordinates).
                let (pi_a2, pi_b2) = (dev_to_placement[&di_a], dev_to_placement[&di_b]);
                let x_gap = (placements[pi_a2].position.x - placements[pi_b2].position.x).abs();
                let min_x_gap = (device_rects[di_a].width + device_rects[di_b].width) / 2.0;
                if x_gap < min_x_gap {
                    continue;
                }

                // Candidate moves: prefer shifting the smaller block, but fall
                // back to the other one if the first would collide (case 30:
                // XN_TAIL's column is blocked at the target row, while XN_TAIL2
                // can safely drop to XN_TAIL's row instead).
                let size_a = blocks[block_a].device_indices.len();
                let size_b = blocks[block_b].device_indices.len();
                let candidates = if size_a <= size_b {
                    [(block_a, di_b, di_a), (block_b, di_a, di_b)]
                } else {
                    [(block_b, di_a, di_b), (block_a, di_b, di_a)]
                };

                let mut aligned = false;
                for (move_block, anchor_di, move_di) in candidates {
                    let anchor_pi = dev_to_placement[&anchor_di];
                    let move_pi = dev_to_placement[&move_di];

                    // Delta to align the moving device with the anchor's y
                    let dy = placements[anchor_pi].position.y - placements[move_pi].position.y;
                    if dy.abs() < opts.grid_size {
                        aligned = true;
                        break;
                    }

                    // Collision check: simulate the shift and skip it if any
                    // moved device would land on a stationary one (case 30: a
                    // block shift dropped XP2 exactly onto XN_TAIL).
                    let move_pis: HashSet<usize> = blocks[move_block]
                        .device_indices
                        .iter()
                        .filter_map(|di| dev_to_placement.get(di).copied())
                        .collect();
                    let collides = blocks[move_block].device_indices.iter().any(|&mdi| {
                        let Some(&mpi) = dev_to_placement.get(&mdi) else {
                            return false;
                        };
                        if mdi >= devices.len() {
                            return false;
                        }
                        let moved_pos =
                            Point::new(placements[mpi].position.x, placements[mpi].position.y + dy);
                        placements.iter().enumerate().any(|(pi, p)| {
                            !move_pis.contains(&pi)
                                && p.device_index < devices.len()
                                && rects_collide(mdi, moved_pos, p.device_index, p.position)
                        })
                    });
                    if collides {
                        continue;
                    }

                    // Shift all devices in the moving block by dy
                    for &di in &blocks[move_block].device_indices {
                        if let Some(&pi) = dev_to_placement.get(&di) {
                            placements[pi].position.y += dy;
                            placements[pi].position =
                                placements[pi].position.snap_to_grid(opts.grid_size);
                        }
                    }
                    moved_any = true;
                    aligned = true;
                    break;
                }

                // Last resort when BOTH whole-block moves are vetoed:
                // passives (R/C/L) are leaf decorations, so moving just the
                // device — without its block — doesn't hurt readability.
                // Case 34's CL1/CL2 load caps were scattered to opposite
                // canvas ends by clustering, and each block shift collided
                // with a legitimate neighbor (VVDD on one side, the XP3/XP4
                // latch on the other).
                if !aligned && matches!(devices[di_a].device_type, 'R' | 'C' | 'L') {
                    for (move_di, anchor_di) in [(di_a, di_b), (di_b, di_a)] {
                        let (anchor_pi, move_pi) =
                            (dev_to_placement[&anchor_di], dev_to_placement[&move_di]);
                        let dy = placements[anchor_pi].position.y - placements[move_pi].position.y;
                        if dy.abs() < opts.grid_size {
                            break;
                        }
                        let moved_pos = Point::new(
                            placements[move_pi].position.x,
                            placements[move_pi].position.y + dy,
                        )
                        .snap_to_grid(opts.grid_size);
                        let collides = placements.iter().enumerate().any(|(pi, p)| {
                            pi != move_pi
                                && p.device_index < devices.len()
                                && rects_collide(move_di, moved_pos, p.device_index, p.position)
                        });
                        if !collides {
                            placements[move_pi].position = moved_pos;
                            moved_any = true;
                            break;
                        }
                    }
                }
            }
            if !moved_any {
                break;
            }
        }
    }

    // ========================================================================
    // Isolated-source y-alignment
    // ========================================================================

    /// Post-processing pass: pull each isolated block — an independent V/I
    /// source, or any single stranded device — next to the device it
    /// connects to, instead of leaving it floating in a column of its own.
    /// Tries an in-place y-align first (P3's original move); when that
    /// can't work (same column, or destination occupied), relocates a
    /// single-device block to a free spot directly beside its load (the
    /// x+y move that the P3 partial fix deferred).
    ///
    /// An "isolated source" is a source-only block with no DAG edges — its
    /// only connections are power nets, which `build_dag` deliberately
    /// excludes, so it never attracts to anything during layering and ends
    /// up floating (the VVDD/VINP/VINN/VCLK scatter seen on the comparator
    /// cases). We align its y-coordinate to the device sitting on its one
    /// signal net so it lands beside its load.
    ///
    /// Guards that keep this safe:
    /// - Pure supplies (only power nets, e.g. VVDD) have no signal net and
    ///   are left alone — this also avoids collapsing genuinely disconnected
    ///   sources together (cf. Bug 2, circuit 14).
    /// - We only move when the source and target are in different columns
    ///   (aligning within one column would stack them at the same point).
    /// - A collision check skips the move if the destination is already
    ///   occupied by another block.
    fn align_isolated_sources(
        placements: &mut [DevicePlacement],
        blocks: &[FunctionalBlock],
        devices: &[SpiceDevice],
        graph: &BlockGraph,
        opts: &PlacerOptions,
    ) {
        let mut has_edges = vec![false; graph.node_count];
        for &(from, to) in &graph.edges {
            has_edges[from] = true;
            has_edges[to] = true;
        }

        let device_rects = &Self::device_bounding_rects(devices)[..];

        let mut dev_to_placement: HashMap<usize, usize> = HashMap::new();
        for (pi, p) in placements.iter().enumerate() {
            dev_to_placement.insert(p.device_index, pi);
        }
        let mut dev_to_block: HashMap<usize, usize> = HashMap::new();
        for (bi, block) in blocks.iter().enumerate() {
            for &di in &block.device_indices {
                dev_to_block.insert(di, bi);
            }
        }
        // net (lowercased) → device indices touching it
        let mut net_devices: HashMap<String, Vec<usize>> = HashMap::new();
        for (di, dev) in devices.iter().enumerate() {
            for node in &dev.nodes {
                net_devices.entry(node.to_lowercase()).or_default().push(di);
            }
        }

        for (bi, block) in blocks.iter().enumerate() {
            if bi >= has_edges.len() || has_edges[bi] || block.device_indices.is_empty() {
                continue;
            }
            // Source-only blocks (the original P3 scope) plus single-device
            // blocks of any type: a lone transistor with no DAG edges (case
            // 34's XPT tail header, case 35's XNC clock inverter) floats in
            // a column of its own exactly like an input source does.
            let all_sources = block
                .device_indices
                .iter()
                .all(|&di| di < devices.len() && matches!(devices[di].device_type, 'V' | 'I'));
            if !all_sources && block.device_indices.len() != 1 {
                continue;
            }
            // Signal nets = this block's nets minus the hard-coded power
            // rails. We must NOT use the analyzer's `power_nets` here: that
            // set is augmented with every voltage-source terminal, so an
            // input source's own signal net (e.g. `inp`) would be classed as
            // power and filtered out. Only true rails disqualify a net.
            let is_rail = |net: &str| {
                matches!(
                    net,
                    "0" | "gnd"
                        | "gnd!"
                        | "vss"
                        | "vss!"
                        | "vdd"
                        | "vdd!"
                        | "vcc"
                        | "vcc!"
                        | "avdd"
                        | "avss"
                        | "vpwr"
                        | "vgnd"
                        | "vpb"
                        | "vnb"
                )
            };
            let signal_nets: Vec<String> = block
                .all_nets
                .iter()
                .map(|s| s.to_lowercase())
                .filter(|s| !is_rail(s))
                .collect();
            if signal_nets.is_empty() {
                continue;
            }
            // Target = device on a signal net, in another block; prefer the
            // most-anchored (largest) block, tie-break by lowest index.
            let mut target_di: Option<usize> = None;
            let mut best_size = 0usize;
            for net in &signal_nets {
                let Some(devs) = net_devices.get(net) else {
                    continue;
                };
                for &di in devs {
                    let Some(&tb) = dev_to_block.get(&di) else {
                        continue;
                    };
                    if tb == bi {
                        continue;
                    }
                    let size = blocks[tb].device_indices.len();
                    let better = match target_di {
                        None => true,
                        Some(cur) => size > best_size || (size == best_size && di < cur),
                    };
                    if better {
                        target_di = Some(di);
                        best_size = size;
                    }
                }
            }
            let Some(target_di) = target_di else {
                continue;
            };

            let src_di = block.device_indices[0];
            let (Some(&src_pi), Some(&tgt_pi)) = (
                dev_to_placement.get(&src_di),
                dev_to_placement.get(&target_di),
            ) else {
                continue;
            };

            let dx = (placements[src_pi].position.x - placements[tgt_pi].position.x).abs();
            let dy = placements[tgt_pi].position.y - placements[src_pi].position.y;

            let block_pis: HashSet<usize> = block
                .device_indices
                .iter()
                .filter_map(|di| dev_to_placement.get(di).copied())
                .collect();

            // A non-source device whose match key appears exactly twice is a
            // pair the symmetry metric scores, and align_matched_pairs (which
            // ran just before this pass) owns its row. Any y-changing move
            // here would undo that alignment — case 30's XN_TAIL2 was pulled
            // off its freshly-aligned row by branch (A). Paired devices are
            // limited to the sideways pull in branch (B).
            let paired =
                !all_sources && block.device_indices.len() == 1 && src_di < devices.len() && {
                    let key = Self::device_match_key(&devices[src_di]);
                    devices
                        .iter()
                        .filter(|d| {
                            !matches!(d.device_type, 'V' | 'I') && Self::device_match_key(d) == key
                        })
                        .count()
                        == 2
                };

            // (A) In-place y-align: different columns, meaningful y move,
            // and every member's destination is collision-free.
            if !paired && dx >= opts.intra_block_spacing && dy.abs() >= opts.grid_size {
                let collides = block.device_indices.iter().any(|&mdi| {
                    let Some(&mpi) = dev_to_placement.get(&mdi) else {
                        return false;
                    };
                    if mdi >= devices.len() {
                        return false;
                    }
                    let moved =
                        Point::new(placements[mpi].position.x, placements[mpi].position.y + dy);
                    placements.iter().enumerate().any(|(pi, p)| {
                        !block_pis.contains(&pi)
                            && p.device_index < devices.len()
                            && Self::rects_collide(
                                device_rects,
                                mdi,
                                moved,
                                p.device_index,
                                p.position,
                            )
                    })
                });
                if !collides {
                    for &di in &block.device_indices {
                        if let Some(&pi) = dev_to_placement.get(&di) {
                            placements[pi].position.y += dy;
                            placements[pi].position =
                                placements[pi].position.snap_to_grid(opts.grid_size);
                        }
                    }
                    continue;
                }
            }

            // (B) x+y relocation: the P3 remainder. When a pure y-shift
            // cannot help (source shares the target's column, or the
            // destination row is occupied), move a single-device block to a
            // free spot directly beside its load.
            //
            // Symmetry guard: devices whose match key appears exactly twice
            // are scored as a pair by the symmetry metric, and
            // align_matched_pairs has already put them on one row — moving
            // one member vertically would break that (the first version of
            // this pass regressed cases 06/19/20/29 exactly this way). Such
            // devices only get a sideways pull at their CURRENT y. Sources
            // and unpaired devices may take any free spot around the target.
            if block.device_indices.len() != 1 || src_di >= devices.len() {
                continue;
            }
            let (sr, tr) = (device_rects[src_di], device_rects[target_di]);
            let clearance = 20.0;
            let cur = placements[src_pi].position;
            let tgt_pos = placements[tgt_pi].position;
            let x_off = (sr.width + tr.width) / 2.0 + clearance;
            let y_off = (sr.height + tr.height) / 2.0 + clearance;
            let candidates: Vec<Point> = if paired {
                vec![
                    Point::new(tgt_pos.x - x_off, cur.y),
                    Point::new(tgt_pos.x + x_off, cur.y),
                ]
            } else {
                vec![
                    Point::new(tgt_pos.x - x_off, tgt_pos.y),
                    Point::new(tgt_pos.x + x_off, tgt_pos.y),
                    Point::new(tgt_pos.x, tgt_pos.y - y_off),
                    Point::new(tgt_pos.x, tgt_pos.y + y_off),
                ]
            };
            let dist_now = (cur.x - tgt_pos.x).abs() + (cur.y - tgt_pos.y).abs();
            for cand in candidates {
                let cand = cand.snap_to_grid(opts.grid_size);
                // Only move if it brings the device meaningfully closer.
                let dist_cand = (cand.x - tgt_pos.x).abs() + (cand.y - tgt_pos.y).abs();
                if dist_cand + opts.grid_size >= dist_now {
                    continue;
                }
                let collides = placements.iter().enumerate().any(|(pi, p)| {
                    pi != src_pi
                        && p.device_index < devices.len()
                        && Self::rects_collide(
                            device_rects,
                            src_di,
                            cand,
                            p.device_index,
                            p.position,
                        )
                });
                if !collides {
                    placements[src_pi].position = cand;
                    break;
                }
            }
        }
    }

    // ========================================================================
    // PMOS-above-NMOS block ordering
    // ========================================================================

    /// Sort blocks within each layer so that PMOS-containing blocks appear
    /// above (earlier in the list = lower y = higher on screen) NMOS-containing
    /// blocks. This enforces the standard schematic convention where power
    /// rails are at the top and ground rails are at the bottom.
    ///
    /// Polarity classification:
    /// - PMOS block: contains at least one PMOS device and no NMOS devices
    /// - NMOS block: contains at least one NMOS device and no PMOS devices
    /// - Mixed/neutral: contains both or neither — keeps original position
    ///
    /// Within the same polarity group, the original crossing-minimized order
    /// is preserved (stable sort).
    fn sort_blocks_by_polarity(
        layers: &mut [Vec<usize>],
        blocks: &[FunctionalBlock],
        devices: &[SpiceDevice],
    ) {
        for layer in layers.iter_mut() {
            if layer.len() <= 1 {
                continue;
            }
            // Stable sort preserves crossing-minimized order within same polarity
            layer.sort_by_key(|&bi| Self::block_polarity_class(&blocks[bi], devices));
        }
    }

    // ========================================================================
    // Multi-column grid layout for large layers
    // ========================================================================

    /// When a layer has many blocks, split them into multiple columns to
    /// achieve a roughly square aspect ratio instead of an extreme vertical strip.
    ///
    /// Returns a vector of columns, each column being a slice of block indices.
    fn compute_grid_columns(
        layer: &[usize],
        block_layouts: &[InternalLayout],
        opts: &PlacerOptions,
        blocks: &[FunctionalBlock],
        devices: &[SpiceDevice],
    ) -> Vec<Vec<usize>> {
        if layer.len() <= 2 {
            // 1-2 blocks: single column is fine
            return vec![layer.to_vec()];
        }

        // Scale Phase 2 — bus alignment. In a WIDE layer (grid regime),
        // stable-sort blocks by a kind signature before the balanced
        // greedy distribution: identical blocks then round-robin across
        // the grid columns at matching row positions, so repeated cells
        // (bit slices, gate banks) line up in visible rows instead of
        // being scattered by barycenter noise. Stable sort preserves the
        // crossing-minimized order within each kind group; small layers
        // keep their pure barycenter order.
        let sorted_layer: Vec<usize>;
        let layer: &[usize] = if layer.len() >= 6 && !devices.is_empty() {
            let kind_key = |bi: usize| -> String {
                let mut parts: Vec<String> = blocks[bi]
                    .device_indices
                    .iter()
                    .filter(|&&di| di < devices.len())
                    .map(|&di| {
                        format!("{}:{}", devices[di].device_type, devices[di].model_or_value)
                    })
                    .collect();
                parts.sort_unstable();
                parts.join(",")
            };
            let mut v: Vec<usize> = layer.to_vec();
            // Composite key: polarity class FIRST — the kind grouping must
            // not undo sort_blocks_by_polarity's PMOS-top ordering (doing
            // so regressed cases 06/17/20/32 on the first attempt).
            v.sort_by_key(|&bi| {
                (
                    Self::block_polarity_class(&blocks[bi], devices),
                    kind_key(bi),
                )
            });
            sorted_layer = v;
            &sorted_layer
        } else {
            layer
        };

        // Compute total height if all blocks were in one column
        let total_height: f64 = layer
            .iter()
            .map(|&bi| block_layouts[bi].height + opts.inter_block_spacing)
            .sum();

        // Compute max block width (gives us the column width)
        let max_block_width: f64 = layer
            .iter()
            .map(|&bi| block_layouts[bi].width)
            .fold(60.0f64, f64::max);

        // Target: aspect ratio close to 1.5 (slightly wider than tall).
        // num_cols = ceil(sqrt(total_height / (target_ratio * col_width)))
        let target_ratio = 1.5;
        let ideal_cols =
            (total_height / (target_ratio * (max_block_width + opts.layer_spacing))).sqrt();
        let num_cols = (ideal_cols.ceil() as usize).max(1).min(layer.len());

        if num_cols <= 1 {
            return vec![layer.to_vec()];
        }

        // Distribute blocks across columns, balancing total height per column.
        // Greedy: assign each block to the column with the smallest current height.
        let mut columns: Vec<Vec<usize>> = (0..num_cols).map(|_| Vec::new()).collect();
        let mut col_heights: Vec<f64> = vec![0.0; num_cols];

        for bi in layer {
            // Find the column with minimum height
            let min_col = col_heights
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            columns[min_col].push(*bi);
            col_heights[min_col] += block_layouts[*bi].height + opts.inter_block_spacing;
        }

        // Remove empty columns
        columns.retain(|c| !c.is_empty());
        columns
    }

    // ========================================================================
    // DAG construction
    // ========================================================================

    fn build_dag(blocks: &[FunctionalBlock], power_nets: &HashSet<String>) -> BlockGraph {
        let n = blocks.len();
        let mut adj = vec![Vec::new(); n];
        let mut radj = vec![Vec::new(); n];
        let mut edges = Vec::new();

        // net → producing block indices
        let mut net_producers: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, b) in blocks.iter().enumerate() {
            for net in &b.output_nets {
                if !power_nets.contains(&net.to_lowercase()) {
                    net_producers.entry(net.clone()).or_default().push(i);
                }
            }
        }

        let mut edge_set: HashSet<(usize, usize)> = HashSet::new();
        for (i, b) in blocks.iter().enumerate() {
            for net in &b.input_nets {
                if power_nets.contains(&net.to_lowercase()) {
                    continue;
                }
                if let Some(producers) = net_producers.get(net) {
                    for &j in producers {
                        if j != i && !edge_set.contains(&(j, i)) {
                            edge_set.insert((j, i));
                            edges.push((j, i));
                            adj[j].push(i);
                            radj[i].push(j);
                        }
                    }
                }
            }
        }

        // Cycle removal via DFS
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }
        let mut color = vec![Color::White; n];
        let mut back_edges: HashSet<usize> = HashSet::new();

        fn dfs(u: usize, color: &mut [Color], edges: &[(usize, usize)], back: &mut HashSet<usize>) {
            color[u] = Color::Gray;
            for (idx, &(from, to)) in edges.iter().enumerate() {
                if from != u {
                    continue;
                }
                match color[to] {
                    Color::Gray => {
                        back.insert(idx);
                    }
                    Color::White => dfs(to, color, edges, back),
                    Color::Black => {}
                }
            }
            color[u] = Color::Black;
        }

        for i in 0..n {
            if color[i] == Color::White {
                dfs(i, &mut color, &edges, &mut back_edges);
            }
        }

        // Reverse back edges
        for &idx in &back_edges {
            let (from, to) = edges[idx];
            adj[from].retain(|&v| v != to);
            radj[to].retain(|&v| v != from);
            edges[idx] = (to, from);
            adj[to].push(from);
            radj[from].push(to);
        }

        BlockGraph {
            node_count: n,
            adj,
            radj,
            edges,
        }
    }

    // ========================================================================
    // Layer assignment (longest path)
    // ========================================================================

    fn assign_layers(graph: &BlockGraph) -> Vec<usize> {
        let n = graph.node_count;
        let mut layers = vec![0usize; n];
        let mut in_deg = vec![0usize; n];
        for &(_, to) in &graph.edges {
            in_deg[to] += 1;
        }

        let mut queue: VecDeque<usize> = VecDeque::new();
        for (i, &deg) in in_deg.iter().enumerate() {
            if deg == 0 {
                queue.push_back(i);
            }
        }

        let mut topo = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            topo.push(u);
            for &v in &graph.adj[u] {
                in_deg[v] -= 1;
                if in_deg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        // Add isolated nodes
        if topo.len() < n {
            let visited: HashSet<usize> = topo.iter().copied().collect();
            for i in 0..n {
                if !visited.contains(&i) {
                    topo.push(i);
                }
            }
        }

        for &u in &topo {
            for &v in &graph.adj[u] {
                if layers[u] + 1 > layers[v] {
                    layers[v] = layers[u] + 1;
                }
            }
        }

        layers
    }

    /// Enforce left-to-right signal flow by pushing terminal sinks and their
    /// chains as far right as the DAG permits (ALAP for downstream nodes).
    ///
    /// A "terminal sink" is a block with incoming DAG edges but no outgoing
    /// edges — e.g., output load caps, output resistors. These end up at
    /// max_layer so outputs appear on the right side of the schematic.
    ///
    /// For each terminal sink, we also walk back through its predecessors and
    /// raise them toward max_layer - 1, max_layer - 2, etc., as long as doing
    /// so does not violate successor constraints. This prevents "gaps" where
    /// a sink was pushed right but its predecessors remained at a low ASAP
    /// layer, which would create long backward-looking connections.
    fn enforce_signal_flow(layers: &mut [usize], graph: &BlockGraph) {
        let n = graph.node_count;
        if n == 0 {
            return;
        }
        let max_layer = *layers.iter().max().unwrap_or(&0);
        if max_layer == 0 {
            return;
        }

        let mut has_in = vec![false; n];
        let mut has_out = vec![false; n];
        for &(from, to) in &graph.edges {
            has_out[from] = true;
            has_in[to] = true;
        }

        // ALAP computation: layer[v] = min over successors (layer[s]) - 1.
        // Seed sinks with max_layer, all others with max_layer (upper bound),
        // then relax iteratively in reverse topo-ish order.
        //
        // Only blocks with at least one outgoing edge are constrained by
        // successors; terminal sinks are fixed at max_layer. Isolated blocks
        // (no in, no out) keep their existing layer — they were already
        // placed by fix_isolated_source_layers and should not move.
        let mut alap = vec![max_layer; n];

        // Iterate until fixpoint (DAG is small, this converges quickly).
        loop {
            let mut changed = false;
            for u in 0..n {
                if !has_out[u] {
                    continue;
                } // sink: stays at max_layer
                  // For each successor, constrain alap[u] <= alap[s] - 1
                let mut new_layer = max_layer;
                for &s in &graph.adj[u] {
                    if alap[s] == 0 {
                        // Successor pinned at 0 — cannot place u earlier than itself;
                        // keep u at its ASAP layer (no change from ALAP side).
                        new_layer = new_layer.min(layers[u]);
                    } else {
                        new_layer = new_layer.min(alap[s] - 1);
                    }
                }
                if new_layer != alap[u] {
                    alap[u] = new_layer;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Final layer: for non-isolated blocks, use max(ASAP, ALAP). Since
        // ALAP >= ASAP by construction (ALAP starts at max_layer and only
        // decreases via successor constraints, which are dominated by
        // ASAP+path_length), this effectively pushes blocks to the right.
        //
        // Isolated blocks (no edges) are untouched.
        for i in 0..n {
            if !has_in[i] && !has_out[i] {
                continue;
            }
            // Pure signal sources (no in-edges) stay at ASAP to keep inputs
            // anchored on the left.
            if !has_in[i] {
                continue;
            }
            if alap[i] > layers[i] {
                layers[i] = alap[i];
            }
        }
    }

    /// Move isolated blocks (no DAG edges) to the same layer as the
    /// non-isolated block they share the most nets with. This prevents
    /// V/I sources from piling up at layer 0 when their terminals are
    /// all classified as power nets.
    fn fix_isolated_source_layers(
        layers: &mut [usize],
        blocks: &[FunctionalBlock],
        graph: &BlockGraph,
    ) {
        let n = graph.node_count;
        let mut has_edges = vec![false; n];
        for &(from, to) in &graph.edges {
            has_edges[from] = true;
            has_edges[to] = true;
        }

        for i in 0..n {
            if has_edges[i] {
                continue;
            }

            // Find non-isolated block sharing the most nets
            let mut best_target: Option<usize> = None;
            let mut best_shared = 0usize;

            for j in 0..n {
                if i == j || !has_edges[j] {
                    continue;
                }
                let shared = blocks[i].all_nets.intersection(&blocks[j].all_nets).count();
                if shared > best_shared {
                    best_shared = shared;
                    best_target = Some(j);
                }
            }

            if let Some(j) = best_target {
                // Place in same layer as target block
                layers[i] = layers[j];
            }
        }
    }

    // ========================================================================
    // Crossing minimization (barycenter)
    // ========================================================================

    fn minimize_crossings(layers: &mut [Vec<usize>], graph: &BlockGraph, iterations: usize) {
        if layers.len() <= 1 {
            return;
        }

        let mut node_layer: HashMap<usize, usize> = HashMap::new();
        for (l, layer) in layers.iter().enumerate() {
            for &n in layer {
                node_layer.insert(n, l);
            }
        }

        let position_in_layer = |node: usize, layer: &[usize]| -> f64 {
            layer.iter().position(|&n| n == node).unwrap_or(0) as f64
        };

        for _ in 0..iterations {
            // Forward sweep
            for l in 1..layers.len() {
                let prev = layers[l - 1].clone();
                let mut bary: Vec<(f64, usize)> = layers[l]
                    .iter()
                    .map(|&node| {
                        let preds: Vec<f64> = graph.radj[node]
                            .iter()
                            .filter(|&&p| node_layer.get(&p) == Some(&(l - 1)))
                            .map(|&p| position_in_layer(p, &prev))
                            .collect();
                        let bc = if preds.is_empty() {
                            node as f64
                        } else {
                            preds.iter().sum::<f64>() / preds.len() as f64
                        };
                        (bc, node)
                    })
                    .collect();
                bary.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                layers[l] = bary.into_iter().map(|(_, n)| n).collect();
            }

            // Backward sweep
            for l in (0..layers.len() - 1).rev() {
                let next = layers[l + 1].clone();
                let mut bary: Vec<(f64, usize)> = layers[l]
                    .iter()
                    .map(|&node| {
                        let succs: Vec<f64> = graph.adj[node]
                            .iter()
                            .filter(|&&s| node_layer.get(&s) == Some(&(l + 1)))
                            .map(|&s| position_in_layer(s, &next))
                            .collect();
                        let bc = if succs.is_empty() {
                            node as f64
                        } else {
                            succs.iter().sum::<f64>() / succs.len() as f64
                        };
                        (bc, node)
                    })
                    .collect();
                bary.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                layers[l] = bary.into_iter().map(|(_, n)| n).collect();
            }
        }
    }

    // ========================================================================
    // Block-internal template layout
    // ========================================================================

    /// Horizontal footprint of the symbol a device renders as, including the
    /// 15px pin stubs on both sides of a subcircuit box. Builtin symbols are
    /// all ~60px wide; X-instance boxes grow with the display-name length
    /// (see `subckt_box_size`), so side-by-side templates must budget for it.
    fn device_width(dev: &SpiceDevice) -> f64 {
        if dev.device_type == 'X' {
            builtin_symbols::subckt_box_size(&dev.model_or_value, dev.nodes.len()).0 + 30.0
        } else {
            60.0
        }
    }

    /// Vertical counterpart of `device_width`: builtin symbols are ~40px
    /// tall, subckt boxes grow with port count (5 ports → 70, 7 → 90),
    /// so stacked layouts must budget for it.
    fn device_height(dev: &SpiceDevice) -> f64 {
        if dev.device_type == 'X' {
            builtin_symbols::subckt_box_size(&dev.model_or_value, dev.nodes.len()).1
        } else {
            40.0
        }
    }

    /// Per-device symbol bounding rects, the SAME geometry the overlap
    /// metric checks (builtin defs; synthesized numbered-pin boxes for X
    /// instances). Collision guards in the alignment passes must use the
    /// metric's exact footprints — a coarser estimate either lets real
    /// overlaps through or vetoes moves the metric would accept (which is
    /// how the first version of the matched-pair guard broke case 06's
    /// R1/R2 symmetry alignment).
    fn device_bounding_rects(devices: &[SpiceDevice]) -> Vec<crate::model::Rect> {
        let builtin = builtin_symbols::all();
        let mut box_cache: HashMap<String, crate::model::Rect> = HashMap::new();
        devices
            .iter()
            .map(|dev| {
                if dev.device_type == 'X' {
                    *box_cache
                        .entry(dev.model_or_value.clone())
                        .or_insert_with(|| {
                            let ports: Vec<String> =
                                (1..=dev.nodes.len()).map(|i| i.to_string()).collect();
                            builtin_symbols::create_subcircuit_symbol(&dev.model_or_value, &ports)
                                .bounding_rect()
                        })
                } else {
                    builtin
                        .get(&Self::symbol_for_device(dev))
                        .map(|s| s.bounding_rect())
                        .unwrap_or(crate::model::Rect::new(-30.0, -20.0, 60.0, 40.0))
                }
            })
            .collect()
    }

    /// Two placed footprints collide when their world rects overlap by
    /// more than the overlap metric's 1px touching margin.
    fn rects_collide(
        device_rects: &[crate::model::Rect],
        di_a: usize,
        pa: Point,
        di_b: usize,
        pb: Point,
    ) -> bool {
        let (ra, rb) = (device_rects[di_a], device_rects[di_b]);
        let (al, ar) = (pa.x + ra.left(), pa.x + ra.right());
        let (at, ab) = (pa.y + ra.top(), pa.y + ra.bottom());
        let (bl, br) = (pb.x + rb.left(), pb.x + rb.right());
        let (bt, bb) = (pb.y + rb.top(), pb.y + rb.bottom());
        let margin = 1.0;
        al + margin < br && bl + margin < ar && at + margin < bb && bt + margin < ab
    }

    fn layout_block(
        block: &FunctionalBlock,
        all_devices: &[SpiceDevice],
        opts: &PlacerOptions,
    ) -> InternalLayout {
        let sp = opts.intra_block_spacing;
        let devices = &block.device_indices;

        // Width lookup with a safe fallback for the device-less `place()`
        // path (all_devices empty → every width is the builtin 60).
        let dev_w = |di: usize| -> f64 {
            if di < all_devices.len() {
                Self::device_width(&all_devices[di])
            } else {
                60.0
            }
        };
        // Center-to-center pitch for two devices sitting side by side:
        // at least the configured spacing, and at least wide enough that
        // the two footprints don't touch (10px clearance).
        let pair_pitch = |a: usize, b: usize| -> f64 { sp.max((dev_w(a) + dev_w(b)) / 2.0 + 10.0) };

        match block.block_type {
            BlockType::DiffPair => {
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    let pitch = pair_pitch(devices[0], devices[1]);
                    let w = pitch + dev_w(devices[0]).max(dev_w(devices[1]));
                    placements.push((
                        devices[0],
                        String::new(),
                        Point::new(-pitch / 2.0, 0.0),
                        0,
                        false,
                    ));
                    placements.push((
                        devices[1],
                        String::new(),
                        Point::new(pitch / 2.0, 0.0),
                        0,
                        false,
                    ));
                    if devices.len() >= 3 {
                        placements.push((devices[2], String::new(), Point::new(0.0, sp), 0, false));
                        return InternalLayout {
                            placements,
                            width: w,
                            height: sp + 40.0,
                        };
                    }
                    return InternalLayout {
                        placements,
                        width: w,
                        height: 40.0,
                    };
                }
                InternalLayout {
                    placements,
                    width: 60.0,
                    height: 40.0,
                }
            }
            BlockType::CurrentMirror => {
                let mut placements = Vec::new();
                let mut x = 0.0;
                for (i, &idx) in devices.iter().enumerate() {
                    if i > 0 {
                        x += pair_pitch(devices[i - 1], idx);
                    }
                    placements.push((idx, String::new(), Point::new(x, 0.0), 0, false));
                }
                let w = if devices.len() > 1 {
                    x + dev_w(devices[0]).max(dev_w(*devices.last().unwrap()))
                } else {
                    devices.first().map(|&d| dev_w(d)).unwrap_or(60.0)
                };
                InternalLayout {
                    placements,
                    width: w,
                    height: 40.0,
                }
            }
            BlockType::CascodePair => {
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    placements.push((devices[0], String::new(), Point::new(0.0, 0.0), 0, false));
                    placements.push((devices[1], String::new(), Point::new(0.0, sp), 0, false));
                }
                InternalLayout {
                    placements,
                    width: 60.0,
                    height: sp + 40.0,
                }
            }
            BlockType::Inverter => {
                // PMOS first (mirrored), NMOS second
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    placements.push((devices[0], String::new(), Point::new(0.0, 0.0), 0, true)); // PMOS mirrored
                    placements.push((devices[1], String::new(), Point::new(0.0, sp), 0, false));
                }
                InternalLayout {
                    placements,
                    width: 60.0,
                    height: sp + 40.0,
                }
            }
            BlockType::SingleDevice => {
                let placements = vec![(devices[0], String::new(), Point::new(0.0, 0.0), 0, false)];
                InternalLayout {
                    placements,
                    width: 60.0,
                    height: 40.0,
                }
            }
            BlockType::Unknown => {
                // Group block members by match key. Members whose key appears
                // exactly twice are placed side-by-side at the same y so the
                // symmetry metric sees them as aligned. Singletons (and any
                // key with 3+ members) fall back to the vertical stack.
                //
                // Without device info, skip the grouping and use the original
                // pure-vertical layout.
                if all_devices.is_empty() {
                    let mut placements = Vec::new();
                    let mut y = 0.0;
                    for &idx in devices {
                        placements.push((idx, String::new(), Point::new(0.0, y), 0, false));
                        y += sp;
                    }
                    let h = if devices.len() > 1 {
                        (devices.len() - 1) as f64 * sp + 40.0
                    } else {
                        40.0
                    };
                    return InternalLayout {
                        placements,
                        width: 60.0,
                        height: h,
                    };
                }

                // First pass: compute a match key per device, preserving the
                // first-seen order so the layout is deterministic.
                let mut keys: Vec<String> = Vec::with_capacity(devices.len());
                let mut key_groups: HashMap<String, Vec<usize>> = HashMap::new();
                let mut order: Vec<String> = Vec::new();
                for &di in devices {
                    let key = if di < all_devices.len() {
                        Self::device_match_key(&all_devices[di])
                    } else {
                        format!("_idx{}", di) // unique → never matches
                    };
                    if !key_groups.contains_key(&key) {
                        order.push(key.clone());
                    }
                    key_groups.entry(key.clone()).or_default().push(di);
                    keys.push(key);
                }

                // Reorder keys so PMOS-only groups come first (top of the
                // block), then passives / mixed, then NMOS-only groups
                // (bottom). This is the within-block analogue of
                // sort_blocks_by_polarity (Phase 2.3) and fixes Bug 1 in
                // the 2026-05-01 expansion findings: when HAC clusters a
                // common-source amp's NMOS and PMOS into the same Unknown
                // block (circuit 13_pdk_mos_model_names), the original
                // device-order placed NMOS above PMOS, violating the
                // PMOS-above-NMOS convention. Stable-sort on the polarity
                // class preserves the original key order within each
                // class.
                let polarity_of_key = |key: &String| -> u8 {
                    let mut has_p = false;
                    let mut has_n = false;
                    for &di in &key_groups[key] {
                        if di >= all_devices.len() {
                            continue;
                        }
                        let sym = Self::polarity_symbol(&all_devices[di]);
                        match sym.as_str() {
                            "pmos4" | "pnp" => has_p = true,
                            "nmos4" | "npn" => has_n = true,
                            _ => {}
                        }
                    }
                    match (has_p, has_n) {
                        (true, false) => 0,
                        (false, true) => 2,
                        _ => 1,
                    }
                };
                order.sort_by_key(|k| polarity_of_key(k));

                // Collect rows first (a matched pair shares one row), then
                // place them with a vertical pitch that budgets the tallest
                // footprint in each row — subckt boxes with many ports are
                // taller than the 40px a fixed `sp` step assumed (case 29:
                // 7-port srlatch_r boxes are 90px tall at an 80px pitch).
                let mut rows: Vec<Vec<usize>> = Vec::new();
                let mut emitted: HashSet<usize> = HashSet::new();
                for key in &order {
                    let group = &key_groups[key];
                    if group.len() == 2 {
                        rows.push(group.clone());
                        emitted.insert(group[0]);
                        emitted.insert(group[1]);
                    } else {
                        // Singletons and 3+-groups: one row each, original order.
                        for &di in group {
                            rows.push(vec![di]);
                            emitted.insert(di);
                        }
                    }
                }
                // Safety: any device not yet emitted (shouldn't happen) stacks at the end.
                for &di in devices {
                    if !emitted.contains(&di) {
                        rows.push(vec![di]);
                    }
                }

                let dev_h = |di: usize| -> f64 {
                    if di < all_devices.len() {
                        Self::device_height(&all_devices[di])
                    } else {
                        40.0
                    }
                };
                let row_h = |row: &Vec<usize>| -> f64 {
                    row.iter().map(|&di| dev_h(di)).fold(40.0, f64::max)
                };

                let mut placements = Vec::new();
                let mut y = 0.0;
                let mut max_width: f64 = 60.0;
                let mut prev_h: Option<f64> = None;
                for row in &rows {
                    let h = row_h(row);
                    if let Some(ph) = prev_h {
                        y += sp.max((ph + h) / 2.0 + 10.0);
                    }
                    if row.len() == 2 {
                        // Matched pair → horizontal split
                        let pitch = pair_pitch(row[0], row[1]);
                        placements.push((
                            row[0],
                            String::new(),
                            Point::new(-pitch / 2.0, y),
                            0,
                            false,
                        ));
                        placements.push((
                            row[1],
                            String::new(),
                            Point::new(pitch / 2.0, y),
                            0,
                            false,
                        ));
                        max_width = max_width.max(pitch + dev_w(row[0]).max(dev_w(row[1])));
                    } else {
                        placements.push((row[0], String::new(), Point::new(0.0, y), 0, false));
                        max_width = max_width.max(dev_w(row[0]));
                    }
                    prev_h = Some(h);
                }

                let h = y + prev_h.unwrap_or(40.0) / 2.0 + 20.0;
                InternalLayout {
                    placements,
                    width: max_width,
                    height: h,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{CircuitAnalyzer, ClusterOptions};
    use crate::parser::SpiceParser;

    fn parse_devices(spice: &str) -> Vec<SpiceDevice> {
        SpiceParser::new().parse(spice).devices
    }

    #[test]
    fn symbol_for_device_maps_passive_letters() {
        let devices = parse_devices(
            "* mix\n\
             R1 a b 1k\n\
             C1 a b 1u\n\
             L1 a b 1m\n\
             D1 a b dmod\n",
        );
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[0]), "resistor");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[1]), "capacitor");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[2]), "inductor");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[3]), "diode");
    }

    #[test]
    fn symbol_for_device_distinguishes_nmos_pmos() {
        let devices = parse_devices(
            "* MOS\n\
             M1 d g s b nch W=1u L=1u\n\
             M2 d g s b pch W=1u L=1u\n",
        );
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[0]), "nmos4");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[1]), "pmos4");
    }

    #[test]
    fn symbol_for_device_handles_controlled_sources() {
        let devices = parse_devices(
            "* controlled\n\
             E1 o 0 a b 2\n\
             G1 o 0 a b 1m\n\
             H1 o 0 V1 500\n\
             F1 o 0 V1 10\n\
             V1 a b 0\n",
        );
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[0]), "vcvs");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[1]), "vccs");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[2]), "ccvs");
        assert_eq!(SchematicPlacer::symbol_for_device(&devices[3]), "cccs");
    }

    #[test]
    fn symbol_for_device_subcircuit_uses_model_name() {
        let devices = parse_devices(
            "* subckt\n\
             X1 a b VDD VSS INV\n",
        );
        assert_eq!(
            SchematicPlacer::symbol_for_device(&devices[0]),
            "subckt_INV"
        );
    }

    #[test]
    fn place_empty_blocks_returns_empty_result() {
        let placer = SchematicPlacer;
        let r = placer.place(&[], &HashSet::new(), &PlacerOptions::default());
        assert!(r.placements.is_empty());
    }

    #[test]
    fn place_inverter_yields_two_placements_with_pmos_above_nmos() {
        // Phase 2.3 invariant: PMOS sits above NMOS.
        // Note: the Inverter template stores symbol_name="" — the router
        // resolves it later via symbol_for_device. So look up by device_index.
        let pr = SpiceParser::new().parse(
            "* CMOS Inverter\n\
             M1 out in vdd vdd pch W=20u L=1u\n\
             M2 out in 0 0 nch W=10u L=1u\n",
        );
        let analyzer = CircuitAnalyzer::new();
        let power_nets = analyzer.identify_power_nets(&pr.devices);
        let blocks = analyzer.analyze(&pr.devices, &ClusterOptions::default());

        let placer = SchematicPlacer;
        let result =
            placer.place_with_devices(&blocks, &power_nets, &PlacerOptions::default(), &pr.devices);
        assert_eq!(result.placements.len(), 2);

        let mut pmos_y = None;
        let mut nmos_y = None;
        for dp in &result.placements {
            let sym = if dp.symbol_name.is_empty() {
                SchematicPlacer::symbol_for_device(&pr.devices[dp.device_index])
            } else {
                dp.symbol_name.clone()
            };
            match sym.as_str() {
                "pmos4" => pmos_y = Some(dp.position.y),
                "nmos4" => nmos_y = Some(dp.position.y),
                _ => {}
            }
        }
        let pmos_y = pmos_y.expect("PMOS missing");
        let nmos_y = nmos_y.expect("NMOS missing");
        assert!(
            pmos_y < nmos_y,
            "PMOS y ({}) should be < NMOS y ({})",
            pmos_y,
            nmos_y
        );
    }

    #[test]
    fn place_snaps_to_grid() {
        let pr = SpiceParser::new().parse(
            "* RC\n\
             V1 in 0 1\n\
             R1 in out 1k\n\
             C1 out 0 1u\n",
        );
        let analyzer = CircuitAnalyzer::new();
        let power_nets = analyzer.identify_power_nets(&pr.devices);
        let blocks = analyzer.analyze(&pr.devices, &ClusterOptions::default());

        let opts = PlacerOptions {
            grid_size: 10.0,
            ..Default::default()
        };
        let placer = SchematicPlacer;
        let result = placer.place_with_devices(&blocks, &power_nets, &opts, &pr.devices);
        for dp in &result.placements {
            assert_eq!(
                dp.position.x,
                (dp.position.x / 10.0).round() * 10.0,
                "x not on grid: {}",
                dp.position.x
            );
            assert_eq!(
                dp.position.y,
                (dp.position.y / 10.0).round() * 10.0,
                "y not on grid: {}",
                dp.position.y
            );
        }
    }
}
