//! Grass (openspec change `gpu-grass`): the rules the GPU grass is drawn by, with no engine in
//! them. Every blade is built in the vertex shader (`assets/shaders/grass.wesl`); what the CPU
//! decides is here, pure and tested headless:
//!
//! - the tuning, `data/world/grass.ron`, loaded by the one data loader;
//! - which square tiles round the eye carry grass, and at which level of detail;
//! - the shared blade mesh of each level, whose vertices carry only (blade index, vertex in blade,
//!   level);
//! - a tile's bake: the ground's height on the terrain's own lattice (read through the one height
//!   rule, `Heightfield::sample` and `terrain::cell_height`) and where grass grows (the shared
//!   `WorldPlan::cover_at`, the one water query, the slope, the beach and the craters), packed into
//!   the words the shader reads.

use crate::data::{Validate, finite, finite3, positive, positive_range, within};
use crate::world::cover::{Clearance, Cover};
use crate::world::layout::Rect;
use crate::world::terrain::cell_height;
use crate::world::{Deformation, WorldConfig, WorldPlan, lod};
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// The most levels of detail the shader holds.
pub const MAX_LODS: usize = 4;
/// The most bodies that flatten grass at once (the shader's pusher list).
pub const MAX_PUSHERS: usize = 8;

/// One level of detail of the grass.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GrassLod {
    /// A tile whose nearest point is nearer than this (metres from the eye) is drawn at this level.
    pub until_m: f32,
    /// Every `stride`-th blade cell along each side is drawn (1: all of them): blades per square
    /// metre fall by `stride` squared. Each level's stride divides the next one's, so a farther
    /// level's blades are a subset of a nearer one's and nothing pops as a tile changes level.
    pub stride: u32,
    /// Segments along a blade (vertices `2 * segments + 1`, triangles `2 * segments - 1`).
    pub segments: u32,
    /// Blade width at this level as a multiple of `width_m` (farther blades are wider, so fewer
    /// of them still cover the ground).
    pub width_scale: f32,
}

/// What grass keeps clear of, metres (`world::cover::Clearance`, as data).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GrassClearance {
    /// Outside a city, no grass within this distance of its square.
    pub city_m: f32,
    /// A park's grass stops this far inside its block's edge.
    pub park_inset_m: f32,
    /// No grass within this distance of a highway's paved edge.
    pub road_m: f32,
    /// No grass within this share of a bridge's length of its centre.
    pub bridge_share: f32,
    /// No grass within this distance of a building's footprint.
    pub building_m: f32,
}

impl Default for GrassClearance {
    fn default() -> Self {
        Self {
            city_m: 3.0,
            park_inset_m: 2.0,
            road_m: 1.5,
            bridge_share: 0.6,
            building_m: 0.5,
        }
    }
}

impl From<GrassClearance> for Clearance {
    fn from(c: GrassClearance) -> Self {
        Clearance {
            city_m: c.city_m,
            park_inset_m: c.park_inset_m,
            road_m: c.road_m,
            bridge_share: c.bridge_share,
            building_m: c.building_m,
        }
    }
}

/// The grass. Every field is in `data/world/grass.ron`; a missing field takes the default below.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GrassConfig {
    /// Side of a grass tile, metres; a multiple of the terrain's cell, of `blade_spacing_m` and of
    /// `density_cell_m`.
    pub tile_m: f32,
    /// Grass is drawn on tiles within this distance of the eye (across the ground and up to the
    /// eye), metres; beyond it the ground's own colour carries the grass.
    pub radius_m: f32,
    /// A tile is let go once this far past `radius_m`, metres (so a pilot pacing on the edge does not
    /// rebuild it).
    pub release_m: f32,
    /// Blades shrink to nothing over the last `fade_m` metres before `radius_m`.
    pub fade_m: f32,
    /// Spacing of the blade grid at the nearest level, metres (0.25 m: 16 blades a square metre).
    pub blade_spacing_m: f32,
    /// Where-grass-grows is baked per cell of this side, metres.
    pub density_cell_m: f32,
    /// Most tile bakes started in one frame (they run off the main thread).
    pub bakes_per_frame: u32,
    /// Levels of detail, nearest first (at most `MAX_LODS`); the last reaches `radius_m`.
    pub lods: Vec<GrassLod>,
    /// A tile changes level only once this far past the boundary, metres.
    pub lod_hysteresis_m: f32,
    /// Blades a farther level drops shrink away over this band before the boundary, metres.
    pub crossfade_m: f32,
    /// What grass keeps clear of.
    pub clearance: GrassClearance,
    /// No grass on ground lower than this above the sea (the sand), metres.
    pub beach_m: f32,
    /// Ground slope (rise over run) where grass starts thinning, and where none is left.
    pub slope_bare: [f32; 2],
    /// Ground dug or raised by a crater by more than this has no grass, metres.
    pub crater_clear_m: f32,
    /// Blade height range, metres.
    pub height_m: [f32; 2],
    /// Blade width at the root, metres.
    pub width_m: f32,
    /// How the blade narrows to its tip: width is `1 - t^taper` of the root's at `t` along it.
    pub taper: f32,
    /// The most a blade leans over at rest, as a share of its height.
    pub lean_max: f32,
    /// How stiff a blade is, 0..1: a stiff blade bends near its tip, a soft one from its root.
    pub stiffness: f32,
    /// Height of the blade's middle control point, as a share of its height.
    pub mid_height: f32,
    /// How far an edge-on blade turns toward the eye so it never thins to nothing, 0..1.
    pub edge_thicken: f32,
    /// How far the normal tilts across the blade's width, so it shades like a curved leaf.
    pub normal_round: f32,
    /// Colour at the root, linear RGB (an albedo).
    pub root_colour: [f32; 3],
    /// Colour at the tip, linear RGB.
    pub tip_colour: [f32; 3],
    /// Colour of a dry blade, linear RGB.
    pub dry_colour: [f32; 3],
    /// Share of blades that are dry, 0..1.
    pub dry_share: f32,
    /// Light reaching the root, 0..1 (the blades shade each other there).
    pub root_light: f32,
    /// Perceptual roughness of a blade, 0..1.
    pub roughness: f32,
    /// Side of a clump (blades in one share a height and a facing), metres.
    pub clump_m: f32,
    /// How strongly a clump overrides each blade's own height and facing, 0..1.
    pub clump_strength: f32,
    /// The wind's heading, degrees from +x toward +z.
    pub wind_heading_deg: f32,
    /// How fast gusts cross the grass, metres a second.
    pub wind_speed_m_s: f32,
    /// How far the steady wind pushes a tip, as a share of the blade's height.
    pub wind_push: f32,
    /// Gust noise cells per metre.
    pub gust_per_m: f32,
    /// How much of the push comes and goes with the gusts, 0..1.
    pub gust_share: f32,
    /// Each blade's own sway, cycles a second.
    pub sway_hz: f32,
    /// Each blade's own sway, as a share of its height.
    pub sway: f32,
    /// Radius about the pilot's feet in which grass is pressed flat, metres.
    pub pilot_press_m: f32,
    /// Radius about each of a mech's feet in which grass is pressed flat, metres.
    pub mech_foot_press_m: f32,
    /// How far a tall blade's tip droops over, as a share of its height at the tallest.
    pub droop: f32,
    /// Share of clumps that stand tall.
    pub clump_tall_share: f32,
    /// A tall clump's height as a multiple of its blades' own.
    pub clump_tall_scale: f32,
    /// Blades at a clump's edge are this share shorter than at its middle (a clump is a mound).
    pub clump_edge_shrink: f32,
    /// Extra darkening at the root in a dense, tall clump, 0..1 (fake ambient occlusion).
    pub ao_dense: f32,
    /// Share of the blade's height over which the root's darkening fades out.
    pub ao_height_share: f32,
    /// Specular reflectance of a blade at normal incidence (F0), 0..1.
    pub reflectance: f32,
    /// A second green, linear RGB: each clump's tips lean toward it by a random share of
    /// `hue_variety`.
    pub hue_colour: [f32; 3],
    /// How far a clump's hue may move toward `hue_colour`, 0..1.
    pub hue_variety: f32,
    /// How much lighter a tip is than the blade's colour, 0..1 toward white.
    pub tip_lighten: f32,
    /// The colour sunlight takes through a blade (translucency), linear RGB.
    pub translucent_colour: [f32; 3],
    /// How strongly a blade glows with the sun behind it, 0..1.
    pub translucency: f32,
    /// How tightly the glow gathers round the sun's direction (an exponent).
    pub translucency_power: f32,
    /// Roughness far away, where the sheen is faded out so it does not sparkle.
    pub far_roughness: f32,
    /// Distance over which a blade's gloss fades from `roughness` to `far_roughness`, metres.
    pub gloss_fade_m: [f32; 2],
    /// Distance over which blade normals blend toward straight up, metres (far grass shades
    /// like the ground and does not alias).
    pub normal_up_m: [f32; 2],
    /// How sharp the gust waves are (an exponent on the gust noise): higher, narrower bands.
    pub gust_sharpness: f32,
    /// How much lighter a blade bent by a gust looks, 0..1 (the bands rolling across a field).
    pub gust_brighten: f32,
    /// Each blade's flutter, cycles a second.
    pub flutter_hz: f32,
    /// Each blade's flutter, as a share of its height, at the tip.
    pub flutter: f32,
    /// Wavelength of the broad sway across the whole field, metres.
    pub broad_wavelength_m: f32,
    /// The broad sway, cycles a second.
    pub broad_hz: f32,
    /// The broad sway, as a share of the height.
    pub broad_push: f32,
    /// The colour the ground takes under grass, a linear albedo (over the ground's neutral grain,
    /// like every ground vertex colour).
    pub ground_tint_colour: [f32; 3],
    /// How far the ground's colour moves toward `ground_tint_colour` where grass grows fully, 0..1.
    pub ground_tint: f32,
}

