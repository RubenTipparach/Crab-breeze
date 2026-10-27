# Design: cockpit-styles

## 1. Styles

`build_cockpit.STYLES`, applied after the cockpit's trim (`apply_style`), so a style can be set on any cockpit.

| Style | Walls (`paint`) | Trims (`paint_lt`) | Structure (`paint_dk`) | Seat (`fabric`) | Strips, lamps |
| --- | --- | --- | --- | --- | --- |
| wood | brown leather, pebbled grain | walnut, rings and fine grain, roughness 0.32 | dark brown | tan leather, quilted 90 mm | amber, warm |
| airliner | off-white plastic, stipple | light grey plastic | mid grey plastic | blue cloth weave | soft white, neutral |
| padded | charcoal leather quilted 120 mm, grain | brushed steel, metal 1 | near-black leather | oxblood leather quilted 70 mm | red-orange, warm |

## 2. Material features (`cockpit_kit.make_material`)

- `wood=(dark, light)`: a Rings wave in object space stretched 14 times across the piece's run, distorted by noise, plus
  fine noise; mixes the two tones; a 0.4 mm grain bump.
- `quilt=pitch_m`: on each face's own plane (weighted by the normal's components to the fourth power), the pillow
  sin(pi frac(u)) sin(pi frac(v)) on the diagonals of a pitch grid; height 12 mm in the bump; seams darkened.
- `grain`: Voronoi distance at scale 900 as a 0.3 mm bump (leather).
- `stipple`: noise at scale 700 as a 0.3 mm bump (moulded plastic).

All in object space, so the patterns bake into the atlases as the existing `weave` does: the albedo, normal and ORM
maps carry them; roughness and metal stay per role (the ORM's G and B).

## 3. Mockups

`--preview PATH` builds the cabin, skips the bake and the export, and renders `PATH_front.png` and `PATH_side.png` in
Cycles (48 samples, denoised, 1280 x 720) from `EYE_pilot` at the game's view angle. About 4 minutes a cockpit and style
on the cloud CPU.

## 4. Cost

No triangles. The maps stay 4096 px; the features only change what the bake writes. A style change costs one rebake
(about 50 minutes a cockpit).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CS1 | Which cockpits get which style | The table in section 6, matched to each mech's work (owner, 2026-10-27: "Match the proper cockpit type with existing mech cockpits") | provisional |

## 5. The mockup

`docs/mockups/cockpit_styles.html` (published; the owner asked for quick JS mockups, not renders): a three.js block-out
of the Iron Horse's cab at true scale with a style switcher, seat and orbit views. It also proposes the wall dressing
(owner: "the wall is kinda empoty, need electrical panels, vents, rivets, buttons") and the wire runs (owner: "tidy up
the wires they go no where", "place wires along the struts"): breaker panels with placards, louvred vents, backlit
button blocks and rivet rows on the lower side walls; the cable bundle from each side console down to the floor, along
it to the corner pillar, up the pillar in clips every 0.22 m, and along the roof beam into a junction box over the front
screen. The textures there are drawn in the page; the builder's are the material features above.

### 5.1 The owner's notes on the first mockup (2026-10-26) and what changed

- "wood trims should sill be metal based, wood is just accent": the wood style keeps painted steel walls and
  structure; the trims are a glossy lacquered burl walnut (after the owner's car photos: a Bentley's dash, a
  Cadillac's console) on the dash fascia, sills and console rails, with chrome edging, a cream leather seat and a cool
  blue ambient strip.
- "airline one is too white, it hurts my eyes" (with 777 flight deck photos): tan-grey lining, taupe trims, charcoal
  switch panels, a tan leather seat, warm dim light.
- "modern exacator cockpit ... very sleek, polycarbonates, fiberglass, matte black/greys": a fifth style, excavator:
  matte black and grey moulded panels, no rivets, a two-tone grey cloth seat, yellow grab handles on the pillars.
- "fire extingushers, first aid kits, signal flares, life preservers, just more crap on the walls": on the solid rear
  quarters of the side walls, the rear bulkhead and the roof: extinguishers in brackets, a first aid kit, a signal
  flare case, a life vest pouch, a life ring, an oxygen mask box, circuit breaker panels, a torch in clips, grab
  handles, an overhead switch panel, gaspers, a dome light, an avionics rack and a stowage bin.
- "NOT ON THE WINDOWS ... cant put a fire hydrant or circuit breaker obstructing my view": nothing is mounted over a
  screen; the kit sits only on solid wall, under the sill, on the bulkhead and roof, and on the pillars (handles and
  wires). The builder's version keeps the same rule: dressing is placed only on faces that are not screen wall.

## 6. Which cockpit wears which style

The owner, 2026-10-27: "go ahead and build detailed versions of the cockpit. Match the proper cockpit type with existing
mech cockpits". Each mech's work picks its room; the Foreman stays as it is (owner: "dont change fromean").

| Mech | Cockpit | Style | Why |
| --- | --- | --- | --- |
| Foreman | Panorama | steel | the gold standard, unchanged |
| Sapper | Twin tier | steel | a siege gun's military room; keeps its red night trim |
| Iron Horse | Cab | excavator | its cockpit is an excavator's cab |
| Driller | Octagon | excavator | a mining machine: a modern digger's moulded cab |
| Mule | Tile wall | wood | a long-haul hauler, a trucker's cab with a wood dash |
| Roustabout | Titan | wood | the rig boss's machine, steel with burl accents |
| Harrow | Canopy | airliner | the glazed canopy is a flight deck |
| Picket | Cinema | padded | the sniper's sound-deadened quilted room |

`build_cockpit.STYLE_OF` holds this; `--style` overrides it for a check.

## 7. The builder's dressing (`build_cockpit.dress`)

The mockup's kit, built from the kit's roles so every style re-dresses it:

- **Plain walls** (an outline side with no screens): ribs at a third and two thirds, the conduit moved up under the
  roof, and between the ribs, from a fixed sequence per cockpit: an extinguisher in brackets, a first aid kit, a
  signal flare case, a life vest pouch, an oxygen mask box, a circuit breaker panel, a torch in clips, a life ring.
- **Under the screens** (the wall band below the lowest screen row, where it is at least 0.3 m tall): breaker
  panels, louvred vents, backlit button blocks and rivet rows.
- **The roof**: an overhead switch panel between the ceiling beams over the pilot's head, out of the 95 degree view
  ahead (its nearest edge is over 60 degrees up).
- **Grab handles** on plain walls (role `handle`: steel, yellow in the excavator style).
- **Nothing over a screen**: dressing goes only on plain walls, the band under the screens and the roof. An item is
  skipped when its box would reach into the seat, a side console, the dash or a pillar (keep-out boxes from the
  layout), and every item rides the part its wall rides (`node_at`), so it opens with the cabin.
- **The wire loom** (owner: "tidy up the wires they go no where", "place wires along the struts"): the cables that
  hung in the air beside each side console now lie along the console's outer top edge in clips, drop down its back
  face to the floor at one end and into its front block at the other.
- **The new roles**: `trim` (the rails round the screen rows and the dash's front edge: the burl in the wood style,
  the paint elsewhere), `handle`, `kit_white`, `kit_orange`, `kit_green`.

Cost: about 15 000 to 25 000 more triangles a cockpit (the breaker caps and the overhead switches are most of it),
under the 600 000 budget; no more draw calls (the atlases join every part).
