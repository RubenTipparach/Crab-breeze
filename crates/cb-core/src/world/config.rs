//! World generation tuning, loaded from `data/world/world.ron` (CLAUDE.md section 6.4).
//!
//! Every field carries its unit in its name. The numbers and why they are what they are live in
//! `openspec/changes/procgen-world/design.md`.

use crate::data::{Validate, finite, positive, positive_range, within};
use serde::Deserialize;

/// Rivers and lakes (openspec change `terrain-water`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WaterConfig {
    /// Every city's ground stands at least this far above sea level.
    pub city_freeboard_m: f32,
    /// Rivers to run.
    pub river_count: u32,
    /// Grid a river's course is chosen on.
    pub river_grid_m: f32,
    /// Most grid steps a river takes.
    pub river_max_steps: u32,
    /// Random height added to each step's choice: larger meanders more.
    pub meander_m: f32,
    /// Least distance between a river and a city square.
    pub river_city_clearance_m: f32,
    /// Water surface width at the source.
    pub river_width_m: f32,
    /// Width grows by this share from source to mouth.
    pub river_widening: f32,
    /// Depth at the channel's centre.
    pub river_depth_m: f32,
    /// Water surface below the surrounding ground.
    pub freeboard_m: f32,
    /// Rise of a riverbank per metre outward (0.5 is about 27 degrees).
    pub bank_slope: f32,
    /// River flow speed.
    pub flow_m_s: f32,
}

impl Default for WaterConfig {
    fn default() -> Self {
        Self {
            city_freeboard_m: 2.0,
            river_count: 6,
            river_grid_m: 32.0,
            river_max_steps: 700,
            meander_m: 6.0,
            river_city_clearance_m: 80.0,
            river_width_m: 36.0,
            river_widening: 0.4,
            river_depth_m: 7.0,
            freeboard_m: 2.0,
            bank_slope: 0.45,
            flow_m_s: 1.5,
        }
    }
}

impl Validate for WaterConfig {
    fn validate(&self) -> Result<(), String> {
        finite("water.city_freeboard_m", self.city_freeboard_m)?;
        positive("water.river_grid_m", self.river_grid_m)?;
        finite("water.meander_m", self.meander_m)?;
        finite("water.river_city_clearance_m", self.river_city_clearance_m)?;
        positive("water.river_width_m", self.river_width_m)?;
        finite("water.river_widening", self.river_widening)?;
        positive("water.river_depth_m", self.river_depth_m)?;
        finite("water.freeboard_m", self.freeboard_m)?;
        positive("water.bank_slope", self.bank_slope)?;
        finite("water.flow_m_s", self.flow_m_s)?;
        Ok(())
    }
}

/// A tree species (openspec change `vegetation`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Species {
    /// Name (`pine`, `oak`, `birch`).
    pub name: String,
    /// Share of trees, relative to the others.
    pub share: f32,
    /// Height range.
    pub height_m: [f32; 2],
    /// Trunk radius range at the base.
    pub trunk_radius_m: [f32; 2],
    /// Momentum that snaps a mid-sized one, kilonewton seconds (a 50 t mech at 1 m/s is 50).
    pub break_impulse_kn_s: f32,
    /// Crown radius as a share of the height (as `tree_mesh` in cb-app draws it).
    pub crown_radius_share: f32,
    /// Height where the crown starts, as a share of the height.
    pub crown_base_share: f32,
}

/// Trees (openspec change `vegetation`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TreeConfig {
    /// Grid spacing of tree sites.
    pub spacing_m: f32,
    /// Wavelength of the forest mask noise.
    pub mask_wavelength_m: f32,
    /// Mask value above which the ground is forest.
    pub forest_threshold: f32,
    /// Chance a site in forest has a tree.
    pub forest_chance: f32,
    /// Chance a site in open country has a tree.
    pub scatter_chance: f32,
    /// Chance a site in a park block has a tree.
    pub park_chance: f32,
    /// Species.
    pub species: Vec<Species>,
    /// A push high in the crown counts this many times a push on the trunk (leverage).
    pub crown_leverage: f32,
    /// Share of a mech's speed past a tree that still pushes it (a grazing hit).
    pub graze_share: f32,
    /// Sites along one side of a tree tile, the unit trees are grown and kept in (island-world).
    pub tile_sites: u32,
    /// Tiles within this of a body (the pilot, every mech) are grown and kept.
    pub keep_radius_m: f32,
    /// Tiles within this of a body are grown at once, whatever the per-step budget.
    pub urgent_radius_m: f32,
    /// Tiles farther than this from every body are let go (their felled trees remembered).
    pub release_radius_m: f32,
    /// Most tiles grown in one fixed step beyond the urgent ones.
    pub tiles_per_step: u32,
    /// Trees are drawn within this of the eye.
    pub draw_radius_m: f32,
}

