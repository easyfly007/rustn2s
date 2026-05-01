//! Obstacle-aware Manhattan routing on a fixed grid.
//!
//! `build_grid` rasterizes all placed components into a 2D occupancy grid;
//! `find_path` runs A* over that grid with a bend penalty so wires prefer
//! straight runs over jagged ones; `simplify_path` collapses collinear
//! segments back into a minimal polyline.
//!
//! Pin cells (and one cell just outward of each pin) are explicitly cleared
//! after blocking, so the path always has a corridor to escape from a
//! component edge.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::model::{Component, PinDirection, Point, SymbolDef};

/// Rectangular grid of cells, each either blocked (hard obstacle) or
/// optionally carrying a soft cost (already-routed wire).
#[derive(Clone)]
pub struct ObstacleGrid {
    pub origin: Point,
    pub cols: usize,
    pub rows: usize,
    pub cell_size: f64,
    blocked: Vec<bool>,
    soft_cost: Vec<f64>,
}

impl ObstacleGrid {
    /// Create an empty grid covering `min..max` plus a 50-unit margin on
    /// each side, with cells of `cell_size` units.
    pub fn new(min: Point, max: Point, cell_size: f64) -> Self {
        const MARGIN: f64 = 50.0;
        let origin = Point::new(min.x - MARGIN, min.y - MARGIN);
        let width = (max.x - min.x) + MARGIN * 2.0;
        let height = (max.y - min.y) + MARGIN * 2.0;
        let cols = (width / cell_size).ceil() as usize + 1;
        let rows = (height / cell_size).ceil() as usize + 1;
        Self {
            origin,
            cols,
            rows,
            cell_size,
            blocked: vec![false; cols * rows],
            soft_cost: vec![0.0; cols * rows],
        }
    }

    fn idx(&self, col: usize, row: usize) -> usize { row * self.cols + col }

    fn in_bounds(&self, col: i32, row: i32) -> bool {
        col >= 0 && (col as usize) < self.cols
            && row >= 0 && (row as usize) < self.rows
    }

    /// Convert a world point to integer cell coordinates (may be out of bounds).
    pub fn world_to_cell(&self, p: Point) -> (i32, i32) {
        let col = ((p.x - self.origin.x) / self.cell_size).round() as i32;
        let row = ((p.y - self.origin.y) / self.cell_size).round() as i32;
        (col, row)
    }

    /// Convert cell coordinates back to a world point at the cell's center.
    pub fn cell_to_world(&self, col: i32, row: i32) -> Point {
        Point::new(
            self.origin.x + col as f64 * self.cell_size,
            self.origin.y + row as f64 * self.cell_size,
        )
    }

    pub fn is_blocked(&self, col: usize, row: usize) -> bool {
        self.blocked[self.idx(col, row)]
    }

    pub fn cell_cost(&self, col: usize, row: usize) -> f64 {
        self.soft_cost[self.idx(col, row)]
    }

    /// Hard-block all cells whose center lies inside `[min..max]` inflated
    /// by `inflate_cells` cells of clearance.
    pub fn block_rect(&mut self, min: Point, max: Point, inflate_cells: i32) {
        let (cmin, rmin) = self.world_to_cell(min);
        let (cmax, rmax) = self.world_to_cell(max);
        let cmin = (cmin - inflate_cells).max(0) as usize;
        let cmax = (cmax + inflate_cells).min(self.cols as i32 - 1).max(0) as usize;
        let rmin = (rmin - inflate_cells).max(0) as usize;
        let rmax = (rmax + inflate_cells).min(self.rows as i32 - 1).max(0) as usize;
        for r in rmin..=rmax {
            for c in cmin..=cmax {
                let i = self.idx(c, r);
                self.blocked[i] = true;
            }
        }
    }

    /// Clear the single cell containing `world`, if any.
    pub fn unblock_at(&mut self, world: Point) {
        let (c, r) = self.world_to_cell(world);
        if self.in_bounds(c, r) {
            let i = self.idx(c as usize, r as usize);
            self.blocked[i] = false;
        }
    }

