//! A mech's data: its stand-in skeleton and boxes, an optional glb, and its locomotion and gait
//! tuning (`data/mechs/<name>.ron`, openspec change `mech-rig`).

use super::damage::{PartSpec, default_parts, validate_parts};
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
    /// Radius of the body for building collision, and the legs' reach against tree trunks.
    pub body_radius_m: f32,
    /// Reach of the hull and arms through tree crowns (vegetation design section 5).
    pub brush_radius_m: f32,
    /// Height from the feet to the top of the mech (survey M1: 12 m).
    pub body_height_m: f32,
    /// Below this ground speed the mech counts as stopped (boarding allowed).
    pub stopped_speed_m_s: f32,
    /// Hip height, for wading: water this deep reaches the hips.
    pub hip_height_m: f32,
    /// Deepest water the mech walks into (the cockpit floor).
    pub max_wade_depth_m: f32,
    /// Share of top speed lost with water at the hips (terrain-water design section 2).
    pub wade_slowdown: f32,
    /// Mass, for knocking things over.
    pub mass_kg: f32,
    /// Share of walking speed left with one leg (mech-damage design section 2).
    pub limp_speed_ratio: f32,
    /// The root within this far under the water surface counts as at the surface (water-skimming).
    pub float_catch_m: f32,
    /// Sinking speed through water.
    pub water_sink_m_s: f32,
    /// Fastest rise through water on the rockets.
    pub water_climb_m_s: f32,
    /// How hard water brakes the mech's motion through it (sinking, entering at speed).
    pub water_drag_m_s2: f32,
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
            brush_radius_m: 5.5,
            body_height_m: 12.0,
            stopped_speed_m_s: 0.2,
            hip_height_m: 5.9,
            max_wade_depth_m: 8.0,
            wade_slowdown: 0.6,
            mass_kg: 50_000.0,
            limp_speed_ratio: 0.22,
            float_catch_m: 1.0,
            water_sink_m_s: 4.0,
            water_climb_m_s: 10.0,
            water_drag_m_s2: 30.0,
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

/// Booster tuning (openspec change `boost-and-skate`; survey M3: heat-limited, fuel ignored).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct JetConfig {
    /// Upward acceleration at full lift, before gravity.
    pub thrust_m_s2: f32,
    /// Fastest climb.
    pub climb_max_m_s: f32,
    /// Fastest fall while lifting at all.
    pub sink_max_m_s: f32,
    /// Fastest fall without the rockets.
    pub fall_max_m_s: f32,
    /// Extra downward acceleration while diving (Ctrl in the air).
    pub dive_m_s2: f32,
    /// Fastest fall while diving.
    pub dive_max_m_s: f32,
    /// Air drag on horizontal speed without thrust, per second.
    pub drag_per_s: f32,
    /// Seconds of burn from full.
    pub burn_s: f32,
    /// Seconds to recharge from empty.
    pub recharge_s: f32,
    /// Pause after burning before recharging starts.
    pub recharge_delay_s: f32,
    /// Emptied, they relight once this share is back.
    pub relight_fraction: f32,
}

impl Default for JetConfig {
    fn default() -> Self {
        Self {
            // Thrust to weight 5.7 (owner, 2026-10-24: "make rocket TWR 2x stronger").
            thrust_m_s2: 56.0,
            climb_max_m_s: 60.0,
            sink_max_m_s: 25.0,
            fall_max_m_s: 60.0,
            dive_m_s2: 30.0,
            dive_max_m_s: 90.0,
            drag_per_s: 0.3,
            burn_s: 20.0,
            recharge_s: 8.0,
            recharge_delay_s: 1.5,
            relight_fraction: 0.3,
        }
    }
}

/// Skate tuning (openspec change `boost-and-skate`; survey M4: foot wheels).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SkateConfig {
    /// Top speed at full throttle.
    pub speed_max_m_s: f32,
    /// Wheel drive from a stand; it fades as `1 - (v / speed_max)^2` toward top speed.
    pub accel_m_s2: f32,
    /// Braking when the throttle asks for less.
    pub brake_m_s2: f32,
    /// How fast the velocity swings onto the heading, per second (low: drifts).
    pub grip_per_s: f32,
    /// Turn rate.
    pub turn_rate_deg_s: f32,
    /// Seconds of use from full.
    pub burn_s: f32,
    /// Seconds to recharge from empty.
    pub recharge_s: f32,
    /// Pause after skating before recharging starts.
    pub recharge_delay_s: f32,
    /// Emptied, they work again once this share is back.
    pub relight_fraction: f32,
}