impl Default for TreeConfig {
    fn default() -> Self {
        let sp = |name: &str, share: f32, h: [f32; 2], r: [f32; 2], b: f32, crown: [f32; 2]| Species {
            name: name.into(),
            share,
            height_m: h,
            trunk_radius_m: r,
            break_impulse_kn_s: b,
            crown_radius_share: crown[0],
            crown_base_share: crown[1],
        };
        Self {
            spacing_m: 11.0,
            mask_wavelength_m: 700.0,
            forest_threshold: 0.12,
            forest_chance: 0.85,
            scatter_chance: 0.04,
            park_chance: 0.5,
            species: vec![
                sp("pine", 0.5, [14.0, 26.0], [0.25, 0.45], 60.0, [0.2, 0.2]),
                sp("oak", 0.25, [10.0, 18.0], [0.4, 0.8], 240.0, [0.34, 0.38]),
                sp("birch", 0.25, [9.0, 16.0], [0.12, 0.25], 25.0, [0.2, 0.42]),
            ],
            crown_leverage: 2.0,
            graze_share: 0.25,
            tile_sites: 24,
            keep_radius_m: 1500.0,
            urgent_radius_m: 400.0,
            release_radius_m: 2000.0,
            tiles_per_step: 2,
            draw_radius_m: 1200.0,
        }
    }
}

impl Validate for TreeConfig {
    fn validate(&self) -> Result<(), String> {
        positive("trees.spacing_m", self.spacing_m)?;
        positive("trees.mask_wavelength_m", self.mask_wavelength_m)?;
        finite("trees.forest_threshold", self.forest_threshold)?;
        within("trees.forest_chance", self.forest_chance, 0.0, 1.0)?;
        within("trees.scatter_chance", self.scatter_chance, 0.0, 1.0)?;
        within("trees.park_chance", self.park_chance, 0.0, 1.0)?;
        if self.species.is_empty() || self.species.len() > 255 {
            return Err("`trees.species` must list 1 to 255 species".into());
        }
        for s in &self.species {
            positive(&format!("trees.species[{}].share", s.name), s.share)?;
            positive_range(&format!("trees.species[{}].height_m", s.name), s.height_m)?;
            positive_range(&format!("trees.species[{}].trunk_radius_m", s.name), s.trunk_radius_m)?;
            positive(&format!("trees.species[{}].break_impulse_kn_s", s.name), s.break_impulse_kn_s)?;
            within(
                &format!("trees.species[{}].crown_radius_share", s.name),
                s.crown_radius_share,
                0.0,
                1.0,
            )?;
            within(&format!("trees.species[{}].crown_base_share", s.name), s.crown_base_share, 0.0, 1.0)?;
        }
        positive("trees.crown_leverage", self.crown_leverage)?;
        within("trees.graze_share", self.graze_share, 0.0, 1.0)?;
        if self.tile_sites == 0 || self.tile_sites > 64 {
            return Err("`trees.tile_sites` must be 1 to 64 (a tile's site index is 12 bits)".into());
        }
        positive("trees.urgent_radius_m", self.urgent_radius_m)?;
        positive("trees.keep_radius_m", self.keep_radius_m)?;
        positive("trees.release_radius_m", self.release_radius_m)?;
        positive("trees.draw_radius_m", self.draw_radius_m)?;
        if !(self.urgent_radius_m <= self.keep_radius_m && self.keep_radius_m < self.release_radius_m) {
            return Err("trees: need `urgent_radius_m` <= `keep_radius_m` < `release_radius_m`".into());
        }
        if self.draw_radius_m > self.keep_radius_m {
            return Err("`trees.draw_radius_m` must not exceed `trees.keep_radius_m`: only kept trees are drawn".into());
        }
        Ok(())
    }
}

