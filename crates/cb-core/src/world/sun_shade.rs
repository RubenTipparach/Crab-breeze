//! The terrain's own shadow for the sun, baked once (openspec change `distant-shadows`): far ground,
//! drawn past the live shadows, reads it to show ridges shading the valleys behind them (owner,
//! 2026-10-25: "distant terrain shadows can be cached", "when I walk up to it within like 500m or so
//! then it can render as normally"). The sun is fixed, so the shadow is too. Pure.

use super::terrain::Heightfield;
use crate::data::{Validate, positive, within};
use glam::{EulerRot, Quat, Vec2, Vec3};
use serde::Deserialize;

/// The game's one sun: its light's rotation (the light shines along its -z). `cb-app` lights the
/// world with it and the bake shades the terrain from it, so the two always agree.
pub fn sun_rotation() -> Quat {
    Quat::from_euler(EulerRot::YXZ, 0.8, -0.75, 0.0)
}

/// The direction toward the sun.
pub fn to_sun() -> Vec3 {
    sun_rotation() * Vec3::Z
}

/// The bake in data, `sun_shade:` in `data/world/world.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SunShadeConfig {
    /// The shadow grid's cell, metres.
    pub cell_m: f32,
    /// Ground this far under the shadow's edge is fully in shade (a soft edge, not a stair), metres.
    pub soft_m: f32,
    /// Far ground in shade keeps this share of its colour (the sun's share of its light is gone).
    pub shade_albedo: f32,
    /// The live shadow maps near the eye: how many cascades.
    pub live_cascades: u32,
    /// Where the first cascade ends, metres from the eye.
    pub live_first_cascade_m: f32,
    /// Where the live shadows end, metres from the eye: about where far ground, wearing the baked
    /// shadow, begins.
    pub live_far_m: f32,
}

impl Default for SunShadeConfig {
    fn default() -> Self {
        Self {
            cell_m: 40.0,
            soft_m: 4.0,
            shade_albedo: 0.55,
            live_cascades: 3,
            live_first_cascade_m: 40.0,
            live_far_m: 700.0,
        }
    }
}

impl Validate for SunShadeConfig {
    fn validate(&self) -> Result<(), String> {
        positive("sun_shade.cell_m", self.cell_m)?;
        positive("sun_shade.soft_m", self.soft_m)?;
        within("sun_shade.shade_albedo", self.shade_albedo, 0.0, 1.0)?;
        if !(1..=4).contains(&self.live_cascades) {
            return Err(format!("sun_shade.live_cascades: {} is not 1 to 4", self.live_cascades));
        }
        positive("sun_shade.live_first_cascade_m", self.live_first_cascade_m)?;
        positive("sun_shade.live_far_m", self.live_far_m)?;
        if self.live_first_cascade_m >= self.live_far_m {
            return Err("sun_shade.live_first_cascade_m must be under live_far_m".into());
        }
        Ok(())
    }
}

/// The ground's heights at the corners of the shadow grid's cells, `n + 1` a side.
struct Corners {
    n: usize,
    h: Vec<f32>,
}

impl Corners {
    fn sample(terrain: &Heightfield, n: usize, origin_m: f32, cell_m: f32) -> Self {
        let ratio = cell_m / terrain.cell_m;
        let offset = (origin_m - terrain.origin_m) / terrain.cell_m;
        let aligned = (ratio - ratio.round()).abs() < 1e-4 && (offset - offset.round()).abs() < 1e-4;
        let (ratio, offset) = (ratio.round() as i64, offset.round() as i64);
        let side = n + 1;
        let mut h = Vec::with_capacity(side * side);
        for j in 0..side {
            for i in 0..side {
                h.push(if aligned {
                    terrain.lattice_at_step(offset + i as i64 * ratio, offset + j as i64 * ratio, 1)
                } else {
                    terrain.height_at(origin_m + i as f32 * cell_m, origin_m + j as f32 * cell_m)
                });
            }
        }
        Self { n, h }
    }

