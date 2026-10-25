//! The sea, lakes, rivers, waterfalls and bridges (openspec changes `terrain-water`, `island-world`,
//! `hydrology`): the sea round the island at its sea level (which also fills inland basins below
//! it), lakes in the basins the drainage filled and under every waterfall, rivers that start as
//! snowmelt streams high on the range, meet and widen on their way down to the sea and fall over the
//! cliff band, carved into the ground, and bridges where they pass under highways. `water_at` is the
//! one water query.

use super::config::WaterConfig;
use super::grid::CellIndex;
use super::hydrology::Drainage;
use super::layout::{City, Highway, Rect};
use super::terrain::{Heightfield, ValueNoise, smoothstep};
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

/// A river: a centre line with a falling surface and a width that grows with its flow.
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

/// A lake: water at `level_m` inside a wandering shore round `centre`, in a bowl carved under it
/// and behind a bank raised round it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lake {
    /// Middle (x, z).
    pub centre: Vec2,
    /// Mean radius of the shore.
    pub radius_m: f32,
    /// Height of the water.
    pub level_m: f32,
    /// Depth of the bowl at the middle.
    pub depth_m: f32,
    /// A plunge pool under a waterfall rather than a basin the drainage filled.
    pub plunge: bool,
}

/// A waterfall: where a river's surface drops over a cliff, from its lip to its foot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Waterfall {
    /// The river.
    pub river: usize,
    /// Index of the lip in the river's path: the segments from here to `foot` are the fall.
    pub lip: usize,
    /// Index of the first point past the foot.
    pub foot: usize,
    /// Water surface at the lip.
    pub top_m: f32,
    /// Water surface at the foot (the plunge pool's level).
    pub bottom_m: f32,
    /// Direction the water leaves the lip in (x, z).
    pub along: Vec2,
    /// Half width of the river at the lip.
    pub half_width_m: f32,
}

/// The lakes and the noise their shores wander by: the one place a lake's shape is worked out, for
/// the water query, the carve and the drawn surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Lakes {
    /// Lakes, basins first, then plunge pools.
    pub list: Vec<Lake>,
    shore: ValueNoise,
    wobble: f32,
}

/// The bank round a lake rises this far above the water at its crest.
const BANK_CREST_M: f32 = 2.7;
/// The bank's crest stands this far outside the shore.
const BANK_CREST_OUT_M: f32 = 8.0;
/// Past the crest the bank falls at the river bank slope; it reaches no farther than this.
const BANK_REACH_M: f32 = 60.0;
/// Inside the shore, ground this far above the water (an islet, a cliff) is left standing.
const KEEP_ABOVE_M: f32 = 8.0;

impl Default for Lakes {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            shore: ValueNoise::new(&SimRng::new(0)),
            wobble: 0.0,
        }
    }
}

impl Lakes {
    /// Signed distance past lake `i`'s shore at `p`, in shore-scaled metres: negative inside.
    pub fn past_shore_m(&self, i: usize, p: Vec2) -> f32 {
        let l = &self.list[i];
        let d = p.distance(l.centre);
        d * (1.0 + self.wobble * self.shore.unit(p.x, p.y, 180.0, 2, 0.5)) - l.radius_m
    }

    /// Farthest from its centre any point lake `i` changes the ground at.
    pub fn reach_m(&self, i: usize) -> f32 {
        (self.list[i].radius_m + BANK_REACH_M) / (1.0 - self.wobble)
    }

    /// The lake whose water `p` is inside the shore of, if any (the first, if two).
    pub fn at(&self, p: Vec2) -> Option<usize> {
        (0..self.list.len()).find(|&i| {
            let l = &self.list[i];
            l.centre.distance_squared(p) < (l.radius_m / (1.0 - self.wobble)).powi(2) && self.past_shore_m(i, p) < 0.0
        })
    }

    /// Distance from lake `i`'s centre along `angle` (radians from +x toward +z) to `out_m` past
    /// its shore: the outline its surface is drawn to.
    pub fn outline_m(&self, i: usize, angle: f32, out_m: f32) -> f32 {
        let l = &self.list[i];
        let dir = Vec2::new(angle.cos(), angle.sin());
        let mut r = l.radius_m + out_m;
        for _ in 0..6 {
            let p = l.centre + dir * r;
            r = (l.radius_m + out_m) / (1.0 + self.wobble * self.shore.unit(p.x, p.y, 180.0, 2, 0.5));
        }
        r
    }

