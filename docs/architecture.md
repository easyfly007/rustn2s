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

#### Phase 2.4 — Source Proximity (DONE)

**Problem:** Voltage/current sources form isolated blocks in the DAG because `identify_power_nets()` marks all V source terminals as power nets, causing empty input/output nets and no DAG connections. Sources pile up at layer 0.

**Solution:** Two changes:

1. **Analyzer** (`src/analyzer/mod.rs`): V/I sources now always include their positive terminal (node[0]) as an output net, giving them connectivity information in their `all_nets` set.

2. **Placer** (`src/placer/mod.rs`): Added `fix_isolated_source_layers()` after layer assignment. For blocks with no DAG edges, finds the non-isolated block sharing the most nets (via `all_nets` intersection) and assigns the isolated block to the same layer. This places sources alongside the circuit blocks they drive instead of in a separate column.

**Results after Phase 2.4:**

| Example | Before 2.4 | After 2.4 | Delta |
|---------|:---:|:---:|:---:|
| **05 current mirror** | 0.930 | **0.966** | **+0.036** |
| **09 inverter chain** | 0.916 | **0.991** | **+0.075** |
| 07 two-stage opamp | 0.958 | 0.956 | -0.002 |
| 10 opamp feedback | 0.950 | 0.936 | -0.014 |
| All others | unchanged | unchanged | — |

**Now 9/11 examples score ≥0.9** (up from 8/11 before Phase 2.4).

### Phase 3 — Router Algorithm Improvements (DONE)

Three improvements to `src/router/mod.rs`:

#### 3.1 Label Deduplication

**Problem:** The old star-topology router emitted a label pair (anchor + target) for every long-distance pin connection. A 5-pin net produced up to 8 labels instead of 5.

**Solution:** Track which pins need labels via a `HashSet`, then emit exactly one label per unique position. Example 07 went from 28 labels to 8.

#### 3.2 Adaptive L-Route Orientation

**Problem:** L-routing always went horizontal-first, which could create crossings when a vertical-first route would be crossing-free.

**Solution:** `l_route_best()` tries both horizontal-first and vertical-first orientations and picks the one with fewer crossings against already-routed wires. Ties default to horizontal-first.

#### 3.3 Minimum Spanning Tree Routing

**Problem:** Star topology connected all pins to pin[0], creating unnecessarily long wires when pin[0] was not geometrically central.

**Solution:** Replaced star topology with Prim's MST algorithm. Each net's pins are connected via the minimum total wire length spanning tree, reducing overall wire length and producing more natural routing patterns.

**Results after Phase 3:**

| Example | Before Phase 3 | After Phase 3 | Delta | Key Change |
|---------|:---:|:---:|:---:|------------|
| 04 NMOS CS amp | 0.821 | **0.825** | +0.004 | Fewer labels |
| 05 current mirror | 0.909 | **0.930** | +0.021 | MST routing |
| 06 BJT diff pair | 0.870 | **0.883** | +0.013 | MST + fewer labels |
| 07 two-stage opamp | 0.929 | **0.958** | +0.029 | Labels: 28→8 |
| 08 bandgap ref | 0.797 | **0.808** | +0.011 | Labels: 16→8 |
| 09 inverter chain | 0.916 | 0.916 | — | Crossing eliminated |
| 10 opamp feedback | 0.946 | **0.950** | +0.004 | Labels: 16→6 |

**Now 8/11 examples score ≥0.9** (up from 7/11 before Phase 3).

### Phase 4 — Advanced Features

#### Phase 4.1 — Hierarchical Schematic Rendering (DONE)

**Problem:** Subcircuit instances (X devices) were always flattened — the pipeline used the first subcircuit's internal devices, discarding the top-level view.

**Solution:** Added `--hierarchical` flag that renders X instances as rectangular boxes with labeled ports:

1. **Dynamic symbol generation** (`builtin_symbols::create_subcircuit_symbol`): Creates a `SymbolDef` for each `.subckt` definition — a rectangle with ports split left/right, stub lines, and the subcircuit name centered.

2. **Mode selection** (`lib.rs`): When `--hierarchical` is set and the netlist has both X instances and `.subckt` definitions, uses top-level devices with generated subcircuit symbols. Otherwise, falls back to flat mode.

3. **Router integration**: `route_with_subcircuits()` maps X device nodes directly to subcircuit symbol pin positions.