/// Clouds (openspec change `clouds`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CloudConfig {
    /// Cloud base height.
    pub base_m: f32,
    /// Highest cloud top.
    pub top_m: f32,
    /// Grid spacing of cluster sites.
    pub spacing_m: f32,
    /// Share of sites with a cluster, 0..1.
    pub coverage: f32,
    /// Cluster width range.
    pub cluster_size_m: [f32; 2],
    /// Clouds continue this far past the world's edge.
    pub beyond_world_m: f32,
    /// Wind drift (x, z).
    pub wind_m_s: [f32; 2],
    /// Cloud puffs are drawn within this of the eye (beyond it the haze has hidden them).
    pub draw_radius_m: f32,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            base_m: 600.0,
            top_m: 1100.0,
            spacing_m: 1000.0,
            coverage: 0.45,
            cluster_size_m: [220.0, 520.0],
            beyond_world_m: 6000.0,
            wind_m_s: [3.0, 1.0],
            draw_radius_m: 9000.0,
        }
    }
}

impl Validate for CloudConfig {
    fn validate(&self) -> Result<(), String> {
        finite("clouds.base_m", self.base_m)?;
        if self.top_m <= self.base_m {
            return Err("`clouds.top_m` must be above `clouds.base_m`".into());
        }
        positive("clouds.spacing_m", self.spacing_m)?;
        within("clouds.coverage", self.coverage, 0.0, 1.0)?;
        positive_range("clouds.cluster_size_m", self.cluster_size_m)?;
        finite("clouds.beyond_world_m", self.beyond_world_m)?;
        finite("clouds.wind_m_s", self.wind_m_s[0])?;
        finite("clouds.wind_m_s", self.wind_m_s[1])?;
        positive("clouds.draw_radius_m", self.draw_radius_m)?;
        Ok(())
    }
}

/// Craters (openspec change `terrain-deformation`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CraterConfig {
    /// Crater radius per cube root of the blast energy in megajoules.
    pub radius_m_per_cbrt_mj: f32,
    /// Bowl depth as a share of the radius.
    pub depth_ratio: f32,
    /// Rim height as a share of the radius.
    pub rim_ratio: f32,
    /// Deepest the ground can be dug below where it was.
    pub max_dig_m: f32,
}

impl Default for CraterConfig {
    fn default() -> Self {
        Self {
            radius_m_per_cbrt_mj: 2.2,
            depth_ratio: 0.35,
            rim_ratio: 0.08,
            max_dig_m: 6.0,
        }
    }
}

impl CraterConfig {
    /// How far from its centre a blast of `energy_mj` changes the ground, metres: 1.8 crater radii,
    /// the bowl, its rim and the rim's fall-off.
    pub fn reach_m(&self, energy_mj: f32) -> f32 {
        self.radius_m_per_cbrt_mj * energy_mj.cbrt() * 1.8
    }
}

impl Validate for CraterConfig {
    fn validate(&self) -> Result<(), String> {
        positive("craters.radius_m_per_cbrt_mj", self.radius_m_per_cbrt_mj)?;
        within("craters.depth_ratio", self.depth_ratio, 0.0, 2.0)?;
        within("craters.rim_ratio", self.rim_ratio, 0.0, 1.0)?;
        positive("craters.max_dig_m", self.max_dig_m)?;
        Ok(())
    }
}

/// The island and the sea round it (openspec change `island-world`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct IslandConfig {
    /// Height of the sea's surface. Ground below it is under the sea or a lake.
    pub sea_level_m: f32,
    /// Distance from the island's centre at which the land starts to fall to the sea, before the
    /// coast noise moves it in or out.
    pub land_radius_m: f32,
    /// Most the coast noise moves the land's edge in or out.
    pub coast_noise_m: f32,
    /// Wavelength of the coast noise's first octave (bays and headlands).
    pub coast_wavelength_m: f32,
    /// Land height above sea level where the land starts to fall to the sea (the hills ride on it).
    pub land_height_m: f32,
    /// Extra height at the island's middle, falling off toward the coast.
    pub highland_m: f32,
    /// Width over which the land falls to the open sea floor (beaches, shallows, the shelf).
    pub shelf_m: f32,
    /// Depth of the open sea floor below sea level.
    pub sea_depth_m: f32,
    /// Most the sea floor's own noise moves it up or down.
    pub floor_noise_m: f32,
    /// Wavelength of the sea floor noise.
    pub floor_wavelength_m: f32,
    /// The sea floor noise fades out over this width inside the world's edge, so the floor meets the
    /// flat floor drawn beyond it exactly.
    pub edge_fade_m: f32,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            sea_level_m: 0.0,
            land_radius_m: 13000.0,
            coast_noise_m: 2500.0,
            coast_wavelength_m: 6000.0,
            land_height_m: 30.0,
            highland_m: 90.0,
            shelf_m: 3000.0,
            sea_depth_m: 38.0,
            floor_noise_m: 9.0,
            floor_wavelength_m: 1500.0,
            edge_fade_m: 1500.0,
        }
    }
}

