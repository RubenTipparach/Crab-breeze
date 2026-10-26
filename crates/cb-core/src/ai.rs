//! The AI pilot (openspec change `enemy-mechs`), after Mining Mike's autopilot
//! (godot-sandbox `scripts/bot/bot_brain.gd` at `5f4feacf`): a task brain that re-decides a few
//! times a second and steers every step, through the same controls a player's seat writes. The
//! enemies fly it, and the scripted pilot flies the same brain for the player (its orders become
//! keyboard and mouse actions). Tasks, by priority:
//!
//!   retreat   badly hurt: back off behind cover, away from the target, until the guns cool
//!   evade     just hit, or a shell is coming: flip the orbit, surge, hop on the jets if cool
//!   reposition the target is hidden or an ally is in the line of fire: step round the orbit
//!   engage    hold a stand-off ring round the target and step round it, the torso on the target
//!
//! A stall (asking for speed, going nowhere) turns it away from whatever stopped it. Seeded; it reads
//! only what a pilot could see.

use crate::input::ModeRequest;
use crate::mech::{LocomotionMode, MechControls, MechState};
use crate::rng::SimRng;
use crate::weapons::{ShotKind, lead};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::f32::consts::{PI, TAU};

/// How an AI pilot fights; every number it uses.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Skill {
    /// Random aim error, degrees.
    pub aim_noise_deg: f32,
    /// Fires when the torso is within this of the lead point, degrees.
    pub fire_error_deg: f32,
    /// The stand-off ring's radius.
    pub range_m: f32,
    /// Farthest it fires from.
    pub max_fire_range_m: f32,
    /// How far round the ring it steps each think, radians (Mining Mike: a radian at most).
    pub orbit_step_rad: f32,
    /// Throttle while orbiting.
    pub orbit_throttle: f32,
    /// Seconds between thinks.
    pub think_s: f32,
    /// Below this share of torso structure it retreats.
    pub retreat_health: f32,
    /// After a hit, how long it evades.
    pub evade_s: f32,
    /// Chance an evade hops on the jets (when they are cool).
    pub hop_chance: f32,
    /// Seconds of lift in a hop.
    pub hop_s: f32,
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            aim_noise_deg: 1.2,
            fire_error_deg: 4.0,
            range_m: 180.0,
            max_fire_range_m: 900.0,
            orbit_step_rad: 0.45,
            orbit_throttle: 0.85,
            think_s: 0.25,
            retreat_health: 0.3,
            evade_s: 2.0,
            hop_chance: 0.35,
            hop_s: 1.4,
        }
    }
}

/// What the AI does this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Orders {
    /// The seat's controls.
    pub controls: MechControls,
    /// Fire groups pulled.
    pub fire: [bool; 2],
    /// Where the guns converge.
    pub aim_point: Vec3,
}

/// The fire rule each kind of gun adds (`weapon-kinds` section 5), given the range to the target,
/// the aim's error in degrees and the weapon heat bank's share used: a shotgun inside 200 m, a
/// railgun beyond 300 m with the aim within 0.3 degrees, a laser while the bank is under 0.8, plasma
/// inside 450 m; anything else whenever the aim is on.
pub fn kind_fires(kind: ShotKind, range_m: f32, err_deg: f32, heat: f32) -> bool {
    match kind {
        ShotKind::Pellets { .. } => range_m < 200.0,
        ShotKind::Slug { .. } => range_m > 300.0 && err_deg < 0.3,
        ShotKind::Beam { .. } => heat < 0.8,
        ShotKind::Plasma { .. } => range_m < 450.0,
        ShotKind::Shell | ShotKind::Rocket => true,
    }
}

/// When an AI pilot holding a multi-lock pod lets go (`multi-lock` section 4): with two or more
/// targets in its cone, at three locks or after 3 s of holding; with one, at every lock.
pub fn pod_release(locks: usize, max_locks: u8, in_cone: usize, held_s: f32) -> bool {
    if locks == 0 {
        return false;
    }
    if in_cone >= 2 {
        locks >= 3 || held_s >= 3.0
    } else {
        locks >= max_locks as usize
    }
}

