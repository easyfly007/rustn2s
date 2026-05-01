# n2s-improve — Iterative Layout Quality Optimizer

`n2s-improve` is an automated feedback loop that iteratively tunes `n2s` placement and routing parameters to maximize schematic layout quality. It replaces manual parameter guessing with a score-driven optimization cycle.

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    n2s-improve loop                         │
│                                                             │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────┐ │
│  │ n2s core │───▶│ n2s-eval │───▶│  Scorer  │───▶│Tuner │ │
│  │ (convert)│    │(evaluate)│    │ (score)  │    │      │ │
│  └────┬─────┘    └──────────┘    └──────────┘    └──┬───┘ │
│       │                                              │     │
│       └──────────── adjusted params ◀────────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

Each iteration:

1. **Convert** — Runs the full `n2s` pipeline (parse → analyze → place → route) with current parameters
2. **Evaluate** — Computes 9 layout quality metrics via the `eval` module
3. **Score** — Combines metrics into a single quality score (0.0–1.0) using weighted sum
4. **Tune** — Analyzes which metrics are weak and suggests parameter adjustments
5. **Repeat** — Applies adjustments and runs again until convergence

The loop terminates when:
- Target score is reached (default: 0.9)
- No further tuning advice is available
- Score has stalled for 3 consecutive iterations
- Maximum iterations reached (default: 10)

The **best-scoring** schematic across all iterations is output, not necessarily the last.

## Installation

```bash
cargo build --release
# Binary at target/release/n2s-improve
```

## Usage

### Basic

```bash
# Optimize and output SVG
n2s-improve circuit.sp -o circuit.svg

# Also save the JSON schematic
n2s-improve circuit.sp -o circuit.svg --json circuit.json

# See the full optimization report
n2s-improve circuit.sp -o circuit.svg --pretty
```

### Options

```
n2s-improve <INPUT> [OPTIONS]

Arguments:
  <INPUT>                    Input SPICE netlist file

Options:
  -o, --output <FILE>        Output SVG file (best result)
      --json <FILE>          Output JSON schematic file (best result)
      --max-iter <N>         Maximum optimization iterations [default: 10]
      --target-score <F>     Stop early if this score is reached [default: 0.9]
      --layer-spacing <F>    Initial horizontal layer spacing [default: 200]
      --block-spacing <F>    Initial block spacing [default: 100]
      --device-spacing <F>   Initial device spacing [default: 80]
      --grid <F>             Grid snap size [default: 10]
      --label-threshold <F>  Initial label distance threshold [default: 300]
      --no-patterns          Disable pattern recognition
      --scale <F>            SVG scale factor [default: 1.0]
      --no-grid              Hide grid in SVG
      --pretty               Pretty-print the JSON report
      --quiet                Suppress iteration logs (only output final report)
      --search               Run multiple restarts from spaced starting
                             points and keep the global best (Phase 4.4)
      --search-restarts <N>  Restart count when --search is on [default: 8]
```

### Multi-Start Search (`--search`)

Without `--search`, `n2s-improve` runs **one** greedy advice-driven loop
from the user-supplied initial parameters. The advisor (`suggest_tuning`
in `src/eval/score.rs`) only nudges parameters in one direction at a
time, so circuits with poor initial geometry can converge to a local
optimum well below their best achievable score.

`--search` runs the same greedy loop from multiple starting points and
keeps the global best:

1. Restart 0 always uses the user-supplied parameters (so `--search`
   never under-performs the no-`--search` run).
2. Restarts 1..N cover deterministic spaced points in parameter space
   (corners + diagonals: wide/narrow columns, tight/loose blocks,
   wire-preferring vs. label-preferring thresholds).
3. As soon as any restart hits `--target-score`, the rest are skipped.

Each restart still respects `--max-iter`, so total work is bounded by
`max_iter × search_restarts` pipeline runs.

#### Example results

Compared with the single-greedy run on the 11 test circuits:

