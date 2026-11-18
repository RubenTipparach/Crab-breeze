//! City layout: where cities go, their street grids, blocks, lots and buildings, and the highways
//! between them. Pure functions of a seeded stream and `WorldConfig` (procgen-world design
//! section 2). What each block is used for (a Fargo-like town: a small downtown, flats, house blocks, the strip
//! where a highway comes in) is openspec change `city-districts`, design section 4.

use super::config::WorldConfig;
use super::grid::CellIndex;
use crate::data::{Validate, positive, within};
use crate::rng::SimRng;
use glam::Vec2;
use serde::Deserialize;

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

/// The car parks under a viaduct lie this far under the streets' surface, metres, so a ramp's landing lies over them.
pub const UNDER_BELOW_STREET_M: f32 = 0.06;

/// What a city block is used for (openspec change `city-districts`, design section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockUse {
    /// The towers.
    Downtown,
    /// Walk-ups round the downtown.
    Flats,
    /// Businesses with their car parks, where a highway comes into the city.
    Strip,
    /// Car parks between the piers of the viaduct through the city, and its ramps (openspec change `city-districts`,
    /// design section 5).
    Under,
    /// Rows of houses with yards and drives.
    Houses,
    /// Grass and trees (no buildings).
    Park,
    /// A school and its playing field.
    School,
}

impl BlockUse {
    /// True where the block is paved kerb to kerb; false where only a pavement runs round it and the ground shows
    /// inside (yards, car parks, a playing field, a park's grass is its own surface).
    pub fn paved(self) -> bool {
        matches!(self, BlockUse::Downtown | BlockUse::Flats)
    }

    /// True where the houses and businesses of the lot placer (`small_buildings::lots`) stand, not `build_city`'s.
    pub fn has_lots(self) -> bool {
        matches!(self, BlockUse::Houses | BlockUse::Strip)
    }
}

/// A city block: the ground between streets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// The block's whole area, kerb to kerb.
    pub rect: Rect,
    /// What stands on it.
    pub kind: BlockUse,
}

impl Block {
    /// True for a park (no buildings).
    pub fn is_park(&self) -> bool {
        self.kind == BlockUse::Park
    }

    /// The ground at `p` inside the block, kerb height or not: on its pavement (all of a paved block, the ring
    /// `sidewalk_m` wide round any other) the kerb's height; inside the ring, `None` (the ground's own).
    pub fn on_pavement(&self, p: Vec2, sidewalk_m: f32) -> bool {
        if self.rect.distance(p) > 0.0 || self.kind == BlockUse::Under {
            return false;
        }
        self.kind.paved() || self.rect.inset(sidewalk_m).is_none_or(|inner| inner.distance(p) > 0.0)
    }
}

/// How a city's blocks are used (`zones:` in `data/world/world.ron`; openspec change `city-districts`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ZonesConfig {
    /// The downtown: blocks whose centre is within this share of the city's half size of its centre, for the
    /// smallest city and for the capital (scaled by size between).
    pub downtown_share: [f32; 2],
    /// A downtown's buildings fall to this many storeys at its edge.
    pub downtown_edge_storeys: u32,
    /// Flats: blocks within this share of the half size, for the smallest city and for the capital (scaled by size
    /// between: the large downtown grows its housing).
    pub flats_share: [f32; 2],
    /// How often a city's downtown takes each shape: compact, main street, two cores (the capital is always compact).
    pub downtown_shapes: [f32; 3],
    /// A main street downtown: its length and width each way from the centre, as shares of the downtown's radius.
    pub main_street_share: [f32; 2],
    /// A two-core downtown: each core's radius and its centre's distance from the city's centre, as shares of the
    /// downtown's radius.
    pub two_cores_share: [f32; 2],
    /// A downtown block is cut into two big lots along its length, or into four with this chance (the mockup's).
    pub downtown_four_lots_chance: f32,
    /// A downtown lot left open as a plaza, this often.
    pub downtown_plaza_chance: f32,
    /// A downtown lot's chance of a tower in the smallest city: this much, plus this much more at the heart (falling
    /// to nothing at the downtown's edge).
    pub tower_chance_small: [f32; 2],
    /// The same in the largest city (scaled by size between).
    pub tower_chance_large: [f32; 2],
    /// A tower's shaft is this share of its lot's narrower side, least and most: it stands on its lot with room round it.
    pub tower_width_share: [f32; 2],
    /// A tower is at least this many storeys.
    pub tower_min_storeys: u32,
    /// The storeys of a downtown lot with no tower, least and most.
    pub downtown_low_storeys: [u32; 2],
    /// Storeys of the flats, least and most.
    pub flats_storeys: [u32; 2],
    /// The strip: blocks within this of where a highway reaches the city.
    pub strip_reach_m: f32,
    /// The share of the house blocks that are schools.
    pub school_chance: f32,
    /// A school's building: this share of its block's inner length and depth.
    pub school_size: [f32; 2],
}

