# Scale Placement Design (cases 29/36: the "hairball" root-caused)

Status: DESIGN — written 2026-07-04, before any code. Companion to
`docs/routing_improvement.md` (which covers only the routing half and
explicitly assumes placement is sane).

## 1. The problem, measured

Debug stats from the placer (`N2S_DEBUG_STATS=1`, temporary
instrumentation, since removed), across the scale range:

| case | devices | blocks | singleton blocks | DAG edges | layers | layer-width profile | quality |
|---|---:|---:|---:|---:|---:|---|---:|
| 07 opamp (M) | 16 | 8 | 4 | 4 | 3 | 5,1,2 | 0.90 |
| 31 TG-DFF (M) | 16 | 8 | 0 | 11 | 3 | 3,2,3 | 0.41 |
| 37 TG-DFF (X) | 24 | 19 | **18** | 34 | 6 | 2,1,2,2,2,10 | 0.32 |
| 29 SAR logic (X boxes) | 92 | 87 | **86** | 127 | 33 | 15, then 1,1,1,…, ends 3,12,16 | 0.26 |
| 36 SAR flat (X FETs) | 684 | 679 | **678** | 21 802 | 261 | 51, then ~250 layers of width 1–4, ends 46,34,41,53,**130** | 0.27 |

Fanout distribution of case 36 (287 nets): 100 nets touch 2 devices
(series-stack midpoints), 127 touch 4 (gate outputs), ~12 hub nets
touch 28–120 (clock/control), rails touch 266–342.

The "hairball" is actually two specific degeneracies:

1. **A 1-wide chain**: 29 has 33 layers mostly of width 1; 36 has 261.
   Canvas width grows linearly with layer count (200px+ per layer),
   so the schematic becomes a kilometer-long ribbon of single boxes
   connected by labels.
2. **A terminal dump layer**: ALAP (`enforce_signal_flow`) pushes every
   weakly-constrained block as late as possible; 130 of 36's blocks
   (19%) pile into the last layer as an unstructured heap.

## 2. Root causes, ranked by certainty

### RC1 — HAC terminates on the first size-capped merge (a bug)

`cluster_hac`'s merge loop:

```rust
let merged_size = cluster_members[&best_a].len() + cluster_members[&best_b].len();
if merged_size > opts.max_cluster_size {
    break;          // <-- terminates ALL clustering
}
```

The `break` was meant to cap one cluster's size but exits the whole
loop. The *globally best-scoring* pair is recomputed each iteration;
once one hub-connected cluster fills to 6, that cluster keeps winning
the score race (hub nets give it the largest `total_weight`), the cap
check fires, and clustering stops with everything else untouched.
That is exactly the measured outcome: 36 produced ONE cluster of 6 and
678 singletons. Small circuits never trip this — everything merges
before any cluster fills — which is why 25 analog cases hid it.

Fix shape (Phase 0): make size-capped pairs *ineligible* (skip them in
the candidate scan) instead of terminating. Complexity note: the scan
is O(C²) per merge; at 684 devices the full loop is ~10⁸ pair checks —
still fine (<1 s), but worth measuring after the fix since the loop
will now run to completion instead of exiting early.

### RC2 — Pattern matchers are blind to X-FETs

`transistor_type()` (the gate for diff-pair/mirror/cascode/inverter
finders) accepts only `M`/`Q`. The 31-vs-37 pair is a controlled
experiment: the *same* TG-DFF topology in M cards yields
`4 Inverter + 4 CascodePair` blocks (3 clean layers); in X cards it
yields 18 singletons (6 layers, quality 0.41 → 0.32). Every sky130
netlist — a third of the suite — runs with zero pattern recognition.

Fix shape (Phase 0): let `transistor_type()` fall through to
`SpiceParser::infer_x_transistor_type`, mapping the (drain, gate,
source) convention that X-FET primitives already follow. The finders
themselves need no change.

### RC3 — Longest-path layering + ALAP degenerate on deep graphs

