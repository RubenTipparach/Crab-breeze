//! City layout: where cities go, their street grids, blocks, lots and buildings, and the highways
//! between them. Pure functions of a seeded stream and `WorldConfig` (procgen-world design
//! section 2).

use super::config::WorldConfig;
use super::grid::CellIndex;
use crate::rng::SimRng;
use glam::Vec2;

/// An axis-aligned rectangle on the ground, in world metres (x and z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Least corner (x, z).
    pub min: Vec2,
    /// Greatest corner (x, z).
    pub max: Vec2,
}

impl Rect {
    /// A rectangle from its centre and half size.
    pub fn centred(centre: Vec2, half: Vec2) -> Self {
        Self {
            min: centre - half,
            max: centre + half,
        }
    }

    /// Centre point.
    pub fn centre(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Width (x) and depth (z).
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// The rectangle shrunk by `d` on every side, or `None` if nothing is left.
    pub fn inset(&self, d: f32) -> Option<Rect> {
        let r = Rect {
            min: self.min + Vec2::splat(d),
            max: self.max - Vec2::splat(d),
        };
        (r.min.x < r.max.x && r.min.y < r.max.y).then_some(r)
    }

    /// True when the two rectangles share any area.
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.min.x < o.max.x && o.min.x < self.max.x && self.min.y < o.max.y && o.min.y < self.max.y
    }

    /// Distance from `p` to the rectangle (zero inside).
    pub fn distance(&self, p: Vec2) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec2::ZERO);
        d.length()
    }
}

/// What kind of street a road piece is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreetKind {
    /// An ordinary street.
    Street,
    /// A wide arterial.
    Arterial,
}

/// A rectangular piece of street surface. Pieces never overlap, so none z-fights another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreetPiece {
    /// Surface.
    pub rect: Rect,
    /// Street or arterial.
    pub kind: StreetKind,
    /// Width across the direction of travel.
    pub width_m: f32,
}

/// A building: a box on a lot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Building {
    /// Stable id, in generation order.
    pub id: u32,
    /// The city it stands in.
    pub city: u32,
    /// Footprint.
    pub footprint: Rect,
    /// Number of storeys.
    pub storeys: u32,
    /// Height from the ground to the roof.
    pub height_m: f32,
}

/// A city block: the pavement between streets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// The block's whole area, kerb to kerb.
    pub rect: Rect,
    /// True for a park (no buildings).
    pub park: bool,
}

/// A city: a flat square with a street grid.
#[derive(Clone, Debug, PartialEq)]
pub struct City {
    /// Index, largest city first.
    pub id: u32,
    /// Centre (x, z).
    pub centre: Vec2,
    /// Half the side of its square.
    pub half_size_m: f32,
    /// Ground height of the whole city.
    pub ground_m: f32,
    /// Street surfaces.
    pub streets: Vec<StreetPiece>,
    /// Blocks between the streets.
    pub blocks: Vec<Block>,
}

impl City {
    /// The city's square.
    pub fn rect(&self) -> Rect {
        Rect::centred(self.centre, Vec2::splat(self.half_size_m))
    }
}

/// A graded road joining two cities, from one city's edge to the other's, along a centre line
/// routed over the island (openspec change `curved-roads`): round the crater's rim, lakes and
/// steep ground, smoothed into curves, with a grade that follows the land within the steepest a
/// road may climb.
#[derive(Clone, Debug, PartialEq)]
pub struct Highway {
    /// The two cities it joins.
    pub cities: [u32; 2],
    /// Start and end of the paved surface (x, z): the centre line's first and last points.
    pub ends: [Vec2; 2],
    /// Ground height at each end (its city's ground).
    pub ends_ground_m: [f32; 2],
    /// Paved width.
    pub width_m: f32,
    /// Length at each end where the road stays level at its city's height, so a city's flat
    /// ground and the road meet exactly.
    pub level_end_m: f32,
    /// Centre line (x, z), `ends[0]` first.
    pub path: Vec<Vec2>,
    /// The road surface's height at each point of `path`.
    pub grade_m: Vec<f32>,
    /// Distance along the road to each point of `path`.
    along_m: Vec<f32>,
    /// The centre line's segments by cell, for `project`.
    index: CellIndex,
}

