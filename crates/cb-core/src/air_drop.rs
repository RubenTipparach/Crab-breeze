//! The air drop (openspec change `mech-shop`, design section 6; owner, 2026-10-26: "when a player orders a mech it
//! should be air dropped close to wherever the player is on a good are on the ground (not stuck on a building)"):
//! where a bought mech lands near the pilot (`drop_site`), on ground good enough to take it (`good_spot`); the drop's
//! clock from the order through the aircraft's pass, the fall under its canopies and the retros to the touchdown
//! (`Drop`, `fall`); keeping it off a pilot who walks into the site (`clear_of`); and which owned mech the pilot on
//! foot is walking up to, the one that kneels for them (`walking_up_to`). Pure: the
//! world is read through `DropGround`, which the generated `WorldPlan` answers, so the rules are tested on ground made
//! up for the test as well as on the island (`PlanGround`).

use crate::data::{Validate, positive};
use crate::mech::MechConfig;
use crate::rng::SimRng;
use crate::world::WorldPlan;
use crate::world::bases::BasesConfig;
use crate::world::cover::Surface;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// How a drop picks its site and comes down, `data/career/air_drop.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DropConfig {
    /// The nearest a mech lands to the pilot, metres: far enough that its fall and dust miss them.
    pub ring_min_m: f32,
    /// The farthest of the ring searched first, metres: near enough to walk to.
    pub ring_max_m: f32,
    /// The search goes on past the ring out to this, metres; with nothing good by then the mech goes to a bay.
    pub search_max_m: f32,
    /// Rings are searched this far apart, metres.
    pub ring_step_m: f32,
    /// Spots on a ring are about this far apart, metres.
    pub spot_spacing_m: f32,
    /// Ground kept clear between the mech's footprint and any building, hangar, tower or rock, metres.
    pub building_clear_m: f32,
    /// Ground kept clear between the mech's footprint and another mech's, metres.
    pub mech_clear_m: f32,
    /// The steepest ground a mech lands on, degrees.
    pub max_slope_deg: f32,
    /// A paved spot (a road, a city street or plaza, a base's apron) counts this much nearer, metres.
    pub paved_bonus_m: f32,
    /// A spot in water (no deeper than the mech's skim depth) counts this much farther, metres.
    pub wet_penalty_m: f32,
    /// Seconds from the order to the release over the site.
    pub delay_s: f32,
    /// The aircraft's speed, m/s.
    pub aircraft_speed_m_s: f32,
    /// The aircraft flies over the site this high above its ground and releases the mech there, metres.
    pub release_height_m: f32,
    /// The aircraft is drawn from this long before the release to as long after, seconds.
    pub aircraft_shown_s: f32,
    /// The aircraft: a civil air traffic kind (`data/world/air_traffic.ron`), drawn and heard as that kind is.
    pub aircraft_kind: String,
    /// Seconds of free fall before the canopies open.
    pub free_fall_s: f32,
    /// The descent under the canopies, m/s.
    pub canopy_m_s: f32,
    /// The retros fire this high above the ground, metres.
    pub retro_height_m: f32,
    /// The speed the retros slow it to at touchdown, m/s.
    pub touchdown_m_s: f32,
    /// While it falls, the site keeps this far beyond the mech's footprint from the pilot (or the mech they sit
    /// in), metres.
    pub pilot_clear_m: f32,
    /// The canopies steer it aside at up to this, m/s.
    pub steer_m_s: f32,
    /// The pilot on foot boards the owned mech they walk up to within this of its feet, metres.
    pub walk_up_m: f32,
}

impl Default for DropConfig {
    fn default() -> Self {
        Self {
            ring_min_m: 40.0,
            ring_max_m: 150.0,
            search_max_m: 600.0,
            ring_step_m: 10.0,
            spot_spacing_m: 15.0,
            building_clear_m: 6.0,
            mech_clear_m: 10.0,
            max_slope_deg: 12.0,
            paved_bonus_m: 30.0,
            wet_penalty_m: 300.0,
            delay_s: 25.0,
            aircraft_speed_m_s: 95.0,
            release_height_m: 320.0,
            aircraft_shown_s: 20.0,
            aircraft_kind: "freighter".into(),
            free_fall_s: 2.0,
            canopy_m_s: 14.0,
            retro_height_m: 25.0,
            touchdown_m_s: 3.0,
            pilot_clear_m: 15.0,
            steer_m_s: 6.0,
            walk_up_m: 40.0,
        }
    }
}

