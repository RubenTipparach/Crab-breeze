//! Lit rooms at night (openspec change `night-voxel-light`; owner, 2026-10-29: "I dont want the glass itself
//! to glow, I want to see the room lits", "we just need to find vertices for each window cell area, and just
//! brighten them by distance from a point", "25% is the ticket. I expect this transition to happen gradually as
//! it gets dark", "Do the same for houses"). The rules the building meshes are baked by and the shaders read:
//! how bright a room's light is at a vertex, how a room and its light are packed into the vertex's second UV,
//! and how far through dusk the glass has turned to its night look. Pure, so the bake and its tests run with
//! no renderer; `cb-app` writes the meshes and the shaders (`building_state.wesl`) unpack them.

use crate::data::within;
use crate::world::terrain::smoothstep;
use glam::Vec3;
use serde::Deserialize;

/// The most room cells one mesh can number (the packed index stays exact in an `f32`).
pub const MAX_CELLS: u32 = 4096;
/// The most room light a vertex can carry (the packed fraction is `light / LIGHT_SCALE`).
pub const LIGHT_SCALE: f32 = 4.0;
/// The room index of a room that is lit every night (the airport's and seaport's buildings and ships, design section
/// 2d): no hash and no hourly share; it comes on with the glass through dusk and stays on until dawn.
pub const ALWAYS_LIT: u32 = MAX_CELLS - 1;
/// The room index of a room lit day and night at `cab_gain` (a control tower's cab; owner, 2026-10-29: "top control
/// towers should always be lit").
pub const ALWAYS_ON: u32 = MAX_CELLS - 2;

/// `day_night.ron`'s `rooms`: the lit rooms and the glass at night.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RoomLights {
    /// A room light's falloff distance, metres: the light is `1 / (1 + (d / r0)^2)`.
    pub r0_m: f32,
    /// The share of a room light a surface facing away from it still gets (the room's walls and
    /// ceiling pass the light round), 0..1.
    pub facing_floor: f32,
    /// A lit room's brightness at its light, cd/m2 (times the exposure on screen).
    pub cd_m2: f32,
    /// A room light's colour, linear (warm incandescent and office tubes mixed).
    pub colour: [f32; 3],
    /// The light point's distance under the ceiling, metres.
    pub below_ceiling_m: f32,
    /// A house's storey height, metres (a kit has no floor plan).
    pub house_storey_m: f32,
    /// A house's room cells, metres square.
    pub house_cell_m: f32,
    /// The glass's alpha at night (owner: "25% is the ticket"): what is behind it shows at `1 - alpha`. By day
    /// it is the glass's own (`sky_light.ron`'s `glass.alpha`, 0.92).
    pub glass_alpha_night: f32,
    /// Sun elevations, degrees, over which the glass turns from its day look to its night look (fully
    /// night at the first): "gradually as it gets dark".
    pub glass_night_deg: [f32; 2],
    /// An always-on room (`ALWAYS_ON`, a tower's cab) is this many times as bright as a lit room.
    pub cab_gain: f32,
    /// A control tower's cab is its rooms whose light is within this of its highest inside surface, metres.
    pub cab_depth_m: f32,
}

impl Default for RoomLights {
    fn default() -> Self {
        Self {
            r0_m: 2.6,
            facing_floor: 0.45,
            cd_m2: 3.0,
            colour: [1.0, 0.82, 0.6],
            below_ceiling_m: 0.3,
            house_storey_m: 3.0,
            house_cell_m: 4.0,
            glass_alpha_night: 0.75,
            glass_night_deg: [-8.0, 4.0],
            cab_gain: 2.0,
            cab_depth_m: 5.0,
        }
    }
}

