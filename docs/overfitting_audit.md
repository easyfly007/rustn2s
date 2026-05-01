# Overfitting Audit (2026-05-01)

This document is an honest catalog of every place in the codebase where a
constant or heuristic was tuned by trial-and-error against the 11-circuit
test set in `tests/examples/`. It exists so future contributors (and
future Claude sessions) can resist the temptation to keep tuning the
same knobs and instead question whether the test set is representative
of the real schematic-generation problem.

> **Why this matters.** With only 11 examples, every "optimization"
> made over the last few months risks being a fit to that specific set
> rather than a general improvement. A real industrial netlist (1000+
> devices, vendor-specific MOS model names, power nets called `VBAT`
> instead of `VDD`, etc.) would expose all of this immediately.

The audit is organized by risk level, where risk = "likelihood that this
thing breaks on circuits outside the current test set".

---

## 🔴 High risk — no theoretical basis, tuned directly against 11 circuits

### A1. `eval/score.rs` weights

```rust
overlap: 0.20, crossings: 0.15, aspect_ratio: 0.20,
wire_length: 0.10, label_ratio: 0.10, symmetry: 0.15, power_convention: 0.10
```

No ablation experiment, no sensitivity analysis. These were picked to
make the eleven test circuits produce "reasonable-looking" scores. A
weight sensitivity sweep (each ±0.05) would tell us if the *ranking*
across circuits is stable. Until that's done, all score-driven work is
optimizing a partly-fictional target.

### A2. `eval/score.rs` aspect-ratio brackets

```rust
ar <= 2.5  → 1.0           // why 2.5 and not 2.0 or 3.0?
ar <= 5.0  → 1.0..0.5
ar <= 10.0 → 0.5..0.2
ar > 10.0  → 0.2..0.0      // saturates at ratio 50, why 50?
```

Bracket boundaries were picked to map the eleven circuits' actual
aspect ratios onto a "looks right" scale. No reference to the literature
on schematic readability.

### A3. `eval/score.rs::ideal_length = comp_count * 100`

The wire-length sub-score normalizes by `comp_count × 100`. The constant
100 has no derivation — it was reverse-engineered from the test set's
actual wire lengths so the sub-score lands near 1.0 on circuits that
look fine.

### A4. `n2s-improve` search presets

```rust
const PRESETS: &[(f64, f64, f64, f64, f64)] = &[
    (300.0, 100.0,  60.0,  450.0, 0.30),  // wider columns, smaller devices
    (100.0, 200.0, 100.0,  600.0, 0.50),  // narrow columns
    ...
];
```

The eight presets were chosen by watching scores on the 11 circuits and
picking variations that helped at least one of them. The comments
("wider columns, smaller devices", etc.) are post-hoc rationalizations,
not principled coverage of parameter space (e.g. Latin Hypercube).

### A5. Tuning advisor thresholds

```rust
if breakdown.aspect_ratio_score < 0.8 { ... }   // why 0.8?
if breakdown.label_ratio_score < 0.7 { ... }    // why 0.7?
let factor = (ar / 2.0).min(3.0);               // why /2 and capped at 3?
suggested_value: layer_spacing * factor,
suggested_value: device_spacing * 0.7,          // why 0.7 and not 0.6 or 0.8?
```

All numerical thresholds were picked by running the advisor on the 11
circuits and adjusting until it produced "useful" suggestions.

---

## 🟡 Medium risk — principled algorithm but constants tuned

### B1. HAC parameters (`analyzer::ClusterOptions`)

```rust
merge_threshold: 0.5,
max_cluster_size: 6,
```

HAC is a standard clustering algorithm, but `0.5` and `6` are choices
matched to the granularity of the 11-circuit set. An industrial netlist
with 100 transistors might want a much higher max_cluster_size, or a
two-stage hierarchical clustering instead.

### B2. Router `long_net_threshold = 300`

The wire-vs-label cutoff is a global constant with units of "world
coordinates" — a number that only makes sense relative to the placer's
spacing parameters and the symbols' physical size. The fact that we
later had to add `adaptive_label_ratio` is itself acknowledgment that
the absolute threshold doesn't generalize.

### B3. Router A\* penalties

```rust
bend_penalty: 0.5,
crossing_penalty: 20.0,
```

The *ratio* (crossing >> bend) is correct in principle. The actual
numbers (0.5, 20.0) come from a small sweep over the 11 circuits — they
make 04/07/08 produce visually-acceptable A\* paths.

### B4. Placer `target_ratio = 1.5` (multi-column grid)

```rust
let target_ratio = 1.5;  // "slightly wider than tall"
```

Hard-coupled to the eval score's aspect_ratio bracket (≤2.5 = perfect).
Optimizing the placer for `target_ratio = 1.5` and then scoring with
`ar ≤ 2.5 = 1.0` is mutually-reinforcing — both constants come from
the same gut feel.

### B5. Hard-coded fallback widths/heights in placer block templates

```rust
.fold(60.0f64, f64::max);
width: 60.0, height: 40.0,
```

"60 wide × 40 tall" is a rough average of MOSFET symbol bounding rects.
It should be derived from `SymbolDef::bounding_rect()`, not hard-coded
in the placer. Right now if the symbol set grows or shrinks, these
constants don't track it.

