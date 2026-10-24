//! The pilot (openspec changes `pilot-boarding` and `mech-boarding`): on foot in first person,
//! asking the mech to sit and open, climbing its stair along one continuous path into
//! `EYE_pilot`, seated, and climbing back out. Pure: state, actions, data, the ground and the
//! mech's socket positions in; state out.

use crate::data::{Validate, positive, within};
use crate::input::Actions;
use crate::world::Ground;
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// Pilot tuning, loaded from `data/pilot/pilot.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct PilotConfig {
    /// Walking speed.
    pub walk_speed_m_s: f32,
    /// Running speed.
    pub run_speed_m_s: f32,
    /// How fast the body reaches its speed.
    pub accel_m_s2: f32,
    /// Eye height above the feet.
    pub eye_height_m: f32,
    /// Body radius for building collision.
    pub body_radius_m: f32,
    /// Look pitch limit up and down.
    pub pitch_limit_deg: f32,
    /// Use reaches `LADDER_base` (the stair's foot) from this far (horizontal).
    pub board_reach_m: f32,
    /// Speed of the eyes along the boarding path (climbing the stair).
    pub climb_speed_m_s: f32,
    /// Free look limit, yaw either side, seated.
    pub free_look_yaw_deg: f32,
    /// Free look limit, pitch up and down, seated.
    pub free_look_pitch_deg: f32,
    /// How fast the head returns to the front when free look is released.
    pub free_look_return_deg_s: f32,
}

impl Default for PilotConfig {
    fn default() -> Self {
        Self {
            walk_speed_m_s: 1.6,
            run_speed_m_s: 4.5,
            accel_m_s2: 12.0,
            eye_height_m: 1.7,
            body_radius_m: 0.35,
            pitch_limit_deg: 85.0,
            board_reach_m: 3.0,
            climb_speed_m_s: 1.2,
            free_look_yaw_deg: 120.0,
            free_look_pitch_deg: 70.0,
            free_look_return_deg_s: 180.0,
        }
    }
}

impl Validate for PilotConfig {
    fn validate(&self) -> Result<(), String> {
        positive("walk_speed_m_s", self.walk_speed_m_s)?;
        positive("run_speed_m_s", self.run_speed_m_s)?;
        positive("accel_m_s2", self.accel_m_s2)?;
        positive("eye_height_m", self.eye_height_m)?;
        positive("body_radius_m", self.body_radius_m)?;
        within("pitch_limit_deg", self.pitch_limit_deg, 0.0, 89.9)?;
        positive("board_reach_m", self.board_reach_m)?;
        positive("climb_speed_m_s", self.climb_speed_m_s)?;
        within("free_look_yaw_deg", self.free_look_yaw_deg, 0.0, 180.0)?;
        within("free_look_pitch_deg", self.free_look_pitch_deg, 0.0, 89.9)?;
        positive("free_look_return_deg_s", self.free_look_return_deg_s)?;
        Ok(())
    }
}

/// A path for the pilot's eyes, walked at a fixed speed.
#[derive(Clone, Debug, PartialEq)]
pub struct EyePath {
    /// Points in order, world metres.
    pub points: Vec<Vec3>,
    /// Distance travelled so far.
    pub travelled_m: f32,
}

