use clap::Parser;
use serde::Serialize;
use n2s::eval;
use n2s::eval::score::{self, ScoreWeights, ScoreBreakdown, TuningAdvice};
use n2s::model::Schematic;
use n2s::parser::{ParseResult, SpiceParser};
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

    /// Maximum optimization iterations (per restart when --search is on)
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

    /// Initial adaptive label-threshold ratio (router knob;
    /// effective threshold = max(label_threshold, bbox_diagonal × this)).
    #[arg(long, default_value_t = 0.3)]
    adaptive_label_ratio: f64,

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

    /// Run multiple restarts from different initial parameter sets and
    /// keep the global best. Without this flag, a single greedy run is
    /// performed from the user-supplied --layer-spacing etc.
    #[arg(long)]
    search: bool,

    /// Number of restarts when --search is on. Restart 0 always uses the
    /// user-supplied initial parameters; the remaining restarts cover
    /// deterministic spaced points across the parameter space.
    #[arg(long, default_value_t = 8)]
    search_restarts: usize,
}

/// Parameters being tuned across iterations.
#[derive(Debug, Clone, Serialize)]
struct TunableParams {
    layer_spacing: f64,
    block_spacing: f64,
    device_spacing: f64,
    label_threshold: f64,
    /// Adaptive label-threshold ratio (router knob). 0.0 means use the
    /// absolute label_threshold only; higher values raise the effective
    /// threshold for larger schematics so more nets stay as wires.
    adaptive_label_ratio: f64,
}

/// Record of a single iteration.
#[derive(Debug, Clone, Serialize)]
struct IterationRecord {
    /// Restart this iteration belongs to (always 0 without --search).
    restart: usize,
    iteration: usize,
    params: TunableParams,
    score: ScoreBreakdown,
    advice: Vec<TuningAdvice>,
}

/// Outcome of a single restart's greedy loop.
#[derive(Debug, Clone, Serialize)]
struct RestartSummary {
    restart: usize,
    initial_params: TunableParams,
    best_score: f64,
    best_params: TunableParams,
    iterations: usize,
    converged: bool,
    convergence_reason: String,
}

/// Final output report.
#[derive(Debug, Serialize)]
struct ImproveReport {
    input_file: String,
    iterations_run: usize,
    restarts: usize,
    converged: bool,
    convergence_reason: String,
    initial_score: f64,
    final_score: f64,
    improvement: f64,
    best_params: TunableParams,
    best_score: ScoreBreakdown,
    /// Per-restart summaries (length 1 unless --search is on).
    restart_summary: Vec<RestartSummary>,
    history: Vec<IterationRecord>,
}

/// Outcome of a single greedy optimization from one starting point.
struct RestartResult {
    summary: RestartSummary,
    history: Vec<IterationRecord>,
    best_schematic: Option<Schematic>,
    best_breakdown: ScoreBreakdown,
}

/// Build a ConvertOptions from the current params + CLI knobs.
fn build_opts(params: &TunableParams, cli: &Cli) -> ConvertOptions {
    ConvertOptions {
        placer: n2s::placer::PlacerOptions {
            layer_spacing: params.layer_spacing,
            inter_block_spacing: params.block_spacing,
            intra_block_spacing: params.device_spacing,
            grid_size: cli.grid,
        },
        router: n2s::router::RouterOptions {
            long_net_threshold: params.label_threshold,
            grid_size: cli.grid,
            adaptive_label_ratio: params.adaptive_label_ratio,
            ..Default::default()
        },
        cluster: n2s::analyzer::ClusterOptions {
            recognize_patterns: !cli.no_patterns,
            ..Default::default()
        },
        hierarchical: false,
    }
}