impl Validate for IslandConfig {
    fn validate(&self) -> Result<(), String> {
        finite("island.sea_level_m", self.sea_level_m)?;
        positive("island.land_radius_m", self.land_radius_m)?;
        finite("island.coast_noise_m", self.coast_noise_m)?;
        positive("island.coast_wavelength_m", self.coast_wavelength_m)?;
        finite("island.land_height_m", self.land_height_m)?;
        finite("island.highland_m", self.highland_m)?;
        positive("island.shelf_m", self.shelf_m)?;
        positive("island.sea_depth_m", self.sea_depth_m)?;
        finite("island.floor_noise_m", self.floor_noise_m)?;
        positive("island.floor_wavelength_m", self.floor_wavelength_m)?;
        positive("island.edge_fade_m", self.edge_fade_m)?;
        if self.floor_noise_m.abs() >= self.sea_depth_m {
            return Err("`island.floor_noise_m` must be less than `island.sea_depth_m`, or the floor breaks the surface".into());
        }
        Ok(())
    }
}

/// Levels of detail of the drawn ground (openspec change `island-world`). The ground is drawn in
/// square chunks; a chunk farther from the eye is drawn with every `step`-th sample.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LodConfig {
    /// Terrain cells along a chunk's side.
    pub chunk_cells: u32,
    /// Outer distance of each ring: within `ring_m[0]` a chunk is drawn at step 1, within
    /// `ring_m[1]` at step 2, and so on; beyond the last ring at step `2^len`.
    pub ring_m: Vec<f32>,
    /// A chunk changes step only when it is this far past a ring's edge, so a body standing on the
    /// edge does not make it flicker between two steps.
    pub hysteresis_m: f32,
    /// Chunks within this of the eye or of a loose physics body get a collider (step-1 triangles).
    pub collider_radius_m: f32,
    /// Most chunk meshes started in one frame (they are built off the main thread).
    pub builds_per_frame: u32,
    /// Levels of the ground's quadtree above the chunk: a root covers `2^max_level` chunks a side
    /// and is drawn as one mesh at step `2^max_level` until the eye comes near enough to split it,
    /// so the far island is a handful of meshes, not thousands (owner, 2026-10-24: "make sure to
    /// lower draw calls").
    pub max_level: u32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            chunk_cells: 64,
            ring_m: vec![700.0, 1400.0, 2800.0, 5600.0, 11200.0],
            hysteresis_m: 64.0,
            collider_radius_m: 400.0,
            builds_per_frame: 6,
            max_level: 4,
        }
    }
}

impl LodConfig {
    /// The coarsest step, past the last ring.
    pub fn max_step(&self) -> usize {
        1 << self.ring_m.len()
    }
}

impl Validate for LodConfig {
    fn validate(&self) -> Result<(), String> {
        if self.ring_m.is_empty() || self.ring_m.len() > 6 {
            return Err("`terrain_lod.ring_m` must list 1 to 6 rings".into());
        }
        let mut last = 0.0;
        for (i, r) in self.ring_m.iter().enumerate() {
            positive(&format!("terrain_lod.ring_m[{i}]"), *r)?;
            if *r <= last {
                return Err("`terrain_lod.ring_m` must rise ring by ring".into());
            }
            last = *r;
        }
        if self.chunk_cells == 0 || !(self.chunk_cells as usize).is_multiple_of(self.max_step()) {
            return Err(format!(
                "`terrain_lod.chunk_cells` ({}) must be a multiple of the coarsest step ({})",
                self.chunk_cells,
                self.max_step()
            ));
        }
        finite("terrain_lod.hysteresis_m", self.hysteresis_m)?;
        if self.hysteresis_m < 0.0 {
            return Err("`terrain_lod.hysteresis_m` must not be negative".into());
        }
        positive("terrain_lod.collider_radius_m", self.collider_radius_m)?;
        if self.builds_per_frame == 0 {
            return Err("`terrain_lod.builds_per_frame` must be at least 1".into());
        }
        if self.max_level > 8 {
            return Err("`terrain_lod.max_level` must be 8 or less".into());
        }
        Ok(())
    }
}

