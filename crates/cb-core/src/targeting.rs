//! Targeting (openspec change `targeting`, after Ace Combat): which enemy the pilot has selected,
//! picking the one nearest the aim or cycling by distance, and the missile lock that builds only
//! while the target is in missile range and near the aim. Pure: `cb-app` feeds it what the pilot
//! can see each fixed step; the cockpit draws what it says and the missiles home on the lock.
//!
//! Fog hides targets (survey F2; owner, 2026-10-26: "Approve all"): the fog along the line from
//! the pilot's eye to a target (`world::mist::line_depth`, the fog the compose pass draws) shortens
//! how far it is boxed and how far it locks, and slows the lock building on it, once less than
//! `fog_clear_share` of its light gets through. `fog_scale` is that rule, and `seen` and
//! `Seeker` are the only places a range is compared, so the boxes (the civil CIV boxes among them),
//! the shoulder missile's lock and a pod's locks agree.
//!
//! A missile's seeker (openspec change `multi-lock`) is one rule for a single lock and for a pod
//! that gathers several: `Seeker::reaches` says what a lock may build on, `Lock` holds the shoulder
//! missile's one lock on the selected target, `Locks` gathers a pod's locks while its trigger is held.
//! Both go through the same fog rule: `Seeker::reaches` and `Seeker::keeps` take the fog toward the
//! target, and both build a lock `fog` times as fast.

use crate::data::{Validate, positive, positive_range, within};
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
    /// While at least this share (0..1) of a target's light gets through the fog, the fog does not
    /// hide it (survey F2).
    pub fog_clear_share: f32,
    /// In fog nothing gets through, the boxing and lock ranges are this share of their own, and a
    /// lock builds this much as fast.
    pub fog_min_scale: f32,
    /// Q passes over a target it picked within this long, seconds (`cycle`).
    pub cycle_cooldown_s: f32,
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
            fog_clear_share: 0.6,
            fog_min_scale: 0.25,
            cycle_cooldown_s: 15.0,
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
        positive_range("box_px", self.box_px)?;
        within("fog_clear_share", self.fog_clear_share, 0.01, 1.0)?;
        within("fog_min_scale", self.fog_min_scale, 0.01, 1.0)?;
        within("cycle_cooldown_s", self.cycle_cooldown_s, 0.0, 600.0)
    }
}

impl TargetingConfig {
    /// The shoulder missile's seeker: one lock, built in `lock_s` inside `lock_range_m` and
    /// `lock_cone_deg` (the single-lock missile is the same rule as a pod, `multi-lock`).
    pub fn seeker(&self, turn_deg_s: f32) -> Seeker {
        Seeker {
            max_locks: 1,
            lock_s: self.lock_s,
            range_m: self.lock_range_m,
            cone_deg: self.lock_cone_deg,
            turn_deg_s,
        }
    }
}

/// A missile's seeker in its weapon's data (openspec change `multi-lock`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Seeker {
    /// Locks it can hold: 1 for the shoulder missile, 6 for a pod.
    pub max_locks: u8,
    /// Seconds to add one lock.
    pub lock_s: f32,
    /// A lock builds only inside this range, metres.
    pub range_m: f32,
    /// ...and inside this angle of the aim, degrees. A held lock drops beyond 1.5 times it.
    pub cone_deg: f32,
    /// How hard the missile turns toward its lock, degrees a second.
    pub turn_deg_s: f32,
}

impl Validate for Seeker {
    fn validate(&self) -> Result<(), String> {
        if self.max_locks == 0 {
            return Err("seeker.max_locks: at least 1".into());
        }
        positive("seeker.lock_s", self.lock_s)?;
        positive("seeker.range_m", self.range_m)?;
        positive("seeker.cone_deg", self.cone_deg)?;
        positive("seeker.turn_deg_s", self.turn_deg_s)
    }
}

impl Seeker {
    /// Something `range_m` away, through fog leaving `fog` of the reach (`fog_scale`), is in this
    /// seeker's range: `range_m` shortened by the fog (survey F2). The one lock range check, for
    /// the shoulder missile and a pod alike.
    pub fn in_range(&self, range_m: f32, fog: f32) -> bool {
        range_m <= self.range_m * fog.clamp(0.0, 1.0)
    }