impl Validate for DropConfig {
    fn validate(&self) -> Result<(), String> {
        positive("ring_min_m", self.ring_min_m)?;
        positive("ring_step_m", self.ring_step_m)?;
        positive("spot_spacing_m", self.spot_spacing_m)?;
        positive("max_slope_deg", self.max_slope_deg)?;
        if self.ring_max_m < self.ring_min_m || self.search_max_m < self.ring_max_m {
            return Err("ring_min_m, ring_max_m, search_max_m: each at least the one before".into());
        }
        if self.search_max_m / self.ring_step_m > 1000.0 {
            return Err("search_max_m / ring_step_m: more than 1000 rings".into());
        }
        crate::data::within("max_slope_deg", self.max_slope_deg, 0.0, 60.0)?;
        if self.aircraft_kind.is_empty() {
            return Err("aircraft_kind: must name a civil aircraft kind".into());
        }
        for (field, v) in [
            ("delay_s", self.delay_s),
            ("aircraft_speed_m_s", self.aircraft_speed_m_s),
            ("release_height_m", self.release_height_m),
            ("aircraft_shown_s", self.aircraft_shown_s),
            ("canopy_m_s", self.canopy_m_s),
            ("retro_height_m", self.retro_height_m),
            ("touchdown_m_s", self.touchdown_m_s),
            ("pilot_clear_m", self.pilot_clear_m),
            ("steer_m_s", self.steer_m_s),
            ("walk_up_m", self.walk_up_m),
        ] {
            positive(field, v)?;
        }
        crate::data::finite("free_fall_s", self.free_fall_s)?;
        if self.free_fall_s < 0.0 {
            return Err("free_fall_s: must not be negative".into());
        }
        if self.touchdown_m_s > self.canopy_m_s {
            return Err("touchdown_m_s: the retros slow it, so no faster than canopy_m_s".into());
        }
        if free_fall_m(self.free_fall_s) + self.retro_height_m >= self.release_height_m {
            return Err("release_height_m: must be above the free fall and the retros' height".into());
        }
        for (field, v) in [
            ("building_clear_m", self.building_clear_m),
            ("mech_clear_m", self.mech_clear_m),
            ("paved_bonus_m", self.paved_bonus_m),
            ("wet_penalty_m", self.wet_penalty_m),
        ] {
            crate::data::finite(field, v)?;
            if v < 0.0 {
                return Err(format!("{field}: must not be negative"));
            }
        }
        Ok(())
    }
}

/// The ground as a drop sees it.
pub trait DropGround {
    /// The ground's height at `p` (x, z), metres.
    fn height_m(&self, p: Vec2) -> f32;
    /// How deep the water over the ground at `p` is, metres; zero when dry.
    fn water_depth_m(&self, p: Vec2) -> f32;
    /// True when a circle of `radius_m` at `p` overlaps anything built or standing on the ground: a building, a
    /// hangar, a tower, a rock.
    fn blocked(&self, p: Vec2, radius_m: f32) -> bool;
    /// True when `p` is paved: a road, a street, a plaza, an apron.
    fn paved(&self, p: Vec2) -> bool;
}

/// What the mech needs of its ground, from its own data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    /// Its footprint's radius, metres (`locomotion.body_radius_m`).
    pub radius_m: f32,
    /// The deepest water it lands in, metres: its skim depth, the lesser of its wading depth and the depth at
    /// which its skates would ride the water.
    pub max_depth_m: f32,
}

impl Needs {
    /// A mech's needs, from its data.
    pub fn of(mech: &MechConfig) -> Self {
        let l = &mech.locomotion;
        Self {
            radius_m: l.body_radius_m,
            max_depth_m: l.max_wade_depth_m.min(l.skim_depth_share * l.body_height_m),
        }
    }
}

/// What a site keeps clear of: the pilot, and every mech already standing (feet x, z, and footprint radius).
#[derive(Clone, Copy, Debug)]
pub struct Keep<'a> {
    /// Where the pilot is: their feet on foot, their mech's feet in the seat.
    pub pilot: Vec2,
    /// The mechs standing, each with its footprint radius, metres.
    pub mechs: &'a [(Vec2, f32)],
}

/// A spot good enough to land on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Where (x, ground height, z), metres.
    pub at: Vec3,
    /// On paving.
    pub paved: bool,
    /// In water (no deeper than the mech's skim depth).
    pub wet: bool,
}

/// Is `p` good ground for a mech with `needs`, clear of everything in `keep` but the pilot: not overlapping
/// anything built with its footprint and the clearance, no deeper in water than it lands in, no steeper across its
/// footprint than the limit, and clear of every other mech. The pilot's own distance is the search's to keep.
pub fn good_spot(cfg: &DropConfig, ground: &dyn DropGround, needs: Needs, keep: &Keep, p: Vec2) -> Option<Spot> {
    if !p.is_finite() {
        return None;
    }
    let r = needs.radius_m;
    if ground.blocked(p, r + cfg.building_clear_m) {
        return None;
    }
    // The slope: the rise between opposite points on the footprint's rim, the steepest of four directions; the
    // water: the deepest of the centre and those eight points.
    let h = ground.height_m(p);
    let limit = cfg.max_slope_deg.to_radians().tan() * 2.0 * r;
    let mut depth = ground.water_depth_m(p);
    for dir in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize(), Vec2::new(1.0, -1.0).normalize()] {
        let rise = (ground.height_m(p + dir * r) - ground.height_m(p - dir * r)).abs();
        if !rise.is_finite() || rise > limit {
            return None;
        }
        depth = depth.max(ground.water_depth_m(p + dir * r)).max(ground.water_depth_m(p - dir * r));
    }
    if !depth.is_finite() || depth > needs.max_depth_m {
        return None;
    }
    if keep.mechs.iter().any(|(m, mr)| m.distance(p) < r + mr + cfg.mech_clear_m) {
        return None;
    }
    Some(Spot {
        at: Vec3::new(p.x, h, p.y),
        paved: ground.paved(p),
        wet: depth > 0.0,
    })
}

/// A good spot's score: its distance from the pilot, a paved one nearer, a wet one farther. Lowest wins.
fn score(cfg: &DropConfig, pilot: Vec2, s: &Spot) -> f32 {
    let d = Vec2::new(s.at.x, s.at.z).distance(pilot);
    d - if s.paved { cfg.paved_bonus_m } else { 0.0 } + if s.wet { cfg.wet_penalty_m } else { 0.0 }
}