4. **SVG export**: `render_to_svg_with_symbols()` / `render_to_file_with_symbols()` accept extra symbols alongside builtins.

**Usage:**
```bash
# Flat mode (default — expands subcircuits to individual devices)
n2s circuit.sp -o schematic.svg

# Hierarchical mode — shows X instances as boxes
n2s circuit.sp -o schematic.svg --hierarchical
```

**Backwards compatible:** Default mode unchanged. Existing scores unaffected. `n2s-improve` always uses flat mode.

#### Phase 4.2 — Signal Flow Direction (DONE)

**Problem:** The longest-path-from-source layer assignment (ASAP) places each block at its earliest feasible layer. Terminal sinks (output load caps, load resistors) therefore end up at whatever layer their predecessor chain reaches, even when max_layer is larger. This makes the rightmost column unpredictable — outputs drift toward the middle, breaking the left-to-right signal flow convention.

**Solution:** Added `enforce_signal_flow()` to `src/placer/mod.rs`, called after `fix_isolated_source_layers()` (step 2). The algorithm computes ALAP (As Late As Possible) layers via iterative relaxation:

1. Terminal sinks (blocks with incoming edges but no outgoing edges) are pinned at `max_layer`.
2. For every other block with outgoing edges, `alap[u] = min_{s in successors}(alap[s] - 1)`. Iterate to fixpoint.
3. For each block with at least one incoming edge, set `layer[i] = max(asap[i], alap[i])`. This pushes sinks and their upstream predecessors to the right edge without affecting pure signal sources or isolated blocks.

Sources (no incoming edges) and isolated blocks (handled by Phase 2.4) are intentionally skipped — sources stay anchored on the left, isolated blocks stay next to their net-sharing neighbor.

**Results after Phase 4.2:**

| Example | Before 4.2 | After 4.2 | Delta |
|---------|:---:|:---:|:---:|
| **07 two-stage opamp** | 0.956 | **0.968** | **+0.012** |
| **10 opamp feedback** | 0.936 | **0.943** | **+0.007** |
| All others | unchanged | unchanged | — |

The op-amp topologies benefit most because they have well-defined signal chains terminating in output loads; pushing those loads (C3 in 07, C1 in 10) to the rightmost layer improves wire length and label ratio sub-scores. Linear circuits (01–03) and circuits already fully balanced (05, 11) see no change because their sinks were already at `max_layer`.

#### Phase 4.3 — Net-Aware Label Placement (DONE)

**Problem:** The router emitted labels at the raw pin world position. The label rectangle (50×16 units) then overlapped the component graphic, because the pin sits on the edge of the symbol body. Net names were often drawn on top of MOSFET channels or BJT bodies, hurting readability and making multi-pin labels visually ambiguous.

**Solution:** Tracked per-pin direction alongside pin world position and shifted each label outward along that direction. Changes in `src/router/mod.rs`:

1. New `PinInfo { position, label_offset }` replaces the bare `Point` in `net_connections`. The `label_offset` is computed from `PinDirection` with `(rotation, mirrored)` applied, so it is valid in world space.
2. `label_offset_for_pin()` maps direction → offset: ±30 units along the horizontal axis (label is 50 wide), ±15 units along the vertical axis (label is 16 tall). This clears the label rectangle past the pin stub.
3. When a label is emitted, a short stub wire from the pin position to the label anchor is added so the net is still electrically connected through the schematic.

**Results after Phase 4.3** (median over 5 runs; scoring has HashMap-induced noise):

| Example | Before 4.3 | After 4.3 | Δ |
|---------|:---:|:---:|:---:|
| **04 NMOS CS amp** | 0.825 | **0.829** | **+0.004** |
| **05 current mirror** | 0.875 | **0.892** | **+0.017** |
| **07 two-stage opamp** | 0.968 | **0.978** | **+0.010** |
| **08 bandgap** | 0.808 | **0.820** | **+0.012** |
| **10 opamp feedback** | 0.943 | **0.952** | **+0.009** |
| All others | unchanged | unchanged | — |

Five circuits improved on the median; no regressions. Gains come from the `label_ratio` sub-score — the offset labels and stub wires count as shorter overall label-to-pin distances, and Phase 4.3 also allowed deduplication (one label per pin anchor) to operate on the offset position rather than the raw pin.

#### Phase 4.4 — Remaining Features (TODO)

| Feature | Description |
|---------|-------------|
| **Interactive parameter search** | `n2s-improve` tries multiple parameter combinations and picks the best score |
