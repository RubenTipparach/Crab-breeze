//! World generation tuning, loaded from `data/world/world.ron` (CLAUDE.md section 6.4).
//!
//! Every field carries its unit in its name. The numbers and why they are what they are live in
//! `openspec/changes/procgen-world/design.md`.

use crate::data::{Validate, finite, positive, positive_range, within};
use glam::Vec2;
use serde::Deserialize;

/// Rivers, lakes and waterfalls (openspec changes `terrain-water`, `hydrology`). Where water goes is
/// worked out on a coarse drainage grid: every cell drains to its lowest neighbour, basins fill to
/// their spill point as lakes, and a cell whose drained snowmelt and rain pass `stream_min_flow` is
/// a stream; streams meet and widen into rivers (hydrology design section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct WaterConfig {
    /// Every city's ground stands at least this far above sea level.
    pub city_freeboard_m: f32,
    /// Cell size of the drainage grid.
    pub drainage_cell_m: f32,
    /// Rain a lowland cell gives, as a share of what a cell above the snowline gives in melt.
    pub lowland_rain: f32,
    /// Melt ramps up over this height below the snowline (`range.snowline_m`).
    pub melt_band_m: f32,
    /// A cell is a stream when the water draining through it is at least this many snow cells' worth.
    pub stream_min_flow: f32,
    /// Fewest drainage cells a stream runs before it meets another or the sea.
    pub stream_min_cells: u32,
    /// Spacing of the points along a river's course.
    pub river_step_m: f32,
    /// Least distance between a river or lake and a city square.
    pub river_city_clearance_m: f32,
    /// Half width of a stream at `stream_min_flow`; a river is wider by the square root of its flow.
    pub stream_half_width_m: f32,
    /// Widest half width of a river.
    pub river_max_half_width_m: f32,
    /// Depth at the channel's centre (a stream's is at most half its width).
    pub river_depth_m: f32,
    /// Water surface below the surrounding ground.
    pub freeboard_m: f32,
    /// Rise of a riverbank per metre outward (0.5 is about 27 degrees).
    pub bank_slope: f32,
    /// River flow speed.
    pub flow_m_s: f32,
    /// A basin is a lake when the water stands at least this deep in it.
    pub lake_min_depth_m: f32,
    /// A basin is a lake when it covers at least this many drainage cells.
    pub lake_min_cells: u32,
    /// Most lakes in basins (the largest are kept); plunge pools under waterfalls come on top.
    pub lake_max_count: u32,
    /// Radius range of a lake.
    pub lake_radius_m: [f32; 2],
    /// Depth of a lake's bowl at its centre.
    pub lake_depth_m: f32,
    /// How far the shore wanders in or out, as a share of the radius.
    pub lake_shore_wobble: f32,
    /// A river falls (a waterfall) where its surface drops at least this much in one `river_step_m`.
    pub fall_step_m: f32,
    /// Least height of a waterfall.
    pub fall_min_height_m: f32,
    /// A waterfall's plunge pool has this radius per metre of fall, within `lake_radius_m`'s lower half.
    pub plunge_radius_per_m: f32,
}

impl Default for WaterConfig {
    fn default() -> Self {
        Self {
            city_freeboard_m: 2.0,
            drainage_cell_m: 128.0,
            lowland_rain: 0.08,
            melt_band_m: 400.0,
            stream_min_flow: 12.0,
            stream_min_cells: 3,
            river_step_m: 16.0,
            river_city_clearance_m: 80.0,
            stream_half_width_m: 5.0,
            river_max_half_width_m: 28.0,
            river_depth_m: 7.0,
            freeboard_m: 2.0,
            bank_slope: 0.45,
            flow_m_s: 1.5,
            lake_min_depth_m: 8.0,
            lake_min_cells: 16,
            lake_max_count: 10,
            lake_radius_m: [70.0, 900.0],
            lake_depth_m: 14.0,
            lake_shore_wobble: 0.12,
            fall_step_m: 12.0,
            fall_min_height_m: 25.0,
            plunge_radius_per_m: 1.4,
        }
    }
}