    /// A lock may build on something `range_m` away, `off_deg` off the aim, through fog leaving
    /// `fog` of the reach.
    pub fn reaches(&self, range_m: f32, off_deg: f32, fog: f32) -> bool {
        self.in_range(range_m, fog) && off_deg <= self.cone_deg
    }

    /// A held lock is kept on something `range_m` away, `off_deg` off the aim, through fog leaving
    /// `fog` of the reach.
    pub fn keeps(&self, range_m: f32, off_deg: f32, fog: f32) -> bool {
        self.in_range(range_m, fog) && off_deg <= self.cone_deg * 1.5
    }
}

/// A pod's locks, gathered while its trigger is held (openspec change `multi-lock`): target ids in
/// the order they were taken, a target listed once per lock.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Locks {
    /// The locks held.
    pub held: Vec<u32>,
    /// Time toward the next lock.
    pub building_s: f32,
    /// Time the trigger has been held.
    pub held_s: f32,
}

impl Locks {
    /// One fixed step with the trigger held: drop locks on targets gone, out of range or beyond 1.5
    /// times the cone; then, below `max_locks`, build toward a lock on the target in range and cone
    /// with the fewest locks, nearest the aim on a tie, then lowest id (stable order). Neutrals are
    /// never locked (survey W47). Each target's `fog` shortens the range as it does the shoulder
    /// missile's, and a lock builds `fog` times as fast (survey F2).
    pub fn step(&mut self, s: &Seeker, eye: Vec3, aim: Vec3, targets: &[Target], dt_s: f32) {
        self.held_s += dt_s;
        let at = |id: u32| targets.iter().find(|t| t.id == id && !t.neutral);
        self.held
            .retain(|id| at(*id).is_some_and(|t| s.keeps(t.torso.distance(eye), off_aim_deg(eye, aim, t.torso), t.fog)));
        if self.held.len() >= s.max_locks as usize {
            self.building_s = 0.0;
            return;
        }
        let candidate = targets
            .iter()
            .filter(|t| !t.neutral)
            .map(|t| (t, off_aim_deg(eye, aim, t.torso)))
            .filter(|(t, off)| s.reaches(t.torso.distance(eye), *off, t.fog))
            .min_by(|a, b| {
                self.count(a.0.id)
                    .cmp(&self.count(b.0.id))
                    .then(a.1.total_cmp(&b.1))
                    .then(a.0.id.cmp(&b.0.id))
            });
        match candidate {
            Some((t, _)) => {
                self.building_s += dt_s * t.fog.clamp(0.0, 1.0);
                if self.building_s >= s.lock_s - 1e-4 {
                    self.held.push(t.id);
                    self.building_s = 0.0;
                }
            }
            None => self.building_s = 0.0,
        }
    }

    /// Locks held on `id`.
    pub fn count(&self, id: u32) -> usize {
        self.held.iter().filter(|h| **h == id).count()
    }

