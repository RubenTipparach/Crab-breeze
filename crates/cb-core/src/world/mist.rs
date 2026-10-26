//! Ground fog (openspec change `ground-fog`; owner, 2026-10-25: "think about some cheap volumetric
//! ground/distance fog too", then "where's ground fog?"): a layer of air densest at sea level and
//! thinning with height, worked out exactly along each view ray (no march), thickened where mist
//! forms (hollows below the ground round them, near water, along the coast) and nowhere above
//! 600 m, and greyer under a storm's rain.
//!
//! The rules are here, pure and tested: `mist_at` is the one mist rule (the drawn map is baked from
//! it, `MistMap`), and `optical_depth` is the one fog integral (the compose pass in
//! `assets/shaders/water_post.wesl` repeats it on the GPU; a test checks the closed form against a
//! fine march). The tuning is in `data/world/world.ron` under `fog:`.

use super::config::WorldConfig;
use super::terrain::{Heightfield, smoothstep};
use super::water::Waters;
use crate::data::{Validate, finite, positive, within};
use glam::Vec2;
use serde::Deserialize;

/// The fog's tuning (`fog:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FogConfig {
    /// Extinction at sea level in plain air, per metre (1 / 12,000: level ground fades to half its
    /// colour by about 8 km).
    pub density_per_m: f32,
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
    /// How much of the sky's ambient light the fog scatters back.
    pub ambient_share: f32,
    /// How much of the sun the fog scatters toward the eye, through the phase function.
    pub sun_share: f32,
    /// Henyey-Greenstein asymmetry of the scattering (0 even, toward 1 a glow round the sun).
    pub phase_g: f32,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            density_per_m: 1.0 / 12000.0,
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
            ambient_share: 1.0,
            sun_share: 0.25,
            phase_g: 0.6,
        }
    }
}

impl Validate for FogConfig {
    fn validate(&self) -> Result<(), String> {
        within("fog.density_per_m", self.density_per_m, 0.0, 0.01)?;
        positive("fog.scale_height_m", self.scale_height_m)?;
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
        within("fog.ambient_share", self.ambient_share, 0.0, 4.0)?;
        within("fog.sun_share", self.sun_share, 0.0, 4.0)?;
        within("fog.phase_g", self.phase_g, -0.95, 0.95)
    }
}

/// The optical depth (the extinction summed) along a straight ray from height `from_y_m`, rising
/// `dir_y` a metre (the ray's direction's vertical part), for `length_m`, through the layer of
/// `cfg` thickened `mist` times. The closed form of the integral of
/// `mist * density * exp(-(y - sea) / H)`, written with both ends' exponentials so it neither
/// overflows looking down nor loses precision looking up.
pub fn optical_depth(cfg: &FogConfig, sea_level_m: f32, from_y_m: f32, dir_y: f32, length_m: f32, mist: f32) -> f32 {
    let h = cfg.scale_height_m;
    let e = |y: f32| (-(y - sea_level_m).max(-200.0) / h).exp();
    let e0 = e(from_y_m);
    let length = length_m.max(0.0);
    let base = mist * cfg.density_per_m;
    if dir_y.abs() < 1e-4 {
        return base * e0 * length;
    }
    let e1 = e(from_y_m + dir_y * length);
    base * h * (e0 - e1) / dir_y
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
    fn the_shipped_fog_is_valid() {
        assert!(FogConfig::default().validate().is_ok());
        let bad = FogConfig {
            phase_g: 1.0,
            ..FogConfig::default()
        };
        assert!(bad.validate().is_err());
    }
}
