# Design: mech-boarding

The reference is the Breeze Boarding mockup (`docs/mockups/cockpit.html`, built by
`tools/mockups/gen_cockpit_page.py` from `assets/models/cockpit_breeze.glb` and
`assets/models/mech_breeze.glb`). Numbers here are that mockup's first draft.

## 1. The sealed pod and the all-view screen

- The pod is the cockpit set (`docs/design/cockpit.md`) closed over: consoles, seat and floor as
  before; above the console line (0.96 m over the floor) the inner wall is the all-view screen, an
  ellipsoid of radii 0.77 x 0.78 x 0.86 m centred 0.96 m up and 0.08 m forward of the eye.
- In the game the screen is drawn from a cube render of the mech's sensor head (six faces, 1024 px
  each at first, rendered at the head's position each frame) sampled by direction from the eye,
  so the view is undistorted wherever the head turns. Tile seams every 20 degrees of azimuth and
  15 of elevation, scanlines, a phosphor grade, and the HUD drawn ahead.
- Cost: six extra scene renders a frame at the cube size. That is the largest cost in this design
  and is measured on the owner's hardware before it is accepted (CLAUDE.md 7); fallbacks are a
  lower cube size, fewer faces (the rear faces at a third of the rate) or one wide render.
- Power: the screen powers up tile by tile over 2.5 s with a flicker, and down in 0.6 s.

## 2. Sitting down

Breeze's legs fold (thigh forward, back knee down, hock flat), the pelvis lowers until it rests
1.4 m above the ground, the torso bows 20 degrees. 3.5 s in, 3.5 s out, eased. Steam from the
leg pistons as they compress.

## 3. The hatch sequence (opening; closing is the same in reverse)

| t (s) | Stage | Nodes | Motion | Vents |
| --- | --- | --- | --- | --- |
| 0.0 | Mech sits | legs, pelvis, torso | section 2 | leg pistons |
| 3.6 | Latches release | `LATCH_*` | retract 40 mm, amber lamps on | puff at each latch |
| 4.2 | Pressure equalises | | | ring of steam round the hatch seam, mist starts |
| 4.6 | Visor lifts | `HATCH_upper` | rotates up 80 degrees about its top hinge, two struts follow | hinge vents |
| 5.0 | Flaps swing out | `FLAP_L`, `FLAP_R` | rotate out 70 degrees about their outer edges | side seam bursts |
| 5.4 | Lower plate drops | `HATCH_lower` | rotates down 100 degrees about its bottom edge to a platform | |
| 5.8 | Shutters retract | `SHUTTER_1..4` | slide up into the roof one after another, 0.15 s apart | mist pours out |
| 6.6 | Stair unfolds | `STEP_1..3` | unfold from under the platform to the ground | |
| 7.2 | Console clears | `CONSOLE_front` | swings up 55 degrees about its top front | |
| 8.0 | Ready | | interior lamps up | mist keeps rolling for 6 s |

Vents are `VENT_*` empties on the nodes, each pointing its burst along its own +z; data gives
each burst's start, duration, rate and speed. Mist is a heavier, slower particle that falls and
spreads on what it meets.

## 4. The pilot's path

With the stair down, Use within 3 m of `LADDER_base` (now the stair's foot) walks the pilot's eyes
up the stair, across the platform, through the opening, turns them to face out of the seat and
lowers them to `EYE_pilot`: about 6 s, look kept free until the last second. The sequence then
closes (section 3 in reverse, 6 s), and the screens power up. Disembarking: screens down, open,
the reverse path, the mech may stand when the pilot is clear (3 m).

## 5. Cost

A boarding step and the timeline a fixed step: microseconds. Particles: up to 1500 billboards
while it plays, one draw call. The all-view screen: section 1.

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C1 | How the pilot sees out | Decided by the owner, 2026-10-24: sealed, all-view screens |
| C6 | How the pilot reaches a hatch 3 m up | A stair that unfolds from the lower hatch plate (survey C6) |
| C7 | All-view screen: seams and tint, or as clear as glass | Visible tile seams and a light phosphor tint (survey C7) |
