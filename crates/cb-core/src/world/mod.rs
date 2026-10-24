//! The procedural world: terrain, cities, streets, buildings and highways from a seed and
//! `data/world/world.ron` (openspec change `procgen-world`).
//!
//! `generate` is pure: the same seed and config always give the same `WorldPlan`. `cb-app::world`
//! draws the plan; the pilot and the mech walk on it through `height_at` and `push_out`.

pub mod clouds;
pub mod collision;
pub mod config;
pub mod deform;
pub mod forest;
pub mod layout;
pub mod terrain;
pub mod water;

pub use clouds::Clouds;
pub use collision::BuildingGrid;
pub use config::WorldConfig;
pub use deform::Deformation;
pub use forest::{Forest, Tree, TreeState};
pub use layout::{Block, Building, City, Highway, Rect, StreetKind, StreetPiece};
pub use terrain::{ChunkMesh, Heightfield};
pub use water::{Bridge, River, WaterSample, Waters};

use crate::rng::{SimRng, label_hash};
use glam::Vec2;
use terrain::{ValueNoise, smoothstep};

/// What a body walking on the world needs from it. The world plan implements it; tests use
/// `FlatGround`. Bodies take this rather than the plan, so the rules do not depend on how the
/// world was made.
pub trait Ground {
    /// Ground height at world `(x, z)`.
    fn height_at(&self, x: f32, z: f32) -> f32;
    /// A body of `radius_m` at `p` (x, z), moved out of walls.
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2;
    /// Water at `(x, z)`, if any. Dry ground by default.
    fn water_at(&self, _x: f32, _z: f32) -> Option<WaterSample> {
        None
    }
    /// What a body with its feet at `feet_y` stands on at `(x, z)`: the ground, or a roof below it.
    fn support_at(&self, x: f32, z: f32, _feet_y: f32) -> f32 {
        self.height_at(x, z)
    }
    /// Like `push_out`, for a body with its feet at `feet_y` (it clears lower roofs).
    fn push_out_at(&self, p: Vec2, radius_m: f32, _feet_y: f32) -> Vec2 {
        self.push_out(p, radius_m)
    }
}

/// The world as it is now: the generated plan plus the ground dug since, and optionally the trees
/// standing now (as walls). The one `Ground` the game's bodies walk on.
pub struct Scene<'a> {
    /// The generated world.
    pub plan: &'a WorldPlan,
    /// Craters dug since.
    pub dug: &'a Deformation,
    /// Trees as walls (the pilot); `None` for bodies that break trees themselves (the mech).
    pub trees: Option<&'a Forest>,
}

impl Ground for Scene<'_> {
    fn height_at(&self, x: f32, z: f32) -> f32 {
        self.plan.terrain.height_at(x, z) + self.dug.delta_at(Vec2::new(x, z))
    }
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        let p = self.plan.collision.push_out(p, radius_m);
        self.trees.map_or(p, |t| t.push_out(p, radius_m))
    }
    fn water_at(&self, x: f32, z: f32) -> Option<WaterSample> {
        self.plan.waters.water_at(Vec2::new(x, z), self.height_at(x, z))
    }
    fn support_at(&self, x: f32, z: f32, feet_y: f32) -> f32 {
        let ground = self.height_at(x, z);
        self.plan
            .collision
            .roof_under(Vec2::new(x, z), feet_y)
            .map_or(ground, |r| r.max(ground))
    }
    fn push_out_at(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let p = self.plan.collision.push_out_at(p, radius_m, feet_y);
        self.trees.map_or(p, |t| t.push_out(p, radius_m))
    }
}

impl Ground for WorldPlan {
    fn height_at(&self, x: f32, z: f32) -> f32 {
        self.terrain.height_at(x, z)
    }
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        self.collision.push_out(p, radius_m)
    }
    fn water_at(&self, x: f32, z: f32) -> Option<WaterSample> {
        self.waters.water_at(Vec2::new(x, z), self.terrain.height_at(x, z))
    }
}

/// Endless flat ground at height zero with optional walls, for tests.
#[derive(Clone, Debug, Default)]
pub struct FlatGround {
    /// Building collision; empty for open ground.
    pub walls: BuildingGrid,
}

impl Ground for FlatGround {
    fn height_at(&self, _x: f32, _z: f32) -> f32 {
        0.0
    }
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        if self.walls == BuildingGrid::default() {
            p
        } else {
            self.walls.push_out(p, radius_m)
        }
    }
}

/// Where the game starts: the mech in a street of the largest city, the pilot behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    /// The mech's root on the ground (x, z).
    pub mech_xz: Vec2,
    /// The mech's heading, radians, 0 facing -z, positive turning left.
    pub mech_heading_rad: f32,
    /// The pilot's feet (x, z).
    pub pilot_xz: Vec2,
}

/// A generated world.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldPlan {
    /// The seed it was built from.
    pub seed: u64,
    /// The ground.
    pub terrain: Heightfield,
    /// Cities, largest first.
    pub cities: Vec<City>,
    /// Highways between cities.
    pub highways: Vec<Highway>,
    /// Every building, by id.
    pub buildings: Vec<Building>,
    /// Where the game starts.
    pub start: Start,
    /// Building collision.
    pub collision: BuildingGrid,
    /// Rivers, lakes and bridges.
    pub waters: Waters,
    /// The trees as grown (their state as the game goes on is a copy the game keeps).
    pub forest: Forest,
    /// The sky's clouds.
    pub clouds: Clouds,
}

