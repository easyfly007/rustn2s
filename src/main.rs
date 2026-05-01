use clap::Parser;
use n2s::export::{svg, json, kicad};
use n2s::ConvertOptions;

#[derive(Parser)]
#[command(name = "n2s", about = "Netlist to Schematic — convert SPICE netlists to visual schematics")]
struct Cli {
    /// Input SPICE netlist file
    input: String,

    /// Output file(s) — format inferred from extension (.svg, .json, .kicad_sch)
    #[arg(short, long, required = true)]
    output: Vec<String>,

    /// Horizontal spacing between layout layers
    #[arg(long, default_value_t = 200.0)]
    layer_spacing: f64,

    /// Spacing between functional blocks
    #[arg(long, default_value_t = 100.0)]
    block_spacing: f64,

    /// Spacing between devices within a block
    #[arg(long, default_value_t = 80.0)]
    device_spacing: f64,

    /// Grid snap size
    #[arg(long, default_value_t = 10.0)]
    grid: f64,

    /// Distance threshold for using labels instead of wires
    #[arg(long, default_value_t = 300.0)]
    label_threshold: f64,

    /// Disable pattern recognition (diff pair, current mirror, etc.)
    #[arg(long)]
    no_patterns: bool,

    /// SVG scale factor
    #[arg(long, default_value_t = 1.0)]
    scale: f64,

    /// Hide grid in SVG output
    #[arg(long)]
    no_grid: bool,

    /// Render subcircuit instances as boxes with ports (hierarchical view)
    #[arg(long)]
    hierarchical: bool,

    /// Enable obstacle-aware A* routing (wires avoid component bodies).
    /// Off by default — see docs/routing_improvement.md for trade-offs.
    #[arg(long)]
    obstacle_avoidance: bool,

    /// Bend penalty for A* routing (higher = preferring straighter wires)
    #[arg(long, default_value_t = 0.5)]
    bend_penalty: f64,

    /// Crossing penalty for A* routing (higher = preferring detours over
    /// crossing already-routed wires)
    #[arg(long, default_value_t = 20.0)]
    crossing_penalty: f64,

    /// Adaptive label-threshold ratio: the effective threshold is
    /// max(--label-threshold, bbox_diagonal × this). Set to 0.0 to use
    /// the absolute --label-threshold only.
    #[arg(long, default_value_t = 0.3)]
    adaptive_label_ratio: f64,
}

fn main() {
    let cli = Cli::parse();

    let opts = ConvertOptions {
        placer: n2s::placer::PlacerOptions {
            layer_spacing: cli.layer_spacing,
            inter_block_spacing: cli.block_spacing,
            intra_block_spacing: cli.device_spacing,
            grid_size: cli.grid,
        },
        router: n2s::router::RouterOptions {
            long_net_threshold: cli.label_threshold,
            grid_size: cli.grid,
            avoid_obstacles: cli.obstacle_avoidance,
            bend_penalty: cli.bend_penalty,
            crossing_penalty: cli.crossing_penalty,
            adaptive_label_ratio: cli.adaptive_label_ratio,
        },
        cluster: n2s::analyzer::ClusterOptions {
            recognize_patterns: !cli.no_patterns,
            ..Default::default()
        },
        hierarchical: cli.hierarchical,
    };

    let spice_text = std::fs::read_to_string(&cli.input)
        .unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", cli.input, e);
            std::process::exit(1);
        });
    let conv = match n2s::convert_full(&spice_text, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let schematic = &conv.schematic;

    for output in &cli.output {
        let result = if output.ends_with(".svg") {
            let svg_opts = svg::SvgOptions {
                scale: cli.scale,
                show_grid: !cli.no_grid,
                ..Default::default()
            };
            svg::render_to_file_with_symbols(schematic, output, &svg_opts, &conv.subcircuit_symbols)
        } else if output.ends_with(".json") {
            json::render_to_file(schematic, output)
        } else if output.ends_with(".kicad_sch") {
            kicad::render_to_file(schematic, output, &conv.subcircuit_symbols)
        } else {
            Err(format!("Unknown output format: {}. Use .svg, .json, or .kicad_sch", output))
        };

        match result {
            Ok(()) => eprintln!("Written: {}", output),
            Err(e) => {
                eprintln!("Error writing {}: {}", output, e);
                std::process::exit(1);
            }
        }
    }
}
