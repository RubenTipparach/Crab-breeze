# Design: building-interiors

## 1. Floorplan algorithm

Work in `(u, v)`: `u` along the storey's long axis, `v` across it. Sizes are the mockup's defaults.

1. **Industrial** storeys are one hall (workshop on the ground, storage above) with a stair in a
   corner bay. Stop.
2. **Core**: one 6 m cell, centred in the top storey's footprint (every lower footprint contains it).
   Only when the top storey is at least 3 x 3 bays.
3. **Corridor**, 2.0 m wide:
   - **Double-loaded** (depth 13 m or more, with a core): a spine along `u` through the core's centre,
     and lanes round the core joining it.
   - **Single-loaded**: along the back, with a 5.5 m x 3.5 m stair at the end away from the street.
4. **Bands**: the strips between corridor and facade. Mandatory cuts at the core lanes' ends.
5. **Rooms**: walk each band along `u`. Pick a room type and width by the storey's use, snap the cut to
   the 3 m bay lines, and merge a remainder under 2.9 m into the last room.

   | Use | Room types and widths (m) |
   | --- | --- |
   | Retail | shop 6, 9, 12 on the street side; stockroom behind |
   | Lobby | lobby up to 12 on the street side, shops; mail room behind |
   | Office | open office 9-15 (45 %), private office 3-6 (35 %), meeting 6 (20 %) |
   | Flat | a flat 6-9 wide: kitchen and 2.4 m bath along the corridor (2.6 m deep), living room on the facade, bedroom too when 8.5 m or wider |

6. **Walls**:
   - Each room writes its corridor-side wall with a 1.0 x 2.1 m door (and a header above).
   - Each cut inside a band writes one wall. Where two bands meet, one wall runs from the corridor edge
     nearest the spine out to the facade, written once so no two walls share a plane.
   - Partitions are 0.12 m and end 2 cm inside the walls they meet. Core walls are 0.25 m concrete.

## 2. Furniture

- Items are boxes in their own frame (origin at the back edge on the floor, facing +z), with real
  sizes: a desk 1.5 x 0.75 x 0.73 m with its chair, a bed 1.5 x 2.0 m, a pallet rack 2.7 x 1.1 x
  4.0 m.
- A room type's list is placed in order with up to 30 tries an item:
  - wall items pick a wall and a position along it, then face inward;
  - centre items pick a spot at least 1 m from the walls, turned 0 or 90 degrees;
  - grid items fill rows (desk pods on 2.4 x 3.2 m, racks on 3.0 x 3.4 m between the columns).
- An item is placed when it and its clearance strip lie inside the room's clear rectangle (0.32 m
  inside the facade, 0.1 m inside partitions), clear of door zones (1.8 m squares), column footprints
  and placed items. Tall items (over 1.2 m) never go on a facade wall.
- Each item is one element resting on the floor bay under its centre.

## 3. Near and far

| Distance from pilot or cockpit | Interiors |
| --- | --- |
| under 150 m, or the building is breached | real: partitions, core, stairs, furniture (generated on a worker thread from the seed, about 2 ms a storey) |
| over 150 m | interior mapping on the glass: a 4 x 4 atlas of room renders per room type, picked by the room behind that window from the same floorplan |

