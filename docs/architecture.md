# n2s — Netlist to Schematic Architecture

## Overview

n2s is a standalone Rust tool that converts SPICE netlists into visual schematics. It outputs SVG (immediate visual verification) and structured JSON (downstream tool consumption). The tool is implemented as both a library crate and a CLI binary.

**Design principle**: Pure computation pipeline with zero GUI dependencies, producing a single statically-linked binary (~1.5 MB).

## Relationship to MySchematic (C++)

n2s is a Rust reimplementation of the N2S pipeline from the MySchematic project. The C++ implementation lives in:

| C++ Module | File | Rust Module |
|-----------|------|-------------|
| SpiceParser | `lib/src/import/spice_parser.cpp` (435 lines) | `src/parser/mod.rs` |
| CircuitAnalyzer | `lib/src/import/circuit_analyzer.cpp` (553 lines) | `src/analyzer/mod.rs` |
| SchematicPlacer | `lib/src/import/schematic_placer.cpp` (479 lines) | `src/placer/mod.rs` |
| SchematicRouter | `lib/src/import/schematic_router.cpp` (185 lines) | `src/router/mod.rs` |
| NetlistImporter | `lib/src/import/netlist_importer.cpp` (167 lines) | `src/lib.rs` (pipeline) |
| SchematicRenderer | `lib/src/export/schematic_renderer.cpp` (606 lines) | `src/export/svg.rs` |
| BuiltinSymbols | `lib/src/symbol/builtin_symbols.cpp` (559 lines) | `src/model/symbol.rs` |

**Key differences from C++**:
- No Qt dependency (C++ uses QString, QJsonDocument, QPointF, etc.)
- Symbol definitions compiled in (no runtime library loading)
- Output formats: SVG + JSON (not `.msch.json` editor format)
- Rust's serde handles all serialization

---

## Project Structure

```
n2s/
├── Cargo.toml
├── docs/
│   └── architecture.md        ← this file
└── src/
    ├── lib.rs                 ← Library entry, pipeline orchestration
    ├── main.rs                ← CLI (clap)
    ├── model/
    │   ├── mod.rs             ← Public re-exports
    │   ├── geometry.rs        ← Point, Rect, transform, snap_to_grid
    │   ├── symbol.rs          ← SymbolDef, SymbolPin, SymbolGraphic, 14 builtin symbols
    │   └── schematic.rs       ← Schematic, Component, Wire, Label, PowerSymbol, Junction
    ├── parser/
    │   └── mod.rs             ← SPICE parser: tokenizer, device parsing, MOS/BJT inference
    ├── analyzer/
    │   └── mod.rs             ← Circuit topology: pattern recognition + HAC clustering
    ├── placer/
    │   └── mod.rs             ← Sugiyama layout: DAG, layers, crossing min, templates
    ├── router/
    │   └── mod.rs             ← Manhattan routing, power symbols, labels, junctions
    └── export/
        ├── mod.rs
        ├── svg.rs             ← SVG renderer with dark theme
        └── json.rs            ← Structured JSON export
```

---

## Data Flow Pipeline

```
SPICE text/file
       │
       ▼
┌─────────────┐
│   Parser    │  → ParseResult { devices: Vec<SpiceDevice>, subcircuits, title, ... }
└──────┬──────┘
       ▼
┌─────────────┐
│  Analyzer   │  → Vec<FunctionalBlock> { type, device_indices, input/output_nets, ... }
│             │    + HashSet<String> (power nets)
└──────┬──────┘
       ▼
┌─────────────┐
│   Placer    │  → PlacementResult { placements: Vec<DevicePlacement>, bounding_rect }
└──────┬──────┘
       ▼
┌─────────────┐
│   Router    │  → Schematic { components, wires, labels, power_symbols, junctions }
└──────┬──────┘
       ▼
┌─────────────┐
│   Export    │  → .svg file  and/or  .n2s.json file
└─────────────┘
```

---

## Module Details

### 1. Parser (`src/parser/mod.rs`)

**Input**: Raw SPICE netlist text.

**Processing**:
1. **Line merging**: `+` continuation lines merged with previous line
2. **Comment removal**: `*` full-line comments, `$` / `;` inline comments
3. **Tokenization**: Whitespace/comma splitting, respecting quoted strings
4. **Title extraction**: First line per SPICE convention
5. **Directive handling**: `.subckt`/`.ends`, `.param`, `.include`/`.lib`
6. **Device parsing**: Type-specific for each SPICE device letter

**Device types supported**:

| Letter | Device | Nodes | Special |
|--------|--------|-------|---------|
| M | MOSFET | D G S B | model name + W/L params |
| Q | BJT | C B E [substrate] | 3 or 4 terminal |
| R | Resistor | P N | value |
| C | Capacitor | P N | value |
| L | Inductor | P N | value |
| D | Diode | A K | model |
| V | Voltage source | P N | value |
| I | Current source | P N | value |
| E/G/H/F | Controlled sources | 4 nodes | gain/value |
| X | Subcircuit instance | N nodes | subckt name |

**MOS type inference** (priority order):
1. Model name contains "nch"/"nmos" → NMOS, "pch"/"pmos" → PMOS
2. Bulk node is GND/VSS → NMOS, VDD/VCC → PMOS
3. Default: NMOS

### 2. Analyzer (`src/analyzer/mod.rs`)

**Two-stage architecture**:

**Stage 1: Global Pattern Extraction** (greedy, priority-ordered)

Scans all unassigned devices in this order:

