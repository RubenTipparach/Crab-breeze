//! Sea lanes (openspec change `harbours`, design section 2): A* paths over a grid of open-water cells, kept clear
//! of the coast and every islet, pulled straight, from each harbour's mouth to every other harbour's and out to the
//! offing past the horizon; and the routes every boat follows, as polylines measured along their length. Pure.

use super::{Harbour, LanesConfig, Sea};
use glam::Vec2;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// What lies at an end of a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// A harbour's berth (its id): a boat slows for it and moors.
    Berth(u32),
    /// The offing, past the horizon: a boat that reaches it is gone.
    Offing,
    /// A fishing ground: a boat works it, then turns for home.
    Ground,
    /// An islet's landing: the ferry moors there.
    Landing,
}

/// A path a boat follows: points (x, z) and the distance along it to each.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    /// The points, at least two.
    pub points: Vec<Vec2>,
    /// The distance along the route to each point, metres; the first is zero.
    pub along_m: Vec<f32>,
    /// What lies at its first and last points.
    pub ends: [End; 2],
}

impl Route {
    /// A route through `points` (at least two) with what lies at each end.
    pub fn new(points: Vec<Vec2>, ends: [End; 2]) -> Self {
        let mut along_m = Vec::with_capacity(points.len());
        let mut s = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                s += p.distance(points[i - 1]);
            }
            along_m.push(s);
        }
        Self { points, along_m, ends }
    }

    /// Its length, metres.
    pub fn length_m(&self) -> f32 {
        self.along_m.last().copied().unwrap_or(0.0)
    }

    /// The point `s_m` along it (clamped to its ends), and the unit direction it runs there.
    pub fn at(&self, s_m: f32) -> (Vec2, Vec2) {
        let n = self.points.len();
        if n < 2 {
            return (self.points.first().copied().unwrap_or(Vec2::ZERO), Vec2::X);
        }
        let s = s_m.clamp(0.0, self.length_m());
        let i = match self.along_m.binary_search_by(|a| a.total_cmp(&s)) {
            Ok(i) => i.min(n - 2),
            Err(i) => i.saturating_sub(1).min(n - 2),
        };
        let (a, b) = (self.points[i], self.points[i + 1]);
        let seg = (self.along_m[i + 1] - self.along_m[i]).max(1e-6);
        let dir = (b - a).normalize_or(Vec2::X);
        (a + (b - a) * ((s - self.along_m[i]) / seg).clamp(0.0, 1.0), dir)
    }

    /// The distance along the route of its point nearest `p`, looking only within `window_m` of `near_m` (the
    /// whole route when `window_m` is infinite), and how far `p` is from it.
    pub fn project(&self, p: Vec2, near_m: f32, window_m: f32) -> (f32, f32) {
        let mut best = (near_m.clamp(0.0, self.length_m()), f32::INFINITY);
        for i in 0..self.points.len().saturating_sub(1) {
            let (s0, s1) = (self.along_m[i], self.along_m[i + 1]);
            if s1 < near_m - window_m || s0 > near_m + window_m {
                continue;
            }
            let (a, b) = (self.points[i], self.points[i + 1]);
            let ab = b - a;
            let u = if ab.length_squared() > 0.0 {
                ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (a + ab * u).distance(p);
            if d < best.1 {
                best = (s0 + (s1 - s0) * u, d);
            }
        }
        best
    }

    /// The same route run the other way.
    pub fn reversed(&self) -> Self {
        let mut points = self.points.clone();
        points.reverse();
        Self::new(points, [self.ends[1], self.ends[0]])
    }
}

/// Open water over the world on a grid: a cell is open when the sea is lane-deep under it and over every point
/// `coast_clear_m` round it.
#[derive(Clone, Debug, PartialEq)]
pub struct SeaGrid {
    /// Cells a side.
    pub n: usize,
    /// A cell's side, metres.
    pub cell_m: f32,
    /// World x and z of cell (0, 0)'s least corner.
    pub origin_m: f32,
    open: Vec<bool>,
}

