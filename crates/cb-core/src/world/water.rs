//! The sea, lakes, rivers and bridges (openspec changes `terrain-water`, `island-world`): the sea
//! round the island at its sea level (which also fills inland basins as lakes), rivers that run
//! downhill from high ground inland, round the cities, to the sea, carved into the ground, and
//! bridges where they pass under highways. `water_at` is the one water query.

use super::config::WaterConfig;
use super::grid::CellIndex;
use super::layout::{City, Highway, Rect};
use super::terrain::Heightfield;
use crate::rng::SimRng;
use glam::Vec2;

/// Water at a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSample {
    /// Height of the water surface.
    pub surface_m: f32,
    /// Depth from the surface to the ground.
    pub depth_m: f32,
    /// Flow velocity (x, z), metres a second.
    pub flow_m_s: Vec2,
}

/// A river: a centre line with a falling surface and a growing width.
#[derive(Clone, Debug, PartialEq)]
pub struct River {
    /// Centre line (x, z), source to mouth.
    pub path: Vec<Vec2>,
    /// Water surface height at each point; never rises downstream.
    pub surface_m: Vec<f32>,
    /// Half width of the water surface at each point.
    pub half_width_m: Vec<f32>,
}

/// A bridge deck carrying a highway over a river.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bridge {
    /// Centre of the span (x, z).
    pub centre: Vec2,
    /// Direction of the road.
    pub along: Vec2,
    /// Length of the span.
    pub length_m: f32,
    /// Width of the deck.
    pub width_m: f32,
    /// Deck surface height.
    pub deck_m: f32,
}

/// All the water in a world: the sea (and the lakes at its level), rivers and bridges.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waters {
    /// Sea level: ground below it is under the sea, or under a lake at the sea's level.
    pub sea_level_m: f32,
    /// Rivers.
    pub rivers: Vec<River>,
    /// Bridges.
    pub bridges: Vec<Bridge>,
    /// Flow speed of rivers.
    pub flow_m_s: f32,
    /// River segments by cell, as `river << 16 | segment`.
    index: CellIndex,
}

/// River segments are indexed in cells this size.
const BUCKET_M: f32 = 64.0;

/// Where `p` projects on segment `a`-`b`: distance and position 0..1.
fn project(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a + ab * t).distance(p), t)
}

/// The river segment closest to `p` within reach: (river, segment, distance, t). Ties go to the
/// lower (river, segment), so the answer never depends on the index's order.
fn nearest(rivers: &[River], index: &CellIndex, p: Vec2, reach_m: f32) -> Option<(usize, usize, f32, f32)> {
    let mut best: Option<(usize, usize, f32, f32)> = None;
    for &key in index.at(p) {
        let (ri, si) = ((key >> 16) as usize, (key & 0xFFFF) as usize);
        let r = &rivers[ri];
        let (d, t) = project(p, r.path[si], r.path[si + 1]);
        if d <= reach_m && best.is_none_or(|b| d < b.2 || (d == b.2 && (ri, si) < (b.0, b.1))) {
            best = Some((ri, si, d, t));
        }
    }
    best
}

/// Indexes every river segment in the cells within its half width plus `pad_m`.
fn index_rivers(rivers: &[River], world_size_m: f32, pad_m: f32) -> CellIndex {
    let mut boxes = Vec::new();
    for (ri, r) in rivers.iter().enumerate() {
        for si in 0..r.path.len().saturating_sub(1) {
            let (a, b) = (r.path[si], r.path[si + 1]);
            let pad = r.half_width_m[si].max(r.half_width_m[si + 1]) + pad_m;
            boxes.push((
                ((ri as u32) << 16) | si as u32,
                a.min(b) - Vec2::splat(pad),
                a.max(b) + Vec2::splat(pad),
            ));
        }
    }
    let side = (world_size_m / BUCKET_M).ceil() as usize + 1;
    CellIndex::build(-world_size_m * 0.5, BUCKET_M, side, &boxes)
}

impl Waters {
    /// River surface, half width and flow at `p`, if a river's water reaches it.
    fn river_at(&self, p: Vec2) -> Option<(f32, f32, Vec2)> {
        let (ri, si, d, t) = nearest(&self.rivers, &self.index, p, 200.0)?;
        let r = &self.rivers[ri];
        let lerp = |v: &[f32]| v[si] + (v[si + 1] - v[si]) * t;
        let hw = lerp(&r.half_width_m);
        (d <= hw).then(|| {
            (
                lerp(&r.surface_m),
                hw,
                (r.path[si + 1] - r.path[si]).normalize_or_zero() * self.flow_m_s,
            )
        })
    }