impl Default for GrassConfig {
    fn default() -> Self {
        Self {
            tile_m: 32.0,
            radius_m: 300.0,
            release_m: 48.0,
            fade_m: 60.0,
            blade_spacing_m: 0.2,
            density_cell_m: 1.0,
            bakes_per_frame: 6,
            lods: vec![
                GrassLod {
                    until_m: 25.0,
                    stride: 1,
                    segments: 5,
                    width_scale: 1.0,
                },
                GrassLod {
                    until_m: 110.0,
                    stride: 2,
                    segments: 3,
                    width_scale: 1.7,
                },
                GrassLod {
                    until_m: 300.0,
                    stride: 4,
                    segments: 2,
                    width_scale: 2.8,
                },
            ],
            lod_hysteresis_m: 6.0,
            crossfade_m: 15.0,
            clearance: GrassClearance::default(),
            beach_m: 2.5,
            slope_bare: [0.6, 1.1],
            crater_clear_m: 0.05,
            height_m: [0.6, 1.6],
            width_m: 0.06,
            taper: 1.4,
            lean_max: 0.45,
            stiffness: 0.5,
            mid_height: 0.55,
            edge_thicken: 0.6,
            normal_round: 0.6,
            root_colour: [0.05, 0.09, 0.025],
            tip_colour: [0.22, 0.33, 0.09],
            dry_colour: [0.36, 0.33, 0.16],
            dry_share: 0.22,
            root_light: 0.35,
            roughness: 0.65,
            clump_m: 2.5,
            clump_strength: 0.5,
            wind_heading_deg: 30.0,
            wind_speed_m_s: 5.0,
            wind_push: 0.22,
            gust_per_m: 0.04,
            gust_share: 0.7,
            sway_hz: 0.7,
            sway: 0.05,
            pilot_press_m: 0.7,
            mech_foot_press_m: 2.6,
            droop: 0.35,
            clump_tall_share: 0.18,
            clump_tall_scale: 1.3,
            clump_edge_shrink: 0.3,
            ao_dense: 0.5,
            ao_height_share: 0.55,
            reflectance: 0.35,
            hue_colour: [0.30, 0.34, 0.07],
            hue_variety: 0.6,
            tip_lighten: 0.12,
            translucent_colour: [0.55, 0.62, 0.12],
            translucency: 0.6,
            translucency_power: 4.0,
            far_roughness: 0.95,
            gloss_fade_m: [20.0, 120.0],
            normal_up_m: [40.0, 220.0],
            gust_sharpness: 2.0,
            gust_brighten: 0.25,
            flutter_hz: 3.0,
            flutter: 0.025,
            broad_wavelength_m: 70.0,
            broad_hz: 0.12,
            broad_push: 0.1,
            ground_tint_colour: [0.40, 0.52, 0.20],
            ground_tint: 0.85,
        }
    }
}

/// True when `a` is a whole multiple of `b` (to a millimetre).
fn multiple_of(a: f32, b: f32) -> bool {
    let k = (a / b).round();
    k >= 1.0 && (a - k * b).abs() < 1e-3
}

