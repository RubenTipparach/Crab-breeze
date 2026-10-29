//! The mech rig contract (`docs/design/mech-rig.md`) as the one list the engine reads. Bones and
//! sockets are found by these names in the stand-in and in every Blender glb; a model that names
//! them otherwise is wrong, not the engine. Also how a pose composes down the bones (rotations and
//! origins in model space), the one composition the drawing and the arm aim both read.

use glam::{Quat, Vec3};

/// A contract bone: its name and its parent's name (`None` for `root`).
pub struct BoneSpec {
    /// Node name in the glb.
    pub name: &'static str,
    /// Parent bone's name.
    pub parent: Option<&'static str>,
}

const fn bone(name: &'static str, parent: &'static str) -> BoneSpec {
    BoneSpec {
        name,
        parent: Some(parent),
    }
}

/// Every contract bone, parents before children.
pub const BONES: &[BoneSpec] = &[
    BoneSpec {
        name: "root",
        parent: None,
    },
    bone("pelvis", "root"),
    bone("torso_twist", "pelvis"),
    bone("torso", "torso_twist"),
    bone("cockpit", "torso"),
    bone("shoulder_L", "torso"),
    bone("upper_arm_L", "shoulder_L"),
    bone("forearm_L", "upper_arm_L"),
    bone("hand_L", "forearm_L"),
    bone("shoulder_R", "torso"),
    bone("upper_arm_R", "shoulder_R"),
    bone("forearm_R", "upper_arm_R"),
    bone("hand_R", "forearm_R"),
    bone("thigh_L", "pelvis"),
    bone("shin_L", "thigh_L"),
    bone("hock_L", "shin_L"),
    bone("foot_L", "hock_L"),
    bone("toe_L", "foot_L"),
    bone("thigh_R", "pelvis"),
    bone("shin_R", "thigh_R"),
    bone("hock_R", "shin_R"),
    bone("foot_R", "hock_R"),
    bone("toe_R", "foot_R"),
];

/// Every contract socket (empty) and the bone it hangs from.
pub const SOCKETS: &[BoneSpec] = &[
    bone("EYE_pilot", "cockpit"),
    bone("SEAT_pilot", "cockpit"),
    bone("HATCH_entry", "cockpit"),
    bone("LADDER_base", "root"),
    bone("MUZZLE_L", "hand_L"),
    bone("MUZZLE_R", "hand_R"),
    bone("JET_L", "torso"),
    bone("JET_R", "torso"),
    bone("WHEEL_L", "foot_L"),
    bone("WHEEL_R", "foot_R"),
];

/// Optional bones a mech may carry; the engine uses them when present (`head`: the sensor head
/// with the main cameras, openspec change `mech-damage`).
pub const OPTIONAL: &[BoneSpec] = &[bone("head", "torso")];

/// The parent an optional node must have, or `None` if the name is not an optional node.
pub fn optional_parent(name: &str) -> Option<Option<&'static str>> {
    OPTIONAL.iter().find(|b| b.name == name).map(|b| b.parent)
}

/// The legs' bone names, outward from the hip, for one side (`"L"` or `"R"`).
pub fn leg_bones(side: &str) -> [String; 5] {
    ["thigh", "shin", "hock", "foot", "toe"].map(|b| format!("{b}_{side}"))
}

/// The parent a contract node must have, or `None` if the name is not in the contract.
pub fn contract_parent(name: &str) -> Option<Option<&'static str>> {
    BONES.iter().chain(SOCKETS.iter()).find(|b| b.name == name).map(|b| b.parent)
}

/// The `BONES` index of a contract bone, by name.
pub fn bone_index(name: &str) -> Option<usize> {
    BONES.iter().position(|b| b.name == name)
}

/// The `BONES` index of bone `i`'s parent (`None` for `root`).
pub fn parent_index(i: usize) -> Option<usize> {
    BONES.get(i).and_then(|b| b.parent).and_then(bone_index)
}

