# Playtest: the city fight, 2026-10-24

`crab-breeze --showcase fight` (stand-in mech, seed 42, two enemy stand-ins from
`data/world/encounters.ron`), recorded with OBS between PLAYTEST_START and PLAYTEST_END, 2 min 23 s.
The scripted pilot writes only input actions (aim by mouse motion, fire by the fire actions).

- Machine: Apple M4 Max, release build, 1920 x 1080 window, vsync, 60 fps capture.
- Result: `PLAYTEST_RESULT ok fight: boarded, fought two mechs, brought down a tower`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-24-city-fight.mp4`, https://drive.google.com/file/d/1txvPYqD8xYsMGjcU7IYoZAYOyfszxn8H/view
- Contact sheet: `2026-10-24-city-fight_frames.png`; frame cost: `docs/benchmarks/2026-10-24-baseline`

| Beat | What it shows |
| --- | --- |
| Board | walk to the stand-in, Use once, it sits, climb in |
| Advance | stand up and walk down the street toward two rust-tinted enemy mechs |
| Fight | torso on the nearest enemy with lead; autocannon tracers and cannon shells; the first enemy loses its right arm, then its torso and blows up; the second goes the same way; their autocannon rounds spark off the player's torso armour |
| Tower | the ground floor of the tallest tower within 350 m shot out across its face; cells break, the storeys above lose their support and fall as Avian chunks |
| Rubble | stride toward the ruin through dust and smoke |

## Found and fixed while playtesting

- **The camera rode in an enemy's cockpit**: with several mechs, the seated camera took the first
  rig it found. Fixed: it takes the piloted mech's rig. The guns, which converge on what the pilot
  looks at, then fired backward.
- **The guns missed a mech they looked at**: the look ray ignored mechs, so the guns converged on
  the ground beyond. Fixed: mechs are in the look ray.
- **Armour wore down too fast**: a hit the armour stops did 10 % of its damage; two autocannons
  killed the player's torso in under a minute through 150 mm of armour. Now 2 %.

## What looked wrong

- Enemy friendly fire: the rear enemy shoots through the front one (it has no line-of-fire check).
- The tower holes but rarely collapses whole within the beat; the storey rule needs most of a
  storey gone, and only the near face is reachable.
- Chunks are plain boxes in the building's tone; no interior, no rebar, no glass.
- Rust tint reads pink at range.
