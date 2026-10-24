//! The AI pilot (openspec change `enemy-mechs`): drives a mech through the same controls a player's
//! seat writes. It closes to its preferred range and circles there, twists the torso onto the
//! target's lead point, and fires when its aim is close enough. Seeded; reads only what a pilot
//! could see (the target's position and velocity).

use crate::mech::{MechControls, MechState};
use crate::rng::SimRng;
use crate::weapons::lead;
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::f32::consts::{PI, TAU};

/// How good an AI pilot is.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Skill {
    /// Random aim error, degrees.
    pub aim_noise_deg: f32,
    /// Fires when the torso is within this of the lead point, degrees.
    pub fire_error_deg: f32,
    /// Range it likes to fight at.
    pub range_m: f32,
    /// Farthest it fires from.
    pub max_fire_range_m: f32,
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            aim_noise_deg: 1.2,
            fire_error_deg: 4.0,
            range_m: 220.0,
            max_fire_range_m: 900.0,
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

/// An AI pilot.
#[derive(Clone, Debug, PartialEq)]
pub struct AiPilot {
    /// Its skill.
    pub skill: Skill,
    rng: SimRng,
    noise: Vec2,
    retarget_s: f32,
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// World yaw of a direction (0 faces -z, positive turns left), as the mech's heading.
fn yaw_of(d: Vec3) -> f32 {
    (-d.x).atan2(-d.z)
}

impl AiPilot {
    /// A pilot with a skill and a seed.
    pub fn new(skill: Skill, rng: SimRng) -> Self {
        Self {
            skill,
            rng,
            noise: Vec2::ZERO,
            retarget_s: 0.0,
        }
    }

    /// Decides this step. `eye` is the AI's sensor position; `speed_m_s` its main gun's muzzle speed.
    pub fn think(&mut self, me: &MechState, eye: Vec3, target: Vec3, target_vel: Vec3, speed_m_s: f32, dt_s: f32) -> Orders {
        // New aim error every 0.8 s, as a hand settles.
        self.retarget_s -= dt_s;
        if self.retarget_s <= 0.0 {
            self.retarget_s = 0.8;
            let n = self.skill.aim_noise_deg.to_radians();
            self.noise = Vec2::new(self.rng.range_f32(-n, n), self.rng.range_f32(-n, n));
        }
        let to = target - me.pos;
        let flat = Vec2::new(to.x, to.z);
        let dist = flat.length();
        let bearing = yaw_of(to);

        // Legs: close in, back off, or circle at the preferred range.
        let r = self.skill.range_m;
        let (want_heading, throttle) = if dist > r + 60.0 {
            (bearing, 0.85)
        } else if dist < r - 60.0 {
            (bearing, -0.35)
        } else {
            (bearing + 1.4, 0.55)
        };
        let turn = (wrap(want_heading - me.heading_rad) * 2.0).clamp(-1.0, 1.0);

        // Torso: onto the lead point, with the hand's error.
        let aim_point = lead(eye, target + Vec3::Y * 7.0, target_vel, speed_m_s);
        let d = aim_point - eye;
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
        let in_range = dist < self.skill.max_fire_range_m;
        let ready = err < self.skill.fire_error_deg && in_range;
        Orders {
            controls: MechControls {
                throttle_axis: ((throttle - me.throttle) * 5.0).clamp(-1.0, 1.0),
                turn_axis: turn,
                aim_delta_deg,
                ..Default::default()
            },
            fire: [ready, ready && err < self.skill.fire_error_deg * 0.6],
            aim_point,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mech::{GaitConfig, MechConfig};
    use crate::world::FlatGround;

    fn run(ai: &mut AiPilot, me: &mut MechState, target: Vec3, vel: Vec3, seconds: f32) -> Orders {
        let cfg = MechConfig::default();
        let mut o = Orders::default();
        for _ in 0..(seconds * 60.0) as usize {
            o = ai.think(me, me.pos + Vec3::Y * 9.6, target, vel, 850.0, 1.0 / 60.0);
            me.step(&o.controls, &cfg, &FlatGround::default(), 1.0 / 60.0);
        }
        o
    }

    #[test]
    fn the_ai_turns_its_torso_onto_the_target_and_fires() {
        let mut ai = AiPilot::new(
            Skill {
                aim_noise_deg: 0.0,
                ..Skill::default()
            },
            SimRng::new(3),
        );
        let mut me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let target = Vec3::new(-150.0, 0.0, -150.0);
        let o = run(&mut ai, &mut me, target, Vec3::ZERO, 4.0);
        let facing = me.heading_rad + me.twist_rad;
        let to = target - me.pos;
        assert!(wrap(facing - yaw_of(to)).abs() < 5f32.to_radians(), "torso on target");
        assert!(o.fire[0], "and fires");
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
        let o = ai.think(
            &me,
            Vec3::Y * 9.6,
            Vec3::new(0.0, 0.0, -400.0),
            Vec3::new(7.0, 0.0, 0.0),
            850.0,
            0.016,
        );
        assert!(o.aim_point.x > 2.0, "aims ahead: {}", o.aim_point);
    }

    #[test]
    fn the_ai_closes_to_its_range() {
        let mut ai = AiPilot::new(Skill::default(), SimRng::new(5));
        let mut me = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let target = Vec3::new(0.0, 0.0, -700.0);
        run(&mut ai, &mut me, target, Vec3::ZERO, 60.0);
        let d = Vec2::new(me.pos.x - target.x, me.pos.z - target.z).length();
        assert!(d < 450.0, "closed from 700 m to {d}");
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
    /// Gun override; `None` keeps the mech's own.
    #[serde(default)]
    pub weapons: Option<Vec<crate::weapons::WeaponSpec>>,
    /// Its pilot.
    #[serde(default)]
    pub skill: Skill,
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
            if let Some(w) = &e.weapons {
                for spec in w {
                    crate::data::Validate::validate(spec)?;
                }
            }
        }
        Ok(())
    }
}
