//! The invasion's vehicles (openspec change `invasion-forces`): tanks, helicopters, fighter jets and
//! the cargo planes that drop tanks by parachute. Each unit is a point with a heading, a speed and a
//! simple brain, pure and tested; it is hit through the one `weapons::fly` (a capsule per unit) and
//! fires `WeaponSpec`s like a mech's guns. Numbers live in `data/world/units.ron`; the crab airship
//! and air-dropped crab mechs come later in the same change.

use crate::data::{Validate, finite3, positive, positive_range};
use crate::weapons::WeaponSpec;
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

const G_M_S2: f32 = 9.81;

/// What a unit is and how its brain flies or drives it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum UnitKind {
    /// Drives to its standoff range, stops and fires; moves on when hit.
    Tank,
    /// Circles the target at its standoff radius and fires salvos when nosed on.
    Helicopter,
    /// Attack runs: in at the target, firing when nosed on, then out and round again.
    Fighter,
    /// Flies the drop line at its height, releases its cargo over the drop point, and leaves.
    CargoPlane,
    /// Rides the sea's surface (openspec change `harbours`): holds station until its target comes within twice its
    /// fire range, then closes to its standoff and circles there, its turret firing within its cone, keeping off
    /// water shallower than its draught.
    Gunboat,
}

/// One kind of unit, in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnitSpec {
    /// Its brain.
    pub kind: UnitKind,
    /// Placeholder size, x across, y up, z along, metres.
    pub size_m: [f32; 3],
    /// Cruising speed.
    pub speed_m_s: f32,
    /// Most it turns, degrees a second.
    pub turn_deg_s: f32,
    /// Structure points.
    pub hp: f32,
    /// Armour a shot must beat to do damage, millimetres of rolled steel equivalent.
    pub armour_mm: f32,
    /// Its gun, if it has one.
    #[serde(default)]
    pub weapon: Option<WeaponSpec>,
    /// Shots in a salvo, fired `interval_s` apart, then a pause of `salvo_pause_s`.
    #[serde(default = "one")]
    pub salvo: u32,
    /// Pause between salvos.
    #[serde(default)]
    pub salvo_pause_s: f32,
    /// It fires when the target is within this range...
    pub fire_range_m: f32,
    /// ...and within this angle of its nose, degrees.
    pub fire_cone_deg: f32,
    /// Tank: the range it stops at. Helicopter: the radius it circles at. Fighter: the range it
    /// breaks off its run at. Cargo plane: unused.
    pub standoff_m: f32,
    /// Aircraft: the band of height above the ground it flies in, [low, high].
    #[serde(default)]
    pub altitude_m: [f32; 2],
    /// Fighter: how long it flies out after a run before turning back.
    #[serde(default)]
    pub extend_s: f32,
    /// Hull colour, sRGB 0..1.
    pub tint: [f32; 3],
    /// Gunboat: how deep its keel sits under the waterline, metres.
    #[serde(default)]
    pub draft_m: f32,
}

fn one() -> u32 {
    1
}

/// Parachute drops.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DropConfig {
    /// Free fall after release before the chute opens.
    pub chute_open_s: f32,
    /// Descent under the open chute.
    pub descent_m_s: f32,
    /// Sideways drift under the chute, as a share of the plane's speed it keeps.
    pub carry: f32,
    /// A cargo plane's load is released over this long, spread along its line.
    pub release_spread_s: f32,
}

impl Default for DropConfig {
    fn default() -> Self {
        Self {
            chute_open_s: 2.0,
            descent_m_s: 8.0,
            carry: 0.15,
            release_spread_s: 3.0,
        }
    }
}

/// `data/world/units.ron`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct UnitsConfig {
    /// Every kind of unit by name.
    pub units: BTreeMap<String, UnitSpec>,
    /// Parachute drops.
    pub drop: DropConfig,
}

