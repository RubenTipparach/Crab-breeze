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
    /// Deepest water the pilot walks into (no swimming yet).
    pub max_wade_depth_m: f32,
    /// Use reaches the mech from this far from `LADDER_base` (its boarding spot, horizontal).
    pub board_reach_m: f32,
    /// Speed of the eyes along the boarding path (climbing the stair).
    pub climb_speed_m_s: f32,
    /// Seated, the view stays within this of the mech's nose unless free look is held
    /// (openspec change `seated-look`).
    pub seated_look_limit_deg: f32,
    /// Seated, holding Use this long climbs out (owner, 2026-10-25: "I should be able to hold E to
    /// exit, there would be a ccircle timer thing that shows progress of confirming E to exit").
    pub exit_hold_s: f32,
    /// The hold counts while the mech is slower than this ("when I'm even close to stop"); the mech
    /// brakes to a stop while it is held, and the pilot climbs out once it has stopped.
    pub exit_near_stop_m_s: f32,
    /// The view's vertical field of view (on foot and seated).
    pub view_fov_deg: f32,
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
            max_wade_depth_m: 1.2,
            board_reach_m: 25.0,
            climb_speed_m_s: 7.0,
            seated_look_limit_deg: 90.0,
            exit_hold_s: 1.0,
            exit_near_stop_m_s: 3.0,
            view_fov_deg: 75.0,
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
        positive("max_wade_depth_m", self.max_wade_depth_m)?;
        positive("board_reach_m", self.board_reach_m)?;
        positive("climb_speed_m_s", self.climb_speed_m_s)?;
        within("seated_look_limit_deg", self.seated_look_limit_deg, 1.0, 179.0)?;
        positive("exit_hold_s", self.exit_hold_s)?;
        positive("exit_near_stop_m_s", self.exit_near_stop_m_s)?;
        within("view_fov_deg", self.view_fov_deg, 30.0, 120.0)?;
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
    /// The way the seat faces (heading plus torso twist): the view comes to rest here on boarding.
    pub seat_yaw_rad: f32,
    /// True when the mech is stopped with its throttle at zero.
    pub stopped: bool,
    /// The mech's ground speed.
    pub speed_m_s: f32,
    /// The mech is destroyed: its pilot ejects.
    pub destroyed: bool,
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
    /// View yaw in the world, radians; 0 faces -z, positive turns left. The player's own, in every
    /// state: only the mouse moves it (owner, 2026-10-24: "never take camera control away from the
    /// player"). On foot it is also the body's heading; seated, the torso turns to follow it.
    pub yaw_rad: f32,
    /// View pitch, radians, up positive.
    pub pitch_rad: f32,
    /// Seated: where `EYE_pilot` was at the last step.
    pub seated_eye: Vec3,
    /// Seated: the view's yaw from the mech's nose after the last step (`coupled_look`).
    pub seated_rel_rad: f32,
    /// Where the pilot is.
    pub mode: PilotMode,
    /// Waiting for the hatch, after pressing Use.
    pub waiting: Option<Waiting>,
    /// Seated: how long Use has been held toward climbing out (0 when it is not).
    pub exit_hold_s: f32,
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
    /// The mech was destroyed and threw its pilot clear (mission-jobs, survey J3).
    Ejected,
}