impl Validate for WaterConfig {
    fn validate(&self) -> Result<(), String> {
        finite("water.city_freeboard_m", self.city_freeboard_m)?;
        positive("water.drainage_cell_m", self.drainage_cell_m)?;
        within("water.lowland_rain", self.lowland_rain, 0.0, 1.0)?;
        positive("water.melt_band_m", self.melt_band_m)?;
        positive("water.stream_min_flow", self.stream_min_flow)?;
        positive("water.river_step_m", self.river_step_m)?;
        finite("water.river_city_clearance_m", self.river_city_clearance_m)?;
        positive("water.stream_half_width_m", self.stream_half_width_m)?;
        positive("water.river_max_half_width_m", self.river_max_half_width_m)?;
        if self.river_max_half_width_m < self.stream_half_width_m {
            return Err("`water.river_max_half_width_m` must be at least `water.stream_half_width_m`".into());
        }
        positive("water.river_depth_m", self.river_depth_m)?;
        finite("water.freeboard_m", self.freeboard_m)?;
        positive("water.bank_slope", self.bank_slope)?;
        finite("water.flow_m_s", self.flow_m_s)?;
        positive("water.lake_min_depth_m", self.lake_min_depth_m)?;
        positive_range("water.lake_radius_m", self.lake_radius_m)?;
        positive("water.lake_depth_m", self.lake_depth_m)?;
        within("water.lake_shore_wobble", self.lake_shore_wobble, 0.0, 0.5)?;
        positive("water.fall_step_m", self.fall_step_m)?;
        positive("water.fall_min_height_m", self.fall_min_height_m)?;
        positive("water.plunge_radius_per_m", self.plunge_radius_per_m)?;
        Ok(())
    }
}

/// The mountain range (openspec change `hydrology`, from `island-biomes`): a crater of mountains
/// round the island's middle (owner, 2026-10-25: "need a crater like moutain range at center of
/// islan"): ridged peaks under an envelope round a rim circle, snow above the snowline, a cliff band
/// along each side of the rim (inside, the crater's wall) that the rivers fall over, and passes
/// cut through it for the rivers and roads.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RangeConfig {
    /// Middle of the crater (x, z).
    pub centre_m: [f32; 2],
    /// Radius of the rim's crest.
    pub radius_m: f32,
    /// The range falls to nothing this far either side of the rim.
    pub half_width_m: f32,
    /// Height of the highest peaks above the land under them; zero for no range.
    pub peak_m: f32,
    /// Wavelength of the ridges' first octave.
    pub ridge_wavelength_m: f32,
    /// Ridge octaves (fewer is smoother).
    pub ridge_octaves: u32,
    /// How far the ridges are pushed about, so they do not run in lattice lines.
    pub warp_m: f32,
    /// Snow lies above this height.
    pub snowline_m: f32,
    /// The cliff band runs this far from the rim on each side.
    pub scarp_offset_m: f32,
    /// Height of the cliff band.
    pub scarp_m: f32,
    /// Width over which the cliff band drops.
    pub scarp_width_m: f32,
    /// Most the cliff band's line wanders in or out.
    pub scarp_wobble_m: f32,
    /// Bearings of the passes through the rim, degrees clockwise from north (-z).
    pub passes_deg: Vec<f32>,
    /// A pass is open within half this of its bearing and closes over the rest.
    pub pass_width_deg: f32,
}

impl Default for RangeConfig {
    fn default() -> Self {
        Self {
            centre_m: [0.0, 0.0],
            radius_m: 5500.0,
            half_width_m: 3000.0,
            peak_m: 1700.0,
            ridge_wavelength_m: 3600.0,
            ridge_octaves: 5,
            warp_m: 900.0,
            snowline_m: 900.0,
            scarp_offset_m: 1800.0,
            scarp_m: 60.0,
            scarp_width_m: 14.0,
            scarp_wobble_m: 300.0,
            passes_deg: vec![35.0, 160.0, 265.0],
            pass_width_deg: 18.0,
        }
    }
}