| Example | Default | `--search` | Δ |
|---------|:---:|:---:|:---:|
| 02 RC filter | 0.860 | **0.876** | **+0.016** |
| 03 halfwave rectifier | 0.844 | **0.860** | **+0.016** |
| 08 bandgap reference | 0.875 | **0.950** | **+0.075** |
| All others | unchanged | unchanged | — |

Circuits already at or near the target score finish in restart 0 and
incur no extra cost. Small linear circuits (02, 03) gain a few points
because the search reaches a parameter region the advisor wouldn't.
Bandgap (08) gains the most because its ideal layout uses very
different spacings from the defaults.

### Iteration Logs

Without `--quiet`, progress is printed to stderr:

```
Iteration 0: score=0.694 [overlap=1.00 cross=1.00 ar=0.14 wire=1.00 label=1.00 sym=0.00 pwr=1.00]
  -> layer_spacing : 200.0 → 600.0 (Aspect ratio 22.9 is too tall; increase horizontal spread)
  -> device_spacing : 80.0 → 56.0 (Reduce vertical stacking to improve aspect ratio 22.9)
Iteration 1: score=0.702 [overlap=1.00 cross=1.00 ar=0.15 wire=1.00 label=1.00 sym=0.00 pwr=1.00]
  ...
Converged: Score stalled at 0.702 for 3 iterations
```

### JSON Report

The structured report goes to stdout. Use `--pretty` for human-readable format:

```bash
n2s-improve circuit.sp --pretty
```

Report structure:

```json
{
  "input_file": "circuit.sp",
  "iterations_run": 3,
  "converged": true,
  "convergence_reason": "Target score 0.900 reached at iteration 2",
  "initial_score": 0.784,
  "final_score": 0.912,
  "improvement": 0.128,
  "best_params": {
    "layer_spacing": 300.0,
    "block_spacing": 100.0,
    "device_spacing": 60.0,
    "label_threshold": 450.0
  },
  "best_score": {
    "overall": 0.912,
    "overlap_score": 1.0,
    "crossings_score": 1.0,
    "aspect_ratio_score": 0.95,
    "wire_length_score": 0.88,
    "label_ratio_score": 0.75,
    "symmetry_score": 0.67,
    "power_convention_score": 1.0
  },
  "history": [ ... ]
}
```

### Piping and Automation

```bash
# Extract just the best parameters
n2s-improve circuit.sp --quiet | jq '.best_params'

# Extract the final score
n2s-improve circuit.sp --quiet | jq '.final_score'

# Use optimized params with vanilla n2s
PARAMS=$(n2s-improve circuit.sp --quiet | jq -r '
  .best_params |
  "--layer-spacing \(.layer_spacing) --block-spacing \(.block_spacing) --device-spacing \(.device_spacing) --label-threshold \(.label_threshold)"
')
eval n2s circuit.sp -o circuit.svg $PARAMS

# Batch optimize all examples
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  n2s-improve "$f" -o "output/${name}_improved.svg" --quiet
done
```

## Scoring System

### Quality Score

A single number in [0.0, 1.0] computed as a weighted sum of 7 sub-scores:

| Sub-Score | Weight | What It Measures | Perfect (1.0) | Zero (0.0) |
|-----------|--------|------------------|---------------|------------|
| `overlap` | 0.20 | Component bounding box overlaps | No overlaps | Any overlap |
| `crossings` | 0.15 | Wire segment intersections | No crossings | Many crossings |
| `aspect_ratio` | 0.20 | Bounding box width/height ratio | Ratio ≤ 2.5 | Ratio > 50 |
| `wire_length` | 0.10 | Total wire length vs ideal | ≤ 100 units/component | Much longer |
| `label_ratio` | 0.10 | Labels used vs direct wires | No labels (all wires) | High label ratio |
| `symmetry` | 0.15 | Matched device pair alignment | All pairs symmetric | No symmetry |
| `power_convention` | 0.10 | PMOS above NMOS | All correct | All violated |

### Aspect Ratio Scoring Detail