/// An angle wrapped to -pi..pi.
pub fn wrap_rad(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// The seated view's yaw from the mech's nose after this step (openspec change `seated-look`).
/// `rel` is it now, after the mech turned; `prev_rel` it after the last step; `turn` the mouse's turn
/// this step (radians, left positive); `limit` the coupled limit. Coupled, the view may sit no
/// further round than the larger of the limit and where it already was: the cockpit carries the
/// view when the mech turns away, and the mouse stops at that bound but always turns back toward the
/// nose. Free look is unlimited.
pub fn coupled_look(rel: f32, prev_rel: f32, turn: f32, limit: f32, free_look: bool) -> f32 {
    if free_look {
        return wrap_rad(rel + turn);
    }
    let bound = limit.max(prev_rel.abs()).min(std::f32::consts::PI);
    // The mech turned away: the seat carries the view.
    let carried = rel.clamp(-bound, bound);
    let moved = carried + turn;
    if moved.abs() > bound && moved.abs() > carried.abs() {
        moved.signum() * bound
    } else {
        moved
    }
}

/// An ejected pilot lands this far out in front of the destroyed mech's hatch.
pub const EJECT_OUT_M: f32 = 18.0;

/// The view's last stretch before the coupled limit over which the mech's turn toward it ramps up.
pub const LOOK_TURN_BAND_DEG: f32 = 20.0;

/// The mech's turn toward the seated view (`turn_axis`, -1..1): nothing until the view is within
/// `LOOK_TURN_BAND_DEG` of the coupled limit, then easing up to full at the limit, so the mech
/// starts turning gently and never snaps between turning and not (owner, 2026-10-25: "theres lots
/// of mech jitter when I turn, mech should turn more smoothly"). `None` (free look held, or the
/// view well inside the limit) leaves the turn to the keys.
pub fn look_turn(rel: f32, limit: f32, free_look: bool) -> Option<f32> {
    let band = LOOK_TURN_BAND_DEG.to_radians().min(limit);
    let into = (rel.abs() - (limit - band)) / band;
    if free_look || into <= 0.0 {
        return None;
    }
    let k = into.min(1.0);
    Some(rel.signum() * k * k * (3.0 - 2.0 * k))
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
            seated_rel_rad: 0.0,
            mode: PilotMode::OnFoot,
            waiting: None,
            exit_hold_s: 0.0,
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

    /// From where the pilot stands to the rope's foot (`ladder_base`), straight up the rope to the
    /// hatch's height, onto the sill, then in to the seat (owner, 2026-10-24: "there should be a rope
    /// dangling down for me the player to climb up").
    fn board_path(&self, m: &MechAccess, cfg: &PilotConfig) -> EyePath {
        let up = Vec3::Y * cfg.eye_height_m;
        let foot = m.ladder_base + up;
        EyePath {
            points: vec![self.feet + up, foot, Self::rope_top(m, cfg), m.hatch_entry + up, m.eye_pilot],
            travelled_m: 0.0,
        }
    }

    /// The boarding path backwards: out over the sill, down the rope, standing at its foot.
    fn leave_path(m: &MechAccess, cfg: &PilotConfig) -> EyePath {
        let up = Vec3::Y * cfg.eye_height_m;
        EyePath {
            points: vec![m.eye_pilot, m.hatch_entry + up, Self::rope_top(m, cfg), m.ladder_base + up],
            travelled_m: 0.0,
        }
    }

    /// Thrown clear of a destroyed mech: out of the seat, up through the hatch and 10 m over it, then
    /// down to the ground `EJECT_OUT_M` out in front, landing on foot.
    fn eject_path(m: &MechAccess, cfg: &PilotConfig) -> EyePath {
        let up = Vec3::Y * cfg.eye_height_m;
        let flat = Vec3::new(m.ladder_base.x - m.hatch_entry.x, 0.0, m.ladder_base.z - m.hatch_entry.z).normalize_or(Vec3::Z);
        let land = Vec3::new(m.hatch_entry.x, m.ladder_base.y, m.hatch_entry.z) + flat * EJECT_OUT_M + up;
        let top = m.hatch_entry + up + Vec3::Y * 10.0 + flat * (EJECT_OUT_M * 0.4);
        EyePath {
            points: vec![m.eye_pilot, m.hatch_entry + up, top, land],
            travelled_m: 0.0,
        }
    }

    /// The eye at the top of the rope: above its foot, level with the eye on the hatch's sill.
    fn rope_top(m: &MechAccess, cfg: &PilotConfig) -> Vec3 {
        Vec3::new(m.ladder_base.x, m.hatch_entry.y + cfg.eye_height_m, m.ladder_base.z)
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
                let mut moved = Vec2::new(self.feet.x, self.feet.z) + self.velocity * dt_s;
                if ground.water_at(moved.x, moved.y).is_some_and(|w| w.depth_m > cfg.max_wade_depth_m) {
                    moved = Vec2::new(self.feet.x, self.feet.z);
                    self.velocity = Vec2::ZERO;
                }
                // out of walls at the feet's own height (a deck 600 m up has no buildings round it)
                let p = ground.push_out_at(moved, cfg.body_radius_m, self.feet.y);
                // what the feet stand on: the ground, a roof, or a platform such as the Hermit's deck (crab-airship)
                self.feet = Vec3::new(p.x, ground.support_at(p.x, p.y, self.feet.y), p.y);
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
                // Stepping in: look input pauses while the view is carried into the seat (the
                // owner's design, CLAUDE.md section 8); seated, it faces out of the cockpit.
                path.travelled_m += cfg.climb_speed_m_s * dt_s;
                if path.done() {
                    if let Some(m) = access {
                        self.yaw_rad = m.seat_yaw_rad.rem_euclid(std::f32::consts::TAU);
                        self.seated_rel_rad = wrap_rad(self.yaw_rad - m.heading_rad);
                    }
                    self.pitch_rad = 0.0;
                    self.seated_eye = *path.points.last().unwrap_or(&self.seated_eye);
                    self.mode = PilotMode::Seated;
                    event = Some(PilotEvent::Seated);
                }
            }
            PilotMode::Seated => {
                if let Some(m) = access {
                    self.seated_eye = m.eye_pilot;
                    // Coupled to the mech's nose unless free look is held (seated-look).
                    let rel = coupled_look(
                        wrap_rad(self.yaw_rad - m.heading_rad),
                        self.seated_rel_rad,
                        -look.x.to_radians(),
                        cfg.seated_look_limit_deg.to_radians(),
                        a.free_look,
                    );
                    self.seated_rel_rad = rel;
                    self.yaw_rad = (m.heading_rad + rel).rem_euclid(std::f32::consts::TAU);
                } else {
                    self.yaw_rad = (self.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
                }
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                if let Some(m) = access.filter(|m| m.destroyed) {
                    // The mech is destroyed: the seat throws the pilot out of the hatch, up and clear,
                    // to land on foot beside the wreck (owner, survey J3: "whenever your mech is
                    // destroyed you eject out of yuor mech"). Only the eye's position is carried; where
                    // the pilot looks stays theirs.
                    self.waiting = None;
                    self.exit_hold_s = 0.0;
                    self.mode = PilotMode::Disembarking(Self::eject_path(m, cfg));
                    return Some(PilotEvent::Ejected);
                }
                if let Some(m) = access {
                    // Holding Use near a stop fills the ring; the mech brakes meanwhile (the app
                    // zeroes its throttle while the hold runs), and the pilot climbs out once the
                    // ring is full and the mech has stopped.
                    let near = m.speed_m_s <= cfg.exit_near_stop_m_s;
                    self.exit_hold_s = if a.use_held && near && self.waiting.is_none() {
                        (self.exit_hold_s + dt_s).min(cfg.exit_hold_s)
                    } else {
                        0.0
                    };
                    if self.exit_hold_s >= cfg.exit_hold_s && m.stopped && self.waiting.is_none() {
                        self.exit_hold_s = 0.0;
                        self.waiting = Some(Waiting::ToLeave);
                        if !m.hatch_open {
                            event = Some(PilotEvent::HatchWanted);
                        }
                    }
                    if self.waiting == Some(Waiting::ToLeave) && m.hatch_open {
                        self.waiting = None;
                        self.mode = PilotMode::Disembarking(Self::leave_path(m, cfg));
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
                    self.feet = Vec3::new(feet.x, ground.support_at(feet.x, feet.z, feet.y), feet.z);
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
            seat_yaw_rad: 0.0,
            stopped: true,
            speed_m_s: 0.0,
            destroyed: false,
            hatch_open: true,
        }
    }

    fn hold_use() -> Actions {
        Actions {
            use_held: true,
            ..Default::default()
        }
    }

    /// Holds Use until the pilot starts climbing out (or two seconds pass).
    fn hold_to_exit(p: &mut PilotState, cfg: &PilotConfig, a: &MechAccess) -> Option<PilotEvent> {
        for _ in 0..120 {
            if let Some(e) = p.step(&hold_use(), cfg, &FlatGround::default(), Some(a), DT) {
                return Some(e);
            }
        }
        None
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
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 63.5), 0.0);
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
        for _ in 0..600 {
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
    fn stepping_in_ignores_look_and_seats_the_view_facing_out() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 1.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        let spin = Actions {
            look_delta_deg: Vec2::new(30.0, 10.0),
            ..Default::default()
        };
        for _ in 0..600 {
            p.step(&spin, &cfg, &FlatGround::default(), Some(&access()), DT);
            if p.mode == PilotMode::Seated {
                break;
            }
        }
        assert_eq!(p.mode, PilotMode::Seated);
        assert!(
            p.yaw_rad.abs() < 1e-4 || (p.yaw_rad - std::f32::consts::TAU).abs() < 1e-4,
            "faces the seat's way: {}",
            p.yaw_rad
        );
        assert_eq!(p.pitch_rad, 0.0);
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
    fn boarding_climbs_straight_up_the_rope() {
        let cfg = PilotConfig::default();
        let a = access();
        let p = PilotState::standing(a.ladder_base + Vec3::new(1.0, 0.0, 0.0), 0.0);
        let path = p.board_path(&a, &cfg);
        let (foot, top) = (path.points[1], path.points[2]);
        assert!(
            (foot.x - top.x).abs() < 1e-5 && (foot.z - top.z).abs() < 1e-5,
            "the climb is vertical"
        );
        assert!(
            (top.y - (a.hatch_entry.y + cfg.eye_height_m)).abs() < 1e-5,
            "it reaches the sill's height"
        );
        assert!(
            (foot.y - (a.ladder_base.y + cfg.eye_height_m)).abs() < 1e-5,
            "it starts at the rope's foot"
        );
    }

    #[test]
    fn disembarking_leaves_the_pilot_at_the_ladder() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut p, &cfg, &access());
        assert_eq!(hold_to_exit(&mut p, &cfg, &access()), Some(PilotEvent::DisembarkStarted));
        run_until_settled(&mut p, &cfg, &access());
        assert_eq!(p.mode, PilotMode::OnFoot);
        assert!(p.feet.distance(access().ladder_base) < 1e-3, "{}", p.feet);
    }

    #[test]
    fn a_destroyed_mech_throws_its_pilot_clear_onto_their_feet() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut p, &cfg, &access());
        assert_eq!(p.mode, PilotMode::Seated);
        let wreck = MechAccess {
            destroyed: true,
            stopped: false,
            speed_m_s: 6.0,
            hatch_open: false,
            ..access()
        };
        let e = p.step(&Actions::default(), &cfg, &FlatGround::default(), Some(&wreck), DT);
        assert_eq!(e, Some(PilotEvent::Ejected));
        run_until_settled(&mut p, &cfg, &wreck);
        assert_eq!(p.mode, PilotMode::OnFoot);
        let out = Vec2::new(p.feet.x - wreck.hatch_entry.x, p.feet.z - wreck.hatch_entry.z).length();
        assert!(out > EJECT_OUT_M * 0.9, "landed clear of the wreck: {out}");
    }

    #[test]
    fn a_tap_of_use_does_not_climb_out_but_a_held_one_does() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut p, &cfg, &access());
        let tap = Actions {
            use_pressed: true,
            use_held: true,
            ..Default::default()
        };
        assert_eq!(p.step(&tap, &cfg, &FlatGround::default(), Some(&access()), DT), None);
        p.step(&Actions::default(), &cfg, &FlatGround::default(), Some(&access()), DT);
        assert_eq!(p.exit_hold_s, 0.0, "letting go empties the ring");
        assert_eq!(p.mode, PilotMode::Seated);
        assert_eq!(hold_to_exit(&mut p, &cfg, &access()), Some(PilotEvent::DisembarkStarted));
    }

    #[test]
    fn the_exit_ring_fills_near_a_stop_and_waits_for_the_mech_to_stop() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        p.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut p, &cfg, &access());
        let rolling = MechAccess {
            stopped: false,
            speed_m_s: 1.5,
            ..access()
        };
        assert_eq!(hold_to_exit(&mut p, &cfg, &rolling), None, "still rolling: no exit yet");
        assert_eq!(p.exit_hold_s, cfg.exit_hold_s, "but the ring is full");
        assert_eq!(hold_to_exit(&mut p, &cfg, &access()), Some(PilotEvent::DisembarkStarted));
        let fast = MechAccess {
            stopped: false,
            speed_m_s: 10.0,
            ..access()
        };
        let mut q = PilotState::standing(Vec3::new(0.0, 0.0, 5.5), 0.0);
        q.step(&press_use(), &cfg, &FlatGround::default(), Some(&access()), DT);
        run_until_settled(&mut q, &cfg, &access());
        hold_to_exit(&mut q, &cfg, &fast);
        assert_eq!(q.exit_hold_s, 0.0, "too fast: the ring does not fill");
    }

    #[test]
    fn the_view_stops_at_ninety_degrees_from_the_nose() {
        let limit = 90f32.to_radians();
        let mut rel = 0.0;
        for _ in 0..100 {
            rel = coupled_look(rel, rel, 5f32.to_radians(), limit, false);
        }
        assert!((rel - limit).abs() < 1e-5, "{}", rel.to_degrees());
        // Turning back always applies.
        let back = coupled_look(rel, rel, -10f32.to_radians(), limit, false);
        assert!((back - (limit - 10f32.to_radians())).abs() < 1e-5);
    }

    #[test]
    fn pushing_past_the_edge_turns_the_mech_at_its_turn_rate() {
        let limit = 90f32.to_radians();
        assert_eq!(look_turn(limit, limit, false), Some(1.0));
        assert_eq!(look_turn(-limit, limit, false), Some(-1.0));
        assert_eq!(look_turn(limit * 0.5, limit, false), None);
        assert_eq!(look_turn(limit, limit, true), None);
        // Partway into the band the turn is partway: it ramps, it does not snap.
        let half = look_turn(limit - (LOOK_TURN_BAND_DEG * 0.5).to_radians(), limit, false).unwrap();
        assert!(half > 0.3 && half < 0.7, "{half}");
    }

    #[test]
    fn free_look_can_look_behind() {
        let limit = 90f32.to_radians();
        let mut rel = 0.0;
        for _ in 0..36 {
            rel = coupled_look(rel, rel, 5f32.to_radians(), limit, true);
        }
        assert!((rel.abs() - std::f32::consts::PI).abs() < 1e-3, "{}", rel.to_degrees());
    }

    #[test]
    fn leaving_free_look_never_moves_the_view() {
        let limit = 90f32.to_radians();
        let behind = 150f32.to_radians();
        // No mouse, the mech has not turned yet: the view stays where free look left it.
        assert!((coupled_look(behind, behind, 0.0, limit, false) - behind).abs() < 1e-6);
        // The mech turns toward it: the view keeps its world direction, so `rel` shrinks, and it
        // cannot be pushed back out.
        let after = coupled_look(behind - 0.05, behind, 0.0, limit, false);
        assert!((after - (behind - 0.05)).abs() < 1e-6);
        let pushed = coupled_look(after, after, 0.2, limit, false);
        assert!(pushed <= after + 1e-6);
    }

    #[test]
    fn the_cockpit_carries_the_view_when_the_mech_turns_away() {
        let limit = 90f32.to_radians();
        // At the edge, the mech turns 10 degrees away from the view: the view is held at the edge.
        let rel = coupled_look(limit + 10f32.to_radians(), limit, 0.0, limit, false);
        assert!((rel - limit).abs() < 1e-6, "{}", rel.to_degrees());
    }
}
