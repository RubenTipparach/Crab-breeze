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
    /// Deepest water the pilot wades; deeper, they swim.
    pub max_wade_depth_m: f32,
    /// Swimming speed (owner, 2026-10-25: "I cant move or swim in water in fps").
    pub swim_speed_m_s: f32,
    /// How fast a swimmer under the surface rises to it.
    pub swim_rise_m_s: f32,
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
    /// The view's vertical field of view seated in the cockpit, degrees (the Options page's setting starts here).
    pub view_fov_deg: f32,
    /// On foot, degrees: narrower, a first-person shooter's view (owner, 2026-10-26: "normal fps fov is 75, it grows
    /// to 95 when inside the mech"; openspec change `seated-fov`).
    pub on_foot_fov_deg: f32,
    /// Seconds the view takes to widen as the pilot sits, or narrow as they stand.
    pub fov_change_s: f32,
    /// On foot, Space jumps at this upward speed (owner, 2026-10-25: "my fps character needs jump!").
    pub jump_speed_m_s: f32,
    /// What pulls a jumping pilot back down.
    pub gravity_m_s2: f32,
    /// Use reaches a car from this far from its driver's door, metres (car-driving).
    pub car_reach_m: f32,
    /// Getting in or out of a car carries the eyes between the door and the seat over this long,
    /// seconds.
    pub car_step_s: f32,
    /// The pilot must face the car within this angle to get in, degrees.
    pub car_face_deg: f32,
    /// A car slower than this counts as stopped for getting in and out, m/s.
    pub car_stopped_m_s: f32,
}

/// The seated view's blend (0 on foot, 1 seated) one step on: toward 1 while `seated`, toward 0 otherwise, the whole
/// way in `change_s` (openspec change `seated-fov`).
pub fn fov_blend(blend: f32, seated: bool, dt_s: f32, change_s: f32) -> f32 {
    let step = if change_s > 0.0 { dt_s / change_s } else { 1.0 };
    let to = if seated { 1.0 } else { 0.0 };
    (blend + (to - blend).clamp(-step, step)).clamp(0.0, 1.0)
}

/// The view's vertical field of view, degrees, at `blend`: the player's seated setting `setting_deg`, and on foot
/// that less the data's difference between the two (never under 30), eased in and out (a smoothstep of the blend).
pub fn fov_deg(cfg: &PilotConfig, setting_deg: f32, blend: f32) -> f32 {
    let foot = (setting_deg - (cfg.view_fov_deg - cfg.on_foot_fov_deg)).max(30.0);
    let b = blend.clamp(0.0, 1.0);
    let s = b * b * (3.0 - 2.0 * b);
    foot + (setting_deg - foot) * s
}

/// Ground this far below the feet is stepped down onto, not fallen to: walking down a slope or a
/// kerb keeps the feet on the ground.
const STEP_DOWN_M: f32 = 0.5;

/// A swimmer's eyes ride this far above the water's surface.
const SWIM_EYE_ABOVE_M: f32 = 0.15;

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
            swim_speed_m_s: 2.5,
            swim_rise_m_s: 1.5,
            board_reach_m: 25.0,
            climb_speed_m_s: 7.0,
            seated_look_limit_deg: 90.0,
            exit_hold_s: 1.0,
            exit_near_stop_m_s: 3.0,
            view_fov_deg: 75.0,
            on_foot_fov_deg: 75.0,
            fov_change_s: 0.8,
            jump_speed_m_s: 4.4,
            gravity_m_s2: 9.81,
            car_reach_m: 1.2,
            car_step_s: 0.8,
            car_face_deg: 75.0,
            car_stopped_m_s: 0.5,
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
        positive("swim_speed_m_s", self.swim_speed_m_s)?;
        positive("swim_rise_m_s", self.swim_rise_m_s)?;
        positive("board_reach_m", self.board_reach_m)?;
        positive("climb_speed_m_s", self.climb_speed_m_s)?;
        within("seated_look_limit_deg", self.seated_look_limit_deg, 1.0, 179.0)?;
        positive("exit_hold_s", self.exit_hold_s)?;
        positive("exit_near_stop_m_s", self.exit_near_stop_m_s)?;
        within("view_fov_deg", self.view_fov_deg, 30.0, 120.0)?;
        within("on_foot_fov_deg", self.on_foot_fov_deg, 30.0, self.view_fov_deg)?;
        positive("fov_change_s", self.fov_change_s)?;
        within("jump_speed_m_s", self.jump_speed_m_s, 0.0, 15.0)?;
        positive("gravity_m_s2", self.gravity_m_s2)?;
        positive("car_reach_m", self.car_reach_m)?;
        positive("car_step_s", self.car_step_s)?;
        within("car_face_deg", self.car_face_deg, 1.0, 180.0)?;
        positive("car_stopped_m_s", self.car_stopped_m_s)?;
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
    /// Getting into a car: the eyes carried from beside its door to `EYE_driver` (car-driving).
    EnteringCar(EyePath),
    /// In a car's driver's seat; the eyes ride `EYE_driver`.
    Driving,
    /// Getting out of a car: the eyes carried from the seat to beside its door.
    ExitingCar(EyePath),
}

