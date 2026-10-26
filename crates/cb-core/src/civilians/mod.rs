//! Civilians (openspec change `civilians`): people on the pavements, in the parks and on the air bases'
//! aprons, brought in round the pilot and let go far from them, walking set routes (`routes`), standing
//! at crossings and idling, sitting on park benches, stopping to answer when greeted (`lines`), and
//! running from gunfire, blasts and mechs' feet. Rounds, blasts and feet can kill them, and the deaths
//! the pilot causes are counted against the nearest base (`toll`, the career's standing hook).
//!
//! Pure and deterministic (CLAUDE.md section 6.3): the same seed, pilot path, dangers and greetings
//! give the same people in the same places. Each person draws from their own stream, forked from the
//! world's seed by their home, the home's visit count and their place in it, so who is brought in never
//! depends on frame timing or on another person. People are kept in a list in the order they were
//! brought in, and every step walks it in that order.

pub mod config;
pub mod lines;
pub mod routes;
pub mod toll;

#[cfg(test)]
mod tests;

pub use config::{DrawConfig, PeopleConfig};
pub use lines::{Answer, Lines, Place, Role};
pub use routes::{Doors, Home, NoDoors};
pub use toll::{Casualty, Cause, StandingLoss, Toll};

use crate::data::Validate;
use crate::rng::SimRng;
use crate::world::{AirBase, City, Ground, Rect, WorldConfig};
use glam::{Vec2, Vec3};
use routes::LAP;
use std::collections::{BTreeMap, BTreeSet};

/// `data/people/people.ron` and `data/people/lines.ron`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PeopleData {
    /// Tuning.
    pub cfg: PeopleConfig,
    /// Roles and lines.
    pub lines: Lines,
}

impl Validate for PeopleData {
    fn validate(&self) -> Result<(), String> {
        self.cfg.validate()?;
        self.lines.validate()
    }
}

/// What a person is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Activity {
    /// Walking their route.
    Walking,
    /// Standing at the kerb, about to cross.
    Waiting {
        /// Seconds left.
        left_s: f32,
    },
    /// Standing a while.
    Idle {
        /// Seconds left.
        left_s: f32,
    },
    /// Sitting on a park bench.
    Sitting {
        /// Seconds left.
        left_s: f32,
    },
    /// Stopped, facing the pilot who greeted them.
    Greeting {
        /// Seconds left.
        left_s: f32,
        /// Where the pilot stood (x, z).
        face: Vec2,
    },
    /// Running from danger.
    Fleeing {
        /// Seconds left.
        left_s: f32,
        /// Which way (a unit vector on the ground).
        dir: Vec2,
        /// The street door they run for, when there is one.
        door: Option<Vec2>,
    },
    /// Struck down; cleared away when the time is up.
    Down {
        /// Seconds left.
        left_s: f32,
    },
}

/// The leg of their route a walker is on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Leg {
    /// Round a loop, toward waypoint `k`, `step` +1 or -1 a waypoint.
    Loop { rect: Rect, k: u8, step: i8 },
    /// Across a street to block `block`, joining its loop at waypoint `k`.
    Crossing { to: Vec2, block: u32, k: u8 },
    /// Straight on (after running off their loop, out of a city).
    Straight { dir: Vec2 },
}

/// One person.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    /// Stable id, in the order people were brought in.
    pub id: u32,
    /// Where they were brought in.
    pub home: Home,
    /// What kind of place it is.
    pub place: Place,
    /// Their role's index in `Lines::roles`.
    pub role: u16,
    /// Feet on the ground (x, z).
    pub pos: Vec2,
    /// Feet height.
    pub feet_y: f32,
    /// Facing, radians, 0 facing -z, positive turning left (the mech's convention).
    pub heading_rad: f32,
    /// Standing height.
    pub height_m: f32,
    /// Walking speed.
    pub walk_m_s: f32,
    /// Running speed.
    pub run_m_s: f32,
    /// How fast they moved this step (for the walk's cadence).
    pub pace_m_s: f32,
    /// A number the look (clothes, skin, hair) is drawn from.
    pub look: u32,
    /// What they are doing.
    pub activity: Activity,
    /// Times greeted.
    pub greeted: u32,
    /// The block they are on now (a city block's index), for pavement people.
    block: u32,
    leg: Leg,
    rng: SimRng,
}

