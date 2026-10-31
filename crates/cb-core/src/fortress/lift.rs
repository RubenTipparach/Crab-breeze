//! The lift rule (design section 12, owner 2026-10-31: "Even with one leg, it'll stay afloat", "the crab carrier
//! either looses thrust completely or it doesnt"): it holds its height while any lift thruster works; with none, its
//! hull thrusters hold it until `hull_kills_to_fall` of them are lost; then it has no thrust at all. A thruster runs at
//! the share of the weight its live fellows leave it, past its rating if it must. The cockpit's lift readout is this
//! function's result (CLAUDE.md 6.1).

use super::{FortressData, PartState};

/// Each side's lift as the rule has it. Side 0 is port (+X), side 1 starboard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lift {
    /// Port's lift, as a share of the weight (0.5 while it holds, 0 once it has no thrust).
    pub port: f32,
    /// Starboard's lift, as a share of the weight.
    pub stbd: f32,
    /// It holds its height.
    pub holds: bool,
    /// Live lift thrusters by side.
    pub outer: [u32; 2],
    /// Live hull thrusters by side.
    pub hull: [u32; 2],
    /// The hull thrusters are lit.
    pub lit: bool,
    /// Each side's live lift thrusters' throttle, 0..1 (1 is white hot; the last ones run past it, shown as 1).
    pub throttle_outer: [f32; 2],
    /// Each side's hull thrusters' throttle, 0..1 (0 while unlit).
    pub throttle_hull: [f32; 2],
}

/// The lift with the parts as they stand; `lit` says whether the hull thrusters burn.
pub fn lift(d: &FortressData, parts: &[PartState], lit: bool) -> Lift {
    let mut outer = [0u32; 2];
    let mut hull = [0u32; 2];
    let mut hull_lost = 0u32;
    for (r, p) in d.shape.parts.iter().zip(parts) {
        if r.side == 0 {
            continue;
        }
        let s = usize::from(r.side < 0);
        if !p.alive {
            hull_lost += u32::from(r.kind == "hull");
            continue;
        }
        match r.kind.as_str() {
            "lift" => outer[s] += 1,
            "hull" => hull[s] += 1,
            _ => {}
        }
    }
    let legs = outer[0] + outer[1];
    let ducts = hull[0] + hull[1];
    // Any leg holds it; with none, the hull holds it (while its doors open too) until half of it is lost.
    let holds = legs > 0 || (ducts > 0 && hull_lost < d.cfg.hull_kills_to_fall);
    let share = if holds { 0.5 } else { 0.0 };
    let (throttle_outer, throttle_hull) = if legs > 0 {
        let t = (1.0 / (legs as f32 * d.cfg.lift_max_share)).min(1.0);
        (outer.map(|n| if n > 0 { t } else { 0.0 }), [0.0; 2])
    } else if holds && lit {
        let t = (1.0 / (ducts as f32 * d.cfg.hull_max_share)).min(1.0);
        ([0.0; 2], hull.map(|n| if n > 0 { t } else { 0.0 }))
    } else {
        ([0.0; 2], [0.0; 2])
    };
    Lift {
        port: share,
        stbd: share,
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
    fn while_any_leg_works_it_holds_its_height() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3"]);
        let l = lift(&d, &f.parts, false);
        assert!(l.holds, "one leg left: it holds");
        assert_eq!((l.port, l.stbd), (0.5, 0.5), "no side short, no list");
        assert_eq!(l.throttle_outer, [0.0, 1.0], "the last leg white hot");
    }

    #[test]
    fn it_falls_only_when_half_its_hull_thrusters_are_gone() {
        let (d, mut f) = fortress();
        kill(&d, &mut f, &["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4"]);
        assert!(lift(&d, &f.parts, false).holds, "the hull takes the weight as its doors open");
        kill(&d, &mut f, &["HL1", "HR1"]);
        let two = lift(&d, &f.parts, true);
        assert!(two.holds, "two of six lost, from both sides: it holds");
        assert!(two.throttle_hull[0] > 0.0 && two.throttle_hull[1] > 0.0);
        kill(&d, &mut f, &["HL2"]);
        let three = lift(&d, &f.parts, true);
        assert!(!three.holds, "three of six: it falls");
        assert_eq!((three.port, three.stbd), (0.0, 0.0), "no thrust at all");
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
        assert!(after.holds);
    }
}
