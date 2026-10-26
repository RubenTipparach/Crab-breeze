//! A car you drive (openspec change `car-driving`, design section 2): arcade handling, grippy and
//! simple (survey J8). Speed eases to the throttle's target, the car turns about its rear axle, the
//! handbrake lets the rear slide, each wheel rides a spring over the ground (kerbs bump, craters drop
//! a wheel), and walls, cars and a mech's legs stop it. Pure and fixed step: a body, the driver's
//! input, the ground and what is in the way in; the body after the step out.
//!
//! The car's own data, `data/cars/<kind>.ron` (`CarSpec`), is the one description of a kind: traffic
//! (`crate::traffic`) reads its size, armour and share of the traffic from it, and this module its
//! handling, so a traffic car you take drives as the same car.

use crate::data::{Validate, finite3, positive, within};
use crate::world::Ground;
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// One kind of car, loaded from `data/cars/<kind>.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CarSpec {
    /// What the player calls it.
    pub name: String,
    /// Its model under `assets/`.
    pub model: String,
    /// Body size: width, height and length, metres.
    pub size_m: [f32; 3],
    /// Mass, kilograms.
    pub mass_kg: f32,
    /// Armour, millimetres of rolled steel equivalent: a hit that beats it sets the car alight.
    pub armour_mm: f32,
    /// Its share of city traffic (relative weight; 0 never in town).
    pub city_weight: f32,
    /// Its share of highway traffic (relative weight; 0 never on a highway).
    pub highway_weight: f32,
    /// Top speed forward, m/s; reverse is `reverse_fraction` of it.
    pub top_speed_m_s: f32,
    /// Reverse top speed as a share of forward.
    pub reverse_fraction: f32,
    /// How fast it gains speed, m/s2.
    pub accel_m_s2: f32,
    /// How hard it brakes, m/s2.
    pub brake_m_s2: f32,
    /// How fast it slows with no pedal, m/s2.
    pub coast_m_s2: f32,
    /// Front axle to rear axle, metres.
    pub wheelbase_m: f32,
    /// Left wheel to right wheel, metres.
    pub track_m: f32,
    /// The wheels' radius, metres (the model's wheels are this size, and turn at speed over it).
    pub wheel_radius_m: f32,
    /// Steering lock at a standstill, degrees.
    pub max_steer_deg: f32,
    /// Steering lock at top speed, as a share of `max_steer_deg` (so it never spins out on its own).
    pub top_speed_steer_fraction: f32,
    /// How fast the front wheels follow the input, radians a second.
    pub steer_rate_rad_s: f32,
    /// How fast full grip takes out a sideways slide, per second (scaled by the rear grip).
    pub lateral_grip_per_s: f32,
    /// The rear's grip while the handbrake is on, 0..1 (1 is full grip).
    pub handbrake_grip: f32,
    /// How fast the rear's grip comes back after the handbrake, per second.
    pub grip_recover_per_s: f32,
    /// How much a loose rear adds to the turn (yaw rate scale at zero grip, minus one).
    pub handbrake_yaw_gain: f32,
    /// How hard the handbrake slows the car, m/s2.
    pub handbrake_brake_m_s2: f32,
    /// The body's height over the ground at each wheel at rest, metres.
    pub ride_height_m: f32,
    /// Each wheel's spring, newtons a metre.
    pub spring_n_m: f32,
    /// Each wheel's damper, newton seconds a metre.
    pub damper_n_s_m: f32,
    /// A hit into something faster than this hurts the car, m/s.
    pub crash_safe_m_s: f32,
    /// Health lost per m/s of impact over `crash_safe_m_s` (health runs 1 to 0).
    pub crash_damage_per_m_s: f32,
    /// `EYE_driver`, the driver's eyes in the car's frame (x right, y up from the ground, z back;
    /// the car faces -z), metres.
    pub eye_driver_m: [f32; 3],
    /// `DOOR_driver`, where a person stands to open the driver's door, same frame, metres.
    pub door_driver_m: [f32; 3],
    /// The speedometer's full scale, km/h.
    pub speedo_full_scale_km_h: f32,
    /// The speedometer needle's sweep from zero to full scale, degrees.
    pub speedo_sweep_deg: f32,
    /// Paint colours (linear RGB, 0..1); each car is one of them.
    pub paints: Vec<[f32; 3]>,
}