impl Validate for GrassConfig {
    fn validate(&self) -> Result<(), String> {
        positive("tile_m", self.tile_m)?;
        positive("radius_m", self.radius_m)?;
        within("release_m", self.release_m, 0.0, 1000.0)?;
        within("fade_m", self.fade_m, 0.0, self.radius_m)?;
        positive("blade_spacing_m", self.blade_spacing_m)?;
        positive("density_cell_m", self.density_cell_m)?;
        if !multiple_of(self.tile_m, self.blade_spacing_m) {
            return Err(format!("`tile_m` ({}) must be a multiple of `blade_spacing_m`", self.tile_m));
        }
        if !multiple_of(self.tile_m, self.density_cell_m) {
            return Err(format!("`tile_m` ({}) must be a multiple of `density_cell_m`", self.tile_m));
        }
        if self.bakes_per_frame == 0 {
            return Err("`bakes_per_frame` must be at least 1".into());
        }
        if self.lods.is_empty() || self.lods.len() > MAX_LODS {
            return Err(format!("`lods` must hold 1 to {MAX_LODS} levels, got {}", self.lods.len()));
        }
        let mut until = 0.0;
        let mut stride = 1;
        for (i, l) in self.lods.iter().enumerate() {
            positive("lods.until_m", l.until_m)?;
            positive("lods.width_scale", l.width_scale)?;
            if l.until_m <= until {
                return Err(format!("`lods[{i}].until_m` must grow level by level, got {}", l.until_m));
            }
            if l.stride == 0 || l.stride % stride != 0 {
                return Err(format!(
                    "`lods[{i}].stride` must be a multiple of the level before's ({stride}), got {}",
                    l.stride
                ));
            }
            if !(1..=8).contains(&l.segments) {
                return Err(format!("`lods[{i}].segments` must be 1 to 8, got {}", l.segments));
            }
            until = l.until_m;
            stride = l.stride;
        }
        if until < self.radius_m {
            return Err(format!(
                "the last level's `until_m` ({until}) must reach `radius_m` ({})",
                self.radius_m
            ));
        }
        within("lod_hysteresis_m", self.lod_hysteresis_m, 0.0, 100.0)?;
        within("crossfade_m", self.crossfade_m, 0.0, 200.0)?;
        let c = &self.clearance;
        within("clearance.city_m", c.city_m, 0.0, 1000.0)?;
        within("clearance.park_inset_m", c.park_inset_m, 0.0, 100.0)?;
        within("clearance.road_m", c.road_m, 0.0, 100.0)?;
        within("clearance.bridge_share", c.bridge_share, 0.0, 2.0)?;
        positive("clearance.building_m", c.building_m)?;
        finite("beach_m", self.beach_m)?;
        positive_range("slope_bare", self.slope_bare)?;
        within("crater_clear_m", self.crater_clear_m, 0.0, 10.0)?;
        positive_range("height_m", self.height_m)?;
        positive("width_m", self.width_m)?;
        positive("taper", self.taper)?;
        within("lean_max", self.lean_max, 0.0, 0.95)?;
        within("stiffness", self.stiffness, 0.0, 1.0)?;
        within("mid_height", self.mid_height, 0.05, 0.95)?;
        within("edge_thicken", self.edge_thicken, 0.0, 1.0)?;
        within("normal_round", self.normal_round, 0.0, 4.0)?;
        for (field, c) in [
            ("root_colour", self.root_colour),
            ("tip_colour", self.tip_colour),
            ("dry_colour", self.dry_colour),
        ] {
            finite3(field, c)?;
            for x in c {
                within(field, x, 0.0, 1.0)?;
            }
        }
        within("dry_share", self.dry_share, 0.0, 1.0)?;
        within("root_light", self.root_light, 0.0, 1.0)?;
        within("roughness", self.roughness, 0.0, 1.0)?;
        positive("clump_m", self.clump_m)?;
        within("clump_strength", self.clump_strength, 0.0, 1.0)?;
        finite("wind_heading_deg", self.wind_heading_deg)?;
        within("wind_speed_m_s", self.wind_speed_m_s, 0.0, 100.0)?;
        within("wind_push", self.wind_push, 0.0, 0.95)?;
        positive("gust_per_m", self.gust_per_m)?;
        within("gust_share", self.gust_share, 0.0, 1.0)?;
        within("sway_hz", self.sway_hz, 0.0, 20.0)?;
        within("sway", self.sway, 0.0, 0.95)?;
        within("pilot_press_m", self.pilot_press_m, 0.0, 50.0)?;
        within("mech_foot_press_m", self.mech_foot_press_m, 0.0, 50.0)?;
        within("droop", self.droop, 0.0, 1.0)?;
        within("clump_tall_share", self.clump_tall_share, 0.0, 1.0)?;
        within("clump_tall_scale", self.clump_tall_scale, 1.0, 4.0)?;
        within("clump_edge_shrink", self.clump_edge_shrink, 0.0, 0.9)?;
        within("ao_dense", self.ao_dense, 0.0, 1.0)?;
        within("ao_height_share", self.ao_height_share, 0.05, 1.0)?;
        within("reflectance", self.reflectance, 0.0, 1.0)?;
        within("hue_variety", self.hue_variety, 0.0, 1.0)?;
        within("tip_lighten", self.tip_lighten, 0.0, 1.0)?;
        within("translucency", self.translucency, 0.0, 4.0)?;
        positive("translucency_power", self.translucency_power)?;
        within("far_roughness", self.far_roughness, 0.0, 1.0)?;
        positive_range("gloss_fade_m", self.gloss_fade_m)?;
        positive_range("normal_up_m", self.normal_up_m)?;
        positive("gust_sharpness", self.gust_sharpness)?;
        within("gust_brighten", self.gust_brighten, 0.0, 1.0)?;
        within("flutter_hz", self.flutter_hz, 0.0, 50.0)?;
        within("flutter", self.flutter, 0.0, 0.5)?;
        positive("broad_wavelength_m", self.broad_wavelength_m)?;
        within("broad_hz", self.broad_hz, 0.0, 10.0)?;
        within("broad_push", self.broad_push, 0.0, 0.95)?;
        within("ground_tint", self.ground_tint, 0.0, 1.0)?;
        for (field, c) in [
            ("hue_colour", self.hue_colour),
            ("translucent_colour", self.translucent_colour),
            ("ground_tint_colour", self.ground_tint_colour),
        ] {
            for x in c {
                within(field, x, 0.0, 1.0)?;
            }
        }
        Ok(())
    }
}

impl GrassConfig {
    /// Checks the grass against the world it grows on: tiles sit on the terrain's lattice, and
    /// every tile in reach is on ground drawn at full detail (so a blade's root, read from the
    /// lattice, is on the drawn ground).
    pub fn validate_with(&self, world: &WorldConfig) -> Result<(), String> {
        if !multiple_of(self.tile_m, world.terrain_cell_m) {
            return Err(format!(
                "`tile_m` ({}) must be a multiple of the terrain's cell ({} m, world.ron `terrain_cell_m`)",
                self.tile_m, world.terrain_cell_m
            ));
        }
        let full = world.terrain_lod.ring_m.first().copied().unwrap_or(0.0) - world.terrain_lod.hysteresis_m;
        if self.radius_m + self.release_m > full {
            return Err(format!(
                "`radius_m` + `release_m` ({} m) must stay inside the ground's full-detail ring ({full} m, world.ron `terrain_lod`)",
                self.radius_m + self.release_m
            ));
        }
        Ok(())
    }

    /// The tallest a blade stands, metres (a tall clump's tallest blade).
    pub fn tallest_m(&self) -> f32 {
        self.height_m[1] * self.clump_tall_scale
    }

    /// Blade cells along a tile's side at the nearest level.
    pub fn cells_per_side(&self) -> u32 {
        (self.tile_m / self.blade_spacing_m).round() as u32
    }

    /// Where-grass-grows cells along a tile's side.
    pub fn density_cells(&self) -> u32 {
        (self.tile_m / self.density_cell_m).round() as u32
    }

    /// Blades in a tile at level `lod`.
    pub fn blades_per_tile(&self, lod: usize) -> u32 {
        let per_row = self.cells_per_side().div_ceil(self.lods[lod].stride);
        per_row * per_row
    }

    /// Blades a square metre at level `lod`.
    pub fn blades_per_m2(&self, lod: usize) -> f32 {
        self.blades_per_tile(lod) as f32 / (self.tile_m * self.tile_m)
    }

    /// The level a tile `distance_m` from the eye is drawn at, given the level it is drawn at now
    /// (`None`: not drawn yet). It moves only once `lod_hysteresis_m` past a boundary, so a tile on
    /// the edge does not flicker between levels.
    pub fn lod_for(&self, distance_m: f32, current: Option<usize>) -> usize {
        let plain = |d: f32| self.lods.iter().position(|l| d < l.until_m).unwrap_or(self.lods.len() - 1);
        let Some(cur) = current else {
            return plain(distance_m);
        };
        let farther = plain(distance_m - self.lod_hysteresis_m);
        let nearer = plain(distance_m + self.lod_hysteresis_m);
        if farther > cur {
            farther
        } else if nearer < cur {
            nearer
        } else {
            cur.min(self.lods.len() - 1)
        }
    }
}

