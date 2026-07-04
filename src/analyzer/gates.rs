//! CMOS gate extraction — scale Phase 1 (docs/scale_placement.md).
//!
//! Flat transistor-level digital netlists (case 36: 684 FETs) defeat both
//! the analog pattern matchers and shared-net HAC; the unit of structure
//! is the CMOS gate. This module recognizes simple static gates by
//! channel-graph template matching and (in the scale regime) collapses
//! each match into ONE synthetic X device rendered as a labeled box, so
//! the existing Sugiyama pipeline runs at gate granularity.
//!
//! Deliberately conservative:
//! - templates: INV, NAND2/3, NOR2/3 only (v1; no TG, no AOI/OAI);
//! - drain/source are treated symmetrically (extraction netlists like
//!   case 36 do not respect the D-before-S convention);
//! - a match is rejected unless the output net's channel contacts are
//!   EXACTLY the matched devices and every internal (stack) net is used
//!   by exactly its two neighbors and nothing else;
//! - collapse only engages when the netlist is big (>= min_devices) AND
//!   mostly gates (coverage >= min_coverage) — analog circuits fail the
//!   coverage test and keep today's transistor-level path untouched.
//!
//! A key structural bonus over library gate boxes (cases 42/43): the
//! template match IDENTIFIES the output net, so synthetic gates carry
//! real port direction (nodes[0] = output) and the block DAG works.

use crate::model::{builtin_symbols, SymbolDef};
use crate::parser::{SpiceDevice, SpiceParser};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedGate {
    /// "inv" | "nand2" | "nand3" | "nor2" | "nor3"
    pub kind: &'static str,
    /// Indices into the original device list, all consumed by this gate.
    pub device_indices: Vec<usize>,
    /// Input nets, sorted.
    pub inputs: Vec<String>,
    /// Output net (known from the match itself — this is what library
    /// boxes can never tell us).
    pub output: String,
}

pub struct GateCollapse {
    /// Replacement device list: unmatched originals (in original order)
    /// followed by one synthetic X device per gate.
    pub devices: Vec<SpiceDevice>,
    /// Box symbols for the synthetic gates, keyed `subckt_gate__<kind>`.
    pub symbols: HashMap<String, SymbolDef>,
    pub gate_count: usize,
    pub consumed_fets: usize,
    pub total_fets: usize,
}

/// One FET as a channel edge: polarity, gate net, and the two channel
/// terminals (order-agnostic).
struct Fet {
    dev: usize,
    is_p: bool,
    gate: String,
    ch: [String; 2],
}

fn fet_view(devices: &[SpiceDevice]) -> Vec<Fet> {
    let mut out = Vec::new();
    for (i, dev) in devices.iter().enumerate() {
        let pol = match dev.device_type {
            'M' if dev.nodes.len() >= 3 => match SpiceParser::infer_mos_type(dev) {
                "pmos4" => Some(true),
                _ => Some(false),
            },
            'X' => SpiceParser::infer_x_transistor_type(dev).map(|t| t == "pmos4"),
            _ => None,
        };
        if let Some(is_p) = pol {
            if dev.nodes.len() >= 3 {
                out.push(Fet {
                    dev: i,
                    is_p,
                    gate: dev.nodes[1].clone(),
                    ch: [dev.nodes[0].clone(), dev.nodes[2].clone()],
                });
            }
        }
    }
    out
}