    /// Bump soft cost along a polyline. Each cell encountered along the
    /// segments gets `cost` added to its existing soft cost so subsequent
    /// A* searches prefer different tracks.
    pub fn add_wire_cost(&mut self, points: &[Point], cost: f64) {
        for window in points.windows(2) {
            self.add_segment_cost(window[0], window[1], cost);
        }
    }

    fn add_segment_cost(&mut self, a: Point, b: Point, cost: f64) {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 { return; }
        let steps = (len / self.cell_size).ceil() as usize + 1;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let p = Point::new(a.x + dx * t, a.y + dy * t);
            let (c, r) = self.world_to_cell(p);
            if self.in_bounds(c, r) {
                let idx = self.idx(c as usize, r as usize);
                self.soft_cost[idx] += cost;
            }
        }
    }
}

/// One-cell offset in world coordinates, in the pin's outward direction
/// (after the component's rotation and mirror have been applied).
fn pin_outward_offset(
    dir: PinDirection, rotation: i32, mirrored: bool, cell_size: f64,
) -> Point {
    let raw = match dir {
        PinDirection::Left  => Point::new(-cell_size, 0.0),
        PinDirection::Right => Point::new( cell_size, 0.0),
        PinDirection::Up    => Point::new(0.0, -cell_size),
        PinDirection::Down  => Point::new(0.0,  cell_size),
    };
    raw.transform(rotation, mirrored)
}

