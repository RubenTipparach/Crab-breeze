//! The simulated flight of an aircraft the pilot has stolen (openspec change `fly-stolen-aircraft`; owner, 2026-10-30:
//! "The player can steal those and planes as well", and in the plan questions "Simulated"). A fixed wing flies on lift
//! and drag from its angle of attack (stalling past its critical angle), its engines' thrust, and the moments of its
//! control surfaces against its stability and damping; a rotorcraft on its rotor's thrust (more in ground effect and in
//! translational lift), its cyclic, its tail rotor against the main rotor's torque. The assists (stall protection,
//! auto-trim, hover hold) write the same controls the pilot would. What the cockpit shows is `Readout`, the numbers the
//! step used (CLAUDE.md section 6.1). Pure, fixed step, no engine; the numbers per kind are `data/aircraft/flight.ron`.
//!
//! Frames: world +Y up; an aircraft's body has its nose toward -Z, its right wing toward +X and its top toward +Y, as
//! every aircraft model is built. Body rates: +X pitches the nose up, +Y yaws it left, +Z rolls the right wing up.

use crate::data::{Validate, positive, within};
use glam::{Quat, Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Gravity, m/s^2.
pub const G_M_S2: f32 = 9.81;

/// Air density at sea level, kg/m^3 (the standard atmosphere).
pub const RHO_SEA_KG_M3: f32 = 1.225;

/// The air's density at `alt_m` above the sea, kg/m^3: the standard troposphere, held at its 11 km value above that.
pub fn density_kg_m3(alt_m: f32) -> f32 {
    let h = alt_m.clamp(0.0, 11_000.0);
    RHO_SEA_KG_M3 * (1.0 - 2.255_77e-5 * h).powf(4.2559)
}

/// A fixed wing's numbers, one kind in `data/aircraft/flight.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlaneSpec {
    /// Its mass, kilograms.
    pub mass_kg: f32,
    /// Its moments of inertia about the body's X (pitch), Y (yaw) and Z (roll) axes, kg m^2.
    pub inertia_kg_m2: [f32; 3],
    /// Wing area, m^2.
    pub wing_area_m2: f32,
    /// Wing span, metres.
    pub span_m: f32,
    /// Mean chord, metres.
    pub chord_m: f32,
    /// Lift coefficient at zero angle of attack.
    pub cl0: f32,
    /// Lift slope, per radian.
    pub cl_alpha: f32,
    /// The critical angle of attack, degrees: lift peaks here.
    pub stall_deg: f32,
    /// The lift coefficient left well past the stall (6 degrees beyond it).
    pub cl_post_stall: f32,
    /// Lift the flaps add, fully down.
    pub cl_flap: f32,
    /// Drag at zero lift.
    pub cd0: f32,
    /// Induced drag factor: `cd0 + k CL^2`.
    pub k_induced: f32,
    /// Drag the gear adds, down.
    pub cd_gear: f32,
    /// Drag the flaps add, fully down.
    pub cd_flap: f32,
    /// Every engine's thrust together at full throttle at sea level, newtons.
    pub thrust_n: f32,
    /// How many engines.
    pub engines: u32,
    /// The speed it is trimmed for at sea level, m/s: the auto-trim and the tests' level flight.
    pub trim_m_s: f32,
    /// Pitch stiffness: the pitching moment per radian of angle of attack (negative: stable).
    pub cm_alpha: f32,
    /// How far full stick moves the angle of attack it trims to, radians.
    pub elevator_alpha_rad: f32,
    /// The roll rate full stick gives at the trim speed, degrees a second.
    pub roll_rate_deg_s: f32,
    /// The sideslip full rudder holds, radians.
    pub rudder_beta_rad: f32,
    /// Weathervane stiffness: the yawing moment per radian of sideslip.
    pub cn_beta: f32,
    /// Dihedral: the rolling moment per radian of sideslip, rolling away from the slip.
    pub cl_beta: f32,
    /// Its wheels' height under its centre at rest, metres (the model's `rest_m`).
    pub rest_m: f32,
    /// How fast the nose wheel turns it on the ground at full rudder, degrees a second.
    pub steer_deg_s: f32,
}

