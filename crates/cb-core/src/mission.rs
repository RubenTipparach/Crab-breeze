//! The city defence mission (openspec change `city-defence`): three waves of raiders, one per city,
//! won by destroying all of them and lost when the attacked city passes half damage, a wave outlasts
//! its clock, or the pilot's mech is destroyed. The rules, the city damage number the cockpit
//! shows, where raiders appear and what each raider goes for. Pure: `cb-app` feeds it what the
//! simulation knows each fixed step and acts on what it says.

use crate::ai::Skill;
use crate::data::{Validate, finite, positive, within};
use crate::rng::SimRng;
use crate::weapons::WeaponSpec;
use crate::world::{Ground, Structures, WorldPlan};
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::f32::consts::PI;

/// One kind of raider; a wave cycles through the kinds in order.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RaiderSpec {
    /// Its mech's data file, relative to `data/`.
    pub mech: String,
    /// Hull colour override (sRGB 0..1).
    #[serde(default)]
    pub tint: Option<[f32; 3]>,
    /// Gun override; `None` keeps the mech's own.
    #[serde(default)]
    pub weapons: Option<Vec<WeaponSpec>>,
    /// Scales every part's armour.
    #[serde(default = "one")]
    pub armour_scale: f32,
    /// Its pilot.
    #[serde(default)]
    pub skill: Skill,
}

fn one() -> f32 {
    1.0
}

/// `data/missions/defence.ron`: every number the mission uses.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DefenceConfig {
    /// The city each wave attacks, by city index (largest first), in wave order.
    pub cities: Vec<u32>,
    /// Raiders in a wave.
    pub raiders_per_wave: u32,
    /// Time a wave has to be destroyed in, from when it appears.
    pub wave_time_s: f32,
    /// The attacked city's damage (0..1) past which the mission is lost.
    pub lose_damage: f32,
    /// Quiet time before each wave appears.
    pub lull_s: f32,
    /// How far beyond the city square's corners raiders appear.
    pub spawn_distance_m: f32,
    /// Width of the arc they appear on, degrees.
    pub spawn_arc_deg: f32,
    /// Random shift of each spawn point, at most.
    pub spawn_jitter_m: f32,
    /// Spawn points stay this far inside the world's edge.
    pub edge_margin_m: f32,
    /// At most this many raiders fight the pilot at once (the nearest).
    pub max_on_player: u32,
    /// A raider this close to the pilot may turn to fight them.
    pub engage_player_m: f32,
    /// A raider hit within this long turns on the pilot.
    pub grudge_s: f32,
    /// ...if the pilot is within this range.
    pub grudge_range_m: f32,
    /// Shelling the city, a raider fires for this long...
    pub siege_burst_s: f32,
    /// ...then holds fire for this long.
    pub siege_pause_s: f32,
    /// A siege aim point stands this far out from the building's wall.
    pub facade_out_m: f32,
    /// The kinds of raider, cycled through each wave.
    pub raiders: Vec<RaiderSpec>,
}

impl Default for DefenceConfig {
    fn default() -> Self {
        Self {
            cities: vec![0, 1, 2],
            raiders_per_wave: 10,
            wave_time_s: 600.0,
            lose_damage: 0.5,
            lull_s: 20.0,
            spawn_distance_m: 450.0,
            spawn_arc_deg: 100.0,
            spawn_jitter_m: 30.0,
            edge_margin_m: 150.0,
            max_on_player: 3,
            engage_player_m: 300.0,
            grudge_s: 8.0,
            grudge_range_m: 700.0,
            siege_burst_s: 1.0,
            siege_pause_s: 5.0,
            facade_out_m: 2.0,
            raiders: Vec::new(),
        }
    }
}