impl PilotMode {
    /// Getting into, driving or getting out of a car.
    pub fn in_car(&self) -> bool {
        matches!(self, Self::EnteringCar(_) | Self::Driving | Self::ExitingCar(_))
    }
}

/// The car nearest the pilot this step, or the one they are in, in the world (car-driving).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarAccess {
    /// Its id in the traffic.
    pub id: u32,
    /// `DOOR_driver`: where a person stands to open the driver's door, on the ground.
    pub door: Vec3,
    /// `EYE_driver`: the driver's eyes.
    pub eye: Vec3,
    /// Its middle on the ground.
    pub centre: Vec3,
    /// Heading, radians (0 faces -z, positive turns left).
    pub heading_rad: f32,
    /// Its speed, m/s.
    pub speed_m_s: f32,
    /// A wreck: it cannot be taken, and whoever is in it is thrown clear.
    pub wrecked: bool,
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
    /// Get into this car (its traffic id) as soon as it stops.
    ToDrive(u32),
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
    /// Vertical velocity on foot, up positive: 0 on the ground, set by a jump.
    pub vel_y_m_s: f32,
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
    /// The pilot wants this moving car: it should stop for them (car-driving).
    CarHailed(u32),
    /// Getting into this car began.
    EnteringCar(u32),
    /// In this car's driver's seat.
    InCar(u32),
    /// Getting out of this car began.
    ExitingCar(u32),
    /// Standing beside this car's door.
    OutOfCar(u32),
    /// This car was wrecked with the pilot in it: they are thrown clear.
    ThrownFromCar(u32),
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

/// The mech's turn toward the seated view (`turn_axis`, -1..1): in proportion to how far the view is
/// off the mech's nose, full at half the coupled limit (45 degrees of 90), so a small look turns it
/// slowly and a large one fast, and it settles onto the view as the gap closes (owner, 2026-10-25:
/// "if move a little bit turn mech slowly to match my cursor, if move a lot, the move faster"). The
/// legs' turn rate eases on top of this (`turn_accel_deg_s2`). `None` while free look is held.
pub fn look_turn(rel: f32, limit: f32, free_look: bool) -> Option<f32> {
    if free_look || !rel.is_finite() {
        return None;
    }
    Some((rel / (0.5 * limit).max(1e-3)).clamp(-1.0, 1.0))
}

impl PilotState {
    /// The feet's height after this step on foot, over ground at `floor_m`: on the ground they follow
    /// it (down steps up to `STEP_DOWN_M`), and Space (the lift key, which jumps on foot) leaves it at
    /// `jump_speed_m_s`; in the air they rise and fall under gravity until they land.
    fn vertical(&mut self, a: &Actions, cfg: &PilotConfig, floor_m: f32, dt_s: f32) -> f32 {
        let grounded = self.vel_y_m_s == 0.0 && self.feet.y - floor_m < STEP_DOWN_M;
        if grounded && a.lift_axis <= 0.0 {
            return floor_m;
        }
        if grounded {
            self.vel_y_m_s = cfg.jump_speed_m_s;
        }
        self.vel_y_m_s -= cfg.gravity_m_s2 * dt_s;
        let y = self.feet.y + self.vel_y_m_s * dt_s;
        if y <= floor_m && self.vel_y_m_s <= 0.0 {
            self.vel_y_m_s = 0.0;
            return floor_m;
        }
        y.max(floor_m)
    }

