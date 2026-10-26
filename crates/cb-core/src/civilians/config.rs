//! People tuning, `data/people/people.ron` (openspec change `civilians`, design section 1): how many
//! live where, how they walk, what frightens them, what kills them and how near a greeting reaches.
//! Every number carries its unit in its key (CLAUDE.md section 6.4).

use crate::data::{Validate, positive, positive_range, within};
use serde::Deserialize;

/// People tuning.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct PeopleConfig {
    /// A block, park or base within this of the pilot is peopled.
    pub spawn_radius_m: f32,
    /// A person farther than this from the pilot is let go; a block farther than this may be peopled
    /// again when the pilot comes back.
    pub keep_radius_m: f32,
    /// Never more people than this at once.
    pub max_people: usize,
    /// People brought in per fixed step at most, so a fresh district fills over a few steps.
    pub spawn_per_step: usize,
    /// The pilot must move this far before the peopled blocks are looked at again.
    pub recheck_m: f32,
    /// People per 100 m of pavement loop, in a city's heart and at its edge.
    pub pavement_per_100m: [f32; 2],
    /// How density falls from the heart to the edge: `heart + (edge - heart) * d^exponent`, `d`
    /// the block's distance from the centre over the city's half size.
    pub density_exponent: f32,
    /// People per hectare of park, at the heart's density (scaled by district like the pavements).
    pub park_per_hectare: f32,
    /// People on each air base's apron.
    pub base_people: usize,
    /// The park path runs this far inside the park's kerb.
    pub park_path_inset_m: f32,
    /// The apron walk in front of the hangars is this deep.
    pub base_apron_depth_m: f32,
    /// Walking speed, least and most.
    pub walk_speed_m_s: [f32; 2],
    /// Running speed, least and most.
    pub run_speed_m_s: [f32; 2],
    /// Standing height, least and most.
    pub height_m: [f32; 2],
    /// Chance, at a pavement corner, of crossing the street.
    pub cross_chance: f32,
    /// Time stood at the kerb before crossing, least and most.
    pub crossing_wait_s: [f32; 2],
    /// Chance, at a park bench, of sitting on it.
    pub bench_chance: f32,
    /// Time sat on a bench, least and most.
    pub bench_sit_s: [f32; 2],
    /// Chance, at a pavement's midpoint or a base's corner, of stopping to stand a while.
    pub idle_chance: f32,
    /// Time stood idle, least and most.
    pub idle_s: [f32; 2],
    /// A shot passing within this of someone (horizontally) frightens them.
    pub shot_fright_m: f32,
    /// A blast frightens out to this many metres per cube root of its megajoules.
    pub blast_fright_m_per_cbrt_mj: f32,
    /// A mech's footfall frightens out to this.
    pub footfall_fright_m: f32,
    /// How long a fright lasts, least and most.
    pub flee_s: [f32; 2],
    /// A shot passing within this of someone's body (feet to head) kills them.
    pub shot_lethal_m: f32,
    /// A blast kills out to this many metres per cube root of its megajoules.
    pub blast_lethal_m_per_cbrt_mj: f32,
    /// A mech's foot coming down within this of someone (horizontally) crushes them.
    pub foot_lethal_m: f32,
    /// A fleeing person makes for a street door within this, when the world has doors.
    pub door_reach_m: f32,
    /// How long someone struck down lies before they are cleared away.
    pub down_s: f32,
    /// Use greets a person within this of the pilot's feet.
    pub greet_reach_m: f32,
    /// ... and within this angle either side of where the pilot faces.
    pub greet_cone_deg: f32,
    /// A greeted person stays stopped and facing the pilot this long.
    pub greet_hold_s: f32,
    /// How people are drawn.
    pub draw: DrawConfig,
}

/// How people are drawn (design section 2): the full body near the eye, a low figure in the middle
/// distance, a flat card far off.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DrawConfig {
    /// Within this of the eye a person is drawn as the full body.
    pub near_m: f32,
    /// ... and at most this many of them (the nearest first in list order past it).
    pub near_max: usize,
    /// Within this, the low figure; beyond it, the card.
    pub mid_m: f32,
    /// A walker's stride: the walk's poses change every half stride.
    pub stride_m: f32,
    /// The crowd past the near distance is rebuilt this often (a person at 1.5 m/s moves 5 cm between
    /// rebuilds at 30 Hz, too little to see past 60 m).
    pub crowd_refresh_hz: f32,
}