impl Person {
    /// Their feet (x, y, z).
    pub fn feet(&self) -> Vec3 {
        Vec3::new(self.pos.x, self.feet_y, self.pos.y)
    }

    /// True while they are alive and in the world.
    pub fn standing(&self) -> bool {
        !matches!(self.activity, Activity::Down { .. })
    }
}

/// Something that frightens or kills people.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Danger {
    /// A round's flight this step, from `from` to `to`.
    Shot {
        /// Where it was.
        from: Vec3,
        /// Where it is now.
        to: Vec3,
        /// The mech or vehicle that fired it.
        by: Option<u32>,
    },
    /// An explosion.
    Blast {
        /// Where.
        at: Vec3,
        /// Its energy, megajoules.
        energy_mj: f32,
        /// Whose it was, when known.
        by: Option<u32>,
    },
    /// A mech's foot coming down.
    Footfall {
        /// The ground under the foot.
        at: Vec3,
        /// The mech.
        by: Option<u32>,
    },
}

/// Everyone brought in, and what decides who comes next.
#[derive(Clone, Debug, PartialEq)]
pub struct People {
    /// Everyone in the world now, in the order they were brought in.
    pub list: Vec<Person>,
    root: SimRng,
    /// Homes peopled now.
    active: BTreeSet<Home>,
    /// Times each home has been peopled.
    visits: BTreeMap<Home, u32>,
    next_id: u32,
    /// Where the pilot was at the last full look round.
    checked_at: Option<Vec2>,
    /// The last look round stopped at `spawn_per_step`: look again next step.
    pending: bool,
}

/// Where everything people walk on comes from.
pub struct World<'a> {
    /// The cities (`WorldPlan::cities`).
    pub cities: &'a [City],
    /// The air bases (`WorldPlan::bases`).
    pub bases: &'a [AirBase],
    /// Its tuning (street and pavement sizes, kerb height, bases).
    pub cfg: &'a WorldConfig,
    /// The ground as it is now (craters included), and its walls.
    pub ground: &'a dyn Ground,
    /// Street doors to run for.
    pub doors: &'a dyn Doors,
}