/// A grass tile, counted in tiles from the world's corner (x, z).
pub type TileKey = (u32, u32);

/// The grass tiles of a world: their side and where they start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileGrid {
    /// World x and z of tile (0, 0)'s corner (the terrain's first sample, so tiles sit on its lattice).
    pub origin_m: f32,
    /// Side of a tile, metres.
    pub tile_m: f32,
    /// Tiles along a side of the world.
    pub per_side: u32,
}

impl TileGrid {
    /// The tiles of a world `world_size_m` across, its terrain starting at `origin_m`.
    pub fn new(origin_m: f32, world_size_m: f32, tile_m: f32) -> Self {
        Self {
            origin_m,
            tile_m,
            per_side: (world_size_m / tile_m).floor().max(0.0) as u32,
        }
    }

    /// The ground a tile covers.
    pub fn rect(&self, key: TileKey) -> Rect {
        let min = Vec2::splat(self.origin_m) + Vec2::new(key.0 as f32, key.1 as f32) * self.tile_m;
        Rect {
            min,
            max: min + Vec2::splat(self.tile_m),
        }
    }

    /// The tiles within `radius_m` of `eye` (across the ground to the tile's nearest point, and up
    /// to the eye's height above `ground_m`), nearest first, ties in key order, with their
    /// distances. None past the world's edge.
    pub fn within(&self, eye: Vec3, ground_m: f32, radius_m: f32) -> Vec<(TileKey, f32)> {
        let mut out = Vec::new();
        if !eye.is_finite() || !radius_m.is_finite() || self.per_side == 0 {
            return out;
        }
        let max = self.per_side as i64 - 1;
        let at = |v: f32| (((v - self.origin_m) / self.tile_m).floor() as i64).clamp(0, max) as u32;
        for z in at(eye.z - radius_m)..=at(eye.z + radius_m) {
            for x in at(eye.x - radius_m)..=at(eye.x + radius_m) {
                let d = lod::eye_distance(&self.rect((x, z)), eye, ground_m);
                if d <= radius_m {
                    out.push(((x, z), d));
                }
            }
        }
        out.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.1.cmp(&b.0.1)).then(a.0.0.cmp(&b.0.0)));
        out
    }

    /// Tiles that can be held at once: every tile within `reach_m` of an eye anywhere.
    pub fn capacity(&self, reach_m: f32) -> u32 {
        let side = (2.0 * reach_m / self.tile_m).ceil() as u32 + 2;
        side * side
    }
}

/// The vertices and triangles of one blade of `segments` segments.
pub fn blade_counts(segments: u32) -> (u32, u32) {
    (2 * segments + 1, 2 * segments - 1)
}

/// The shared blade mesh of one level: every blade of a tile, each vertex only
/// `(blade index, vertex in blade, level)`; the vertex shader builds the blade from them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BladeMesh {
    /// One per vertex: blade index, vertex in blade, level (as floats: exact below 2^24).
    pub vertices: Vec<[f32; 3]>,
    /// Triangle list.
    pub indices: Vec<u32>,
}

impl BladeMesh {
    /// The mesh for level `lod` of `cfg`. A blade's vertices run root to tip, left and right in
    /// pairs (`2i` left, `2i + 1` right at `i / segments` along it), and one tip vertex.
    pub fn new(cfg: &GrassConfig, lod: usize) -> Self {
        let segments = cfg.lods[lod].segments;
        let (per_blade, tris) = blade_counts(segments);
        let blades = cfg.blades_per_tile(lod);
        let mut vertices = Vec::with_capacity((blades * per_blade) as usize);
        let mut indices = Vec::with_capacity((blades * tris * 3) as usize);
        for b in 0..blades {
            let first = b * per_blade;
            for k in 0..per_blade {
                vertices.push([b as f32, k as f32, lod as f32]);
            }
            for i in 0..segments - 1 {
                let (l, r) = (first + 2 * i, first + 2 * i + 1);
                indices.extend([l, r, l + 2, r, r + 2, l + 2]);
            }
            let (l, r) = (first + 2 * (segments - 1), first + 2 * (segments - 1) + 1);
            indices.extend([l, r, first + 2 * segments]);
        }
        Self { vertices, indices }
    }
}

/// A tile's bake: what the shader reads about its ground.
#[derive(Clone, Debug, PartialEq)]
pub struct TileBake {
    /// The ground's height on the terrain's lattice over the tile, row by row (`n` a side).
    pub heights: Vec<f32>,
    /// Lattice samples along a side.
    pub n: usize,
    /// Where grass grows, 0..255 per density cell, row by row, craters not yet masked.
    pub open: Vec<u8>,
    /// The same with the craters masked: what is drawn.
    pub density: Vec<u8>,
    /// Lowest and highest ground in the tile, metres.
    pub ground_m: [f32; 2],
}

impl TileBake {
    /// True when no blade grows anywhere in the tile (it needs no entity and no draw).
    pub fn bare(&self) -> bool {
        self.density.iter().all(|d| *d == 0)
    }

    /// The ground's height at `local` (metres from the tile's corner), by the one cell rule.
    pub fn height_at(&self, local: Vec2, cell_m: f32) -> f32 {
        let g = local / cell_m;
        let n = self.n;
        let i = (g.x.max(0.0) as usize).min(n - 2);
        let j = (g.y.max(0.0) as usize).min(n - 2);
        let (u, v) = (g.x - i as f32, g.y - j as f32);
        let h = |a: usize, b: usize| self.heights[b * n + a];
        cell_height([h(i, j), h(i + 1, j), h(i, j + 1), h(i + 1, j + 1)], u, v)
    }

    /// Masks the craters out of the drawn density: no grass where the ground was dug or raised by
    /// more than `crater_clear_m` (the shader stands blades on the undug lattice, so a blade there
    /// would float over the bowl or sink in the rim).
    pub fn mask_craters(&mut self, dug: &Deformation, corner: Vec2, cfg: &GrassConfig) {
        let d = cfg.density_cells() as usize;
        self.density.clone_from(&self.open);
        for j in 0..d {
            for i in 0..d {
                let p = corner + (Vec2::new(i as f32, j as f32) + 0.5) * cfg.density_cell_m;
                if crater_clears(dug.delta_at(p), cfg) {
                    self.density[j * d + i] = 0;
                }
            }
        }
    }

    /// `pack`, with every density cell whose middle lies inside one of `crush` packed as zero (a
    /// mech's print or skate groove, openspec change `ground-marks`); the bake itself is kept, so a
    /// cell grows again when its mark is gone. `corner` is the tile's corner (x, z).
    pub fn pack_crushed(&self, out: &mut [u32], crush: &[CrushRect], corner: Vec2, cfg: &GrassConfig) {
        if crush.is_empty() {
            self.pack(out);
            return;
        }
        let d = cfg.density_cells() as usize;
        let mut copy = self.clone();
        for j in 0..d {
            for i in 0..d {
                let p = corner + (Vec2::new(i as f32, j as f32) + 0.5) * cfg.density_cell_m;
                if crush.iter().any(|c| c.contains(p)) {
                    copy.density[j * d + i] = 0;
                }
            }
        }
        copy.pack(out);
    }

