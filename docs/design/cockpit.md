# The Breeze cockpit

The owner, 2026-10-24: "Design the mech cockpit. Use detailed textures. Use the texturing skill we
had on star crew bridge sim. Baked lighting with AO. Beautiful panels, warning labels, buttons,
circuitry."

This is the cockpit you fight the whole game from (the law, CLAUDE.md). It is an art asset and a
review page today: nothing in the game drives it yet. The review page is `docs/mockups/cockpit.html`
(published, link in `docs/design/README.md`).

## What the pilot sees

Sitting in the seat, eye 1.20 m above the tub floor, looking ahead through a bubble canopy:

| Where | What | Size |
| --- | --- | --- |
| Ahead, below the sightline | The dash, tilted back 18 degrees: three MFDs in bezels with real push buttons (damage and heat, the tactical radar, weapons and jet fuel), the caution and warning panel and the master caution and warning lights on the left, reactor and jet fuel standby gauges on the right, the mode row (walk, skate, jet, hover, nav, target...) | 1.56 x 0.44 m, top edge 13 degrees below the eye |
| On top of the dash | The glare shield, its floodlight lip, and the HUD projector with the combiner glass at eye level | 1.56 x 0.16 m |
| At eye level | The HUD: pitch ladder, heading, speed, height, mode, missile lock, Ace Combat style | 0.20 x 0.15 m combiner |
| Under the dash | The knee panel: four standby gauges and the emergency panel in hazard stripes, reactor scram, jet cutoff and jettison arms under red and yellow guards | 1.10 x 0.30 m |
| Left hand | The left console: throttle in its quadrant (idle, walk, run, skate, then the jet gate with its caution), reactor pumps and vent, comms keypad and radio | 0.32 x 0.80 m |
| Right hand | The right console: the twist grip that turns the torso (120 degree limit), the guarded master arm, weapon select, station lights, countermeasures | 0.32 x 0.80 m |
| Between the knees | The centre stick, and the yellow and black eject handle at the front of the seat | |
| Beside the legs | Circuit breaker rows on the inside faces of the consoles | 0.84 x 0.40 m each |
| Walls above the consoles | Inspection windows onto live circuit boards (traces, chips, capacitors), the 880 V danger placard, cable troughs to the canopy sill | 0.78 x 0.26 m each |
| Behind the seat | The main breaker board, the reactor interface board under a window, reactor coolant lines and their warning, the canopy jettison handle | 1.30 x 0.90 m |
| Underfoot | Diamond tread plate, the pedals, a hazard strip at the footwell | 1.30 x 1.30 m |
| Through the glass | The mech's nose and cheek armour with its hazard and livery stripes; the canopy's front and rear bows and sill ring | canopy 1.54 x 0.78 x 1.72 m |

Every control is a placeholder for a simulated system: when the system exists, the control is wired
to the number it shows (CLAUDE.md 6.1). The controls the pilot moves are their own nodes, pivots at
the origin: `CTRL_stick`, `CTRL_throttle`, `CTRL_twist`. The screens are `SCREEN_left`,
`SCREEN_center`, `SCREEN_right` and `HUD_combiner`, for the game to draw live. The canopy (glass
and bows) is the `canopy` node, hinged at the back, so boarding can open it. Sockets: `EYE_pilot`,
`SEAT_pilot`, `HATCH_entry` (docs/design/mech-rig.md).

## How it is made

Two stages, both committed generators, both run headless in Blender 4.4 (CLAUDE.md 10):

1. **The panels** (`tools/blender/cockpit_kit.py`, `cockpit_panels.py`, run by
   `bake_cockpit_panels.py`). Star Crew's panel-textures method: each console face is modelled out
   of real pieces (plates with seams, screws, square buttons with backlit caps and legends, bat
   toggles in hex nuts, hinged guards, knurled knobs with detents, dials with ticks and needles,
   keypads, riveted placards, hazard stripes, louvres, circuit boards with buses, traces, chips,
   capacitors and vias) and rendered face-on, orthographic, in Cycles. One render writes the
   albedo (with crevice grime, edge wear and scuffs in it), the normal map, occlusion, roughness
   and metalness, and the emission of the lit caps. 12 panels at 900 to 2000 px per metre.
   `tools/textures/cockpit_contact_sheet.py` makes the contact sheet; look at it before calling a
   bake done.
2. **The set** (`tools/blender/build_cockpit.py`). The tub, consoles, dash, seat, controls,
   canopy and nose are built in metres around the eye, the panels laid on as real plates carrying
   their maps. Cycles then bakes the cabin's light with occlusion into two atlases (front: dash,
   consoles and controls; cabin: everything else): sky through the canopy, a warm floodlight
   under the glare shield and under each canopy sill, the dome lamp, a red footwell lamp, the glow
   of the screens and buttons, three diffuse bounces. A short contact occlusion bake is multiplied
   in, the result is denoised (OIDN), tone mapped and written as JPEG. The baked surfaces export
   unlit (`KHR_materials_unlit`), so what the page shows is the bake.

The display pictures are `tools/textures/gen_cockpit_displays.py` (PIL): pictures of what the
screens will show, not a texture imitating a material, so CLAUDE.md 10's Material Maker rule does
not apply. The panel textures are modelled and baked, the Star Crew way the owner asked for, which
is the documented exception in CLAUDE.md 13.

## What it costs

Measured off the build (`assets/models/cockpit_breeze.json`); frame time is not measured in a
cloud session (CLAUDE.md 7).

| | |
| --- | --- |
| Triangles | see the manifest's `tris`, budget 200,000 |
| Draw calls | one per material: 2 baked atlases, 3 screens, the HUD, the glass |
| Texture memory | two 4096 x 4096 atlases, RGBA8 with mips: 2 x 85 MiB = 170 MiB on the GPU (JPEG on disk) |
| Bake | about the time in the build log, on 4 CPU cores |

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C3 | How the baked light changes in play | A lightmap under live light (sun, flashes, alarms) |
| C4 | Control layout | Centre stick, throttle left, twist grip right, as drawn |
| C5 | Colours | Dark gunmetal, amber, green and red lamps, hazard stripes, as drawn |
| M6 | Which chassis first | Breeze, so this is the Breeze cockpit |
