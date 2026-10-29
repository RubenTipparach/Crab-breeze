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

`Strike { arm, t_s }` runs three phases: wind-up 0.45 s, hit 0.1 s, recovery 0.4 s (the lab's jab, which the game plays: section 3a).
- During the hit, the arm sweeps a capsule from its shoulder to `reach_m` (9 m) ahead of the chest, at
  the torso's twist and pitch.
- Anything the capsule meets takes one hit:
  - a mech part or a vehicle: `Hit { penetration_mm: 300, damage: 60 }`, through `Damage::resolve`
    and `Unit::hit`;
  - a building: its cells are broken in a 3 m sphere at the fist;
  - a tree: knocked down.
- Each arm strikes on its own clock, so two strikes can overlap. A lost arm cannot strike.

## 2b. Stance and knockdown (survey G19; owner, 2026-10-29)

The owner, 2026-10-29:
- "when hit enough, the stance of enemy mech should break, (usually 3 times) and they fall over";
- then on the classes: "lets adjust 2,3,4 light mech, mid, heavy", "guns do not", and the player's mech too,
  "correct";
- then on the fall: "make the fall animation more dynamic", "move the mech back a bit, as if they have some
  horizontal velocity".

This replaces "a strike that lands knocks it down". Designed on the Mech Arms Lab page
(https://claude.ai/artifact/2tnWFtFr33Q6vyAiCe6f4n); the rules and the pose are `tools/mockups/mech_stance.js`, and
the game's are `cb_core::mech::stance`.

- **`Stance` on every mech with a rig:** its stance damage, the time since the last blow, and a `Knockdown`
  while it is down. The numbers are in `melee.stance_*` and `melee.knockdown_*` in data.
- **`stance_break_at` by class:**
  - light 2 blows: Picket, Ironhorse;
  - mid 3: Harrow, Roustabout, Driller;
  - heavy 4: Foreman, Sapper, Mule.

  The classes follow the builds, and mass where it differs (the Ironhorse is the lightest, the Foreman the
  heaviest). They are written into `data/mechs/<id>.ron` by `tools/mechs/gen_mech_data.py`.
- **What a blow adds:** each strike that lands on a mech adds its move's `stance`: 1 for the jab, hook, uppercut
  and kick, and 1.5 for the overhead smash and the shoulder charge. The game's one strike today is the jab, which
  adds 1.
- **Gun hits never wear the stance.**
- **Below `stance_break_at`,** a blow staggers the mech (the hit pose). It keeps walking and firing.
- **The blow that reaches it breaks the stance:**
  - the mech is driven back along the blow at `knockdown_speed_m_s` (8). The speed falls by
    `knockdown_drag_per_s` (1.4) while it falls, then by `knockdown_skid_m_s2` (9) along the ground;
  - this moves the simulation's position, so it slides about 4 m;
  - the fall takes 1.0 s:
    - for the first 0.35 s it stumbles a step: knees buckling, the torso whipped back and recoiling, the head
      snapping, the arms flung up and out;
    - then it goes over backwards about its heels, tipping to 1.45 rad with the angle growing as the square of
      time, rolling 0.22 rad toward the side it was struck from, and kicking one leg up;
  - it lands with two bounces, the arms slamming down after the body, a skid and a burst of dust;
  - it lies 1.6 s;
  - it gets up in 1.3 s: it rolls level, rises onto its knees through the boarding kneel (`sit`), and stands
    where it slid to.
- **While knocked down:**
  - it does not walk, turn or fire (its controls are zeroed, its triggers held);
  - its AI's clock keeps running.
- **No stun lock:** blows add nothing while it is down. It stands with its stance whole.
- **Mending:** untouched for `stance_recover_delay_s` (3), the damage drains at `stance_recover_per_s` (0.5).
- **The player's mech** falls by the same rule. The pilot stays in the seat; the cabin goes over with them and
  the screens show the sky. The view's position rides the cabin, and only the pilot turns the view, as everywhere.
- **Vehicles and crabs** take the hit only: they have no rig to fall with.

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

**In the game (tasks 2.4, 2.5):**
- `tools/mockups/export_melee_moves.mjs` writes the lab's moves to `data/combat/melee_moves.ron`, and
  `cb_core::mech::melee_moves` plays them as the page does.
- Z and V throw the jab. The strike clock (`melee.windup_s` 0.45, `hit_s` 0.1, `recover_s` 0.4) lands and ends
  with the shipped jab, and a test holds the two together.
- Every shot from an arm's gun plays `recoil_L` or `recoil_R` on that arm, over the aimed pose.
- The other moves are in the data but not on keys yet (open question G20).
- The lab's leg kick in the knockdown and the stomp kick's leg turns are not drawn. The game solves each leg bone
  on its own, so a turn on one thigh would part it from its shin.

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
| G20 | Which moves the game plays, and on which keys | The jab on Z and V as now; the others once the owner has shaped them in the Melee lab | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
| G21 | Does every gun shot kick the arm | Yes: each shot plays the recoil move on its arm, layered over the gait and any strike | decided (owner, 2026-10-29: "shooting guns should cause mech arm to recoil", "approve of that fight") |
| S1 | Blows to break a stance | Light 2, mid 3, heavy 4, per mech in data | decided (owner, 2026-10-29: "2,3,4 light mech, mid, heavy") |
| S2 | Do gun hits wear the stance | No | decided (owner: "guns do not") |
| S3 | Can the player's mech be knocked down | Yes, by the same rule, seen from the seat | decided (owner: "correct") |
| S4 | Which class each mech is | Light: Picket, Ironhorse; mid: Harrow, Roustabout, Driller; heavy: Foreman, Sapper, Mule | decided (owner, 2026-10-29: "all looks good, implement", then "approve of that fight") |
| G19 | What a strike does | Same damage rules as a shot (300 mm, 60), breaks cells and trees, and knocks an enemy mech down for about 3 s | decided (survey G19) |
