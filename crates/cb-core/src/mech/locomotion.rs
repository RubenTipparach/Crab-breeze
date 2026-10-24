//! The mech's movement state and its fixed step (mech-rig design sections 2 and 4,
//! `boost-and-skate`): the throttle lever, one horizontal velocity carried between walking,
//! skating and boosting, the leg heading, torso twist and pitch, the boost and skate budgets, and
//! the gait clock. Pure: state, controls, data, ground and `dt_s` in; state out.

use super::config::{GaitConfig, LocomotionConfig, MechConfig};
use crate::input::{Actions, ModeRequest};
use crate::world::Ground;
use glam::{Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

/// How the mech is moving.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocomotionMode {
    /// On the legs.
    #[default]
    Walk,
    /// On the feet's wheels.
    Skate,
    /// On the boosters.
    Jet,
}

impl LocomotionMode {
    /// True when this mode has a model and can be entered.
    pub fn is_built(self) -> bool {
        true
    }
}

/// A limited feature's budget (boosters, skates): heat is the share used, 0 full to 1 empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Budget {
    /// Share used, 0..1.
    pub heat: f32,
    /// Emptied: out until the relight share is back.
    pub locked: bool,
    /// Seconds since it was last used.
    pub idle_s: f32,
}

impl Budget {
    /// One step: burns while `burning`, recharges after the delay otherwise.
    pub fn tick(&mut self, burning: bool, burn_s: f32, recharge_s: f32, delay_s: f32, relight: f32, dt_s: f32) {
        if burning && !self.locked {
            self.idle_s = 0.0;
            self.heat = (self.heat + dt_s / burn_s).min(1.0);
            if self.heat >= 1.0 {
                self.locked = true;
            }
        } else {
            self.idle_s += dt_s;
            if self.idle_s >= delay_s {
                self.heat = (self.heat - dt_s / recharge_s).max(0.0);
            }
            if self.locked && self.heat <= 1.0 - relight {
                self.locked = false;
            }
        }
    }

    /// Seconds of use left.
    pub fn left_s(&self, burn_s: f32) -> f32 {
        (1.0 - self.heat) * burn_s
    }

    /// Seconds to recharge fully (after the delay).
    pub fn recharge_left_s(&self, recharge_s: f32) -> f32 {
        self.heat * recharge_s
    }
}

/// What the pilot's seat asks of the mech this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MechControls {
    /// Throttle lever motion, -1..1.
    pub throttle_axis: f32,
    /// Snap the lever to zero.
    pub throttle_zero: bool,
    /// Leg turn, left positive, -1..1.
    pub turn_axis: f32,
    /// Aim change: twist right (+x) and pitch up (+y), degrees.
    pub aim_delta_deg: Vec2,
    /// A mode change asked for.
    pub mode_request: Option<ModeRequest>,
    /// Boost lift, 0..1 (holding boost fires the boosters).
    pub lift_axis: f32,
    /// Boost thrust direction asked: strafe right (+x) and forward (+y), relative to the view.
    pub move_xy: Vec2,
    /// Where the pilot looks (world yaw): boost thrust is relative to it.
    pub view_yaw_rad: f32,
    /// How far up the pilot looks (radians, up positive): boosting forward climbs or dives with it.
    pub view_pitch_rad: f32,
    /// Skates toggled this step.
    pub skate_toggle: bool,
}

impl MechControls {
    /// The seat's controls from the pilot's actions. In free look the mouse turns the head, not
    /// the torso. The view yaw is the pilot's, set by the caller.
    pub fn from_actions(a: &Actions) -> Self {
        Self {
            throttle_axis: a.throttle_axis,
            throttle_zero: a.throttle_zero,
            turn_axis: a.turn_axis,
            aim_delta_deg: if a.free_look { Vec2::ZERO } else { a.look_delta_deg },
            mode_request: a.mode_request,
            lift_axis: a.lift_axis,
            move_xy: a.move_xy,
            view_yaw_rad: 0.0,
            view_pitch_rad: 0.0,
            skate_toggle: a.skate_toggle,
        }
    }
}

