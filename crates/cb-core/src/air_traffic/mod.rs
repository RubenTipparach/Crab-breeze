//! Civil air traffic (openspec change `civil-air-traffic`; owner, 2026-10-26: "Do we have regular air
//! traffic?", survey W33: airliners and cargo planes between the five air bases, light planes and news
//! or police helicopters over the cities, keeping clear of a fight, and crabs can shoot them down).
//!
//! `Traffic` keeps the aircraft near the pilot only: it spawns them where the pilot cannot see them
//! appear, removes them far away, and never simulates the whole island. Each aircraft flies through
//! `flight`; its shape, hit capsules and wreck are `shape`. The pilot's own hits are tallied per base in
//! `Standing`; `crate::civil_standing` charges them to the career (survey W34). Pure and seeded.

pub mod config;
pub mod fields;
pub mod flight;
pub mod model;
pub mod shape;
#[cfg(test)]
mod tests;

pub use config::{AirTrafficConfig, AircraftModel, AircraftSpec, Engine, EngineSound, Role};
pub use fields::{CIVIL_FIELD_ID, Field};
pub use flight::{
    Air, Aircraft, Flight, FlightStep, HitOutcome, Phase, Strip, Then, Zone, departure_strip, in_zone, landing_strip, rest_m,
    rest_pitch_rad,
};

