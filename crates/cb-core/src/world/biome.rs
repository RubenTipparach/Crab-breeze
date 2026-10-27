//! What grows where on the island (openspec change `island-biomes`): the island mockup's ten biomes
//! (`docs/mockups/island_gen.js`, `moisture` and `biomeWeights`, ported line for line). Moisture
//! falls across a 12 km band east of the range's spine (the westerlies' rain shadow), wetter to
//! the south; with height, slope, the coast, the canyons, the islets and the swamps at the western
//! river mouths it gives a weight for each biome. The one biome rule: the ground's colour, the
//! grass, the trees and the map all ask it.
//!
//! Every border is a smoothstep over a band of a field that itself wanders, so no border is a
//! straight line (owner, 2026-10-25: "Make sure the transition is more gradually instead of hard
//! borders"). The desert and the canyons cover about a quarter of the land ("Desert should be 1/4
//! the island"), pinned by a test.

use super::config::BiomeConfig;
use super::island::smooth;
use super::terrain::Shape;
use glam::Vec2;

impl Biome {
    /// The biome called `name`, as its variant is spelt (`Temperate`, `Desert`...), for data files.
    pub fn named(name: &str) -> Option<Biome> {
        BIOMES.into_iter().find(|b| format!("{b:?}") == name)
    }
}

/// The biomes, in the order `Biomes::weights` gives them (the mockup's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    /// Snow and bare rock high on the range.
    Alpine,
    /// Conifer forest on the range's moist flanks.
    Montane,
    /// Tall dense forest on the wet western and southern lowlands.
    Jungle,
    /// Flat wetland at the western river mouths.
    Swamp,
    /// Grassland and broadleaf woods.
    Temperate,
    /// Dry grass with scattered trees.
    Savanna,
    /// Sand and scrub in the rain shadow.
    Desert,
    /// The canyons cut into the desert plateau.
    Canyon,
    /// Sand along the shore.
    Beach,
    /// Cliffs and the steepest slopes.
    Cliff,
}

/// How many biomes there are.
pub const BIOME_COUNT: usize = 10;

/// Every biome, in weight order.
pub const BIOMES: [Biome; BIOME_COUNT] = [
    Biome::Alpine,
    Biome::Montane,
    Biome::Jungle,
    Biome::Swamp,
    Biome::Temperate,
    Biome::Savanna,
    Biome::Desert,
    Biome::Canyon,
    Biome::Beach,
    Biome::Cliff,
];

/// Share of full grass each biome grows.
const GRASS: [f32; BIOME_COUNT] = [0.0, 0.6, 0.55, 0.9, 1.0, 0.75, 0.04, 0.0, 0.0, 0.0];
/// Trees in each biome, as a share of the forest rules' chance.
const TREES: [f32; BIOME_COUNT] = [0.02, 1.0, 1.0, 0.7, 0.8, 0.2, 0.02, 0.0, 0.03, 0.0];

/// A swamp at a western river mouth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swamp {
    /// Its centre (x, z).
    pub centre: Vec2,
    /// Its radius.
    pub radius_m: f32,
}

/// The island's biome rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Biomes {
    cfg: BiomeConfig,
    shape: Shape,
    sea_level_m: f32,
    swamps: Vec<Swamp>,
}

/// Slope in degrees from rise over run.
fn degrees(slope: f32) -> f64 {
    f64::from(slope.max(0.0)).atan().to_degrees()
}

impl Biomes {
    /// The rules for an island of `shape`, with swamps at `swamps`.
    pub fn new(cfg: &BiomeConfig, shape: Shape, sea_level_m: f32, swamps: Vec<Swamp>) -> Self {
        Self {
            cfg: cfg.clone(),
            shape,
            sea_level_m,
            swamps,
        }
    }

    /// The swamps.
    pub fn swamps(&self) -> &[Swamp] {
        &self.swamps
    }

    /// How much of a swamp `p` is in, 0..1, on ground `h` above the sea `coast_m` inland.
    fn swamp_at(&self, p: Vec2, h: f64, coast_m: f64) -> f64 {
        self.swamps.iter().fold(0.0, |m, s| {
            let r = f64::from(s.radius_m);
            let d = f64::from(p.distance(s.centre));
            m.max((1.0 - smooth(r * 0.5, r, d)) * smooth(34.0, 6.0, h) * smooth(-150.0, 200.0, coast_m))
        })
    }