/// How far the pilot sees (openspec change `island-world`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ViewConfig {
    /// The camera's far plane: nothing farther is drawn (Bevy's projection is reverse-Z infinite,
    /// so this only culls; it costs no depth precision).
    pub far_m: f32,
    /// Distance at which clear air has hidden things in haze.
    pub clear_visibility_m: f32,
    /// The pilot's camera exposure (EV100) under the scattering sky's raw sunlight
    /// (sky-atmosphere); Bevy's default is 9.7.
    pub exposure_ev100: f32,
    /// The sun disk's apparent diameter in degrees (sky-atmosphere). The real sun is 0.53, about
    /// 8 px across at 1080p; it reads bigger to the eye through glare (owner, 2026-10-24: "sun
    /// should be a sizable circle like in real life").
    pub sun_disk_deg: f32,
    /// The sun disk's brightness over its physical value.
    pub sun_disk_intensity: f32,
    /// The colour grade on the pilot's eye (Bevy's `ColorGrading`): saturation after
    /// tonemapping, 1 unchanged.
    pub grade_saturation: f32,
    /// Contrast across shadows, midtones and highlights, 1 unchanged.
    pub grade_contrast: f32,
    /// White balance, a CIE 1931 x offset: positive warmer, negative cooler, 0 unchanged.
    pub grade_temperature: f32,
    /// Screen-space ambient occlusion's sample radius (openspec change `ambient-occlusion`); 0
    /// switches it off.
    pub ssao_radius_m: f32,
    /// Its quality level.
    pub ssao_quality: SsaoQuality,
    /// GPU occlusion culling on the pilot's eye (openspec change `occlusion-culling`).
    pub occlusion_culling: bool,
}

/// Screen-space ambient occlusion quality, Bevy's levels.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum SsaoQuality {
    /// Fewest samples.
    Low,
    /// Bevy's default.
    #[default]
    Medium,
    /// More samples.
    High,
    /// Most samples.
    Ultra,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            far_m: 40000.0,
            clear_visibility_m: 9000.0,
            exposure_ev100: 13.0,
            sun_disk_deg: 2.0,
            sun_disk_intensity: 2.0,
            grade_saturation: 1.0,
            grade_contrast: 1.0,
            grade_temperature: 0.0,
            ssao_radius_m: 1.5,
            ssao_quality: SsaoQuality::Medium,
            occlusion_culling: true,
        }
    }
}

impl Validate for ViewConfig {
    fn validate(&self) -> Result<(), String> {
        positive("view.far_m", self.far_m)?;
        positive("view.clear_visibility_m", self.clear_visibility_m)?;
        within("view.exposure_ev100", self.exposure_ev100, 0.0, 20.0)?;
        within("view.sun_disk_deg", self.sun_disk_deg, 0.0, 20.0)?;
        within("view.sun_disk_intensity", self.sun_disk_intensity, 0.0, 100.0)?;
        within("view.ssao_radius_m", self.ssao_radius_m, 0.0, 20.0)?;
        within("view.grade_saturation", self.grade_saturation, 0.0, 3.0)?;
        within("view.grade_contrast", self.grade_contrast, 0.5, 2.0)?;
        within("view.grade_temperature", self.grade_temperature, -1.0, 1.0)?;
        if self.clear_visibility_m > self.far_m {
            return Err("`view.clear_visibility_m` must not exceed `view.far_m`".into());
        }
        Ok(())
    }
}