impl Validate for UnitsConfig {
    fn validate(&self) -> Result<(), String> {
        for (name, u) in &self.units {
            let f = |field: &str| format!("units.{name}.{field}");
            finite3(&f("size_m"), u.size_m)?;
            for (i, s) in u.size_m.iter().enumerate() {
                positive(&f(&format!("size_m[{i}]")), *s)?;
            }
            positive(&f("speed_m_s"), u.speed_m_s)?;
            positive(&f("turn_deg_s"), u.turn_deg_s)?;
            positive(&f("hp"), u.hp)?;
            if !(u.armour_mm >= 0.0 && u.armour_mm.is_finite()) {
                return Err(format!("`{}` must be zero or more", f("armour_mm")));
            }
            positive(&f("fire_range_m"), u.fire_range_m)?;
            positive(&f("fire_cone_deg"), u.fire_cone_deg)?;
            positive(&f("standoff_m"), u.standoff_m)?;
            finite3(&f("tint"), u.tint)?;
            if u.salvo == 0 {
                return Err(format!("`{}` must be above zero", f("salvo")));
            }
            if matches!(u.kind, UnitKind::Helicopter | UnitKind::Fighter | UnitKind::CargoPlane) {
                positive_range(&f("altitude_m"), u.altitude_m)?;
            }
            if u.kind == UnitKind::Fighter {
                positive(&f("extend_s"), u.extend_s)?;
            }
            if u.kind == UnitKind::Gunboat {
                positive(&f("draft_m"), u.draft_m)?;
                if u.draft_m >= u.size_m[1] {
                    return Err(format!("`{}` must be less than its height", f("draft_m")));
                }
            }
            if let Some(w) = &u.weapon {
                positive(&f("weapon.speed_m_s"), w.speed_m_s)?;
                positive(&f("weapon.interval_s"), w.interval_s)?;
            }
        }
        positive("drop.chute_open_s", self.drop.chute_open_s)?;
        positive("drop.descent_m_s", self.drop.descent_m_s)?;
        positive("drop.release_spread_s", self.drop.release_spread_s)?;
        if !(0.0..=1.0).contains(&self.drop.carry) {
            return Err("`drop.carry` must be within 0..1".into());
        }
        Ok(())
    }
}

/// The way a unit faces: heading 0 looks along -z, turning left is positive (Bevy's `Quat::from_rotation_y`).
pub fn forward(heading_rad: f32) -> Vec3 {
    Vec3::new(-heading_rad.sin(), 0.0, -heading_rad.cos())
}

/// The heading that faces along `dir` (its horizontal part).
pub fn heading_of(dir: Vec3) -> f32 {
    (-dir.x).atan2(-dir.z)
}

/// An angle wrapped into `[-pi, pi)`.
pub fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}

/// Turns `heading` toward `want` by at most `max_rad`.
pub fn turn_toward(heading: f32, want: f32, max_rad: f32) -> f32 {
    heading + wrap(want - heading).clamp(-max_rad, max_rad)
}

/// The one armour rule, one copy for every vehicle and aircraft with a single armour value (units, boats, civil
/// aircraft: `civil-air-traffic`, `harbours`): a round that beats `armour_mm` takes `damage` from `hp`; one that does
/// not takes nothing. True when it leaves `hp` at zero or below.
pub fn armour_hit(hp: &mut f32, penetration_mm: f32, damage: f32, armour_mm: f32) -> bool {
    if penetration_mm >= armour_mm {
        *hp -= damage;
    }
    *hp <= 0.0
}

/// Where a unit is in its life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Falling from a plane: `t_s` since release.
    Dropping {
        /// Time since release.
        t_s: f32,
    },
    /// Fighting.
    Active,
    /// A cargo plane that has dropped its load and is leaving.
    Leaving,
    /// Destroyed.
    Dead,
}

/// Cargo carried by a plane: units to release over the drop point.
#[derive(Clone, Debug, PartialEq)]
pub struct Cargo {
    /// The units' kinds, released in order.
    pub load: Vec<String>,
    /// Where they are to land (the plane releases them upwind of it by the drift).
    pub drop_at: Vec2,
    /// Released so far.
    pub released: usize,
}

/// A unit in the world.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    /// Stable id (hits, targeting, missiles).
    pub id: u32,
    /// Its kind's name in `UnitsConfig::units`.
    pub kind: String,
    /// Centre of its body.
    pub pos: Vec3,
    /// Velocity.
    pub vel: Vec3,
    /// Facing (see `forward`).
    pub heading_rad: f32,
    /// Structure points left.
    pub hp: f32,
    /// Where it is in its life.
    pub phase: Phase,
    /// Time until it may fire again.
    pub reload_s: f32,
    /// Shots left in the current salvo.
    pub salvo_left: u32,
    /// Fighter: time left flying out after a run (0 while attacking).
    pub extend_left_s: f32,
    /// Helicopter: which way it circles (+1 left, -1 right).
    pub circle: f32,
    /// A cargo plane's load.
    pub cargo: Option<Cargo>,
    /// Hit this step (the brain moves a tank on when hit).
    pub took_hit: bool,
}