impl Default for CarSpec {
    fn default() -> Self {
        Self {
            name: "Sedan".into(),
            model: "models/cars/sedan.glb".into(),
            size_m: [1.8, 1.4, 4.7],
            mass_kg: 1400.0,
            armour_mm: 5.0,
            city_weight: 1.0,
            highway_weight: 1.0,
            top_speed_m_s: 50.0,
            reverse_fraction: 1.0 / 3.0,
            accel_m_s2: 4.0,
            brake_m_s2: 9.0,
            coast_m_s2: 0.8,
            wheelbase_m: 2.7,
            track_m: 1.5,
            wheel_radius_m: 0.33,
            max_steer_deg: 35.0,
            top_speed_steer_fraction: 0.2,
            steer_rate_rad_s: 3.0,
            lateral_grip_per_s: 60.0,
            handbrake_grip: 0.3,
            grip_recover_per_s: 2.0,
            handbrake_yaw_gain: 0.8,
            handbrake_brake_m_s2: 3.0,
            ride_height_m: 0.3,
            spring_n_m: 35_000.0,
            damper_n_s_m: 3_500.0,
            crash_safe_m_s: 3.0,
            crash_damage_per_m_s: 0.04,
            eye_driver_m: [-0.38, 1.15, 0.1],
            door_driver_m: [-1.4, 0.0, 0.1],
            speedo_full_scale_km_h: 200.0,
            speedo_sweep_deg: 240.0,
            paints: vec![[0.5, 0.1, 0.08]],
        }
    }
}

impl Validate for CarSpec {
    fn validate(&self) -> Result<(), String> {
        for (i, s) in self.size_m.iter().enumerate() {
            positive(&format!("size_m[{i}]"), *s)?;
        }
        positive("mass_kg", self.mass_kg)?;
        within("armour_mm", self.armour_mm, 0.0, 1000.0)?;
        within("city_weight", self.city_weight, 0.0, 1000.0)?;
        within("highway_weight", self.highway_weight, 0.0, 1000.0)?;
        positive("top_speed_m_s", self.top_speed_m_s)?;
        within("reverse_fraction", self.reverse_fraction, 0.0, 1.0)?;
        positive("accel_m_s2", self.accel_m_s2)?;
        positive("brake_m_s2", self.brake_m_s2)?;
        within("coast_m_s2", self.coast_m_s2, 0.0, 20.0)?;
        positive("wheelbase_m", self.wheelbase_m)?;
        positive("track_m", self.track_m)?;
        within("wheel_radius_m", self.wheel_radius_m, 0.1, 1.5)?;
        within("max_steer_deg", self.max_steer_deg, 1.0, 60.0)?;
        within("top_speed_steer_fraction", self.top_speed_steer_fraction, 0.0, 1.0)?;
        positive("steer_rate_rad_s", self.steer_rate_rad_s)?;
        positive("lateral_grip_per_s", self.lateral_grip_per_s)?;
        within("handbrake_grip", self.handbrake_grip, 0.0, 1.0)?;
        positive("grip_recover_per_s", self.grip_recover_per_s)?;
        within("handbrake_yaw_gain", self.handbrake_yaw_gain, 0.0, 5.0)?;
        within("handbrake_brake_m_s2", self.handbrake_brake_m_s2, 0.0, 20.0)?;
        positive("ride_height_m", self.ride_height_m)?;
        positive("spring_n_m", self.spring_n_m)?;
        positive("damper_n_s_m", self.damper_n_s_m)?;
        within("crash_safe_m_s", self.crash_safe_m_s, 0.0, 50.0)?;
        within("crash_damage_per_m_s", self.crash_damage_per_m_s, 0.0, 1.0)?;
        finite3("eye_driver_m", self.eye_driver_m)?;
        finite3("door_driver_m", self.door_driver_m)?;
        positive("speedo_full_scale_km_h", self.speedo_full_scale_km_h)?;
        within("speedo_sweep_deg", self.speedo_sweep_deg, 1.0, 360.0)?;
        if self.paints.is_empty() {
            return Err("`paints` needs at least one colour".into());
        }
        for p in &self.paints {
            for &c in p {
                within("paints", c, 0.0, 1.0)?;
            }
        }
        if self.wheelbase_m >= self.size_m[2] || self.track_m >= self.size_m[0] + 0.3 {
            return Err("the wheels must fit under the body (`wheelbase_m` < length, `track_m` about the width)".into());
        }
        Ok(())
    }
}