impl SeaGrid {
    /// The grid over a world `world_size_m` across, centred on the origin: each cell's centre is sampled once,
    /// then open water is shrunk by the coast clearance (a cell is open only when every cell within
    /// `coast_clear_m` of it is lane-deep).
    pub fn new(sea: &Sea, world_size_m: f32, cfg: &LanesConfig) -> Self {
        let n = (world_size_m / cfg.cell_m).ceil() as usize;
        let origin_m = -(n as f32) * cfg.cell_m * 0.5;
        let mut deep = vec![false; n * n];
        for iz in 0..n {
            for ix in 0..n {
                let c = Vec2::new(origin_m + (ix as f32 + 0.5) * cfg.cell_m, origin_m + (iz as f32 + 0.5) * cfg.cell_m);
                deep[iz * n + ix] = sea.sea_depth(c) >= cfg.lane_depth_m;
            }
        }
        let r = (cfg.coast_clear_m / cfg.cell_m).ceil() as i64;
        let mut open = vec![false; n * n];
        for iz in 0..n as i64 {
            for ix in 0..n as i64 {
                let mut ok = deep[iz as usize * n + ix as usize];
                'round: for dz in -r..=r {
                    for dx in -r..=r {
                        if !ok {
                            break 'round;
                        }
                        if (dx * dx + dz * dz) as f32 * cfg.cell_m * cfg.cell_m > (cfg.coast_clear_m + cfg.cell_m * 0.5).powi(2) {
                            continue;
                        }
                        let (x, z) = (ix + dx, iz + dz);
                        // Past the world's edge is open sea.
                        if x >= 0 && z >= 0 && (x as usize) < n && (z as usize) < n && !deep[z as usize * n + x as usize] {
                            ok = false;
                        }
                    }
                }
                open[iz as usize * n + ix as usize] = ok;
            }
        }
        Self {
            n,
            cell_m: cfg.cell_m,
            origin_m,
            open,
        }
    }

    /// The cell holding `p`, if it is on the grid.
    pub fn cell(&self, p: Vec2) -> Option<(usize, usize)> {
        let f = (p - Vec2::splat(self.origin_m)) / self.cell_m;
        (f.x >= 0.0 && f.y >= 0.0 && (f.x as usize) < self.n && (f.y as usize) < self.n).then_some((f.x as usize, f.y as usize))
    }

    /// A cell's centre.
    pub fn centre(&self, c: (usize, usize)) -> Vec2 {
        Vec2::new(
            self.origin_m + (c.0 as f32 + 0.5) * self.cell_m,
            self.origin_m + (c.1 as f32 + 0.5) * self.cell_m,
        )
    }

    /// True when the cell is open water.
    pub fn is_open(&self, c: (usize, usize)) -> bool {
        self.open[c.1 * self.n + c.0]
    }

    /// True when `p` is in an open cell.
    pub fn open_at(&self, p: Vec2) -> bool {
        self.cell(p).is_some_and(|c| self.is_open(c))
    }

    /// The open cell nearest `p` within `within_m`, by rings outward, ties to the lower index.
    pub fn nearest_open(&self, p: Vec2, within_m: f32) -> Option<(usize, usize)> {
        let (cx, cz) = self.cell(p)?;
        let rings = (within_m / self.cell_m).ceil() as i64;
        for r in 0..=rings {
            let mut best: Option<(f32, (usize, usize))> = None;
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let (x, z) = (cx as i64 + dx, cz as i64 + dz);
                    if x < 0 || z < 0 || x as usize >= self.n || z as usize >= self.n {
                        continue;
                    }
                    let c = (x as usize, z as usize);
                    if self.is_open(c) {
                        let d = self.centre(c).distance(p);
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, c));
                        }
                    }
                }
            }
            if let Some(b) = best {
                return Some(b.1);
            }
        }
        None
    }

    /// True when the straight line from `a` to `b` stays on open water (sampled every half cell).
    pub fn clear_line(&self, a: Vec2, b: Vec2) -> bool {
        let steps = (a.distance(b) / (self.cell_m * 0.5)).ceil().max(1.0) as usize;
        (0..=steps).all(|i| self.open_at(a.lerp(b, i as f32 / steps as f32)))
    }

    /// The A* path over open cells from `from` to `to` (each snapped to its nearest open cell within 2 km),
    /// pulled straight, as world points; `None` when they are not joined by open water.
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        let start = self.nearest_open(from, 2000.0)?;
        let goal = self.nearest_open(to, 2000.0)?;
        let cells = self.astar(start, goal)?;
        let raw: Vec<Vec2> = cells.into_iter().map(|c| self.centre(c)).collect();
        Some(self.pull(&raw))
    }

    fn astar(&self, start: (usize, usize), goal: (usize, usize)) -> Option<Vec<(usize, usize)>> {
        let n = self.n;
        let idx = |c: (usize, usize)| c.1 * n + c.0;
        let h = |c: (usize, usize)| {
            let (dx, dz) = ((c.0 as f32 - goal.0 as f32).abs(), (c.1 as f32 - goal.1 as f32).abs());
            dx.max(dz) + (std::f32::consts::SQRT_2 - 1.0) * dx.min(dz)
        };
        let mut cost = vec![f32::INFINITY; n * n];
        let mut from = vec![u32::MAX; n * n];
        let mut heap = BinaryHeap::new();
        cost[idx(start)] = 0.0;
        heap.push(Node {
            f: h(start),
            g: 0.0,
            cell: idx(start) as u32,
        });
        while let Some(Node { g, cell, .. }) = heap.pop() {
            let cell = cell as usize;
            if g > cost[cell] {
                continue;
            }
            let c = (cell % n, cell / n);
            if c == goal {
                let mut out = vec![c];
                let mut at = cell;
                while from[at] != u32::MAX {
                    at = from[at] as usize;
                    out.push((at % n, at / n));
                }
                out.reverse();
                return Some(out);
            }
            for (dx, dz) in [(-1i64, -1i64), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (x, z) = (c.0 as i64 + dx, c.1 as i64 + dz);
                if x < 0 || z < 0 || x as usize >= n || z as usize >= n {
                    continue;
                }
                let nc = (x as usize, z as usize);
                if !self.is_open(nc) {
                    continue;
                }
                // No corner cutting past a closed cell.
                if dx != 0 && dz != 0 && (!self.is_open((x as usize, c.1)) || !self.is_open((c.0, z as usize))) {
                    continue;
                }
                let step = if dx != 0 && dz != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                let ng = g + step;
                let ni = idx(nc);
                if ng < cost[ni] {
                    cost[ni] = ng;
                    from[ni] = cell as u32;
                    heap.push(Node {
                        f: ng + h(nc),
                        g: ng,
                        cell: ni as u32,
                    });
                }
            }
        }
        None
    }

    /// String pulling: from each kept point, on to the farthest later point in clear line of it.
    fn pull(&self, raw: &[Vec2]) -> Vec<Vec2> {
        let Some(&first) = raw.first() else { return Vec::new() };
        let mut out = vec![first];
        let mut i = 0;
        while i + 1 < raw.len() {
            let mut j = raw.len() - 1;
            while j > i + 1 && !self.clear_line(raw[i], raw[j]) {
                j -= 1;
            }
            out.push(raw[j]);
            i = j;
        }
        out
    }
}