    /// The one water query: water over the ground `ground_m` at `p`, if any. Ground below sea level
    /// is under the sea (or a lake at its level), anywhere, past the world's edge too.
    pub fn water_at(&self, p: Vec2, ground_m: f32) -> Option<WaterSample> {
        let river = self.river_at(p).map(|(s, _, f)| (s, f));
        let sea = (ground_m < self.sea_level_m).then_some((self.sea_level_m, Vec2::ZERO));
        let (surface_m, flow_m_s) = match (river, sea) {
            (Some(r), Some(l)) => {
                if r.0 >= l.0 {
                    r
                } else {
                    l
                }
            }
            (Some(r), None) => r,
            (None, Some(l)) => l,
            (None, None) => return None,
        };
        let depth_m = surface_m - ground_m;
        (depth_m > 0.0).then_some(WaterSample {
            surface_m,
            depth_m,
            flow_m_s,
        })
    }

    /// The channels the rivers cut, for the terrain to carve. Cities' flat ground (with
    /// `city_margin_m` round it) is never cut, nor a highway's flat band (`road_flat_m` either side
    /// of its centre line) except under a bridge.
    pub fn channels(&self, cfg: &WaterConfig, cities: &[City], city_margin_m: f32, highways: &[Highway], road_flat_m: f32) -> Channels {
        Channels {
            rivers: self.rivers.clone(),
            index: self.index.clone(),
            depth_m: cfg.river_depth_m,
            bank_slope: cfg.bank_slope,
            cities: cities.iter().map(|c| c.rect()).collect(),
            city_margin_m,
            highways: highways.to_vec(),
            road_flat_m,
            // Cut only well inside the deck, so every terrain triangle under the road beyond it
            // (a cell's diagonal, 11.3 m at 8 m cells) keeps the road's grade: the abutments.
            spans: self.bridges.iter().map(|b| (b.centre, b.length_m * 0.5 - 12.0)).collect(),
        }
    }
}

/// River channels cut into the ground: a U channel under the water and banks rising at
/// `bank_slope` beside it, never raising the ground, never inside a city's flat ground and never
/// under a highway except where a bridge spans the river.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Channels {
    rivers: Vec<River>,
    index: CellIndex,
    depth_m: f32,
    bank_slope: f32,
    cities: Vec<Rect>,
    city_margin_m: f32,
    highways: Vec<Highway>,
    road_flat_m: f32,
    /// Bridge spans: centre and half length.
    spans: Vec<(Vec2, f32)>,
}

impl Channels {
    /// The ground `h` at `p` with the channels carved into it.
    pub fn carve(&self, p: Vec2, h: f32) -> f32 {
        let Some((ri, si, d, t)) = nearest(&self.rivers, &self.index, p, 400.0) else {
            return h;
        };
        if self.cities.iter().any(|c| c.distance(p) <= self.city_margin_m) {
            return h;
        }
        let on_road = self.highways.iter().any(|hw| hw.project(p).0 <= self.road_flat_m);
        if on_road && !self.spans.iter().any(|(c, half)| c.distance(p) <= *half) {
            return h;
        }
        let r = &self.rivers[ri];
        let lerp = |v: &[f32]| v[si] + (v[si + 1] - v[si]) * t;
        let (s, hw) = (lerp(&r.surface_m), lerp(&r.half_width_m));
        let target = if d < hw {
            s - self.depth_m * (1.0 - (d / hw).powi(2))
        } else {
            s + (d - hw) * self.bank_slope
        };
        h.min(target)
    }
}

/// Chaikin corner cutting: a smoother line through the same region.
fn smooth(path: &[Vec2]) -> Vec<Vec2> {
    if path.len() < 3 {
        return path.to_vec();
    }
    let mut out = vec![path[0]];
    for w in path.windows(2) {
        out.push(w[0].lerp(w[1], 0.25));
        out.push(w[0].lerp(w[1], 0.75));
    }
    out.push(*path.last().unwrap_or(&path[0]));
    out
}

/// Where rivers may rise and how far they may run, for `make_waters`.
pub struct RiverBounds {
    /// Side of the square world.
    pub world_size_m: f32,
    /// The sea's level: a river ends when it reaches it, and never runs below it.
    pub sea_level_m: f32,
    /// Sources are drawn within this of the island's centre.
    pub source_within_m: f32,
}

