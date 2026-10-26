//! Weapons (openspec change `weapons`): guns in the mech's data, fire groups with reload intervals,
//! and ballistic projectiles that fly under gravity and stop at the first thing they meet: a mech
//! part, a standing building cell, a tree trunk, the water or the ground. Rockets (openspec change
//! `rockets`) are the same projectiles with a motor: they fly straight. The kinds of shot (openspec
//! change `weapon-kinds`: shells, rockets, a shotgun's pellets, a railgun's slug, a laser's beam, a
//! plasma ball) are resolved here and only here, one `match` each in `launch`, `Guns::trigger` and
//! `burst_due`, and every weapon that runs hot shares the mech's one weapon heat bank. Pure.

use crate::data::{Validate, positive};
use crate::mech::PartKind;
use crate::mech::locomotion::{Budget, BudgetUse};
use crate::rng::SimRng;
use crate::world::{Forest, Ground, Scene, Structures, TreeState};
use glam::{Vec2, Vec3};
use serde::Deserialize;

const G_M_S2: f32 = 9.81;
/// A projectile older than this is gone.
const MAX_AGE_S: f32 = 6.0;

/// A gun in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeaponSpec {
    /// Display name.
    pub name: String,
    /// The muzzle socket it fires from (`MUZZLE_L`, `MUZZLE_R`).
    pub socket: String,
    /// The arm that carries it; losing the arm loses the gun.
    pub arm: PartKind,
    /// Fire group: 0 primary (the right arm), 1 secondary (the left arm), 2 missiles, 3 back units
    /// (`weapon-kinds` section 4).
    pub group: u8,
    /// Muzzle speed.
    pub speed_m_s: f32,
    /// Least time between shots.
    pub interval_s: f32,
    /// Armour it beats, millimetres of rolled steel equivalent.
    pub penetration_mm: f32,
    /// Structure points it takes from a part it beats.
    pub damage: f32,
    /// Explosive energy on impact, megajoules (0 for solid shot).
    pub blast_mj: f32,
    /// What kind of shot it throws (openspec change `weapon-kinds`); a shell when not given.
    #[serde(default)]
    pub kind: ShotKind,
    /// The old spelling of `kind: Rocket`, still read so older data loads (`weapon-kinds` 1.1).
    /// `shot()` folds it in; a file may not give both.
    #[serde(default)]
    pub rocket: bool,
    /// Share of the mech's weapon heat bank (0 to 1) a shot adds; for a beam, a second of firing
    /// adds. Zero for a gun that does not run hot. While the bank is locked, no weapon with heat fires.
    #[serde(default)]
    pub heat: f32,
    /// A missile's seeker: the most it turns toward its locked target, degrees a second (0 for
    /// guns and rockets; openspec change `targeting`). The shoulder missile's single lock; a pod
    /// gives a whole `seeker` instead.
    #[serde(default)]
    pub seeker_turn_deg_s: f32,
    /// A multi-lock pod's seeker (openspec change `multi-lock`): it gathers up to `max_locks` while
    /// its trigger is held and fires one missile per lock on release.
    #[serde(default)]
    pub seeker: Option<crate::targeting::Seeker>,
    /// Several rounds from one pull (`signature-weapons`): the Foreman's boom cannon, the Mule's ripple.
    #[serde(default)]
    pub burst: Option<Burst>,
    /// A lobbing gun (`signature-weapons`): it lays itself to land its shell on the aim point by an arc.
    #[serde(default)]
    pub lob: Option<Lob>,
    /// The mech's signature weapon (`signature-weapons`): listed first and starred on the WEAPONS page.
    #[serde(default)]
    pub signature: bool,
    /// Rounds carried a sortie; `None` for no limit.
    #[serde(default)]
    pub rounds: Option<u32>,
}

/// The default guns: an autocannon on the right arm, a heavy cannon on the left.
pub fn default_weapons() -> Vec<WeaponSpec> {
    vec![
        WeaponSpec {
            name: "35 mm autocannon".into(),
            socket: "MUZZLE_R".into(),
            arm: PartKind::ArmR,
            group: 0,
            speed_m_s: 1100.0,
            interval_s: 0.12,
            penetration_mm: 90.0,
            damage: 12.0,
            blast_mj: 0.4,
            kind: ShotKind::Shell,
            rocket: false,
            heat: 0.0,
            seeker_turn_deg_s: 0.0,
            seeker: None,
            burst: None,
            lob: None,
            signature: false,
            rounds: None,
        },
        WeaponSpec {
            name: "120 mm cannon".into(),
            socket: "MUZZLE_L".into(),
            arm: PartKind::ArmL,
            group: 1,
            speed_m_s: 850.0,
            interval_s: 2.5,
            penetration_mm: 260.0,
            damage: 90.0,
            blast_mj: 6.0,
            kind: ShotKind::Shell,
            rocket: false,
            heat: 0.0,
            seeker_turn_deg_s: 0.0,
            seeker: None,
            burst: None,
            lob: None,
            signature: false,
            rounds: None,
        },
        WeaponSpec {
            name: "shoulder missile".into(),
            socket: "MISSILE_POD".into(),
            arm: PartKind::Torso,
            group: 2,
            speed_m_s: 260.0,
            interval_s: 1.2,
            penetration_mm: 150.0,
            damage: 45.0,
            blast_mj: 2.5,
            kind: ShotKind::Rocket,
            rocket: false,
            heat: 0.0,
            seeker_turn_deg_s: 70.0,
            seeker: None,
            burst: None,
            lob: None,
            signature: false,
            // Owner, 2026-10-25, survey G13: "40" missiles a sortie.
            rounds: Some(40),
        },
    ]
}

impl Validate for WeaponSpec {
    fn validate(&self) -> Result<(), String> {
        let f = |k: &str| format!("weapons[{}].{k}", self.name);
        positive(&f("speed_m_s"), self.speed_m_s)?;
        positive(&f("interval_s"), self.interval_s)?;
        positive(&f("penetration_mm"), self.penetration_mm)?;
        positive(&f("damage"), self.damage)?;
        crate::data::finite(&f("blast_mj"), self.blast_mj)?;
        crate::data::finite(&f("seeker_turn_deg_s"), self.seeker_turn_deg_s)?;
        crate::data::within(&f("heat"), self.heat, 0.0, 1.0)?;
        if self.group > 3 {
            return Err(format!("{}: groups are 0, 1, 2 (missiles) and 3 (back units)", f("group")));
        }
        if let Some(seeker) = &self.seeker {
            seeker.validate().map_err(|e| format!("{}: {e}", f("seeker")))?;
            if self.shot() != ShotKind::Rocket {
                return Err(format!("{}: only a rocket carries a seeker", f("seeker")));
            }
        }
        if let Some(b) = &self.burst {
            if b.count < 2 {
                return Err(format!("{}: a burst is at least 2 rounds", f("burst.count")));
            }
            positive(&f("burst.gap_s"), b.gap_s)?;
            crate::data::within(&f("burst.jitter_deg"), b.jitter_deg, 0.0, 10.0)?;
        }
        if let Some(l) = &self.lob {
            crate::data::within(&f("lob.elevation_deg"), l.elevation_deg, 5.0, 80.0)?;
            positive(&f("lob.min_m"), l.min_m)?;
            if l.max_m < l.min_m {
                return Err(format!("{}: max_m below min_m", f("lob")));
            }
            if self.shot().straight() {
                return Err(format!("{}: only a falling shell can lob", f("lob")));
            }
        }
        if self.rocket && self.kind != ShotKind::Shell {
            return Err(format!("{}: give `kind` or the old `rocket: true`, not both", f("rocket")));
        }
        self.kind.validate().map_err(|e| format!("{}: {e}", f("kind")))
    }
}

/// What a pull of the trigger throws (openspec change `weapon-kinds`). Everything after the hit is
/// the one hit path, whatever the kind.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ShotKind {
    /// A shell: flies under gravity, no drag.
    #[default]
    Shell,
    /// A rocket or a missile: its motor holds it straight, no drop.
    Rocket,
    /// A shotgun: `count` pellets a pull, spread evenly over the disc of a cone of half-angle
    /// `spread_deg` round the aim; each pellet a shell.
    Pellets {
        /// Pellets a pull.
        count: u8,
        /// The cone's half-angle, degrees.
        spread_deg: f32,
    },
    /// A railgun: the slug leaves `charge_s` after the trigger is pressed, along the aim at that
    /// moment; let go early and nothing fires and nothing heats. Falls like a shell.
    Slug {
        /// Seconds from the press to the shot.
        charge_s: f32,
    },
    /// A laser: while held, every step it burns the first thing on the aim out to `range_m`, at
    /// `damage` a second. No flight time.
    Beam {
        /// How far the beam reaches, metres.
        range_m: f32,
    },
    /// A plasma cannon: a ball that flies dead straight and bursts with its blast on contact or
    /// after `burst_m` of flight, whichever comes first.
    Plasma {
        /// Flight before it bursts in the air, metres.
        burst_m: f32,
    },
}