/// Where a bought mech lands (design section 6.1): rings outward from the pilot, `ring_step_m` apart from
/// `ring_min_m`, spots about `spot_spacing_m` apart on each, the first spot on each ring at an angle drawn from
/// `rng`. Every good spot inside the ring (to `ring_max_m`) is scored, and the best wins, ties to the spot tried
/// first; only when the ring holds none does the search go on, a ring at a time, and the first ring with a good
/// spot gives the site. `None` when nothing out to `search_max_m` is good: the mech goes to a bay instead.
pub fn drop_site(cfg: &DropConfig, ground: &dyn DropGround, needs: Needs, keep: &Keep, rng: &SimRng) -> Option<Spot> {
    if !keep.pilot.is_finite() {
        return None;
    }
    let rings = ((cfg.search_max_m - cfg.ring_min_m) / cfg.ring_step_m).floor().max(0.0) as usize;
    let mut best: Option<(f32, Spot)> = None;
    for i in 0..=rings {
        let r = cfg.ring_min_m + i as f32 * cfg.ring_step_m;
        let mut ring_rng = rng.fork(&format!("ring/{i}"));
        let phase = ring_rng.range_f32(0.0, std::f32::consts::TAU);
        let n = ((std::f32::consts::TAU * r / cfg.spot_spacing_m).ceil() as usize).clamp(8, 4096);
        for k in 0..n {
            let a = phase + std::f32::consts::TAU * k as f32 / n as f32;
            let p = keep.pilot + Vec2::new(a.cos(), a.sin()) * r;
            let Some(s) = good_spot(cfg, ground, needs, keep, p) else {
                continue;
            };
            let sc = score(cfg, keep.pilot, &s);
            if best.as_ref().is_none_or(|(b, _)| sc < *b) {
                best = Some((sc, s));
            }
        }
        // Inside the ring every spot competes; past it, the first ring with a good spot ends the search.
        if best.is_some() && r + cfg.ring_step_m > cfg.ring_max_m {
            break;
        }
    }
    best.map(|(_, s)| s)
}

/// Gravity, m/s/s.
const G_M_S2: f32 = 9.81;

/// How far a body falls freely in `t_s` seconds from rest, metres.
fn free_fall_m(t_s: f32) -> f32 {
    0.5 * G_M_S2 * t_s * t_s
}

/// Where in its fall a dropped mech is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallStage {
    /// Out of the aircraft, the canopies not yet open.
    FreeFall,
    /// Under its canopies.
    Canopies,
    /// Its retros firing over the ground.
    Retros,
    /// On the ground.
    Down,
}

/// A dropped mech's fall at a moment: how high its feet are over the site's ground, how fast it falls, and the stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    /// Its feet over the site's ground, metres.
    pub height_m: f32,
    /// Its falling speed, m/s (downward positive).
    pub speed_m_s: f32,
    /// The stage.
    pub stage: FallStage,
}

/// The fall `t_s` seconds after the release (design section 6.2): free fall for `free_fall_s`, then the canopies at
/// `canopy_m_s` down to `retro_height_m`, then the retros slowing it evenly to `touchdown_m_s` at the ground.
pub fn fall(cfg: &DropConfig, t_s: f32) -> Fall {
    let t = if t_s.is_finite() { t_s.max(0.0) } else { 0.0 };
    let h0 = cfg.release_height_m;
    if t < cfg.free_fall_s {
        return Fall {
            height_m: h0 - free_fall_m(t),
            speed_m_s: G_M_S2 * t,
            stage: FallStage::FreeFall,
        };
    }
    let opened_m = h0 - free_fall_m(cfg.free_fall_s);
    let canopy_s = (opened_m - cfg.retro_height_m) / cfg.canopy_m_s;
    let t = t - cfg.free_fall_s;
    if t < canopy_s {
        return Fall {
            height_m: opened_m - cfg.canopy_m_s * t,
            speed_m_s: cfg.canopy_m_s,
            stage: FallStage::Canopies,
        };
    }
    let (v0, v1, h) = (cfg.canopy_m_s, cfg.touchdown_m_s, cfg.retro_height_m);
    let retro_s = 2.0 * h / (v0 + v1);
    let t = t - canopy_s;
    if t < retro_s {
        let a = (v0 - v1) / retro_s;
        return Fall {
            height_m: h - (v0 * t - 0.5 * a * t * t),
            speed_m_s: v0 - a * t,
            stage: FallStage::Retros,
        };
    }
    Fall {
        height_m: 0.0,
        speed_m_s: 0.0,
        stage: FallStage::Down,
    }
}

/// Seconds from the release to the touchdown.
pub fn fall_s(cfg: &DropConfig) -> f32 {
    let opened_m = cfg.release_height_m - free_fall_m(cfg.free_fall_s);
    cfg.free_fall_s + (opened_m - cfg.retro_height_m) / cfg.canopy_m_s + 2.0 * cfg.retro_height_m / (cfg.canopy_m_s + cfg.touchdown_m_s)
}