impl EyePath {
    /// Total length.
    pub fn length_m(&self) -> f32 {
        self.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// The point `travelled_m` along the path, and the index of the leg it is on.
    pub fn at(&self) -> (Vec3, usize) {
        let mut left = self.travelled_m;
        for (i, w) in self.points.windows(2).enumerate() {
            let len = w[0].distance(w[1]);
            if left <= len {
                let t = if len > 0.0 { left / len } else { 1.0 };
                return (w[0].lerp(w[1], t), i);
            }
            left -= len;
        }
        (*self.points.last().unwrap_or(&Vec3::ZERO), self.points.len().saturating_sub(2))
    }

    /// Fraction of the last leg done, 0..1 (the view turns to the seat over it).
    pub fn last_leg_progress(&self) -> f32 {
        let n = self.points.len();
        if n < 2 {
            return 1.0;
        }
        let before: f32 = self.points[..n - 1].windows(2).map(|w| w[0].distance(w[1])).sum();
        let last = self.points[n - 2].distance(self.points[n - 1]);
        if last <= 0.0 {
            1.0
        } else {
            ((self.travelled_m - before) / last).clamp(0.0, 1.0)
        }
    }

    fn done(&self) -> bool {
        self.travelled_m >= self.length_m()
    }
}

/// Where the pilot is.
#[derive(Clone, Debug, PartialEq)]
pub enum PilotMode {
    /// Walking.
    OnFoot,
    /// Climbing the stair and into the seat.
    Boarding(EyePath),
    /// In the seat; the camera is `EYE_pilot`.
    Seated,
    /// Climbing out and down the stair.
    Disembarking(EyePath),
}

/// The mech's boarding sockets in the world this step, and whether it is stopped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MechAccess {
    /// `LADDER_base`, the stair's foot, where a pilot on foot starts (feet).
    pub ladder_base: Vec3,
    /// `HATCH_entry`, the platform at the stair's top (feet).
    pub hatch_entry: Vec3,
    /// `EYE_pilot`, the seated eyes.
    pub eye_pilot: Vec3,
    /// The mech's heading (radians; 0 faces -z), for standing up facing the same way.
    pub heading_rad: f32,
    /// True when the mech is stopped with its throttle at zero.
    pub stopped: bool,
    /// True when the mech sits with its hatch open and stair down.
    pub hatch_open: bool,
}

/// What the pilot is waiting for the hatch to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waiting {
    /// Board as soon as the stair is down.
    ToBoard,
    /// Climb out as soon as the stair is down.
    ToLeave,
}

/// The pilot's state.
#[derive(Clone, Debug, PartialEq)]
pub struct PilotState {
    /// Feet, world metres (on foot); the eyes' position while boarding is in the path.
    pub feet: Vec3,
    /// Horizontal velocity on foot (x, z).
    pub velocity: Vec2,
    /// View yaw, radians; 0 faces -z, positive turns left. On foot it is also the body's heading.
    pub yaw_rad: f32,
    /// View pitch, radians, up positive.
    pub pitch_rad: f32,
    /// Seated: where `EYE_pilot` was at the last step.
    pub seated_eye: Vec3,
    /// Seated free look yaw from the seat's forward.
    pub head_yaw_rad: f32,
    /// Seated free look pitch.
    pub head_pitch_rad: f32,
    /// Where the pilot is.
    pub mode: PilotMode,
    /// Waiting for the hatch, after pressing Use.
    pub waiting: Option<Waiting>,
}

/// What happened this step that other systems react to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotEvent {
    /// The pilot asked the mech to sit and open its hatch.
    HatchWanted,
    /// Boarding began.
    BoardingStarted,
    /// The pilot is in the seat.
    Seated,
    /// Climbing out began.
    DisembarkStarted,
    /// The pilot is standing at the ladder.
    Disembarked,
}

fn approach(v: f32, target: f32, step: f32) -> f32 {
    v + (target - v).clamp(-step, step)
}

impl PilotState {
    /// A pilot standing at `feet`, looking along `yaw_rad`.
    pub fn standing(feet: Vec3, yaw_rad: f32) -> Self {
        Self {
            feet,
            velocity: Vec2::ZERO,
            yaw_rad,
            pitch_rad: 0.0,
            seated_eye: Vec3::ZERO,
            head_yaw_rad: 0.0,
            head_pitch_rad: 0.0,
            mode: PilotMode::OnFoot,
            waiting: None,
        }
    }

