//! The procedural digitigrade walk (mech-rig design section 3): where the feet go in the gait
//! cycle, how the pelvis bobs and sways, and the leg solver that turns a foot target into bone
//! rotations. All in model space (glTF: +y up, +z forward, +x the mech's left), as rotations
//! relative to the rest pose, so any model on the contract is driven the same way.

use super::config::GaitConfig;
use super::locomotion::MechState;
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

/// Solves one digitigrade leg: the hock keeps its rest angle, thigh and shin bend as a two-bone
/// chain from `hip` to the hock joint, the knee bending the way it bends at rest. A target out of
/// reach is pulled in to 99.5 % of the chain. Works in the leg's (y, z) plane.
pub fn solve_leg(rest: &LegRest, hip: Vec3, ankle_target: Vec3) -> LegPose {
    let l1 = plane(rest.knee - rest.hip).length();
    let l2 = plane(rest.hock - rest.knee).length();
    let hock_vec = plane(rest.ankle - rest.hock);
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
        hock: Quat::IDENTITY,
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
/// twisted and pitched.
pub fn pose(state: &MechState, gait: &GaitConfig, legs: &[LegRest; 2]) -> MechPose {
    let phase = state.gait_cycles;
    let swing = state.gait_swing;
    let a = state.stride_m * 0.25 * swing;
    let lift = gait.step_height_m * swing;
    let p = phase.rem_euclid(1.0) as f32;
    let pelvis_offset = Vec3::new(
        gait.sway_m * swing * (TAU * p).sin(),
        -gait.bob_m * swing * 0.5 * (1.0 - (2.0 * TAU * p).cos()),
        0.0,
    );
    let solve = |rest: &LegRest, ph: f64| solve_leg(rest, rest.hip + pelvis_offset, rest.ankle + foot_offset(ph, a, lift));
    let twist = Quat::from_rotation_y(state.twist_rad);
    MechPose {
        pelvis_offset,
        torso_twist: twist,
        // Pitch up turns +z toward +y: a negative rotation about +x.
        torso: twist * Quat::from_rotation_x(-state.pitch_rad),
        left: solve(&legs[0], phase),
        right: solve(&legs[1], phase + 0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mech::config::LocomotionConfig;
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

    /// Forward kinematics with the solved rotations, to check the solver against the bones.
    fn ankle_of(r: &LegRest, hip: Vec3, p: &LegPose) -> Vec3 {
        let knee = hip + p.thigh * (r.knee - r.hip);
        let hock = knee + p.shin * (r.hock - r.knee);
        hock + p.hock * (r.ankle - r.hock)
    }

    #[test]
    fn the_solver_returns_the_rest_pose_for_the_rest_target() {
        let r = rest();
        let p = solve_leg(&r, r.hip, r.ankle);
        assert!(p.thigh.angle_between(Quat::IDENTITY) < 1e-3);
        assert!(p.shin.angle_between(Quat::IDENTITY) < 1e-3);
    }

    #[test]
    fn the_solver_reaches_a_reachable_target() {
        let r = rest();
        for target in [Vec3::new(1.6, 1.2, 2.0), Vec3::new(1.6, 0.5, -2.0), Vec3::new(1.6, 0.9, 0.5)] {
            let p = solve_leg(&r, r.hip, target);
            let got = ankle_of(&r, r.hip, &p);
            assert!(got.distance(target) < 1e-3, "{got} vs {target}");
        }
    }

    #[test]
    fn the_knee_bends_the_way_the_rest_pose_bends() {
        let r = rest();
        let p = solve_leg(&r, r.hip + Vec3::new(0.0, -0.8, 0.0), r.ankle);
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
        let legs = [rest(), rest()];
        let pose = pose(&s, &GaitConfig::default(), &legs);
        for leg in [pose.left, pose.right] {
            assert!(leg.ankle.distance(rest().ankle) < 1e-3, "{}", leg.ankle);
        }
        assert_eq!(pose.pelvis_offset, Vec3::ZERO);
    }

    #[test]
    fn a_stance_foot_does_not_slide_at_steady_speed() {
        let (l, g) = (LocomotionConfig::default(), GaitConfig::default());
        let mut s = MechState::standing(Vec3::ZERO, 0.0, &g);
        let go = MechControls {
            throttle_axis: 1.0,
            ..Default::default()
        };
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            s.step(&go, &l, &g, &FlatGround::default(), dt);
        }
        let legs = [rest(), rest()];
        // World z of the left foot: body position plus the foot's model offset (model +z is
        // forward, world forward is -z at heading 0).
        let foot_world = |s: &MechState| s.pos.z - pose(s, &g, &legs).left.ankle.z;
        let mut slide: f32 = 0.0;
        let mut start = None;
        for _ in 0..240 {
            s.step(&go, &l, &g, &FlatGround::default(), dt);
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