/// Everything the world generator is tuned by.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WorldConfig {
    /// Side of the square world.
    pub world_size_m: f32,
    /// Spacing of the terrain height samples.
    pub terrain_cell_m: f32,
    /// Wavelength of the first terrain noise octave.
    pub noise_wavelength_m: f32,
    /// Peak height of the first terrain noise octave above or below zero.
    pub noise_amplitude_m: f32,
    /// Noise octaves, each half the wavelength of the last.
    pub noise_octaves: u32,
    /// Amplitude ratio between octaves.
    pub noise_persistence: f32,
    /// Most cities to place.
    pub city_count: u32,
    /// Placement tries before giving up on more cities.
    pub city_attempts: u32,
    /// Least distance between city centres.
    pub city_min_spacing_m: f32,
    /// Least open ground between two city squares.
    pub open_ground_min_m: f32,
    /// Least distance from a city square to the world's edge.
    pub city_edge_margin_m: f32,
    /// The first city placed, the largest (where the game starts), has its centre within this of
    /// the island's centre, where f32 positions are finest (island-world design section 5).
    pub capital_within_m: f32,
    /// Half the side of a city square, [smallest, largest].
    pub city_half_size_m: [f32; 2],
    /// Flat ground kept beyond a city's square, so its edge streets never meet a slope inside one
    /// terrain cell. Must exceed a cell's diagonal.
    pub plateau_flat_margin_m: f32,
    /// Width over which a city's flat ground blends back into the hills.
    pub plateau_blend_m: f32,
    /// Block size between street edges, [smallest, largest].
    pub block_size_m: [f32; 2],
    /// Width of an ordinary street.
    pub street_width_m: f32,
    /// Width of an arterial street.
    pub arterial_width_m: f32,
    /// Every n-th street (counting from the edge) is an arterial.
    pub arterial_every: u32,
    /// Sidewalk inset from a block's edge to its lots.
    pub sidewalk_m: f32,
    /// Lot size, [smallest, largest].
    pub lot_size_m: [f32; 2],
    /// Gap between a building and its lot's edge, [smallest, largest].
    pub setback_m: [f32; 2],
    /// Share of blocks left as parks, 0..1.
    pub park_block_chance: f32,
    /// Share of lots left as plazas, 0..1.
    pub plaza_lot_chance: f32,
    /// Height of one storey.
    pub storey_height_m: f32,
    /// Storeys at the centre of the smallest and of the largest city.
    pub storeys_downtown: [f32; 2],
    /// Storeys at a city's edge (the least any building has).
    pub storeys_edge: u32,
    /// How fast building height falls from the centre: `max = lerp(downtown, edge, d^k)`.
    pub storey_falloff_exponent: f32,
    /// A building's share of its lot's maximum storeys, [least, most].
    pub storey_fill: [f32; 2],
    /// Width of a highway's paved surface.
    pub highway_width_m: f32,
    /// Flat ground on each side of a highway beyond its surface. Must exceed a cell's diagonal.
    pub highway_flat_margin_m: f32,
    /// Width over which a highway's grading blends back into the hills.
    pub highway_blend_m: f32,
    /// Height of a road surface above the ground under it (no z-fighting).
    pub road_lift_m: f32,
    /// Height of a block's pavement (the kerb) above the ground.
    pub kerb_height_m: f32,
    /// How far behind the mech the pilot starts.
    pub start_pilot_distance_m: f32,
    /// The island and the sea.
    pub island: IslandConfig,
    /// Levels of detail of the drawn ground.
    pub terrain_lod: LodConfig,
    /// How far the pilot sees.
    pub view: ViewConfig,
    /// Rivers and lakes.
    pub water: WaterConfig,
    /// Trees.
    pub trees: TreeConfig,
    /// Clouds.
    pub clouds: CloudConfig,
    /// Craters.
    pub craters: CraterConfig,
    /// Building frames (openspec change `structural-destruction`).
    pub structure: super::structure::StructureConfig,
    /// Building dressing (openspec change `building-generation`).
    pub dressing: super::dressing::DressingConfig,
    /// Physics debris (openspec change `debris-physics`).
    pub debris: DebrisConfig,
}

/// Physics debris: where chunks become rigid bodies, and how many (openspec change
/// `debris-physics`, "Physics near the player only").
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct DebrisConfig {
    /// A chunk breaking off within this distance of the pilot's eye becomes a rigid body; farther,
    /// it is a dust puff only. 0 switches debris physics off: every chunk is a puff.
    pub physics_radius_m: f32,
    /// A moving body farther than `physics_radius_m` times this is made static where it is.
    pub freeze_factor: f32,
    /// Most moving bodies; past it the oldest settle where they are.
    pub max_dynamic: u32,
    /// Most bodies kept at all; past it the oldest are removed.
    pub max_kept: u32,
}

impl Default for DebrisConfig {
    fn default() -> Self {
        Self {
            physics_radius_m: 400.0,
            freeze_factor: 1.2,
            max_dynamic: 600,
            max_kept: 2500,
        }
    }
}

