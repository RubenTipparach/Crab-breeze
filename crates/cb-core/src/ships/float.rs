//! Floating (openspec change `harbours`, design section 4): a hull rides the water's surface and heaves, pitches
//! and rolls on the swell the water draws (`water_look.ron`, read through `WaterLook::swell_height`, the one
//! reading of the swell). Sampled at the bow, stern, port and starboard, so a long hull averages short waves away.

use crate::water_look::WaterLook;
use glam::Vec2;

/// How a hull sits on the swell this moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ride {
    /// Up from the still surface, metres.
    pub heave_m: f32,
    /// Bow up, radians (a turn about the hull's own across axis).
    pub pitch_rad: f32,
    /// Starboard up, radians (a turn about the hull's own fore and aft axis).
    pub roll_rad: f32,
}

/// The ride of a hull `length_m` by `beam_m` at `pos` facing `heading_rad` (see `ships::ahead`) at `seconds`.
pub fn ride(look: &WaterLook, pos: Vec2, heading_rad: f32, length_m: f32, beam_m: f32, seconds: f32) -> Ride {
    let fwd = super::ahead(heading_rad);
    let right = fwd.perp();
    let h = |p: Vec2| look.swell_height(p, seconds);
    let (bow, stern) = (h(pos + fwd * length_m * 0.5), h(pos - fwd * length_m * 0.5));
    let (star, port) = (h(pos + right * beam_m * 0.5), h(pos - right * beam_m * 0.5));
    Ride {
        heave_m: (bow + stern + star + port + h(pos)) / 5.0,
        pitch_rad: ((bow - stern) / length_m.max(0.1)).atan(),
        roll_rad: ((star - port) / beam_m.max(0.1)).atan(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> WaterLook {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/water_look.ron");
        crate::data::load_ron(&p).unwrap()
    }

    /// The worst a hull pitches and heaves over a minute of swell.
    fn worst(length: f32, beam: f32) -> (f32, f32) {
        let l = look();
        (0..600).fold((0.0f32, 0.0f32), |(p, h), i| {
            let r = ride(&l, Vec2::new(37.0, -12.0), 0.4, length, beam, i as f32 * 0.1);
            (p.max(r.pitch_rad.abs()), h.max(r.heave_m.abs()))
        })
    }

    #[test]
    fn a_long_hull_rides_steadier_than_a_rowing_boat() {
        let (row_pitch, row_heave) = worst(4.2, 1.4);
        let (ship_pitch, ship_heave) = worst(120.0, 18.0);
        assert!(ship_pitch < row_pitch * 0.5, "pitch {ship_pitch} against {row_pitch}");
        assert!(ship_heave < row_heave, "heave {ship_heave} against {row_heave}");
        assert!(row_heave > 0.05, "a rowing boat bobs: {row_heave}");
    }

    #[test]
    fn the_ride_never_exceeds_the_swell() {
        let l = look();
        let most: f32 = l.swell_height_m.iter().sum();
        for i in 0..200 {
            let r = ride(&l, Vec2::new(i as f32 * 13.0, 5.0), 1.0, 8.0, 2.6, i as f32 * 0.37);
            assert!(r.heave_m.abs() <= most + 1e-4, "{r:?}");
        }
    }

    #[test]
    fn the_swell_is_the_shaders_deep_water_waves() {
        // One train alone: its height is h sin(k x - sqrt(g k) t), the phase the shader's slope differentiates.
        let mut l = look();
        l.swell_height_m = [1.0, 0.0, 0.0];
        l.swell_heading_deg = [0.0, 0.0, 0.0];
        l.swell_wavelength_m = [60.0, 10.0, 10.0];
        let k = std::f32::consts::TAU / 60.0;
        let (x, t) = (7.0f32, 3.0f32);
        let want = (k * x - (9.81 * k).sqrt() * t).sin();
        assert!((l.swell_height(Vec2::new(x, 99.0), t) - want).abs() < 1e-5);
    }
}