impl ShotKind {
    /// It flies straight, with no drop (a rocket, a plasma ball; a beam has no flight).
    pub fn straight(self) -> bool {
        matches!(self, Self::Rocket | Self::Plasma { .. } | Self::Beam { .. })
    }

    /// The speed to lead it at: a beam has no flight time, so its lead is the target itself.
    pub fn lead_speed_m_s(self, speed_m_s: f32) -> f32 {
        match self {
            Self::Beam { .. } => f32::INFINITY,
            _ => speed_m_s,
        }
    }

    fn validate(self) -> Result<(), String> {
        match self {
            Self::Shell | Self::Rocket => Ok(()),
            Self::Pellets { count, spread_deg } => {
                if count == 0 {
                    return Err("a shotgun throws at least one pellet".into());
                }
                crate::data::within("spread_deg", spread_deg, 0.0, 45.0)
            }
            Self::Slug { charge_s } => positive("charge_s", charge_s),
            Self::Beam { range_m } => positive("range_m", range_m),
            Self::Plasma { burst_m } => positive("burst_m", burst_m),
        }
    }
}

impl WeaponSpec {
    /// The kind of shot, the old `rocket: true` folded in.
    pub fn shot(&self) -> ShotKind {
        if self.rocket { ShotKind::Rocket } else { self.kind }
    }

    /// How hard its missiles turn toward their lock, degrees a second (0: it does not home).
    pub fn turn_deg_s(&self) -> f32 {
        self.seeker.map_or(self.seeker_turn_deg_s, |s| s.turn_deg_s)
    }

    /// A pod that gathers several locks while held and fires on release (`multi-lock`).
    pub fn multi_lock(&self) -> Option<crate::targeting::Seeker> {
        self.seeker.filter(|s| s.max_locks > 1)
    }
}

/// A burst: `count` rounds from one pull, `gap_s` apart, each along the aim at its own moment, the
/// aim jittered by up to `jitter_deg` from the seeded stream (`signature-weapons`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Burst {
    /// Rounds a pull.
    pub count: u8,
    /// Seconds between rounds.
    pub gap_s: f32,
    /// Each round's aim scattered within this cone, degrees (0: dead on).
    #[serde(default)]
    pub jitter_deg: f32,
}

/// A lobbing gun's lay (`signature-weapons`): a fixed elevation, the muzzle speed set for the range to
/// the aim point, within `min_m` to `max_m`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lob {
    /// The barrel's elevation, degrees above the horizon.
    pub elevation_deg: f32,
    /// Shortest range it lays for, metres.
    pub min_m: f32,
    /// Longest range it lays for, metres.
    pub max_m: f32,
}

/// The velocity that lands a lobbed shell from `muzzle` on `aim` (the range clamped to the gun's),
/// at the gun's fixed elevation, for shells flown by `fly` at fixed steps of `step_s`. Solves
/// `dy = d tan(e) - g d^2 / (2 v^2 cos^2 e)` for the speed `v`, then takes off the half step of
/// gravity that `fly`'s stepping adds, so the stepped shell lands where the smooth arc would.
/// `None` when the aim is too high above the muzzle for that elevation to reach.
pub fn lob_velocity(muzzle: Vec3, aim: Vec3, lob: &Lob, step_s: f32) -> Option<Vec3> {
    let flat = Vec2::new(aim.x - muzzle.x, aim.z - muzzle.z);
    let dir = flat.normalize_or(Vec2::new(0.0, -1.0));
    let d = flat.length().clamp(lob.min_m, lob.max_m);
    let dy = if flat.length() > lob.max_m || flat.length() < lob.min_m {
        0.0
    } else {
        aim.y - muzzle.y
    };
    let e = lob.elevation_deg.to_radians();
    let climb = d * e.tan() - dy;
    if climb <= 0.0 {
        return None;
    }
    let v = (G_M_S2 * d * d / (2.0 * e.cos().powi(2) * climb)).sqrt();
    let out = Vec3::new(dir.x * v * e.cos(), v * e.sin() - 0.5 * G_M_S2 * step_s, dir.y * v * e.cos());
    out.is_finite().then_some(out)
}

/// `dir` scattered evenly over a cone of half-angle `half_deg` by `rng`: a shotgun's pellets and a
/// ripple's rockets.
pub fn scatter(dir: Vec3, half_deg: f32, rng: &mut SimRng) -> Vec3 {
    let (a, b) = dir.any_orthonormal_pair();
    // Uniform over the disc: radius by the square root of a uniform draw.
    let r = half_deg.to_radians().tan() * rng.next_f32().sqrt();
    let t = std::f32::consts::TAU * rng.next_f32();
    (dir + (a * t.cos() + b * t.sin()) * r).normalize()
}

/// Seconds between the missiles of one salvo (`multi-lock`).
pub const SALVO_GAP_S: f32 = 0.08;

/// A shot waiting its turn: a salvo's missile on its lock, or a burst's next round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Queued {
    /// The weapon's index.
    pub weapon: usize,
    /// The locked target it homes on, if any.
    pub target: Option<u32>,
    /// Seconds until it leaves.
    pub in_s: f32,
}

/// The mech's weapon heat bank (openspec change `weapon-kinds` section 3; survey G23, owner,
/// 2026-10-26: a bank of its own, apart from the jets', refilling in 20 s).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WeaponHeatConfig {
    /// Seconds for a full bank to cool to empty (0.05 a second at 20 s).
    pub recharge_s: f32,
    /// A locked bank unlocks once it has cooled under this share.
    pub relight_heat: f32,
}

impl Default for WeaponHeatConfig {
    fn default() -> Self {
        Self {
            recharge_s: 20.0,
            relight_heat: 0.4,
        }
    }
}

impl Validate for WeaponHeatConfig {
    fn validate(&self) -> Result<(), String> {
        positive("weapon_heat.recharge_s", self.recharge_s)?;
        crate::data::within("weapon_heat.relight_heat", self.relight_heat, 0.0, 1.0)
    }
}

/// Where each shot of one pull leaves for, as velocities at `speed_m_s` round the unit aim `dir`:
/// one for most kinds, `count` for a shotgun (spread evenly over the cone's disc by `rng`), none for
/// a beam (it is traced, not flown: `trace`). The one place a pull becomes shots.
pub fn launch(kind: ShotKind, dir: Vec3, speed_m_s: f32, rng: &mut SimRng) -> Vec<Vec3> {
    match kind {
        ShotKind::Beam { .. } => Vec::new(),
        ShotKind::Pellets { count, spread_deg } => (0..count).map(|_| scatter(dir, spread_deg, rng) * speed_m_s).collect(),
        ShotKind::Shell | ShotKind::Rocket | ShotKind::Slug { .. } | ShotKind::Plasma { .. } => vec![dir * speed_m_s],
    }
}

/// The seeded stream for one pull of one mech's guns: forked by the mech and the pull's number, so a
/// golden seed throws the same pellet pattern every run (CLAUDE.md 6.3).
pub fn shot_rng(mech: u32, pull: u32) -> SimRng {
    SimRng::new(u64::from(mech)).fork("shots").fork(&pull.to_string())
}

/// A plasma ball has flown its `burst_m` and bursts in the air (straight flight: its speed times its
/// time aloft).
pub fn burst_due(kind: ShotKind, shot: &Projectile) -> bool {
    match kind {
        ShotKind::Plasma { burst_m } => shot.age_s * shot.vel.length() >= burst_m,
        _ => false,
    }
}

/// What a station is doing, for the cockpit: read from the state the shots fire from (CLAUDE.md 6.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StationState {
    /// Its arm is gone.
    Lost,
    /// No rounds left.
    Empty,
    /// The heat bank is locked and this weapon runs hot.
    Hot,
    /// A railgun charging: the share of its charge, 0 to 1.
    Charging(f32),
    /// Reloading: seconds left.
    Reloading(f32),
    /// Ready to fire.
    Ready,
}