/// The highway's segment index cells are this size.
const ROAD_CELL_M: f32 = 256.0;
/// `project` finds the centre line within this of a point (farther reads as infinitely far).
pub const ROAD_REACH_M: f32 = 300.0;

impl Highway {
    /// A highway along `path` (at least two points) with `grade_m` at each, in a world
    /// `world_size_m` across.
    pub fn new(cities: [u32; 2], path: Vec<Vec2>, grade_m: Vec<f32>, width_m: f32, level_end_m: f32, world_size_m: f32) -> Self {
        let mut along_m = vec![0.0];
        for w in path.windows(2) {
            along_m.push(along_m.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
        }
        let boxes: Vec<(u32, Vec2, Vec2)> = path
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                (
                    i as u32,
                    w[0].min(w[1]) - Vec2::splat(ROAD_REACH_M),
                    w[0].max(w[1]) + Vec2::splat(ROAD_REACH_M),
                )
            })
            .collect();
        let side = (world_size_m / ROAD_CELL_M).ceil() as usize + 1;
        let index = CellIndex::build(-world_size_m * 0.5, ROAD_CELL_M, side, &boxes);
        let n = path.len();
        Self {
            cities,
            ends: [path[0], path[n - 1]],
            ends_ground_m: [grade_m[0], grade_m[n - 1]],
            width_m,
            level_end_m,
            path,
            grade_m,
            along_m,
            index,
        }
    }

    /// A straight road from `a` to `b` at `ground_m` at each end (a probe, or a road with nothing
    /// in its way), graded level at each end for `level_end_m` and on an S-curve between.
    pub fn straight(cities: [u32; 2], a: Vec2, b: Vec2, ground_m: [f32; 2], width_m: f32, level_end_m: f32, world_size_m: f32) -> Self {
        let len = a.distance(b);
        let steps = (len / 16.0).ceil().max(1.0) as usize;
        let middle = (len - 2.0 * level_end_m).max(1.0);
        let path: Vec<Vec2> = (0..=steps).map(|i| a.lerp(b, i as f32 / steps as f32)).collect();
        let grade = (0..=steps)
            .map(|i| {
                let u = ((i as f32 / steps as f32 * len - level_end_m) / middle).clamp(0.0, 1.0);
                ground_m[0] + (ground_m[1] - ground_m[0]) * u * u * (3.0 - 2.0 * u)
            })
            .collect();
        Self::new(cities, path, grade, width_m, level_end_m, world_size_m)
    }

    /// Distance from `p` to the centre line, and the position along it in 0..1. Farther than
    /// `ROAD_REACH_M` from the road reads as infinitely far.
    pub fn project(&self, p: Vec2) -> (f32, f32) {
        let mut best = (f32::INFINITY, 0.0);
        for &i in self.index.at(p) {
            let i = i as usize;
            let (a, b) = (self.path[i], self.path[i + 1]);
            let ab = b - a;
            let u = if ab.length_squared() > 0.0 {
                ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (a + ab * u).distance(p);
            if d < best.0 {
                best = (d, (self.along_m[i] + ab.length() * u) / self.length_m().max(1e-3));
            }
        }
        best
    }

    /// The centre line's segments within `ROAD_REACH_M` of `p`, as (start, end).
    pub fn segments_near(&self, p: Vec2) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.index.at(p).iter().map(|&i| (self.path[i as usize], self.path[i as usize + 1]))
    }

    /// Paved length.
    pub fn length_m(&self) -> f32 {
        self.along_m.last().copied().unwrap_or(0.0)
    }

    /// The segment and the share of it at position `t` (0..1) along the road.
    fn locate(&self, t: f32) -> (usize, f32) {
        let s = t.clamp(0.0, 1.0) * self.length_m();
        let i = self.along_m.partition_point(|&a| a <= s).saturating_sub(1).min(self.path.len() - 2);
        let len = (self.along_m[i + 1] - self.along_m[i]).max(1e-6);
        (i, ((s - self.along_m[i]) / len).clamp(0.0, 1.0))
    }

    /// The point on the centre line at position `t` (0..1) along the road.
    pub fn point_at(&self, t: f32) -> Vec2 {
        let (i, u) = self.locate(t);
        self.path[i].lerp(self.path[i + 1], u)
    }

    /// The road's direction at position `t` (0..1).
    pub fn dir_at(&self, t: f32) -> Vec2 {
        let (i, _) = self.locate(t);
        (self.path[i + 1] - self.path[i]).normalize_or(Vec2::X)
    }

    /// The road's grade at position `t` (0..1) along it.
    pub fn grade_at(&self, t: f32) -> f32 {
        let (i, u) = self.locate(t);
        self.grade_m[i] + (self.grade_m[i + 1] - self.grade_m[i]) * u
    }
}