/// Pitch damping (per radian of `q c / 2V`), roll damping and yaw damping: one set for every fixed wing.
const CM_Q: f32 = -12.0;
const CL_P: f32 = -0.5;
const CN_R: f32 = -0.15;

impl PlaneSpec {
    /// Its weight, newtons.
    pub fn weight_n(&self) -> f32 {
        self.mass_kg * G_M_S2
    }

    /// The lift coefficient at angle of attack `alpha` (radians) with flaps `flaps` (0 up, 1 down): the slope up to the
    /// critical angle (2 degrees sooner with the flaps down), falling to `cl_post_stall` over the 6 degrees after it.
    /// Symmetric for a negative angle.
    pub fn cl(&self, alpha: f32, flaps: f32) -> f32 {
        let stall = (self.stall_deg - 2.0 * flaps).to_radians();
        let fade = 6f32.to_radians();
        let a = alpha.abs();
        let s = alpha.signum();
        let peak = self.cl_alpha * stall;
        let lin = if a <= stall {
            self.cl_alpha * a
        } else {
            let t = ((a - stall) / fade).min(1.0);
            peak + (self.cl_post_stall - peak) * t
        };
        self.cl0 + self.cl_flap * flaps + s * lin
    }

    /// The angle of attack it stalls at with flaps `flaps`, radians.
    pub fn stall_rad(&self, flaps: f32) -> f32 {
        (self.stall_deg - 2.0 * flaps).to_radians()
    }

    /// The angle of attack that carries its weight at `speed_m_s` at sea level, flaps up, radians.
    pub fn trim_alpha(&self, speed_m_s: f32) -> f32 {
        let q = 0.5 * RHO_SEA_KG_M3 * speed_m_s * speed_m_s;
        (self.weight_n() / (q * self.wing_area_m2) - self.cl0) / self.cl_alpha
    }

    /// The pitching moment at zero angle and stick: it balances `cm_alpha` at the trim speed's angle.
    fn cm0(&self) -> f32 {
        -self.cm_alpha * self.trim_alpha(self.trim_m_s)
    }
}

impl Validate for PlaneSpec {
    fn validate(&self) -> Result<(), String> {
        positive("mass_kg", self.mass_kg)?;
        for v in self.inertia_kg_m2 {
            positive("inertia_kg_m2", v)?;
        }
        positive("wing_area_m2", self.wing_area_m2)?;
        positive("span_m", self.span_m)?;
        positive("chord_m", self.chord_m)?;
        positive("cl_alpha", self.cl_alpha)?;
        within("stall_deg", self.stall_deg, 5.0, 40.0)?;
        within("cl_post_stall", self.cl_post_stall, 0.0, 3.0)?;
        within("cl_flap", self.cl_flap, 0.0, 2.0)?;
        positive("cd0", self.cd0)?;
        positive("k_induced", self.k_induced)?;
        within("cd_gear", self.cd_gear, 0.0, 0.2)?;
        within("cd_flap", self.cd_flap, 0.0, 0.3)?;
        positive("thrust_n", self.thrust_n)?;
        if self.engines == 0 {
            return Err("`engines` must be at least 1".into());
        }
        positive("trim_m_s", self.trim_m_s)?;
        within("cm_alpha", self.cm_alpha, -5.0, -0.01)?;
        positive("elevator_alpha_rad", self.elevator_alpha_rad)?;
        positive("roll_rate_deg_s", self.roll_rate_deg_s)?;
        positive("rudder_beta_rad", self.rudder_beta_rad)?;
        positive("cn_beta", self.cn_beta)?;
        within("cl_beta", self.cl_beta, 0.0, 1.0)?;
        positive("rest_m", self.rest_m)?;
        positive("steer_deg_s", self.steer_deg_s)?;
        let a = self.trim_alpha(self.trim_m_s);
        if !(a > -0.1 && a < self.stall_rad(0.0) - 2f32.to_radians()) {
            return Err(format!(
                "trim_m_s {} needs {:.1} degrees of angle of attack: not below the stall",
                self.trim_m_s,
                a.to_degrees()
            ));
        }
        Ok(())
    }
}