| Ratio | Score |
|-------|-------|
| ≤ 2.5 | 1.0 |
| 2.5 – 5.0 | 1.0 → 0.5 (linear) |
| 5.0 – 10.0 | 0.5 → 0.2 (linear) |
| 10.0 – 50.0 | 0.2 → 0.0 (linear) |
| > 50.0 | 0.0 |

## Tuning Rules

The tuner maps weak sub-scores to parameter adjustments:

### Aspect Ratio Too Tall (score < 0.8, height > width)

| Adjustment | Formula | Rationale |
|------------|---------|-----------|
| `layer_spacing` ↑ | × min(ratio/2, 3) | Spread blocks across more horizontal layers |
| `device_spacing` ↓ | × 0.7, min 40 | Compress vertical stacking within blocks |

### Aspect Ratio Too Wide (score < 0.8, width > height)

| Adjustment | Formula | Rationale |
|------------|---------|-----------|
| `layer_spacing` ↓ | ÷ min(ratio/2, 3), min 100 | Reduce horizontal spread |

### Component Overlap Detected (score < 1.0)

| Adjustment | Formula | Rationale |
|------------|---------|-----------|
| `block_spacing` ↑ | × 1.5 | More room between functional blocks |
| `device_spacing` ↑ | × 1.3 | More room between devices in a block |

### Too Many Labels (score < 0.7)

| Adjustment | Formula | Rationale |
|------------|---------|-----------|
| `label_threshold` ↑ | × 1.5 | Allow longer wires before switching to labels |

### Parameter Bounds

All parameters are clamped to prevent runaway:

| Parameter | Min | Max |
|-----------|-----|-----|
| `layer_spacing` | 50 | 1000 |
| `block_spacing` | 30 | 500 |
| `device_spacing` | 30 | 300 |
| `label_threshold` | 100 | 2000 |

## Results on Test Examples

Single representative run on all 11 test circuits, current code (post Phase
4.4). Scores have small HashMap-iteration-order noise (~ ±0.02–0.05 between
runs); the figures below are illustrative, not regression-locked.

| Example | No-search<br>final | `--search`<br>final | Δ | Restarts<br>used | Limiting sub-score (no-search) |
|---------|:---:|:---:|:---:|:---:|---|
| 01 voltage divider | 1.000 | 1.000 | — | 1 | — |
| **02 RC filter** | 0.844 | **0.876** | +0.032 | 8 | aspect_ratio = 0.22 (3 devices, ratio 9.7) |
| **03 halfwave rectifier** | 0.844 | **0.860** | +0.016 | 8 | aspect_ratio = 0.19 (4 devices, ratio 12.3) |
| **04 NMOS CS amp** | 0.979 | **0.994** | +0.015 | 5 | label_ratio = 0.79 |
| **05 current mirror** | 0.892 | **0.967** | +0.075 | 8 | crossings = 0.50 (1 crossing) + label_ratio 0.67 |
| 06 BJT diff pair | 1.000 | 1.000 | — | 1 | — |
| 07 two-stage opamp | 0.978 | 0.978 | — | 8 | label_ratio = 0.78 |
| **08 bandgap reference** | 0.875 | **0.949** | +0.074 | 8 | crossings = 0.50 + label 0.70 + wire 0.80 |
| 09 inverter chain | 0.916–0.991 | 0.916–0.991 | — | 1 | — (run-to-run noise) |
| **10 opamp feedback** | 0.952 | **0.956** | +0.004 | 8 | label_ratio = 0.74 |
| 11 RLC controlled | 1.000 | 1.000 | — | 1 | — |

**With `--search`, 9/11 examples score ≥ 0.94, with 5 of those at ≥ 0.99**.
Multi-start search makes the biggest difference on 02 (+3 pts), 04 (+1.5),
05 (+7.5), and 08 (+7.4) — all circuits where the user-supplied default
parameters happen to land in a poor local optimum.

### Key Observations