/// The mech's state, advanced each fixed step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MechState {
    /// Root position on the ground, world metres.
    pub pos: Vec3,
    /// Leg heading, radians; 0 faces -z, positive turns left.
    pub heading_rad: f32,
    /// Throttle lever, `throttle_min`..1.
    pub throttle: f32,
    /// Speed along the heading, m/s (negative backing up).
    pub speed_m_s: f32,
    /// Horizontal velocity (x, z), every mode.
    pub vel: Vec2,
    /// Current leg turn rate, rad/s.
    pub turn_rate_rad_s: f32,
    /// Torso twist from the legs, radians, left positive.
    pub twist_rad: f32,
    /// Torso pitch, radians, up positive.
    pub pitch_rad: f32,
    /// Where the pilot aims the twist.
    pub aim_twist_rad: f32,
    /// Where the pilot aims the pitch.
    pub aim_pitch_rad: f32,
    /// Gait clock in cycles (fractional part is the phase).
    pub gait_cycles: f64,
    /// Stride swing, 0 settled to 1 full.
    pub gait_swing: f32,
    /// Stride length per cycle at the moment, metres.
    pub stride_m: f32,
    /// Movement mode.
    pub mode: LocomotionMode,
    /// Skates are switched on (the mode is `Skate` on the ground).
    pub skates_on: bool,
    /// Water depth at the root, 0 on dry ground.
    pub wading_depth_m: f32,
    /// Vertical speed, up positive.
    pub vel_y_m_s: f32,
    /// The boosters' budget.
    pub boost: Budget,
    /// The skates' budget.
    pub skate: Budget,
    /// The boosters fired this step.
    pub boosting: bool,
    /// Legs shot off: [left, right].
    pub legs_lost: [bool; 2],
}

const G_M_S2: f32 = 9.81;

/// World forward for a heading: 0 is -z, positive turns left.
pub fn forward(heading_rad: f32) -> Vec3 {
    Vec3::new(-heading_rad.sin(), 0.0, -heading_rad.cos())
}

fn flat_forward(heading_rad: f32) -> Vec2 {
    Vec2::new(-heading_rad.sin(), -heading_rad.cos())
}

