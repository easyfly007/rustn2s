# Project Status (2026-05-01 end of day)

A short note for whoever picks this up tomorrow (likely the same person,
possibly a fresh Claude session with no memory of today's 30 commits).

Read this first. Then `docs/metric_reform.md` if you need the
reasoning behind the current score system, or `docs/overfitting_audit.md`
if you're tempted to tune any constant.

---

## Where things stand

**State**: stable. Working tree clean, all 154 tests pass, all
commits pushed to `origin/main`.

**Pipeline**: parse → analyze → place → route → export. Three real
bugs were fixed today (commits `2f22edf`, `5da968e`, `bbb1a0d`) plus
a fourth that turned out to be a metric bias rather than a code bug
(`00f0bb7`).

**Eval**: two-tier API as of today (commits `9804c67`, `f6f2d65`,
`39806da`):

- Tier 1 — `SafetyReport { no_overlap, power_convention_clean, symmetry_clean }`. Pass/fail booleans. A failure is a real bug; today every test circuit passes.
- Tier 2 — `QualityProfile { aspect_ratio, crossings, wire_length, label_ratio }`. Continuous values in [0, 1]. `aspect_ratio` is reported separately as a "shape signal" and excluded from the new comparators (`worst_quality()` and `quality_score()`).
- Legacy `compute_score` and `ScoreWeights` still work for
  back-compat.

**Test set**: 25 circuits in `tests/examples/`. Original 11 from
the C++ MySchematic suite + 14 added today to break audit blind
spots. Adding circuits was the highest-value move — it found the
bugs that the original 11 couldn't reveal.

---

## What's intentionally not done

These were considered today and deliberately skipped, with reasoning:

- **Bug 4 (long chains stack vertically)**. Two attempts at a code
  fix were tried and reverted (commit history shows the spike).
  Diagnosed as a metric bias, not a placement bug — the placement
  is already correct, the score formula is what punishes wide
  layouts. Step 7 of the metric reform partially addresses this by
  treating `aspect_ratio` as a shape signal.
- **More search presets in `n2s-improve`**. Audit flagged the
  current 8 as already-overfit; do not add more.
- **More tuning of A\*'s bend / crossing penalties**. Same.
- **Updating `n2s-improve`'s tuning advisor** (`suggest_tuning` in
  `eval/score.rs`) to use the new metric. Considered low ROI — the
  advisor itself is in the audit's high-risk list. Rewriting it
  needs design first.

---

## What to do tomorrow (in priority order)

### Highest value: validate against real-world data

Don't add features. Pick one circuit from outside the current
suite and run `n2s` on it visually. The ngspice distribution has
example netlists; sky130 PDK has standard cells; Berkeley course
material has analog circuits. Anything you didn't write yourself
will probably surface a bug.

After running, write down what's wrong **visually**, not by score.
The score tells you what the eval module noticed; the eye tells
you what an engineer would object to.

### Medium value: more adversarial test circuits

If you do want to keep adding to `tests/examples/`, the audit
items still uncovered are:

- **C1** — power nets that aren't sourced by V/I devices (e.g. a
  net coming from inside an LDO subckt). The current
  V-source-terminal rule masks the hardcoded power-name list, but
  some real netlists won't have the rescue.
- **C2** — PMOS with a non-power bulk node and a model name that
  doesn't match the `nch`/`pch` keyword check. Today's circuit 13
  was close but bulk-on-power saved it; build one where bulk is
  truly biased.
- **Scale** — current largest circuit is 30 components. An
  industrial netlist is 500+. Try 100+ and see what HAC, the
  Sugiyama placer, and A\* do at that scale.

Each new circuit is a lottery ticket for finding more real bugs.

### Lower value: documentation polish

`docs/architecture.md` still describes the eval system as a
weighted sum and stops at Phase 4.4. After today's metric reform
this is out of date. Half a page of additions would suffice.

`docs/examples.md` only documents circuits 1–11. Circuits 12–25
are covered in `docs/test_set_expansion_findings.md` but not in the
examples doc. Adding short descriptions there would be ~30
minutes.

### Avoid: more tuning of existing knobs

The audit (`docs/overfitting_audit.md`) lists the constants that
were tuned by trial-and-error. Resist the urge to nudge any of
them — `ScoreWeights`, `merge_threshold = 0.5`,
`max_cluster_size = 6`, `bend_penalty = 0.5`,
`crossing_penalty = 20.0`, etc. These are all already overfit to
the 25-circuit suite. Tuning them further before expanding the
suite is exactly the cycle the audit warned against.

If a regression appears, the right response is "find the new test
data that triggered it" not "change a constant to make it go
away."

---

## Useful commands

```bash
# Build
cargo build --release

# Tests
cargo test                # 154 tests, ~1s

# Two-tier evaluation profile across all 25 examples
mkdir -p /tmp/sweep
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  ./target/release/n2s "$f" -o "/tmp/sweep/${name}.json"
  ./target/release/n2s-eval -n "$f" -s "/tmp/sweep/${name}.json" --profile
done

# Per-circuit limiting-factor analysis
mkdir -p /tmp/sweep
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  ./target/release/n2s "$f" -o "/tmp/sweep/${name}.json"
  ./target/release/n2s-eval -n "$f" -s "/tmp/sweep/${name}.json" --pretty \
    > "/tmp/sweep/${name}_eval.json"
done
python3 scripts/profile_circuits.py /tmp/sweep
python3 scripts/sensitivity.py /tmp/sweep
```

---

## Today's commit log (high → low level)

```
71734d1  Mark all four metric-reform steps as done in metric_reform.md
39806da  Step 7: aspect_ratio is a shape signal, not a quality metric
f6f2d65  Step 6: n2s-improve --lex-min for worst-sub-score-first optimization
9804c67  Step 5: Two-tier evaluation API (SafetyReport + QualityProfile)
6df97f3  Add Tier 1 safety regression test for all example circuits
bfa231a  Metric reform analysis: profile circuits + n2s-eval --profile
00f0bb7  Document Bug 4 as a metric bias, not a placement bug
bbb1a0d  Fix Bug 1: PMOS-below-NMOS within an Unknown block
5da968e  Fix Bug 2: independent V/I sources collapse to identical (x, y)
2f22edf  Fix Bug 3: subckt-default-mode silently drops top-level content
bcd56eb  Expand test set: scale + parallel subckt instances (batch 4, final)
e5cf5d8  Expand test set: pattern-matcher edge cases (batch 3)
344a7f1  Expand test set: long chains and non-textbook patterns (batch 2)
9d21208  Expand test set: industrial conventions + disconnected DAGs (batch 1)
bd78273  Document findings from test-set expansion (11 → 25 circuits)
61fce15  Add score-weight sensitivity analysis (the audit, made concrete)
5c1159d  Document overfitting audit (2026-05-01)
```

---

## A reminder to future self

The single biggest lesson today: **the test suite was the bottleneck,
not the algorithms**. Eleven circuits couldn't tell us whether the
score was working. Twenty-five revealed three real bugs and a major
bias. Whatever you do tomorrow, more samples is rarely the wrong
move.
