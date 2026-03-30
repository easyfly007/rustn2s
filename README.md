# n2s — Netlist to Schematic

A standalone Rust tool that converts SPICE netlists into visual schematics. Outputs SVG and structured JSON with zero GUI dependencies.

This is a Rust reimplementation of the N2S pipeline from the [MySchematic](https://github.com/) C++ project, eliminating the Qt dependency and producing a single statically-linked binary.

## Features

- **SPICE Netlist Parsing** — MOSFET (M), BJT (Q), R/C/L/D, voltage/current sources (V/I), controlled sources (E/F/G/H), subcircuit instances (X)
- **Analog Pattern Recognition** — Automatically identifies differential pairs, current mirrors, cascode pairs, and inverters
- **Hierarchical Layout** — Sugiyama-based layer assignment with barycenter crossing minimization
- **Manhattan Routing** — L-shaped wires for short nets, labels for long nets, power symbols for supply nets
- **14 Builtin Symbols** — nmos4, pmos4, npn, pnp, resistor, capacitor, inductor, diode, vsource, isource, vcvs, vccs, ccvs, cccs
- **SVG Output** — Dark theme, grid, legends, configurable scale
- **JSON Output** — Structured schematic data for downstream tools

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
use n2s::export::{svg, json};

let opts = ConvertOptions::default();
let schematic = convert_file("circuit.sp", &opts)?;

svg::render_to_file(&schematic, "circuit.svg", &svg::SvgOptions::default())?;
json::render_to_file(&schematic, "circuit.json")?;
```

## Architecture

```
SPICE file
    │
    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Parser  │───▶│ Analyzer │───▶│  Placer  │───▶│  Router  │───▶│  Export  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
  Tokenize &      Pattern        Sugiyama        Manhattan       SVG / JSON
  parse SPICE     recognition    hierarchical    routing &
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
| `export` | SVG renderer (dark theme) and JSON serializer |

See [docs/architecture.md](docs/architecture.md) for detailed design documentation.

## License

MIT