impl CarSpec {
    /// A point in the car's frame (x right, y up, z back), placed in the world for a car at `pos`
    /// heading `heading_rad`.
    pub fn to_world(pos: Vec3, heading_rad: f32, local: [f32; 3]) -> Vec3 {
        let (s, c) = heading_rad.sin_cos();
        // heading 0 faces -z; a positive heading turns left (counter-clockwise from above)
        let x = local[0] * c + local[2] * s;
        let z = -local[0] * s + local[2] * c;
        pos + Vec3::new(x, local[1], z)
    }

    /// The speedometer needle's angle from its zero for a speed, radians: what the dashboard turns
    /// the needle to, from the speed the simulation uses (CLAUDE.md section 6.1).
    pub fn speedo_needle_rad(&self, speed_m_s: f32) -> f32 {
        let km_h = speed_m_s.abs() * 3.6;
        (km_h / self.speedo_full_scale_km_h).clamp(0.0, 1.0) * self.speedo_sweep_deg.to_radians()
    }

    /// What a needle at `angle_rad` reads, km/h: the dial's printed scale.
    pub fn speedo_reading_km_h(&self, angle_rad: f32) -> f32 {
        angle_rad / self.speedo_sweep_deg.to_radians() * self.speedo_full_scale_km_h
    }
}

/// The car's forward direction on the ground (x, z) for a heading (0 faces -z, positive turns left).
pub fn forward2(heading_rad: f32) -> Vec2 {
    Vec2::new(-heading_rad.sin(), -heading_rad.cos())
}

/// The car's right on the ground (x, z).
pub fn right2(heading_rad: f32) -> Vec2 {
    crate::world::lanes::right_of(forward2(heading_rad))
}

/// What the driver asks for this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drive {
    /// Pedals: +1 full throttle, -1 brake (and reverse once stopped).
    pub throttle: f32,
    /// Steering: +1 full right, -1 full left.
    pub steer: f32,
    /// The handbrake is pulled.
    pub handbrake: bool,
}

/// Something in the car's way this step, as a circle on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obstacle {
    /// What it is, for the caller (a car id, a mech id).
    pub id: u32,
    /// Its middle (x, z).
    pub at: Vec2,
    /// Its radius, metres.
    pub radius_m: f32,
    /// Its velocity (x, z), m/s.
    pub vel: Vec2,
    /// Its mass, kilograms; infinite for something the car cannot move (a mech's leg).
    pub mass_kg: f32,
}

/// One wheel's corner of the body: its height and how fast it moves up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corner {
    /// The body's height over this wheel, world metres.
    pub y_m: f32,
    /// Its upward speed, m/s.
    pub vel_y_m_s: f32,
}

