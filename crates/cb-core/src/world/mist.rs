//! Ground fog (openspec change `ground-fog`; owner, 2026-10-25: "think about some cheap volumetric
//! ground/distance fog too", then "where's ground fog?"): a layer of air densest at sea level and
//! thinning with height, worked out exactly along each view ray (no march), thickened where mist
//! forms (hollows below the ground round them, near water, along the coast) and nowhere above
//! 600 m, and greyer under a storm's rain.
//!
//! The rules are here, pure and tested: `mist_at` is the one mist rule (the drawn map is baked from
//! it, `MistMap`), and `optical_depth` is the one fog integral (`layer_depth` in
//! `assets/shaders/water_common.wesl` repeats it on the GPU for the fog pass and the clouds; a
//! test checks the closed form against a fine march). The tuning is in `data/world/world.ron` under `fog:`.
//!
//! How thick it is right now (survey F1; owner, 2026-10-26: "Approve all"): a morning mist that
//! thins by day, on the fog's own clock (the game has no time of day: the sun stands still), or a
//! mission's own preset (`clear` for the combat-heavy ones). `FogConfig::strength` is that rule.
//!
//! What lies between two points (survey F2: fog also hides targets from the sensors):
//! `line_depth` is the compose pass's own fog along a pixel's ray, worked out on the CPU for the
//! straight line from the pilot's eye to a target, so the fog the targeting sees through is the
//! fog drawn.

use super::config::WorldConfig;
use super::terrain::{Heightfield, smoothstep};
use super::water::Waters;
use crate::data::{Validate, finite, positive, positive_range, within};
use crate::missions::MissionKind;
use glam::{Vec2, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

/// How thick the fog is (survey F1): a morning mist, a day's thinner one, or clear air.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum FogPreset {
    /// Thick at sea level, full mist in the hollows and by the water.
    Morning,
    /// Plain air, half the mist.
    Day,
    /// Thin air and no mist: for the combat-heavy missions.
    Clear,
}

/// What a preset does to the fog: scales of the sea-level density and of the mist's strength.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FogStrength {
    /// Times `density_per_m`.
    pub density_scale: f32,
    /// Times `mist_strength`.
    pub mist_scale: f32,
}

impl FogStrength {
    /// The fog as `fog:` sets it, unscaled.
    pub const PLAIN: Self = Self {
        density_scale: 1.0,
        mist_scale: 1.0,
    };

    /// Part way from `self` to `to`, `f` 0..1.
    pub fn lerp(self, to: Self, f: f32) -> Self {
        let f = f.clamp(0.0, 1.0);
        Self {
            density_scale: self.density_scale + (to.density_scale - self.density_scale) * f,
            mist_scale: self.mist_scale + (to.mist_scale - self.mist_scale) * f,
        }
    }

    fn validate(&self, field: &str) -> Result<(), String> {
        within(&format!("{field}.density_scale"), self.density_scale, 0.0, 20.0)?;
        within(&format!("{field}.mist_scale"), self.mist_scale, 0.0, 20.0)
    }
}

/// The presets' strengths (`fog.presets`).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FogPresets {
    /// `Morning`.
    pub morning: FogStrength,
    /// `Day`.
    pub day: FogStrength,
    /// `Clear`.
    pub clear: FogStrength,
}

impl Default for FogPresets {
    fn default() -> Self {
        Self {
            morning: FogStrength {
                density_scale: 2.0,
                mist_scale: 1.0,
            },
            day: FogStrength {
                density_scale: 1.0,
                mist_scale: 0.5,
            },
            clear: FogStrength {
                density_scale: 0.5,
                mist_scale: 0.0,
            },
        }
    }
}

impl FogPresets {
    /// A preset's strength.
    pub fn get(&self, p: FogPreset) -> FogStrength {
        match p {
            FogPreset::Morning => self.morning,
            FogPreset::Day => self.day,
            FogPreset::Clear => self.clear,
        }
    }
}