    /// The ground `h` at `p` with the lakes' bowls carved and their banks raised: inside a shore
    /// the ground falls below the water (unless it stands well above it), and just outside it
    /// rises above, so the shore is where the water's edge is.
    pub fn shape(&self, p: Vec2, h: f32, bank_slope: f32) -> f32 {
        let mut h = h;
        for (i, l) in self.list.iter().enumerate() {
            if l.centre.distance_squared(p) > self.reach_m(i).powi(2) {
                continue;
            }
            let x = self.past_shore_m(i, p);
            if x < 0.0 {
                let u = (1.0 + x / l.radius_m).clamp(0.0, 1.0);
                let bed = l.level_m - 0.3 - l.depth_m * (1.0 - u * u);
                let w = 1.0 - smoothstep(l.level_m + 1.0, l.level_m + KEEP_ABOVE_M, h);
                h += (h.min(bed) - h) * w;
            } else if x < BANK_REACH_M {
                let crest = l.level_m + 0.3 + x * (BANK_CREST_M - 0.3) / BANK_CREST_OUT_M;
                let y = (x - BANK_CREST_OUT_M).max(0.0);
                // Falling at the bank slope, steepening so it is below any ground by the reach.
                let fall = y * ((BANK_CREST_M - 0.3) / BANK_CREST_OUT_M + bank_slope)
                    + (y / (BANK_REACH_M - BANK_CREST_OUT_M)).powi(2) * CARVE_RISE_M;
                h = h.max(crest - fall);
            }
        }
        h
    }
}

/// All the water in a world: the sea (and the basins below its level), lakes, rivers, waterfalls
/// and bridges.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waters {
    /// Sea level: ground below it is under the sea, or under a lake at the sea's level.
    pub sea_level_m: f32,
    /// Rivers, the longest-running first: a later one ends where it meets an earlier one.
    pub rivers: Vec<River>,
    /// Lakes.
    pub lakes: Lakes,
    /// Waterfalls.
    pub falls: Vec<Waterfall>,
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

