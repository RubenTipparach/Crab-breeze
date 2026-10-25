//! The mech's movement state and its fixed step (mech-rig design sections 2 and 4,
//! `boost-and-skate`, `water-skimming`): the throttle lever, one horizontal velocity carried
//! between walking, skating and flying, the leg heading, torso twist and pitch, the rocket and skate
//! budgets, water (skimming, the rocket backup, sinking), and the gait clock. Pure: state, controls,
//! data, ground and `dt_s` in; state out.

use super::config::{GaitConfig, LocomotionConfig, MechConfig, SkateConfig};
use crate::input::{Actions, ModeRequest};
use crate::world::Ground;
use glam::{Vec2, Vec3};

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

/// Height above the ground or a roof within which the mech counts as touching it.
const GROUND_TOUCH_M: f32 = 0.5;

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

/// What a budget does this step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetUse {
    /// In use: it burns.
    Burn,
    /// Resting: it recharges after the delay.
    Rest,
    /// Neither: riding the water, the systems cannot cool (`water-skimming`), so a mech afloat
    /// sinks once both budgets are spent.
    Hold,
}

impl Budget {
    /// One step: burns in use, recharges after the delay at rest, holds otherwise.
    pub fn tick(&mut self, use_: BudgetUse, burn_s: f32, recharge_s: f32, delay_s: f32, relight: f32, dt_s: f32) {
        if use_ == BudgetUse::Hold {
            return;
        }
        if use_ == BudgetUse::Burn && !self.locked {
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
    /// Rocket lift, 0..1 (holding boost fires the rockets; they push up only).
    pub lift_axis: f32,
    /// Skate thrust asked in the air: strafe right (+x) and forward (+y), relative to the view.
    pub move_xy: Vec2,
    /// Where the pilot looks (world yaw): skate thrust in the air is relative to it.
    pub view_yaw_rad: f32,
    /// Skates toggled this step.
    pub skate_toggle: bool,
    /// Ctrl held: dive in the air, sink on water.
    pub sink: bool,
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
            skate_toggle: a.skate_toggle,
            sink: a.sink,
        }
    }
}

/// Where the mech is against water (`water-skimming`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnWater {
    /// Not on deep water.
    #[default]
    Dry,
    /// Riding the surface on the skates.
    Skimming,
    /// Held at the surface by the rockets, the skates empty.
    Hovering,
    /// Under the surface of deep water: sinking, or walking the bottom.
    Sunk,
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
    /// The rockets fired this step.
    pub boosting: bool,
    /// Off the ground, a roof or the water it rides.
    pub in_air: bool,
    /// Against water.
    pub on_water: OnWater,
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

fn approach_vec(v: Vec2, target: Vec2, max_step: f32) -> Vec2 {
    v + (target - v).clamp_length_max(max_step)
}

