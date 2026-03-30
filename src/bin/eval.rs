use clap::Parser;
use n2s::eval;
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

    // Output
    let output = if cli.pretty {
        serde_json::to_string_pretty(&report).unwrap()
    } else {
        serde_json::to_string(&report).unwrap()
    };
    println!("{}", output);
}
