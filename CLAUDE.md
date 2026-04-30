# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`n2s` (Netlist to Schematic) is a Rust tool/library that converts SPICE netlists into visual schematics (SVG, JSON, KiCad `.kicad_sch`). It is a Rust reimplementation of the N2S pipeline from the C++ MySchematic project, with no Qt or other GUI dependency.

## Common Commands

```bash
# Build (debug / release)
cargo build
cargo build --release

# Run the three binaries
cargo run --bin n2s -- tests/examples/07_two_stage_opamp.sp -o out.svg
cargo run --bin n2s-eval -- -n circuit.sp -s schematic.json --pretty
cargo run --bin n2s-improve -- circuit.sp -o out.svg --pretty

# Tests (parser + analyzer have unit tests; placer/router currently do not)
cargo test                            # all tests
cargo test --lib parser::tests        # one module
cargo test test_inverter_detection    # one test by name

# Lint / format
cargo clippy --all-targets
cargo fmt
```

End-to-end manual check on the test corpus (`tests/examples/01_*.sp` … `11_*.sp`):

```bash
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  cargo run --release --bin n2s -- "$f" -o "output/${name}.svg" -o "output/${name}.json"
  cargo run --release --bin n2s-eval -- -n "$f" -s "output/${name}.json" --pretty
done
```

## Crate Layout

Single Cargo package (`Cargo.toml`) producing one library and three binaries:

| Target | Path | Role |
|---|---|---|
| lib `n2s` | `src/lib.rs` | Pipeline orchestration + module re-exports |
| bin `n2s` | `src/main.rs` | CLI: SPICE → SVG/JSON/`.kicad_sch` |
| bin `n2s-eval` | `src/bin/eval.rs` | Layout quality metrics (JSON) |
| bin `n2s-improve` | `src/bin/improve.rs` | Iterative parameter tuner that loops convert → eval → score → adjust |

Dependencies are intentionally minimal: only `clap`, `serde`, `serde_json`. No C deps, no async, no proc-macro frameworks beyond serde/clap derive. Keep it that way — adding deps is a deliberate decision.

## Pipeline Architecture (the big picture)

The library is a strict five-stage pipeline. Each stage has its own module, owns its own data types, and feeds the next:

```
SPICE text
   │  parser::SpiceParser            → ParseResult { devices, subcircuits, title }
   ▼
   │  analyzer::CircuitAnalyzer      → Vec<FunctionalBlock> + power_nets: HashSet<String>
   ▼
   │  placer::SchematicPlacer        → PlacementResult { placements, bounding_rect }
   ▼
   │  router::SchematicRouter        → model::Schematic { components, wires, labels, power_symbols, junctions }
   ▼
   export::{svg,json,kicad}
```

Top-level entry point: `convert_full()` in `src/lib.rs`. It also handles flat-vs-hierarchical mode selection: with `--hierarchical` plus both X instances and `.subckt` definitions, X instances are kept as boxes (subcircuit symbols built dynamically by `model::builtin_symbols::create_subcircuit_symbol`); otherwise it flattens to either top-level devices or the first subcircuit's internal devices.

Things worth knowing before changing stages:

- **Parser** (`src/parser/mod.rs`): handles `+` line continuations, `*`/`$`/`;` comments, `.subckt`/`.ends`. MOS type (NMOS vs PMOS) is inferred in priority order: model name keyword (`nch`/`nmos`/`pch`/`pmos`) → bulk node (`GND`/`VSS` → NMOS, `VDD`/`VCC` → PMOS) → default NMOS.
- **Analyzer** (`src/analyzer/mod.rs`): two stages.
  1. *Greedy pattern recognition* in fixed priority order: differential pair → current mirror → cascode pair → inverter. Diff pair / mirror / cascode are transistor-family-agnostic (MOSFET + BJT) via the shared `transistor_type()` helper and the `(drain|collector, gate|base, source|emitter)` node-0/1/2 convention.
  2. *Hierarchical agglomerative clustering* on remaining devices using shared-net weight, with threshold and max-cluster-size cutoffs.
  Power nets come from a hard-coded set (`0`, `gnd`, `vss`, `vdd`, `vcc`, `avdd`, `avss`, with `!` variants) plus voltage-source terminals.
- **Placer** (`src/placer/mod.rs`): Sugiyama-style hierarchical layout. Phases run in a specific order and several have been added incrementally — preserve their sequence: DAG construction → layer assignment (Kahn / longest path) → `fix_isolated_source_layers` (Phase 2.4) → `enforce_signal_flow` ALAP (Phase 4.2) → barycenter crossing minimization (4 sweeps) → `sort_blocks_by_polarity` (Phase 2.3, PMOS-top / NMOS-bottom) → block-internal templates → absolute coordinates → `align_matched_pairs` (Phase 2.2). Block templates differ per `BlockType` (DiffPair / CurrentMirror / CascodePair / Inverter / SingleDevice / Unknown); the `Unknown` template is pair-aware (groups members by `(symbol, W, L, model)`).
- **Router** (`src/router/mod.rs`): per-net, picks one of three strategies — power symbols (any distance, power nets), L-route Manhattan (short signal nets, < `long_net_threshold`), or labels + stubs (long signal nets). Topology is MST (Prim) over pin world positions, not a star. L-routes try both H-first and V-first and pick fewer crossings against already-routed wires. Labels are deduplicated per pin position and offset outward via `PinInfo.label_offset` so they don't overlap symbol bodies.
- **Model** (`src/model/`): geometry primitives, the `Schematic` data structure (serde-serializable), and 14 builtin `SymbolDef`s in `symbol.rs` with pin offsets in schematic coordinates. `pin_names_for_symbol()` maps SPICE node order to symbol pin names (e.g. MOSFET `D G S B` → pins `D G S B`).
- **Export** (`src/export/`): three sibling renderers (`svg.rs`, `json.rs`, `kicad.rs`) that all consume the same `Schematic`. SVG is dark-themed with rotation/mirror transforms applied to all symbol graphics. KiCad output is native `.kicad_sch`.
- **Eval** (`src/eval/`): nine independent metric modules (`connectivity`, `overlap`, `wire_crossings`, `wire_length`, `wire_bends`, `bounding_box`, `label_usage`, `symmetry`, `power_convention`) plus `score.rs` which combines them into a weighted overall score and produces tuning advice consumed by `n2s-improve`.

## Working with this codebase

- The `convert_full` pipeline takes `&ConvertOptions`, which bundles `PlacerOptions`, `RouterOptions`, `ClusterOptions`, and the `hierarchical` flag. CLI flags in `main.rs` map 1:1 to these. When adding a new tunable, add it to the relevant sub-options struct, expose a clap flag, and (if it's a layout/routing knob) consider adding a tuning rule in `eval/score.rs` so `n2s-improve` can use it.
- Quality is regression-tracked via the 11 example circuits in `tests/examples/`. The baseline scores per example are documented in `docs/architecture.md` and `docs/improve.md` under each "Phase X" section. After non-trivial placer/router changes, run the batch loop above and compare scores; document deltas in those tables when you ship a phase.
- Scoring has HashMap-iteration-order noise. If you're comparing scores across runs, take the median of several runs (the docs use 5).
- `docs/routing_improvement.md` describes a planned A\* grid-routing replacement for `l_route_best()`. Treat it as the design spec if/when implementing obstacle-aware routing.
- The `eval` module is consumed both by the `n2s-eval` binary and inside `n2s-improve`'s loop — keep its public types stable.
