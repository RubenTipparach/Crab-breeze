//! The keyframed melee moves and gun recoil, `data/combat/melee_moves.ron` (openspec change `close-combat`, tasks
//! 2.4 and 2.5). They are designed on the Mech Arms Lab page and exported from its one copy,
//! `tools/mockups/mech_melee.js`, by `tools/mockups/export_melee_moves.mjs`; this module plays them the way the
//! page does: per bone an offset over the pose beneath, eased (smoothstep) between keys, nothing before the first
//! key or after the last, so a move starts and ends on the gait. Pure.

use crate::data::Validate;
use crate::mech::rig::BONES;
use glam::{EulerRot, Quat, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Every move, by name (`jab_L`, `jab_R`, `recoil_R`...).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeleeMoves {
    /// The moves.
    pub moves: BTreeMap<String, MeleeMove>,
}

/// One move.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeleeMove {
    /// The arm that strikes or fires: "L", "R" or "both".
    pub arm: String,
    /// When the blow lands, seconds in (0 for a gun's recoil).
    pub hit_s: f32,
    /// How much of a struck mech's stance it breaks, in blows.
    pub stance: f32,
    /// A gun's recoil, not a blow.
    pub fire: bool,
    /// The keys, in time order; the first and last are rest.
    pub keys: Vec<MoveKey>,
}

/// One key of a move.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MoveKey {
    /// Seconds into the move.
    pub t_s: f32,
    /// Per bone: (rx, ry, rz) radians in the bone's own axes, (px, py, pz) metres in its parent's.
    #[serde(default)]
    pub bones: BTreeMap<String, [f32; 6]>,
    /// How far the legs fold, as the boarding kneel's amount.
    #[serde(default)]
    pub crouch: f32,
}

/// A move's pose at one moment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MovePose {
    /// Turns in each bone's own rest axes, over the pose beneath.
    pub turns: Vec<(&'static str, Quat)>,
    /// Slides of each bone in its parent's space, metres.
    pub slides: Vec<(&'static str, Vec3)>,
    /// How far the legs fold.
    pub crouch: f32,
}

impl MovePose {
    /// Adds another move's pose on top of this one (a gun's recoil over a strike).
    pub fn add(&mut self, other: &MovePose) {
        self.turns.extend(other.turns.iter().copied());
        self.slides.extend(other.slides.iter().copied());
        self.crouch = self.crouch.max(other.crouch);
    }
}

fn bone(name: &str) -> Option<&'static str> {
    BONES.iter().map(|b| b.name).find(|n| *n == name)
}

impl MeleeMove {
    /// The move's length: its last key, seconds.
    pub fn end_s(&self) -> f32 {
        self.keys.last().map_or(0.0, |k| k.t_s)
    }

    /// The pose `t_s` seconds in: the offsets eased between the two keys around it (the lab's `meleeSample`).
    pub fn sample(&self, t_s: f32) -> MovePose {
        let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
            return MovePose::default();
        };
        if !t_s.is_finite() || t_s <= first.t_s || t_s >= last.t_s {
            return MovePose::default();
        }
        let i = self.keys.windows(2).position(|w| t_s <= w[1].t_s).unwrap_or(0);
        let (a, c) = (&self.keys[i], &self.keys[i + 1]);
        let u = ((t_s - a.t_s) / (c.t_s - a.t_s).max(1e-6)).clamp(0.0, 1.0);
        let k = u * u * (3.0 - 2.0 * u);
        let mut out = MovePose {
            crouch: a.crouch + (c.crouch - a.crouch) * k,
            ..MovePose::default()
        };
        let zero = [0.0; 6];
        let names = a.bones.keys().chain(c.bones.keys()).collect::<std::collections::BTreeSet<_>>();
        for name in names {
            let Some(b) = bone(name) else { continue };
            let (x, y) = (a.bones.get(name).unwrap_or(&zero), c.bones.get(name).unwrap_or(&zero));
            let v: [f32; 6] = std::array::from_fn(|j| x[j] + (y[j] - x[j]) * k);
            if v[..3].iter().any(|r| *r != 0.0) {
                out.turns.push((b, Quat::from_euler(EulerRot::XYZ, v[0], v[1], v[2])));
            }
            if v[3..].iter().any(|p| *p != 0.0) {
                out.slides.push((b, Vec3::new(v[3], v[4], v[5])));
            }
        }
        out
    }
}

impl MeleeMoves {
    /// The move named `name`, if there is one.
    pub fn get(&self, name: &str) -> Option<&MeleeMove> {
        self.moves.get(name)
    }
}