use crate::rng::SimRng;
use crate::world::{AirBase, City};
use config::NearConfig;
use glam::{Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet};

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
    /// Every runway traffic flies to: the airfields' bases (`Field::base`), civil airports and military bases.
    pub bases: &'a [AirBase],
    /// The airfields, for traffic on the ground (none: aircraft only fly, as before airfield-traffic).
    pub fields: &'a [Field],
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
        self.ground(cfg, island);
        self.since_check_s += dt_s;
        if self.since_check_s >= cfg.near.check_s {
            self.since_check_s = 0.0;
            self.spawn(cfg, w, island);
            self.fill_stands(cfg, w, island);
        }
        events
    }

    /// The stands held, by (field, stand): whoever stands at one or taxis to or from it.
    pub fn stands_held(&self) -> BTreeMap<(u32, usize), u32> {
        self.list.iter().filter_map(|a| Some((a.stand?, a.id))).collect()
    }

    /// The pilot steals aircraft `id` (`fly-stolen-aircraft`): only one standing still on the ground and whole. It leaves
    /// the traffic's care, its stand freed: from now its place is the pilot's flight model's (`fly`). True when taken.
    pub fn steal(&mut self, id: u32) -> bool {
        let Some(a) = self.get_mut(id) else { return false };
        if a.stolen {
            return true;
        }
        let still = matches!(a.phase, Phase::Stand { .. } | Phase::Parked { .. } | Phase::Hold { .. })
            || (matches!(a.phase, Phase::Taxi { .. }) && a.speed_m_s < 1.0);
        if !still || a.downed() {
            return false;
        }
        a.stolen = true;
        a.stand = None;
        a.field = None;
        a.taxi.clear();
        a.departure = None;
        a.speed_m_s = 0.0;
        a.vel = Vec3::ZERO;
        a.phase = Phase::Piloted { wheels: true };
        true
    }

    /// Where the pilot's flight model has a stolen aircraft this step: its centre, velocity and attitude, and whether it
    /// is on its wheels. Ignored for any aircraft not stolen, or downed.
    #[allow(clippy::too_many_arguments)]
    pub fn fly(&mut self, id: u32, pos: Vec3, vel: Vec3, heading_rad: f32, pitch_rad: f32, bank_rad: f32, wheels: bool) {
        let Some(a) = self.get_mut(id) else { return };
        if !a.stolen || a.downed() {
            return;
        }
        a.pos = pos;
        a.vel = vel;
        a.speed_m_s = vel.length();
        a.heading_rad = heading_rad;
        a.pitch_rad = pitch_rad;
        a.bank_rad = bank_rad;
        a.phase = Phase::Piloted { wheels };
    }

    /// A stolen aircraft flown into the ground: it comes down (and crashes as any downed one does).
    pub fn crash(&mut self, id: u32) {
        if let Some(a) = self.get_mut(id)
            && !a.downed()
        {
            a.hp = 0.0;
            a.phase = Phase::Falling;
        }
    }

    /// A military field's alarm (owner's plan answer, 2026-10-30: "Patrol, scramble if hostile"): every fighter and
    /// gunship standing at field `field` leaves at once to patrol over `over` (x, z), and every one already on patrol
    /// from it turns to circle there instead. How many answered.
    pub fn scramble(&mut self, cfg: &AirTrafficConfig, field: u32, over: Vec2) -> usize {
        let mut n = 0;
        for a in &mut self.list {
            let Some(spec) = cfg.kinds.get(&a.kind) else { continue };
            if !matches!(spec.role, Role::Fighter | Role::Gunship) {
                continue;
            }
            match (&mut a.flight, a.phase) {
                (_, Phase::Stand { .. }) if a.field == Some(field) => {
                    a.phase = Phase::Stand { t_s: spec.turn_s };
                    a.scramble = Some(over);
                    a.hostile = true;
                    n += 1;
                }
                (Flight::Patrol { home, centre, left_s, .. }, _) if *home == field => {
                    *centre = over;
                    *left_s = left_s.max(cfg.routes.patrol_s * 0.5);
                    a.hostile = true;
                    n += 1;
                }
                _ => {}
            }
        }
        n
    }

    /// The scrambled aircraft now circling over the pilot: they leave the traffic, to be flown and fired by the
    /// invasion's units (design F2: "using the existing unit weapons"). In id order.
    pub fn take_attackers(&mut self) -> Vec<Aircraft> {
        let ready = |a: &Aircraft| a.hostile && !a.downed() && a.phase == Phase::Orbit && matches!(a.flight, Flight::Patrol { .. });
        let out: Vec<Aircraft> = self.list.iter().filter(|a| ready(a)).cloned().collect();
        self.list.retain(|a| !ready(a));
        out
    }

    /// The fields whose runway is in use: an aircraft lining up or rolling on it, or one on final or rolling out.
    fn runways_busy(&self) -> BTreeSet<u32> {
        let mut busy = BTreeSet::new();
        for a in &self.list {
            let lining_up = matches!(a.phase, Phase::Taxi { then: Then::Roll, .. } | Phase::Roll { .. });
            if lining_up && let Some(f) = a.field {
                busy.insert(f);
            }
            if let (Phase::Final | Phase::Rollout { .. }, Flight::Route { to, .. }) = (a.phase, &a.flight) {
                busy.insert(*to);
            }
        }
        busy
    }

    /// The traffic's decisions on the ground (openspec change `airfield-traffic`): an aircraft whose turnaround is
    /// over leaves its stand, one at its holding point is cleared onto a free runway, and one that has landed taxis
    /// to a free stand of its size. In id order, each decision seen by the next.
    fn ground(&mut self, cfg: &AirTrafficConfig, island: &Island) {
        if island.fields.is_empty() {
            return;
        }
        let r = &cfg.routes;
        let mut held = self.stands_held();
        let mut busy = self.runways_busy();
        for i in 0..self.list.len() {
            let a = &self.list[i];
            let Some(spec) = cfg.kinds.get(&a.kind) else { continue };
            match a.phase {
                Phase::Stand { t_s } if t_s >= spec.turn_s => {
                    let Some(field) = a.field.and_then(|f| island.fields.iter().find(|x| x.id == f)) else {
                        continue;
                    };
                    let mut rng = self.rng.fork(&format!("depart {} {}", a.id, self.checks));
                    depart(&mut self.list[i], spec, field, island, r, &mut rng);
                }
                Phase::Hold { t_s } if t_s >= r.hold_s => {
                    let (Some(f), Some(strip)) = (a.field, a.departure) else { continue };
                    if busy.contains(&f) {
                        continue;
                    }
                    let a = &mut self.list[i];
                    a.taxi = vec![a.xz(), strip.start];
                    a.phase = Phase::Taxi {
                        s_m: 0.0,
                        push_m: 0.0,
                        then: Then::Roll,
                    };
                    // its stand is free for the next arrival once it rolls
                    if !spec.role.rotorcraft() {
                        a.stand = None;
                    }
                    busy.insert(f);
                }
                Phase::Parked { .. } => {
                    let Flight::Route { to, strip, .. } = a.flight else { continue };
                    let Some(field) = island.fields.iter().find(|x| x.id == to) else {
                        continue;
                    };
                    let (here, dir) = (a.xz(), strip.dir());
                    let Some(exit) = field.exit_for(here, dir) else { continue };
                    let Some(k) = free_stand(field, &a.kind, &held) else { continue };
                    let Some(route) = field.taxi.route(exit, field.stands[k].node) else {
                        continue;
                    };
                    let a = &mut self.list[i];
                    let mut line = vec![here];
                    line.extend(field.taxi.line(&route));
                    a.taxi = line;
                    a.phase = Phase::Taxi {
                        s_m: 0.0,
                        push_m: 0.0,
                        then: Then::Stand,
                    };
                    a.stand = Some((field.id, k));
                    a.stand_heading = field.stands[k].heading_rad;
                    a.field = Some(field.id);
                    a.departure = None;
                    held.insert((field.id, k), a.id);
                }
                _ => {}
            }
        }
    }

    /// Every check: the empty stands of each field near the pilot that the pilot cannot see fill, three in four, with
    /// the kind the stand is for, part way through its turnaround (so they leave in turn, not all at once).
    fn fill_stands(&mut self, cfg: &AirTrafficConfig, w: &Watcher, island: &Island) {
        let held = self.stands_held();
        let eye = Vec2::new(w.eye.x, w.eye.z);
        let mut rng = self.rng.fork(&format!("stands {}", self.checks));
        for f in island.fields {
            if f.base.centre.distance(eye) >= cfg.routes.stand_fill_m {
                continue;
            }
            for (k, st) in f.stands.iter().enumerate() {
                let Some(spec) = cfg.kinds.get(&st.kind) else { continue };
                let p = f.taxi.points[st.node];
                let at = Vec3::new(p.x, (island.ground)(p.x, p.y), p.y);
                let roll = rng.chance(0.75);
                let t_s = rng.range_f32(0.0, spec.turn_s);
                // the first check fills them in sight as well: the world is still loading
                let unseen = self.checks <= 1 || hidden(w, at, &cfg.near);
                if held.contains_key(&(f.id, k)) || !unseen || !roll {
                    continue;
                }
                let id = self.take_id();
                self.list
                    .push(Aircraft::standing(id, &st.kind, spec, f.id, k, at, st.heading_rad, t_s));
            }
        }
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
            // a military base's fighters and gunships live on its stands only
            if spec.role.military() && !spec.role.flies_routes() {
                continue;
            }
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

/// The runways a kind flies between: a military kind the bases', a civil one the civil airports' (when the island has
/// two; before it did, every runway).
fn route_bases<'b>(island: &Island<'b>, spec: &AircraftSpec) -> Vec<&'b AirBase> {
    let side: Vec<&AirBase> = island
        .bases
        .iter()
        .filter(|b| (b.id >= CIVIL_FIELD_ID) != spec.role.military())
        .collect();
    if side.len() >= 2 { side } else { island.bases.iter().collect() }
}