impl Validate for RangeConfig {
    fn validate(&self) -> Result<(), String> {
        finite("range.centre_m", self.centre_m[0])?;
        finite("range.centre_m", self.centre_m[1])?;
        positive("range.radius_m", self.radius_m)?;
        positive("range.half_width_m", self.half_width_m)?;
        for &b in &self.passes_deg {
            finite("range.passes_deg", b)?;
        }
        within("range.pass_width_deg", self.pass_width_deg, 0.0, 180.0)?;
        within("range.peak_m", self.peak_m, 0.0, 5000.0)?;
        positive("range.ridge_wavelength_m", self.ridge_wavelength_m)?;
        if self.ridge_octaves == 0 || self.ridge_octaves > 8 {
            return Err("`range.ridge_octaves` must be 1 to 8".into());
        }
        finite("range.warp_m", self.warp_m)?;
        finite("range.snowline_m", self.snowline_m)?;
        positive("range.scarp_offset_m", self.scarp_offset_m)?;
        if self.scarp_offset_m >= self.half_width_m {
            return Err("`range.scarp_offset_m` must be inside `range.half_width_m`".into());
        }
        within("range.scarp_m", self.scarp_m, 0.0, 500.0)?;
        positive("range.scarp_width_m", self.scarp_width_m)?;
        finite("range.scarp_wobble_m", self.scarp_wobble_m)?;
        Ok(())
    }
}

/// The biomes (openspec change `island-biomes`): where the island dries out, and the bands each
/// biome's border fades over.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BiomeConfig {
    /// The island dries out toward this direction (x, z): the lee of the prevailing wind.
    pub dry_toward: [f32; 2],
    /// Along that direction, moisture starts to fall this far from the centre...
    pub dry_from_m: f32,
    /// ...and is gone this far.
    pub dry_to_m: f32,
    /// Most the dry side's edge wanders in or out.
    pub wander_m: f32,
    /// Wavelength of that wandering.
    pub wander_wavelength_m: f32,
    /// Share of the moisture that is patchy noise rather than the lee's fall, 0..1.
    pub patch_share: f32,
    /// Wavelength of the patches.
    pub patch_wavelength_m: f32,
    /// Below this moisture the desert takes over (tuned so it is a quarter of the land).
    pub desert_moisture: f32,
    /// Width of each moisture border, in moisture.
    pub border_band: f32,
    /// Above this moisture low ground is wetland.
    pub wetland_moisture: f32,
    /// Most the height borders (snow, rock, forest) wander up or down.
    pub height_jitter_m: f32,
    /// Bare rock below the snowline over this height.
    pub alpine_band_m: f32,
    /// The range's forest fades in over this height range.
    pub montane_m: [f32; 2],
    /// Slope (rise over run) over which ground turns to bare rock.
    pub rock_slope: [f32; 2],
    /// Beach sand up to this height above the sea.
    pub beach_m: f32,
}

impl Default for BiomeConfig {
    fn default() -> Self {
        Self {
            dry_toward: [0.92, 0.38],
            dry_from_m: -2000.0,
            dry_to_m: 12000.0,
            wander_m: 3500.0,
            wander_wavelength_m: 6500.0,
            patch_share: 0.25,
            patch_wavelength_m: 3000.0,
            desert_moisture: 0.40,
            border_band: 0.13,
            wetland_moisture: 0.8,
            height_jitter_m: 120.0,
            alpine_band_m: 250.0,
            montane_m: [250.0, 450.0],
            rock_slope: [0.9, 1.4],
            beach_m: 4.0,
        }
    }
}