impl WorldPlan {
    /// A stable 64-bit summary of the plan, quantised to centimetres, for golden-seed tests.
    pub fn fingerprint(&self) -> u64 {
        let mut words: Vec<i64> = Vec::new();
        let mut put = |v: f32| words.push((v * 100.0).round() as i64);
        for c in &self.cities {
            put(c.centre.x);
            put(c.centre.y);
            put(c.half_size_m);
            put(c.ground_m);
            for s in &c.streets {
                put(s.rect.min.x);
                put(s.rect.min.y);
                put(s.rect.max.x);
                put(s.rect.max.y);
            }
        }
        for h in &self.highways {
            put(h.ends[0].x);
            put(h.ends[0].y);
            put(h.ends[1].x);
            put(h.ends[1].y);
        }
        for b in &self.buildings {
            put(b.footprint.min.x);
            put(b.footprint.min.y);
            put(b.footprint.max.x);
            put(b.footprint.max.y);
            put(b.storeys as f32);
        }
        put(self.waters.sea_level_m);
        for r in &self.waters.rivers {
            for (p, s) in r.path.iter().zip(&r.surface_m) {
                put(p.x);
                put(p.y);
                put(*s);
            }
        }
        for h in self.terrain.heights_m.iter().step_by(97) {
            put(*h);
        }
        let mut text = String::with_capacity(words.len() * 8);
        for w in words {
            text.push_str(&w.to_string());
            text.push(',');
        }
        label_hash(&text)
    }
}

/// Builds the world for `seed` (procgen-world design section 2).
pub fn generate(seed: u64, cfg: &WorldConfig) -> WorldPlan {
    let root = SimRng::new(seed).fork("world");
    let noise = ValueNoise::new(&root.fork("terrain"));
    let base = |p: Vec2| {
        noise.height(
            p.x,
            p.y,
            cfg.noise_wavelength_m,
            cfg.noise_amplitude_m,
            cfg.noise_octaves,
            cfg.noise_persistence,
        )
    };

    let placed = layout::place_cities(&mut root.fork("cities"), cfg);
    let cities: Vec<City> = placed
        .iter()
        .enumerate()
        .map(|(i, &(c, half))| layout::lay_out_city(&mut root.fork(&format!("city/{i}")), i as u32, c, half, base(c), cfg))
        .collect();
    let highways = layout::connect_cities(&cities, cfg);

    let largest = placed.first().map(|p| p.1).unwrap_or(1.0);
    let mut buildings = Vec::new();
    for city in &cities {
        let mut rng = root.fork(&format!("city/{}/buildings", city.id));
        let first = buildings.len() as u32;
        buildings.extend(layout::build_city(&mut rng, city, &highways, first, largest, cfg));
    }

    let n = cfg.samples_per_side();
    let mut terrain = Heightfield {
        n,
        cell_m: cfg.terrain_cell_m,
        origin_m: -cfg.world_size_m * 0.5,
        heights_m: Vec::with_capacity(n * n),
    };
    let flat_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    for iz in 0..n {
        for ix in 0..n {
            let p = terrain.sample_xz(ix, iz);
            let mut h = base(p);
            for c in &cities {
                let d = c.rect().distance(p) - cfg.plateau_flat_margin_m;
                let w = 1.0 - smoothstep(0.0, cfg.plateau_blend_m, d);
                h += (c.ground_m - h) * w;
            }
            for hw in &highways {
                let (d, t) = hw.project(p);
                let w = 1.0 - smoothstep(0.0, cfg.highway_blend_m, d - flat_m);
                h += (hw.grade_at(t) - h) * w;
            }
            terrain.heights_m.push(h);
        }
    }

    let waters = water::make_waters(
        &root.fork("water"),
        &mut terrain,
        &cities,
        &highways,
        &cfg.water,
        cfg.world_size_m,
        cfg.plateau_flat_margin_m,
    );

    let start = start_point(&cities, cfg);
    let roofs = buildings.iter().map(|b| cities[b.city as usize].ground_m + b.height_m).collect();
    let collision = BuildingGrid::new(&buildings, cfg.world_size_m, 64.0).with_roofs(roofs);
    let clouds = clouds::make_clouds(&root.fork("clouds"), &cfg.clouds, cfg.world_size_m);
    let mut plan = WorldPlan {
        seed,
        terrain,
        cities,
        highways,
        buildings,
        start,
        collision,
        waters,
        forest: Forest::default(),
        clouds,
    };
    plan.forest = forest::grow(&root.fork("trees"), &plan, &cfg.trees, cfg.world_size_m);
    plan
}

/// The street piece running along z nearest the largest city's centre (ties to the first).
fn start_point(cities: &[City], cfg: &WorldConfig) -> Start {
    let Some(city) = cities.first() else {
        return Start {
            mech_xz: Vec2::ZERO,
            mech_heading_rad: 0.0,
            pilot_xz: Vec2::new(0.0, cfg.start_pilot_distance_m),
        };
    };
    let along_z = city.streets.iter().filter(|s| s.rect.size().y > s.rect.size().x);
    let mut best: Option<(f32, Vec2)> = None;
    for s in along_z {
        let c = s.rect.centre();
        let d = c.distance(city.centre);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, c));
        }
    }
    let mech_xz = best.map(|b| b.1).unwrap_or(city.centre);
    Start {
        mech_xz,
        mech_heading_rad: 0.0,
        pilot_xz: mech_xz + Vec2::new(0.0, cfg.start_pilot_distance_m),
    }
}

#[cfg(test)]
mod tests;
