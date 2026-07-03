mod bounding_box;
mod connectivity;
mod label_usage;
mod overlap;
mod power_convention;
pub mod score;
mod symmetry;
mod wire_bends;
mod wire_crossings;
mod wire_length;

use crate::model::{builtin_symbols, Schematic, SymbolDef};
use crate::parser::ParseResult;
use serde::Serialize;
use std::collections::HashMap;

pub use bounding_box::BoundingBoxReport;
pub use connectivity::ConnectivityReport;
pub use label_usage::LabelUsageReport;
pub use overlap::OverlapReport;
pub use power_convention::PowerConventionReport;
pub use symmetry::SymmetryReport;
pub use wire_bends::WireBendReport;
pub use wire_crossings::WireCrossingReport;
pub use wire_length::WireLengthReport;

#[derive(Debug, Serialize)]
pub struct EvalReport {
    pub connectivity: ConnectivityReport,
    pub component_overlap: OverlapReport,
    pub wire_crossings: WireCrossingReport,
    pub wire_length: WireLengthReport,
    pub wire_bends: WireBendReport,
    pub bounding_box: BoundingBoxReport,
    pub label_usage: LabelUsageReport,
    pub symmetry: SymmetryReport,
    pub power_convention: PowerConventionReport,
}

/// Symbol table for the schematic's `subckt_*` box components: locally
/// defined subcircuits plus a synthesized numbered-pin box for every X
/// instance whose model has no local definition (mirrors convert_full's
/// synthesis). Built as a superset over all modes — X instances are
/// collected from the top level AND every subckt interior, so it covers
/// whichever device slice the pipeline actually rendered. Geometric checks
/// (overlap) need it to size box components; without it they are invisible.
fn subckt_symbol_table(pr: &ParseResult) -> HashMap<String, SymbolDef> {
    let mut map = HashMap::new();
    for sub in &pr.subcircuits {
        map.insert(
            format!("subckt_{}", sub.name),
            builtin_symbols::create_subcircuit_symbol(&sub.name, &sub.ports),
        );
    }
    let all_x = pr
        .devices
        .iter()
        .chain(pr.subcircuits.iter().flat_map(|s| s.devices.iter()))
        .filter(|d| d.device_type == 'X');
    for dev in all_x {
        let key = format!("subckt_{}", dev.model_or_value);
        map.entry(key).or_insert_with(|| {
            let ports: Vec<String> = (1..=dev.nodes.len()).map(|i| i.to_string()).collect();
            builtin_symbols::create_subcircuit_symbol(&dev.model_or_value, &ports)
        });
    }
    map
}

pub fn evaluate(parse_result: &ParseResult, schematic: &Schematic) -> EvalReport {
    EvalReport {
        connectivity: connectivity::check(parse_result, schematic),
        component_overlap: overlap::check(schematic, &subckt_symbol_table(parse_result)),
        wire_crossings: wire_crossings::check(schematic),
        wire_length: wire_length::check(schematic),
        wire_bends: wire_bends::check(schematic),
        bounding_box: bounding_box::check(schematic),
        label_usage: label_usage::check(schematic),
        symmetry: symmetry::check(schematic),
        power_convention: power_convention::check(schematic),
    }
}
