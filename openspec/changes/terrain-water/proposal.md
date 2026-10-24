# Proposal: terrain-water

## Why

Owner, 2026-10-24: "Water, river, ripples? Ways to make mech fights in the water epic?" The world
has hills, cities and highways but no water. Rivers and lakes give the terrain shape and purpose,
and a 12 m machine wading through a river is a set piece: spray on every stride, the water
dragging at its legs, steam where hot armour meets it, and the fight slowed down to the river's
pace.

## What changes

- The world generator carves rivers and fills lakes from the seed: a river runs downhill from
  the world's edge, through open ground, under the highways, and may pass a city on an embankment.
  A sea level fills low basins as lakes. All of it is in `WorldPlan`, deterministic, golden-tested.
- One water query, `water_at(x, z) -> Option<WaterSample { surface_m, depth_m, flow_m_s }>`,
  beside `height_at`. The pilot, the mech, splashes and drawing all use it.
- The mech wades: drag on each leg in proportion to its submerged length and the flow, speed
  capped by depth, a stride that lifts higher, cooling for hot armour (feeds the heat model when it
  exists), and it cannot walk deeper than the cockpit. The pilot on foot cannot swim yet: water
  over 1.2 m stops them.
- Water is drawn as one surface mesh per water body with an animated normal-map ripple, flow
  direction, depth tint and shore foam, and interactive ripples: rings from every mech foot and
  splash, from a small ripple texture around the camera.
- Splashes and spray: a foot entering water, a shot or an explosion on water throws a sheet of
  spray and leaves a ring. Steam when hot metal goes in.
- The cockpit screens show it: water climbing the lower screens when wading deep, droplets
  running off the camera head (the raindrop effect borrowed from Pale Blue Dot, see `clouds`).

## Out of scope

Swimming, boats, underwater combat, buoyancy of loose objects, and rain filling rivers.