A gate-level netlist has logic depth 15–25; a *transistor*-level graph
of the same circuit has depth several times that (each gate
contributes 1–3 internal levels), and singleton blocks mean the DAG
sees transistors, not gates. Longest-path layering faithfully produces
261 layers because the graph really is that deep at transistor
granularity. ALAP then dumps the weakly-connected residue (reset
transistors, output buffers) into the final layer. Neither algorithm
is wrong; they are being fed the wrong granularity.

### RC4 — Shared-net HAC cannot recover gate structure

Even with RC1 fixed, HAC merges by shared-net count with a
`min(|A|,|B|)` normalizer. In a digital netlist the strongest
signals are hub nets (clk touches 120 devices in case 36), so HAC
gravitates toward "everything on clk" clusters, capped at 6 — an
arbitrary 6-pack, not a gate. The correct unit of structure — the
CMOS gate (complementary P/N networks between rails sharing inputs
and one output) — is invisible to net-counting.

### RC5 — Routing cannot rescue placement

36 scores cross=0.00 / wire=0.04 with ~1 700 wires. The A* plan in
`routing_improvement.md` reduces crossings *given* sane pin
positions; it cannot fix a 261-column ribbon. Placement first.

## 3. Design principle

**Recover the hierarchy, then reuse the pipeline we already trust.**

The existing Sugiyama flow handles 8–30 nodes well (every analog case
proves it). A gate-level view of case 36 is ~150–170 gates with logic
depth ~20 — inside or near that comfortable envelope, especially
after row-balancing. So the design is NOT a new placement engine; it
is a granularity fix feeding the existing engine bigger, meaningful
nodes — exactly the move that already worked once this cycle
(X instances → synthesized boxes → existing pipeline).

## 4. Phased plan

### Phase 0 — bounded fixes, then RE-MEASURE — **DONE 2026-07-04**

1. ✅ HAC early-exit fixed (capped pairs are skipped, not a stop signal).
2. ✅ `transistor_type()` accepts X-FETs.
3. ✅ Stats re-measured (`N2S_DEBUG_STATS=1` is now a permanent
   env-gated diagnostic on the placer).

Results:

| case | blocks (before → after) | singletons | layers | quality |
|---|---|---|---|---|
| 29 | 87 → **23** | 86 → 5 | 33 → **2** | 0.27 → 0.32 |
| 36 | 679 → **213** | 678 → 2 | 261 → **132** | 0.27 → 0.26 |
| 37 | 19 → **12** | 18 → 0 | 6 → 8 | 0.32 → 0.33 |

- **Case 29 is transformed**: 2 layers × grid columns, visually a
  near-legible gate-level SAR controller (bit-slices in columns).
- **Case 36 remains a ribbon** (132 layers) and its quality didn't
  move. Worse, mirror-matching misfires at transistor granularity:
  one "CurrentMirror" block glommed 56 clock-gated nfets (gate=clk,
  source=vss looks diode-adjacent to the matcher). **Decision: 36
  needs Phase 1 (gate extraction) — pattern matchers built for
  analog idioms cannot parse flat digital transistor soup, exactly
  as RC4 predicted.**
- Suite effect: net positive (21 +0.16, 33 +0.08, 37 +0.07, 29 +0.06,
  30 +0.04); case 35 dropped 0.76 → 0.47 — the bootstrap's
  non-textbook topology now triggers pattern matches that reshape
  its layout (crossings + text crowding). Recorded, not tuned away.
- Determinism follow-ups shaken out: the diff-pair and mirror
  finders iterated HashMaps (flaky Tier 1 on case 34 once X-FETs
  flooded them) — both now iterate sorted keys; and pair alignment
  gained a last-resort single-device move for passives (R/C/L leaf
  devices may leave their block when both whole-block shifts are
  vetoed — case 34's CL1/CL2 caps, scattered to opposite canvas
  ends, each blocked by a legitimate neighbor).
- Runtime at 684 devices: 0.19 s (HAC now runs to completion).

### Phase 1 — CMOS gate extraction (the structural answer to RC4)

A transistor-level structural matcher, run before HAC:

- Identify each **channel-connected component** between rails: the
  P-network (rail→output through pfets) and N-network (output→rail
  through nfets) sharing an output net and input set.