/// Runs the rivers over `terrain` (not yet carved) and finds the bridges. A river starts at the
/// highest of several spots inland and runs downhill until it reaches the sea or a lake. The caller
/// carves the channels into the terrain (`Waters::channels`).
pub fn make_waters(
    rng: &SimRng,
    terrain: &Heightfield,
    cities: &[City],
    highways: &[Highway],
    cfg: &WaterConfig,
    bounds: &RiverBounds,
) -> Waters {
    let half = bounds.world_size_m * 0.5;
    let sea_level_m = bounds.sea_level_m;
    let blocked = |p: Vec2| cities.iter().any(|c| c.rect().distance(p) < cfg.river_city_clearance_m);
    let grid = cfg.river_grid_m;
    let n = (bounds.world_size_m / grid) as i32;
    let at = |ix: i32, iz: i32| Vec2::new(-half + (ix as f32 + 0.5) * grid, -half + (iz as f32 + 0.5) * grid);
    let cell_of = |p: Vec2| (((p.x + half) / grid).floor() as i32, ((p.y + half) / grid).floor() as i32);

    let mut rivers = Vec::new();
    for r in 0..cfg.river_count {
        let mut rr = rng.fork(&format!("river/{r}"));
        // Source: the highest of 24 candidate spots inland, clear of cities.
        let mut source: Option<(i32, i32, f32)> = None;
        for _ in 0..24 {
            let a = rr.next_f32() * std::f32::consts::TAU;
            let d = rr.next_f32().sqrt() * bounds.source_within_m;
            let (ix, iz) = cell_of(Vec2::new(a.cos(), a.sin()) * d);
            if ix < 1 || iz < 1 || ix >= n - 1 || iz >= n - 1 {
                continue;
            }
            let p = at(ix, iz);
            let h = terrain.height_at(p.x, p.y);
            if !blocked(p) && h > sea_level_m && source.is_none_or(|s| h > s.2) {
                source = Some((ix, iz, h));
            }
        }
        let Some((mut ix, mut iz, _)) = source else { continue };
        let mut visited = std::collections::HashSet::new();
        let mut cells = vec![at(ix, iz)];
        visited.insert((ix, iz));
        for step in 0..cfg.river_max_steps {
            let mut best: Option<(f32, i32, i32)> = None;
            for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (jx, jz) = (ix + dx, iz + dz);
                if jx < 0 || jz < 0 || jx >= n || jz >= n || visited.contains(&(jx, jz)) {
                    continue;
                }
                let p = at(jx, jz);
                if blocked(p) {
                    continue;
                }
                let score = terrain.height_at(p.x, p.y) + rr.next_f32() * cfg.meander_m;
                if best.is_none_or(|b| score < b.0) {
                    best = Some((score, jx, jz));
                }
            }
            let Some((_, jx, jz)) = best else { break };
            ix = jx;
            iz = jz;
            visited.insert((ix, iz));
            let p = at(ix, iz);
            cells.push(p);
            let on_edge = ix == 0 || iz == 0 || ix == n - 1 || iz == n - 1;
            if step > 10 && (on_edge || terrain.height_at(p.x, p.y) < sea_level_m) {
                break;
            }
        }
        if cells.len() < 8 {
            continue;
        }
        let path = smooth(&smooth(&cells));
        let count = path.len();
        let mut surface_m = Vec::with_capacity(count);
        let mut half_width_m = Vec::with_capacity(count);
        let mut last = f32::INFINITY;
        for (i, p) in path.iter().enumerate() {
            // Falls downstream, and never below the sea it runs into.
            let s = (terrain.height_at(p.x, p.y) - cfg.freeboard_m).min(last).max(sea_level_m);
            surface_m.push(s);
            last = s;
            half_width_m.push(cfg.river_width_m * 0.5 * (1.0 + cfg.river_widening * i as f32 / count as f32));
        }
        rivers.push(River {
            path,
            surface_m,
            half_width_m,
        });
    }

    // The carve reaches past the water by banks rising at `bank_slope` from the surface.
    let reach_pad = 40.0 / cfg.bank_slope;
    let index = index_rivers(&rivers, bounds.world_size_m, reach_pad);
    let mut waters = Waters {
        sea_level_m,
        rivers,
        bridges: Vec::new(),
        flow_m_s: cfg.flow_m_s,
        index,
    };

    // Bridges: where a river's centre line crosses a highway's.
    let mut bridges = Vec::new();
    for h in highways {
        for r in &waters.rivers {
            for w in r.path.windows(2) {
                if let Some(c) = segment_cross(h.ends[0], h.ends[1], w[0], w[1]) {
                    let along = (h.ends[1] - h.ends[0]).normalize();
                    let (_, t) = h.project(c);
                    let hw = waters.river_at(c).map(|x| x.1).unwrap_or(cfg.river_width_m * 0.5);
                    bridges.push(Bridge {
                        centre: c,
                        along,
                        length_m: 2.0 * (hw + cfg.river_depth_m / cfg.bank_slope) + 20.0,
                        width_m: h.width_m,
                        deck_m: h.grade_at(t),
                    });
                }
            }
        }
    }
    waters.bridges = bridges;
    waters
}

/// The crossing point of segments a-b and c-d, if they cross.
fn segment_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Vec2> {
    let r = b - a;
    let s = d - c;
    let den = r.perp_dot(s);
    if den.abs() < 1e-9 {
        return None;
    }
    let t = (c - a).perp_dot(s) / den;
    let u = (c - a).perp_dot(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| a + r * t)
}
