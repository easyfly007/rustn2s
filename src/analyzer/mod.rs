use std::collections::{HashMap, HashSet, BTreeMap};
use crate::parser::{SpiceDevice, SpiceParser};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    DiffPair,
    CurrentMirror,
    CascodePair,
    Inverter,
    Unknown,
    SingleDevice,
}

#[derive(Debug, Clone)]
pub struct FunctionalBlock {
    pub block_type: BlockType,
    pub id: String,
    pub device_indices: Vec<usize>,
    pub input_nets: Vec<String>,
    pub output_nets: Vec<String>,
    pub internal_nets: HashSet<String>,
    pub all_nets: HashSet<String>,
}

pub struct ClusterOptions {
    pub merge_threshold: f64,
    pub max_cluster_size: usize,
    pub recognize_patterns: bool,
}

impl Default for ClusterOptions {
    fn default() -> Self {
        Self {
            merge_threshold: 0.5,
            max_cluster_size: 6,
            recognize_patterns: true,
        }
    }
}

pub struct CircuitAnalyzer;

impl CircuitAnalyzer {
    pub fn new() -> Self { Self }

    pub fn analyze(&self, devices: &[SpiceDevice], opts: &ClusterOptions) -> Vec<FunctionalBlock> {
        if devices.is_empty() { return Vec::new(); }

        let power_nets = self.identify_power_nets(devices);
        let mut assigned: HashSet<usize> = HashSet::new();
        let mut blocks = Vec::new();

        // Stage 1: Global pattern extraction
        if opts.recognize_patterns {
            self.find_diff_pairs(devices, &power_nets, &mut assigned, &mut blocks);
            self.find_current_mirrors(devices, &power_nets, &mut assigned, &mut blocks);
            self.find_cascode_pairs(devices, &power_nets, &mut assigned, &mut blocks);
            self.find_inverters(devices, &power_nets, &mut assigned, &mut blocks);
        }

        // Stage 2: Cluster remaining devices
        let remaining: Vec<usize> = (0..devices.len())
            .filter(|i| !assigned.contains(i))
            .collect();

        if !remaining.is_empty() {
            let clusters = self.cluster_devices(devices, &power_nets, opts, &remaining);
            for cluster in clusters {
                let mut sub = self.annotate_cluster(&cluster, devices, &power_nets);
                blocks.append(&mut sub);
            }
        }

        // Assign IDs
        for (i, b) in blocks.iter_mut().enumerate() {
            b.id = format!("block_{}", i);
        }

        blocks
    }

    pub fn identify_power_nets(&self, devices: &[SpiceDevice]) -> HashSet<String> {
        let mut power: HashSet<String> = [
            "0", "gnd", "gnd!", "vss", "vss!", "vdd", "vdd!", "vcc", "vcc!", "avdd", "avss",
        ].iter().map(|s| s.to_string()).collect();

        for dev in devices {
            if dev.device_type == 'V' {
                for node in &dev.nodes {
                    power.insert(node.to_lowercase());
                }
            }
        }
        power
    }

    // ========================================================================
    // Pattern finders
    // ========================================================================