    /// Let go: the locks to fire on, in lock order, and a fresh start.
    pub fn release(&mut self) -> Vec<u32> {
        let held = std::mem::take(&mut self.held);
        *self = Self::default();
        held
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
    /// How much of the sensors' reach the fog leaves toward it (`fog_scale`): 1 in clear air
    /// (survey F2).
    pub fog: f32,
}

/// How much of the sensors' reach the fog leaves toward a target (survey F2), from the fog's optical
/// depth along the line to it (`world::mist::line_depth`): 1 while at least `fog_clear_share` of
/// its light gets through, falling in step with that share to `fog_min_scale` when none does.
pub fn fog_scale(cfg: &TargetingConfig, depth: f32) -> f32 {
    let through = (-depth.max(0.0)).exp();
    if !through.is_finite() || through >= cfg.fog_clear_share {
        return 1.0;
    }
    cfg.fog_min_scale + (1.0 - cfg.fog_min_scale) * through / cfg.fog_clear_share
}

/// Whether a target `range_m` away, through fog leaving `fog` of the reach (`fog_scale`), is boxed
/// and can be selected: within `max_range_m` shortened by the fog. The one boxing range check.
pub fn seen(cfg: &TargetingConfig, range_m: f32, fog: f32) -> bool {
    range_m <= cfg.max_range_m * fog.clamp(0.0, 1.0)
}

/// Whether a target `range_m` away, through fog leaving `fog` of the reach, is in missile range:
/// `lock_range_m` shortened by the fog, by the shoulder missile's own `Seeker::in_range`.
pub fn in_lock_range(cfg: &TargetingConfig, range_m: f32, fog: f32) -> bool {
    cfg.seeker(0.0).in_range(range_m, fog)
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

/// What Q remembers: the targets it picked within `cycle_cooldown_s`, and how much each matters.
#[derive(Clone, Copy)]
pub struct Recent<'a> {
    /// Picked lately, by id.
    pub ids: &'a [u32],
    /// How much a target matters, higher first (`cycle`'s second round).
    pub priority: &'a dyn Fn(u32) -> u8,
}

/// The target Q picks (owner, 2026-10-29: "Q shuold first cycle nearest object with in last 30 seconds, if all objects
/// in the neares q area have been selected it should try other objects by distance and priority", "then if those are
/// exhausted it should try nearest again to cursor", "cool down 15 seconds for objects close to q"). Never the one
/// selected now, never a neutral, never one picked lately (`recent`):
/// 1. the enemy nearest the aim within `pick_cone_deg`;
/// 2. with every one in that cone picked lately, the rest by priority, then distance from `eye`;
/// 3. with those exhausted too, the enemy nearest the aim again, wherever it is, picked lately or not (the caller starts
///    its memory over when the result is in `recent`). The one selected now when it is the only enemy.
pub fn cycle(selected: Option<u32>, targets: &[Target], eye: Vec3, aim: Vec3, cfg: &TargetingConfig, recent: Recent<'_>) -> Option<u32> {
    let enemies = || targets.iter().filter(|t| !t.neutral && Some(t.id) != selected);
    let fresh: Vec<Target> = enemies().filter(|t| !recent.ids.contains(&t.id)).copied().collect();
    let off = |t: &Target| off_aim_deg(eye, aim, t.torso);
    nearest_to_aim(eye, aim, &fresh, cfg.pick_cone_deg)
        .or_else(|| {
            fresh
                .iter()
                .min_by(|a, b| {
                    (recent.priority)(b.id)
                        .cmp(&(recent.priority)(a.id))
                        .then(a.torso.distance(eye).total_cmp(&b.torso.distance(eye)))
                        .then(a.id.cmp(&b.id))
                })
                .map(|t| t.id)
        })
        .or_else(|| {
            enemies()
                .min_by(|a, b| off(a).total_cmp(&off(b)).then(a.id.cmp(&b.id)))
                .map(|t| t.id)
        })
        .or_else(|| selected.filter(|s| targets.iter().any(|t| t.id == *s && !t.neutral)))
}

/// The selection after this step: the one kept from the last step while it is still a target, else
/// the nearest the aim within `cfg.pick_cone_deg`; with the cycle key pressed (`q`, what it remembers), `cycle`'s
/// pick. A neutral is never the result, however it was reached (survey W47), so the lock, which
/// builds only on the selection, never builds on one and no missile homes on one.
pub fn select(
    selected: Option<u32>,
    q: Option<Recent<'_>>,
    eye: Vec3,
    aim: Vec3,
    targets: &[Target],
    cfg: &TargetingConfig,
) -> Option<u32> {
    let still = selected.filter(|s| targets.iter().any(|t| t.id == *s && !t.neutral));
    if let Some(recent) = q {
        cycle(still, targets, eye, aim, cfg, recent)
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
    /// Builds while `target` is within the lock range (shortened by the fog, `fog` from `fog_scale`)
    /// and cone of the aim, `fog` times as fast as in clear air; falls back at twice the rate
    /// otherwise, restarts on a new target. True when locked.
    pub fn step(&mut self, target: Option<u32>, off_deg: f32, range_m: f32, fog: f32, cfg: &TargetingConfig, dt_s: f32) -> bool {
        if target != self.target {
            *self = Lock { target, held_s: 0.0 };
        }
        let on = target.is_some() && cfg.seeker(0.0).reaches(range_m, off_deg, fog);
        self.held_s = if on {
            (self.held_s + dt_s * fog.clamp(0.0, 1.0)).min(cfg.lock_s)
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
            fog: 1.0,
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
        let cfg = TargetingConfig::default();
        let q = |sel| cycle(sel, &ts, Vec3::ZERO, Vec3::NEG_Z, &cfg, none());
        assert_eq!(q(None), Some(2));
        assert_eq!(q(Some(2)), Some(2), "the cycle has one enemy in it");
        assert_eq!(q(Some(9)), Some(2));
    }

    #[test]
    fn a_civil_aircraft_is_never_selected_or_locked_however_the_pilot_tries() {
        // Owner, 2026-10-26, survey W47: "Never: only unguided fire can hit one".
        let cfg = TargetingConfig::default();
        let alone = [civ(9, 0.0, -600.0)];
        let (eye, aim) = (Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(select(None, None, eye, aim, &alone, &cfg), None, "the crosshair held on it");
        assert_eq!(
            select(None, Some(none()), eye, aim, &alone, &cfg),
            None,
            "Q with the crosshair on it"
        );
        assert_eq!(
            select(Some(9), None, eye, aim, &alone, &cfg),
            None,
            "a stale selection of it is dropped"
        );
        let mixed = [civ(9, 0.0, -300.0), t(2, 60.0, -400.0)];
        assert_eq!(
            select(Some(9), Some(none()), eye, aim, &mixed, &cfg),
            Some(2),
            "Q goes to the enemy"
        );
        let mut held = None;
        let mut lock = Lock::default();
        for _ in 0..300 {
            held = select(held, None, eye, aim, &alone, &cfg);
            assert!(!lock.step(held, 0.0, 600.0, 1.0, &cfg, 0.01), "no lock ever builds on an airliner");
        }
        assert_eq!(lock.target, None);
    }

    #[test]
    fn the_target_nearest_the_crosshair_is_picked() {
        let ts = [t(1, 100.0, -500.0), t(2, -10.0, -400.0), t(3, 0.0, 500.0)];
        assert_eq!(nearest_to_aim(Vec3::ZERO, Vec3::NEG_Z, &ts, 30.0), Some(2));
        assert_eq!(nearest_to_aim(Vec3::ZERO, Vec3::X, &ts, 30.0), None, "nothing within the cone");
    }

    /// Q with nothing remembered and nothing mattering more than anything else.
    fn none() -> Recent<'static> {
        Recent {
            ids: &[],
            priority: &|_| 0,
        }
    }

    /// Q's order (owner, 2026-10-29): nearest the crosshair first, skipping what it picked in the last 15 s; with the
    /// cone used up, the rest by priority then distance; with those used up too, nearest the crosshair again.
    #[test]
    fn q_takes_the_nearest_the_crosshair_then_the_rest_by_priority_and_distance_then_starts_over() {
        let cfg = TargetingConfig::default();
        // 1 and 2 in the cone (2 nearer the crosshair), 3 behind and near, 4 behind and far but it matters more.
        let ts = [t(1, 100.0, -500.0), t(2, -10.0, -400.0), t(3, 0.0, 200.0), t(4, 0.0, 900.0)];
        let (eye, aim) = (Vec3::ZERO, Vec3::NEG_Z);
        let mut picked: Vec<u32> = Vec::new();
        let mut sel = None;
        let mut order = Vec::new();
        for _ in 0..5 {
            let recent = Recent {
                ids: &picked,
                priority: &|id| u8::from(id == 4),
            };
            let p = cycle(sel, &ts, eye, aim, &cfg, recent).unwrap();
            if picked.contains(&p) {
                picked.clear();
            }
            picked.push(p);
            order.push(p);
            sel = Some(p);
        }
        assert_eq!(
            order,
            vec![2, 1, 4, 3, 2],
            "the cone by the crosshair, then priority, then distance, then over"
        );
        // A target picked lately is passed over while another in the cone is fresh.
        let recent = Recent {
            ids: &[2],
            priority: &|_| 0,
        };
        assert_eq!(cycle(None, &ts, eye, aim, &cfg, recent), Some(1));
    }

    #[test]
    fn a_lock_builds_in_a_second_in_range() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        let mut t_s = 0.0;
        while !l.step(Some(4), 3.0, 800.0, 1.0, &cfg, 0.01) {
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
            assert!(!l.step(Some(4), 0.0, 1500.0, 1.0, &cfg, 0.01));
        }
    }

    #[test]
    fn looking_away_breaks_the_lock() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        for _ in 0..150 {
            l.step(Some(4), 1.0, 800.0, 1.0, &cfg, 0.01);
        }
        assert!(l.locked(&cfg));
        assert!(!l.step(Some(4), 40.0, 800.0, 1.0, &cfg, 0.01), "40 degrees off loses it");
        assert!(!l.step(Some(9), 0.0, 800.0, 1.0, &cfg, 0.01), "a new target starts over");
    }

    #[test]
    fn fog_thinner_than_the_threshold_hides_nothing() {
        let cfg = TargetingConfig::default();
        assert_eq!(fog_scale(&cfg, 0.0), 1.0);
        // Exactly the clear share gets through: still no effect.
        assert_eq!(fog_scale(&cfg, -cfg.fog_clear_share.ln()), 1.0);
        assert!(seen(&cfg, cfg.max_range_m, fog_scale(&cfg, 0.1)));
    }

    #[test]
    fn thick_fog_shortens_the_boxing_and_lock_ranges_in_step_with_it() {
        let cfg = TargetingConfig::default();
        let mut last = 1.0;
        for k in 1..40 {
            let s = fog_scale(&cfg, k as f32 * 0.25);
            assert!(s <= last && s >= cfg.fog_min_scale, "{s}");
            last = s;
        }
        // Nothing gets through: the floor.
        assert!((fog_scale(&cfg, 1e9) - cfg.fog_min_scale).abs() < 1e-6);
        // Half the clear share gets through: the ranges are part way down.
        let half = fog_scale(&cfg, -(cfg.fog_clear_share * 0.5).ln());
        assert!((half - (cfg.fog_min_scale + (1.0 - cfg.fog_min_scale) * 0.5)).abs() < 1e-5);
        // A target at 800 m is boxed and in missile range in clear air, but not in a fog bank.
        assert!(seen(&cfg, 800.0, 1.0) && in_lock_range(&cfg, 800.0, 1.0));
        let bank = fog_scale(&cfg, 4.0);
        assert!(!in_lock_range(&cfg, 800.0, bank), "fog leaving {bank} of the reach");
        assert!(!seen(&cfg, 800.0, bank));
    }

    #[test]
    fn no_lock_builds_on_a_target_hidden_in_fog_and_a_lock_through_light_fog_is_slower() {
        let cfg = TargetingConfig::default();
        let mut l = Lock::default();
        for _ in 0..500 {
            assert!(!l.step(Some(4), 0.0, 800.0, fog_scale(&cfg, 4.0), &cfg, 0.01));
        }
        // Through fog leaving 0.8 of the reach (still in range at 800 m): it locks, later.
        let thin = fog_scale(&cfg, -(cfg.fog_clear_share * 0.73).ln());
        assert!(in_lock_range(&cfg, 800.0, thin), "{thin}");
        let mut l = Lock::default();
        let mut t_s = 0.0;
        while !l.step(Some(4), 0.0, 800.0, thin, &cfg, 0.01) {
            t_s += 0.01;
            assert!(t_s < 5.0);
        }
        assert!(t_s > cfg.lock_s * 1.15, "{t_s}");
    }

    #[test]
    fn the_shipped_targeting_is_valid() {
        assert!(TargetingConfig::default().validate().is_ok());
        let bad = TargetingConfig {
            fog_min_scale: 0.0,
            ..TargetingConfig::default()
        };
        assert!(bad.validate().is_err());
    }

    fn pod() -> Seeker {
        Seeker {
            max_locks: 6,
            lock_s: 0.35,
            range_m: 1500.0,
            cone_deg: 25.0,
            turn_deg_s: 60.0,
        }
    }

    fn hold(l: &mut Locks, s: &Seeker, ts: &[Target], seconds: f32) {
        for _ in 0..(seconds * 60.0).round() as usize {
            l.step(s, Vec3::ZERO, Vec3::NEG_Z, ts, 1.0 / 60.0);
        }
    }

    #[test]
    fn six_locks_take_two_point_one_seconds() {
        let ts = [t(1, -40.0, -600.0), t(2, 0.0, -500.0), t(3, 50.0, -700.0)];
        let mut l = Locks::default();
        hold(&mut l, &pod(), &ts, 2.08);
        assert_eq!(l.held.len(), 5, "not yet: {:?}", l.held);
        hold(&mut l, &pod(), &ts, 0.02);
        assert_eq!(l.held.len(), 6);
        for id in [1, 2, 3] {
            assert_eq!(l.count(id), 2, "two on each crab: {:?}", l.held);
        }
        assert_eq!(l.held[0], 2, "the one nearest the aim first");
    }

    #[test]
    fn a_lone_target_takes_every_lock() {
        let mut l = Locks::default();
        hold(&mut l, &pod(), &[t(4, 10.0, -800.0)], 3.0);
        assert_eq!(l.held, vec![4; 6]);
    }

    #[test]
    fn a_target_leaving_the_cone_loses_its_locks() {
        let mut l = Locks::default();
        let ts = [t(1, 0.0, -600.0), t(2, 30.0, -600.0)];
        hold(&mut l, &pod(), &ts, 1.4);
        assert_eq!((l.count(1), l.count(2)), (2, 2));
        // Crab 2 ducks out 40 degrees off the aim: past 1.5 x 25.
        let away = [t(1, 0.0, -600.0), t(2, 600.0 * 40f32.to_radians().tan(), -600.0)];
        hold(&mut l, &pod(), &away, 1.0 / 60.0);
        assert_eq!(l.count(2), 0);
        assert_eq!(l.count(1), 2);
        hold(&mut l, &pod(), &away, 0.7);
        assert_eq!(l.count(1), 4, "new locks build on what remains");
    }

    #[test]
    fn a_pod_never_locks_a_civil_aircraft() {
        let mut l = Locks::default();
        hold(&mut l, &pod(), &[civ(9, 0.0, -500.0)], 3.0);
        assert!(l.held.is_empty());
    }

    #[test]
    fn a_pod_takes_the_fog_rule_the_shoulder_missile_does() {
        // Survey F2 on multi-lock: every lock path goes through `Seeker`, so fog that keeps the
        // shoulder missile from locking keeps a pod from locking too, and thin fog slows both.
        let cfg = TargetingConfig::default();
        let bank = fog_scale(&cfg, 4.0);
        let fogged = |id, x, z, fog| Target { fog, ..t(id, x, z) };
        let mut l = Locks::default();
        hold(&mut l, &pod(), &[fogged(4, 0.0, -800.0, bank)], 3.0);
        assert!(l.held.is_empty(), "no lock through a fog bank: {:?}", l.held);
        assert_eq!(
            cfg.seeker(0.0).reaches(800.0, 0.0, bank),
            in_lock_range(&cfg, 800.0, bank),
            "one range rule"
        );
        // A target already locked that slips into the bank loses its locks.
        let mut l = Locks::default();
        hold(&mut l, &pod(), &[t(4, 0.0, -800.0)], 0.4);
        assert_eq!(l.count(4), 1);
        hold(&mut l, &pod(), &[fogged(4, 0.0, -800.0, bank)], 1.0 / 60.0);
        assert_eq!(l.count(4), 0);
        // Fog leaving 0.8 of the reach (in range at 800 m): the six locks come, a quarter slower.
        let thin = 0.8;
        let mut clear = Locks::default();
        let mut slow = Locks::default();
        hold(&mut clear, &pod(), &[t(4, 0.0, -800.0)], 2.1);
        hold(&mut slow, &pod(), &[fogged(4, 0.0, -800.0, thin)], 2.1);
        assert_eq!(clear.held.len(), 6);
        assert!(slow.held.len() < 6, "{:?}", slow.held);
        hold(&mut slow, &pod(), &[fogged(4, 0.0, -800.0, thin)], 0.6);
        assert_eq!(slow.held.len(), 6);
    }

    #[test]
    fn the_shoulder_missile_keeps_its_lock_numbers() {
        let s = TargetingConfig::default().seeker(70.0);
        assert_eq!((s.max_locks, s.lock_s, s.range_m, s.cone_deg), (1, 1.0, 1200.0, 12.0));
        let shipped: TargetingConfig =
            crate::data::load_ron(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/hud/targeting.ron")).unwrap();
        let s = shipped.seeker(70.0);
        assert_eq!((s.lock_s, s.range_m, s.cone_deg), (1.0, 1200.0, 12.0));
    }
}
