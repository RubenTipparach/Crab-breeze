//! Air bases in the world plan (openspec change `air-bases`, design section 1): five flat sites beside
//! the highways, clear of the cities and of each other, the first the one nearest the start. Placed
//! after the cities and highways and before the ground is shaped, so the terrain flattens each base's
//! ground the way it does a city's square and the rivers keep out of it. Pure and deterministic.

use super::layout::{City, Highway, Rect};
use super::terrain::Shape;
use crate::data::{Validate, positive};
use glam::Vec2;
use serde::Deserialize;

/// Air base placement, `bases:` in `data/world/world.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BasesConfig {
    /// Bases on the island (owner, survey J5: five).
    pub count: usize,
    /// Half the base's footprint, x and z (a 700 x 500 m site: apron, hangars, repair bay, tower).
    pub half_m: Vec2,
    /// Least distance from a base's edge to any city's square.
    pub city_clear_m: f32,
    /// Least distance between two bases' centres.
    pub spacing_m: f32,
    /// A base's centre sits this far to the side of its highway's centre line, or 1.6 or 2.4 times as far.
    pub highway_offset_m: f32,
    /// The ground under a site may rise at most this much across it (it is flattened).
    pub max_rise_m: f32,
    /// Hangar bays a base has, along its back edge.
    pub hangars: usize,
    /// A hangar bay's inside: width, height and depth, metres (a 24 m mech walks in standing).
    pub hangar_m: glam::Vec3,
}

impl Default for BasesConfig {
    fn default() -> Self {
        Self {
            count: 5,
            half_m: Vec2::new(350.0, 250.0),
            // 600 m: 2 km left no site on the 13-city island (2026-10-25: all 72 candidates were by a city).
            city_clear_m: 600.0,
            spacing_m: 4000.0,
            // Far enough out that the site's flattening never reaches its highway's grading.
            highway_offset_m: 750.0,
            max_rise_m: 30.0,
            hangars: 3,
            hangar_m: glam::Vec3::new(44.0, 34.0, 50.0),
        }
    }
}

impl Validate for BasesConfig {
    fn validate(&self) -> Result<(), String> {
        positive("bases.half_m.x", self.half_m.x)?;
        positive("bases.half_m.y", self.half_m.y)?;
        positive("bases.city_clear_m", self.city_clear_m)?;
        positive("bases.spacing_m", self.spacing_m)?;
        positive("bases.highway_offset_m", self.highway_offset_m)?;
        positive("bases.max_rise_m", self.max_rise_m)?;
        positive("bases.hangar_m.x", self.hangar_m.x)?;
        positive("bases.hangar_m.y", self.hangar_m.y)?;
        positive("bases.hangar_m.z", self.hangar_m.z)?;
        if self.hangars as f32 * (self.hangar_m.x + HANGAR_GAP_M) > 2.0 * self.half_m.x {
            return Err("bases: the hangars do not fit along the base's back edge".into());
        }
        Ok(())
    }
}

/// An air base.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirBase {
    /// Index, the one nearest the start first.
    pub id: u32,
    /// Centre (x, z).
    pub centre: Vec2,
    /// Half its footprint, x and z.
    pub half_m: Vec2,
    /// The height its ground is flattened to.
    pub ground_m: f32,
    /// The highway it stands beside.
    pub highway: u32,
}

/// The space between two hangar bays, metres.
pub const HANGAR_GAP_M: f32 = 12.0;

/// How far a hangar's back wall stands in from the base's back edge, metres.
const HANGAR_BACK_M: f32 = 30.0;

impl AirBase {
    /// Its footprint.
    pub fn rect(&self) -> Rect {
        Rect::centred(self.centre, self.half_m)
    }

    /// Its hangar bays' insides (x, z), along the back edge (high z), open toward the apron (low z).
    /// The one rule for where a hangar is: the game draws them here and repairs a mech standing in one.
    pub fn hangars(&self, cfg: &BasesConfig) -> Vec<Rect> {
        let n = cfg.hangars;
        let pitch = cfg.hangar_m.x + HANGAR_GAP_M;
        let z = self.centre.y + self.half_m.y - HANGAR_BACK_M - cfg.hangar_m.z * 0.5;
        (0..n)
            .map(|i| {
                let x = self.centre.x + (i as f32 - (n as f32 - 1.0) * 0.5) * pitch;
                Rect::centred(Vec2::new(x, z), Vec2::new(cfg.hangar_m.x, cfg.hangar_m.z) * 0.5)
            })
            .collect()
    }
}

