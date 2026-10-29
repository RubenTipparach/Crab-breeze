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

## 2b. Knockdown (survey G19)

A strike that lands on an enemy mech (any part) knocks it down:
- `Knockdown { t_s }` on the struck mech: falling 0.6 s, down 1.4 s, getting up 1.2 s (3.2 s in all,
  `melee.knockdown_*_s` in data).
- While knocked down it does not walk, turn or fire (its controls are zeroed, its guns' triggers
  held), and its AI's clock keeps running so it picks up where it left off.
- The pose: the whole body tips back about the heels to lying (pelvis down, torso back) and comes
  back up through a kneel, blended over the gait pose like the sit (mech-boarding), so one pose code
  path serves both.
- A mech already down is not knocked down again until it stands (no stun lock).
- Vehicles and crabs are not knocked down (they have no rig to fall with); they take the hit only.

## 3. The arm pose

While a strike runs, the arm's shoulder and elbow bones are posed over the gait pose:
- wind-up: drawn back;
- hit: thrown forward and up;
- recovery: eased back to the gait pose.

The pilot sees the arm swing past the screens: every mesh of the mech is on the cockpit layer while
seated.

### 3a. The moves, designed in the mech artifact (owner, 2026-10-29)

Owner, 2026-10-29: "I will need your help desigining melee animaitons in the mech artifact" and "shooting
guns should cause mech arm to recoil". The moves are designed on the Mech Melee Lab page
(https://claude.ai/artifact/2tnWFtFr33Q6vyAiCe6f4n, built by `tools/mockups/gen_melee_lab.py`; its own page
because the mech design page is about 190 MB: owner, "Just make a new one", "It might be too big"), where
they play on any of the eight detailed mechs against a copy of it that takes the hit, and where each key
can be posed and saved. The page's moves are the one source:
`tools/mockups/mech_melee.js` holds the defaults, the page's database (collection `melee`, one
document per move) holds the owner's edits, and the game's moves are written from those, never typed
again by hand.

A move is a list of keys. Each key has a time in seconds and, for each bone, an offset over the gait
pose: three turns in radians about the bone's own axes, and optionally three slides in metres. A key can
also set `crouch`, how far the legs fold (the boarding sit's leg fold, 0 standing to 1.25 sat). Between
keys the offsets ease (smoothstep). The first and last keys are rest, so a move starts and ends on the
gait. `hit_s` is when the blow lands.

| Move | Arm | Lands at | Length | What the pilot sees and feels |
|---|---|---|---|---|
| Jab | L or R | 0.45 s | 0.95 s | the arm cocks back 0.35 s, then the torso turns 0.3 rad and the arm shoots out level, with a 1.2 m lunge |
| Hook | L or R | 0.55 s | 1.15 s | the arm swings out wide and the torso winds 0.45 rad away, then sweeps across the chest; the target is knocked sideways |
| Uppercut | L or R | 0.55 s | 1.2 s | a deep crouch with the forearm upright, then the arm drives up past the screens as the legs straighten |
| Overhead smash | both | 0.85 s | 1.8 s | both arms rise overhead as the torso leans back, then come down together with a crouch and a 2 m surge |
| Shoulder charge | both | 0.7 s | 1.5 s | the torso turns 0.85 rad to lead with the left shoulder, arms tucked, and the mech drives 4.5 m forward |
| Stomp kick | L or R | 0.7 s | 1.45 s | one leg lifts and drives forward; the torso leans back to balance |
| Gun recoil | L or R | (each shot) | 0.32 s | the forearm jerks up 0.16 rad and the gun slides 0.7 m back along its length in 35 ms, settling in 0.3 s with one small overshoot |

The left-handed moves are made from the right-handed ones by mirroring (bone names swapped, turns and
slides across the body negated), never authored twice. Recoil is a move like the others, layered over
whatever the arm is doing, so a gun fired mid-walk or mid-swing still kicks.

The bones' axes, read off all eight rigs: every contract bone's x axis lies along the mech's left, so a
negative turn about x swings a bone's far end forward and up (the shoulder reaches the arm out ahead,
the forearm lifts the gun), a positive turn about y turns toward the mech's left, and a shoulder lifts
its arm out to the side with a negative z turn on the right and a positive one on the left.

**Found while designing:** the upper arm's mesh hangs on the `shoulder_*` bone, and `upper_arm_*` carries
no mesh (measured on the Foreman, the Sapper and the Mule). `cb_core::mech::arm_rotations` turns
`upper_arm_*`, so in the game a strike swings the forearm and gun about the shoulder while the upper arm's
plating stays where it was: the arm comes apart at the elbow. The moves above pose `shoulder_*` and
`forearm_*`, which keeps the arm whole. Task 2.4 below moves the game onto them.

Cost: a move is at most a few dozen numbers, sampled once per frame per mech that is striking: under a
microsecond of CPU, nothing on the GPU. Not measured (a cloud session has no GPU and this is not yet
game code).

## 4. Input

Owner, 2026-10-25, choosing "Guns keep firing in close combat, and punches move to other keys",
and "MELEE MODE on the main screen, not mfd". This replaces the mouse-button strikes below:
- In close combat the guns fire as always.
- `strike_left` (Z) and `strike_right` (V) swing the arms.
- `[MELEE MODE]` and the punch keys show under the crosshair.
- Tab with Alt held is ignored, so Alt+Tab no longer switches close combat on.

Earlier design (replaced):

- `close_combat_toggle` (Tab) is a latch, like `target_next`.
- In close combat, `fire[0]` and `fire[1]` start strikes and the guns hold.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G17 | What the triggers do in close combat | Arm strikes on the mouse buttons, guns hold | decided (survey G17) |
| G18 | Melee range | 80 m (owner, 2026-10-25: "range should be 80m instead"; was 30 m) | decided |
| G20 | Which moves the game plays, and on which keys | The jab on Z and V as now; the others once the owner has shaped them in the Melee lab | provisional |
| G21 | Does every gun shot kick the arm | Yes: each shot plays the recoil move on its arm, layered over the gait and any strike | provisional (owner, 2026-10-29: "shooting guns should cause mech arm to recoil") |
| G19 | What a strike does | Same damage rules as a shot (300 mm, 60), breaks cells and trees, and knocks an enemy mech down for about 3 s | decided (survey G19) |