/// Every contract bone's rest pose in model space (glTF: +y up, +z forward, +x the mech's left), by `BONES`
/// index, measured off the bound model by chaining its rest transforms (never typed in), and the two muzzle
/// sockets' rest positions and barrel directions (their +z), left then right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RestFrames {
    /// Each bone's origin.
    pub pos: [Vec3; BONES.len()],
    /// Each bone's rotation.
    pub rot: [Quat; BONES.len()],
    /// `MUZZLE_L` and `MUZZLE_R`: where each sits and where its +z points.
    pub muzzles: [(Vec3, Vec3); 2],
}

/// Each bone's model-space rotation from rest for a pose: a bone named in `own` takes that rotation, any
/// other inherits its parent's; then a bone named in `turns` turns that much more in its own rest axes (the
/// kneel's arm offsets). The one composition the drawing and the arm aim both read.
pub fn bone_deltas(own: &[(&str, Quat)], turns: &[(&str, Quat)], rest_rot: &[Quat; BONES.len()]) -> [Quat; BONES.len()] {
    let mut d = [Quat::IDENTITY; BONES.len()];
    for (i, b) in BONES.iter().enumerate() {
        let inherited = parent_index(i).map_or(Quat::IDENTITY, |p| d[p]);
        d[i] = own.iter().find(|(n, _)| *n == b.name).map_or(inherited, |(_, q)| *q);
        if let Some((_, off)) = turns.iter().find(|(n, q)| *n == b.name && *q != Quat::IDENTITY) {
            d[i] *= in_rest_axes(rest_rot[i], *off);
        }
    }
    d
}

/// Layers more turns over a posed rig (`base`, from [`bone_deltas`]): a bone named in `turns` turns that much
/// more in its own rest axes, and everything beneath it is carried along, a child with a rotation of its own
/// (a strike's forearm) included. The arm aim's offsets (openspec change `arm-aim`).
pub fn turned_deltas(base: &[Quat; BONES.len()], turns: &[(&str, Quat)], rest_rot: &[Quat; BONES.len()]) -> [Quat; BONES.len()] {
    let mut d = *base;
    for (i, b) in BONES.iter().enumerate() {
        // What the parent's extra turn did to it carries to this bone.
        let carried = parent_index(i).map_or(Quat::IDENTITY, |p| d[p] * base[p].inverse());
        d[i] = carried * base[i];
        if let Some((_, off)) = turns.iter().find(|(n, q)| *n == b.name && *q != Quat::IDENTITY) {
            d[i] *= in_rest_axes(rest_rot[i], *off);
        }
    }
    d
}

/// A turn `off` in a bone's own rest axes, as a model-space rotation.
pub fn in_rest_axes(rest_rot: Quat, off: Quat) -> Quat {
    rest_rot * off * rest_rot.inverse()
}

/// Each bone's model-space origin under rotations `d` (from [`bone_deltas`]): every bone keeps its rest offset
/// from its parent, turned by the parent; the pelvis is moved by `pelvis_offset_m`.
pub fn bone_positions(d: &[Quat; BONES.len()], rest_pos: &[Vec3; BONES.len()], pelvis_offset_m: Vec3) -> [Vec3; BONES.len()] {
    let mut p = *rest_pos;
    for (i, b) in BONES.iter().enumerate() {
        if let Some(pi) = parent_index(i) {
            p[i] = p[pi] + d[pi] * (rest_pos[i] - rest_pos[pi]);
        }
        if b.name == "pelvis" {
            p[i] += pelvis_offset_m;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_bone_has_a_parent_in_the_contract() {
        for (i, b) in BONES.iter().enumerate() {
            if let Some(p) = b.parent {
                let at = BONES.iter().position(|o| o.name == p);
                assert!(at.is_some_and(|at| at < i), "{} needs {} listed before it", b.name, p);
            }
        }
        for s in SOCKETS {
            assert!(BONES.iter().any(|b| Some(b.name) == s.parent), "{}", s.name);
        }
    }
}
