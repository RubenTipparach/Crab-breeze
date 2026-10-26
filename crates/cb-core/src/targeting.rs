//! Targeting (openspec change `targeting`, after Ace Combat): which enemy the pilot has selected,
//! picking the one nearest the aim or cycling by distance, and the missile lock that builds only
//! while the target is in missile range and near the aim. Pure: `cb-app` feeds it what the pilot
//! can see each fixed step; the cockpit draws what it says and the missiles home on the lock.

use crate::data::{Validate, positive, positive_range};
use glam::Vec3;
use serde::Deserialize;

/// `data/hud/targeting.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TargetingConfig {
    /// Enemies farther than this are not targets.
    pub max_range_m: f32,
    /// The pick takes the enemy nearest the aim within this cone.
    pub pick_cone_deg: f32,
    /// A lock builds only inside this range (missile range).
    pub lock_range_m: f32,
    /// ...and with the target within this angle of the aim.
    pub lock_cone_deg: f32,
    /// Time on target to lock.
    pub lock_s: f32,
    /// A target box's size on screen, [smallest, largest] pixels.
    pub box_px: [f32; 2],
}

impl Default for TargetingConfig {
    fn default() -> Self {
        Self {
            max_range_m: 2000.0,
            pick_cone_deg: 30.0,
            lock_range_m: 1200.0,
            lock_cone_deg: 12.0,
            lock_s: 1.0,
            box_px: [18.0, 160.0],
        }
    }
}

impl Validate for TargetingConfig {
    fn validate(&self) -> Result<(), String> {
        positive("max_range_m", self.max_range_m)?;
        positive("pick_cone_deg", self.pick_cone_deg)?;
        positive("lock_range_m", self.lock_range_m)?;
        positive("lock_cone_deg", self.lock_cone_deg)?;
        positive("lock_s", self.lock_s)?;
        positive_range("box_px", self.box_px)
    }
}

/// A target the pilot can see: its stable id and where its torso is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// The mech's id.
    pub id: u32,
    /// Its torso centre.
    pub torso: Vec3,
    /// Not an enemy (a civil aircraft): boxed as information, but never selected, so never locked
    /// (owner, 2026-10-26, survey W47: "Never: only unguided fire can hit one").
    pub neutral: bool,
}

/// The angle in degrees between `aim` and the direction from `eye` to `p`.
pub fn off_aim_deg(eye: Vec3, aim: Vec3, p: Vec3) -> f32 {
    aim.normalize_or_zero().angle_between((p - eye).normalize_or_zero()).to_degrees()
}

/// The enemy nearest the aim within `cone_deg`, ties by id (a neutral is never picked this way).
pub fn nearest_to_aim(eye: Vec3, aim: Vec3, targets: &[Target], cone_deg: f32) -> Option<u32> {
    targets
        .iter()
        .filter(|t| !t.neutral)
        .map(|t| (off_aim_deg(eye, aim, t.torso), t.id))
        .filter(|(a, _)| *a <= cone_deg)
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| id)
}

/// The next enemy after `selected` by distance from `eye`, wrapping; the nearest if none is selected
/// (neutrals are not in the cycle).
pub fn next(selected: Option<u32>, targets: &[Target], eye: Vec3) -> Option<u32> {
    let mut by_range: Vec<(f32, u32)> = targets
        .iter()
        .filter(|t| !t.neutral)
        .map(|t| (t.torso.distance(eye), t.id))
        .collect();
    by_range.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let at = selected.and_then(|s| by_range.iter().position(|(_, id)| *id == s));
    match at {
        Some(i) => by_range.get((i + 1) % by_range.len()).map(|(_, id)| *id),
        None => by_range.first().map(|(_, id)| *id),
    }
}

/// The selection after this step: the one kept from the last step while it is still a target, else
/// the nearest the aim within `cfg.pick_cone_deg`; with the cycle key (`cycle`), the next by
/// distance. A neutral is never the result, however it was reached (survey W47), so the lock, which
/// builds only on the selection, never builds on one and no missile homes on one.
pub fn select(selected: Option<u32>, cycle: bool, eye: Vec3, aim: Vec3, targets: &[Target], cfg: &TargetingConfig) -> Option<u32> {
    let still = selected.filter(|s| targets.iter().any(|t| t.id == *s && !t.neutral));
    if cycle {
        next(still, targets, eye)
    } else {
        still.or_else(|| nearest_to_aim(eye, aim, targets, cfg.pick_cone_deg))
    }
}

/// A missile lock building on one target.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lock {
    /// The target it builds on.
    pub target: Option<u32>,
    /// Time on target so far.
    pub held_s: f32,
}