impl Default for DrawConfig {
    fn default() -> Self {
        Self {
            near_m: 60.0,
            near_max: 200,
            mid_m: 250.0,
            stride_m: 1.4,
            crowd_refresh_hz: 30.0,
        }
    }
}

impl Default for PeopleConfig {
    fn default() -> Self {
        Self {
            spawn_radius_m: 400.0,
            keep_radius_m: 600.0,
            max_people: 2000,
            spawn_per_step: 60,
            recheck_m: 8.0,
            pavement_per_100m: [8.0, 1.5],
            density_exponent: 1.5,
            park_per_hectare: 12.0,
            base_people: 24,
            park_path_inset_m: 8.0,
            base_apron_depth_m: 18.0,
            walk_speed_m_s: [1.1, 1.7],
            run_speed_m_s: [3.5, 5.5],
            height_m: [1.55, 1.95],
            cross_chance: 0.35,
            crossing_wait_s: [2.0, 9.0],
            bench_chance: 0.4,
            bench_sit_s: [20.0, 120.0],
            idle_chance: 0.15,
            idle_s: [3.0, 15.0],
            shot_fright_m: 35.0,
            blast_fright_m_per_cbrt_mj: 30.0,
            footfall_fright_m: 70.0,
            flee_s: [10.0, 20.0],
            shot_lethal_m: 0.9,
            blast_lethal_m_per_cbrt_mj: 5.0,
            foot_lethal_m: 4.0,
            door_reach_m: 60.0,
            down_s: 30.0,
            greet_reach_m: 2.5,
            greet_cone_deg: 50.0,
            greet_hold_s: 6.0,
            draw: DrawConfig::default(),
        }
    }
}

impl Validate for PeopleConfig {
    fn validate(&self) -> Result<(), String> {
        positive("spawn_radius_m", self.spawn_radius_m)?;
        positive("keep_radius_m", self.keep_radius_m)?;
        if self.keep_radius_m < self.spawn_radius_m {
            return Err("`keep_radius_m` must be at least `spawn_radius_m`".into());
        }
        if self.max_people == 0 || self.spawn_per_step == 0 {
            return Err("`max_people` and `spawn_per_step` must be above zero".into());
        }
        positive("recheck_m", self.recheck_m)?;
        within("pavement_per_100m", self.pavement_per_100m[0], 0.0, 100.0)?;
        within("pavement_per_100m", self.pavement_per_100m[1], 0.0, 100.0)?;
        positive("density_exponent", self.density_exponent)?;
        within("park_per_hectare", self.park_per_hectare, 0.0, 500.0)?;
        positive("park_path_inset_m", self.park_path_inset_m)?;
        positive("base_apron_depth_m", self.base_apron_depth_m)?;
        positive_range("walk_speed_m_s", self.walk_speed_m_s)?;
        positive_range("run_speed_m_s", self.run_speed_m_s)?;
        positive_range("height_m", self.height_m)?;
        within("cross_chance", self.cross_chance, 0.0, 1.0)?;
        positive_range("crossing_wait_s", self.crossing_wait_s)?;
        within("bench_chance", self.bench_chance, 0.0, 1.0)?;
        positive_range("bench_sit_s", self.bench_sit_s)?;
        within("idle_chance", self.idle_chance, 0.0, 1.0)?;
        positive_range("idle_s", self.idle_s)?;
        positive("shot_fright_m", self.shot_fright_m)?;
        positive("blast_fright_m_per_cbrt_mj", self.blast_fright_m_per_cbrt_mj)?;
        positive("footfall_fright_m", self.footfall_fright_m)?;
        positive_range("flee_s", self.flee_s)?;
        positive("shot_lethal_m", self.shot_lethal_m)?;
        positive("blast_lethal_m_per_cbrt_mj", self.blast_lethal_m_per_cbrt_mj)?;
        positive("foot_lethal_m", self.foot_lethal_m)?;
        positive("door_reach_m", self.door_reach_m)?;
        positive("down_s", self.down_s)?;
        positive("greet_reach_m", self.greet_reach_m)?;
        within("greet_cone_deg", self.greet_cone_deg, 1.0, 180.0)?;
        positive("greet_hold_s", self.greet_hold_s)?;
        positive("draw.near_m", self.draw.near_m)?;
        positive("draw.mid_m", self.draw.mid_m)?;
        positive("draw.stride_m", self.draw.stride_m)?;
        positive("draw.crowd_refresh_hz", self.draw.crowd_refresh_hz)?;
        if self.draw.mid_m < self.draw.near_m {
            return Err("`draw.mid_m` must be at least `draw.near_m`".into());
        }
        Ok(())
    }
}
