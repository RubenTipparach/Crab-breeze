# Proposal: mech-rig

## Why

The mech is the game, and the art and the engine are being built in parallel (the cloud session
models mechs in Blender while this Mac session builds the engine). `docs/design/mech-rig.md` is the
contract between them. The engine needs a stand-in mech on that exact skeleton now, walking and
twisting, so the city can be judged at mech scale from the cockpit, and so the first Blender mech
drops in with no code change.

## What changes

- `cb-core::mech`: the contract's bone and socket names as one list, the mech's locomotion state
  (throttle, ground speed, leg heading, torso twist and pitch), a procedural digitigrade walk
  (gait phase, foot paths, pelvis bob and sway) and a leg solver that turns foot targets into
  bone rotations, all pure functions of state, input, data and `dt_s`.
- A locomotion mode list with walking built and skating (survey `M4`) and jets (survey `M3`) as
  named modes that are refused for now, so adding them is adding a mode, not rewiring the mech.
- `data/mechs/placeholder.ron`: the stand-in's bones, sockets and boxes, and its tuning; `model:`
  names a glb instead (`models/mech_<name>.glb`), and the same driver moves it.
- `cb-app::mech`: spawns the mech, binds bones and sockets by name (the same code for boxes and for
  a glb), drives them each frame, and refuses a glb that is missing a contract bone, naming it.

## Out of scope

Skating and jet flight themselves (their own changes after `M3`, `M4`, `M5`), weapons, damage,
heat and power, the detailed cockpit (survey `C1`, `C2`), and foot placement on slopes.