/// What the world tells a unit each step.
pub struct UnitView<'a> {
    /// Its kind's data.
    pub spec: &'a UnitSpec,
    /// The target's centre (the pilot's mech), if any.
    pub target: Option<Vec3>,
    /// The target's velocity (for the lead).
    pub target_vel: Vec3,
    /// Something else it may shoot at when the target is not its shot: a civil aircraft in reach
    /// (where, velocity), `civil-air-traffic`.
    pub opportunity: Option<(Vec3, Vec3)>,
    /// Ground height under a point.
    pub ground: &'a dyn Fn(f32, f32) -> f32,
    /// Drops.
    pub drop: &'a DropConfig,
    /// The sea's level, metres (a gunboat rides it).
    pub sea_level_m: f32,
}

/// What a unit did this step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnitStep {
    /// A shot, from its muzzle with its aim direction.
    pub fire: Option<(Vec3, Vec3)>,
    /// Units this cargo plane released: (kind, where, velocity).
    pub released: Vec<(String, Vec3, Vec3)>,
    /// It landed under its chute this step.
    pub landed: bool,
}

impl Unit {
    /// A new unit facing `heading_rad` at `pos`, flying or driving at its cruising speed.
    pub fn new(id: u32, kind: &str, spec: &UnitSpec, pos: Vec3, heading_rad: f32, salt: u32) -> Self {
        let moving = spec.kind != UnitKind::Tank;
        Self {
            id,
            kind: kind.into(),
            pos,
            vel: if moving {
                forward(heading_rad) * spec.speed_m_s
            } else {
                Vec3::ZERO
            },
            heading_rad,
            hp: spec.hp,
            phase: Phase::Active,
            reload_s: 0.0,
            salvo_left: spec.salvo,
            extend_left_s: 0.0,
            circle: if salt.is_multiple_of(2) { 1.0 } else { -1.0 },
            cargo: None,
            took_hit: false,
        }
    }

    /// A unit released from a plane at `pos` with the plane's velocity: it falls, opens its chute
    /// and lands, and only then starts its brain.
    pub fn dropped(id: u32, kind: &str, spec: &UnitSpec, pos: Vec3, vel: Vec3) -> Self {
        let mut u = Self::new(id, kind, spec, pos, heading_of(vel), id);
        u.vel = vel;
        u.phase = Phase::Dropping { t_s: 0.0 };
        u
    }

    /// True while it fights or falls (a target for the pilot).
    pub fn alive(&self) -> bool {
        !matches!(self.phase, Phase::Dead)
    }

    /// Takes a hit: damage when the shot beats its armour (`armour_hit`). Returns true if this hit destroyed it.
    pub fn hit(&mut self, penetration_mm: f32, damage: f32, armour_mm: f32) -> bool {
        if !self.alive() {
            return false;
        }
        self.took_hit = true;
        if armour_hit(&mut self.hp, penetration_mm, damage, armour_mm) {
            self.phase = Phase::Dead;
            return true;
        }
        false
    }

    /// Where a gunboat's body centre rides on a sea at `sea_level_m`: its keel `draft_m` under the surface.
    pub fn riding_y(spec: &UnitSpec, sea_level_m: f32) -> f32 {
        sea_level_m - spec.draft_m + spec.size_m[1] * 0.5
    }

    /// One fixed step of `dt_s`.
    pub fn step(&mut self, view: &UnitView, dt_s: f32) -> UnitStep {
        let mut out = UnitStep::default();
        let spec = view.spec;
        match self.phase {
            Phase::Dead => return out,
            Phase::Dropping { t_s } => {
                let t = t_s + dt_s;
                if t < view.drop.chute_open_s {
                    self.vel.y -= G_M_S2 * dt_s;
                } else {
                    // Under the chute: the fall eases to the descent speed and the drift dies down.
                    let h = Vec2::new(self.vel.x, self.vel.z) * (1.0 - (dt_s * 1.5).min(1.0));
                    let floor = view.drop.descent_m_s;
                    let vy = self.vel.y + (-floor - self.vel.y) * (dt_s * 3.0).min(1.0);
                    self.vel = Vec3::new(h.x, vy, h.y);
                }
                self.pos += self.vel * dt_s;
                let ground = (view.ground)(self.pos.x, self.pos.z) + spec.size_m[1] * 0.5;
                if self.pos.y <= ground {
                    self.pos.y = ground;
                    self.vel = Vec3::ZERO;
                    self.phase = Phase::Active;
                    out.landed = true;
                } else {
                    self.phase = Phase::Dropping { t_s: t };
                }
                return out;
            }
            Phase::Active | Phase::Leaving => {}
        }
        self.reload_s = (self.reload_s - dt_s).max(0.0);
        let turn = spec.turn_deg_s.to_radians() * dt_s;
        match spec.kind {
            UnitKind::Tank => self.drive(view, turn, dt_s),
            UnitKind::Helicopter => self.hover(view, turn, dt_s),
            UnitKind::Fighter => self.run_in(view, turn, dt_s),
            UnitKind::Gunboat => self.sail(view, turn, dt_s),
            UnitKind::CargoPlane => {
                out.released = self.fly_drop(view, dt_s);
                return out;
            }
        }
        out.fire = self.try_fire(view);
        self.took_hit = false;
        out
    }

