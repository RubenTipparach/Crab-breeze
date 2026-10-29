//! Close combat (openspec change `close-combat`): the arms' strikes, thrown at any time from the seat (owner,
//! 2026-10-29: "remove melee toggle, always allo z/v to punch") (a wind-up, a hit, a
//! recovery, each arm on its own clock). Where a strike lands is resolved by the weapons' hit rules;
//! this module only says when it lands and how the arm is posed. Pure.

use crate::data::{Validate, positive};
use glam::Quat;
use serde::Deserialize;

/// Close combat in a mech's data (`melee:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MeleeConfig {
    /// The mode switches on when the target is this close to the pilot's eye.
    pub range_m: f32,
    /// How far ahead of the shoulder a strike reaches.
    pub reach_m: f32,
    /// The arm draws back.
    pub windup_s: f32,
    /// The arm is thrown; the strike lands at the start of this phase.
    pub hit_s: f32,
    /// The arm comes back to its rest pose.
    pub recover_s: f32,
    /// Armour a strike beats, millimetres of rolled steel equivalent.
    pub penetration_mm: f32,
    /// Structure points it takes from a part it beats.
    pub damage: f32,
}

impl Default for MeleeConfig {
    fn default() -> Self {
        Self {
            // Owner, 2026-10-25: "range should be 80m instead" (was 30 m, survey G18).
            range_m: 80.0,
            reach_m: 9.0,
            windup_s: 0.35,
            hit_s: 0.15,
            recover_s: 0.4,
            penetration_mm: 300.0,
            damage: 60.0,
        }
    }
}

impl Validate for MeleeConfig {
    fn validate(&self) -> Result<(), String> {
        for (k, v) in [
            ("melee.range_m", self.range_m),
            ("melee.reach_m", self.reach_m),
            ("melee.windup_s", self.windup_s),
            ("melee.hit_s", self.hit_s),
            ("melee.recover_s", self.recover_s),
            ("melee.penetration_mm", self.penetration_mm),
            ("melee.damage", self.damage),
        ] {
            positive(k, v)?;
        }
        Ok(())
    }
}

impl MeleeConfig {
    /// A whole strike, wind-up to rest.
    pub fn strike_s(&self) -> f32 {
        self.windup_s + self.hit_s + self.recover_s
    }
}

/// The arms' strikes: seconds into each arm's strike, [left, right], or `None` at rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Strikes {
    /// Time into each arm's strike.
    pub t_s: [Option<f32>; 2],
}

impl Strikes {
    /// Advances the strikes by `dt_s` and starts one on each pressed arm that is at rest and still
    /// on. Returns the arms whose strike lands this step (its hit phase begins).
    pub fn step(&mut self, cfg: &MeleeConfig, pressed: [bool; 2], arm_lost: [bool; 2], dt_s: f32) -> [bool; 2] {
        let mut lands = [false; 2];
        for arm in 0..2 {
            if arm_lost[arm] {
                self.t_s[arm] = None;
                continue;
            }
            match self.t_s[arm] {
                Some(t) => {
                    let next = t + dt_s;
                    lands[arm] = t < cfg.windup_s && next >= cfg.windup_s;
                    self.t_s[arm] = (next < cfg.strike_s()).then_some(next);
                }
                None if pressed[arm] => self.t_s[arm] = Some(0.0),
                None => {}
            }
        }
        lands
    }

    /// Where an arm is in its swing: 0 at rest, toward -1 drawn back in the wind-up, 1 thrown at
    /// the end of the hit, easing back to 0 in the recovery.
    pub fn swing(&self, cfg: &MeleeConfig, arm: usize) -> f32 {
        let Some(t) = self.t_s.get(arm).copied().flatten() else {
            return 0.0;
        };
        let ease = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        if t < cfg.windup_s {
            -ease(t / cfg.windup_s)
        } else if t < cfg.windup_s + cfg.hit_s {
            -1.0 + 2.0 * ease((t - cfg.windup_s) / cfg.hit_s)
        } else {
            1.0 - ease((t - cfg.windup_s - cfg.hit_s) / cfg.recover_s)
        }
    }
}

/// Drawn back, the upper arm swings this far behind its rest, and the forearm curls this far.
const DRAW_UPPER_RAD: f32 = 0.52;
const DRAW_FORE_RAD: f32 = -0.35;
/// Thrown, the upper arm swings forward to about level and the forearm straightens along it.
const THROW_UPPER_RAD: f32 = -1.4;
const THROW_FORE_RAD: f32 = 1.05;

/// The arm bones' model-space rotations for a swing (see [`Strikes::swing`]) of each arm, under the
/// torso's rotation `torso`: the upper arm turns about the shoulder's side axis (+x in model space,
/// +z forward) and the forearm turns with it and about the elbow. An arm at rest (swing 0) is left
/// to follow the torso and is not listed.
pub fn arm_rotations(torso: Quat, swing: [f32; 2]) -> Vec<(&'static str, Quat)> {
    let mut out = Vec::new();
    for (arm, (upper, fore)) in [("upper_arm_L", "forearm_L"), ("upper_arm_R", "forearm_R")].into_iter().enumerate() {
        let s = swing[arm];
        if s == 0.0 || !s.is_finite() {
            continue;
        }
        let (u, f) = if s < 0.0 {
            (DRAW_UPPER_RAD * -s, DRAW_FORE_RAD * -s)
        } else {
            (THROW_UPPER_RAD * s, THROW_FORE_RAD * s)
        };
        let up = torso * Quat::from_rotation_x(u);
        out.push((upper, up));
        out.push((fore, up * Quat::from_rotation_x(f)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn a_strike_lands_once_after_its_wind_up_and_ends_at_rest() {
        let cfg = MeleeConfig::default();
        let mut s = Strikes::default();
        let mut landed_at = Vec::new();
        let mut t = 0.0;
        s.step(&cfg, [false, true], [false; 2], DT);
        for _ in 0..120 {
            t += DT;
            if s.step(&cfg, [false, true], [false; 2], DT)[1] {
                landed_at.push(t);
            }
            if s.t_s[1].is_none() {
                break;
            }
        }
        assert_eq!(landed_at.len(), 1);
        assert!((landed_at[0] - cfg.windup_s).abs() <= DT, "{landed_at:?}");
        assert_eq!(s.t_s[1], None);
        assert_eq!(s.swing(&cfg, 1), 0.0);
    }

    #[test]
    fn a_lost_arm_cannot_strike() {
        let cfg = MeleeConfig::default();
        let mut s = Strikes::default();
        for _ in 0..60 {
            assert_eq!(s.step(&cfg, [true, false], [true, false], DT), [false; 2]);
        }
        assert_eq!(s.t_s[0], None);
    }

    #[test]
    fn the_arm_draws_back_then_is_thrown_then_comes_home() {
        let cfg = MeleeConfig::default();
        let at = |t: f32| Strikes { t_s: [Some(t), None] }.swing(&cfg, 0);
        assert!(at(cfg.windup_s * 0.9) < -0.9);
        assert!((at(cfg.windup_s + cfg.hit_s) - 1.0).abs() < 1e-3);
        assert!(at(cfg.strike_s() - 0.001).abs() < 0.01);
    }

    #[test]
    fn a_thrown_arm_points_its_hanging_upper_arm_forward() {
        let r = arm_rotations(Quat::IDENTITY, [1.0, 0.0]);
        assert_eq!(r.len(), 2, "only the swinging arm is posed");
        let down = glam::Vec3::NEG_Y;
        let thrown = r[0].1 * down;
        assert!(thrown.z > 0.95, "forward is +z in model space: {thrown:?}");
    }
}