/// A drop under way, kept on the career (`Career::drops`) so a save resumes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drop {
    /// The owned mech's key.
    pub key: u32,
    /// Where it lands (x, ground height, z), metres: fixed at the order, moved aside only for the pilot.
    pub site: [f32; 3],
    /// The career's clock at the order, seconds.
    pub ordered_s: f64,
    /// The aircraft's track over the site, radians (0 flies toward -z, left positive), drawn from the seed.
    pub track_rad: f32,
    /// Landed, and waiting for the pilot to board it: the HUD keeps its marker until they do (owner, survey J16).
    #[serde(default)]
    pub landed: bool,
}

/// Where a drop is on its clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// The aircraft is on its way; the release is this many seconds off.
    Inbound(f32),
    /// Released and falling.
    Falling(Fall),
    /// On the ground.
    Landed,
}

impl Drop {
    /// A drop of mech `key` ordered at `now_s` to land at `site`, on a track drawn from `rng`.
    pub fn new(key: u32, site: Vec3, now_s: f64, rng: &SimRng) -> Self {
        let mut r = rng.fork(&format!("drop/{key}/track"));
        Self {
            key,
            site: site.to_array(),
            ordered_s: now_s,
            track_rad: r.range_f32(0.0, std::f32::consts::TAU),
            landed: false,
        }
    }

    /// The site (x, z).
    pub fn site_xz(&self) -> Vec2 {
        Vec2::new(self.site[0], self.site[2])
    }

    /// Seconds since the release at career time `now_s` (negative before it).
    pub fn since_release_s(&self, cfg: &DropConfig, now_s: f64) -> f32 {
        (now_s - self.ordered_s) as f32 - cfg.delay_s
    }

    /// Its stage at career time `now_s`.
    pub fn stage(&self, cfg: &DropConfig, now_s: f64) -> Stage {
        if self.landed {
            return Stage::Landed;
        }
        let t = self.since_release_s(cfg, now_s);
        if t < 0.0 {
            return Stage::Inbound(-t);
        }
        let f = fall(cfg, t);
        if f.stage == FallStage::Down {
            Stage::Landed
        } else {
            Stage::Falling(f)
        }
    }

    /// The aircraft's position at career time `now_s`, while it is shown: on its track at `release_height_m` over the
    /// site's ground, over the site at the release.
    pub fn aircraft(&self, cfg: &DropConfig, now_s: f64) -> Option<Vec3> {
        let t = self.since_release_s(cfg, now_s);
        if t.abs() > cfg.aircraft_shown_s {
            return None;
        }
        let along = Vec3::new(-self.track_rad.sin(), 0.0, -self.track_rad.cos());
        Some(Vec3::from_array(self.site) + Vec3::Y * cfg.release_height_m + along * cfg.aircraft_speed_m_s * t)
    }
}

/// Where a drop comes down when someone is in the way (design section 6.2): `site` itself when `body` (the pilot,
/// or the mech they sit in, with `body_radius_m`) is at least the mech's footprint (`needs`) plus `pilot_clear_m` and
/// `body_radius_m` away; else the good spot (`good`) on that circle round `body` nearest `site`, tried every 10
/// degrees; else straight out from `body` to the circle's edge, just beside them. It never lands on them.
pub fn clear_of(cfg: &DropConfig, needs: Needs, site: Vec2, body: Vec2, body_radius_m: f32, good: &dyn Fn(Vec2) -> bool) -> Vec2 {
    let clear = needs.radius_m + cfg.pilot_clear_m + body_radius_m;
    if !(site.is_finite() && body.is_finite()) || site.distance(body) >= clear {
        return site;
    }
    let away = (site - body).normalize_or(Vec2::X);
    let edge = body + away * clear;
    (0..36)
        .map(|i| {
            let a = i as f32 * 10f32.to_radians();
            body + Vec2::from_angle(a).rotate(away) * clear
        })
        .filter(|p| good(*p))
        .min_by(|a, b| a.distance(site).total_cmp(&b.distance(site)))
        .unwrap_or(edge)
}

/// Moves `from` toward `to` by at most `max_m`.
pub fn steer(from: Vec2, to: Vec2, max_m: f32) -> Vec2 {
    let d = to - from;
    let len = d.length();
    if len <= max_m || len < 1e-6 {
        to
    } else {
        from + d / len * max_m.max(0.0)
    }
}

/// Which owned mech the pilot on foot is walking up to (design section 6.3): the nearest of `mechs` (key, feet x and
/// z) within `walk_up_m` of their feet, ties to the lower key. It is the one that kneels and lowers its rope.
pub fn walking_up_to(cfg: &DropConfig, feet: Vec2, mechs: &[(u32, Vec2)]) -> Option<u32> {
    mechs
        .iter()
        .filter(|(_, at)| at.distance(feet) <= cfg.walk_up_m)
        .min_by(|a, b| a.1.distance(feet).total_cmp(&b.1.distance(feet)).then(a.0.cmp(&b.0)))
        .map(|(k, _)| *k)
}

/// The island as a drop sees it: the generated world and its bases' layout (the hangars' size is data the plan
/// does not keep).
#[derive(Clone, Copy)]
pub struct PlanGround<'a> {
    /// The generated world.
    pub plan: &'a WorldPlan,
    /// `bases:` in `data/world/world.ron`.
    pub bases: &'a BasesConfig,
}

