# The Breeze cockpit

The owner, 2026-10-24: "Design the mech cockpit. Use detailed textures. Use the texturing skill we
had on star crew bridge sim. Baked lighting with AO. Beautiful panels, warning labels, buttons,
circuitry."

This is the cockpit you fight the whole game from (the law, CLAUDE.md). It is an art asset and a
review page today: nothing in the game drives it yet. The review page is `docs/mockups/cockpit.html`
(published, link in `docs/design/README.md`): the boarding sequence and the cockpit from the seat.

## What the pilot sees

The owner, later on 2026-10-24: "The cockpit should be enclosed fully. We can view outside via
screens like in gundam", with references (Armored Core, Titanfall, the Gundam Senjou no Kizuna pods,
80s mecha anime cockpits): a big front screen with angled side screens, heavy dark pillars and
frames between them, small monitors hanging from the overhead, the HUD drawn on the screens, twin
grips at the ends of the consoles, cyan accent lights. The cockpit is a sealed armoured pod at the
front of the chest, and a boxy cabin: every screen is flat, set in a straight frame between straight
pillars (the owner, on a curved first draft: "That's not a mech cockpit", "The curved ones are just
fish eye pov"). Sitting in the seat, eye 1.20 m above the floor:

| Where | What | Size |
| --- | --- | --- |
| Ahead | The front screen, 100 degrees wide and 45 high, in a heavy bezel; the HUD on it (pitch ladder, heading, speed, height, mode, missile lock) | 1.2 x 0.6 m |
| Either side | An angled side screen each, 51 degrees wide; a smaller screen behind each shoulder | |
| Between them | Pillars 90 mm wide with cyan light down their inner edges, rivet rows | |
| Overhead | A frame ring at the screens' top, ribs, round vents, three hanging monitors (status, tactical, systems) | 0.24 x 0.18 m each |
| Below the screens | The dash, tilted back 18 degrees: three MFDs in bezels with push buttons, the caution and warning panel and master lights, reactor and jet fuel standby gauges, the mode row (walk, skate, jet...) | 1.56 x 0.44 m |
| On top of the dash | The glare shield and its floodlight | 1.56 x 0.16 m |
| Under the dash | The knee panel: four standby gauges and the emergency panel in hazard stripes, reactor scram, jet cutoff and jettison arms under red and yellow guards | 1.10 x 0.30 m |
| Left hand | The left console and the throttle grip in its quadrant (idle, walk, run, skate, then the jet gate), reactor pumps, comms keypad | 0.32 x 0.80 m |
| Right hand | The right console and the flight grip (it twists to turn the torso), master arm under a red guard, weapon select, countermeasures | 0.32 x 0.80 m |
| Between the knees | The yellow and black eject handle | |
| Beside the legs | Circuit breaker rows on the inside faces of the consoles | 0.84 x 0.40 m each |
| Walls under the screens | Inspection windows onto live circuit boards, the 880 V danger placard, cable runs | 0.78 x 0.26 m each |
| Behind the seat | The main breaker board, the reactor interface board, coolant lines and their warning, the canopy jettison handle, a lamp strip | 1.30 x 0.90 m |
| Underfoot | Diamond tread plate, the pedals, a hazard strip at the footwell | 1.30 x 1.30 m |

Every control is a placeholder for a simulated system: when the system exists, the control is wired
to the number it shows (CLAUDE.md 6.1).

## Getting in and out

The owner: "have the mech sit down on the ground so that pilots can enter/exit. The cockpit should
open up with intricate doors and flaps and puffs of steam and myst coming out. I want getting in and
out of a mech to be a beautiful cinematic event." The sequence, its timings and its nodes are
`openspec/changes/mech-boarding`; in short: the mech sits, the latches release with a puff each, a
ring of steam round the visor seam, the armoured visor lifts on two rams carrying the front screen,
the side screens swing out as doors, the corner flaps fold back, the lower plate drops into a step,
a three section stair unfolds to the ground, the front console swings clear, and cold mist rolls
out over the sill. The pilot climbs in and sits; it all closes in reverse, a last hiss seals it,
the screens power up one by one, and the mech stands. All of it from the pilot's eyes.

The pod's nodes (pivots in `assets/models/cockpit_breeze.json`; the model is
`assets/models/cockpit_breeze/cockpit_breeze.gltf`): `HATCH_upper`, `STRUT_L/R` with
`_rod` and `_tip`, `DOOR_L/R`, `FLAP_L/R`, `HATCH_lower`, `STAIR_1..3`, `CONSOLE_front` (with
`SCREEN_left/center/right` on it), `OVH_left/center/right`, `LATCH_1..6`, `CTRL_stick`,
`CTRL_throttle`, and `VENT_<stage>_<n>` empties (extras: `kind` steam or mist, `stage`, `rate`,
`dir`). Sockets: `EYE_pilot`, `SEAT_pilot`, `HATCH_entry`, `LADDER_base`.

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
2. **The pod** (`tools/blender/build_cockpit.py`). The tub, consoles, dash, seat, controls, the
   boxy cabin with its flat screens, the hatch and the armour are built in metres around the eye,
   the panels laid on as real plates carrying their maps. Cycles then bakes the inside's materials
   into two 4096 px atlases (front: dash, consoles, grips; cabin: everything else inside), each as
   four maps: base colour, a tangent-space normal map (the panel relief and the bevelled edges),
   ORM (R occlusion: each panel's own occlusion times a short contact occlusion bake, denoised with
   OIDN; G roughness; B metal) and emission. No light is baked (owner, 2026-10-24: "Aim for that pbr
   triple a goodness that bevy provides"): the pod exports as lit PBR with its own real-time lamps
   (glTF `KHR_lights_punctual`): a cool wash from the front screen, warm floods under the glare
   shield and along the sills, a lamp on the rear bulkhead, a red footwell lamp. The outside armour
   uses the mech plating's colour, normal and ORM maps. The screens export unlit
   (`KHR_materials_unlit`) for the game to draw live.

The display pictures are `tools/textures/gen_cockpit_displays.py` (PIL): pictures of what the
screens will show, not a texture imitating a material, so CLAUDE.md 10's Material Maker rule does
not apply. The panel textures are modelled and rendered, the Star Crew way the owner asked for, which
is the documented exception in CLAUDE.md 13.

## What it costs

Measured off the build (`assets/models/cockpit_breeze.json`); frame time is not measured in a
cloud session (CLAUDE.md 7).

| | |
| --- | --- |
| Target | an RTX 3070 (owner, 2026-10-24: "don't worry about 200k triangle count") |
| Triangles | see the manifest's `tris` (about 33,000), budget 600,000 |
| Draw calls | one per mesh material, about 70 (two atlases, the screens, the outside armour roles); merging is for the engine |
| Texture memory | two atlases of four 4096 x 4096 maps: 8 x 21 MiB = 171 MiB as BC7 with mips (683 MiB as RGBA8; JPEG on disk) |
| Lights | six real-time lamps, two of them casting shadows |
| Bake | about the time in the build log, on 4 CPU cores |

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C1 | How the pilot sees out | Decided: sealed, screens (owner, 2026-10-24) |
| C3 | How the cabin is lit | Decided: live PBR light, baked occlusion (owner, 2026-10-24) |
| C4 | Control layout | Twin grips, throttle left, flight and torso right, as the references |
| C6 | How the pilot climbs to the hatch | A stair unfolding from the lower plate |
| C7 | How the screen wall looks | Separate screens in heavy bezels, phosphor tint, scanlines |
| C5 | Colours | Dark gunmetal, amber, green and red lamps, hazard stripes, as drawn |
| M6 | Which chassis first | Breeze, so this is the Breeze cockpit |