/// An A* node, ordered so the heap pops the least `f` first, ties to the lower cell.
#[derive(Clone, Copy, Debug)]
struct Node {
    f: f32,
    g: f32,
    cell: u32,
}

impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f).then_with(|| o.cell.cmp(&self.cell))
    }
}

/// The sea lanes: harbour to harbour, and each harbour out to its offing. Each runs from the first harbour's berth
/// out through its mouth, over open water, and in through the other's mouth to its berth (or out to the offing).
pub fn sea_lanes(harbours: &[Harbour], grid: &SeaGrid, cfg: &LanesConfig) -> Vec<Route> {
    let mut out = Vec::new();
    for (i, a) in harbours.iter().enumerate() {
        for b in &harbours[i + 1..] {
            if let Some(mid) = grid.path(a.mouth, b.mouth) {
                let pts = std::iter::once(a.berth)
                    .chain(std::iter::once(a.mouth))
                    .chain(mid)
                    .chain([b.mouth, b.berth])
                    .collect();
                out.push(Route::new(dedupe(pts), [End::Berth(a.id), End::Berth(b.id)]));
            }
        }
    }
    // The offing stays a few cells inside the grid, whatever the world's size.
    let half = grid.n as f32 * grid.cell_m * 0.5 - grid.cell_m * 2.0;
    for a in harbours {
        let dir = a.mouth.normalize_or(a.out);
        let offing = dir * cfg.offing_m.min(half);
        if let Some(mid) = grid.path(a.mouth, offing) {
            let mut pts: Vec<Vec2> = std::iter::once(a.berth).chain(std::iter::once(a.mouth)).chain(mid).collect();
            pts.push(offing);
            out.push(Route::new(dedupe(pts), [End::Berth(a.id), End::Offing]));
        }
    }
    out
}

