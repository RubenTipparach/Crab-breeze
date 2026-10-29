//! The procedural digitigrade walk (mech-rig design section 3): where the feet go in the gait
//! cycle, how the pelvis bobs and sways, and the leg solver that turns a foot target into bone
//! rotations. All in model space (glTF: +y up, +z forward, +x the mech's left), as rotations
//! relative to the rest pose, so any model on the contract is driven the same way.

use super::config::GaitConfig;
use super::locomotion::MechState;
use crate::boarding::SitPose;
use glam::{Quat, Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// A leg's joint positions at rest, in model space: the origins of thigh, shin, hock, foot, toe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegRest {
    /// Thigh origin (hip joint).
    pub hip: Vec3,
    /// Shin origin (knee).
    pub knee: Vec3,
    /// Hock origin (back knee).
    pub hock: Vec3,
    /// Foot origin (ankle).
    pub ankle: Vec3,
}

/// The rest positions the pose needs, measured from the bound model (never from a copy).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigRest {
    /// `pelvis` origin, model space.
    pub pelvis: Vec3,
    /// Left then right leg.
    pub legs: [LegRest; 2],
}

/// Bone rotations for one leg, model space, relative to rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegPose {
    /// Thigh.
    pub thigh: Quat,
    /// Shin.
    pub shin: Quat,
    /// Hock (keeps its rest angle).
    pub hock: Quat,
    /// Foot (kept flat).
    pub foot: Quat,
    /// Where the solver put the ankle (model space), for tests and debugging.
    pub ankle: Vec3,
}