    fn drive(&mut self, view: &UnitView, turn: f32, dt_s: f32) {
        let spec = view.spec;
        let Some(target) = view.target else {
            self.vel = Vec3::ZERO;
            return;
        };
        let to = target - self.pos;
        let flat = Vec3::new(to.x, 0.0, to.z);
        let far = flat.length() > spec.standoff_m;
        // Tracks turn in place: facing the target first, then driving. A hit tank drives on.
        let want = heading_of(flat);
        self.heading_rad = turn_toward(self.heading_rad, want, turn);
        let facing = wrap(want - self.heading_rad).abs() < 0.3;
        let speed = if (far || self.took_hit) && facing { spec.speed_m_s } else { 0.0 };
        self.vel = forward(self.heading_rad) * speed;
        self.pos += self.vel * dt_s;
        self.pos.y = (view.ground)(self.pos.x, self.pos.z) + spec.size_m[1] * 0.5;
    }

    fn sail(&mut self, view: &UnitView, turn: f32, dt_s: f32) {
        let spec = view.spec;
        let depth = |p: Vec2| view.sea_level_m - (view.ground)(p.x, p.y);
        let here = Vec2::new(self.pos.x, self.pos.z);
        let mut speed = 0.0;
        let mut want = self.heading_rad;
        if let Some(target) = view.target {
            let rel = here - Vec2::new(target.x, target.z);
            let r = rel.length().max(1.0);
            if r <= spec.fire_range_m * 2.0 {
                // Circle the target at the standoff (the helicopter's rule, on the water).
                let radial = rel / r;
                let tangent = Vec2::new(-radial.y, radial.x) * self.circle;
                let correct = (spec.standoff_m - r) / spec.standoff_m;
                let along = (tangent + radial * correct * 2.0).normalize_or(tangent);
                want = heading_of(Vec3::new(along.x, 0.0, along.y));
                speed = spec.speed_m_s;
            }
        }
        // Off the shallows: the side with more water, hard over.
        let reach = (self.vel.length() * 8.0).max(spec.size_m[2] * 3.0);
        let probe = |h: f32| {
            let f = forward(h);
            depth(here + Vec2::new(f.x, f.z) * reach)
        };
        let need = spec.draft_m + 1.0;
        if probe(want) < need {
            let (left, right) = (probe(want + 0.6), probe(want - 0.6));
            want += if left >= right { 1.2 } else { -1.2 };
            speed *= 0.5;
        }
        self.heading_rad = turn_toward(self.heading_rad, want, turn);
        let f = forward(self.heading_rad);
        let next = here + Vec2::new(f.x, f.z) * speed * dt_s;
        if depth(next) < spec.draft_m && depth(next) < depth(here) {
            speed = 0.0;
        }
        self.vel = f * speed;
        self.pos += self.vel * dt_s;
        self.pos.y = Self::riding_y(spec, view.sea_level_m);
    }

    fn altitude_hold(&mut self, view: &UnitView, want_agl: f32, climb_m_s: f32, dt_s: f32) {
        let ground = (view.ground)(self.pos.x, self.pos.z);
        let err = ground + want_agl - self.pos.y;
        self.vel.y = err.clamp(-climb_m_s, climb_m_s);
        let _ = dt_s;
    }

    fn hover(&mut self, view: &UnitView, turn: f32, dt_s: f32) {
        let spec = view.spec;
        let Some(target) = view.target else {
            self.pos += self.vel * dt_s;
            return;
        };
        // Circle the target at the standoff radius: aim for the point a quarter-turn ahead on the
        // circle, nose kept toward the target when it can fire.
        let rel = Vec2::new(self.pos.x - target.x, self.pos.z - target.z);
        let r = rel.length().max(1.0);
        let radial = rel / r;
        let tangent = Vec2::new(-radial.y, radial.x) * self.circle;
        let correct = (spec.standoff_m - r) / spec.standoff_m;
        let along = (tangent + radial * correct * 2.0).normalize_or(tangent);
        let move_dir = Vec3::new(along.x, 0.0, along.y);
        let to_target = Vec3::new(-rel.x, 0.0, -rel.y);
        let want_face = if r < spec.fire_range_m {
            heading_of(to_target)
        } else {
            heading_of(move_dir)
        };
        self.heading_rad = turn_toward(self.heading_rad, want_face, turn);
        let h = move_dir * spec.speed_m_s;
        let mid = (spec.altitude_m[0] + spec.altitude_m[1]) * 0.5;
        self.altitude_hold(view, mid, spec.speed_m_s * 0.3, dt_s);
        self.vel = Vec3::new(h.x, self.vel.y, h.z);
        self.pos += self.vel * dt_s;
    }