/// Extract gates from the device list. Deterministic: candidate output
/// nets are visited in sorted order.
pub fn extract(devices: &[SpiceDevice], power_nets: &HashSet<String>) -> Vec<ExtractedGate> {
    let fets = fet_view(devices);
    let is_rail = |n: &str| power_nets.contains(&n.to_lowercase());

    // net → fet ordinals whose CHANNEL touches it (rails excluded: too big
    // and never an output or stack node).
    let mut chan: HashMap<&str, Vec<usize>> = HashMap::new();
    for (fi, f) in fets.iter().enumerate() {
        for t in &f.ch {
            if !is_rail(t) {
                chan.entry(t.as_str()).or_default().push(fi);
            }
        }
    }
    // Total usage count of every net across ALL device terminals (gates,
    // channels, bulk, non-FET devices) — internal stack nets must be
    // touched by exactly their two neighbors and nothing else.
    let mut total_use: HashMap<&str, usize> = HashMap::new();
    for dev in devices {
        for n in &dev.nodes {
            *total_use.entry(n.as_str()).or_default() += 1;
        }
    }

    fn other<'a>(f: &'a Fet, n: &str) -> &'a str {
        if f.ch[0] == n {
            f.ch[1].as_str()
        } else {
            f.ch[0].as_str()
        }
    }

    let mut consumed: Vec<bool> = vec![false; fets.len()];
    let mut gates: Vec<ExtractedGate> = Vec::new();

    let mut candidates: Vec<&str> = chan.keys().copied().collect();
    candidates.sort_unstable();

    for out_net in candidates {
        let touching: Vec<usize> = chan[out_net]
            .iter()
            .copied()
            .filter(|&fi| !consumed[fi])
            .collect();
        if touching.is_empty() {
            continue;
        }
        // The output net must be touched ONLY by the gate's own devices —
        // if any already-consumed fet also touched it, or if the sets
        // below don't account for every contact, reject.
        if touching.len() != chan[out_net].len() {
            continue;
        }
        let p_side: Vec<usize> = touching
            .iter()
            .copied()
            .filter(|&fi| fets[fi].is_p)
            .collect();
        let n_side: Vec<usize> = touching
            .iter()
            .copied()
            .filter(|&fi| !fets[fi].is_p)
            .collect();
        if p_side.is_empty() || n_side.is_empty() {
            continue;
        }

        // Follow a series chain of same-polarity fets from `out_net` down
        // to a rail. Returns (member ordinals, gate nets) or None.
        let follow_chain = |start: usize, pol_p: bool| -> Option<(Vec<usize>, Vec<String>)> {
            let mut members = vec![start];
            let mut gate_nets = vec![fets[start].gate.clone()];
            let mut cur = start;
            let mut cur_net = out_net.to_string();
            for _ in 0..3 {
                let next_net = other(&fets[cur], &cur_net).to_string();
                if is_rail(&next_net) {
                    return Some((members, gate_nets));
                }
                // Internal stack net: exactly two terminals in the whole
                // netlist (this fet and the next one), nothing else.
                if total_use.get(next_net.as_str()).copied().unwrap_or(0) != 2 {
                    return None;
                }
                let nexts: Vec<usize> = chan
                    .get(next_net.as_str())?
                    .iter()
                    .copied()
                    .filter(|&fi| fi != cur && !consumed[fi] && fets[fi].is_p == pol_p)
                    .collect();
                if nexts.len() != 1 {
                    return None;
                }
                cur = nexts[0];
                cur_net = next_net;
                members.push(cur);
                gate_nets.push(fets[cur].gate.clone());
            }
            None // deeper than 3 → not a v1 template
        };

        // All fets of `list` are parallel out_net→(same) rail?
        let all_parallel_to_rail = |list: &[usize]| -> bool {
            let mut rail: Option<&str> = None;
            for &fi in list {
                let o = other(&fets[fi], out_net);
                if !is_rail(o) {
                    return false;
                }
                match rail {
                    None => rail = Some(o),
                    Some(r) if r == o => {}
                    _ => return false,
                }
            }
            true
        };

        let mut matched: Option<(&'static str, Vec<usize>)> = None;

        // NAND-form / INV: P parallel to rail, single N chain to rail.
        if all_parallel_to_rail(&p_side) && n_side.len() == 1 {
            if let Some((chain, chain_gates)) = follow_chain(n_side[0], false) {
                let mut pg: Vec<&str> = p_side.iter().map(|&fi| fets[fi].gate.as_str()).collect();
                let mut ng: Vec<&str> = chain_gates.iter().map(|s| s.as_str()).collect();
                pg.sort_unstable();
                ng.sort_unstable();
                if pg == ng && p_side.len() == chain.len() {
                    let kind = match p_side.len() {
                        1 => Some("inv"),
                        2 => Some("nand2"),
                        3 => Some("nand3"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let mut members = p_side.clone();
                        members.extend(&chain);
                        matched = Some((kind, members));
                    }
                }
            }
        }
        // NOR-form: N parallel to rail, single P chain to rail.
        if matched.is_none() && all_parallel_to_rail(&n_side) && p_side.len() == 1 {
            if let Some((chain, chain_gates)) = follow_chain(p_side[0], true) {
                let mut ng: Vec<&str> = n_side.iter().map(|&fi| fets[fi].gate.as_str()).collect();
                let mut pg: Vec<&str> = chain_gates.iter().map(|s| s.as_str()).collect();
                ng.sort_unstable();
                pg.sort_unstable();
                if ng == pg && n_side.len() == chain.len() {
                    let kind = match n_side.len() {
                        1 => Some("inv"),
                        2 => Some("nor2"),
                        3 => Some("nor3"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let mut members = n_side.clone();
                        members.extend(&chain);
                        matched = Some((kind, members));
                    }
                }
            }
        }

        if let Some((kind, member_fis)) = matched {
            let mut inputs: Vec<String> = member_fis
                .iter()
                .map(|&fi| fets[fi].gate.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            inputs.sort_unstable();
            for &fi in &member_fis {
                consumed[fi] = true;
            }
            gates.push(ExtractedGate {
                kind,
                device_indices: member_fis.iter().map(|&fi| fets[fi].dev).collect(),
                inputs,
                output: out_net.to_string(),
            });
        }
    }

    gates
}

/// Run extraction and, if the scale regime applies, produce the collapsed
/// device list. Returns None when the netlist is small or not gate-like —
/// callers then proceed exactly as before.
pub fn try_collapse(
    devices: &[SpiceDevice],
    power_nets: &HashSet<String>,
    min_devices: usize,
    min_coverage: f64,
) -> Option<GateCollapse> {
    if devices.len() < min_devices {
        return None;
    }
    let total_fets = fet_view(devices).len();
    if total_fets == 0 {
        return None;
    }
    let gates = extract(devices, power_nets);
    let consumed_fets: usize = gates.iter().map(|g| g.device_indices.len()).sum();
    if std::env::var("N2S_DEBUG_STATS").is_ok() {
        eprintln!(
            "STATS gate_extract: {} gates, {}/{} fets covered ({:.0}%)",
            gates.len(),
            consumed_fets,
            total_fets,
            100.0 * consumed_fets as f64 / total_fets as f64
        );
    }
    if (consumed_fets as f64) < min_coverage * total_fets as f64 {
        return None;
    }

    let consumed_set: HashSet<usize> = gates
        .iter()
        .flat_map(|g| g.device_indices.iter().copied())
        .collect();

    let mut new_devices: Vec<SpiceDevice> = devices
        .iter()
        .enumerate()
        .filter(|(i, _)| !consumed_set.contains(i))
        .map(|(_, d)| d.clone())
        .collect();

    let mut symbols = HashMap::new();
    for (seq, gate) in gates.iter().enumerate() {
        let model = format!("gate__{}", gate.kind);
        let mut nodes = vec![gate.output.clone()];
        nodes.extend(gate.inputs.iter().cloned());
        let mut parameters = HashMap::new();
        // Self-describing port count so the eval side can size the box
        // without re-running extraction.
        parameters.insert("n2s_ports".to_string(), nodes.len().to_string());
        new_devices.push(SpiceDevice {
            device_type: 'X',
            instance_name: format!("G{}_{}", seq, gate.kind.to_uppercase()),
            nodes,
            model_or_value: model.clone(),
            parameters,
            line_number: 0,
        });
        symbols
            .entry(format!("subckt_{}", model))
            .or_insert_with(|| {
                let mut ports = vec!["Y".to_string()];
                ports.extend(
                    ["A", "B", "C"][..gate.inputs.len()]
                        .iter()
                        .map(|s| s.to_string()),
                );
                builtin_symbols::create_subcircuit_symbol(&model, &ports)
            });
    }

    Some(GateCollapse {
        devices: new_devices,
        symbols,
        gate_count: gates.len(),
        consumed_fets,
        total_fets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::CircuitAnalyzer;
    use crate::parser::SpiceParser;

    fn parse(text: &str) -> (Vec<SpiceDevice>, HashSet<String>) {
        let pr = SpiceParser::new().parse(text);
        let devices = if pr.devices.is_empty() && !pr.subcircuits.is_empty() {
            pr.subcircuits[0].devices.clone()
        } else {
            pr.devices.clone()
        };
        let power = CircuitAnalyzer::new().identify_power_nets(&devices);
        (devices, power)
    }

    #[test]
    fn extracts_inverter() {
        let (devices, power) = parse(
            "* inv\n\
             MP out in vdd vdd pch W=1u L=0.1u\n\
             MN out in vss vss nch W=0.5u L=0.1u\n",
        );
        let gates = extract(&devices, &power);
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].kind, "inv");
        assert_eq!(gates[0].output, "out");
        assert_eq!(gates[0].inputs, vec!["in"]);
    }

    #[test]
    fn extracts_nand2_with_swapped_drain_source() {
        // Extraction-style netlist: the series nfet chain is written with
        // rail in the drain position (case-36 convention).
        let (devices, power) = parse(
            "* nand2\n\
             MP1 out a vdd vdd pch\n\
             MP2 vdd b out vdd pch\n\
             MN1 out a mid vss nch\n\
             MN2 vss b mid vss nch\n",
        );
        let gates = extract(&devices, &power);
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].kind, "nand2");
        assert_eq!(gates[0].inputs, vec!["a", "b"]);
    }

    #[test]
    fn extracts_nor3() {
        let (devices, power) = parse(
            "* nor3\n\
             MP1 m1 a vdd vdd pch\n\
             MP2 m2 b m1 vdd pch\n\
             MP3 out c m2 vdd pch\n\
             MN1 out a vss vss nch\n\
             MN2 out b vss vss nch\n\
             MN3 out c vss vss nch\n",
        );
        let gates = extract(&devices, &power);
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].kind, "nor3");
    }

    #[test]
    fn rejects_stack_net_used_elsewhere() {
        // `mid` also feeds another gate → not an internal stack net.
        let (devices, power) = parse(
            "* not a clean nand2\n\
             MP1 out a vdd vdd pch\n\
             MP2 out b vdd vdd pch\n\
             MN1 out a mid vss nch\n\
             MN2 mid b vss vss nch\n\
             MX  probe mid vss vss nch\n",
        );
        let gates = extract(&devices, &power);
        assert!(gates.iter().all(|g| g.kind != "nand2"));
    }

    #[test]
    fn rejects_mismatched_gate_sets() {
        // P side gates {a,b}, N side gates {a,c} → no gate.
        let (devices, power) = parse(
            "* mismatch\n\
             MP1 out a vdd vdd pch\n\
             MP2 out b vdd vdd pch\n\
             MN1 out a mid vss nch\n\
             MN2 mid c vss vss nch\n",
        );
        assert!(extract(&devices, &power).is_empty());
    }

    #[test]
    fn ground_truth_case_32_comp_clk_gen() {
        // Case 32's header documents its composition: 5x NOR2 + 2x NAND2
        // + 1x INV, flattened to 30 MOSFETs. The extractor must recover
        // exactly that.
        let text = std::fs::read_to_string("tests/examples/32_comp_clk_gen_28nm.sp").unwrap();
        let (devices, power) = parse(&text);
        let gates = extract(&devices, &power);
        let count = |k: &str| gates.iter().filter(|g| g.kind == k).count();
        assert_eq!(count("nor2"), 5, "gates: {:?}", gates);
        assert_eq!(count("nand2"), 2);
        assert_eq!(count("inv"), 1);
        assert_eq!(
            gates.iter().map(|g| g.device_indices.len()).sum::<usize>(),
            30
        );
    }

    #[test]
    fn collapse_respects_regime_gates() {
        let text = std::fs::read_to_string("tests/examples/32_comp_clk_gen_28nm.sp").unwrap();
        let (devices, power) = parse(&text);
        // 30 devices < 60 → no collapse even at full coverage.
        assert!(try_collapse(&devices, &power, 60, 0.7).is_none());
        // Lowered threshold → collapses into 8 gate boxes.
        let c = try_collapse(&devices, &power, 10, 0.7).unwrap();
        assert_eq!(c.gate_count, 8);
        assert_eq!(c.consumed_fets, 30);
        assert_eq!(c.devices.len(), 8);
        assert!(c.symbols.contains_key("subckt_gate__nor2"));
    }
}
