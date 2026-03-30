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
│   ├── architecture.md        ← this file
│   ├── examples.md            ← test circuit documentation
│   └── improve.md             ← n2s-improve documentation
├── tests/
│   └── examples/              ← 11 SPICE test netlists
└── src/
    ├── lib.rs                 ← Library entry, pipeline orchestration
    ├── main.rs                ← CLI: n2s (netlist → schematic)
    ├── bin/
    │   ├── eval.rs            ← CLI: n2s-eval (layout quality metrics)
    │   └── improve.rs         ← CLI: n2s-improve (iterative optimizer)
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
    ├── eval/
    │   ├── mod.rs             ← Evaluation module entry, EvalReport
    │   ├── connectivity.rs    ← Net connectivity verification
    │   ├── overlap.rs         ← Component overlap detection
    │   ├── wire_crossings.rs  ← Wire segment intersection counting
    │   ├── wire_length.rs     ← Wire length statistics
    │   ├── wire_bends.rs      ← Wire bend counting
    │   ├── bounding_box.rs    ← Bounding box metrics
    │   ├── label_usage.rs     ← Label usage statistics
    │   ├── symmetry.rs        ← Matched device pair scoring
    │   ├── power_convention.rs← PMOS-above-NMOS check
    │   └── score.rs           ← Quality scoring and tuning advisor
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

---

## Layout Quality Evaluation (`n2s-eval`)

A standalone binary that reads the original SPICE netlist and generated JSON schematic, then outputs structured JSON metrics. See [examples.md](examples.md) for test circuits and evaluation results.

### Metrics

| Metric | Description |
|--------|-------------|
| `connectivity` | Net count match, missing connections, orphan/duplicate labels |
| `component_overlap` | Pairwise bounding box overlap detection |
| `wire_crossings` | Wire segment intersection count (excluding junctions) |
| `wire_length` | Total, average, min, max wire length |
| `wire_bends` | Bend count per wire and overall |
| `bounding_box` | Width, height, area, aspect ratio |
| `label_usage` | Label pairs vs direct wires ratio |
| `symmetry` | Matched device pair placement score (0–1) |
| `power_convention` | PMOS-above-NMOS placement score (0–1) |

### Key Findings from Current Implementation

Evaluation of 11 test circuits revealed the following quality patterns:

| Finding | Affected Examples | Severity |
|---------|-------------------|----------|
| Extreme vertical aspect ratios (up to 42:1) | 01, 03, 06, 11 | High |
| Low symmetry for matched device pairs | 05, 06, 08 | High |
| Duplicate labels for same net (e.g., `vout` x8) | 07, 08, 10 | Medium |
| Wire crossings in complex circuits | 08, 09 | Medium |
| Sources stacked vertically, disconnected from topology | 07 | Low |

---

## TODO: Improvement Roadmap

### Phase 1 — Parameter Auto-Tuning (`n2s-improve`) ✓ COMPLETED

Implemented as `src/bin/improve.rs` with scoring in `src/eval/score.rs`. The binary runs an automated feedback loop: `n2s → n2s-eval → score → adjust params → re-run n2s`.

See [docs/improve.md](improve.md) for full documentation, scoring system, and benchmark results.

**Summary**: Parameter tuning effectively improves medium-complexity circuits (scores ≥0.9 for op-amps and hierarchical designs). However, simple linear circuits and symmetry issues cannot be resolved by parameter tuning alone — they require the algorithmic changes in Phases 2–3.

### Phase 2 — Placer Algorithm Improvements

#### Phase 2.1 — Multi-Column Grid Layout ✓ COMPLETED

**Problem**: When many blocks land in the same DAG layer (common for circuits with few inter-block dependencies), they were stacked in a single vertical column, creating extreme aspect ratios (up to 42:1).

**Solution**: Added `compute_grid_columns()` to `src/placer/mod.rs`. When a layer has 3+ blocks, it computes the optimal number of columns to achieve a target aspect ratio of ~1.5, then distributes blocks across columns using a greedy height-balancing algorithm.

**Results**:

| Example | Before AR | After AR | Score Before | Score After |
|---------|-----------|----------|-------------|-------------|
| 06 BJT diff pair | 22.86 | 1.79 | 0.677 | 0.872 |
| 11 RLC controlled | 42.0 | 1.05 | 0.658 | 0.949 |
| 04 NMOS CS amp | 3.19 | 1.18 | 0.784 | 0.818 |
| 10 opamp feedback | 1.14 | 1.81 | 0.920 | 0.946 |

**Now 6/11 examples score ≥0.9** (up from 3/11 before).