/// The fog's own clock (`fog.clock`): the game has no time of day, so the mist burns off on this.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FogClock {
    /// The hour (0..24) when the world begins.
    pub start_h: f32,
    /// Game hours that pass in a second.
    pub hours_per_s: f32,
    /// The morning mist thins to the day's between these hours.
    pub burn_off_h: [f32; 2],
}

impl Default for FogClock {
    fn default() -> Self {
        Self {
            start_h: 6.0,
            hours_per_s: 1.0 / 300.0,
            burn_off_h: [7.0, 11.0],
        }
    }
}

impl FogClock {
    /// The hour `elapsed_s` after the world began (not wrapped: once the mist has burnt off it
    /// stays off).
    pub fn hour_at(&self, elapsed_s: f32) -> f32 {
        self.start_h + elapsed_s.max(0.0) * self.hours_per_s
    }
}

/// The fog's tuning (`fog:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FogConfig {
    /// Extinction at sea level in plain air, per metre (1 / 12,000: level ground fades to half its
    /// colour by about 8 km).
    pub density_per_m: f32,
    /// The aerial haze's extinction at sea level, per metre (design section 2a): a far thinner,
    /// deeper layer than the fog, so far land and sea blue and soften at every height.
    pub haze_density_per_m: f32,
    /// The haze thins by e every this much height, metres.
    pub haze_scale_height_m: f32,
    /// The layer thins by e every this much height.
    pub scale_height_m: f32,
    /// In full mist the density is this many times more.
    pub mist_strength: f32,
    /// Mist forms only below this height.
    pub mist_top_m: f32,
    /// Ground this far below the average round it (over `hollow_reach_m`) is a full hollow.
    pub hollow_depth_m: f32,
    /// See `hollow_depth_m`.
    pub hollow_reach_m: f32,
    /// Water within this adds mist.
    pub water_reach_m: f32,
    /// The coast within this adds mist.
    pub coast_reach_m: f32,
    /// The mist map's cell.
    pub map_cell_m: f32,
    /// The view ray reads the mist at four points out to at most this far.
    pub mist_taps_m: f32,
    /// A sky pixel's ray is this long.
    pub sky_length_m: f32,
    /// Extra extinction while the eye is under a storm's rain, per metre, times the rain.
    pub rain_per_m: f32,
    /// How far along a sight line the rain's haze reaches: a shaft is a few kilometres across, so a
    /// line leaves the rain within about this far. Taken over a sky ray's whole length, a trace of
    /// rain at the eye turned the whole sky to fog, and flying past a shaft's edge the sky pulsed
    /// grey (owner, 2026-10-26: "the whole sky ios flikering").
    pub rain_reach_m: f32,
    /// Time constant in seconds over which the weather sampled at the eye (cloud, rain, the clouds'
    /// shade) is followed, so the fog over the whole view eases rather than jumping as a fast mech
    /// passes the edge of a puff, a shaft or a cloud's shadow.
    pub eye_weather_s: f32,
    /// How much darker the fog's colour is under the full shade of the clouds over the eye, 0..1
    /// (the scattering sky it takes its colour from knows nothing of clouds).
    pub storm_dim: f32,
    /// How much of the sky's ambient light the fog scatters back.
    pub ambient_share: f32,
    /// How much of the sun the fog scatters toward the eye, through the phase function.
    pub sun_share: f32,
    /// Henyey-Greenstein asymmetry of the scattering (0 even, toward 1 a glow round the sun).
    pub phase_g: f32,
    /// What each preset does (survey F1).
    pub presets: FogPresets,
    /// The clock the morning mist burns off on.
    pub clock: FogClock,
    /// A mission's own preset while it runs; with none running, or one not listed, the morning
    /// mist thins by the clock.
    pub missions: BTreeMap<MissionKind, FogPreset>,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            density_per_m: 1.0 / 12000.0,
            haze_density_per_m: 1.0 / 45000.0,
            haze_scale_height_m: 1500.0,
            scale_height_m: 120.0,
            mist_strength: 5.0,
            mist_top_m: 600.0,
            hollow_depth_m: 100.0,
            hollow_reach_m: 1500.0,
            water_reach_m: 300.0,
            coast_reach_m: 600.0,
            map_cell_m: 160.0,
            mist_taps_m: 6000.0,
            sky_length_m: 40000.0,
            rain_per_m: 1.0 / 1500.0,
            rain_reach_m: 1500.0,
            eye_weather_s: 1.5,
            storm_dim: 0.45,
            ambient_share: 1.0,
            sun_share: 0.25,
            phase_g: 0.6,
            presets: FogPresets::default(),
            clock: FogClock::default(),
            // Combat-heavy: the three-city defence and the air fortress (survey F1).
            missions: BTreeMap::from([(MissionKind::Defence, FogPreset::Clear), (MissionKind::Fortress, FogPreset::Clear)]),
        }
    }
}

