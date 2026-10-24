//! World generation tuning, loaded from `data/world/world.ron` (CLAUDE.md section 6.4).
//!
//! Every field carries its unit in its name. The numbers and why they are what they are live in
//! `openspec/changes/procgen-world/design.md`.

use crate::data::{Validate, finite, positive, positive_range, within};
use serde::Deserialize;

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
        Ok(())
    }
}