/// A mech's guns' reload clocks, railgun charges and weapon heat bank.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guns {
    /// Seconds until each gun can fire again.
    pub cooldown_s: Vec<f32>,
    /// Rounds left in each gun (`None`: no limit), filled from its spec on the first trigger.
    pub rounds_left: Vec<Option<u32>>,
    /// Seconds each railgun has been charging (0 when not).
    pub charge_s: Vec<f32>,
    /// The weapon heat bank: share used, 0 to 1, locked when full (`weapon-kinds` section 3).
    pub heat: Budget,
    /// Pulls fired so far, numbering each pull's seeded stream (`shot_rng`).
    pub pulls: u32,
    /// Shots waiting their turn: a salvo's missiles, a burst's rounds.
    pub queue: Vec<Queued>,
    /// A multi-lock pod's locks while its trigger is held (`multi-lock`).
    pub locks: crate::targeting::Locks,
}

/// The station `step` stations on from `current` (+1 next, -1 back), wrapping, passing over any
/// `usable` refuses (a lost arm's gun); `current` if no other is usable (openspec change
/// `weapon-select`).
pub fn next_station(specs: &[WeaponSpec], current: usize, step: i32, usable: impl Fn(usize) -> bool) -> usize {
    let n = specs.len();
    if n == 0 || step == 0 {
        return current.min(n.saturating_sub(1));
    }
    let mut i = current.min(n - 1);
    for _ in 0..n {
        i = (i as i64 + step.signum() as i64).rem_euclid(n as i64) as usize;
        if usable(i) {
            return i;
        }
    }
    current.min(n - 1)
}

/// The station a mech starts with selected: its first primary-group weapon (or the first).
pub fn first_station(specs: &[WeaponSpec]) -> usize {
    specs.iter().position(|w| w.group == 0).unwrap_or(0)
}

/// What the pilot's triggers pull, weapon by weapon: the selected station on the primary, every
/// secondary-group weapon on the secondary, every missile on the missile key, every back unit on the
/// back key (`fire`: primary, secondary, missile, back).
pub fn station_pulls(specs: &[WeaponSpec], selected: usize, fire: [bool; 4]) -> Vec<bool> {
    specs
        .iter()
        .enumerate()
        .map(|(i, w)| (fire[0] && i == selected) || (w.group > 0 && fire.get(w.group as usize).copied().unwrap_or(false)))
        .collect()
}

/// Every weapon of each pulled group (primary, secondary, missiles, back units): how the enemy AI fires.
pub fn group_pulls(specs: &[WeaponSpec], pulled: [bool; 4]) -> Vec<bool> {
    specs
        .iter()
        .map(|w| pulled.get(w.group as usize).copied().unwrap_or(false))
        .collect()
}

impl Guns {
    /// Advances reloads, charges and the heat bank's cooling by `dt_s`, and fires every ready, pulled
    /// gun whose arm is still on (`pulled` weapon by weapon: `station_pulls` or `group_pulls`). A
    /// railgun fires once it has been held for its charge; a beam fires every step it is held. A
    /// weapon that runs hot holds while the bank is locked, and each shot (a beam's each second) adds
    /// its heat. Returns the indices of the guns that fired this step.
    pub fn trigger(
        &mut self,
        specs: &[WeaponSpec],
        pulled: &[bool],
        arm_lost: impl Fn(PartKind) -> bool,
        heat: &WeaponHeatConfig,
        dt_s: f32,
    ) -> Vec<usize> {
        self.cooldown_s.resize(specs.len(), 0.0);
        self.charge_s.resize(specs.len(), 0.0);
        if self.rounds_left.len() != specs.len() {
            self.rounds_left = specs.iter().map(|w| w.rounds).collect();
        }
        // The bank cools all the time (survey G23: 20 s from full to empty), and unlocks under its mark.
        self.heat
            .tick(BudgetUse::Rest, 1.0, heat.recharge_s, 0.0, 1.0 - heat.relight_heat, dt_s);
        let mut fired = Vec::new();
        for (i, w) in specs.iter().enumerate() {
            self.cooldown_s[i] = (self.cooldown_s[i] - dt_s).max(0.0);
            let loaded = self.rounds_left[i] != Some(0);
            let cool = w.heat <= 0.0 || !self.heat.locked;
            let held = pulled.get(i).copied().unwrap_or(false) && loaded && cool && !arm_lost(w.arm);
            let fires = match w.shot() {
                // A pod fires on release, through `release`, never on the pull.
                _ if w.multi_lock().is_some() => false,
                ShotKind::Beam { .. } => {
                    if held {
                        self.load_heat(w.heat * dt_s);
                    }
                    held
                }
                ShotKind::Slug { charge_s } => {
                    if held && self.cooldown_s[i] == 0.0 {
                        self.charge_s[i] += dt_s;
                    } else {
                        // Let go early: the charge is lost, and nothing fired, so nothing heats.
                        self.charge_s[i] = 0.0;
                    }
                    let ready = self.charge_s[i] >= charge_s - 1e-4;
                    if ready {
                        self.charge_s[i] = 0.0;
                    }
                    ready
                }
                _ => held && self.cooldown_s[i] == 0.0,
            };
            if !fires {
                continue;
            }
            if !matches!(w.shot(), ShotKind::Beam { .. }) {
                if let Some(n) = &mut self.rounds_left[i] {
                    *n -= 1;
                }
                self.cooldown_s[i] = w.interval_s;
                self.load_heat(w.heat);
                // A burst queues its other rounds; its interval runs from the last.
                if let Some(b) = w.burst {
                    for k in 1..b.count {
                        self.queue.push(Queued {
                            weapon: i,
                            target: None,
                            in_s: k as f32 * b.gap_s,
                        });
                    }
                    self.cooldown_s[i] += (b.count - 1) as f32 * b.gap_s;
                }
            }
            fired.push(i);
        }
        if !fired.is_empty() {
            self.pulls = self.pulls.wrapping_add(1);
        }
        fired
    }

    /// Releases a pod's salvo: one missile per lock in `targets`, `SALVO_GAP_S` apart in lock order,
    /// if the pod is ready (reloaded, loaded, its station on). Its interval runs from the last missile.
    /// True when the salvo is on its way.
    pub fn release(&mut self, specs: &[WeaponSpec], i: usize, targets: &[u32], station_lost: bool) -> bool {
        let Some(w) = specs.get(i) else { return false };
        self.cooldown_s.resize(specs.len(), 0.0);
        if self.rounds_left.len() != specs.len() {
            self.rounds_left = specs.iter().map(|w| w.rounds).collect();
        }
        if targets.is_empty() || station_lost || self.cooldown_s[i] > 0.0 || self.rounds_left[i] == Some(0) {
            return false;
        }
        for (k, t) in targets.iter().enumerate() {
            self.queue.push(Queued {
                weapon: i,
                target: Some(*t),
                in_s: k as f32 * SALVO_GAP_S,
            });
        }
        self.cooldown_s[i] = w.interval_s + (targets.len() - 1) as f32 * SALVO_GAP_S;
        true
    }

    /// The queued shots due this step, with their targets; a shot whose station is lost or whose
    /// rounds ran out is dropped (losing the arm mid-burst ends it).
    pub fn take_queued(&mut self, specs: &[WeaponSpec], arm_lost: impl Fn(PartKind) -> bool, dt_s: f32) -> Vec<(usize, Option<u32>)> {
        let mut due = Vec::new();
        let mut keep = Vec::with_capacity(self.queue.len());
        for mut q in std::mem::take(&mut self.queue) {
            q.in_s -= dt_s;
            let Some(w) = specs.get(q.weapon) else { continue };
            if arm_lost(w.arm) {
                continue;
            }
            if q.in_s > 1e-4 {
                keep.push(q);
                continue;
            }
            match self.rounds_left.get_mut(q.weapon) {
                Some(Some(0)) => continue,
                Some(Some(n)) => *n -= 1,
                _ => {}
            }
            due.push((q.weapon, q.target));
        }
        self.queue = keep;
        due
    }

    /// Adds `share` to the weapon heat bank, locking it when it fills.
    fn load_heat(&mut self, share: f32) {
        if share <= 0.0 {
            return;
        }
        self.heat.idle_s = 0.0;
        self.heat.heat = (self.heat.heat + share).min(1.0);
        if self.heat.heat >= 1.0 {
            self.heat.locked = true;
        }
    }