impl Validate for MeleeMoves {
    fn validate(&self) -> Result<(), String> {
        for (name, m) in &self.moves {
            let at = |f: &str| format!("moves.{name}.{f}");
            if !["L", "R", "both"].contains(&m.arm.as_str()) {
                return Err(format!("{}: \"{}\" is not L, R or both", at("arm"), m.arm));
            }
            for (f, v) in [("hit_s", m.hit_s), ("stance", m.stance)] {
                if !v.is_finite() || v < 0.0 {
                    return Err(format!("{}: {v} must be a finite number, zero or more", at(f)));
                }
            }
            if m.keys.len() < 2 {
                return Err(format!("{}: a move needs a first and a last key", at("keys")));
            }
            if m.hit_s > m.end_s() {
                return Err(format!("{}: {} lands after the move ends at {}", at("hit_s"), m.hit_s, m.end_s()));
            }
            for (i, k) in m.keys.iter().enumerate() {
                if !k.t_s.is_finite() || (i > 0 && k.t_s <= m.keys[i - 1].t_s) {
                    return Err(format!("{}: key times must rise", at(&format!("keys[{i}].t_s"))));
                }
                if (i == 0 || i == m.keys.len() - 1) && (!k.bones.is_empty() || k.crouch != 0.0) {
                    return Err(format!("{}: the first and last keys are rest", at(&format!("keys[{i}]"))));
                }
                for (b, v) in &k.bones {
                    if bone(b).is_none() {
                        return Err(format!("{}: \"{b}\" is not a contract bone", at(&format!("keys[{i}].bones"))));
                    }
                    if v.iter().any(|x| !x.is_finite()) {
                        return Err(format!("{}: not finite", at(&format!("keys[{i}].bones.{b}"))));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shipped() -> MeleeMoves {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        crate::data::load_ron(&root.join("combat/melee_moves.ron")).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn the_shipped_moves_load_and_hold_the_jabs_and_the_recoil() {
        let m = shipped();
        for id in ["jab_L", "jab_R", "recoil_L", "recoil_R", "hook_L", "smash", "charge", "kick_R"] {
            assert!(m.get(id).is_some(), "{id}");
        }
        assert!(m.get("recoil_R").unwrap().fire);
        assert_eq!(m.get("smash").unwrap().stance, 1.5);
    }

    #[test]
    fn a_move_adds_nothing_before_its_first_key_and_after_its_last() {
        let m = shipped();
        let jab = m.get("jab_R").unwrap();
        assert_eq!(jab.sample(0.0), MovePose::default());
        assert_eq!(jab.sample(jab.end_s()), MovePose::default());
        assert_eq!(jab.sample(jab.end_s() + 1.0), MovePose::default());
    }

    #[test]
    fn at_the_hit_the_right_jab_throws_the_right_shoulder_forward_and_lunges() {
        let m = shipped();
        let jab = m.get("jab_R").unwrap();
        let p = jab.sample(jab.hit_s);
        let sh = p.turns.iter().find(|(n, _)| *n == "shoulder_R").unwrap().1;
        let (rx, _, _) = sh.to_euler(EulerRot::XYZ);
        assert!((rx + 1.35).abs() < 1e-3, "{rx}");
        assert!(p.slides.iter().any(|(n, v)| *n == "pelvis" && v.z > 1.0));
        assert!(!p.turns.iter().any(|(n, _)| n.ends_with("_L") && n.starts_with("shoulder")));
    }

    #[test]
    fn the_left_jab_mirrors_the_right() {
        let m = shipped();
        let (l, r) = (m.get("jab_L").unwrap().sample(0.4), m.get("jab_R").unwrap().sample(0.4));
        let get = |p: &MovePose, n: &str| p.turns.iter().find(|(b, _)| *b == n).unwrap().1.to_euler(EulerRot::XYZ);
        let (lx, ly, _) = get(&l, "shoulder_L");
        let (rx, ry, _) = get(&r, "shoulder_R");
        assert!((lx - rx).abs() < 1e-4 && (ly + ry).abs() < 1e-4);
    }

    #[test]
    fn the_strike_clock_lands_and_ends_when_the_shipped_jab_does() {
        let m = shipped();
        let cfg = crate::mech::MeleeConfig::default();
        for id in ["jab_L", "jab_R"] {
            let jab = m.get(id).unwrap();
            assert!(
                (jab.hit_s - cfg.windup_s).abs() < 1e-4,
                "{id} lands at {} not {}",
                jab.hit_s,
                cfg.windup_s
            );
            assert!((jab.end_s() - cfg.strike_s()).abs() < 1e-4, "{id} ends at {}", jab.end_s());
        }
    }

    #[test]
    fn a_key_naming_a_bone_outside_the_contract_is_refused() {
        let mut m = shipped();
        let jab = m.moves.get_mut("jab_R").unwrap();
        jab.keys[1].bones.insert("elbow_R".into(), [0.0; 6]);
        assert!(m.validate().unwrap_err().contains("elbow_R"));
    }
}