/// Build a grid covering `bounds` with `cell_size`, hard-blocking each
/// component's transformed bounding rect (inflated by one cell), then
/// re-clearing each pin cell plus the cell one step outward so paths can
/// always exit.
pub fn build_grid(
    components: &[Component],
    symbols: &HashMap<String, SymbolDef>,
    bounds: (Point, Point),
    cell_size: f64,
) -> ObstacleGrid {
    let mut grid = ObstacleGrid::new(bounds.0, bounds.1, cell_size);

    for comp in components {
        let Some(sym) = symbols.get(&comp.symbol_name) else { continue; };

        // Transformed bounding rect in world coords
        let base = sym.bounding_rect();
        let corners = [
            Point::new(base.left(), base.top()),
            Point::new(base.right(), base.top()),
            Point::new(base.left(), base.bottom()),
            Point::new(base.right(), base.bottom()),
        ];
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for c in &corners {
            let world = comp.position + c.transform(comp.rotation, comp.mirrored);
            min_x = min_x.min(world.x);
            min_y = min_y.min(world.y);
            max_x = max_x.max(world.x);
            max_y = max_y.max(world.y);
        }
        grid.block_rect(Point::new(min_x, min_y), Point::new(max_x, max_y), 1);

        // Pin tip cell + one cell outward stay reachable.
        for pin in &sym.pins {
            let pin_world = comp.position
                + pin.offset.transform(comp.rotation, comp.mirrored);
            grid.unblock_at(pin_world);
            let outward = pin_outward_offset(
                pin.direction, comp.rotation, comp.mirrored, cell_size,
            );
            grid.unblock_at(pin_world + outward);
        }
    }

    grid
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir { None, Up, Down, Left, Right }

#[derive(Clone, Copy)]
struct Node {
    col: usize,
    row: usize,
    dir: Dir,
    f: f64,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool { self.f == other.f }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap by f
        other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}

fn dir_idx(d: Dir) -> usize {
    match d { Dir::None => 0, Dir::Up => 1, Dir::Down => 2, Dir::Left => 3, Dir::Right => 4 }
}

/// Grid-based A* with a bend penalty. The path returned has its endpoints
/// replaced with the actual `from` / `to` world points (so callers see a
/// pin-to-pin polyline, not a cell-center polyline) and is run through
/// `simplify_path` to merge collinear segments.
///
/// Returns `None` if either endpoint is out of bounds or no path exists.
pub fn find_path(
    grid: &ObstacleGrid, from: Point, to: Point, bend_penalty: f64,
) -> Option<Vec<Point>> {
    let (sc, sr) = grid.world_to_cell(from);
    let (gc, gr) = grid.world_to_cell(to);
    if !grid.in_bounds(sc, sr) || !grid.in_bounds(gc, gr) {
        return None;
    }
    let (sc, sr) = (sc as usize, sr as usize);
    let (gc, gr) = (gc as usize, gr as usize);

    if (sc, sr) == (gc, gr) {
        return Some(vec![from, to]);
    }

    let n = grid.cols * grid.rows;
    let state_count = n * 5;
    let state_idx = |c: usize, r: usize, d: Dir| -> usize {
        (r * grid.cols + c) * 5 + dir_idx(d)
    };

    let mut g_cost = vec![f64::INFINITY; state_count];
    let mut parent: Vec<Option<(u32, u32, u8)>> = vec![None; state_count];
    let mut heap = BinaryHeap::new();

    let h0 = ((sc as i32 - gc as i32).abs() + (sr as i32 - gr as i32).abs()) as f64;
    g_cost[state_idx(sc, sr, Dir::None)] = 0.0;
    heap.push(Node { col: sc, row: sr, dir: Dir::None, f: h0 });

    let neighbors: [(Dir, i32, i32); 4] = [
        (Dir::Up,    0, -1),
        (Dir::Down,  0,  1),
        (Dir::Left, -1,  0),
        (Dir::Right, 1,  0),
    ];

    while let Some(node) = heap.pop() {
        if node.col == gc && node.row == gr {
            // Reconstruct
            let mut path: Vec<Point> = Vec::new();
            let mut cur_c = node.col;
            let mut cur_r = node.row;
            let mut cur_d = node.dir;
            loop {
                path.push(grid.cell_to_world(cur_c as i32, cur_r as i32));
                let key = state_idx(cur_c, cur_r, cur_d);
                match parent[key] {
                    Some((pc, pr, pd)) => {
                        cur_c = pc as usize;
                        cur_r = pr as usize;
                        cur_d = match pd {
                            0 => Dir::None,
                            1 => Dir::Up,
                            2 => Dir::Down,
                            3 => Dir::Left,
                            _ => Dir::Right,
                        };
                    }
                    None => break,
                }
            }
            path.reverse();
            if !path.is_empty() {
                path[0] = from;
                let last = path.len() - 1;
                path[last] = to;
            }
            return Some(simplify_path(&path));
        }

        let cur_g = g_cost[state_idx(node.col, node.row, node.dir)];
        // Stale heap entry — a better g_cost was recorded after this was pushed.
        if cur_g + 1e-9 < node.f - manhattan(node.col, node.row, gc, gr) {
            continue;
        }

        for &(d, dc, dr) in &neighbors {
            let nc = node.col as i32 + dc;
            let nr = node.row as i32 + dr;
            if !grid.in_bounds(nc, nr) { continue; }
            let (nc, nr) = (nc as usize, nr as usize);
            // Allow movement into the goal cell even if blocked (rare but
            // possible — the goal is always a pin cell, but inflation might
            // creep in).
            if grid.is_blocked(nc, nr) && (nc, nr) != (gc, gr) { continue; }

            let bend = if node.dir != Dir::None && node.dir != d { bend_penalty } else { 0.0 };
            let step = 1.0 + bend + grid.cell_cost(nc, nr);
            let new_g = cur_g + step;

            let key = state_idx(nc, nr, d);
            if new_g + 1e-9 < g_cost[key] {
                g_cost[key] = new_g;
                parent[key] = Some((node.col as u32, node.row as u32, dir_idx(node.dir) as u8));
                let h = manhattan(nc, nr, gc, gr);
                heap.push(Node { col: nc, row: nr, dir: d, f: new_g + h });
            }
        }
    }

    None
}

fn manhattan(c: usize, r: usize, gc: usize, gr: usize) -> f64 {
    ((c as i32 - gc as i32).abs() + (r as i32 - gr as i32).abs()) as f64
}

/// Drop interior points that lie on the line connecting their neighbors.
/// Idempotent: simplify_path(simplify_path(p)) == simplify_path(p).
pub fn simplify_path(pts: &[Point]) -> Vec<Point> {
    if pts.len() < 3 { return pts.to_vec(); }
    let mut result: Vec<Point> = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let prev = *result.last().unwrap();
        let cur = pts[i];
        let next = pts[i + 1];
        let dx1 = cur.x - prev.x;
        let dy1 = cur.y - prev.y;
        let dx2 = next.x - cur.x;
        let dy2 = next.y - cur.y;
        let cross = dx1 * dy2 - dy1 * dx2;
        // Same direction (no zigzag, no reversal) AND collinear → drop cur.
        if cross.abs() < 1e-6 && dx1 * dx2 >= 0.0 && dy1 * dy2 >= 0.0 {
            continue;
        }
        result.push(cur);
    }
    result.push(*pts.last().unwrap());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::builtin_symbols;

    fn p(x: f64, y: f64) -> Point { Point::new(x, y) }

    // ---- ObstacleGrid ----

    #[test]
    fn grid_world_cell_round_trip() {
        let g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        let (c, r) = g.world_to_cell(p(50.0, 30.0));
        let back = g.cell_to_world(c, r);
        assert!((back.x - 50.0).abs() < 1e-6);
        assert!((back.y - 30.0).abs() < 1e-6);
    }

    #[test]
    fn grid_block_rect_marks_interior_cells() {
        let mut g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        g.block_rect(p(40.0, 40.0), p(60.0, 60.0), 0);
        let (c, r) = g.world_to_cell(p(50.0, 50.0));
        assert!(g.is_blocked(c as usize, r as usize));
        // Far corner stays clear
        let (c2, r2) = g.world_to_cell(p(0.0, 0.0));
        assert!(!g.is_blocked(c2 as usize, r2 as usize));
    }

    #[test]
    fn grid_unblock_at_clears_single_cell() {
        let mut g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        g.block_rect(p(0.0, 0.0), p(100.0, 100.0), 0);
        g.unblock_at(p(50.0, 50.0));
        let (c, r) = g.world_to_cell(p(50.0, 50.0));
        assert!(!g.is_blocked(c as usize, r as usize));
        // Adjacent cells still blocked
        let (c2, r2) = g.world_to_cell(p(60.0, 50.0));
        assert!(g.is_blocked(c2 as usize, r2 as usize));
    }

    #[test]
    fn grid_add_wire_cost_accumulates() {
        let mut g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        g.add_wire_cost(&[p(10.0, 10.0), p(50.0, 10.0)], 2.0);
        let (c, r) = g.world_to_cell(p(30.0, 10.0));
        assert!(g.cell_cost(c as usize, r as usize) > 0.0);
        // Off the wire, no cost
        let (c2, r2) = g.world_to_cell(p(30.0, 80.0));
        assert_eq!(g.cell_cost(c2 as usize, r2 as usize), 0.0);
    }

    // ---- build_grid ----

    #[test]
    fn build_grid_blocks_component_body_but_keeps_pins_clear() {
        let symbols = builtin_symbols::all();
        let comp = Component {
            instance_name: "M1".into(),
            symbol_name: "nmos4".into(),
            position: p(100.0, 100.0),
            rotation: 0,
            mirrored: false,
            properties: vec![],
        };
        let g = build_grid(&[comp], &symbols, (p(0.0, 0.0), p(200.0, 200.0)), 10.0);

        // NMOS pins (offsets relative to position 100,100):
        //  G at (-30, 0) → world (70, 100)
        //  D at (0, -20) → world (100, 80)
        //  S at (0,  20) → world (100, 120)
        //  B at (10,  0) → world (110, 100)
        for pin_world in [p(70.0, 100.0), p(100.0, 80.0), p(100.0, 120.0), p(110.0, 100.0)] {
            let (c, r) = g.world_to_cell(pin_world);
            assert!(!g.is_blocked(c as usize, r as usize),
                "pin cell {:?} should be reachable", pin_world);
        }
        // Body interior should be blocked
        let (c, r) = g.world_to_cell(p(95.0, 100.0));
        assert!(g.is_blocked(c as usize, r as usize),
            "interior of body should be blocked");
    }

    // ---- find_path ----

    #[test]
    fn find_path_straight_line_no_obstacles() {
        let g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        let path = find_path(&g, p(10.0, 50.0), p(90.0, 50.0), 0.5).unwrap();
        // Should be straight: 2 points, both endpoints
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], p(10.0, 50.0));
        assert_eq!(path[1], p(90.0, 50.0));
    }

    #[test]
    fn find_path_l_shape_one_bend() {
        let g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        let path = find_path(&g, p(10.0, 10.0), p(90.0, 90.0), 0.5).unwrap();
        // Bend penalty discourages zigzags → expect 3 points (one corner).
        assert_eq!(path.len(), 3, "got {:?}", path);
        assert_eq!(path[0], p(10.0, 10.0));
        assert_eq!(path[2], p(90.0, 90.0));
    }

    #[test]
    fn find_path_routes_around_obstacle() {
        let mut g = ObstacleGrid::new(p(0.0, 0.0), p(200.0, 200.0), 10.0);
        // Wall blocking the direct route from (10,100) to (190,100)
        g.block_rect(p(80.0, 50.0), p(120.0, 150.0), 0);
        let path = find_path(&g, p(10.0, 100.0), p(190.0, 100.0), 0.5).unwrap();
        assert!(path.len() >= 3, "must bend around obstacle, got {:?}", path);
        // No path point should land on a blocked cell
        for pt in &path[1..path.len() - 1] {
            let (c, r) = g.world_to_cell(*pt);
            assert!(!g.is_blocked(c as usize, r as usize),
                "path goes through blocked cell at {:?}", pt);
        }
    }

    #[test]
    fn find_path_returns_none_when_fully_walled_off() {
        let mut g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        // Block a vertical wall across the entire grid (and inflate).
        g.block_rect(p(0.0, -100.0), p(100.0, 200.0), 0);
        // The unblock_at on the start cell ensures the start is reachable, but
        // no neighbors are.
        g.unblock_at(p(20.0, 50.0));
        g.unblock_at(p(80.0, 50.0));
        let result = find_path(&g, p(20.0, 50.0), p(80.0, 50.0), 0.5);
        assert!(result.is_none(), "should fail when fully walled off");
    }

    #[test]
    fn find_path_same_endpoint_is_trivial() {
        let g = ObstacleGrid::new(p(0.0, 0.0), p(100.0, 100.0), 10.0);
        let path = find_path(&g, p(50.0, 50.0), p(50.0, 50.0), 0.5).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], path[1]);
    }

    // ---- simplify_path ----

    #[test]
    fn simplify_path_drops_collinear_interior() {
        let path = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(30.0, 0.0)];
        let s = simplify_path(&path);
        assert_eq!(s, vec![p(0.0, 0.0), p(30.0, 0.0)]);
    }

    #[test]
    fn simplify_path_keeps_real_corners() {
        let path = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)];
        let s = simplify_path(&path);
        assert_eq!(s, path);
    }

    #[test]
    fn simplify_path_idempotent() {
        let path = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(20.0, 10.0)];
        let once = simplify_path(&path);
        let twice = simplify_path(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn simplify_path_passes_through_short_input() {
        assert_eq!(simplify_path(&[]), vec![]);
        assert_eq!(simplify_path(&[p(1.0, 2.0)]), vec![p(1.0, 2.0)]);
        let two = vec![p(0.0, 0.0), p(5.0, 5.0)];
        assert_eq!(simplify_path(&two), two);
    }
}