/// A free stand at `field` for `kind`: one made for it, else (for a fixed wing) any free stand that is no pad.
fn free_stand(field: &Field, kind: &str, held: &BTreeMap<(u32, usize), u32>) -> Option<usize> {
    let free = |k: &usize| !held.contains_key(&(field.id, *k));
    let ks = 0..field.stands.len();
    ks.clone()
        .filter(free)
        .find(|&k| field.stands[k].kind == kind)
        .or_else(|| ks.filter(free).find(|&k| !field.stands[k].pad))
}

/// Sends `a` off its stand at `field`: a rotorcraft lifts off for a patrol (over a city, or over its base) and keeps
/// its pad to settle back on; a fighter taxis out for a patrol over its base; anything else taxis out for a route to
/// another field of its side. Pushed back first where its stand's way out is behind it.
fn depart(a: &mut Aircraft, spec: &AircraftSpec, field: &Field, island: &Island, r: &config::RouteConfig, rng: &mut SimRng) {
    let Some((_, k)) = a.stand else { return };
    let Some(stand) = field.stands.get(k) else { return };
    let here = a.xz();
    let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
    if spec.role.rotorcraft() {
        let (centre, radius_m) = if let Some(p) = a.scramble.take() {
            (p, spec.orbit_radius_m[0])
        } else if field.civil {
            let c = island
                .cities
                .iter()
                .min_by(|x, y| x.centre.distance(here).total_cmp(&y.centre.distance(here)).then(x.id.cmp(&y.id)));
            (
                c.map_or(field.base.centre, |c| c.centre),
                rng.range_f32(spec.orbit_radius_m[0], spec.orbit_radius_m[1]),
            )
        } else {
            (field.base.centre, r.patrol_radius_m)
        };
        a.flight = Flight::Patrol {
            home: field.id,
            centre,
            radius_m,
            dir,
            left_s: r.patrol_s,
            pad: Some(here),
        };
        a.phase = Phase::Lift;
        return;
    }
    let (toward, flight) = if spec.role == Role::Fighter {
        let scrambled = a.scramble.take();
        let centre = scrambled.unwrap_or(field.base.centre);
        let flight = Flight::Patrol {
            home: field.id,
            centre,
            radius_m: if scrambled.is_some() {
                spec.orbit_radius_m[0]
            } else {
                r.patrol_radius_m
            },
            dir,
            left_s: r.patrol_s,
            pad: None,
        };
        (centre + (centre - here).normalize_or(Vec2::X) * r.patrol_radius_m, flight)
    } else {
        let others: Vec<&AirBase> = route_bases(island, spec).into_iter().filter(|b| b.id != field.id).collect();
        let Some(to) = pick(others, rng) else {
            a.phase = Phase::Stand { t_s: 0.0 };
            return;
        };
        let flight = Flight::Route {
            to: to.id,
            strip: landing_strip(to, r, field.base.centre),
            via: Vec::new(),
            next_via: 0,
        };
        (to.centre, flight)
    };
    let strip = departure_strip(&field.base, r, toward);
    let entry = field.entry_for(strip.start);
    let way_out = stand.out.unwrap_or(stand.node);
    let Some(route) = field.taxi.route(way_out, entry) else {
        a.phase = Phase::Stand { t_s: 0.0 };
        return;
    };
    let out_p = field.taxi.points[way_out];
    let behind = (out_p - here).dot(a.fwd2()) < 0.0;
    let mut line = vec![here];
    line.extend(field.taxi.line(&route));
    line.dedup_by(|x, y| x.distance(*y) < 0.5);
    a.taxi = line;
    a.phase = Phase::Taxi {
        s_m: 0.0,
        push_m: if behind { here.distance(out_p) } else { 0.0 },
        then: Then::Hold,
    };
    a.flight = flight;
    a.departure = Some(strip);
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
    let open: Vec<&AirBase> = route_bases(island, spec)
        .into_iter()
        .filter(|b| !in_zone(b.centre, island.zones, 0.0))
        .collect();
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