/// A rotorcraft's numbers, one kind in `data/aircraft/flight.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RotorSpec {
    /// Its mass, kilograms.
    pub mass_kg: f32,
    /// Its moments of inertia about the body's X (pitch), Y (yaw) and Z (roll) axes, kg m^2.
    pub inertia_kg_m2: [f32; 3],
    /// The main rotor's diameter, metres.
    pub rotor_d_m: f32,
    /// The rotor's thrust at full collective out of ground effect at sea level, newtons.
    pub thrust_max_n: f32,
    /// The body's drag area (drag coefficient times frontal area), m^2.
    pub drag_area_m2: f32,
    /// The pitch and roll rate full cyclic commands, degrees a second.
    pub cyclic_rate_deg_s: f32,
    /// The yaw rate full pedal commands, degrees a second.
    pub pedal_rate_deg_s: f32,
    /// The yaw acceleration the main rotor's torque gives at full collective with no tail rotor to answer it, degrees
    /// a second squared.
    pub torque_deg_s2: f32,
    /// Its skids' height under its centre at rest, metres.
    pub rest_m: f32,
}

impl RotorSpec {
    /// Its weight, newtons.
    pub fn weight_n(&self) -> f32 {
        self.mass_kg * G_M_S2
    }

    /// The thrust share the rotor gains near the ground, `h_m` over it (the classic `1 / (1 - (R / 4h)^2)`, at most
    /// 25 % more, nothing past a diameter up).
    pub fn ground_effect(&self, h_m: f32) -> f32 {
        let r = self.rotor_d_m * 0.5;
        let h = h_m.max(r * 0.3);
        if h >= self.rotor_d_m {
            return 1.0;
        }
        (1.0 / (1.0 - (r / (4.0 * h)).powi(2))).min(1.25)
    }

    /// The thrust share translational lift adds at airspeed `v_m_s`: up to 15 % between 8 and 20 m/s.
    pub fn translational(v_m_s: f32) -> f32 {
        let t = ((v_m_s - 8.0) / 12.0).clamp(0.0, 1.0);
        1.0 + 0.15 * t * t * (3.0 - 2.0 * t)
    }
}

impl Validate for RotorSpec {
    fn validate(&self) -> Result<(), String> {
        positive("mass_kg", self.mass_kg)?;
        for v in self.inertia_kg_m2 {
            positive("inertia_kg_m2", v)?;
        }
        positive("rotor_d_m", self.rotor_d_m)?;
        positive("drag_area_m2", self.drag_area_m2)?;
        positive("cyclic_rate_deg_s", self.cyclic_rate_deg_s)?;
        positive("pedal_rate_deg_s", self.pedal_rate_deg_s)?;
        positive("torque_deg_s2", self.torque_deg_s2)?;
        positive("rest_m", self.rest_m)?;
        if self.thrust_max_n <= self.weight_n() * 1.1 {
            return Err(format!(
                "thrust_max_n {} cannot lift {} kg with a margin",
                self.thrust_max_n, self.mass_kg
            ));
        }
        Ok(())
    }
}

/// Every flyable kind's flight, `data/aircraft/flight.ron`: keyed as `air_traffic`'s kinds are.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlightConfig {
    /// The fixed wings.
    pub planes: BTreeMap<String, PlaneSpec>,
    /// The rotorcraft.
    pub rotors: BTreeMap<String, RotorSpec>,
}

impl Validate for FlightConfig {
    fn validate(&self) -> Result<(), String> {
        for (k, p) in &self.planes {
            p.validate().map_err(|e| format!("planes.{k}: {e}"))?;
        }
        for (k, r) in &self.rotors {
            r.validate().map_err(|e| format!("rotors.{k}: {e}"))?;
            if self.planes.contains_key(k) {
                return Err(format!("{k} is both a plane and a rotorcraft"));
            }
        }
        Ok(())
    }
}

