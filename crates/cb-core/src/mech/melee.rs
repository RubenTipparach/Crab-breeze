//! Close combat (openspec change `close-combat`): the mode, which switches on when the target comes
//! within melee range and which the pilot toggles, and the arms' strikes (a wind-up, a hit, a
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

/// The close-combat mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloseCombat {
    /// In close combat: the fire controls strike and the guns hold.
    pub on: bool,
    /// Switched on by the pilot, so it stays on with no one in range.
    by_hand: bool,
    /// Switched off by the pilot while the target was in range: it stays off until the target
    /// leaves range.
    held_off: bool,
    /// A targeted hostile has been in range while it was on: when that target goes down or moves
    /// away, it goes off, however it was switched on.
    engaged: bool,
}

impl CloseCombat {
    /// One step: `near_m` is the distance from the eye to the targeted hostile, `None` with no
    /// target (an enemy that is near but not targeted never switches it on: owner, 2026-10-24,
    /// "only hostiles targeted and gets close does this auto melee"); `toggled` is the pilot's
    /// toggle this step.
    pub fn step(&mut self, near_m: Option<f32>, toggled: bool, cfg: &MeleeConfig) {
        let in_range = near_m.is_some_and(|d| d.is_finite() && d <= cfg.range_m);
        if toggled {
            *self = if self.on {
                Self {
                    held_off: in_range,
                    ..Self::default()
                }
            } else {
                Self {
                    on: true,
                    by_hand: true,
                    held_off: false,
                    engaged: in_range,
                }
            };
        } else if self.held_off {
            self.held_off = in_range;
        } else if in_range {
            self.on = true;
            self.engaged = true;
        } else if self.engaged || !self.by_hand {
            // The target went down or moved away (owner, 2026-10-24: "melee is off if hostile is
            // down, or moves away").
            *self = Self::default();
        }
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

/// Drawn back, the arm swings at the shoulder this far behind its rest, and the forearm curls this far.
const DRAW_UPPER_RAD: f32 = 0.52;
const DRAW_FORE_RAD: f32 = -0.35;
/// Thrown, the arm swings forward at the shoulder to about level and the forearm straightens along it.
const THROW_UPPER_RAD: f32 = -1.4;
const THROW_FORE_RAD: f32 = 1.05;

/// The arm bones' model-space rotations for a swing (see [`Strikes::swing`]) of each arm, under the
/// torso's rotation `torso`: the shoulder turns about its side axis (+x in model space, +z forward),
/// carrying the upper arm, and the forearm turns with it and about the elbow. The shoulder, not
/// `upper_arm_*`, which carries no mesh: turning that swung the forearm away from the upper arm's
/// plating (openspec change `arm-aim`). An arm at rest (swing 0) is left to follow the torso and is
/// not listed.
pub fn arm_rotations(torso: Quat, swing: [f32; 2]) -> Vec<(&'static str, Quat)> {
    let mut out = Vec::new();
    for (arm, (upper, fore)) in [("shoulder_L", "forearm_L"), ("shoulder_R", "forearm_R")].into_iter().enumerate() {
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
    fn close_combat_comes_on_when_the_target_walks_into_reach_and_off_when_it_leaves() {
        let cfg = MeleeConfig::default();
        let mut c = CloseCombat::default();
        c.step(Some(cfg.range_m + 50.0), false, &cfg);
        assert!(!c.on);
        c.step(Some(25.0), false, &cfg);
        assert!(c.on);
        c.step(Some(cfg.range_m + 30.0), false, &cfg);
        assert!(!c.on);
    }

    #[test]
    fn toggling_it_off_holds_it_off_until_the_target_leaves_range() {
        let cfg = MeleeConfig::default();
        let mut c = CloseCombat::default();
        c.step(Some(20.0), false, &cfg);
        c.step(Some(20.0), true, &cfg);
        assert!(!c.on);
        c.step(Some(15.0), false, &cfg);
        assert!(!c.on, "held off while the target stays near");
        c.step(Some(cfg.range_m + 20.0), false, &cfg);
        c.step(Some(20.0), false, &cfg);
        assert!(c.on, "back on when it comes into range again");
    }

    #[test]
    fn toggling_it_on_with_no_one_near_keeps_it_on_until_toggled_again() {
        let cfg = MeleeConfig::default();
        let mut c = CloseCombat::default();
        c.step(None, true, &cfg);
        assert!(c.on);
        c.step(Some(500.0), false, &cfg);
        assert!(c.on);
        c.step(None, true, &cfg);
        assert!(!c.on);
    }

    #[test]
    fn it_goes_off_when_the_target_goes_down_even_if_switched_on_by_hand() {
        let cfg = MeleeConfig::default();
        let mut c = CloseCombat::default();
        c.step(None, true, &cfg);
        c.step(Some(20.0), false, &cfg);
        assert!(c.on);
        // Down: no longer a target.
        c.step(None, false, &cfg);
        assert!(!c.on);
    }

    #[test]
    fn an_untargeted_enemy_close_by_does_not_switch_it_on() {
        let cfg = MeleeConfig::default();
        let mut c = CloseCombat::default();
        // The caller passes only the targeted hostile's range: with none targeted, `None`.
        for _ in 0..10 {
            c.step(None, false, &cfg);
        }
        assert!(!c.on);
    }

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
    fn a_thrown_arm_turns_its_shoulder_and_points_the_hanging_arm_forward() {
        let r = arm_rotations(Quat::IDENTITY, [1.0, 0.0]);
        assert_eq!(r.len(), 2, "only the swinging arm is posed");
        assert_eq!(
            [r[0].0, r[1].0],
            ["shoulder_L", "forearm_L"],
            "the strike turns the shoulder, which carries the upper arm's mesh, and the forearm"
        );
        let down = glam::Vec3::NEG_Y;
        let thrown = r[0].1 * down;
        assert!(thrown.z > 0.95, "forward is +z in model space: {thrown:?}");
    }
}
