# Proposal: arm-aim

## Why

The owner (2026-10-29): "Currently for arms aim in the direction the player is looking in the cockpit. Do we
have ik for that?", then "Shot is ok to deviate from muzzle but we want muzzle to aim as close as possible to
shooting point. Write it up in an artifact first. Have a widget to move target point so I can see how mech ik
aims."

Today the arms have no aim of their own. They hang at their rest pose on the torso, and the torso turns toward
the pilot's look at its own rates (`cb-core` locomotion: twist 70 deg/s, pitch 40 deg/s). A shot leaves the
muzzle and flies to the aim point under the crosshair (`crates/cb-app/src/mech.rs`, the shot's `target`), so it
lands where the pilot looks. But it leaves at whatever angle the barrel happens to make with that line: wide
while the torso is still turning, and wide on any close target, where the guns, 9 to 13 m apart, should angle
in and do not.

## What changes

- **Arm aim, an inverse kinematics solve per gun arm:**
  - Each frame, the shoulder's pitch and yaw and the elbow's pitch turn so the barrel (the muzzle socket's +z
    axis) points at the aim point. The solve is the one on the Mech Arms Lab page
    (`tools/mockups/arm_aim.js`).
  - Each joint stays inside its limits and moves no faster than its slew rate.
  - What cannot be reached is left as error.
- **The shot does not change:**
  - It still flies from the muzzle to the aim point (owner: "Shot is ok to deviate from muzzle").
  - Arm aim only shrinks the angle between the barrel and that line, so what the pilot sees the gun do and
    where the round goes agree.
- **Arms lead, the torso follows:**
  - A fast aim change is met by the arms first (in the lab, within 1 degree 0.45 s after a 60 degree jump),
    and they unwind as the torso catches up.
  - The arms alone cover about 30 degrees either side of the torso.
- **Strikes and recoil:**
  - A striking arm lets go of its aim (weight eased to 0) and takes it back after the strike.
  - A gun's recoil plays over the aimed pose.
- **Launchers are not aimed:**
  - An arm whose muzzle does not point within 60 degrees of the mech's forward at rest (the Sapper's
    left-forearm missile pod fires up) keeps its pose.
  - Its missiles steer themselves.
- **The arms are posed on `shoulder_*` and `forearm_*`,** never `upper_arm_*`, which carries no mesh (found in
  the `close-combat` lab work).
