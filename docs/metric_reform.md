# Metric Reform Proposal (2026-05-01)

This document is the third step in the audit-driven sequence:

1. [`overfitting_audit.md`](overfitting_audit.md) — diagnosed which
   constants and heuristics were tuned by trial-and-error against the
   original 11-circuit set.
2. [`sensitivity_analysis.md`](sensitivity_analysis.md) — confirmed
   that 45% of the `ScoreWeights` budget was inactive on the original
   suite, and that ±0.10 weight perturbations move scores by up to 0.16.
3. [`test_set_expansion_findings.md`](test_set_expansion_findings.md)
   — expanded the suite to 25 circuits and exposed four real bugs that
   the original suite couldn't reach.
4. **This doc** — having fixed three of the four bugs (3, 2, 1), what
   does the score formula actually measure now?

The answer is: not what we thought.

## Method

Run `scripts/profile_circuits.py` against the current 25-circuit suite.
For each circuit:

- Compute the seven sub-scores using the formulas in
  `src/eval/score.rs`.
- **Categorize** the circuit by which sub-scores fall below `0.95`:
  - `perfect` — overall ≥ 0.99, nothing weak.
  - `ar-only` — only `aspect_ratio` is weak (likely metric bias).
  - `real` — non-`aspect_ratio` sub-scores are weak (genuine layout
    issues).
  - `both` — `aspect_ratio` and at least one other sub-score weak.
- Compute the **potential ceiling** — what would `overall` be if every
  weak sub-score were lifted to `1.0`?
- Compute **`overall` without `aspect_ratio`** — drop that one knob to
  isolate metric-bias effects.

## Result 1 — three sub-scores have zero variation across the suite

```
overlap          0/25 circuits below 0.95
symmetry         0/25 circuits below 0.95
power_convention 0/25 circuits below 0.95
```

**Combined weight: 0.45 (45% of the total score budget).**

These three sub-scores are **safety metrics** — they reflect *bugs*
in the schematic (component overlap, mismatched layouts, wrong-
polarity stacking), not stylistic differences. After fixing Bug 1
(PMOS-NMOS within-block ordering) and Bug 2 (V/I source false
matching), every test circuit passes them at 1.0. They contribute to
ranking only when something has gone seriously wrong — and on a clean
test suite, nothing ever has.

This is misleading: 45% of the "score" is actually meaningless on
nominal output and only fires under bugs. The current formula treats
"my schematic has a power-convention violation" the same as "my
schematic has 30% extra wire length", weighted only by 0.10 vs 0.10.
Those should not be on the same scale.

## Result 2 — `aspect_ratio` is half-real, half-bias

```
ar-only circuits (only aspect_ratio is weak):
  02_rc_lowpass_filter   ar=9.7  → if-ar-removed→1.000
  03_halfwave_rectifier  ar=12.3 → if-ar-removed→1.000

both (aspect_ratio + real issues):
  16_inverter_chain_5stage  ar=7.3 + crossings, wire_length, label_ratio
  17_folded_cascode_opamp   ar=3.7 + crossings, wire_length, label_ratio
  21_deep_signal_chain      ar=3.9 + wire_length, label_ratio
```

`02_rc_lowpass_filter` is a 3-device RC filter; it scores 0.844
*entirely because* the placer produces a 1×3 vertical column and the
formula penalizes anything not roughly square. The placement is fine
schematically. There is no fix in the placer; the formula is the
problem.

`16_inverter_chain_5stage` has a *legitimate* horizontal cascade
(verified visually in the Bug 4 follow-up) but scores 0.723 because
the 1560×140 bounding box is graded harshly. Even setting
`aspect_ratio = 1.0` lifts it only to 0.851 — there are real issues
too (crossings, wire length).

The takeaway: `aspect_ratio` mixes a real signal (some circuits have
genuinely cramped or sprawling layouts) with a bias (legitimate wide
or tall layouts get punished alongside).

## Result 3 — the four sub-scores that *do* vary

```
label_ratio         12/25  (48%)
wire_length          7/25  (28%)
crossings            5/25  (20%)
aspect_ratio         5/25  (20%)
```

**`label_ratio` is the dominant signal**: nearly half of the test
circuits have label-vs-wire choices that hurt the score. This is a
router decision, not a placer one — every circuit in the "real" or
"both" categories has this weakness.

