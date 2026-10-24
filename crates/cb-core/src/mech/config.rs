//! A mech's data: its stand-in skeleton and boxes, an optional glb, and its locomotion and gait
//! tuning (`data/mechs/<name>.ron`, openspec change `mech-rig`).

use super::rig::{BONES, SOCKETS, contract_parent};
use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;

/// One node of the stand-in: a contract bone or socket, placed relative to its parent.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Contract name.
    pub name: String,
    /// Parent's contract name; empty for `root`.
    #[serde(default)]
    pub parent: String,
    /// Offset from the parent at rest, in glTF space (+y up, +z forward, +x the mech's left).
    pub offset_m: [f32; 3],
}

/// A box of the stand-in's body, carried by a bone.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoxSpec {
    /// The bone that carries it.
    pub bone: String,
    /// Centre, relative to the bone.
    pub centre_m: [f32; 3],
    /// Full size. With `along_m`, the y size is replaced by the segment's length.
    pub size_m: [f32; 3],
    /// When set, the box runs from the bone's origin along this vector (its local y axis turned
    /// onto it), and `centre_m` is ignored: a limb segment.
    #[serde(default)]
    pub along_m: Option<[f32; 3]>,
    /// Paint tone index (0 hull, 1 dark, 2 joint, 3 cockpit frame, 4 glass, 5 accent).
    #[serde(default)]
    pub tone: u32,
}

/// Locomotion tuning (mech-rig design section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct LocomotionConfig {
    /// Ground speed at full throttle.
    pub walk_speed_max_m_s: f32,
    /// Lowest throttle lever setting (reverse), -1..0.
    pub throttle_min: f32,
    /// Throttle lever travel per second with the key held.
    pub throttle_rate_per_s: f32,
    /// Acceleration toward a higher speed.
    pub accel_m_s2: f32,
    /// Deceleration toward a lower speed.
    pub brake_m_s2: f32,
    /// Leg turn rate at full turn input.
    pub turn_rate_deg_s: f32,
    /// Torso twist limit either side of the legs.
    pub twist_max_deg: f32,
    /// Torso twist rate.
    pub twist_rate_deg_s: f32,
    /// Torso pitch limits, [down, up] (down negative).
    pub pitch_limits_deg: [f32; 2],
    /// Torso pitch rate.
    pub pitch_rate_deg_s: f32,
    /// Radius of the body for building collision.
    pub body_radius_m: f32,
    /// Below this ground speed the mech counts as stopped (boarding allowed).
    pub stopped_speed_m_s: f32,
}

impl Default for LocomotionConfig {
    fn default() -> Self {
        Self {
            walk_speed_max_m_s: 7.0,
            throttle_min: -0.4,
            throttle_rate_per_s: 0.6,
            accel_m_s2: 2.5,
            brake_m_s2: 4.0,
            turn_rate_deg_s: 30.0,
            twist_max_deg: 100.0,
            twist_rate_deg_s: 70.0,
            pitch_limits_deg: [-20.0, 25.0],
            pitch_rate_deg_s: 40.0,
            body_radius_m: 3.0,
            stopped_speed_m_s: 0.2,
        }
    }
}

/// Walk tuning (mech-rig design section 3).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GaitConfig {
    /// Distance covered per full gait cycle (two steps) at top speed.
    pub stride_max_m: f32,
    /// Shortest stride, used when slow or turning in place.
    pub stride_min_m: f32,
    /// Foot lift at the top of a swing.
    pub step_height_m: f32,
    /// Pelvis drop at mid-stance.
    pub bob_m: f32,
    /// Pelvis sway toward the stance foot.
    pub sway_m: f32,
    /// Turning in place steps as if walking at turn rate times this radius.
    pub turn_step_radius_m: f32,
    /// Speed by which the stride reaches its full swing; below it the feet settle together.
    pub settle_speed_m_s: f32,
}

impl Default for GaitConfig {
    fn default() -> Self {
        Self {
            stride_max_m: 9.0,
            stride_min_m: 2.0,
            step_height_m: 0.9,
            bob_m: 0.25,
            sway_m: 0.15,
            turn_step_radius_m: 2.5,
            settle_speed_m_s: 0.6,
        }
    }
}

/// A mech: its model, its stand-in, and its tuning.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct MechConfig {
    /// Display name.
    pub name: String,
    /// A glb under `assets/` following the rig contract (`models/mech_<name>.glb`). When set it
    /// replaces the stand-in's boxes; the stand-in skeleton is then unused.
    pub model: Option<String>,
    /// Locomotion tuning.
    pub locomotion: LocomotionConfig,
    /// Walk tuning.
    pub gait: GaitConfig,
    /// The stand-in's bones and sockets.
    pub nodes: Vec<NodeSpec>,
    /// The stand-in's boxes.
    pub boxes: Vec<BoxSpec>,
}

impl Default for MechConfig {
    fn default() -> Self {
        Self {
            name: "stand-in".into(),
            model: None,
            locomotion: LocomotionConfig::default(),
            gait: GaitConfig::default(),
            nodes: Vec::new(),
            boxes: Vec::new(),
        }
    }
}