impl RoomLights {
    /// Checks the ranges.
    pub fn check(&self) -> Result<(), String> {
        within("rooms.r0_m", self.r0_m, 0.1, 50.0)?;
        within("rooms.facing_floor", self.facing_floor, 0.0, 1.0)?;
        within("rooms.cd_m2", self.cd_m2, 0.0, 1.0e5)?;
        for c in self.colour {
            within("rooms.colour", c, 0.0, 1.0)?;
        }
        within("rooms.below_ceiling_m", self.below_ceiling_m, 0.0, 2.0)?;
        within("rooms.house_storey_m", self.house_storey_m, 2.0, 10.0)?;
        within("rooms.house_cell_m", self.house_cell_m, 1.0, 20.0)?;
        within("rooms.glass_alpha_night", self.glass_alpha_night, 0.0, 1.0)?;
        within("rooms.cab_gain", self.cab_gain, 0.0, LIGHT_SCALE)?;
        within("rooms.cab_depth_m", self.cab_depth_m, 0.0, 50.0)?;
        within("rooms.glass_night_deg", self.glass_night_deg[0], -90.0, 90.0)?;
        within("rooms.glass_night_deg", self.glass_night_deg[1], -90.0, 90.0)?;
        if self.glass_night_deg[0] < self.glass_night_deg[1] {
            Ok(())
        } else {
            Err(format!(
                "`rooms.glass_night_deg` must run from low to high, got {:?}",
                self.glass_night_deg
            ))
        }
    }

    /// The light a surface at `p` facing `n` gets from a room light at `point`, 0..1 (1 at the light):
    /// `1 / (1 + (d / r0)^2)`, scaled from `facing_floor` (facing away) to 1 (facing it).
    pub fn light_at(&self, p: Vec3, n: Vec3, point: Vec3) -> f32 {
        let to = point - p;
        let d = to.length();
        if !d.is_finite() {
            return 0.0;
        }
        let facing = if d > 1e-4 { (n.dot(to) / d).max(0.0) } else { 1.0 };
        let t = d / self.r0_m;
        (self.facing_floor + (1.0 - self.facing_floor) * facing) / (1.0 + t * t)
    }

    /// How far the glass has turned to its night look at a sun elevation, 0 by day to 1 at night.
    pub fn glass_night_share(&self, sun_elevation_deg: f32) -> f32 {
        let [night, day] = self.glass_night_deg;
        1.0 - smoothstep(night, day, sun_elevation_deg)
    }

    /// The alpha of glass whose daytime alpha is `day_alpha`, at a night share (`glass_night_share`). Glass
    /// already clearer than the night's keeps its own.
    pub fn glass_alpha(&self, day_alpha: f32, night_share: f32) -> f32 {
        let s = night_share.clamp(0.0, 1.0);
        let night = self.glass_alpha_night.min(day_alpha);
        day_alpha + (night - day_alpha) * s
    }

    /// The light point of a house's room cell holding `local` (a point in the kit's frame, its floor at
    /// `floor_y`), and the cell's storey: house rooms are `house_cell_m` squares of each
    /// `house_storey_m` storey, lit from under their ceiling's middle.
    pub fn house_cell(&self, local: Vec3, floor_y: f32) -> (Vec3, u32) {
        let storey = ((local.y - floor_y) / self.house_storey_m).floor().max(0.0);
        let c = self.house_cell_m;
        let mid = |v: f32| ((v / c).floor() + 0.5) * c;
        let y = floor_y + (storey + 1.0) * self.house_storey_m - self.below_ceiling_m;
        (Vec3::new(mid(local.x), y, mid(local.z)), storey as u32)
    }
}

/// The second UV's `y` for a vertex in room cell `cell` (1..`MAX_CELLS`, 0 for none) with room light
/// `light`: the cell's index in the whole part, the light in the fraction (`LIGHT_SCALE` at most).
pub fn pack_room(cell: u32, light: f32) -> f32 {
    if cell == 0 || cell >= MAX_CELLS || !light.is_finite() || light <= 0.0 {
        return 0.0;
    }
    let frac = (light / LIGHT_SCALE).clamp(0.0, 0.999);
    cell as f32 + frac
}