#### Phase 2.2 — Cross-Block Symmetry Alignment (DONE)

**Problem:** Matched device pairs (e.g., Q1/Q2 in diff pair, R1/R2 loads) placed in different functional blocks end up at different y-coordinates, causing low symmetry scores.

**Solution:** Added `align_matched_pairs()` to the placer, called after absolute coordinate assignment (step 6). The algorithm:

1. Groups all devices by a matching key: `(symbol_name, W, L, model)` — devices with identical electrical properties are considered matched
2. For groups of exactly 2 devices in different blocks, aligns their y-coordinates by shifting the smaller block
3. The shift is applied to all devices in the moved block, preserving internal block layout
4. Coordinates are snapped to the grid after alignment

**API:** Added `place_with_devices()` method alongside the existing `place()` (which remains as a backwards-compatible wrapper). The `convert()` pipeline now passes device info to the placer.

**Results after Phase 2.2:**

| Example | Before 2.2 | After 2.2 | Delta |
|---------|-----------|----------|-------|
| 01 voltage divider | 0.702 | 0.702 | — |
| 02 RC filter | 0.860 | 0.860 | — |
| 03 half-wave rectifier | 0.844 | 0.844 | — |
| 04 NMOS CS amp | 0.818 | 0.818 | — |
| 05 current mirror | 0.909 | 0.909 | — |
| 06 BJT diff pair | 0.872 | 0.870 | -0.002 |
| **07 two-stage opamp** | 0.920 | **0.929** | **+0.009** |
| 08 bandgap reference | 0.797 | 0.797 | — |
| 09 inverter chain | 0.916 | 0.916 | — |
| 10 opamp feedback | 0.946 | 0.946 | — |
| **11 RLC controlled** | 0.949 | **1.000** | **+0.051** |

**Now 7/11 examples score ≥0.9** (up from 6/11 before Phase 2.2). Example 11 achieves a perfect 1.0 score.

**Note:** The symmetry alignment can introduce overlaps when blocks are shifted (visible in example 06's initial iteration). The `n2s-improve` tuner resolves these by increasing spacing, so the overall score remains stable.

#### Phase 2.3 — PMOS-Above-NMOS Block Ordering (DONE)

**Problem:** Block ordering within a layer does not consider device polarity, so NMOS blocks can appear above PMOS blocks, violating the standard schematic convention.

**Solution:** Added `sort_blocks_by_polarity()` to the placer, called after crossing minimization (step 3.5). The algorithm:

1. Classifies each block by polarity: PMOS-only (pmos4/pnp) → top, NMOS-only (nmos4/npn) → bottom, mixed/passive → middle
2. Stable-sorts blocks within each layer by polarity, preserving the crossing-minimized order within the same group

**Results after Phase 2.3:**

| Example | Before 2.3 | After 2.3 | Delta |
|---------|-----------|----------|-------|
| 04 NMOS CS amp | 0.818 | **0.821** | **+0.003** |
| All others | unchanged | unchanged | — |

Power convention was already at 1.0 for all examples. The polarity sort primarily improves layout aesthetics and slightly improves aspect ratio for mixed-polarity circuits.

#### Phase 2.4 — Remaining Placer Issues (TODO)

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| **Sources disconnected from topology** | Voltage/current sources form their own blocks stacked vertically at x=0 | Place source blocks adjacent to the blocks they drive, not in a separate column |

### Phase 3 — Router Algorithm Improvements

These issues require changes to `src/router/mod.rs`:

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| **Duplicate labels per net** | `route_signal_net()` creates a label pair for every pin-to-anchor connection beyond threshold, resulting in e.g., 8 labels for a 5-pin net | Deduplicate: emit one label at the anchor and one at each remote pin, not one pair per connection. For N pins, emit N labels (not 2*(N-1)) |
| **Wire crossings** | L-routing always goes horizontal-first, no consideration of other wires | Add crossing detection during routing; try vertical-first L-route as alternative and pick the one with fewer crossings |
| **Star topology creates long wires** | All pins connect to first pin (anchor), which may not be geometrically central | Use minimum spanning tree or Steiner tree instead of star topology for multi-pin nets |

### Phase 4 — Advanced Features

| Feature | Description |
|---------|-------------|
| **Hierarchical schematic rendering** | Currently subcircuits are flattened; render subcircuit instances as boxes with labeled ports |
| **Signal flow direction** | Enforce left-to-right signal flow: inputs on left, outputs on right |
| **Net-aware label placement** | Place labels at pin positions with offset to avoid overlapping component graphics |
| **Interactive parameter search** | `n2s-improve` tries multiple parameter combinations and picks the best score |
