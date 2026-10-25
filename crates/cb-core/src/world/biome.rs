//! What grows where on the island (openspec change `island-biomes`): a moisture field drying
//! toward the island's lee side, and from it, with height and slope, a weight for each biome. The
//! one biome rule: the ground's colour, the grass, the trees and the map all ask it.
//!
//! Every border is a smoothstep over a band of a field that itself wanders, so no border is a
//! straight line and every change is gradual (owner, 2026-10-25: "Make sure the transition is more
//! gradually instead of hard borders"). The desert covers about a quarter of the land ("Desert
//! should be 1/4 the island"), pinned by a test.

use super::config::BiomeConfig;
use super::terrain::{ValueNoise, smoothstep};
use crate::rng::SimRng;
use glam::Vec2;

/// The biomes, in the order `Biomes::weights` gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    /// Snow above the snowline.
    Snow,
    /// Bare rock: cliffs, steep slopes and the high ground below the snow.
    Rock,
    /// Conifer forest on the range's moist flanks.
    Montane,
    /// Grassland and broadleaf woods.
    Temperate,
    /// Dry grass with scattered trees.
    Savanna,
    /// Sand and scrub.
    Desert,
    /// Sand along the shore.
    Beach,
    /// Wet low ground: reeds and marsh.
    Wetland,
}

/// How many biomes there are.
pub const BIOME_COUNT: usize = 8;

/// Every biome, in weight order.
pub const BIOMES: [Biome; BIOME_COUNT] = [
    Biome::Snow,
    Biome::Rock,
    Biome::Montane,
    Biome::Temperate,
    Biome::Savanna,
    Biome::Desert,
    Biome::Beach,
    Biome::Wetland,
];

/// Share of full grass each biome grows.
const GRASS: [f32; BIOME_COUNT] = [0.0, 0.0, 0.6, 1.0, 0.75, 0.04, 0.0, 0.9];
/// Trees in each biome, as a share of the forest rules' chance.
const TREES: [f32; BIOME_COUNT] = [0.0, 0.05, 1.0, 1.0, 0.2, 0.0, 0.0, 0.7];

/// The island's biome rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Biomes {
    cfg: BiomeConfig,
    wander: ValueNoise,
    patches: ValueNoise,
    sea_level_m: f32,
    snowline_m: f32,
}

impl Biomes {
    /// The rules for a world, from its stream.
    pub fn new(rng: &SimRng, cfg: &BiomeConfig, sea_level_m: f32, snowline_m: f32) -> Self {
        Self {
            cfg: cfg.clone(),
            wander: ValueNoise::new(&rng.fork("wander")),
            patches: ValueNoise::new(&rng.fork("patches")),
            sea_level_m,
            snowline_m,
        }
    }

    /// Moisture at `p`, 0 (desert) to 1 (marsh): falling toward `dry_toward` across the island,
    /// along an axis pushed about by noise so the dry side's edge meanders, plus patchy noise.
    pub fn moisture(&self, p: Vec2) -> f32 {
        let c = &self.cfg;
        let toward = Vec2::from(c.dry_toward).normalize_or(Vec2::X);
        let wander = c.wander_m * self.wander.unit(p.x, p.y, c.wander_wavelength_m, 3, 0.5);
        let dry = smoothstep(c.dry_from_m, c.dry_to_m, p.dot(toward) + wander);
        let patch = self.patches.unit(p.x, p.y, c.patch_wavelength_m, 3, 0.5);
        ((1.0 - dry) * (1.0 - c.patch_share) + c.patch_share * (0.5 + 0.5 * patch)).clamp(0.0, 1.0)
    }

    /// The weight of each biome at `p` on ground `ground_m` high with slope `slope` (rise over
    /// run), summing to one, in `BIOMES` order.
    pub fn weights(&self, p: Vec2, ground_m: f32, slope: f32) -> [f32; BIOME_COUNT] {
        let c = &self.cfg;
        let m = self.moisture(p);
        let jitter = c.height_jitter_m * self.wander.unit(p.x + 5100.0, p.y - 3300.0, 1500.0, 3, 0.5);
        let h = ground_m + jitter;
        let snow_line = self.snowline_m;
        let dry = c.desert_moisture;
        let band = c.border_band;

        let snow = smoothstep(snow_line - 40.0, snow_line + 60.0, h) * (1.0 - smoothstep(c.rock_slope[1], c.rock_slope[1] * 1.6, slope));
        let steep = smoothstep(c.rock_slope[0], c.rock_slope[1], slope);
        let alpine = smoothstep(snow_line - c.alpine_band_m, snow_line, h);
        let rock = (steep + 0.7 * alpine) * (1.0 - snow);
        let high = smoothstep(c.montane_m[0], c.montane_m[1], h);
        let montane = high * (1.0 - alpine) * smoothstep(dry, dry + band, m);
        let low = (1.0 - snow) * (1.0 - rock.min(1.0)) * (1.0 - montane);
        let desert = 1.0 - smoothstep(dry - band, dry + 0.3 * band, m);
        let savanna = smoothstep(dry - band, dry + 0.3 * band, m) * (1.0 - smoothstep(dry + band, dry + 3.0 * band, m));
        let temperate = smoothstep(dry + band, dry + 3.0 * band, m);
        let wet = smoothstep(c.wetland_moisture, c.wetland_moisture + band, m) * (1.0 - smoothstep(6.0, 25.0, ground_m - self.sea_level_m));
        let beach = (1.0 - smoothstep(c.beach_m * 0.5, c.beach_m, ground_m - self.sea_level_m)) * (1.0 - steep);

        let mut w = [
            snow,
            rock,
            montane,
            low * temperate * (1.0 - wet),
            low * savanna,
            low * desert,
            beach * 3.0,
            low * temperate * wet,
        ];
        let sum: f32 = w.iter().map(|v| v.max(0.0)).sum();
        if sum < 1e-4 {
            return [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        }
        for v in &mut w {
            *v = v.max(0.0) / sum;
        }
        w
    }

    /// The strongest biome at a point.
    pub fn dominant(&self, p: Vec2, ground_m: f32, slope: f32) -> Biome {
        let w = self.weights(p, ground_m, slope);
        let mut best = 0;
        for i in 1..BIOME_COUNT {
            if w[i] > w[best] {
                best = i;
            }
        }
        BIOMES[best]
    }

    /// Share of full grass the biomes at a point grow, 0..1.
    pub fn grass_share(&self, p: Vec2, ground_m: f32, slope: f32) -> f32 {
        blend(&self.weights(p, ground_m, slope), &GRASS)
    }

    /// Trees at a point, as a share of the forest rules' chance.
    pub fn tree_share(&self, p: Vec2, ground_m: f32, slope: f32) -> f32 {
        blend(&self.weights(p, ground_m, slope), &TREES)
    }
}

/// The weights' blend of a per-biome value.
fn blend(w: &[f32; BIOME_COUNT], v: &[f32; BIOME_COUNT]) -> f32 {
    w.iter().zip(v).map(|(a, b)| a * b).sum()
}