/// The pilot's controls, each as a share. A rotorcraft reads `throttle` as its collective, `pitch` and `roll` as its
/// cyclic and `yaw` as its pedals.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    /// Stick back (+) and forward (-), -1 to 1.
    pub pitch: f32,
    /// Stick right (+) and left (-), -1 to 1.
    pub roll: f32,
    /// Right pedal (+) and left (-), -1 to 1.
    pub yaw: f32,
    /// Throttle or collective, 0 to 1.
    pub throttle: f32,
    /// Flaps, 0 up to 1 down.
    pub flaps: f32,
    /// The gear down.
    pub gear_down: bool,
    /// Wheel brakes, 0 to 1.
    pub brake: f32,
}

impl Controls {
    /// Every value brought into its range; a value that is not a number is zero.
    pub fn clamped(self) -> Self {
        let c = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 0.0 };
        Self {
            pitch: c(self.pitch, -1.0, 1.0),
            roll: c(self.roll, -1.0, 1.0),
            yaw: c(self.yaw, -1.0, 1.0),
            throttle: c(self.throttle, 0.0, 1.0),
            flaps: c(self.flaps, 0.0, 1.0),
            gear_down: self.gear_down,
            brake: c(self.brake, 0.0, 1.0),
        }
    }
}

/// The assists, each a setting the pilot can turn off (surveys G2 and G5), on by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assists {
    /// A fixed wing's elevator is held back so the angle of attack stays 2 degrees under the stall.
    pub stall: bool,
    /// With the stick centred a fixed wing holds its pitch attitude.
    pub trim: bool,
    /// With the cyclic centred a rotorcraft holds its place over the ground, and with the collective at half its height.
    pub hover: bool,
}

impl Default for Assists {
    fn default() -> Self {
        Self {
            stall: true,
            trim: true,
            hover: true,
        }
    }
}

/// What damage has left of it (from its broken zones: `Damage::from_broken`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage {
    /// Share of its thrust left.
    pub thrust: f32,
    /// Share of its roll control left.
    pub roll: f32,
    /// Share of its pitch control left.
    pub pitch: f32,
    /// Share of its yaw control left.
    pub yaw: f32,
    /// A rotorcraft's tail rotor turns.
    pub tail_rotor: bool,
    /// Where across the aircraft the thrust it has left pushes, metres right of the centre line: an engine lost on one
    /// side yaws it toward that side.
    pub thrust_arm_m: f32,
}

impl Default for Damage {
    fn default() -> Self {
        Self {
            thrust: 1.0,
            roll: 1.0,
            pitch: 1.0,
            yaw: 1.0,
            tail_rotor: true,
            thrust_arm_m: 0.0,
        }
    }
}

impl Damage {
    /// What is left with the damage zones named in `broken` gone (the zone names of `air_traffic::model`): each
    /// `engine_*` takes its share of `engines`' thrust; an outer wing or aileron half the roll control; a tailplane or
    /// elevator half the pitch; the fin or rudder the yaw; the tail rotor or the tail boom the tail rotor.
    pub fn from_broken(broken: &[&str], engines: u32) -> Self {
        let mut d = Self::default();
        let n = engines.max(1) as f32;
        for z in broken {
            let z = z.to_ascii_lowercase();
            if z.starts_with("engine") || z.starts_with("prop") {
                d.thrust -= 1.0 / n;
            }
            if z.contains("wing_outer") || z.contains("aileron") || z.contains("wing_inner") {
                d.roll -= 0.5;
            }
            if z.contains("tailplane") || z.contains("stabiliser") || z.contains("elevator") {
                d.pitch -= 0.5;
            }
            if z.contains("fin") || z.contains("rudder") {
                d.yaw = 0.0;
            }
            if z.contains("tail_rotor") || z.contains("tail_boom") {
                d.tail_rotor = false;
                d.yaw = 0.0;
            }
        }
        d.thrust = d.thrust.max(0.0);
        d.roll = d.roll.max(0.0);
        d.pitch = d.pitch.max(0.0);
        d
    }

