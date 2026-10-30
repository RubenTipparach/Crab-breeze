//! Routing the highways over the island (openspec change `curved-roads`), the way Mining Mike's road
//! crew surveys its corridor (godot-sandbox `scripts/terrain.gd` and `road_corridor.gd`, revision
//! 950a46d): an A* search over the terrain's cells for the cheapest way between two cities, paying
//! for distance and steepness, with mountains past a road's climb, the sea, lakes and other cities
//! out of bounds and streams costing a bridge; then the cell path pulled taut between sight lines,
//! rounded into curves and given a
//! grade that follows the land within the steepest a road may climb.

use super::config::WorldConfig;
use super::hydrology::Drainage;
use super::layout::{City, Highway, level_end_m};
use super::terrain::{Shape, smoothstep};
use glam::Vec2;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A search front entry: lowest estimated cost first, ties to the lower cell (a stable order).
#[derive(PartialEq)]
struct Front(f32, u32);

impl Eq for Front {}

impl Ord for Front {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

impl PartialOrd for Front {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The eight neighbours, by grid step.
const NEIGHBOURS: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

/// Longest straight the taut route keeps before the curves round it off.
const TAUT_MAX_M: f32 = 1500.0;

/// Spacing of the finished centre line's points.
const STEP_M: f32 = 16.0;

/// Lays a highway from city `a`'s edge point `from` to city `b`'s `to`, routed over `drainage`'s
/// cells, or `None` when no route stays within the road's climb and clear of the water and the
/// other cities.
pub fn lay_highway(
    shape: &Shape,
    drainage: &Drainage,
    cities: &[City],
    (a, b): (usize, usize),
    (from, to): (Vec2, Vec2),
    cfg: &WorldConfig,
) -> Option<Highway> {
    let end = |k: usize, at: Vec2| RoadEnd {
        city: Some(k),
        id: cities[k].id,
        rect: cities[k].rect(),
        ground_m: cities[k].ground_m,
        at,
    };
    lay_road(shape, drainage, cities, [end(a, from), end(b, to)], cfg)
}

/// The id a road's second end carries when it ends at a site's gate, not at a city (`Highway::cities`).
pub const SITE_END: u32 = u32::MAX;

/// A site's access road (openspec change `island-sites-layout`, design section 3): from the edge of the site's city
/// nearest its gate to the gate (`gate`, on the site's footprint `site`, its ground `ground_m`), routed and graded as a
/// highway is. Its second city is `SITE_END`: the lane graph ends it at a gate of its own.
pub fn lay_access(
    shape: &Shape,
    drainage: &Drainage,
    cities: &[City],
    city: usize,
    (site, gate, ground_m): (super::layout::Rect, Vec2, f32),
    cfg: &WorldConfig,
) -> Option<Highway> {
    let r = cities[city].rect();
    let from = gate.clamp(r.min, r.max);
    let ends = [
        RoadEnd {
            city: Some(city),
            id: cities[city].id,
            rect: r,
            ground_m: cities[city].ground_m,
            at: from,
        },
        RoadEnd {
            city: None,
            id: SITE_END,
            rect: site,
            ground_m,
            at: gate,
        },
    ];
    lay_road(shape, drainage, cities, ends, cfg)
}

/// One end of a road: the city it starts in (none for a site), the id it carries, the flat ground it leaves (the
/// city's square or the site's footprint) at that ground's height, and the point it starts from.
struct RoadEnd {
    city: Option<usize>,
    id: u32,
    rect: super::layout::Rect,
    ground_m: f32,
    at: Vec2,
}

fn lay_road(shape: &Shape, drainage: &Drainage, cities: &[City], ends: [RoadEnd; 2], cfg: &WorldConfig) -> Option<Highway> {
    let (from, to) = (ends[0].at, ends[1].at);
    let (a, b) = (ends[0].city.unwrap_or(usize::MAX), ends[1].city.unwrap_or(usize::MAX));
    let ends_ground = [ends[0].ground_m, ends[1].ground_m];
    let top = ends_ground[0].max(ends_ground[1]) + cfg.highway_max_climb_m;
    let n = drainage.n;
    let cell = drainage.cell_m;
    // The search stays within a box round the two ends.
    let lo = from.min(to) - Vec2::splat(cfg.highway_detour_m);
    let hi = from.max(to) + Vec2::splat(cfg.highway_detour_m);
    // Clear of a third city's blend and the road's own graded band, with half a cell to spare for
    // the curves the route is rounded into.
    let reach = cfg.plateau_flat_margin_m
        + cfg.plateau_blend_m
        + cfg.highway_width_m * 0.5
        + cfg.highway_flat_margin_m
        + cfg.highway_blend_m
        + cell * 0.5;
    let other_city = |p: Vec2| {
        cities
            .iter()
            .enumerate()
            .any(|(k, c)| k != a && k != b && c.rect().distance(p) < reach)
    };
    let passable = |k: usize| {
        let p = drainage.centre(k);
        let g = drainage.ground[k];
        p.cmpge(lo).all()
            && p.cmple(hi).all()
            && g >= cfg.island.sea_level_m + cfg.water.city_freeboard_m
            && g <= top
            && !other_city(p)
            && drainage
                .basins
                .iter()
                .all(|l| l.centre.distance(p) > l.radius_m + cfg.highway_width_m + 60.0)
    };
    let (start, goal) = (drainage.cell_of(from), drainage.cell_of(to));
    let goal_p = drainage.centre(goal);
    let mut cost = vec![f32::INFINITY; n * n];
    let mut came = vec![u32::MAX; n * n];
    let mut front = BinaryHeap::new();
    cost[start] = 0.0;
    front.push(Front(drainage.centre(start).distance(goal_p), start as u32));
    while let Some(Front(_, k)) = front.pop() {
        let k = k as usize;
        if k == goal {
            break;
        }
        let (ix, iz) = ((k % n) as i32, (k / n) as i32);
        for (dx, dz) in NEIGHBOURS {
            let (jx, jz) = (ix + dx, iz + dz);
            if jx < 0 || jz < 0 || jx >= n as i32 || jz >= n as i32 {
                continue;
            }
            let j = jz as usize * n + jx as usize;
            // The ends' own cells are always passable (they stand in or by a city).
            if j != goal && !passable(j) {
                continue;
            }
            let run = cell * ((dx * dx + dz * dz) as f32).sqrt();
            let grade = (drainage.ground[j] - drainage.ground[k]).abs() / run;
            let steep = grade / cfg.highway_max_grade;
            let mut step = run * (1.0 + cfg.highway_slope_cost * steep * steep);
            if drainage.stream[j] {
                step += cfg.highway_bridge_cost_m;
            }
            let c = cost[k] + step;
            if c < cost[j] {
                cost[j] = c;
                came[j] = k as u32;
                front.push(Front(c + drainage.centre(j).distance(goal_p), j as u32));
            }
        }
    }
    if !cost[goal].is_finite() {
        return None;
    }
    let mut cells = vec![goal];
    while let Some(&k) = cells.last() {
        if k == start {
            break;
        }
        cells.push(came[k] as usize);
    }
    cells.reverse();
    // The cell path, from the first edge point to the second, leaving out the cells inside the
    // two end cities (the road starts at their edges).
    let (ra, rb) = (ends[0].rect, ends[1].rect);
    let mut points = vec![from];
    points.extend(
        cells
            .iter()
            .map(|&k| drainage.centre(k))
            .filter(|p| ra.distance(*p) > cell * 0.5 && rb.distance(*p) > cell * 0.5),
    );
    points.push(to);
    // Pulled taut: from each kept point, on to the farthest later point in plain sight (every cell
    // on the straight line between passable, at most `TAUT_MAX_M` away), so the road sweeps instead
    // of stepping along the grid.
    let clear = |p: Vec2, q: Vec2| {
        let steps = (p.distance(q) / (cell * 0.5)).ceil().max(1.0) as usize;
        (1..steps).all(|i| {
            let k = drainage.cell_of(p.lerp(q, i as f32 / steps as f32));
            k == start || k == goal || passable(k)
        })
    };
    let mut taut = vec![points[0]];
    let mut i = 0;
    while i + 1 < points.len() {
        let mut j = i + 1;
        while j + 1 < points.len() && points[i].distance(points[j + 1]) <= TAUT_MAX_M && clear(points[i], points[j + 1]) {
            j += 1;
        }
        taut.push(points[j]);
        i = j;
    }
    let mut line = taut;
    for _ in 0..4 {
        line = chaikin(&line);
    }
    let path = resample(&line, STEP_M);
    if path.len() < 2 {
        return None;
    }
    // Each end stays level at its city's ground until the road is clear of the city's blend band
    // (at least `level_end_m`), however it leaves the square.
    let clear_of = |r: &super::layout::Rect, order: &mut dyn Iterator<Item = usize>| {
        let mut along = 0.0;
        let mut last = None;
        for i in order {
            if let Some(l) = last {
                along += path[i].distance(path[l]);
            }
            last = Some(i);
            if r.distance(path[i]) > reach {
                break;
            }
        }
        along.max(level_end_m(cfg))
    };
    let n_path = path.len();
    let level = [clear_of(&ra, &mut (0..n_path)), clear_of(&rb, &mut (0..n_path).rev())];
    let grade = grade_along(&path, shape, ends_ground, level, cfg);
    Some(Highway::new(
        [ends[0].id, ends[1].id],
        path,
        grade,
        cfg.highway_width_m,
        level[0].max(level[1]),
        cfg.world_size_m,
    ))
}

/// The road's height along `path`: the land under it averaged over `highway_grade_smooth_m`
/// either side, level at each city's ground for `level_end` (at each end) and easing into the land over the next
/// `highway_grade_smooth_m`, then held within `highway_max_grade` of its neighbours both ways.
fn grade_along(path: &[Vec2], shape: &Shape, ends: [f32; 2], level_end: [f32; 2], cfg: &WorldConfig) -> Vec<f32> {
    let n = path.len();
    let land: Vec<f32> = path.iter().map(|p| shape.height(*p)).collect();
    let mut along = vec![0.0f32];
    for w in path.windows(2) {
        along.push(along.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
    }
    let total = along[n - 1];
    let half = (cfg.highway_grade_smooth_m / STEP_M).round().max(1.0) as usize;
    let mut g: Vec<f32> = (0..n)
        .map(|i| {
            let (s, e) = (i.saturating_sub(half), (i + half).min(n - 1));
            land[s..=e].iter().sum::<f32>() / (e - s + 1) as f32
        })
        .collect();
    for i in 0..n {
        let from_a = along[i];
        let from_b = total - along[i];
        let wa = 1.0 - smoothstep(level_end[0], level_end[0] + cfg.highway_grade_smooth_m, from_a);
        let wb = 1.0 - smoothstep(level_end[1], level_end[1] + cfg.highway_grade_smooth_m, from_b);
        g[i] += (ends[0] - g[i]) * wa;
        g[i] += (ends[1] - g[i]) * wb;
    }
    // The level stretches at the ends are pinned; between them the grade is held within
    // `highway_max_grade` of its neighbours both ways.
    let pinned: Vec<bool> = (0..n)
        .map(|i| along[i] <= level_end[0] || total - along[i] <= level_end[1])
        .collect();
    for i in 0..n {
        if along[i] <= level_end[0] {
            g[i] = ends[0];
        } else if total - along[i] <= level_end[1] {
            g[i] = ends[1];
        }
    }
    for _ in 0..4 {
        for i in 1..n {
            if !pinned[i] {
                let d = cfg.highway_max_grade * (along[i] - along[i - 1]);
                g[i] = g[i].clamp(g[i - 1] - d, g[i - 1] + d);
            }
        }
        for i in (0..n - 1).rev() {
            if !pinned[i] {
                let d = cfg.highway_max_grade * (along[i + 1] - along[i]);
                g[i] = g[i].clamp(g[i + 1] - d, g[i + 1] + d);
            }
        }
    }
    g
}

/// Chaikin corner cutting: a smoother line through the same region, keeping both ends.
fn chaikin(path: &[Vec2]) -> Vec<Vec2> {
    if path.len() < 3 {
        return path.to_vec();
    }
    let mut out = vec![path[0]];
    for w in path.windows(2) {
        out.push(w[0].lerp(w[1], 0.25));
        out.push(w[0].lerp(w[1], 0.75));
    }
    out.push(path[path.len() - 1]);
    out
}

/// Points every `step_m` along `path`, both ends kept.
fn resample(path: &[Vec2], step_m: f32) -> Vec<Vec2> {
    let mut out = vec![path[0]];
    let mut carry = 0.0;
    for w in path.windows(2) {
        let len = w[0].distance(w[1]);
        let mut at = step_m - carry;
        while at < len {
            out.push(w[0].lerp(w[1], at / len));
            at += step_m;
        }
        carry = (carry + len) % step_m;
    }
    let last = path[path.len() - 1];
    if out.last().is_some_and(|p| p.distance(last) > step_m * 0.3) {
        out.push(last);
    } else if let Some(e) = out.last_mut() {
        *e = last;
    }
    out
}