/// A driven car's state.
#[derive(Clone, Debug, PartialEq)]
pub struct CarBody {
    /// The ground point under the car's middle (x, z), and the body's base height (y).
    pub pos: Vec3,
    /// Heading, radians (0 faces -z, positive turns left).
    pub heading_rad: f32,
    /// Velocity on the ground (x, z), m/s.
    pub vel: Vec2,
    /// The front wheels' angle, radians (right positive).
    pub steer_rad: f32,
    /// The rear's grip, 0..1.
    pub rear_grip: f32,
    /// The body over each wheel: front left, front right, rear left, rear right.
    pub corners: [Corner; 4],
    /// Nose up, radians.
    pub pitch_rad: f32,
    /// Left side up, radians.
    pub roll_rad: f32,
    /// Health, 1 whole to 0 wrecked.
    pub health: f32,
}

/// What happened to the car this step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CarStep {
    /// The hardest impact this step, m/s along the contact (0 for none).
    pub impact_m_s: f32,
    /// Velocity changes for the obstacles the car pushed (their id, the change), m/s.
    pub pushed: Vec<(u32, Vec2)>,
}

/// Gravity, m/s2.
const G_M_S2: f32 = 9.81;
/// A wheel's corner never goes below this over the ground (the body on its bump stops), metres.
const BUMP_STOP_M: f32 = 0.12;
/// Bounce between a car and another car.
const RESTITUTION: f32 = 0.2;

impl CarBody {
    /// A car at rest on the ground at `pos` (x, z; y is read from the ground), facing `heading_rad`.
    pub fn parked(spec: &CarSpec, ground: &impl Ground, pos: Vec2, heading_rad: f32) -> Self {
        let mut b = Self {
            pos: Vec3::new(pos.x, ground.height_at(pos.x, pos.y), pos.y),
            heading_rad,
            vel: Vec2::ZERO,
            steer_rad: 0.0,
            rear_grip: 1.0,
            corners: [Corner::default(); 4],
            pitch_rad: 0.0,
            roll_rad: 0.0,
            health: 1.0,
        };
        for (i, w) in b.wheels(spec).into_iter().enumerate() {
            b.corners[i].y_m = ground.height_at(w.x, w.y) + spec.ride_height_m;
        }
        b
    }

    /// Speed along the heading, m/s (negative reversing).
    pub fn speed_m_s(&self) -> f32 {
        self.vel.dot(forward2(self.heading_rad))
    }

    /// The four wheels' contact points (x, z): front left, front right, rear left, rear right.
    pub fn wheels(&self, spec: &CarSpec) -> [Vec2; 4] {
        let (f, r) = (forward2(self.heading_rad), right2(self.heading_rad));
        let c = Vec2::new(self.pos.x, self.pos.z);
        let (hb, ht) = (spec.wheelbase_m * 0.5, spec.track_m * 0.5);
        [c + f * hb - r * ht, c + f * hb + r * ht, c - f * hb - r * ht, c - f * hb + r * ht]
    }

    /// The two circles the body collides as: over the front and the rear axle, half its width across.
    pub fn hull(&self, spec: &CarSpec) -> [(Vec2, f32); 2] {
        let f = forward2(self.heading_rad);
        let c = Vec2::new(self.pos.x, self.pos.z);
        let r = spec.size_m[0] * 0.5;
        let half = (spec.size_m[2] * 0.5 - r).max(0.0);
        [(c + f * half, r), (c - f * half, r)]
    }

