# Test Set Expansion: Findings (2026-05-01)

Summary of the bugs and surprises uncovered when expanding the test
suite from 11 to 25 circuits, in response to the
[overfitting audit](overfitting_audit.md). The expansion was driven by
the audit's diagnosis that the original 11 circuits saturated three of
seven score sub-dimensions and didn't exercise several of the audit's
predicted blind spots.

The results justify the expansion ten times over — adding 14 circuits
revealed **four real bugs** (three in shipped algorithms, one in lib.rs's
default mode) and **two evaluator gaps**. None of these were visible
under the original suite.

## Bugs uncovered

### Bug 1 — PMOS-below-NMOS sort fails across DAG layers (affects Phase 2.3)

**Circuit**: `13_pdk_mos_model_names.sp`

`sort_blocks_by_polarity()` (Phase 2.3) reorders blocks **within a
single DAG layer** so PMOS-only blocks come above NMOS-only blocks.
But when a matched MOSFET pair is split across layers — because the
analyzer puts them in different functional blocks — the within-layer
sort can't help. In 13, M1 (NMOS, layer 1) ends up at y=140 and M2
(PMOS, layer 2) ends up at y=220, putting PMOS *below* NMOS, the
opposite of the convention.

**Score impact**: `power_convention=0.0` (vs. 1.0 on every original
test circuit). Lowers overall score from 1.000 to 0.900.

**Severity**: medium — only affects circuits where PMOS and NMOS go
to different DAG layers, which happens in real designs but didn't
appear in the original suite.

### Bug 2 — Multiple isolated sources collapse to the same position (affects Phase 2.4)

**Circuit**: `14_disconnected_filters.sp`

`fix_isolated_source_layers()` (Phase 2.4) places blocks with no DAG
edges at the same layer as the non-isolated block sharing the most
nets. When two isolated sources both have **zero shared nets** with
anything, they both fall back to the same default position — V1 and
V2 end up at identical (x, y).

**Score impact**: `overlap=0.0` (vs. 1.0 on every original test
circuit). 14's overall score = 0.800.

**Severity**: high — any netlist with two genuinely-disconnected
sub-circuits (multi-domain mixed-signal, decoupling networks, etc.)
hits this bug.

**Note**: `22_three_isolated_sources.sp` does NOT reproduce the bug
because the V/R pairs cluster into the same HAC group, so the
sources never become "isolated" in the placer's sense. The bug
surface is subtler than the audit predicted.

### Bug 3 — Default mode silently renders only the first .subckt's interior

**Circuit**: `25_subckt_array.sp` (and retrospectively 09 and 10)

`src/lib.rs:64` has this branch:

```rust
} else if has_subckt_defs {
    // Flat mode: use first subcircuit's internal devices
    (&pr.subcircuits[0].devices, HashMap::new())
}
```

If a netlist contains *both* `.subckt` definitions *and* top-level
content (X instances + V/R/etc.), and the user has not passed
`--hierarchical`, the pipeline renders **only the first subckt's
internal devices** and silently drops the top-level entirely.

For `25_subckt_array.sp` (8 top-level items including 3 X instances
and 3 C loads), this means `comps=2` — the INV's M1/M2.

For `09_inverter_chain_hier.sp` and `10_opamp_feedback_hier.sp`, this
means **every "default-mode" score we ever quoted for those circuits
was scoring the INV / OPAMP subckt's *interior layout*, not the
top-level circuit the user wrote.**

**Score impact**: indirectly, every quoted score for 09 / 10 in
`docs/improve.md`, `docs/architecture.md`, and the README is
measuring the wrong artifact.

**Severity**: high — the documented results in multiple docs are
based on a misinterpretation. Phase 4.2 / 4.3 / 4.4's "+0.01 / +0.07
on 10" claims are about the OPAMP's interior, not the feedback
amplifier the user wrote.

### Bug 4 — Long flat chains stack vertically instead of cascading horizontally

**Circuits**: `16_inverter_chain_5stage.sp` (5 inverter stages),
`21_deep_signal_chain.sp` (10 RC stages)

**Status update (2026-05-01, after attempted fix)**: this turned out
to be a **score-formula bias, not a placement bug**. Inspecting the
actual placement of circuit 16:

```
M1p (0,0)    M1n (0,80)
V1  (260,0)  V2  (260,140)
M2p (520,0)  M2n (520,80)
M3p (780,0)  M3n (780,80)
M4p (1040,0) M4n (1040,80)
M5p (1300,0) M5n (1300,80)
C1  (1560,0)
```