/// Each river's nearest segment to `p` within `reach_m`: (river, segment, distance, along), one
/// per river, in river order. Where two rivers meet at different heights (a tributary in a canyon
/// that the coarse drainage joins to a river on the plateau above), each must be carved and read
/// on its own, or the higher one's channel buries the lower one's water.
fn nearest_each(rivers: &[River], index: &CellIndex, p: Vec2, reach_m: f32) -> Vec<(usize, usize, f32, f32)> {
    let mut out: Vec<(usize, usize, f32, f32)> = Vec::new();
    for &key in index.at(p) {
        let (ri, si) = ((key >> 16) as usize, (key & 0xFFFF) as usize);
        let r = &rivers[ri];
        let (d, t) = project(p, r.path[si], r.path[si + 1]);
        if d > reach_m {
            continue;
        }
        match out.iter_mut().find(|b| b.0 == ri) {
            Some(b) => {
                if d < b.2 || (d == b.2 && si < b.1) {
                    *b = (ri, si, d, t);
                }
            }
            None => out.push((ri, si, d, t)),
        }
    }
    out.sort_by_key(|b| b.0);
    out
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
    /// River surface, half width and flow at `p`, if a river's water reaches it. Where two rivers'
    /// water overlaps, the lower surface (the higher one falls into it).
    fn river_at(&self, p: Vec2) -> Option<(f32, f32, Vec2)> {
        nearest_each(&self.rivers, &self.index, p, 200.0)
            .into_iter()
            .filter_map(|(ri, si, d, t)| {
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
            })
            .reduce(|a, b| if b.0 < a.0 { b } else { a })
    }

    /// The one water query: water over the ground `ground_m` at `p`, if any: the highest of a
    /// river's, a lake's and the sea's surfaces that stands above the ground. Ground below sea level
    /// is under the sea (or a basin at its level), anywhere, past the world's edge too.
    pub fn water_at(&self, p: Vec2, ground_m: f32) -> Option<WaterSample> {
        let river = self.river_at(p).map(|(s, _, f)| (s, f));
        let lake = self.lakes.at(p).map(|i| (self.lakes.list[i].level_m, Vec2::ZERO));
        let sea = Some((self.sea_level_m, Vec2::ZERO));
        let (surface_m, flow_m_s) = [river, lake, sea]
            .into_iter()
            .flatten()
            .filter(|(s, _)| *s > ground_m)
            .reduce(|a, b| if b.0 > a.0 { b } else { a })?;
        Some(WaterSample {
            surface_m,
            depth_m: surface_m - ground_m,
            flow_m_s,
        })
    }

    /// The channels the rivers cut and the lakes' bowls and banks, for the terrain to carve.
    /// Cities' flat ground (with `city_margin_m` round it) is never cut, nor a highway's flat band
    /// (`road_flat_m` either side of its centre line) except under a bridge.
    pub fn channels(&self, cfg: &WaterConfig, cities: &[City], city_margin_m: f32, highways: &[Highway], road_flat_m: f32) -> Channels {
        Channels {
            rivers: self.rivers.clone(),
            index: self.index.clone(),
            lakes: self.lakes.clone(),
            depth_m: cfg.river_depth_m,
            max_half_width_m: cfg.river_max_half_width_m,
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

/// River channels and lakes cut into the ground: lakes' bowls and banks first, then a U channel
/// under each river's water and banks rising at `bank_slope` beside it, never raising the ground,
/// never inside a city's flat ground and never under a highway except where a bridge spans the river.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Channels {
    rivers: Vec<River>,
    index: CellIndex,
    lakes: Lakes,
    depth_m: f32,
    max_half_width_m: f32,
    bank_slope: f32,
    cities: Vec<Rect>,
    city_margin_m: f32,
    highways: Vec<Highway>,
    road_flat_m: f32,
    /// Bridge spans: centre and half length.
    spans: Vec<(Vec2, f32)>,
}

impl Channels {
    /// The ground `h` at `p` with the lakes and channels carved into it.
    pub fn carve(&self, p: Vec2, h: f32) -> f32 {
        let h = self.lakes.shape(p, h, self.bank_slope);
        let near = nearest_each(&self.rivers, &self.index, p, CARVE_REACH_M + self.max_half_width_m);
        if near.is_empty() || self.cities.iter().any(|c| c.distance(p) <= self.city_margin_m) {
            return h;
        }
        let on_road = self.highways.iter().any(|hw| hw.project(p).0 <= self.road_flat_m);
        if on_road && !self.spans.iter().any(|(c, half)| c.distance(p) <= *half) {
            return h;
        }
        // Every river within reach cuts its own channel; the ground is the lowest of them, so no
        // river's water is buried where two meet at different heights.
        near.into_iter().fold(h, |h, (ri, si, d, t)| {
            let r = &self.rivers[ri];
            let lerp = |v: &[f32]| v[si] + (v[si + 1] - v[si]) * t;
            let (s, hw) = (lerp(&r.surface_m), lerp(&r.half_width_m));
            // A stream's channel is at most as deep as it is half wide.
            let depth = self.depth_m.min(hw);
            // Past the water the bank rises at `bank_slope`, steepening with distance so it always
            // meets the ground well inside the reach the channel is indexed for (no cut ends at a
            // cell's edge, however high the ground beside the river stands).
            let x = (d - hw).max(0.0);
            let target = if d < hw {
                s - depth * (1.0 - (d / hw).powi(2))
            } else {
                s + x * self.bank_slope + (x / CARVE_REACH_M).powi(2) * CARVE_RISE_M
            };
            h.min(target)
        })
    }
}

impl Channels {
    /// The ground `h` at `p` as far terrain draws it, sampled `band_m` apart (one coarse cell): the
    /// lakes as `carve` shapes them, but each river's channel held to just under its water out to
    /// `band_m` past the water, instead of cut 7 m deep. A coarse grid then neither digs a dark notch
    /// where one sample falls in a channel nor bridges over the water between two bank samples, so
    /// far rivers show as water on the land (owner, 2026-10-25: "rivers misssing", "can we fix ther
    /// river issues on the terrain?"). Near ground (step 1) keeps `carve`.
    pub fn carve_far(&self, p: Vec2, h: f32, band_m: f32) -> f32 {
        let h = self.lakes.shape(p, h, self.bank_slope);
        let near = nearest_each(&self.rivers, &self.index, p, CARVE_REACH_M + self.max_half_width_m + band_m);
        if near.is_empty() || self.cities.iter().any(|c| c.distance(p) <= self.city_margin_m) {
            return h;
        }
        let on_road = self.highways.iter().any(|hw| hw.project(p).0 <= self.road_flat_m);
        if on_road && !self.spans.iter().any(|(c, half)| c.distance(p) <= *half) {
            return h;
        }
        // Every river within reach holds the ground under its own water (as `carve`).
        near.into_iter().fold(h, |h, (ri, si, d, t)| {
            let r = &self.rivers[ri];
            let lerp = |v: &[f32]| v[si] + (v[si + 1] - v[si]) * t;
            let (s, hw) = (lerp(&r.surface_m), lerp(&r.half_width_m));
            // Water falls downstream: hold the ground under the lowest water within the band, so a
            // far cell across a steep reach does not stand over the water lower down it.
            let mut s = s;
            for j in si + 1..r.path.len() {
                if r.path[j].distance(p) > hw + band_m {
                    break;
                }
                s = s.min(r.surface_m[j]);
            }
            let x = (d - hw - band_m).max(0.0);
            let target = if d < hw + band_m {
                s - FAR_UNDER_WATER_M
            } else {
                s - FAR_UNDER_WATER_M + x * self.bank_slope + (x / CARVE_REACH_M).powi(2) * CARVE_RISE_M
            };
            h.min(target)
        })
    }
}

/// Far ground lies this far under a river's water, so the water is drawn over it.
const FAR_UNDER_WATER_M: f32 = 0.3;

/// The carve reaches this far past a river's water.
const CARVE_REACH_M: f32 = 80.0;
/// By `CARVE_REACH_M` past the water the bank stands this far above it.
const CARVE_RISE_M: f32 = 600.0;

/// Chaikin corner cutting: a smoother line through the same region, keeping both ends.
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

/// Points every `step_m` along `path` (both ends kept), with each one's position along the
/// original points as a fractional index.
fn resample(path: &[Vec2], step_m: f32) -> Vec<(Vec2, f32)> {
    let mut out = vec![(path[0], 0.0)];
    let mut carry = 0.0;
    for (i, w) in path.windows(2).enumerate() {
        let len = w[0].distance(w[1]);
        let mut at = step_m - carry;
        while at < len {
            out.push((w[0].lerp(w[1], at / len), i as f32 + at / len));
            at += step_m;
        }
        carry = (carry + len) % step_m;
    }
    let last = path.len() - 1;
    if out.last().is_some_and(|(p, _)| p.distance(path[last]) > step_m * 0.3) {
        out.push((path[last], last as f32));
    } else if let Some(end) = out.last_mut() {
        *end = (path[last], last as f32);
    }
    out
}

/// What `make_waters` needs to know about the world.
pub struct WaterBounds {
    /// Side of the square world.
    pub world_size_m: f32,
    /// The sea's level: a river ends when it reaches it, and never runs below it.
    pub sea_level_m: f32,
}

/// Lays the drainage's courses on `terrain` (shaped by cities and roads, not yet carved) as rivers,
/// with lakes in its basins, waterfalls where a river drops over a cliff and a plunge pool under
/// each, and finds the bridges. The caller carves the channels and lakes into the terrain
/// (`Waters::channels`).
pub fn make_waters(
    rng: &SimRng,
    drainage: &Drainage,
    terrain: &Heightfield,
    cities: &[City],
    highways: &[Highway],
    cfg: &WaterConfig,
    bounds: &WaterBounds,
) -> Waters {
    let sea_level_m = bounds.sea_level_m;
    let wobble = cfg.lake_shore_wobble;
    // A lake stays clear of every city and highway: its bank would cut them.
    let clear = |c: Vec2, r: f32| {
        let reach = (r + BANK_REACH_M) / (1.0 - wobble);
        cities.iter().all(|k| k.rect().distance(c) > reach + cfg.river_city_clearance_m)
            && highways.iter().all(|h| h.project(c).0 > reach + h.width_m)
    };
    let mut lakes = Lakes {
        list: Vec::new(),
        shore: ValueNoise::new(&rng.fork("shores")),
        wobble,
    };
    let overlaps = |list: &[Lake], c: Vec2, r: f32| list.iter().any(|l| l.centre.distance(c) < l.radius_m + r + 2.0 * BANK_REACH_M);
    for b in &drainage.basins {
        if b.level_m > sea_level_m && clear(b.centre, b.radius_m) && !overlaps(&lakes.list, b.centre, b.radius_m) {
            lakes.list.push(Lake {
                centre: b.centre,
                radius_m: b.radius_m,
                level_m: b.level_m,
                depth_m: cfg.lake_depth_m,
                plunge: false,
            });
        }
    }

    let mut rivers: Vec<River> = Vec::new();
    let mut falls = Vec::new();
    // Courses on the drainage grid run in its eight directions; each point is pushed about by up
    // to `MEANDER_SHARE` of a cell so a river on a long even slope bends instead of ruling a line.
    let meander = ValueNoise::new(&rng.fork("meander"));
    let push = |p: Vec2| {
        let w = 2.5 * cfg.drainage_cell_m;
        Vec2::new(
            meander.unit(p.x, p.y, w, 2, 0.5),
            meander.unit(p.x + 7700.0, p.y - 1300.0, w, 2, 0.5),
        ) * cfg.drainage_cell_m
            * MEANDER_SHARE
    };
    for course in &drainage.courses {
        let mut points: Vec<Vec2> = course.points.iter().map(|&p| p + push(p)).collect();
        // A course that stopped on another river's cell ends on that river's line.
        let mut joins: Option<f32> = None;
        if let Some(end) = points.last().copied() {
            let mut best: Option<(f32, Vec2, f32)> = None;
            for r in &rivers {
                for (q, s) in r.path.iter().zip(&r.surface_m) {
                    let d = q.distance(end);
                    if d < drainage_reach(cfg) && best.is_none_or(|b| d < b.0) {
                        best = Some((d, *q, *s));
                    }
                }
            }
            if let Some((_, q, s)) = best {
                if let Some(e) = points.last_mut() {
                    *e = q;
                }
                joins = Some(s);
            }
        }
        let line = smooth(&smooth(&points));
        let along = resample(&line, cfg.river_step_m);
        let flow_at = |f: f32| {
            // `line` has 4n - 6 points for n course points after two cuts; map back proportionally.
            let k = (f / (line.len() - 1).max(1) as f32 * (course.flow.len() - 1) as f32).round() as usize;
            course.flow[k.min(course.flow.len() - 1)]
        };
        let mut path = Vec::with_capacity(along.len());
        let mut surface_m = Vec::with_capacity(along.len());
        let mut half_width_m = Vec::with_capacity(along.len());
        let mut last = f32::INFINITY;
        for (i, &(p, f)) in along.iter().enumerate() {
            let ground = terrain.height_at(p.x, p.y);
            let mut s = (ground - cfg.freeboard_m).min(last);
            if let Some(li) = lakes.at(p) {
                s = s.max(lakes.list[li].level_m).min(last);
            }
            if i + 1 == along.len()
                && let Some(js) = joins
            {
                s = s.min(js);
            }
            let s = s.max(sea_level_m);
            path.push(p);
            surface_m.push(s);
            last = s;
            let hw = cfg.stream_half_width_m * (flow_at(f) / cfg.stream_min_flow).max(1.0).sqrt();
            half_width_m.push(hw.min(cfg.river_max_half_width_m));
        }
        if path.len() < 3 {
            continue;
        }
        let ri = rivers.len();
        let (path, surface_m, half_width_m, found) = find_falls(ri, path, surface_m, half_width_m, terrain, cfg);
        for f in found {
            let foot = path[f.foot.min(path.len() - 1)];
            let height = f.top_m - f.bottom_m;
            let r = (height * cfg.plunge_radius_per_m).clamp(cfg.lake_radius_m[0], cfg.lake_radius_m[1]);
            let c = foot + f.along * r * 0.35;
            // Only a river (twice a stream's width) digs a pool; a stream lands in its own foam. Nor
            // does a fall whose pool would sit on ground standing above its water (a ledge of the
            // range's terraces): the carving keeps such ground dry, so the pool would hold none.
            let river = f.half_width_m >= 2.0 * cfg.stream_half_width_m;
            let holds = terrain.height_at(c.x, c.y) < f.bottom_m + KEEP_ABOVE_M * 0.5;
            if river && holds && lakes.at(foot).is_none() && clear(c, r) && !overlaps(&lakes.list, c, r) {
                lakes.list.push(Lake {
                    centre: c,
                    radius_m: r,
                    level_m: f.bottom_m,
                    depth_m: (height * 0.3).clamp(4.0, cfg.lake_depth_m),
                    plunge: true,
                });
            }
            falls.push(f);
        }
        rivers.push(River {
            path,
            surface_m,
            half_width_m,
        });
    }

    let index = index_rivers(&rivers, bounds.world_size_m, CARVE_REACH_M);
    let mut waters = Waters {
        sea_level_m,
        rivers,
        lakes,
        falls,
        bridges: Vec::new(),
        flow_m_s: cfg.flow_m_s,
        index,
    };

    // Bridges: where a river's centre line crosses a highway's.
    let mut bridges = Vec::new();
    for h in highways {
        for r in &waters.rivers {
            for w in r.path.windows(2) {
                for (ha, hb) in h.segments_near(w[0]) {
                    let Some(c) = segment_cross(ha, hb, w[0], w[1]) else { continue };
                    let along = (hb - ha).normalize_or(Vec2::X);
                    let (_, t) = h.project(c);
                    let hw = waters.river_at(c).map(|x| x.1).unwrap_or(cfg.stream_half_width_m);
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

/// A fall drops within this many river steps (a cliff); a longer steep run is rapids.
const FALL_MAX_STEPS: usize = 4;

/// Most a course's point is pushed off its drainage cell's centre, as a share of a cell.
const MEANDER_SHARE: f32 = 0.45;

/// How far a tributary's end may be moved onto the river it meets.
fn drainage_reach(cfg: &WaterConfig) -> f32 {
    cfg.drainage_cell_m * 1.5
}

/// River points, surfaces, half widths, and the waterfalls found in them.
type FallSearch = (Vec<Vec2>, Vec<f32>, Vec<f32>, Vec<Waterfall>);

/// Finds where river `ri`'s surface drops at least `fall_step_m` a step for at least
/// `fall_min_height_m` in all, and puts a lip point where the ground first falls away, so the
/// river runs level to the cliff's edge and falls from there.
fn find_falls(ri: usize, path: Vec<Vec2>, surface: Vec<f32>, half: Vec<f32>, terrain: &Heightfield, cfg: &WaterConfig) -> FallSearch {
    let mut out = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut i = 0;
    while i < path.len() {
        out.0.push(path[i]);
        out.1.push(surface[i]);
        out.2.push(half[i]);
        let mut j = i;
        while j + 1 < path.len() && surface[j] - surface[j + 1] >= cfg.fall_step_m {
            j += 1;
        }
        if j > i && j - i <= FALL_MAX_STEPS && surface[i] - surface[j] >= cfg.fall_min_height_m {
            // The lip: a metre short of where the ground first drops a metre below the top's.
            let (a, b) = (path[i], path[i + 1]);
            let top_ground = terrain.height_at(a.x, a.y);
            let len = a.distance(b);
            let dir = (b - a).normalize_or(Vec2::X);
            let mut lip = a;
            let mut x = 1.0;
            while x < len {
                let q = a + dir * x;
                if terrain.height_at(q.x, q.y) < top_ground - 1.0 {
                    break;
                }
                lip = a + dir * (x - 1.0).max(0.0);
                x += 1.0;
            }
            if lip.distance(a) > 0.5 {
                out.0.push(lip);
                out.1.push(surface[i]);
                out.2.push(half[i]);
            }
            let lip_index = out.0.len() - 1;
            out.3.push(Waterfall {
                river: ri,
                lip: lip_index,
                foot: lip_index + (j - i),
                top_m: surface[i],
                bottom_m: surface[j],
                along: dir,
                half_width_m: half[i],
            });
            // The points down the fall, then on from its foot.
            for k in i + 1..j {
                out.0.push(path[k]);
                out.1.push(surface[k]);
                out.2.push(half[k]);
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
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