1. **Symmetry is no longer a bottleneck.** Phases 2.2 (cross-block
   alignment), 2.3 (PMOS-above-NMOS), and the pair-aware Unknown block
   template together drove the symmetry sub-score to **1.0** on every test
   circuit. The earlier "04/06/08 limited by symmetry" claim is stale.

2. **Today's bottlenecks** are:
   - **Aspect ratio** for small linear circuits (02, 03) — inherent
     limitation when the netlist has only 3–4 devices.
   - **Wire crossings** for 05 and 08 — one crossing each that the router
     can't avoid given the current placement.
   - **Label ratio** for several mid-sized circuits (04, 07, 10) — the
     router uses labels for nets longer than the threshold; many circuits
     would be more readable with longer wires than labels, but bumping
     the threshold trades against wire-length and crossings sub-scores.

3. **Multi-start search (Phase 4.4) is the cheapest remaining lever.** It
   never under-performs the single-greedy run (restart 0 always uses the
   user defaults) and exits early once any restart hits target. For
   circuits in the bottom half it routinely picks up several percent.

4. **Run-to-run noise.** The placer/router internals iterate `HashMap`s,
   which makes the final score wobble by 0.02–0.05 between runs on
   circuits with multiple equally-good arrangements (notably 09 swings
   between 0.916 and 0.991). Don't read finer-than-percent differences
   between runs as signal — take a median over a few runs when comparing.

## Limitations

| Issue | Why Parameters Can't Help | Status |
|-------|---------------------------|--------|
| ~~Matched devices at different y~~ | ~~Devices in separate blocks~~ | **DONE (Phase 2.2)** |
| ~~Duplicate labels per net~~ | ~~Router emits 2 labels per pin pair~~ | **DONE (Phase 3.1)** |
| ~~Wire crossings on simple cases~~ | ~~Fixed horizontal-first L-routing~~ | **DONE (Phase 3.2)** |
| ~~Sources separated from circuit~~ | ~~Source blocks have no DAG edges~~ | **DONE (Phase 2.4)** |
| ~~Labels overlapping component bodies~~ | ~~Router emitted labels at raw pin positions~~ | **DONE (Phase 4.3)** |
| ~~BJT diff pair / mirror not recognized~~ | ~~Pattern finders hard-coded to `device_type == 'M'`~~ | **DONE (BJT pattern extension)** |
| ~~Low symmetry for matched pairs inside the same cluster block~~ | ~~`align_matched_pairs` only shifted whole blocks~~ | **DONE (pair-aware `Unknown` template)** |
| ~~Greedy advisor stuck at local optima for circuits with poor initial geometry~~ | ~~Single starting point in the parameter space~~ | **DONE (Phase 4.4 multi-start search)** |
| Wires walking through component bodies | L-router doesn't see geometry | **WORKAROUND** (`--obstacle-avoidance`) |
| Small circuit aspect ratio | Only 3-4 devices, too few for multi-column | Inherent limitation |
| Residual crossings on 05 / 08 | Placement leaves no detour room | Open |
| Aggressive label use on mid-sized opamps | `label_threshold` is one global knob | Open (could go adaptive) |

## Architecture

### Files

| File | Purpose |
|------|---------|
| `src/bin/improve.rs` | CLI entry point, iteration loop, convergence detection |
| `src/eval/score.rs` | Quality scoring (weighted sum) and tuning advisor |
| `src/eval/mod.rs` | Evaluation module (9 metric checkers) |

### Data Flow

```
improve.rs
  │
  ├── SpiceParser::parse()          ← parse once, reuse
  │
  └── loop {
        ├── n2s::convert()          ← full pipeline with current params
        ├── eval::evaluate()        ← compute 9 metrics
        ├── score::compute_score()  ← weighted sum → overall score
        ├── score::suggest_tuning() ← identify weak spots → param changes
        ├── check convergence       ← target/stall/max-iter
        └── apply adjustments       ← update params for next iteration
      }
```

### Dependencies

No new dependencies. Uses the existing `n2s` library crate (`convert`, `eval`, `parser`) and `clap`/`serde`/`serde_json`.
