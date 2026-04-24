use std::collections::{HashMap, HashSet, VecDeque};
use crate::parser::{SpiceDevice, SpiceParser};
use crate::analyzer::{FunctionalBlock, BlockType};
use crate::model::Point;

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

    pub fn place(
        &self, blocks: &[FunctionalBlock], power_nets: &HashSet<String>, opts: &PlacerOptions,
    ) -> PlacementResult {
        self.place_with_devices(blocks, power_nets, opts, &[])
    }

    pub fn place_with_devices(
        &self, blocks: &[FunctionalBlock], power_nets: &HashSet<String>, opts: &PlacerOptions,
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

        // 3. Crossing minimization
        Self::minimize_crossings(&mut layers, &graph, 4);

        // 3.5. Sort blocks within each layer: PMOS-containing above NMOS-containing
        if !devices.is_empty() {
            Self::sort_blocks_by_polarity(&mut layers, blocks, devices);
        }

        // 4. Block-internal layouts
        let block_layouts: Vec<InternalLayout> = blocks.iter()
            .map(|b| Self::layout_block(b, opts))
            .collect();

        // 5. Absolute coordinates
        //    When a layer has many blocks, arrange them in a grid to avoid
        //    extreme vertical aspect ratios. The target is roughly square.
        let mut placements = Vec::new();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        // Compute column widths for multi-column layers so subsequent layers
        // are offset correctly.
        let mut layer_x_start: Vec<f64> = Vec::new();
        let mut x_cursor = 0.0;
        for layer in &layers {
            layer_x_start.push(x_cursor);
            let cols = Self::compute_grid_columns(layer, &block_layouts, opts);
            // Advance x_cursor by the total width of this layer's grid
            let mut max_col_width = 0.0f64;
            for col_blocks in &cols {
                let col_w = col_blocks.iter()
                    .map(|&&bi| block_layouts[bi].width)
                    .fold(0.0f64, f64::max);
                max_col_width += col_w + opts.layer_spacing;
            }
            // Use at least one layer_spacing worth of width
            x_cursor += max_col_width.max(opts.layer_spacing);
        }

        for (l, layer) in layers.iter().enumerate() {
            let base_x = layer_x_start[l];
            let cols = Self::compute_grid_columns(layer, &block_layouts, opts);

            let mut col_x = base_x;
            for col_blocks in &cols {
                let mut y_cursor = 0.0;
                let col_width = col_blocks.iter()
                    .map(|&&bi| block_layouts[bi].width)
                    .fold(0.0f64, f64::max);

                for &&block_idx in col_blocks {
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

        // 6. Align matched device pairs (symmetry improvement)
        if !devices.is_empty() {
            Self::align_matched_pairs(&mut placements, blocks, devices, opts);
            // Recompute bounding rect after alignment
            min_x = f64::MAX; min_y = f64::MAX;
            max_x = f64::MIN; max_y = f64::MIN;
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

        // Group devices by matching key: (symbol_name, sorted properties)
        let mut match_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (di, dev) in devices.iter().enumerate() {
            let sym_name = Self::symbol_for_device(dev);
            let mut key_parts = vec![sym_name];
            let mut props: Vec<(&str, &str)> = dev.parameters.iter()
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
            let key = key_parts.join("|");
            match_groups.entry(key).or_default().push(di);
        }

        // For groups of exactly 2, align y-coordinates if in different blocks
        for group in match_groups.values() {
            if group.len() != 2 { continue; }

            let di_a = group[0];
            let di_b = group[1];

            let block_a = match dev_to_block.get(&di_a) { Some(&b) => b, None => continue };
            let block_b = match dev_to_block.get(&di_b) { Some(&b) => b, None => continue };

            // Skip if same block (already handled by block template)
            if block_a == block_b { continue; }

            let pi_a = match dev_to_placement.get(&di_a) { Some(&p) => p, None => continue };
            let pi_b = match dev_to_placement.get(&di_b) { Some(&p) => p, None => continue };

            let y_a = placements[pi_a].position.y;
            let y_b = placements[pi_b].position.y;
            let y_diff = (y_a - y_b).abs();

            // Only align if they're at meaningfully different y positions
            if y_diff < opts.grid_size { continue; }

            // Determine which block to move (prefer moving the smaller block)
            let size_a = blocks[block_a].device_indices.len();
            let size_b = blocks[block_b].device_indices.len();

            let (move_block, anchor_di, move_di) = if size_a <= size_b {
                (block_a, di_b, di_a)
            } else {
                (block_b, di_a, di_b)
            };

            let anchor_pi = dev_to_placement[&anchor_di];
            let move_pi = dev_to_placement[&move_di];

            // Compute the delta to align the moving device with the anchor's y
            let dy = placements[anchor_pi].position.y - placements[move_pi].position.y;

            if dy.abs() < opts.grid_size { continue; }

            // Shift all devices in the moving block by dy
            for &di in &blocks[move_block].device_indices {
                if let Some(&pi) = dev_to_placement.get(&di) {
                    placements[pi].position.y += dy;
                    placements[pi].position = placements[pi].position.snap_to_grid(opts.grid_size);
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
        // Classify each block's polarity
        // 0 = PMOS (top), 1 = mixed/neutral (middle), 2 = NMOS (bottom)
        let classify_block = |block: &FunctionalBlock| -> u8 {
            let mut has_pmos = false;
            let mut has_nmos = false;
            for &di in &block.device_indices {
                if di < devices.len() {
                    let sym = Self::symbol_for_device(&devices[di]);
                    match sym.as_str() {
                        "pmos4" | "pnp" => has_pmos = true,
                        "nmos4" | "npn" => has_nmos = true,
                        _ => {}
                    }
                }
            }
            match (has_pmos, has_nmos) {
                (true, false) => 0,  // PMOS → top
                (false, true) => 2,  // NMOS → bottom
                _ => 1,              // mixed or passive → middle
            }
        };

        for layer in layers.iter_mut() {
            if layer.len() <= 1 { continue; }
            // Stable sort preserves crossing-minimized order within same polarity
            layer.sort_by_key(|&bi| classify_block(&blocks[bi]));
        }
    }

    // ========================================================================
    // Multi-column grid layout for large layers
    // ========================================================================

    /// When a layer has many blocks, split them into multiple columns to
    /// achieve a roughly square aspect ratio instead of an extreme vertical strip.
    ///
    /// Returns a vector of columns, each column being a slice of block indices.
    fn compute_grid_columns<'a>(
        layer: &'a [usize],
        block_layouts: &[InternalLayout],
        opts: &PlacerOptions,
    ) -> Vec<Vec<&'a usize>> {
        if layer.len() <= 2 {
            // 1-2 blocks: single column is fine
            return vec![layer.iter().collect()];
        }

        // Compute total height if all blocks were in one column
        let total_height: f64 = layer.iter()
            .map(|&bi| block_layouts[bi].height + opts.inter_block_spacing)
            .sum();

        // Compute max block width (gives us the column width)
        let max_block_width: f64 = layer.iter()
            .map(|&bi| block_layouts[bi].width)
            .fold(60.0f64, f64::max);

        // Target: aspect ratio close to 1.5 (slightly wider than tall).
        // num_cols = ceil(sqrt(total_height / (target_ratio * col_width)))
        let target_ratio = 1.5;
        let ideal_cols = (total_height / (target_ratio * (max_block_width + opts.layer_spacing))).sqrt();
        let num_cols = (ideal_cols.ceil() as usize).max(1).min(layer.len());

        if num_cols <= 1 {
            return vec![layer.iter().collect()];
        }

        // Distribute blocks across columns, balancing total height per column.
        // Greedy: assign each block to the column with the smallest current height.
        let mut columns: Vec<Vec<&usize>> = (0..num_cols).map(|_| Vec::new()).collect();
        let mut col_heights: Vec<f64> = vec![0.0; num_cols];

        for bi in layer {
            // Find the column with minimum height
            let min_col = col_heights.iter()
                .enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            columns[min_col].push(bi);
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
                if power_nets.contains(&net.to_lowercase()) { continue; }
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
        enum Color { White, Gray, Black }
        let mut color = vec![Color::White; n];
        let mut back_edges: HashSet<usize> = HashSet::new();

        fn dfs(u: usize, color: &mut [Color], edges: &[(usize, usize)], back: &mut HashSet<usize>) {
            color[u] = Color::Gray;
            for (idx, &(from, to)) in edges.iter().enumerate() {
                if from != u { continue; }
                match color[to] {
                    Color::Gray => { back.insert(idx); }
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

        BlockGraph { node_count: n, adj, radj, edges }
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
        for i in 0..n {
            if in_deg[i] == 0 { queue.push_back(i); }
        }

        let mut topo = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            topo.push(u);
            for &v in &graph.adj[u] {
                in_deg[v] -= 1;
                if in_deg[v] == 0 { queue.push_back(v); }
            }
        }

        // Add isolated nodes
        if topo.len() < n {
            let visited: HashSet<usize> = topo.iter().copied().collect();
            for i in 0..n {
                if !visited.contains(&i) { topo.push(i); }
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
        if n == 0 { return; }
        let max_layer = *layers.iter().max().unwrap_or(&0);
        if max_layer == 0 { return; }

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
                if !has_out[u] { continue; } // sink: stays at max_layer
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
            if !changed { break; }
        }

        // Final layer: for non-isolated blocks, use max(ASAP, ALAP). Since
        // ALAP >= ASAP by construction (ALAP starts at max_layer and only
        // decreases via successor constraints, which are dominated by
        // ASAP+path_length), this effectively pushes blocks to the right.
        //
        // Isolated blocks (no edges) are untouched.
        for i in 0..n {
            if !has_in[i] && !has_out[i] { continue; }
            // Pure signal sources (no in-edges) stay at ASAP to keep inputs
            // anchored on the left.
            if !has_in[i] { continue; }
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
            if has_edges[i] { continue; }

            // Find non-isolated block sharing the most nets
            let mut best_target: Option<usize> = None;
            let mut best_shared = 0usize;

            for j in 0..n {
                if i == j || !has_edges[j] { continue; }
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
        if layers.len() <= 1 { return; }

        let mut node_layer: HashMap<usize, usize> = HashMap::new();
        for (l, layer) in layers.iter().enumerate() {
            for &n in layer { node_layer.insert(n, l); }
        }

        let position_in_layer = |node: usize, layer: &[usize]| -> f64 {
            layer.iter().position(|&n| n == node).unwrap_or(0) as f64
        };

        for _ in 0..iterations {
            // Forward sweep
            for l in 1..layers.len() {
                let prev = layers[l - 1].clone();
                let mut bary: Vec<(f64, usize)> = layers[l].iter().map(|&node| {
                    let preds: Vec<f64> = graph.radj[node].iter()
                        .filter(|&&p| node_layer.get(&p) == Some(&(l - 1)))
                        .map(|&p| position_in_layer(p, &prev))
                        .collect();
                    let bc = if preds.is_empty() { node as f64 } else { preds.iter().sum::<f64>() / preds.len() as f64 };
                    (bc, node)
                }).collect();
                bary.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                layers[l] = bary.into_iter().map(|(_, n)| n).collect();
            }

            // Backward sweep
            for l in (0..layers.len() - 1).rev() {
                let next = layers[l + 1].clone();
                let mut bary: Vec<(f64, usize)> = layers[l].iter().map(|&node| {
                    let succs: Vec<f64> = graph.adj[node].iter()
                        .filter(|&&s| node_layer.get(&s) == Some(&(l + 1)))
                        .map(|&s| position_in_layer(s, &next))
                        .collect();
                    let bc = if succs.is_empty() { node as f64 } else { succs.iter().sum::<f64>() / succs.len() as f64 };
                    (bc, node)
                }).collect();
                bary.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                layers[l] = bary.into_iter().map(|(_, n)| n).collect();
            }
        }
    }

    // ========================================================================
    // Block-internal template layout
    // ========================================================================

    fn layout_block(block: &FunctionalBlock, opts: &PlacerOptions) -> InternalLayout {
        let sp = opts.intra_block_spacing;
        let devices = &block.device_indices;

        match block.block_type {
            BlockType::DiffPair => {
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    placements.push((devices[0], String::new(), Point::new(-sp / 2.0, 0.0), 0, false));
                    placements.push((devices[1], String::new(), Point::new(sp / 2.0, 0.0), 0, false));
                    if devices.len() >= 3 {
                        placements.push((devices[2], String::new(), Point::new(0.0, sp), 0, false));
                        return InternalLayout { placements, width: sp + 60.0, height: sp + 40.0 };
                    }
                    return InternalLayout { placements, width: sp + 60.0, height: 40.0 };
                }
                InternalLayout { placements, width: 60.0, height: 40.0 }
            }
            BlockType::CurrentMirror => {
                let mut placements = Vec::new();
                let mut x = 0.0;
                for &idx in devices {
                    placements.push((idx, String::new(), Point::new(x, 0.0), 0, false));
                    x += sp;
                }
                let w = if devices.len() > 1 { (devices.len() - 1) as f64 * sp + 60.0 } else { 60.0 };
                InternalLayout { placements, width: w, height: 40.0 }
            }
            BlockType::CascodePair => {
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    placements.push((devices[0], String::new(), Point::new(0.0, 0.0), 0, false));
                    placements.push((devices[1], String::new(), Point::new(0.0, sp), 0, false));
                }
                InternalLayout { placements, width: 60.0, height: sp + 40.0 }
            }
            BlockType::Inverter => {
                // PMOS first (mirrored), NMOS second
                let mut placements = Vec::new();
                if devices.len() >= 2 {
                    placements.push((devices[0], String::new(), Point::new(0.0, 0.0), 0, true)); // PMOS mirrored
                    placements.push((devices[1], String::new(), Point::new(0.0, sp), 0, false));
                }
                InternalLayout { placements, width: 60.0, height: sp + 40.0 }
            }
            BlockType::SingleDevice => {
                let placements = vec![(devices[0], String::new(), Point::new(0.0, 0.0), 0, false)];
                InternalLayout { placements, width: 60.0, height: 40.0 }
            }
            BlockType::Unknown => {
                let mut placements = Vec::new();
                let mut y = 0.0;
                for &idx in devices {
                    placements.push((idx, String::new(), Point::new(0.0, y), 0, false));
                    y += sp;
                }
                let h = if devices.len() > 1 { (devices.len() - 1) as f64 * sp + 40.0 } else { 40.0 };
                InternalLayout { placements, width: 60.0, height: h }
            }
        }
    }
}
