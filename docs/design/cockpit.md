# The cockpits

The owner, 2026-10-24: "Design the mech cockpit. Use detailed textures. Use the texturing skill we
had on star crew bridge sim. Baked lighting with AO. Beautiful panels, warning labels, buttons,
circuitry."

This is the cockpit you fight the whole game from (the law, CLAUDE.md). It is an art asset and a
review page today: nothing in the game drives it yet. The review page is the Crab Breeze Mechs page,
`docs/mockups/mech_designs.html` (published, link in `docs/design/README.md`): each chassis kneels and
opens its chest on its interior, and you can sit in each cockpit.

Later the same day the owner asked for "a couple variations of cockpit designs" and set where the
cockpit lives: "Follow design philosophy of boss mechs from mining mike. The cockpit is part of the
mech, inside of it with the armor opening up at the chest". So there are now three interiors (below),
each an interior only: the mech's chest armour (`tools/blender/build_mechs.py`) is the outside and
opens first, then the interior's own front opens and the pilot steps in.

## Three interiors

All three are built by one generator, `tools/blender/build_cockpit.py --variant <name>`, from the
same kit, panels, screens and bake, and promise the chest an envelope (the chest's cavity is that box
plus 0.1 m). Every screen showing the outside is the material `SCREEN_allview`; the HUD is on the
front screen, straight ahead of the eye in all three.

| Variant | The pilot | The screens | Envelope (x, y, z, metres) | Eye |
| --- | --- | --- | --- | --- |
| `wall` | seated, twin grips at the console ends | a belt of flat screens round the cabin: front, chamfers, sides, three overhead monitors | +-1.05, 0 to 2.1, -0.8 to 1.25 | 0, 1.20, 0 |
| `jockey` | rides forward on a saddle, chest on a pad, knees in pads, hands on handlebar grips | a low wide bank (a front screen and two wings) and a strip of screen overhead | +-0.85, 0 to 1.75, -1.0 to 1.3 | 0, 1.10, 0.30 |
| `harness` | stands in a harness against a backplate, forearms on two rests ending in grips, feet in stirrups | a tall octagon of flat facets all round (seven: the spine fills the rear), a ring of six monitors overhead | +-1.05, 0 to 2.4, -0.75 to 0.95 | 0, 1.65, 0 |

