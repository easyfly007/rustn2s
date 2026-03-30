pub mod model;
pub mod parser;
pub mod analyzer;
pub mod placer;
pub mod router;
pub mod export;
pub mod eval;

use model::Schematic;
use parser::ParseResult;
use analyzer::{CircuitAnalyzer, ClusterOptions};
use placer::{SchematicPlacer, PlacerOptions};
use router::{SchematicRouter, RouterOptions};

/// Options for the full N2S conversion pipeline.
pub struct ConvertOptions {
    pub placer: PlacerOptions,
    pub router: RouterOptions,
    pub cluster: ClusterOptions,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            placer: PlacerOptions::default(),
            router: RouterOptions::default(),
            cluster: ClusterOptions::default(),
        }
    }
}

/// Full pipeline: SPICE text → Schematic
pub fn convert(spice_text: &str, opts: &ConvertOptions) -> Result<Schematic, String> {
    // 1. Parse
    let pr: ParseResult = parser::SpiceParser::new().parse(spice_text);

    // Use subcircuit devices if available, else top-level
    let devices = if !pr.subcircuits.is_empty() {
        &pr.subcircuits[0].devices
    } else {
        &pr.devices
    };

    if devices.is_empty() {
        return Err("No devices found in SPICE input".into());
    }

    // 2. Analyze
    let analyzer = CircuitAnalyzer::new();
    let power_nets = analyzer.identify_power_nets(devices);
    let blocks = analyzer.analyze(devices, &opts.cluster);

    // 3. Place
    let placer = SchematicPlacer;
    let placement = placer.place(&blocks, &power_nets, &opts.placer);

    // 4. Route
    let router = SchematicRouter;
    let schematic = router.route(placement, devices, &power_nets, &opts.router);

    Ok(schematic)
}

/// Convenience: convert from file path
pub fn convert_file(path: &str, opts: &ConvertOptions) -> Result<Schematic, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file {}: {}", path, e))?;
    convert(&text, opts)
}
