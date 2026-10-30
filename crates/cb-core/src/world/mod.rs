//! The procedural world: terrain, cities, streets, buildings and highways from a seed and
//! `data/world/world.ron` (openspec change `procgen-world`).
//!
//! `generate` is pure: the same seed and config always give the same `WorldPlan`. `cb-app::world`
//! draws the plan; the pilot and the mech walk on it through `height_at` and `push_out`. The world
//! is an island in the sea (openspec change `island-world`).

pub mod approach;
pub mod bases;
pub mod biome;
pub mod clouds;
pub mod collision;
pub mod config;
pub mod cover;
pub mod deform;
pub mod destruct;
pub mod door;
pub mod dressing;
pub mod far_facade;
pub mod forest;
pub mod grid;
pub mod hydrology;
pub mod interior;
pub mod island;
pub mod lamps;
pub mod lanes;
pub mod layout;
pub mod lod;
pub mod map;
pub mod mist;
pub mod platform;
pub mod roads;
pub mod rocks;
pub mod sites;
pub mod small_buildings;
pub mod structure;
pub mod sun_shade;
pub mod terrain;
pub mod water;
pub mod weather;

pub use bases::{AirBase, BasesConfig};
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
pub use platform::{Platform, Slab};
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

/// The farthest a facade's clear distance is looked for (`WorldPlan::fronts`): past it, a facade
/// faces open street.
pub const FRONT_LIMIT_M: f32 = 40.0;

/// How far outside a footprint its colliders may hold something to stand on (a stoop).
const STOOP_REACH_M: f32 = 2.0;

/// Trees push a body whose feet are no higher than this over the ground under it.
const TREE_REACH_M: f32 = 30.0;

/// A site's solid (a quay) rising more than this over a body's feet is a wall to it (a quay's face from the water); its
/// top is stood on as a fixed platform (`cb-app::boats`).
const SITE_STEP_M: f32 = 1.5;

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
    /// Moving surfaces bodies stand on (the Stratostacean's deck: `platform`); empty for none.
    pub platforms: &'a [Platform],
    /// The detailed colliders of buildings near a pilot on foot (`interior::Interiors`); `None` for
    /// everything else (the mech, shells, debris), which meets buildings as their cells. Read only
    /// with `structures`.
    pub interiors: Option<&'a interior::Interiors>,
}

