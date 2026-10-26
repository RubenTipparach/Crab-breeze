//! Civil air traffic (openspec change `civil-air-traffic`; owner, 2026-10-26: "Do we have regular air
//! traffic?", survey W33: airliners and cargo planes between the five air bases, light planes and news
//! or police helicopters over the cities, keeping clear of a fight, and crabs can shoot them down).
//!
//! `Traffic` keeps the aircraft near the pilot only: it spawns them where the pilot cannot see them
//! appear, removes them far away, and never simulates the whole island. Each aircraft flies through
//! `flight`; its shape, hit capsules and wreck are `shape`. The pilot's own hits are tallied per base in
//! `Standing`; `crate::civil_standing` charges them to the career (survey W34). Pure and seeded.

pub mod config;
pub mod flight;
pub mod shape;
#[cfg(test)]
mod tests;

pub use config::{AirTrafficConfig, AircraftSpec, Engine, EngineSound, Role};
pub use flight::{Air, Aircraft, Flight, FlightStep, Phase, Strip, Zone, departure_strip, in_zone, landing_strip, rest_m};

use crate::rng::SimRng;
use crate::world::{AirBase, City};
use config::NearConfig;
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;

/// Where the pilot is and looks: traffic appears only where this cannot see it appear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Watcher {
    /// The pilot's eye.
    pub eye: Vec3,
    /// Where the view looks (any length).
    pub look: Vec3,
}

/// True when an aircraft may appear at `p` unseen: beyond `spawn_far_m` (a speck at most), or beyond
/// `spawn_near_m` and outside the view cone.
pub fn hidden(w: &Watcher, p: Vec3, near: &NearConfig) -> bool {
    let d = w.eye.distance(p);
    if d >= near.spawn_far_m {
        return true;
    }
    if d < near.spawn_near_m {
        return false;
    }
    let look = w.look.normalize_or_zero();
    look == Vec3::ZERO || look.angle_between((p - w.eye).normalize_or_zero()).to_degrees() > near.view_half_deg
}

/// The island as traffic needs it.
pub struct Island<'a> {
    /// Ground height under a point (x, z).
    pub ground: &'a dyn Fn(f32, f32) -> f32,
    /// The air bases.
    pub bases: &'a [AirBase],
    /// The cities.
    pub cities: &'a [City],
    /// Fights to keep clear of.
    pub zones: &'a [Zone],
}

/// Something that happened to an aircraft this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Happened {
    /// It crashed (shot down): a wreck at `at`.
    Crashed,
    /// It touched down.
    Landed,
    /// It passed over its route's `via` point with this index.
    Passed(usize),
    /// Its destination came under attack and it turned for another base.
    Diverted,
}

/// An event from `Traffic::step`.
#[derive(Clone, Debug, PartialEq)]
pub struct TrafficEvent {
    /// The aircraft.
    pub id: u32,
    /// Its kind.
    pub kind: String,
    /// Where it was.
    pub at: Vec3,
    /// Its heading.
    pub heading_rad: f32,
    /// What happened.
    pub what: Happened,
}

/// The civil aircraft near the pilot, in id order.
#[derive(Clone, Debug, PartialEq)]
pub struct Traffic {
    /// The aircraft.
    pub list: Vec<Aircraft>,
    next_id: u32,
    since_check_s: f32,
    checks: u64,
    rng: SimRng,
}