/// Places city squares: centres far enough apart, squares separated by open ground, clear of the
/// world's edge, and only where `on_land(centre, half_size)` says the square stands on dry land.
/// The first is the capital: the largest size, its centre within `capital_within_m` of the
/// island's centre (it is drawn until it fits, up to `city_attempts` times). Returned largest
/// first (ties by placement order), ids assigned in that order.
pub fn place_cities(rng: &mut SimRng, cfg: &WorldConfig, on_land: &dyn Fn(Vec2, f32) -> bool) -> Vec<(Vec2, f32)> {
    let mut placed: Vec<(Vec2, f32)> = Vec::new();
    let half_world = cfg.world_size_m * 0.5;
    for _ in 0..cfg.city_attempts {
        if placed.len() >= cfg.city_count as usize {
            break;
        }
        let capital = placed.is_empty();
        let half = if capital {
            cfg.city_half_size_m[1]
        } else {
            rng.range_f32(cfg.city_half_size_m[0], cfg.city_half_size_m[1])
        };
        let mut reach = half_world - cfg.city_edge_margin_m - half - cfg.plateau_flat_margin_m - cfg.plateau_blend_m;
        if capital {
            reach = reach.min(cfg.capital_within_m);
        }
        if reach <= 0.0 {
            continue;
        }
        let c = Vec2::new(rng.range_f32(-reach, reach), rng.range_f32(-reach, reach));
        if capital && c.length() > cfg.capital_within_m {
            continue;
        }
        let fits = placed.iter().all(|&(o, oh)| {
            let gap = (c - o).abs() - Vec2::splat(half + oh);
            c.distance(o) >= cfg.city_min_spacing_m && gap.max_element() >= cfg.open_ground_min_m
        });
        if fits && on_land(c, half) {
            placed.push((c, half));
        }
    }
    // Largest first; a stable sort keeps placement order for equal sizes.
    placed.sort_by(|a, b| b.1.total_cmp(&a.1));
    placed
}

/// Cuts `[lo, hi]` into alternating streets and blocks, starting and ending with a street whose
/// outer edge is the city's edge. Returns street centre lines with widths, and block spans.
/// Street centre lines as (centre, width), and block spans as (start, end), in metres.
type Lines = (Vec<(f32, f32)>, Vec<(f32, f32)>);