impl Validate for DefenceConfig {
    fn validate(&self) -> Result<(), String> {
        if self.cities.is_empty() {
            return Err("`cities` must name at least one city".into());
        }
        if self.raiders.is_empty() {
            return Err("`raiders` must list at least one kind of raider".into());
        }
        if self.raiders_per_wave == 0 {
            return Err("`raiders_per_wave` must be above zero".into());
        }
        positive("wave_time_s", self.wave_time_s)?;
        within("lose_damage", self.lose_damage, 0.0, 1.0)?;
        finite("lull_s", self.lull_s)?;
        positive("spawn_distance_m", self.spawn_distance_m)?;
        within("spawn_arc_deg", self.spawn_arc_deg, 0.0, 360.0)?;
        finite("spawn_jitter_m", self.spawn_jitter_m)?;
        finite("edge_margin_m", self.edge_margin_m)?;
        positive("engage_player_m", self.engage_player_m)?;
        finite("grudge_s", self.grudge_s)?;
        finite("grudge_range_m", self.grudge_range_m)?;
        positive("siege_burst_s", self.siege_burst_s)?;
        finite("siege_pause_s", self.siege_pause_s)?;
        finite("facade_out_m", self.facade_out_m)?;
        for (i, r) in self.raiders.iter().enumerate() {
            positive(&format!("raiders[{i}].armour_scale"), r.armour_scale)?;
            positive(&format!("raiders[{i}].skill.range_m"), r.skill.range_m)?;
            for w in r.weapons.iter().flatten() {
                w.validate()?;
            }
        }
        Ok(())
    }
}

/// Why the mission was lost.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loss {
    /// The attacked city passed the damage limit.
    CityFell {
        /// The city.
        city: u32,
        /// Its damage then, 0..1.
        damage: f32,
    },
    /// A wave was not destroyed in time.
    OutOfTime {
        /// The wave (0 first).
        wave: usize,
    },
    /// The pilot's mech was destroyed.
    PilotDown,
}

/// Where the mission is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Waiting for the pilot to take the seat.
    Briefing,
    /// Quiet before a wave.
    Lull {
        /// The wave coming (0 first).
        wave: usize,
        /// Seconds until it appears.
        left_s: f32,
    },
    /// A wave is attacking.
    Wave {
        /// The wave (0 first).
        wave: usize,
        /// Seconds since it appeared.
        elapsed_s: f32,
    },
    /// Every wave destroyed.
    Won,
    /// Lost.
    Lost(Loss),
}

/// Something that happened this step, for the game to act on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A wave appears now: spawn its raiders.
    WaveStarts(usize),
    /// A wave was destroyed.
    WaveCleared(usize),
    /// The mission is won.
    Won,
    /// The mission is lost.
    Lost(Loss),
}

/// What the simulation knows this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Situation {
    /// The pilot is in the seat.
    pub seated: bool,
    /// Raiders of the current wave destroyed so far.
    pub destroyed: u32,
    /// The attacked city's damage, 0..1 (`city_damage`).
    pub city_damage: f32,
    /// The pilot's mech is destroyed.
    pub pilot_down: bool,
}

/// The mission's state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Defence {
    /// Where it is.
    pub phase: Phase,
}

impl Default for Defence {
    fn default() -> Self {
        Self { phase: Phase::Briefing }
    }
}

impl Defence {
    /// The wave fighting or coming, if any.
    pub fn wave(&self) -> Option<usize> {
        match self.phase {
            Phase::Lull { wave, .. } | Phase::Wave { wave, .. } => Some(wave),
            _ => None,
        }
    }

    /// The city the current or coming wave attacks.
    pub fn city(&self, cfg: &DefenceConfig) -> Option<u32> {
        self.wave().and_then(|w| cfg.cities.get(w).copied())
    }

    /// Seconds left on the current wave's clock.
    pub fn time_left_s(&self, cfg: &DefenceConfig) -> Option<f32> {
        match self.phase {
            Phase::Wave { elapsed_s, .. } => Some((cfg.wave_time_s - elapsed_s).max(0.0)),
            _ => None,
        }
    }

