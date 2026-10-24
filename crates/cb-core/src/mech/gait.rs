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
    /// `torso_twist` rotation.
    pub torso_twist: Quat,
    /// `torso` rotation (twist then pitch).
    pub torso: Quat,
    /// Left leg.
    pub left: LegPose,
    /// Right leg.
    pub right: LegPose,
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
    let a = state.stride_m * 0.25 * swing;
    let lift = gait.step_height_m * swing;
    let p = phase.rem_euclid(1.0) as f32;
    let walk_offset = Vec3::new(
        gait.sway_m * swing * (TAU * p).sin(),
        -gait.bob_m * swing * 0.5 * (1.0 - (2.0 * TAU * p).cos()),
        0.0,
    );
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
        solve_leg(leg, leg.hip + pelvis_offset, leg.ankle + foot_offset(ph, a, lift), turn)
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
        torso_twist: twist,
        torso: twist * Quat::from_rotation_x(pitch),
        left: solve(&rest.legs[0], phase, lost[0]),
        right: solve(&rest.legs[1], phase + 0.5, lost[1]),
    }
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
}
