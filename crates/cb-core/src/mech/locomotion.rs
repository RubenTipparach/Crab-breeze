//! The mech's movement state and its fixed step (mech-rig design sections 2 and 4): the throttle
//! lever, ground speed, leg heading, torso twist and pitch, and the gait clock. Pure: state,
//! controls, data, ground and `dt_s` in; state out.

use super::config::{GaitConfig, LocomotionConfig};
use crate::input::{Actions, ModeRequest};
use crate::world::Ground;
use glam::{Vec2, Vec3};

/// How the mech is moving. Only `Walk` has a model yet (surveys M3, M4, M5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocomotionMode {
    /// On the legs.
    #[default]
    Walk,
    /// On the feet's wheels (not built).
    Skate,
    /// On the jets (not built).
    Jet,
}

impl LocomotionMode {
    /// True when this mode has a model and can be entered.
    pub fn is_built(self) -> bool {
        matches!(self, LocomotionMode::Walk)
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
}

impl MechControls {
    /// The seat's controls from the pilot's actions. In free look the mouse turns the head, not
    /// the torso.
    pub fn from_actions(a: &Actions) -> Self {
        Self {
            throttle_axis: a.throttle_axis,
            throttle_zero: a.throttle_zero,
            turn_axis: a.turn_axis,
            aim_delta_deg: if a.free_look { Vec2::ZERO } else { a.look_delta_deg },
            mode_request: a.mode_request,
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
    /// Ground speed along the heading, m/s (negative backing up).
    pub speed_m_s: f32,
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
}

/// World forward for a heading: 0 is -z, positive turns left.
pub fn forward(heading_rad: f32) -> Vec3 {
    Vec3::new(-heading_rad.sin(), 0.0, -heading_rad.cos())
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
        self.throttle == 0.0 && self.speed_m_s.abs() < cfg.stopped_speed_m_s
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(&mut self, c: &MechControls, loco: &LocomotionConfig, gait: &GaitConfig, ground: &impl Ground, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        if let Some(req) = c.mode_request {
            let want = match req {
                ModeRequest::Walk => LocomotionMode::Walk,
                ModeRequest::Skate => LocomotionMode::Skate,
                ModeRequest::Jet => LocomotionMode::Jet,
            };
            if want.is_built() {
                self.mode = want;
            }
        }

        // Throttle lever and ground speed.
        if c.throttle_zero {
            self.throttle = 0.0;
        }
        self.throttle = (self.throttle + c.throttle_axis * loco.throttle_rate_per_s * dt_s).clamp(loco.throttle_min, 1.0);
        let target = self.throttle * loco.walk_speed_max_m_s;
        let rate = if target.abs() > self.speed_m_s.abs() && target * self.speed_m_s >= 0.0 {
            loco.accel_m_s2
        } else {
            loco.brake_m_s2
        };
        self.speed_m_s = approach(self.speed_m_s, target, rate * dt_s);

        // Legs.
        self.turn_rate_rad_s = c.turn_axis * loco.turn_rate_deg_s.to_radians();
        self.heading_rad = (self.heading_rad + self.turn_rate_rad_s * dt_s).rem_euclid(std::f32::consts::TAU);
        let moved = self.pos + forward(self.heading_rad) * self.speed_m_s * dt_s;
        let pushed = ground.push_out(Vec2::new(moved.x, moved.z), loco.body_radius_m);
        if pushed.distance(Vec2::new(moved.x, moved.z)) > 1e-4 {
            // Walked into a wall: the legs stop where the body stopped.
            let real = (pushed - Vec2::new(self.pos.x, self.pos.z)).length() / dt_s;
            self.speed_m_s = self.speed_m_s.signum() * real.min(self.speed_m_s.abs());
        }
        self.pos = Vec3::new(pushed.x, ground.height_at(pushed.x, pushed.y), pushed.y);

        // Torso, independent of the legs: aim moves the target, the torso follows at its rate.
        let twist_max = loco.twist_max_deg.to_radians();
        let [pitch_lo, pitch_hi] = loco.pitch_limits_deg.map(f32::to_radians);
        self.aim_twist_rad = (self.aim_twist_rad - c.aim_delta_deg.x.to_radians()).clamp(-twist_max, twist_max);
        self.aim_pitch_rad = (self.aim_pitch_rad + c.aim_delta_deg.y.to_radians()).clamp(pitch_lo, pitch_hi);
        self.twist_rad = approach(self.twist_rad, self.aim_twist_rad, loco.twist_rate_deg_s.to_radians() * dt_s);
        self.pitch_rad = approach(self.pitch_rad, self.aim_pitch_rad, loco.pitch_rate_deg_s.to_radians() * dt_s);

        // Gait clock: cycles advance by distance over stride; turning in place steps too.
        let v = self.speed_m_s.abs();
        let effective = v + self.turn_rate_rad_s.abs() * gait.turn_step_radius_m;
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
        let (l, g) = (LocomotionConfig::default(), GaitConfig::default());
        for _ in 0..(seconds / DT) as usize {
            state.step(&c, &l, &g, ground, DT);
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

    #[test]
    fn asking_for_an_unbuilt_mode_keeps_walking() {
        let mut m = MechState::default();
        for req in [ModeRequest::Skate, ModeRequest::Jet] {
            let c = MechControls {
                mode_request: Some(req),
                ..Default::default()
            };
            run(&mut m, c, DT, &FlatGround::default());
            assert_eq!(m.mode, LocomotionMode::Walk);
        }
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