    /// A pilot standing at `feet`, looking along `yaw_rad`.
    pub fn standing(feet: Vec3, yaw_rad: f32) -> Self {
        Self {
            feet,
            velocity: Vec2::ZERO,
            vel_y_m_s: 0.0,
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
            PilotMode::Boarding(p) | PilotMode::Disembarking(p) | PilotMode::EnteringCar(p) | PilotMode::ExitingCar(p) => p.at().0,
            PilotMode::Seated | PilotMode::Driving => self.seated_eye,
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
            // in, at or out of a car: `step_car` moves the pilot
            PilotMode::EnteringCar(_) | PilotMode::Driving | PilotMode::ExitingCar(_) => {}
            PilotMode::OnFoot => {
                self.yaw_rad = (self.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                // In water deeper than wading, the pilot swims: slower, rising to float with their eyes just
                // above the surface, and out onto the shore by walking up it.
                let swim = ground
                    .water_at(self.feet.x, self.feet.z)
                    .filter(|w| w.depth_m > cfg.max_wade_depth_m && self.feet.y <= w.surface_m);
                let speed = match (swim, a.run) {
                    (Some(_), _) => cfg.swim_speed_m_s,
                    (None, true) => cfg.run_speed_m_s,
                    (None, false) => cfg.walk_speed_m_s,
                };
                let fwd = Vec2::new(-self.yaw_rad.sin(), -self.yaw_rad.cos());
                let right = Vec2::new(-fwd.y, fwd.x);
                let wish = (fwd * a.move_xy.y + right * a.move_xy.x).clamp_length_max(1.0) * speed;
                let dv = (wish - self.velocity).clamp_length_max(cfg.accel_m_s2 * dt_s);
                self.velocity += dv;
                let moved = Vec2::new(self.feet.x, self.feet.z) + self.velocity * dt_s;
                // out of walls at the feet's own height (a deck 600 m up has no buildings round it)
                let p = ground.push_out_at(moved, cfg.body_radius_m, self.feet.y);
                // what the feet stand on: the ground, a roof, or a platform such as the Hermit's deck (crab-airship)
                let floor = ground.support_at(p.x, p.y, self.feet.y);
                let y = match swim {
                    Some(w) => {
                        self.vel_y_m_s = 0.0;
                        let float = w.surface_m + SWIM_EYE_ABOVE_M - cfg.eye_height_m;
                        (self.feet.y + (float - self.feet.y).clamp(-cfg.swim_rise_m_s * dt_s, cfg.swim_rise_m_s * dt_s)).max(floor)
                    }
                    None => self.vertical(a, cfg, floor, dt_s),
                };
                self.feet = Vec3::new(p.x, y, p.y);
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

impl PilotState {
    /// True when Use would reach this car's driver's door now: on foot, within `car_reach_m` of the
    /// door, facing the car, and it is not a wreck.
    pub fn can_take(&self, car: &CarAccess, cfg: &PilotConfig) -> bool {
        if self.mode != PilotMode::OnFoot || car.wrecked {
            return false;
        }
        let near = Vec2::new(self.feet.x - car.door.x, self.feet.z - car.door.z).length() <= cfg.car_reach_m;
        let to = Vec2::new(car.centre.x - self.feet.x, car.centre.z - self.feet.z).normalize_or_zero();
        let look = Vec2::new(-self.yaw_rad.sin(), -self.yaw_rad.cos());
        near && look.dot(to) >= cfg.car_face_deg.to_radians().cos()
    }

    /// One fixed step of the pilot and a car (openspec change `car-driving`): Use at a car's driver
    /// door gets in (a moving car is hailed first and entered once it stops), the eyes are carried to
    /// `EYE_driver` over `car_step_s` while the view stays the player's, driving the eyes ride the
    /// seat and the mouse looks round from the car's nose, and Use with the car stopped gets out
    /// beside the door. `car` is the car nearest the door on foot, or the one the pilot is in. Run
    /// before `step` each fixed step; it does nothing on foot unless Use reaches a car, and returns
    /// what happened. While it returns an event or the pilot is in a car, `step` has nothing to do
    /// with the Use press.
    pub fn step_car(
        &mut self,
        a: &Actions,
        cfg: &PilotConfig,
        ground: &impl Ground,
        car: Option<&CarAccess>,
        dt_s: f32,
    ) -> Option<PilotEvent> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        let look = a.look_delta_deg;
        let pitch_limit = cfg.pitch_limit_deg.to_radians();
        let free_look = |me: &mut Self| {
            me.yaw_rad = (me.yaw_rad - look.x.to_radians()).rem_euclid(std::f32::consts::TAU);
            me.pitch_rad = (me.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
        };
        let up = Vec3::Y * cfg.eye_height_m;
        match &mut self.mode {
            PilotMode::OnFoot => {
                let c = car?;
                if a.use_pressed && self.waiting.is_none() && self.can_take(c, cfg) {
                    self.waiting = Some(Waiting::ToDrive(c.id));
                    if c.speed_m_s > cfg.car_stopped_m_s {
                        return Some(PilotEvent::CarHailed(c.id));
                    }
                }
                if self.waiting == Some(Waiting::ToDrive(c.id)) {
                    let near = Vec2::new(self.feet.x - c.door.x, self.feet.z - c.door.z).length() <= cfg.car_reach_m * 2.0;
                    if !near || c.wrecked {
                        self.waiting = None;
                    } else if c.speed_m_s <= cfg.car_stopped_m_s {
                        self.waiting = None;
                        self.velocity = Vec2::ZERO;
                        self.mode = PilotMode::EnteringCar(EyePath {
                            points: vec![self.feet + up, c.door + up, c.eye],
                            travelled_m: 0.0,
                        });
                        return Some(PilotEvent::EnteringCar(c.id));
                    }
                }
                None
            }
            PilotMode::EnteringCar(path) => {
                // only the eyes' position is carried: the view stays the player's (CLAUDE.md section 8)
                let rate = path.length_m() / cfg.car_step_s;
                path.travelled_m += rate * dt_s;
                let done = path.done();
                free_look(self);
                let c = car?;
                if c.wrecked {
                    return self.thrown(c, cfg, ground);
                }
                if done {
                    self.seated_eye = c.eye;
                    self.seated_rel_rad = wrap_rad(self.yaw_rad - c.heading_rad);
                    self.mode = PilotMode::Driving;
                    return Some(PilotEvent::InCar(c.id));
                }
                None
            }
            PilotMode::Driving => {
                let c = car?;
                if c.wrecked {
                    return self.thrown(c, cfg, ground);
                }
                self.seated_eye = c.eye;
                // the view turns with the car; the mouse looks round from its nose
                let rel = coupled_look(
                    wrap_rad(self.yaw_rad - c.heading_rad),
                    self.seated_rel_rad,
                    -look.x.to_radians(),
                    cfg.seated_look_limit_deg.to_radians(),
                    a.free_look,
                );
                self.seated_rel_rad = rel;
                self.yaw_rad = (c.heading_rad + rel).rem_euclid(std::f32::consts::TAU);
                self.pitch_rad = (self.pitch_rad + look.y.to_radians()).clamp(-pitch_limit, pitch_limit);
                if a.use_pressed && c.speed_m_s <= cfg.car_stopped_m_s {
                    self.mode = PilotMode::ExitingCar(EyePath {
                        points: vec![c.eye, c.door + up],
                        travelled_m: 0.0,
                    });
                    return Some(PilotEvent::ExitingCar(c.id));
                }
                None
            }
            PilotMode::ExitingCar(path) => {
                let rate = path.length_m() / cfg.car_step_s;
                path.travelled_m += rate * dt_s;
                let done = path.done();
                let end = *path.points.last().unwrap_or(&self.feet);
                free_look(self);
                if done {
                    let feet = end - up;
                    self.feet = Vec3::new(feet.x, ground.support_at(feet.x, feet.z, feet.y + 1.0), feet.z);
                    self.mode = PilotMode::OnFoot;
                    return Some(PilotEvent::OutOfCar(car.map_or(u32::MAX, |c| c.id)));
                }
                None
            }
            _ => None,
        }
    }

    /// Thrown clear of a wrecked car: on foot beside its door, hurt (the pilot has no health on foot
    /// yet, so only the position is decided here).
    fn thrown(&mut self, c: &CarAccess, cfg: &PilotConfig, ground: &impl Ground) -> Option<PilotEvent> {
        let out = Vec3::new(c.door.x - c.centre.x, 0.0, c.door.z - c.centre.z).normalize_or(Vec3::X) * 2.0;
        let at = c.door + out;
        self.feet = Vec3::new(at.x, ground.support_at(at.x, at.z, at.y + 1.0), at.z);
        self.velocity = Vec2::ZERO;
        self.vel_y_m_s = 0.0;
        self.waiting = None;
        self.mode = PilotMode::OnFoot;
        let _ = cfg;
        Some(PilotEvent::ThrownFromCar(c.id))
    }
}

#[cfg(test)]
mod car_tests {
    use super::*;
    use crate::world::FlatGround;

    const DT: f32 = 1.0 / 60.0;

    /// A sedan at the origin facing -z, its driver's door on its left (-x).
    fn sedan(speed_m_s: f32) -> CarAccess {
        CarAccess {
            id: 3,
            door: Vec3::new(-1.35, 0.0, 0.15),
            eye: Vec3::new(-0.37, 1.12, 0.15),
            centre: Vec3::ZERO,
            heading_rad: 0.0,
            speed_m_s,
            wrecked: false,
        }
    }

    /// Standing at the door, facing the car (+x is yaw -90 degrees).
    fn at_door() -> PilotState {
        PilotState::standing(Vec3::new(-1.6, 0.0, 0.15), -std::f32::consts::FRAC_PI_2)
    }

    fn use_key() -> Actions {
        Actions {
            use_pressed: true,
            ..Default::default()
        }
    }

    fn run(p: &mut PilotState, a: &Actions, car: &CarAccess, s: f32) -> Vec<PilotEvent> {
        let cfg = PilotConfig::default();
        let mut out = Vec::new();
        for _ in 0..(s / DT).round() as usize {
            out.extend(p.step_car(a, &cfg, &FlatGround::default(), Some(car), DT));
        }
        out
    }

    #[test]
    fn use_at_a_parked_sedans_door_carries_the_eyes_to_the_seat_and_the_mouse_still_turns_the_view() {
        let cfg = PilotConfig::default();
        let mut p = at_door();
        let car = sedan(0.0);
        let e = p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&car), DT);
        assert_eq!(e, Some(PilotEvent::EnteringCar(3)));
        let yaw = p.yaw_rad;
        let turn = Actions {
            look_delta_deg: Vec2::new(1.0, 0.0),
            ..Default::default()
        };
        let events = run(&mut p, &turn, &car, cfg.car_step_s + 0.1);
        assert_eq!(events, vec![PilotEvent::InCar(3)]);
        assert_eq!(p.mode, PilotMode::Driving);
        assert!(p.eye(&cfg).distance(car.eye) < 1e-4, "the eyes are at EYE_driver");
        assert!(
            (wrap_rad(p.yaw_rad - yaw)).abs() > 0.5,
            "the mouse kept turning the view while getting in"
        );
    }

    #[test]
    fn a_moving_car_is_hailed_and_entered_once_it_stops() {
        let cfg = PilotConfig::default();
        let mut p = at_door();
        let e = p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&sedan(4.0)), DT);
        assert_eq!(e, Some(PilotEvent::CarHailed(3)));
        assert_eq!(p.mode, PilotMode::OnFoot);
        let e = p.step_car(&Actions::default(), &cfg, &FlatGround::default(), Some(&sedan(0.2)), DT);
        assert_eq!(e, Some(PilotEvent::EnteringCar(3)));
    }

    #[test]
    fn use_too_far_from_the_door_or_facing_away_does_nothing() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::new(-4.0, 0.0, 0.15), -std::f32::consts::FRAC_PI_2);
        assert_eq!(p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&sedan(0.0)), DT), None);
        let mut p = PilotState::standing(Vec3::new(-1.6, 0.0, 0.15), std::f32::consts::FRAC_PI_2);
        assert_eq!(p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&sedan(0.0)), DT), None);
        let wreck = CarAccess {
            wrecked: true,
            ..sedan(0.0)
        };
        assert_eq!(at_door().step_car(&use_key(), &cfg, &FlatGround::default(), Some(&wreck), DT), None);
    }

    fn driving() -> PilotState {
        let mut p = at_door();
        run(&mut p, &use_key(), &sedan(0.0), DT);
        run(&mut p, &Actions::default(), &sedan(0.0), 1.0);
        assert_eq!(p.mode, PilotMode::Driving);
        p
    }

    #[test]
    fn driving_the_view_turns_with_the_car_and_the_mouse_looks_round_from_its_nose() {
        let cfg = PilotConfig::default();
        let mut p = driving();
        let rel = p.seated_rel_rad;
        let turned = CarAccess {
            heading_rad: 0.5,
            ..sedan(10.0)
        };
        p.step_car(&Actions::default(), &cfg, &FlatGround::default(), Some(&turned), DT);
        assert!((wrap_rad(p.yaw_rad - 0.5) - rel).abs() < 1e-4, "the view rode the car's turn");
        let look = Actions {
            look_delta_deg: Vec2::new(-10.0, 0.0),
            ..Default::default()
        };
        p.step_car(&look, &cfg, &FlatGround::default(), Some(&turned), DT);
        assert!((p.seated_rel_rad - rel - 10f32.to_radians()).abs() < 1e-4);
    }

    #[test]
    fn use_gets_out_beside_the_door_only_once_the_car_has_stopped() {
        let cfg = PilotConfig::default();
        let mut p = driving();
        assert_eq!(p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&sedan(5.0)), DT), None);
        assert_eq!(p.mode, PilotMode::Driving);
        assert_eq!(
            p.step_car(&use_key(), &cfg, &FlatGround::default(), Some(&sedan(0.0)), DT),
            Some(PilotEvent::ExitingCar(3))
        );
        let events = run(&mut p, &Actions::default(), &sedan(0.0), 1.0);
        assert_eq!(events, vec![PilotEvent::OutOfCar(3)]);
        assert_eq!(p.mode, PilotMode::OnFoot);
        assert!(Vec2::new(p.feet.x + 1.35, p.feet.z - 0.15).length() < 1e-3, "standing at the door");
    }

    #[test]
    fn a_car_wrecked_under_the_pilot_throws_them_clear_on_foot() {
        let cfg = PilotConfig::default();
        let mut p = driving();
        let wreck = CarAccess {
            wrecked: true,
            ..sedan(0.0)
        };
        assert_eq!(
            p.step_car(&Actions::default(), &cfg, &FlatGround::default(), Some(&wreck), DT),
            Some(PilotEvent::ThrownFromCar(3))
        );
        assert_eq!(p.mode, PilotMode::OnFoot);
        assert!(p.feet.x < -2.0, "clear of the car on the door's side");
    }

    #[test]
    fn the_mech_step_leaves_a_pilot_in_a_car_alone() {
        let cfg = PilotConfig::default();
        let mut p = driving();
        let before = p.clone();
        let e = p.step(&use_key(), &cfg, &FlatGround::default(), None, DT);
        assert_eq!(e, None);
        assert_eq!(p, before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fov_cfg() -> PilotConfig {
        PilotConfig {
            view_fov_deg: 95.0,
            on_foot_fov_deg: 75.0,
            fov_change_s: 0.8,
            ..PilotConfig::default()
        }
    }

    #[test]
    fn the_view_is_75_on_foot_and_95_seated() {
        let c = fov_cfg();
        assert_eq!(fov_deg(&c, 95.0, 0.0), 75.0);
        assert_eq!(fov_deg(&c, 95.0, 1.0), 95.0);
    }

    #[test]
    fn a_wider_setting_widens_both_views() {
        let c = fov_cfg();
        assert_eq!(fov_deg(&c, 100.0, 1.0), 100.0);
        assert_eq!(fov_deg(&c, 100.0, 0.0), 80.0);
    }

    #[test]
    fn sitting_widens_the_view_over_the_change_time_with_no_jump() {
        let c = fov_cfg();
        let dt = 1.0 / 60.0;
        let (mut b, mut last) = (0.0, fov_deg(&c, 95.0, 0.0));
        let mut steps = 0;
        while b < 1.0 {
            b = fov_blend(b, true, dt, c.fov_change_s);
            let f = fov_deg(&c, 95.0, b);
            assert!(f >= last && f - last < 1.0, "a smooth widening: {last} to {f}");
            last = f;
            steps += 1;
        }
        assert!((steps as f32 * dt - 0.8).abs() <= dt + 1e-6, "0.8 s: {steps} steps");
        assert_eq!(
            fov_blend(1.0, false, dt, c.fov_change_s),
            1.0 - dt / 0.8,
            "standing narrows it again"
        );
    }
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

    /// Sea 30 m deep (surface at 0) for z < 0, a beach at 0 behind it.
    struct Sea;

    impl Ground for Sea {
        fn height_at(&self, _x: f32, z: f32) -> f32 {
            if z < 0.0 { -30.0 } else { 0.0 }
        }
        fn push_out(&self, p: Vec2, _r: f32) -> Vec2 {
            p
        }
        fn water_at(&self, _x: f32, z: f32) -> Option<crate::world::WaterSample> {
            (z < 0.0).then_some(crate::world::WaterSample {
                surface_m: 0.0,
                depth_m: 30.0,
                flow_m_s: Vec2::ZERO,
            })
        }
    }

    #[test]
    fn a_pilot_in_deep_water_rises_swims_and_climbs_out() {
        let cfg = PilotConfig::default();
        // Under the sea, 20 m out, as when climbing out of a sunk mech.
        let mut p = PilotState::standing(Vec3::new(0.0, -20.0, -20.0), std::f32::consts::PI);
        for _ in 0..(20.0 / DT) as usize {
            p.step(&Actions::default(), &cfg, &Sea, None, DT);
        }
        let eye = p.eye(&cfg).y;
        assert!((eye - SWIM_EYE_ABOVE_M).abs() < 1e-3, "floats with the eyes at the surface: {eye}");
        // Facing the shore (yaw pi faces +z), swimming forward.
        let ahead = Actions {
            move_xy: Vec2::new(0.0, 1.0),
            ..Default::default()
        };
        for _ in 0..(12.0 / DT) as usize {
            p.step(&ahead, &cfg, &Sea, None, DT);
        }
        assert!(p.feet.z > 0.0, "swam ashore: {}", p.feet);
        assert_eq!(p.feet.y, 0.0, "standing on the beach");
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
    fn space_on_foot_jumps_about_a_metre_and_lands() {
        let cfg = PilotConfig::default();
        let mut p = PilotState::standing(Vec3::ZERO, 0.0);
        let jump = Actions {
            lift_axis: 1.0,
            ..Default::default()
        };
        p.step(&jump, &cfg, &FlatGround::default(), None, DT);
        let mut top = 0.0f32;
        for _ in 0..240 {
            p.step(&Actions::default(), &cfg, &FlatGround::default(), None, DT);
            top = top.max(p.feet.y);
        }
        assert!((0.8..1.2).contains(&top), "a jump about a metre high: {top}");
        assert_eq!(p.feet.y, 0.0, "it lands on the ground");
        assert_eq!(p.vel_y_m_s, 0.0, "and stands");
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
    fn a_small_look_turns_the_mech_slowly_and_a_large_one_fast() {
        let limit = 90f32.to_radians();
        let small = look_turn(5f32.to_radians(), limit, false).unwrap();
        let large = look_turn(40f32.to_radians(), limit, false).unwrap();
        assert!(small > 0.0 && small < 0.15, "a small look, a slow turn: {small}");
        assert!(large > 0.8, "a large look, a fast turn: {large}");
        assert_eq!(look_turn(limit, limit, false), Some(1.0));
        assert_eq!(look_turn(-limit, limit, false), Some(-1.0));
        assert_eq!(look_turn(0.0, limit, false), Some(0.0), "looking ahead, no turn");
        assert_eq!(look_turn(limit, limit, true), None, "free look turns nothing");
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