fn street_lines(rng: &mut SimRng, lo: f32, hi: f32, cfg: &WorldConfig) -> Lines {
    let width = |i: usize| {
        if i.is_multiple_of(cfg.arterial_every as usize) {
            cfg.arterial_width_m
        } else {
            cfg.street_width_m
        }
    };
    // Draw blocks until the next one would not fit, then stretch them all to fill exactly.
    let mut blocks = Vec::new();
    let mut used = width(0);
    loop {
        let b = rng.range_f32(cfg.block_size_m[0], cfg.block_size_m[1]);
        let next = used + b + width(blocks.len() + 1);
        if next > hi - lo && !blocks.is_empty() {
            break;
        }
        blocks.push(b);
        used = next;
    }
    let stretch = 1.0 + ((hi - lo) - used) / blocks.iter().sum::<f32>();
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut at = lo;
    for (i, b) in blocks.iter().enumerate() {
        let w = width(i);
        lines.push((at + w * 0.5, w));
        at += w;
        let span = b * stretch;
        spans.push((at, at + span));
        at += span;
    }
    let w = width(blocks.len());
    lines.push((at + w * 0.5, w));
    (lines, spans)
}

fn kind_of(width_m: f32, cfg: &WorldConfig) -> StreetKind {
    if width_m >= cfg.arterial_width_m {
        StreetKind::Arterial
    } else {
        StreetKind::Street
    }
}

/// Lays out one city's streets and blocks.
pub fn lay_out_city(rng: &mut SimRng, id: u32, centre: Vec2, half: f32, ground_m: f32, cfg: &WorldConfig) -> City {
    let r = Rect::centred(centre, Vec2::splat(half));
    let (xs, x_spans) = street_lines(rng, r.min.x, r.max.x, cfg);
    let (zs, z_spans) = street_lines(rng, r.min.y, r.max.y, cfg);
    let mut streets = Vec::new();
    // Streets running along x take the whole width; streets along z are cut between them, so no
    // two pieces overlap.
    for &(z, w) in &zs {
        streets.push(StreetPiece {
            rect: Rect {
                min: Vec2::new(r.min.x, z - w * 0.5),
                max: Vec2::new(r.max.x, z + w * 0.5),
            },
            kind: kind_of(w, cfg),
            width_m: w,
        });
    }
    for &(x, w) in &xs {
        for &(z0, z1) in &z_spans {
            streets.push(StreetPiece {
                rect: Rect {
                    min: Vec2::new(x - w * 0.5, z0),
                    max: Vec2::new(x + w * 0.5, z1),
                },
                kind: kind_of(w, cfg),
                width_m: w,
            });
        }
    }
    let mut blocks = Vec::new();
    for &(z0, z1) in &z_spans {
        for &(x0, x1) in &x_spans {
            blocks.push(Block {
                rect: Rect {
                    min: Vec2::new(x0, z0),
                    max: Vec2::new(x1, z1),
                },
                park: rng.chance(cfg.park_block_chance),
            });
        }
    }
    City {
        id,
        centre,
        half_size_m: half,
        ground_m,
        streets,
        blocks,
    }
}

