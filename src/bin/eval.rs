use clap::Parser;
use n2s::eval;
use n2s::eval::score::{compute_score, ScoreWeights};
use n2s::model::Schematic;
use n2s::parser::SpiceParser;

#[derive(Parser)]
#[command(name = "n2s-eval", about = "Evaluate schematic layout quality")]
struct Cli {
    /// Path to the original SPICE netlist file
    #[arg(short = 'n', long = "netlist")]
    netlist: String,

    /// Path to the generated JSON schematic file
    #[arg(short = 's', long = "schematic")]
    schematic: String,

    /// Pretty-print the JSON output
    #[arg(long)]
    pretty: bool,

    /// Emit a single-line sub-score profile instead of the full eval JSON.
    /// Format:
    ///   <circuit-name> safety=PASS|FAIL profile=<7 sub-scores> overall=<float>
    /// Useful for triage across many circuits and for the metric-reform
    /// proposal in docs/metric_reform.md — separates the three "safety"
    /// sub-scores (overlap, symmetry, power_convention) from the four
    /// continuous quality metrics.
    #[arg(long)]
    profile: bool,
}

fn main() {
    let cli = Cli::parse();

    // Parse SPICE netlist
    let spice_text = std::fs::read_to_string(&cli.netlist)
        .unwrap_or_else(|e| {
            eprintln!("Error reading netlist {}: {}", cli.netlist, e);
            std::process::exit(1);
        });
    let parse_result = SpiceParser::new().parse(&spice_text);

    // Load JSON schematic
    let json_text = std::fs::read_to_string(&cli.schematic)
        .unwrap_or_else(|e| {
            eprintln!("Error reading schematic {}: {}", cli.schematic, e);
            std::process::exit(1);
        });
    let schematic: Schematic = serde_json::from_str(&json_text)
        .unwrap_or_else(|e| {
            eprintln!("Error parsing schematic JSON: {}", e);
            std::process::exit(1);
        });

    // Evaluate
    let report = eval::evaluate(&parse_result, &schematic);

    if cli.profile {
        let weights = ScoreWeights::default();
        let breakdown = compute_score(&report, &weights);
        // Tier 1 (safety): hard pass/fail. Any of these below 1.0 means
        // the schematic has a real bug, not a quality issue.
        let safety_pass = breakdown.overlap_score >= 0.999
            && breakdown.symmetry_score >= 0.999
            && breakdown.power_convention_score >= 0.999;
        let circuit_name = std::path::Path::new(&cli.netlist)
            .file_stem().and_then(|s| s.to_str()).unwrap_or("circuit");
        // Compact one-line profile: safety | the four continuous quality
        // sub-scores | overall (legacy weighted sum).
        println!(
            "{circuit_name:<32} safety={} ovrlp={:.2} sym={:.2} pwr={:.2} | ar={:.2} cross={:.2} wire={:.2} lbl={:.2} | overall={:.3}",
            if safety_pass { "PASS" } else { "FAIL" },
            breakdown.overlap_score,
            breakdown.symmetry_score,
            breakdown.power_convention_score,
            breakdown.aspect_ratio_score,
            breakdown.crossings_score,
            breakdown.wire_length_score,
            breakdown.label_ratio_score,
            breakdown.overall,
        );
        return;
    }

    let output = if cli.pretty {
        serde_json::to_string_pretty(&report).unwrap()
    } else {
        serde_json::to_string(&report).unwrap()
    };
    println!("{}", output);
}