impl Validate for BiomeConfig {
    fn validate(&self) -> Result<(), String> {
        finite("biomes.dry_toward", self.dry_toward[0])?;
        finite("biomes.dry_toward", self.dry_toward[1])?;
        if Vec2::from(self.dry_toward).length() < 1e-3 {
            return Err("`biomes.dry_toward` must be a direction, not zero".into());
        }
        finite("biomes.dry_from_m", self.dry_from_m)?;
        if self.dry_to_m <= self.dry_from_m {
            return Err("`biomes.dry_to_m` must be past `biomes.dry_from_m`".into());
        }
        finite("biomes.wander_m", self.wander_m)?;
        positive("biomes.wander_wavelength_m", self.wander_wavelength_m)?;
        within("biomes.patch_share", self.patch_share, 0.0, 1.0)?;
        positive("biomes.patch_wavelength_m", self.patch_wavelength_m)?;
        within("biomes.desert_moisture", self.desert_moisture, 0.0, 1.0)?;
        positive("biomes.border_band", self.border_band)?;
        within("biomes.wetland_moisture", self.wetland_moisture, 0.0, 1.0)?;
        finite("biomes.height_jitter_m", self.height_jitter_m)?;
        positive("biomes.alpine_band_m", self.alpine_band_m)?;
        positive_range("biomes.montane_m", self.montane_m)?;
        positive_range("biomes.rock_slope", self.rock_slope)?;
        positive("biomes.beach_m", self.beach_m)?;
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

/// Clouds (openspec changes `clouds`, `cloud-puffs`, `cloud-weather`): the wind, how far they are
/// drawn and how they are lit. Where they are is the weather's (`weather:`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CloudConfig {
    /// Wind drift (x, z): everything in the sky moves with it.
    pub wind_m_s: [f32; 2],
    /// Cloud puffs are drawn within this of the eye (beyond it the haze has hidden them).
    pub draw_radius_m: f32,
    /// The drawn puffs are gathered and sorted again when the eye has moved this far through them.
    pub resort_m: f32,
    /// How far the sun wraps round a puff, 0 (hard) to 1 (soft) (`cloud-puffs`).
    pub look_wrap: f32,
    /// The silver lining's strength toward the sun.
    pub look_silver: f32,
    /// Light scattered many times inside a puff, a share of the sun's (so a backlit puff is grey).
    pub look_scattered: f32,
    /// How white a white cloud is (its albedo, 0..1); a storm-grey one is `look_storm_albedo`.
    pub look_albedo: f32,
    /// The albedo of a puff at full darkness.
    pub look_storm_albedo: f32,
    /// How much darker a white cloud's base is than its top, 0..1; a storm's is `look_storm_base_shade`.
    pub look_base_shade: f32,
    /// The base darkening of a puff at full darkness.
    pub look_storm_base_shade: f32,
    /// A puff fades in from this distance from the eye (closer, the cloud fog takes over).
    pub look_near_fade_m: f32,
    /// How far noise eats into a billow's edge (0 a clean disc).
    pub look_erode: f32,
    /// A billboard fades out over this depth in front of the ground behind it (soft particles).
    pub look_soft_m: f32,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            wind_m_s: [6.0, 0.0],
            draw_radius_m: 24000.0,
            resort_m: 30.0,
            look_wrap: 0.35,
            look_silver: 1.6,
            look_scattered: 0.2,
            look_albedo: 0.92,
            look_storm_albedo: 0.55,
            look_base_shade: 0.55,
            look_storm_base_shade: 0.85,
            look_near_fade_m: 140.0,
            look_erode: 0.62,
            look_soft_m: 60.0,
        }
    }
}