/// The whole mech's pose relative to rest, model space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MechPose {
    /// Pelvis offset from rest (bob and sway).
    pub pelvis_offset: Vec3,
    /// The part of `pelvis_offset` that is the stride: its bob and sway, or a limp's hop. The
    /// pilot's view leaves it out (owner, 2026-10-25: "I dont like foot step motion in the
    /// cocopit"); the body still moves with it.
    pub walk_offset: Vec3,
    /// `torso_twist` rotation.
    pub torso_twist: Quat,
    /// `torso` rotation (twist then pitch).
    pub torso: Quat,
    /// Left leg.
    pub left: LegPose,
    /// Right leg.
    pub right: LegPose,
    /// The kneel's arm offset (openspec change `mech-arm-clearance`): each arm bone's extra turn
    /// in its own rest axes, on top of what it inherits, by contract name. Identity standing.
    pub arms: [(&'static str, Quat); 4],
}

impl MechPose {
    /// Each driven bone's model-space rotation from rest, by contract name.
    pub fn rotations(&self) -> [(&'static str, Quat); 10] {
        [
            ("torso_twist", self.torso_twist),
            ("torso", self.torso),
            ("thigh_L", self.left.thigh),
            ("shin_L", self.left.shin),
            ("hock_L", self.left.hock),
            ("foot_L", self.left.foot),
            ("thigh_R", self.right.thigh),
            ("shin_R", self.right.shin),
            ("hock_R", self.right.hock),
            ("foot_R", self.right.foot),
        ]
    }
}

/// The kneel's arm pose at sit amount `s` (0 standing, 1 sat), as the Mechs page's `sitPose`
/// adds it (tools/mockups/mech_pose.js: its own arm pose every mech takes, plus the mech's
/// `KNEEL_ARMS` clearance row, summed in the data by `gen_mech_data.py`): each shoulder turns `arm_forward_deg` about
/// its own side axis then `arm_out_deg` outward about its own forward axis (the right shoulder the
/// mirror way), and each forearm bends `elbow_deg` about its side axis, all scaled by `s`. Each is
/// a turn in the bone's own rest axes, the way the page adds Euler angles to the node's rest.
pub fn kneel_arm_offsets(sit: &SitPose, s: f32) -> [(&'static str, Quat); 4] {
    if s <= 0.0 || (sit.arm_out_deg == 0.0 && sit.arm_forward_deg == 0.0 && sit.elbow_deg == 0.0) {
        return [
            ("shoulder_L", Quat::IDENTITY),
            ("shoulder_R", Quat::IDENTITY),
            ("forearm_L", Quat::IDENTITY),
            ("forearm_R", Quat::IDENTITY),
        ];
    }
    let fwd = Quat::from_rotation_x(sit.arm_forward_deg.to_radians() * s);
    let out = sit.arm_out_deg.to_radians() * s;
    let elbow = Quat::from_rotation_x(sit.elbow_deg.to_radians() * s);
    [
        ("shoulder_L", fwd * Quat::from_rotation_z(out)),
        ("shoulder_R", fwd * Quat::from_rotation_z(-out)),
        ("forearm_L", elbow),
        ("forearm_R", elbow),
    ]
}

/// A segment's angle in the leg's plane (y, z): rotating about +x by `d` adds `d` to it.
fn plane_angle(v: Vec3) -> f32 {
    v.z.atan2(v.y)
}

fn plane(v: Vec3) -> Vec2 {
    Vec2::new(v.y, v.z)
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// Solves one digitigrade leg: the hock turns `hock_turn_rad` from its rest angle (0 when
/// walking), thigh and shin bend as a two-bone chain from `hip` to the hock joint, the knee
/// bending the way it bends at rest. A target out of reach is pulled in to 99.5 % of the chain.
/// Works in the leg's (y, z) plane.
pub fn solve_leg(rest: &LegRest, hip: Vec3, ankle_target: Vec3, hock_turn_rad: f32) -> LegPose {
    let l1 = plane(rest.knee - rest.hip).length();
    let l2 = plane(rest.hock - rest.knee).length();
    let hock_rest = plane(rest.ankle - rest.hock);
    let (sin, cos) = hock_turn_rad.sin_cos();
    // Turning about +x adds to the plane angle atan2(z, y).
    let hock_vec = Vec2::new(hock_rest.x * cos - hock_rest.y * sin, hock_rest.x * sin + hock_rest.y * cos);
    let thigh_rest = plane_angle(rest.knee - rest.hip);
    let shin_rest = plane_angle(rest.hock - rest.knee);
    let base_rest = plane_angle(rest.hock - rest.hip);
    let bend = wrap(thigh_rest - base_rest).signum();

    let hip2 = plane(hip);
    let top = plane(ankle_target) - hock_vec;
    let to_top = top - hip2;
    let d = to_top.length().clamp((l1 - l2).abs() + 1e-3, 0.995 * (l1 + l2));
    let base = to_top.y.atan2(to_top.x);
    let cos_a = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let thigh = base + bend * cos_a.acos();
    let knee = hip2 + Vec2::new(thigh.cos(), thigh.sin()) * l1;
    let top_reached = hip2 + to_top.normalize_or_zero() * d;
    let shin_v = top_reached - knee;
    let shin = shin_v.y.atan2(shin_v.x);
    let ankle2 = top_reached + hock_vec;

    LegPose {
        thigh: Quat::from_rotation_x(wrap(thigh - thigh_rest)),
        shin: Quat::from_rotation_x(wrap(shin - shin_rest)),
        hock: Quat::from_rotation_x(hock_turn_rad),
        foot: Quat::IDENTITY,
        ankle: Vec3::new(ankle_target.x, ankle2.x, ankle2.y),
    }
}

/// A foot's offset from its rest ankle at gait phase `p` (0..1): stance slides back from
/// `+a` to `-a` in the first half; swing arcs forward `lift` high in the second.
pub fn foot_offset(p: f64, a: f32, lift: f32) -> Vec3 {
    let p = p.rem_euclid(1.0) as f32;
    if p < 0.5 {
        Vec3::new(0.0, 0.0, a * (1.0 - 4.0 * p))
    } else {
        let q = (p - 0.5) / 0.5;
        Vec3::new(0.0, lift * (PI * q).sin(), -a * (PI * q).cos())
    }
}

/// The pose for a mech state: feet from the gait clock, pelvis bob and sway, legs solved, torso
/// twisted and pitched; then blended toward sitting by `sit_amount` (0 standing, 1 sat), with
/// the pelvis lowered, the hocks swung back and the torso bowed (mech-boarding design section 2).
pub fn pose(state: &MechState, gait: &GaitConfig, rest: &RigRest, sit: &SitPose, sit_amount: f32) -> MechPose {
    let s = sit_amount.clamp(0.0, 1.0);
    let phase = state.gait_cycles;
    let swing = state.gait_swing * (1.0 - s);
    let a = stance_lead_m(state) * (1.0 - s);
    let lift = gait.step_height_m * swing;
    let p = phase.rem_euclid(1.0) as f32;
    let walk_offset = Vec3::new(
        gait.sway_m * swing * (TAU * p).sin(),
        -gait.bob_m * swing * 0.5 * (1.0 - (2.0 * TAU * p).cos()),
        0.0,
    );
    let mut walk_offset = walk_offset;
    let mut pelvis_offset = walk_offset + Vec3::Y * (sit.pelvis_height_m - rest.pelvis.y) * s;
    // A lost leg leaves its thigh as a crutch (mech-damage design section 2): the stump points
    // down and a little forward, the hips drop onto it, and the body hops on the good leg.
    let lost = state.legs_lost;
    let limping = lost.iter().any(|l| *l);
    let stump_angle = (0.35f32).atan2(-1.0);
    if limping {
        let leg = if lost[0] { &rest.legs[0] } else { &rest.legs[1] };
        let thigh = plane(leg.knee - leg.hip).length();
        let hip_y = thigh * stump_angle.cos().abs() + 0.3;
        let hop = -0.35 * swing * (TAU * p).sin().abs();
        pelvis_offset.y = (hip_y - leg.hip.y) + hop;
        pelvis_offset.x = 0.0;
        walk_offset = Vec3::Y * hop;
    }
    // Skating: crouched on the wheels, one foot ahead of the other (boost-and-skate).
    let skating = state.mode == super::locomotion::LocomotionMode::Skate;
    if skating {
        pelvis_offset.y -= 0.6;
    }
    let solve = |leg: &LegRest, ph: f64, gone: bool| {
        if gone {
            let turn = wrap(stump_angle - plane_angle(leg.knee - leg.hip));
            return LegPose {
                thigh: Quat::from_rotation_x(turn),
                shin: Quat::IDENTITY,
                hock: Quat::IDENTITY,
                foot: Quat::IDENTITY,
                ankle: leg.hip + pelvis_offset,
            };
        }
        // The sat hock lies `hock_angle_deg` back from straight down: plane angle pi - angle.
        let hock_rest = plane_angle(leg.ankle - leg.hock);
        let hock_sat = PI - sit.hock_angle_deg.to_radians();
        let turn = wrap(hock_sat - hock_rest) * s;
        let stagger = if skating {
            Vec3::Z * if leg.hip.x > 0.0 { 0.8 } else { -0.8 }
        } else {
            Vec3::ZERO
        };
        solve_leg(leg, leg.hip + pelvis_offset, leg.ankle + foot_offset(ph, a, lift) + stagger, turn)
    };
    // The torso rolls toward the stump (+x is the mech's left; rolling left is about -z).
    let roll = match lost {
        [true, false] => -12f32.to_radians(),
        [false, true] => 12f32.to_radians(),
        _ => 0.0,
    };
    let twist = Quat::from_rotation_z(roll) * Quat::from_rotation_y(state.twist_rad);
    // Pitch up turns +z toward +y: a negative rotation about +x. A bow is a positive one.
    let pitch = -state.pitch_rad + sit.torso_bow_deg.to_radians() * s;
    MechPose {
        pelvis_offset,
        walk_offset,
        torso_twist: twist,
        torso: twist * Quat::from_rotation_x(pitch),
        left: solve(&rest.legs[0], phase, lost[0]),
        right: solve(&rest.legs[1], phase + 0.5, lost[1]),
        arms: kneel_arm_offsets(sit, s),
    }
}

/// How far ahead of its rest ankle a foot comes down at the start of its stance (the swing ends at
/// `foot_offset(1.0) = +a`), metres along the mech's heading. The pose and the footprints both read
/// it, so a print lies where the drawn foot lands (openspec change `ground-marks`).
pub fn stance_lead_m(state: &MechState) -> f32 {
    state.stride_m * 0.25 * state.gait_swing
}

/// Which feet (left, right) start their stance between two steps: the left at each whole gait
/// cycle, the right half a cycle later, while the stride swings (at least 0.3) and the leg is
/// there; both on the step the mech lands from the air (`landed`, a stump included). The one foot-strike rule: the
/// footfall dust, the footprints and the pilot's jolt all ask here (openspec change `ground-marks`).
pub fn foot_strikes(prev: &MechState, curr: &MechState, landed: bool) -> [bool; 2] {
    [(0usize, 0.0f64), (1, 0.5)].map(|(side, offset)| {
        let crossed = (prev.gait_cycles - offset).floor() != (curr.gait_cycles - offset).floor();
        landed || (crossed && curr.gait_swing >= 0.3 && !curr.legs_lost[side])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mech::locomotion::MechControls;
    use crate::world::FlatGround;

    /// The stand-in's left leg (data/mechs/placeholder.ron), in model space.
    fn rest() -> LegRest {
        let hip = Vec3::new(1.6, 5.9, 0.0);
        let knee = hip + Vec3::new(0.0, -2.4, 1.4);
        let hock = knee + Vec3::new(0.0, -1.9, -1.8);
        let ankle = hock + Vec3::new(0.0, -1.1, 0.3);
        LegRest { hip, knee, hock, ankle }
    }

    fn rig() -> RigRest {
        let l = rest();
        let mut r = l;
        r.hip.x = -l.hip.x;
        r.knee.x = -l.knee.x;
        r.hock.x = -l.hock.x;
        r.ankle.x = -l.ankle.x;
        RigRest {
            pelvis: Vec3::new(0.0, 6.2, 0.0),
            legs: [l, r],
        }
    }

    #[test]
    fn a_mech_without_its_left_leg_stands_on_its_stump_and_its_good_foot() {
        let mut s = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        s.legs_lost = [true, false];
        let p = pose(&s, &GaitConfig::default(), &rig(), &SitPose::default(), 0.0);
        let r = rig();
        let (l, rr) = (r.legs[0], r.legs[1]);
        let stump_tip = l.hip + p.pelvis_offset + p.left.thigh * (l.knee - l.hip);
        assert!(
            (stump_tip.y - 0.3).abs() < 0.05,
            "the stump reaches down to the ground: {stump_tip}"
        );
        let foot = ankle_of(&rr, rr.hip + p.pelvis_offset, &p.right);
        assert!(foot.distance(rr.ankle) < 1e-2, "the good foot stays planted: {foot}");
    }

    #[test]
    fn sitting_lowers_the_pelvis_lays_the_hock_back_and_keeps_the_feet() {
        let sit = SitPose {
            pelvis_height_m: 2.4,
            ..SitPose::default()
        };
        let s = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let p = pose(&s, &GaitConfig::default(), &rig(), &sit, 1.0);
        assert!((rig().pelvis.y + p.pelvis_offset.y - 2.4).abs() < 1e-4);
        let r = rest();
        let hip = r.hip + p.pelvis_offset;
        let got = ankle_of(&r, hip, &p.left);
        assert!(got.distance(r.ankle) < 1e-3, "the foot stays planted: {got} vs {}", r.ankle);
        let hock_joint = hip + p.left.thigh * (r.knee - r.hip) + p.left.shin * (r.hock - r.knee);
        assert!(
            (hock_joint.y - r.ankle.y).abs() < 1e-3 && hock_joint.z < r.ankle.z,
            "the hock lies flat behind the foot: {hock_joint}"
        );
    }

    #[test]
    fn a_sat_foreman_bends_its_elbows_by_its_kneel_offset_and_a_standing_one_is_unchanged() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/mechs/foreman/boarding.ron");
        let tl: crate::boarding::BoardingTimeline = crate::data::load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        assert!(tl.sit.elbow_deg < 0.0, "the Foreman's kneel folds its elbows: {}", tl.sit.elbow_deg);
        let s = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let arm = |p: &MechPose, name: &str| p.arms.iter().find(|(n, _)| *n == name).map(|(_, q)| *q).unwrap();
        let sat = pose(&s, &GaitConfig::default(), &rig(), &tl.sit, 1.0);
        for side in ["forearm_L", "forearm_R"] {
            let (axis, angle) = arm(&sat, side).to_axis_angle();
            let signed = angle * axis.x.signum();
            assert!(
                (signed - tl.sit.elbow_deg.to_radians()).abs() < 1e-4 && axis.x.abs() > 0.999,
                "{side} bends {signed} rad about {axis}, want {} about x",
                tl.sit.elbow_deg.to_radians()
            );
        }
        let half = pose(&s, &GaitConfig::default(), &rig(), &tl.sit, 0.5);
        let (_, a) = arm(&half, "forearm_L").to_axis_angle();
        assert!(
            (a - 0.5 * tl.sit.elbow_deg.to_radians().abs()).abs() < 1e-4,
            "half sat, half the bend: {a}"
        );
        let standing = pose(&s, &GaitConfig::default(), &rig(), &tl.sit, 0.0);
        assert!(
            standing.arms.iter().all(|(_, q)| *q == Quat::IDENTITY),
            "standing, the arms are at rest: {:?}",
            standing.arms
        );
        let plain = pose(&s, &GaitConfig::default(), &rig(), &SitPose::default(), 0.0);
        assert_eq!(standing, plain, "standing, the offset changes nothing");
    }

    #[test]
    fn every_mech_kneels_with_the_pages_shared_arm_pose_and_only_the_clearance_four_add_to_it() {
        let sit_of = |id: &str| {
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/mechs/{id}/boarding.ron"));
            let tl: crate::boarding::BoardingTimeline = crate::data::load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
            tl.sit
        };
        let arms = |s: &SitPose| (s.arm_out_deg, s.arm_forward_deg, s.elbow_deg);
        let base = arms(&sit_of("driller"));
        assert!(
            base.0 > 0.0 && base.1 > 0.0 && base.2 < 0.0,
            "the shared kneel swings the shoulders and folds the elbows: {base:?}"
        );
        for id in ["ironhorse", "picket", "sapper"] {
            assert_eq!(
                arms(&sit_of(id)),
                base,
                "{id} has no clearance row, so it kneels with the shared arms"
            );
        }
        for id in ["foreman", "harrow", "roustabout", "mule"] {
            assert_ne!(arms(&sit_of(id)), base, "{id} adds its clearance row to the shared arms");
        }
        let foreman = sit_of("foreman");
        let o = kneel_arm_offsets(&foreman, 1.0);
        let want = Quat::from_rotation_x(foreman.arm_forward_deg.to_radians()) * Quat::from_rotation_z(foreman.arm_out_deg.to_radians());
        assert!(
            o[0].1.angle_between(want) < 1e-5,
            "the Foreman's left shoulder turns forward then out"
        );
        assert!(
            o[0].1.angle_between(Quat::IDENTITY) > 0.4,
            "sat, the shoulders come in: {:?}",
            o[0].1
        );
    }

    #[test]
    fn the_right_shoulder_swings_out_the_mirror_way() {
        let sit = SitPose {
            arm_out_deg: 30.0,
            ..SitPose::default()
        };
        let o = kneel_arm_offsets(&sit, 1.0);
        let down = Vec3::NEG_Y;
        let (l, r) = (o[0].1 * down, o[1].1 * down);
        assert!(l.x > 0.3 && r.x < -0.3, "the left hand swings left (+x), the right right: {l} {r}");
    }

    #[test]
    fn the_stride_bob_is_all_in_the_walk_offset_so_the_pilot_view_can_leave_it_out() {
        let g = GaitConfig::default();
        let mut s = MechState::standing(Vec3::ZERO, 0.0, &g);
        s.gait_swing = 1.0;
        let mut levels = Vec::new();
        for i in 0..20 {
            s.gait_cycles = i as f64 / 20.0;
            let p = pose(&s, &g, &rig(), &SitPose::default(), 0.0);
            levels.push(p.pelvis_offset - p.walk_offset);
            if i == 5 {
                assert!(
                    p.walk_offset.length() > 0.1 * g.bob_m,
                    "mid-stride the body bobs: {}",
                    p.walk_offset
                );
            }
        }
        assert!(
            levels.iter().all(|l| l.distance(levels[0]) < 1e-5),
            "without the walk offset the pelvis rides level: {levels:?}"
        );
    }

    /// Forward kinematics with the solved rotations, to check the solver against the bones.
    fn ankle_of(r: &LegRest, hip: Vec3, p: &LegPose) -> Vec3 {
        let knee = hip + p.thigh * (r.knee - r.hip);
        let hock = knee + p.shin * (r.hock - r.knee);
        hock + p.hock * (r.ankle - r.hock)
    }

    #[test]
    fn the_solver_returns_the_rest_pose_for_the_rest_target() {
        let r = rest();
        let p = solve_leg(&r, r.hip, r.ankle, 0.0);
        assert!(p.thigh.angle_between(Quat::IDENTITY) < 1e-3);
        assert!(p.shin.angle_between(Quat::IDENTITY) < 1e-3);
    }

    #[test]
    fn the_solver_reaches_a_reachable_target() {
        let r = rest();
        for target in [Vec3::new(1.6, 1.2, 2.0), Vec3::new(1.6, 0.5, -2.0), Vec3::new(1.6, 0.9, 0.5)] {
            let p = solve_leg(&r, r.hip, target, 0.0);
            let got = ankle_of(&r, r.hip, &p);
            assert!(got.distance(target) < 1e-3, "{got} vs {target}");
        }
    }

    #[test]
    fn the_knee_bends_the_way_the_rest_pose_bends() {
        let r = rest();
        let p = solve_leg(&r, r.hip + Vec3::new(0.0, -0.8, 0.0), r.ankle, 0.0);
        let knee = r.hip + Vec3::new(0.0, -0.8, 0.0) + p.thigh * (r.knee - r.hip);
        assert!(knee.z > r.hip.z, "the knee points forward, as at rest: {knee}");
    }

    #[test]
    fn the_feet_alternate() {
        let l = foot_offset(0.25, 1.0, 1.0);
        let r = foot_offset(0.75, 1.0, 1.0);
        assert_eq!(l.y, 0.0, "left is in stance");
        assert!(r.y > 0.9, "right is in swing");
    }

    #[test]
    fn feet_settle_together_at_rest() {
        let s = MechState::standing(Vec3::ZERO, 0.0, &GaitConfig::default());
        let pose = pose(&s, &GaitConfig::default(), &rig(), &SitPose::default(), 0.0);
        for (leg, r) in [(pose.left, rig().legs[0]), (pose.right, rig().legs[1])] {
            assert!(leg.ankle.distance(r.ankle) < 1e-3, "{}", leg.ankle);
        }
        assert_eq!(pose.pelvis_offset, Vec3::ZERO);
    }

    #[test]
    fn a_stance_foot_does_not_slide_at_steady_speed() {
        let cfg = crate::mech::MechConfig::default();
        let g = cfg.gait.clone();
        let mut s = MechState::standing(Vec3::ZERO, 0.0, &g);
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            s.step(&go, &cfg, &FlatGround::default(), dt);
        }
        let rig = rig();
        // World z of the left foot: body position plus the foot's model offset (model +z is
        // forward, world forward is -z at heading 0).
        let foot_world = |s: &MechState| s.pos.z - pose(s, &g, &rig, &SitPose::default(), 0.0).left.ankle.z;
        let mut slide: f32 = 0.0;
        let mut start = None;
        for _ in 0..240 {
            s.step(&go, &cfg, &FlatGround::default(), dt);
            let p = s.gait_cycles.rem_euclid(1.0);
            if (0.05..0.45).contains(&p) {
                let z = foot_world(&s);
                let z0 = *start.get_or_insert(z);
                slide = slide.max((z - z0).abs());
            } else {
                start = None;
            }
        }
        assert!(slide < 0.05 * s.stride_m, "stance foot slid {slide} m of a {} m stride", s.stride_m);
    }

    fn walking(cycles: f64) -> MechState {
        MechState {
            gait_cycles: cycles,
            gait_swing: 1.0,
            stride_m: 8.0,
            ..MechState::default()
        }
    }

    #[test]
    fn the_left_foot_strikes_at_each_whole_cycle_and_the_right_half_a_cycle_later() {
        assert_eq!(foot_strikes(&walking(0.98), &walking(1.02), false), [true, false]);
        assert_eq!(foot_strikes(&walking(1.48), &walking(1.52), false), [false, true]);
        assert_eq!(
            foot_strikes(&walking(1.10), &walking(1.20), false),
            [false, false],
            "mid-stance: none"
        );
    }

    #[test]
    fn a_settled_stride_or_a_lost_leg_strikes_nothing_but_a_landing_strikes_both() {
        let mut still = walking(1.02);
        still.gait_swing = 0.1;
        assert_eq!(foot_strikes(&walking(0.98), &still, false), [false, false], "shuffling in place");
        let mut limping = walking(1.02);
        limping.legs_lost = [true, false];
        assert_eq!(foot_strikes(&walking(0.98), &limping, false), [false, false]);
        assert_eq!(foot_strikes(&walking(1.1), &walking(1.2), true), [true, true]);
    }

    #[test]
    fn a_foot_comes_down_a_quarter_stride_ahead_of_its_rest_ankle() {
        let s = walking(1.0);
        assert!((stance_lead_m(&s) - 2.0).abs() < 1e-6);
        assert!(
            (foot_offset(1.0, stance_lead_m(&s), 1.0).z - 2.0).abs() < 1e-5,
            "where the swing ends"
        );
    }
}