The atlas renders come from the Blender furniture kit in staged rooms (Star Crew's render method).

## 4. Collision on foot

One rule for everything solid inside a building: every wall, tread, landing, floor, door leaf and
furniture box is an axis-aligned box. For a person with feet at `y`:

- a box whose top is at or below `y + step_m` (0.45 m) is something to stand on: the floor under
  the feet is the highest such top under them (the ground if none);
- a box whose top is above that and whose bottom is below `y + head_m` (1.8 m) is a wall, and the
  pilot's circle (0.35 m) is pushed out of it.

So a tread (0.175 m) is climbed, a desk (0.73 m) is walked round, a door header (2.1 m up) is walked
under, and a floor 3.5 m above is a ceiling that never touches. Doors are gaps in the walls.

The owner, 2026-10-26: "when on foot I should be able to wlak into the buildings, I think you would
generate detailed colliders for buildings only when the player is on foot". So:

- **On foot**, the boxes of a building (`Walkable`) are built from the same plans that are drawn: the
  facade walls (0.3 m, the skin's depth, one per bay per storey) with the entrance cut out, the floors
  (one per cell per storey) with the stairwell cut out, the roof, every storey's partitions and core
  walls, the stair, the furniture, and, once the building has a frame, its columns. They are bucketed
  by storey, and built only for buildings within `collision_radius_m` (60 m) of the pilot: smaller
  than the 150 m interior radius, because colliders only have to be ready before a walker reaches the
  door (13 s at a run), and the job takes milliseconds. Those within `collision_sync_m` (12 m) are
  built at once, in the step that needs them; the rest on a worker thread. Until a building's boxes
  arrive it is the solid box it always was.
- **Breached or intact alike.** Every box remembers the frame element it hangs on (`Anchor`: the floor
  bay under a partition or a desk, the facade panel a wall bay is, the column itself). While a building
  is whole every box stands; once it is hit, a box stands while its element does, so a blown-out wall
  bay is a way in and a fallen floor is a drop. The frame's own floor bays are cut round the
  stairwell, drawn and solid alike. (The owner's screenshot was a breached building they could not
  enter.)
- **In the mech** there are no interior colliders at all: the moment the pilot is not on foot
  (boarding, seated, climbing out) every `Walkable` is freed and buildings are the coarse boxes and
  cells they are today. The mech (3 m radius) never reads them: it collides with standing cells and
  breaks partitions and furniture by walking through them (`structural-destruction`).
- **Switching cleanly.** Climbing out ends standing at the rope's foot, which is outside every
  standing cell; the boxes within 12 m are built in that same step, before the first step on foot
  moves the pilot, so no step stands the pilot inside a coarse box or over a floor that is not there.
  A building is never freed while the pilot is on foot within its footprint plus the margin.

## 5. The street entrance

The owner, 2026-10-25: "each building needs doors and staairs that i as a player can walk into them".

**Which facade.** Sides are numbered as the dressing numbers them: 0 faces -z, 1 +z, 2 -x, 3 +x.
For each side, the strip from that facade out to the block's edge (the kerb) is checked against the
other buildings of the block. Of the sides whose strip is clear, the nearest to the kerb wins (ties:
a long face, then the lower side). An inner lot has no clear side: its door goes on the side with
the widest gap in front (at least 2 m, two setbacks), an alley that reaches the pavement. The side is
worked out once, when the world is generated (`WorldPlan::street_sides`).

**Where on the facade, and where it leads.** Work in the plan's `(u, v)` (section 1).

| Layout | Street on a long face | Street on an end |
| --- | --- | --- |
| Double-loaded (a core) | the door is on the core's centre line; the band in front of the core becomes a **lobby** on the ground floor, its corridor door lined up with the street door, and the core's door turns to face that lane: one straight walk from the pavement to the stair | the door is on the spine's centre line: the spine, a side lane round the core, the far lane, the core door |
| Single-loaded | the corridor runs along the back; the door is in the middle bay clear of the stair, through a lobby room lined up with it | the door is in the corridor's end, kept clear of the corner column; the stair is at the far end |
| | if the street is on the corridor side, the door opens straight into the corridor | |

**What it looks like** (clear height 2.2 m above its sill):

| Style | Door | Width | Sill |
| --- | --- | --- | --- |
| Walk-up | a timber front door under a glazed transom, up a three-step stone stoop (0.12 m risers) and over a stone threshold, one step down into the hall | 1.2 m | 0.36 m |
| Office, tower | a pair of glass lobby doors in the shopfront, kick plate and plate glass cut round them | 1.9 m | 0 |
| Warehouse | a steel personnel door in the brick; the roller doors stay shut (no hall behind them yet) | 1.0 m | 0 |

The door is an **open doorway with its leaf standing open** at a right angle inside, hinged on one
jamb (a lobby's pair, one leaf each side). It is drawn and it is solid: you walk past it. This is
the simplest build that lets the owner walk in; whether the door should swing as you reach it, or
shut and open with Use, is survey W30 (provisional).

**Nothing in the way.** The route (pavement, door, lobby or corridor, stair door, landing) is a
polyline the plan keeps (`Entrance::route`). Furniture keeps out of a strip 0.4 m either side of it
past the door's half-width (the lobby's keep-out, like a door zone), and no column stands on it.

## 6. The stair

A dog-leg in the stair core, the same place on every storey:

- **Risers**: an even count, `2 * ceil(storey / (2 * 0.18 m))`: 20 risers of 0.175 m for a 3.5 m
  storey, ten a flight.
- **Treads**: nine a flight at a run of `min(0.28 m, room / 9)`, where the room is what the core
  leaves after a 1.3 m landing at the door and a half landing of at least 1.1 m. The smallest core
  (a 5 m cell) gives 0.26 m. A run under 0.22 m is refused (the test walks every shipped building).
- **Lanes**: two 1.2 m lanes side by side with a 0.1 m concrete spine wall between, flight one up
  from the landing away from the door, flight two back up to the next storey's landing. Each tread
  is a 0.2 m concrete waist under its top, so the flight above clears the head of someone on the
  flight below (3.5 m apart).
- **Walls**: the spine and a balustrade on the open side of the stair, a storey tall. In a
  single-loaded plan the stair room opens off the corridor at its landing only.
- **Stairwell**: every floor above the ground has the flights' footprint cut out of it (lanes, spine
  and half landing), drawn and in the collision alike.
- The stair runs from the ground floor to the top storey. The roof is not reached (the bulkhead on
  it stays a prop).

## 7. Drawing it

The dressing draws the entrance bay from `Entrance`: the opening, its frame, the open leaf and the
stoop (one set of boxes, which the collision also reads). A whole building's floors are drawn with
the stairwell cut out, and a ground-floor slab 2 cm above the pavement. The stair (treads, landings,
spine and balustrades) is drawn in the rooms mesh in concrete, so it shows wherever rooms show
(within `rooms_radius_m`, 250 m). A damaged building draws its frame's floor bays with the
stairwell cut out too.

## 8. Cost

From the mockup (not a game measurement):

- 795 rooms and 2 951 furniture items on 7 buildings.
- Interiors are about 60 % of the 527 k triangles.

Expected in the game:

- 25 buildings inside the interior radius downtown is about 3 M interior triangles. Culled per storey
  band by the camera frustum and by facade occlusion; measured in `perf-bench`.
- Furniture is instanced per kit item: one draw per item type in view (about 30).
- Memory: about 40 bytes per item and 64 bytes per partition. A 19-storey tower's interiors are about
  110 kB.

Entrances and stairs (worked out, not measured; the game's frame time is measured on the owner's
machine, section 7 of CLAUDE.md):

- A stair storey is 18 treads, a half landing and 3 walls: about 260 triangles; a 20-storey tower's
  stair about 5 k, inside the rooms mesh (no new draw call).
- An entrance bay is about 20 boxes, roughly what the shopfront it replaces cost.
- Collision: about 230 boxes a storey (24 bytes each), 5.5 kB a storey; a 20-storey tower about
  110 kB. Built on the async pool in about the time the storey's plan takes (0.03 ms a storey). A
  pilot step tests the boxes of the storeys round its feet in the buildings round it: a few hundred
  comparisons, no allocation.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W11 | How far do real interiors go? | every building has them, built within 150 m and when breached; interior mapping further out, from the same plan | accepted (survey W11) |
| W30 | The street door: open doorway with its leaf standing open, a door that swings as you walk into it, or a shut door you open with Use? | open doorway, leaf standing open (built) | provisional (survey W30) |
| W31 | A warehouse's roller doors: stay shut, or open onto a ground-floor hall you can walk (and drive) into? | stay shut until the industrial hall plan is built; the personnel door is the way in | provisional (survey W31) |