    /// The words the shader reads for this tile: the lattice heights (f32 bits), then the
    /// density, four cells a word, low byte first. `GrassLayout::words_per_slot` long.
    pub fn pack(&self, out: &mut [u32]) {
        let (heights, density) = out.split_at_mut(self.heights.len());
        for (w, h) in heights.iter_mut().zip(&self.heights) {
            *w = h.to_bits();
        }
        density.fill(0);
        for (i, d) in self.density.iter().enumerate() {
            density[i / 4] |= u32::from(*d) << (8 * (i % 4));
        }
    }
}

/// A rectangle on the ground where the grass is crushed (a mech's print or skate groove, openspec
/// change `ground-marks`): no blade stands in a density cell whose middle lies inside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrushRect {
    /// Its middle (x, z).
    pub centre: Vec2,
    /// Its long axis, a unit vector (x, z).
    pub along: Vec2,
    /// Half its size across and along, metres.
    pub half_m: Vec2,
}

impl CrushRect {
    /// True if `p` (x, z) lies inside.
    pub fn contains(&self, p: Vec2) -> bool {
        let d = p - self.centre;
        let across = Vec2::new(self.along.y, -self.along.x);
        d.dot(self.along).abs() <= self.half_m.y && d.dot(across).abs() <= self.half_m.x
    }

    /// The radius of the circle round its middle that holds it, metres.
    pub fn reach_m(&self) -> f32 {
        self.half_m.length()
    }

    /// True if it may touch the axis-aligned `rect`.
    pub fn touches(&self, rect: &Rect) -> bool {
        rect.distance(self.centre) <= self.reach_m()
    }
}

/// Where-grass-grows at one point, 0..1, before craters: bare on what is built (the shared
/// cover rule), on the beach and in water, thinning on steep ground. `ground_m` and `slope` are
/// the ground's height and steepness (rise over run) there.
pub fn grows_at(plan: &WorldPlan, p: Vec2, ground_m: f32, slope: f32, cfg: &GrassConfig) -> f32 {
    if plan.cover_at(p, &cfg.clearance.into()) == Cover::Bare {
        return 0.0;
    }
    if ground_m < plan.waters.sea_level_m + cfg.beach_m || plan.waters.water_at(p, ground_m).is_some() {
        return 0.0;
    }
    let [lo, hi] = cfg.slope_bare;
    (1.0 - crate::world::terrain::smoothstep(lo, hi, slope)) * plan.biomes.grass_share(p, ground_m, slope)
}

/// The nearest open meadow to the usual start (owner, 2026-10-24: "Screenshots with grass?"): a
/// spot where grass grows thick across a square of `2 * half_m`, searched in rings out from the
/// city start, with the mech there and the pilot 30 m behind it. For takes and screenshots that
/// need grass in view; `None` if the island has no such meadow within `reach_m`.
pub fn meadow_start(plan: &WorldPlan, cfg: &GrassConfig, half_m: f32, reach_m: f32) -> Option<crate::world::Start> {
    use crate::world::Ground;
    let origin = plan.start.mech_xz;
    let thick_at = |p: Vec2| {
        let h = plan.height_at(p.x, p.y);
        let dx = plan.height_at(p.x + 2.0, p.y) - plan.height_at(p.x - 2.0, p.y);
        let dz = plan.height_at(p.x, p.y + 2.0) - plan.height_at(p.x, p.y - 2.0);
        let slope = Vec2::new(dx, dz).length() / 4.0;
        grows_at(plan, p, h, slope, cfg) >= 0.8
    };
    let mut r = 100.0;
    while r <= reach_m {
        let steps = ((std::f32::consts::TAU * r) / 50.0).ceil().max(8.0) as u32;
        for i in 0..steps {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            let c = origin + Vec2::new(a.cos(), a.sin()) * r;
            // Open country, not a city park: the parks are the city's own paved and planted lots,
            // and the terrain's grass ground is what the meadow is for.
            if plan.cities.iter().any(|city| city.rect().distance(c) < half_m * 2.0 + 60.0) {
                continue;
            }
            let all = (-2..=2).all(|j| (-2..=2).all(|k| thick_at(c + Vec2::new(j as f32, k as f32) * (half_m / 2.0))));
            if all {
                let ahead = crate::mech::forward(plan.start.mech_heading_rad);
                return Some(crate::world::Start {
                    mech_xz: c,
                    mech_heading_rad: plan.start.mech_heading_rad,
                    pilot_xz: c - Vec2::new(ahead.x, ahead.z) * 30.0,
                });
            }
        }
        r += 50.0;
    }
    None
}

/// A grassy rise overlooking a city, for a poster still (`--start overlook`): around each city, on
/// rings from 120 m to 620 m beyond its edge (near enough that the skyline fills the background) , on the side away from the sun (so the view from it to the city
/// looks into the sun), the highest point with thick grass in a 40 m patch and no tree within 35 m
/// (grown as the game grows them). The mech stands there facing away from the city, toward a pilot
/// downhill. Returns the start and how high the spot stands over the city's ground, metres.
pub fn overlook_start(plan: &WorldPlan, cfg: &GrassConfig, to_sun: Vec3) -> Option<(crate::world::Start, f32)> {
    use crate::world::Ground;
    let thick_at = |p: Vec2| {
        let h = plan.height_at(p.x, p.y);
        let dx = plan.height_at(p.x + 2.0, p.y) - plan.height_at(p.x - 2.0, p.y);
        let dz = plan.height_at(p.x, p.y + 2.0) - plan.height_at(p.x, p.y - 2.0);
        grows_at(plan, p, h, Vec2::new(dx, dz).length() / 4.0, cfg) >= 0.8
    };
    let away = -Vec2::new(to_sun.x, to_sun.z).normalize_or(Vec2::Y);
    let mut best: Option<(f32, Vec2, Vec2)> = None;
    for city in &plan.cities {
        for ring in 0..11 {
            let r = city.half_size_m + 120.0 + ring as f32 * 50.0;
            for k in -6..=6 {
                let dir = Vec2::from_angle(k as f32 * 0.08).rotate(away);
                let c = city.centre + dir * r;
                if plan.cities.iter().any(|o| o.rect().distance(c) < 60.0) || plan.water_at(c.x, c.y).is_some() {
                    continue;
                }
                let rise = plan.height_at(c.x, c.y) - city.ground_m;
                if best.as_ref().is_some_and(|b| rise <= b.0) {
                    continue;
                }
                if !(-2..=2).all(|j| (-2..=2).all(|i| thick_at(c + Vec2::new(i as f32, j as f32) * 10.0))) {
                    continue;
                }
                // Flat where the mech stands: the 40 m square round it within 3 m of level.
                let hs: Vec<f32> = (-2..=2)
                    .flat_map(|j| (-2..=2).map(move |i| c + Vec2::new(i as f32, j as f32) * 10.0))
                    .map(|q| plan.height_at(q.x, q.y))
                    .collect();
                if hs.iter().cloned().fold(f32::MIN, f32::max) - hs.iter().cloned().fold(f32::MAX, f32::min) > 3.0 {
                    continue;
                }
                let mut woods = plan.forest.clone();
                woods.grow_around(plan, c, 80.0);
                if woods.near(c, 35.0).is_empty() {
                    best = Some((rise, c, city.centre));
                }
            }
        }
    }
    let (rise, c, city) = best?;
    let to_city = (city - c).normalize_or(Vec2::Y);
    // Facing away from the city: `forward(h)` is (-sin h, -cos h) on (x, z).
    let heading = to_city.x.atan2(to_city.y);
    Some((
        crate::world::Start {
            mech_xz: c,
            mech_heading_rad: heading,
            pilot_xz: c - to_city * 34.0,
        },
        rise,
    ))
}

