//! Where the houses and small businesses stand (openspec change `small-buildings`, design section 7.4):
//! suburbs round every city, small towns strung along the highways with a main street of businesses,
//! and roadside stops between them. Each lot is a kit, a centre, a heading and the height its pad
//! levels the ground to. Pure and deterministic: the same seed and ground give the same lots.

use super::kit::{Kind, Kits};
use crate::data::{Validate, positive, positive_range};
use crate::rng::SimRng;
use crate::world::biome::Biome;
use crate::world::grid::CellIndex;
use crate::world::layout::{City, Highway, Rect};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Placement tuning: `small_buildings:` in `data/world/world.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SmallBuildingsConfig {
    /// Suburbs stand this far out from a city's square, least and most.
    pub suburb_band_m: [f32; 2],
    /// Between the suburb's rows.
    pub suburb_row_spacing_m: f32,
    /// Open ground between neighbouring houses in a row.
    pub suburb_gap_m: f32,
    /// The rows wander in and out by up to this.
    pub suburb_wander_m: f32,
    /// A house turns from facing the city by up to this.
    pub suburb_heading_jitter_deg: f32,
    /// The share of a row's lots left empty.
    pub suburb_empty_share: f32,
    /// How many towns, least and most.
    pub town_count: [u32; 2],
    /// Buildings in a town, least and most.
    pub town_buildings: [u32; 2],
    /// The share of a town's buildings that are businesses (in its middle).
    pub town_business_share: f32,
    /// A town's centre stays this far from any city's square.
    pub town_city_clearance_m: f32,
    /// Least distance between town centres.
    pub town_spacing_m: f32,
    /// Open ground between neighbouring buildings on a main street.
    pub town_gap_m: f32,
    /// From the road's flat shoulder to a lot's front.
    pub setback_m: f32,
    /// A roadside stop every so far along a highway, least and most.
    pub roadside_every_m: [f32; 2],
    /// Buildings at a roadside stop, least and most.
    pub roadside_buildings: [u32; 2],
    /// A pad is flat this far round a lot's footprint...
    pub pad_flat_m: f32,
    /// ...and blends to the ground over this.
    pub pad_blend_m: f32,
    /// A lot whose ground varies more than this across it is not built on.
    pub max_relief_m: f32,
    /// ...except at a roadside stop, cut into the slope beside the road, which may take this.
    pub roadside_max_relief_m: f32,
    /// Lots keep this clear of rivers and lakes.
    pub water_clearance_m: f32,
    /// Lots keep this clear of a city's square.
    pub city_clearance_m: f32,
    /// The words of the manifest's `biomes` that suit a house in each biome (`Temperate`, ...).
    pub houses_by_biome: BTreeMap<String, Vec<String>>,
    /// The manifest words that put a business at a roadside stop.
    pub roadside_words: Vec<String>,
}

impl Default for SmallBuildingsConfig {
    fn default() -> Self {
        let words = |b: &str, w: &[&str]| (b.to_string(), w.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        Self {
            suburb_band_m: [40.0, 150.0],
            suburb_row_spacing_m: 36.0,
            suburb_gap_m: 12.0,
            suburb_wander_m: 6.0,
            suburb_heading_jitter_deg: 8.0,
            suburb_empty_share: 0.15,
            town_count: [3, 8],
            town_buildings: [20, 60],
            town_business_share: 0.4,
            town_city_clearance_m: 900.0,
            town_spacing_m: 1500.0,
            town_gap_m: 6.0,
            setback_m: 4.0,
            roadside_every_m: [3000.0, 4000.0],
            roadside_buildings: [1, 3],
            pad_flat_m: 2.0,
            pad_blend_m: 10.0,
            max_relief_m: 4.0,
            roadside_max_relief_m: 8.0,
            water_clearance_m: 15.0,
            city_clearance_m: 12.0,
            houses_by_biome: [
                words("Temperate", &["temperate"]),
                words("Savanna", &["savanna"]),
                words("Desert", &["desert"]),
                words("Canyon", &["desert"]),
                words("Beach", &["beach", "coast"]),
                words("Montane", &["montane forest"]),
                words("Jungle", &["jungle"]),
                words("Swamp", &["swamp"]),
            ]
            .into_iter()
            .collect(),
            roadside_words: vec!["roadside".into(), "any roadside".into()],
        }
    }
}

impl Validate for SmallBuildingsConfig {
    fn validate(&self) -> Result<(), String> {
        positive_range("small_buildings.suburb_band_m", self.suburb_band_m)?;
        positive("small_buildings.suburb_row_spacing_m", self.suburb_row_spacing_m)?;
        positive("small_buildings.town_spacing_m", self.town_spacing_m)?;
        positive_range("small_buildings.roadside_every_m", self.roadside_every_m)?;
        positive("small_buildings.max_relief_m", self.max_relief_m)?;
        positive("small_buildings.roadside_max_relief_m", self.roadside_max_relief_m)?;
        for (field, v) in [
            ("suburb_gap_m", self.suburb_gap_m),
            ("suburb_wander_m", self.suburb_wander_m),
            ("town_gap_m", self.town_gap_m),
            ("setback_m", self.setback_m),
            ("pad_flat_m", self.pad_flat_m),
            ("pad_blend_m", self.pad_blend_m),
            ("water_clearance_m", self.water_clearance_m),
            ("city_clearance_m", self.city_clearance_m),
            ("town_city_clearance_m", self.town_city_clearance_m),
            ("suburb_heading_jitter_deg", self.suburb_heading_jitter_deg),
        ] {
            if !v.is_finite() || v < 0.0 {
                return Err(format!("`small_buildings.{field}` must be zero or more"));
            }
        }
        for (field, v) in [
            ("suburb_empty_share", self.suburb_empty_share),
            ("town_business_share", self.town_business_share),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return Err(format!("`small_buildings.{field}` must be within 0..1"));
            }
        }
        for (field, r) in [
            ("town_count", self.town_count),
            ("town_buildings", self.town_buildings),
            ("roadside_buildings", self.roadside_buildings),
        ] {
            if r[0] > r[1] {
                return Err(format!("`small_buildings.{field}` must run from least to most"));
            }
        }
        for b in self.houses_by_biome.keys() {
            if biome_named(b).is_none() {
                return Err(format!("`small_buildings.houses_by_biome`: `{b}` is not a biome"));
            }
        }
        Ok(())
    }
}

/// The biome called `name` (as `Biome`'s variants are spelt).
pub fn biome_named(name: &str) -> Option<Biome> {
    crate::world::biome::BIOMES.into_iter().find(|b| format!("{b:?}") == name)
}

/// Which kind of place a lot is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// Round a city.
    Suburb,
    /// On a town's main street.
    Town,
    /// A roadside stop.
    Roadside,
}

/// One building on its lot.
#[derive(Clone, Debug, PartialEq)]
pub struct Lot {
    /// Its kit (index into `Kits::kits`).
    pub kit: u16,
    /// Where the kit's origin stands (x, z).
    pub centre: Vec2,
    /// The height its pad levels the ground to: the kit's y = 0.
    pub ground_m: f32,
    /// Its turn about the vertical (Bevy's `Quat::from_rotation_y`): the front, the kit's +z, faces
    /// `(sin, cos)` of it.
    pub heading_rad: f32,
    /// The footprint as built, (x, z), with the pad's flat margin.
    pub rect: Rect,
    /// The kind of place.
    pub zone: Zone,
}

impl Lot {
    /// A point of the building's frame (x, z) in the world.
    pub fn to_world(&self, local: Vec2) -> Vec2 {
        let (s, c) = self.heading_rad.sin_cos();
        self.centre + Vec2::new(local.x * c + local.y * s, -local.x * s + local.y * c)
    }

    /// A world point (x, z) in the building's frame.
    pub fn to_local(&self, world: Vec2) -> Vec2 {
        let (s, c) = self.heading_rad.sin_cos();
        let d = world - self.centre;
        Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c)
    }

    /// A world direction (x, z) in the building's frame.
    pub fn dir_to_local(&self, d: Vec2) -> Vec2 {
        let (s, c) = self.heading_rad.sin_cos();
        Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c)
    }

    /// A world point in the building's frame, height included.
    pub fn point_to_local(&self, p: Vec3) -> Vec3 {
        let l = self.to_local(Vec2::new(p.x, p.z));
        Vec3::new(l.x, p.y - self.ground_m, l.y)
    }

    /// A point of the building's frame in the world, height included.
    pub fn point_to_world(&self, p: Vec3) -> Vec3 {
        let w = self.to_world(Vec2::new(p.x, p.z));
        Vec3::new(w.x, p.y + self.ground_m, w.y)
    }
}

/// The footprint of a kit's bounds turned by `heading_rad` about `centre`, grown by `margin_m`.
fn footprint(lo: Vec2, hi: Vec2, centre: Vec2, heading_rad: f32, margin_m: f32) -> Rect {
    let (s, c) = heading_rad.sin_cos();
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for p in [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)] {
        let w = centre + Vec2::new(p.x * c + p.y * s, -p.x * s + p.y * c);
        min = min.min(w);
        max = max.max(w);
    }
    Rect {
        min: min - Vec2::splat(margin_m),
        max: max + Vec2::splat(margin_m),
    }
}

/// The heading that turns a building's front to face `dir` (x, z).
pub fn heading_facing(dir: Vec2) -> f32 {
    dir.x.atan2(dir.y)
}

/// Every lot, and an index over them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lots {
    /// The kits they are built from.
    pub kits: Arc<Kits>,
    /// The lots, by id.
    pub lots: Vec<Lot>,
    index: CellIndex,
}

/// Cell size of the lots' index.
const LOTS_CELL_M: f32 = 64.0;

impl Lots {
    /// The lots, indexed over a world `world_size_m` across.
    pub fn new(kits: Arc<Kits>, lots: Vec<Lot>, world_size_m: f32) -> Self {
        let boxes: Vec<(u32, Vec2, Vec2)> = lots.iter().enumerate().map(|(i, l)| (i as u32, l.rect.min, l.rect.max)).collect();
        let side = (world_size_m / LOTS_CELL_M).ceil() as usize;
        let index = CellIndex::build(-world_size_m * 0.5, LOTS_CELL_M, side, &boxes);
        Self { kits, lots, index }
    }

    /// Lots whose footprint comes within `r` of `p`, ascending id.
    pub fn candidates(&self, p: Vec2, r: f32) -> Vec<u32> {
        if self.lots.is_empty() || !p.is_finite() {
            return Vec::new();
        }
        let mut ids = self.index.within(p - Vec2::splat(r), p + Vec2::splat(r));
        ids.retain(|&i| self.lots[i as usize].rect.distance(p) <= r);
        ids
    }

    /// True when a lot's footprint is within `r` of `p`.
    pub fn overlaps(&self, p: Vec2, r: f32) -> bool {
        !self.candidates(p, r).is_empty()
    }

    /// The kit a lot is built from.
    pub fn kit(&self, lot: u32) -> &super::kit::Kit {
        &self.kits.kits[self.lots[lot as usize].kit as usize]
    }

    /// The pads the ground is levelled to: each lot's footprint and height (`Flats::with_pads` grows them by
    /// a terrain cell, so the ground's lattice is flat right across the footprint; the nearest pad wins).
    pub fn pads(&self) -> Vec<(Rect, f32)> {
        self.lots.iter().map(|l| (l.rect, l.ground_m)).collect()
    }
}

/// What placement asks of the world being generated.
pub struct Ground<'a> {
    /// The ground with the cities' squares and the roads levelled, before any lot.
    pub height: &'a dyn Fn(Vec2) -> f32,
    /// How far past the land's edge a point is (negative inland).
    pub past_land_edge_m: &'a dyn Fn(Vec2) -> f32,
    /// True when a river or lake comes within the clearance of a box (least, greatest, clearance).
    pub water_near: &'a dyn Fn(Vec2, Vec2, f32) -> bool,
    /// The biome at a point, its ground and its slope.
    pub biome: &'a dyn Fn(Vec2, f32, f32) -> Biome,
    /// The cities.
    pub cities: &'a [City],
    /// The highways.
    pub highways: &'a [Highway],
    /// Places nothing is built on (air bases, runways).
    pub keep_out: &'a [Rect],
    /// Lowest ground a lot may stand on.
    pub min_ground_m: f32,
    /// From a highway's centre line to the edge of its flat shoulder.
    pub shoulder_m: f32,
    /// The terrain's sample spacing: a pad is flat a cell beyond its lot.
    pub cell_m: f32,
}

/// The kits a house may be in each biome, and the businesses for towns and roadside stops.
struct Choices {
    houses: BTreeMap<String, Vec<u16>>,
    town: Vec<u16>,
    roadside: Vec<u16>,
}

impl Choices {
    fn new(kits: &Kits, cfg: &SmallBuildingsConfig) -> Self {
        let houses = cfg
            .houses_by_biome
            .iter()
            .map(|(b, words)| {
                let ids = kits
                    .kits
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| k.kind == Kind::House && words.iter().any(|w| k.stands_in(w)))
                    .map(|(i, _)| i as u16)
                    .collect();
                (b.clone(), ids)
            })
            .collect();
        let business = |k: &&super::kit::Kit| k.kind == Kind::Business;
        let town = kits
            .kits
            .iter()
            .enumerate()
            .filter(|(_, k)| business(k))
            .map(|(i, _)| i as u16)
            .collect();
        let roadside = kits
            .kits
            .iter()
            .enumerate()
            .filter(|(_, k)| business(k) && cfg.roadside_words.iter().any(|w| k.stands_in(w)))
            .map(|(i, _)| i as u16)
            .collect();
        Self { houses, town, roadside }
    }

    fn houses_in(&self, b: Biome) -> &[u16] {
        self.houses.get(&format!("{b:?}")).map_or(&[], |v| v.as_slice())
    }
}

fn pick(rng: &mut SimRng, from: &[u16]) -> Option<u16> {
    if from.is_empty() {
        return None;
    }
    Some(from[((rng.next_f32() * from.len() as f32) as usize).min(from.len() - 1)])
}

/// Placement in progress: the lots so far and a coarse grid of them for the overlap test.
struct Placer<'a> {
    cfg: &'a SmallBuildingsConfig,
    kits: &'a Kits,
    ground: &'a Ground<'a>,
    lots: Vec<Lot>,
    grid: BTreeMap<(i32, i32), Vec<u32>>,
}

const GRID_M: f32 = 64.0;

impl Placer<'_> {
    fn cells(r: &Rect) -> impl Iterator<Item = (i32, i32)> {
        let (x0, z0) = ((r.min.x / GRID_M).floor() as i32, (r.min.y / GRID_M).floor() as i32);
        let (x1, z1) = ((r.max.x / GRID_M).floor() as i32, (r.max.y / GRID_M).floor() as i32);
        (x0..=x1).flat_map(move |x| (z0..=z1).map(move |z| (x, z)))
    }

    /// The lot of `kit` with its origin at `centre` turned by `heading`, if it may be built: on dry,
    /// level enough land, clear of water, roads, cities, bases and other lots.
    fn try_lot(&self, kit: u16, centre: Vec2, heading: f32, zone: Zone) -> Option<Lot> {
        let k = &self.kits.kits[kit as usize];
        let (lo, hi) = k.footprint();
        let rect = footprint(lo, hi, centre, heading, self.cfg.pad_flat_m);
        let g = self.ground;
        let probes = [
            rect.min,
            Vec2::new(rect.max.x, rect.min.y),
            rect.max,
            Vec2::new(rect.min.x, rect.max.y),
            centre,
        ];
        let heights: Vec<f32> = probes.iter().map(|&p| (g.height)(p)).collect();
        if heights.iter().any(|h| !h.is_finite()) {
            return None;
        }
        let (lo_h, hi_h) = heights.iter().fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
        let relief = if zone == Zone::Roadside {
            self.cfg.roadside_max_relief_m
        } else {
            self.cfg.max_relief_m
        };
        if hi_h - lo_h > relief || lo_h < g.min_ground_m {
            return None;
        }
        if probes.iter().any(|&p| (g.past_land_edge_m)(p) > -self.cfg.pad_blend_m) {
            return None;
        }
        if (g.water_near)(rect.min, rect.max, self.cfg.water_clearance_m) {
            return None;
        }
        let reach = self.cfg.pad_blend_m;
        if g.cities.iter().any(|c| rect_gap(&c.rect(), &rect) < self.cfg.city_clearance_m)
            || g.keep_out.iter().any(|r| rect_gap(r, &rect) < reach)
        {
            return None;
        }
        // The lot's footprint (its turned corners, edges and middle) keeps off the roads' flat shoulders.
        let (s, c) = heading.sin_cos();
        let turn = |p: Vec2| centre + Vec2::new(p.x * c + p.y * s, -p.x * s + p.y * c);
        let corners = [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)].map(turn);
        let edges = [0, 1, 2, 3].map(|i| (corners[i] + corners[(i + 1) % 4]) * 0.5);
        let road_ok = g.highways.iter().all(|h| {
            corners
                .iter()
                .chain(&edges)
                .chain([&centre])
                .all(|&p| h.project(p).0 > g.shoulder_m + 1.0)
        });
        if !road_ok {
            return None;
        }
        let ground_m = heights[4];
        // No overlap; and a neighbour within two terrain cells (where the ground's lattice is shared between
        // their pads) stands at the same height, or the lattice would tilt one of them.
        let reach = 2.0 * g.cell_m + 0.5;
        let near = Rect {
            min: rect.min - Vec2::splat(reach),
            max: rect.max + Vec2::splat(reach),
        };
        if Self::cells(&near).any(|c| {
            self.grid.get(&c).is_some_and(|ids| {
                ids.iter().any(|&i| {
                    let o = &self.lots[i as usize];
                    o.rect.overlaps(&rect) || (rect_gap(&o.rect, &rect) < reach && (o.ground_m - ground_m).abs() > 0.2)
                })
            })
        }) {
            return None;
        }
        Some(Lot {
            kit,
            centre,
            ground_m,
            heading_rad: heading,
            rect,
            zone,
        })
    }

    fn add(&mut self, lot: Lot) {
        let id = self.lots.len() as u32;
        for c in Self::cells(&lot.rect) {
            self.grid.entry(c).or_default().push(id);
        }
        self.lots.push(lot);
    }

    /// Takes back every lot from `n` on (a town that could not be built).
    fn truncate(&mut self, n: usize) {
        self.lots.truncate(n);
        for ids in self.grid.values_mut() {
            ids.retain(|&i| (i as usize) < n);
        }
    }

    fn biome_at(&self, p: Vec2) -> Biome {
        let g = self.ground;
        let h = (g.height)(p);
        let d = 8.0;
        let slope = (((g.height)(p + Vec2::X * d) - h).abs().max(((g.height)(p + Vec2::Y * d) - h).abs())) / d;
        (g.biome)(p, h, slope)
    }
}

/// The gap between two rects (0 when they overlap).
fn rect_gap(a: &Rect, b: &Rect) -> f32 {
    let d = (a.min - b.max).max(b.min - a.max).max(Vec2::ZERO);
    d.length()
}

/// The point at arc length `s` round `r` grown by `d` (a rounded rectangle), and the outward normal there.
fn round_rect_at(r: &Rect, d: f32, s: f32) -> (Vec2, Vec2) {
    let size = r.size();
    let quarter = std::f32::consts::FRAC_PI_2 * d;
    // Bottom edge (z = min), corner, right edge, corner, top edge, corner, left edge, corner.
    let legs = [size.x, quarter, size.y, quarter, size.x, quarter, size.y, quarter];
    let total: f32 = legs.iter().sum();
    let mut s = s.rem_euclid(total.max(1e-3));
    let corners = [Vec2::new(r.max.x, r.min.y), r.max, Vec2::new(r.min.x, r.max.y), r.min];
    let starts = [r.min, Vec2::new(r.max.x, r.min.y), r.max, Vec2::new(r.min.x, r.max.y)];
    let dirs = [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y];
    let normals = [-Vec2::Y, Vec2::X, Vec2::Y, -Vec2::X];
    for i in 0..4 {
        if s <= legs[2 * i] {
            return (starts[i] + dirs[i] * s + normals[i] * d, normals[i]);
        }
        s -= legs[2 * i];
        if s <= legs[2 * i + 1] {
            let a = s / d.max(1e-3);
            let n = normals[i] * a.cos() + normals[(i + 1) % 4] * a.sin();
            return (corners[i] + n * d, n);
        }
        s -= legs[2 * i + 1];
    }
    (r.min - Vec2::Y * d, -Vec2::Y)
}

/// Every lot of a world: suburbs, then towns, then roadside stops. `rng` is forked per place.
pub fn place(rng: &SimRng, cfg: &SmallBuildingsConfig, kits: &Kits, ground: &Ground) -> Vec<Lot> {
    if kits.kits.is_empty() {
        return Vec::new();
    }
    let choices = Choices::new(kits, cfg);
    let mut placer = Placer {
        cfg,
        kits,
        ground,
        lots: Vec::new(),
        grid: BTreeMap::new(),
    };
    suburbs(rng, &choices, &mut placer);
    let towns = towns(rng, &choices, &mut placer);
    roadside(rng, &choices, &mut placer, &towns);
    placer.lots
}

/// Rows of houses round each city, facing it, wandering in and out with the ground.
fn suburbs(rng: &SimRng, choices: &Choices, placer: &mut Placer) {
    let cfg = placer.cfg;
    for city in placer.ground.cities {
        let mut r = rng.fork(&format!("suburbs/{}", city.id));
        let square = city.rect();
        let mut d = cfg.suburb_band_m[0];
        let mut row = 0;
        while d <= cfg.suburb_band_m[1] {
            let phase = r.range_f32(0.0, std::f32::consts::TAU);
            let wave_m = r.range_f32(60.0, 140.0);
            let size = square.size();
            let total = 2.0 * (size.x + size.y) + std::f32::consts::TAU * d;
            let mut s = r.range_f32(0.0, 20.0);
            while s < total {
                let wander = cfg.suburb_wander_m * (s / wave_m + phase).sin();
                let (p, n) = round_rect_at(&square, d + wander, s);
                let biome = placer.biome_at(p);
                let Some(kit) = pick(&mut r, choices.houses_in(biome)) else {
                    s += 20.0;
                    continue;
                };
                let k = &placer.kits.kits[kit as usize];
                let width = k.hi.x - k.lo.x;
                let skip = r.next_f32() < cfg.suburb_empty_share;
                let jitter = r.range_f32(-1.0, 1.0) * cfg.suburb_heading_jitter_deg.to_radians();
                // The front faces the city; the kit's origin sits back from the row line by its depth in front.
                let face = -n;
                let heading = heading_facing(face) + jitter;
                let centre = p - face * k.hi.z;
                if !skip && let Some(lot) = placer.try_lot(kit, centre, heading, Zone::Suburb) {
                    placer.add(lot);
                }
                s += width + cfg.suburb_gap_m + r.range_f32(0.0, cfg.suburb_gap_m);
            }
            row += 1;
            d += cfg.suburb_row_spacing_m + r.range_f32(-4.0, 4.0) * (row as f32).min(1.0);
        }
    }
}

/// A town on a highway: its centre, the road and where along it.
struct Town {
    centre: Vec2,
}

/// Picks town sites along the highways, then lays each one's main street.
fn towns(rng: &SimRng, choices: &Choices, placer: &mut Placer) -> Vec<Town> {
    let cfg = placer.cfg;
    let g = placer.ground;
    let mut r = rng.fork("towns");
    let want = cfg.town_count[0] + (r.next_f32() * (cfg.town_count[1] - cfg.town_count[0] + 1) as f32) as u32;
    let want = want.min(cfg.town_count[1]);
    // Candidate sites every 400 m along every highway, far from cities, on land.
    let mut sites: Vec<(usize, f32, f32)> = Vec::new();
    for (hi, h) in g.highways.iter().enumerate() {
        let len = h.length_m();
        let mut s = 400.0;
        while s < len - 400.0 {
            let t = s / len;
            let p = h.point_at(t);
            let clear = g.cities.iter().all(|c| c.rect().distance(p) >= cfg.town_city_clearance_m);
            if clear && (g.past_land_edge_m)(p) < -200.0 {
                sites.push((hi, t, r.next_f32()));
            }
            s += 400.0;
        }
    }
    sites.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)).then(a.1.total_cmp(&b.1)));
    let mut towns: Vec<Town> = Vec::new();
    for (hi, t, _) in sites {
        if towns.len() as u32 >= want {
            break;
        }
        let h = &g.highways[hi];
        let p = h.point_at(t);
        if towns.iter().any(|o| o.centre.distance(p) < cfg.town_spacing_m) {
            continue;
        }
        let mut tr = r.fork(&format!("town/{}", towns.len()));
        let n = cfg.town_buildings[0] + (tr.next_f32() * (cfg.town_buildings[1] - cfg.town_buildings[0] + 1) as f32) as u32;
        let n = n.min(cfg.town_buildings[1]);
        let before = placer.lots.len();
        let built = main_street(&mut tr, choices, placer, h, t, n);
        if built >= n / 2 {
            towns.push(Town { centre: p });
        } else {
            placer.truncate(before);
        }
    }
    towns
}

/// Lays `n` buildings along highway `h` either side of position `t`: businesses in the middle,
/// houses towards the ends, all facing the road. Returns how many were built.
fn main_street(r: &mut SimRng, choices: &Choices, placer: &mut Placer, h: &Highway, t: f32, n: u32) -> u32 {
    let cfg = placer.cfg;
    let len = h.length_m().max(1.0);
    let business = (n as f32 * cfg.town_business_share).round() as u32;
    // A cursor along the road for each side and direction: [side +1 ahead, side -1 ahead, +1 back, -1 back].
    let mut cursor = [0.0f32; 4];
    let mut built = 0;
    let mut tries = 0;
    let mut i = 0;
    while built < n && tries < n * 4 {
        tries += 1;
        let slot = i % 4;
        i += 1;
        let side = if slot % 2 == 0 { 1.0 } else { -1.0 };
        let ahead = if slot < 2 { 1.0 } else { -1.0 };
        let kits = if built < business {
            &choices.town
        } else {
            let mid = h.point_at(t);
            choices.houses_in(placer.biome_at(mid))
        };
        let Some(kit) = pick(r, kits) else { break };
        let k = &placer.kits.kits[kit as usize];
        let width = k.hi.x - k.lo.x;
        let along = cursor[slot] + width * 0.5;
        let at = (t + ahead * along / len).clamp(0.0, 1.0);
        let road = h.point_at(at);
        let dir = h.dir_at(at);
        let normal = Vec2::new(-dir.y, dir.x) * side;
        let face = -normal;
        let front = road + normal * (placer.ground.shoulder_m + cfg.setback_m);
        let centre = front - face * k.hi.z;
        cursor[slot] += width + cfg.town_gap_m;
        if let Some(lot) = placer.try_lot(kit, centre, heading_facing(face), Zone::Town) {
            placer.add(lot);
            built += 1;
        }
    }
    built
}

/// A roadside stop every few kilometres of highway, away from cities and towns: the first place within a
/// few hundred metres of each mark, either side of the road, where its buildings fit.
fn roadside(rng: &SimRng, choices: &Choices, placer: &mut Placer, towns: &[Town]) {
    let cfg = placer.cfg;
    let g = placer.ground;
    for (hi, h) in g.highways.iter().enumerate() {
        let mut r = rng.fork(&format!("roadside/{hi}"));
        let len = h.length_m().max(1.0);
        let mut s = r.range_f32(cfg.roadside_every_m[0], cfg.roadside_every_m[1]) * 0.5;
        while s < len {
            let n = cfg.roadside_buildings[0] + (r.next_f32() * (cfg.roadside_buildings[1] - cfg.roadside_buildings[0] + 1) as f32) as u32;
            let n = n.min(cfg.roadside_buildings[1]);
            let first_side = if r.next_f32() < 0.5 { 1.0 } else { -1.0 };
            let kits: Vec<u16> = (0..n).filter_map(|_| pick(&mut r, &choices.roadside)).collect();
            'site: for shift in [0.0f32, 80.0, -80.0, 160.0, -160.0, 240.0, -240.0] {
                for side in [first_side, -first_side] {
                    let t0 = (s + shift) / len;
                    let p = h.point_at(t0.clamp(0.0, 1.0));
                    let clear = (0.0..=1.0).contains(&t0)
                        && g.cities.iter().all(|c| c.rect().distance(p) >= 400.0)
                        && towns.iter().all(|tw| tw.centre.distance(p) >= 800.0);
                    if !clear {
                        continue;
                    }
                    let start = placer.lots.len();
                    let mut along = 0.0;
                    for &kit in &kits {
                        let k = &placer.kits.kits[kit as usize];
                        let width = k.hi.x - k.lo.x;
                        let at = (t0 + (along + width * 0.5) / len).clamp(0.0, 1.0);
                        let road = h.point_at(at);
                        let dir = h.dir_at(at);
                        let normal = Vec2::new(-dir.y, dir.x) * side;
                        let face = -normal;
                        let front = road + normal * (g.shoulder_m + cfg.setback_m);
                        if let Some(lot) = placer.try_lot(kit, front - face * k.hi.z, heading_facing(face), Zone::Roadside) {
                            placer.add(lot);
                        }
                        along += width + cfg.town_gap_m * 2.0;
                    }
                    if placer.lots.len() > start {
                        break 'site;
                    }
                }
            }
            s += r.range_f32(cfg.roadside_every_m[0], cfg.roadside_every_m[1]);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lot_turns_its_front_to_face_where_it_is_told() {
        for dir in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y, Vec2::new(0.6, -0.8)] {
            let lot = Lot {
                kit: 0,
                centre: Vec2::new(10.0, -4.0),
                ground_m: 3.0,
                heading_rad: heading_facing(dir),
                rect: Rect::centred(Vec2::ZERO, Vec2::ONE),
                zone: Zone::Suburb,
            };
            let front = lot.to_world(Vec2::new(0.0, 1.0)) - lot.centre;
            assert!(front.distance(dir) < 1e-5, "{dir} -> {front}");
            let p = Vec2::new(3.0, -7.0);
            assert!(lot.to_local(lot.to_world(p)).distance(p) < 1e-4);
        }
    }

    #[test]
    fn the_round_rect_runs_round_the_square_at_its_distance() {
        let r = Rect::centred(Vec2::ZERO, Vec2::splat(100.0));
        for i in 0..200 {
            let (p, n) = round_rect_at(&r, 50.0, i as f32 * 7.3);
            assert!((r.distance(p) - 50.0).abs() < 0.05, "{p}");
            assert!((n.length() - 1.0).abs() < 1e-4);
            assert!(r.distance(p + n) > r.distance(p), "the normal points out");
        }
    }
}