fn approach(v: f32, target: f32, max_step: f32) -> f32 {
    v + (target - v).clamp(-max_step, max_step)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl MechState {
    /// A mech standing still at `pos` facing `heading_rad`.
    pub fn standing(pos: Vec3, heading_rad: f32, gait: &GaitConfig) -> Self {
        Self {
            pos,
            heading_rad,
            stride_m: gait.stride_min_m,
            ..Default::default()
        }
    }

    /// True when the mech is stopped with its throttle at zero (boarding and climbing out allowed).
    pub fn is_stopped(&self, cfg: &LocomotionConfig) -> bool {
        self.throttle == 0.0 && self.vel.length() < cfg.stopped_speed_m_s && !self.airborne()
    }

    /// True when the mech is off the ground (falling or on its boosters).
    pub fn airborne(&self) -> bool {
        self.vel_y_m_s.abs() > 1e-3 || self.mode == LocomotionMode::Jet
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(&mut self, c: &MechControls, cfg: &MechConfig, ground: &impl Ground, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let (loco, gait, jet, sk) = (&cfg.locomotion, &cfg.gait, &cfg.jet, &cfg.skate);
        let legs_left = 2 - self.legs_lost.iter().filter(|l| **l).count();

        // Skates on and off (a toggle, survey M5); they need both legs and a budget.
        let skate_asked = c.skate_toggle || c.mode_request == Some(ModeRequest::Skate);
        if skate_asked {
            self.skates_on = !self.skates_on;
        }
        if c.mode_request == Some(ModeRequest::Walk) {
            self.skates_on = false;
        }
        if self.skate.locked || legs_left < 2 {
            self.skates_on = false;
        }

        // Boosters fire while boost is held and they are not locked out.
        self.boosting = c.lift_axis > 0.0 && !self.boost.locked;
        if self.boosting {
            self.mode = LocomotionMode::Jet;
        } else if self.mode != LocomotionMode::Jet {
            self.mode = if self.skates_on {
                LocomotionMode::Skate
            } else {
                LocomotionMode::Walk
            };
        }
        let flying = self.mode == LocomotionMode::Jet;

        // Throttle lever.
        if c.throttle_zero {
            self.throttle = 0.0;
        }
        self.throttle = (self.throttle + c.throttle_axis * loco.throttle_rate_per_s * dt_s).clamp(loco.throttle_min, 1.0);

        // Legs (or wheels) turn the heading.
        let turn_rate = match self.mode {
            LocomotionMode::Skate => sk.turn_rate_deg_s,
            _ if legs_left == 1 => loco.turn_rate_deg_s * 0.5,
            _ => loco.turn_rate_deg_s,
        };
        self.turn_rate_rad_s = c.turn_axis * turn_rate.to_radians();
        self.heading_rad = (self.heading_rad + self.turn_rate_rad_s * dt_s).rem_euclid(std::f32::consts::TAU);
        let fwd = flat_forward(self.heading_rad);

        // Boosting in a direction flies along the view, pitch included.
        let ask = c.move_xy.clamp_length_max(1.0);
        let thrusting = self.boosting && ask != Vec2::ZERO;
        let pitch = if c.view_pitch_rad.is_finite() {
            c.view_pitch_rad.clamp(-FRAC_PI_2, FRAC_PI_2)
        } else {
            0.0
        };

        // Horizontal velocity, by mode.
        match self.mode {
            LocomotionMode::Jet => {
                if thrusting {
                    let view = flat_forward(c.view_yaw_rad);
                    let right = Vec2::new(-view.y, view.x);
                    let dir = view * ask.y * pitch.cos() + right * ask.x;
                    self.vel += dir * jet.accel_m_s2 * dt_s;
                    self.vel = self.vel.clamp_length_max(jet.speed_max_m_s);
                } else {
                    self.vel *= (1.0 - jet.drag_per_s * dt_s).max(0.0);
                }
            }
            LocomotionMode::Skate => {
                // The wheels drive along the heading; side grip is low, so a turn slides.
                let target = fwd * self.throttle * sk.speed_max_m_s;
                let blend = 1.0 - (-sk.grip_per_s * dt_s).exp();
                let dv = ((target - self.vel) * blend).clamp_length_max(sk.accel_m_s2 * dt_s * 3.0);
                self.vel += dv;
            }
            LocomotionMode::Walk => {
                // Wading drags the legs back (terrain-water); a lost leg leaves a limp
                // (mech-damage); no legs, no walking.
                let wade = (self.wading_depth_m / loco.hip_height_m).clamp(0.0, 1.0);
                let limp = match legs_left {
                    2 => 1.0,
                    1 => loco.limp_speed_ratio,
                    _ => 0.0,
                };
                let target = self.throttle * loco.walk_speed_max_m_s * (1.0 - loco.wade_slowdown * wade) * limp;
                let along = self.vel.dot(fwd);
                let rate = if target.abs() > along.abs() && target * along >= 0.0 {
                    loco.accel_m_s2
                } else {
                    loco.brake_m_s2
                };
                let speed = approach(along, target, rate * dt_s);
                // Walking legs carry no slide: what is not along the heading is braked off.
                let side = self.vel - fwd * along;
                let side = side * (1.0 - (loco.brake_m_s2 * dt_s / side.length().max(1e-3)).min(1.0));
                self.vel = fwd * speed + side;
            }
        }

        let here = Vec2::new(self.pos.x, self.pos.z);
        let mut moved = here + self.vel * dt_s;
        if !flying && ground.water_at(moved.x, moved.y).is_some_and(|w| w.depth_m > loco.max_wade_depth_m) {
            // Too deep: the water stops the mech as a wall does.
            moved = here;
            self.vel = Vec2::ZERO;
        }
        let pushed = ground.push_out_at(moved, loco.body_radius_m, self.pos.y);
        if pushed.distance(moved) > 1e-4 {
            // Into a wall: what was lost into it is gone.
            let real = (pushed - here) / dt_s;
            self.vel = real.clamp_length_max(self.vel.length());
        }

        // Budgets.
        self.boost.tick(
            self.boosting,
            jet.burn_s,
            jet.recharge_s,
            jet.recharge_delay_s,
            jet.relight_fraction,
            dt_s,
        );
        let skating = self.mode == LocomotionMode::Skate;
        self.skate
            .tick(skating, sk.burn_s, sk.recharge_s, sk.recharge_delay_s, sk.relight_fraction, dt_s);
        if self.skate.locked {
            self.skates_on = false;
        }

        // Vertical. Thrusting in a direction, the jets carry the mech's weight and the climb rate
        // follows the view's pitch (level flies level); boost alone lifts against gravity.
        if thrusting {
            let want = ask.y * pitch.sin() * jet.climb_max_m_s;
            self.vel_y_m_s = approach(self.vel_y_m_s, want, jet.accel_m_s2 * dt_s);
        } else {
            let lift = if self.boosting { jet.thrust_m_s2 * c.lift_axis } else { 0.0 };
            self.vel_y_m_s += (lift - G_M_S2) * dt_s;
        }
        let sink = if self.boosting { -jet.sink_max_m_s } else { -60.0 };
        self.vel_y_m_s = self.vel_y_m_s.clamp(sink, jet.climb_max_m_s);
        let mut y = self.pos.y + self.vel_y_m_s * dt_s;
        let support = ground.support_at(pushed.x, pushed.y, self.pos.y);
        if y <= support || (!flying && self.vel_y_m_s <= 0.0 && self.pos.y - support < 1.0) {
            // On the ground (or a roof). Touching down ends a flight: back on the wheels or the legs.
            if flying && self.vel_y_m_s <= 0.0 && !self.boosting {
                self.mode = if self.skates_on {
                    LocomotionMode::Skate
                } else {
                    LocomotionMode::Walk
                };
            }
            y = support;
            if !self.boosting {
                self.vel_y_m_s = 0.0;
            } else {
                self.vel_y_m_s = self.vel_y_m_s.max(0.0);
            }
        }
        self.pos = Vec3::new(pushed.x, y, pushed.y);
        self.wading_depth_m = ground.water_at(pushed.x, pushed.y).map_or(0.0, |w| (w.surface_m - y).max(0.0));
        self.speed_m_s = self.vel.dot(flat_forward(self.heading_rad));

        // Torso, independent of the legs: aim moves the target, the torso follows at its rate.
        let twist_max = loco.twist_max_deg.to_radians();
        let [pitch_lo, pitch_hi] = loco.pitch_limits_deg.map(f32::to_radians);
        self.aim_twist_rad = (self.aim_twist_rad - c.aim_delta_deg.x.to_radians()).clamp(-twist_max, twist_max);
        self.aim_pitch_rad = (self.aim_pitch_rad + c.aim_delta_deg.y.to_radians()).clamp(pitch_lo, pitch_hi);
        self.twist_rad = approach(self.twist_rad, self.aim_twist_rad, loco.twist_rate_deg_s.to_radians() * dt_s);
        self.pitch_rad = approach(self.pitch_rad, self.aim_pitch_rad, loco.pitch_rate_deg_s.to_radians() * dt_s);

        // Gait clock: cycles advance by distance over stride; turning in place steps too. Skating
        // and flying, the legs do not step.
        let stepping = self.mode == LocomotionMode::Walk && !self.airborne();
        let v = if stepping { self.speed_m_s.abs() } else { 0.0 };
        let effective = v + if stepping {
            self.turn_rate_rad_s.abs() * gait.turn_step_radius_m
        } else {
            0.0
        };
        self.stride_m = (gait.stride_max_m * v / loco.walk_speed_max_m_s).max(gait.stride_min_m);
        let dir = if self.speed_m_s < -1e-3 { -1.0 } else { 1.0 };
        self.gait_cycles += f64::from(dir * effective / self.stride_m * dt_s);
        self.gait_swing = smoothstep(0.0, gait.settle_speed_m_s, effective);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::{Building, Rect};
    use crate::world::{BuildingGrid, FlatGround};

    const DT: f32 = 1.0 / 60.0;

    fn run(state: &mut MechState, c: MechControls, seconds: f32, ground: &impl Ground) {
        let cfg = MechConfig::default();
        for _ in 0..(seconds / DT) as usize {
            state.step(&c, &cfg, ground, DT);
        }
    }

    #[test]
    fn full_throttle_reaches_top_speed() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let up = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, up, 2.0, &FlatGround::default());
        run(&mut m, MechControls::default(), 4.0, &FlatGround::default());
        assert_eq!(m.throttle, 1.0);
        assert!((m.speed_m_s - 7.0).abs() < 1e-3, "{}", m.speed_m_s);
        assert!(m.pos.z < -20.0, "heading 0 walks toward -z: {}", m.pos);
    }

    #[test]
    fn torso_twist_is_limited_and_independent_of_the_legs() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let aim_left = MechControls {
            aim_delta_deg: Vec2::new(-180.0, 0.0),
            ..Default::default()
        };
        run(&mut m, aim_left, DT, &FlatGround::default());
        run(&mut m, MechControls::default(), 4.0, &FlatGround::default());
        assert!((m.twist_rad - 100f32.to_radians()).abs() < 1e-4, "{}", m.twist_rad.to_degrees());
        assert_eq!(m.heading_rad, 0.0, "the legs keep their heading");
    }

    #[test]
    fn the_mech_cannot_walk_into_a_building() {
        let wall = Building {
            id: 0,
            city: 0,
            footprint: Rect {
                min: glam::Vec2::new(-50.0, -30.0),
                max: glam::Vec2::new(50.0, -20.0),
            },
            storeys: 10,
            height_m: 35.0,
        };
        let ground = FlatGround {
            walls: BuildingGrid::new(&[wall], 1024.0, 64.0),
        };
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 15.0, &ground);
        assert!(m.pos.z >= -20.0 + 3.0 - 1e-3, "{}", m.pos);
        assert!(m.speed_m_s.abs() < 0.5, "the legs stop at the wall: {}", m.speed_m_s);
    }

    /// Flat ground under a pool `depth_m` deep for z < -20.
    struct Pool {
        depth_m: f32,
    }

    impl Ground for Pool {
        fn height_at(&self, _x: f32, _z: f32) -> f32 {
            0.0
        }
        fn push_out(&self, p: Vec2, _r: f32) -> Vec2 {
            p
        }
        fn water_at(&self, _x: f32, z: f32) -> Option<crate::world::WaterSample> {
            (z < -20.0).then_some(crate::world::WaterSample {
                surface_m: self.depth_m,
                depth_m: self.depth_m,
                flow_m_s: Vec2::ZERO,
            })
        }
    }

    #[test]
    fn wading_deeper_is_slower() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 20.0, &Pool { depth_m: 2.95 });
        assert!(m.wading_depth_m > 2.9);
        assert!(
            (m.speed_m_s - 7.0 * (1.0 - 0.6 * 0.5)).abs() < 0.05,
            "half-hip water: {}",
            m.speed_m_s
        );
    }

    #[test]
    fn water_over_the_cockpit_stops_the_mech() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 20.0, &Pool { depth_m: 12.0 });
        assert!(m.pos.z > -20.0, "stopped at the water's edge: {}", m.pos);
    }

    #[test]
    fn boosters_burn_for_twenty_seconds_then_lock_and_recharge() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let up = MechControls {
            lift_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, up, 10.0, &FlatGround::default());
        assert!(m.pos.y > 150.0, "climbing: {}", m.pos.y);
        assert!((m.boost.left_s(20.0) - 10.0).abs() < 0.1, "half the burn left");
        run(&mut m, up, 10.5, &FlatGround::default());
        assert!(m.boost.locked, "20 s of burn empties them");
        run(&mut m, MechControls::default(), 60.0, &FlatGround::default());
        assert_eq!(m.pos.y, 0.0, "back on the ground");
        assert_eq!(m.mode, LocomotionMode::Walk);
        assert!(!m.boost.locked && m.boost.heat == 0.0, "recharged");
    }

    #[test]
    fn boosting_with_a_strafe_moves_sideways_fast() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let c = MechControls {
            lift_axis: 0.4,
            move_xy: Vec2::new(1.0, 0.0),
            ..Default::default()
        };
        run(&mut m, c, 3.0, &FlatGround::default());
        assert!(m.pos.x > 60.0, "slid right on the boosters: {}", m.pos);
        assert!(m.vel.x > 40.0);
    }

    #[test]
    fn boosting_forward_while_looking_level_dashes_at_height() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let hop = MechControls {
            lift_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, hop, 1.0, &FlatGround::default());
        let start_y = m.pos.y;
        let dash = MechControls {
            lift_axis: 1.0,
            move_xy: Vec2::new(0.0, 1.0),
            ..Default::default()
        };
        run(&mut m, dash, 5.0, &FlatGround::default());
        assert!(m.pos.z < -150.0, "dashed forward: {}", m.pos);
        assert!((m.pos.y - start_y).abs() < 10.0, "held near its height: {} from {start_y}", m.pos.y);
        run(&mut m, dash, 3.0, &FlatGround::default());
        let y = m.pos.y;
        run(&mut m, dash, 2.0, &FlatGround::default());
        assert!((m.pos.y - y).abs() < 1.0, "level once settled: {} from {y}", m.pos.y);
    }

    #[test]
    fn boosting_forward_while_looking_up_climbs() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let c = MechControls {
            lift_axis: 1.0,
            move_xy: Vec2::new(0.0, 1.0),
            view_pitch_rad: 30f32.to_radians(),
            ..Default::default()
        };
        run(&mut m, c, 5.0, &FlatGround::default());
        assert!(m.pos.y > 50.0, "climbed: {}", m.pos.y);
        assert!(m.pos.z < -100.0, "and went forward: {}", m.pos);
    }

    #[test]
    fn skates_last_forty_seconds_then_it_walks() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let on = MechControls {
            skate_toggle: true,
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, on, 1.0 / 60.0, &FlatGround::default());
        assert_eq!(m.mode, LocomotionMode::Skate);
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 10.0, &FlatGround::default());
        assert!(m.vel.length() > 25.0, "fast on wheels: {}", m.vel.length());
        run(&mut m, go, 31.0, &FlatGround::default());
        assert!(m.skate.locked);
        assert_eq!(m.mode, LocomotionMode::Walk);
    }

    #[test]
    fn a_skating_mech_slides_through_a_turn() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let on = MechControls {
            skate_toggle: true,
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, on, 1.0 / 60.0, &FlatGround::default());
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 8.0, &FlatGround::default());
        let turn = MechControls {
            throttle_axis: 1.0,
            turn_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, turn, 0.6, &FlatGround::default());
        let fwd = flat_forward(m.heading_rad);
        let side = (m.vel - fwd * m.vel.dot(fwd)).length();
        assert!(side > 5.0, "the velocity lags the heading: slides {side} m/s sideways");
    }

    #[test]
    fn a_one_legged_mech_limps_at_under_a_quarter_speed() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        m.legs_lost = [true, false];
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 10.0, &FlatGround::default());
        assert!(m.speed_m_s > 0.5 && m.speed_m_s <= 0.25 * 7.0, "{}", m.speed_m_s);
        m.legs_lost = [true, true];
        run(&mut m, go, 10.0, &FlatGround::default());
        assert_eq!(m.speed_m_s, 0.0, "no legs, no walking");
    }

    #[test]
    fn asking_for_an_unbuilt_mode_keeps_walking() {
        let mut m = MechState {
            legs_lost: [true, false],
            ..Default::default()
        };
        let c = MechControls {
            mode_request: Some(ModeRequest::Skate),
            ..Default::default()
        };
        run(&mut m, c, DT, &FlatGround::default());
        assert_eq!(m.mode, LocomotionMode::Walk, "skates need both legs");
    }

    #[test]
    fn the_mech_is_stopped_only_with_the_throttle_at_zero() {
        let l = LocomotionConfig::default();
        let mut m = MechState::default();
        assert!(m.is_stopped(&l));
        m.throttle = 0.1;
        assert!(!m.is_stopped(&l));
    }
}
