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

use crate::model::Schematic;
use crate::parser::ParseResult;
use serde::Serialize;

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

pub fn evaluate(parse_result: &ParseResult, schematic: &Schematic) -> EvalReport {
    EvalReport {
        connectivity: connectivity::check(parse_result, schematic),
        component_overlap: overlap::check(schematic),
        wire_crossings: wire_crossings::check(schematic),
        wire_length: wire_length::check(schematic),
        wire_bends: wire_bends::check(schematic),
        bounding_box: bounding_box::check(schematic),
        label_usage: label_usage::check(schematic),
        symmetry: symmetry::check(schematic),
        power_convention: power_convention::check(schematic),
    }
}