impl People {
    /// Nobody yet, for the world of `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            root: SimRng::new(seed).fork("people"),
            active: BTreeSet::new(),
            visits: BTreeMap::new(),
            next_id: 0,
            checked_at: None,
            pending: false,
        }
    }

    /// Lets go of people farther than `keep_radius_m` from the pilot at `pilot` (x, z), and peoples the
    /// blocks, parks and bases within `spawn_radius_m` that have nobody yet, nearest first, up to
    /// `spawn_per_step` people this call and `max_people` in all. Returns how many were brought in.
    pub fn keep_around(&mut self, w: &World, data: &PeopleData, pilot: Vec2) -> usize {
        let cfg = &data.cfg;
        if !pilot.is_finite() {
            return 0;
        }
        let keep2 = cfg.keep_radius_m * cfg.keep_radius_m;
        self.list.retain(|p| p.pos.distance_squared(pilot) <= keep2);
        let far: Vec<Home> = self
            .active
            .iter()
            .copied()
            .filter(|h| home_rect(w, *h).is_none_or(|r| r.distance(pilot) > cfg.keep_radius_m))
            .collect();
        for h in far {
            self.active.remove(&h);
        }
        let moved = self.checked_at.is_none_or(|c| c.distance(pilot) >= cfg.recheck_m);
        if !(moved || self.pending) {
            return 0;
        }
        self.checked_at = Some(pilot);
        self.pending = false;
        let mut wanted: Vec<(f32, Home)> = Vec::new();
        for (ci, city) in w.cities.iter().enumerate() {
            if city.rect().distance(pilot) > cfg.spawn_radius_m {
                continue;
            }
            for (bi, b) in city.blocks.iter().enumerate() {
                let d = b.rect.distance(pilot);
                let home = Home::Block(ci as u32, bi as u32);
                if d <= cfg.spawn_radius_m && !self.active.contains(&home) {
                    wanted.push((d, home));
                }
            }
        }
        for base in w.bases {
            let d = base.rect().distance(pilot);
            let home = Home::Base(base.id);
            if d <= cfg.spawn_radius_m && !self.active.contains(&home) {
                wanted.push((d, home));
            }
        }
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut brought = 0;
        for (_, home) in wanted {
            if self.list.len() >= cfg.max_people {
                break;
            }
            if brought >= cfg.spawn_per_step {
                self.pending = true;
                break;
            }
            brought += self.people_home(w, data, home);
            self.active.insert(home);
        }
        brought
    }

    /// Brings in a home's people; returns how many.
    fn people_home(&mut self, w: &World, data: &PeopleData, home: Home) -> usize {
        let cfg = &data.cfg;
        let Some((rect, place)) = routes::home_loop(w.cities, w.bases, w.cfg, cfg.park_path_inset_m, cfg.base_apron_depth_m, home) else {
            return 0;
        };
        let visit = {
            let v = self.visits.entry(home).or_default();
            *v += 1;
            *v
        };
        let mut rng = self.root.fork(&format!("{home:?}/{visit}"));
        let count = rng_count(&mut rng, home_count(w, cfg, home, &rect, place));
        let room = cfg.max_people.saturating_sub(self.list.len());
        let block = match home {
            Home::Block(_, b) => b,
            Home::Base(_) => 0,
        };
        for i in 0..count.min(room) {
            let mut r = rng.fork(&format!("person/{i}"));
            let Some(role) = data.lines.draw_role(place, &mut r) else { break };
            let k0 = (r.next_u64() % u64::from(LAP)) as u8;
            let t = r.next_f32();
            let forward = r.chance(0.5);
            let (a, b) = (routes::waypoint(&rect, k0), routes::waypoint(&rect, k0 + 1));
            let pos = a.lerp(b, t);
            let (k, step) = if forward { ((k0 + 1) % LAP, 1) } else { (k0, -1) };
            let mut p = Person {
                id: self.next_id,
                home,
                place,
                role,
                pos,
                feet_y: 0.0,
                heading_rad: 0.0,
                height_m: r.range_f32(cfg.height_m[0], cfg.height_m[1]),
                walk_m_s: r.range_f32(cfg.walk_speed_m_s[0], cfg.walk_speed_m_s[1]),
                run_m_s: r.range_f32(cfg.run_speed_m_s[0], cfg.run_speed_m_s[1]),
                pace_m_s: 0.0,
                look: r.next_u64() as u32,
                activity: Activity::Walking,
                greeted: 0,
                block,
                leg: Leg::Loop { rect, k, step },
                rng: r,
            };
            p.heading_rad = heading_of(routes::waypoint(&rect, k) - pos).unwrap_or(0.0);
            p.feet_y = feet_y(w, &p);
            self.next_id += 1;
            self.list.push(p);
        }
        count.min(room)
    }

    /// One fixed step of `dt_s` seconds: walkers walk, waits count down, the fleeing run, the struck
    /// are cleared away when their time is up, and anyone who reached a street door goes inside.
    pub fn step(&mut self, w: &World, data: &PeopleData, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let cfg = &data.cfg;
        let mut gone = Vec::new();
        for (i, p) in self.list.iter_mut().enumerate() {
            let before = p.pos;
            if step_person(p, w, cfg, dt_s) {
                gone.push(i);
            }
            p.pace_m_s = before.distance(p.pos) / dt_s;
            p.feet_y = feet_y(w, p);
        }
        for i in gone.into_iter().rev() {
            self.list.remove(i);
        }
    }

    /// Applies a danger: people it reaches run from it, and those within its lethal reach fall.
    /// Returns who was killed, in list order.
    pub fn danger(&mut self, w: &World, data: &PeopleData, d: &Danger) -> Vec<Casualty> {
        let cfg = &data.cfg;
        let mut dead = Vec::new();
        // The horizontal box anything this danger does lies inside, to skip the rest quickly.
        let (lo, hi) = match *d {
            Danger::Shot { from, to, .. } => {
                let (a, b) = (Vec2::new(from.x, from.z), Vec2::new(to.x, to.z));
                (a.min(b) - Vec2::splat(cfg.shot_fright_m), a.max(b) + Vec2::splat(cfg.shot_fright_m))
            }
            Danger::Blast { at, energy_mj, .. } => {
                let r = cfg.blast_fright_m_per_cbrt_mj.max(cfg.blast_lethal_m_per_cbrt_mj) * energy_mj.max(0.0).cbrt();
                let c = Vec2::new(at.x, at.z);
                (c - Vec2::splat(r), c + Vec2::splat(r))
            }
            Danger::Footfall { at, .. } => {
                let r = cfg.footfall_fright_m.max(cfg.foot_lethal_m);
                let c = Vec2::new(at.x, at.z);
                (c - Vec2::splat(r), c + Vec2::splat(r))
            }
        };
        if !(lo.is_finite() && hi.is_finite()) {
            return dead;
        }
        for p in &mut self.list {
            if !p.standing() || p.pos.cmplt(lo).any() || p.pos.cmpgt(hi).any() {
                continue;
            }
            let foot = p.feet() + Vec3::Y * 0.1;
            let head = p.feet() + Vec3::Y * p.height_m;
            let (kill, fright, from, cause, by) = match *d {
                Danger::Shot { from, to, by } => {
                    let near = segment_distance(from, to, foot, head);
                    let (a, b) = (Vec2::new(from.x, from.z), Vec2::new(to.x, to.z));
                    let c = closest_on_segment(a, b, p.pos);
                    (
                        near <= cfg.shot_lethal_m,
                        c.distance(p.pos) <= cfg.shot_fright_m,
                        c,
                        Cause::Shot,
                        by,
                    )
                }
                Danger::Blast { at, energy_mj, by } => {
                    let r = energy_mj.max(0.0).cbrt();
                    let near = closest_on_segment3(foot, head, at).distance(at);
                    (
                        near <= cfg.blast_lethal_m_per_cbrt_mj * r,
                        near <= cfg.blast_fright_m_per_cbrt_mj * r,
                        Vec2::new(at.x, at.z),
                        Cause::Blast,
                        by,
                    )
                }
                Danger::Footfall { at, by } => {
                    let c = Vec2::new(at.x, at.z);
                    let near = c.distance(p.pos);
                    (near <= cfg.foot_lethal_m, near <= cfg.footfall_fright_m, c, Cause::Crushed, by)
                }
            };
            if kill {
                p.activity = Activity::Down { left_s: cfg.down_s };
                dead.push(Casualty {
                    person: p.id,
                    at: p.feet(),
                    cause,
                    by,
                });
            } else if fright {
                frighten(p, w, cfg, from);
            }
        }
        dead
    }

    /// The pilot, feet at `pilot` facing `yaw_rad` (0 facing -z, positive turning left), presses Use:
    /// the nearest standing, unfrightened person within `greet_reach_m` and `greet_cone_deg` of where
    /// they face stops, turns to them and answers. `job` names an open job a line may tell of.
    /// Returns the person's id and their answer, or `None` when nobody is in reach.
    pub fn greet(&mut self, data: &PeopleData, pilot: Vec3, yaw_rad: f32, job: Option<&str>) -> Option<(u32, Answer)> {
        let i = self.in_reach(&data.cfg, pilot, yaw_rad)?;
        let p = &mut self.list[i];
        let answer = data.lines.answer(p.role, p.id, p.greeted, job)?;
        p.greeted += 1;
        p.activity = Activity::Greeting {
            left_s: data.cfg.greet_hold_s,
            face: Vec2::new(pilot.x, pilot.z),
        };
        Some((p.id, answer))
    }

    /// The one rule for who a greeting reaches: the index of the nearest standing, unfrightened person
    /// within `greet_reach_m` of the pilot's feet `pilot` and `greet_cone_deg` of where they face
    /// (`yaw_rad`), if any. The prompt shows it; `greet` greets it.
    pub fn in_reach(&self, cfg: &PeopleConfig, pilot: Vec3, yaw_rad: f32) -> Option<usize> {
        let at = Vec2::new(pilot.x, pilot.z);
        let fwd = Vec2::new(-yaw_rad.sin(), -yaw_rad.cos());
        let cos_cone = cfg.greet_cone_deg.to_radians().cos();
        let mut best: Option<(f32, usize)> = None;
        for (i, p) in self.list.iter().enumerate() {
            if matches!(p.activity, Activity::Down { .. } | Activity::Fleeing { .. }) {
                continue;
            }
            let v = p.pos - at;
            let d = v.length();
            if d > cfg.greet_reach_m || (d > 0.3 && v.dot(fwd) / d < cos_cone) {
                continue;
            }
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    /// How many people stand within `r` of `p`.
    pub fn count_within(&self, p: Vec2, r: f32) -> usize {
        self.list.iter().filter(|q| q.standing() && q.pos.distance(p) <= r).count()
    }
}

/// The rectangle a home's people are brought in from (a block's or a base's ground).
fn home_rect(w: &World, home: Home) -> Option<Rect> {
    match home {
        Home::Block(c, b) => Some(w.cities.get(c as usize)?.blocks.get(b as usize)?.rect),
        Home::Base(id) => w.bases.iter().find(|b| b.id == id).map(|b| b.rect()),
    }
}

/// How many people a home holds, as a real number (`rng_count` rounds it): pavements by the length of
/// their loop and their district, parks by area and district, bases a fixed number.
pub fn home_count(w: &World, cfg: &PeopleConfig, home: Home, rect: &Rect, place: Place) -> f32 {
    let district = |c: u32, at: Vec2| {
        let Some(city) = w.cities.get(c as usize) else { return 0.0 };
        let d = (at.distance(city.centre) / city.half_size_m).clamp(0.0, 1.0);
        let [heart, edge] = cfg.pavement_per_100m;
        heart + (edge - heart) * d.powf(cfg.density_exponent)
    };
    match (home, place) {
        (Home::Base(_), _) => cfg.base_people as f32,
        (Home::Block(c, _), Place::Park) => {
            let share = if cfg.pavement_per_100m[0] > 0.0 {
                district(c, rect.centre()) / cfg.pavement_per_100m[0]
            } else {
                0.0
            };
            let s = rect.size();
            s.x * s.y / 10_000.0 * cfg.park_per_hectare * share
        }
        (Home::Block(c, _), _) => {
            let s = rect.size();
            2.0 * (s.x + s.y) / 100.0 * district(c, rect.centre())
        }
    }
}

/// `x` rounded down, plus one more with the chance of its fraction.
fn rng_count(rng: &mut SimRng, x: f32) -> usize {
    if !(x.is_finite() && x > 0.0) {
        return 0;
    }
    let whole = x.floor();
    whole as usize + usize::from(rng.chance(x - whole))
}

/// The heading that faces `d`, if `d` is not zero.
fn heading_of(d: Vec2) -> Option<f32> {
    (d.length_squared() > 1e-8).then(|| (-d.x).atan2(-d.y))
}

/// Feet height: the ground, raised by the kerb on a city block's pavement.
fn feet_y(w: &World, p: &Person) -> f32 {
    let ground = w.ground.height_at(p.pos.x, p.pos.y);
    let lift = match p.home {
        Home::Base(_) => 0.0,
        Home::Block(c, _) => match w.cities.get(c as usize).and_then(|c| c.blocks.get(p.block as usize)) {
            Some(b) if b.rect.distance(p.pos) <= 0.0 => w.cfg.kerb_height_m,
            _ => w.cfg.road_lift_m,
        },
    };
    ground + lift
}

/// The city a pavement or park person lives in.
fn city_of(p: &Person) -> Option<u32> {
    match p.home {
        Home::Block(c, _) => Some(c),
        Home::Base(_) => None,
    }
}

/// Moves `p` toward `target` at `speed`; true on arriving.
fn walk_to(p: &mut Person, target: Vec2, speed: f32, dt_s: f32) -> bool {
    let d = target - p.pos;
    let len = d.length();
    if let Some(h) = heading_of(d) {
        p.heading_rad = h;
    }
    if len <= speed * dt_s {
        p.pos = target;
        true
    } else {
        p.pos += d / len * speed * dt_s;
        false
    }
}

/// Steps one person; true when they leave the world (cleared away, or gone indoors).
fn step_person(p: &mut Person, w: &World, cfg: &PeopleConfig, dt_s: f32) -> bool {
    match p.activity {
        Activity::Down { left_s } => {
            let left_s = left_s - dt_s;
            p.activity = Activity::Down { left_s };
            return left_s <= 0.0;
        }
        Activity::Greeting { left_s, face } => {
            if let Some(h) = heading_of(face - p.pos) {
                p.heading_rad = h;
            }
            let left_s = left_s - dt_s;
            p.activity = if left_s <= 0.0 {
                Activity::Walking
            } else {
                Activity::Greeting { left_s, face }
            };
        }
        Activity::Waiting { left_s } => {
            let left_s = left_s - dt_s;
            p.activity = if left_s <= 0.0 {
                Activity::Walking
            } else {
                Activity::Waiting { left_s }
            };
        }
        Activity::Idle { left_s } => {
            let left_s = left_s - dt_s;
            p.activity = if left_s <= 0.0 {
                Activity::Walking
            } else {
                Activity::Idle { left_s }
            };
        }
        Activity::Sitting { left_s } => {
            let left_s = left_s - dt_s;
            p.activity = if left_s <= 0.0 {
                Activity::Walking
            } else {
                Activity::Sitting { left_s }
            };
        }
        Activity::Fleeing { left_s, dir, door } => {
            if let Some(door) = door {
                if walk_to(p, door, p.run_m_s, dt_s) {
                    return true;
                }
            } else {
                let to = p.pos + dir * p.run_m_s * dt_s;
                p.pos = w.ground.push_out(to, 0.3);
                if let Some(h) = heading_of(dir) {
                    p.heading_rad = h;
                }
                track_block(p, w);
            }
            let left_s = left_s - dt_s;
            if left_s > 0.0 {
                p.activity = Activity::Fleeing { left_s, dir, door };
            } else {
                rejoin(p, w, cfg, dir);
            }
        }
        Activity::Walking => walk(p, w, cfg, dt_s),
    }
    false
}

/// Keeps a pavement person's block the one under their feet, as they run across streets.
fn track_block(p: &mut Person, w: &World) {
    let Some(city) = city_of(p).and_then(|c| w.cities.get(c as usize)) else {
        return;
    };
    if city.blocks.get(p.block as usize).is_some_and(|b| b.rect.distance(p.pos) <= 0.0) {
        return;
    }
    if let Some(i) = city.blocks.iter().position(|b| b.rect.distance(p.pos) <= 0.0) {
        p.block = i as u32;
    }
}

/// After a fright, back to a route: the nearest pavement within 6 m, or their park path or apron walk
/// within 30 m; otherwise on the way they were running, at a walk.
fn rejoin(p: &mut Person, w: &World, cfg: &PeopleConfig, dir: Vec2) {
    p.activity = Activity::Walking;
    if p.place == Place::Pavement
        && let Some(city) = city_of(p).and_then(|c| w.cities.get(c as usize))
        && let Some((b, k)) = routes::pavement_near(city, w.cfg, p.pos, 6.0)
        && let Some(rect) = city.blocks[b as usize].rect.inset(w.cfg.sidewalk_m * 0.5)
    {
        p.block = b;
        let step = if p.rng.chance(0.5) { 1 } else { -1 };
        p.leg = Leg::Loop { rect, k, step };
        return;
    }
    if p.place != Place::Pavement
        && let Some((rect, _)) = routes::home_loop(w.cities, w.bases, w.cfg, cfg.park_path_inset_m, cfg.base_apron_depth_m, p.home)
        && rect.distance(p.pos) <= 30.0
    {
        let k = routes::nearest_waypoint(&rect, p.pos);
        p.leg = Leg::Loop { rect, k, step: 1 };
        return;
    }
    p.leg = Leg::Straight { dir };
}

/// A walker's step along their leg, and what they do on reaching its end.
fn walk(p: &mut Person, w: &World, cfg: &PeopleConfig, dt_s: f32) {
    match p.leg {
        Leg::Straight { dir } => {
            let to = p.pos + dir * p.walk_m_s * dt_s;
            p.pos = w.ground.push_out(to, 0.3);
            if let Some(h) = heading_of(dir) {
                p.heading_rad = h;
            }
            track_block(p, w);
        }
        Leg::Crossing { to, block, k } => {
            if walk_to(p, to, p.walk_m_s, dt_s) {
                p.block = block;
                let rect = city_of(p)
                    .and_then(|c| w.cities.get(c as usize))
                    .and_then(|c| c.blocks.get(block as usize))
                    .and_then(|b| b.rect.inset(w.cfg.sidewalk_m * 0.5));
                p.leg = match rect {
                    Some(rect) => Leg::Loop {
                        rect,
                        k,
                        step: if p.rng.chance(0.5) { 1 } else { -1 },
                    },
                    None => Leg::Straight {
                        dir: Vec2::new(-p.heading_rad.sin(), -p.heading_rad.cos()),
                    },
                };
            }
        }
        Leg::Loop { rect, k, step } => {
            if !walk_to(p, routes::waypoint(&rect, k), p.walk_m_s, dt_s) {
                return;
            }
            arrive(p, w, cfg, rect, k, step);
        }
    }
}

/// A walker reached waypoint `k` of their loop: cross, sit, stand a while, or walk on.
fn arrive(p: &mut Person, w: &World, cfg: &PeopleConfig, rect: Rect, k: u8, step: i8) {
    let next = ((i16::from(k) + i16::from(step)).rem_euclid(i16::from(LAP))) as u8;
    p.leg = Leg::Loop { rect, k: next, step };
    let corner = k.is_multiple_of(2);
    match p.place {
        Place::Pavement if corner => {
            if !p.rng.chance(cfg.cross_chance) {
                return;
            }
            let out = routes::corner_outs(k)[(p.rng.next_u64() % 2) as usize];
            let Some(city) = city_of(p).and_then(|c| w.cities.get(c as usize)) else {
                return;
            };
            if let Some((block, to, k2)) = routes::crossing(city, w.cfg, p.block, k, out) {
                p.leg = Leg::Crossing { to, block, k: k2 };
                p.activity = Activity::Waiting {
                    left_s: p.rng.range_f32(cfg.crossing_wait_s[0], cfg.crossing_wait_s[1]),
                };
                if let Some(h) = heading_of(out) {
                    p.heading_rad = h;
                }
            }
        }
        Place::Park if !corner => {
            if p.rng.chance(cfg.bench_chance) {
                let (seat, facing) = routes::park_benches(&rect)[usize::from(k / 2)];
                p.pos = seat;
                p.heading_rad = heading_of(facing).unwrap_or(p.heading_rad);
                p.activity = Activity::Sitting {
                    left_s: p.rng.range_f32(cfg.bench_sit_s[0], cfg.bench_sit_s[1]),
                };
            }
        }
        Place::Pavement | Place::Base => {
            if (p.place == Place::Base || !corner) && p.rng.chance(cfg.idle_chance) {
                p.activity = Activity::Idle {
                    left_s: p.rng.range_f32(cfg.idle_s[0], cfg.idle_s[1]),
                };
            }
        }
        Place::Park => {}
    }
}

/// Sends `p` running from a danger at `from` (x, z): for a street door when one is in reach and not
/// toward the danger, else along their pavement's line (or straight away in the open).
fn frighten(p: &mut Person, w: &World, cfg: &PeopleConfig, from: Vec2) {
    let left = p.rng.range_f32(cfg.flee_s[0], cfg.flee_s[1]);
    let on_line = match (p.activity, p.leg) {
        (Activity::Walking | Activity::Waiting { .. } | Activity::Idle { .. } | Activity::Greeting { .. }, Leg::Loop { rect, .. })
            if p.place == Place::Pavement =>
        {
            Some(rect)
        }
        _ => None,
    };
    let dir = routes::flee_dir(p.pos, from, on_line.as_ref());
    let door = w.doors.refuge(p.pos, from, cfg.door_reach_m);
    let left_s = match p.activity {
        Activity::Fleeing { left_s, .. } => left_s.max(left),
        _ => left,
    };
    p.activity = Activity::Fleeing { left_s, dir, door };
}

/// The point of segment `a`-`b` nearest `p`.
fn closest_on_segment(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 <= 1e-12 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
}

/// The point of segment `a`-`b` nearest `p`, in 3D.
fn closest_on_segment3(a: Vec3, b: Vec3, p: Vec3) -> Vec3 {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 <= 1e-12 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
}

/// The least distance between segments `p1`-`q1` and `p2`-`q2` (Ericson, Real-Time Collision
/// Detection, 5.1.9).
pub fn segment_distance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.length_squared(), d2.length_squared(), d2.dot(r));
    let eps = 1e-9;
    let (s, t) = if a <= eps && e <= eps {
        (0.0, 0.0)
    } else if a <= eps {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= eps {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s = if denom > eps {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (p1 + d1 * s).distance(p2 + d2 * t)
}
