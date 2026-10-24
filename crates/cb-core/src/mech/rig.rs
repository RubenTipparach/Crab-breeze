//! The mech rig contract (`docs/design/mech-rig.md`) as the one list the engine reads. Bones and
//! sockets are found by these names in the stand-in and in every Blender glb; a model that names
//! them otherwise is wrong, not the engine.

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

/// The legs' bone names, outward from the hip, for one side (`"L"` or `"R"`).
pub fn leg_bones(side: &str) -> [String; 5] {
    ["thigh", "shin", "hock", "foot", "toe"].map(|b| format!("{b}_{side}"))
}

/// The parent a contract node must have, or `None` if the name is not in the contract.
pub fn contract_parent(name: &str) -> Option<Option<&'static str>> {
    BONES.iter().chain(SOCKETS.iter()).find(|b| b.name == name).map(|b| b.parent)
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