impl Traffic {
    /// No traffic yet; the first look for room to spawn is on the first step.
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            next_id: 0,
            since_check_s: f32::INFINITY,
            checks: 0,
            rng: SimRng::new(seed).fork("air-traffic"),
        }
    }

    /// The aircraft with this id.
    pub fn get(&self, id: u32) -> Option<&Aircraft> {
        self.list.iter().find(|a| a.id == id)
    }

    /// The aircraft with this id, to change.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Aircraft> {
        self.list.iter_mut().find(|a| a.id == id)
    }

    /// One fixed step: every aircraft flies, finished, crashed and far ones leave, and every
    /// `check_s` each kind below its `max_near` may gain one, out of sight.
    pub fn step(&mut self, cfg: &AirTrafficConfig, w: &Watcher, island: &Island, dt_s: f32) -> Vec<TrafficEvent> {
        let air = Air {
            ground: island.ground,
            zones: island.zones,
            bases: island.bases,
            cfg,
        };
        let mut events = Vec::new();
        let mut leave = Vec::new();
        for a in &mut self.list {
            let Some(spec) = cfg.kinds.get(&a.kind) else {
                leave.push(a.id);
                continue;
            };
            let s = a.step(spec, &air, dt_s);
            let mut say = |what| {
                events.push(TrafficEvent {
                    id: a.id,
                    kind: a.kind.clone(),
                    at: a.pos,
                    heading_rad: a.heading_rad,
                    what,
                })
            };
            if s.crashed {
                say(Happened::Crashed);
            }
            if s.landed {
                say(Happened::Landed);
            }
            if let Some(i) = s.passed {
                say(Happened::Passed(i));
            }
            if s.diverted {
                say(Happened::Diverted);
            }
            let far = !a.downed() && a.pos.distance(w.eye) > cfg.near.despawn_m;
            if s.done || s.crashed || far {
                leave.push(a.id);
            }
        }
        self.list.retain(|a| !leave.contains(&a.id));
        self.since_check_s += dt_s;
        if self.since_check_s >= cfg.near.check_s {
            self.since_check_s = 0.0;
            self.spawn(cfg, w, island);
        }
        events
    }

    /// The career loop's hook (survey: "bought mechs air dropped near the player by a cargo aircraft"):
    /// a `kind` rolls now from `from` for `to`, over each `via` point first, reporting each as it
    /// passes (`Happened::Passed`). Wherever the pilot is; it still leaves the world far from them.
    pub fn launch(&mut self, cfg: &AirTrafficConfig, kind: &str, from: &AirBase, to: &AirBase, via: Vec<Vec2>) -> Option<u32> {
        let spec = cfg.kinds.get(kind)?;
        let first = via.first().copied().unwrap_or(to.centre);
        let strip = departure_strip(from, &cfg.routes, first);
        let id = self.take_id();
        let mut a = Aircraft::departing(id, kind, spec, strip, to, &cfg.routes);
        if let Flight::Route { via: v, .. } = &mut a.flight {
            *v = via;
        }
        self.list.push(a);
        Some(id)
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn spawn(&mut self, cfg: &AirTrafficConfig, w: &Watcher, island: &Island) {
        let mut rng = self.rng.fork(&format!("check {}", self.checks));
        self.checks += 1;
        for (name, spec) in &cfg.kinds {
            let have = self.list.iter().filter(|a| &a.kind == name).count() as u32;
            if have >= spec.max_near {
                continue;
            }
            let id = self.next_id;
            let a = if spec.role.flies_routes() {
                route_spawn(id, name, spec, cfg, w, island, &mut rng)
            } else {
                city_spawn(id, name, spec, cfg, w, island, &mut rng)
            };
            if let Some(a) = a {
                self.take_id();
                self.list.push(a);
            }
        }
    }
}

/// Room for a spawn: unseen, not about to be removed, not in a fight.
fn room(p: Vec3, w: &Watcher, island: &Island, cfg: &AirTrafficConfig) -> bool {
    hidden(w, p, &cfg.near) && p.distance(w.eye) < cfg.near.despawn_m * 0.9 && !in_zone(Vec2::new(p.x, p.z), island.zones, 0.0)
}

fn pick<T>(mut v: Vec<T>, rng: &mut SimRng) -> Option<T> {
    if v.is_empty() {
        return None;
    }
    let i = (rng.next_u64() % v.len() as u64) as usize;
    Some(v.swap_remove(i))
}