    /// What station `i` is doing, from the clocks the shots fire from.
    pub fn state(&self, specs: &[WeaponSpec], i: usize, arm_lost: bool) -> StationState {
        let Some(w) = specs.get(i) else { return StationState::Lost };
        let cd = self.cooldown_s.get(i).copied().unwrap_or(0.0);
        let rounds = self.rounds_left.get(i).copied().unwrap_or(w.rounds);
        let charge = self.charge_s.get(i).copied().unwrap_or(0.0);
        if arm_lost {
            StationState::Lost
        } else if rounds == Some(0) {
            StationState::Empty
        } else if w.heat > 0.0 && self.heat.locked {
            StationState::Hot
        } else if let (ShotKind::Slug { charge_s }, true) = (w.shot(), charge > 0.0) {
            StationState::Charging((charge / charge_s).clamp(0.0, 1.0))
        } else if cd > 0.05 {
            StationState::Reloading(cd)
        } else {
            StationState::Ready
        }
    }
}

/// A shot in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projectile {
    /// The mech that fired it.
    pub owner: u32,
    /// Its gun's index in the owner's weapon list.
    pub weapon: usize,
    /// Position.
    pub pos: Vec3,
    /// Velocity.
    pub vel: Vec3,
    /// Time in flight.
    pub age_s: f32,
    /// It flies straight, with no drop: a rocket or a plasma ball (`ShotKind::straight`).
    pub rocket: bool,
    /// A missile's locked target (a mech id), if any.
    pub target: Option<u32>,
}

/// A hittable part of a mech this step: a capsule from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartCapsule {
    /// The mech.
    pub mech: u32,
    /// The part's index in that mech's part list.
    pub part: usize,
    /// One end.
    pub a: Vec3,
    /// The other end.
    pub b: Vec3,
    /// Radius.
    pub radius_m: f32,
}

/// What a shot hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Struck {
    /// A mech part.
    Mech {
        /// The mech.
        mech: u32,
        /// The part's index.
        part: usize,
    },
    /// A building's standing cell.
    Building(u32),
    /// A tree trunk.
    Tree(u32),
    /// The water.
    Water,
    /// The ground.
    Ground,
}

/// Where a shot stopped and what it hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impact {
    /// What it hit.
    pub struck: Struck,
    /// Where.
    pub at: Vec3,
    /// The shot.
    pub shot: Projectile,
}

fn point_segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// What is solid at `p`, testing in order: mech parts (not those of `skip`), buildings, trees,
/// water, ground.
fn solid_at(p: Vec3, skip: Option<u32>, scene: &Scene, structures: &Structures, forest: &Forest, parts: &[PartCapsule]) -> Option<Struck> {
    for c in parts {
        if Some(c.mech) == skip {
            continue;
        }
        if point_segment_distance(p, c.a, c.b) <= c.radius_m {
            return Some(Struck::Mech {
                mech: c.mech,
                part: c.part,
            });
        }
    }
    structure_at(p, scene, structures, forest).or_else(|| terrain_at(p, scene).0)
}

/// A standing building cell or a tree trunk at `p` (cheap: a grid lookup each).
fn structure_at(p: Vec3, scene: &Scene, structures: &Structures, forest: &Forest) -> Option<Struck> {
    let p2 = Vec2::new(p.x, p.z);
    for b in scene.plan.collision.candidates(p2, 0.5) {
        if structures.buildings[b as usize].solid_at(p) {
            return Some(Struck::Building(b));
        }
    }
    for id in forest.near(p2, 1.5) {
        let Some(t) = forest.tree(id) else { continue };
        if forest.state(id) == TreeState::Standing
            && p.y < t.base.y + t.height_m * 0.8
            && Vec2::new(t.base.x, t.base.z).distance(p2) < t.trunk_radius_m + 0.6
        {
            return Some(Struck::Tree(id));
        }
    }
    None
}

/// The water or the ground at `p`, and how far `p` stands above the ground: one terrain height
/// lookup (the costly part of a hit test, about 3 us in release).
fn terrain_at(p: Vec3, scene: &Scene) -> (Option<Struck>, f32) {
    let ground = scene.height_at(p.x, p.z);
    let p2 = Vec2::new(p.x, p.z);
    let hit = if scene.plan.waters.water_at(p2, ground).is_some_and(|w| p.y <= w.surface_m) {
        Some(Struck::Water)
    } else {
        (p.y <= ground).then_some(Struck::Ground)
    };
    (hit, p.y - ground)
}

/// The first solid thing on the segment `from` to `to`, tested every `step_m`, and where: the one
/// hit test for shots in flight and for close combat's strikes (`close-combat`). Parts of `skip`
/// (the mech striking, or a shot's own mech as it leaves the muzzle) are passed through.
pub fn sweep(
    from: Vec3,
    to: Vec3,
    step_m: f32,
    skip: Option<u32>,
    scene: &Scene,
    structures: &Structures,
    forest: &Forest,
    parts: &[PartCapsule],
) -> Option<(Struck, Vec3)> {
    let steps = ((from.distance(to) / step_m.max(0.05)).ceil() as usize).max(1);
    (1..=steps).find_map(|s| {
        let p = from.lerp(to, s as f32 / steps as f32);
        solid_at(p, skip, scene, structures, forest, parts).map(|struck| (struck, p))
    })
}

/// Where the ray from `o` along the unit `d` first enters the capsule `c`, as a distance along the
/// ray (exact: the cylinder, then its end caps; after Inigo Quilez's capsule intersection). `None`
/// if it misses or the capsule is behind.
pub fn ray_capsule(o: Vec3, d: Vec3, c: &PartCapsule) -> Option<f32> {
    let ba = c.b - c.a;
    let oa = o - c.a;
    let (baba, bard, baoa, rdoa, oaoa) = (ba.dot(ba), ba.dot(d), ba.dot(oa), d.dot(oa), oa.dot(oa));
    let ra2 = c.radius_m * c.radius_m;
    let a = baba - bard * bard;
    if a > 1e-6 {
        let b = baba * rdoa - baoa * bard;
        let cc = baba * oaoa - baoa * baoa - ra2 * baba;
        let h = b * b - a * cc;
        if h < 0.0 {
            return None;
        }
        let t = (-b - h.sqrt()) / a;
        let y = baoa + t * bard;
        if y > 0.0 && y < baba {
            return (t >= 0.0).then_some(t);
        }
    }
    // The caps: the sphere at whichever end the ray meets first.
    [c.a, c.b]
        .into_iter()
        .filter_map(|end| {
            let oc = o - end;
            let b = d.dot(oc);
            let h = b * b - (oc.dot(oc) - ra2);
            (h >= 0.0).then(|| -b - h.sqrt()).filter(|t| *t >= 0.0)
        })
        .min_by(f32::total_cmp)
}

/// Spacing of a beam's march past buildings and trees, metres, and of its refine round a hit.
const BEAM_MARCH_M: f32 = 4.0;
const BEAM_REFINE_M: f32 = 0.25;
/// The steepest the ground climbs, rise over run, that a beam's march over open ground relies on.
const GROUND_SLOPE_MAX: f32 = 1.5;

