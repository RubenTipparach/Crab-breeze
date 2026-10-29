//! Arm aim (openspec change `arm-aim`): each gun arm turns its shoulder (pitch and yaw) and its elbow (pitch)
//! over the pose beneath so its barrel, the muzzle socket's +z, points at the aim point. Damped least squares on
//! the barrel's error, a few iterations each fixed step from the last answer, each joint held inside its limits
//! and pulled lightly toward rest; the arm then moves toward that answer no faster than its slew. What the joints
//! cannot reach is left as error: the shot still flies from the muzzle to the aim point. Pure, in model space;
//! the reference is the Mech Arms Lab's solver, `tools/mockups/arm_aim.js`.

use super::rig::{BONES, RestFrames, bone_index, in_rest_axes};
use crate::data::{Validate, finite, positive, within};
use glam::{Quat, Vec3};
use serde::Deserialize;

/// Arm aim in a mech's data (`arm_aim:`). Joint angles are offsets from the pose beneath, radians; the
/// shoulder's yaw limits are the right arm's, mirrored on the left.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ArmAimConfig {
    /// Shoulder pitch (about `shoulder_*`'s x), [low, high]: below zero raises the arm forward and up.
    pub shoulder_pitch_limits_rad: [f32; 2],
    /// Shoulder pitch slew.
    pub shoulder_pitch_slew_rad_s: f32,
    /// Shoulder yaw (about `shoulder_*`'s y) on the right arm, [low, high]: above zero swings it in.
    pub shoulder_yaw_limits_rad: [f32; 2],
    /// Shoulder yaw slew.
    pub shoulder_yaw_slew_rad_s: f32,
    /// Elbow pitch (about `forearm_*`'s x), [low, high]: below zero lifts the gun.
    pub elbow_pitch_limits_rad: [f32; 2],
    /// Elbow pitch slew.
    pub elbow_pitch_slew_rad_s: f32,
    /// Damped least squares iterations each fixed step.
    pub iterations: u32,
    /// The solve's damping (lambda).
    pub damping: f32,
    /// How strongly a bend the aim does not need is pulled back to rest (rho).
    pub rest_pull: f32,
    /// A striking arm lets go of its aim over this long.
    pub release_s: f32,
    /// After the strike, the arm takes its aim back over this long.
    pub retake_s: f32,
    /// An arm whose muzzle points further than this off the mech's forward at rest is a launcher, not aimed.
    pub gun_cone_deg: f32,
}

impl Default for ArmAimConfig {
    fn default() -> Self {
        // Measured on the Mech Arms Lab page (arm-aim design section 1).
        Self {
            shoulder_pitch_limits_rad: [-1.6, 0.7],
            shoulder_pitch_slew_rad_s: 1.6,
            shoulder_yaw_limits_rad: [-0.45, 0.6],
            shoulder_yaw_slew_rad_s: 1.6,
            elbow_pitch_limits_rad: [-1.1, 0.9],
            elbow_pitch_slew_rad_s: 2.4,
            iterations: 4,
            damping: 0.02,
            rest_pull: 0.02,
            release_s: 0.1,
            retake_s: 0.3,
            gun_cone_deg: 60.0,
        }
    }
}

impl Validate for ArmAimConfig {
    fn validate(&self) -> Result<(), String> {
        for (k, [lo, hi]) in [
            ("arm_aim.shoulder_pitch_limits_rad", self.shoulder_pitch_limits_rad),
            ("arm_aim.shoulder_yaw_limits_rad", self.shoulder_yaw_limits_rad),
            ("arm_aim.elbow_pitch_limits_rad", self.elbow_pitch_limits_rad),
        ] {
            within(&format!("{k}[0]"), lo, -std::f32::consts::PI, 0.0)?;
            within(&format!("{k}[1]"), hi, 0.0, std::f32::consts::PI)?;
        }
        for (k, v) in [
            ("arm_aim.shoulder_pitch_slew_rad_s", self.shoulder_pitch_slew_rad_s),
            ("arm_aim.shoulder_yaw_slew_rad_s", self.shoulder_yaw_slew_rad_s),
            ("arm_aim.elbow_pitch_slew_rad_s", self.elbow_pitch_slew_rad_s),
            ("arm_aim.damping", self.damping),
            ("arm_aim.release_s", self.release_s),
            ("arm_aim.retake_s", self.retake_s),
        ] {
            positive(k, v)?;
        }
        if !(1..=16).contains(&self.iterations) {
            return Err(format!("`arm_aim.iterations` must be 1 to 16, got {}", self.iterations));
        }
        finite("arm_aim.rest_pull", self.rest_pull)?;
        within("arm_aim.rest_pull", self.rest_pull, 0.0, 10.0)?;
        within("arm_aim.gun_cone_deg", self.gun_cone_deg, 0.0, 180.0)
    }
}

