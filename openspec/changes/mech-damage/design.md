# Design: mech-damage

## 1. Parts (data/mechs/<mech>.ron, `parts:`)

| Part | Bones cut on loss | Armour (mm RHA equivalent) | Structure (points) |
| --- | --- | --- | --- |
| head (the sensor head) | head bone, or `cockpit`'s camera mount on a mech without one | 60 | 40 |
| arm_L / arm_R | `shoulder_*` and below | 80 | 80 |
| leg_L / leg_R | `thigh_*` and below | 120 | 140 |
| torso | (destroyed) | 150 | 200 |

A hit carries penetration and damage; armour subtracts, the rest goes to structure. Pure
function `resolve_hit(parts, hit) -> (parts, Vec<PartLost>)` in `cb-core::damage`. Golden tests.

Note: the contract has no head bone yet. The all-view cameras sit on the sensor head; the rig
contract gains `head` (parent `torso`, with `CAM_main`) in the same commit as the art, and a mech
without it treats `cockpit` as its head.

## 2. Losing a leg: the limp

A gait mode `Limp(side)`: the good leg steps; the stump plants as a crutch (its remaining
segment's end as the foot) for a short stance; the pelvis tilts 12 degrees toward the stump and
drops 1.2 m; top speed `0.22 * walk_speed_max_m_s`; turn rate halved. Losing both legs: the mech
sits (the `mech-boarding` sit pose, as a fall) and can only twist and fire. The same leg solver;
the stump has a shorter chain.

## 3. Losing a part, in the game

The cut bones are reparented under a new Avian dynamic body at their current world transform
(mass from data), given the hit's impulse; the stump bone shows a damage cap mesh (art: `STUMP_*`
nodes, hidden until used). Sparks, smoke and fire particles at the cut for 20 s.

## 4. Cost

Hit resolution microseconds. A falling wreck: one or two Avian bodies. Particles: up to 1 k per
stump while burning.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M7 | Losing a leg | limp on the stump at under 25 % speed; both legs gone, it sits and fights | provisional (survey M7) |
| M8 | Head destroyed | screens fall back to torso backup cameras: lower resolution, narrower, static | provisional (survey M8) |

## As built (2026-10-24)

- `head` is an optional contract bone (`rig::OPTIONAL`); the stand-in has one, Breeze not yet.
- A lost part's bones become a tumbling wreck (`fx::Debris`, no Avian yet) and the stump burns for 25 s.
- Head lost: the view is graded as the backup cameras (desaturated, darker) until the all-view screen exists.
- Test keys F5-F8 (or the scripted pilot) shoot off left arm, right arm, head, left leg.