    fn run_in(&mut self, view: &UnitView, turn: f32, dt_s: f32) {
        let spec = view.spec;
        let Some(target) = view.target else {
            self.vel = forward(self.heading_rad) * spec.speed_m_s;
            self.pos += self.vel * dt_s;
            return;
        };
        let to = target - self.pos;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if self.extend_left_s > 0.0 {
            // Out after a run: straight on and climbing, then turn back in.
            self.extend_left_s = (self.extend_left_s - dt_s).max(0.0);
            self.altitude_hold(view, spec.altitude_m[1], spec.speed_m_s * 0.25, dt_s);
        } else {
            self.heading_rad = turn_toward(self.heading_rad, heading_of(flat), turn);
            // The dive: down toward the low edge of its band as it closes.
            self.altitude_hold(view, spec.altitude_m[0], spec.speed_m_s * 0.25, dt_s);
            if flat.length() < spec.standoff_m {
                self.extend_left_s = spec.extend_s;
            }
        }
        let h = forward(self.heading_rad) * spec.speed_m_s;
        self.vel = Vec3::new(h.x, self.vel.y, h.z);
        self.pos += self.vel * dt_s;
    }

    fn fly_drop(&mut self, view: &UnitView, dt_s: f32) -> Vec<(String, Vec3, Vec3)> {
        let spec = view.spec;
        let mut released = Vec::new();
        let ground = (view.ground)(self.pos.x, self.pos.z);
        let want_y = ground + spec.altitude_m[0];
        self.vel.y = (want_y - self.pos.y).clamp(-10.0, 10.0);
        let h = forward(self.heading_rad) * spec.speed_m_s;
        self.vel = Vec3::new(h.x, self.vel.y, h.z);
        self.pos += self.vel * dt_s;
        if let Some(c) = &mut self.cargo
            && c.released < c.load.len()
        {
            // Release so each load lands near the drop point: it keeps `carry` of the plane's
            // speed over the fall, so the plane lets go that far short of it. Loads go out
            // `release_spread_s / n` apart, centred on the point.
            let fall_s = view.drop.chute_open_s + spec.altitude_m[0] / view.drop.descent_m_s;
            let lead_m = spec.speed_m_s * view.drop.carry * fall_s.min(6.0);
            let n = c.load.len() as f32;
            let gap_m = spec.speed_m_s * view.drop.release_spread_s / n;
            let fwd = Vec2::new(h.x, h.z).normalize_or(Vec2::Y);
            let here = Vec2::new(self.pos.x, self.pos.z);
            let past = (here - c.drop_at).dot(fwd) + lead_m;
            let due = past + gap_m * (n - 1.0) * 0.5 - gap_m * c.released as f32;
            if due >= 0.0 {
                let carry = Vec3::new(h.x, 0.0, h.z) * view.drop.carry;
                released.push((c.load[c.released].clone(), self.pos - Vec3::Y * spec.size_m[1], carry));
                c.released += 1;
            }
            if c.released == c.load.len() {
                self.phase = Phase::Leaving;
            }
        }
        released
    }

    /// A shot at the target when nosed on and in range; failing that, a shot of opportunity at
    /// `view.opportunity` (a civil aircraft in its guns' reach: `civil-air-traffic`) by the same rule.
    fn try_fire(&mut self, view: &UnitView) -> Option<(Vec3, Vec3)> {
        if self.reload_s > 0.0 {
            return None;
        }
        let aimed = [view.target.map(|t| (t, view.target_vel)), view.opportunity]
            .into_iter()
            .flatten()
            .find_map(|(t, v)| self.aim_at(view.spec, t, v))?;
        let spec = view.spec;
        let w = spec.weapon.as_ref()?;
        self.salvo_left = self.salvo_left.saturating_sub(1);
        if self.salvo_left == 0 {
            self.salvo_left = spec.salvo;
            self.reload_s = w.interval_s + spec.salvo_pause_s;
        } else {
            self.reload_s = w.interval_s;
        }
        Some(aimed)
    }