    fn find_diff_pairs(
        &self, devices: &[SpiceDevice], power_nets: &HashSet<String>,
        assigned: &mut HashSet<usize>, blocks: &mut Vec<FunctionalBlock>,
    ) {
        // Group unassigned MOSFETs by (mos_type, source_net)
        let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();

        for (i, dev) in devices.iter().enumerate() {
            if assigned.contains(&i) || dev.device_type != 'M' || dev.nodes.len() < 4 { continue; }
            let mos_type = SpiceParser::infer_mos_type(dev).to_string();
            let source = &dev.nodes[2];
            if power_nets.contains(&source.to_lowercase()) { continue; }
            groups.entry((mos_type, source.clone())).or_default().push(i);
        }

        for group in groups.values() {
            if group.len() < 2 { continue; }
            for a in 0..group.len() {
                if assigned.contains(&group[a]) { continue; }
                for b in (a + 1)..group.len() {
                    if assigned.contains(&group[b]) { continue; }
                    let ma = &devices[group[a]];
                    let mb = &devices[group[b]];
                    // Different gates, different drains
                    if ma.nodes[1] != mb.nodes[1] && ma.nodes[0] != mb.nodes[0] {
                        let mut block = FunctionalBlock {
                            block_type: BlockType::DiffPair,
                            id: String::new(),
                            device_indices: vec![group[a], group[b]],
                            input_nets: vec![ma.nodes[1].clone(), mb.nodes[1].clone()],
                            output_nets: vec![ma.nodes[0].clone(), mb.nodes[0].clone()],
                            internal_nets: [ma.nodes[2].clone()].into(),
                            all_nets: HashSet::new(),
                        };
                        for &idx in &block.device_indices {
                            for n in &devices[idx].nodes { block.all_nets.insert(n.clone()); }
                        }
                        assigned.insert(group[a]);
                        assigned.insert(group[b]);

                        // Try to find tail current source
                        let tail_net = &ma.nodes[2];
                        for (k, dev) in devices.iter().enumerate() {
                            if assigned.contains(&k) { continue; }
                            if dev.device_type == 'M' && !dev.nodes.is_empty() && dev.nodes[0] == *tail_net {
                                block.device_indices.push(k);
                                for n in &dev.nodes { block.all_nets.insert(n.clone()); }
                                assigned.insert(k);
                                break;
                            }
                        }

                        blocks.push(block);
                        break;
                    }
                }
                if assigned.contains(&group[a]) { break; }
            }
        }
    }

    fn find_current_mirrors(
        &self, devices: &[SpiceDevice], _power_nets: &HashSet<String>,
        assigned: &mut HashSet<usize>, blocks: &mut Vec<FunctionalBlock>,
    ) {
        // Group by (mos_type, gate_net, source_net)
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, dev) in devices.iter().enumerate() {
            if assigned.contains(&i) || dev.device_type != 'M' || dev.nodes.len() < 3 { continue; }
            let key = format!("{}|{}|{}", SpiceParser::infer_mos_type(dev), dev.nodes[1], dev.nodes[2]);
            groups.entry(key).or_default().push(i);
        }