/// True when a crater changed the ground by `dug_delta_m` enough to clear its grass.
pub fn crater_clears(dug_delta_m: f32, cfg: &GrassConfig) -> bool {
    dug_delta_m.abs() > cfg.crater_clear_m
}

/// How fully grass covers the ground at `p`, 0..1: where grass grows (`grows_at`) and no crater
/// cleared it. The ground's colour is tinted toward `ground_tint_colour` by this share, so the
/// ground under the blades is the grass's own colour and no pale soil shows between them; the
/// same rule decides both, so they never disagree (CLAUDE.md 6.1). `slope` is rise over run.
pub fn ground_cover(plan: &WorldPlan, p: Vec2, ground_m: f32, slope: f32, dug_delta_m: f32, cfg: &GrassConfig) -> f32 {
    if crater_clears(dug_delta_m, cfg) {
        return 0.0;
    }
    grows_at(plan, p, ground_m, slope, cfg)
}

/// The slope (rise over run) of ground whose unit normal points up by `normal_y`.
pub fn slope_of_normal(normal_y: f32) -> f32 {
    if normal_y <= 1e-3 {
        return 1e3;
    }
    (1.0 - normal_y * normal_y).max(0.0).sqrt() / normal_y
}

/// How the tiles are laid out in the shader's buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrassLayout {
    /// Lattice samples along a tile's side.
    pub heights_per_side: u32,
    /// Density cells along a tile's side.
    pub density_per_side: u32,
    /// Words one tile takes.
    pub words_per_slot: u32,
}

impl GrassLayout {
    /// The layout for `cfg` on a terrain of `cell_m` cells.
    pub fn new(cfg: &GrassConfig, cell_m: f32) -> Self {
        let heights_per_side = (cfg.tile_m / cell_m).round() as u32 + 1;
        let density_per_side = cfg.density_cells();
        Self {
            heights_per_side,
            density_per_side,
            words_per_slot: heights_per_side * heights_per_side + (density_per_side * density_per_side).div_ceil(4),
        }
    }
}