Each inverter stage gets its own DAG layer; the chain runs
horizontally left-to-right with PMOS above NMOS. The bbox is roughly
1560 wide × 140 tall, and that is the *correct* schematic for a
5-stage cascade — a circuit engineer would draw it that way.

The eval scores 0.723 because `aspect_ratio` is `max(w/h, h/w)` —
the formula treats wide-and-shallow identically to tall-and-narrow
and penalizes both. So a legitimate horizontal cascade is graded the
same as a pathological vertical stack, even though one is good
schematic style and the other isn't.

**Two attempted fixes were tried and reverted**:

1. *Multi-column reflow inside the Unknown block template*. Broke
   circuit 11 (1.000 → 0.800) by changing block widths and
   propagating overlap into adjacent blocks.

2. *Linear-chain detection in `annotate_cluster`* — break long chains
   into one SingleDevice block per device, expecting Sugiyama
   layering to spread them. Broke circuit 09 (1.000 → 0.788) by
   producing a single-row 5-device cascade with aspect_ratio 10.5
   (the "spread them horizontally" outcome the score punishes).

Both reverts confirmed the underlying truth: **the placement is
already doing the right thing; the score formula is the problem.**

**Score impact**: 16 stays at 0.723 and 21 at 0.866. These are floor
values of the current scoring formula on *correct* layouts.

**Severity reclassified**: not a code bug. The proper response is
either:

(a) Change `aspect_ratio` to be less aggressive on wide-but-shallow
    bboxes — but doing this just to recover 16 / 21's score is the
    exact "tune the metric to make scores look good" overfitting the
    audit warned against.

(b) Stop reporting a single weighted overall score; expose the
    seven sub-scores as a profile and let consumers pick what
    matters for their use case.

Option (b) is the recommendation. It's a UI / API change, not an
algorithm change, so it doesn't disturb the placer.

**Severity**: low (no algorithm fix needed).

## Evaluator gaps uncovered

### Gap 1 — `symmetry` sub-score is vacuously 1.0 when no pairs exist

**Circuit**: `20_asymmetric_pair.sp`

`eval/symmetry.rs` returns `overall_score = 1.0` when `matched_pairs`
is empty. This makes the metric unable to distinguish:
- "the layout is perfectly symmetric" (intended 1.0), from
- "there is nothing here to be symmetric about" (vacuous 1.0), from
- "two devices look like a pair but aren't grouped because their
  attributes differ" (current behavior: vacuous 1.0).

Combined with the original audit finding that symmetry was at 1.0
on every original circuit, **the symmetry sub-score is currently
incapable of producing useful signal on the test set**.

### Gap 2 — `connectivity.found_net_count` only counts labels and power symbols

**Circuit**: `15_pi_attenuator.sp`

`expected_nets=6` but `found_nets=3`. The pi-attenuator's nets are
mostly connected by direct wires (no labels needed because they're
short), so they don't appear in `found_net_count` even though they
are properly routed.

**Severity**: low — doesn't feed into the overall score, but the
field is misleading when read from the eval JSON.

## Sensitivity sweep update on 25 circuits

The sweep originally run on 11 circuits showed `overlap`, `symmetry`,
and `power_convention` were **dead** — perturbing them produced zero
ranking changes. Re-running on 25 circuits:

| Sub-score | Was dead on 11? | Still dead on 25? |
|---|---|---|
| `overlap` | ✓ dead | **alive** — `−0.10` shifts 14 by 7 ranks |
| `symmetry` | ✓ dead | still dead — no circuit yet hits the actual symmetry penalty |
| `power_convention` | ✓ dead | mostly dead — score moves but rank doesn't, because tied at 1.0 with many others |
| `crossings` | medium-active | medium-active (unchanged) |
| `aspect_ratio` | most active | still most active |
| `wire_length` | low-active | low-active |
| `label_ratio` | medium-active | medium-active |

Per-circuit score range under ±0.10 weight perturbation:

| Bottom of new table | range under perturbation |
|---|---:|
| 13_pdk_mos_model_names | 0.200 |
| 14_disconnected_filters | 0.200 |
| 17_folded_cascode_opamp | 0.129 |
| 03_halfwave_rectifier | 0.162 |
| 02_rc_lowpass_filter | 0.156 |

**The middle-of-table circuits are now even more weight-sensitive**
than the original 11 — 13 and 14 swing 0.200 between weight choices,
larger than the algorithmic gains of any individual phase shipped
this year.

## Summary table (all 25 circuits, baseline weights)