impl ArmAimConfig {
    /// The limits of arm `side` (0 left, 1 right), by joint: [shoulder pitch, shoulder yaw, elbow pitch].
    pub fn limits(&self, side: usize) -> [[f32; 2]; 3] {
        let [lo, hi] = self.shoulder_yaw_limits_rad;
        let yaw = if side == 0 { [-hi, -lo] } else { [lo, hi] };
        [self.shoulder_pitch_limits_rad, yaw, self.elbow_pitch_limits_rad]
    }

    /// Each joint's slew, rad/s, in the same order.
    pub fn slews(&self) -> [f32; 3] {
        [
            self.shoulder_pitch_slew_rad_s,
            self.shoulder_yaw_slew_rad_s,
            self.elbow_pitch_slew_rad_s,
        ]
    }
}

const SHOULDERS: [&str; 2] = ["shoulder_L", "shoulder_R"];
const FOREARMS: [&str; 2] = ["forearm_L", "forearm_R"];

/// The aim's turns for arm `side` (0 left, 1 right) at joint offsets `q_rad` [shoulder pitch, shoulder yaw, elbow
/// pitch], each in its bone's own rest axes, for [`super::rig::turned_deltas`].
pub fn turns(side: usize, q_rad: [f32; 3]) -> [(&'static str, Quat); 2] {
    let side = side.min(1);
    [
        (SHOULDERS[side], Quat::from_rotation_x(q_rad[0]) * Quat::from_rotation_y(q_rad[1])),
        (FOREARMS[side], Quat::from_rotation_x(q_rad[2])),
    ]
}

/// One arm as it stands under the pose beneath the aim: the chain shoulder, upper arm, forearm, hand and the
/// muzzle, in model space. Read off the rig's rest frames and the pose's rotations and origins (never typed in).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmFrame {
    side: usize,
    /// The shoulder's origin now.
    shoulder: Vec3,
    /// Each chain bone's rotation from rest under the pose beneath.
    delta: [Quat; 4],
    /// Each chain bone's rest rotation.
    rest_rot: [Quat; 4],
    /// Each chain bone's rest origin.
    rest_pos: [Vec3; 4],
    /// The muzzle's rest position and barrel direction.
    muzzle: (Vec3, Vec3),
}

impl ArmFrame {
    /// Arm `side` (0 left, 1 right) of a rig with rest `rest`, posed with rotations `delta` and origins `pos`
    /// (from [`super::rig::bone_deltas`] and [`super::rig::bone_positions`]).
    pub fn new(rest: &RestFrames, side: usize, delta: &[Quat; BONES.len()], pos: &[Vec3; BONES.len()]) -> Self {
        let side = side.min(1);
        let s = ["L", "R"][side];
        let chain = ["shoulder", "upper_arm", "forearm", "hand"].map(|b| bone_index(&format!("{b}_{s}")).unwrap_or(0));
        Self {
            side,
            shoulder: pos[chain[0]],
            delta: chain.map(|i| delta[i]),
            rest_rot: chain.map(|i| rest.rot[i]),
            rest_pos: chain.map(|i| rest.pos[i]),
            muzzle: rest.muzzles[side],
        }
    }

    /// A gun: its muzzle points within `gun_cone_deg` of the mech's forward (+z) at rest. Anything else (the
    /// Sapper's left pod, which fires up) is a launcher whose missiles steer themselves.
    pub fn is_gun(&self, cfg: &ArmAimConfig) -> bool {
        let dir = self.muzzle.1.normalize_or_zero();
        dir != Vec3::ZERO && dir.z > cfg.gun_cone_deg.to_radians().cos()
    }

    /// The barrel at joint offsets `q_rad`: the muzzle's position and its +z, model space. The same composition
    /// the drawing applies (`turned_deltas`, `bone_positions`).
    pub fn barrel(&self, q_rad: [f32; 3]) -> (Vec3, Vec3) {
        let [(_, ts), (_, tf)] = turns(self.side, q_rad);
        let turn = [ts, Quat::IDENTITY, tf, Quat::IDENTITY];
        let mut d = self.delta[0] * in_rest_axes(self.rest_rot[0], turn[0]);
        let mut p = self.shoulder;
        for (k, t) in turn.iter().enumerate().skip(1) {
            let carried = d * self.delta[k - 1].inverse();
            p += d * (self.rest_pos[k] - self.rest_pos[k - 1]);
            d = carried * self.delta[k] * in_rest_axes(self.rest_rot[k], *t);
        }
        (p + d * (self.muzzle.0 - self.rest_pos[3]), (d * self.muzzle.1).normalize_or_zero())
    }

