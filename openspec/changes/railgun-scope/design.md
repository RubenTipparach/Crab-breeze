# Design: railgun-scope

## 1. The window

A third 3D camera, `ScopeCamera`, renders the world layer (the same layers the pilot's eye sees
through the screens) into an 896 x 640 texture. It exists only while Scope is held. The texture is
shown on a quad in the cockpit pass on the front screen's centre, 0.42 of the screen's width,
framed with a bezel and the reticle, so it is part of the cabin and lit by nothing (unlit, like the
screens).

- **Where it looks**: from the head's `CAM_main` empty along the pilot's aim ray (the same ray the
  guns fire along: one implementation of "where am I aiming").
- **Field of view**: the pilot's view is 85 degrees tall (`data/pilot/pilot.ron`; Bevy's
  field of view is vertical), so 4 times is 25.8
  degrees and 10 times is 10.5 degrees (`2 atan(tan(42.5 deg) / zoom)`). A torso 6 m across at 2 km
  is about 2 px tall on the pilot's 1080 px view; in the window it is about 4 px of its 640 px height
  at 4 times and 10 px at 10 times, with the target box (`targeting`) drawn round it: enough to put the reticle on.
- **Mouse look while scoped**: input is scaled by `1 / zoom` (4 times: a quarter of the speed). The
  player still turns the view; nothing moves it for them (CLAUDE.md section 8). The seated look limits
  (`seated-look`) are unchanged.

## 2. The sight

Drawn into the window's overlay from simulation state each frame:

| Mark | Source |
| --- | --- |
| Centre reticle | the aim ray |
| Range, metres | the first hit of the aim ray, from the beam trace in `weapon-kinds` |
| Drop mark | `weapons::lead(muzzle, target, vel, 2400, drop: true)` for the selected target, projected into the window: where to put the reticle |
| Charge ring | the railgun's charge, 0 to 1.0 s |
| Heat bar | the weapon heat bank |
| `NO CAM` | the head is destroyed: the window shows static and closes |

## 3. Data

`WeaponSpec` gains `scope_zoom: Vec<f32>` (default empty: no scope), validated as finite and above
1. The bindings gain `scope` and `scope_zoom`. Holding Scope with no scoped weapon on board does
nothing.

## 4. Cost

One more 3D pass while scoped, at 896 x 640 (0.57 megapixels, 28 % of a 1080p frame) with a narrow
frustum, so it culls most of the city and draws fewer objects than the main view. Estimate: a third
of the main pass's GPU time, or less. It is not running when the scope is closed. Unmeasured: a
cloud session has no GPU (CLAUDE.md section 7). The owner's machine measures it with the bench's
`fight` scene scoped against unscoped before this ships. If it is too dear, render the window every
second frame.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G24 | Where the scope picture shows | A window in the middle of the front screen | provisional (survey G24) |