impl Scene<'_> {
    /// Building `b`'s detailed colliders, when it has them.
    fn walkable(&self, b: u32) -> Option<(&interior::Walkable, &interior::InteriorConfig)> {
        let i = self.interiors?;
        i.buildings.get(&b).map(|w| (w, &i.cfg))
    }

    /// Pushes a pilot on foot out of the houses and small businesses whose colliders are built (in the mech
    /// there are none: the mech breaks what it walks into).
    fn push_out_houses(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let Some(i) = self.interiors else { return p };
        if i.houses.is_empty() {
            return p;
        }
        let mut p = p;
        for l in self.plan.small.candidates(p, radius_m) {
            if let Some((w, _)) = i.houses.get(&l) {
                p = small_buildings::walk::push_out(w, &self.plan.small.lots[l as usize], p, radius_m, feet_y, &i.cfg);
            }
        }
        p
    }

    /// Pushes a body out of a site's solids that rise more than `SITE_STEP_M` over its feet (a quay's wall from the
    /// water).
    fn push_out_sites(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let start = p;
        let mut p = p;
        for s in self.plan.sites.iter().filter(|s| s.footprint().distance(start) <= radius_m) {
            for (lo, hi) in &s.solids {
                if hi.y > feet_y + SITE_STEP_M && lo.y < feet_y + 12.0 {
                    let r = Rect {
                        min: Vec2::new(lo.x, lo.z),
                        max: Vec2::new(hi.x, hi.z),
                    };
                    p = collision::push_out_of_rect(p, radius_m, &r);
                }
            }
        }
        p
    }

    /// The highest top of a built house under `p` within a step of `feet_y`.
    fn house_support(&self, p: Vec2, feet_y: f32) -> Option<f32> {
        let i = self.interiors?;
        if i.houses.is_empty() {
            return None;
        }
        self.plan
            .small
            .candidates(p, 0.0)
            .into_iter()
            .filter_map(|l| {
                let (w, _) = i.houses.get(&l)?;
                small_buildings::walk::support(w, &self.plan.small.lots[l as usize], p, feet_y, &i.cfg)
            })
            .reduce(f32::max)
    }

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
                if let Some((w, cfg)) = self.walkable(b) {
                    p = w.push_out(p, radius_m, feet_y, cfg, cells.frame.as_deref());
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
        let p = Vec2::new(x, z);
        // the drawn road or pavement, not the bare grade under it (a crater in it lowers it as it lowers the ground)
        let dug = self.dug.delta_at(p);
        let bare = self.plan.terrain.height_at(x, z) + dug;
        let ground = self.plan.walk_top_at(p).map_or(bare, |top| (top + dug).max(bare));
        let roof = match self.structures {
            None => self.plan.collision.roof_under(p, feet_y),
            Some(st) => self
                .plan
                .collision
                .candidates(p, STOOP_REACH_M)
                .into_iter()
                .filter_map(|b| match self.walkable(b) {
                    Some((w, cfg)) => {
                        let cells = &st.buildings[b as usize];
                        (cells.standing > 0)
                            .then(|| w.support(p, feet_y, cfg, cells.frame.as_deref()))
                            .flatten()
                    }
                    None => st.roof_under(b, p, feet_y),
                })
                .reduce(f32::max),
        };
        let roof = match (roof, self.house_support(p, feet_y)) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        let under = roof.map_or(ground, |r| r.max(ground));
        platform::top_under(self.platforms, x, z, feet_y).map_or(under, |t| t.max(under))
    }
    fn push_out_at(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        let p = self.push_out_buildings(p, radius_m, feet_y);
        let p = self.push_out_houses(p, radius_m, feet_y);
        let p = self.push_out_sites(p, radius_m, feet_y);
        let p = rocks::push_out(&self.plan.rocks, p, radius_m, feet_y);
        // trunks stand on the ground: a body far above it (on a deck, a roof) passes over them
        match self.trees {
            Some(t) if feet_y <= self.height_at(p.x, p.y) + TREE_REACH_M => t.push_out(p, radius_m),
            _ => p,
        }
    }
}

impl WorldPlan {
    /// True when `q` is under the ground or inside a building's box as generated (the one test a sight line or a
    /// camera boom walks: `cb-app::traffic`'s cover, the chase camera's clip).
    pub fn solid_at(&self, q: glam::Vec3) -> bool {
        let q2 = Vec2::new(q.x, q.z);
        let ground = self.terrain.height_at(q.x, q.z);
        q.y < ground
            || self.collision.candidates(q2, 0.0).into_iter().any(|b| {
                let bd = &self.buildings[b as usize];
                bd.footprint.distance(q2) == 0.0 && q.y < ground + bd.height_m
            })
            || self
                .sites
                .iter()
                .any(|s| s.footprint().distance(q2) <= 0.0 && s.solids.iter().any(|(lo, hi)| q.cmpge(*lo).all() && q.cmple(*hi).all()))
    }

    /// The drawn road's surface at `p` (x, z), if a road is there: a city's street at the city's ground plus
    /// `road_lift_m`, a highway within its half width at its grade plus `road_lift_m` (as `cb-app::world` draws
    /// them), a suburb street, drive or car park at the ground plus its own lift (openspec change `lot-access`: draped
    /// on the ground). `None` off the roads.
    pub fn road_top_at(&self, p: Vec2) -> Option<f32> {
        for c in &self.cities {
            if c.rect().distance(p) <= 0.0 && c.streets.iter().any(|s| s.rect.distance(p) <= 0.0) {
                return Some(c.ground_m + self.road_lift_m);
            }
        }
        let highway = self.highways.iter().find_map(|h| {
            let (d, t) = h.project(p);
            (d <= h.width_m * 0.5).then(|| h.grade_at(t) + self.road_lift_m)
        });
        highway.or_else(|| self.small.paving.lift_at(p).map(|l| self.terrain.height_at(p.x, p.y) + l))
    }