    /// The barrel's error toward `target`: its direction minus the one from the muzzle to the target.
    fn residual(&self, q_rad: [f32; 3], target: Vec3) -> Vec3 {
        let (from, dir) = self.barrel(q_rad);
        dir - (target - from).normalize_or_zero()
    }

    /// The angle between the barrel at `q_rad` and the line from its muzzle to `target`, degrees.
    pub fn error_deg(&self, q_rad: [f32; 3], target: Vec3) -> f32 {
        let (from, dir) = self.barrel(q_rad);
        dir.angle_between(target - from).to_degrees()
    }

    /// The joint offsets that point the barrel at `target`, from `q0`: damped least squares, each joint clamped
    /// to its limits after every iteration.
    pub fn solve(&self, q0: [f32; 3], target: Vec3, cfg: &ArmAimConfig) -> [f32; 3] {
        const EPS: f32 = 1e-3;
        let lim = cfg.limits(self.side);
        let mut q = q0;
        for _ in 0..cfg.iterations {
            let r = self.residual(q, target);
            let jac = [0, 1, 2].map(|j| {
                let mut p = q;
                p[j] += EPS;
                (self.residual(p, target) - r) / EPS
            });
            // (J^T J + (lambda + rho) I) dq = -(J^T r + rho q)
            let mut a = [[0.0f32; 3]; 3];
            let mut g = [0.0f32; 3];
            for i in 0..3 {
                for k in 0..3 {
                    a[i][k] = jac[i].dot(jac[k]);
                }
                a[i][i] += cfg.damping + cfg.rest_pull;
                g[i] = -jac[i].dot(r) - cfg.rest_pull * q[i];
            }
            let dq = solve3(a, g);
            for j in 0..3 {
                let next = q[j] + dq[j];
                q[j] = if next.is_finite() { next.clamp(lim[j][0], lim[j][1]) } else { q[j] };
            }
        }
        q
    }
}

