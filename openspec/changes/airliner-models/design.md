# Design: airliner-models

## 1. The three aircraft

Shapes from the NASA Dryden three-views in `docs/reference/airliner-blueprints/` (public domain; NASA says they are
not engineering-accurate, so sizes come from the published dimensions). Frame as the freighter's: glTF metres, +Y up,
nose toward -Z, +X the right wing, origin on the fuselage centre line half way along its length; the ground under
the gear is at `y = -rest_m`.

| Kind | After | Length x span x height | Fuselage width | Engines | Seats (rows x abreast) | Crew seats |
| --- | --- | --- | --- | --- | --- | --- |
| `propliner` | Douglas DC-3 | 19.7 x 29.0 x 5.2 m | 2.35 m | 2 radial, 3-blade props 3.5 m | 21 (7 x 2+1, one aisle) | 2 |
| `airliner` | Boeing 720 | 41.3 x 39.9 x 12.7 m | 3.76 m | 4 turbofans on pylons | 138 (23 x 3+3) | 2 + engineer |
| `widebody` | Lockheed L-1011 | 54.2 x 47.3 x 16.9 m | 5.97 m | 3 turbofans (2 wing, 1 tail with S-duct) | 306 (34 x 2+5+2, two aisles) | 2 + engineer |

The propliner is a taildragger (two main legs folding forward into the nacelles, a tail wheel that stays down); the
other two stand on a nose leg and main bogies that fold inward into the belly.

**Livery**: each kind's own 1980s scheme, no real airline: white crown, a coloured cheat line along the windows, a bare
metal or grey belly, the fin in the line's colour. The skin wears the modern plating set (`plate`, `plateB`: base,
normal, ORM) tinted per region; engines and gear `frameM`; glass, nav lights and landing lights their own materials;
the cabin a fabric seat material, a carpet floor and a light plastic trim. Every texture is the full PBR set
(CLAUDE.md section 10). No light baking: the windows are small and the cabin is lit live like every other interior.

## 2. The rig (moving parts as data)

Every moving part is an empty pivot node on its hinge with the mesh under it, named `RIG_<what>_<n>`. The manifest's
`rig` lists each with the axis (in the glTF frame), the angle at the far end of its travel, and its time:

| Part | Travel | Time |
| --- | --- | --- |
| `RIG_gear_nose`, `RIG_gear_main_l/r` | folds into its well, about 90-100 degrees | 8 s (doors open first 1.5 s, legs 5 s, doors shut 1.5 s) |
| `RIG_door_<well>_<n>` | opens 80-90 degrees on its outer edge | in the gear sequence |
| `RIG_prop_l/r` | spins on the shaft | 1 200 rpm at cruise (20 rev/s), shown by the blades and a blur disc above 8 rev/s |
| `RIG_fan_<n>` | the fan face spins | 3 000 rpm |
| `RIG_aileron_l/r`, `RIG_elevator_l/r`, `RIG_rudder` | +/- 20-25 degrees | as the flight banks and pitches |
| `RIG_flap_l/r` | 0 / 15 / 30 degrees | 10 s each way |

The gear sequence is one table, `gear(t)` for `t` from 0 (down and locked) to 1 (up and shut), read by both the page
and the game. In flight the game lowers the gear 250 m over the ground on approach and raises it 30 s after lift-off;
flaps follow the same phases.

## 3. Lights

| Light | Where | Colour | Pattern |
| --- | --- | --- | --- |
| Nav left / right | wing tips | red / green | steady |
| Nav tail | tail cone | white | steady |
| Beacon top / bottom | crown and belly at the wing | red | 1 flash a second, 0.1 s |
| Strobes | both wing tips (and tail on the jets) | white | double flash every 1.2 s, 0.05 s each, 0.15 s apart |
| Landing | wing roots (propliner: wing leading edge) | white | below 600 m over the ground, gear down |
| Logo | tailplane, lighting the fin (jets) | white | at night |

Each is an emissive mesh `LIGHT_<what>` with an empty `LAMP_<what>` at its centre. In the game only the landing lights
and the nearest aircraft's beacon cast light (night rule N3: only your own vehicle's lamps cast shadows); the rest are
emission and the night glow sprite (`night-voxel-light`).

## 4. Interiors

The fuselage is a shell above the floor and solid below it (owner, 2026-10-30: "botoom half of plane should be solid"):
the outer loft minus an inner loft 0.12 m inside it (0.08 m on the propliner) whose bottom is flat 4 cm under the floor's
top. Window holes cut by Exact booleans open into the cabin, and a broken section shows its wall, the cabin over the
floor and a solid lower half (the hold, the wing box and the wheel wells are not modelled inside). Each window is a hole with a pane
of glass 0.03 m in from the outer skin. Inside, by section:

- **Flight deck**: floor, two seats (three on the jets), the panel, the glare shield, the yoke columns.
- **Cabin**: floor at the window line minus 0.95 m, carpet; seat rows at a 0.81 m pitch (0.86 m on the propliner),
  each seat 12 faces at LOD0 (pan, back, arm rests); overhead bins on the jets; bulkheads at the doors; galleys
  fore and aft; the cabin's ceiling panels.
- **Cargo hold** under the floor on the jets: empty, dark.