impl Default for ZonesConfig {
    fn default() -> Self {
        Self {
            downtown_share: [0.18, 0.30],
            downtown_edge_storeys: 6,
            flats_share: [0.45, 0.62],
            downtown_shapes: [0.5, 0.25, 0.25],
            main_street_share: [2.2, 0.45],
            two_cores_share: [0.65, 0.85],
            downtown_four_lots_chance: 0.5,
            downtown_plaza_chance: 0.08,
            tower_chance_small: [0.12, 0.4],
            tower_chance_large: [0.3, 0.6],
            tower_width_share: [0.5, 0.75],
            tower_min_storeys: 10,
            downtown_low_storeys: [3, 8],
            flats_storeys: [3, 5],
            strip_reach_m: 220.0,
            school_chance: 0.03,
            school_size: [0.6, 0.4],
        }
    }
}

impl Validate for ZonesConfig {
    fn validate(&self) -> Result<(), String> {
        within("zones.downtown_share[0]", self.downtown_share[0], 0.0, 1.0)?;
        within("zones.downtown_share[1]", self.downtown_share[1], self.downtown_share[0], 1.0)?;
        within("zones.flats_share[0]", self.flats_share[0], self.downtown_share[0], 1.0)?;
        within(
            "zones.flats_share[1]",
            self.flats_share[1],
            self.downtown_share[1].max(self.flats_share[0]),
            1.0,
        )?;
        for (i, w) in self.downtown_shapes.iter().enumerate() {
            within(&format!("zones.downtown_shapes[{i}]"), *w, 0.0, 1.0)?;
        }
        if self.downtown_shapes.iter().sum::<f32>() <= 0.0 {
            return Err("zones.downtown_shapes: at least one shape must have a share".into());
        }
        within("zones.main_street_share[0]", self.main_street_share[0], 0.1, 5.0)?;
        within("zones.main_street_share[1]", self.main_street_share[1], 0.1, 5.0)?;
        within("zones.two_cores_share[0]", self.two_cores_share[0], 0.1, 2.0)?;
        within("zones.two_cores_share[1]", self.two_cores_share[1], 0.0, 3.0)?;
        within("zones.downtown_four_lots_chance", self.downtown_four_lots_chance, 0.0, 1.0)?;
        within("zones.downtown_plaza_chance", self.downtown_plaza_chance, 0.0, 1.0)?;
        for (name, c) in [
            ("tower_chance_small", self.tower_chance_small),
            ("tower_chance_large", self.tower_chance_large),
        ] {
            within(&format!("zones.{name}[0]"), c[0], 0.0, 1.0)?;
            within(&format!("zones.{name}[1]"), c[1], 0.0, 1.0 - c[0])?;
        }
        within("zones.tower_width_share[0]", self.tower_width_share[0], 0.1, 1.0)?;
        within(
            "zones.tower_width_share[1]",
            self.tower_width_share[1],
            self.tower_width_share[0],
            1.0,
        )?;
        if self.tower_min_storeys < 2 {
            return Err("zones.tower_min_storeys: at least 2".into());
        }
        if self.downtown_low_storeys[0] < 1 || self.downtown_low_storeys[1] < self.downtown_low_storeys[0] {
            return Err("zones.downtown_low_storeys: least at least 1, most at least the least".into());
        }
        if self.flats_storeys[0] < 2 || self.flats_storeys[1] < self.flats_storeys[0] {
            return Err("zones.flats_storeys: least at least 2, most at least the least".into());
        }
        if self.downtown_edge_storeys < self.flats_storeys[1] {
            return Err("zones.downtown_edge_storeys: at least the flats' most".into());
        }
        positive("zones.strip_reach_m", self.strip_reach_m)?;
        within("zones.school_chance", self.school_chance, 0.0, 1.0)?;
        within("zones.school_size[0]", self.school_size[0], 0.1, 0.95)?;
        within("zones.school_size[1]", self.school_size[1], 0.1, 0.95)
    }
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
    /// The pavement round a block that is not paved kerb to kerb (`BlockUse::paved`).
    pub sidewalk_m: f32,
    /// The shape of its downtown (`downtown`): compact until `zone_city` draws it.
    pub downtown_shape: DowntownShape,
}