/// The base and hangar bay a point (x, z) stands in, if any.
pub fn hangar_at(bases: &[AirBase], cfg: &BasesConfig, p: Vec2) -> Option<(u32, usize)> {
    bases.iter().find_map(|b| {
        if b.rect().distance(p) > 0.0 {
            return None;
        }
        b.hangars(cfg).iter().position(|h| h.distance(p) <= 0.0).map(|i| (b.id, i))
    })
}

/// What a base's site keeps clear of, besides the cities: every highway's grading (`road_m`, from the
/// centre line, the site's own flattening included) and the island's drainage courses, where rivers
/// run (`courses`, kept `river_m` away), so a site never cuts a road or a river's channel. The courses
/// exist whether rivers are drawn or not, so the bases stand in the same places either way.
pub struct KeepClear<'a> {
    /// The highways.
    pub highways: &'a [Highway],
    /// Least distance from a site's edge to a highway's centre line.
    pub road_m: f32,
    /// The drainage courses' points.
    pub courses: &'a [Vec<Vec2>],
    /// Least distance from a site's edge to a course.
    pub river_m: f32,
}

/// Places the bases: candidate sites beside every highway, kept when on land, flat enough and clear of
/// the cities, the roads and the rivers (`clear`); then the one nearest `start` first, and after it,
/// again and again, the candidate farthest from every base so far, at least `spacing_m` from each, until
/// `count` stand.
pub fn place_bases(shape: &Shape, cities: &[City], clear: &KeepClear, start: Vec2, cfg: &BasesConfig, min_ground_m: f32) -> Vec<AirBase> {
    let highways = clear.highways;
    let mut candidates = Vec::new();
    for (hi, h) in highways.iter().enumerate() {
        let along = h.ends[1] - h.ends[0];
        if along.length_squared() < 1.0 {
            continue;
        }
        let side = along.normalize().perp();
        for t in [0.15f32, 0.3, 0.45, 0.6, 0.75, 0.9] {
            for (s, out) in [1.0f32, -1.0].into_iter().flat_map(|s| [1.0f32, 1.6, 2.4].map(|k| (s, k))) {
                let c = h.ends[0] + along * t + side * s * cfg.highway_offset_m * out;
                let site = Rect::centred(c, cfg.half_m);
                if !keeps_clear(&site, clear) {
                    continue;
                }
                if let Some(ground_m) = site_ground(shape, cities, c, cfg, min_ground_m) {
                    candidates.push(AirBase {
                        id: 0,
                        centre: c,
                        half_m: cfg.half_m,
                        ground_m,
                        highway: hi as u32,
                    });
                }
            }
        }
    }
    let mut chosen: Vec<AirBase> = Vec::new();
    if let Some(first) = candidates
        .iter()
        .min_by(|a, b| a.centre.distance(start).total_cmp(&b.centre.distance(start)))
    {
        chosen.push(*first);
    }
    while chosen.len() < cfg.count {
        let next = candidates
            .iter()
            .map(|c| (chosen.iter().map(|b| b.centre.distance(c.centre)).fold(f32::INFINITY, f32::min), c))
            .filter(|(d, _)| *d >= cfg.spacing_m)
            .max_by(|a, b| a.0.total_cmp(&b.0));
        match next {
            Some((_, c)) => chosen.push(*c),
            None => break,
        }
    }
    for (i, b) in chosen.iter_mut().enumerate() {
        b.id = i as u32;
    }
    chosen
}

/// True when no highway's centre line comes within `road_m` of `site`, and no drainage course within
/// `river_m`. A highway is checked along its path, the site at its edge points and corners.
fn keeps_clear(site: &Rect, clear: &KeepClear) -> bool {
    let (lo, hi) = (site.min, site.max);
    let edge = [
        lo,
        hi,
        Vec2::new(lo.x, hi.y),
        Vec2::new(hi.x, lo.y),
        Vec2::new((lo.x + hi.x) * 0.5, lo.y),
        Vec2::new((lo.x + hi.x) * 0.5, hi.y),
        Vec2::new(lo.x, (lo.y + hi.y) * 0.5),
        Vec2::new(hi.x, (lo.y + hi.y) * 0.5),
    ];
    let roads = clear
        .highways
        .iter()
        .all(|h| edge.iter().all(|p| h.project(*p).0 > clear.road_m) && h.path.iter().all(|p| site.distance(*p) > clear.road_m));
    let rivers = clear.courses.iter().flatten().all(|p| site.distance(*p) > clear.river_m);
    roads && rivers
}