impl FogConfig {
    /// How thick the fog is (survey F1): the running mission's preset if it has one, else the
    /// morning mist thinning to the day's as the clock passes `clock.burn_off_h`.
    pub fn strength(&self, mission: Option<MissionKind>, hour_h: f32) -> FogStrength {
        if let Some(p) = mission.and_then(|m| self.missions.get(&m)) {
            return self.presets.get(*p);
        }
        let [a, b] = self.clock.burn_off_h;
        self.presets.day.lerp(self.presets.morning, 1.0 - smoothstep(a, b, hour_h))
    }
}

impl Validate for FogConfig {
    fn validate(&self) -> Result<(), String> {
        within("fog.density_per_m", self.density_per_m, 0.0, 0.01)?;
        positive("fog.scale_height_m", self.scale_height_m)?;
        within("fog.haze_density_per_m", self.haze_density_per_m, 0.0, 0.01)?;
        positive("fog.haze_scale_height_m", self.haze_scale_height_m)?;
        within("fog.mist_strength", self.mist_strength, 0.0, 50.0)?;
        finite("fog.mist_top_m", self.mist_top_m)?;
        positive("fog.hollow_depth_m", self.hollow_depth_m)?;
        positive("fog.hollow_reach_m", self.hollow_reach_m)?;
        positive("fog.water_reach_m", self.water_reach_m)?;
        positive("fog.coast_reach_m", self.coast_reach_m)?;
        positive("fog.map_cell_m", self.map_cell_m)?;
        positive("fog.mist_taps_m", self.mist_taps_m)?;
        positive("fog.sky_length_m", self.sky_length_m)?;
        within("fog.rain_per_m", self.rain_per_m, 0.0, 0.1)?;
        within("fog.rain_reach_m", self.rain_reach_m, 1.0, 100000.0)?;
        within("fog.eye_weather_s", self.eye_weather_s, 0.0, 30.0)?;
        within("fog.storm_dim", self.storm_dim, 0.0, 1.0)?;
        within("fog.ambient_share", self.ambient_share, 0.0, 4.0)?;
        within("fog.sun_share", self.sun_share, 0.0, 4.0)?;
        within("fog.phase_g", self.phase_g, -0.95, 0.95)?;
        self.presets.morning.validate("fog.presets.morning")?;
        self.presets.day.validate("fog.presets.day")?;
        self.presets.clear.validate("fog.presets.clear")?;
        within("fog.clock.start_h", self.clock.start_h, 0.0, 24.0)?;
        within("fog.clock.hours_per_s", self.clock.hours_per_s, 0.0, 10.0)?;
        positive_range("fog.clock.burn_off_h", self.clock.burn_off_h)
    }
}

/// The optical depth (the extinction summed) along a straight ray from height `from_y_m`, rising
/// `dir_y` a metre (the ray's direction's vertical part), for `length_m`, through the layer of
/// `cfg` thickened `mist` times. The closed form of the integral of
/// `mist * density * exp(-(y - sea) / H)`, written with both ends' exponentials so it neither
/// overflows looking down nor loses precision looking up.
pub fn optical_depth(cfg: &FogConfig, sea_level_m: f32, from_y_m: f32, dir_y: f32, length_m: f32, mist: f32) -> f32 {
    layer_depth(mist * cfg.density_per_m, cfg.scale_height_m, sea_level_m, from_y_m, dir_y, length_m)
}

