# Design: ship-traffic

## 1. A ship's day

Each ship slot at a port loops: `Anchored` (out at the anchorage, waiting for a berth) -> `Inbound` (along the
approach path through the `mouth` mark, slowing to 3 m/s inside) -> `Berthing` (the last 200 m at 1 m/s, turning
alongside) -> `Moored { dwell_s }` (cranes work it) -> `Unberthing` -> `Outbound` -> `AtSea` (on a lane to the other
port, or off the map edge and back after a wait).

A berth holds one ship; a ship waits at anchor until its berth is free (stable order: the earliest arrival goes
first, ties by slot id). Paths inside the harbour are the port template's new `channel` marks, a polyline from the
mouth to each berth.

## 2. Determinism and streaming

The timetable is a pure function of the seed and the world clock, as now (`place(i, cfg, t)`): a ship's phase and
position are computed, not integrated, so a port seen for the first time is mid-day, not freshly reset. Berth
waiting is resolved in the same pure way, by laying out every slot's schedule at generation time with no two ships
overlapping a berth.

## 3. Port ground traffic

Gantry cranes (one per berth) travel their rails to the moored ship's bays and run their trolleys while a ship is
`Moored`; yard trucks (8 a port) loop berth to stack on fixed paths. Both are instanced: one draw per kind. Their
motion is a pure function of the clock, so they cost nothing on the fixed step.

## 4. Cost

Up to 17 ships per port moving instead of standing: the same draw calls (one per kind and LOD), transforms updated
for ships within the 3 km draw range only. Under 0.05 ms CPU a frame for both ports (to be measured on the desktop).
## 5. As built (2026-10-30)

- Every ship a seaport's page lays (17 a port, both ports) is a timetable slot (`Plan::Port`) of its own kind, the
  boat drawn by its port kit as before (`Kits::ship`): at its own berth, where the page laid it, for `port_dwell_s`
  (300-600 s), out through the mouth to the anchorage, waiting there `port_away_s` (120-300 s), and back. Each ship
  has its own berth, so no two share one. The ships are no longer the port's lots (`PlacedSite::lot_of` skips them),
  and their lamps on the page are left out: a sailing ship is the boats'.
- A lane ship moors `dwell_s` (the existing `moor_s`) at each end that is a berth; at the offing it is over the
  horizon for as long.
- Not built yet: the gantry cranes' and yard trucks' moves (task 3.2), and a ship's own lights at night.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| S1 | Should ships on the open sea between the ports be shootable targets with cargo spills? | Shootable as boats already are; no cargo spill yet |
| S2 | Dwell time at a berth | 6 game minutes (real time 6 min at the 24 min day) |
