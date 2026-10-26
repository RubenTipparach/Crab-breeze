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
| G24 | Where the scope picture shows | A window in the middle of the front screen | **Decided**, owner 2026-10-26 ("Approve all"): a magnified window in the middle of the front screen, at 4x and 10x; the pilot's own eyes never zoom |

## Built, and the calls made on the way

Built 2026-10-26 on the owner's "Approve all" and "Go and implement"; "Use recommended for survey" for the choices on
the way:

- **The pilot's view is now 95 degrees seated** (`seated-fov`), not the 85 written in section 1, so the window's field
  of view is computed from the live view: 30.5 degrees at 4x and 12.5 at 10x (`cb_core::scope::fov_rad`; the 85
  degree figures are still tested).
- **Where the window sits**: where the torso's forward line from the eye meets the screen wall's glass
  (`CabinGlass`), 1 cm in front of it toward the eye, facing the eye, 30 degrees wide as seen from the seat (about
  0.42 of a front screen). A mech with no interior shows it 1.2 m ahead.
- **Bindings**: Scope is the middle mouse only (Z is the left punch, `close-combat`); the magnification steps on B,
  or on the wheel while Scope is held (the wheel steps the weapon otherwise). Merged with the Use list (survey W40,
  `option_cycle`, also on the wheel), the wheel has one job at a time: out of the mech's seat it moves the Use list
  only (`interact.rs` scrolls it only when not seated); seated it steps the weapon, or, while the window is open, the
  magnification (`scope.rs` applies B and the wheel only while open, so a wheel turn with the middle mouse held on
  foot changes nothing).
- **The look is scaled once, at the source** (`input::take_step_actions`), so the view and the torso's aim both turn
  one over the magnification as far.
- **The rangefinder** is `weapons::trace` against the world and the hostiles' bodies; the drop mark is
  `weapons::lead` with drop for the selected target, projected into the picture; the state line reads the weapon,
  the magnification, the rail's charge and the heat bank from `Guns`. `NO CAM` with the head lost, and the window
  closes. The charge is a number, not yet a ring.
- **Who has one**: the Picket's rail rifle and the hangar's rail rifle, `scope_zoom: [4.0, 10.0]`.
- **Not measured**: the third world pass's cost (task 2.1) needs the owner's machine; nothing was rendered here, as
  no playtest was allowed in this session.
