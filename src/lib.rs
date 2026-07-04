pub mod analyzer;
pub mod eval;
pub mod export;
pub mod model;
pub mod parser;
pub mod placer;
pub mod router;

use analyzer::{CircuitAnalyzer, ClusterOptions};
use model::{builtin_symbols, Schematic};
use parser::ParseResult;
use placer::{PlacerOptions, SchematicPlacer};
use router::{RouterOptions, SchematicRouter};
use std::collections::HashMap;

/// Options for the full N2S conversion pipeline.
#[derive(Default)]
pub struct ConvertOptions {
    pub placer: PlacerOptions,
    pub router: RouterOptions,
    pub cluster: ClusterOptions,
    /// When true, render subcircuit instances as boxes with ports
    /// instead of expanding them to individual devices.
    pub hierarchical: bool,
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

    // Decide whether to use hierarchical or flat mode.
    //
    // Three rendering paths:
    //   (a) hierarchical — top-level devices, X instances rendered as
    //       boxes with ports. Used when the netlist has top-level X
    //       instances *or* the user passed --hierarchical.
    //   (b) subckt-only — render the first .subckt's interior. Only used
    //       when the netlist defines a subckt but never instantiates it
    //       at the top level (e.g. a standalone library file).
    //   (c) simple — top-level devices only. Used when there are no
    //       .subckt definitions at all.
    //
    // Prior to this fix, path (a) required the user to pass
    // --hierarchical explicitly, so any netlist that defined a subckt
    // *and* instantiated it at the top level fell into path (b) and
    // silently dropped all top-level content. This was Bug 3 in the
    // 2026-05-01 test-set-expansion findings.
    let has_x_instances = pr.devices.iter().any(|d| d.device_type == 'X');
    let has_subckt_defs = !pr.subcircuits.is_empty();
    let use_hierarchical = (opts.hierarchical || has_x_instances) && has_subckt_defs;

    let (devices, mut subckt_symbols) = if use_hierarchical {
        // Hierarchical mode: top-level devices, render X instances as boxes
        let syms = build_subcircuit_symbols(&pr);
        (&pr.devices, syms)
    } else if has_subckt_defs && pr.devices.is_empty() {
        // Subckt-only mode: the netlist defines a subckt but has no
        // top-level devices. Render the first subckt's interior. The
        // interior may instantiate OTHER locally-defined subckts (OpenRAM
        // hierarchies do), so build symbols for all local defs — real
        // port names beat the numbered-pin fallback below.
        (&pr.subcircuits[0].devices, build_subcircuit_symbols(&pr))
    } else {
        // Simple mode: top-level devices only
        (&pr.devices, HashMap::new())
    };

    // Synthesize a generic box symbol for every X instance whose subcircuit
    // is NOT defined in this file (typical for PDK primitives resolved via
    // .lib/.include, e.g. sky130_fd_pr__nfet_01v8). Port names are unknown,
    // so pins are numbered by node position. Without this, such devices had
    // no SymbolDef at all: the router collapsed every pin (and its net
    // label) onto the component centre, and the SVG fell back to a blank
    // rectangle -- labels printed on top of boxes.
    for dev in devices.iter().filter(|d| d.device_type == 'X') {
        let key = format!("subckt_{}", dev.model_or_value);
        subckt_symbols.entry(key).or_insert_with(|| {
            let ports: Vec<String> = (1..=dev.nodes.len()).map(|i| i.to_string()).collect();
            builtin_symbols::create_subcircuit_symbol(&dev.model_or_value, &ports)
        });
    }

    if devices.is_empty() {
        return Err("No devices found in SPICE input".into());
    }

    // 2. Analyze
    let analyzer = CircuitAnalyzer::new();
    let mut power_nets = analyzer.identify_power_nets(devices);
    // Rails declared in-netlist via `* n2s: power_net <name>` directives
    // (audit item C1: e.g. an LDO's regulated output used as a supply).
    for net in &pr.extra_power_nets {
        power_nets.insert(net.to_lowercase());
    }
    let blocks = analyzer.analyze_with_power_nets(devices, &opts.cluster, &power_nets);

    // 3. Place (with device info for cross-block symmetry alignment)
    let placer = SchematicPlacer;
    let placement = placer.place_with_devices(&blocks, &power_nets, &opts.placer, devices);

    // 4. Route (pass subcircuit symbols for X instance pin mapping)
    let router = SchematicRouter;
    let schematic = router.route_with_subcircuits(
        placement,
        devices,
        &power_nets,
        &opts.router,
        &subckt_symbols,
    );

    Ok(ConvertResult {
        schematic,
        subcircuit_symbols: subckt_symbols,
    })
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
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("Cannot read file {}: {}", path, e))?;
    convert(&text, opts)
}