/// Drops points within 1 m of the one before.
fn dedupe(pts: Vec<Vec2>) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_none_or(|q| q.distance(p) > 1.0) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::test_world;
    use super::*;

    fn grid() -> SeaGrid {
        let cfg = test_world::shipped().lanes;
        SeaGrid::new(&test_world::sea(), 16000.0, &cfg)
    }

    #[test]
    fn open_water_keeps_clear_of_the_coast() {
        let cfg = test_world::shipped().lanes;
        let g = grid();
        let sea = test_world::sea();
        for iz in 0..g.n {
            for ix in 0..g.n {
                if g.is_open((ix, iz)) {
                    let c = g.centre((ix, iz));
                    assert!(sea.sea_depth(c) >= cfg.lane_depth_m, "{c:?}");
                    // The island's lane-deep line is at 3 520 m: nothing open inside 3 520 + clearance - half a cell.
                    assert!(c.length() >= 3520.0 + cfg.coast_clear_m - g.cell_m, "{c:?} too near the coast");
                }
            }
        }
    }

    #[test]
    fn a_path_round_the_island_stays_on_open_water() {
        let g = grid();
        let p = g.path(Vec2::new(5000.0, 0.0), Vec2::new(-5000.0, 0.0)).expect("joined");
        assert!(p.len() >= 3, "it must go round, not through: {p:?}");
        for w in p.windows(2) {
            assert!(g.clear_line(w[0], w[1]), "{w:?}");
        }
        assert!(p.iter().all(|q| q.length() > 3500.0));
    }

    #[test]
    fn string_pulling_leaves_a_straight_run_straight() {
        let g = grid();
        let p = g.path(Vec2::new(5000.0, -6000.0), Vec2::new(5000.0, 6000.0)).unwrap();
        assert_eq!(p.len(), 2, "one leg: {p:?}");
    }

    #[test]
    fn a_route_is_measured_along_its_length() {
        let r = Route::new(
            vec![Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::new(100.0, 50.0)],
            [End::Offing, End::Ground],
        );
        assert_eq!(r.length_m(), 150.0);
        let (p, d) = r.at(125.0);
        assert!(p.distance(Vec2::new(100.0, 25.0)) < 1e-4 && d.distance(Vec2::Y) < 1e-6);
        let (s, off) = r.project(Vec2::new(40.0, 10.0), 0.0, f32::INFINITY);
        assert!((s - 40.0).abs() < 1e-4 && (off - 10.0).abs() < 1e-4);
        let back = r.reversed();
        assert_eq!(back.ends, [End::Ground, End::Offing]);
        assert_eq!(back.at(0.0).0, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn seed_42_lanes_run_on_open_water_from_berth_to_berth_and_out() {
        let (plan, hs) = super::super::harbour::tests::seed_42();
        let cfg = test_world::shipped();
        let sea = Sea {
            ground: plan,
            sea_level_m: plan.waters.sea_level_m,
        };
        let g = SeaGrid::new(&sea, 40960.0, &cfg.lanes);
        let lanes = sea_lanes(hs, &g, &cfg.lanes);
        assert!(lanes.iter().any(|l| l.ends[1] == End::Offing), "a lane out to sea");
        for l in &lanes {
            // Between the mouths every leg stays on open water.
            let inner = &l.points[2..l.points.len() - if matches!(l.ends[1], End::Berth(_)) { 2 } else { 1 }];
            for w in inner.windows(2) {
                assert!(g.clear_line(w[0], w[1]), "{:?} {w:?}", l.ends);
            }
            assert!(l.length_m() > 0.0);
        }
    }
}