The interior is drawn at LOD0 only, inside 250 m, and the seats of one section are one mesh (one draw). Seen through
the small windows it reads as rows of seats; seen through a break it is the whole cabin.

## 5. Destructible aircraft: the zones

The practice this follows (from shipped games that let you shoot vehicles apart: Battlefield, GTA, Ace Combat,
BeamNG-style part trees, and our own `structural-destruction`):

1. **Break where the structure breaks.** Zones are cut at joints a real airframe fails at: the wing root, the
   outer wing at the engine or mid span, the tail at the rear pressure bulkhead, the fuselage at a frame by a door.
   Never through an engine or a gear well.
2. **Each zone is its own mesh** under `ZONE_<name>`, with the zone's own LOD meshes, so detaching it is moving a node,
   not cutting a mesh at run time.
3. **Every cut is closed and dressed.** A cut through a solid (a wing, a fin) is capped with the `torn` material: dark
   scorched metal with a ragged normal map; a cut through the hollow fuselage shows the wall's thickness and the cabin.
   No zone, detached or not, shows a hole into nothing.
4. **A part tree.** Each zone names its parent; detaching a zone takes its children with it (the outer wing goes
   with the inner, the engine with its wing panel, the fin and tailplanes with the tail).
5. **Hit shapes per zone**, capsules measured off the zone's mesh by the build into the manifest, so a shot lands on the
   part it hits; one armour rule for all (`units::armour_hit`).
6. **Health per zone** as a share of the kind's `hp`; at zero a zone breaks: it detaches and flies on as debris with
   the aircraft's velocity and a spin, and the aircraft flies on without it if it can.
7. **What a lost zone does** to the flight is a rule, not a cut scene: an engine stops (thrust lost, fire and smoke
   trail from `FX_engine_<n>`), an outer wing roll plus spiral, the tail a pitch down, the fuselage broken in two
   both halves fall. Any zone on the main box (centre fuselage) at zero downs the aircraft.
8. **Damage shows before it breaks**: smoke at a zone below half its health, fire below a quarter (anchors `FX_*`).
9. **Debris are the real pieces, and a break throws more** (owner, 2026-10-30: "there should be debris coming out when
   pecies are shot"): a detached zone is its own mesh as a rigid body (`debris-physics`), never a box, trailing smoke
   and now and then fire as it falls. The break itself throws out a burst at the hit and at the joint: about 45 torn
   skin and frame fragments (0.1-0.7 m plates), seats and luggage when a cabin section opens, a flash of fire and a
   cloud of smoke. Near you the fragments are rigid bodies that bounce and settle; far away, a dust puff.
10. **Deterministic**: the zone tree, shares and capsules are data; the same hits break the same zones.

| Kind | Zones |
| --- | --- |
| `propliner` | nose (flight deck), centre (cabin over the wing), aft cabin, tail (cone, fin, tailplanes), wing inner l/r, wing outer l/r, engine 1-2 (each with its main gear) |
| `airliner` | nose, fwd cabin, centre (wing box), aft cabin, tail (fin, tailplanes), wing inner l/r, wing outer l/r, engine 1-4 |
| `widebody` | nose, fwd cabin, centre, aft cabin, tail (fin, tailplanes, centre engine), wing inner l/r, wing outer l/r, engine 1-3 |

Health shares (of the kind's `hp`): centre 30 %, each cabin section 15 %, nose 10 %, tail 12 %, each inner wing 14 %,
each outer wing 8 %, each engine 6 %. Provisional, retuned in play.

## 6. Levels of detail and cost

| Kind | LOD0 (within 250 m, interior) | LOD1 (to 1.5 km, zones, no interior) | LOD2 (beyond, one mesh) |
| --- | --- | --- | --- |
| `propliner` | budget 30 000 triangles | 6 000 | 2 000 |
| `airliner` | 60 000 | 10 000 | 2 500 |
| `widebody` | 90 000 | 14 000 | 3 000 |

Built (2026-10-29): propliner 10 121 / 3 684 / 1 530 triangles, airliner 20 426 / 5 608 / 2 378, wide-body
26 304 / 5 170 / 2 192, each well inside its budget. The build refuses a level over its budget and writes the real counts
into the manifest. Draws: about 12 materials at
LOD0 (one draw per zone per material: about 60-120 draws for one aircraft up close), 4 at LOD1, 2 at LOD2. Textures
are the shared plating set (already resident for the mechs and the freighter) plus a seat fabric and a torn-metal set,
1024 px each: about 8 MB with mips. A hit test is one capsule sweep per zone, 10-16 per aircraft, against today's 2.

Frame time is not measured here (cloud session, no GPU; CLAUDE.md section 7): it is to be measured on the 3070 with
three aircraft parked at the airport and one flying past the cockpit at LOD0.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| A1 | Do the propliner and the wide-body join the air traffic as new kinds, and does the airliner take the 720's size (41.3 m x 39.9 m, four jets) in place of the approved 40 x 35 m twin-jet stand-in? | Yes to both: the propliner on the short hops between the small bases, the wide-body on the longest route |
| A2 | Seats: the 1980s rule of thumb above (21, 138, 306), or fewer, roomier rows? | As above |
| A3 | Liveries: three fictional island airlines, or one airline in three sizes? | Three, each its own colour |