    /// Advances the mission by `dt_s` with what the simulation knows; returns what happened.
    pub fn step(&mut self, cfg: &DefenceConfig, s: &Situation, dt_s: f32) -> Option<Event> {
        if matches!(self.phase, Phase::Won | Phase::Lost(_)) {
            return None;
        }
        if s.pilot_down && self.phase != Phase::Briefing {
            return Some(self.lose(Loss::PilotDown));
        }
        match self.phase {
            Phase::Briefing => {
                if s.seated {
                    self.phase = Phase::Lull {
                        wave: 0,
                        left_s: cfg.lull_s,
                    };
                }
                None
            }
            Phase::Lull { wave, left_s } => {
                let left_s = left_s - dt_s;
                if left_s <= 0.0 {
                    self.phase = Phase::Wave { wave, elapsed_s: 0.0 };
                    Some(Event::WaveStarts(wave))
                } else {
                    self.phase = Phase::Lull { wave, left_s };
                    None
                }
            }
            Phase::Wave { wave, elapsed_s } => {
                let city = cfg.cities[wave];
                if s.city_damage > cfg.lose_damage {
                    return Some(self.lose(Loss::CityFell {
                        city,
                        damage: s.city_damage,
                    }));
                }
                if s.destroyed >= cfg.raiders_per_wave {
                    if wave + 1 >= cfg.cities.len() {
                        self.phase = Phase::Won;
                        return Some(Event::Won);
                    }
                    self.phase = Phase::Lull {
                        wave: wave + 1,
                        left_s: cfg.lull_s,
                    };
                    return Some(Event::WaveCleared(wave));
                }
                let elapsed_s = elapsed_s + dt_s;
                if elapsed_s >= cfg.wave_time_s {
                    return Some(self.lose(Loss::OutOfTime { wave }));
                }
                self.phase = Phase::Wave { wave, elapsed_s };
                None
            }
            Phase::Won | Phase::Lost(_) => None,
        }
    }

    fn lose(&mut self, why: Loss) -> Event {
        self.phase = Phase::Lost(why);
        Event::Lost(why)
    }
}

/// A city's damage, 0..1: the share of its buildings' cells destroyed since the start. The one
/// number the rule checks and the cockpit shows.
pub fn city_damage(plan: &WorldPlan, structures: &Structures, city: u32) -> f32 {
    let (mut standing, mut total) = (0usize, 0usize);
    for b in plan.buildings.iter().filter(|b| b.city == city) {
        let c = &structures.buildings[b.id as usize];
        standing += c.standing;
        total += c.intact.len();
    }
    if total == 0 { 0.0 } else { 1.0 - standing as f32 / total as f32 }
}

/// World yaw of a direction (0 faces -z, positive turns left), as a mech's heading.
fn yaw_of(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

/// Where wave `wave`'s raiders appear and the heading each starts on (facing the city): an arc
/// beyond the city's corners on the side away from the previous wave's city (wave 0: a seeded
/// bearing), each point pulled toward the city until it is dry and inside the world.
pub fn spawn_points(plan: &WorldPlan, cfg: &DefenceConfig, wave: usize, rng: &SimRng) -> Vec<(Vec2, f32)> {
    let mut rng = rng.fork(&format!("defence/{wave}"));
    let city = &plan.cities[cfg.cities[wave] as usize];
    let centre = city.centre;
    let bearing = match wave.checked_sub(1).map(|w| plan.cities[cfg.cities[w] as usize].centre) {
        Some(prev) if prev.distance(centre) > 1.0 => {
            let d = centre - prev;
            d.y.atan2(d.x)
        }
        _ => rng.range_f32(-PI, PI),
    };
    let radius = city.half_size_m * std::f32::consts::SQRT_2 + cfg.spawn_distance_m;
    let inside = -plan.terrain.origin_m - cfg.edge_margin_m;
    let n = cfg.raiders_per_wave as usize;
    let arc = cfg.spawn_arc_deg.to_radians();
    (0..n)
        .map(|i| {
            let k = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
            let a = bearing + k * arc;
            let jitter = Vec2::new(rng.range_f32(-1.0, 1.0), rng.range_f32(-1.0, 1.0)) * cfg.spawn_jitter_m;
            let mut p = centre + Vec2::new(a.cos(), a.sin()) * radius + jitter;
            let toward = (centre - p).normalize_or(Vec2::X);
            for _ in 0..200 {
                let out = p.x.abs() > inside || p.y.abs() > inside;
                if !out && plan.water_at(p.x, p.y).is_none() {
                    break;
                }
                p += toward * 10.0;
            }
            (p, yaw_of(centre - p))
        })
        .collect()
}

/// What a raider goes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// The pilot's mech.
    Pilot,
    /// The attacked city.
    City,
}

