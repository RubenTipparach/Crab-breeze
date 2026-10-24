//! Rivers and lakes (openspec change `terrain-water`): a sea level that fills low basins as lakes,
//! rivers that run downhill from a high edge of the world round the cities and are carved into the
//! ground, and bridges where they pass under highways. `water_at` is the one water query.

use super::config::WaterConfig;
use super::layout::{City, Highway};
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

/// All the water in a world.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waters {
    /// Lake surface height: ground below it is under water.
    pub sea_level_m: f32,
    /// Rivers.
    pub rivers: Vec<River>,
    /// Bridges.
    pub bridges: Vec<Bridge>,
    /// Flow speed of rivers.
    pub flow_m_s: f32,
    /// Segment buckets for lookup: (river, segment) per 64 m cell.
    buckets: Vec<Vec<(u16, u16)>>,
    origin_m: f32,
    side: usize,
}

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

impl Waters {
    /// The river closest to `p` within reach: (river, segment, distance, t).
    fn nearest(&self, p: Vec2, reach_m: f32) -> Option<(usize, usize, f32, f32)> {
        if self.buckets.is_empty() {
            return None;
        }
        let cell = |v: f32| (((v - self.origin_m) / BUCKET_M).floor().max(0.0) as usize).min(self.side - 1);
        let mut best: Option<(usize, usize, f32, f32)> = None;
        for &(ri, si) in &self.buckets[cell(p.y) * self.side + cell(p.x)] {
            let r = &self.rivers[ri as usize];
            let (d, t) = project(p, r.path[si as usize], r.path[si as usize + 1]);
            if d <= reach_m && best.is_none_or(|b| d < b.2 || (d == b.2 && (ri as usize, si as usize) < (b.0, b.1))) {
                best = Some((ri as usize, si as usize, d, t));
            }
        }
        best
    }

    /// River surface, half width and flow at `p`, if a river's water reaches it.
    fn river_at(&self, p: Vec2) -> Option<(f32, f32, Vec2)> {
        let (ri, si, d, t) = self.nearest(p, 200.0)?;
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

    /// The one water query: water over the ground `ground_m` at `p`, if any.
    pub fn water_at(&self, p: Vec2, ground_m: f32) -> Option<WaterSample> {
        let river = self.river_at(p).map(|(s, _, f)| (s, f));
        let lake = (ground_m < self.sea_level_m).then_some((self.sea_level_m, Vec2::ZERO));
        let (surface_m, flow_m_s) = match (river, lake) {
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

    fn index(&mut self, world_size_m: f32, pad_m: f32) {
        self.origin_m = -world_size_m * 0.5;
        self.side = (world_size_m / BUCKET_M).ceil() as usize + 1;
        self.buckets = vec![Vec::new(); self.side * self.side];
        let cell = |v: f32, o: f32, side: usize| (((v - o) / BUCKET_M).floor().max(0.0) as usize).min(side - 1);
        for (ri, r) in self.rivers.iter().enumerate() {
            for si in 0..r.path.len() - 1 {
                let (a, b) = (r.path[si], r.path[si + 1]);
                let pad = r.half_width_m[si].max(r.half_width_m[si + 1]) + pad_m;
                let lo = a.min(b) - Vec2::splat(pad);
                let hi = a.max(b) + Vec2::splat(pad);
                for z in cell(lo.y, self.origin_m, self.side)..=cell(hi.y, self.origin_m, self.side) {
                    for x in cell(lo.x, self.origin_m, self.side)..=cell(hi.x, self.origin_m, self.side) {
                        self.buckets[z * self.side + x].push((ri as u16, si as u16));
                    }
                }
            }
        }
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

/// Fills lakes, runs and carves rivers, and finds bridges. Changes `terrain` in place.
pub fn make_waters(
    rng: &SimRng,
    terrain: &mut Heightfield,
    cities: &[City],
    highways: &[Highway],
    cfg: &WaterConfig,
    world_size_m: f32,
    city_margin_m: f32,
) -> Waters {
    // Sea level: a percentile of the ground, kept below every city.
    let mut sample: Vec<f32> = terrain.heights_m.iter().step_by(7).copied().collect();
    sample.sort_by(f32::total_cmp);
    let pct = sample[((sample.len() - 1) as f32 * cfg.sea_level_percentile) as usize];
    let lowest_city = cities.iter().map(|c| c.ground_m).fold(f32::INFINITY, f32::min);
    let sea_level_m = pct.min(lowest_city - cfg.city_freeboard_m);

    let half = world_size_m * 0.5;
    let blocked = |p: Vec2| cities.iter().any(|c| c.rect().distance(p) < cfg.river_city_clearance_m);
    let grid = cfg.river_grid_m;
    let n = (world_size_m / grid) as i32;
    let at = |ix: i32, iz: i32| Vec2::new(-half + (ix as f32 + 0.5) * grid, -half + (iz as f32 + 0.5) * grid);

    let mut rivers = Vec::new();
    for r in 0..cfg.river_count {
        let mut rr = rng.fork(&format!("river/{r}"));
        // Source: the highest of 24 candidate cells along the world's edge, clear of cities.
        let mut source: Option<(i32, i32, f32)> = None;
        for _ in 0..24 {
            let side = (rr.next_u64() % 4) as i32;
            let along = (rr.next_f32() * (n - 2) as f32) as i32 + 1;
            let (ix, iz) = match side {
                0 => (along, 0),
                1 => (along, n - 1),
                2 => (0, along),
                _ => (n - 1, along),
            };
            let p = at(ix, iz);
            let h = terrain.height_at(p.x, p.y);
            if !blocked(p) && source.is_none_or(|s| h > s.2) {
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
            let s = (terrain.height_at(p.x, p.y) - cfg.freeboard_m).min(last);
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

    let mut waters = Waters {
        sea_level_m,
        rivers,
        bridges: Vec::new(),
        flow_m_s: cfg.flow_m_s,
        ..Default::default()
    };
    // The carve reaches past the water by banks rising at `bank_slope` from the surface.
    let reach_pad = 40.0 / cfg.bank_slope;
    waters.index(world_size_m, reach_pad);

    // Carve: lower the ground under each river to a U channel with sloped banks, never raising it,
    // never inside a city's flat ground.
    for iz in 0..terrain.n {
        for ix in 0..terrain.n {
            let p = terrain.sample_xz(ix, iz);
            if cities.iter().any(|c| c.rect().distance(p) <= city_margin_m) {
                continue;
            }
            let Some((ri, si, d, t)) = waters.nearest(p, 400.0) else { continue };
            let r = &waters.rivers[ri];
            let lerp = |v: &[f32]| v[si] + (v[si + 1] - v[si]) * t;
            let (s, hw) = (lerp(&r.surface_m), lerp(&r.half_width_m));
            let target = if d < hw {
                s - cfg.river_depth_m * (1.0 - (d / hw).powi(2))
            } else {
                s + (d - hw) * cfg.bank_slope
            };
            let h = &mut terrain.heights_m[iz * terrain.n + ix];
            if target < *h {
                *h = target;
            }
        }
    }

    // Bridges: where a river's centre line crosses a highway's.
    for h in highways {
        for r in &waters.rivers {
            for w in r.path.windows(2) {
                if let Some(c) = segment_cross(h.ends[0], h.ends[1], w[0], w[1]) {
                    let along = (h.ends[1] - h.ends[0]).normalize();
                    let (_, t) = h.project(c);
                    let hw = waters.river_at(c).map(|x| x.1).unwrap_or(cfg.river_width_m * 0.5);
                    waters.bridges.push(Bridge {
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