impl Default for SkateConfig {
    fn default() -> Self {
        Self {
            // Owner, 2026-10-24: "skate speed 4x faster, good acceleration curve, just takes time
            // to build speed": 55 m/s at 5 s, 109 m/s at 15 s.
            speed_max_m_s: 120.0,
            accel_m_s2: 12.0,
            brake_m_s2: 15.0,
            grip_per_s: 1.2,
            turn_rate_deg_s: 90.0,
            burn_s: 40.0,
            recharge_s: 10.0,
            recharge_delay_s: 1.0,
            relight_fraction: 0.2,
        }
    }
}

/// The sizes the game reads that the rig does not carry, per mech (openspec change `mech-roster` section 3). The
/// defaults are the 12 m stand-in's, so a mech file that leaves them out flies as it always did.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BodyConfig {
    /// Height of the pilot's eye over the feet at rest: where the AI and the targeting look from.
    pub eye_height_m: f32,
    /// The torso's hit capsule, from and to this far over the `torso` bone.
    pub torso_hit_m: [f32; 2],
    /// The torso's hit capsule radius.
    pub torso_hit_radius_m: f32,
    /// An arm's hit capsule radius, shoulder to hand.
    pub arm_hit_radius_m: f32,
    /// A leg's hit capsule radius, thigh to foot.
    pub leg_hit_radius_m: f32,
    /// The head's hit capsule radius.
    pub head_hit_radius_m: f32,
    /// Height of the centre of mass over the feet, for the wreck's topple.
    pub wreck_centre_m: f32,
    /// Height over the feet of the blast when the torso goes.
    pub blast_height_m: f32,
    /// Height over the feet of the point that breaks a wall the mech strides into.
    pub smash_height_m: f32,
    /// Half extents of the physics box a shot-off part falls as.
    pub cut_box_m: [f32; 3],
}

impl Default for BodyConfig {
    fn default() -> Self {
        Self {
            eye_height_m: 9.6,
            torso_hit_m: [0.5, 3.9],
            torso_hit_radius_m: 2.4,
            arm_hit_radius_m: 1.0,
            leg_hit_radius_m: 1.1,
            head_hit_radius_m: 1.1,
            wreck_centre_m: 10.0,
            blast_height_m: 9.0,
            smash_height_m: 5.0,
            cut_box_m: [1.6, 3.0, 1.6],
        }
    }
}

/// An interior part that opens on a chest joint: the two open as one body (`mech_boarding.pod`, mode "inner").
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RideSpec {
    /// The interior's node, moved with everything under it.
    pub part: String,
    /// The mech's chest joint it rides.
    pub joint: String,
}