    /// Moisture at `p`, 0 (desert) to 1 (rain forest), and how far south it is (0..1): the rain
    /// shadow east of the spine over a 12 km band, wetter to the south, wetter in the swamps, with
    /// 3 km noise; its inputs warped by 900 m of noise so the bands meander.
    fn moisture_south(&self, p: Vec2, perp: f64, swamp: f64) -> (f64, f64) {
        let i = self.shape.island();
        let (x, z) = (f64::from(p.x), f64::from(p.y));
        let ws = perp + 900.0 * i.fbm(x / 2600.0, z / 2600.0, 3, 22);
        let south = smooth(-9000.0, 4000.0, z + 1500.0 * i.fbm(x / 5000.0, z / 5000.0, 2, 24));
        let mut m = 1.0 - smooth(-2000.0, 10000.0, ws);
        m = m * (0.72 + 0.36 * south) + 0.22 * i.fbm(x / 3000.0, z / 3000.0, 3, 21) + 0.25 * swamp;
        (m.clamp(0.0, 1.0), south)
    }

    /// Moisture at `p`, 0 (desert) to 1 (rain forest).
    pub fn moisture(&self, p: Vec2) -> f32 {
        let (h, f) = self.shape.height_and_fields(p);
        let swamp = self.swamp_at(p, f64::from(h - self.sea_level_m), f64::from(f.coast_m));
        self.moisture_south(p, f.spine.perp, swamp).0 as f32
    }

    /// The weight of each biome at `p` on ground `ground_m` high with slope `slope` (rise over
    /// run), summing to one, in `BIOMES` order.
    pub fn weights(&self, p: Vec2, ground_m: f32, slope: f32) -> [f32; BIOME_COUNT] {
        let i = self.shape.island();
        let (_, f) = self.shape.height_and_fields(p);
        let (x, z) = (f64::from(p.x), f64::from(p.y));
        let h = f64::from(ground_m - self.sea_level_m);
        let cd = f64::from(f.coast_m);
        let cliff_w = f64::from(f.cliff_coast);
        let swamp = self.swamp_at(p, h, cd);
        let (m, south) = self.moisture_south(p, f.spine.perp, swamp);
        let dry = f64::from(self.cfg.desert_moisture);
        let j = 220.0 * i.fbm(x / 1500.0, z / 1500.0, 3, 20);
        let ht = h + j;
        let steep = smooth(40.0, 50.0, degrees(slope));
        let coast_low = smooth(480.0, 60.0, cd + j * 0.5) * smooth(9.0, 2.5, h) * (1.0 - cliff_w);
        let isle = if f.islet_q < 1.2 { 1.0 } else { 0.0 };
        let low = smooth(540.0, 360.0, ht);
        let mid = smooth(360.0, 540.0, ht) * (1.0 - smooth(1500.0, 1750.0, ht));
        let canyon = smooth(25.0, 70.0, f64::from(f.canyon_m));
        let mut w = [
            smooth(1450.0, 1750.0, ht) + smooth(1000.0, 1250.0, h) * steep,
            mid * smooth(0.25, 0.45, m),
            low * smooth(0.6, 0.8, m) * (0.3 + 0.7 * south) + isle * 0.6,
            swamp * 2.0,
            low * smooth(dry + 0.06, dry + 0.32, m) * (1.0 - smooth(0.68, 0.86, m * (0.55 + 0.45 * south))),
            (low + 0.6 * mid) * smooth(dry - 0.12, dry + 0.06, m) * (1.0 - smooth(dry + 0.14, dry + 0.4, m)),
            smooth(dry + 0.06, dry - 0.14, m) * (1.0 - smooth(1100.0, 1400.0, ht)) * (1.0 - canyon),
            canyon * 1.5,
            coast_low * 2.2,
            steep * 1.2 * smooth(800.0, 1200.0, 2400.0 - h),
        ];
        // Under the sea: sand (the beach's layers), not whatever the land rules leave.
        if h < 0.5 {
            let sub = smooth(0.5, -1.5, h);
            for v in &mut w {
                *v *= 1.0 - sub;
            }
            w[8] += 3.0 * sub;
        }
        let sum: f64 = w.iter().map(|v| v.max(0.0)).sum();
        let mut out = [0.0f32; BIOME_COUNT];
        if sum < 1e-4 {
            out[4] = 1.0;
            return out;
        }
        for (o, v) in out.iter_mut().zip(w) {
            *o = (v.max(0.0) / sum) as f32;
        }
        out
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

/// Swamps at the mouths of the rivers that reach the sea on the west (bearings 200-320 degrees
/// from the centre), at least 3.5 km apart, as the mockup lays them.
pub fn swamps_at_mouths(mouths: impl IntoIterator<Item = Vec2>, radius_m: f32) -> Vec<Swamp> {
    let mut out: Vec<Swamp> = Vec::new();
    for m in mouths {
        let b = m.x.atan2(-m.y).to_degrees().rem_euclid(360.0);
        if (200.0..320.0).contains(&b) && !out.iter().any(|s| s.centre.distance(m) < 3500.0) {
            out.push(Swamp { centre: m, radius_m });
        }
    }
    out
}