    /// Advances one fixed step of `dt_s` seconds.
    pub fn step(&mut self, spec: &CarSpec, input: Drive, ground: &impl Ground, obstacles: &[Obstacle], dt_s: f32) -> CarStep {
        let mut out = CarStep::default();
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return out;
        }
        let throttle = if input.throttle.is_finite() {
            input.throttle.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let steer_in = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let fwd = forward2(self.heading_rad);
        let mut v_f = self.vel.dot(fwd);
        let top = spec.top_speed_m_s;

        // Pedals: ease toward the target speed at the rate the pedal allows.
        let toward = |v: f32, target: f32, rate: f32| {
            let d = target - v;
            v + d.clamp(-rate * dt_s, rate * dt_s)
        };
        v_f = if input.handbrake {
            toward(v_f, 0.0, spec.handbrake_brake_m_s2)
        } else if throttle > 0.0 {
            if v_f < -0.5 {
                toward(v_f, 0.0, spec.brake_m_s2)
            } else {
                toward(v_f, top * throttle, spec.accel_m_s2)
            }
        } else if throttle < 0.0 {
            if v_f > 0.5 {
                toward(v_f, 0.0, spec.brake_m_s2 * -throttle)
            } else {
                toward(v_f, -top * spec.reverse_fraction * -throttle, spec.accel_m_s2)
            }
        } else {
            toward(v_f, 0.0, spec.coast_m_s2)
        };

        // Steering: the lock falls with speed; the wheels follow the input at their own rate.
        let speed_share = (v_f.abs() / top).clamp(0.0, 1.0);
        let lock = spec.max_steer_deg.to_radians() * (1.0 + (spec.top_speed_steer_fraction - 1.0) * speed_share);
        let want = steer_in * lock;
        self.steer_rad += (want - self.steer_rad).clamp(-spec.steer_rate_rad_s * dt_s, spec.steer_rate_rad_s * dt_s);

        // Grip: the handbrake lets the rear go; it comes back when released.
        self.rear_grip = if input.handbrake {
            spec.handbrake_grip
        } else {
            (self.rear_grip + spec.grip_recover_per_s * dt_s).min(1.0)
        };

        // Turning about the rear axle: yaw rate = speed / wheelbase x tan(steer); right is negative.
        let loose = 1.0 + (1.0 - self.rear_grip) * spec.handbrake_yaw_gain;
        let yaw_rate = -(v_f / spec.wheelbase_m) * self.steer_rad.tan() * loose;
        let lateral_before = self.vel.dot(right2(self.heading_rad));
        self.heading_rad = (self.heading_rad + yaw_rate * dt_s).rem_euclid(std::f32::consts::TAU);

        // The velocity in the new frame: forward is the pedals' speed, the slide dies at the grip's rate
        // (at full grip at once: the car goes where it points).
        let (f2, r2) = (forward2(self.heading_rad), right2(self.heading_rad));
        let old_dir = v_f * fwd + lateral_before * right2(self.heading_rad - yaw_rate * dt_s);
        let mut lateral = old_dir.dot(r2);
        if self.rear_grip >= 0.999 {
            lateral = 0.0;
        } else {
            lateral *= (-spec.lateral_grip_per_s * self.rear_grip * dt_s).exp();
        }
        self.vel = f2 * v_f + r2 * lateral;

        // Move, then out of walls and obstacles.
        let step = self.vel * dt_s;
        self.pos.x += step.x;
        self.pos.z += step.y;
        self.collide(spec, ground, obstacles, &mut out);

        // Springs over the ground at each wheel.
        self.suspend(spec, ground, dt_s);
        if !self.pos.is_finite() || !self.vel.is_finite() {
            *self = Self::parked(spec, ground, Vec2::ZERO, 0.0);
        }
        out
    }

    fn collide(&mut self, spec: &CarSpec, ground: &impl Ground, obstacles: &[Obstacle], out: &mut CarStep) {
        let feet = self.pos.y;
        for (centre, r) in self.hull(spec) {
            let moved = ground.push_out_at(centre, r, feet);
            let d = moved - centre;
            if d.length_squared() > 1e-8 {
                let n = d.normalize();
                let into = -self.vel.dot(n);
                if into > 0.0 {
                    // a wall stops the car along its face
                    self.vel += n * into;
                    out.impact_m_s = out.impact_m_s.max(into);
                }
                self.pos.x += d.x;
                self.pos.z += d.y;
            }
        }
        for o in obstacles {
            for (centre, r) in self.hull(spec) {
                let gap = centre.distance(o.at) - (r + o.radius_m);
                if gap >= 0.0 {
                    continue;
                }
                let n = (centre - o.at).normalize_or(-forward2(self.heading_rad));
                self.pos.x -= n.x * gap;
                self.pos.z -= n.y * gap;
                let closing = (o.vel - self.vel).dot(n);
                if closing <= 0.0 {
                    continue;
                }
                out.impact_m_s = out.impact_m_s.max(closing);
                if !o.mass_kg.is_finite() {
                    // a mech's leg: the car stops dead
                    self.vel = Vec2::ZERO;
                } else {
                    let j = (1.0 + RESTITUTION) * closing / (1.0 / spec.mass_kg + 1.0 / o.mass_kg);
                    self.vel += n * (j / spec.mass_kg);
                    out.pushed.push((o.id, -n * (j / o.mass_kg)));
                }
            }
        }
        if out.impact_m_s > spec.crash_safe_m_s {
            self.health = (self.health - (out.impact_m_s - spec.crash_safe_m_s) * spec.crash_damage_per_m_s).max(0.0);
        }
    }

