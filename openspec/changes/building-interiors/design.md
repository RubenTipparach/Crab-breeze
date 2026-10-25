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

The pilot's capsule (0.35 m) collides with the AABBs of standing floor bays, partitions, core walls
and furniture taller than 0.4 m. Those AABBs are indexed by `(storey, cell)`, the same grid as the
structure. Stairs are ramps at 34 degrees. Doors are gaps. The mech (3 m radius) collides only with
columns, walls and floors. It breaks partitions and furniture by walking through them
(`structural-destruction`).

## 5. Cost

From the mockup (not a game measurement):

- 795 rooms and 2 951 furniture items on 7 buildings.
- Interiors are about 60 % of the 527 k triangles.

Expected in the game:

- 25 buildings inside the interior radius downtown is about 3 M interior triangles. Culled per storey
  band by the camera frustum and by facade occlusion; measured in `perf-bench`.
- Furniture is instanced per kit item: one draw per item type in view (about 30).
- Memory: about 40 bytes per item and 64 bytes per partition. A 19-storey tower's interiors are about
  110 kB.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W11 | How far do real interiors go? | every building has them, built within 150 m and when breached; interior mapping further out, from the same plan | provisional (survey W11) |
