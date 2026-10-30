# Design: airfield-traffic

## 1. The taxi graph

Each site template gains marks: `stand_<n>` (a parking spot and heading, with its size class S/M/L), `taxi_<n>`
(taxiway nodes), `hold_<runway>_<end>` (holding points), `rwy_<runway>_<end>` (thresholds) and `pad_<n>`
(helicopter pads). `TaxiGraph::of_site` joins them by the template's `taxi_edges` list into a graph; a route is A*
over it. One graph per site, built at generation.

Taxiways are one-way where the template says so; an aircraft reserves the edges ahead of it out to 150 m and waits
if another holds them (stable order: lower aircraft id waits). The runway is one resource: one aircraft on it at a
time, departures held at the holding point while an arrival is inside 6 km on final.

## 2. Phases

`Phase` gains `Stand { t_s }`, `Pushback`, `Taxi { route }`, `Hold`, `LineUp`, `Vacate` before `Roll`, and after
`Rollout`. `Parked` (on the runway) goes. Speeds: pushback 1.5 m/s, taxi 8 m/s straight and 4 m/s in turns, a 10 s
pause at the hold. Turnaround at a stand: `turn_s` per kind (propliner 600 s, airliner 1,200 s, widebody 1,800 s at
the game's clock), during which the ground vehicles visit it.

Helicopters: `PadIdle`, `Lift` (vertical to 30 m at 3 m/s), `Transit`, `Approach` (descending to 30 m over the pad),
`Settle`. They fly to city helipads and back or orbit, as now.

## 3. Near and far

Within `sim_radius_m` (6 km) of the pilot an aircraft runs the phases above every fixed step. Farther away, a site's
traffic advances on a coarse timetable (a phase and its end time) and is drawn only as lights at night. When the
pilot comes near, each aircraft is placed where its timetable says, mid-taxi or at its stand. Cost: a busy airport
holds about 25 aircraft and 15 vehicles; under 0.1 ms per fixed step simulated (to be measured).

## 4. Ground vehicles

Kinds `tug`, `fuel_truck`, `baggage_train`, `catering`, `follow_me`, from the `combined-lowpoly-vehicle` pipeline.
They drive the taxi graph's service lanes (marks `svc_<n>`) between their depot and a stand being turned round, and
give way to aircraft. Instanced, one draw per kind and LOD.

## 5. Military bases

Kinds `fighter`, `military_cargo`, `military_helicopter` (models from `military-base`). Each base runs:

- **Patrols**: a pair of fighters takes off every `patrol_every_s` (900 s), flies a circuit of three points over its
  sector at 4,000 m, lands after `patrol_s` (1,200 s).
- **Supply runs**: a cargo plane between two bases every 1,800 s.
- **Helicopter patrols** over the base's nearest city.

**Hostility**: a base becomes `Hostile` for `hostile_s` (600 s) when the player damages anything on it, steals an
aircraft from it, or shoots down one of its aircraft. A hostile base scrambles its alert pair (two fighters held at
the alert apron) and a gunship. They take off in 60 s and attack using `cb_core::units`' brains (`run_in` for
fighters, `hover` for the gunship), which today start in mid-air: they now start at the runway. Civil airports never
turn hostile.

## 6. Stealing

Any aircraft at a stand or a pad, with no crew boarding, can be taken: the pilot walks to its door or steps and
presses Use (`fly-stolen-aircraft`). The aircraft leaves the traffic (`take`, as `traffic::take` does for cars) and
its stand is marked empty; the traffic's next arrival there is held until it is free.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| F1 | Does stealing from a civil airport have any response (police helicopter)? | None yet |
| F2 | Fighters: missiles, guns or both against the mech? | Both, using the existing unit weapons |
| F3 | How many aircraft at each civil airport at once | The 21 stands of the template, two thirds filled |
