//! The procedural world: terrain, cities, streets, buildings and highways from a seed and
//! `data/world/world.ron` (openspec change `procgen-world`).
//!
//! `generate` is pure: the same seed and config always give the same `WorldPlan`. `cb-app::world`
//! draws the plan; the pilot and the mech walk on it through `height_at` and `push_out`. The world
//! is an island in the sea (openspec change `island-world`).

pub mod biome;
pub mod clouds;
pub mod collision;
pub mod config;
pub mod cover;
pub mod deform;
pub mod destruct;
pub mod dressing;
pub mod forest;
pub mod grid;
pub mod hydrology;
pub mod interior;
pub mod layout;
pub mod lod;
pub mod structure;
pub mod terrain;
pub mod water;

pub use biome::{BIOME_COUNT, BIOMES, Biome, Biomes};
pub use clouds::Clouds;
pub use collision::BuildingGrid;
pub use config::{DebrisConfig, WorldConfig};
pub use cover::{Clearance, Cover, Surface};
pub use deform::Deformation;
pub use destruct::{Chunk, Moment, Structures};
pub use dressing::DressingConfig;
pub use forest::{Body, Bump, Forest, Push, TileChanges, Tree, TreeState};
pub use layout::{Block, Building, City, Highway, Rect, StreetKind, StreetPiece};
pub use structure::{Frame, StructureConfig};
pub use terrain::{ChunkMesh, Heightfield};
pub use water::{Bridge, Lake, Lakes, River, WaterSample, Waterfall, Waters};

use crate::rng::{SimRng, label_hash};
use glam::Vec2;
use terrain::{Flats, Shape};

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
    /// Buildings as they stand now; `None` means every building is whole.
    pub structures: Option<&'a Structures>,
}

impl Scene<'_> {
    /// Pushes a body at height `[feet_y, feet_y + 12 m]` out of standing building cells.
    fn push_out_buildings(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let Some(st) = self.structures else {
            return self.plan.collision.push_out_at(p, radius_m, feet_y);
        };
        let mut p = p;
        for _ in 0..2 {
            for b in self.plan.collision.candidates(p, radius_m) {
                let cells = &st.buildings[b as usize];
                if cells.standing == 0 {
                    continue;
                }
                let top = cells.origin.y + cells.size.y * cells.n[1] as f32;
                if top <= feet_y + 0.5 {
                    continue;
                }
                if cells.whole() {
                    p = collision::push_out_of_rect(p, radius_m, &self.plan.buildings[b as usize].footprint);
                } else {
                    for r in st.solid_rects(b, feet_y + 0.5, feet_y + 12.0) {
                        p = collision::push_out_of_rect(p, radius_m, &r);
                    }
                }
            }
        }
        p
    }
}

impl Ground for Scene<'_> {
    fn height_at(&self, x: f32, z: f32) -> f32 {
        self.plan.terrain.height_at(x, z) + self.dug.delta_at(Vec2::new(x, z))
    }
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        let ground = self.height_at(p.x, p.y);
        self.push_out_at(p, radius_m, ground)
    }
    fn water_at(&self, x: f32, z: f32) -> Option<WaterSample> {
        self.plan.waters.water_at(Vec2::new(x, z), self.height_at(x, z))
    }
    fn support_at(&self, x: f32, z: f32, feet_y: f32) -> f32 {
        let ground = self.height_at(x, z);
        let p = Vec2::new(x, z);
        let roof = match self.structures {
            None => self.plan.collision.roof_under(p, feet_y),
            Some(st) => self
                .plan
                .collision
                .candidates(p, 0.0)
                .into_iter()
                .filter_map(|b| st.roof_under(b, p, feet_y))
                .reduce(f32::max),
        };
        roof.map_or(ground, |r| r.max(ground))
    }
    fn push_out_at(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let p = self.push_out_buildings(p, radius_m, feet_y);
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
    /// What grows where.
    pub biomes: Biomes,
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
        for l in &self.waters.lakes.list {
            put(l.centre.x);
            put(l.centre.y);
            put(l.radius_m);
            put(l.level_m);
        }
        for f in &self.waters.falls {
            put(f.top_m);
            put(f.bottom_m);
        }
        for iz in (0..self.terrain.n).step_by(97) {
            for ix in (0..self.terrain.n).step_by(97) {
                put(self.terrain.sample(ix, iz));
            }
        }
        put(self.clouds.puffs.len() as f32);
        let mut text = String::with_capacity(words.len() * 8);
        for w in words {
            text.push_str(&w.to_string());
            text.push(',');
        }
        label_hash(&text)
    }
}

