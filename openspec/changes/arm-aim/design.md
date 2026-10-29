# Design: arm-aim

Designed and measured on the Mech Arms Lab page (https://claude.ai/artifact/2tnWFtFr33Q6vyAiCe6f4n,
`tools/mockups/gen_melee_lab.py`, the solver `tools/mockups/arm_aim.js`). That page has a target that can be
moved, the barrel's line and the shot's line drawn from each muzzle, and a readout of each gun's error.

## 1. The solve (`cb-core::mech::arm_aim`, pure)

Per gun arm, three joint offsets over the pose beneath (gait, kneel, strike):

| Joint | Bone, axis | Limits, rad | Slew, rad/s |
|---|---|---|---|
| Shoulder pitch | `shoulder_*` x (below zero raises the arm forward and up) | -1.6 to 0.7 | 1.6 |
| Shoulder yaw | `shoulder_*` y (right arm: above zero swings it in; mirrored on the left) | -0.45 to 0.6 | 1.6 |
| Elbow pitch | `forearm_*` x (below zero lifts the gun) | -1.1 to 0.9 | 2.4 |

The steps:
- **The barrel** is the muzzle socket's position and its +z axis. On every gun arm of the eight detailed
  mechs, that axis lies along the barrel (read off the glbs).
- **The error** is the barrel's direction minus the direction from the muzzle to the aim point: three numbers.
- **Each fixed step** takes four damped least squares iterations, starting from the last answer:
  - the error's Jacobian over the three joints, by finite differences on the arm's forward kinematics;
  - the step `(JᵀJ + λI + ρI) Δ = -(Jᵀr + ρq)`, with damping λ 0.02 and a pull to rest ρ 0.02, so a reachable
    aim is met with the least bend;
  - each joint is clamped to its limits.
- **The arm moves toward the answer** no faster than each joint's slew.
- **What is left** is the gun's error, in degrees. The shot still flies from the muzzle to the aim point
  (`crates/cb-app/src/mech.rs`); the error is how far its line leaves the barrel's.

Rust: a pure function `solve(rig, arm, pose, target, state, cfg, dt) -> (offsets, error_deg)` over the rig's rest
transforms (the forward kinematics the leg solver already uses), with no Bevy in it. The app applies the offsets
like the strike's arm rotations.

## 2. Measured in the lab

Measured in a browser (three.js, software WebGL), with the mech standing and idling; each figure is the error
after 4 s.

| Case | Error, the eight mechs' gun arms |
|---|---|
| Torso follows; targets at 22 m, 120 m and 400 m, 45 deg right at 80 m, 40 m up at 60 m, the ground at 30 m | 0.0 to 0.3 deg, all 15 gun arms, every case |
| Torso held still (Foreman), 15 / 30 deg right of it | 0.4 / 0.6 deg |
| Torso held still (Foreman), 45 deg right | 17 to 22 deg: past the shoulder yaw limit |
| Torso held still, 20 deg up / 15 deg down | 0.4 / 0.1 deg |
| A 60 deg jump with the torso following (Foreman, Sapper) | both guns within 1 deg after 0.43 to 0.48 s; the torso takes about 1.1 s |

- **The idle sway** is the 0.1 to 0.3 deg left over: the arms follow it at their slew.
- **The Sapper's left arm** is a launcher (its muzzle points up), so it is not aimed.

## 3. Layering

The order:
1. gait, kneel and hit;
2. the strike's arm pose;
3. the torso twist and pitch (as now);
4. arm aim, weighted per arm;
5. recoil.

- **A striking arm's aim weight** eases to 0 in about 0.1 s, then back to 1 over about 0.3 s after the strike.
- **Seated, the pilot sees the arms aim** through the screens: they are on the cockpit layer.

## 4. Cost

- **CPU:** per gun arm, 4 iterations of 4 forward-kinematics passes over 4 bones: about 64 small transform
  products and one 3 x 3 solve. Two arms on every mech in view at 60 Hz is well under a microsecond each in Rust.
  This is an estimate, not measured: a cloud session has no GPU, and there is no game code yet.
- **The lab page's JavaScript** takes 0.2 to 0.6 ms a frame for the whole mech. That includes the pose, the
  matrix updates and three.js's bookkeeping, so it is not a measure of the Rust.
- **GPU:** nothing.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| A1 | How fast the arms slew | Shoulder 1.6 rad/s (92 deg/s), elbow 2.4 rad/s, faster than the torso's 70 deg/s so the guns lead it | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
| A2 | How far the arms reach without the torso | Shoulder yaw -0.45 to 0.6 rad (about 26 deg out, 34 deg in) | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
| A3 | Do AI mechs aim their arms too | Yes, at their own aim point: the same solve | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
| A4 | Are the limits and slews per mech | Yes, in `arm_aim:` in `data/mechs/<id>.ron`, defaulting to the table above | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