    /// Its muzzle and the led aim at a target at `target` moving at `target_vel`, when in range and
    /// within its fire cone; the one firing rule.
    fn aim_at(&self, spec: &UnitSpec, target: Vec3, target_vel: Vec3) -> Option<(Vec3, Vec3)> {
        let w = spec.weapon.as_ref()?;
        let muzzle = self.pos + forward(self.heading_rad) * (spec.size_m[2] * 0.5) + Vec3::Y * (spec.size_m[1] * 0.3);
        if muzzle.distance(target) > spec.fire_range_m {
            return None;
        }
        let aim = crate::weapons::lead(muzzle, target, target_vel, w.speed_m_s, !w.rocket);
        let dir = (aim - muzzle).normalize_or_zero();
        let nose = forward(self.heading_rad);
        let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        if nose.angle_between(flat).to_degrees() > spec.fire_cone_deg {
            return None;
        }
        Some((muzzle, dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;

    fn cfg() -> UnitsConfig {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/units.ron");
        load_ron(&p).unwrap()
    }

    fn flat(_: f32, _: f32) -> f32 {
        0.0
    }

    fn run(u: &mut Unit, spec: &UnitSpec, target: Option<Vec3>, drop: &DropConfig, seconds: f32) -> Vec<UnitStep> {
        let ground = flat;
        let view = UnitView {
            spec,
            target,
            target_vel: Vec3::ZERO,
            opportunity: None,
            ground: &ground,
            drop,
            sea_level_m: 0.0,
        };
        (0..(seconds * 60.0) as usize).map(|_| u.step(&view, 1.0 / 60.0)).collect()
    }

    #[test]
    fn a_dropped_tank_lands_where_it_was_released_plus_drift() {
        let c = cfg();
        let spec = &c.units["tank"];
        let carry = Vec3::new(90.0 * c.drop.carry, 0.0, 0.0);
        let mut u = Unit::dropped(1, "tank", spec, Vec3::new(0.0, 400.0, 0.0), carry);
        let steps = run(&mut u, spec, None, &c.drop, 120.0);
        assert!(steps.iter().any(|s| s.landed), "never landed");
        assert_eq!(u.phase, Phase::Active);
        assert!((u.pos.y - spec.size_m[1] * 0.5).abs() < 0.01);
        // It drifts downwind of the release point, by less than the plane would have flown.
        assert!(u.pos.x > 0.0 && u.pos.x < 90.0 * 6.0, "{:?}", u.pos);
        assert!(u.pos.z.abs() < 0.01);
    }

    #[test]
    fn a_unit_killed_under_its_chute_never_lands() {
        let c = cfg();
        let spec = &c.units["tank"];
        let mut u = Unit::dropped(1, "tank", spec, Vec3::new(0.0, 400.0, 0.0), Vec3::ZERO);
        run(&mut u, spec, None, &c.drop, 3.0);
        assert!(u.hit(1000.0, spec.hp, spec.armour_mm));
        let steps = run(&mut u, spec, None, &c.drop, 120.0);
        assert!(steps.iter().all(|s| !s.landed));
        assert_eq!(u.phase, Phase::Dead);
    }

    #[test]
    fn a_shot_that_does_not_beat_the_armour_does_no_damage() {
        let c = cfg();
        let spec = &c.units["tank"];
        let mut u = Unit::new(1, "tank", spec, Vec3::ZERO, 0.0, 0);
        u.hit(spec.armour_mm - 1.0, 50.0, spec.armour_mm);
        assert_eq!(u.hp, spec.hp);
        u.hit(spec.armour_mm, 50.0, spec.armour_mm);
        assert_eq!(u.hp, spec.hp - 50.0);
    }

    #[test]
    fn a_tank_drives_to_its_standoff_range_and_stops_there() {
        let c = cfg();
        let spec = &c.units["tank"];
        let mut u = Unit::new(1, "tank", spec, Vec3::ZERO, 1.0, 0);
        let target = Vec3::new(0.0, 0.0, -2000.0);
        run(&mut u, spec, Some(target), &c.drop, 400.0);
        let d = Vec2::new(u.pos.x - target.x, u.pos.z - target.z).length();
        assert!(d <= spec.standoff_m + 1.0 && d > spec.standoff_m - spec.speed_m_s, "{d}");
        assert_eq!(u.vel, Vec3::ZERO);
    }

    #[test]
    fn a_helicopter_circles_its_target_inside_its_height_band() {
        let c = cfg();
        let spec = &c.units["helicopter"];
        let mut u = Unit::new(1, "helicopter", spec, Vec3::new(1500.0, 90.0, 0.0), 1.57, 0);
        run(&mut u, spec, Some(Vec3::ZERO), &c.drop, 90.0);
        let mut far = 0.0f32;
        for _ in 0..30 {
            run(&mut u, spec, Some(Vec3::ZERO), &c.drop, 1.0);
            far = far.max((Vec2::new(u.pos.x, u.pos.z).length() - spec.standoff_m).abs());
            assert!(
                u.pos.y >= spec.altitude_m[0] - 1.0 && u.pos.y <= spec.altitude_m[1] + 1.0,
                "{:?}",
                u.pos
            );
        }
        assert!(far < spec.standoff_m * 0.3, "{far}");
    }

    #[test]
    fn a_fighter_comes_back_for_another_run() {
        let c = cfg();
        let spec = &c.units["fighter"];
        let mut u = Unit::new(1, "fighter", spec, Vec3::new(0.0, 300.0, 3000.0), 0.0, 0);
        let mut closest = Vec::new();
        let mut d_prev = f32::INFINITY;
        let mut closing = true;
        for _ in 0..(180 * 10) {
            run(&mut u, spec, Some(Vec3::ZERO), &c.drop, 0.1);
            let d = Vec2::new(u.pos.x, u.pos.z).length();
            if closing && d > d_prev + 1.0 {
                closest.push(d_prev);
                closing = false;
            } else if !closing && d < d_prev - 1.0 {
                closing = true;
            }
            d_prev = d;
        }
        assert!(closest.len() >= 2, "runs {closest:?}");
        assert!(closest.iter().all(|d| *d < spec.standoff_m * 2.0), "{closest:?}");
    }

    #[test]
    fn a_cargo_plane_drops_its_whole_load_near_the_drop_point_then_leaves() {
        let c = cfg();
        let spec = &c.units["cargo_plane"];
        let tank = &c.units["tank"];
        let mut plane = Unit::new(1, "cargo_plane", spec, Vec3::new(0.0, spec.altitude_m[0], 3000.0), 0.0, 0);
        plane.cargo = Some(Cargo {
            load: vec!["tank".into(); 3],
            drop_at: Vec2::ZERO,
            released: 0,
        });
        let mut landed = Vec::new();
        let mut falling: Vec<Unit> = Vec::new();
        for _ in 0..(120 * 60) {
            let ground = flat;
            let view = UnitView {
                spec,
                target: None,
                target_vel: Vec3::ZERO,
                opportunity: None,
                ground: &ground,
                drop: &c.drop,
                sea_level_m: 0.0,
            };
            for (k, at, v) in plane.step(&view, 1.0 / 60.0).released {
                falling.push(Unit::dropped(9, &k, tank, at, v));
            }
            for f in &mut falling {
                let tv = UnitView {
                    spec: tank,
                    target: None,
                    target_vel: Vec3::ZERO,
                    opportunity: None,
                    ground: &ground,
                    drop: &c.drop,
                    sea_level_m: 0.0,
                };
                if f.step(&tv, 1.0 / 60.0).landed {
                    landed.push(f.pos);
                }
            }
        }
        assert_eq!(plane.phase, Phase::Leaving);
        assert_eq!(landed.len(), 3);
        for p in &landed {
            assert!(Vec2::new(p.x, p.z).length() < 250.0, "landed {p:?}");
        }
    }

    #[test]
    fn a_unit_fires_only_when_nosed_on_and_in_range() {
        let c = cfg();
        let spec = &c.units["tank"];
        let mut u = Unit::new(1, "tank", spec, Vec3::ZERO, 0.0, 0);
        // Behind it: no shot until it has turned.
        let target = Vec3::new(0.0, 0.0, spec.standoff_m * 0.8);
        let steps = run(&mut u, spec, Some(target), &c.drop, 0.05);
        assert!(steps.iter().all(|s| s.fire.is_none()));
        let steps = run(&mut u, spec, Some(target), &c.drop, 30.0);
        assert!(steps.iter().any(|s| s.fire.is_some()));
        // Out of range: none.
        let mut far = Unit::new(2, "tank", spec, Vec3::ZERO, 0.0, 0);
        far.phase = Phase::Active;
        let steps = run(&mut far, spec, Some(Vec3::new(0.0, 0.0, -spec.fire_range_m * 3.0)), &c.drop, 0.5);
        assert!(steps.iter().all(|s| s.fire.is_none()));
    }

    #[test]
    fn a_unit_takes_a_shot_of_opportunity_only_when_its_target_is_not_its_shot() {
        let c = cfg();
        let spec = &c.units["helicopter"];
        let ground = flat;
        let plane = Vec3::new(0.0, 90.0, -spec.fire_range_m * 0.5);
        let view = |target: Option<Vec3>| UnitView {
            spec,
            target,
            target_vel: Vec3::ZERO,
            opportunity: Some((plane, Vec3::ZERO)),
            ground: &ground,
            drop: &c.drop,
            sea_level_m: 0.0,
        };
        // The pilot behind it, out of its cone: it shoots the airliner ahead instead.
        let mut u = Unit::new(1, "helicopter", spec, Vec3::new(0.0, 90.0, 0.0), 0.0, 0);
        let behind = Some(Vec3::new(0.0, 90.0, spec.standoff_m));
        let (_, dir) = u.try_fire(&view(behind)).expect("a shot of opportunity");
        assert!(dir.z < -0.9, "at the plane: {dir:?}");
        // The pilot ahead and in reach: the pilot gets the shot.
        let mut u = Unit::new(1, "helicopter", spec, Vec3::new(0.0, 90.0, 0.0), 0.0, 0);
        let ahead = Vec3::new(0.0, 90.0, -spec.fire_range_m * 0.3);
        let (from, dir) = u.try_fire(&view(Some(ahead))).unwrap();
        assert!((ahead - from).normalize().dot(dir) > 0.999);
    }

    fn deep_sea(_: f32, _: f32) -> f32 {
        -60.0
    }

    fn run_at_sea(u: &mut Unit, spec: &UnitSpec, target: Vec3, ground: &dyn Fn(f32, f32) -> f32, seconds: f32) -> Vec<UnitStep> {
        let drop = DropConfig::default();
        let view = UnitView {
            spec,
            target: Some(target),
            target_vel: Vec3::ZERO,
            opportunity: None,
            ground,
            drop: &drop,
            sea_level_m: 0.0,
        };
        (0..(seconds * 10.0) as usize).map(|_| u.step(&view, 0.1)).collect()
    }

    #[test]
    fn the_armour_rule_takes_damage_only_from_a_round_that_beats_it() {
        let mut hp = 100.0;
        assert!(!armour_hit(&mut hp, 29.0, 500.0, 30.0));
        assert_eq!(hp, 100.0);
        assert!(!armour_hit(&mut hp, 30.0, 40.0, 30.0));
        assert_eq!(hp, 60.0);
        assert!(armour_hit(&mut hp, 90.0, 60.0, 30.0));
    }

    #[test]
    fn a_gunboat_holds_station_until_its_target_is_near_then_circles_it_on_the_water() {
        let c = cfg();
        let spec = &c.units["gunboat"];
        let y = Unit::riding_y(spec, 0.0);
        let mut u = Unit::new(1, "gunboat", spec, Vec3::new(0.0, y, 0.0), 0.0, 0);
        let far = Vec3::new(spec.fire_range_m * 3.0, 0.0, 0.0);
        run_at_sea(&mut u, spec, far, &deep_sea, 20.0);
        assert!(Vec2::new(u.pos.x, u.pos.z).length() < 1.0, "held station: {:?}", u.pos);
        let near = Vec3::new(spec.fire_range_m * 1.5, 0.0, 0.0);
        let steps = run_at_sea(&mut u, spec, near, &deep_sea, 240.0);
        let r = Vec2::new(u.pos.x - near.x, u.pos.z - near.z).length();
        assert!((r - spec.standoff_m).abs() < spec.standoff_m * 0.5, "circling at its standoff: {r}");
        assert!(steps.iter().any(|s| s.fire.is_some()), "its turret fires");
        assert!((u.pos.y - y).abs() < 1e-4, "rides the sea");
    }

    #[test]
    fn a_gunboat_keeps_off_the_shallows() {
        let c = cfg();
        let spec = &c.units["gunboat"];
        // Land east of x = 200 m: its target sits on the beach.
        let coast = |x: f32, _: f32| if x > 200.0 { 5.0 } else { -60.0 };
        let mut u = Unit::new(1, "gunboat", spec, Vec3::new(-400.0, Unit::riding_y(spec, 0.0), 0.0), 0.0, 0);
        for _ in 0..300 {
            run_at_sea(&mut u, spec, Vec3::new(260.0, 5.0, 0.0), &coast, 1.0);
            assert!(u.pos.x <= 200.0, "aground at {:?}", u.pos);
        }
    }
}