/// The room cell and light packed by `pack_room` (what `building_state.wesl` decodes).
pub fn unpack_room(y: f32) -> (u32, f32) {
    if !y.is_finite() || y < 1.0 {
        return (0, 0.0);
    }
    let cell = y.floor();
    (cell as u32, (y - cell) * LIGHT_SCALE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_light_is_brightest_at_its_point_and_falls_with_distance() {
        let r = RoomLights::default();
        let p = Vec3::new(0.0, 3.0, 0.0);
        let near = r.light_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y, p);
        let far = r.light_at(Vec3::new(4.0, 0.0, 0.0), Vec3::Y, p);
        assert!(near > far && far > 0.0, "{near} then {far}");
        assert!((r.light_at(p, Vec3::Y, p) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_surface_facing_away_gets_the_floor_share() {
        let r = RoomLights::default();
        let p = Vec3::new(0.0, 3.0, 0.0);
        let up = r.light_at(Vec3::ZERO, Vec3::Y, p);
        let down = r.light_at(Vec3::ZERO, -Vec3::Y, p);
        assert!((down / up - r.facing_floor).abs() < 1e-5);
    }

    #[test]
    fn a_packed_room_comes_back_whole() {
        for (cell, light) in [(1, 0.01), (7, 1.0), (4095, 3.5), (300, 0.25)] {
            let (c, l) = unpack_room(pack_room(cell, light));
            assert_eq!(c, cell);
            assert!((l - light).abs() < 2e-3, "{cell}: {light} came back {l}");
        }
    }

    #[test]
    fn no_room_or_no_light_packs_to_zero() {
        assert_eq!(pack_room(0, 1.0), 0.0);
        assert_eq!(pack_room(MAX_CELLS, 1.0), 0.0);
        assert_eq!(pack_room(5, 0.0), 0.0);
        assert_eq!(pack_room(5, f32::NAN), 0.0);
        assert_eq!(unpack_room(0.0), (0, 0.0));
    }

    #[test]
    fn too_much_light_is_held_under_the_scale() {
        let (c, l) = unpack_room(pack_room(9, 100.0));
        assert_eq!(c, 9);
        assert!(l < LIGHT_SCALE && l > LIGHT_SCALE * 0.99);
    }

    #[test]
    fn the_glass_turns_to_night_gradually_as_it_gets_dark() {
        let r = RoomLights::default();
        assert_eq!(r.glass_night_share(30.0), 0.0, "day");
        assert_eq!(r.glass_night_share(-30.0), 1.0, "night");
        let dusk = r.glass_night_share(-2.0);
        assert!(dusk > 0.0 && dusk < 1.0, "between at dusk: {dusk}");
        assert!(r.glass_night_share(-5.0) > dusk, "darker, further");
        assert!((r.glass_alpha(0.92, 0.0) - 0.92).abs() < 1e-6 && (r.glass_alpha(0.92, 1.0) - 0.75).abs() < 1e-6);
        assert!((r.glass_alpha(0.5, 1.0) - 0.5).abs() < 1e-6, "clearer glass keeps its own");
    }

    #[test]
    fn a_house_cell_is_lit_from_under_its_ceiling_middle() {
        let r = RoomLights::default();
        let (p, storey) = r.house_cell(Vec3::new(1.0, 4.0, 5.0), 0.5);
        assert_eq!(storey, 1);
        assert!((p.y - (0.5 + 2.0 * 3.0 - 0.3)).abs() < 1e-5);
        assert!((p.x - 2.0).abs() < 1e-5 && (p.z - 6.0).abs() < 1e-5);
        assert_eq!(
            r.house_cell(Vec3::new(0.0, -1.0, 0.0), 0.5).1,
            0,
            "below the floor is the ground storey"
        );
    }

    #[test]
    fn the_defaults_check() {
        RoomLights::default().check().unwrap();
        let bad = RoomLights {
            glass_night_deg: [4.0, -8.0],
            ..RoomLights::default()
        };
        assert!(bad.check().is_err());
    }
}