/// The cockpit interior a mech carries in its chest (openspec change `mech-roster` section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InteriorConfig {
    /// A glTF under `assets/`, hung on the `cockpit` bone at its origin.
    pub model: String,
    /// The interior parts that ride chest joints.
    #[serde(default)]
    pub rides: Vec<RideSpec>,
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
    /// The cockpit interior hung in its chest, for a glb whose bay is empty.
    pub interior: Option<InteriorConfig>,
    /// Sizes the game reads that the rig does not carry.
    pub body: BodyConfig,
    /// Locomotion tuning.
    pub locomotion: LocomotionConfig,
    /// Walk tuning.
    pub gait: GaitConfig,
    /// Booster tuning.
    pub jet: JetConfig,
    /// Skate tuning.
    pub skate: SkateConfig,
    /// Parts that can be shot off.
    pub parts: Vec<PartSpec>,
    /// Guns.
    pub weapons: Vec<crate::weapons::WeaponSpec>,
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
            interior: None,
            body: BodyConfig::default(),
            locomotion: LocomotionConfig::default(),
            gait: GaitConfig::default(),
            jet: JetConfig::default(),
            skate: SkateConfig::default(),
            parts: default_parts(),
            weapons: crate::weapons::default_weapons(),
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
        positive("locomotion.brush_radius_m", l.brush_radius_m)?;
        positive("locomotion.body_height_m", l.body_height_m)?;
        positive("locomotion.stopped_speed_m_s", l.stopped_speed_m_s)?;
        positive("locomotion.hip_height_m", l.hip_height_m)?;
        positive("locomotion.max_wade_depth_m", l.max_wade_depth_m)?;
        within("locomotion.wade_slowdown", l.wade_slowdown, 0.0, 0.95)?;
        positive("locomotion.mass_kg", l.mass_kg)?;
        within("locomotion.limp_speed_ratio", l.limp_speed_ratio, 0.0, 1.0)?;
        positive("locomotion.float_catch_m", l.float_catch_m)?;
        positive("locomotion.water_sink_m_s", l.water_sink_m_s)?;
        positive("locomotion.water_climb_m_s", l.water_climb_m_s)?;
        positive("locomotion.water_drag_m_s2", l.water_drag_m_s2)?;
        let g = &self.gait;
        positive("gait.stride_max_m", g.stride_max_m)?;
        positive("gait.stride_min_m", g.stride_min_m)?;
        finite("gait.step_height_m", g.step_height_m)?;
        finite("gait.bob_m", g.bob_m)?;
        finite("gait.sway_m", g.sway_m)?;
        finite("gait.turn_step_radius_m", g.turn_step_radius_m)?;
        positive("gait.settle_speed_m_s", g.settle_speed_m_s)?;
        let j = &self.jet;
        positive("jet.thrust_m_s2", j.thrust_m_s2)?;
        positive("jet.climb_max_m_s", j.climb_max_m_s)?;
        positive("jet.sink_max_m_s", j.sink_max_m_s)?;
        positive("jet.fall_max_m_s", j.fall_max_m_s)?;
        positive("jet.dive_m_s2", j.dive_m_s2)?;
        positive("jet.dive_max_m_s", j.dive_max_m_s)?;
        crate::data::finite("jet.drag_per_s", j.drag_per_s)?;
        positive("jet.burn_s", j.burn_s)?;
        positive("jet.recharge_s", j.recharge_s)?;
        crate::data::finite("jet.recharge_delay_s", j.recharge_delay_s)?;
        within("jet.relight_fraction", j.relight_fraction, 0.0, 1.0)?;
        let k = &self.skate;
        positive("skate.speed_max_m_s", k.speed_max_m_s)?;
        positive("skate.accel_m_s2", k.accel_m_s2)?;
        positive("skate.brake_m_s2", k.brake_m_s2)?;
        positive("skate.grip_per_s", k.grip_per_s)?;
        positive("skate.turn_rate_deg_s", k.turn_rate_deg_s)?;
        positive("skate.burn_s", k.burn_s)?;
        positive("skate.recharge_s", k.recharge_s)?;
        crate::data::finite("skate.recharge_delay_s", k.recharge_delay_s)?;
        within("skate.relight_fraction", k.relight_fraction, 0.0, 1.0)?;
        let b = &self.body;
        positive("body.eye_height_m", b.eye_height_m)?;
        finite("body.torso_hit_m[0]", b.torso_hit_m[0])?;
        positive("body.torso_hit_m[1]", b.torso_hit_m[1])?;
        if b.torso_hit_m[1] <= b.torso_hit_m[0] {
            return Err("body.torso_hit_m: the capsule must end above where it starts".into());
        }
        positive("body.torso_hit_radius_m", b.torso_hit_radius_m)?;
        positive("body.arm_hit_radius_m", b.arm_hit_radius_m)?;
        positive("body.leg_hit_radius_m", b.leg_hit_radius_m)?;
        positive("body.head_hit_radius_m", b.head_hit_radius_m)?;
        positive("body.wreck_centre_m", b.wreck_centre_m)?;
        finite("body.blast_height_m", b.blast_height_m)?;
        finite("body.smash_height_m", b.smash_height_m)?;
        for (i, v) in b.cut_box_m.iter().enumerate() {
            positive(&format!("body.cut_box_m[{i}]"), *v)?;
        }
        if let Some(i) = &self.interior {
            if self.model.is_none() {
                return Err("interior: only a glb mech carries an interior".into());
            }
            if !(i.model.ends_with(".glb") || i.model.ends_with(".gltf")) {
                return Err(format!("interior.model: `{}` must be a .glb or .gltf under assets/", i.model));
            }
            for r in &i.rides {
                if r.part.is_empty() || r.joint.is_empty() {
                    return Err("interior.rides: a ride needs a part and a joint".into());
                }
            }
        }
        validate_parts(&self.parts)?;
        for w in &self.weapons {
            w.validate()?;
        }
        for n in &self.nodes {
            finite3(&format!("nodes[{}].offset_m", n.name), n.offset_m)?;
            let parent = (!n.parent.is_empty()).then_some(n.parent.as_str());
            match contract_parent(&n.name).or_else(|| super::rig::optional_parent(&n.name)) {
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
    fn a_mech_without_a_body_section_keeps_the_stand_ins_sizes() {
        let m = parse_ron::<MechConfig>(r#"(name: "x")"#, Path::new("m.ron")).unwrap();
        assert_eq!(m.body.eye_height_m, 9.6);
        assert_eq!(m.body.torso_hit_m, [0.5, 3.9]);
    }

    #[test]
    fn an_interior_without_a_model_is_refused() {
        let e = parse_ron::<MechConfig>(r#"(interior: Some((model: "models/c.gltf")))"#, Path::new("m.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("interior"), "{e}");
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