/// Fills a city's blocks with buildings, skipping lots under a highway's flat ground.
pub fn build_city(
    rng: &mut SimRng,
    city: &City,
    highways: &[Highway],
    first_id: u32,
    largest_half_m: f32,
    cfg: &WorldConfig,
) -> Vec<Building> {
    let mut out = Vec::new();
    let size_t = (city.half_size_m / largest_half_m).clamp(0.0, 1.0);
    let downtown = cfg.storeys_downtown[0] + (cfg.storeys_downtown[1] - cfg.storeys_downtown[0]) * size_t;
    let edge = cfg.storeys_edge as f32;
    let clear_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    for block in &city.blocks {
        if block.park {
            continue;
        }
        let Some(inner) = block.rect.inset(cfg.sidewalk_m) else {
            continue;
        };
        let lot = rng.range_f32(cfg.lot_size_m[0], cfg.lot_size_m[1]);
        let size = inner.size();
        let nx = ((size.x / lot).floor() as usize).max(1);
        let nz = ((size.y / lot).floor() as usize).max(1);
        let cell = Vec2::new(size.x / nx as f32, size.y / nz as f32);
        for iz in 0..nz {
            for ix in 0..nx {
                let lot_rect = Rect {
                    min: inner.min + cell * Vec2::new(ix as f32, iz as f32),
                    max: inner.min + cell * Vec2::new(ix as f32 + 1.0, iz as f32 + 1.0),
                };
                let plaza = rng.chance(cfg.plaza_lot_chance);
                let setback = rng.range_f32(cfg.setback_m[0], cfg.setback_m[1]);
                let fill = rng.range_f32(cfg.storey_fill[0], cfg.storey_fill[1]);
                if plaza {
                    continue;
                }
                let Some(footprint) = lot_rect.inset(setback) else {
                    continue;
                };
                if highways.iter().any(|h| {
                    corners(&footprint)
                        .iter()
                        .chain([footprint.centre()].iter())
                        .any(|&p| h.project(p).0 < clear_m)
                }) {
                    continue;
                }
                let d = (footprint.centre().distance(city.centre) / city.half_size_m).clamp(0.0, 1.0);
                let max = downtown + (edge - downtown) * d.powf(cfg.storey_falloff_exponent);
                let storeys = ((max * fill).round() as u32).max(cfg.storeys_edge);
                out.push(Building {
                    id: first_id + out.len() as u32,
                    city: city.id,
                    footprint,
                    storeys,
                    height_m: storeys as f32 * cfg.storey_height_m,
                });
            }
        }
    }
    out
}

/// How far each facade of `footprint` looks out before another building's footprint, in the side
/// order of the dressing (0 faces -z, 1 +z, 2 -x, 3 +x), at most `limit_m`: the strip in front of a
/// facade, its full width, out to the first footprint of `others` that overlaps it. Half of it is
/// what that facade's trim may stand out (two neighbours' awnings must not meet).
pub fn clear_in_front(footprint: &Rect, others: &[Rect], limit_m: f32) -> [f32; 4] {
    let (lo, hi) = (footprint.min, footprint.max);
    let mut out = [limit_m; 4];
    for o in others {
        let across_x = o.min.x < hi.x && o.max.x > lo.x;
        let across_z = o.min.y < hi.y && o.max.y > lo.y;
        if across_x && o.max.y <= lo.y {
            out[0] = out[0].min(lo.y - o.max.y);
        }
        if across_x && o.min.y >= hi.y {
            out[1] = out[1].min(o.min.y - hi.y);
        }
        if across_z && o.max.x <= lo.x {
            out[2] = out[2].min(lo.x - o.max.x);
        }
        if across_z && o.min.x >= hi.x {
            out[3] = out[3].min(o.min.x - hi.x);
        }
    }
    out
}

/// The facade of `footprint` that faces the street, as a side (0 faces -z, 1 +z, 2 -x, 3 +x): of the
/// facades with nothing built between them and the edge of `block` (the kerb; `fronts` is
/// `clear_in_front`), the nearest the kerb, ties to a long face and then the lower side. An inner lot
/// has none: its door faces the widest gap in front, an alley to the pavement. Without a block, the
/// widest gap.
pub fn street_side(footprint: &Rect, block: Option<&Rect>, fronts: [f32; 4]) -> u8 {
    let size = footprint.size();
    let long = |s: usize| (s < 2) == (size.x >= size.y);
    let widest = || (0..4).max_by(|&a, &b| fronts[a].total_cmp(&fronts[b]).then(b.cmp(&a))).unwrap_or(0) as u8;
    let Some(block) = block else { return widest() };
    let kerb = [
        footprint.min.y - block.min.y,
        block.max.y - footprint.max.y,
        footprint.min.x - block.min.x,
        block.max.x - footprint.max.x,
    ];
    (0..4)
        .filter(|&s| fronts[s] >= kerb[s] - 1e-3)
        .min_by(|&a, &b| kerb[a].total_cmp(&kerb[b]).then(long(b).cmp(&long(a))).then(a.cmp(&b)))
        .map_or_else(widest, |s| s as u8)
}