/// Run the greedy advice-driven loop from one starting point until the
/// target score is reached, no advice is left, the score stalls, or
/// `max_iter` iterations have been spent.
fn optimize_from(
    restart_idx: usize,
    initial_params: TunableParams,
    cli: &Cli,
    spice_text: &str,
    parse_result: &ParseResult,
    weights: &ScoreWeights,
    log_prefix: Option<&str>,
) -> RestartResult {
    let mut params = initial_params.clone();
    let mut history: Vec<IterationRecord> = Vec::new();
    let mut best_score = f64::NEG_INFINITY;
    let mut best_params = params.clone();
    let mut best_schematic: Option<Schematic> = None;
    let mut best_breakdown = ScoreBreakdown {
        overall: 0.0, overlap_score: 0.0, crossings_score: 0.0,
        aspect_ratio_score: 0.0, wire_length_score: 0.0,
        label_ratio_score: 0.0, symmetry_score: 0.0, power_convention_score: 0.0,
    };
    let mut converged = false;
    let mut convergence_reason = String::new();

    for iter in 0..cli.max_iter {
        let opts = build_opts(&params, cli);

        let schematic = match n2s::convert(spice_text, &opts) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error at iteration {}: {}", iter, e);
                break;
            }
        };

        let report = eval::evaluate(parse_result, &schematic);
        let breakdown = score::compute_score(&report, weights);
        let advice = score::suggest_tuning(
            &report, &breakdown,
            params.layer_spacing, params.block_spacing,
            params.device_spacing, params.label_threshold,
        );

        if let Some(prefix) = log_prefix {
            eprintln!(
                "{}iter {}: score={:.3} [overlap={:.2} cross={:.2} ar={:.2} wire={:.2} label={:.2} sym={:.2} pwr={:.2}]",
                prefix, iter, breakdown.overall,
                breakdown.overlap_score, breakdown.crossings_score,
                breakdown.aspect_ratio_score, breakdown.wire_length_score,
                breakdown.label_ratio_score, breakdown.symmetry_score,
                breakdown.power_convention_score,
            );
            for a in &advice {
                eprintln!("  -> {} : {:.1} → {:.1} ({})",
                    a.parameter, a.current_value, a.suggested_value, a.reason);
            }
        }

        if breakdown.overall > best_score {
            best_score = breakdown.overall;
            best_params = params.clone();
            best_schematic = Some(schematic);
            best_breakdown = breakdown.clone();
        }

        history.push(IterationRecord {
            restart: restart_idx,
            iteration: iter,
            params: params.clone(),
            score: breakdown.clone(),
            advice: advice.clone(),
        });

        if breakdown.overall >= cli.target_score {
            converged = true;
            convergence_reason = format!(
                "Target score {:.3} reached at iteration {}", cli.target_score, iter,
            );
            break;
        }

        if advice.is_empty() {
            converged = true;
            convergence_reason = format!("No further tuning advice at iteration {}", iter);
            break;
        }

        if history.len() >= 3 {
            let recent: Vec<f64> = history[history.len()-3..].iter()
                .map(|r| r.score.overall).collect();
            let max_diff = recent.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0f64, f64::max);
            if max_diff < 0.001 {
                converged = true;
                convergence_reason = format!(
                    "Score stalled at {:.3} for 3 iterations", breakdown.overall,
                );
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
        params.layer_spacing = params.layer_spacing.clamp(50.0, 1000.0);
        params.block_spacing = params.block_spacing.clamp(30.0, 500.0);
        params.device_spacing = params.device_spacing.clamp(30.0, 300.0);
        params.label_threshold = params.label_threshold.clamp(100.0, 2000.0);
    }

    if !converged {
        convergence_reason = format!("Max iterations ({}) reached", cli.max_iter);
    }

    let summary = RestartSummary {
        restart: restart_idx,
        initial_params,
        best_score: round3(best_score),
        best_params,
        iterations: history.len(),
        converged,
        convergence_reason,
    };

    RestartResult { summary, history, best_schematic, best_breakdown }
}

/// Generate `n` deterministic spaced parameter sets covering the param
/// space. Restart 0 always returns the user-supplied initial point so the
/// simple --search-disabled greedy is one of the candidates.
fn generate_starting_points(initial: &TunableParams, n: usize) -> Vec<TunableParams> {
    let mut points = Vec::with_capacity(n);
    points.push(initial.clone());

    // Each preset is (layer, block, device, label_threshold,
    // adaptive_label_ratio). The adaptive ratio is co-tuned with
    // label_threshold so search can pick "absolute-only" vs "scaled by
    // bbox" regimes per circuit.
    const PRESETS: &[(f64, f64, f64, f64, f64)] = &[
        (300.0, 100.0,  60.0,  450.0, 0.30),  // wider columns, smaller devices, default ratio
        (100.0, 200.0, 100.0,  600.0, 0.50),  // narrow columns, more wire-friendly
        (400.0, 150.0,  80.0,  300.0, 0.60),  // big spread, aggressive wire preference
        (200.0,  60.0,  50.0,  900.0, 0.00),  // tight blocks, absolute threshold only
        (150.0, 250.0,  60.0,  300.0, 0.40),  // narrow + tall blocks
        (500.0, 100.0,  80.0, 1200.0, 0.30),  // very wide
        (250.0, 120.0, 100.0,  500.0, 0.50),  // medium-everything alt
        (100.0, 100.0,  60.0,  300.0, 0.30),  // compact in every dimension
    ];

    for &(ls, bs, ds, lt, ar) in PRESETS.iter().take(n.saturating_sub(1)) {
        points.push(TunableParams {
            layer_spacing: ls,
            block_spacing: bs,
            device_spacing: ds,
            label_threshold: lt,
            adaptive_label_ratio: ar,
        });
    }
    // If user asked for more restarts than presets, repeat the last preset
    // with small deterministic perturbations.
    while points.len() < n {
        let i = points.len() as f64;
        let last = points.last().unwrap().clone();
        points.push(TunableParams {
            layer_spacing:  (last.layer_spacing  * (1.0 + 0.05 * (i % 3.0 - 1.0))).clamp(50.0, 1000.0),
            block_spacing:  (last.block_spacing  * (1.0 + 0.05 * (i % 5.0 - 2.0))).clamp(30.0, 500.0),
            device_spacing: (last.device_spacing * (1.0 + 0.05 * (i % 4.0 - 1.5))).clamp(30.0, 300.0),
            label_threshold:(last.label_threshold* (1.0 + 0.05 * (i % 7.0 - 3.0))).clamp(100.0, 2000.0),
            adaptive_label_ratio: (last.adaptive_label_ratio + 0.1 * ((i % 6.0) - 3.0) / 3.0)
                .clamp(0.0, 0.8),
        });
    }

    points
}