`wire_length` and `crossings` are smaller but real. They are
genuinely useful — circuits with these weaknesses either route too
long or have wires that cross. Both are visually noticeable.

## What this means

The current `compute_score(report, weights) → f64` API is doing three
incompatible things at once:

1. **Safety check.** "Does this layout violate basic correctness?"
   (overlap, power_convention, symmetry — should be Pass/Fail, not
   weighted.)
2. **Style check.** "Is the layout shape conventional?" (aspect_ratio
   — needs calibration, not a blanket rule.)
3. **Quality check.** "How readable is the routing?" (crossings,
   wire_length, label_ratio — actual continuous metrics.)

Mashing all three into a weighted sum makes none of them informative:
- A circuit with 1 overlap and 0 other issues scores 0.80, the same
  as a circuit with 5 wire crossings and label-heavy routing.
- A 3-device circuit with bad aspect_ratio scores 0.84, the same as
  a 30-device opamp with serious routing problems.
- The "improvement" we saw across various phases ranged 0.005–0.05;
  weight perturbations alone produce 0.10–0.16 swings.

## Proposed reform

### Tier 1: Safety constraints (boolean)

```rust
struct SafetyReport {
    pub no_overlap: bool,            // overlap_count == 0
    pub power_convention_clean: bool,// no PMOS-below-NMOS violations
    pub symmetry_clean: bool,        // every matched pair y-aligned
}
```

These are **hard pass/fail**. A schematic that fails any of these has
a bug, not a "0.85 score". Reporting them as "score 0.0" or "score
1.0" hides what kind of bug.

### Tier 2: Quality profile (continuous, separate)

```rust
struct QualityProfile {
    pub aspect_ratio: f64,
    pub crossings: f64,
    pub wire_length: f64,
    pub label_ratio: f64,
}
```

Four sub-scores, each in `[0.0, 1.0]`. Don't combine them. Let
consumers (CI, `n2s-improve`'s search advisor, downstream tools) pick
which dimensions they care about and how to combine them.

### Backwards compatibility

Keep `compute_score()` as a *convenience* function that combines the
profile into a single number for legacy consumers, but make
`evaluate()` return both `SafetyReport` and `QualityProfile`
separately. New consumers should read the profile.

`n2s-eval` should grow a `--profile` flag that emits the profile in a
compact format for humans and tools.

### What `n2s-improve` should optimize

Right now `n2s-improve` maximizes `overall`. With the reform:

- Bail out immediately if Tier 1 fails (the schematic is buggy, no
  amount of parameter tuning will rescue it from a real bug).
- Optimize the profile lex-min: maximize the worst sub-score first,
  then the second-worst, etc. This avoids the current "sum-of-weights"
  failure mode where improving one sub-score by 0.10 at the cost of
  another by 0.05 looks like a win.

## What we already know would happen

Running the proposed reform on the current 25 circuits:

| Tier 1 (safety) | Tier 2 weakness | Action |
|---|---|---|
| Pass | None (10 circuits) | Already perfect, nothing to do |
| Pass | aspect_ratio only (2 circuits) | This is metric bias, calibrate or skip |
| Pass | label_ratio dominant (10 circuits) | Real router work needed |
| Pass | wire_length / crossings (3 circuits) | Real router work needed |
| Fail | — | (none currently) |

So the reform reveals that, after Bugs 1–3 are fixed, the remaining
work the test suite asks for is **all in the router** (label_ratio,
wire_length, crossings) plus **two metric-bias artifacts** (02, 03).
The placer is essentially done for this suite. No more placer
optimization is justified by the current data.

## Recommended implementation steps

In priority order:

1. **Add `--profile` to `n2s-eval`** — emit the seven sub-scores in a
   compact line-per-circuit format. Cheap, immediately useful for
   triage.
2. **Split `EvalReport` into `SafetyReport` + `QualityProfile`** at
   the API level, while keeping `compute_score` as a back-compat
   wrapper.
3. **Update `n2s-improve`** to short-circuit on Tier 1 failure and
   use lex-min on Tier 2.
4. **Drop `aspect_ratio` from the weighted sum entirely.** Compute and
   report it as a separate "shape signal" — the engineer can decide
   whether their long signal chain is supposed to be wide.

Step 1 is the smallest unit of useful change and lets the rest of the
proposal be evaluated against real data.
