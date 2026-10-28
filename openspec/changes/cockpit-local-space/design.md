# Design: cockpit-local-space

## 1. The cockpit pass in the mech's frame

Today (`pilot.rs`): the cockpit camera (`CockpitCamera`, render layer `COCKPIT_LAYER`, order 1, no clear) is a child of
the eye camera; `mech::cockpit_layer` puts the cabin's meshes (the mech's descendants that are its interior) on that
layer while the pilot is seated with the hatch shut, and its lamps are on it from the start.

Change:

- **Stand-ins.** While the cabin is on the cockpit layer, each of its mesh and lamp entities has a stand-in: a root
  entity (no parent) on `COCKPIT_LAYER` with the same mesh and material (or a copy of the light), and the original
  goes to the unseen layer. Off the cockpit layer (hatch open, climbing out) the stand-ins go and the originals return.
- **Placement.** Every frame after `PresentSet::Pose`, each stand-in's `Transform` is the product of the local
  transforms from its original up to the mech's root, not including the root, the way `socket_steady` finds the eye
  (with the same stride steadying, so cabin and eye agree). No world transform is read.
- **The camera.** The cockpit camera leaves the eye camera's children and takes, each frame, the eye's seat position
  in the mech's frame (`socket_steady("EYE_pilot")`) and the view's rotation in that frame (the world view's rotation
  with the mech's drawn heading taken off). Its projection stays the eye's.
- **What it costs.** One stand-in per cabin mesh and lamp (tens), a transform product each per frame (microseconds),
  no new draw: the originals stop drawing as the stand-ins start. A part that does not move keeps its transform, so
  Bevy's change detection sends it to the GPU once; today every cabin part's world transform changes every frame the
  mech moves.

Order: placed after the pose and before transform propagation, in the same frame as the world camera, so the two
passes show one moment.

## 2. The seated view blended between steps

`place_camera`, seated: the yaw drawn is `prev.yaw_rad` to `curr.yaw_rad` blended by the step's overstep fraction (the
shorter way round), plus the mouse not yet stepped, instead of the mech's blended heading plus `curr.seated_rel_rad`.
`yaw_rad` is the view's own direction (`heading + rel`, kept by the pilot's step), so the blend meets it at each step
and glides between. Pitch as now.

## 3. Measured

`CB_VIEW_TRACE=1` (an instrument) logs, each frame, the drawn view yaw and the cockpit camera's yaw in the mech's frame.
A script reads the log: over a turning stretch, the frame-to-frame change of the view yaw should keep its sign and
vary smoothly; a saw-tooth shows as alternating large and small (or reversed) steps at the fixed rate.

Tests: the blend's rule beside it (a turning view at fractions of each step never runs backwards).

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| CL1 | Should the world pass also move to a floating origin (the world re-centred near the pilot)? | Not now: the world is far away, where a millimetre does not show; the cockpit is the near thing that does. |
