# Score-Weight Sensitivity Analysis (2026-05-01)

Companion to [`overfitting_audit.md`](overfitting_audit.md). This is the
first concrete diagnostic of how reliable the eval score is, and the
findings make the audit's "high risk" tag concrete.

## Method

1. Generate `n2s-eval` JSON for each of the 11 test circuits with the
   default placer/router parameters.
2. Re-derive each of the seven sub-scores in Python using the same
   formulas as `src/eval/score.rs`. (The eval tool emits raw metrics;
   sub-scores are pure functions of those metrics, so any-language
   reimplementation is equivalent.)
3. For each weight `w_i` in `ScoreWeights`, perturb it by ±0.05 and
   ±0.10 (with the other weights renormalized so they still sum to 1).
   Compute every circuit's overall score under each perturbation.
4. Rank circuits high-to-low under baseline weights and under each
   perturbation. Compare rankings.

The runner is at `scripts/sensitivity.py` (committed alongside this doc).

## Result 1 — three weights are dead on this test set

| Sub-score | Baseline value across all 11 circuits |
|---|---|
| `overlap` | 1.0 (no circuit has any overlapping components) |
| `symmetry` | 1.0 (every matched pair is aligned) |
| `power_convention` | 1.0 (every PMOS sits above every NMOS) |
| `crossings` | varies (0.50 or 1.0) |
| `aspect_ratio` | varies (0.19 to 1.0) |
| `wire_length` | varies (0.80 to 1.0) |
| `label_ratio` | varies (0.67 to 1.0) |

**Three of seven sub-scores are at the ceiling for every test circuit.**
Their weights — totalling 0.45 of the score budget — are effectively
inert. Changing any of them by ±0.10 produces *zero* ranking changes:

```
overlap          -0.10 / -0.05 / +0.05 / +0.10  → (no change)
symmetry         -0.10 / -0.05 / +0.05 / +0.10  → (no change)
power_convention -0.10 / -0.05 / +0.05 / +0.10  → (no change)
```

Three dead weights × 11 circuits = a strong signal that **the test set
does not exercise three of seven scoring dimensions at all**. Whatever
"work" Phases 2.2/2.3, the symmetry alignment, and the PMOS-above-NMOS
sort accomplished is invisible from the score's point of view, because
every circuit was already at the ceiling.

## Result 2 — `aspect_ratio` dominates the bottom of the table

| Perturbation | Rank changes |
|---|---|
| `aspect_ratio -0.10` | 02 ↓3, 03 ↓3, 05 ↑2, 08 ↑2, 09 ↑2 |
| `aspect_ratio -0.05` | 02 ↓1, 03 ↓1, 08 ↑2 |
| `aspect_ratio +0.05` | (no change) |
| `aspect_ratio +0.10` | (no change) |

When we *reduce* `aspect_ratio`'s weight, 02 and 03 (whose only weakness
is aspect_ratio) jump up the table. Increasing it does nothing because
the ceiling has already done its work — most circuits score 1.0 on this
metric.

The implication: **02's score of 0.844 and 03's score of 0.838 are
single-cause failures.** They are not "overall mediocre circuits" — they
are excellent on six of seven dimensions and only fail aspect_ratio
because of an inherent geometric limit (3–4 devices in a Sugiyama
column).

## Result 3 — `crossings` and `label_ratio` are medium-sensitive

| Perturbation | Rank changes |
|---|---|
| `crossings +0.10` | 02 ↓2, 03 ↓2, 05 ↑2, 08 ↑2 |
| `label_ratio -0.10` | 04 ↓2, 05 ↓1, 06 ↑1, 09 ↑1, 11 ↑1 |
| `label_ratio +0.10` | 02 ↓1, 08 ↑1 |

These are both real signals — they reflect circuits that genuinely have
different label-pair counts and wire crossings. But the magnitudes
(2-rank swaps under a ±0.10 weight change) suggest the absolute weight
values aren't doing principled work — they just set ratios between
sub-scores that the test set didn't pick.

## Result 4 — per-circuit score range under perturbation