/// What the AI is doing, for logs and the take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Task {
    /// Holding the ring and stepping round it.
    #[default]
    Engage,
    /// Just hit: flip, surge, maybe hop.
    Evade,
    /// No clear line: stepping round to find one.
    Reposition,
    /// Badly hurt: backing off behind cover.
    Retreat,
}

/// What the AI can see this step.
pub struct View<'a> {
    /// Its sensor position.
    pub eye: Vec3,
    /// The target's position (its feet).
    pub target: Vec3,
    /// How far above `target` to aim, metres: a mech's middle (`MechConfig::aim_height_m`),
    /// zero for a point.
    pub aim_height_m: f32,
    /// The target's velocity.
    pub target_vel: Vec3,
    /// Its main gun's muzzle speed.
    pub muzzle_speed_m_s: f32,
    /// Its main gun's shots drop under gravity (false for a rocket).
    pub drop: bool,
    /// Its main gun's kind of shot, for the fire rule each kind has (`weapon-kinds` section 5).
    pub kind: ShotKind,
    /// Its weapon heat bank's share used, 0 to 1.
    pub heat: f32,
    /// Its torso's structure left, 0..1.
    pub health: f32,
    /// It took a hit since the last step.
    pub hit: bool,
    /// Its allies' positions (not to be shot through).
    pub allies: &'a [Vec3],
    /// True when the straight line between two points is clear of buildings and ground.
    pub clear: &'a dyn Fn(Vec3, Vec3) -> bool,
    /// The nearest point behind cover from a threat at `from`, near `me`, if any.
    pub cover: &'a dyn Fn(Vec3, Vec3) -> Option<Vec3>,
    /// True when a mech can walk the straight line between two points (x, z) without meeting a
    /// building: the legs look ahead with it instead of walking into walls.
    pub open: &'a dyn Fn(Vec2, Vec2) -> bool,
}

/// The heading to walk: `want` if the way is open for `look_m`, else the nearest open heading up to
/// 80 degrees either side (the side it was already favouring first); `None` when every way ahead is
/// shut and it should back off.
pub fn avoid(here: Vec2, want: f32, look_m: f32, favour: f32, open: &dyn Fn(Vec2, Vec2) -> bool) -> Option<f32> {
    let ahead = |h: f32| here + Vec2::new(-h.sin(), -h.cos()) * look_m;
    if open(here, ahead(want)) {
        return Some(want);
    }
    for deg in [20.0f32, 40.0, 60.0, 80.0] {
        for side in [favour, -favour] {
            let h = want + side * deg.to_radians();
            if open(here, ahead(h)) {
                return Some(h);
            }
        }
    }
    None
}