/// The ground a site at `c` is flattened to (the mean of its corners and centre), or `None` when the
/// site is off the land, too steep, or too near a city.
fn site_ground(shape: &Shape, cities: &[City], c: Vec2, cfg: &BasesConfig, min_ground_m: f32) -> Option<f32> {
    let reach = cfg.half_m.max_element();
    if cities.iter().any(|city| city.rect().distance(c) - reach < cfg.city_clear_m) {
        return None;
    }
    let mut hs = Vec::with_capacity(9);
    for u in [-1.0f32, 0.0, 1.0] {
        for v in [-1.0f32, 0.0, 1.0] {
            let p = c + Vec2::new(u * cfg.half_m.x, v * cfg.half_m.y);
            if shape.past_land_edge_m(p) >= 0.0 {
                return None;
            }
            hs.push(shape.height(p));
        }
    }
    let (lo, hi) = hs
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), h| (a.min(*h), b.max(*h)));
    let mean = hs.iter().sum::<f32>() / hs.len() as f32;
    (hi - lo <= cfg.max_rise_m && lo >= min_ground_m).then_some(mean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mech_inside_a_hangar_bay_is_in_it_and_one_on_the_apron_is_not() {
        let cfg = BasesConfig::default();
        let base = AirBase {
            id: 3,
            centre: Vec2::new(1000.0, -2000.0),
            half_m: cfg.half_m,
            ground_m: 40.0,
            highway: 0,
        };
        let bays = base.hangars(&cfg);
        assert_eq!(bays.len(), cfg.hangars);
        for (i, bay) in bays.iter().enumerate() {
            assert!(
                base.rect().distance(bay.min) == 0.0 && base.rect().distance(bay.max) == 0.0,
                "bay {i} is inside the base"
            );
            assert_eq!(hangar_at(&[base], &cfg, bay.centre()), Some((3, i)));
        }
        assert_eq!(
            hangar_at(&[base], &cfg, base.centre - Vec2::new(0.0, 100.0)),
            None,
            "the apron is not a bay"
        );
        assert_eq!(hangar_at(&[base], &cfg, Vec2::ZERO), None, "nor is anywhere else");
    }

    /// A measurement instrument (`air-bases`): why candidate sites fail on the shipped seed-42 island.
    #[test]
    #[ignore]
    fn measure_why_base_sites_fail() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
        let cfg: crate::world::WorldConfig = crate::data::load_ron(&path).expect("world data");
        let plan = crate::world::generate(42, &cfg);
        let shape = Shape::new(42, &cfg);
        let b = &cfg.bases;
        let (mut total, mut city, mut sea, mut steep, mut low) = (0, 0, 0, 0, 0);
        let mut rises = Vec::new();
        for h in &plan.highways {
            let along = h.ends[1] - h.ends[0];
            let side = along.normalize_or_zero().perp();
            for t in [0.25f32, 0.5, 0.75] {
                for s in [1.0f32, -1.0] {
                    total += 1;
                    let c = h.ends[0] + along * t + side * s * b.highway_offset_m;
                    let reach = b.half_m.max_element();
                    if plan.cities.iter().any(|k| k.rect().distance(c) - reach < b.city_clear_m) {
                        city += 1;
                        continue;
                    }
                    let mut hs = Vec::new();
                    let mut wet = false;
                    for u in [-1.0f32, 0.0, 1.0] {
                        for v in [-1.0f32, 0.0, 1.0] {
                            let p = c + Vec2::new(u * b.half_m.x, v * b.half_m.y);
                            wet |= shape.past_land_edge_m(p) >= 0.0;
                            hs.push(shape.height(p));
                        }
                    }
                    if wet {
                        sea += 1;
                        continue;
                    }
                    let (lo, hi) = hs
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, z), h| (a.min(*h), z.max(*h)));
                    rises.push(hi - lo);
                    if hi - lo > b.max_rise_m {
                        steep += 1;
                    } else if lo < 2.0 {
                        low += 1;
                    }
                }
            }
        }
        rises.sort_by(f32::total_cmp);
        eprintln!("{total} candidates: {city} by a city, {sea} at sea, {steep} too steep, {low} too low; rises {rises:?}");
    }
}
