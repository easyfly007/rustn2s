# Routing Improvement Plan

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

## Phase C: Channel Routing (Future)

After Phase B is complete. Core idea:
- Define horizontal/vertical channels between layout layers
- Assign wire tracks within channels
- Wires are ordered within channels without crossing
- Requires placer cooperation to reserve channel space

This is a larger effort (~500+ lines) and will be planned separately after Phase B results are evaluated.
