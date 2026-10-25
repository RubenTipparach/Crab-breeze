//! Where the island's water goes (openspec change `hydrology`): a coarse drainage grid over the
//! island's shape, before cities and roads exist, so they can be placed clear of the water.
//!
//! Every cell drains to a neighbour by a priority flood from the sea (Barnes, Lehman and Mulla,
//! 2014): cells are visited lowest first outward from the coast, so a closed basin fills to its
//! spill point and drains over it, and every cell's water reaches the sea. Snowmelt and rain are
//! summed down the drainage, so streams start high on the snowy range, meet and widen into rivers.
//! A basin the flood filled deep enough, that a stream runs into, is a lake. `water::make_waters` turns the courses into
//! rivers on the finished ground, with their waterfalls.

use super::config::WaterConfig;
use super::terrain::{Shape, smoothstep};
use glam::Vec2;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// No neighbour: the cell drains into the sea.
const SEA: u32 = u32::MAX;

/// A basin the flood filled: a lake, as a circle of the basin's area round its middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Basin {
    /// Middle of the flooded cells (x, z).
    pub centre: Vec2,
    /// Radius of a circle of the flooded area, within the configured range.
    pub radius_m: f32,
    /// Height of the water: the basin's spill point.
    pub level_m: f32,
}

/// A stream's course down the drainage grid, source to where it meets another, a lake's outflow
/// or the sea.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    /// Cell centres (x, z), source first.
    pub points: Vec<Vec2>,
    /// Water draining through each point, in snow cells' worth.
    pub flow: Vec<f32>,
}

/// The drainage of an island's shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Drainage {
    cell_m: f32,
    origin_m: f32,
    n: usize,
    /// Cells that are streams, for keeping cities clear of them.
    stream: Vec<bool>,
    /// Lakes.
    pub basins: Vec<Basin>,
    /// Stream courses, highest source first.
    pub courses: Vec<Course>,
}

/// A flood front entry: lowest height first, ties to the lower cell index (a stable order).
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

impl Drainage {
    /// The drainage of `shape` over a square world of `world_size_m` whose sea is at `sea_level_m`.
    pub fn new(shape: &Shape, world_size_m: f32, sea_level_m: f32, cfg: &WaterConfig) -> Self {
        let cell_m = cfg.drainage_cell_m;
        let n = (world_size_m / cell_m).floor().max(2.0) as usize;
        let origin_m = -world_size_m * 0.5;
        let at = |k: usize| {
            Vec2::new(
                origin_m + ((k % n) as f32 + 0.5) * cell_m,
                origin_m + ((k / n) as f32 + 0.5) * cell_m,
            )
        };
        let ground: Vec<f32> = (0..n * n).map(|k| shape.height(at(k))).collect();

        // The flood: from the sea and the world's edge inward, lowest first. A cell's filled height
        // is at least its neighbour's plus a centimetre, so a filled basin still drains to its spill.
        let mut filled = ground.clone();
        let mut receiver = vec![SEA; n * n];
        let mut seen = vec![false; n * n];
        let mut front = BinaryHeap::new();
        for k in 0..n * n {
            let (ix, iz) = (k % n, k / n);
            if ix == 0 || iz == 0 || ix == n - 1 || iz == n - 1 || ground[k] < sea_level_m {
                seen[k] = true;
                front.push(Front(ground[k], k as u32));
            }
        }
        let mut order = Vec::with_capacity(n * n);
        while let Some(Front(h, k)) = front.pop() {
            let k = k as usize;
            order.push(k);
            let (ix, iz) = ((k % n) as i32, (k / n) as i32);
            for (dx, dz) in NEIGHBOURS {
                let (jx, jz) = (ix + dx, iz + dz);
                if jx < 0 || jz < 0 || jx >= n as i32 || jz >= n as i32 {
                    continue;
                }
                let j = jz as usize * n + jx as usize;
                if seen[j] {
                    continue;
                }
                seen[j] = true;
                filled[j] = ground[j].max(h + 0.01);
                receiver[j] = k as u32;
                front.push(Front(filled[j], j as u32));
            }
        }

        // Melt and rain, summed downstream: the flood's order reversed visits every cell before the
        // cell it drains to.
        let snow = shape.snowline_m();
        let mut flow: Vec<f32> = ground
            .iter()
            .map(|&h| {
                if h < sea_level_m {
                    0.0
                } else {
                    cfg.lowland_rain + (1.0 - cfg.lowland_rain) * smoothstep(snow - cfg.melt_band_m, snow, h)
                }
            })
            .collect();
        for &k in order.iter().rev() {
            let r = receiver[k];
            if r != SEA {
                flow[r as usize] += flow[k];
            }
        }

        let basins = find_basins(&ground, &filled, &flow, n, cfg, &at);
        let stream: Vec<bool> = (0..n * n)
            .map(|k| flow[k] >= cfg.stream_min_flow && ground[k] >= sea_level_m)
            .collect();

        // Courses: from every stream cell no stream drains into, highest first, down to the sea or
        // to a cell an earlier course already took (where it meets that river).
        let mut fed = vec![false; n * n];
        for k in 0..n * n {
            let r = receiver[k];
            if stream[k] && r != SEA {
                fed[r as usize] = true;
            }
        }
        let mut heads: Vec<usize> = (0..n * n).filter(|&k| stream[k] && !fed[k]).collect();
        heads.sort_by(|&a, &b| ground[b].total_cmp(&ground[a]).then(a.cmp(&b)));
        let mut taken = vec![false; n * n];
        let mut courses = Vec::new();
        for head in heads {
            let mut cells = vec![head];
            taken[head] = true;
            let mut k = head;
            loop {
                let r = receiver[k];
                if r == SEA {
                    break;
                }
                k = r as usize;
                cells.push(k);
                if taken[k] || ground[k] < sea_level_m {
                    break;
                }
                taken[k] = true;
            }
            if cells.len() > cfg.stream_min_cells as usize {
                courses.push(Course {
                    points: cells.iter().map(|&k| at(k)).collect(),
                    flow: cells.iter().map(|&k| flow[k]).collect(),
                });
            }
        }

        Self {
            cell_m,
            origin_m,
            n,
            stream,
            basins,
            courses,
        }
    }

