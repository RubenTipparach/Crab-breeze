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
    /// Lakes fill the ground below this share of all terrain heights, 0..1.
    pub sea_level_percentile: f32,
    /// Sea level stays at least this far below every city's ground.
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
            sea_level_percentile: 0.12,
            city_freeboard_m: 2.0,
            river_count: 2,
            river_grid_m: 32.0,
            river_max_steps: 400,
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
        within("water.sea_level_percentile", self.sea_level_percentile, 0.0, 1.0)?;
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
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            base_m: 600.0,
            top_m: 1100.0,
            spacing_m: 700.0,
            coverage: 0.45,
            cluster_size_m: [220.0, 520.0],
            beyond_world_m: 2000.0,
            wind_m_s: [3.0, 1.0],
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

impl Validate for CraterConfig {
    fn validate(&self) -> Result<(), String> {
        positive("craters.radius_m_per_cbrt_mj", self.radius_m_per_cbrt_mj)?;
        within("craters.depth_ratio", self.depth_ratio, 0.0, 2.0)?;
        within("craters.rim_ratio", self.rim_ratio, 0.0, 1.0)?;
        positive("craters.max_dig_m", self.max_dig_m)?;
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
    /// Rivers and lakes.
    pub water: WaterConfig,
    /// Trees.
    pub trees: TreeConfig,
    /// Clouds.
    pub clouds: CloudConfig,
    /// Craters.
    pub craters: CraterConfig,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            world_size_m: 4096.0,
            terrain_cell_m: 8.0,
            noise_wavelength_m: 900.0,
            noise_amplitude_m: 35.0,
            noise_octaves: 4,
            noise_persistence: 0.5,
            city_count: 5,
            city_attempts: 400,
            city_min_spacing_m: 1100.0,
            open_ground_min_m: 300.0,
            city_edge_margin_m: 300.0,
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
            water: WaterConfig::default(),
            trees: TreeConfig::default(),
            clouds: CloudConfig::default(),
            craters: CraterConfig::default(),
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
        self.water.validate()?;
        self.trees.validate()?;
        self.clouds.validate()?;
        self.craters.validate()?;
        Ok(())
    }
}
