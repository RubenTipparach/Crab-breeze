# Proposal: pilot-boarding

## Why

The law (CLAUDE.md): first person always, and boarding is walked and climbed, with no cut and no
third-person camera. The pilot has to exist on foot, walk the street, reach the mech and ride up
into the cockpit in one continuous view before any playtest can show the game as it is played.

## What changes

- Input actions in `cb-app::input`, mapped from devices in `data/input/bindings.ron`; the core sees
  only an `Actions` value, which the scripted pilot (`playtest-pipeline`) will also write.
- `cb-core::pilot`: the pilot on foot (walk, run, mouse look, ground and building queries shared
  with the mech), and boarding: from within 3 m of `LADDER_base`, pressing Use carries the pilot's
  eyes up the lift to `HATCH_entry` and down into `EYE_pilot` along one continuous path; Use in the
  seat with the mech stopped climbs back out the same way.
- One camera, the pilot's eyes, in every state. In the seat it is `EYE_pilot`, moving with the
  mech's gait and torso; free look (hold Alt) turns the head within limits.
- A capture instrument `--stills <dir>` (CLAUDE.md section 4) that writes screenshots from the
  street and the cockpit and logs frame time, until `playtest-pipeline` replaces it.

## Out of scope

Climbing animation and the pilot's body and hands (`blender-humanoid-characters`), the cockpit's
controls and start-up sequence, and the scripted pilot itself (`playtest-pipeline`).