/// The aerial haze's optical depth along the same ray (design section 2a): the layer of
/// `haze_density_per_m` thinning every `haze_scale_height_m`, not thickened by mist.
pub fn haze_depth(cfg: &FogConfig, sea_level_m: f32, from_y_m: f32, dir_y: f32, length_m: f32) -> f32 {
    layer_depth(
        cfg.haze_density_per_m,
        cfg.haze_scale_height_m,
        sea_level_m,
        from_y_m,
        dir_y,
        length_m,
    )
}

/// The optical depth through an exponential layer of `density` per metre at sea level, thinning
/// by e every `h` metres, along a ray from height `from_y_m` rising `dir_y` a metre for
/// `length_m`. The one closed form of the fog and the haze (`layer_depth` in `water_common.wesl`).
fn layer_depth(density: f32, h: f32, sea_level_m: f32, from_y_m: f32, dir_y: f32, length_m: f32) -> f32 {
    let e = |y: f32| (-(y - sea_level_m).max(-200.0) / h).exp();
    let e0 = e(from_y_m);
    let length = length_m.max(0.0);
    if dir_y.abs() < 1e-4 {
        return density * e0 * length;
    }
    let e1 = e(from_y_m + dir_y * length);
    density * h * (e0 - e1) / dir_y
}

/// Where the compose pass reads the mist along a ray, as shares of the way out to
/// `mist_taps_m` (`fog_depth` in `water_post.wesl`).
pub const MIST_TAPS: [f32; 4] = [0.1, 0.3, 0.6, 1.0];

/// The fog's optical depth along the straight line from `from` to `to`, worked out as the compose
/// pass works it out for a pixel (`fog_depth` in `water_post.wesl`): the mist read at the four
/// taps and averaged, thickening the layer's exact integral, scaled by the strength now
/// (`FogConfig::strength`), plus the rain's haze `rain` (0..1, under a shaft at `from`) along it as
/// far as `rain_reach_m`.
/// `exp(-line_depth)` is the share of the target's light that reaches the eye.
pub fn line_depth(cfg: &FogConfig, strength: FogStrength, map: &MistMap, sea_level_m: f32, from: Vec3, to: Vec3, rain: f32) -> f32 {
    let len = from.distance(to);
    if !len.is_finite() || len <= 0.0 {
        return 0.0;
    }
    let dir = (to - from) / len;
    let thick = thickening(cfg, strength, map, from, dir, len.min(cfg.mist_taps_m));
    let layer = optical_depth(cfg, sea_level_m, from.y, dir.y, len, thick) * strength.density_scale;
    layer + haze_depth(cfg, sea_level_m, from.y, dir.y, len) + rain_depth(cfg, rain, len)
}

/// How many times the mist thickens the layer along a sight line from `from` going `dir` (a unit vector): the mist read
/// at the four taps out to `reach_m` and averaged, `1 + mist_strength * mist_scale * mist` (`fog_depth` in
/// `water_post.wesl` writes it the same way). The clouds read it round the eye on level bearings out to `mist_taps_m`
/// (`clouds_render`), so a far cloud is thickened by the same mist as the far land under it.
pub fn thickening(cfg: &FogConfig, strength: FogStrength, map: &MistMap, from: Vec3, dir: Vec3, reach_m: f32) -> f32 {
    let mist = MIST_TAPS
        .iter()
        .map(|f| {
            let p = from + dir * (reach_m * f);
            map.at(Vec2::new(p.x, p.z))
        })
        .sum::<f32>()
        / MIST_TAPS.len() as f32;
    1.0 + cfg.mist_strength * strength.mist_scale * mist
}

/// The rain's haze along `len` metres of a sight line from an eye under rain `rain` (0..1): only
/// as far as `rain_reach_m`, where the line has left the shaft. The one copy (`fog_depth` in
/// `water_post.wesl` writes it the same way).
pub fn rain_depth(cfg: &FogConfig, rain: f32, len: f32) -> f32 {
    cfg.rain_per_m * rain.clamp(0.0, 1.0) * len.max(0.0).min(cfg.rain_reach_m)
}

