use clap::Parser;
use serde::Serialize;
use n2s::eval;
use n2s::eval::score::{self, ScoreWeights, ScoreBreakdown, TuningAdvice};
use n2s::model::Schematic;
use n2s::parser::SpiceParser;
use n2s::ConvertOptions;
use n2s::export::{svg, json};

#[derive(Parser)]
#[command(name = "n2s-improve", about = "Iteratively improve schematic layout quality")]
struct Cli {
    /// Input SPICE netlist file
    input: String,

    /// Output SVG file
    #[arg(short, long)]
    output: Option<String>,

    /// Output JSON schematic file
    #[arg(long)]
    json: Option<String>,

    /// Maximum optimization iterations
    #[arg(long, default_value_t = 10)]
    max_iter: usize,

    /// Target quality score (0.0–1.0); stop early if reached
    #[arg(long, default_value_t = 0.9)]
    target_score: f64,

    /// Initial layer spacing
    #[arg(long, default_value_t = 200.0)]
    layer_spacing: f64,

    /// Initial block spacing
    #[arg(long, default_value_t = 100.0)]
    block_spacing: f64,

    /// Initial device spacing
    #[arg(long, default_value_t = 80.0)]
    device_spacing: f64,

    /// Grid snap size
    #[arg(long, default_value_t = 10.0)]
    grid: f64,

    /// Initial label threshold
    #[arg(long, default_value_t = 300.0)]
    label_threshold: f64,

    /// Disable pattern recognition
    #[arg(long)]
    no_patterns: bool,

    /// SVG scale factor
    #[arg(long, default_value_t = 1.0)]
    scale: f64,

    /// Hide grid in SVG output
    #[arg(long)]
    no_grid: bool,

    /// Pretty-print the report JSON
    #[arg(long)]
    pretty: bool,

    /// Only output the final report (suppress iteration logs)
    #[arg(long)]
    quiet: bool,
}

/// Parameters being tuned across iterations.
#[derive(Debug, Clone, Serialize)]
struct TunableParams {
    layer_spacing: f64,
    block_spacing: f64,
    device_spacing: f64,
    label_threshold: f64,
}

/// Record of a single iteration.
#[derive(Debug, Clone, Serialize)]
struct IterationRecord {
    iteration: usize,
    params: TunableParams,
    score: ScoreBreakdown,
    advice: Vec<TuningAdvice>,
}

/// Final output report.
#[derive(Debug, Serialize)]
struct ImproveReport {
    input_file: String,
    iterations_run: usize,
    converged: bool,
    convergence_reason: String,
    initial_score: f64,
    final_score: f64,
    improvement: f64,
    best_params: TunableParams,
    best_score: ScoreBreakdown,
    history: Vec<IterationRecord>,
}

