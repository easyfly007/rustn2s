# Project Status (2026-07-04)

A handoff note for whoever picks this up next (possibly a fresh
Claude session with no memory of the last three days' ~15 commits).

Read this first. Then `docs/test_set_expansion_findings.md` for the
full find/fix log of the real-world validation campaign, or
`docs/overfitting_audit.md` before you feel tempted to tune any
constant.

---

## Where things stand

**State**: stable. Working tree clean, ~164 tests pass, all commits
pushed to `origin/main`.

**Test set**: 41 circuits in `tests/examples/`, three sources:
- 01–25: MySchematic suite + 2026-05-01 adversarial expansion;
- 26–36: real netlists from the sibling `myadc` SAR-ADC repo
  (batches 1–2);
- 37–41: sky130 stdcell library cells + hand-written probes for
  audit items C1/C2 (batch 3).
All 41 pass Tier 1 safety, deterministically (placer alignment is
now sorted + fixpoint-iterated; HashMap-order flakiness in Tier 1 is
gone, though Tier 2 scores still jitter slightly).

**Eval**: two-tier API, now with a fifth Tier 2 sub-score:
- Tier 1 — `SafetyReport { no_overlap, power_convention_clean,
  symmetry_clean }`. All hardened this cycle: overlap sizes subckt
  boxes (was blind to them), power_convention pairs each PMOS with
  its nearest same-column NMOS (|dx| < one symbol width), symmetry
  accepts vertical stacks as clean.
- Tier 2 — `QualityProfile { aspect_ratio, crossings, wire_length,
  label_ratio, text_clarity }`. `text_clarity` (label/caption
  collisions vs text and symbol bodies) joins the lex-min
  comparators but NOT the weighted sum (no tuned weight, on purpose).

**Pipeline changes this cycle** (all documented with reasoning in
`test_set_expansion_findings.md`):
- X instances without local `.subckt` defs get synthesized box
  symbols; placer templates budget their real footprints.
- X instances of PDK FET primitives (`*nfet*`/`*pfet*` model names)
  get full port semantics: DAG direction, layering, polarity sort.
- Isolated blocks (sources OR lone devices) relocate beside their
  loads when a pure y-shift can't help (the P3 remainder).
- Netlist comment directive `* n2s: power_net <name>` declares
  rails the tool cannot discover (audit C1). The `n2s:` namespace
  exists now — future hints (e.g. device polarity for C2) go there.

---

## Known gaps, deliberately left open

- **C2 (case 41)**: an M card with a foundry-opaque model name
  (`g45p1svt`) and bulk on a bias net renders PMOS-as-NMOS. No
  metric can catch a wrong symbol. Real fix = model-card lookup or
  an `n2s:` polarity hint; the test case keeps the gap visible.
- **Scale (cases 29/36)**: root-caused and designed, not yet built —
  see `docs/scale_placement.md` (2026-07-04). Headline findings: the
  "hairball" is a 261-layer 1-wide ribbon plus a 130-block ALAP dump
  layer; HAC has an early-exit bug (the max_cluster_size check
  `break`s ALL clustering on the first capped merge — case 36 ends
  with 678 singletons); pattern matchers don't accept X-FETs (the
  M-card DFF gets 8 pattern blocks, the identical X-card DFF gets 18
  singletons). Plan: Phase 0 = fix those two + re-measure (decision
  gate), Phase 1 = CMOS gate extraction → collapse to boxes → the
  existing Sugiyama at gate granularity.

## What to do next (in priority order)

(Source-pin input classification — the former #1 — landed on
2026-07-04: sources/emitters are block inputs, external diff-pair
tails are inputs, XPT sits beside its pair. See the findings doc.)

(SkyWater rails — the former #1 — resolved on 2026-07-04:
`vpwr`/`vgnd`/`vpb`/`vnb` joined the builtin vocabulary across all
five hardcoded rail lists; they are ecosystem-standard names, not
tuning. Cases 37–39 improved across the board.)

(Scale Phase 0 landed 2026-07-04: HAC early-exit fixed, X-FET
pattern matching on, case 29 transformed (33 layers → 2). Decision
from the re-measure: case 36 still needs Phase 1 — see
scale_placement.md for the numbers.)

(Scale Phase 1 landed 2026-07-04: `analyzer::gates` extracts
INV/NAND/NOR by channel-graph matching — exact on case 32's ground
truth — and collapses them to real-direction boxes in the scale
regime. Case 36: 173 gates, layers 131→67, canvas halved. Batch 4
added OpenRAM cases 42/43 as the prerequisite.)

(Scale Phase 2 landed 2026-07-04: depth folding — 36's ribbon became
a 2 930 x 10 570 banded page, shape 0.10 → 0.81 — plus kind-sorted
grid distribution for bus alignment, polarity class as primary key.)

(Scale Phase 3 landed 2026-07-04: high-fanout nets label-routed,
adaptive threshold capped, and A* auto-on at scale. Scoreboard:
29: 0.26→0.41, 36: 0.27→0.47, 42: 0.98, 43: 0.95. The scale campaign
in scale_placement.md is complete through Phase 3.)

(Routing Phase C resolved 2026-07-04 WITHOUT channel routing: the
~100 through-body survivors were mis-anchored label stubs, not failed
routes. A*-failure now labels instead of drawing dirty wires, and
label anchors are collision-aware. 36's through-body wires: 143 → 36.
See routing_improvement.md.)

1. **More external sources** — ngspice examples, Berkeley circuits.
2. **C2 real fix** — model-card lookup or an `n2s:` polarity hint
   directive (case 41 keeps the gap visible).
3. **True channel routing** — only if the last ~36 congestion stubs
   and trunk crossings ever matter enough; payoff now small.

### Avoid (unchanged from the 2026-05-01 audit)

Do not tune `ScoreWeights`, `merge_threshold`, `max_cluster_size`,
`bend_penalty`, `crossing_penalty`, or the `n2s-improve` presets.
If a regression appears, find the test data that triggers it; don't
move a constant until the suite says the constant is the problem.

---

## Useful commands

```bash
cargo build --release
cargo test                    # ~164 tests, all green expected

# Full sweep with the two-tier profile (txt= is the new column)
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  ./target/release/n2s "$f" -o "output/${name}.svg" -o "output/${name}.json"
  ./target/release/n2s-eval -n "$f" -s "output/${name}.json" --profile
done

# Render SVG → PNG for visual inspection (cairosvg is installed)
python3 -c "import cairosvg; cairosvg.svg2png(url='output/34_pmos_comparator_sky130.svg', write_to='/tmp/34.png', scale=2)"
```

---

## A reminder to future self

Two lessons this cycle, both rhymes of the 2026-05-01 one:

1. **Every blind spot you close catches real bugs the same day.**
   The overlap-sees-boxes fix immediately flagged three placement
   bugs; the C1 probe failed Tier 1 on first run. Metrics that can't
   see a defect class are worse than no metric — they certify junk.
2. **Guards must share geometry with the metrics they defend.**
   The first collision guard used a coarser size estimate than the
   overlap metric and broke a legitimate alignment; the fix was to
   make placer guards and eval checks read the same symbol rects.
   When two subsystems disagree about geometry, you get either false
   alarms or silent overlaps — never neither.
