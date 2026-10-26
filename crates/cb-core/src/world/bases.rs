//! Air bases in the world plan (openspec change `air-bases`, design section 1): five flat sites beside
//! the highways, clear of the cities and of each other, the first the one nearest the start. Placed
//! after the cities and highways and before the ground is shaped, so the terrain flattens each base's
//! ground the way it does a city's square and the rivers keep out of it. Pure and deterministic.
//!
//! Each base has a runway (survey W45, owner 2026-10-26: "Add a real runway, about 1.5 to 2 km, to each base"): a
//! straight strip beside its site, flattened to the site's ground, on dry land clear of the hangars, the tower, the
//! cities, the highways and the rivers. It is tried in front of the apron first, then behind the hangars, then down
//! either side, and longest first; a candidate site that can hold none is not used (`runway_for`).

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
    /// A bay's terminal stands this far in from the bay's left wall and its mouth, metres (x, z): clear
    /// of a mech walking in down the middle.
    pub terminal_in_m: Vec2,
    /// The pilot can use a terminal standing within this of it, metres.
    pub terminal_reach_m: f32,
    /// ... and looking within this of it, degrees either side.
    pub terminal_look_deg: f32,
    /// A runway's length: the longest tried first, down to the shortest, metres (survey W45: 1.5 to 2 km).
    pub runway_m: [f32; 2],
    /// A runway's paved width, metres.
    pub runway_width_m: f32,
    /// Between the site's edge and the runway's edge (the taxiway's length), metres.
    pub runway_gap_m: f32,
    /// Lengths are tried this far apart, and the runway slid along its line this far either way, metres.
    pub runway_step_m: f32,
    /// The natural ground under a runway may lie at most this far above or below the site's ground (it is
    /// flattened to it), metres.
    pub runway_max_cut_m: f32,
    /// Least distance from a runway to any city's square, metres.
    pub runway_city_clear_m: f32,
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
            terminal_in_m: Vec2::new(3.0, 6.0),
            terminal_reach_m: 3.0,
            terminal_look_deg: 45.0,
            runway_m: [2000.0, 1500.0],
            runway_width_m: 45.0,
            runway_gap_m: 60.0,
            runway_step_m: 100.0,
            runway_max_cut_m: 45.0,
            runway_city_clear_m: 300.0,
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
        positive("bases.terminal_reach_m", self.terminal_reach_m)?;
        positive("bases.terminal_look_deg", self.terminal_look_deg)?;
        crate::data::positive_range("bases.runway_m", [self.runway_m[1], self.runway_m[0]])?;
        positive("bases.runway_width_m", self.runway_width_m)?;
        positive("bases.runway_gap_m", self.runway_gap_m)?;
        positive("bases.runway_step_m", self.runway_step_m)?;
        positive("bases.runway_max_cut_m", self.runway_max_cut_m)?;
        positive("bases.runway_city_clear_m", self.runway_city_clear_m)?;
        if !(0.0..self.hangar_m.x).contains(&self.terminal_in_m.x) || !(0.0..self.hangar_m.z).contains(&self.terminal_in_m.y) {
            return Err("bases.terminal_in_m: the terminal must stand inside its bay".into());
        }
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
    /// Its runway (survey W45).
    pub runway: Runway,
}

/// Where a runway lies beside its base's site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunwaySide {
    /// In front of the apron (low z), along x: where it is put when it fits.
    Front,
    /// Behind the hangars (high z), along x.
    Back,
    /// Down the low-x side, along z.
    West,
    /// Down the high-x side, along z.
    East,
}

/// A base's runway: a straight paved strip, its centre line from `a` to `b` (x, z), flattened to its base's ground
/// and flown either way (survey W45). Along x or along z, so its footprint is a `Rect` the ground flattens like a
/// site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Runway {
    /// One end of the centre line (x, z): the west or north end.
    pub a: Vec2,
    /// The other end (x, z).
    pub b: Vec2,
    /// Paved width, metres.
    pub width_m: f32,
    /// Which side of the site it lies on.
    pub side: RunwaySide,
}

impl Runway {
    /// The runway on `side` of a site centred on `centre` with half size `half`, `length_m` long, slid `shift_m`
    /// along its line, `cfg.runway_gap_m` out from the site's edge.
    pub fn beside(centre: Vec2, half: Vec2, side: RunwaySide, length_m: f32, shift_m: f32, cfg: &BasesConfig) -> Self {
        let out = cfg.runway_gap_m + cfg.runway_width_m * 0.5;
        let (mid, along) = match side {
            RunwaySide::Front => (Vec2::new(centre.x + shift_m, centre.y - half.y - out), Vec2::X),
            RunwaySide::Back => (Vec2::new(centre.x + shift_m, centre.y + half.y + out), Vec2::X),
            RunwaySide::West => (Vec2::new(centre.x - half.x - out, centre.y + shift_m), Vec2::Y),
            RunwaySide::East => (Vec2::new(centre.x + half.x + out, centre.y + shift_m), Vec2::Y),
        };
        Self {
            a: mid - along * length_m * 0.5,
            b: mid + along * length_m * 0.5,
            width_m: cfg.runway_width_m,
            side,
        }
    }