- Match against a small template library by network shape:
  INV, NAND2/3, NOR2/3, AOI/OAI (by series/parallel decomposition),
  TG (pass pair), half-latch (cross-coupled INV pair).
- Each match becomes a `GateBlock` collapsed to a synthesized box
  (reuse `create_subcircuit_symbol`; pins = the gate's I/O nets),
  labeled with the matched function (`NAND2`, `TG`, …).
- Unmatched residue falls through to HAC as today.

Rendering: at scale the gate boxes ARE the schematic (a gate-level
diagram is what an engineer wants for 684 FETs); a `--expand-gates`
flag can keep today's transistor view for small circuits. Regime
selection: extraction always runs; collapse-to-box engages when
device count > threshold (e.g. 60) AND gate coverage > ~70%. Analog
circuits fail the coverage test and keep the current path unchanged —
cases 01–28 must not change output at all (regression-gated).

### Phase 2 — width control at the gate level (RC3's residue)

With gates as nodes, depth ~20 is fine but some layers will be wide
(bit-slices: 8 identical DFFs in one layer). Two additions:

- **Row balancing**: split layers wider than `W ≈ sqrt(total)` into
  sub-rows (already exists as `compute_grid_columns`; verify it
  engages sanely at gate granularity).
- **Bus alignment**: generalize `align_matched_pairs` from pairs to
  N-member groups of identical gates (same match key) — bit-slice
  columns should align vertically. (The pair machinery, sorted keys +
  fixpoint + rect collision sim, extends naturally.)

### Phase 3 — routing at scale

Defer to `routing_improvement.md` (A* grid). Two scale notes for that
work: the obstacle grid at 36's canvas is large (build once, share —
already the design); and high-fanout nets (clk, reset, rails) must
stay label-routed regardless of A* — only 2–4 terminal nets earn
wires. The adaptive label threshold already points this direction.

## 5. Acceptance criteria

On cases 29 / 36 / 37 (and at least two NEW ≥100-device netlists from
outside the current corpus — see prerequisites):

- layer count ≤ 2× estimated logic depth (36: ≤ ~40, not 261);
- no terminal layer holding > 10% of all blocks;
- crossings sub-score ≥ 0.30, wire ≥ 0.30 (from 0.00/0.04);
- canvas area reduced ≥ 3× on 36;
- runtime ≤ 2 s at 700 devices (currently 0.15 s — headroom for the
  extraction pass, not a license to be slow);
- all 41 examples Tier 1 green, deterministically;
- cases 01–28 byte-identical output in default mode (regime gate).

## 6. Prerequisites and risks

- **Two test cases are not a design basis.** ✅ Batch 4 (2026-07-04)
  added cases 42/43 from OpenRAM-generated SRAM netlists (third
  author): a 131-instance hierarchical decoder tree and a
  130-instance replica bitcell column. The column case renders as a
  clean regular array (quality 0.775) already; the decoder scores
  0.331 (crossing storm in a single-layer grid). Both exposed a new
  known gap: **library subckt boxes have unknowable port
  directions**, so gate-level blocks form zero DAG edges
  (dag_edges=0, one flat layer). Phase 1's extracted gates don't
  have this problem — the template match itself identifies the
  output net. For library boxes it remains open (possible future
  `n2s: output_port` directive).
- **Gate-extraction correctness**: a wrong match renders a wrong box.
  Mitigation: match conservatively (exact template shapes only),
  label boxes with the matched function so errors are visible, and
  test extraction against the stdcell library netlists (cases 37–39)
  where ground truth is in the cell name.
- **Score churn**: Phase 0 alone will move Tier 2 numbers across the
  suite. Per the audit: no constant may be "re-tuned to compensate";
  regressions get investigated, not papered over.
- `merge_threshold` / `max_cluster_size` stay untouched (audit list);
  Phase 0 changes control flow, not constants.

## 7. Non-goals

- No force-directed / simulated-annealing engine.
- No change to the analog pipeline for small circuits.
- No new tunable constants without a sensitivity analysis first.