impl Validate for MechConfig {
    fn validate(&self) -> Result<(), String> {
        let l = &self.locomotion;
        positive("locomotion.walk_speed_max_m_s", l.walk_speed_max_m_s)?;
        within("locomotion.throttle_min", l.throttle_min, -1.0, 0.0)?;
        positive("locomotion.throttle_rate_per_s", l.throttle_rate_per_s)?;
        positive("locomotion.accel_m_s2", l.accel_m_s2)?;
        positive("locomotion.brake_m_s2", l.brake_m_s2)?;
        positive("locomotion.turn_rate_deg_s", l.turn_rate_deg_s)?;
        within("locomotion.twist_max_deg", l.twist_max_deg, 0.0, 180.0)?;
        positive("locomotion.twist_rate_deg_s", l.twist_rate_deg_s)?;
        within("locomotion.pitch_limits_deg[0]", l.pitch_limits_deg[0], -89.0, 0.0)?;
        within("locomotion.pitch_limits_deg[1]", l.pitch_limits_deg[1], 0.0, 89.0)?;
        positive("locomotion.pitch_rate_deg_s", l.pitch_rate_deg_s)?;
        positive("locomotion.body_radius_m", l.body_radius_m)?;
        positive("locomotion.stopped_speed_m_s", l.stopped_speed_m_s)?;
        let g = &self.gait;
        positive("gait.stride_max_m", g.stride_max_m)?;
        positive("gait.stride_min_m", g.stride_min_m)?;
        finite("gait.step_height_m", g.step_height_m)?;
        finite("gait.bob_m", g.bob_m)?;
        finite("gait.sway_m", g.sway_m)?;
        finite("gait.turn_step_radius_m", g.turn_step_radius_m)?;
        positive("gait.settle_speed_m_s", g.settle_speed_m_s)?;
        for n in &self.nodes {
            finite3(&format!("nodes[{}].offset_m", n.name), n.offset_m)?;
            let parent = (!n.parent.is_empty()).then_some(n.parent.as_str());
            match contract_parent(&n.name) {
                None => return Err(format!("nodes: `{}` is not a contract bone or socket", n.name)),
                Some(p) if p != parent => {
                    return Err(format!(
                        "nodes: `{}` must hang from `{}`, not `{}`",
                        n.name,
                        p.unwrap_or(""),
                        n.parent
                    ));
                }
                Some(_) => {}
            }
        }
        if !self.nodes.is_empty() {
            for b in BONES.iter().chain(SOCKETS.iter()) {
                if !self.nodes.iter().any(|n| n.name == b.name) {
                    return Err(format!("nodes: the stand-in is missing contract node `{}`", b.name));
                }
            }
        }
        for b in &self.boxes {
            finite3("boxes.centre_m", b.centre_m)?;
            finite3("boxes.size_m", b.size_m)?;
            if let Some(v) = b.along_m {
                finite3("boxes.along_m", v)?;
            }
            if !self.nodes.iter().any(|n| n.name == b.bone) {
                return Err(format!("boxes: bone `{}` is not a stand-in node", b.bone));
            }
        }
        if let Some(m) = &self.model
            && !(m.ends_with(".glb") || m.ends_with(".gltf"))
        {
            return Err(format!("model: `{m}` must be a .glb or .gltf under assets/"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{load_ron, parse_ron};
    use std::path::{Path, PathBuf};

    fn shipped() -> MechConfig {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/mechs/placeholder.ron");
        load_ron(&p).expect("placeholder mech data loads")
    }

    #[test]
    fn the_placeholder_follows_the_contract() {
        let m = shipped();
        for b in BONES.iter().chain(SOCKETS.iter()) {
            let node = m
                .nodes
                .iter()
                .find(|n| n.name == b.name)
                .unwrap_or_else(|| panic!("{} missing", b.name));
            assert_eq!(node.parent, b.parent.unwrap_or(""), "{}", b.name);
        }
    }

    #[test]
    fn the_placeholder_is_twelve_metres_with_eyes_at_nine_point_six() {
        let m = shipped();
        let world_y = |name: &str| {
            let mut y = 0.0;
            let mut at = name.to_string();
            while let Some(n) = m.nodes.iter().find(|n| n.name == at) {
                y += n.offset_m[1];
                at = n.parent.clone();
            }
            y
        };
        assert!((world_y("EYE_pilot") - 9.6).abs() < 1e-3);
        let torso_top = world_y("torso") + 2.2 + 2.2;
        assert!((torso_top - 12.0).abs() < 1e-3, "{torso_top}");
    }

    #[test]
    fn a_node_off_the_contract_is_refused_by_name() {
        let e = parse_ron::<MechConfig>(
            r#"(nodes: [(name: "knee_L", parent: "thigh_L", offset_m: (0.0, 0.0, 0.0))])"#,
            Path::new("m.ron"),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("knee_L"), "{e}");
    }
}