/// Solves a 3 x 3 system by Cramer's rule; a singular one moves nothing.
fn solve3(a: [[f32; 3]; 3], g: [f32; 3]) -> [f32; 3] {
    let det = |m: [[f32; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(a);
    if d.abs() < 1e-12 || !d.is_finite() {
        return [0.0; 3];
    }
    [0, 1, 2].map(|k| {
        let mut m = a;
        for (row, gi) in m.iter_mut().zip(g) {
            row[k] = gi;
        }
        det(m) / d
    })
}

/// One arm's aim: where its joints are and how much of the aim it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmAim {
    /// The joint offsets now, radians: [shoulder pitch, shoulder yaw, elbow pitch].
    pub q_rad: [f32; 3],
    /// How much of the aim the arm takes, 0 (let go, as while it strikes) to 1.
    pub weight: f32,
    /// The angle between the barrel and the shot's line (muzzle to aim point), degrees; `None` with no aim
    /// point or for a launcher.
    pub err_deg: Option<f32>,
}

impl Default for ArmAim {
    fn default() -> Self {
        Self {
            q_rad: [0.0; 3],
            weight: 1.0,
            err_deg: None,
        }
    }
}

impl ArmAim {
    /// The offsets the arm is posed with: the joints, scaled by the weight eased.
    pub fn applied(&self) -> [f32; 3] {
        let w = self.weight.clamp(0.0, 1.0);
        let w = w * w * (3.0 - 2.0 * w);
        self.q_rad.map(|q| q * w)
    }

    /// The offsets drawn a fraction `t` of the way from step `a` to step `b`.
    pub fn drawn(a: &Self, b: &Self, t: f32) -> [f32; 3] {
        let (qa, qb) = (a.applied(), b.applied());
        [0, 1, 2].map(|j| qa[j] + (qb[j] - qa[j]) * t)
    }

    /// One fixed step of `dt_s`: the weight eases out while `release` (a strike, a lost arm) and back in after;
    /// the joints solve toward `target` (model space; `None` relaxes them to rest) and move no faster than
    /// their slews. A launcher arm is left at rest.
    pub fn step(&mut self, frame: &ArmFrame, target: Option<Vec3>, release: bool, dt_s: f32, cfg: &ArmAimConfig) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        if !frame.is_gun(cfg) {
            *self = Self::default();
            return;
        }
        self.weight = if release {
            self.weight - dt_s / cfg.release_s
        } else {
            self.weight + dt_s / cfg.retake_s
        }
        .clamp(0.0, 1.0);
        let target = target.filter(|t| t.is_finite());
        let want = match target {
            Some(t) if self.weight > 0.0 => frame.solve(self.q_rad, t, cfg),
            _ => [0.0; 3],
        };
        let slew = cfg.slews();
        for j in 0..3 {
            let m = slew[j] * dt_s;
            self.q_rad[j] += (want[j] - self.q_rad[j]).clamp(-m, m);
        }
        self.err_deg = target.map(|t| frame.error_deg(self.applied(), t));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mech::rig::{bone_deltas, bone_positions};
    use crate::mech::{MechConfig, NodeSpec};

    const DT: f32 = 1.0 / 60.0;

    /// The stand-in's rest frames from its data (data/mechs/placeholder.ron): each node's offset chained
    /// from its parent's, no node turned at rest.
    fn stand_in() -> RestFrames {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let cfg = MechConfig::load(&root, "mechs/placeholder.ron").unwrap_or_else(|e| panic!("{e}"));
        let at = |name: &str| -> Vec3 {
            let mut p = Vec3::ZERO;
            let mut n: Option<&NodeSpec> = cfg.nodes.iter().find(|x| x.name == name);
            while let Some(node) = n {
                p += Vec3::from_array(node.offset_m);
                n = cfg.nodes.iter().find(|x| x.name == node.parent);
            }
            p
        };
        RestFrames {
            pos: std::array::from_fn(|i| at(BONES[i].name)),
            rot: [Quat::IDENTITY; BONES.len()],
            muzzles: [(at("MUZZLE_L"), Vec3::Z), (at("MUZZLE_R"), Vec3::Z)],
        }
    }

    fn frame(rest: &RestFrames, side: usize) -> ArmFrame {
        let d = bone_deltas(&[], &[], &rest.rot);
        let p = bone_positions(&d, &rest.pos, Vec3::ZERO);
        ArmFrame::new(rest, side, &d, &p)
    }

    fn run(arm: &mut ArmAim, f: &ArmFrame, target: Vec3, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            arm.step(f, Some(target), false, DT, &ArmAimConfig::default());
        }
    }

    #[test]
    fn at_rest_with_no_offsets_the_barrel_is_the_muzzle_socket() {
        let rest = stand_in();
        let (from, dir) = frame(&rest, 1).barrel([0.0; 3]);
        assert!(from.distance(rest.muzzles[1].0) < 1e-5 && dir.distance(Vec3::Z) < 1e-5);
    }

    #[test]
    fn a_gun_meets_a_reachable_aim_within_half_a_degree_after_it_has_slewed() {
        let rest = stand_in();
        for side in [0, 1] {
            let f = frame(&rest, side);
            // Close ahead, so the guns angle in; far ahead; up and to the side.
            for target in [Vec3::new(0.0, 9.0, 22.0), Vec3::new(0.0, 8.0, 400.0), Vec3::new(-15.0, 25.0, 60.0)] {
                let mut arm = ArmAim::default();
                run(&mut arm, &f, target, 3.0);
                let err = arm.err_deg.unwrap_or(f32::NAN);
                assert!(err < 0.5, "arm {side} toward {target}: {err} deg off");
            }
        }
    }

    #[test]
    fn both_guns_angle_in_toward_a_close_target() {
        let rest = stand_in();
        let target = Vec3::new(0.0, 9.0, 22.0);
        let (l, r) = (frame(&rest, 0), frame(&rest, 1));
        let (mut al, mut ar) = (ArmAim::default(), ArmAim::default());
        run(&mut al, &l, target, 3.0);
        run(&mut ar, &r, target, 3.0);
        let (dl, dr) = (l.barrel(al.applied()).1, r.barrel(ar.applied()).1);
        assert!(
            dl.x < -0.05 && dr.x > 0.05,
            "the left gun turns right (-x), the right left: {dl} {dr}"
        );
    }

    #[test]
    fn an_arm_stays_inside_its_limits_for_an_unreachable_aim() {
        let rest = stand_in();
        let cfg = ArmAimConfig::default();
        for side in [0, 1] {
            let f = frame(&rest, side);
            for target in [
                Vec3::new(0.0, 8.0, -300.0),
                Vec3::new(400.0, 8.0, 10.0),
                Vec3::new(0.0, -300.0, -100.0),
            ] {
                let mut arm = ArmAim::default();
                for _ in 0..240 {
                    arm.step(&f, Some(target), false, DT, &cfg);
                    for (j, [lo, hi]) in cfg.limits(side).into_iter().enumerate() {
                        let q = arm.q_rad[j];
                        assert!(
                            (lo - 1e-6..=hi + 1e-6).contains(&q),
                            "arm {side} joint {j} at {q}, limits {lo}..{hi}"
                        );
                    }
                }
                assert!(
                    arm.err_deg.is_some_and(|e| e > 5.0),
                    "out of reach, the error is left: {:?}",
                    arm.err_deg
                );
            }
        }
    }

    #[test]
    fn an_arm_moves_no_faster_than_its_slew() {
        let rest = stand_in();
        let cfg = ArmAimConfig::default();
        let f = frame(&rest, 1);
        let mut arm = ArmAim::default();
        let slew = cfg.slews();
        for target in [Vec3::new(-40.0, 40.0, 30.0), Vec3::new(20.0, -10.0, 30.0)] {
            for _ in 0..60 {
                let before = arm.q_rad;
                arm.step(&f, Some(target), false, DT, &cfg);
                for j in 0..3 {
                    let moved = (arm.q_rad[j] - before[j]).abs();
                    assert!(moved <= slew[j] * DT + 1e-6, "joint {j} moved {moved} rad in one step");
                }
            }
        }
    }

    #[test]
    fn a_launcher_arm_is_left_at_rest() {
        let mut rest = stand_in();
        rest.muzzles[0].1 = Vec3::new(0.0, 1.0, 0.2).normalize();
        let f = frame(&rest, 0);
        let cfg = ArmAimConfig::default();
        assert!(!f.is_gun(&cfg) && frame(&rest, 1).is_gun(&cfg));
        let mut arm = ArmAim::default();
        run(&mut arm, &f, Vec3::new(10.0, 30.0, 40.0), 1.0);
        assert_eq!(arm.applied(), [0.0; 3]);
        assert_eq!(arm.err_deg, None);
    }

    #[test]
    fn a_striking_arm_lets_go_of_its_aim_and_takes_it_back_after() {
        let rest = stand_in();
        let cfg = ArmAimConfig::default();
        let f = frame(&rest, 1);
        let target = Vec3::new(10.0, 30.0, 40.0);
        let mut arm = ArmAim::default();
        run(&mut arm, &f, target, 2.0);
        assert!(arm.applied().iter().any(|q| q.abs() > 0.05));
        for _ in 0..(cfg.release_s / DT).ceil() as usize + 1 {
            arm.step(&f, Some(target), true, DT, &cfg);
        }
        assert_eq!(arm.applied(), [0.0; 3], "let go within its release time");
        run(&mut arm, &f, target, 2.0);
        assert!(arm.err_deg.is_some_and(|e| e < 0.5), "aimed again: {:?}", arm.err_deg);
    }

    #[test]
    fn the_aims_turns_carry_a_strikes_forearm_with_the_shoulder() {
        let rest = stand_in();
        let own = crate::mech::arm_rotations(Quat::IDENTITY, [0.0, 1.0]);
        let base = bone_deltas(&own, &[], &rest.rot);
        let q = [-0.3, 0.2, 0.1];
        let turned = crate::mech::rig::turned_deltas(&base, &turns(1, q), &rest.rot);
        let pos = bone_positions(&turned, &rest.pos, Vec3::ZERO);
        let hand = bone_index("hand_R").unwrap_or(0);
        let want = pos[hand] + turned[hand] * (rest.muzzles[1].0 - rest.pos[hand]);
        let f = ArmFrame::new(&rest, 1, &base, &bone_positions(&base, &rest.pos, Vec3::ZERO));
        assert!(f.barrel(q).0.distance(want) < 1e-4, "the solve's arm is the drawn arm");
    }

    #[test]
    fn the_default_arm_aim_is_valid_and_a_bad_one_is_refused() {
        assert_eq!(ArmAimConfig::default().validate(), Ok(()));
        let bad = ArmAimConfig {
            shoulder_yaw_slew_rad_s: 0.0,
            ..ArmAimConfig::default()
        };
        assert!(bad.validate().is_err());
        let bad = ArmAimConfig {
            elbow_pitch_limits_rad: [0.2, 0.9],
            ..ArmAimConfig::default()
        };
        assert!(bad.validate().is_err(), "rest must lie inside the limits");
    }
}