| Circuit | min | baseline | max | range |
|---|---:|---:|---:|---:|
| 01 voltage_divider | 1.000 | 1.000 | 1.000 | 0.000 |
| 02 rc_lowpass_filter | 0.766 | **0.844** | 0.922 | **0.156** |
| 03 halfwave_rectifier | 0.757 | **0.838** | 0.919 | **0.162** |
| 04 nmos_common_source | 0.959 | 0.979 | 1.000 | 0.041 |
| 05 nmos_current_mirror | 0.846 | 0.892 | 0.938 | 0.092 |
| 06 bjt_diff_pair | 1.000 | 1.000 | 1.000 | 0.000 |
| 07 two_stage_opamp | 0.953 | 0.974 | 0.995 | 0.043 |
| 08 bandgap_reference | 0.831 | 0.875 | 0.919 | 0.088 |
| 09 inverter_chain_hier | 0.867 | 0.916 | 0.965 | 0.098 |
| 10 opamp_feedback_hier | 0.928 | 0.952 | 0.976 | 0.048 |
| 11 rlc_controlled_sources | 1.000 | 1.000 | 1.000 | 0.000 |

**The bottom four circuits (02/03/05/08) move by 0.09–0.16 points just
from a ±0.10 weight tweak.** That's larger than most of the
"improvements" we shipped over the last several phases (Phase 4.2 gave
+0.012 on 07, Phase 4.3 ~+0.01 averages). In other words, the noise
introduced by *unjustified weight choices* is bigger than the signal
from real algorithm improvements.

## Top-3 stability

Baseline top-3 is {01, 06, 11} — three circuits all scoring 1.000.
Under 28 weight perturbations, the top-3 changed only once. But this
"stability" is a ceiling artifact: those three are at 1.0 on *all* the
varying sub-scores too, so any weight scheme keeps them on top. This
is *not* evidence that the score is reliable — it's evidence that the
test set saturates.

## What this tells us

1. **Half the score's budget (45%) does no work on this test set.**
   `overlap`, `symmetry`, `power_convention` never dip below 1.0. We
   should either:
   - Add test circuits where these constraints are violated (better),
   - Or drop those weights from the score formula (gives up the ability
     to detect those violations on real-world circuits where they matter).

2. **`aspect_ratio` is solely responsible for 02/03's position at the
   bottom.** Any "fix" for those circuits is either improving aspect
   ratio specifically (legit) or rebalancing weights to hide the
   problem (overfitting).

3. **The score is not a reliable cross-circuit ranking tool.** A ±0.05
   tweak to one weight can swap circuits' positions by 2-3 ranks. If we
   can't even agree on whether circuit X is "better" than circuit Y
   without nailing down the exact weights, then any benchmark that
   reports a single-number "score" is hiding too much.

4. **Phases 2.2 / 2.3 / pair-aware Unknown produced no measurable score
   improvement on this test set,** even though those features are real
   improvements — because the test set never had non-symmetric or
   PMOS-below-NMOS layouts to begin with.

## Concrete next actions

In priority order:

1. **Add circuits that violate the saturated dimensions.**
   - One with built-in component overlap (force a tight `block_spacing`
     and a deliberately wide symbol).
   - One where the current placer puts NMOS above PMOS (e.g. inverter
     with NMOS as the input device, or a logic gate where convention
     would be inverted).
   - One with mismatched device pairs that should have been symmetric
     but aren't (e.g. asymmetric diff pair, two non-matching MOSFETs
     placed as a pair).

   These will give `overlap` / `power_convention` / `symmetry` something
   to actually do.

2. **Replace the single overall score with a profile**, at least
   internally. `n2s-eval` already returns the seven sub-scores
   separately; consumers should report all seven, not collapse them.
   The "Pareto front" of {aspect_ratio, label_ratio, wire_length,
   crossings} is more honest than a weighted sum.

3. **Stop tuning until the test set is wider.** Specifically: don't
   ship more search presets, advisor thresholds, or A* penalty values
   until we have circuits whose results can disagree meaningfully with
   each other across weight perturbations.

## Appendix — running the sweep

The Python script and this doc together let any future contributor
re-run the sweep:

```bash
mkdir -p /tmp/sweep
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  ./target/release/n2s "$f" -o "/tmp/sweep/${name}.json"
  ./target/release/n2s-eval -n "$f" -s "/tmp/sweep/${name}.json" --pretty \
    > "/tmp/sweep/${name}_eval.json"
done
python3 scripts/sensitivity.py /tmp/sweep
```

The script depends only on Python stdlib.