```
   1. 01_voltage_divider           1.000
   2. 06_bjt_diff_pair             1.000
   3. 11_rlc_controlled_sources    1.000
   4. 12_industrial_power_names    1.000  (V-source-terminal rule rescues VBAT/VIO)
   5. 20_asymmetric_pair           1.000  (vacuous — Gap 1)
   6. 18_star_fanout               0.994
   7. 09_inverter_chain_hier       0.991  (BUG 3 — measuring wrong thing)
   8. 04_nmos_common_source        0.979
   9. 15_pi_attenuator             0.975
  10. 07_two_stage_opamp           0.974
  11. 23_pmos_input_inverter       0.959
  12. 10_opamp_feedback_hier       0.952  (BUG 3 — measuring wrong thing)
  13. 22_three_isolated_sources    0.950
  14. 24_dense_amp_array           0.938
  15. 25_subckt_array              0.916  (BUG 3 — only 2 of 8 components rendered)
  16. 19_degenerated_diff_pair     0.903
  17. 13_pdk_mos_model_names       0.900  (BUG 1 — PMOS-below-NMOS)
  18. 05_nmos_current_mirror       0.892
  19. 08_bandgap_reference         0.875
  20. 21_deep_signal_chain         0.866  (BUG 4 — long-chain stacking)
  21. 02_rc_lowpass_filter         0.844
  22. 03_halfwave_rectifier        0.838
  23. 14_disconnected_filters      0.800  (BUG 2 — V1/V2 overlap)
  24. 17_folded_cascode_opamp      0.750  (real complexity, not a bug)
  25. 16_inverter_chain_5stage     0.723  (BUG 4 — long-chain stacking)
```

## Triage: what to fix, what to leave

### Fix now (real bugs, single-circuit reproduction, low-risk fix)

1. **Bug 3 — subckt-default-mode silent dropping**.
   `lib.rs:64` should clearly distinguish "user wrote a stand-alone
   subcircuit definition with no top-level usage" (current behavior is
   reasonable) from "user wrote a hierarchical netlist with both
   subckts and top-level instances" (current behavior is wrong; it
   should auto-flatten the X instances or at minimum warn loudly).
   This is a documentation bug as much as a code bug — every doc that
   quotes 09's or 10's score is silently misleading.

2. **Bug 2 — isolated-source overlap**.
   `fix_isolated_source_layers` should disambiguate "two isolated
   sources" by giving them distinct columns rather than collapsing
   both to the default. Probably a few lines in placer/mod.rs.

### Investigate (real bugs, but the right fix isn't obvious)

3. **Bug 1 — PMOS-below-NMOS across DAG layers**.
   The within-layer sort assumption is fundamentally limited.
   Cross-layer alignment is conceptually similar to Phase 2.2's
   pair alignment but for polarity. Needs design.

4. **Bug 4 — long-chain stacking**.
   This is the same issue 02/03 hit but on bigger circuits. Likely
   the right fix is a "linear-chain" detection in the placer that
   bypasses HAC and uses pure DAG-layer placement.

### Leave for now (evaluator gaps; lower priority than fixing bugs)

5. **Gap 1 — vacuous symmetry score**.
   Not a bug exactly, but the symmetry sub-score doesn't carry
   information when the test set has no asymmetric pairs. Adding
   "no matched pairs found" → score = N/A (excluded from weighted
   sum, with weights renormalized) would be more honest.

6. **Gap 2 — connectivity found_net_count is misleading**.
   Cosmetic; doesn't affect the overall score.

## What this expansion does NOT settle

- **The audit's high-risk items A1–A5** (score weights, advisor
  thresholds, search presets, aspect-ratio brackets) remain unaddressed.
  Even with 25 circuits, three of seven sub-dimensions are still mostly
  inert. Real ablation work on the score weights themselves is the
  natural next step.
- **Audit's C1 (power names)** is partially mitigated by the
  V-source-terminal rule, but a netlist whose power net comes from
  an ldo subckt or a control IC pin would still fail.
- **Audit's C2 (MOS keywords)** falls through to bulk inference for
  the cases tested. A truly broken case (e.g. a PMOS with bulk = a
  non-power node, model name = `g45p1svt`) is hard to construct
  realistically.

These remain open and worth follow-up later.

---

# Batch 2 expansion (31–36, 2026-07-03)

Six more real circuits from `myadc` (see `docs/examples.md` for the
table). Everything below is from running the pipeline and *looking at
the SVGs*, per STATUS.md's "the eye tells you what an engineer would
object to".

## Found and fixed