/// The first thing on the ray from `from` along `dir` out to `range_m`, and where: a laser's beam
/// (`weapon-kinds` section 7) and the scope's rangefinder (`railgun-scope`). Mech parts by the exact
/// ray-capsule test (not those of `skip`); buildings and trees sampled every 4 m with a 0.25 m refine;
/// the water and the ground, whose height is the costly lookup, sampled at a stride that grows with
/// the beam's clearance over the ground (no slope steeper than 1.5 can reach the beam sooner), then
/// bisected to 0.1 m.
#[allow(clippy::too_many_arguments)]
pub fn trace(
    from: Vec3,
    dir: Vec3,
    range_m: f32,
    skip: Option<u32>,
    scene: &Scene,
    structures: &Structures,
    forest: &Forest,
    parts: &[PartCapsule],
) -> Option<(Struck, Vec3)> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO || !from.is_finite() || !range_m.is_finite() {
        return None;
    }
    let part = parts
        .iter()
        .filter(|c| Some(c.mech) != skip)
        .filter_map(|c| ray_capsule(from, dir, c).filter(|t| *t <= range_m).map(|t| (t, c)))
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.mech.cmp(&b.1.mech)).then(a.1.part.cmp(&b.1.part)));
    let reach = part.map_or(range_m, |(t, _)| t);
    // How much faster than the beam the ground can climb toward it, per metre along it.
    let closing = GROUND_SLOPE_MAX * Vec2::new(dir.x, dir.z).length() - dir.y;
    let (mut t, mut next_ground, mut clear_ground) = (0.0f32, 0.0f32, 0.0f32);
    let world = loop {
        let p = from + dir * t;
        if structure_at(p, scene, structures, forest).is_some() {
            // Refine back to the last clear sample at 0.25 m.
            let back = from + dir * (t - BEAM_MARCH_M).max(0.0);
            break sweep(back, p, BEAM_REFINE_M, skip, scene, structures, forest, &[]);
        }
        if t >= next_ground {
            let (hit, clearance) = terrain_at(p, scene);
            if hit.is_some() {
                // Bisect between the last clear ground sample and this one.
                let (mut lo, mut hi) = (clear_ground, t);
                while hi - lo > 0.1 {
                    let mid = 0.5 * (lo + hi);
                    if terrain_at(from + dir * mid, scene).0.is_some() {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                let at = from + dir * hi;
                break terrain_at(at, scene).0.or(hit).map(|s| (s, at));
            }
            clear_ground = t;
            let stride = if closing > 0.0 { clearance / closing } else { f32::INFINITY };
            next_ground = t + stride.max(BEAM_MARCH_M);
        }
        if t >= reach {
            break None;
        }
        t = (t + BEAM_MARCH_M).min(reach);
    };
    // The world was searched only as far as the part, so a world hit is always the nearer.
    world.or_else(|| {
        part.map(|(t, c)| {
            (
                Struck::Mech {
                    mech: c.mech,
                    part: c.part,
                },
                from + dir * t,
            )
        })
    })
}

/// Flies a shot one step of `dt_s`, testing its path every metre. Returns where it stopped, or
/// `None` if it flies on. A shot past its age is reported as hitting nothing (`None`, with
/// `age_s` over the limit; the caller drops it).
pub fn fly(
    shot: &mut Projectile,
    scene: &Scene,
    structures: &Structures,
    forest: &Forest,
    parts: &[PartCapsule],
    dt_s: f32,
) -> Option<Impact> {
    let from = shot.pos;
    let to = from + shot.vel * dt_s;
    // A shot cannot hit the mech that fired it in its first 0.3 s (it leaves the muzzle).
    let skip = (shot.age_s < 0.3).then_some(shot.owner);
    if let Some((struck, p)) = sweep(from, to, 1.0, skip, scene, structures, forest, parts) {
        shot.pos = p;
        return Some(Impact {
            struck,
            at: p,
            shot: *shot,
        });
    }
    shot.pos = to;
    if !shot.rocket {
        shot.vel.y -= G_M_S2 * dt_s;
    }
    shot.age_s += dt_s;
    None
}

/// Turns a missile's velocity toward `target` by at most `turn_deg_s * dt_s`, keeping its speed.
pub fn home(shot: &mut Projectile, target: Vec3, turn_deg_s: f32, dt_s: f32) {
    let speed = shot.vel.length();
    let (from, to) = (shot.vel.normalize_or_zero(), (target - shot.pos).normalize_or_zero());
    if speed <= 0.0 || to == Vec3::ZERO {
        return;
    }
    let angle = from.angle_between(to);
    let step = (turn_deg_s.to_radians() * dt_s).min(angle);
    if angle > 1e-5 {
        let axis = from.cross(to).normalize_or(Vec3::Y);
        shot.vel = glam::Quat::from_axis_angle(axis, step) * from * speed;
    }
}

/// True when a shot has flown too long.
pub fn spent(shot: &Projectile) -> bool {
    shot.age_s > MAX_AGE_S || !shot.pos.is_finite()
}

/// Where to aim a shot at `speed_m_s` from `from` to meet a target at `target` moving at
/// `target_vel` (constant velocity; with `drop`, the gravity drop for the flight time is added, as
/// for a shell; a rocket flies straight).
pub fn lead(from: Vec3, target: Vec3, target_vel: Vec3, speed_m_s: f32, drop: bool) -> Vec3 {
    let mut t = from.distance(target) / speed_m_s;
    for _ in 0..4 {
        let p = target + target_vel * t;
        t = from.distance(p) / speed_m_s;
    }
    let fall = if drop { 0.5 * G_M_S2 * t * t } else { 0.0 };
    target + target_vel * t + Vec3::Y * fall
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use crate::world::{Deformation, WorldConfig, WorldPlan, generate};
    use std::sync::OnceLock;

    fn heat() -> WeaponHeatConfig {
        WeaponHeatConfig::default()
    }

    fn plan() -> &'static WorldPlan {
        static P: OnceLock<WorldPlan> = OnceLock::new();
        P.get_or_init(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            generate(42, &load_ron::<WorldConfig>(&p).unwrap())
        })
    }

    #[test]
    fn a_cannon_cannot_fire_faster_than_its_interval() {
        let w = default_weapons();
        let mut g = Guns::default();
        let mut shots = 0;
        for _ in 0..600 {
            shots += g
                .trigger(&w, &group_pulls(&w, [false, true, false, false]), |_| false, &heat(), 1.0 / 60.0)
                .len();
        }
        assert_eq!(shots, 4, "10 s at one shot per 2.5 s");
    }

    #[test]
    fn a_lost_arm_cannot_fire() {
        let w = default_weapons();
        let mut g = Guns::default();
        assert!(
            g.trigger(
                &w,
                &group_pulls(&w, [true, true, false, false]),
                |k| k == PartKind::ArmL,
                &heat(),
                0.016
            )
            .iter()
            .all(|i| w[*i].arm != PartKind::ArmL)
        );
    }

    #[test]
    fn the_missiles_run_out_after_forty() {
        let w = default_weapons();
        let m = w.iter().position(|w| w.group == 2).unwrap();
        let mut g = Guns::default();
        let mut shots = 0;
        for _ in 0..(60 * 120) {
            shots += g
                .trigger(
                    &w,
                    &station_pulls(&w, 0, [false, false, true, false]),
                    |_| false,
                    &heat(),
                    1.0 / 60.0,
                )
                .len();
        }
        assert_eq!(shots, 40);
        assert_eq!(g.rounds_left[m], Some(0));
    }

    #[test]
    fn the_primary_trigger_fires_only_the_selected_station() {
        let w = default_weapons();
        let sel = first_station(&w);
        let pulls = station_pulls(&w, sel, [true, false, false, false]);
        assert_eq!(pulls.iter().filter(|p| **p).count(), 1);
        assert!(pulls[sel]);
        let mut g = Guns::default();
        assert_eq!(g.trigger(&w, &pulls, |_| false, &heat(), 0.016), vec![sel]);
    }

    #[test]
    fn stepping_the_selection_wraps_and_skips_a_lost_arms_gun() {
        let w = default_weapons();
        assert!(w.len() >= 2);
        let n = w.len();
        assert_eq!(next_station(&w, n - 1, 1, |_| true), 0, "wraps forward");
        assert_eq!(next_station(&w, 0, -1, |_| true), n - 1, "wraps back");
        let skip = 1 % n;
        let after = next_station(&w, 0, 1, |i| i != skip);
        assert_ne!(after, skip);
        assert_eq!(next_station(&w, 0, 1, |i| i == 0), 0, "nothing else usable: it stays");
    }

    #[test]
    fn a_shell_falls_under_gravity() {
        let mut s = Projectile {
            owner: 0,
            weapon: 0,
            pos: Vec3::new(0.0, 5000.0, 0.0),
            vel: Vec3::new(800.0, 0.0, 0.0),
            age_s: 0.0,
            rocket: false,
            target: None,
        };
        let dug = Deformation::new(4096.0);
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let st = Structures::new(plan());
        for _ in 0..60 {
            fly(&mut s, &scene, &st, &Forest::default(), &[], 1.0 / 60.0);
        }
        assert!((s.vel.y + 9.81).abs() < 1e-3);
        assert!(s.pos.y < 5000.0 - 4.0);
    }

    #[test]
    fn a_shell_stops_at_the_first_thing_it_hits() {
        let b = &plan().buildings[0];
        let ground = plan().cities[b.city as usize].ground_m;
        let wall = b.footprint.centre();
        let from = Vec3::new(b.footprint.min.x - 150.0, ground + 12.0, wall.y);
        let mech_in_the_way = PartCapsule {
            mech: 7,
            part: 2,
            a: from + Vec3::new(60.0, -5.0, 0.0),
            b: from + Vec3::new(60.0, 5.0, 0.0),
            radius_m: 1.5,
        };
        let mut s = Projectile {
            owner: 0,
            weapon: 1,
            pos: from,
            vel: Vec3::new(850.0, 0.0, 0.0),
            age_s: 0.0,
            rocket: false,
            target: None,
        };
        let dug = Deformation::new(4096.0);
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let st = Structures::new(plan());
        let hit = (0..60)
            .find_map(|_| fly(&mut s, &scene, &st, &Forest::default(), &[mech_in_the_way], 1.0 / 60.0))
            .expect("a hit");
        assert_eq!(hit.struck, Struck::Mech { mech: 7, part: 2 });
        let mut s2 = Projectile {
            pos: from,
            vel: Vec3::new(850.0, 0.0, 0.0),
            ..s
        };
        s2.age_s = 0.0;
        let hit2 = (0..60)
            .find_map(|_| fly(&mut s2, &scene, &st, &Forest::default(), &[], 1.0 / 60.0))
            .expect("a hit");
        assert!(matches!(hit2.struck, Struck::Building(_) | Struck::Ground), "{:?}", hit2.struck);
    }

    #[test]
    fn leading_a_crossing_target_aims_ahead_of_it() {
        let aim = lead(Vec3::ZERO, Vec3::new(0.0, 0.0, -500.0), Vec3::new(10.0, 0.0, 0.0), 1000.0, true);
        assert!(aim.x > 4.0 && aim.x < 6.0, "{aim}");
        assert!(aim.y > 0.0, "and above it for the drop");
    }

    #[test]
    fn a_missile_turns_toward_its_target_no_faster_than_its_seeker() {
        let mut m = Projectile {
            owner: 0,
            weapon: 2,
            pos: Vec3::ZERO,
            vel: Vec3::new(0.0, 0.0, -260.0),
            age_s: 0.0,
            rocket: true,
            target: Some(5),
        };
        let target = Vec3::new(600.0, 0.0, 0.0);
        home(&mut m, target, 70.0, 0.1);
        let turned = Vec3::NEG_Z.angle_between(m.vel.normalize()).to_degrees();
        assert!((turned - 7.0).abs() < 0.01, "7 degrees in 0.1 s at 70 deg/s, got {turned}");
        assert!((m.vel.length() - 260.0).abs() < 0.01, "speed kept");
        assert!(m.vel.x > 0.0, "toward the target");
        for _ in 0..100 {
            home(&mut m, target, 70.0, 0.1);
        }
        assert!(m.vel.normalize().dot(Vec3::X) > 0.9999, "on it, and no overshoot");
    }

    #[test]
    fn an_unlocked_missile_flies_straight() {
        let mut m = Projectile {
            owner: 0,
            weapon: 2,
            pos: Vec3::new(0.0, 5000.0, 0.0),
            vel: Vec3::new(260.0, 0.0, 0.0),
            age_s: 0.0,
            rocket: true,
            target: None,
        };
        let dug = Deformation::new(4096.0);
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let st = Structures::new(plan());
        for _ in 0..60 {
            fly(&mut m, &scene, &st, &Forest::default(), &[], 1.0 / 60.0);
        }
        assert!((m.pos - Vec3::new(260.0, 5000.0, 0.0)).length() < 0.1, "{}", m.pos);
    }

    #[test]
    fn a_rocket_is_led_without_drop() {
        let aim = lead(Vec3::ZERO, Vec3::new(0.0, 0.0, -500.0), Vec3::ZERO, 240.0, false);
        assert_eq!(aim, Vec3::new(0.0, 0.0, -500.0));
    }

    #[test]
    fn a_rocket_flies_straight() {
        let mut s = Projectile {
            owner: 0,
            weapon: 0,
            pos: Vec3::new(0.0, 5000.0, 0.0),
            vel: Vec3::new(240.0, 0.0, 0.0),
            age_s: 0.0,
            rocket: true,
            target: None,
        };
        let dug = Deformation::new(4096.0);
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let st = Structures::new(plan());
        for _ in 0..120 {
            assert!(fly(&mut s, &scene, &st, &Forest::default(), &[], 1.0 / 60.0).is_none());
        }
        assert!((s.pos.y - 5000.0).abs() < 1e-3, "{}", s.pos);
        assert!((s.pos.x - 480.0).abs() < 0.1, "{}", s.pos);
    }

    fn gun(kind: ShotKind, heat_share: f32) -> WeaponSpec {
        WeaponSpec {
            name: format!("{kind:?}"),
            kind,
            heat: heat_share,
            ..default_weapons()[0].clone()
        }
    }

    fn open_air() -> (Deformation, Structures) {
        (Deformation::new(4096.0), Structures::new(plan()))
    }

    const STEP: f32 = 1.0 / 60.0;

    #[test]
    fn a_spec_with_an_unknown_kind_field_is_refused() {
        let text = r#"(name: "x", socket: "MUZZLE_R", arm: ArmR, group: 0, speed_m_s: 600.0, interval_s: 1.0,
            penetration_mm: 40.0, damage: 9.0, blast_mj: 0.0, kind: Pellets(count: 12, spread_deg: 1.5, choke: 2.0))"#;
        assert!(crate::data::parse_ron::<WeaponSpec>(text, std::path::Path::new("w.ron")).is_err());
        let good = text.replace(", choke: 2.0", "");
        let w = crate::data::parse_ron::<WeaponSpec>(&good, std::path::Path::new("w.ron")).expect("loads");
        assert_eq!(
            w.shot(),
            ShotKind::Pellets {
                count: 12,
                spread_deg: 1.5
            }
        );
    }

    #[test]
    fn old_rocket_data_loads_as_kind_rocket() {
        let text = r#"(name: "pod", socket: "MUZZLE_R", arm: ArmR, group: 0, speed_m_s: 240.0, interval_s: 1.6,
            penetration_mm: 120.0, damage: 30.0, blast_mj: 2.0, rocket: true)"#;
        let w = crate::data::parse_ron::<WeaponSpec>(text, std::path::Path::new("w.ron")).expect("loads");
        assert_eq!(w.shot(), ShotKind::Rocket);
        assert!(w.shot().straight());
        let both = WeaponSpec {
            kind: ShotKind::Beam { range_m: 100.0 },
            ..w
        };
        assert!(both.validate().is_err(), "both spellings at once are refused");
    }

    /// Pellets of one pull that cross a disc of `radius_m` square to the aim `range_m` out.
    fn pellets_on(range_m: f32, pulls: u32) -> f32 {
        let kind = ShotKind::Pellets {
            count: 12,
            spread_deg: 1.5,
        };
        let mut on = 0;
        for pull in 0..pulls {
            for v in launch(kind, Vec3::NEG_Z, 600.0, &mut shot_rng(3, pull)) {
                let d = v.normalize();
                let at = d * (range_m / -d.z);
                if Vec2::new(at.x, at.y).length() <= 3.0 {
                    on += 1;
                }
            }
        }
        on as f32 / pulls as f32
    }

    #[test]
    fn a_shotgun_puts_all_pellets_on_a_torso_at_100_m() {
        assert_eq!(pellets_on(100.0, 100), 12.0);
    }

    #[test]
    fn a_shotgun_mostly_misses_at_400_m() {
        let n = pellets_on(400.0, 100);
        assert!(n < 2.0, "{n} pellets a shot at 400 m");
        let mid = pellets_on(200.0, 100);
        assert!((2.5..5.5).contains(&mid), "about a third at 200 m: {mid}");
    }

    #[test]
    fn the_same_pull_throws_the_same_pattern() {
        let kind = ShotKind::Pellets {
            count: 12,
            spread_deg: 1.5,
        };
        assert_eq!(
            launch(kind, Vec3::X, 600.0, &mut shot_rng(9, 4)),
            launch(kind, Vec3::X, 600.0, &mut shot_rng(9, 4))
        );
        assert_ne!(
            launch(kind, Vec3::X, 600.0, &mut shot_rng(9, 4)),
            launch(kind, Vec3::X, 600.0, &mut shot_rng(9, 5))
        );
    }

    #[test]
    fn releasing_a_charging_railgun_fires_nothing() {
        let w = vec![gun(ShotKind::Slug { charge_s: 1.0 }, 0.35)];
        let mut g = Guns::default();
        for _ in 0..36 {
            assert!(g.trigger(&w, &[true], |_| false, &heat(), STEP).is_empty());
        }
        assert!(g.trigger(&w, &[false], |_| false, &heat(), STEP).is_empty());
        assert_eq!(g.heat.heat, 0.0, "nothing fired, nothing heated");
        assert_eq!(g.charge_s[0], 0.0, "the charge is lost");
    }

    #[test]
    fn a_railgun_fires_one_second_after_the_press() {
        let w = vec![gun(ShotKind::Slug { charge_s: 1.0 }, 0.35)];
        let mut g = Guns::default();
        let step = (0..120).find(|_| !g.trigger(&w, &[true], |_| false, &heat(), STEP).is_empty());
        assert_eq!(step, Some(59), "on the 60th step: 1.0 s");
        assert!((g.heat.heat - 0.35).abs() < 1e-3, "{}", g.heat.heat);
    }

    #[test]
    fn plasma_bursts_at_650_m_in_open_air() {
        let w = gun(ShotKind::Plasma { burst_m: 650.0 }, 0.2);
        let mut s = Projectile {
            owner: 0,
            weapon: 0,
            pos: Vec3::new(0.0, 5000.0, 0.0),
            vel: launch(w.shot(), Vec3::X, 180.0, &mut shot_rng(0, 0))[0],
            age_s: 0.0,
            rocket: w.shot().straight(),
            target: None,
        };
        let (dug, st) = open_air();
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let mut steps = 0;
        while !burst_due(w.shot(), &s) {
            assert!(
                fly(&mut s, &scene, &st, &Forest::default(), &[], STEP).is_none(),
                "nothing in the sky"
            );
            steps += 1;
            assert!(steps < 400);
        }
        assert!((s.pos.x - 650.0).abs() < 3.1, "bursts at 650 m: {}", s.pos);
        assert!((s.pos.y - 5000.0).abs() < 1e-3, "no drop");
    }

    #[test]
    fn the_laser_fills_the_bank_in_five_seconds() {
        let w = vec![gun(ShotKind::Beam { range_m: 1500.0 }, 0.25)];
        let mut g = Guns::default();
        let mut steps = 0;
        while !g.heat.locked {
            assert_eq!(g.trigger(&w, &[true], |_| false, &heat(), STEP), vec![0], "burns every held step");
            steps += 1;
            assert!(steps < 600);
        }
        let s = steps as f32 * STEP;
        assert!((s - 5.0).abs() < 0.05, "0.25 in, 0.05 out a second: {s} s");
    }

    #[test]
    fn a_beam_cuts_out_when_the_bank_fills() {
        let w = vec![gun(ShotKind::Beam { range_m: 1500.0 }, 0.25)];
        let mut g = Guns {
            heat: Budget {
                heat: 0.999,
                ..Budget::default()
            },
            ..Guns::default()
        };
        g.trigger(&w, &[true], |_| false, &heat(), STEP);
        assert!(g.heat.locked);
        assert!(g.trigger(&w, &[true], |_| false, &heat(), STEP).is_empty(), "cut out mid-beam");
        assert_eq!(g.state(&w, 0, false), StationState::Hot);
    }

    #[test]
    fn a_rail_shot_into_a_hot_bank_locks_every_heat_weapon() {
        let w = vec![
            gun(ShotKind::Slug { charge_s: 1.0 }, 0.35),
            gun(ShotKind::Beam { range_m: 1500.0 }, 0.25),
            gun(ShotKind::Plasma { burst_m: 650.0 }, 0.2),
            gun(ShotKind::Shell, 0.0),
        ];
        let mut g = Guns {
            charge_s: vec![1.0, 0.0, 0.0, 0.0],
            heat: Budget {
                heat: 0.8,
                ..Budget::default()
            },
            ..Guns::default()
        };
        assert_eq!(g.trigger(&w, &[true, false, false, false], |_| false, &heat(), STEP), vec![0]);
        assert!(g.heat.locked, "the bank fills and locks");
        g.cooldown_s[0] = 0.0;
        let mut steps = 0;
        let fired = g.trigger(&w, &[true, true, true, true], |_| false, &heat(), STEP);
        assert_eq!(fired, vec![3], "only the cold gun fires");
        while g.heat.locked {
            g.trigger(&w, &[false, false, false, false], |_| false, &heat(), STEP);
            steps += 1;
            assert!(steps < 2000);
        }
        assert!(g.heat.heat <= 0.4 + 1e-3, "unlocked under 0.4: {}", g.heat.heat);
        let fired = g.trigger(&w, &[false, true, true, false], |_| false, &heat(), STEP);
        assert_eq!(fired, vec![1, 2], "the laser and the plasma fire again");
        let s = (steps + 1) as f32 * STEP;
        assert!((s - 12.0).abs() < 0.1, "1.0 to 0.4 at 0.05 a second: {s} s");
    }

    #[test]
    fn the_weapon_bank_refills_in_twenty_seconds() {
        let w = vec![gun(ShotKind::Shell, 0.0)];
        let mut g = Guns {
            heat: Budget {
                heat: 1.0,
                ..Budget::default()
            },
            ..Guns::default()
        };
        for _ in 0..(20 * 60 - 1) {
            g.trigger(&w, &[false], |_| false, &heat(), STEP);
        }
        assert!(g.heat.heat > 0.0);
        g.trigger(&w, &[false], |_| false, &heat(), STEP);
        assert!(g.heat.heat < 1e-3, "{}", g.heat.heat);
    }

    #[test]
    fn two_seconds_of_laser_on_a_torso_takes_120() {
        let w = vec![WeaponSpec {
            damage: 60.0,
            penetration_mm: 120.0,
            ..gun(ShotKind::Beam { range_m: 1500.0 }, 0.25)
        }];
        let mut g = Guns::default();
        let mut taken = 0.0;
        for _ in 0..120 {
            for i in g.trigger(&w, &[true], |_| false, &heat(), STEP) {
                // The hit rule (mech-damage): it beats 100 mm, so the part takes it.
                assert!(w[i].penetration_mm > 100.0);
                taken += w[i].damage * STEP;
            }
        }
        assert!((taken - 120.0).abs() < 0.01, "{taken}");
    }

    #[test]
    fn a_ray_enters_a_capsule_at_its_surface() {
        let c = PartCapsule {
            mech: 1,
            part: 0,
            a: Vec3::new(0.0, -5.0, -100.0),
            b: Vec3::new(0.0, 5.0, -100.0),
            radius_m: 3.0,
        };
        let t = ray_capsule(Vec3::ZERO, Vec3::NEG_Z, &c).expect("a hit");
        assert!((t - 97.0).abs() < 1e-3, "{t}");
        assert_eq!(ray_capsule(Vec3::new(3.5, 0.0, 0.0), Vec3::NEG_Z, &c), None, "passes beside");
        assert_eq!(ray_capsule(Vec3::ZERO, Vec3::Z, &c), None, "behind");
        let cap = ray_capsule(Vec3::new(0.0, 7.0, 0.0), Vec3::NEG_Z, &c).expect("the top cap");
        assert!((cap - (100.0 - 5.0f32.sqrt())).abs() < 1e-2, "{cap}");
    }

    #[test]
    fn a_beam_stops_on_the_first_part_and_passes_its_own() {
        let (dug, st) = open_air();
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let from = Vec3::new(0.0, 5000.0, 0.0);
        let at = |mech: u32, z: f32| PartCapsule {
            mech,
            part: 2,
            a: from + Vec3::new(0.0, -5.0, z),
            b: from + Vec3::new(0.0, 5.0, z),
            radius_m: 2.0,
        };
        let parts = [at(0, -1.0), at(9, -800.0), at(7, -300.0)];
        let hit = trace(from, Vec3::NEG_Z, 1500.0, Some(0), &scene, &st, &Forest::default(), &parts).expect("a hit");
        assert_eq!(hit.0, Struck::Mech { mech: 7, part: 2 });
        assert!((hit.1.z + 298.0).abs() < 1e-2, "{}", hit.1);
        assert!(
            trace(from, Vec3::NEG_Z, 200.0, Some(0), &scene, &st, &Forest::default(), &parts).is_none(),
            "out of reach"
        );
    }

    #[test]
    fn a_beam_stops_on_the_ground_before_a_part_behind_it() {
        let (dug, st) = open_air();
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let ground = scene.height_at(0.0, 0.0);
        let from = Vec3::new(0.0, ground + 50.0, 0.0);
        let dir = Vec3::new(0.0, -1.0, -1.0).normalize();
        let behind = PartCapsule {
            mech: 4,
            part: 0,
            a: from + dir * 400.0 + Vec3::Y * -5.0,
            b: from + dir * 400.0 + Vec3::Y * 5.0,
            radius_m: 2.0,
        };
        let hit = trace(from, dir, 1500.0, None, &scene, &st, &Forest::default(), &[behind]).expect("a hit");
        assert_ne!(hit.0, Struck::Mech { mech: 4, part: 0 });
        assert!(from.distance(hit.1) < 400.0);
    }

    /// The beam's cost a fixed step against its 0.1 ms budget (`weapon-kinds` section 7): a measurement
    /// instrument, run in release (`cargo test --release -p cb-core -- --ignored a_beam_trace_costs --nocapture`).
    /// Measured 2026-10-26 on an Apple M4 Max in release, CPU only: 0.113 ms, 30 m over a city; the terrain height
    /// lookup (3.3 us) is most of it. It guards against regressions at 0.15 ms.
    #[test]
    #[ignore]
    fn a_beam_trace_costs_about_a_tenth_of_a_millisecond() {
        let (dug, st) = open_air();
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: Some(&st),
            platforms: &[],
            interiors: None,
        };
        let b = &plan().buildings[0];
        let ground = plan().cities[b.city as usize].ground_m;
        let c = b.footprint.centre();
        let from = Vec3::new(c.x - 1400.0, ground + 30.0, c.y);
        let parts: Vec<PartCapsule> = (0..40)
            .map(|i| PartCapsule {
                mech: i,
                part: 0,
                a: from + Vec3::new(i as f32 * 30.0, 0.0, 300.0),
                b: from + Vec3::new(i as f32 * 30.0, 10.0, 300.0),
                radius_m: 2.0,
            })
            .collect();
        let n = 2000;
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let dir = Vec3::new(1.0, 0.0, (i % 7) as f32 * 0.001).normalize();
            std::hint::black_box(trace(from, dir, 1500.0, None, &scene, &st, &Forest::default(), &parts));
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!("beam trace: {ms:.4} ms a step (budget 0.1 ms)");
        assert!(ms < 0.15, "{ms} ms");
    }

    fn pod() -> WeaponSpec {
        WeaponSpec {
            group: 2,
            interval_s: 8.0,
            seeker: Some(crate::targeting::Seeker {
                max_locks: 6,
                lock_s: 0.35,
                range_m: 1500.0,
                cone_deg: 25.0,
                turn_deg_s: 60.0,
            }),
            rounds: Some(24),
            ..gun(ShotKind::Rocket, 0.0)
        }
    }

    #[test]
    fn a_pod_fires_nothing_on_the_pull() {
        let w = vec![pod()];
        let mut g = Guns::default();
        for _ in 0..120 {
            assert!(g.trigger(&w, &[true], |_| false, &heat(), STEP).is_empty());
        }
    }

    #[test]
    fn a_salvo_leaves_a_missile_per_lock_and_reloads_for_eight_seconds() {
        let w = vec![pod()];
        let mut g = Guns::default();
        assert!(g.release(&w, 0, &[5, 6, 5], false));
        let mut out = Vec::new();
        let mut t = 0.0;
        while out.len() < 3 {
            for (i, target) in g.take_queued(&w, |_| false, STEP) {
                out.push((i, target, t));
            }
            t += STEP;
            assert!(t < 1.0);
        }
        assert_eq!(
            out.iter().map(|o| o.1).collect::<Vec<_>>(),
            vec![Some(5), Some(6), Some(5)],
            "in lock order"
        );
        assert!(out[2].2 <= 0.16 + 1e-4, "within 0.16 s: {}", out[2].2);
        assert_eq!(g.rounds_left[0], Some(21));
        assert!(!g.release(&w, 0, &[5], false), "reloading");
        let mut wait = 0.0;
        while g.cooldown_s[0] > 0.0 {
            g.trigger(&w, &[false], |_| false, &heat(), STEP);
            wait += STEP;
        }
        assert!((wait - 8.16).abs() < 0.05, "8 s from the last missile: {wait}");
        assert!(g.release(&w, 0, &[5], false));
    }

    #[test]
    fn a_lost_station_drops_its_queued_shots() {
        let w = vec![pod()];
        let mut g = Guns::default();
        g.release(&w, 0, &[1, 2, 3], false);
        assert_eq!(g.take_queued(&w, |_| false, STEP).len(), 1);
        assert!(g.take_queued(&w, |k| k == w[0].arm, 0.2).is_empty());
        assert!(g.queue.is_empty());
    }

    #[test]
    fn a_burst_fires_three_rounds_and_reloads_from_the_last() {
        let w = vec![WeaponSpec {
            interval_s: 4.0,
            burst: Some(Burst {
                count: 3,
                gap_s: 0.2,
                jitter_deg: 0.0,
            }),
            ..gun(ShotKind::Shell, 0.0)
        }];
        let mut g = Guns::default();
        let mut times = Vec::new();
        for n in 0..(60 * 6) {
            let t = n as f32 * STEP;
            let now = g.trigger(&w, &[true], |_| false, &heat(), STEP);
            let queued = g.take_queued(&w, |_| false, STEP);
            for _ in now.iter().map(|_| ()).chain(queued.iter().map(|_| ())) {
                times.push(t);
            }
        }
        assert!(times.len() >= 4, "{times:?}");
        assert!(
            (times[1] - times[0] - 0.2).abs() < 0.02 && (times[2] - times[1] - 0.2).abs() < 0.02,
            "{times:?}"
        );
        assert!((times[3] - times[2] - 4.0).abs() < 0.02, "4.0 s after the third: {times:?}");
    }

    #[test]
    fn losing_the_arm_mid_burst_ends_it() {
        let w = vec![WeaponSpec {
            burst: Some(Burst {
                count: 3,
                gap_s: 0.2,
                jitter_deg: 0.0,
            }),
            ..gun(ShotKind::Shell, 0.0)
        }];
        let mut g = Guns::default();
        assert_eq!(g.trigger(&w, &[true], |_| false, &heat(), STEP), vec![0]);
        assert!(g.take_queued(&w, |_| true, 1.0).is_empty());
    }

    /// Where a lobbed shell from `muzzle` comes down through the aim's height, flown by `fly`.
    fn lob_lands(muzzle: Vec3, aim: Vec3, lob: &Lob) -> (Vec3, f32, f32) {
        let (dug, st) = open_air();
        let scene = Scene {
            plan: plan(),
            dug: &dug,
            trees: None,
            structures: None,
            platforms: &[],
            interiors: None,
        };
        let mut s = Projectile {
            owner: 0,
            weapon: 0,
            pos: muzzle,
            vel: lob_velocity(muzzle, aim, lob, STEP).expect("reachable"),
            age_s: 0.0,
            rocket: false,
            target: None,
        };
        let mut apex: f32 = muzzle.y;
        loop {
            let before = s.pos;
            assert!(fly(&mut s, &scene, &st, &Forest::default(), &[], STEP).is_none());
            apex = apex.max(s.pos.y);
            if s.vel.y < 0.0 && s.pos.y <= aim.y {
                let k = (before.y - aim.y) / (before.y - s.pos.y);
                return (before.lerp(s.pos, k), apex - muzzle.y, s.age_s);
            }
            assert!(s.age_s < 30.0);
        }
    }

    #[test]
    fn a_lobbed_grenade_lands_on_the_aim_point() {
        // The Sapper's grenade launcher (survey G21): 30 degrees, 60 to 600 m.
        let lob = Lob {
            elevation_deg: 30.0,
            min_m: 60.0,
            max_m: 600.0,
        };
        let muzzle = Vec3::new(0.0, 5020.0, 0.0);
        for d in [100.0, 300.0, 600.0] {
            let aim = Vec3::new(d * 0.6, 5000.0, -d * 0.8);
            let (at, apex, t) = lob_lands(muzzle, aim, &lob);
            assert!(at.distance(aim) < 1.0, "{d} m: landed {} m off", at.distance(aim));
            assert!(apex > d * 30f32.to_radians().tan() / 4.0 - 21.0, "{d} m: apex {apex}");
            assert!(t < 9.5, "{d} m in {t} s");
        }
    }

    #[test]
    fn a_lob_beyond_its_range_falls_short_at_its_longest() {
        let lob = Lob {
            elevation_deg: 30.0,
            min_m: 60.0,
            max_m: 600.0,
        };
        let muzzle = Vec3::new(0.0, 5000.0, 0.0);
        let (at, _, _) = lob_lands(muzzle, Vec3::new(1500.0, 5000.0, 0.0), &lob);
        assert!((at.x - 600.0).abs() < 1.0, "{at}");
        assert!(
            lob_velocity(muzzle, Vec3::new(100.0, 5100.0, 0.0), &lob, STEP).is_none(),
            "too steep to reach"
        );
    }
}