    /// The height at grid position `(gx, gz)` (cells from the grid's origin, inside it), bilinear.
    fn at(&self, gx: f32, gz: f32) -> f32 {
        let (fx, fz) = (gx.floor(), gz.floor());
        let (i, j) = ((fx as usize).min(self.n - 1), (fz as usize).min(self.n - 1));
        let (u, v) = (gx - i as f32, gz - j as f32);
        let side = self.n + 1;
        let k = j * side + i;
        let (a, b, c, d) = (self.h[k], self.h[k + 1], self.h[k + side], self.h[k + side + 1]);
        (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
    }
}

/// The shadow grid: light from 0 (in shade) to 255 (sunlit), `n` cells a side from `origin_m`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SunShade {
    n: usize,
    origin_m: f32,
    cell_m: f32,
    albedo: f32,
    light: Vec<u8>,
}

impl SunShade {
    /// Bakes the shadow `terrain` casts for a sun toward `to_sun`, over a square `size_m` across centred
    /// on the origin: lines along the sun's way across the ground, each walked from the sun's side,
    /// carrying the height the shadow stands at (the highest ground behind, falling by the sun's
    /// slope per metre). The lines read a grid of the ground's heights at the cell corners, sampled
    /// once (the terrain's own lattice where the cells line up with it), not the terrain itself.
    pub fn bake(terrain: &Heightfield, size_m: f32, to_sun: Vec3, cfg: &SunShadeConfig) -> Self {
        let n = (size_m / cfg.cell_m).ceil().max(1.0) as usize;
        let origin_m = -size_m * 0.5;
        let mut light = vec![255u8; n * n];
        let flat = Vec2::new(to_sun.x, to_sun.z);
        let elevation = to_sun.y.clamp(-1.0, 1.0).asin();
        if flat.length() < 1e-4 || elevation <= 0.0 {
            return Self {
                n,
                origin_m,
                cell_m: cfg.cell_m,
                albedo: cfg.shade_albedo,
                light,
            };
        }
        let heights = Corners::sample(terrain, n, origin_m, cfg.cell_m);
        // Light travels away from the sun: walk that way.
        let along = -flat.normalize();
        let across = along.perp();
        // Metres the shadow's height falls per metre walked away from the sun.
        let drop = elevation.tan();
        let step = cfg.cell_m * 0.5;
        let reach = size_m * 0.75;
        let lines = (2.0 * reach / step) as i32;
        for k in 0..=lines {
            let v = -reach + k as f32 * step;
            let mut shadow = f32::NEG_INFINITY;
            for s in 0..=lines {
                let u = -reach + s as f32 * step;
                let p = along * u + across * v;
                shadow -= drop * step;
                let (gx, gz) = ((p.x - origin_m) / cfg.cell_m, (p.y - origin_m) / cfg.cell_m);
                if gx < 0.0 || gz < 0.0 || gx >= n as f32 || gz >= n as f32 {
                    continue;
                }
                let h = heights.at(gx, gz);
                let lit = ((h - shadow + cfg.soft_m) / cfg.soft_m).clamp(0.0, 1.0);
                let cell = gz as usize * n + gx as usize;
                light[cell] = light[cell].min((lit * 255.0) as u8);
                shadow = shadow.max(h);
            }
        }
        Self {
            n,
            origin_m,
            cell_m: cfg.cell_m,
            albedo: cfg.shade_albedo,
            light,
        }
    }

    /// What far ground in shade keeps of its colour (`SunShadeConfig::shade_albedo`).
    pub fn shade_albedo(&self) -> f32 {
        self.albedo
    }

    /// How sunlit the ground at `xz` is, 0 in shade to 1 lit (bilinear; lit past the grid).
    pub fn at(&self, xz: Vec2) -> f32 {
        if self.n == 0 {
            return 1.0;
        }
        let g = (xz - Vec2::splat(self.origin_m)) / self.cell_m - Vec2::splat(0.5);
        let (fx, fz) = (g.x.floor(), g.y.floor());
        let (u, v) = (g.x - fx, g.y - fz);
        let cell = |x: f32, z: f32| {
            if x < 0.0 || z < 0.0 || x >= self.n as f32 || z >= self.n as f32 {
                1.0
            } else {
                f32::from(self.light[z as usize * self.n + x as usize]) / 255.0
            }
        };
        let (a, b, c, d) = (cell(fx, fz), cell(fx + 1.0, fz), cell(fx, fz + 1.0), cell(fx + 1.0, fz + 1.0));
        (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
    }
}