    /// Whether any stream cell or lake comes within `clearance_m` of the box `lo`..`hi`.
    pub fn water_near(&self, lo: Vec2, hi: Vec2, clearance_m: f32) -> bool {
        let pad = clearance_m + self.cell_m * 0.5;
        let cell = |v: f32| ((v - self.origin_m) / self.cell_m).floor() as i64;
        let clamp = |i: i64| i.clamp(0, self.n as i64 - 1) as usize;
        let (x0, x1) = (clamp(cell(lo.x - pad)), clamp(cell(hi.x + pad)));
        let (z0, z1) = (clamp(cell(lo.y - pad)), clamp(cell(hi.y + pad)));
        let streams = (z0..=z1).any(|iz| (x0..=x1).any(|ix| self.stream[iz * self.n + ix]));
        let box_distance = |p: Vec2| (p - p.clamp(lo, hi)).length();
        streams || self.basins.iter().any(|b| box_distance(b.centre) < b.radius_m + clearance_m)
    }
}

/// The flooded basins deep and wide enough to be lakes and fed by a stream, the largest
/// `lake_max_count` (a hollow no stream runs into stays dry).
fn find_basins(ground: &[f32], filled: &[f32], flow: &[f32], n: usize, cfg: &WaterConfig, at: &dyn Fn(usize) -> Vec2) -> Vec<Basin> {
    let wet: Vec<bool> = (0..n * n).map(|k| filled[k] - ground[k] > 0.5).collect();
    let mut group = vec![false; n * n];
    let mut basins = Vec::new();
    let area_m2 = cfg.drainage_cell_m * cfg.drainage_cell_m;
    for start in 0..n * n {
        if !wet[start] || group[start] {
            continue;
        }
        group[start] = true;
        let mut stack = vec![start];
        let (mut count, mut sum, mut level, mut depth, mut fed) = (0u32, Vec2::ZERO, f32::MAX, 0.0f32, 0.0f32);
        while let Some(k) = stack.pop() {
            count += 1;
            sum += at(k);
            level = level.min(filled[k]);
            depth = depth.max(filled[k] - ground[k]);
            fed = fed.max(flow[k]);
            let (ix, iz) = ((k % n) as i32, (k / n) as i32);
            for (dx, dz) in NEIGHBOURS {
                let (jx, jz) = (ix + dx, iz + dz);
                if jx < 0 || jz < 0 || jx >= n as i32 || jz >= n as i32 {
                    continue;
                }
                let j = jz as usize * n + jx as usize;
                if wet[j] && !group[j] {
                    group[j] = true;
                    stack.push(j);
                }
            }
        }
        if depth >= cfg.lake_min_depth_m && count >= cfg.lake_min_cells && fed >= cfg.stream_min_flow {
            let radius = (count as f32 * area_m2 / std::f32::consts::PI).sqrt();
            basins.push(Basin {
                centre: sum / count as f32,
                radius_m: radius.clamp(cfg.lake_radius_m[0], cfg.lake_radius_m[1]),
                level_m: level,
            });
        }
    }
    // The largest, then by cell order (a stable choice).
    basins.sort_by(|a, b| b.radius_m.total_cmp(&a.radius_m));
    basins.truncate(cfg.lake_max_count as usize);
    basins
}