    /// The eyes' world position. Seated, it is `EYE_pilot` as of the last step; the app reads the
    /// socket itself between steps.
    pub fn eye(&self, cfg: &PilotConfig) -> Vec3 {
        match &self.mode {
            PilotMode::Boarding(p) | PilotMode::Disembarking(p) => p.at().0,
            PilotMode::Seated => self.seated_eye,
            _ => self.feet + Vec3::Y * cfg.eye_height_m,
        }
    }

    /// Horizontal distance from the feet to the stair's foot.
    pub fn distance_to_stair(&self, access: &MechAccess) -> f32 {
        Vec2::new(self.feet.x - access.ladder_base.x, self.feet.z - access.ladder_base.z).length()
    }

    /// True when Use would reach the mech now.
    pub fn can_board(&self, access: &MechAccess, cfg: &PilotConfig) -> bool {
        self.mode == PilotMode::OnFoot && access.stopped && self.distance_to_stair(access) <= cfg.board_reach_m
    }

    fn board_path(&self, m: &MechAccess, cfg: &PilotConfig) -> EyePath {
        let up = Vec3::Y * cfg.eye_height_m;
        EyePath {
            points: vec![self.feet + up, m.ladder_base + up, m.hatch_entry + up, m.eye_pilot],
            travelled_m: 0.0,
        }
    }