| Priority | Pattern | Detection Criteria |
|---------|---------|-------------------|
| 1 | **Differential Pair** | Two MOSFETs: same type + same source (non-power) + different gates + different drains. Optional tail current source. |
| 2 | **Current Mirror** | ≥2 MOSFETs: same type + same gate + same source. At least one diode-connected (drain=gate). |
| 3 | **Cascode Pair** | Two MOSFETs: same type + upper.source = lower.drain + different gates. |
| 4 | **Inverter** | NMOS + PMOS: same gate + same drain + both sources on power nets. |

**Stage 2: Hierarchical Agglomerative Clustering (HAC)**

For remaining unassigned devices:
1. Build net adjacency graph (excluding power nets)
2. Score = `shared_net_weight / min(|cluster_A|, |cluster_B|)`
3. Merge best pair if score ≥ threshold (default 0.5)
4. Stop when: score < threshold, or merged size > max (default 6)
5. Annotate: single device → `SingleDevice`, multi → `Unknown`

**Power net identification**: Hard-coded set (0, gnd, gnd!, vss, vss!, vdd, vdd!, vcc, vcc!, avdd, avss) + voltage source terminal nodes.

### 3. Placer (`src/placer/mod.rs`)

**Sugiyama hierarchical layout** in 5 phases:

1. **DAG Construction**: Blocks as nodes, edges from output-net producer to input-net consumer. Cycle removal via DFS back-edge reversal.

2. **Layer Assignment**: Kahn's topological sort → longest path gives layer index per block.

3. **Crossing Minimization**: Barycenter heuristic with 4 iterations of forward/backward sweeps.

4. **Block-Internal Templates**:

| Block Type | Layout |
|-----------|--------|
| DiffPair | M1 left, M2 right (symmetric), tail below center |
| CurrentMirror | Horizontal chain, reference on left |
| CascodePair | Vertical stack (upper on top, lower below) |
| Inverter | PMOS on top (mirrored), NMOS below |
| SingleDevice | Centered |
| Unknown | Vertical stack |

5. **Absolute Coordinates**: Layer index × `layer_spacing` for X, blocks stacked vertically within layer, all snapped to grid.

### 4. Router (`src/router/mod.rs`)

**Responsibilities**:
- Create `Component` objects from placements
- Map SPICE nodes to pin world positions using symbol pin offsets + rotation/mirror transform
- Route each net

**Routing strategy**:

| Net Type | Distance | Action |
|---------|----------|--------|
| Power (GND/VDD/VSS/VCC) | any | Place `PowerSymbol` at each pin |
| Signal | < threshold (300) | L-shaped Manhattan wire (horizontal first) |
| Signal | ≥ threshold | `Label` at both endpoints |

**Star topology**: All pins connected to first pin (anchor). Junction added at anchor if >2 pins.

### 5. Export

**SVG** (`src/export/svg.rs`):
- Dark theme by default (configurable via `SvgTheme`)
- Renders: grid dots, wires (polylines), symbol graphics (lines, rects, circles, arcs, polylines, text), pin dots, instance names, power symbols (GND 3-bar / VDD bar), labels (rounded boxes), junctions, legend
- Full rotation/mirror transform for all symbol graphics

**JSON** (`src/export/json.rs`):
- Serializes `Schematic` struct via serde
- Contains: components (with properties), wires, labels, power_symbols, junctions
- Deterministic output via `serde_json::to_string_pretty`

---

## Builtin Symbols

14 analog circuit symbols with pin definitions and graphics:

| Symbol | Category | Pins |
|--------|----------|------|
| nmos4, pmos4 | Transistors | G, D, S, B |
| npn, pnp | Transistors | B, C, E |
| resistor | Passives | P, N |
| capacitor | Passives | P, N |
| inductor | Passives | P, N |
| diode | Semiconductors | A, K |
| vsource, isource | Sources | P, N |
| vcvs, vccs, ccvs, cccs | Sources | NP, NN, CP, CN |

Pin offsets are defined in schematic coordinates. The `pin_names_for_symbol()` function maps SPICE node order to symbol pin names (e.g., MOSFET D,G,S,B → pins D,G,S,B).

---

## CLI Interface

```
n2s <input.sp> -o <output.svg> [-o <output.json>] [options]

Options:
  --layer-spacing <f64>     Horizontal spacing between layers (default: 200)
  --block-spacing <f64>     Spacing between functional blocks (default: 100)
  --device-spacing <f64>    Spacing within blocks (default: 80)
  --grid <f64>              Grid snap size (default: 10)
  --label-threshold <f64>   Distance for label substitution (default: 300)
  --no-patterns             Disable pattern recognition
  --scale <f64>             SVG scale factor (default: 1.0)
  --no-grid                 Hide grid in SVG
```

**Library usage** (Rust):
```rust
use n2s::{convert_file, ConvertOptions};
use n2s::export::{svg, json};

let opts = ConvertOptions::default();
let schematic = convert_file("circuit.sp", &opts)?;
svg::render_to_file(&schematic, "circuit.svg", &svg::SvgOptions::default())?;
json::render_to_file(&schematic, "circuit.json")?;
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| clap 4 | CLI argument parsing (derive mode) |
| serde 1 | Serialization traits |
| serde_json 1 | JSON output |

No C dependencies. No runtime dependencies. Single static binary.

---

## Test Coverage

Unit tests:
- `parser::tests` — SPICE parsing for inverter, diff_pair
- `analyzer::tests` — Inverter detection, diff pair + tail detection

Integration validation:
- `inverter.sp` → SVG + JSON
- `diff_pair.sp` → SVG + JSON
- `twostage_opamp.sp` → SVG + JSON
- `bandgap.sp` → SVG
- `folded_cascode.sp` → SVG

All test SPICE files from the MySchematic C++ test suite.
