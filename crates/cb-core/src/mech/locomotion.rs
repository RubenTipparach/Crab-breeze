//! The mech's movement state and its fixed step (mech-rig design sections 2 and 4): the throttle
//! lever, ground speed, leg heading, torso twist and pitch, and the gait clock. Pure: state,
//! controls, data, ground and `dt_s` in; state out.

use super::config::{GaitConfig, LocomotionConfig, MechConfig};
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
        matches!(self, LocomotionMode::Walk | LocomotionMode::Jet)
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
    /// Jet lift, 0..1.
    pub lift_axis: f32,
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
            lift_axis: a.lift_axis,
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
    /// Water depth at the root, 0 on dry ground.
    pub wading_depth_m: f32,
    /// Vertical speed, up positive.
    pub vel_y_m_s: f32,
    /// Jet heat, 0 cold to 1 the limit.
    pub jet_heat: f32,
    /// The jets overheated and are out until they cool to the relight heat.
    pub jet_overheated: bool,
    /// Legs shot off: [left, right].
    pub legs_lost: [bool; 2],
}

const G_M_S2: f32 = 9.81;

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

    /// True when the mech is off the ground (falling or on its jets).
    pub fn airborne(&self) -> bool {
        self.vel_y_m_s.abs() > 1e-3 || self.mode == LocomotionMode::Jet
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(&mut self, c: &MechControls, cfg: &MechConfig, ground: &impl Ground, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let (loco, gait, jet) = (&cfg.locomotion, &cfg.gait, &cfg.jet);
        if let Some(req) = c.mode_request {
            let want = match req {
                ModeRequest::Walk => LocomotionMode::Walk,
                ModeRequest::Skate => LocomotionMode::Skate,
                ModeRequest::Jet => LocomotionMode::Jet,
            };
            if want.is_built() && !(want == LocomotionMode::Walk && self.mode == LocomotionMode::Jet) {
                self.mode = want;
            }
        }
        let legs_left = 2 - self.legs_lost.iter().filter(|l| **l).count();
        let flying = self.mode == LocomotionMode::Jet;

        // Throttle lever and ground speed.
        if c.throttle_zero {
            self.throttle = 0.0;
        }
        self.throttle = (self.throttle + c.throttle_axis * loco.throttle_rate_per_s * dt_s).clamp(loco.throttle_min, 1.0);
        let (target, accel, brake) = if flying {
            (self.throttle.max(0.0) * jet.speed_max_m_s, jet.accel_m_s2, jet.accel_m_s2)
        } else {
            // Wading drags the legs back (terrain-water design section 2); a lost leg leaves a limp
            // (mech-damage design section 2); no legs, no walking.
            let wade = (self.wading_depth_m / loco.hip_height_m).clamp(0.0, 1.0);
            let limp = match legs_left {
                2 => 1.0,
                1 => loco.limp_speed_ratio,
                _ => 0.0,
            };
            (
                self.throttle * loco.walk_speed_max_m_s * (1.0 - loco.wade_slowdown * wade) * limp,
                loco.accel_m_s2,
                loco.brake_m_s2,
            )
        };
        let rate = if target.abs() > self.speed_m_s.abs() && target * self.speed_m_s >= 0.0 {
            accel
        } else {
            brake
        };
        self.speed_m_s = approach(self.speed_m_s, target, rate * dt_s);

        // Legs (or jets) turn the heading.
        let turn_scale = if legs_left == 1 && !flying { 0.5 } else { 1.0 };
        self.turn_rate_rad_s = c.turn_axis * loco.turn_rate_deg_s.to_radians() * turn_scale;
        self.heading_rad = (self.heading_rad + self.turn_rate_rad_s * dt_s).rem_euclid(std::f32::consts::TAU);
        let mut moved = self.pos + forward(self.heading_rad) * self.speed_m_s * dt_s;
        if !flying && ground.water_at(moved.x, moved.z).is_some_and(|w| w.depth_m > loco.max_wade_depth_m) {
            // Too deep: the water stops the mech as a wall does.
            moved = self.pos;
            self.speed_m_s = 0.0;
        }
        let pushed = ground.push_out_at(Vec2::new(moved.x, moved.z), loco.body_radius_m, self.pos.y);
        if pushed.distance(Vec2::new(moved.x, moved.z)) > 1e-4 {
            // Walked into a wall: the legs stop where the body stopped.
            let real = (pushed - Vec2::new(self.pos.x, self.pos.z)).length() / dt_s;
            self.speed_m_s = self.speed_m_s.signum() * real.min(self.speed_m_s.abs());
        }

        // Jets: lift against gravity, limited by heat (survey M3: fuel ignored for now).
        let lifting = flying && c.lift_axis > 0.0 && !self.jet_overheated;
        if lifting {
            self.jet_heat += jet.heat_rise_per_s * c.lift_axis * dt_s;
            if self.jet_heat >= 1.0 {
                self.jet_heat = 1.0;
                self.jet_overheated = true;
            }
        } else {
            self.jet_heat = (self.jet_heat - jet.cool_per_s * dt_s).max(0.0);
            if self.jet_overheated && self.jet_heat <= jet.relight_heat {
                self.jet_overheated = false;
            }
        }
        let lift = if lifting { jet.thrust_m_s2 * c.lift_axis } else { 0.0 };
        self.vel_y_m_s += (lift - G_M_S2) * dt_s;
        let sink = if lifting { -jet.sink_max_m_s } else { -60.0 };
        self.vel_y_m_s = self.vel_y_m_s.clamp(sink, jet.climb_max_m_s);
        let mut y = self.pos.y + self.vel_y_m_s * dt_s;
        let support = ground.support_at(pushed.x, pushed.y, self.pos.y);
        if y <= support || (!flying && self.vel_y_m_s <= 0.0 && self.pos.y - support < 1.0) {
            // On the ground (or a roof): stand on it. Touching down ends a flight.
            if flying && self.vel_y_m_s <= 0.0 && !lifting {
                self.mode = LocomotionMode::Walk;
                self.speed_m_s = self.speed_m_s.clamp(-loco.walk_speed_max_m_s, loco.walk_speed_max_m_s);
            }
            y = support;
            self.vel_y_m_s = self.vel_y_m_s.max(0.0);
            if !lifting {
                self.vel_y_m_s = 0.0;
            }
        }
        self.pos = Vec3::new(pushed.x, y, pushed.y);
        self.wading_depth_m = ground.water_at(pushed.x, pushed.y).map_or(0.0, |w| (w.surface_m - y).max(0.0));

        // Torso, independent of the legs: aim moves the target, the torso follows at its rate.
        let twist_max = loco.twist_max_deg.to_radians();
        let [pitch_lo, pitch_hi] = loco.pitch_limits_deg.map(f32::to_radians);
        self.aim_twist_rad = (self.aim_twist_rad - c.aim_delta_deg.x.to_radians()).clamp(-twist_max, twist_max);
        self.aim_pitch_rad = (self.aim_pitch_rad + c.aim_delta_deg.y.to_radians()).clamp(pitch_lo, pitch_hi);
        self.twist_rad = approach(self.twist_rad, self.aim_twist_rad, loco.twist_rate_deg_s.to_radians() * dt_s);
        self.pitch_rad = approach(self.pitch_rad, self.aim_pitch_rad, loco.pitch_rate_deg_s.to_radians() * dt_s);

        // Gait clock: cycles advance by distance over stride; turning in place steps too. In the
        // air the legs hang.
        let v = if self.airborne() { 0.0 } else { self.speed_m_s.abs() };
        let effective = v + if self.airborne() {
            0.0
        } else {
            self.turn_rate_rad_s.abs() * gait.turn_step_radius_m
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
    fn jets_lift_the_mech_until_they_overheat_then_it_lands_and_walks() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let up = MechControls {
            mode_request: Some(ModeRequest::Jet),
            lift_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, up, 10.0, &FlatGround::default());
        assert!(m.pos.y > 150.0, "climbing: {}", m.pos.y);
        let hold = MechControls {
            lift_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, hold, 20.0, &FlatGround::default());
        assert!(m.jet_overheated, "25 s of full lift overheats");
        run(&mut m, MechControls::default(), 60.0, &FlatGround::default());
        assert_eq!(m.pos.y, 0.0, "back on the ground");
        assert_eq!(m.mode, LocomotionMode::Walk);
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
        let mut m = MechState::default();
        let c = MechControls {
            mode_request: Some(ModeRequest::Skate),
            ..Default::default()
        };
        run(&mut m, c, DT, &FlatGround::default());
        assert_eq!(m.mode, LocomotionMode::Walk);
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