fn main() {
    let cli = Cli::parse();

    // Parse SPICE netlist once (shared across all iterations)
    let spice_text = std::fs::read_to_string(&cli.input)
        .unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", cli.input, e);
            std::process::exit(1);
        });
    let parse_result = SpiceParser::new().parse(&spice_text);

    let weights = ScoreWeights::default();

    // Initial parameters
    let mut params = TunableParams {
        layer_spacing: cli.layer_spacing,
        block_spacing: cli.block_spacing,
        device_spacing: cli.device_spacing,
        label_threshold: cli.label_threshold,
    };

    let mut history: Vec<IterationRecord> = Vec::new();
    let mut best_score = f64::NEG_INFINITY;
    let mut best_params = params.clone();
    let mut best_schematic: Option<Schematic> = None;
    let mut initial_score = 0.0;
    let mut converged = false;
    let mut convergence_reason = String::new();

    for iter in 0..cli.max_iter {
        // Build ConvertOptions from current params
        let opts = ConvertOptions {
            placer: n2s::placer::PlacerOptions {
                layer_spacing: params.layer_spacing,
                inter_block_spacing: params.block_spacing,
                intra_block_spacing: params.device_spacing,
                grid_size: cli.grid,
            },
            router: n2s::router::RouterOptions {
                long_net_threshold: params.label_threshold,
                grid_size: cli.grid,
            },
            cluster: n2s::analyzer::ClusterOptions {
                recognize_patterns: !cli.no_patterns,
                ..Default::default()
            },
            hierarchical: false, // Always use flat mode for optimization
        };

        // Run the pipeline
        let schematic = match n2s::convert(&spice_text, &opts) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error at iteration {}: {}", iter, e);
                break;
            }
        };

        // Evaluate
        let report = eval::evaluate(&parse_result, &schematic);
        let breakdown = score::compute_score(&report, &weights);
        let advice = score::suggest_tuning(
            &report, &breakdown,
            params.layer_spacing, params.block_spacing,
            params.device_spacing, params.label_threshold,
        );

        if iter == 0 {
            initial_score = breakdown.overall;
        }

        if !cli.quiet {
            eprintln!(
                "Iteration {}: score={:.3} [overlap={:.2} cross={:.2} ar={:.2} wire={:.2} label={:.2} sym={:.2} pwr={:.2}]",
                iter, breakdown.overall,
                breakdown.overlap_score, breakdown.crossings_score,
                breakdown.aspect_ratio_score, breakdown.wire_length_score,
                breakdown.label_ratio_score, breakdown.symmetry_score,
                breakdown.power_convention_score,
            );
            if !advice.is_empty() {
                for a in &advice {
                    eprintln!("  -> {} : {:.1} → {:.1} ({})",
                        a.parameter, a.current_value, a.suggested_value, a.reason);
                }
            }
        }

        // Track best
        if breakdown.overall > best_score {
            best_score = breakdown.overall;
            best_params = params.clone();
            best_schematic = Some(schematic);
        }

        let record = IterationRecord {
            iteration: iter,
            params: params.clone(),
            score: breakdown.clone(),
            advice: advice.clone(),
        };
        history.push(record);

        // Check convergence
        if breakdown.overall >= cli.target_score {
            converged = true;
            convergence_reason = format!("Target score {:.3} reached at iteration {}", cli.target_score, iter);
            if !cli.quiet {
                eprintln!("Converged: {}", convergence_reason);
            }
            break;
        }

        if advice.is_empty() {
            converged = true;
            convergence_reason = format!("No further tuning advice at iteration {}", iter);
            if !cli.quiet {
                eprintln!("Converged: {}", convergence_reason);
            }
            break;
        }

        // Check if score is not improving (stalled for 3 iterations)
        if history.len() >= 3 {
            let recent: Vec<f64> = history[history.len()-3..].iter()
                .map(|r| r.score.overall).collect();
            let max_diff = recent.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0f64, f64::max);
            if max_diff < 0.001 {
                converged = true;
                convergence_reason = format!(
                    "Score stalled at {:.3} for 3 iterations", breakdown.overall
                );
                if !cli.quiet {
                    eprintln!("Converged: {}", convergence_reason);
                }
                break;
            }
        }

        // Apply tuning advice for next iteration
        for a in &advice {
            match a.parameter.as_str() {
                "layer_spacing" => params.layer_spacing = a.suggested_value,
                "block_spacing" => params.block_spacing = a.suggested_value,
                "device_spacing" => params.device_spacing = a.suggested_value,
                "label_threshold" => params.label_threshold = a.suggested_value,
                _ => {}
            }
        }

        // Clamp parameters to reasonable ranges
        params.layer_spacing = params.layer_spacing.clamp(50.0, 1000.0);
        params.block_spacing = params.block_spacing.clamp(30.0, 500.0);
        params.device_spacing = params.device_spacing.clamp(30.0, 300.0);
        params.label_threshold = params.label_threshold.clamp(100.0, 2000.0);
    }

    if !converged {
        convergence_reason = format!("Max iterations ({}) reached", cli.max_iter);
    }

    // Write best schematic to output files
    if let Some(schematic) = &best_schematic {
        if let Some(ref svg_path) = cli.output {
            let svg_opts = svg::SvgOptions {
                scale: cli.scale,
                show_grid: !cli.no_grid,
                ..Default::default()
            };
            match svg::render_to_file(schematic, svg_path, &svg_opts) {
                Ok(()) => eprintln!("Written: {}", svg_path),
                Err(e) => eprintln!("Error writing SVG: {}", e),
            }
        }

        if let Some(ref json_path) = cli.json {
            match json::render_to_file(schematic, json_path) {
                Ok(()) => eprintln!("Written: {}", json_path),
                Err(e) => eprintln!("Error writing JSON: {}", e),
            }
        }
    }

    // Output report to stdout
    let report = ImproveReport {
        input_file: cli.input.clone(),
        iterations_run: history.len(),
        converged,
        convergence_reason,
        initial_score: round3(initial_score),
        final_score: round3(best_score),
        improvement: round3(best_score - initial_score),
        best_params,
        best_score: history.iter()
            .max_by(|a, b| a.score.overall.partial_cmp(&b.score.overall).unwrap())
            .map(|r| r.score.clone())
            .unwrap_or_else(|| ScoreBreakdown {
                overall: 0.0, overlap_score: 0.0, crossings_score: 0.0,
                aspect_ratio_score: 0.0, wire_length_score: 0.0,
                label_ratio_score: 0.0, symmetry_score: 0.0, power_convention_score: 0.0,
            }),
        history,
    };

    let output = if cli.pretty {
        serde_json::to_string_pretty(&report).unwrap()
    } else {
        serde_json::to_string(&report).unwrap()
    };
    println!("{}", output);
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}
