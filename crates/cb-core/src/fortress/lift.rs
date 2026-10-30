//! The lift rule (design sections 2 and 2b): each side carries half the weight; a lift thruster carries at most
//! `lift_max_share` of it and a lit hull thruster `hull_max_share`. A side's lift is the least of what its live
//! thrusters can carry and its half; the fortress holds while both sides carry their half. The cockpit's lift
//! readout is this function's result (CLAUDE.md 6.1).

use super::{FortressData, PartState};

/// Each side's lift as the rule has it. Side 0 is port (+X), side 1 starboard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lift {
    /// Port's lift, as a share of the weight (at most 0.5).
    pub port: f32,
    /// Starboard's lift, as a share of the weight.
    pub stbd: f32,
    /// Both sides carry their half.
    pub holds: bool,
    /// Live lift thrusters by side.
    pub outer: [u32; 2],
    /// Live hull thrusters by side.
    pub hull: [u32; 2],
    /// The hull thrusters are lit.
    pub lit: bool,
    /// Each side's live lift thrusters' throttle, 0..1 (1 is white hot).
    pub throttle_outer: [f32; 2],
    /// Each side's hull thrusters' throttle, 0..1 (0 while unlit).
    pub throttle_hull: [f32; 2],
}

/// The lift with the parts as they stand; `lit` counts the hull thrusters.
pub fn lift(d: &FortressData, parts: &[PartState], lit: bool) -> Lift {
    let mut outer = [0u32; 2];
    let mut hull = [0u32; 2];
    let mut hull_lost = [0u32; 2];
    for (r, p) in d.shape.parts.iter().zip(parts) {
        if r.side == 0 {
            continue;
        }
        let s = usize::from(r.side < 0);
        if !p.alive {
            hull_lost[s] += u32::from(r.kind == "hull");
            continue;
        }
        match r.kind.as_str() {
            "lift" => outer[s] += 1,
            "hull" => hull[s] += 1,
            _ => {}
        }
    }
    let (tmax, hmax) = (d.cfg.lift_max_share, d.cfg.hull_max_share);
    let mut share = [0.0f32; 2];
    let mut throttle_outer = [0.0f32; 2];
    let mut throttle_hull = [0.0f32; 2];
    let mut holds = true;
    for s in 0..2 {
        let mut cap = outer[s] as f32 * tmax + if lit { hull[s] as f32 * hmax } else { 0.0 };
        // Lit, the hull thrusters run past their rating to hold the side until `hull_kills_to_fall` of them are lost.
        if lit && hull[s] > 0 && hull_lost[s] < d.cfg.hull_kills_to_fall {
            cap = cap.max(0.5);
        }
        share[s] = cap.min(0.5);
        let throttle = if cap > 0.0 { (0.5 / cap).min(1.0) } else { 0.0 };
        throttle_outer[s] = if outer[s] > 0 { throttle } else { 0.0 };
        throttle_hull[s] = if lit && hull[s] > 0 { throttle } else { 0.0 };
        holds &= cap >= 0.5 - 1e-5;
    }
    Lift {
        port: share[0],
        stbd: share[1],
        holds,
        outer,
        hull,
        lit,
        throttle_outer,
        throttle_hull,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fortress, kill};
    use super::*;

    #[test]
    fn a_side_with_two_thrusters_holds_its_half() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2"]);
        let l = lift(&d, &f.parts, false);
        assert!(l.holds);
        assert_eq!(l.port, 0.5);
        assert_eq!(l.throttle_outer[0], 1.0, "two thrusters at full throttle");
        assert_eq!(l.throttle_outer[1], 0.5, "four at half");
    }

    #[test]
    fn the_hull_thrusters_hold_a_side_with_one_outer_thruster() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["R1", "R2", "R3"]);
        assert!(!lift(&d, &f.parts, false).holds);
        let lit = lift(&d, &f.parts, true);
        assert!(lit.holds);
        assert_eq!(lit.stbd, 0.5);
        assert!(
            lit.throttle_hull[1] > lit.throttle_hull[0],
            "the weak side's hull thrusters work hardest: {lit:?}"
        );
    }

    #[test]
    fn the_lift_readout_is_the_rule() {
        // The cockpit reads `Fortress::lift`: the same numbers the step flies by, changing in the same step as a
        // thruster is lost.
        let (d, mut f) = fortress();
        let before = f.lift(&d);
        kill(&d, &mut f, &["L1", "L2", "L3"]);
        let after = f.lift(&d);
        assert_ne!(before, after);
        assert_eq!(after, lift(&d, &f.parts, f.hull_lit(&d.cfg)));
        assert_eq!(after.port, d.cfg.lift_max_share);
    }
}
