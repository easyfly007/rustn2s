# Routing Improvement Plan

## Status

- **Phase B (A\* obstacle-aware routing)** — Implemented and shipped as
  **opt-in** behind `--obstacle-avoidance` (default off). See "Phase B —
  results and trade-offs" below.
- **Phase C (channel routing)** — Future, designed below.

## Phase B: A* Grid Routing

### Problem

Current router uses simple L-shaped routing (`l_route_best`) that is unaware of component obstacles. Wires can pass through component symbols, reducing schematic readability.

### Current Flow (`src/router/mod.rs`)

1. MST (Prim) determines pin-to-pin connection topology per net
2. Short edges (< 300 units): L-route — pick horizontal-first or vertical-first by fewer crossings
3. Long edges (>= 300 units): place labels + stub wires
4. Power nets: place power symbols directly, no wires

### Design

**Only replace `l_route_best()` with A\* pathfinding.** Everything else (MST, labels, power symbols) stays unchanged.

#### 1. Obstacle Grid

```
struct ObstacleGrid {
    min: Point,           // grid origin (world coords)
    cols: usize,
    rows: usize,
    grid_size: f64,       // cell size (10.0)
    blocked: Vec<bool>,   // cols x rows
}
```

Build process:
1. Expand `PlacementResult.bounding_rect` by ±50 units margin
2. For each placed component: compute rotated/mirrored bounding box from `SymbolDef.bounding_rect()`, mark cells as blocked
3. Inflate obstacles by 1 cell (10 units) for clearance
4. Pin positions are NOT blocked (they are route endpoints)

#### 2. A* Search

- State: `(col, row)` grid coordinates
- Neighbors: 4-directional Manhattan
- Cost `g`: 1.0 per step, **+0.5 bend penalty** (encourages straight lines)
- Heuristic `h`: Manhattan distance to goal
- Fallback: if no path found, use original L-route (graceful degradation)

Path simplification: merge collinear segments.

#### 3. Wire-as-Obstacle

After routing each wire, mark its path cells as soft-blocked (higher cost), so subsequent wires prefer different tracks. Route shorter edges first.

#### 4. Integration

```
src/router/astar.rs  — NEW (~250 lines): ObstacleGrid + A* + simplify_path
src/router/mod.rs    — MODIFY: build grid, call A* in route_signal_net
src/main.rs          — OPTIONAL: --no-obstacle-avoidance flag
```

New `RouterOptions` fields:
- `avoid_obstacles: bool` (default: true)
- `bend_penalty: f64` (default: 0.5)

### Key Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| grid_size | 10 units | already used for snap |
| grid margin | 50 units | extra space around bounding rect |
| inflate | 1 cell | obstacle clearance |
| bend penalty | 0.5 | per direction change |
| wire cost | +2.0 | soft-block for routed wires |

### Validation

Compare SVG outputs before/after. Key test cases:
- `04_nmos_common_source` — wires should not cross MOSFET bodies
- `07_two_stage_opamp` — complex routing with multiple wire avoidance
- `08_bandgap_reference` — dense layout, verify fallback works

Use `n2s-eval` to verify `wire_crossings` metric decreases.

---

### Phase B — results and trade-offs (as shipped)

Implemented in `src/router/astar.rs` + `src/router/mod.rs` (gated on
`RouterOptions::avoid_obstacles`, exposed by the CLI as
`--obstacle-avoidance`, off by default).

Key implementation choices that differ from the original design:

1. **L-route first, A\* as fallback.** `route_signal_net` always tries
   `l_route_best` first and runs `polyline_clear` against the obstacle
   grid; A\* is only invoked when the L-route would walk through a
   component body. This keeps simple circuits at L-route quality and
   avoids A\*'s detour cost when there is no obstacle.

2. **No inflation around blocked rects.** The symbol's bounding rect
   already includes pin-stub offsets (e.g. NMOS pin G at x=-30 from a
   body at x=-10), so even one cell of clearance seals off most routing
   channels around dense placements. We block exactly the bounding rect
   and re-clear pin cells + one cell outward in the pin direction.

3. **Wire-aware crossing penalty (Phase B step 3, shipped).** Each
   routed wire stamps its cells with a horizontal/vertical orientation
   flag (`ObstacleGrid::mark_wire_orientation`). When A* later expands
   a neighbor, stepping into a cell perpendicular to its existing wire
   adds `crossing_penalty` (default 20.0) to the step cost. This
   discourages new crossings without forcing detours when there is no
   alternative. MST edges are also routed shortest-first so tightly-
   constrained pin pairs commit before flexible long ones.

#### Score comparison (11 test circuits, default vs `--obstacle-avoidance`)

| Example | Default | `--obstacle-avoidance` | Δ |
|---------|:---:|:---:|:---:|
| 01 voltage divider | 1.000 | 1.000 | — |
| 02 RC filter | 0.844 | 0.844 | — |
| 03 halfwave rectifier | 0.838 | 0.838 | — |
| 04 NMOS common-source | 0.979 | 0.979 | — |
| 05 NMOS current mirror | 0.967 | 0.967 | — |
| 06 BJT diff pair | 1.000 | 1.000 | — |
| **07 two-stage opamp** | 0.978 | **0.871** | **-0.107** |
| **08 bandgap** | 0.950 | **0.840** | **-0.110** |
| 09 inverter chain | 0.991 | 0.961 | -0.030 |
| **10 opamp feedback** | 0.952 | **0.853** | **-0.099** |
| 11 RLC controlled | 1.000 | 1.000 | — |

The losses on 07/08/10 come from two sources: A\*'s detour around
component bodies introduces new wire crossings (eval weight 0.15), and
the longer total wire length drops the wire-length sub-score for
circuits where the L-route was just blowing through bodies but staying
short.

The `tests/pipeline.rs::obstacle_avoidance_keeps_wires_off_component_bodies`
integration test verifies the **visual** improvement: with A\* enabled,
the count of wire-points inside any component bounding rect for example
04 drops below the L-router baseline. So the trade-off is clear: better
schematic readability, slightly worse score.

The wire-aware crossing penalty (Phase B step 3) cut some of the new
crossings on dense circuits but did not eliminate them — when placement
density makes detours infeasible, A* still has to cross. A* therefore
remains opt-in. Becoming default-on would need either better placement
(more whitespace between blocks) or a smarter routing model (rip-up &
reroute, channel routing, etc).

---

## Phase C: Channel Routing (Future)

After Phase B is complete. Core idea:
- Define horizontal/vertical channels between layout layers
- Assign wire tracks within channels
- Wires are ordered within channels without crossing
- Requires placer cooperation to reserve channel space

This is a larger effort (~500+ lines) and will be planned separately after Phase B results are evaluated.