impl DropGround for PlanGround<'_> {
    fn height_m(&self, p: Vec2) -> f32 {
        self.plan.terrain.height_at(p.x, p.y)
    }

    fn water_depth_m(&self, p: Vec2) -> f32 {
        self.plan
            .waters
            .water_at(p, self.plan.terrain.height_at(p.x, p.y))
            .map_or(0.0, |w| w.depth_m)
    }

    /// Every building's footprint (the one collision grid), every base's hangars with their walls and its tower,
    /// and every rock's feet.
    fn blocked(&self, p: Vec2, radius_m: f32) -> bool {
        if self.plan.collision.overlaps(p, radius_m) {
            return true;
        }
        if self
            .plan
            .bases
            .iter()
            .filter(|b| b.rect().distance(p) < radius_m)
            .flat_map(|b| b.structures(self.bases))
            .any(|s| s.distance(p) < radius_m)
        {
            return true;
        }
        self.plan
            .rocks
            .iter()
            .flat_map(|r| r.feet())
            .any(|(c, rr)| c.distance(p) < radius_m + rr)
    }

    /// The world's one cover rule (`WorldPlan::surface_at`): a highway, a city outside its parks, a base's site.
    fn paved(&self, p: Vec2) -> bool {
        self.plan.surface_at(p) == Surface::Concrete
    }
}

impl DropConfig {
    /// The aircraft kind it names is one civil air traffic flies (`data/world/air_traffic.ron`).
    pub fn check_aircraft(&self, air: &crate::air_traffic::AirTrafficConfig) -> Result<(), String> {
        if air.kinds.contains_key(&self.aircraft_kind) {
            Ok(())
        } else {
            Err(format!(
                "aircraft_kind: \"{}\" is not a kind in world/air_traffic.ron",
                self.aircraft_kind
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::Rect;

    #[test]
    fn the_drop_flies_civil_air_traffics_freighter() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let drop: DropConfig = crate::data::load_ron(&root.join("career/air_drop.ron")).unwrap();
        let air: crate::air_traffic::AirTrafficConfig = crate::data::load_ron(&root.join("world/air_traffic.ron")).unwrap();
        assert_eq!(drop.aircraft_kind, "freighter");
        assert_eq!(drop.check_aircraft(&air), Ok(()));
        let wrong = DropConfig {
            aircraft_kind: "zeppelin".into(),
            ..drop
        };
        assert!(wrong.check_aircraft(&air).is_err());
    }

    /// Ground made up for a test: flat at zero unless a slope or water says otherwise.
    #[derive(Default)]
    struct Made {
        buildings: Vec<Rect>,
        /// Water this deep over each rectangle.
        water: Vec<(Rect, f32)>,
        /// Ground rising at this gradient (rise over run) along +x inside each rectangle.
        slopes: Vec<(Rect, f32)>,
        paved: Vec<Rect>,
    }

    impl DropGround for Made {
        fn height_m(&self, p: Vec2) -> f32 {
            self.slopes
                .iter()
                .find(|(r, _)| r.distance(p) == 0.0)
                .map_or(0.0, |(r, g)| (p.x - r.min.x) * g)
        }
        fn water_depth_m(&self, p: Vec2) -> f32 {
            self.water
                .iter()
                .filter(|(r, _)| r.distance(p) == 0.0)
                .map(|(_, d)| *d)
                .fold(0.0, f32::max)
        }
        fn blocked(&self, p: Vec2, radius_m: f32) -> bool {
            self.buildings.iter().any(|b| b.distance(p) < radius_m)
        }
        fn paved(&self, p: Vec2) -> bool {
            self.paved.iter().any(|r| r.distance(p) == 0.0)
        }
    }

    /// A Foreman's needs (`data/mechs/foreman.ron`: 7.56 m footprint, 12 m skim depth).
    const NEEDS: Needs = Needs {
        radius_m: 7.56,
        max_depth_m: 12.0,
    };

    fn cfg() -> DropConfig {
        crate::data::load_ron(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/air_drop.ron")).unwrap()
    }

    fn keep(pilot: Vec2) -> Keep<'static> {
        Keep { pilot, mechs: &[] }
    }

    /// A dense block: 40 m towers on a 52 m pitch (12 m streets, too narrow for a mech) for 600 m round the
    /// pilot, with one 120 m plaza 260 m east of them.
    fn dense_city() -> Made {
        let mut g = Made::default();
        let plaza = Rect::centred(Vec2::new(260.0, 0.0), Vec2::splat(60.0));
        for i in -12..=12 {
            for j in -12..=12 {
                let c = Vec2::new(i as f32 * 52.0, j as f32 * 52.0);
                let b = Rect::centred(c, Vec2::splat(20.0));
                if !b.overlaps(&plaza) {
                    g.buildings.push(b);
                }
            }
        }
        g.paved.push(Rect::centred(Vec2::ZERO, Vec2::splat(700.0)));
        g
    }

    #[test]
    fn in_a_dense_city_block_the_site_is_never_in_a_building() {
        let c = cfg();
        let g = dense_city();
        let pilot = Vec2::new(26.0, 26.0); // a street crossing among the towers
        for seed in 0..16 {
            let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(seed)).expect("the plaza takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(
                !g.blocked(at, NEEDS.radius_m + c.building_clear_m),
                "seed {seed}: {at} is clear of every tower by the footprint and the clearance"
            );
            assert!(g.buildings.iter().all(|b| b.distance(at) >= NEEDS.radius_m));
        }
    }

    #[test]
    fn the_site_is_never_in_deep_water() {
        let c = cfg();
        // A lake 30 m deep everywhere but a dry strip 200 m east, 60 m wide.
        let strip = Rect::centred(Vec2::new(200.0, 0.0), Vec2::new(30.0, 800.0));
        let made = Made {
            water: vec![
                (Rect::centred(Vec2::new(-315.0, 0.0), Vec2::new(485.0, 800.0)), 30.0),
                (Rect::centred(Vec2::new(615.0, 0.0), Vec2::new(385.0, 800.0)), 30.0),
            ],
            ..Default::default()
        };
        for seed in 0..8 {
            let s = drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(seed)).expect("the strip takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(
                made.water_depth_m(at) <= NEEDS.max_depth_m,
                "seed {seed}: {at} is not in deep water"
            );
            assert!(strip.distance(at) == 0.0, "seed {seed}: {at} is on the dry strip");
        }
    }

    #[test]
    fn dry_ground_wins_over_shallow_water_in_the_ring() {
        let c = cfg();
        // Shallow water (2 m) everywhere: it lands in it, as the last resort.
        let shallow = Made {
            water: vec![(Rect::centred(Vec2::ZERO, Vec2::splat(800.0)), 2.0)],
            ..Default::default()
        };
        let s = drop_site(&c, &shallow, NEEDS, &keep(Vec2::ZERO), &SimRng::new(3)).unwrap();
        assert!(s.wet, "with no dry ground the shallow water takes it");
        // The same water with a dry island 140 m north, 50 m across: the dry ground wins though it is farther.
        let island = Rect::centred(Vec2::new(0.0, 140.0), Vec2::splat(25.0));
        let dry = Made {
            water: vec![
                (
                    Rect {
                        min: Vec2::splat(-800.0),
                        max: Vec2::new(800.0, 115.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(-800.0, 165.0),
                        max: Vec2::splat(800.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(-800.0, 115.0),
                        max: Vec2::new(-25.0, 165.0),
                    },
                    2.0,
                ),
                (
                    Rect {
                        min: Vec2::new(25.0, 115.0),
                        max: Vec2::new(800.0, 165.0),
                    },
                    2.0,
                ),
            ],
            ..Default::default()
        };
        let s = drop_site(&c, &dry, NEEDS, &keep(Vec2::ZERO), &SimRng::new(3)).unwrap();
        assert!(!s.wet, "{:?} is on the island", s.at);
        assert!(island.distance(Vec2::new(s.at.x, s.at.z)) == 0.0);
    }

    #[test]
    fn the_site_is_never_on_a_steep_slope() {
        let c = cfg();
        // A hillside rising at 1 in 2 (27 degrees) everywhere west of x = 120 m; level beyond.
        let made = Made {
            slopes: vec![(
                Rect {
                    min: Vec2::new(-800.0, -800.0),
                    max: Vec2::new(120.0, 800.0),
                },
                0.5,
            )],
            ..Default::default()
        };
        for seed in 0..8 {
            let s = drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(seed)).expect("the level ground takes it");
            let at = Vec2::new(s.at.x, s.at.z);
            assert!(at.x >= 120.0 + NEEDS.radius_m - 0.01, "seed {seed}: {at} is off the hillside");
        }
    }

    #[test]
    fn on_open_ground_the_site_is_always_inside_the_ring() {
        let c = cfg();
        let g = Made::default();
        for seed in 0..32 {
            let pilot = Vec2::new(seed as f32 * 37.0, -(seed as f32) * 11.0);
            let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(seed)).unwrap();
            let d = Vec2::new(s.at.x, s.at.z).distance(pilot);
            assert!(
                (c.ring_min_m - 0.01..=c.ring_max_m + 0.01).contains(&d),
                "seed {seed}: {d:.1} m from the pilot"
            );
        }
    }

    #[test]
    fn the_ring_holds_good_ground_so_it_lands_in_the_ring_even_when_far_off_is_paved() {
        let c = cfg();
        // One good spot inside the ring, wide open paved ground just past it.
        let mut g = Made::default();
        g.paved.push(Rect::centred(Vec2::new(400.0, 0.0), Vec2::splat(200.0)));
        let s = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(1)).unwrap();
        assert!(Vec2::new(s.at.x, s.at.z).length() <= c.ring_max_m + 0.01);
    }

