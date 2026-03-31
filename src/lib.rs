pub mod model;
pub mod parser;
pub mod analyzer;
pub mod placer;
pub mod router;
pub mod export;
pub mod eval;

use std::collections::HashMap;
use model::{Schematic, builtin_symbols};
use parser::ParseResult;
use analyzer::{CircuitAnalyzer, ClusterOptions};
use placer::{SchematicPlacer, PlacerOptions};
use router::{SchematicRouter, RouterOptions};

/// Options for the full N2S conversion pipeline.
pub struct ConvertOptions {
    pub placer: PlacerOptions,
    pub router: RouterOptions,
    pub cluster: ClusterOptions,
    /// When true, render subcircuit instances as boxes with ports
    /// instead of expanding them to individual devices.
    pub hierarchical: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            placer: PlacerOptions::default(),
            router: RouterOptions::default(),
            cluster: ClusterOptions::default(),
            hierarchical: false,
        }
    }
}

/// Result of the full conversion pipeline.
pub struct ConvertResult {
    pub schematic: Schematic,
    /// Dynamic symbols for subcircuit instances (empty if no hierarchy)
    pub subcircuit_symbols: HashMap<String, model::SymbolDef>,
}

/// Full pipeline: SPICE text → Schematic
pub fn convert(spice_text: &str, opts: &ConvertOptions) -> Result<Schematic, String> {
    convert_full(spice_text, opts).map(|r| r.schematic)
}

/// Full pipeline with subcircuit symbol output for rendering.
pub fn convert_full(spice_text: &str, opts: &ConvertOptions) -> Result<ConvertResult, String> {
    // 1. Parse
    let pr: ParseResult = parser::SpiceParser::new().parse(spice_text);

    // Decide whether to use hierarchical or flat mode
    let has_x_instances = pr.devices.iter().any(|d| d.device_type == 'X');
    let has_subckt_defs = !pr.subcircuits.is_empty();
    let use_hierarchical = opts.hierarchical && has_x_instances && has_subckt_defs;

    let (devices, subckt_symbols) = if use_hierarchical {
        // Hierarchical mode: use top-level devices, render X instances as boxes
        let syms = build_subcircuit_symbols(&pr);
        (&pr.devices, syms)
    } else if has_subckt_defs {
        // Flat mode: use first subcircuit's internal devices
        (&pr.subcircuits[0].devices, HashMap::new())
    } else {
        // Simple mode: top-level devices only
        (&pr.devices, HashMap::new())
    };

    if devices.is_empty() {
        return Err("No devices found in SPICE input".into());
    }

    // 2. Analyze
    let analyzer = CircuitAnalyzer::new();
    let power_nets = analyzer.identify_power_nets(devices);
    let blocks = analyzer.analyze(devices, &opts.cluster);

    // 3. Place (with device info for cross-block symmetry alignment)
    let placer = SchematicPlacer;
    let placement = placer.place_with_devices(&blocks, &power_nets, &opts.placer, devices);

    // 4. Route (pass subcircuit symbols for X instance pin mapping)
    let router = SchematicRouter;
    let schematic = router.route_with_subcircuits(
        placement, devices, &power_nets, &opts.router, &subckt_symbols,
    );

    Ok(ConvertResult { schematic, subcircuit_symbols: subckt_symbols })
}

/// Build dynamic SymbolDef for each subcircuit definition referenced by X instances.
fn build_subcircuit_symbols(pr: &ParseResult) -> HashMap<String, model::SymbolDef> {
    let mut syms = HashMap::new();
    for subckt in &pr.subcircuits {
        let sym = builtin_symbols::create_subcircuit_symbol(&subckt.name, &subckt.ports);
        syms.insert(format!("subckt_{}", subckt.name), sym);
    }
    syms
}

/// Convenience: convert from file path
pub fn convert_file(path: &str, opts: &ConvertOptions) -> Result<Schematic, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file {}: {}", path, e))?;
    convert(&text, opts)
}