fn main() {
    let cli = Cli::parse();

    let spice_text = std::fs::read_to_string(&cli.input)
        .unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", cli.input, e);
            std::process::exit(1);
        });
    let parse_result = SpiceParser::new().parse(&spice_text);
    let weights = ScoreWeights::default();

    let user_params = TunableParams {
        layer_spacing: cli.layer_spacing,
        block_spacing: cli.block_spacing,
        device_spacing: cli.device_spacing,
        label_threshold: cli.label_threshold,
        adaptive_label_ratio: cli.adaptive_label_ratio,
    };

    let starting_points = if cli.search {
        generate_starting_points(&user_params, cli.search_restarts.max(1))
    } else {
        vec![user_params.clone()]
    };

    let mut all_history: Vec<IterationRecord> = Vec::new();
    let mut all_summaries: Vec<RestartSummary> = Vec::new();
    let mut global_best_score = f64::NEG_INFINITY;
    let mut global_best_params = user_params.clone();
    let mut global_best_schematic: Option<Schematic> = None;
    let mut global_best_breakdown = ScoreBreakdown {
        overall: 0.0, overlap_score: 0.0, crossings_score: 0.0,
        aspect_ratio_score: 0.0, wire_length_score: 0.0,
        label_ratio_score: 0.0, symmetry_score: 0.0, power_convention_score: 0.0,
    };
    let mut initial_score = 0.0;

    for (restart_idx, start) in starting_points.iter().enumerate() {
        let prefix = if cli.quiet {
            None
        } else {
            Some(if cli.search {
                format!("[restart {}/{}] ", restart_idx + 1, starting_points.len())
            } else {
                String::new()
            })
        };
        let prefix_ref = prefix.as_deref();

        if !cli.quiet && cli.search {
            eprintln!(
                "[restart {}/{}] start: layer={:.0} block={:.0} device={:.0} label={:.0}",
                restart_idx + 1, starting_points.len(),
                start.layer_spacing, start.block_spacing,
                start.device_spacing, start.label_threshold,
            );
        }

        let result = optimize_from(
            restart_idx, start.clone(), &cli,
            &spice_text, &parse_result, &weights, prefix_ref,
        );

        // Initial score is the very first iteration of the very first restart.
        if restart_idx == 0 {
            initial_score = result.history.first()
                .map(|r| r.score.overall).unwrap_or(0.0);
        }

        if result.summary.best_score > global_best_score {
            global_best_score = result.summary.best_score;
            global_best_params = result.summary.best_params.clone();
            global_best_schematic = result.best_schematic;
            global_best_breakdown = result.best_breakdown;
        }

        if !cli.quiet && cli.search {
            eprintln!(
                "[restart {}/{}] best={:.3} after {} iters ({})",
                restart_idx + 1, starting_points.len(),
                result.summary.best_score, result.summary.iterations,
                result.summary.convergence_reason,
            );
        }

        all_summaries.push(result.summary);
        all_history.extend(result.history);

        // If we've already hit the target score, no need to keep restarting.
        if global_best_score >= cli.target_score {
            if !cli.quiet && cli.search {
                eprintln!(
                    "Stopping early: target score {:.3} reached after restart {}",
                    cli.target_score, restart_idx + 1,
                );
            }
            break;
        }
    }

    if let Some(schematic) = &global_best_schematic {
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

    let converged = global_best_score >= cli.target_score
        || all_summaries.iter().any(|s| s.converged);
    let convergence_reason = if global_best_score >= cli.target_score {
        format!("Target score {:.3} reached", cli.target_score)
    } else if cli.search {
        format!(
            "Best across {} restarts: {:.3}",
            all_summaries.len(), global_best_score,
        )
    } else {
        all_summaries.first()
            .map(|s| s.convergence_reason.clone())
            .unwrap_or_else(|| "no iterations run".to_string())
    };

    let report = ImproveReport {
        input_file: cli.input.clone(),
        iterations_run: all_history.len(),
        restarts: all_summaries.len(),
        converged,
        convergence_reason,
        initial_score: round3(initial_score),
        final_score: round3(global_best_score),
        improvement: round3(global_best_score - initial_score),
        best_params: global_best_params,
        best_score: global_best_breakdown,
        restart_summary: all_summaries,
        history: all_history,
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