/// What the mission knows of a raider when choosing its goal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaiderView {
    /// Where it is (its feet).
    pub pos: Vec3,
    /// Seconds since it was last hit (`f32::INFINITY` if never).
    pub since_hit_s: f32,
}

/// The goal of raider `i` among `raiders` (the live raiders of the wave, in stable order): the pilot
/// when it is one of the `max_on_player` nearest the pilot within `engage_player_m`, or was hit in
/// the last `grudge_s` with the pilot within `grudge_range_m`; the city otherwise.
pub fn raider_goal(i: usize, raiders: &[RaiderView], pilot: Vec3, cfg: &DefenceConfig) -> Goal {
    let me = raiders[i];
    let d = |r: &RaiderView| Vec2::new(r.pos.x - pilot.x, r.pos.z - pilot.z).length();
    if me.since_hit_s < cfg.grudge_s && d(&me) < cfg.grudge_range_m {
        return Goal::Pilot;
    }
    if d(&me) >= cfg.engage_player_m {
        return Goal::City;
    }
    // Rank among those in reach by distance, ties by index (stable).
    let closer = raiders
        .iter()
        .enumerate()
        .filter(|(j, r)| *j != i && d(r) < cfg.engage_player_m && (d(r) < d(&me) || (d(r) == d(&me) && *j < i)))
        .count();
    if closer < cfg.max_on_player as usize {
        Goal::Pilot
    } else {
        Goal::City
    }
}

