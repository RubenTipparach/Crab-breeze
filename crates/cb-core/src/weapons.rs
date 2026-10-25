//! Weapons (openspec change `weapons`): guns in the mech's data, fire groups with reload intervals,
//! and ballistic projectiles that fly under gravity and stop at the first thing they meet: a mech
//! part, a standing building cell, a tree trunk, the water or the ground. Rockets (openspec change
//! `rockets`) are the same projectiles with a motor: they fly straight. Pure.

use crate::data::{Validate, positive};
use crate::mech::PartKind;
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
    /// Fire group: 0 primary, 1 secondary.
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
    /// A rocket: its motor holds it straight for its whole flight, with no drop.
    #[serde(default)]
    pub rocket: bool,
    /// A missile's seeker: the most it turns toward its locked target, degrees a second (0 for
    /// guns and rockets; openspec change `targeting`).
    #[serde(default)]
    pub seeker_turn_deg_s: f32,
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
            rocket: false,
            seeker_turn_deg_s: 0.0,
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
            rocket: false,
            seeker_turn_deg_s: 0.0,
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
            rocket: true,
            seeker_turn_deg_s: 70.0,
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
        if self.group > 2 {
            return Err(format!("{}: groups are 0, 1 and 2 (missiles)", f("group")));
        }
        Ok(())
    }
}

/// A mech's guns' reload clocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guns {
    /// Seconds until each gun can fire again.
    pub cooldown_s: Vec<f32>,
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
/// secondary-group weapon on the secondary, every missile on the missile key.
pub fn station_pulls(specs: &[WeaponSpec], selected: usize, primary: bool, secondary: bool, missile: bool) -> Vec<bool> {
    specs
        .iter()
        .enumerate()
        .map(|(i, w)| (primary && i == selected) || (secondary && w.group == 1) || (missile && w.group == 2))
        .collect()
}

/// Every weapon of each pulled group (primary, secondary, missiles): how the enemy AI fires.
pub fn group_pulls(specs: &[WeaponSpec], pulled: [bool; 3]) -> Vec<bool> {
    specs
        .iter()
        .map(|w| pulled.get(w.group as usize).copied().unwrap_or(false))
        .collect()
}

impl Guns {
    /// Advances reloads by `dt_s`, and fires every ready, pulled gun whose arm is still on (`pulled`
    /// weapon by weapon: `station_pulls` or `group_pulls`). Returns the indices of the guns that fired.
    pub fn trigger(&mut self, specs: &[WeaponSpec], pulled: &[bool], arm_lost: impl Fn(PartKind) -> bool, dt_s: f32) -> Vec<usize> {
        self.cooldown_s.resize(specs.len(), 0.0);
        let mut fired = Vec::new();
        for (i, w) in specs.iter().enumerate() {
            self.cooldown_s[i] = (self.cooldown_s[i] - dt_s).max(0.0);
            if pulled.get(i).copied().unwrap_or(false) && self.cooldown_s[i] == 0.0 && !arm_lost(w.arm) {
                self.cooldown_s[i] = w.interval_s;
                fired.push(i);
            }
        }
        fired
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
    /// A rocket (no drop).
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
    if scene.water_at(p.x, p.z).is_some_and(|w| p.y <= w.surface_m) {
        return Some(Struck::Water);
    }
    (p.y <= scene.height_at(p.x, p.z)).then_some(Struck::Ground)
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
            shots += g.trigger(&w, &group_pulls(&w, [false, true, false]), |_| false, 1.0 / 60.0).len();
        }
        assert_eq!(shots, 4, "10 s at one shot per 2.5 s");
    }

    #[test]
    fn a_lost_arm_cannot_fire() {
        let w = default_weapons();
        let mut g = Guns::default();
        assert!(
            g.trigger(&w, &group_pulls(&w, [true, true, false]), |k| k == PartKind::ArmL, 0.016)
                .iter()
                .all(|i| w[*i].arm != PartKind::ArmL)
        );
    }

    #[test]
    fn the_primary_trigger_fires_only_the_selected_station() {
        let w = default_weapons();
        let sel = first_station(&w);
        let pulls = station_pulls(&w, sel, true, false, false);
        assert_eq!(pulls.iter().filter(|p| **p).count(), 1);
        assert!(pulls[sel]);
        let mut g = Guns::default();
        assert_eq!(g.trigger(&w, &pulls, |_| false, 0.016), vec![sel]);
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
        let from = Vec3::new(b.footprint.min.x - 150.0, ground + 4.0, wall.y);
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
        };
        let st = Structures::new(plan());
        for _ in 0..120 {
            assert!(fly(&mut s, &scene, &st, &Forest::default(), &[], 1.0 / 60.0).is_none());
        }
        assert!((s.pos.y - 5000.0).abs() < 1e-3, "{}", s.pos);
        assert!((s.pos.x - 480.0).abs() < 0.1, "{}", s.pos);
    }
}