    /// Its length, metres.
    pub fn length_m(&self) -> f32 {
        self.a.distance(self.b)
    }

    /// Its direction from `a` to `b`, a unit vector (x, z).
    pub fn dir(&self) -> Vec2 {
        (self.b - self.a).normalize_or(Vec2::X)
    }

    /// Its paved footprint.
    pub fn rect(&self) -> Rect {
        let side = self.dir().perp().abs() * self.width_m * 0.5;
        Rect {
            min: self.a.min(self.b) - side,
            max: self.a.max(self.b) + side,
        }
    }

    /// The taxiway joining it to its site, half the runway's width, straight across the gap from the site's edge to
    /// the runway's edge at the runway's middle: its footprint, touching both and overlapping neither.
    pub fn taxiway(&self, site: &Rect) -> Rect {
        let mid = (self.a + self.b) * 0.5;
        let r = self.rect();
        let q = self.width_m * 0.25;
        match self.side {
            RunwaySide::Front => Rect {
                min: Vec2::new(mid.x - q, r.max.y),
                max: Vec2::new(mid.x + q, site.min.y),
            },
            RunwaySide::Back => Rect {
                min: Vec2::new(mid.x - q, site.max.y),
                max: Vec2::new(mid.x + q, r.min.y),
            },
            RunwaySide::West => Rect {
                min: Vec2::new(r.max.x, mid.y - q),
                max: Vec2::new(site.min.x, mid.y + q),
            },
            RunwaySide::East => Rect {
                min: Vec2::new(site.max.x, mid.y - q),
                max: Vec2::new(r.min.x, mid.y + q),
            },
        }
    }

    /// Points over its footprint every `step_m` along it, on its centre line and both edges.
    pub fn samples(&self, step_m: f32) -> Vec<Vec2> {
        let n = (self.length_m() / step_m.max(1.0)).ceil().max(1.0) as usize;
        let side = self.dir().perp() * self.width_m * 0.5;
        (0..=n)
            .flat_map(|i| {
                let p = self.a + (self.b - self.a) * (i as f32 / n as f32);
                [p - side, p, p + side]
            })
            .collect()
    }
}

/// The space between two hangar bays, metres.
pub const HANGAR_GAP_M: f32 = 12.0;

/// How far a hangar's back wall stands in from the base's back edge, metres.
const HANGAR_BACK_M: f32 = 30.0;

/// A hangar's walls and roof stand this thick outside its inside, metres.
pub const HANGAR_WALL_M: f32 = 1.5;

/// The control tower's half width at its cab (the widest part), metres.
const TOWER_HALF_M: f32 = 9.0;

/// The control tower stands this far in from the base's front corner (high x, low z), metres.
const TOWER_IN_M: f32 = 40.0;

impl AirBase {
    /// Its footprint.
    pub fn rect(&self) -> Rect {
        Rect::centred(self.centre, self.half_m)
    }

    /// Its control tower's footprint at its widest (the cab), by the front corner of the apron.
    pub fn tower(&self) -> Rect {
        let r = self.rect();
        Rect::centred(Vec2::new(r.max.x - TOWER_IN_M, r.min.y + TOWER_IN_M), Vec2::splat(TOWER_HALF_M))
    }