fn corners(r: &Rect) -> [Vec2; 4] {
    [r.min, Vec2::new(r.max.x, r.min.y), r.max, Vec2::new(r.min.x, r.max.y)]
}

/// Where a ray from `from` toward `to` leaves the square `r` (from inside).
fn exit_point(r: &Rect, from: Vec2, to: Vec2) -> Vec2 {
    let d = to - from;
    let tx = if d.x > 0.0 {
        (r.max.x - from.x) / d.x
    } else if d.x < 0.0 {
        (r.min.x - from.x) / d.x
    } else {
        f32::INFINITY
    };
    let tz = if d.y > 0.0 {
        (r.max.y - from.y) / d.y
    } else if d.y < 0.0 {
        (r.min.y - from.y) / d.y
    } else {
        f32::INFINITY
    };
    from + d * tx.min(tz).min(1.0)
}

/// Joins the cities with a minimum spanning tree of highways (Prim's on the straight distance,
/// ties to the lower index; a spanning forest where cities cannot be joined). `build(a, b, from,
/// to)` lays the road from city `a`'s edge point `from` to city `b`'s `to`, or refuses (no route
/// round the mountains), and a refused pair is not tried again. An edge whose straight line would
/// cross a third city's square is not used.
pub fn connect_cities(cities: &[City], cfg: &WorldConfig, build: &dyn Fn(usize, usize, Vec2, Vec2) -> Option<Highway>) -> Vec<Highway> {
    let n = cities.len();
    if n < 2 {
        return Vec::new();
    }
    let crosses_a_city = |a: usize, b: usize| {
        let (pa, pb) = (cities[a].centre, cities[b].centre);
        cities.iter().enumerate().any(|(k, c)| {
            if k == a || k == b {
                return false;
            }
            let ab = pb - pa;
            let t = ((c.centre - pa).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let reach = c.half_size_m * std::f32::consts::SQRT_2 + cfg.plateau_flat_margin_m + cfg.plateau_blend_m + cfg.highway_width_m;
            (pa + ab * t).distance(c.centre) < reach
        })
    };
    let mut refused = std::collections::BTreeSet::new();
    let mut in_tree = vec![false; n];
    in_tree[0] = true;
    let mut out = Vec::new();
    while in_tree.iter().any(|t| !t) {
        let mut best: Option<(f32, usize, usize)> = None;
        for a in (0..n).filter(|&a| in_tree[a]) {
            for b in (0..n).filter(|&b| !in_tree[b]) {
                if refused.contains(&(a, b)) || crosses_a_city(a, b) {
                    continue;
                }
                let d = cities[a].centre.distance(cities[b].centre);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, b));
                }
            }
        }
        // Nothing joins the tree: start another from the first city left out (a spanning forest).
        let Some((_, a, b)) = best else {
            if let Some(k) = (0..n).find(|&k| !in_tree[k]) {
                in_tree[k] = true;
            }
            continue;
        };
        let (ca, cb) = (&cities[a], &cities[b]);
        match build(
            a,
            b,
            exit_point(&ca.rect(), ca.centre, cb.centre),
            exit_point(&cb.rect(), cb.centre, ca.centre),
        ) {
            Some(h) => {
                in_tree[b] = true;
                out.push(h);
            }
            None => {
                refused.insert((a, b));
            }
        }
    }
    out
}

/// Length at each end of a highway that stays level at its city's height: a ray from a city's
/// centre leaves its square at 45 degrees or steeper, so every point of the square within the
/// road's graded band projects within this of the end.
pub fn level_end_m(cfg: &WorldConfig) -> f32 {
    (cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m + cfg.highway_blend_m) * std::f32::consts::SQRT_2
}