    fn suspend(&mut self, spec: &CarSpec, ground: &impl Ground, dt_s: f32) {
        let corner_mass = spec.mass_kg * 0.25;
        let wheels = self.wheels(spec);
        for (c, w) in self.corners.iter_mut().zip(wheels) {
            let g = ground.support_at(w.x, w.y, c.y_m);
            let rest = g + spec.ride_height_m;
            let spring = (spec.spring_n_m * (rest - c.y_m) - spec.damper_n_s_m * c.vel_y_m_s) / corner_mass;
            // A spring cannot pull a wheel down faster than it falls.
            let a = if c.y_m > rest { spring.max(-G_M_S2) } else { spring };
            c.vel_y_m_s += a * dt_s;
            c.y_m += c.vel_y_m_s * dt_s;
            if c.y_m < g + BUMP_STOP_M {
                c.y_m = g + BUMP_STOP_M;
                c.vel_y_m_s = c.vel_y_m_s.max(0.0);
            }
        }
        let [fl, fr, rl, rr] = self.corners.map(|c| c.y_m);
        self.pos.y = (fl + fr + rl + rr) * 0.25 - spec.ride_height_m;
        self.pitch_rad = ((fl + fr) * 0.5 - (rl + rr) * 0.5).atan2(spec.wheelbase_m);
        self.roll_rad = ((fl + rl) * 0.5 - (fr + rr) * 0.5).atan2(spec.track_m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::{Building, Rect};
    use crate::world::{BuildingGrid, FlatGround};

    const DT: f32 = 1.0 / 60.0;

    fn sedan() -> CarSpec {
        CarSpec::default()
    }

    fn run(b: &mut CarBody, spec: &CarSpec, d: Drive, ground: &impl Ground, s: f32) {
        for _ in 0..(s / DT).round() as usize {
            b.step(spec, d, ground, &[], DT);
        }
    }

    fn gas() -> Drive {
        Drive {
            throttle: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn full_throttle_reaches_100_km_h_in_the_time_its_acceleration_gives() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        let mut t = 0.0;
        while b.speed_m_s() < 100.0 / 3.6 {
            b.step(&spec, gas(), &g, &[], DT);
            t += DT;
            assert!(t < 20.0);
        }
        let expected = 100.0 / 3.6 / spec.accel_m_s2;
        assert!((t - expected).abs() < 0.1, "0-100 km/h in {t:.2} s, expected {expected:.2} s");
        assert!(b.pos.z < 0.0, "heading 0 drives toward -z");
    }

    #[test]
    fn braking_from_50_km_h_stops_in_v_squared_over_twice_the_brake() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * (50.0 / 3.6);
        let start = b.pos;
        let brake = Drive {
            throttle: -1.0,
            ..Default::default()
        };
        for _ in 0..600 {
            b.step(&spec, brake, &g, &[], DT);
            if b.speed_m_s() <= 0.5 {
                break;
            }
        }
        let v: f32 = 50.0 / 3.6;
        let expected = v * v / (2.0 * spec.brake_m_s2);
        let d = start.distance(b.pos);
        assert!((d - expected).abs() < 1.0, "stopped in {d:.1} m, expected about {expected:.1} m");
    }

    #[test]
    fn the_turning_circle_is_the_wheelbase_over_the_tangent_of_full_lock() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        let creep = Drive {
            throttle: 4.0 / spec.top_speed_m_s,
            steer: 1.0,
            handbrake: false,
        };
        run(&mut b, &spec, creep, &g, 3.0);
        let mut pts = Vec::new();
        for _ in 0..1200 {
            b.step(&spec, creep, &g, &[], DT);
            pts.push(Vec2::new(b.pos.x, b.pos.z));
        }
        let centre = pts.iter().copied().sum::<Vec2>() / pts.len() as f32;
        let r = pts.iter().map(|p| p.distance(centre)).sum::<f32>() / pts.len() as f32;
        let lock = spec.max_steer_deg.to_radians() * (1.0 + (spec.top_speed_steer_fraction - 1.0) * 4.0 / spec.top_speed_m_s);
        let expected = spec.wheelbase_m / lock.tan();
        assert!((r - expected).abs() / expected < 0.05, "circle {r:.2} m, expected {expected:.2} m");
        // right turn: from heading 0 (facing -z), the circle's centre is to the right (+x)
        assert!(centre.x > 0.0);
    }

    #[test]
    fn full_grip_goes_where_it_points_and_the_handbrake_slides_the_rear() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * 20.0;
        let turn = Drive {
            throttle: 0.4,
            steer: 0.5,
            handbrake: false,
        };
        run(&mut b, &spec, turn, &g, 1.0);
        let slip = b.vel.dot(right2(b.heading_rad)).abs();
        assert!(slip < 1e-3, "grippy: no sideways slide, got {slip}");
        let slide = Drive { handbrake: true, ..turn };
        let heading_before = b.heading_rad;
        run(&mut b, &spec, slide, &g, 0.5);
        let slip = b.vel.dot(right2(b.heading_rad)).abs();
        assert!(slip > 1.0, "the handbrake lets the rear go: {slip:.2} m/s sideways");
        assert!((b.heading_rad - heading_before).abs() > 0.3);
        run(&mut b, &spec, turn, &g, 1.0);
        assert!((b.rear_grip - 1.0).abs() < 1e-6, "grip comes back");
    }