/// Builds the world for `seed` (procgen-world design section 2, island-world design section 2).
/// Nothing here walks the whole ground: heights are asked for where they are needed, and trees
/// grow later, by tile, near the bodies in the world (`Forest::keep_around`).
pub fn generate(seed: u64, cfg: &WorldConfig) -> WorldPlan {
    let root = SimRng::new(seed).fork("world");
    let shape = Shape::new(&root, cfg);
    let sea_level_m = cfg.island.sea_level_m;

    // Where the water goes, worked out on the island's shape before anything is built on it.
    let drainage = hydrology::Drainage::new(&shape, cfg.world_size_m, sea_level_m, &cfg.water);

    // A city stands inland, on ground above the sea and below the mountains, its square and blend
    // clear of the shelf, on land that is not too steep, and clear of the rivers and lakes.
    let on_land = |c: Vec2, half: f32| {
        let reach = half + cfg.plateau_flat_margin_m + cfg.plateau_blend_m;
        let ground = shape.height(c);
        if ground < sea_level_m + cfg.water.city_freeboard_m || ground > sea_level_m + cfg.city_max_ground_m {
            return false;
        }
        let probes = [-1.0f32, 0.0, 1.0]
            .into_iter()
            .flat_map(|x| [-1.0f32, 0.0, 1.0].map(|z| Vec2::new(x, z)));
        let level = probes.into_iter().all(|k| {
            let q = c + k * reach;
            shape.past_land_edge_m(q) < 0.0 && (shape.height(q) - ground).abs() <= cfg.city_max_relief_m
        });
        level && !drainage.water_near(c - Vec2::splat(half), c + Vec2::splat(half), cfg.water.river_city_clearance_m)
    };
    let placed = layout::place_cities(&mut root.fork("cities"), cfg, &on_land);
    let cities: Vec<City> = placed
        .iter()
        .enumerate()
        .map(|(i, &(c, half))| layout::lay_out_city(&mut root.fork(&format!("city/{i}")), i as u32, c, half, shape.height(c), cfg))
        .collect();
    // A highway is built only where it does not climb over the mountains (a lake it crosses is
    // left dry: `make_waters` keeps lakes clear of the roads).
    let road_ok = |a: Vec2, b: Vec2, ends_ground_m: [f32; 2]| {
        let top = ends_ground_m[0].max(ends_ground_m[1]) + cfg.highway_max_climb_m;
        let steps = (a.distance(b) / 64.0).ceil().max(1.0) as usize;
        (0..=steps).all(|i| shape.height(a.lerp(b, i as f32 / steps as f32)) <= top)
    };
    let highways = layout::connect_cities(&cities, cfg, &road_ok);

    let largest = placed.first().map(|p| p.1).unwrap_or(1.0);
    let mut buildings = Vec::new();
    for city in &cities {
        let mut rng = root.fork(&format!("city/{}/buildings", city.id));
        let first = buildings.len() as u32;
        buildings.extend(layout::build_city(&mut rng, city, &highways, first, largest, cfg));
    }

    let flats = Flats::new(&cities, &highways, cfg);
    let terrain = Heightfield::new(cfg, shape, flats);
    let bounds = water::WaterBounds {
        world_size_m: cfg.world_size_m,
        sea_level_m,
    };
    let waters = water::make_waters(&root.fork("water"), &drainage, &terrain, &cities, &highways, &cfg.water, &bounds);
    let road_flat_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    let channels = waters.channels(&cfg.water, &cities, cfg.plateau_flat_margin_m, &highways, road_flat_m);
    let terrain = terrain.with_channels(channels);

    let start = start_point(&cities, cfg);
    let roofs = buildings.iter().map(|b| cities[b.city as usize].ground_m + b.height_m).collect();
    let collision = BuildingGrid::new(&buildings, cfg.world_size_m, 64.0).with_roofs(roofs);
    let clouds = clouds::make_clouds(&root.fork("clouds"), &cfg.clouds, cfg.world_size_m);
    let biomes = Biomes::new(&root.fork("biomes"), &cfg.biomes, sea_level_m, cfg.range.snowline_m);
    WorldPlan {
        seed,
        terrain,
        cities,
        highways,
        buildings,
        start,
        collision,
        waters,
        forest: Forest::new(&root.fork("trees"), &cfg.trees, cfg.world_size_m),
        clouds,
        biomes,
    }
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
