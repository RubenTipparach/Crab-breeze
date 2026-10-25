//! Walking controls (openspec change `walk-controls`): in walking mode WASD is direct, as on foot
//! (owner, 2026-10-25: "I dont like the legs being tank controls, tank controls on skate makes sense,
//! but when in walking mode, wasd should be standard walking"). W walks toward where the pilot looks,
//! S backs away, A and D step left and right; the legs turn toward the way the mech walks while the
//! torso keeps facing the view; with no key held the mech stops. Skating and the jets keep the lever
//! and the leg turn. Pure: the seat's keys and the view in, the throttle and the leg turn out.

use glam::Vec2;
use std::f32::consts::PI;

/// The leg turn, per radian the legs are off the way to walk (full turn past about 30 degrees).
const TURN_GAIN: f32 = 2.0;

/// What the walking keys ask of the legs this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walk {
    /// The throttle to hold (`throttle_min`..1): 0 with no key held.
    pub throttle: f32,
    /// The leg turn, -1..1, left positive.
    pub turn_axis: f32,
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}

/// The walking controls for keys `keys` (x right, y forward, each -1..1) with the view at world yaw
/// `view_yaw_rad` and the legs at `heading_rad` (yaw 0 faces -z, positive turns left).
///
/// Walking forward or to a side, the legs turn toward the way to walk and the throttle is the keys'
/// strength, eased off while the legs are still turning so the mech does not stride off sideways.
/// Straight back, the legs face the view and back up (`throttle_min`). No keys: stop, no turn.
pub fn walk(keys: Vec2, view_yaw_rad: f32, heading_rad: f32, throttle_min: f32) -> Walk {
    let keys = keys.clamp_length_max(1.0);
    if keys.length() < 0.1 {
        return Walk {
            throttle: 0.0,
            turn_axis: 0.0,
        };
    }
    let backing = keys.y < 0.0 && keys.x.abs() < 0.3;
    // A key to the right turns the way to walk to the right of the view: a smaller yaw.
    let target = if backing {
        view_yaw_rad
    } else {
        view_yaw_rad + (-keys.x).atan2(keys.y.max(0.0))
    };
    let off = wrap(target - heading_rad);
    let turn_axis = (off * TURN_GAIN).clamp(-1.0, 1.0);
    let strength = keys.length();
    let throttle = if backing {
        throttle_min * strength
    } else {
        strength * off.cos().max(0.0)
    };
    Walk { throttle, turn_axis }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: f32 = -0.4;

    #[test]
    fn no_keys_stops_the_mech() {
        assert_eq!(
            walk(Vec2::ZERO, 1.0, 0.0, MIN),
            Walk {
                throttle: 0.0,
                turn_axis: 0.0
            }
        );
    }

    #[test]
    fn w_walks_toward_the_view() {
        let w = walk(Vec2::Y, 0.0, 0.0, MIN);
        assert!((w.throttle - 1.0).abs() < 1e-5);
        assert!(w.turn_axis.abs() < 1e-5);
        // The view half a turn to the left: the legs turn left toward it.
        let w = walk(Vec2::Y, 0.5, 0.0, MIN);
        assert!(w.turn_axis > 0.5, "{w:?}");
    }

    #[test]
    fn d_steps_to_the_right_of_the_view() {
        // Legs facing the view, D held: the legs turn right (negative) toward the way to walk, and
        // hold back while they are still well off it.
        let w = walk(Vec2::X, 0.0, 0.0, MIN);
        assert!(w.turn_axis < -0.9, "{w:?}");
        assert!(w.throttle < 0.05, "it does not stride off sideways: {w:?}");
        // Once the legs face right, it walks.
        let w = walk(Vec2::X, 0.0, -PI / 2.0, MIN);
        assert!(w.throttle > 0.95 && w.turn_axis.abs() < 1e-4, "{w:?}");
    }

    #[test]
    fn s_backs_away_facing_the_view() {
        let w = walk(-Vec2::Y, 0.0, 0.0, MIN);
        assert!((w.throttle - MIN).abs() < 1e-5);
        assert!(w.turn_axis.abs() < 1e-5);
    }
}