The floor plates sit 0.06 to 0.08 m under y = 0, inside the cavity's margin.

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
out of a mech to be a beautiful cinematic event." The mech sits and its chest armour opens with steam
(the mech's nodes and vents, `build_mechs.py`); then, in about 3 s, the interior opens its front, cold
mist rolls out over the floor (`VENT_inner_open_*`), and the pilot steps in through the front face.
Closing plays it backwards; the screens power up one by one; the mech stands. All of it from the
pilot's eyes. The timings in each manifest's `motions` are suggestions inside that 3 s.

| Variant | What opens (node: motion, start s, length s) | Doorway |
| --- | --- | --- |
| `wall` | `INNER_front` (front screen, header, ceiling front, overhead monitors, and the dash `CONSOLE_front` with its MFDs riding on it): turns -1.65 rad about x, 0.0, 1.6. `INNER_side_L/R` (the chamfer screens): +-0.8 rad about y, 0.5, 1.2. `INNER_sill` (the footwell's front wall): +1.45 rad about x down to a step, 1.6, 1.0 | 1.0 m wide between the front pillars, floor to 1.74 m |
| `jockey` | `INNER_front` (the screen bank, its frame and the roof front): -1.5 rad about x, up like a visor, 0.0, 1.4. `INNER_yoke` (chest pad and handlebars on a carriage): slides 0.45 m up its two columns, 0.9, 1.2. `CONSOLE_front` (the dash): +1.35 rad about x, down to a step, 1.5, 1.2 | 0.94 m between the yoke columns, 1.68 m at the face; the pilot ducks under the raised yoke (1.27 m) to the saddle |
| `harness` | `INNER_front` (the front facet): -1.5 rad about x, up and out, 0.0, 1.5. `INNER_side_L/R` (the front diagonal facets and the pillars between them): +-0.9 rad about y, 0.4, 1.2. `INNER_arm_L/R` (the forearm rests and grips): -1.3 rad about x, up, 1.0, 1.0. `CONSOLE_front` (the low front console): +1.3 rad about x, down to a step, 1.6, 1.2 | 1.5 m between the hinges (0.92 m between the side pods below 0.48 m), floor to roof (2.25 m) |

Nodes and sockets: `EYE_pilot`, `SEAT_pilot` (`STAND_pilot` in the harness), `HATCH_entry` at the
front face's sill, the grips `CTRL_stick` (right hand) and `CTRL_throttle` (left hand), the picture
screens `SCREEN_<name>` and `OVH_<n>`, and the `INNER_*` nodes above. Pivots, motions, and the bounds
closed and open are in `assets/models/cockpit_<variant>.json`. Open, the moving parts reach out of the
envelope (the front units lift above and out of the chest), so the chest armour has to be open first.

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
2. **The interior** (`tools/blender/build_cockpit.py --variant wall|jockey|harness`). The cabin, its
   consoles, seat or saddle or harness, controls and flat screens are built in metres around the
   eye, the panels laid on as real plates carrying their maps, with Mining Mike's boss-kit hardware
   inside (bolt rows, hoses with collars, hazard bands, glowing core lamps). Cycles then bakes the
   inside's materials into two atlases (4096 px for the game, 2048 px for review) (front: dash, consoles, grips; cabin: everything else inside), each as
   four maps: base colour, a tangent-space normal map (the panel relief and the bevelled edges),
   ORM (R occlusion: each panel's own occlusion times a short contact occlusion bake, denoised with
   OIDN; G roughness; B metal) and emission. No light is baked (owner, 2026-10-24: "Aim for that pbr
   triple a goodness that bevy provides"): the pod exports as lit PBR with its own real-time lamps
   (glTF `KHR_lights_punctual`): a cool wash from the front screens, warm floods, a rear lamp, a red
   footwell lamp. The wall cabin's plain outer shell uses the mech plating's maps. The screens export
   unlit (`KHR_materials_unlit`) for the game to draw live.

The display pictures are `tools/textures/gen_cockpit_displays.py` (PIL): pictures of what the
screens will show, not a texture imitating a material, so CLAUDE.md 10's Material Maker rule does
not apply. The panel textures are modelled and rendered, the Star Crew way the owner asked for, which
is the documented exception in CLAUDE.md 13.

## What it costs

Measured off the builds (`assets/models/cockpit_<variant>.json`, 2048 px review bake); frame time is
not measured in a cloud session (CLAUDE.md 7). Target: an RTX 3070 (owner, 2026-10-24: "don't worry
about 200k triangle count"); budget 600,000 triangles each.

| | `wall` | `jockey` | `harness` |
| --- | --- | --- | --- |
| Triangles | 27,420 | 13,084 | 16,626 |
| Draw calls (mesh materials) | 21 | 11 | 21 |
| Real-time lamps | 6 | 6 | 6 |
| Atlases | two, four maps each | two, four maps each | two, four maps each |
| On disk (glTF, bin, JPEG and PNG) | 10.1 MB | 4.5 MB | 5.1 MB |
| Bake on 4 CPU cores, 2048 px | about 12 min | about 14 min | about 13 min |

Texture memory per variant: eight maps, as BC7 with mips 8 x 5.3 MiB = 43 MiB at 2048 px, 171 MiB at
4096 px (the game's size).

## Owner review of the built three, 2026-10-24

What the owner found on the Mech Cockpits page, to be fixed in whichever layout is modelled in detail next:

- Harness (Stevedore): the combat view is bad, metal bars across it and no displays for health, walking
  direction, ammunition or enemies; the MFDs up top and the way it opens are good. Standing is out: every
  cockpit is seated.
- Wall (Foreman): "really good". The far right window has z-fighting at its corners and extra window frames;
  the second window from the right opens for no reason (the side windows should not open); the main HUD panel's
  UVs are stretched and it clips the bottom plate as it opens.
- Jockey (Ironhorse): the leaning pose "doesnt really make a lot of sense"; seated instead.
- All: no geometry passing through other geometry, opening and closing seamless and logical, the cabin airtight
  (underwater, the stratosphere), the windows closed shapes.

The answer so far is ten seated block-outs (`tools/blender/build_cockpit_blockouts.py`): an airtight CSG shell,
square screens cut into its inner wall, structure outside about 40 degrees, and live MFDs (status, nav, weapons,
radar). The clear view ahead is measured on each: 74 to 95 degrees between pillars for the flat fronts, 190 and 210
for the two round ones (their opening seams fall on the screens' own seams), all round for the sphere, against 42 to
58 for the built three.

Each one opens its own way (owner, 2026-10-24: "I want more elaborate door opening animations. Doors don't have to
be flat walls. Top of mech could open up like a fighter jet"). The moving parts are cut off the shell by booleans,
so they are whole pieces of the cabin, roof and bubble included, not flat plates. Every opening runs in stages: orange
clamps on the seams slide clear, the part pops off its seal with a puff of steam, then lifts, slides and tilts on
hydraulic rams. Every part only ever moves away from the body and the cabin, so nothing passes through anything. Each one can also
be entered by the back (owner, 2026-10-24: "Or back of mech can open"): the same cabin with a door in its back wall,
and the seat swivels round to face it once it is open, so the screens ahead never move and the front stays whole
armour. The page switches between the two with Entry: Front or Back. Where the pilot climbs out through the front, the dash rides the part
that opens, on brackets from its wall, and is sized to the cabin (owner, 2026-10-24: "cant climb out of this one",
"control panel is in the way, you had control panel attacked to the door earlier, do that again"):

| Block-out | How it opens | Or by the back |
| --- | --- | --- |
| Panorama | Clamshell: the top of the front swings up, the bottom drops to the deck as a step | Hatchback: the back wall and the back of the roof swing up on two rams |
| Tile wall | Petals: the middle three tiles swing up, the two beside them swing out like doors | The back wall drops back as a ramp |
| All-view sphere | A visor plate turns up over the ball about the eye, then the seat runs forward on its rail | A visor plate behind the ball turns up over the top |
| Titan | The top half rises past the roof line and tilts back; the bottom half drops as a step | The back wall drops back as a ramp |
| Cinema | A plug door pushes out 20 cm, then slides aside on rails | Hatchback on two rams |
| Excavator cab | The whole front drops forward as a ramp | The back wall drops back as a ramp |
| Octagon | Gull-wing side doors turn up about hinges on the roof | Hatchback on two rams |
| Canopy | A fighter's canopy: the bubble lifts, slides back and tilts up on two rams | Two doors swing out on hinges at the back corners |
| Twin tier | The upper ring and its bubble rise on four rams and tilt back; the lower front drops as a step | Two doors swing out on hinges at the back corners |
| Low bank | A fighter's canopy hinged at the nose: the bubble swings up and forward on two rams | The back wall drops back as a ramp |

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C1 | How the pilot sees out | Decided: sealed, screens (owner, 2026-10-24) |
| C3 | How the cabin is lit | Decided: live PBR light, baked occlusion (owner, 2026-10-24) |
| C4 | Control layout | Twin grips, throttle left, flight and torso right, as the references |
| C6 | How the pilot climbs to the hatch | A stair unfolding from the lower plate |
| C7 | How the screen wall looks | Separate screens in heavy bezels, phosphor tint, scanlines |
| C5 | Colours | Dark gunmetal, amber, green and red lamps, hazard stripes, as drawn |
| C8 | Which cockpit for which chassis | Provisional: chosen with the new chassis designs (build_mechs.py) |
| C9 | Which block-outs to model in detail | A few will be picked (owner, 2026-10-24: "we're gonna pick a few"). Liked so far: Twin tier ("cool"), Canopy, Low bank ("good too"), Excavator cab ("excellent, classic") |