    /// One more of `engines` (set symmetrically about the centre line) lost, the one `x_m` right of it: its share of
    /// thrust goes, and what is left pushes from the other side.
    pub fn engine_lost(mut self, x_m: f32, engines: u32) -> Self {
        let n = engines.max(1) as f32;
        let left = (self.thrust - 1.0 / n).max(0.0);
        if left > 0.0 {
            // the remaining engines' centre: their sum (the lost one's arm undone) over what is left
            self.thrust_arm_m = (self.thrust_arm_m * self.thrust - x_m / n) / left;
        }
        self.thrust = left;
        self
    }
}

/// An aircraft in flight (or on its wheels): where it is and how it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Its centre, world metres.
    pub pos: Vec3,
    /// Its velocity, m/s.
    pub vel: Vec3,
    /// Its attitude: body to world.
    pub rot: Quat,
    /// Its rates about its own axes, radians a second (X pitch up, Y yaw left, Z roll right wing up).
    pub omega: Vec3,
    /// On its wheels or skids.
    pub on_ground: bool,
    /// The auto-trim's held pitch attitude, radians, while the stick is centred.
    pub trim_pitch: Option<f32>,
    /// The hover hold's held height, metres, while the collective rests at half.
    pub hold_y: Option<f32>,
}

impl Body {
    /// At rest on the ground at `pos` (its centre, wheels under it), nose toward `heading_rad` (the game's heading:
    /// 0 faces -Z).
    pub fn resting(pos: Vec3, heading_rad: f32) -> Self {
        Self {
            pos,
            vel: Vec3::ZERO,
            rot: Quat::from_rotation_y(heading_rad),
            omega: Vec3::ZERO,
            on_ground: true,
            trim_pitch: None,
            hold_y: None,
        }
    }

    /// Its nose's direction.
    pub fn forward(&self) -> Vec3 {
        self.rot * Vec3::NEG_Z
    }

    /// Its top's direction.
    pub fn up(&self) -> Vec3 {
        self.rot * Vec3::Y
    }

    /// Its right wing's direction.
    pub fn right(&self) -> Vec3 {
        self.rot * Vec3::X
    }

    /// Its heading (0 faces -Z, positive turns left), pitch (nose up positive) and bank (right wing down positive),
    /// radians.
    pub fn attitude(&self) -> (f32, f32, f32) {
        let f = self.forward();
        let heading = crate::units::heading_of(f);
        let pitch = f.y.clamp(-1.0, 1.0).asin();
        let flat_right = Vec3::Y.cross(-f).normalize_or(self.right());
        let bank = (-self.right().dot(Vec3::Y)).atan2(self.right().dot(flat_right));
        (heading, pitch, bank)
    }
}

/// What the cockpit shows: the numbers the step used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Readout {
    /// Airspeed, m/s.
    pub airspeed_m_s: f32,
    /// Height above the sea, metres.
    pub alt_m: f32,
    /// Height of the wheels or skids over the ground, metres.
    pub agl_m: f32,
    /// Vertical speed, m/s.
    pub climb_m_s: f32,
    /// Angle of attack, radians (a fixed wing).
    pub alpha_rad: f32,
    /// Within 2 degrees of the stall, or past it.
    pub stall_warning: bool,
    /// Heading, pitch and bank, radians (as `Body::attitude`).
    pub heading_rad: f32,
    /// Pitch, radians.
    pub pitch_rad: f32,
    /// Bank, radians.
    pub bank_rad: f32,
    /// Thrust (or rotor thrust), newtons.
    pub thrust_n: f32,
    /// The controls as the step applied them, after the assists.
    pub applied: Controls,
}

/// Brings a vector that is not finite back to zero, so a bad step cannot poison the next.
fn finite(v: Vec3) -> Vec3 {
    if v.is_finite() { v } else { Vec3::ZERO }
}

/// Turns `rot` by body rates `omega` over `dt_s`.
fn spin(rot: Quat, omega: Vec3, dt_s: f32) -> Quat {
    let a = omega * dt_s;
    let len = a.length();
    let d = if len > 1e-9 {
        Quat::from_axis_angle(a / len, len)
    } else {
        Quat::IDENTITY
    };
    (rot * d).normalize()
}

