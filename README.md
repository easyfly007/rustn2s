# n2s — Netlist to Schematic

A standalone Rust tool that converts SPICE netlists into visual schematics. Outputs SVG, structured JSON, and KiCad schematics with zero GUI dependencies.

This is a Rust reimplementation of the N2S pipeline from the [MySchematic](https://github.com/) C++ project, eliminating the Qt dependency and producing a single statically-linked binary.

## Features

- **SPICE Netlist Parsing** — MOSFET (M), BJT (Q), R/C/L/D, voltage/current sources (V/I), controlled sources (E/F/G/H), subcircuit instances (X)
- **Analog Pattern Recognition** — Automatically identifies differential pairs, current mirrors, cascode pairs, and inverters
- **Hierarchical Layout** — Sugiyama-based layer assignment with barycenter crossing minimization
- **Manhattan Routing** — L-shaped wires for short nets, labels for long nets, power symbols for supply nets
- **Optional A\* Obstacle-Aware Routing** — `--obstacle-avoidance` falls back to grid A\* with bend & crossing penalties when an L-route would walk through a component body
- **14 Builtin Symbols** — nmos4, pmos4, npn, pnp, resistor, capacitor, inductor, diode, vsource, isource, vcvs, vccs, ccvs, cccs
- **SVG Output** — Dark theme, grid, legends, configurable scale
- **JSON Output** — Structured schematic data for downstream tools
- **KiCad Output** — Native `.kicad_sch` files, open directly in KiCad for editing

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/n2s
```

## Usage

### Basic

```bash
# Generate SVG
n2s circuit.sp -o schematic.svg

# Generate both SVG and JSON
n2s circuit.sp -o schematic.svg -o schematic.json

# Generate KiCad schematic
n2s circuit.sp -o schematic.kicad_sch

# Generate all formats at once
n2s circuit.sp -o schematic.svg -o schematic.json -o schematic.kicad_sch
```

### Options

```
n2s <INPUT> --output <OUTPUT>...

Options:
  --layer-spacing <F64>      Horizontal spacing between layers [default: 200]
  --block-spacing <F64>      Spacing between functional blocks [default: 100]
  --device-spacing <F64>     Spacing between devices within a block [default: 80]
  --grid <F64>               Grid snap size [default: 10]
  --label-threshold <F64>    Distance threshold for labels vs wires [default: 300]
  --no-patterns              Disable pattern recognition
  --scale <F64>              SVG scale factor [default: 1.0]
  --no-grid                  Hide grid in SVG output
  --hierarchical             Render subcircuit instances as boxes with ports
  --obstacle-avoidance       Use A* routing so wires avoid walking through
                             component bodies (opt-in; see "Obstacle-Aware
                             Routing" below for trade-offs)
  --bend-penalty <F64>       A* bend penalty when --obstacle-avoidance is on
                             [default: 0.5]
  --crossing-penalty <F64>   A* crossing penalty when --obstacle-avoidance is
                             on (higher = prefer detours over crossing
                             already-routed wires) [default: 20.0]
```

### Example SPICE Input

```spice
* CMOS Inverter
M1 out in VDD VDD pmos_3p3 W=2u L=0.35u
M2 out in GND GND nmos_3p3 W=1u L=0.35u
```

### Library Usage

```rust
use n2s::{convert_file, ConvertOptions};
use n2s::export::{svg, json, kicad};

let opts = ConvertOptions::default();
let schematic = convert_file("circuit.sp", &opts)?;

svg::render_to_file(&schematic, "circuit.svg", &svg::SvgOptions::default())?;
json::render_to_file(&schematic, "circuit.json")?;
kicad::render_to_file(&schematic, "circuit.kicad_sch", &Default::default())?;
```

## Architecture

```
SPICE file
    │
    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Parser  │───▶│ Analyzer │───▶│  Placer  │───▶│  Router  │───▶│  Export  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
  Tokenize &      Pattern        Sugiyama        Manhattan       SVG / JSON /
  parse SPICE     recognition    hierarchical    routing &       KiCad
  devices         + HAC          layout          labeling
                  clustering
```

| Module | Description |
|--------|-------------|
| `parser` | SPICE tokenizer, line continuation, device & subcircuit parsing |
| `analyzer` | Pattern recognition (diff pair, mirror, cascode, inverter) + HAC clustering |
| `placer` | DAG construction, layer assignment, crossing minimization, coordinate assignment |
| `router` | Net routing (wires, labels, power symbols), pin mapping with transforms |
| `model` | Geometry primitives, symbol definitions, schematic data structures |
| `export` | SVG renderer (dark theme), JSON serializer, and KiCad `.kicad_sch` exporter |

See [docs/architecture.md](docs/architecture.md) for detailed design documentation, and [docs/learning_resources.md](docs/learning_resources.md) for a curated reading list (algorithms, papers, open-source projects, courses) to study netlist-to-schematic systematically.

## Obstacle-Aware Routing (`--obstacle-avoidance`)

By default the router uses **L-routing**: each wire is a single horizontal-then-vertical (or vertical-then-horizontal) Manhattan path picked to minimize crossings against already-routed wires. L-routing is fast and produces short wires, but it does not know about component geometry — wires can walk straight through MOSFET bodies if the direct route happens to pass over one.

`--obstacle-avoidance` enables a second routing layer:

1. The router rasterizes every placed component's bounding rectangle into a grid (default cell size 10 units, matching the placer grid).
2. For each MST edge in a signal net, the L-route is tried first. If it lands clear of every body, it is kept (this is the common case).
3. If the L-route would pass through a body, the router falls back to **grid A\*** with two penalties:
   - `--bend-penalty` (default `0.5`) — extra cost per direction change, so paths prefer straight runs over staircases.
   - `--crossing-penalty` (default `20.0`) — extra cost when stepping perpendicular through a cell already covered by an earlier wire, so paths prefer detours over creating visible crossings (still allowed when no detour exists).
4. Each routed wire stamps its cells with a horizontal/vertical orientation flag, feeding the next iteration's crossing penalty.
5. MST edges are routed shortest-first so tightly-constrained pin pairs commit before more flexible ones.

**Trade-off.** A\* trades wire length for visual cleanliness:
- ✅ No wires walk through component bodies (verified by an integration test on example 04).
- ⚠️ Total wire length usually grows by 20–60% on circuits where the L-router was crossing bodies, because the detour goes around.
- ⚠️ On dense op-amp circuits (07, 08, 10) the placement leaves no room to detour around earlier wires, so A\* still has to cross — and the extra wire length drags the eval score down by 0.05–0.1.

**Recommended use:** turn on for human-readable schematic output (the project's actual goal); leave off for `n2s-improve`-style score-driven optimization. The flag has no effect on simple linear circuits or sparse layouts, where the L-route is already obstacle-free.

See [docs/routing_improvement.md](docs/routing_improvement.md) for the full design and per-circuit benchmark numbers.

## Evaluating Schematic Quality

The `n2s-eval` binary evaluates layout quality of generated schematics by comparing the original netlist against the JSON output.

### Build

```bash
cargo build --release
# Binary at target/release/n2s-eval
```

### Usage

```bash
# Single circuit, pretty-printed
n2s-eval -n circuit.sp -s schematic.json --pretty

# Compact JSON (for piping to another tool)
n2s-eval -n circuit.sp -s schematic.json | jq '.symmetry.overall_score'
```

### Full Pipeline (generate + evaluate)

```bash
n2s circuit.sp -o circuit.svg -o circuit.json
n2s-eval -n circuit.sp -s circuit.json --pretty
```

### Batch Evaluate All Examples

```bash
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  n2s "$f" -o "output/${name}.json"
  echo "=== $name ==="
  n2s-eval -n "$f" -s "output/${name}.json" --pretty
done
```

### Options

```
n2s-eval --netlist <SPICE_FILE> --schematic <JSON_FILE> [--pretty]

Options:
  -n, --netlist <PATH>      Path to the original SPICE netlist
  -s, --schematic <PATH>    Path to the generated JSON schematic
  --pretty                   Pretty-print the JSON output
```

### Metrics

| Metric | Description |
|--------|-------------|
| `connectivity` | Net count match, missing connections, orphan labels |
| `component_overlap` | Pairwise bounding box overlap detection |
| `wire_crossings` | Wire segment intersection count |
| `wire_length` | Total, average, min, max wire length |
| `wire_bends` | Bend count per wire and overall |
| `bounding_box` | Width, height, area, aspect ratio |
| `label_usage` | Label pairs vs direct wires ratio |
| `symmetry` | Matched device pair placement score (0–1) |
| `power_convention` | PMOS-above-NMOS placement score (0–1) |

Output is structured JSON for consumption by downstream tools or agents.

See [docs/examples.md](docs/examples.md) for test circuits and run commands.

## Iterative Layout Optimization

The `n2s-improve` binary automates the feedback loop: generate schematic → evaluate quality → adjust parameters → regenerate. It maximizes a weighted quality score (0.0–1.0) combining overlap, wire crossings, aspect ratio, wire length, label usage, symmetry, and power convention metrics.

### Build

```bash
cargo build --release
# Binary at target/release/n2s-improve
```

### Usage

```bash
# Optimize and output best SVG
n2s-improve circuit.sp -o circuit.svg

# With JSON output and detailed report
n2s-improve circuit.sp -o circuit.svg --json circuit.json --pretty

# Custom targets
n2s-improve circuit.sp -o circuit.svg --target-score 0.95 --max-iter 20

# Extract optimized parameters for use with n2s directly
n2s-improve circuit.sp --quiet | jq '.best_params'
```

### Options

```
n2s-improve <INPUT> [OPTIONS]

Options:
  -o, --output <FILE>        Output SVG file (best result)
      --json <FILE>          Output JSON schematic file (best result)
      --max-iter <N>         Maximum iterations [default: 10]
      --target-score <F>     Target quality score [default: 0.9]
      --layer-spacing <F>    Initial layer spacing [default: 200]
      --block-spacing <F>    Initial block spacing [default: 100]
      --device-spacing <F>   Initial device spacing [default: 80]
      --label-threshold <F>  Initial label threshold [default: 300]
      --pretty               Pretty-print the JSON report
      --quiet                Suppress iteration logs
```

See [docs/improve.md](docs/improve.md) for detailed scoring system, tuning rules, and benchmark results.

## License

MIT
