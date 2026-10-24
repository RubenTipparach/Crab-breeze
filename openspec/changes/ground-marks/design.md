# Design: ground-marks

## 1. What is ported, from where

Mining Mike, `godot-sandbox`, branch `origin/claude/long-haul-playtest`, commit 74e1412c:

| Mining Mike file | What Crab Breeze takes |
| --- | --- |
| `scripts/fx/ground_dents.gd` (GroundDents) | One batch for every dent; shapes FOOT 0, GROOVE 1, CRATER 2, TRAIL 3; each dent a quad 1.55 times its half size (a trail fills its length, so segments butt end to end); fade `min(1, 3 (1 - age / life))`, so a dent shows fully for two thirds of its life; past the cap the oldest goes first; `add_trail` laying a straight segment from a to b. |
| `shaders/ground_dent.gdshader` | The height field `H` (sole with tread bars, groove, ragged crater bowl with a lip, straight channel), the parallax march down it (28 steps), the normal off the field, the sun with a 10 step shadow march, the lit-over-flat ratio and the shape darkening; drawn as a multiply (`blend_mul`, unshaded, no depth write, no shadows). |
| `scenes/fx/ground_dents.tscn` | `render_priority -20` (drawn before anything see-through), no shadow cast, sun direction and ambient 0.7 as parameters. |
| `scripts/rivals/uber_race.gd` (`_lay_trails`, `_on_footfall`) | A wheel's line laid from where its last segment ended, every so many metres it glides, ending when the mech leaves its skates; a print on each footfall, none while skating. |
| `tools/shot_ground_dents.gd` | The test arrangement (a line of prints, grooves and two craters) for the screenshot pass the main session takes. |
| `scripts/fx/mech_trail.gd`, `resources/shaders/mech_trail.gdshader` | Read, not ported: the glowing dash ribbon is a light trail, not a ground mark. The skate trail here is the dent TRAIL. |

Crab Breeze adds: surfaces (soil, paving, none on water), cracks, scuff, dust and rubber streaks on
paving, the 5 by 5 ground-height grid per mark, the effects per surface and the pilot's jolt.

## 2. The surface rule

`WorldPlan::surface_at(p)` is `cover_at(p, Clearance::NONE)`: `Bare` is `Concrete`, `Open` and `Park`
are `Dirt`. With no clearance, `Bare` is exactly what is built: a highway's paved width (bridges carry
highways), a city's square outside its parks, a building's footprint. `ground_marks::mark_surface`
asks the one water query first: water gives no mark. One rule, shared with the trees and the grass
(CLAUDE.md 6.1).

## 3. When a mark is laid

- **A step**: `cb_core::mech::foot_strikes(prev, curr, landed)` is true for the left foot when the
  gait clock crosses a whole cycle and for the right half a cycle later, while the stride swings
  (`gait_swing >= 0.3`) and the leg is there; both on the step the mech lands from the air. This was
  inline in `mech.rs`; it moves into the core with tests and `mech.rs` calls it. The print's middle is
  the rest ankle plus `stance_lead_m` (a quarter stride times the swing: where the swing ends,
  `foot_offset(1.0) = +a`) plus `sole_ahead_m` (0.4 m, the stand-in's sole box) along the heading.
  The print is 1.6 m by 2.6 m (the stand-in's sole is 1.4 m by 2.6 m), its toe along the heading.
- **Skating**: each fixed step on the ground, each wheel reports where it is. `TrailLayer::feed` lays
  a segment from where the last ended each time the wheel has glided 4 m (at 40 m/s, 10 a second a
  wheel), starts a new line after a jump of 14 m, and forgets a wheel not reported for 0.3 s (off its
  skates, in the air). Trails are laid above 1 m/s; dirt and sparks fly above 5 m/s (as the grit did).
- **A crater**: each new blast in `Live::dug` gets a scorch of the crater's radius
  (`radius_m_per_cbrt_mj * cbrt(E)`), turned by its place.
- Only within 800 m of the pilot's eye; drawn out to 450 m, fading from 360 m.

## 4. Numbers per mark

| | Soil | Paving |
| --- | --- | --- |
| Footprint depth, lip | 0.22 m, 0.09 m | 0.03 m, 0 |
| Landing | 1.6 times deeper | 1.6 times deeper |
| Trail half width, depth, lip | 0.45 m, 0.12 m, 0.05 m | 0.45 m, 0, 0 (rubber streaks) |
| Crater scorch depth | 0.15 m | 0.05 m |
| Life | prints 90 s, trails 40 s, craters 240 s | same |

All of it is `data/fx/ground_marks.ron`, validated through `GameData` (`GroundMarksConfig`): unknown
keys refused, ranges checked, `trail_break_m` longer than `trail_segment_m`.

## 5. The draw

- One entity: `Mesh3d` of 1 024 slots times a 5 by 5 vertex grid (25 600 vertices, 32 768 triangles),
  each vertex only (slot, grid x, grid z); one `GroundMarksMaterial` (`AlphaMode::Multiply`, no
  prepass, no shadows, `NoFrustumCulling`); one storage buffer of 1 024 rows of 160 bytes (ten
  `vec4`: centre and yaw; quad half size, depth, lip; shape, surface, fade, seed; 25 ground heights).
  **One draw call.**
- The vertex shader puts each grid vertex on its sampled ground height (the CPU sampled it from the
  one height query, `Scene::height_at`, when the mark was laid), lifts it 4 cm and pulls it toward
  the eye by 0.2 % of its distance (at most 0.5 m), so a mark never shares the ground's depth (no
  z-fighting). A slot with fade 0, or past the draw distance, collapses to a clipped point.
- The fragment shader is Mining Mike's (section 1) with fewer parallax steps far away (28 near, 8 past
  120 m), then the surface: soil multiplies the pressed bottom toward dark brown and darkens it;
  paving scuffs the inside, draws nine wandering radial cracks of random reach and a crushed rim, and
  a ring of pale dust; a trail on paving draws rubber streaks and scuffs.