    #[test]
    fn a_road_in_the_ring_is_preferred_to_open_ground_nearer() {
        let c = cfg();
        // A road 20 m wide, 45-65 m north of the pilot: paved, 30 m of bonus, beats open ground at 40 m.
        let mut g = Made::default();
        g.paved.push(Rect::centred(Vec2::new(0.0, 55.0), Vec2::new(800.0, 10.0)));
        let s = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(9)).unwrap();
        assert!(s.paved, "{:?} is on the road", s.at);
    }

    #[test]
    fn it_keeps_clear_of_other_mechs() {
        let c = cfg();
        let g = Made::default();
        let mechs: Vec<(Vec2, f32)> = (0..64)
            .map(|k| {
                let a = k as f32 / 64.0 * std::f32::consts::TAU;
                (Vec2::new(a.cos(), a.sin()) * 45.0, 7.56)
            })
            .collect();
        let k = Keep {
            pilot: Vec2::ZERO,
            mechs: &mechs,
        };
        let s = drop_site(&c, &g, NEEDS, &k, &SimRng::new(2)).unwrap();
        let at = Vec2::new(s.at.x, s.at.z);
        for (m, r) in &mechs {
            assert!(m.distance(at) >= NEEDS.radius_m + r + c.mech_clear_m - 0.01);
        }
    }

    #[test]
    fn with_no_good_ground_in_the_search_there_is_no_site() {
        let c = cfg();
        let made = Made {
            buildings: vec![Rect::centred(Vec2::ZERO, Vec2::splat(2000.0))],
            ..Default::default()
        };
        assert_eq!(drop_site(&c, &made, NEEDS, &keep(Vec2::ZERO), &SimRng::new(4)), None);
        let deep = Made {
            water: vec![(Rect::centred(Vec2::ZERO, Vec2::splat(2000.0)), 40.0)],
            ..Default::default()
        };
        assert_eq!(drop_site(&c, &deep, NEEDS, &keep(Vec2::ZERO), &SimRng::new(4)), None);
    }

    #[test]
    fn the_same_seed_gives_the_same_site_and_another_seed_another() {
        let c = cfg();
        let g = Made::default();
        let a = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(77)).unwrap();
        let b = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(77)).unwrap();
        assert_eq!(a, b);
        let other = drop_site(&c, &g, NEEDS, &keep(Vec2::ZERO), &SimRng::new(78)).unwrap();
        assert_ne!(a.at, other.at, "the seed turns the ring");
    }

    #[test]
    fn the_fall_frees_opens_retros_and_lands_slow_and_continuous() {
        let c = cfg();
        let total = fall_s(&c);
        let mut last = fall(&c, 0.0);
        assert_eq!(last.height_m, c.release_height_m);
        let mut t = 0.0;
        while t < total + 1.0 {
            t += 0.05;
            let f = fall(&c, t);
            assert!(f.height_m <= last.height_m + 1e-3, "it never rises");
            assert!(last.height_m - f.height_m < 2.0, "{t:.2} s: no jump");
            last = f;
        }
        assert_eq!(last.stage, FallStage::Down);
        let just_before = fall(&c, total - 0.01);
        assert_eq!(just_before.stage, FallStage::Retros);
        assert!(
            just_before.speed_m_s <= c.touchdown_m_s + 0.2,
            "it touches down at the retros' speed"
        );
        assert!((20.0..30.0).contains(&total), "about 25 s from the release: {total:.1} s");
    }

    #[test]
    fn a_drop_is_inbound_then_falling_then_landed_and_the_aircraft_passes_over_the_site() {
        let c = cfg();
        let d = Drop::new(3, Vec3::new(100.0, 12.0, -40.0), 1000.0, &SimRng::new(5));
        assert_eq!(d, Drop::new(3, Vec3::new(100.0, 12.0, -40.0), 1000.0, &SimRng::new(5)), "seeded");
        assert!(matches!(d.stage(&c, 1000.0), Stage::Inbound(s) if (s - c.delay_s).abs() < 1e-3));
        assert!(matches!(d.stage(&c, 1000.0 + f64::from(c.delay_s) + 1.0), Stage::Falling(_)));
        assert_eq!(d.stage(&c, 1000.0 + f64::from(c.delay_s + fall_s(&c)) + 0.1), Stage::Landed);
        let over = d.aircraft(&c, 1000.0 + f64::from(c.delay_s)).unwrap();
        assert!((over - Vec3::new(100.0, 12.0 + c.release_height_m, -40.0)).length() < 1e-3);
        assert_eq!(d.aircraft(&c, 1000.0), None, "not shown 25 s out");
        let back = ron::from_str::<Drop>(&ron::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d, "a drop survives a save");
    }

    #[test]
    fn a_pilot_in_the_site_moves_it_beside_them_never_onto_them() {
        let c = cfg();
        let open = |_: Vec2| true;
        let site = Vec2::new(50.0, 0.0);
        assert_eq!(
            clear_of(&c, NEEDS, site, Vec2::new(-80.0, 0.0), 0.5, &open),
            site,
            "far off: unchanged"
        );
        let pilot = Vec2::new(48.0, 3.0);
        let moved = clear_of(&c, NEEDS, site, pilot, 0.5, &open);
        assert!(moved.distance(pilot) >= NEEDS.radius_m + c.pilot_clear_m + 0.5 - 1e-3);
        assert!(
            moved.distance(site) < NEEDS.radius_m + c.pilot_clear_m + 3.0,
            "just beside, not far off"
        );
        // Nothing good round them: still beside them, never on them.
        let moved = clear_of(&c, NEEDS, site, pilot, 0.5, &|_| false);
        assert!(moved.distance(pilot) >= NEEDS.radius_m + c.pilot_clear_m + 0.5 - 1e-3);
        // Standing exactly on it.
        let moved = clear_of(&c, NEEDS, site, site, 0.5, &open);
        assert!(moved.distance(site) > NEEDS.radius_m);
        assert_eq!(steer(Vec2::ZERO, Vec2::new(10.0, 0.0), 4.0), Vec2::new(4.0, 0.0));
        assert_eq!(steer(Vec2::ZERO, Vec2::new(1.0, 0.0), 4.0), Vec2::new(1.0, 0.0));
    }

    #[test]
    fn the_pilot_boards_the_owned_mech_they_walk_up_to() {
        let c = cfg();
        let mechs = [(0, Vec2::new(0.0, 0.0)), (4, Vec2::new(100.0, 0.0)), (2, Vec2::new(100.0, 0.0))];
        assert_eq!(walking_up_to(&c, Vec2::new(5.0, 0.0), &mechs), Some(0));
        assert_eq!(walking_up_to(&c, Vec2::new(90.0, 0.0), &mechs), Some(2), "ties to the lower key");
        assert_eq!(walking_up_to(&c, Vec2::new(50.0, 0.0), &mechs), None, "too far from both");
    }

    /// The career-loop playtest's beat 5 in the rules: at bay 1's terminal of the first base, the Sapper is bought;
    /// it lands near the pilot, clear of the hangars, on the career's clock; walked up to, it is the mech that kneels;
    /// seated, its drop is done; a save keeps it where it landed.
    #[test]
    fn beat_5_a_sapper_bought_at_the_terminal_lands_near_the_pilot_and_is_boarded() {
        use crate::career::{Career, CareerData, Field};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let data = CareerData::load(&root).unwrap();
        let world: crate::world::WorldConfig = crate::data::load_ron(&root.join("world/world.ron")).unwrap();
        let plan = crate::world::generate(42, &world);
        let g = PlanGround {
            plan: &plan,
            bases: &world.bases,
        };
        let terminal = plan.bases[0].terminals(&world.bases)[0];
        let mut c = Career::new("t", 7, &data.career, &data.whole("mechs/foreman.ron").unwrap());
        c.pay(650_000, "playtest grant", 200);
        let foreman_at = plan.bases[0].hangars(&world.bases)[0].centre();
        let sapper = data.design("mechs/sapper.ron").unwrap();
        let mechs = [(foreman_at, 7.56)];
        let keep = Keep {
            pilot: terminal.at,
            mechs: &mechs,
        };
        let rng = SimRng::new(c.seed).fork("drop/1");
        let site = drop_site(&data.drop, &g, Needs::of(sapper), &keep, &rng).expect("the apron takes it");
        let at = Vec2::new(site.at.x, site.at.z);
        let d = at.distance(terminal.at);
        assert!(
            (data.drop.ring_min_m - 0.01..=data.drop.ring_max_m + 0.01).contains(&d),
            "{d:.2} m from the pilot"
        );
        let centres: Vec<Vec2> = plan.bases.iter().map(|b| b.centre).collect();
        let bays = crate::shop::Bays {
            per_base: world.bases.hangars as u32,
            centres: &centres,
        };
        let order = crate::shop::Order {
            design: "mechs/sapper.ron",
            at: Some(0),
            from: terminal.at,
            site: Some(site.at),
            rng: &rng,
        };
        let whole = data.whole("mechs/sapper.ron").unwrap();
        let bought = crate::shop::buy(&mut c, &data.catalogue, &bays, &order, &whole, 200).unwrap();
        let key = bought.key;
        let drop = c.drop_of(key).unwrap().clone();
        assert!(matches!(drop.stage(&data.drop, c.time_s), Stage::Inbound(_)));
        c.time_s += f64::from(data.drop.delay_s + fall_s(&data.drop)) + 0.1;
        assert_eq!(drop.stage(&data.drop, c.time_s), Stage::Landed);
        c.landed(
            key,
            Field {
                pos: site.at.to_array(),
                heading_rad: drop.track_rad,
            },
        );
        // Walking from the terminal toward it: the Foreman in bay 1 first, then the Sapper once nearer.
        let standing = [(c.mechs[0].key, foreman_at), (key, at)];
        let walk_to = |f: f32| terminal.at.lerp(at, f);
        assert_eq!(walking_up_to(&data.drop, walk_to(0.0), &standing), Some(c.mechs[0].key));
        assert_eq!(walking_up_to(&data.drop, walk_to(0.95), &standing), Some(key));
        c.boarded(key);
        assert_eq!(c.piloting, Some(key));
        assert!(c.drop_of(key).is_none(), "the marker goes once it is boarded");
        let back = Career::from_ron(&c.to_ron()).unwrap();
        assert_eq!(
            back.mech(key).unwrap().field.map(|f| f.pos),
            Some(site.at.to_array()),
            "a save keeps it there"
        );
    }

    #[test]
    fn on_the_island_a_drop_in_the_biggest_city_misses_every_building() {
        let world = crate::world::WorldConfig::default();
        let plan = crate::world::generate(42, &world);
        let g = PlanGround {
            plan: &plan,
            bases: &world.bases,
        };
        let c = cfg();
        let city = &plan.cities[0];
        let pilot = city.streets[0].rect.centre();
        let s = drop_site(&c, &g, NEEDS, &keep(pilot), &SimRng::new(42)).expect("the city has a street wide enough");
        let at = Vec2::new(s.at.x, s.at.z);
        assert!(!plan.collision.overlaps(at, NEEDS.radius_m + c.building_clear_m), "{at}");
        assert!(g.water_depth_m(at) <= NEEDS.max_depth_m);
        // And at a base: never in a hangar or the tower.
        let base = plan.bases[0];
        let inside = base.hangars(&world.bases)[1].centre();
        let s = drop_site(&c, &g, NEEDS, &keep(inside), &SimRng::new(42)).expect("the apron takes it");
        let at = Vec2::new(s.at.x, s.at.z);
        assert!(
            base.structures(&world.bases).iter().all(|r| r.distance(at) >= NEEDS.radius_m),
            "{at}"
        );
        assert!(s.paved, "the apron is paved");
    }
}