    /// The drawn surface a body on foot stands on at `p` (x, z), over the bare ground: a road (`road_top_at`), or
    /// a city block's pavement, `kerb_height_m` over the city's ground, outside the buildings' footprints (inside
    /// one, its own floors carry the feet). `None` where the bare ground is the surface. The one rule for the pilot,
    /// the people on the pavements and the mech's feet (`Scene::support_at`).
    pub fn walk_top_at(&self, p: Vec2) -> Option<f32> {
        for c in &self.cities {
            if let Some(top) = c.surface_top(p, self.road_lift_m, self.kerb_height_m) {
                let in_building = self
                    .collision
                    .candidates(p, 0.0)
                    .into_iter()
                    .any(|b| self.buildings[b as usize].footprint.distance(p) <= 0.0);
                return (!in_building).then_some(top);
            }
        }
        self.road_top_at(p)
    }
}

/// The ground as a car's wheels meet it (car-traffic, car-driving): the drawn road's surface where there is a road,
/// carrying any crater dug into it, and the ground elsewhere. Cars were set on the bare grade while the road is drawn
/// `road_lift_m` (0.15 m) over it, so every car sat sunk in the asphalt (2026-10-26, measured).
pub struct OnRoads<'a, G: Ground> {
    /// The ground under the roads.
    pub ground: &'a G,
    /// The roads.
    pub plan: &'a WorldPlan,
}

impl<G: Ground> Ground for OnRoads<'_, G> {
    fn height_at(&self, x: f32, z: f32) -> f32 {
        let g = self.ground.height_at(x, z);
        match self.plan.road_top_at(Vec2::new(x, z)) {
            // a crater in the road lowers it as it lowers the ground
            Some(top) => top + (g - self.plan.terrain.height_at(x, z)),
            None => g,
        }
    }
    fn push_out(&self, p: Vec2, radius_m: f32) -> Vec2 {
        self.ground.push_out(p, radius_m)
    }
    fn water_at(&self, x: f32, z: f32) -> Option<WaterSample> {
        self.ground.water_at(x, z)
    }
    fn support_at(&self, x: f32, z: f32, feet_y: f32) -> f32 {
        self.ground.support_at(x, z, feet_y).max(self.height_at(x, z))
    }
    fn push_out_at(&self, p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
        self.ground.push_out_at(p, radius_m, feet_y)
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

/// The island as the weather sees it: the ground's height, the coast and the moisture.
struct SkyGround<'a> {
    terrain: &'a Heightfield,
    biomes: &'a Biomes,
}