/// Where a raider shelling city `city` aims from `from`: the nearest standing building's wall point
/// nearest it, `facade_out_m` out from the wall, at the city's ground. `None` when nothing stands.
pub fn siege_point(plan: &WorldPlan, structures: &Structures, city: u32, from: Vec3, facade_out_m: f32) -> Option<Vec3> {
    let here = Vec2::new(from.x, from.z);
    let ground = plan.cities.get(city as usize)?.ground_m;
    plan.buildings
        .iter()
        .filter(|b| b.city == city && structures.buildings[b.id as usize].standing > 0)
        .map(|b| {
            let wall = here.clamp(b.footprint.min, b.footprint.max);
            (wall, wall.distance(here), b.footprint.centre())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(wall, _, centre)| {
            let out = (here - wall).normalize_or((here - centre).normalize_or(Vec2::X));
            let p = wall + out * facade_out_m;
            Vec3::new(p.x, ground, p.y)
        })
}

/// True when raider `index` (of `count`) may fire on the city at mission time `t_s`: a burst of
/// `siege_burst_s` in every `siege_burst_s + siege_pause_s`, staggered across the wave.
pub fn siege_fires(index: usize, count: usize, t_s: f32, cfg: &DefenceConfig) -> bool {
    let period = cfg.siege_burst_s + cfg.siege_pause_s;
    let offset = period * index as f32 / count.max(1) as f32;
    (t_s + offset).rem_euclid(period) < cfg.siege_burst_s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use crate::world::{WorldConfig, generate};
    use std::sync::OnceLock;

    fn plan() -> &'static WorldPlan {
        static P: OnceLock<WorldPlan> = OnceLock::new();
        P.get_or_init(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
            generate(42, &load_ron::<WorldConfig>(&p).unwrap())
        })
    }

    fn cfg() -> DefenceConfig {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/missions/defence.ron");
        load_ron(&p).unwrap()
    }

    fn fighting(d: &mut Defence, c: &DefenceConfig) {
        d.step(
            c,
            &Situation {
                seated: true,
                ..Default::default()
            },
            0.1,
        );
        let mut started = None;
        for _ in 0..1000 {
            if let Some(e) = d.step(
                c,
                &Situation {
                    seated: true,
                    ..Default::default()
                },
                0.1,
            ) {
                started = Some(e);
                break;
            }
        }
        assert_eq!(started, Some(Event::WaveStarts(d.wave().unwrap())));
    }

    #[test]
    fn clearing_a_wave_starts_the_next_after_the_lull() {
        let c = cfg();
        let mut d = Defence::default();
        fighting(&mut d, &c);
        let all = Situation {
            seated: true,
            destroyed: c.raiders_per_wave,
            ..Default::default()
        };
        assert_eq!(d.step(&c, &all, 0.1), Some(Event::WaveCleared(0)));
        assert!(matches!(d.phase, Phase::Lull { wave: 1, .. }));
        let quiet = Situation {
            seated: true,
            ..Default::default()
        };
        let mut t = 0.0;
        loop {
            t += 0.1;
            if let Some(e) = d.step(&c, &quiet, 0.1) {
                assert_eq!(e, Event::WaveStarts(1));
                break;
            }
        }
        assert!((t - c.lull_s).abs() < 0.15, "{t}");
        assert_eq!(d.city(&c), Some(c.cities[1]));
    }

    #[test]
    fn a_city_past_half_damage_loses_the_mission() {
        let c = cfg();
        let mut d = Defence::default();
        fighting(&mut d, &c);
        let hurt = Situation {
            seated: true,
            city_damage: 0.501,
            ..Default::default()
        };
        assert_eq!(
            d.step(&c, &hurt, 0.1),
            Some(Event::Lost(Loss::CityFell {
                city: c.cities[0],
                damage: 0.501
            }))
        );
        assert_eq!(d.step(&c, &hurt, 0.1), None, "lost stays lost");
    }

    #[test]
    fn a_wave_not_cleared_in_ten_minutes_loses() {
        let c = cfg();
        assert_eq!(c.wave_time_s, 600.0);
        let mut d = Defence::default();
        fighting(&mut d, &c);
        let nine = Situation {
            seated: true,
            destroyed: c.raiders_per_wave - 1,
            ..Default::default()
        };
        let mut lost_at = None;
        for i in 1..=7000 {
            if let Some(e) = d.step(&c, &nine, 0.1) {
                lost_at = Some((i as f32 * 0.1, e));
                break;
            }
        }
        let (t, e) = lost_at.unwrap();
        assert_eq!(e, Event::Lost(Loss::OutOfTime { wave: 0 }));
        assert!((t - 600.0).abs() < 0.15, "{t}");
    }

    #[test]
    fn clearing_all_three_waves_wins() {
        let c = cfg();
        let mut d = Defence::default();
        let all = Situation {
            seated: true,
            destroyed: c.raiders_per_wave,
            ..Default::default()
        };
        for w in 0..c.cities.len() {
            fighting(&mut d, &c);
            let e = d.step(&c, &all, 0.1);
            if w + 1 < c.cities.len() {
                assert_eq!(e, Some(Event::WaveCleared(w)));
            } else {
                assert_eq!(e, Some(Event::Won));
            }
        }
        assert_eq!(d.phase, Phase::Won);
    }

    #[test]
    fn losing_the_mech_loses_the_mission() {
        let c = cfg();
        let mut d = Defence::default();
        fighting(&mut d, &c);
        let down = Situation {
            seated: true,
            pilot_down: true,
            ..Default::default()
        };
        assert_eq!(d.step(&c, &down, 0.1), Some(Event::Lost(Loss::PilotDown)));
    }

    #[test]
    fn city_damage_counts_cells_not_buildings() {
        let p = plan();
        let mut s = Structures::new(p);
        assert_eq!(city_damage(p, &s, 0), 0.0);
        let b = p.buildings.iter().filter(|b| b.city == 0).max_by_key(|b| b.storeys).unwrap();
        let cells = s.buildings[b.id as usize].intact.len();
        let total: usize = p
            .buildings
            .iter()
            .filter(|x| x.city == 0)
            .map(|x| s.buildings[x.id as usize].intact.len())
            .sum();
        let c = &mut s.buildings[b.id as usize];
        c.intact.iter_mut().for_each(|x| *x = false);
        c.standing = 0;
        let want = cells as f32 / total as f32;
        assert!((city_damage(p, &s, 0) - want).abs() < 1e-6);
        let buildings = p.buildings.iter().filter(|x| x.city == 0).count();
        assert!(
            want > 1.0 / buildings as f32,
            "the tallest tower counts more than one building's share"
        );
        assert_eq!(city_damage(p, &s, 1), 0.0, "other cities are untouched");
    }

    #[test]
    fn spawn_points_are_the_same_for_the_same_seed() {
        let c = cfg();
        let rng = SimRng::new(42);
        for w in 0..c.cities.len() {
            assert_eq!(spawn_points(plan(), &c, w, &rng), spawn_points(plan(), &c, w, &rng));
        }
    }

    #[test]
    fn spawn_points_are_dry_and_inside_the_world() {
        let c = cfg();
        let p = plan();
        let inside = -p.terrain.origin_m - c.edge_margin_m + 1.0;
        for w in 0..c.cities.len() {
            let pts = spawn_points(p, &c, w, &SimRng::new(42));
            assert_eq!(pts.len(), c.raiders_per_wave as usize);
            let city = &p.cities[c.cities[w] as usize];
            for (pt, heading) in pts {
                assert!(pt.x.abs() <= inside && pt.y.abs() <= inside, "wave {w}: {pt}");
                assert!(p.water_at(pt.x, pt.y).is_none(), "wave {w}: {pt} is wet");
                assert!(
                    pt.distance(city.centre) > city.half_size_m,
                    "wave {w}: {pt} starts outside the city"
                );
                let facing = Vec2::new(-heading.sin(), -heading.cos());
                assert!(facing.dot((city.centre - pt).normalize()) > 0.99, "faces the city");
            }
        }
    }

    fn at(x: f32, z: f32) -> RaiderView {
        RaiderView {
            pos: Vec3::new(x, 0.0, z),
            since_hit_s: f32::INFINITY,
        }
    }

    #[test]
    fn a_raider_far_from_the_pilot_shells_the_city() {
        let c = cfg();
        assert_eq!(raider_goal(0, &[at(900.0, 0.0)], Vec3::ZERO, &c), Goal::City);
        let p = plan();
        let s = Structures::new(p);
        let city = &p.cities[0];
        let from = Vec3::new(city.centre.x + city.half_size_m + 400.0, 0.0, city.centre.y);
        let aim = siege_point(p, &s, 0, from, c.facade_out_m).unwrap();
        let b = p
            .buildings
            .iter()
            .filter(|b| b.city == 0)
            .min_by(|a, b| {
                let h = Vec2::new(from.x, from.z);
                h.clamp(a.footprint.min, a.footprint.max)
                    .distance(h)
                    .total_cmp(&h.clamp(b.footprint.min, b.footprint.max).distance(h))
            })
            .unwrap();
        let wall = Vec2::new(aim.x, aim.z).clamp(b.footprint.min, b.footprint.max);
        assert!(
            (wall.distance(Vec2::new(aim.x, aim.z)) - c.facade_out_m).abs() < 0.01,
            "just outside its wall"
        );
    }

    #[test]
    fn only_the_nearest_three_turn_on_the_pilot() {
        let c = cfg();
        assert_eq!(c.max_on_player, 3);
        let rs: Vec<RaiderView> = [250.0, 50.0, 200.0, 100.0, 150.0].iter().map(|x| at(*x, 0.0)).collect();
        let goals: Vec<Goal> = (0..rs.len()).map(|i| raider_goal(i, &rs, Vec3::ZERO, &c)).collect();
        assert_eq!(goals, vec![Goal::City, Goal::Pilot, Goal::City, Goal::Pilot, Goal::Pilot]);
    }

    #[test]
    fn a_raider_hit_turns_on_the_pilot() {
        let c = cfg();
        let mut rs: Vec<RaiderView> = [50.0, 60.0, 70.0, 80.0, 500.0].iter().map(|x| at(*x, 0.0)).collect();
        assert_eq!(raider_goal(4, &rs, Vec3::ZERO, &c), Goal::City);
        rs[4].since_hit_s = 1.0;
        assert_eq!(raider_goal(4, &rs, Vec3::ZERO, &c), Goal::Pilot);
        rs[4].since_hit_s = c.grudge_s + 1.0;
        assert_eq!(raider_goal(4, &rs, Vec3::ZERO, &c), Goal::City, "the grudge passes");
    }

    #[test]
    fn a_wave_never_fires_on_the_city_all_at_once() {
        let c = cfg();
        let n = c.raiders_per_wave as usize;
        for step in 0..600 {
            let t = step as f32 * 0.01;
            let firing = (0..n).filter(|i| siege_fires(*i, n, t, &c)).count();
            assert!(firing <= n.div_ceil(2), "{firing} at {t}");
        }
    }
}