/// Bakes tile `key` of `grid` on `plan`: the lattice heights and where grass grows (craters are
/// masked separately, on the main thread, by `TileBake::mask_craters`). Pure; runs off the main
/// thread. The ground in the tile is the terrain's lattice read through `cell_height`, exactly as
/// `Heightfield::height_at` reads it.
pub fn bake_tile(plan: &WorldPlan, grid: &TileGrid, key: TileKey, cfg: &GrassConfig) -> TileBake {
    let t = &plan.terrain;
    let cells = (cfg.tile_m / t.cell_m).round() as usize;
    let n = cells + 1;
    let (ix0, iz0) = (key.0 as usize * cells, key.1 as usize * cells);
    let mut heights = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            heights.push(t.sample(ix0 + i, iz0 + j));
        }
    }
    let lo = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut bake = TileBake {
        heights,
        n,
        open: Vec::new(),
        density: Vec::new(),
        ground_m: [lo, hi],
    };
    let corner = grid.rect(key).min;
    let d = cfg.density_cells() as usize;
    let half = cfg.density_cell_m * 0.5;
    let mut open = Vec::with_capacity(d * d);
    for j in 0..d {
        for i in 0..d {
            let local = (Vec2::new(i as f32, j as f32) + 0.5) * cfg.density_cell_m;
            let h = bake.height_at(local, t.cell_m);
            let gx = bake.height_at(local + Vec2::X * half, t.cell_m) - bake.height_at(local - Vec2::X * half, t.cell_m);
            let gz = bake.height_at(local + Vec2::Y * half, t.cell_m) - bake.height_at(local - Vec2::Y * half, t.cell_m);
            let slope = Vec2::new(gx, gz).length() / cfg.density_cell_m;
            let g = grows_at(plan, corner + local, h, slope, cfg);
            open.push((g.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    bake.density.clone_from(&open);
    bake.open = open;
    bake
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Ground, generate};
    use std::sync::OnceLock;

    fn plan() -> &'static WorldPlan {
        static PLAN: OnceLock<WorldPlan> = OnceLock::new();
        PLAN.get_or_init(|| generate(42, &WorldConfig::default()))
    }

    fn grid(cfg: &GrassConfig) -> TileGrid {
        let w = WorldConfig::default();
        TileGrid::new(plan().terrain.origin_m, w.world_size_m, cfg.tile_m)
    }

    fn key_of(g: &TileGrid, p: Vec2) -> TileKey {
        (
            ((p.x - g.origin_m) / g.tile_m).floor() as u32,
            ((p.y - g.origin_m) / g.tile_m).floor() as u32,
        )
    }

    #[test]
    fn seed_42_has_a_grassy_overlook_facing_a_city_and_the_sun() {
        let cfg = GrassConfig::default();
        let to_sun = Vec3::new(0.4, 0.68, -0.6).normalize();
        let (s, rise) = overlook_start(plan(), &cfg, to_sun).expect("a grassy overlook");
        let p = s.mech_xz;
        let h = plan().height_at(p.x, p.y);
        assert!(grows_at(plan(), p, h, 0.0, &cfg) >= 0.8, "on thick grass");
        assert!(plan().cities.iter().all(|c| c.rect().distance(p) >= 60.0), "outside the city");
        let mut woods = plan().forest.clone();
        woods.grow_around(plan(), p, 80.0);
        assert!(woods.near(p, 35.0).is_empty(), "no tree within 35 m");
        let ahead = crate::mech::forward(s.mech_heading_rad);
        assert!(
            Vec2::new(ahead.x, ahead.z).dot((s.pilot_xz - p).normalize()) > 0.99,
            "the mech faces the pilot"
        );
        assert!(rise.is_finite());
    }

    #[test]
    fn seed_42_has_a_meadow_near_the_start_where_grass_grows_thick() {
        let cfg = GrassConfig::default();
        let m = meadow_start(plan(), &cfg, 40.0, 6000.0).expect("a meadow within 6 km");
        let p = m.mech_xz;
        let h = plan().height_at(p.x, p.y);
        assert!(grows_at(plan(), p, h, 0.0, &cfg) >= 0.8);
        assert!((m.pilot_xz - m.mech_xz).length() > 25.0, "the pilot starts a walk away");
        assert!(
            plan().cities.iter().all(|c| c.rect().distance(p) > 100.0),
            "in open country, not a city park"
        );
    }

    #[test]
    fn a_crushed_cell_packs_zero_and_the_rest_packs_as_baked() {
        let cfg = GrassConfig::default();
        let d = cfg.density_cells() as usize;
        let bake = TileBake {
            heights: vec![0.0; 4],
            n: 2,
            open: vec![200; d * d],
            density: vec![200; d * d],
            ground_m: [0.0, 0.0],
        };
        let words = |crush: &[CrushRect]| {
            let mut out = vec![0u32; 4 + (d * d).div_ceil(4)];
            bake.pack_crushed(&mut out, crush, Vec2::ZERO, &cfg);
            out
        };
        let cell = |out: &[u32], i: usize, j: usize| (out[4 + (j * d + i) / 4] >> (8 * ((j * d + i) % 4))) & 0xff;
        let print = CrushRect {
            centre: Vec2::new(10.0, 10.0),
            along: Vec2::new(0.0, 1.0),
            half_m: Vec2::new(1.0, 1.6),
        };
        let crushed = words(&[print]);
        assert_eq!(cell(&crushed, 10, 10), 0, "under the sole");
        assert_eq!(cell(&crushed, 9, 11), 0, "under the sole");
        assert_eq!(cell(&crushed, 13, 10), 200, "3 m away");
        let mut plain = vec![0u32; 4 + (d * d).div_ceil(4)];
        bake.pack(&mut plain);
        assert_eq!(words(&[]), plain, "no rectangle: packed as baked");
        assert_eq!(bake.density[10 * d + 10], 200, "the bake is kept");
    }

    #[test]
    fn the_shipped_defaults_are_valid_against_the_shipped_world() {
        let cfg = GrassConfig::default();
        cfg.validate().unwrap();
        cfg.validate_with(&WorldConfig::default()).unwrap();
    }

    #[test]
    fn a_level_whose_stride_does_not_divide_the_next_is_refused() {
        let mut cfg = GrassConfig::default();
        cfg.lods[2].stride = 3;
        let e = cfg.validate().unwrap_err();
        assert!(e.contains("lods[2].stride"), "{e}");
    }

    #[test]
    fn grass_beyond_the_grounds_full_detail_ring_is_refused() {
        let cfg = GrassConfig {
            radius_m: 900.0,
            lods: vec![GrassLod {
                until_m: 900.0,
                stride: 1,
                segments: 2,
                width_scale: 1.0,
            }],
            ..Default::default()
        };
        cfg.validate().unwrap();
        let e = cfg.validate_with(&WorldConfig::default()).unwrap_err();
        assert!(e.contains("full-detail"), "{e}");
    }

    #[test]
    fn the_blade_meshes_have_the_counts_in_the_design() {
        // design.md section 3: 25,600 / 6,400 / 1,600 blades a 32 m tile; 11 / 7 / 5 vertices and
        // 9 / 5 / 3 triangles a blade.
        let cfg = GrassConfig::default();
        let want = [(25_600, 281_600, 230_400), (6_400, 44_800, 32_000), (1_600, 8_000, 4_800)];
        for (lod, (blades, verts, tris)) in want.into_iter().enumerate() {
            let m = BladeMesh::new(&cfg, lod);
            assert_eq!(cfg.blades_per_tile(lod), blades, "level {lod} blades");
            assert_eq!(m.vertices.len(), verts, "level {lod} vertices");
            assert_eq!(m.indices.len(), tris * 3, "level {lod} triangles");
            assert!(m.indices.iter().all(|i| (*i as usize) < m.vertices.len()));
        }
        assert_eq!(cfg.blades_per_m2(0), 25.0);
        assert_eq!(cfg.blades_per_m2(2), 1.5625);
    }

    #[test]
    fn a_blade_vertex_carries_only_its_blade_its_place_and_its_level() {
        let cfg = GrassConfig::default();
        let m = BladeMesh::new(&cfg, 1);
        assert_eq!(m.vertices[0], [0.0, 0.0, 1.0]);
        assert_eq!(m.vertices[7], [1.0, 0.0, 1.0], "blade 1 starts after blade 0's 7 vertices");
        let last = *m.vertices.last().unwrap();
        assert_eq!(last, [6399.0, 6.0, 1.0], "the last blade's tip");
    }

    #[test]
    fn near_tiles_draw_finest_and_a_tile_on_a_boundary_keeps_its_level() {
        let cfg = GrassConfig::default();
        assert_eq!(cfg.lod_for(0.0, None), 0);
        assert_eq!(cfg.lod_for(60.0, None), 1);
        assert_eq!(cfg.lod_for(250.0, None), 2);
        assert_eq!(cfg.lod_for(999.0, None), 2, "past the last boundary: the last level");
        // 3 m past the 25 m boundary, inside the 6 m hysteresis: it stays where it was.
        assert_eq!(cfg.lod_for(28.0, Some(0)), 0);
        assert_eq!(cfg.lod_for(22.0, Some(1)), 1);
        assert_eq!(cfg.lod_for(32.0, Some(0)), 1, "well past: it moves");
    }

    #[test]
    fn tiles_come_nearest_first_and_stop_at_the_radius() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let eye = Vec3::new(10.0, 1.7, 10.0);
        let tiles = g.within(eye, 0.0, cfg.radius_m);
        assert!(tiles.windows(2).all(|w| w[0].1 <= w[1].1));
        assert_eq!(tiles[0].1, 1.7, "the tile under the eye: only the eye's height");
        assert!(tiles.iter().all(|(_, d)| *d <= cfg.radius_m));
        // About (pi r^2 + 4 r t + t^2) / t^2 tiles: 315 for 300 m and 32 m (design section 3).
        assert!((300..=330).contains(&tiles.len()), "{}", tiles.len());
        let high = g.within(Vec3::new(10.0, 400.0, 10.0), 0.0, cfg.radius_m);
        assert!(high.is_empty(), "from 400 m up no grass is near enough to draw");
    }

    #[test]
    fn no_tiles_past_the_worlds_edge() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let edge = g.origin_m + g.per_side as f32 * g.tile_m;
        let tiles = g.within(Vec3::new(edge + 100.0, 0.0, 0.0), 0.0, cfg.radius_m);
        assert!(tiles.iter().all(|((x, z), _)| *x < g.per_side && *z < g.per_side));
    }

    #[test]
    fn a_tiles_heights_are_the_grounds_heights() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let p = plan();
        let key = key_of(&g, p.start.pilot_xz + Vec2::new(900.0, 400.0));
        let bake = bake_tile(p, &g, key, &cfg);
        let corner = g.rect(key).min;
        for k in 0..50 {
            let local = Vec2::new((k * 7 % 32) as f32 + 0.37, (k * 13 % 32) as f32 + 0.61);
            let w = corner + local;
            let want = p.height_at(w.x, w.y);
            let got = bake.height_at(local, p.terrain.cell_m);
            assert!((want - got).abs() < 1e-3, "at {w}: ground {want}, tile {got}");
        }
    }

    /// The fraction of a tile's density cells with grass.
    fn grassy(bake: &TileBake) -> f32 {
        bake.density.iter().filter(|d| **d > 0).count() as f32 / bake.density.len() as f32
    }

    #[test]
    fn no_grass_grows_in_a_city() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let p = plan();
        let city = &p.cities[0];
        let bake = bake_tile(p, &g, key_of(&g, city.centre), &cfg);
        // The city's centre is downtown: streets and buildings, no park this big.
        assert!(grassy(&bake) < 0.3, "{}", grassy(&bake));
        let at = |q: Vec2| grows_at(p, q, city.ground_m, 0.0, &cfg);
        for s in city.streets.iter().take(20) {
            assert_eq!(at(s.rect.centre()), 0.0, "a street");
        }
        for b in p.buildings.iter().take(20) {
            assert_eq!(at(b.footprint.centre()), 0.0, "a building");
        }
    }

    #[test]
    fn no_grass_grows_on_a_highway() {
        let cfg = GrassConfig::default();
        let p = plan();
        let h = &p.highways[0];
        let mid = h.point_at(0.5);
        let ground = p.height_at(mid.x, mid.y);
        assert_eq!(grows_at(p, mid, ground, 0.0, &cfg), 0.0);
        // Well off the road (and off any bridge or city), on dry level ground above the beach: grass.
        let across = h.dir_at(0.5).perp() * (h.width_m * 0.5 + 40.0);
        let off = mid + across;
        let g = p.height_at(off.x, off.y);
        let open = p.cover_at(off, &cfg.clearance.into()) != Cover::Bare
            && p.waters.water_at(off, g).is_none()
            && g > p.waters.sea_level_m + cfg.beach_m;
        if open {
            // As much as the biomes there grow (`Biomes::grass_share`).
            let share = p.biomes.grass_share(off, g, 0.0);
            assert!((grows_at(p, off, g, 0.0, &cfg) - share).abs() < 0.01);
        }
    }

    #[test]
    fn no_grass_grows_in_water() {
        let cfg = GrassConfig::default();
        let p = plan();
        let r = &p.waters.rivers[0];
        let mid = r.path[r.path.len() / 2];
        let ground = p.height_at(mid.x, mid.y);
        assert!(p.waters.water_at(mid, ground).is_some(), "the river's middle is wet");
        assert_eq!(grows_at(p, mid, ground, 0.0, &cfg), 0.0);
        // The open sea: ground under sea level.
        assert_eq!(grows_at(p, Vec2::new(19_000.0, 19_000.0), -30.0, 0.0, &cfg), 0.0);
    }

    #[test]
    fn steep_ground_thins_the_grass() {
        let cfg = GrassConfig::default();
        let p = plan();
        let (mut found, mut q) = (false, Vec2::ZERO);
        // Some open dry grassland above the beach.
        for k in 0..400 {
            let c = Vec2::new(-6000.0 + (k % 20) as f32 * 600.0, -6000.0 + (k / 20) as f32 * 600.0);
            let g = p.height_at(c.x, c.y);
            if p.cover_at(c, &cfg.clearance.into()) == Cover::Open
                && p.waters.water_at(c, g).is_none()
                && g > p.waters.sea_level_m + cfg.beach_m + 1.0
                && p.biomes.grass_share(c, g, 0.0) > 0.5
            {
                found = true;
                q = c;
                break;
            }
        }
        assert!(found, "seed 42 has open dry ground");
        let g = p.height_at(q.x, q.y);
        let share = p.biomes.grass_share(q, g, 0.0);
        assert!(
            (grows_at(p, q, g, 0.0, &cfg) - share).abs() < 1e-6,
            "level: as much as the biomes grow"
        );
        assert!(grows_at(p, q, g, 0.85, &cfg) < 0.9 * share, "steep");
        assert_eq!(grows_at(p, q, g, 2.0, &cfg), 0.0, "a cliff");
    }

    #[test]
    fn a_crater_clears_the_grass_it_dug() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let p = plan();
        let w = WorldConfig::default();
        // Open ground near the start, away from the city.
        let c = p.cities[0].centre + Vec2::new(p.cities[0].half_size_m + 200.0, 0.0);
        let key = key_of(&g, c);
        let corner = g.rect(key).min;
        let mut bake = bake_tile(p, &g, key, &cfg);
        bake.open.fill(255);
        let mut dug = Deformation::new(w.world_size_m);
        let centre = corner + Vec2::splat(16.0);
        let blast = crate::world::deform::Blast { centre, energy_mj: 5.0 };
        dug.dig(p, blast, &w.craters);
        bake.mask_craters(&dug, corner, &cfg);
        let d = cfg.density_cells() as usize;
        let mid = (d / 2) * d + d / 2;
        assert_eq!(bake.density[mid], 0, "the bowl is bare");
        assert_eq!(bake.density[0], 255, "a corner 22 m off is untouched");
        assert_eq!(bake.open[mid], 255, "the undug bake is kept for the next crater");
    }

    #[test]
    fn the_ground_is_tinted_exactly_where_grass_grows_and_not_in_a_crater() {
        let cfg = GrassConfig::default();
        let p = plan();
        let h = &p.highways[0];
        let road = h.point_at(0.5);
        let g = p.height_at(road.x, road.y);
        assert_eq!(ground_cover(p, road, g, 0.0, 0.0, &cfg), 0.0, "no tint on a road");
        let m = meadow_start(p, &cfg, 40.0, 6000.0).expect("a meadow").mech_xz;
        let g = p.height_at(m.x, m.y);
        let grows = grows_at(p, m, g, 0.1, &cfg);
        assert!(grows > 0.8);
        assert_eq!(
            ground_cover(p, m, g, 0.1, 0.0, &cfg),
            grows,
            "the one rule: tint is where grass grows"
        );
        assert_eq!(ground_cover(p, m, g, 0.1, -0.3, &cfg), 0.0, "a crater's bowl is bare");
        assert_eq!(ground_cover(p, m, g, 0.1, 0.01, &cfg), grows, "a scratch is not a crater");
    }

    #[test]
    fn a_normals_slope_is_rise_over_run() {
        assert_eq!(slope_of_normal(1.0), 0.0);
        let n45 = std::f32::consts::FRAC_1_SQRT_2;
        assert!((slope_of_normal(n45) - 1.0).abs() < 1e-5);
        assert!(slope_of_normal(0.0) > 100.0, "a wall");
    }

    #[test]
    fn tall_clumps_stand_about_two_metres() {
        let cfg = GrassConfig::default();
        assert!((1.9..=2.2).contains(&cfg.tallest_m()), "{}", cfg.tallest_m());
        assert!(cfg.height_m[0] >= 0.5, "meadow grass reaches a mech's ankle, not a person's");
    }

    #[test]
    fn a_tile_packs_into_its_slot_heights_then_four_cells_a_word() {
        let cfg = GrassConfig::default();
        let layout = GrassLayout::new(&cfg, 8.0);
        assert_eq!(layout.heights_per_side, 5);
        assert_eq!(layout.density_per_side, 32);
        assert_eq!(layout.words_per_slot, 25 + 256);
        let mut bake = TileBake {
            heights: (0..25).map(|i| i as f32).collect(),
            n: 5,
            open: vec![0; 1024],
            density: vec![0; 1024],
            ground_m: [0.0, 24.0],
        };
        bake.density[0] = 1;
        bake.density[1] = 2;
        bake.density[5] = 255;
        let mut words = vec![7u32; layout.words_per_slot as usize];
        bake.pack(&mut words);
        assert_eq!(f32::from_bits(words[3]), 3.0);
        assert_eq!(words[25], 1 | (2 << 8));
        assert_eq!(words[26], 255 << 8);
        assert_eq!(words[27], 0, "a stale word is cleared");
    }

    #[test]
    fn the_buffer_holds_every_tile_in_reach() {
        let cfg = GrassConfig::default();
        let g = grid(&cfg);
        let reach = cfg.radius_m + cfg.release_m;
        let most = g.within(Vec3::new(5.0, 0.0, 5.0), 0.0, reach).len() as u32;
        assert!(g.capacity(reach) >= most, "{} < {most}", g.capacity(reach));
    }
}