impl DebrisConfig {
    /// True when a chunk breaking off `distance_m` from the pilot's eye becomes a rigid body.
    pub fn is_physical(&self, distance_m: f32) -> bool {
        self.physics_radius_m > 0.0 && distance_m.is_finite() && distance_m <= self.physics_radius_m
    }

    /// True when a moving body `distance_m` from the pilot's eye is made static.
    pub fn freezes(&self, distance_m: f32) -> bool {
        !distance_m.is_finite() || distance_m > self.physics_radius_m * self.freeze_factor
    }

    fn validate(&self) -> Result<(), String> {
        within("debris.physics_radius_m", self.physics_radius_m, 0.0, 1.0e5)?;
        within("debris.freeze_factor", self.freeze_factor, 1.0, 4.0)?;
        if self.max_dynamic == 0 || self.max_kept < self.max_dynamic {
            return Err("debris: max_dynamic must be above 0 and max_kept at least max_dynamic".into());
        }
        Ok(())
    }
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            world_size_m: 40960.0,
            terrain_cell_m: 8.0,
            noise_wavelength_m: 1100.0,
            noise_amplitude_m: 35.0,
            noise_octaves: 4,
            noise_persistence: 0.5,
            city_count: 24,
            city_attempts: 4000,
            city_min_spacing_m: 2600.0,
            open_ground_min_m: 600.0,
            city_edge_margin_m: 300.0,
            capital_within_m: 2500.0,
            city_half_size_m: [280.0, 480.0],
            plateau_flat_margin_m: 12.0,
            plateau_blend_m: 120.0,
            block_size_m: [70.0, 110.0],
            street_width_m: 24.0,
            arterial_width_m: 36.0,
            arterial_every: 4,
            sidewalk_m: 4.0,
            lot_size_m: [18.0, 36.0],
            setback_m: [1.0, 3.0],
            park_block_chance: 0.06,
            plaza_lot_chance: 0.08,
            storey_height_m: 3.5,
            storeys_downtown: [24.0, 60.0],
            storeys_edge: 2,
            storey_falloff_exponent: 0.7,
            storey_fill: [0.35, 1.0],
            highway_width_m: 28.0,
            highway_flat_margin_m: 12.0,
            highway_blend_m: 60.0,
            road_lift_m: 0.1,
            kerb_height_m: 0.2,
            start_pilot_distance_m: 25.0,
            island: IslandConfig::default(),
            terrain_lod: LodConfig::default(),
            view: ViewConfig::default(),
            water: WaterConfig::default(),
            trees: TreeConfig::default(),
            clouds: CloudConfig::default(),
            craters: CraterConfig::default(),
            structure: super::structure::StructureConfig::default(),
            dressing: super::dressing::DressingConfig::default(),
            debris: DebrisConfig::default(),
        }
    }
}

impl WorldConfig {
    /// Height samples along one side of the world.
    pub fn samples_per_side(&self) -> usize {
        (self.world_size_m / self.terrain_cell_m).round() as usize + 1
    }
}