    fn leave_path(m: &MechAccess, cfg: &PilotConfig) -> EyePath {
        let up = Vec3::Y * cfg.eye_height_m;
        EyePath {
            points: vec![m.eye_pilot, m.hatch_entry + up, m.ladder_base + up],
            travelled_m: 0.0,
        }
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(
        &mut self,
        a: &Actions,
        cfg: &PilotConfig,
        ground: &impl Ground,
        access: Option<&MechAccess>,
        dt_s: f32,
    ) -> Option<PilotEvent> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        let look = a.look_delta_deg;
        let pitch_limit = cfg.pitch_limit_deg.to_radians();
        let mut event = None;
        match &mut self.mode {
            PilotMode::OnFoot => {
                self.yaw_rad = (self.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                let speed = if a.run { cfg.run_speed_m_s } else { cfg.walk_speed_m_s };
                let fwd = Vec2::new(-self.yaw_rad.sin(), -self.yaw_rad.cos());
                let right = Vec2::new(-fwd.y, fwd.x);
                let wish = (fwd * a.move_xy.y + right * a.move_xy.x).clamp_length_max(1.0) * speed;
                let dv = (wish - self.velocity).clamp_length_max(cfg.accel_m_s2 * dt_s);
                self.velocity += dv;
                let moved = Vec2::new(self.feet.x, self.feet.z) + self.velocity * dt_s;
                let p = ground.push_out(moved, cfg.body_radius_m);
                self.feet = Vec3::new(p.x, ground.height_at(p.x, p.y), p.y);
                if let Some(m) = access {
                    if a.use_pressed && self.can_board(m, cfg) && self.waiting.is_none() {
                        self.waiting = Some(Waiting::ToBoard);
                        if !m.hatch_open {
                            event = Some(PilotEvent::HatchWanted);
                        }
                    }
                    if self.waiting == Some(Waiting::ToBoard) {
                        if self.distance_to_stair(m) > cfg.board_reach_m {
                            self.waiting = None;
                        } else if m.hatch_open {
                            self.waiting = None;
                            self.velocity = Vec2::ZERO;
                            self.mode = PilotMode::Boarding(self.board_path(m, cfg));
                            event = Some(PilotEvent::BoardingStarted);
                        }
                    }
                }
            }
            PilotMode::Boarding(path) => {
                self.yaw_rad = (self.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                path.travelled_m += cfg.climb_speed_m_s * dt_s;
                if path.done() {
                    self.seated_eye = *path.points.last().unwrap_or(&self.seated_eye);
                    self.mode = PilotMode::Seated;
                    self.head_yaw_rad = 0.0;
                    self.head_pitch_rad = 0.0;
                    event = Some(PilotEvent::Seated);
                }
            }
            PilotMode::Seated => {
                if let Some(m) = access {
                    self.seated_eye = m.eye_pilot;
                }
                if a.free_look {
                    let (ly, lp) = (cfg.free_look_yaw_deg.to_radians(), cfg.free_look_pitch_deg.to_radians());
                    self.head_yaw_rad = (self.head_yaw_rad - look.x.to_radians()).clamp(-ly, ly);
                    self.head_pitch_rad = (self.head_pitch_rad + look.y.to_radians()).clamp(-lp, lp);
                } else {
                    let step = cfg.free_look_return_deg_s.to_radians() * dt_s;
                    self.head_yaw_rad = approach(self.head_yaw_rad, 0.0, step);
                    self.head_pitch_rad = approach(self.head_pitch_rad, 0.0, step);
                }
                if let Some(m) = access {
                    if a.use_pressed && m.stopped && self.waiting.is_none() {
                        self.waiting = Some(Waiting::ToLeave);
                        if !m.hatch_open {
                            event = Some(PilotEvent::HatchWanted);
                        }
                    }
                    if self.waiting == Some(Waiting::ToLeave) && m.hatch_open {
                        self.waiting = None;
                        self.mode = PilotMode::Disembarking(Self::leave_path(m, cfg));
                        self.yaw_rad = m.heading_rad + self.head_yaw_rad;
                        self.pitch_rad = self.head_pitch_rad;
                        event = Some(PilotEvent::DisembarkStarted);
                    }
                }
            }
            PilotMode::Disembarking(path) => {
                self.yaw_rad = (self.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                path.travelled_m += cfg.climb_speed_m_s * dt_s;
                if path.done() {
                    let end = *path.points.last().unwrap_or(&self.feet);
                    let feet = end - Vec3::Y * cfg.eye_height_m;
                    self.feet = Vec3::new(feet.x, ground.height_at(feet.x, feet.z), feet.z);
                    self.mode = PilotMode::OnFoot;
                    event = Some(PilotEvent::Disembarked);
                }
            }
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::{Building, Rect};
    use crate::world::{BuildingGrid, FlatGround};

    const DT: f32 = 1.0 / 60.0;

    fn access() -> MechAccess {
        MechAccess {
            ladder_base: Vec3::new(0.0, 0.0, 3.5),
            hatch_entry: Vec3::new(0.0, 10.6, -1.4),
            eye_pilot: Vec3::new(0.0, 9.6, -2.0),
            heading_rad: 0.0,
            stopped: true,
            hatch_open: true,
        }
    }

    fn press_use() -> Actions {
        Actions {
            use_pressed: true,
            ..Default::default()
        }
    }

    fn run_until_settled(p: &mut PilotState, cfg: &PilotConfig, a: &Access) -> Vec<Vec3> {
        let mut eyes = vec![p.eye(cfg)];
        for _ in 0..2000 {
            p.step(&Actions::default(), cfg, &FlatGround::default(), Some(a), DT);
            eyes.push(p.eye(cfg));
            if matches!(p.mode, PilotMode::Seated | PilotMode::OnFoot) {
                break;
            }
        }
        eyes
    }

    type Access = MechAccess;

    #[test]
    fn the_shipped_pilot_data_loads() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/pilot/pilot.ron");
        crate::data::load_ron::<PilotConfig>(&path).expect("pilot data loads");
    }

    #[test]
    fn walking_forward_moves_along_the_view() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::ZERO, std::f32::consts::FRAC_PI_2);
        let fwd = Actions {
            move_xy: Vec2::new(0.0, 1.0),
            ..Default::default()
        };
        for _ in 0..120 {
            p.step(&fwd, &cfg, &FlatGround::default(), None, DT);
        }
        assert!(p.feet.x < -2.5 && p.feet.z.abs() < 1e-3, "yaw +90 degrees faces -x: {}", p.feet);
    }

    #[test]
    fn the_pilot_cannot_walk_through_a_building() {
        let cfg = PilotConfig::default();
        let wall = Building {
            id: 0,
            city: 0,
            footprint: Rect {
                min: Vec2::new(-10.0, -6.0),
                max: Vec2::new(10.0, -4.0),
            },
            storeys: 3,
            height_m: 10.5,
        };
        let ground = FlatGround {
            walls: BuildingGrid::new(&[wall], 256.0, 64.0),
        };
        let mut p = PilotState::standing(Vec3::ZERO, 0.0);
        let fwd = Actions {
            move_xy: Vec2::new(0.0, 1.0),
            run: true,
            ..Default::default()
        };
        for _ in 0..300 {
            p.step(&fwd, &cfg, &ground, None, DT);
        }
        assert!(p.feet.z >= -4.0 + cfg.body_radius_m - 1e-3, "{}", p.feet);
    }

    #[test]
    fn use_far_from_the_ladder_does_nothing() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 13.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(p.mode, PilotMode::OnFoot);
    }

    #[test]
    fn a_walking_mech_cannot_be_boarded() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 4.5), 0.0);
        let moving = MechAccess {
            stopped: false,
            ..access()
        };
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&moving), DT);
        assert_eq!(p.mode, PilotMode::OnFoot);
    }

    #[test]
    fn use_at_a_closed_mech_asks_it_to_open_and_boards_once_it_has() {
        let cfg = PilotConfig::default();
        let closed = MechAccess {
            hatch_open: false,
            ..access()
        };
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        let e = p.step(&press_use(), &cfg, &FlatGround::default(), Some(&closed), DT);
        assert_eq!(e, Some(PilotEvent::HatchWanted));
        for _ in 0..60 {
            p.step(&Actions::default(), &cfg, &FlatGround::default(), Some(&closed), DT);
        }
        assert_eq!(p.mode, PilotMode::OnFoot, "waits while the hatch opens");
        let e = p.step(&Actions::default(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(e, Some(PilotEvent::BoardingStarted), "one press is enough");
    }

    #[test]
    fn walking_away_stops_waiting_for_the_hatch() {
        let cfg = PilotConfig::default();
        let closed = MechAccess {
            hatch_open: false,
            ..access()
        };
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&closed), DT);
        let back = Actions {
            move_xy: Vec2::new(0.0, -1.0),
            run: true,
            ..Default::default()
        };
        for _ in 0..120 {
            p.step(&back, &cfg, &FlatGround::default(), Some(&closed), DT);
        }
        assert_eq!(p.waiting, None);
        p.step(&Actions::default(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(p.mode, PilotMode::OnFoot);
    }

    #[test]
    fn boarding_ends_seated_at_the_eye_point() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        let e = p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(e, Some(PilotEvent::BoardingStarted));
        let eyes = run_until_settled(&mut p, &cfg, &access());
        assert_eq!(p.mode, PilotMode::Seated);
        assert!(eyes.last().unwrap().distance(access().eye_pilot) < 1e-4);
    }

    #[test]
    fn the_boarding_path_is_continuous() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        let start = p.eye(&cfg);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        let eyes = run_until_settled(&mut p, &cfg, &access());
        assert!(eyes[0].distance(start) < 1e-4, "boarding starts where the eyes were");
        for w in eyes.windows(2) {
            assert!(
                w[0].distance(w[1]) <= cfg.climb_speed_m_s * DT + 1e-4,
                "a jump from {} to {}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn disembarking_leaves_the_pilot_at_the_ladder() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut p, &cfg, &access());
        let e = p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(e, Some(PilotEvent::DisembarkStarted));
        run_until_settled(&mut p, &cfg, &access());
        assert_eq!(p.mode, PilotMode::OnFoot);
        assert!(p.feet.distance(access().ladder_base) < 1e-3, "{}", p.feet);
    }
}
