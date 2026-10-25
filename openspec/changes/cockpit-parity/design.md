# Design: cockpit-parity

All four detailed interiors come from one builder (`tools/blender/build_cockpit.py`, `build_layout`) reading the
block-out layouts (`tools/blender/build_cockpit_blockouts.py`, `LAYOUTS`). A layout's `mfd` says which MFD sets it
gets: `"top"` (three MFDs hung from the front screen's top bar) and `"dash"` (`dash_unit`: the dash panel, its three
MFDs `SCREEN_left/center/right`, the glare shield). The Panorama and the Titan ask for both; the Cab only for the top
set; the Canopy only for the dash. Parity is two layout edits and two builder parameters, no new geometry code.

Frame and units as the builder's: glTF, +y up, the pilot faces +z, metres, origin on the floor under the eye at
(0, 1.20, 0).

## 1. The Cab: a dash, and the seam out of the view

| | Now | After |
| --- | --- | --- |
| `mfd` | `("top",)` | `("top", "dash")` |
| `rows` | (0.30, 1.10), (1.13, 1.93) | (0.30, 0.94), (0.97, 1.93) |
| Seam rail between the rows | 1.085 to 1.145 m, 5.5 cm to 11.5 cm under the eye | 0.925 to 0.985 m, behind the glare shield |
| Dash | none | the Panorama's `dash_unit` at 0.745 scale (the Titan's), riding the ramp |

**Where the dash sits.** `dash_unit` puts the dash panel's centre at `rows[0][0] - 0.16`, which is right for a cabin
whose screens start at the dash (Panorama 0.79 m) and wrong for the Cab's floor-to-roof glass (0.14 m, under the
knees). A new layout key `dash_y_m` overrides it; the Cab's is 0.79 m, the Panorama's own height, so the dash, its
MFDs and the glare shield sit where the Foreman's do: the panel from 0.57 to 1.01 m, 0.30 m in from the front at
z = 0.50 m, the glare shield's top at 1.05 m reaching to z = 0.60 m.

**Why 0.955 m for the seam.** The eye at (1.20, 0) over the glare shield's far edge (1.052, 0.60) meets the front
screens' inner face (z = 0.77 m) at y = 1.20 - 0.148 x 0.77 / 0.60 = 1.01 m. The rail is 6 cm tall; centred at
0.955 m its top is at 0.985 m, 2.5 cm under that line, so from the seat it is hidden behind the shield. The upper
row then runs 0.97 to 1.93 m: 96 cm of unbroken front screen from 16.6 degrees under the eye line to 43.5 degrees
over it, where the Foreman's runs 0.95 to 1.80 m.

**The dash rides the ramp** (`dash=True` on the `RAMP` part), as the Titan's rides its lower half: when the front
drops forward 86 degrees about its foot, the dash lands face-up on the ramp and is the step out. Its farthest point
is 1.05 m from the hinge and the ramp's sweep is outward and down, so it passes through nothing; the open bounds in
the manifest grow by the dash's depth and are checked against the chest's doorway as before.

**What the dash covers.** Seen from the eye, the dash covers the lower row straight ahead (0.30 to 0.94 m, the
ground 5 to 25 m in front of the feet). The lower row's side screens and the floor screen between the pedals still
show the ground beside and under the mech, which is the Cab's point. The owner's rule of no frame within about 40
degrees of straight ahead now holds: nothing crosses the front screen between 16.6 degrees under and 43.5 degrees over
the eye line, across its whole 87 degree width.

**MFDs above.** Unchanged: three 0.30 m MFDs hung from the upper row's top bar, 16 cm in, as in the Panorama.

## 2. The Canopy: MFDs above the front screen

| | Now | After |
| --- | --- | --- |
| `mfd` | `("dash",)` | `("top", "dash")` |
| Hanging MFDs | none | three, 0.30 m, centres at 1.58 m, 22 cm in from the front screen |

The Canopy's one screen row runs 0.85 to 1.45 m and the bubble narrows from 1.57 m (in 0.20 m by 1.87 m, 0.42 m by
2.05 m). Hung below the bar as in the Panorama (centre at the bar's height less 2 cm) they would cover the front
screen from 1.28 m up, 5 to 15 degrees above the eye line, in the middle of the view. So the builder gains a second
key, `mfd_top_m: (rise, out)`, how far the hanging MFDs' centres move up from today's place (2 cm under the screen's top
edge) and their distance in from the screen (defaults 0 and 0.16 m, today's). For the Canopy: rise 0.15 m (centres at
1.58 m), out 0.22 m.

**Why those numbers.** The MFDs tilt down at the eye about 27 degrees, so a 0.34 m bezel's top edge leans 7.7 cm
toward the bubble and its bottom edge 7.7 cm toward the pilot. Two limits:

- **Under the bubble:** the top edge, at z = 0.65 m, must stay under the bubble's inner face there (1.80 m): centre
  below 1.65 m.
- **Over the screen:** the bottom edge, at z = 0.51 m, must stay above the line from the eye to the screen's top
  edge (17.8 degrees up): centre above 1.51 m.

1.58 m sits in the middle of that 14 cm window. They ride the canopy (the front screen's node), so they lift with it.

## 3. The same six screens everywhere

After this, every interior a roster mech names has `HUD_mfd_0_screen`, `HUD_mfd_1_screen`, `HUD_mfd_2_screen`,
`SCREEN_left_screen`, `SCREEN_center_screen` and `SCREEN_right_screen`, and `cb-app::mfd` binds all six live: the hung
three start on MECH, FLIGHT and MISSION, the dash's on MECH, TARGET and WEAPONS, and any of them steps through all
five pages when pressed (free look and click).

**The check** (`cb-tools validate`, beside the mech glb contract check): for every `data/mechs/*.ron` with an
`interior`, open the interior glTF it names and refuse it, with the file and the missing name, unless it has the six
screen nodes, `EYE_pilot`, `SEAT_pilot`, at least four `LIGHT_*` lamps and every `rides[].part` it is asked to ride.
It reads the real glTF, not a list typed in the test (CLAUDE.md 6.5).

## 4. The bake

Both rebuilt at the Panorama's settings: `--size 4096 --samples 16 --map-samples 4`, denoised, the same six lamps
(`LIGHT_screen`, `LIGHT_overhead`, `LIGHT_sill_L/R`, `LIGHT_rear`, `LIGHT_footwell`), two atlases (front, cabin) of
base colour, normal, ORM and emission. Cloud sessions run Blender as the `bpy` 4.4 module from PyPI when the Blender
mirror is closed to them; it is the same Blender and Cycles build.

## 5. Cost

| | Cab now | Cab after | Canopy now | Canopy after |
| --- | --- | --- | --- | --- |
| Triangles | 19 506 | about 29 500 (the dash unit is about 10 000) | 25 022 | about 25 100 (three bezels and quads) |
| Draw calls | 10 | 15 (the dash's three MFD screens, glass, bezels) | 10 | 13 |
| Texture memory | two 4096 px atlas sets, unchanged | unchanged | unchanged | unchanged |

Against the 600 000 triangle budget per interior, both are under 5 %. Six live screens cost one MFD texture
(2 560 x 512 px, 5 MB) that exists already; binding three more screens costs three materials. Cloud sessions cannot
measure frame time (CLAUDE.md 7); the owner's machine measures the seated view in the Cab and the Canopy against
the Foreman's.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C12 | The Cab's lower front screens are mostly behind the new dash. Keep them, or make the lower row sides only | Keep them: they light the footwell with the outside and show past the dash's ends | provisional |