impl Lock {
    /// Builds while `target` is within the lock range and cone of the aim, falls back at twice the
    /// rate otherwise, restarts on a new target. True when locked.
    pub fn step(&mut self, target: Option<u32>, off_deg: f32, range_m: f32, cfg: &TargetingConfig, dt_s: f32) -> bool {
        if target != self.target {
            *self = Lock { target, held_s: 0.0 };
        }
        let on = target.is_some() && range_m <= cfg.lock_range_m && off_deg <= cfg.lock_cone_deg;
        self.held_s = if on {
            (self.held_s + dt_s).min(cfg.lock_s)
        } else {
            (self.held_s - 2.0 * dt_s).max(0.0)
        };
        self.locked(cfg)
    }

    /// Locked now.
    pub fn locked(&self, cfg: &TargetingConfig) -> bool {
        self.target.is_some() && self.held_s >= cfg.lock_s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u32, x: f32, z: f32) -> Target {
        Target {
            id,
            torso: Vec3::new(x, 0.0, z),
            neutral: false,
        }
    }

    fn civ(id: u32, x: f32, z: f32) -> Target {
        Target {
            neutral: true,
            ..t(id, x, z)
        }
    }

    #[test]
    fn a_civil_aircraft_is_never_picked_or_cycled_to_by_itself() {
        let ts = [civ(9, 0.0, -300.0), t(2, 60.0, -400.0)];
        assert_eq!(
            nearest_to_aim(Vec3::ZERO, Vec3::NEG_Z, &ts, 30.0),
            Some(2),
            "the enemy, not the airliner on the crosshair"
        );
        assert_eq!(nearest_to_aim(Vec3::ZERO, Vec3::NEG_Z, &ts[..1], 30.0), None);
        assert_eq!(next(None, &ts, Vec3::ZERO), Some(2));
        assert_eq!(next(Some(2), &ts, Vec3::ZERO), Some(2), "the cycle has one enemy in it");
        assert_eq!(next(Some(9), &ts, Vec3::ZERO), Some(2));
    }

    #[test]
    fn a_civil_aircraft_is_never_selected_or_locked_however_the_pilot_tries() {
        // Owner, 2026-10-26, survey W47: "Never: only unguided fire can hit one".
        let cfg = TargetingConfig::default();
        let alone = [civ(9, 0.0, -600.0)];
        let (eye, aim) = (Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(select(None, false, eye, aim, &alone, &cfg), None, "the crosshair held on it");
        assert_eq!(select(None, true, eye, aim, &alone, &cfg), None, "Q with the crosshair on it");
        assert_eq!(
            select(Some(9), false, eye, aim, &alone, &cfg),
            None,
            "a stale selection of it is dropped"
        );
        let mixed = [civ(9, 0.0, -300.0), t(2, 60.0, -400.0)];
        assert_eq!(select(Some(9), true, eye, aim, &mixed, &cfg), Some(2), "Q goes to the enemy");
        let mut held = None;
        let mut lock = Lock::default();
        for _ in 0..300 {
            held = select(held, false, eye, aim, &alone, &cfg);
            assert!(!lock.step(held, 0.0, 600.0, &cfg, 0.01), "no lock ever builds on an airliner");
        }
        assert_eq!(lock.target, None);
    }

    #[test]
    fn the_target_nearest_the_crosshair_is_picked() {
        let ts = [t(1, 100.0, -500.0), t(2, -10.0, -400.0), t(3, 0.0, 500.0)];
        assert_eq!(nearest_to_aim(Vec3::ZERO, Vec3::NEG_Z, &ts, 30.0), Some(2));
        assert_eq!(nearest_to_aim(Vec3::ZERO, Vec3::X, &ts, 30.0), None, "nothing within the cone");
    }

    #[test]
    fn next_cycles_by_distance_and_wraps() {
        let ts = [t(7, 0.0, -900.0), t(3, 0.0, -300.0), t(5, 0.0, -600.0)];
        assert_eq!(next(None, &ts, Vec3::ZERO), Some(3));
        assert_eq!(next(Some(3), &ts, Vec3::ZERO), Some(5));
        assert_eq!(next(Some(5), &ts, Vec3::ZERO), Some(7));
        assert_eq!(next(Some(7), &ts, Vec3::ZERO), Some(3), "wraps");
    }

    #[test]
    fn a_lock_builds_in_a_second_in_range() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        let mut t_s = 0.0;
        while !l.step(Some(4), 3.0, 800.0, &cfg, 0.01) {
            t_s += 0.01;
            assert!(t_s < 2.0);
        }
        assert!((t_s - cfg.lock_s).abs() < 0.02, "{t_s}");
    }

    #[test]
    fn no_lock_out_of_range() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        for _ in 0..500 {
            assert!(!l.step(Some(4), 0.0, 1500.0, &cfg, 0.01));
        }
    }

    #[test]
    fn looking_away_breaks_the_lock() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        for _ in 0..150 {
            l.step(Some(4), 1.0, 800.0, &cfg, 0.01);
        }
        assert!(l.locked(&cfg));
        assert!(!l.step(Some(4), 40.0, 800.0, &cfg, 0.01), "40 degrees off loses it");
        assert!(!l.step(Some(9), 0.0, 800.0, &cfg, 0.01), "a new target starts over");
    }
}