/// A route aircraft: rolling at a base out of sight, or cruising between two bases out of sight.
fn route_spawn(
    id: u32,
    name: &str,
    spec: &AircraftSpec,
    cfg: &AirTrafficConfig,
    w: &Watcher,
    island: &Island,
    rng: &mut SimRng,
) -> Option<Aircraft> {
    let open: Vec<&AirBase> = island.bases.iter().filter(|b| !in_zone(b.centre, island.zones, 0.0)).collect();
    if open.len() < 2 {
        return None;
    }
    enum Start<'b> {
        Depart(Strip, &'b AirBase),
        Cruise(Vec2, &'b AirBase),
    }
    let mut starts = Vec::new();
    for from in &open {
        for to in open.iter().filter(|t| t.id != from.id) {
            let strip = departure_strip(from, &cfg.routes, to.centre);
            let at = Vec3::new(strip.start.x, strip.ground_m + rest_m(spec), strip.start.y);
            if room(at, w, island, cfg) {
                starts.push(Start::Depart(strip, to));
            }
            for t in [0.25f32, 0.5, 0.75] {
                let p = from.centre.lerp(to.centre, t);
                // The bases' ground stands in for the ground between them here: reading the ground is
                // dear and a candidate only needs to be out of sight (the chosen one reads it once).
                let guess = from.ground_m + (to.ground_m - from.ground_m) * t;
                let at = Vec3::new(p.x, guess + spec.cruise_agl_m, p.y);
                if room(at, w, island, cfg) {
                    starts.push(Start::Cruise(p, to));
                }
            }
        }
    }
    Some(match pick(starts, rng)? {
        Start::Depart(strip, to) => Aircraft::departing(id, name, spec, strip, to, &cfg.routes),
        Start::Cruise(p, to) => {
            let strip = landing_strip(to, &cfg.routes, p);
            let way = strip.fix(cfg.routes.final_m) - p;
            let heading = crate::units::heading_of(Vec3::new(way.x, 0.0, way.y));
            let at = Vec3::new(p.x, (island.ground)(p.x, p.y) + spec.cruise_agl_m, p.y);
            Aircraft::new(
                id,
                name,
                spec,
                at,
                heading,
                spec.cruise_m_s,
                Phase::Enroute,
                Flight::Route {
                    to: to.id,
                    strip,
                    via: Vec::new(),
                    next_via: 0,
                },
            )
        }
    })
}

/// A city aircraft: on its circle round a city near the pilot, or inbound to it from farther out,
/// out of sight either way.
fn city_spawn(
    id: u32,
    name: &str,
    spec: &AircraftSpec,
    cfg: &AirTrafficConfig,
    w: &Watcher,
    island: &Island,
    rng: &mut SimRng,
) -> Option<Aircraft> {
    let eye = Vec2::new(w.eye.x, w.eye.z);
    let radius = rng.range_f32(spec.orbit_radius_m[0], spec.orbit_radius_m[1]);
    let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let mut starts = Vec::new();
    for c in island.cities {
        if c.centre.distance(eye) > cfg.near.despawn_m * 0.8 || in_zone(c.centre, island.zones, radius) {
            continue;
        }
        for k in 0..8 {
            let ray = Vec2::from_angle(k as f32 * std::f32::consts::TAU / 8.0);
            for (out, on_circle) in [(radius, true), (cfg.near.spawn_near_m * 1.2 + radius, false)] {
                let p = c.centre + ray * out;
                // The city's ground stands in until one is chosen (reading the ground is dear).
                let at = Vec3::new(p.x, c.ground_m + spec.cruise_agl_m, p.y);
                if room(at, w, island, cfg) {
                    starts.push((c, at, ray, on_circle));
                }
            }
        }
    }
    let (c, mut at, ray, on_circle) = pick(starts, rng)?;
    at.y = (island.ground)(at.x, at.z) + spec.cruise_agl_m;
    let way = if on_circle { ray.perp() * dir } else { -ray };
    let heading = crate::units::heading_of(Vec3::new(way.x, 0.0, way.y));
    Some(Aircraft::new(
        id,
        name,
        spec,
        at,
        heading,
        spec.cruise_m_s,
        Phase::Orbit,
        Flight::Orbit {
            city: c.id,
            centre: c.centre,
            radius_m: radius,
            dir,
        },
    ))
}

/// The air base nearest a point (x, z), ties by id: whose standing a hit there costs. The one rule, in
/// `civil_standing`.
pub use crate::civil_standing::nearest_base;

/// The pilot's hits on civil aircraft at one base.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Hits that did damage or not.
    pub hits: u32,
    /// Of those, the ones that brought an aircraft down.
    pub downed: u32,
}

/// The pilot's hits on civil aircraft, per air base (survey W34). A counter only: what a hit costs the
/// career's standing is `crate::civil_standing`'s to decide.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// By base id.
    pub by_base: BTreeMap<u32, Tally>,
}

impl Standing {
    /// One hit near `base`.
    pub fn record(&mut self, base: u32, downed: bool) {
        let t = self.by_base.entry(base).or_default();
        t.hits += 1;
        if downed {
            t.downed += 1;
        }
    }
}