- The material sorts first in the transparent pass (a depth bias of -1e9 on the sort key), so smoke,
  dust and water rings draw over it: Mining Mike's `render_priority -20`.
- The CPU ages the ring and rewrites the buffer only on frames where any mark lives (or the frame
  after the last one dies). The fade is computed once, on the CPU (`ground_marks::fade`), and read by
  the shader.

## 6. Impact

- **Effects** (`data/fx/fx.ron`, bevy_hanabi through the `Fx` message): `foot_strike_dirt` (22 small
  and 8 large clods, 14 dust puffs), `foot_strike_concrete` (12 chips, 4 sparks, 9 pale dust puffs),
  `landing_dirt` and `landing_concrete` (a dust ring and shell, three times the debris),
  `skate_dirt` (3 clods and a puff a wheel a fixed step, thrown back and up), `skate_grit` on paving
  (as before). A step's density grows with speed, 1 to 2.5 times. `foot_dust` is replaced.
- **The pilot's jolt**: on the piloted mech's strikes while seated, the eye drops `step_cm + per_m_s_cm
  * speed` (1 cm + 0.2 cm per m/s, at most 4 cm) or 7 cm on a landing, shoved toward the striking foot
  by a quarter of that, over 0.15 s on the curve `sin(pi sqrt(k))`: deepest at 0.04 s, back to zero at
  0.15 s. Up to four overlap. `pilot::place_camera` adds the offset to the seated eye's POSITION
  through `ground_marks::jolted`, which keeps the rotation (tested). The cockpit pass's camera is the
  eye's child, so the cabin and the world jolt together relative to the seat.

## 7. Grass

`cb_core::grass::MAX_PUSHERS` goes from 8 to 16. The grass's pressing list is the bodies (the pilot's
feet, every mech's feet) and the six newest footprints at their middles with a 1.6 m radius, sorted
nearest the eye, the nearest 16 kept. A print's grass stands up again once newer prints or nearer
bodies push it out of the list: grass stays flat under a mech's recent steps, not for the print's
whole life. Persistent trampling (a per-tile texture) is survey `M10`.

## 8. Costs

| What | Cost |
| --- | --- |
| Draw calls | 1 for every mark |
| Vertices | 25 600 a frame, always (each reads one 160 byte row); under 0.02 ms on an RTX 3070, estimated |
| Fragments | Only where marks are drawn: about 45 field evaluations a pixel near (28 march, 4 normal, up to 10 shadow), 20 far. A print 10 m away covers about 15 000 px at 1080p; thirty prints in view at mixed distances about 100 000 px: about 0.05 ms estimated; a print filling a quarter of the screen up close about 0.25 ms |
| GPU memory | 160 KiB buffer, 25 600 vertices of 12 bytes (300 KiB), 98 304 indices (384 KiB) |
| CPU a frame | Age 1 024 slots and pack them (40 floats each) while any mark lives: about 20 microseconds; a 160 KiB upload |
| CPU a strike | One surface query, 26 height queries, one recipe |
| Grass | 8 more pressing spheres tested per blade vertex: estimated under 0.1 ms |

These are estimates: cloud and subagent sessions measure nothing (CLAUDE.md section 7). The benchmark
compares `CB_NO_GROUND_MARKS=1` against the default on the owner's machine.

## 9. Where the code lives

- `cb-core::ground_marks`: surface choice, marks, ring, fade, trail layer, jolt, config, mesh grid.
- `cb-core::world::cover`: `Surface`, `Clearance::NONE`, `WorldPlan::surface_at`.
- `cb-core::mech::gait`: `foot_strikes`, `stance_lead_m` (the pose uses it too).
- `cb-app::ground_marks`: the `GroundContact` message, the plugin, the material, `ViewJolt`,
  `PrintPress`, `jolted`.
- `assets/shaders/ground_marks.wesl`, compiled with naga in `cargo test`.
- Touched outside: `mech.rs` (calls `foot_strikes`, writes `GroundContact` instead of the dry-ground
  dust and the grit), `pilot.rs` (adds the jolt to the seated eye), `grass.rs` (reads `PrintPress`),
  `main.rs` (the plugin and the switch), `fx.rs`/`fx.ron` (recipes).

## Open questions

| # | Question | Provisional answer | Who decides |
| --- | --- | --- | --- |
| M9 | How long do footprints and trails stay? | Prints 90 s, trails 40 s, craters 240 s, 1 024 marks at most (survey `M9`). | owner (survey) |
| M10 | Should walked ground stay trampled for good (a per-tile texture)? | Not now; the ring fades. Grass is held flat by the six newest prints only (survey `M10`). | owner (survey) |
| M11 | How strong is the jolt in the seat? | 1 to 4 cm a step by speed, 7 cm on landing, 0.15 s (survey `M11`). | owner (survey), after a playtest |
| 4 | Do the soil tint, crack colour and dust brightness read right against the ground textures? | Set by reasoning, not by eye: tune in the screenshot pass. | playtest |
| 5 | Should the crater scorch follow the dug bowl more closely? | The 5 by 5 grid follows it coarsely; the pull toward the eye keeps it visible. Revisit after screenshots. | playtest |