/// An AI pilot.
#[derive(Clone, Debug, PartialEq)]
pub struct AiPilot {
    /// Its skill.
    pub skill: Skill,
    /// What it is doing.
    pub task: Task,
    rng: SimRng,
    noise: Vec2,
    think_in_s: f32,
    /// Orbit direction round the target: +1 or -1.
    side: f32,
    /// Where its legs are going.
    goal: Vec2,
    evade_left_s: f32,
    hop_left_s: f32,
    stall_s: f32,
    /// Backing away from what stopped it, seconds left.
    backoff_s: f32,
    /// The line to the target was clear (no wall, no ally) at the last think.
    line_ok: bool,
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// World yaw of a direction (0 faces -z, positive turns left), as the mech's heading.
fn yaw_of(d: Vec3) -> f32 {
    (-d.x).atan2(-d.z)
}

fn flat(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

impl AiPilot {
    /// A pilot with a skill and a seed.
    pub fn new(skill: Skill, rng: SimRng) -> Self {
        let mut rng = rng;
        let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
        Self {
            skill,
            task: Task::Engage,
            rng,
            noise: Vec2::ZERO,
            think_in_s: 0.0,
            side,
            goal: Vec2::ZERO,
            evade_left_s: 0.0,
            hop_left_s: 0.0,
            stall_s: 0.0,
            backoff_s: 0.0,
            line_ok: true,
        }
    }

    /// A point on the ring round `target`, `step` radians on from where `me` is, on side `side`.
    fn ring_point(&self, me: Vec2, target: Vec2, radius: f32, step: f32) -> Vec2 {
        let from = (me - target).normalize_or(Vec2::X);
        let a = from.y.atan2(from.x) + self.side * step;
        target + Vec2::new(a.cos(), a.sin()) * radius
    }

    /// Re-decides the task and the legs' goal.
    fn think(&mut self, me: &MechState, v: &View) {
        let here = flat(me.pos);
        let target = flat(v.target);
        let aim_from = v.eye;
        let aim_to = v.target + Vec3::Y * v.aim_height_m;
        let line_clear = (v.clear)(aim_from, aim_to);
        let ally_in_line = v.allies.iter().any(|a| {
            let p = *a + Vec3::Y * 8.0;
            let ab = aim_to - aim_from;
            let t = ((p - aim_from).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            (aim_from + ab * t).distance(p) < 5.0 && t < 0.98
        });
        self.line_ok = line_clear && !ally_in_line;
        self.task = if v.health < self.skill.retreat_health {
            Task::Retreat
        } else if self.evade_left_s > 0.0 {
            Task::Evade
        } else if !line_clear || ally_in_line {
            Task::Reposition
        } else {
            Task::Engage
        };
        let r = self.skill.range_m;
        self.goal = match self.task {
            Task::Retreat => (v.cover)(me.pos, v.target)
                .map(flat)
                .unwrap_or(here + (here - target).normalize_or(Vec2::X) * 150.0),
            // Round the ring the other way, further, and a little closer: hard to track.
            Task::Evade => self.ring_point(here, target, r * 0.8, self.skill.orbit_step_rad * 2.0),
            // Round the ring to find a clear line.
            Task::Reposition => self.ring_point(here, target, r, self.skill.orbit_step_rad * 1.5),
            Task::Engage => self.ring_point(here, target, r, self.skill.orbit_step_rad),
        };
    }

    /// Decides this step: steers every step, re-thinks every `think_s`.
    pub fn orders(&mut self, me: &MechState, v: &View, dt_s: f32) -> Orders {
        if v.hit && self.evade_left_s <= 0.0 {
            self.evade_left_s = self.skill.evade_s;
            self.side = -self.side;
            if me.boost.heat < 0.3 && self.rng.chance(self.skill.hop_chance) {
                self.hop_left_s = self.skill.hop_s;
            }
            self.think_in_s = 0.0;
        }
        self.evade_left_s = (self.evade_left_s - dt_s).max(0.0);
        self.hop_left_s = (self.hop_left_s - dt_s).max(0.0);
        // A stall: asking to move, not moving. Turn away from whatever stopped it.
        if me.throttle.abs() > 0.3 && me.speed_m_s.abs() < 0.8 && !me.airborne() {
            self.stall_s += dt_s;
        } else {
            self.stall_s = 0.0;
        }
        if self.stall_s > 1.0 {
            // Stuck against something: back away from it for a moment, then try the other way.
            self.side = -self.side;
            self.stall_s = 0.0;
            self.backoff_s = 1.2;
            self.think_in_s = 0.0;
        }
        self.backoff_s = (self.backoff_s - dt_s).max(0.0);
        self.think_in_s -= dt_s;
        if self.think_in_s <= 0.0 {
            self.think_in_s = self.skill.think_s;
            self.think(me, v);
            let n = self.skill.aim_noise_deg.to_radians();
            self.noise = Vec2::new(self.rng.range_f32(-n, n), self.rng.range_f32(-n, n));
        }

        // Legs: to the goal.
        let here = flat(me.pos);
        let to_goal = self.goal - here;
        let want_heading = (-to_goal.x).atan2(-to_goal.y);
        // Look ahead along the way it wants to go (further the faster it goes) and swing to the
        // nearest open heading; every way shut, back off.
        let look_m = (25.0 + me.speed_m_s.abs() * 1.5).min(to_goal.length().max(8.0));
        let steer = avoid(here, want_heading, look_m, self.side, v.open);
        if steer.is_none() && self.backoff_s <= 0.0 {
            self.backoff_s = 1.0;
        }
        let want_heading = steer.unwrap_or(want_heading);
        let turn_err = wrap(want_heading - me.heading_rad);
        let mut turn = (turn_err * 2.5).clamp(-1.0, 1.0);
        let dist = to_goal.length();
        let mut throttle = match self.task {
            Task::Retreat => 1.0,
            Task::Evade => 1.0,
            _ => self.skill.orbit_throttle,
        };
        // Slow for tight turns, ease in at the goal.
        throttle *= (1.0 - turn_err.abs() / PI).max(0.35) * (dist / 30.0).clamp(0.3, 1.0);
        if self.backoff_s > 0.0 {
            // Reversing out, turning toward the side it will try next.
            throttle = -0.6;
            turn = self.side;
        }

        // Torso: on the lead point, with the hand's error.
        let aim_point = lead(v.eye, v.target + Vec3::Y * v.aim_height_m, v.target_vel, v.muzzle_speed_m_s, v.drop);
        let d = aim_point - v.eye;
        let yaw = yaw_of(d) + self.noise.x;
        let pitch = d.y.atan2(Vec2::new(d.x, d.z).length()) + self.noise.y;
        let want_twist = wrap(yaw - me.heading_rad);
        let aim_delta_deg = Vec2::new(
            -(want_twist - me.aim_twist_rad).to_degrees(),
            (pitch - me.aim_pitch_rad).to_degrees(),
        );
        let err = Vec2::new(wrap(me.heading_rad + me.twist_rad - yaw), pitch - me.pitch_rad)
            .length()
            .to_degrees();
        let range_m = flat(v.target).distance(here);
        let ready = err < self.skill.fire_error_deg
            && range_m < self.skill.max_fire_range_m
            && kind_fires(v.kind, range_m, err, v.heat)
            && self.line_ok
            && matches!(self.task, Task::Engage | Task::Evade);

        let hop = self.hop_left_s > 0.0 && !me.boost.locked;
        Orders {
            controls: MechControls {
                throttle_axis: ((throttle - me.throttle) * 5.0).clamp(-1.0, 1.0),
                turn_axis: turn,
                aim_delta_deg,
                mode_request: (hop && me.mode != LocomotionMode::Jet).then_some(ModeRequest::Jet),
                // A short hop: 0.4 of the rockets' doubled thrust is a net 12 m/s2 up.
                lift_axis: if hop { 0.4 } else { 0.0 },
                ..Default::default()
            },
            fire: [ready, ready && err < self.skill.fire_error_deg * 0.6],
            aim_point,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_legs_steer_round_a_wall_ahead() {
        // A wall across x = 0..40, z = -20 (straight ahead is -z): ahead is shut, to the right open.
        let open = |a: Vec2, b: Vec2| {
            let blocked = |p: Vec2| p.x > -10.0 && p.x < 40.0 && (p.y + 20.0).abs() < 2.0;
            (0..=20).all(|k| !blocked(a.lerp(b, k as f32 / 20.0)))
        };
        assert_eq!(avoid(Vec2::ZERO, 0.0, 30.0, 1.0, &|_, _| true), Some(0.0));
        let h = avoid(Vec2::ZERO, 0.0, 30.0, 1.0, &open).expect("a way round");
        let dir = Vec2::new(-h.sin(), -h.cos());
        assert!(dir.x < 0.0, "it should swing toward the open side (-x), went {dir:?}");
        assert_eq!(avoid(Vec2::ZERO, 0.0, 30.0, 1.0, &|_, _| false), None);
    }

    use super::*;

    #[test]
    fn an_ai_pod_lets_go_at_three_locks_on_a_group_and_all_six_on_one() {
        assert!(!pod_release(2, 6, 3, 1.0));
        assert!(pod_release(3, 6, 3, 1.0));
        assert!(pod_release(2, 6, 3, 3.0), "or after 3 s");
        assert!(!pod_release(5, 6, 1, 5.0), "one target: every lock");
        assert!(pod_release(6, 6, 1, 2.1));
        assert!(!pod_release(0, 6, 2, 9.0), "nothing to fire");
    }

    #[test]
    fn each_kind_keeps_to_its_own_fire_rule() {
        let shotgun = ShotKind::Pellets {
            count: 12,
            spread_deg: 1.5,
        };
        assert!(kind_fires(shotgun, 150.0, 1.0, 0.0) && !kind_fires(shotgun, 250.0, 1.0, 0.0));
        let rail = ShotKind::Slug { charge_s: 1.0 };
        assert!(kind_fires(rail, 800.0, 0.2, 0.0));
        assert!(!kind_fires(rail, 800.0, 0.5, 0.0), "not on the lead");
        assert!(!kind_fires(rail, 200.0, 0.1, 0.0), "too close");
        let laser = ShotKind::Beam { range_m: 1500.0 };
        assert!(kind_fires(laser, 900.0, 1.0, 0.5) && !kind_fires(laser, 900.0, 1.0, 0.85));
        let plasma = ShotKind::Plasma { burst_m: 650.0 };
        assert!(kind_fires(plasma, 400.0, 1.0, 0.0) && !kind_fires(plasma, 500.0, 1.0, 0.0));
        assert!(kind_fires(ShotKind::Shell, 2000.0, 1.0, 1.0));
    }
    use crate::mech::{GaitConfig, MechConfig};
    use crate::world::FlatGround;

    fn open(_: Vec3, _: Vec3) -> bool {
        true
    }
    fn no_cover(_: Vec3, _: Vec3) -> Option<Vec3> {
        None
    }

    fn run(ai: &mut AiPilot, me: &mut MechState, target: Vec3, seconds: f32, health: f32) -> (Orders, f32) {
        let cfg = MechConfig::default();
        let mut o = Orders::default();
        let mut travelled = 0.0;
        for _ in 0..(seconds * 60.0) as usize {
            let before = me.pos;
            let v = View {
                eye: me.pos + Vec3::Y * 9.6,
                target,
                aim_height_m: 9.8,
                target_vel: Vec3::ZERO,
                muzzle_speed_m_s: 850.0,
                drop: true,
                kind: Default::default(),
                heat: 0.0,
                health,
                hit: false,
                allies: &[],
                clear: &open,
                cover: &no_cover,
                open: &|_, _| true,
            };
            o = ai.orders(me, &v, 1.0 / 60.0);
            me.step(&o.controls, &cfg, &FlatGround::default(), 1.0 / 60.0);
            travelled += before.distance(me.pos);
        }
        (o, travelled)
    }

    #[test]
    fn the_ai_keeps_moving_round_its_target_and_fires() {
        let mut ai = AiPilot::new(
            Skill {
                aim_noise_deg: 0.0,
                ..Skill::default()
            },
            SimRng::new(3),
        );
        let mut me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let target = Vec3::new(-150.0, 0.0, -150.0);
        let (_, travelled) = run(&mut ai, &mut me, target, 20.0, 1.0);
        assert!(travelled > 60.0, "it moves: {travelled} m in 20 s");
        let d = flat(me.pos).distance(flat(target));
        assert!(d > 100.0 && d < 280.0, "near its ring: {d}");
        let (o, _) = run(&mut ai, &mut me, target, 3.0, 1.0);
        assert!(o.fire[0] || ai.task != Task::Engage, "fires while engaged");
    }

    #[test]
    fn the_ai_leads_a_moving_target() {
        let mut ai = AiPilot::new(
            Skill {
                aim_noise_deg: 0.0,
                ..Skill::default()
            },
            SimRng::new(3),
        );
        let me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let v = View {
            eye: Vec3::Y * 9.6,
            target: Vec3::new(0.0, 0.0, -400.0),
            aim_height_m: 9.8,
            target_vel: Vec3::new(7.0, 0.0, 0.0),
            muzzle_speed_m_s: 850.0,
            drop: true,
            kind: Default::default(),
            heat: 0.0,
            health: 1.0,
            hit: false,
            allies: &[],
            clear: &open,
            cover: &no_cover,
            open: &|_, _| true,
        };
        let o = ai.orders(&me, &v, 0.016);
        assert!(o.aim_point.x > 2.0, "aims ahead: {}", o.aim_point);
    }

    #[test]
    fn a_badly_hurt_ai_retreats_away_from_the_target() {
        let mut ai = AiPilot::new(Skill::default(), SimRng::new(5));
        let mut me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let target = Vec3::new(0.0, 0.0, -150.0);
        run(&mut ai, &mut me, target, 15.0, 0.1);
        assert_eq!(ai.task, Task::Retreat);
        assert!(
            flat(me.pos).distance(flat(target)) > 200.0,
            "backed off to {}",
            flat(me.pos).distance(flat(target))
        );
    }

    #[test]
    fn the_ai_holds_fire_with_an_ally_in_the_line() {
        let mut ai = AiPilot::new(
            Skill {
                aim_noise_deg: 0.0,
                ..Skill::default()
            },
            SimRng::new(7),
        );
        let mut me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let target = Vec3::new(0.0, 0.0, -300.0);
        let cfg = MechConfig::default();
        let mut fired = false;
        for _ in 0..240 {
            let ally = [me.pos + (target - me.pos) * 0.5];
            let v = View {
                eye: me.pos + Vec3::Y * 9.6,
                target,
                aim_height_m: 9.8,
                target_vel: Vec3::ZERO,
                muzzle_speed_m_s: 850.0,
                drop: true,
                kind: Default::default(),
                heat: 0.0,
                health: 1.0,
                hit: false,
                allies: &ally,
                clear: &open,
                cover: &no_cover,
                open: &|_, _| true,
            };
            let o = ai.orders(&me, &v, 1.0 / 60.0);
            fired |= o.fire[0];
            me.step(&o.controls, &cfg, &FlatGround::default(), 1.0 / 60.0);
        }
        assert!(!fired, "never fires through an ally");
        assert_eq!(ai.task, Task::Reposition);
    }

    #[test]
    fn a_hit_makes_it_evade_and_switch_direction() {
        let mut ai = AiPilot::new(Skill::default(), SimRng::new(9));
        let me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let side = ai.side;
        let v = View {
            eye: Vec3::Y * 9.6,
            target: Vec3::new(0.0, 0.0, -200.0),
            aim_height_m: 9.8,
            target_vel: Vec3::ZERO,
            muzzle_speed_m_s: 850.0,
            drop: true,
            kind: Default::default(),
            heat: 0.0,
            health: 1.0,
            hit: true,
            allies: &[],
            clear: &open,
            cover: &no_cover,
            open: &|_, _| true,
        };
        ai.orders(&me, &v, 0.016);
        assert_eq!(ai.task, Task::Evade);
        assert_eq!(ai.side, -side);
    }
}

/// An enemy in an encounter (`data/world/encounters.ron`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemySpec {
    /// Its mech's data file, relative to `data/`.
    pub mech: String,
    /// Where it starts, relative to the player's mech (x, z).
    pub offset_m: [f32; 2],
    /// Its heading, degrees (0 faces -z, positive turns left).
    pub heading_deg: f32,
    /// Hull colour override (linear-ish sRGB 0..1), to tell it from the player's.
    #[serde(default)]
    pub tint: Option<[f32; 3]>,
    /// The block-out it wears over the stand-in (`data/enemies/skins.ron`); `None` shows the stand-in.
    #[serde(default)]
    pub skin: Option<String>,
    /// Gun override; `None` keeps the mech's own.
    #[serde(default)]
    pub weapons: Option<Vec<crate::weapons::WeaponSpec>>,
    /// Scales every part's armour (a lighter variant below 1).
    #[serde(default = "one")]
    pub armour_scale: f32,
    /// Its pilot.
    #[serde(default)]
    pub skill: Skill,
}

fn one() -> f32 {
    1.0
}

/// The enemies of the first fight.
#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Encounters {
    /// Enemies, spawned in order.
    pub enemies: Vec<EnemySpec>,
}

impl crate::data::Validate for Encounters {
    fn validate(&self) -> Result<(), String> {
        for (i, e) in self.enemies.iter().enumerate() {
            crate::data::finite(&format!("enemies[{i}].offset_m"), e.offset_m[0])?;
            crate::data::finite(&format!("enemies[{i}].offset_m"), e.offset_m[1])?;
            crate::data::finite(&format!("enemies[{i}].heading_deg"), e.heading_deg)?;
            crate::data::positive(&format!("enemies[{i}].skill.range_m"), e.skill.range_m)?;
            crate::data::positive(&format!("enemies[{i}].armour_scale"), e.armour_scale)?;
            if let Some(w) = &e.weapons {
                for spec in w {
                    crate::data::Validate::validate(spec)?;
                }
            }
        }
        Ok(())
    }
}