impl weather::WeatherGround for SkyGround<'_> {
    fn ground_m(&self, p: Vec2) -> f32 {
        self.terrain.height_at(p.x, p.y)
    }
    fn past_coast_m(&self, p: Vec2) -> f32 {
        self.terrain.shape().past_land_edge_m(p)
    }
    fn moisture(&self, p: Vec2) -> f32 {
        self.biomes.moisture(p)
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
    /// The clear distance in front of each building's four facades (`layout::clear_in_front`), by
    /// building id, capped at `FRONT_LIMIT_M`.
    pub fronts: Vec<[f32; 4]>,
    /// The facade of each building that faces the street (`layout::street_side`), by building id:
    /// where its street door is.
    pub street_sides: Vec<u8>,
    /// Rivers, lakes and bridges.
    pub waters: Waters,
    /// The trees as grown (their state as the game goes on is a copy the game keeps).
    pub forest: Forest,
    /// The sky's clouds.
    pub clouds: Clouds,
    /// What grows where.
    pub biomes: Biomes,
    /// Sea stacks, arches and hoodoos (openspec change `rock-formations`).
    pub rocks: Vec<rocks::Rock>,
    /// Where mist forms (openspec change `ground-fog`).
    pub mist: mist::MistMap,
    /// The terrain's own shadow for the sun, for far ground (distant-shadows).
    pub sun_shade: sun_shade::SunShade,
    /// The air bases (air-bases), the one nearest the start first.
    pub bases: Vec<AirBase>,
    /// How far every road is drawn over its grade, metres (`WorldConfig::road_lift_m`).
    pub road_lift_m: f32,
    /// How far a city block's pavement is drawn over the city's ground, metres (`WorldConfig::kerb_height_m`).
    pub kerb_height_m: f32,
    /// The houses and small businesses on their lots (openspec change `small-buildings`), and the airport's and the
    /// seaport's buildings as lots of their own zones.
    pub small: small_buildings::Lots,
    /// The airport and the seaport (openspec changes `airport-layout`, `seaport-layout`).
    pub sites: Vec<sites::PlacedSite>,
    /// How many airport candidates each placing rule turned down (`sites::Rejections`), for `cb-tools sites`.
    pub airport_rejections: sites::Rejections,
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
            for (p, g) in h.path.iter().zip(&h.grade_m).step_by(8) {
                put(p.x);
                put(p.y);
                put(*g);
            }
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
        put(self.clouds.shafts.len() as f32);
        put(self.clouds.cirrus.len() as f32);
        for r in &self.rocks {
            put(r.base.x);
            put(r.base.z);
            put(r.scale.y);
        }
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
    let shape = Shape::new(seed, cfg);
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
    // Highways are routed round the mountains, lakes and other cities (openspec change
    // `curved-roads`).
    let lay = |a: usize, b: usize, from: Vec2, to: Vec2| roads::lay_highway(&shape, &drainage, &cities, (a, b), (from, to), cfg);
    let mut highways = layout::connect_cities(&cities, cfg, &lay);

    let largest = placed.first().map(|p| p.1).unwrap_or(1.0);
    let mut buildings = Vec::new();
    for city in &cities {
        let mut rng = root.fork(&format!("city/{}/buildings", city.id));
        let first = buildings.len() as u32;
        buildings.extend(layout::build_city(&mut rng, city, &highways, first, largest, cfg));
    }

    // The civil airports first, then the seaports, then the military bases in the room they leave (openspec change
    // `island-sites-layout`; owner, 2026-10-30: "Place civilians airports first"). Every site keeps clear of the
    // highways and the rivers; the bases keep clear of the civil sites and their runways' approach cones.
    let start_xz = cities.first().map_or(Vec2::ZERO, |c| c.centre);
    let courses: Vec<Vec<Vec2>> = drainage.courses.iter().map(|c| c.points.clone()).collect();
    let clear = bases::KeepClear {
        highways: &highways,
        road_m: cfg.highway_width_m * 0.5
            + cfg.highway_flat_margin_m
            + cfg.highway_blend_m
            + cfg.plateau_flat_margin_m
            + cfg.plateau_blend_m,
        courses: &courses,
        river_m: cfg.plateau_flat_margin_m + cfg.plateau_blend_m + cfg.water.river_max_half_width_m + cfg.water.river_city_clearance_m,
        sites: &[],
        site_m: cfg.sites.airport_base_clear_m,
        cones: &[],
    };
    // The airport and the seaport (airport-layout, seaport-layout): their footprints keep the houses and the
    // rivers out, and their pads shape the ground with the lots'.
    // A site keeps clear of the lakes. A stream under it runs on in a culvert: its channel is never cut inside a
    // kept-out footprint (`Waters::channels`), so its water lies under the levelled ground (the island's flat land is
    // its valleys, and every valley has a stream: on seed 42 all 28 flat, dry, clear airport sites had one).
    let site_water = |lo: Vec2, hi: Vec2| {
        let reach = cfg.water.river_city_clearance_m;
        drainage
            .basins
            .iter()
            .any(|b| (b.centre - b.centre.clamp(lo, hi)).length() < b.radius_m + reach)
    };
    let site_clear = sites::Clear {
        cities: &cities,
        bases: &[],
        keep: &clear,
        water_near: &site_water,
        min_ground_m: sea_level_m + cfg.water.city_freeboard_m,
        sea_level_m,
        placed: &[],
        buildings: &buildings,
    };
    let (airports, airport_rejections) = cfg
        .site_templates
        .airport
        .as_ref()
        .map_or((Vec::new(), [0; 6]), |t| sites::place_airports(&shape, t, &site_clear, &cfg.sites));
    let seaports = cfg.site_templates.seaport.as_ref().map_or(Vec::new(), |t| {
        sites::place_seaports(
            &shape,
            t,
            &sites::Clear {
                placed: &airports,
                ..site_clear
            },
            &cfg.sites,
        )
    });
    let mut placed_sites: Vec<sites::PlacedSite> = airports.into_iter().chain(seaports).collect();
    // The military bases (military-base section 4): the base page's site, placed by the airports' rules clear of
    // every site and cone before it, each one's base record read off its marks; a block-out base where the page is not
    // shipped.
    let bases = match &cfg.site_templates.military_base {
        Some(t) => {
            let (built, _) = sites::place_runway_sites(
                &shape,
                t,
                &sites::Clear {
                    placed: &placed_sites,
                    ..site_clear
                },
                &cfg.sites,
                cfg.bases.count,
            );
            let first = placed_sites.len() as u32;
            let bases: Vec<bases::AirBase> = built
                .iter()
                .enumerate()
                .filter_map(|(i, s)| bases::AirBase::of_site(i as u32, first + i as u32, s, &cfg.bases))
                .collect();
            placed_sites.extend(built);
            bases
        }
        None => {
            let site_rects: Vec<layout::Rect> = placed_sites.iter().map(|s| s.footprint()).collect();
            let site_cones: Vec<approach::Approach> = placed_sites.iter().flat_map(|s| s.approaches(cfg.sites.approach)).collect();
            let base_clear = bases::KeepClear {
                sites: &site_rects,
                cones: &site_cones,
                ..clear
            };
            bases::place_bases(
                &shape,
                &cities,
                &base_clear,
                start_xz,
                &cfg.bases,
                sea_level_m + cfg.water.city_freeboard_m,
            )
        }
    };
    // Every site's access road, from its city's edge to its gate (the footprint's point nearest the city), graded as
    // a highway and joined to the streets by the lane graph, so cars reach every site (island-sites-layout).
    // Its own city first; where no road can be laid from it (a base out in the country, behind steep ground), the next
    // nearest cities in turn.
    for s in &placed_sites {
        let f = s.footprint();
        let mut order: Vec<usize> = (0..cities.len()).collect();
        order.sort_by(|&a, &b| {
            let key = |i: usize| (cities[i].id != s.city, cities[i].rect().distance(f.centre()));
            key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal)
        });
        for city in order.into_iter().take(4) {
            let gate = cities[city].centre.clamp(f.min, f.max);
            let ground_m = s
                .pads()
                .iter()
                .filter(|(r, _)| r.distance(gate) < 1.0)
                .map(|(_, y)| *y)
                .fold(s.ground_m, f32::max);
            if let Some(road) = roads::lay_access(&shape, &drainage, &cities, city, (f, gate, ground_m), cfg) {
                highways.push(road);
                break;
            }
        }
    }
    // a block-out base is levelled here; a site-built one by its site's pads
    let block_outs = || bases.iter().filter(|b| b.site.is_none());
    let plateaus: Vec<(layout::Rect, f32)> = cities
        .iter()
        .map(|c| (c.rect(), c.ground_m))
        .chain(block_outs().map(|b| (b.rect(), b.ground_m)))
        // each base's runway and taxiway, flattened to its site's ground (survey W45)
        .chain(block_outs().map(|b| (b.runway.rect(), b.ground_m)))
        .chain(block_outs().map(|b| (b.runway.taxiway(&b.rect()), b.ground_m)))
        .collect();
    let keep_out: Vec<layout::Rect> = plateaus
        .iter()
        .map(|p| p.0)
        .chain(placed_sites.iter().map(|s| s.footprint()))
        .collect();
    // Houses and small businesses on their lots (openspec change `small-buildings`), placed on the ground
    // as the cities and roads level it, each lot then levelled by its own pad.
    let small = {
        let pre = Flats::new(&plateaus, &highways, cfg);
        let height = |p: Vec2| pre.apply(p, shape.height(p));
        let early_biomes = Biomes::new(&cfg.biomes, shape.clone(), sea_level_m, Vec::new());
        let land_edge = |p: Vec2| shape.past_land_edge_m(p);
        let water_near = |lo: Vec2, hi: Vec2, c: f32| drainage.water_near(lo, hi, c);
        let biome = |p: Vec2, g: f32, slope: f32| early_biomes.dominant(p, g, slope);
        let ground = small_buildings::lots::Ground {
            height: &height,
            past_land_edge_m: &land_edge,
            water_near: &water_near,
            biome: &biome,
            cities: &cities,
            highways: &highways,
            keep_out: &keep_out,
            min_ground_m: sea_level_m + cfg.water.city_freeboard_m,
            road_lift_m: cfg.road_lift_m,
            shoulder_m: cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m,
            cell_m: cfg.terrain_cell_m,
        };
        let mut placed = small_buildings::lots::place(&root.fork("small_buildings"), &cfg.small_buildings, &cfg.small_kits, &ground);
        for s in &mut placed_sites {
            s.first_lot = placed.lots.len() as u32;
            placed.lots.extend(s.lots(&cfg.small_kits));
        }
        small_buildings::Lots::new(cfg.small_kits.clone(), placed, cfg.world_size_m)
    };
    let flats = Flats::new(&plateaus, &highways, cfg).with_pads(
        small
            .pads()
            .into_iter()
            .chain(placed_sites.iter().flat_map(|s| s.shaping_pads()))
            .collect(),
        cfg.terrain_cell_m,
        cfg.small_buildings.pad_blend_m,
        cfg.world_size_m,
    );
    let terrain = Heightfield::new(cfg, shape, flats);
    let bounds = water::WaterBounds {
        world_size_m: cfg.world_size_m,
        sea_level_m,
    };
    let waters = water::make_waters(&root.fork("water"), &drainage, &terrain, &keep_out, &highways, &cfg.water, &bounds);
    let road_flat_m = cfg.highway_width_m * 0.5 + cfg.highway_flat_margin_m;
    let channels = waters.channels(&cfg.water, &keep_out, cfg.plateau_flat_margin_m, &highways, road_flat_m);
    let terrain = terrain.with_channels(channels);
    let sun_shade = sun_shade::SunShade::bake(&terrain, cfg.world_size_m, sun_shade::to_sun(), &cfg.sun_shade);

    let start = start_point(&cities, cfg);
    let roofs = buildings.iter().map(|b| cities[b.city as usize].ground_m + b.height_m).collect();
    let collision = BuildingGrid::new(&buildings, cfg.world_size_m, 64.0).with_roofs(roofs);
    let fronts = buildings
        .iter()
        .map(|b| {
            let reach = b.footprint.size().max_element() + FRONT_LIMIT_M;
            let others: Vec<Rect> = collision
                .candidates(b.footprint.centre(), reach)
                .into_iter()
                .filter(|&o| o != b.id)
                .map(|o| buildings[o as usize].footprint)
                .collect();
            layout::clear_in_front(&b.footprint, &others, FRONT_LIMIT_M)
        })
        .collect::<Vec<_>>();
    let street_sides = buildings
        .iter()
        .zip(&fronts)
        .map(|(b, f)| {
            let c = b.footprint.centre();
            let block = cities[b.city as usize]
                .blocks
                .iter()
                .find(|k| k.rect.distance(c) == 0.0)
                .map(|k| k.rect);
            layout::street_side(&b.footprint, block.as_ref(), *f)
        })
        .collect();
    // Swamps at the western river mouths (the island mockup's rule, on the game's own rivers).
    let mouths = waters.rivers.iter().filter_map(|r| {
        r.path
            .last()
            .copied()
            .filter(|_| r.surface_m.last().is_some_and(|s| *s < sea_level_m + 3.0))
    });
    let swamps = biome::swamps_at_mouths(mouths, cfg.biomes.swamp_radius_m);
    let biomes = Biomes::new(&cfg.biomes, terrain.shape().clone(), sea_level_m, swamps);
    let wind = Vec2::from(cfg.clouds.wind_m_s);
    let under = SkyGround {
        terrain: &terrain,
        biomes: &biomes,
    };
    let sky = weather::make_sky(&root.fork("clouds"), &cfg.weather, wind, &under);
    let mist = mist::MistMap::bake(&cfg.fog, cfg, &terrain, &waters);
    let road_clear = cfg.highway_width_m * 0.5 + 20.0;
    let rocks = rocks::make_rocks(terrain.shape().island(), &|p| terrain.height_at(p.x, p.y), &|p| {
        waters.water_at(p, terrain.height_at(p.x, p.y)).is_some()
            || highways.iter().any(|h| h.project(p).0 <= road_clear)
            || cities.iter().any(|c| c.rect().distance(p) < 60.0)
            || small.overlaps(p, 20.0)
    });
    let clouds = Clouds::new(sky, wind, cfg.weather.shaft_rain);
    WorldPlan {
        seed,
        terrain,
        cities,
        highways,
        buildings,
        start,
        collision,
        fronts,
        street_sides,
        waters,
        forest: Forest::new(&root.fork("trees"), &cfg.trees, cfg.world_size_m),
        clouds,
        biomes,
        sun_shade,
        rocks,
        mist,
        bases,
        road_lift_m: cfg.road_lift_m,
        kerb_height_m: cfg.kerb_height_m,
        small,
        sites: placed_sites,
        airport_rejections,
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