/// How misty a place is, 0 (plain air) to 1 (full mist): a hollow below the ground round it, near
/// water, near the coast; nothing above `mist_top_m`. `ground` is the ground's height, `around` the
/// average ground round it, `near_water` and `near_coast` 0..1.
pub fn mist_at(cfg: &FogConfig, ground_m: f32, around_m: f32, near_water: f32, near_coast: f32, sea_level_m: f32) -> f32 {
    let hollow = ((around_m - ground_m) / cfg.hollow_depth_m).clamp(0.0, 1.0);
    let low = 1.0 - smoothstep(cfg.mist_top_m * 0.6, cfg.mist_top_m, ground_m - sea_level_m);
    (low * (0.6 * hollow + 0.5 * near_water + 0.3 * near_coast)).clamp(0.0, 1.0)
}

/// The mist over the whole world on a grid, baked from `mist_at` (the drawn map).
#[derive(Clone, Debug, PartialEq)]
pub struct MistMap {
    /// Cells a side.
    pub side: usize,
    /// The grid's least corner (x and z), metres.
    pub origin_m: f32,
    /// A cell's side, metres.
    pub cell_m: f32,
    /// Mist 0..1, row by row (z then x).
    pub mist: Vec<f32>,
}

impl MistMap {
    /// Bakes the map for a world.
    pub fn bake(cfg: &FogConfig, world: &WorldConfig, terrain: &Heightfield, waters: &Waters) -> Self {
        let cell = cfg.map_cell_m;
        let side = (world.world_size_m / cell).ceil() as usize;
        let origin = -world.world_size_m * 0.5;
        let sea = world.island.sea_level_m;
        let centre = |i: usize, j: usize| Vec2::new(origin + (i as f32 + 0.5) * cell, origin + (j as f32 + 0.5) * cell);
        let ground: Vec<f32> = (0..side * side)
            .map(|k| {
                let p = centre(k % side, k / side);
                terrain.height_at(p.x, p.y)
            })
            .collect();
        let wet: Vec<bool> = (0..side * side)
            .map(|k| {
                let p = centre(k % side, k / side);
                waters.water_at(p, ground[k]).is_some_and(|w| w.surface_m > sea + 0.5)
            })
            .collect();
        let reach = (cfg.hollow_reach_m / cell).round().max(1.0) as i64;
        let water_cells = (cfg.water_reach_m / cell).ceil().max(1.0) as i64;
        let at = |i: i64, j: i64| (j.clamp(0, side as i64 - 1) as usize) * side + i.clamp(0, side as i64 - 1) as usize;
        let mut mist = Vec::with_capacity(side * side);
        for j in 0..side as i64 {
            for i in 0..side as i64 {
                let k = at(i, j);
                // The ground round it: eight points on a ring at the hollow's reach.
                let ring = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];
                let around = ring.iter().map(|(a, b)| ground[at(i + a * reach, j + b * reach)]).sum::<f32>() / 8.0;
                let mut near_water: f32 = 0.0;
                for dj in -water_cells..=water_cells {
                    for di in -water_cells..=water_cells {
                        if wet[at(i + di, j + dj)] {
                            let d = Vec2::new(di as f32, dj as f32).length() * cell;
                            near_water = near_water.max(1.0 - (d / cfg.water_reach_m).min(1.0));
                        }
                    }
                }
                let p = centre(i as usize, j as usize);
                let coast = terrain.shape().past_land_edge_m(p).abs();
                let near_coast = 1.0 - (coast / cfg.coast_reach_m).min(1.0);
                mist.push(mist_at(cfg, ground[k], around, near_water, near_coast, sea));
            }
        }
        Self {
            side,
            origin_m: origin,
            cell_m: cell,
            mist,
        }
    }

    /// The mist at `p`, bilinear between cell centres (0 past the world).
    pub fn at(&self, p: Vec2) -> f32 {
        let f = (p - Vec2::splat(self.origin_m)) / self.cell_m - 0.5;
        let (i0, j0) = (f.x.floor(), f.y.floor());
        let (u, v) = (f.x - i0, f.y - j0);
        let get = |i: f32, j: f32| {
            if i < 0.0 || j < 0.0 || i >= self.side as f32 || j >= self.side as f32 {
                0.0
            } else {
                self.mist[j as usize * self.side + i as usize]
            }
        };
        let top = get(i0, j0) + (get(i0 + 1.0, j0) - get(i0, j0)) * u;
        let bottom = get(i0, j0 + 1.0) + (get(i0 + 1.0, j0 + 1.0) - get(i0, j0 + 1.0)) * u;
        top + (bottom - top) * v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The optical depth by a fine march, for checking the closed form.
    fn marched(cfg: &FogConfig, y0: f32, dy: f32, len: f32) -> f32 {
        let n = 20000;
        let ds = len / n as f32;
        (0..n)
            .map(|k| {
                let y = y0 + dy * (k as f32 + 0.5) * ds;
                cfg.density_per_m * (-(y.max(-200.0)) / cfg.scale_height_m).exp() * ds
            })
            .sum()
    }

    #[test]
    fn the_fog_is_exact_along_a_ray_at_every_angle() {
        let cfg = FogConfig::default();
        for (y0, dy, len) in [
            (2.0, 0.0, 8000.0),
            (2.0, 0.05, 20000.0),
            (300.0, -0.08, 3000.0),
            (2500.0, -0.6, 4000.0),
            (50.0, 0.9, 40000.0),
            (10.0, 0.00005, 5000.0),
        ] {
            let exact = optical_depth(&cfg, 0.0, y0, dy, len, 1.0);
            let m = marched(&cfg, y0, dy, len);
            assert!((exact - m).abs() <= 0.01 * m.max(1e-6), "y {y0} dir {dy} len {len}: {exact} vs {m}");
            assert!(exact.is_finite() && exact >= 0.0);
        }
    }

    #[test]
    fn mist_thickens_the_fog_and_none_forms_up_high() {
        let cfg = FogConfig::default();
        let (thick, plain) = (
            optical_depth(&cfg, 0.0, 5.0, 0.0, 1000.0, 3.0),
            optical_depth(&cfg, 0.0, 5.0, 0.0, 1000.0, 1.0),
        );
        assert!((thick - 3.0 * plain).abs() < 1e-6 * thick);
        // A valley 150 m below the ground round it, by a river, near sea level: full mist.
        assert!(mist_at(&cfg, 80.0, 230.0, 1.0, 0.0, 0.0) > 0.9);
        // The ridge above it: none.
        assert_eq!(mist_at(&cfg, 230.0, 180.0, 0.0, 0.0, 0.0), 0.0);
        // Nothing above the mist's top, hollow or not.
        assert_eq!(mist_at(&cfg, 900.0, 1100.0, 1.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn level_ground_fades_to_half_by_about_eight_kilometres() {
        let cfg = FogConfig::default();
        let t = (-optical_depth(&cfg, 0.0, 2.0, 0.0, 8000.0, 1.0)).exp();
        assert!((0.45..0.6).contains(&t), "{t}");
    }

    #[test]
    fn seed_42s_valleys_are_mistier_than_its_ridges() {
        let cfg = WorldConfig::default();
        let plan = crate::world::generate(42, &cfg);
        let m = &plan.mist;
        assert_eq!(m.mist.len(), m.side * m.side);
        assert!(m.mist.iter().all(|v| (0.0..=1.0).contains(v)));
        // Along each river, the mist over its water beats the mist 1 km off it, on average.
        let (mut on, mut off, mut n) = (0.0, 0.0, 0);
        for r in &plan.waters.rivers {
            for w in r.path.windows(2).step_by(10) {
                let across = (w[1] - w[0]).normalize_or_zero().perp();
                on += m.at(w[0]);
                off += m.at(w[0] + across * 1000.0);
                n += 1;
            }
        }
        assert!(
            n > 100 && on > off * 1.3,
            "on the rivers {} against 1 km off {}",
            on / n as f32,
            off / n as f32
        );
    }

    #[test]
    fn the_morning_mist_thins_by_day_on_the_clock() {
        let cfg = FogConfig::default();
        let at = |s: f32| cfg.strength(None, cfg.clock.hour_at(s));
        // The world begins at dawn in the full morning mist.
        assert_eq!(at(0.0), cfg.presets.morning);
        // It thins as the clock runs, never thickening.
        let mut last = at(0.0);
        for k in 1..=40 {
            let now = at(k as f32 * 60.0);
            assert!(now.density_scale <= last.density_scale && now.mist_scale <= last.mist_scale);
            last = now;
        }
        // By late morning it is the day's, and it stays so.
        assert_eq!(cfg.strength(None, 11.0), cfg.presets.day);
        assert_eq!(cfg.strength(None, 30.0), cfg.presets.day);
        // Half way through the burn-off it is between the two.
        let mid = cfg.strength(None, 9.0);
        assert!(mid.mist_scale < cfg.presets.morning.mist_scale && mid.mist_scale > cfg.presets.day.mist_scale);
    }

    #[test]
    fn combat_heavy_missions_fly_in_clear_air() {
        let cfg = FogConfig::default();
        for m in [MissionKind::Defence, MissionKind::Fortress] {
            // At dawn too: the mission's preset, not the clock.
            assert_eq!(cfg.strength(Some(m), 6.0), cfg.presets.clear);
        }
        assert_eq!(cfg.presets.clear.mist_scale, 0.0);
        assert!(cfg.presets.clear.density_scale < cfg.presets.day.density_scale);
        // The morning mist is thicker than the day's, the day's than clear air.
        let (m, d) = (cfg.presets.morning, cfg.presets.day);
        assert!(m.density_scale > d.density_scale && m.mist_scale > d.mist_scale);
    }

    /// A map of `side` cells of 100 m, misty (1) in x below 0 and clear above.
    fn half_misty(side: usize) -> MistMap {
        let origin = -(side as f32) * 50.0;
        let mist = (0..side * side)
            .map(|k| {
                if origin + ((k % side) as f32 + 0.5) * 100.0 < 0.0 {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        MistMap {
            side,
            origin_m: origin,
            cell_m: 100.0,
            mist,
        }
    }

    #[test]
    fn rain_hazes_a_sight_line_only_as_far_as_the_rain_reaches() {
        let cfg = FogConfig::default();
        let near = rain_depth(&cfg, 1.0, 500.0);
        let reach = rain_depth(&cfg, 1.0, cfg.rain_reach_m);
        let sky = rain_depth(&cfg, 1.0, cfg.sky_length_m);
        assert!(near < reach);
        assert_eq!(sky, reach, "a sky ray is no hazier than the rain's reach");
        // A trace of rain at the eye leaves the sky nearly clear (it hid 6 % of it before, at 40 km).
        assert!((-rain_depth(&cfg, 0.01, cfg.sky_length_m)).exp() > 0.99);
    }

    #[test]
    fn the_haze_blues_the_distance_from_altitude_where_the_low_fog_has_none() {
        // From 1.5 km up, looking 20 km out and gently down, the 120 m fog layer adds almost
        // nothing; the haze still takes a share (design section 2a).
        let cfg = FogConfig::default();
        let dir_y = -1400.0 / 20000.0;
        let fog = optical_depth(&cfg, 0.0, 1500.0, dir_y, 20000.0, 1.0);
        let haze = haze_depth(&cfg, 0.0, 1500.0, dir_y, 20000.0);
        assert!(haze > 0.1 && haze < 1.0, "haze {haze}");
        assert!(haze > 3.0 * fog, "haze {haze} against fog {fog}");
        // Straight up from the ground it thins to almost nothing past a few scale heights.
        assert!(haze_depth(&cfg, 0.0, 0.0, 1.0, 40000.0) < cfg.haze_density_per_m * cfg.haze_scale_height_m * 1.001);
    }

    #[test]
    fn a_line_through_the_mist_is_foggier_than_one_through_clear_air() {
        let cfg = FogConfig::default();
        let map = half_misty(80);
        let eye = Vec3::new(0.0, 10.0, 0.0);
        let s = cfg.presets.morning;
        let misty = line_depth(&cfg, s, &map, 0.0, eye, Vec3::new(-1500.0, 10.0, 0.0), 0.0);
        let clear = line_depth(&cfg, s, &map, 0.0, eye, Vec3::new(1500.0, 10.0, 0.0), 0.0);
        assert!(misty > 3.0 * clear, "{misty} against {clear}");
        // Clear air with no mist: the layer's own integral and the aerial haze, nothing more.
        let plain = optical_depth(&cfg, 0.0, 10.0, 0.0, 1500.0, 1.0) * s.density_scale + haze_depth(&cfg, 0.0, 10.0, 0.0, 1500.0);
        assert!((clear - plain).abs() < 1e-3 * plain.max(1e-6), "{clear} against {plain}");
        // The clear preset lifts the mist off: both ways read alike.
        let c = cfg.presets.clear;
        let a = line_depth(&cfg, c, &map, 0.0, eye, Vec3::new(-1500.0, 10.0, 0.0), 0.0);
        let b = line_depth(&cfg, c, &map, 0.0, eye, Vec3::new(1500.0, 10.0, 0.0), 0.0);
        assert!((a - b).abs() < 1e-5, "{a} against {b}");
        // Rain at the eye thickens it; a line of no length has none.
        assert!(line_depth(&cfg, s, &map, 0.0, eye, Vec3::new(1500.0, 10.0, 0.0), 1.0) > clear + 0.9);
        assert_eq!(line_depth(&cfg, s, &map, 0.0, eye, eye, 1.0), 0.0);
    }

    #[test]
    fn a_far_cloud_is_fogged_by_the_mist_it_is_seen_across_as_the_land_is() {
        // The clouds thicken the layer by the mist read on the level bearing toward them (`thickening`, out to
        // `mist_taps_m`), the fog pass along the sight line itself: over even mist the two agree, and the misty way is
        // the foggier (owner, 2026-10-27: "cloud fog blend should be a thing").
        let cfg = FogConfig::default();
        let map = half_misty(160);
        let s = cfg.presets.morning;
        let eye = Vec3::new(0.0, 80.0, 0.0);
        let depth = |cloud: Vec3, bearing: Vec3| {
            let len = eye.distance(cloud);
            let dir_y = (cloud.y - eye.y) / len;
            let thick = thickening(&cfg, s, &map, eye, bearing, cfg.mist_taps_m);
            optical_depth(&cfg, 0.0, eye.y, dir_y, len, thick) * s.density_scale + haze_depth(&cfg, 0.0, eye.y, dir_y, len)
        };
        let misty_cloud = Vec3::new(-7000.0, 900.0, 0.0);
        let misty = depth(misty_cloud, Vec3::NEG_X);
        let line = line_depth(&cfg, s, &map, 0.0, eye, misty_cloud, 0.0);
        assert!(
            (misty - line).abs() < 1e-3 * line,
            "the cloud's {misty} against the sight line's {line}"
        );
        let clear = depth(Vec3::new(7000.0, 900.0, 0.0), Vec3::X);
        assert!(misty > 1.5 * clear, "{misty} against {clear}");
    }

    #[test]
    fn looking_up_out_of_the_mist_sees_through_less_of_it() {
        let cfg = FogConfig::default();
        let map = half_misty(80);
        let eye = Vec3::new(-2000.0, 5.0, 0.0);
        let s = cfg.presets.morning;
        let level = line_depth(&cfg, s, &map, 0.0, eye, eye + Vec3::new(1000.0, 0.0, 0.0), 0.0);
        let up = line_depth(&cfg, s, &map, 0.0, eye, eye + Vec3::new(800.0, 600.0, 0.0), 0.0);
        assert!(up < level, "{up} against {level}");
    }

    #[test]
    fn the_shipped_fog_is_valid() {
        assert!(FogConfig::default().validate().is_ok());
        let bad = FogConfig {
            phase_g: 1.0,
            ..FogConfig::default()
        };
        assert!(bad.validate().is_err());
        let backwards = FogConfig {
            clock: FogClock {
                burn_off_h: [11.0, 7.0],
                ..FogClock::default()
            },
            ..FogConfig::default()
        };
        assert!(backwards.validate().is_err());
    }
}