impl Validate for CloudConfig {
    fn validate(&self) -> Result<(), String> {
        finite("clouds.wind_m_s", self.wind_m_s[0])?;
        finite("clouds.wind_m_s", self.wind_m_s[1])?;
        positive("clouds.draw_radius_m", self.draw_radius_m)?;
        positive("clouds.resort_m", self.resort_m)?;
        within("clouds.look_wrap", self.look_wrap, 0.0, 1.0)?;
        within("clouds.look_silver", self.look_silver, 0.0, 10.0)?;
        within("clouds.look_scattered", self.look_scattered, 0.0, 2.0)?;
        within("clouds.look_albedo", self.look_albedo, 0.0, 1.0)?;
        within("clouds.look_storm_albedo", self.look_storm_albedo, 0.0, 1.0)?;
        within("clouds.look_base_shade", self.look_base_shade, 0.0, 1.0)?;
        within("clouds.look_storm_base_shade", self.look_storm_base_shade, 0.0, 1.0)?;
        positive("clouds.look_near_fade_m", self.look_near_fade_m)?;
        within("clouds.look_erode", self.look_erode, 0.0, 2.0)?;
        positive("clouds.look_soft_m", self.look_soft_m)?;
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
            land_height_m: 50.0,
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
    /// The mountain range.
    pub range: RangeConfig,
    /// The biomes.
    pub biomes: BiomeConfig,
    /// A city's ground stands at most this far above sea level (off the mountains).
    pub city_max_ground_m: f32,
    /// The land under a city square and its blend rises or falls at most this far from its ground.
    pub city_max_relief_m: f32,
    /// A highway's route never climbs more than this above the higher of its two cities.
    pub highway_max_climb_m: f32,
    /// Steepest a highway's grade climbs (rise over run).
    pub highway_max_grade: f32,
    /// A route step at the steepest grade costs this many times more than one on the level (more
    /// at steeper, as the square of the grade).
    pub highway_slope_cost: f32,
    /// Crossing a stream costs the route this many metres of road (a bridge).
    pub highway_bridge_cost_m: f32,
    /// A route may swing this far outside the box round its two ends.
    pub highway_detour_m: f32,
    /// The grade follows the land averaged over this distance either side.
    pub highway_grade_smooth_m: f32,
    /// Levels of detail of the drawn ground.
    pub terrain_lod: LodConfig,
    /// How far the pilot sees.
    pub view: ViewConfig,
    /// Rivers and lakes.
    pub water: WaterConfig,
    /// Trees.
    pub trees: TreeConfig,
    /// Clouds: wind, drawing and light.
    pub clouds: CloudConfig,
    /// The weather systems that lay out the sky (openspec change `cloud-weather`).
    pub weather: super::weather::WeatherConfig,
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
            noise_amplitude_m: 28.0,
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
            road_lift_m: 0.15,
            kerb_height_m: 0.2,
            start_pilot_distance_m: 25.0,
            island: IslandConfig::default(),
            range: RangeConfig::default(),
            biomes: BiomeConfig::default(),
            city_max_ground_m: 260.0,
            city_max_relief_m: 70.0,
            highway_max_climb_m: 45.0,
            highway_max_grade: 0.06,
            highway_slope_cost: 6.0,
            highway_bridge_cost_m: 600.0,
            highway_detour_m: 4000.0,
            highway_grade_smooth_m: 240.0,
            terrain_lod: LodConfig::default(),
            view: ViewConfig::default(),
            water: WaterConfig::default(),
            trees: TreeConfig::default(),
            clouds: CloudConfig::default(),
            weather: super::weather::WeatherConfig::default(),
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
        self.range.validate()?;
        self.biomes.validate()?;
        positive("city_max_ground_m", self.city_max_ground_m)?;
        positive("city_max_relief_m", self.city_max_relief_m)?;
        positive("highway_max_climb_m", self.highway_max_climb_m)?;
        positive("highway_max_grade", self.highway_max_grade)?;
        finite("highway_slope_cost", self.highway_slope_cost)?;
        finite("highway_bridge_cost_m", self.highway_bridge_cost_m)?;
        positive("highway_detour_m", self.highway_detour_m)?;
        positive("highway_grade_smooth_m", self.highway_grade_smooth_m)?;
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
        self.weather.validate()?;
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
