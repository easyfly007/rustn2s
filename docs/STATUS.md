# Project Status (2026-07-04, end of day)

A handoff note for whoever picks this up next (possibly a fresh
Claude session with no memory of today's 22 commits).

Read this first. Then `docs/scale_placement.md` for the scale
campaign (measured evidence → phased fixes, all landed),
`docs/test_set_expansion_findings.md` for the find/fix log of the
real-world validation campaign, and `docs/overfitting_audit.md`
before you feel tempted to tune any constant.

---

## Where things stand

**State**: stable. Working tree clean, 173 tests pass (~9 s: the
test profile builds with opt-level 1 because integration tests now
convert a 4256-instance netlist), all commits pushed to
`origin/main`. The full 49-case convert+eval sweep runs in ~2 s.

**Test set**: 49 circuits in `tests/examples/`, six authors:
- 01–25: MySchematic C++ suite + hand-written adversarial cases;
- 26–36: real netlists from the sibling `myadc` SAR-ADC repo;
- 37–39: SkyWater sky130 stdcell library cells;
- 40–41: hand-written audit probes (C1 LDO rail, C2 opaque models);
- 42–43: OpenRAM-generated SRAM decoder + replica column;
- 44–46: ngspice distribution examples + xschem Verilog-import
  post-PnR multiplier;
- 47–49: xschem audiodac (4256 instances, the scale record —
  first-run PASS, 1.2 s) + two original UC Berkeley SPICE2 decks
  (RCA 3040, 1966; the mosamp NMOS opamp).

All 49 pass Tier 1 safety, deterministically (every HashMap-ordered
decision point that affects geometry now iterates sorted keys; Tier 2
scores still jitter a little on a few circuits — case 16 flips
between 0.575/0.647 run to run).

**Eval**: two-tier API.
- Tier 1 — `SafetyReport { no_overlap, power_convention_clean,
  symmetry_clean }`, all hardened this cycle: overlap sizes every box
  (builtin, local-def, synthesized, gate), power_convention pairs
  each PMOS with its nearest same-column NMOS within one symbol
  width, symmetry accepts vertical stacks and excludes non-FET boxes
  and V/I sources from pairing.
- Tier 2 — `QualityProfile { aspect_ratio, crossings, wire_length,
  label_ratio, text_clarity }`. `text_clarity` (text-vs-text and
  text-vs-body collisions, mirroring the SVG renderer's geometry) is
  in the lex-min comparators but deliberately NOT in the weighted sum.
- Metric-artifact warning that keeps proving itself: wire-less
  components inflate ratio denominators (case 46 scored HIGHER with
  700 filler boxes than without), and zero-crossing scores can mean
  "no real wires" rather than "clean routing". Trust the eye first.

**Pipeline capabilities added this cycle** (each with reasoning in
the findings doc or scale_placement.md):
- Synthesized box symbols for X instances without local defs; real
  footprints budgeted throughout placer templates and eval.
- X-FET port semantics (`sky130_fd_pr__*fet` etc.): DAG direction,
  polarity sorting, and pattern matching all work on all-X netlists.
- Source/emitter nets classify as block inputs (tail/header devices
  sit beside the pairs they feed).
- Netlist directives (comment-based, `* n2s: <directive> ...`):
  `power_net <name>...` (audit C1) and `pmos_model / nmos_model
  <name>...` (audit C2). Both audit items are CLOSED.
- SkyWater rail vocabulary (vpwr/vgnd/vpb/vnb) in all five hardcoded
  rail lists.
- **Scale regime** (>= 60 components, shared across placer and
  router): CMOS gate extraction collapses INV/NAND2/3/NOR2/3 into
  labeled boxes (exact on case 32's ground truth); depth folding
  wraps >= 12-layer designs into square-ish bands; high-fanout nets
  (> 4 pins) are label-routed; the adaptive label threshold stops
  growing with the canvas; A* obstacle avoidance turns on
  automatically. Small/analog circuits are untouched by ALL of this.
- Collision-aware label anchors (candidate ladder, checked against
  body rects and other labels) and A*-failure → label fallback:
  through-body wires on case 36 went 143 → 36.
- Physical-only cells (X instances with all-power pins) filtered by
  default; `--keep-physical-cells` opts out.
- Performance: A* uses sparse state + an expansion budget (was
  ~50 MB dense allocation per call); HAC scores only net-sharing
  cluster pairs (was all-C² scans). Case 46: 42 s → 0.16 s.
- `N2S_DEBUG_STATS=1` prints placer structure stats (blocks,
  singletons, DAG edges, layer widths) to stderr — the diagnostic
  that root-caused the scale campaign.

---

## What's left (in priority order)

1. **Corpus growth / periodic re-validation** — six authors and 49
   cases; batch 6 (audiodac + Berkeley decks) found only one
   self-inflicted footgun (embedded title lines parse as devices
   when a header displaces them from line 1 — comment them out).
   Untapped: xschem `.sch` designs (need xschem to netlist), more
   Berkeley decks (`schmitt.cir`, `diffpair.cir` in ngspice
   tests/general), sky130_fd_sc_hvl cells.
2. **True channel routing** — only if the last ~36 congestion label
   stubs on case 36 or trunk-crossing aesthetics ever matter enough;
   `routing_improvement.md` explains why the payoff shrank.
3. **Small visual nits** (accumulate before acting): pin-number
   digits on synthesized boxes are mild noise; power symbols crowd
   the gap between tightly paired boxes; case-16-class Tier 2 jitter.

### Avoid (unchanged from the 2026-05-01 audit)

Do not tune `ScoreWeights`, `merge_threshold`, `max_cluster_size`,
`bend_penalty`, `crossing_penalty`, or the `n2s-improve` presets.
The scale-regime gates (60 components, 4-pin fanout, 12 layers, 0.5
gate coverage) are REGIME SELECTORS calibrated against measured
specimens, documented in scale_placement.md — they are not layout
knobs and also should not be nudged to move a score. If a regression
appears, find the test data that triggers it.

---

## Useful commands

```bash
cargo build --release
cargo test                    # 173 tests, all green expected

# Full sweep with the two-tier profile (~0.6 s for all 46)
mkdir -p output
for f in tests/examples/*.sp; do
  name=$(basename "$f" .sp)
  ./target/release/n2s "$f" -o "output/${name}.svg" -o "output/${name}.json"
  ./target/release/n2s-eval -n "$f" -s "output/${name}.json" --profile
done

# Placer structure diagnostics (the scale-campaign workhorse)
N2S_DEBUG_STATS=1 ./target/release/n2s tests/examples/36_sar_logic_flat_sky130.sp -o /tmp/x.json

# Render SVG → PNG for visual inspection (cairosvg is installed;
# scale down for the big cases)
python3 -c "import cairosvg; cairosvg.svg2png(url='output/46_spm_postpnr_sky130.svg', write_to='/tmp/46.png', scale=0.5)"

# Netlist directives (in-file comments)
#   * n2s: power_net vreg
#   * n2s: pmos_model g45p1svt
#   * n2s: nmos_model g45n1svt
```

---

## Lessons this cycle (rhymes with 2026-05-01's)

1. **Every blind spot you close catches real bugs the same day.**
   Overlap-sees-boxes flagged three placement bugs within minutes;
   the C1 probe failed Tier 1 on first run; case 46 found a Tier 1
   bug plus two performance cliffs on its first conversion.
2. **Guards must share geometry with the metrics they defend.**
   Placer collision guards, eval overlap, and label placement all
   read the same symbol rects now; every time two subsystems had
   different ideas about size, we got either false vetoes (case 06)
   or silent overlaps (case 34).
3. **Diagnose before building.** Phase C's planned ~500-line channel
   router dissolved into two 30-line fixes once the "through-body
   wires" were actually inspected (they were mis-anchored label
   stubs). The scale campaign's biggest wins were a one-line `break`
   bug and a match-arm addition.
4. **Fixed pipelines want regime gates, not new constants.** Scale
   behavior changes are all gated on measured regime selectors, so
   46 analog circuits stay bit-identical while the big ones get a
   different strategy.