1. **`power_convention` metric false positive (Tier 1)** — case 31
   (TG DFF) places two CMOS inverters stacked vertically in one
   column, each internally P-above-N. The metric compared every PMOS
   against every NMOS in the column, so the lower stage's PMOS
   (y=220) vs the upper stage's NMOS (y=80) was flagged and Tier 1
   failed on a perfectly good layout. Fixed: each PMOS is now checked
   only against its *nearest* NMOS in the column (stage-local
   pairing). An upside-down stage is still caught (unit-tested). All
   36 examples pass Tier 1 after the fix; no other case's score
   changed.

## Open visual defects (not yet fixed)

2. **Net-label boxes drawn on top of subckt-box symbols** (33, 34,
   35). ~~When a device renders as a subckt box, pin net-labels are
   placed at the pin position, which is on/inside the box outline.~~
   **FIXED (2026-07-03, follow-up commits).** Root cause was not the
   label offset at all: X instances whose subckt is defined only in a
   `.lib`/`.include` (all sky130 primitives) had **no SymbolDef** —
   the router collapsed every pin (and its label) to the component
   centre and the SVG drew a blank fallback rectangle. Fix 1:
   `convert_full` now synthesizes a generic numbered-pin box symbol
   for every X model without a local definition, so pins sit on box
   edges and labels land beside them. Fix 2: the synthesized boxes
   are wider than the 60px the placer's side-by-side templates
   assumed, which made matched pairs overlap; `layout_block` now
   budgets real footprints (`subckt_box_size`) for DiffPair /
   CurrentMirror / Unknown-pair pitches. Two things the score never
   saw: label-on-box illegibility (34 scored quality=0.77 while
   illegible) and the box-pair overlap — the `overlap` eval skips
   any component whose symbol_name isn't a *builtin* symbol, so
   subckt boxes are invisible to the Tier 1 no_overlap check. That
   evaluator gap is now the open item (see below).

3. **Isolated blocks still float** (34: `VVDD` far left, `XPT`
   detached; 35: `XNC` alone bottom-right). The P3 partial fix
   (y-align only) does not help when the source/blocks would need an
   x+y relocation. Same limitation documented in commit `0b14d54`.

4. **Scale (case 36, 684 devices)**: pipeline is fast (~0.15 s) and
   Tier 1 passes, but the layout is a hairball — `cross=0.00`,
   `wire=0.04`, shape=0.15. HAC + Sugiyama produce a very wide,
   sparse canvas. This is the documented scale ceiling, now with a
   7×-larger probe than case 29.

## New evaluator gap found while fixing defect 2

- **`eval/overlap` is blind to subckt boxes.** ~~It sizes components
  from `builtin_symbols::all()` only and skips unknown symbol names.~~
  **FIXED (2026-07-04).** `evaluate()` now builds a subckt symbol
  table from the netlist (local defs + synthesized boxes for X models
  without one, superset over all pipeline modes) and hands it to the
  overlap check. Turning the check on immediately flagged three real
  overlaps the eye had missed or tolerated:
  - case 29 `XSR7`/`XBR7` — vertical stack pitch didn't budget box
    heights (7-port boxes are 90px tall at an 80px step). Fixed: the
    Unknown template now advances rows by real footprint heights.
  - case 34 `XNR1`/`XNR2` — `align_matched_pairs` collapsed a
    same-column pair onto identical coordinates. Fixed: same-column
    guard.
  - case 30 `XP2`/`XN_TAIL` — the same pass shifted a block onto an
    unrelated device with no collision check. Fixed: the shift is
    simulated first against the *same* symbol bounding rects the
    overlap metric uses (a coarser estimate initially broke case 06's
    legitimate alignment — guard and metric must share geometry), and
    if the preferred block can't move safely the pass tries the other
    block (which is what rescues 30's tail pair).
  Related `symmetry` refinement: a matched pair stacked in one column
  (x_diff ≈ 0) now scores 1.0 — a vertical/cascode arrangement is a
  clean layout, and for a same-column pair it is the only one that
  doesn't overlap. Diagonal misalignment still scores low.

## Still uncovered after batch 2

- **C1** (power net not sourced by a V/I device) — no myadc netlist
  exercises this; needs an LDO-style case.
- **C2** (M card with non-matching model name AND non-power bulk) —
  the real bulk-on-signal circuits in myadc (bootstrap variants) are
  either `nch`/`pch` M cards (keyword check hits first) or sky130 X
  instances (no M-card inference at all). A realistic C2 specimen
  still doesn't exist in this corpus.