    #[test]
    fn steering_lock_falls_with_speed_so_it_never_spins_out_on_its_own() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * spec.top_speed_m_s;
        let hard = Drive {
            throttle: 1.0,
            steer: 1.0,
            handbrake: false,
        };
        run(&mut b, &spec, hard, &g, 2.0);
        assert!(b.steer_rad <= spec.max_steer_deg.to_radians() * spec.top_speed_steer_fraction * 1.1);
        assert!(b.vel.dot(right2(b.heading_rad)).abs() < 1e-3);
    }

    /// Ground with a 0.15 m kerb for z < -20 and a 1 m pit at z in -40..-37.
    struct Kerb;
    impl Ground for Kerb {
        fn height_at(&self, _x: f32, z: f32) -> f32 {
            if (-40.0..-37.0).contains(&z) {
                -1.0
            } else if z < -20.0 {
                0.15
            } else {
                0.0
            }
        }
        fn push_out(&self, p: Vec2, _r: f32) -> Vec2 {
            p
        }
    }

    #[test]
    fn a_kerb_bumps_the_nose_up_and_a_crater_drops_it() {
        let spec = sedan();
        let mut b = CarBody::parked(&spec, &Kerb, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * 5.0;
        let hold = Drive {
            throttle: 5.0 / spec.top_speed_m_s,
            ..Default::default()
        };
        let mut most_up: f32 = 0.0;
        let mut most_down: f32 = 0.0;
        for _ in 0..600 {
            b.step(&spec, hold, &Kerb, &[], DT);
            if b.pos.z > -30.0 {
                most_up = most_up.max(b.pitch_rad);
            } else {
                most_down = most_down.min(b.pitch_rad);
            }
        }
        assert!(most_up > 0.01, "the kerb lifts the nose: {most_up}");
        assert!(most_down < -0.05, "the pit drops it: {most_down}");
        assert!(b.pos.y > 0.0 && b.pos.y < 0.4, "settles on the kerb's top: {}", b.pos.y);
    }

    #[test]
    fn a_wall_stops_the_car_and_a_hard_hit_hurts_it() {
        let spec = sedan();
        let wall = Building {
            id: 0,
            city: 0,
            footprint: Rect {
                min: Vec2::new(-20.0, -30.0),
                max: Vec2::new(20.0, -20.0),
            },
            storeys: 2,
            height_m: 8.0,
        };
        let g = FlatGround {
            walls: BuildingGrid::new(&[wall], 256.0, 64.0),
        };
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * 15.0;
        let mut worst: f32 = 0.0;
        for _ in 0..180 {
            worst = worst.max(b.step(&spec, Drive::default(), &g, &[], DT).impact_m_s);
        }
        assert!(b.pos.z > -20.0 - 0.01 + spec.size_m[0] * 0.5 - spec.size_m[2] * 0.5 - 0.5);
        assert!(b.speed_m_s().abs() < 0.5, "stopped against the wall");
        assert!(worst > 10.0 && b.health < 1.0, "hit at {worst:.1} m/s, health {}", b.health);
    }

    #[test]
    fn a_mech_leg_stops_the_car_dead_and_another_car_is_shoved() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * 10.0;
        let leg = Obstacle {
            id: 1,
            at: Vec2::new(0.0, -6.0),
            radius_m: 1.5,
            vel: Vec2::ZERO,
            mass_kg: f32::INFINITY,
        };
        let mut hit = 0.0f32;
        for _ in 0..60 {
            hit = hit.max(b.step(&spec, Drive::default(), &g, &[leg], DT).impact_m_s);
        }
        assert!(hit > 9.0 && b.vel.length() < 1e-3);

        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        b.vel = forward2(0.0) * 10.0;
        let other = Obstacle {
            id: 7,
            at: Vec2::new(0.0, -5.0),
            radius_m: 0.9,
            vel: Vec2::ZERO,
            mass_kg: 1400.0,
        };
        let s = b.step(&spec, Drive::default(), &g, &[other], DT * 20.0);
        assert_eq!(s.pushed.len(), 1);
        assert_eq!(s.pushed[0].0, 7);
        assert!(s.pushed[0].1.y < -1.0, "shoved ahead");
    }

    #[test]
    fn the_speedometer_needle_reads_the_simulations_speed() {
        let spec = sedan();
        let needle = spec.speedo_needle_rad(50.0 / 3.6);
        assert!((spec.speedo_reading_km_h(needle) - 50.0).abs() < 1e-3);
        assert_eq!(spec.speedo_needle_rad(500.0), spec.speedo_sweep_deg.to_radians());
    }

    #[test]
    fn the_eye_and_door_sockets_turn_with_the_car() {
        let p = CarSpec::to_world(Vec3::ZERO, 0.0, [-1.0, 0.0, 0.0]);
        assert!((p - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5, "left is -x facing -z");
        let p = CarSpec::to_world(Vec3::ZERO, std::f32::consts::FRAC_PI_2, [0.0, 0.0, -1.0]);
        let f = forward2(std::f32::consts::FRAC_PI_2);
        assert!((Vec2::new(p.x, p.z) - f).length() < 1e-5, "-z local is forward");
    }

    #[test]
    fn bad_input_changes_nothing_harmful() {
        let spec = sedan();
        let g = FlatGround::default();
        let mut b = CarBody::parked(&spec, &g, Vec2::ZERO, 0.0);
        let before = b.clone();
        b.step(&spec, gas(), &g, &[], f32::NAN);
        assert_eq!(b, before);
        b.step(
            &spec,
            Drive {
                throttle: f32::NAN,
                steer: f32::INFINITY,
                handbrake: false,
            },
            &g,
            &[],
            DT,
        );
        assert!(b.pos.is_finite() && b.vel.is_finite());
    }
}
