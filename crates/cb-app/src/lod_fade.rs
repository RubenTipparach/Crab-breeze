//! The one rule for how far a thing is drawn and how it fades (openspec change `lod-crossfade`, after Pale Blue
//! Dot's `distance-lod-fade`): a Bevy `VisibilityRange` whose margins make it dither in and out through a
//! screen-door mask instead of popping. Every system that limits how far something is drawn builds its range here.

use bevy::camera::visibility::VisibilityRange;

/// Drawn from the eye out to `far_m`, dithering out over the last `fade_share` of it.
pub fn out_to(far_m: f32, fade_share: f32) -> VisibilityRange {
    let fade = (far_m * fade_share.clamp(0.0, 1.0)).max(0.0);
    VisibilityRange {
        start_margin: 0.0..0.0,
        end_margin: (far_m - fade).max(0.0)..far_m.max(0.0),
        use_aabb: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_dithers_out_over_the_last_share_of_its_distance() {
        let r = out_to(200.0, 0.15);
        assert_eq!(r.start_margin, 0.0..0.0);
        assert_eq!(r.end_margin, 170.0..200.0);
    }

    #[test]
    fn a_zero_share_is_a_hard_edge_and_a_range_is_never_inverted() {
        assert_eq!(out_to(50.0, 0.0).end_margin, 50.0..50.0);
        let r = out_to(10.0, 5.0);
        assert!(r.end_margin.start <= r.end_margin.end);
    }
}