/// The shape of a city's downtown (openspec change `city-districts`, design section 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DowntownShape {
    /// Round the centre.
    Compact,
    /// A band through the centre, along x or along z.
    MainStreet {
        /// Whether the band runs along x.
        along_x: bool,
    },
    /// Two round cores either side of the centre on a diagonal.
    TwoCores {
        /// The diagonal the cores lie on (x, z), unit length; one core each way along it.
        dir: Vec2,
    },
}

/// A city's downtown: its centre, its radius and its shape, with the one rule for where it is (`reach`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Downtown {
    /// The city's centre (x, z).
    pub centre: Vec2,
    /// The downtown's radius, metres (`ZonesConfig::downtown_share`, scaled by the city's size).
    pub radius_m: f32,
    /// Its shape.
    pub shape: DowntownShape,
    /// A main street's length and width each way, shares of the radius.
    main_street: [f32; 2],
    /// Two cores' radius and offset, shares of the radius.
    two_cores: [f32; 2],
}

impl Downtown {
    /// How far `p` (x, z) lies out through the downtown: 0 at its heart, 1 at its edge, more outside.
    pub fn reach(&self, p: Vec2) -> f32 {
        let r = self.radius_m.max(1.0);
        let d = p - self.centre;
        match self.shape {
            DowntownShape::Compact => d.length() / r,
            DowntownShape::MainStreet { along_x } => {
                let (along, across) = if along_x { (d.x, d.y) } else { (d.y, d.x) };
                (along.abs() / (r * self.main_street[0])).max(across.abs() / (r * self.main_street[1]))
            }
            DowntownShape::TwoCores { dir } => {
                let off = dir * r * self.two_cores[1];
                let core = r * self.two_cores[0];
                (d - off).length().min((d + off).length()) / core
            }
        }
    }
}

/// A city's downtown shape, drawn from its own stream (`rng`): the capital (`id` 0) is compact; a main street runs
/// along the street axis nearer the way to `toward` (where its nearest highway arrives), as a main street leads out to
/// the road into town.
fn draw_downtown_shape(rng: &mut SimRng, id: u32, toward: Option<Vec2>, cfg: &ZonesConfig) -> DowntownShape {
    if id == 0 {
        return DowntownShape::Compact;
    }
    let w = cfg.downtown_shapes;
    let pick = rng.next_f32() * w.iter().sum::<f32>();
    let diagonal = rng.chance(0.5);
    let along_x = toward.map_or_else(|| rng.chance(0.5), |d| d.x.abs() >= d.y.abs());
    if pick < w[0] {
        DowntownShape::Compact
    } else if pick < w[0] + w[1] {
        DowntownShape::MainStreet { along_x }
    } else {
        let dir = if diagonal { Vec2::new(1.0, 1.0) } else { Vec2::new(1.0, -1.0) };
        DowntownShape::TwoCores { dir: dir.normalize() }
    }
}