/// Share of the skates' drive left at speed `v`: 1 from a stand, 0 at top speed.
fn skate_curve(v: f32, v_max: f32) -> f32 {
    (1.0 - (v / v_max).powi(2)).max(0.0)
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

    /// True when the mech is off the ground (falling, or lifting on its rockets).
    pub fn airborne(&self) -> bool {
        self.in_air || self.vel_y_m_s > 1e-3
    }

    /// The skates drive along the heading toward `throttle * speed_max` on a curve that fades
    /// toward top speed; across the heading the velocity decays at the grip, so turns drift.
    fn skate_drive(&mut self, fwd: Vec2, sk: &SkateConfig, dt_s: f32) {
        let target = self.throttle * sk.speed_max_m_s;
        let along = self.vel.dot(fwd);
        let side = self.vel - fwd * along;
        let push = sk.accel_m_s2 * skate_curve(along, sk.speed_max_m_s) * dt_s;
        let along = if target > along && along >= 0.0 {
            (along + push).min(target)
        } else if target < along && along <= 0.0 {
            (along - push).max(target)
        } else {
            approach(along, target, sk.brake_m_s2 * dt_s)
        };
        self.vel = fwd * along + side * (-sk.grip_per_s * dt_s).exp();
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(&mut self, c: &MechControls, cfg: &MechConfig, ground: &impl Ground, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let (loco, gait, jet, sk) = (&cfg.locomotion, &cfg.gait, &cfg.jet, &cfg.skate);
        let legs_left = 2 - self.legs_lost.iter().filter(|l| **l).count();
        let here = Vec2::new(self.pos.x, self.pos.z);
        let water = ground.water_at(here.x, here.y);
        let deep = water.is_some_and(|w| w.depth_m > loco.max_wade_depth_m);
        // Deep enough to skim: over half the mech (data). Shallower is waded, skates or not.
        let skimmable = water.is_some_and(|w| w.depth_m > loco.skim_depth_share * loco.body_height_m);
        let at_surface = water.is_some_and(|w| self.pos.y >= w.surface_m - loco.float_catch_m);
        // Wading with the skates on into water deep enough: the wheels bring the mech up to ride it.
        let rising = water.is_some_and(|w| self.pos.y < w.surface_m - loco.float_catch_m) && self.skates_on && skimmable && !self.in_air;

        // Skates on and off (Shift, a toggle anywhere: survey M5); they need both legs and power.
        let skate_asked = c.skate_toggle || c.mode_request == Some(ModeRequest::Skate);
        if skate_asked {
            self.skates_on = !self.skates_on;
        }
        if c.mode_request == Some(ModeRequest::Walk) {
            self.skates_on = false;
        }
        let skate_power = !self.skate.locked && legs_left == 2;
        // Water (water-skimming): with skate power the wheels ride the surface of water deeper than
        // half the mech, switching on by themselves at the surface; water shallower than that is
        // waded, skates or not (owner, 2026-10-24: "shallow water dont need autoskate", "The skis
        // only kicking if you're above water deeper than 50% of the mech"); with the skates empty
        // the rockets hold the mech up on deep water; Ctrl, or both empty, and it sinks.
        let skim = (at_surface || rising) && !c.sink && skate_power && skimmable;
        if skim {
            self.skates_on = true;
        }
        if !skate_power {
            self.skates_on = false;
        }
        let hover = at_surface && deep && !c.sink && !skim && !self.boost.locked;
        let afloat = skim || hover;

        // Rockets: vertical only, while boost is held (or holding the mech on water).
        let lifting = c.lift_axis > 0.0 && !self.boost.locked;
        self.boosting = lifting || hover;
        self.mode = if self.boosting {
            LocomotionMode::Jet
        } else if self.skates_on {
            LocomotionMode::Skate
        } else {
            LocomotionMode::Walk
        };
        let floor_here = {
            let f = ground.support_at(here.x, here.y, self.pos.y);
            match water {
                Some(w) if afloat => f.max(w.surface_m),
                _ => f,
            }
        };
        let grounded = self.pos.y - floor_here < GROUND_TOUCH_M;
        let sunk = deep && !afloat && water.is_some_and(|w| self.pos.y < w.surface_m - loco.float_catch_m);

        // Throttle lever.
        if c.throttle_zero {
            self.throttle = 0.0;
        }
        self.throttle = (self.throttle + c.throttle_axis * loco.throttle_rate_per_s * dt_s).clamp(loco.throttle_min, 1.0);

        // Legs (or wheels) turn the heading.
        let turn_rate = match self.skates_on {
            true => sk.turn_rate_deg_s,
            false if legs_left == 1 => loco.turn_rate_deg_s * 0.5,
            false => loco.turn_rate_deg_s,
        };
        // The turn rate eases toward what is asked at the legs' turn acceleration.
        let want = c.turn_axis * turn_rate.to_radians();
        let ease = loco.turn_accel_deg_s2.to_radians() * dt_s;
        self.turn_rate_rad_s += (want - self.turn_rate_rad_s).clamp(-ease, ease);
        self.heading_rad = (self.heading_rad + self.turn_rate_rad_s * dt_s).rem_euclid(std::f32::consts::TAU);
        let fwd = flat_forward(self.heading_rad);

        // Horizontal velocity. Only the skates push for free movement's price; the rockets never
        // push sideways.
        let ask = c.move_xy.clamp_length_max(1.0);
        let air_skating = !grounded && !sunk && self.skates_on && ask != Vec2::ZERO;
        if sunk && !grounded {
            // Sinking: the water drags the mech to a stop.
            self.vel = approach_vec(self.vel, Vec2::ZERO, loco.water_drag_m_s2 * dt_s);
        } else if air_skating {
            let view = flat_forward(c.view_yaw_rad);
            let right = Vec2::new(-view.y, view.x);
            let dir = (view * ask.y + right * ask.x).normalize_or_zero();
            let push = sk.accel_m_s2 * skate_curve(self.vel.dot(dir).max(0.0), sk.speed_max_m_s);
            self.vel += dir * push * ask.length() * dt_s;
        } else if !grounded {
            // Coasting in the air.
            self.vel *= (1.0 - jet.drag_per_s * dt_s).max(0.0);
        } else if self.skates_on {
            self.skate_drive(fwd, sk, dt_s);
        } else {
            // Walking, on the ground, the sea floor, or paddling on the rockets at the surface.
            // Wading drags the legs back (terrain-water); a lost leg leaves a limp (mech-damage);
            // no legs, no walking.
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

        let moved = here + self.vel * dt_s;
        let pushed = ground.push_out_at(moved, loco.body_radius_m, self.pos.y);
        if pushed.distance(moved) > 1e-4 {
            // Into a wall: what was lost into it is gone.
            let real = (pushed - here) / dt_s;
            self.vel = real.clamp_length_max(self.vel.length());
        }

        // Budgets: the rockets burn while firing, the skates while carrying the mech or pushing it
        // in the air; riding the water, neither recharges.
        let budget_use = |burning: bool| match (burning, afloat) {
            (true, _) => BudgetUse::Burn,
            (false, true) => BudgetUse::Hold,
            (false, false) => BudgetUse::Rest,
        };
        self.boost.tick(
            budget_use(self.boosting),
            jet.burn_s,
            jet.recharge_s,
            jet.recharge_delay_s,
            jet.relight_fraction,
            dt_s,
        );
        let skating = self.skates_on && ((grounded && !sunk) || air_skating);
        self.skate.tick(
            budget_use(skating),
            sk.burn_s,
            sk.recharge_s,
            sk.recharge_delay_s,
            sk.relight_fraction,
            dt_s,
        );
        if self.skate.locked {
            self.skates_on = false;
        }

        // Vertical.
        let lift = if lifting { jet.thrust_m_s2 * c.lift_axis } else { 0.0 };
        if sunk {
            // In the water: the rockets lift slowly; without them the mech settles at sinking speed.
            if lifting {
                self.vel_y_m_s = (self.vel_y_m_s + (lift - G_M_S2) * dt_s).min(loco.water_climb_m_s);
            } else {
                self.vel_y_m_s = approach(self.vel_y_m_s, -loco.water_sink_m_s, loco.water_drag_m_s2 * dt_s);
            }
        } else {
            let diving = c.sink && !grounded;
            let dive = if diving { jet.dive_m_s2 } else { 0.0 };
            self.vel_y_m_s += (lift - G_M_S2 - dive) * dt_s;
            let fall = if lifting {
                jet.sink_max_m_s
            } else if diving {
                jet.dive_max_m_s
            } else {
                jet.fall_max_m_s
            };
            self.vel_y_m_s = self.vel_y_m_s.clamp(-fall, jet.climb_max_m_s);
        }
        let mut y = self.pos.y + self.vel_y_m_s * dt_s;
        let water_there = ground.water_at(pushed.x, pushed.y);
        let mut support = ground.support_at(pushed.x, pushed.y, self.pos.y);
        if let Some(w) = water_there.filter(|_| afloat) {
            support = support.max(w.surface_m);
        }
        let on_floor = y <= support || (!lifting && self.vel_y_m_s <= 0.0 && self.pos.y - support < 1.0);
        if skim && y < support - loco.skim_rise_m_s * dt_s {
            // Rising from the bed to the surface it will ride, at the skates' rate (no snap).
            y = self.pos.y + loco.skim_rise_m_s * dt_s;
            self.vel_y_m_s = 0.0;
        } else if on_floor {
            // On the ground, a roof, the sea floor or the surface it rides.
            y = support;
            self.vel_y_m_s = if lifting { self.vel_y_m_s.max(0.0) } else { 0.0 };
        }
        self.pos = Vec3::new(pushed.x, y, pushed.y);
        self.in_air = !on_floor;
        self.wading_depth_m = water_there.map_or(0.0, |w| (w.surface_m - y).max(0.0));
        self.on_water = match water_there {
            Some(w) if w.depth_m > loco.max_wade_depth_m => {
                if y < w.surface_m - loco.float_catch_m {
                    OnWater::Sunk
                } else if hover && !skim {
                    OnWater::Hovering
                } else if skim {
                    OnWater::Skimming
                } else {
                    OnWater::Dry
                }
            }
            Some(_) if skim => OnWater::Skimming,
            _ => OnWater::Dry,
        };
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
    fn skates_do_not_lift_the_mech_onto_shallow_water() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        m.skates_on = true;
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        run(&mut m, go, 5.0, &Pool { depth_m: 0.8 });
        assert_ne!(m.on_water, OnWater::Skimming, "wades in 0.8 m of water");
        assert!(m.wading_depth_m > 0.7, "feet on the bed: {}", m.wading_depth_m);
    }

    #[test]
    fn skates_wade_in_water_up_to_half_the_mech_and_skim_deeper() {
        let cfg = MechConfig::default();
        let half = cfg.locomotion.skim_depth_share * cfg.locomotion.body_height_m;
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        let mut shallow = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        shallow.skates_on = true;
        run(&mut shallow, go, 8.0, &Pool { depth_m: half * 0.8 });
        assert_ne!(shallow.on_water, OnWater::Skimming, "wades in water under half the mech");
        let mut deep = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        deep.skates_on = true;
        run(&mut deep, go, 8.0, &Pool { depth_m: half * 1.2 });
        assert_eq!(deep.on_water, OnWater::Skimming, "skims water deeper than half the mech");
        assert!((deep.pos.y - half * 1.2).abs() < 0.5, "riding the surface: {}", deep.pos.y);
    }

    #[test]
    fn rising_to_skim_takes_time_not_a_snap() {
        let cfg = MechConfig::default();
        let depth = cfg.locomotion.body_height_m * 0.8;
        let mut m = MechState::standing(Vec3::new(0.0, 0.0, -100.0), 0.0, &GaitConfig::default());
        m.skates_on = true;
        let ground = Pool { depth_m: depth };
        let dt = 1.0 / 60.0;
        m.step(&MechControls::default(), &cfg, &ground, dt);
        assert!(
            m.pos.y <= cfg.locomotion.skim_rise_m_s * dt + 1e-3,
            "one step rises one step's worth: {}",
            m.pos.y
        );
        run(&mut m, MechControls::default(), depth / cfg.locomotion.skim_rise_m_s + 0.5, &ground);
        assert!((m.pos.y - depth).abs() < 0.2, "and reaches the surface: {}", m.pos.y);
    }

    #[test]
    fn rockets_burn_for_twenty_seconds_then_lock_and_recharge() {
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

    /// A shore at z = -20: dry ground at 0 behind it, sea 30 m deep (surface at 0) beyond.
    struct Sea;

    impl Ground for Sea {
        fn height_at(&self, _x: f32, z: f32) -> f32 {
            if z < -20.0 { -30.0 } else { 0.0 }
        }
        fn push_out(&self, p: Vec2, _r: f32) -> Vec2 {
            p
        }
        fn water_at(&self, _x: f32, z: f32) -> Option<crate::world::WaterSample> {
            (z < -20.0).then_some(crate::world::WaterSample {
                surface_m: 0.0,
                depth_m: 30.0,
                flow_m_s: Vec2::ZERO,
            })
        }
    }

    fn skimming_at_sea() -> MechState {
        let mut m = MechState::standing(Vec3::new(0.0, 0.0, -200.0), 0.0, &GaitConfig::default());
        run(&mut m, MechControls::default(), 0.5, &Sea);
        assert_eq!(m.on_water, OnWater::Skimming, "on the sea with skate power it skims");
        m
    }

    #[test]
    fn walking_into_deep_water_sinks_to_the_bottom_and_walks_on() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        m.throttle = 1.0;
        run(&mut m, MechControls::default(), 30.0, &Pool { depth_m: 12.0 });
        assert!(m.pos.z < -60.0, "walked on along the bottom: {}", m.pos);
        assert_eq!(m.on_water, OnWater::Sunk);
    }

    #[test]
    fn rockets_never_push_sideways() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let c = MechControls {
            lift_axis: 1.0,
            move_xy: Vec2::new(1.0, 0.0),
            ..Default::default()
        };
        run(&mut m, c, 3.0, &FlatGround::default());
        assert!(m.pos.y > 50.0, "climbed: {}", m.pos.y);
        assert!(m.pos.x.abs() < 1.0, "no sideways push: {}", m.pos);
        assert_eq!(m.skate.heat, 0.0);
    }

    #[test]
    fn skating_builds_speed_over_time() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        m.throttle = 1.0;
        let on = MechControls {
            skate_toggle: true,
            ..Default::default()
        };
        run(&mut m, on, DT, &FlatGround::default());
        run(&mut m, MechControls::default(), 5.0, &FlatGround::default());
        let v5 = m.vel.length();
        assert!((40.0..70.0).contains(&v5), "5 s: {v5} m/s");
        run(&mut m, MechControls::default(), 10.0, &FlatGround::default());
        let v15 = m.vel.length();
        assert!(v15 > 100.0 && v15 <= 120.0, "15 s: {v15} m/s");
    }

    #[test]
    fn skates_strafe_in_the_air_on_their_own_budget() {
        let mut m = MechState::standing(Vec3::new(0.0, 500.0, 0.0), 0.0, &GaitConfig::default());
        m.skates_on = true;
        let strafe = MechControls {
            move_xy: Vec2::new(1.0, 0.0),
            ..Default::default()
        };
        run(&mut m, strafe, 3.0, &FlatGround::default());
        assert!(m.pos.x > 30.0, "pushed right: {}", m.pos);
        assert!(m.skate.heat > 0.0, "the skates paid");
        assert_eq!(m.boost.heat, 0.0, "the rockets did not");
    }

    #[test]
    fn ctrl_dives_faster_than_a_fall() {
        let start = MechState::standing(Vec3::new(0.0, 300.0, 0.0), 0.0, &GaitConfig::default());
        let (mut fall, mut dive) = (start, start);
        run(&mut fall, MechControls::default(), 3.0, &FlatGround::default());
        let ctrl = MechControls {
            sink: true,
            ..Default::default()
        };
        run(&mut dive, ctrl, 3.0, &FlatGround::default());
        assert!(dive.pos.y < fall.pos.y - 50.0, "dive {} against fall {}", dive.pos.y, fall.pos.y);
    }

    #[test]
    fn a_skating_mech_skims_onto_deep_water() {
        let mut m = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        m.throttle = 1.0;
        m.skates_on = true;
        m.vel = Vec2::new(0.0, -60.0);
        run(&mut m, MechControls::default(), 6.0, &Sea);
        assert!(m.pos.z < -300.0, "out to sea: {}", m.pos);
        assert_eq!(m.pos.y, 0.0, "on the surface");
        assert_eq!(m.on_water, OnWater::Skimming);
        assert!(m.vel.length() > 60.0, "kept its speed: {}", m.vel.length());
    }

    #[test]
    fn rockets_hold_the_mech_up_when_the_skates_run_dry() {
        let mut m = skimming_at_sea();
        m.skate.heat = 0.999;
        run(&mut m, MechControls::default(), 2.0, &Sea);
        assert!(m.skate.locked);
        assert_eq!(m.on_water, OnWater::Hovering);
        assert_eq!(m.pos.y, 0.0, "held at the surface");
        assert!(m.boost.heat > 0.05, "on the rockets' budget");
        run(&mut m, MechControls::default(), 25.0, &Sea);
        assert_eq!(m.on_water, OnWater::Sunk, "both empty: it sinks");
        assert!(m.pos.y < -25.0, "on its way down: {}", m.pos.y);
    }

    #[test]
    fn ctrl_sinks_a_skimming_mech() {
        let mut m = skimming_at_sea();
        let ctrl = MechControls {
            sink: true,
            ..Default::default()
        };
        run(&mut m, ctrl, 15.0, &Sea);
        assert_eq!(m.pos.y, -30.0, "on the bottom");
        assert_eq!(m.on_water, OnWater::Sunk);
    }

    #[test]
    fn a_sunk_mech_recharges_and_rises() {
        let mut m = MechState::standing(Vec3::new(0.0, -30.0, -200.0), 0.0, &GaitConfig::default());
        m.boost = Budget {
            heat: 1.0,
            locked: true,
            idle_s: 0.0,
        };
        m.skate = m.boost;
        run(&mut m, MechControls::default(), 15.0, &Sea);
        assert_eq!(m.pos.y, -30.0, "waited on the bottom");
        assert!(!m.boost.locked && !m.skate.locked, "recharged");
        let up = MechControls {
            lift_axis: 1.0,
            ..Default::default()
        };
        let cfg = MechConfig::default();
        let mut t = 0.0;
        while m.pos.y < 0.0 && t < 10.0 {
            m.step(&up, &cfg, &Sea, DT);
            t += DT;
        }
        assert!(t < 5.0, "rose 30 m in {t} s");
        run(&mut m, MechControls::default(), 5.0, &Sea);
        assert_eq!(m.pos.y, 0.0, "back up, riding the surface");
        assert_eq!(m.on_water, OnWater::Skimming);
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
