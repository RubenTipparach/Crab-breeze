# Design: car-traffic

## 1. The lane graph (`cb-core::world::lanes`, pure)

Built once from the world plan's `StreetPiece`s and `Highway`s:
- **Streets:** each piece becomes two lanes, one each way, 3.5 m from its centre line. Arterials get
  two lanes each way.
- **Highways:** two lanes each way.
- **Nodes:** a junction is wherever two pieces' centre lines cross or meet. Lanes connect through it:
  straight on, left and right.
- **Highways meet cities** at the edge of the city square, joined to the nearest arterial.

The graph is deterministic from the plan (stable ids in plan order), with a test that every lane has
a way out, so a car never dead-ends.

## 2. Cars (`cb-core::traffic`, pure, fixed step)

```text
Car { id: u32, kind: CarKind, lane: u32, s_m: f32 /* along the lane */, speed_m_s: f32,
      state: Driving | Stopping | Stopped | Crushed | Burning, next: u32 /* lane after the node */ }
```

- **Following:** the intelligent driver model, which accelerates to the lane's limit and keeps a
  2 s gap to the car ahead. Its constants are in `data/world/traffic.ron`.
- **Junctions:** right of way by arrival order at the node. A car waits while a crossing lane's car
  is within 15 m of the node, and the order is stable by car id.
- **Turns:** the next lane is chosen at the node by a `SimRng` forked from the car's id and the node,
  so a replay drives the same.
- **Fear:**
  - within 120 m of an explosion, or 60 m of a moving mech, a car brakes at 7 m/s2 and stays
    stopped for 20-40 s;
  - a car in the mech's path that cannot stop is pushed aside and damaged.
- **Crushing:** a mech's foot strike (`foot_strikes`, the one footfall rule) whose foot covers a car
  crushes it. The footprint and dust are as today.
- **Hits:** cars carry a capsule in the weapons' part list, like vehicles (invasion-forces). A hit
  that beats their 5 mm of armour sets them Burning.
- **Spawn and despawn:**
  - Lanes within 1.5 km of the eye are kept filled to their density, on lane positions not in the
    view frustum, or beyond 400 m and behind cover.
  - Cars beyond 2 km are dropped.
  - Wrecks (Crushed, Burning) stay for 10 minutes or until 2 km away.

## 3. Drawing (`cb-app::traffic`)

- **Instancing:** one instanced draw per car kind and LOD (gpu-instancing), with the transform
  written each frame from the interpolated lane position. Wheels turn in the shader from the speed.
- **Models:** `tools/blender/build_cars.py`, CSG hard surface:
  - 7 kinds, 3 paint colours each from a palette;
  - LOD 0 (within 80 m) about 6 000 triangles, LOD 1 (to 400 m) 1 200, LOD 2 (beyond) 200;
  - a crushed variant per kind, about 2 000 triangles;
  - headlights and tail lights as emission, lit at dusk.
- **Materials:** Material Maker car paint (base, clear-coat roughness), chrome, glass, tyre rubber;
  full PBR sets.

## 4. Numbers (survey J6)

| Item | Value |
| --- | --- |
| City lanes | one car per 60 m |
| Highway lanes | one car per 300 m |
| Cars within 1.5 km of downtown | about 150-250 |
| City speed | 40-60 km/h |
| Highway speed | 90-110 km/h |

## 5. Cost

| Item | Cost |
| --- | --- |
| Simulation | 250 cars x a following step and a node check: about 0.05 ms a fixed step (measured when built) |
| Drawing | 7 kinds x 3 LODs, about 21 instanced draws; about 250 x 1 200 = 300 000 triangles worst case downtown |
| Memory | under 1 MB of lane graph; car meshes about 6 MB; paint sets about 40 MB at 2048 px |

Measured downtown at rush density on the 3070 before it is called done.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J6 | Traffic density | One car per 60 m in cities, 300 m on highways, within 1.5 km | decided (survey J6) |
| J7 | Which cars | 1980s civilian, no real brands: sedan, wagon, pickup, van, taxi, bus, semi | decided (survey J7) |