impl City {
    /// The city's square.
    pub fn rect(&self) -> Rect {
        Rect::centred(self.centre, Vec2::splat(self.half_size_m))
    }

    /// The drawn top of the city's surface at `p` (x, z): on a street its road, `road_lift_m` over the city's
    /// ground, on a block its pavement, `kerb_height_m` over it (as `cb-app::world` draws them). `None` outside the
    /// city's streets and blocks. Buildings stand on blocks: `WorldPlan::walk_top_at` leaves their footprints to
    /// their own floors.
    pub fn surface_top(&self, p: Vec2, road_lift_m: f32, kerb_height_m: f32) -> Option<f32> {
        if self.rect().distance(p) > 0.0 {
            return None;
        }
        if self.streets.iter().any(|s| s.rect.distance(p) <= 0.0) {
            return Some(self.ground_m + road_lift_m);
        }
        // the car parks under a viaduct: their asphalt, a little under the streets (as `cb-app::world` draws them)
        if self.blocks.iter().any(|b| b.kind == BlockUse::Under && b.rect.distance(p) <= 0.0) {
            return Some(self.ground_m + road_lift_m - UNDER_BELOW_STREET_M);
        }
        // a block's pavement (all of a paved block, the ring round any other); inside the ring, the ground's own
        // (yards, car parks and playing fields lie on the ground)
        self.blocks
            .iter()
            .any(|b| b.on_pavement(p, self.sidewalk_m))
            .then_some(self.ground_m + kerb_height_m)
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
    let every = cfg.arterial_every.max(1) as usize;
    loop {
        // a block beside an arterial is long enough for an exit's ramp (openspec change `city-districts`, design 5)
        let i = blocks.len();
        let beside = cfg.trunk.enabled && (i.is_multiple_of(every) || (i + 1).is_multiple_of(every));
        let range = if beside { cfg.trunk.arterial_block_m } else { cfg.block_size_m };
        let b = rng.range_f32(range[0], range[1]);
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
                // a house block until `zone_city` says otherwise (it is drawn here so the stream is the same)
                kind: if rng.chance(cfg.park_block_chance) {
                    BlockUse::Park
                } else {
                    BlockUse::Houses
                },
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
        sidewalk_m: cfg.sidewalk_m,
        downtown_shape: DowntownShape::Compact,
    }
}

/// Says what each of a city's blocks is used for, once its highways are laid (openspec change `city-districts`,
/// design section 4): the downtown in the middle (at least the block nearest the centre), the strip where a highway
/// reaches the city, flats round the downtown, a school now and then, houses everywhere else; parks stay parks.
pub fn zone_city(rng: &mut SimRng, city: &mut City, highways: &[Highway], largest_half_m: f32, cfg: &WorldConfig) {
    let z = &cfg.zones;
    let size_t = (city.half_size_m / largest_half_m).clamp(0.0, 1.0);
    let toward = highways
        .iter()
        .flat_map(|h| h.cities.iter().zip(h.ends).filter(|(c, _)| **c == city.id).map(|(_, e)| e))
        .min_by(|a, b| a.distance(city.centre).total_cmp(&b.distance(city.centre)))
        .map(|e| e - city.centre);
    city.downtown_shape = draw_downtown_shape(&mut rng.fork("downtown"), city.id, toward, z);
    let dt = downtown(city, largest_half_m, cfg);
    let flats_r = city.half_size_m * (z.flats_share[0] + (z.flats_share[1] - z.flats_share[0]) * size_t);
    let ends: Vec<Vec2> = highways
        .iter()
        .flat_map(|h| h.cities.iter().zip(h.ends).filter(|(c, _)| **c == city.id).map(|(_, e)| e))
        .collect();
    // the block nearest the downtown's heart is always downtown, so a small city still has its tallest building
    let nearest = (0..city.blocks.len()).filter(|&i| !city.blocks[i].is_park()).min_by(|&a, &b| {
        let d = |i: usize| dt.reach(city.blocks[i].rect.centre());
        d(a).total_cmp(&d(b)).then(a.cmp(&b))
    });
    for (i, b) in city.blocks.iter_mut().enumerate() {
        let school = rng.chance(z.school_chance);
        if b.is_park() {
            continue;
        }
        let d = b.rect.centre().distance(city.centre);
        b.kind = if dt.reach(b.rect.centre()) <= 1.0 || Some(i) == nearest {
            BlockUse::Downtown
        } else if ends.iter().any(|&e| b.rect.distance(e) < z.strip_reach_m) {
            BlockUse::Strip
        } else if d <= flats_r {
            BlockUse::Flats
        } else if school {
            BlockUse::School
        } else {
            BlockUse::Houses
        };
    }
}

/// A city's downtown: its radius (`ZonesConfig::downtown_share`, scaled by the city's size) and its shape.
pub fn downtown(city: &City, largest_half_m: f32, cfg: &WorldConfig) -> Downtown {
    let z = &cfg.zones;
    let size_t = (city.half_size_m / largest_half_m).clamp(0.0, 1.0);
    Downtown {
        centre: city.centre,
        radius_m: city.half_size_m * (z.downtown_share[0] + (z.downtown_share[1] - z.downtown_share[0]) * size_t),
        shape: city.downtown_shape,
        main_street: z.main_street_share,
        two_cores: z.two_cores_share,
    }
}

/// Fills a city's downtown, flats and school blocks with buildings, skipping lots under a highway's flat ground (the
/// house and strip blocks are the lot placer's, `small_buildings::lots`; a park has none). Downtown storeys fall from
/// the downtown's tallest at its heart (`Downtown::reach`; the lot nearest the heart stands that full height) to
/// `zones.downtown_edge_storeys` at its edge.
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
    let tallest = cfg.storeys_downtown[0] + (cfg.storeys_downtown[1] - cfg.storeys_downtown[0]) * size_t;
    let edge = cfg.zones.downtown_edge_storeys as f32;
    let dt = self::downtown(city, largest_half_m, cfg);
    // the downtown lot nearest its heart: (its reach, its index in `out`, its full height in storeys, its tower's shaft)
    let mut heart: Option<(f32, usize, f32, Rect)> = None;
    let clear_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    let fp_clear = |footprint: &Rect| {
        !highways.iter().any(|h| {
            corners(footprint)
                .iter()
                .chain([footprint.centre()].iter())
                .any(|&p| h.project(p).0 < clear_m)
        })
    };
    for block in &city.blocks {
        if !matches!(block.kind, BlockUse::Downtown | BlockUse::Flats | BlockUse::School) {
            continue;
        }
        let Some(inner) = block.rect.inset(cfg.sidewalk_m) else {
            continue;
        };
        if block.kind == BlockUse::School {
            // One long low building at one end of the block, its playing field beside it.
            let size = inner.size();
            let footprint = Rect {
                min: inner.min + Vec2::splat(2.0),
                max: inner.min + Vec2::new(size.x * cfg.zones.school_size[0], size.y * cfg.zones.school_size[1]),
            };
            if fp_clear(&footprint) {
                out.push(Building {
                    id: first_id + out.len() as u32,
                    city: city.id,
                    footprint,
                    storeys: 2,
                    height_m: 2.0 * cfg.storey_height_m,
                });
            }
            continue;
        }
        if block.kind == BlockUse::Downtown {
            // The mockup's downtown (city-districts design section 6): two or four big lots, a tower on some (more near
            // the heart and in a large city), a slim shaft with room round it; mid-rise blocks on the rest.
            let z = &cfg.zones;
            let size = inner.size();
            let four = rng.chance(z.downtown_four_lots_chance);
            let (nx, nz) = match (four, size.x >= size.y) {
                (true, _) => (2, 2),
                (false, true) => (2, 1),
                (false, false) => (1, 2),
            };
            let cell = Vec2::new(size.x / nx as f32, size.y / nz as f32);
            let chance = |share: f32| {
                let c = |k: usize| z.tower_chance_small[k] + (z.tower_chance_large[k] - z.tower_chance_small[k]) * size_t;
                c(0) + c(1) * share
            };
            for iz in 0..nz {
                for ix in 0..nx {
                    let lot_rect = Rect {
                        min: inner.min + cell * Vec2::new(ix as f32, iz as f32),
                        max: inner.min + cell * Vec2::new(ix as f32 + 1.0, iz as f32 + 1.0),
                    };
                    let plaza = rng.chance(z.downtown_plaza_chance);
                    let setback = rng.range_f32(cfg.setback_m[0], cfg.setback_m[1]);
                    let fill = rng.range_f32(cfg.storey_fill[0], cfg.storey_fill[1]);
                    let roll = rng.next_f32();
                    let width = rng.range_f32(z.tower_width_share[0], z.tower_width_share[1]);
                    let aspect = rng.range_f32(0.8, 1.2);
                    if plaza {
                        continue;
                    }
                    let Some(lot_fp) = lot_rect.inset(setback) else {
                        continue;
                    };
                    let d = dt.reach(lot_fp.centre()).clamp(0.0, 1.0);
                    let max = tallest + (edge - tallest) * d.powf(cfg.storey_falloff_exponent);
                    // the tower's shaft, centred on its lot
                    let side = lot_fp.size().min_element() * width;
                    let half = Vec2::new(side, side * aspect).min(lot_fp.size()) * 0.5;
                    let shaft = Rect::centred(lot_fp.centre(), half);
                    let nearest_yet = heart.is_none_or(|(hd, ..)| d < hd) && fp_clear(&shaft);
                    let tower = roll < chance(1.0 - d);
                    let (footprint, storeys) = if tower {
                        (shaft, ((max * fill).round() as u32).max(z.tower_min_storeys))
                    } else {
                        let [lo, hi] = z.downtown_low_storeys;
                        (lot_fp, lo + ((hi - lo + 1) as f32 * fill_share(fill, cfg)).floor() as u32)
                    };
                    if !fp_clear(&footprint) {
                        continue;
                    }
                    if nearest_yet {
                        heart = Some((d, out.len(), max, shaft));
                    }
                    out.push(Building {
                        id: first_id + out.len() as u32,
                        city: city.id,
                        footprint,
                        storeys,
                        height_m: storeys as f32 * cfg.storey_height_m,
                    });
                }
            }
            continue;
        }
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
                if !fp_clear(&footprint) {
                    continue;
                }
                // only the flats come here (the downtown has its own lots, above)
                let [lo, hi] = cfg.zones.flats_storeys;
                let storeys = lo + ((hi - lo + 1) as f32 * fill_share(fill, cfg)).floor() as u32;
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
    // the lot nearest the downtown's heart stands its full height: the city's tallest tower is a rule, not a draw
    if let Some((_, i, max, shaft)) = heart {
        let b = &mut out[i];
        b.footprint = shaft;
        b.storeys = b.storeys.max(max.round() as u32);
        b.height_m = b.storeys as f32 * cfg.storey_height_m;
    }
    out
}

/// Where `fill` lies in `storey_fill`, 0 to just under 1: the share a building draws its storeys by.
fn fill_share(fill: f32, cfg: &WorldConfig) -> f32 {
    ((fill - cfg.storey_fill[0]) / (cfg.storey_fill[1] - cfg.storey_fill[0]).max(1e-6)).clamp(0.0, 0.999)
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