/// One fixed step of a fixed wing under `controls`, over ground `ground`.
pub fn step_plane(
    spec: &PlaneSpec,
    body: &mut Body,
    controls: Controls,
    assists: Assists,
    damage: Damage,
    ground: &dyn Fn(f32, f32) -> f32,
    dt_s: f32,
) -> Readout {
    let mut c = controls.clamped();
    if !(dt_s > 0.0 && dt_s.is_finite()) {
        return Readout::default();
    }
    let rho = density_kg_m3(body.pos.y);
    let v = body.vel.length();
    let inv = body.rot.inverse();
    let vb = inv * body.vel;
    let u = -vb.z;
    let alpha = if v > 1.0 { (-vb.y).atan2(u.max(0.1)) } else { 0.0 };
    let beta = if v > 1.0 { vb.x.atan2(u.max(0.1)) } else { 0.0 };
    let (_, pitch_now, _) = body.attitude();
    // the assists write the stick as a pilot would
    if assists.trim && !body.on_ground {
        if c.pitch.abs() < 0.02 {
            let held = *body.trim_pitch.get_or_insert(pitch_now);
            c.pitch = ((held - pitch_now) * 4.0 - body.omega.x * 1.5).clamp(-1.0, 1.0);
        } else {
            body.trim_pitch = None;
        }
    } else {
        body.trim_pitch = None;
    }
    let stall = spec.stall_rad(c.flaps);
    if assists.stall && v > 1.0 {
        let limit = stall - 2f32.to_radians();
        let room = (limit - alpha) / spec.elevator_alpha_rad;
        c.pitch = c.pitch.min(room.max(-1.0));
    }
    let q = 0.5 * rho * v * v;
    let s = spec.wing_area_m2;
    let cl = spec.cl(alpha, c.flaps);
    let cd = spec.cd0 + spec.k_induced * cl * cl + if c.gear_down { spec.cd_gear } else { 0.0 } + spec.cd_flap * c.flaps;
    let (fwd, up, right) = (body.forward(), body.up(), body.right());
    let mut force = Vec3::new(0.0, -spec.weight_n(), 0.0);
    let thrust = c.throttle * spec.thrust_n * damage.thrust * (rho / RHO_SEA_KG_M3);
    force += fwd * thrust;
    if v > 1.0 {
        let vd = body.vel / v;
        let lift_dir = right.cross(vd).normalize_or(up);
        force += lift_dir * q * s * cl;
        force -= vd * q * s * cd;
        force -= right * q * s * 0.6 * beta;
    }
    // moments about the body's axes
    let vn = v.max(spec.trim_m_s * 0.3);
    let (b, ch) = (spec.span_m, spec.chord_m);
    let cm_e = -spec.cm_alpha * spec.elevator_alpha_rad * damage.pitch;
    let cm = spec.cm0() + spec.cm_alpha * alpha + cm_e * c.pitch + CM_Q * body.omega.x * ch / (2.0 * vn);
    let p_max = spec.roll_rate_deg_s.to_radians();
    let cl_a = -CL_P * p_max * b / (2.0 * spec.trim_m_s) * damage.roll;
    let roll_m = -cl_a * c.roll + spec.cl_beta * beta + CL_P * body.omega.z * b / (2.0 * vn);
    let cn_rud = spec.cn_beta * spec.rudder_beta_rad * damage.yaw;
    let yaw_m = -spec.cn_beta * beta * damage.yaw.max(0.3) - cn_rud * c.yaw + CN_R * body.omega.y * b / (2.0 * vn);
    // thrust off the centre line (an engine lost) turns the nose toward the dead side
    let moment = Vec3::new(
        q * s * ch * cm,
        q * s * b * yaw_m + thrust * damage.thrust_arm_m,
        q * s * b * roll_m,
    );
    let inertia = Vec3::from_array(spec.inertia_kg_m2);
    body.omega = finite(body.omega + moment / inertia * dt_s);
    // the wheels
    let g = ground(body.pos.x, body.pos.z);
    let wheels = body.pos.y - spec.rest_m - g;
    let gear = c.gear_down || !body.on_ground;
    if wheels <= 0.0 && body.vel.y <= 0.5 {
        body.on_ground = true;
        // held on the ground: the wheels carry what the wings do not
        let load = (-force.y).max(0.0);
        force.y += load;
        body.vel.y = body.vel.y.max(0.0);
        body.pos.y = g + spec.rest_m;
        let flat_fwd = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or(Vec3::NEG_Z);
        let side = flat_fwd.cross(Vec3::Y);
        let along = body.vel.dot(flat_fwd);
        let across = body.vel.dot(side);
        // tyres: the sideways slip is taken out, rolling resistance and the brakes slow it along
        let mu = if gear { 0.02 + 0.4 * c.brake } else { 0.5 };
        let slow = (mu * load / spec.mass_kg * dt_s).min(along.abs());
        body.vel -= side * across * (1.0 - (-8.0 * dt_s).exp());
        body.vel -= flat_fwd * slow * along.signum();
        // the nose wheel steers it, slower as it gathers speed; no roll on the ground, no nose below level
        let steer = spec.steer_deg_s.to_radians() * c.yaw * (1.0 - (along.abs() / spec.trim_m_s).min(1.0) * 0.8);
        body.omega.y = -steer;
        body.omega.z = 0.0;
        let (h, p, _) = body.attitude();
        let p = p.max(0.0);
        if p <= 0.0 && body.omega.x < 0.0 {
            body.omega.x = 0.0;
        }
        body.rot = Quat::from_rotation_y(h) * Quat::from_rotation_x(p);
    } else if wheels > 0.5 {
        body.on_ground = false;
    }
    body.vel = finite(body.vel + force / spec.mass_kg * dt_s);
    body.pos += body.vel * dt_s;
    body.rot = spin(body.rot, body.omega, dt_s);
    let floor = ground(body.pos.x, body.pos.z) + spec.rest_m;
    if body.pos.y < floor {
        body.pos.y = floor;
        body.vel.y = body.vel.y.max(0.0);
    }
    let (heading, pitch, bank) = body.attitude();
    Readout {
        airspeed_m_s: v,
        alt_m: body.pos.y,
        agl_m: body.pos.y - spec.rest_m - ground(body.pos.x, body.pos.z),
        climb_m_s: body.vel.y,
        alpha_rad: alpha,
        stall_warning: v > 5.0 && alpha.abs() > stall - 2f32.to_radians(),
        heading_rad: heading,
        pitch_rad: pitch,
        bank_rad: bank,
        thrust_n: thrust,
        applied: c,
    }
}