        for group in groups.values() {
            if group.len() < 2 { continue; }
            // Need at least one diode-connected device
            let has_diode = group.iter().any(|&idx| devices[idx].nodes[0] == devices[idx].nodes[1]);
            if !has_diode { continue; }

            let mut block = FunctionalBlock {
                block_type: BlockType::CurrentMirror,
                id: String::new(),
                device_indices: Vec::new(),
                input_nets: Vec::new(),
                output_nets: Vec::new(),
                internal_nets: [devices[group[0]].nodes[1].clone()].into(),
                all_nets: HashSet::new(),
            };

            for &idx in group {
                block.device_indices.push(idx);
                for n in &devices[idx].nodes { block.all_nets.insert(n.clone()); }
                assigned.insert(idx);

                if devices[idx].nodes[0] == devices[idx].nodes[1] {
                    if !block.input_nets.contains(&devices[idx].nodes[0]) {
                        block.input_nets.push(devices[idx].nodes[0].clone());
                    }
                } else if !block.output_nets.contains(&devices[idx].nodes[0]) {
                    block.output_nets.push(devices[idx].nodes[0].clone());
                }
            }

            blocks.push(block);
        }
    }

    fn find_cascode_pairs(
        &self, devices: &[SpiceDevice], _power_nets: &HashSet<String>,
        assigned: &mut HashSet<usize>, blocks: &mut Vec<FunctionalBlock>,
    ) {
        for i in 0..devices.len() {
            if assigned.contains(&i) || devices[i].device_type != 'M' || devices[i].nodes.len() < 4 {
                continue;
            }
            for j in 0..devices.len() {
                if i == j || assigned.contains(&j) || devices[j].device_type != 'M' || devices[j].nodes.len() < 4 {
                    continue;
                }
                if SpiceParser::infer_mos_type(&devices[i]) != SpiceParser::infer_mos_type(&devices[j]) {
                    continue;
                }
                // di.source == dj.drain (di is upper, dj is lower)
                if devices[i].nodes[2] == devices[j].nodes[0] && devices[i].nodes[1] != devices[j].nodes[1] {
                    let mut block = FunctionalBlock {
                        block_type: BlockType::CascodePair,
                        id: String::new(),
                        device_indices: vec![i, j],
                        input_nets: vec![devices[j].nodes[1].clone(), devices[i].nodes[1].clone()],
                        output_nets: vec![devices[i].nodes[0].clone()],
                        internal_nets: [devices[i].nodes[2].clone()].into(),
                        all_nets: HashSet::new(),
                    };
                    for &idx in &block.device_indices {
                        for n in &devices[idx].nodes { block.all_nets.insert(n.clone()); }
                    }
                    assigned.insert(i);
                    assigned.insert(j);
                    blocks.push(block);
                    break;
                }
            }
        }
    }

    fn find_inverters(
        &self, devices: &[SpiceDevice], power_nets: &HashSet<String>,
        assigned: &mut HashSet<usize>, blocks: &mut Vec<FunctionalBlock>,
    ) {
        let nmos_idx: Vec<usize> = (0..devices.len())
            .filter(|&i| !assigned.contains(&i) && devices[i].device_type == 'M'
                && devices[i].nodes.len() >= 3
                && SpiceParser::infer_mos_type(&devices[i]) == "nmos4")
            .collect();
        let pmos_idx: Vec<usize> = (0..devices.len())
            .filter(|&i| !assigned.contains(&i) && devices[i].device_type == 'M'
                && devices[i].nodes.len() >= 3
                && SpiceParser::infer_mos_type(&devices[i]) == "pmos4")
            .collect();

        for &ni in &nmos_idx {
            if assigned.contains(&ni) { continue; }
            for &pi in &pmos_idx {
                if assigned.contains(&pi) { continue; }
                let mn = &devices[ni];
                let mp = &devices[pi];
                if mn.nodes[1] == mp.nodes[1]    // same gate
                    && mn.nodes[0] == mp.nodes[0] // same drain
                    && power_nets.contains(&mn.nodes[2].to_lowercase())
                    && power_nets.contains(&mp.nodes[2].to_lowercase())
                {
                    let mut block = FunctionalBlock {
                        block_type: BlockType::Inverter,
                        id: String::new(),
                        device_indices: vec![pi, ni], // PMOS first
                        input_nets: vec![mn.nodes[1].clone()],
                        output_nets: vec![mn.nodes[0].clone()],
                        internal_nets: HashSet::new(),
                        all_nets: HashSet::new(),
                    };
                    for &idx in &block.device_indices {
                        for n in &devices[idx].nodes { block.all_nets.insert(n.clone()); }
                    }
                    assigned.insert(ni);
                    assigned.insert(pi);
                    blocks.push(block);
                    break;
                }
            }
        }
    }

    // ========================================================================
    // HAC clustering
    // ========================================================================

    fn cluster_devices(
        &self, devices: &[SpiceDevice], power_nets: &HashSet<String>,
        opts: &ClusterOptions, device_indices: &[usize],
    ) -> Vec<Vec<usize>> {
        let n = device_indices.len();
        if n == 0 { return Vec::new(); }

        // Build net → local indices
        let mut net_to_local: HashMap<String, Vec<usize>> = HashMap::new();
        for (li, &gi) in device_indices.iter().enumerate() {
            for node in &devices[gi].nodes {
                let lower = node.to_lowercase();
                if !power_nets.contains(&lower) {
                    net_to_local.entry(lower).or_default().push(li);
                }
            }
        }

        // Build adjacency
        let mut adjacency: HashMap<(usize, usize), i32> = HashMap::new();
        for locals in net_to_local.values() {
            for a in 0..locals.len() {
                for b in (a + 1)..locals.len() {
                    let lo = locals[a].min(locals[b]);
                    let hi = locals[a].max(locals[b]);
                    *adjacency.entry((lo, hi)).or_insert(0) += 1;
                }
            }
        }

        // Initialize clusters
        let mut cluster_members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..n {
            cluster_members.insert(i, vec![i]);
        }

        // Iteratively merge
        loop {
            if cluster_members.len() <= 1 { break; }

            let active: Vec<usize> = cluster_members.keys().copied().collect();
            let mut best_a = 0usize;
            let mut best_b = 0usize;
            let mut best_score = 0.0f64;
            let mut found = false;

            for ci in 0..active.len() {
                for cj in (ci + 1)..active.len() {
                    let id_a = active[ci];
                    let id_b = active[cj];
                    let members_a = &cluster_members[&id_a];
                    let members_b = &cluster_members[&id_b];

                    let mut total_weight = 0i32;
                    for &da in members_a {
                        for &db in members_b {
                            let lo = da.min(db);
                            let hi = da.max(db);
                            total_weight += adjacency.get(&(lo, hi)).copied().unwrap_or(0);
                        }
                    }
                    if total_weight == 0 { continue; }

                    let score = total_weight as f64 / members_a.len().min(members_b.len()) as f64;
                    if score > best_score {
                        best_score = score;
                        best_a = id_a;
                        best_b = id_b;
                        found = true;
                    }
                }
            }

            if !found || best_score < opts.merge_threshold { break; }

            let merged_size = cluster_members[&best_a].len() + cluster_members[&best_b].len();
            if merged_size > opts.max_cluster_size { break; }

            let members_b = cluster_members.remove(&best_b).unwrap();
            cluster_members.get_mut(&best_a).unwrap().extend(members_b);
        }

        // Convert local → global indices
        let mut result: Vec<Vec<usize>> = cluster_members
            .values()
            .map(|locals| locals.iter().map(|&li| device_indices[li]).collect())
            .collect();
        result.sort_by_key(|c| c[0]);
        result
    }

    fn annotate_cluster(
        &self, cluster: &[usize], devices: &[SpiceDevice], power_nets: &HashSet<String>,
    ) -> Vec<FunctionalBlock> {
        let mut all_nets = HashSet::new();
        for &idx in cluster {
            for n in &devices[idx].nodes { all_nets.insert(n.clone()); }
        }

        if cluster.len() == 1 {
            let mut block = FunctionalBlock {
                block_type: BlockType::SingleDevice,
                id: String::new(),
                device_indices: cluster.to_vec(),
                input_nets: Vec::new(),
                output_nets: Vec::new(),
                internal_nets: HashSet::new(),
                all_nets,
            };
            self.infer_single_device_io(&mut block, devices, power_nets);
            return vec![block];
        }

        let mut block = FunctionalBlock {
            block_type: BlockType::Unknown,
            id: String::new(),
            device_indices: cluster.to_vec(),
            input_nets: Vec::new(),
            output_nets: Vec::new(),
            internal_nets: HashSet::new(),
            all_nets,
        };
        self.infer_cluster_io(&mut block, devices, power_nets);
        vec![block]
    }

    fn infer_single_device_io(
        &self, block: &mut FunctionalBlock, devices: &[SpiceDevice], power_nets: &HashSet<String>,
    ) {
        let dev = &devices[block.device_indices[0]];
        match dev.device_type {
            'M' | 'Q' if dev.nodes.len() >= 3 => {
                if !power_nets.contains(&dev.nodes[1].to_lowercase()) {
                    block.input_nets.push(dev.nodes[1].clone());
                }
                if !power_nets.contains(&dev.nodes[0].to_lowercase()) {
                    block.output_nets.push(dev.nodes[0].clone());
                }
            }
            'E' | 'G' | 'H' | 'F' if dev.nodes.len() >= 4 => {
                block.input_nets = vec![dev.nodes[2].clone(), dev.nodes[3].clone()];
                block.output_nets = vec![dev.nodes[0].clone(), dev.nodes[1].clone()];
            }
            // Voltage/current sources: always include the positive terminal
            // (node[0]) as an output so the source block connects in the DAG
            // to the blocks it drives, even when the terminal is a power net.
            'V' | 'I' if dev.nodes.len() >= 2 => {
                // Positive terminal is always an output (the supply it provides)
                block.output_nets.push(dev.nodes[0].clone());
                // Negative terminal is an input only if it's not a standard ground
                let node1_lower = dev.nodes[1].to_lowercase();
                let is_ground = matches!(node1_lower.as_str(),
                    "0" | "gnd" | "gnd!" | "vss" | "vss!" | "avss");
                if !is_ground {
                    block.input_nets.push(dev.nodes[1].clone());
                }
            }
            _ if dev.nodes.len() >= 2 => {
                if !power_nets.contains(&dev.nodes[0].to_lowercase()) {
                    block.input_nets.push(dev.nodes[0].clone());
                }
                if !power_nets.contains(&dev.nodes[1].to_lowercase()) {
                    block.output_nets.push(dev.nodes[1].clone());
                }
            }
            _ => {}
        }
    }

    fn infer_cluster_io(
        &self, block: &mut FunctionalBlock, devices: &[SpiceDevice], power_nets: &HashSet<String>,
    ) {
        let cluster_names: HashSet<&str> = block.device_indices.iter()
            .map(|&i| devices[i].instance_name.as_str())
            .collect();

        let cluster_nets: HashSet<String> = block.device_indices.iter()
            .flat_map(|&i| devices[i].nodes.iter().cloned())
            .collect();

        let mut external_nets = HashSet::new();
        let mut internal_only = HashSet::new();

        for net in &cluster_nets {
            if power_nets.contains(&net.to_lowercase()) { continue; }
            let used_externally = devices.iter()
                .any(|d| !cluster_names.contains(d.instance_name.as_str()) && d.nodes.contains(net));
            if used_externally {
                external_nets.insert(net.clone());
            } else {
                internal_only.insert(net.clone());
            }
        }

        block.internal_nets = internal_only;

        for net in &external_nets {
            let mut is_input = false;
            let mut is_output = false;
            for &idx in &block.device_indices {
                let dev = &devices[idx];
                if (dev.device_type == 'M' || dev.device_type == 'Q') && dev.nodes.len() >= 3 {
                    if dev.nodes[1] == *net { is_input = true; }
                    if dev.nodes[0] == *net { is_output = true; }
                }
            }
            if is_input && !block.input_nets.contains(net) { block.input_nets.push(net.clone()); }
            if is_output && !block.output_nets.contains(net) { block.output_nets.push(net.clone()); }
            if !is_input && !is_output && !block.output_nets.contains(net) {
                block.output_nets.push(net.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SpiceParser;

    #[test]
    fn test_inverter_detection() {
        let spice = "* CMOS Inverter\nM1 out in vdd vdd pch W=20u L=1u\nM2 out in 0 0 nch W=10u L=1u\n";
        let mut parser = SpiceParser::new();
        let pr = parser.parse(spice);
        let analyzer = CircuitAnalyzer::new();
        let blocks = analyzer.analyze(&pr.devices, &ClusterOptions::default());
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Inverter);
    }

    #[test]
    fn test_diff_pair_detection() {
        let spice = "* Diff Pair\nM1 out1 inp tail 0 nch W=10u L=1u\nM2 out2 inm tail 0 nch W=10u L=1u\nM3 tail bias 0 0 nch W=20u L=2u\n";
        let mut parser = SpiceParser::new();
        let pr = parser.parse(spice);
        let analyzer = CircuitAnalyzer::new();
        let blocks = analyzer.analyze(&pr.devices, &ClusterOptions::default());
        let dp = blocks.iter().find(|b| b.block_type == BlockType::DiffPair);
        assert!(dp.is_some());
        // Diff pair should include M3 as tail
        assert_eq!(dp.unwrap().device_indices.len(), 3);
    }
}