    /// Everything built on the site that stands up off the apron (x, z): each hangar bay with its walls and roof,
    /// and the control tower. The one list of what a body must not be put inside (`air_drop`).
    pub fn structures(&self, cfg: &BasesConfig) -> Vec<Rect> {
        let mut out: Vec<Rect> = self
            .hangars(cfg)
            .into_iter()
            .map(|h| Rect {
                min: h.min - Vec2::new(HANGAR_WALL_M, 0.0),
                max: h.max + Vec2::splat(HANGAR_WALL_M),
            })
            .collect();
        out.push(self.tower());
        out
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

/// A terminal: the screen the pilot walks up to on foot in a hangar bay (air-bases design section 5), where
/// jobs are signed for, mechs bought and the mech in the bay refitted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terminal {
    /// Its base.
    pub base: u32,
    /// Its bay at that base.
    pub bay: usize,
    /// Where it stands (x, z).
    pub at: Vec2,
}

impl AirBase {
    /// Its terminals, one a bay, each just inside the bay's mouth by its left (low x) wall.
    pub fn terminals(&self, cfg: &BasesConfig) -> Vec<Terminal> {
        self.hangars(cfg)
            .iter()
            .enumerate()
            .map(|(bay, h)| Terminal {
                base: self.id,
                bay,
                at: Vec2::new(h.min.x + cfg.terminal_in_m.x, h.min.y + cfg.terminal_in_m.y),
            })
            .collect()
    }
}

/// The terminal a pilot on foot at `feet` (x, z), looking along `yaw_rad` (0 faces -z, left positive), can
/// use: within reach and in front of them; the nearest if more than one.
pub fn terminal_in_reach(bases: &[AirBase], cfg: &BasesConfig, feet: Vec2, yaw_rad: f32) -> Option<Terminal> {
    let look = Vec2::new(-yaw_rad.sin(), -yaw_rad.cos());
    let cone = cfg.terminal_look_deg.to_radians().cos();
    bases
        .iter()
        .filter(|b| b.rect().distance(feet) <= cfg.terminal_reach_m)
        .flat_map(|b| b.terminals(cfg))
        .filter(|t| {
            let to = t.at - feet;
            let d = to.length();
            d <= cfg.terminal_reach_m && (d < 0.3 || to.dot(look) / d >= cone)
        })
        .min_by(|a, b| a.at.distance(feet).total_cmp(&b.at.distance(feet)))
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
                let Some(ground_m) = site_ground(shape, cities, c, cfg, min_ground_m) else {
                    continue;
                };
                if let Some(runway) = runway_for(shape, cities, clear, c, ground_m, cfg, min_ground_m) {
                    candidates.push(AirBase {
                        id: 0,
                        centre: c,
                        half_m: cfg.half_m,
                        ground_m,
                        highway: hi as u32,
                        runway,
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

/// The runway a site centred on `c` with ground `ground_m` can hold (survey W45): in front of the apron, behind the
/// hangars, then down the west and east sides; at each, slid along its line by none, then a step either way, then two;
/// longest first, from `cfg.runway_m[0]` down to `cfg.runway_m[1]`. A runway fits when every point over it is on dry
/// land above `min_ground_m` and within `runway_max_cut_m` of the site's ground, it keeps `runway_city_clear_m` from
/// every city and clear of the highways and the rivers (`keeps_clear`). It stands off the site by `runway_gap_m`, so it
/// never meets a hangar or the tower. `None` when nothing fits.
fn runway_for(
    shape: &Shape,
    cities: &[City],
    clear: &KeepClear,
    c: Vec2,
    ground_m: f32,
    cfg: &BasesConfig,
    min_ground_m: f32,
) -> Option<Runway> {
    let [longest, shortest] = cfg.runway_m;
    let lengths: Vec<f32> = (0..)
        .map(|i| longest - i as f32 * cfg.runway_step_m)
        .take_while(|l| *l >= shortest - 1e-3)
        .collect();
    let shifts = [0.0, -1.0, 1.0, -2.0, 2.0].map(|k| k * cfg.runway_step_m);
    for side in [RunwaySide::Front, RunwaySide::Back, RunwaySide::West, RunwaySide::East] {
        for &len in &lengths {
            for shift in shifts {
                let r = Runway::beside(c, cfg.half_m, side, len, shift, cfg);
                if runway_fits(shape, cities, clear, &r, ground_m, cfg, min_ground_m) {
                    return Some(r);
                }
            }
        }
    }
    None
}

fn runway_fits(shape: &Shape, cities: &[City], clear: &KeepClear, r: &Runway, ground_m: f32, cfg: &BasesConfig, min_ground_m: f32) -> bool {
    let pts = r.samples(50.0);
    let dry = pts.iter().all(|p| {
        let h = shape.height(*p);
        shape.past_land_edge_m(*p) < 0.0 && h >= min_ground_m && (h - ground_m).abs() <= cfg.runway_max_cut_m
    });
    dry && pts
        .iter()
        .all(|p| cities.iter().all(|city| city.rect().distance(*p) > cfg.runway_city_clear_m))
        && pts.iter().all(|p| clear.highways.iter().all(|h| h.project(*p).0 > clear.road_m))
        && keeps_clear(&r.rect(), clear)
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
            runway: Runway::beside(Vec2::new(1000.0, -2000.0), cfg.half_m, RunwaySide::Front, 1800.0, 0.0, &cfg),
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

    #[test]
    fn a_pilot_can_use_a_terminal_only_standing_at_it_and_facing_it() {
        let cfg = BasesConfig::default();
        let base = AirBase {
            id: 1,
            centre: Vec2::new(0.0, 0.0),
            half_m: cfg.half_m,
            ground_m: 10.0,
            highway: 0,
            runway: Runway::beside(Vec2::ZERO, cfg.half_m, RunwaySide::Front, 1800.0, 0.0, &cfg),
        };
        let t = base.terminals(&cfg)[2];
        assert_eq!((t.base, t.bay), (1, 2));
        assert!(base.hangars(&cfg)[2].distance(t.at) == 0.0, "it stands inside its bay");
        // Two metres in front of it (toward -z, the apron), facing +z (yaw pi): in reach.
        let feet = t.at - Vec2::new(0.0, 2.0);
        assert_eq!(terminal_in_reach(&[base], &cfg, feet, std::f32::consts::PI), Some(t));
        assert_eq!(terminal_in_reach(&[base], &cfg, feet, 0.0), None, "with its back to it");
        assert_eq!(
            terminal_in_reach(&[base], &cfg, t.at - Vec2::new(0.0, 10.0), std::f32::consts::PI),
            None,
            "too far"
        );
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