impl Validate for WorldConfig {
    fn validate(&self) -> Result<(), String> {
        positive("world_size_m", self.world_size_m)?;
        positive("terrain_cell_m", self.terrain_cell_m)?;
        positive("noise_wavelength_m", self.noise_wavelength_m)?;
        finite("noise_amplitude_m", self.noise_amplitude_m)?;
        within("noise_persistence", self.noise_persistence, 0.0, 1.0)?;
        positive("city_min_spacing_m", self.city_min_spacing_m)?;
        finite("open_ground_min_m", self.open_ground_min_m)?;
        finite("city_edge_margin_m", self.city_edge_margin_m)?;
        positive("capital_within_m", self.capital_within_m)?;
        positive_range("city_half_size_m", self.city_half_size_m)?;
        positive("plateau_blend_m", self.plateau_blend_m)?;
        let diagonal = self.terrain_cell_m * std::f32::consts::SQRT_2;
        for (field, v) in [
            ("plateau_flat_margin_m", self.plateau_flat_margin_m),
            ("highway_flat_margin_m", self.highway_flat_margin_m),
        ] {
            finite(field, v)?;
            if v <= diagonal {
                return Err(format!(
                    "`{field}` ({v}) must exceed a terrain cell's diagonal ({diagonal:.2} m), \
                     or terrain triangles would cut through the road"
                ));
            }
        }
        positive_range("block_size_m", self.block_size_m)?;
        positive("street_width_m", self.street_width_m)?;
        positive("arterial_width_m", self.arterial_width_m)?;
        if self.arterial_every == 0 {
            return Err("`arterial_every` must be at least 1".into());
        }
        finite("sidewalk_m", self.sidewalk_m)?;
        positive_range("lot_size_m", self.lot_size_m)?;
        positive_range("setback_m", self.setback_m)?;
        within("park_block_chance", self.park_block_chance, 0.0, 1.0)?;
        within("plaza_lot_chance", self.plaza_lot_chance, 0.0, 1.0)?;
        positive("storey_height_m", self.storey_height_m)?;
        positive_range("storeys_downtown", self.storeys_downtown)?;
        if self.storeys_edge == 0 {
            return Err("`storeys_edge` must be at least 1".into());
        }
        positive("storey_falloff_exponent", self.storey_falloff_exponent)?;
        positive_range("storey_fill", self.storey_fill)?;
        positive("highway_width_m", self.highway_width_m)?;
        positive("highway_blend_m", self.highway_blend_m)?;
        positive("road_lift_m", self.road_lift_m)?;
        positive("kerb_height_m", self.kerb_height_m)?;
        if self.kerb_height_m <= self.road_lift_m {
            return Err("`kerb_height_m` must be above `road_lift_m`".into());
        }
        positive("start_pilot_distance_m", self.start_pilot_distance_m)?;
        self.island.validate()?;
        let i = &self.island;
        let reach = i.land_radius_m + i.coast_noise_m.abs() + i.shelf_m;
        let flat_from = self.world_size_m * 0.5 - i.edge_fade_m;
        if reach > flat_from {
            return Err(format!(
                "the island's shelf reaches {reach} m from the centre, past {flat_from} m where the sea floor \
                 must already be flat (`world_size_m` / 2 - `island.edge_fade_m`)"
            ));
        }
        self.terrain_lod.validate()?;
        let chunk_m = self.terrain_lod.chunk_cells as f32 * self.terrain_cell_m;
        let chunks = self.world_size_m / chunk_m;
        if (chunks - chunks.round()).abs() > 1e-3 {
            return Err(format!(
                "`world_size_m` ({}) must be a whole number of terrain chunks ({chunk_m} m)",
                self.world_size_m
            ));
        }
        if !(chunks.round() as u32).is_multiple_of(1 << self.terrain_lod.max_level) {
            return Err("`2^terrain_lod.max_level` must divide the chunks along the world's side".into());
        }
        self.view.validate()?;
        self.water.validate()?;
        self.trees.validate()?;
        self.clouds.validate()?;
        self.craters.validate()?;
        self.structure.validate()?;
        self.dressing.validate()?;
        self.debris.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod debris_tests {
    use super::{DebrisConfig, Validate, ViewConfig};

    #[test]
    fn ssao_settings_are_validated() {
        assert!(ViewConfig::default().validate().is_ok());
        let off = ViewConfig {
            ssao_radius_m: 0.0,
            ..ViewConfig::default()
        };
        assert!(off.validate().is_ok(), "0 switches it off");
        let bad = ViewConfig {
            ssao_radius_m: f32::NAN,
            ..ViewConfig::default()
        };
        assert!(bad.validate().is_err(), "a broken radius stops startup");
        let neg = ViewConfig {
            ssao_radius_m: -1.0,
            ..ViewConfig::default()
        };
        assert!(neg.validate().is_err());
    }

    #[test]
    fn debris_is_physical_only_near_the_pilot() {
        let d = DebrisConfig::default();
        assert!(d.is_physical(10.0) && d.is_physical(400.0), "near chunks are bodies");
        assert!(!d.is_physical(401.0) && !d.is_physical(1000.0), "far chunks are only dust");
        assert!(!d.is_physical(f32::NAN), "a broken distance makes no body");
        assert!(!d.freezes(450.0), "a body just past the radius keeps moving");
        assert!(d.freezes(481.0), "a body well past it is made static");
        let off = DebrisConfig {
            physics_radius_m: 0.0,
            ..d
        };
        assert!(off.validate().is_ok(), "0 is allowed");
        assert!(!off.is_physical(0.0) && !off.is_physical(5.0), "0 switches debris physics off");
    }
}