/// One fixed step of a rotorcraft under `controls` (collective in `throttle`), over ground `ground`.
pub fn step_rotor(
    spec: &RotorSpec,
    body: &mut Body,
    controls: Controls,
    assists: Assists,
    damage: Damage,
    ground: &dyn Fn(f32, f32) -> f32,
    dt_s: f32,
) -> Readout {
    let mut c = controls.clamped();
    if !(dt_s > 0.0 && dt_s.is_finite()) {
        return Readout::default();
    }
    let rho = density_kg_m3(body.pos.y);
    let g = ground(body.pos.x, body.pos.z);
    let agl = body.pos.y - spec.rest_m - g;
    let (heading, pitch_now, bank_now) = body.attitude();
    let ge = spec.ground_effect(agl.max(0.0) + spec.rest_m);
    let flat_v = Vec2::new(body.vel.x, body.vel.z);
    let tl = RotorSpec::translational(flat_v.length());
    let per_collective = spec.thrust_max_n * ge * tl * (rho / RHO_SEA_KG_M3);
    if assists.hover {
        // the cyclic, centred, flies the aircraft back to still over the ground: it leans against its drift
        if c.pitch.abs() < 0.02 && c.roll.abs() < 0.02 {
            let fwd = Vec3::new(-heading.sin(), 0.0, -heading.cos());
            let right = Vec3::new(-fwd.z, 0.0, fwd.x);
            let v_fwd = body.vel.dot(fwd);
            let v_right = body.vel.dot(right);
            let want_pitch = (v_fwd * 0.04).clamp(-0.25, 0.25);
            let want_bank = (-v_right * 0.04).clamp(-0.25, 0.25);
            let rate = spec.cyclic_rate_deg_s.to_radians();
            c.pitch = ((want_pitch - pitch_now) * 2.0 / rate).clamp(-1.0, 1.0);
            c.roll = ((want_bank - bank_now) * 2.0 / rate).clamp(-1.0, 1.0);
        }
        // the collective, at half, holds the height it had
        if (c.throttle - 0.5).abs() < 0.03 && !body.on_ground {
            let held = *body.hold_y.get_or_insert(body.pos.y);
            let want_vy = ((held - body.pos.y) * 0.8).clamp(-4.0, 4.0);
            let tilt = body.up().y.max(0.3);
            let need = (spec.weight_n() + spec.mass_kg * (want_vy - body.vel.y) * 2.0) / tilt;
            c.throttle = (need / per_collective).clamp(0.0, 1.0);
        } else {
            body.hold_y = None;
        }
    }
    let thrust = c.throttle * per_collective;
    let up = body.up();
    let mut force = up * thrust + Vec3::new(0.0, -spec.weight_n(), 0.0);
    let v = body.vel.length();
    force -= body.vel * 0.5 * rho * spec.drag_area_m2 * v;
    // the rates the cyclic and pedals command, reached through the airframe's inertia; the main rotor's torque yaws it
    // when no tail rotor answers
    let cyc = spec.cyclic_rate_deg_s.to_radians();
    let ped = spec.pedal_rate_deg_s.to_radians() * damage.yaw;
    let want = Vec3::new(c.pitch * cyc, -c.yaw * ped, -c.roll * cyc * damage.roll);
    let mut accel = (want - body.omega) * 4.0;
    if !damage.tail_rotor {
        accel.y = spec.torque_deg_s2.to_radians() * c.throttle - body.omega.y * 0.1;
    }
    body.omega = finite(body.omega + accel * dt_s);
    if agl <= 0.0 && body.vel.y <= 0.3 {
        body.on_ground = true;
        let load = (-force.y).max(0.0);
        force.y += load;
        body.pos.y = g + spec.rest_m;
        body.vel.y = body.vel.y.max(0.0);
        // skids: they hold it where it sits and level it
        body.vel.x *= (-6.0 * dt_s).exp();
        body.vel.z *= (-6.0 * dt_s).exp();
        let (h, p, _) = body.attitude();
        body.rot = Quat::from_rotation_y(h) * Quat::from_rotation_x(p * (1.0 - 4.0 * dt_s).max(0.0));
        body.omega.x = body.omega.x.max(0.0).min(cyc);
        body.omega.z = 0.0;
        if load > 0.0 {
            body.omega.y *= (-6.0 * dt_s).exp();
        }
    } else if agl > 0.3 {
        body.on_ground = false;
    }
    body.vel = finite(body.vel + force / spec.mass_kg * dt_s);
    body.pos += body.vel * dt_s;
    body.rot = spin(body.rot, body.omega, dt_s);
    let floor = ground(body.pos.x, body.pos.z) + spec.rest_m;
    if body.pos.y < floor {
        body.pos.y = floor;
        body.vel.y = body.vel.y.max(0.0);
    }
    let (heading, pitch, bank) = body.attitude();
    Readout {
        airspeed_m_s: v,
        alt_m: body.pos.y,
        agl_m: body.pos.y - spec.rest_m - ground(body.pos.x, body.pos.z),
        climb_m_s: body.vel.y,
        alpha_rad: 0.0,
        stall_warning: false,
        heading_rad: heading,
        pitch_rad: pitch,
        bank_rad: bank,
        thrust_n: thrust,
        applied: c,
    }
}

#[cfg(test)]
mod tests;
