//! City layout: where cities go, their street grids, blocks, lots and buildings, and the highways
//! between them. Pure functions of a seeded stream and `WorldConfig` (procgen-world design
//! section 2).

use super::config::WorldConfig;
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

/// A straight graded road joining two cities, from one city's edge to the other's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Highway {
    /// The two cities it joins.
    pub cities: [u32; 2],
    /// Start and end of the paved surface (x, z).
    pub ends: [Vec2; 2],
    /// Ground height at each end.
    pub ends_ground_m: [f32; 2],
    /// Paved width.
    pub width_m: f32,
    /// Length at each end where the road stays level at its city's height, so a city's flat
    /// ground and the road meet exactly.
    pub level_end_m: f32,
}

impl Highway {
    /// Distance from `p` to the centre line, and the position along it in 0..1.
    pub fn project(&self, p: Vec2) -> (f32, f32) {
        let ab = self.ends[1] - self.ends[0];
        let t = ((p - self.ends[0]).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        ((self.ends[0] + ab * t - p).length(), t)
    }

    /// Paved length.
    pub fn length_m(&self) -> f32 {
        self.ends[0].distance(self.ends[1])
    }

    /// The road's grade at position `t` (0..1) along it: level for `level_end_m` at each end,
    /// then an S-curve (smoothstep) between the two city heights, so the slope never jumps.
    pub fn grade_at(&self, t: f32) -> f32 {
        let len = self.length_m();
        let middle = (len - 2.0 * self.level_end_m).max(1.0);
        let u = ((t * len - self.level_end_m) / middle).clamp(0.0, 1.0);
        let s = u * u * (3.0 - 2.0 * u);
        self.ends_ground_m[0] + (self.ends_ground_m[1] - self.ends_ground_m[0]) * s
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

/// Joins the cities with a minimum spanning tree of straight highways (Prim's, ties to the lower
/// index). An edge that would cross a third city's square is not used.
pub fn connect_cities(cities: &[City], cfg: &WorldConfig) -> Vec<Highway> {
    let n = cities.len();
    if n < 2 {
        return Vec::new();
    }
    let blocked = |a: usize, b: usize| {
        let (pa, pb) = (cities[a].centre, cities[b].centre);
        cities.iter().enumerate().any(|(k, c)| {
            if k == a || k == b {
                return false;
            }
            let probe = Highway {
                cities: [0, 0],
                ends: [pa, pb],
                ends_ground_m: [0.0, 0.0],
                width_m: 0.0,
                level_end_m: 0.0,
            };
            let reach = c.half_size_m * std::f32::consts::SQRT_2 + cfg.plateau_flat_margin_m + cfg.plateau_blend_m + cfg.highway_width_m;
            probe.project(c.centre).0 < reach
        })
    };
    let mut in_tree = vec![false; n];
    in_tree[0] = true;
    let mut out = Vec::new();
    for _ in 1..n {
        let mut best: Option<(f32, usize, usize)> = None;
        for a in (0..n).filter(|&a| in_tree[a]) {
            for b in (0..n).filter(|&b| !in_tree[b]) {
                if blocked(a, b) {
                    continue;
                }
                let d = cities[a].centre.distance(cities[b].centre);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, b));
                }
            }
        }
        let Some((_, a, b)) = best else { break };
        in_tree[b] = true;
        let (ca, cb) = (&cities[a], &cities[b]);
        out.push(Highway {
            cities: [ca.id, cb.id],
            ends: [
                exit_point(&ca.rect(), ca.centre, cb.centre),
                exit_point(&cb.rect(), cb.centre, ca.centre),
            ],
            ends_ground_m: [ca.ground_m, cb.ground_m],
            width_m: cfg.highway_width_m,
            // A ray from a city's centre leaves its square at 45 degrees or steeper, so every point
            // of the square within the road's graded band projects within this of the end.
            level_end_m: (cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m + cfg.highway_blend_m) * std::f32::consts::SQRT_2,
        });
    }
    out
}
