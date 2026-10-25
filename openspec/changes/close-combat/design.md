# Design: close-combat

All numbers live in the mech data (`melee:` in `data/mechs/*.ron`), with defaults in code.

## 1. The mode (`cb-core::mech::melee`, pure)

`CloseCombat { on, held_off }`, stepped each fixed step with:
- the distance from the pilot's eye to the targeted hostile, if one is targeted (an untargeted enemy
  never counts: owner, 2026-10-24, "only hostiles targeted and gets close does this auto melee");
- whether the toggle was pressed.

The rules:
- **Toggle while on:** off, and `held_off` until the target leaves melee range.
- **Toggle while off:** on (by hand, with no one near).
- **Target comes within range** (not held off): on.
- **The targeted hostile goes down or moves out of range:** off, however it was switched on (owner,
  2026-10-24: "melee is off if hostile is down, or moves away").
- **Switched on by hand with no target in range:** stays on until Tab, or until a target that came
  into range goes down or leaves.

## 2. Strikes

`Strike { arm, t_s }` runs three phases: wind-up 0.35 s, hit 0.15 s, recovery 0.4 s.
- During the hit, the arm sweeps a capsule from its shoulder to `reach_m` (9 m) ahead of the chest, at
  the torso's twist and pitch.
- Anything the capsule meets takes one hit:
  - a mech part or a vehicle: `Hit { penetration_mm: 300, damage: 60 }`, through `Damage::resolve`
    and `Unit::hit`;
  - a building: its cells are broken in a 3 m sphere at the fist;
  - a tree: knocked down.
- Each arm strikes on its own clock, so two strikes can overlap. A lost arm cannot strike.

## 3. The arm pose

While a strike runs, the arm's shoulder and elbow bones are posed over the gait pose:
- wind-up: drawn back;
- hit: thrown forward and up;
- recovery: eased back to the gait pose.

The pilot sees the arm swing past the screens: every mesh of the mech is on the cockpit layer while
seated.

## 4. Input

- `close_combat_toggle` (Tab) is a latch, like `target_next`.
- In close combat, `fire[0]` and `fire[1]` start strikes and the guns hold.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G17 | What the triggers do in close combat | Arm strikes on the mouse buttons, guns hold | provisional (survey G17) |
| G18 | Melee range | 80 m (owner, 2026-10-25: "range should be 80m instead"; was 30 m) | decided |
| G19 | What a strike does | Same damage rules as a shot (300 mm, 60), breaks cells and trees | provisional (survey G19) |