### B6. `power_convention::x_threshold = 100`

The "are these two devices in the same column" tolerance. 100 units is
half the default layer_spacing (200), which is an internal coupling, not
a principled choice. If the user passes `--layer-spacing 500`, this
threshold becomes too tight.

---

## 🔴 High risk — pattern matchers with hard data assumptions

### C1. Power-net hard-coded set (`analyzer::identify_power_nets`)

```rust
"0", "gnd", "gnd!", "vss", "vss!", "vdd", "vdd!", "vcc", "vcc!", "avdd", "avss"
```

Real industrial netlists use names like `vbat`, `vio`, `vcore`,
`vssa_iso`, `pwr1v8`, `vdd_18`, `vssp`, `gnda`, `dgnd`. None of these
match. The eleven test circuits happen to use only the canonical names
above, so the limitation is invisible.

A principled replacement would identify power nets structurally:
- A net that is **only ever sourced** by V or I sources (no transistor
  drives it) and has high fanout, OR
- A net whose sourcing V source has a constant DC value, OR
- Topological: nets at the boundary of every transistor's source/bulk.

### C2. MOS type inference keywords

```rust
model contains "nch" or "nmos" → NMOS
model contains "pch" or "pmos" → PMOS
```

Industrial PDK model names: `sky130_fd_pr__nfet_01v8_lvt`, `g45n1svt`,
`tn22_lp_e_lv_nch`, `pmos2v_18_ana`. Some match (`nch` is a substring),
but many don't (`nfet`, `tn22`). Falls back to bulk-node-based inference
which works most of the time, but not for floating-bulk or biased-bulk
circuits.

### C3. Topological assumptions in pattern recognition

- **Diff pair**: requires the two transistors share an *identical* source
  or emitter node. Real circuits sometimes have a small resistor between
  source and the actual tail (degeneration) — that breaks recognition.
- **Current mirror**: requires at least one diode-connected device
  (drain == gate). Wide-swing mirrors and feedback-corrected mirrors do
  not meet this criterion.
- **Cascode**: requires `upper.source == lower.drain`. Some folded
  cascodes share bias rather than route source-to-drain directly.

The eleven test circuits use textbook patterns, so all of these
limitations are invisible.

---

## 🟢 Low risk — standard algorithms or model-derived

| Item | Why low risk |
|---|---|
| Sugiyama framework (DAG → layer → barycenter → coordinates) | Standard graph-drawing algorithm |
| Prim's MST for net topology | Optimal for the given distance metric |
| A\* search itself | Provably correct given admissible heuristic |
| Symbol pin offsets | Static model data, not fit |
| Manhattan distance heuristic | Admissible for grid routing |

These could still be subtly wrong (e.g. MST on Euclidean distance is not
the same as MST on Manhattan distance), but they aren't *tuned* — they
are textbook algorithms.

---

## What this audit does not cover

- **Runtime performance.** None of the constants above are tuned for
  performance, only for the score metric. Industrial-size netlists may
  hit performance walls (HAC O(N²), A\* O(grid_cells × directions))
  before they hit quality walls.
- **The eleven test circuits as a benchmark.** They are themselves a
  source of bias — they were hand-picked from the C++ MySchematic suite
  to demonstrate specific features. A representative survey of analog
  schematic patterns would weight them differently.
- **The choice of "schematic readability" as the optimization target.**
  Real-world goals could be very different: minimizing crossings for an
  IEEE-standard floor plan, fitting on a single A4 page, matching a
  specific company's drawing style, etc.

---

## Concrete next steps

Tracked here so we don't forget:

1. **Expand the test set to 25+ netlists.** Cover:
   - Larger linear/passive networks (5–10 R/C/L)
   - Long inverter chains (5, 10, 20 stages)
   - Industrial power-net names (`VBAT`, `VIO`, etc.) — *will fail*
   - Industrial MOS model names — *may fail*
   - Disconnected sub-circuits in the same netlist
   - Star fanout / heavy fan-in nodes

2. **Set up a hold-out.** Pick 3 circuits (one small, one medium, one
   complex) that are never used to tune any parameter. Watch them on
   every commit.

3. **Sensitivity analysis on `ScoreWeights`.** Sweep each weight ±0.05.
   If the per-circuit ranking is unstable, the weights are unreliable.

4. **Replace `identify_power_nets` with topology-based detection.**

5. **Document each magic constant** with (a) where it came from, (b)
   what changes if you change it, (c) what would tell us it's wrong.

Each of these is a separate, low-risk change — none of them touch the
algorithms, only the *scaffolding* around them.

---

## Reading this audit

When you find yourself reaching for one of the constants flagged above
to fix a regression, **stop**. Ask:

- Did the regression appear because of new test data, or because of an
  unrelated code change? If it's the former, the test data is a signal,
  not noise — don't suppress it by re-tuning.
- Is the fix a principled algorithm change, or another magic-number
  tweak? If the latter, you are deepening the overfit.
- Would the fix make sense to a circuit engineer who has never seen
  the eleven test circuits?

If you can't answer "yes" to that last question, the fix is probably
overfit.
