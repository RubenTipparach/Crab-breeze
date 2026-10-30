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

## As built (2026-10-30)

What the code does now, and where it differs from the plan above.

- **Fields.** `cb_core::air_traffic::fields` turns every civil airport and every base into a `Field`: its runway (as an
  `AirBase` the flight code already flies to), a `TaxiGraph` (Dijkstra, ties to the lower node, so it is stable) and its
  stands. A civil airport's network is its page's: `tools/mockups/airport_src.html` lays out `entry_`, `edge_`, `hold_`,
  `taxi_`, `stand_`, `out_` and `pad_` marks and the edges between them, and `tools/buildings/export_sites.cjs` writes them
  into `assets/models/sites/airport/site.json` (`taxi_edges`), so the game and the page cannot disagree. A base has no
  page yet, so its network is laid out from its site and runway: six stands in a row (three fighters, a transport, two
  helicopter pads), an apron lane, a taxiway to the runway's middle and the runway back-tracked to either end. Civil
  field ids are 100 and up.
- **Phases.** `Stand`, `Taxi { push_m, then }` (pushback is the first `push_m` of the line, tail first, at 1.5 m/s;
  taxi 8 m/s), `Hold`, `Lift` and `Settle`. At the end of a taxi line the aircraft turns in place onto the runway or its
  stand before stopping. `Parked` stays as the fallback when a landing finds no free stand (it waits on the runway's end
  for `park_s` and leaves, as before).
- **The ground decisions** are `Traffic::ground`, in id order: a stand's turnaround over (`turn_s`, per kind, default
  900 s) sends it off; a holding aircraft is cleared onto the runway only when nothing is lining up, rolling, on final or
  rolling out there (one aircraft on a runway at a time, read from the traffic each step, no stored reservation); a
  landed one taxis from the exit ahead of it to a free stand of its kind (else any free stand that is not a pad).
  Taxiway edges are not reserved yet: two aircraft can meet on a taxiway (open).
- **Filling.** Every `check_s`, each field within `stand_fill_m` (6 km) of the pilot fills its empty stands that the
  pilot cannot see, three in four, part way through their turnaround so they leave in turn. The first check fills them in
  sight too, while the world loads. The airport's 21 parked aircraft are no longer scenery: `park_aircraft` is gone.
- **Routes.** A civil kind flies between the civil airports, a military one between the bases (when the island has two
  of its side; before that, any runway, as the old tests do).
- **Military.** Fighters taxi out and patrol a 5 km circle over their base for `patrol_s` (1,200 s), then land and taxi
  back. Gunships and civil helicopters lift straight off their pad, patrol (over the base, or the nearest city) and
  settle back on the same pad, which they keep while away. The transport (`max_near` 1) flies supply runs between bases.
  The stand-only kinds (`max_near` 0) are never spawned in the air.
- **Scramble.** `Traffic::scramble`: when the pilot hits a military aircraft, its home field's fighters and gunships
  leave their stands at once and circle over where the pilot was, and those already on patrol from it turn to circle
  there. Once circling there they leave the traffic (`Traffic::take_attackers`) and become units (`base_fighter`, `base_gunship` in `data/world/units.ron`): the invasion's brains fly them, attack runs for the fighter and a firing circle for the gunship, with its cannon and rockets, drawn as the aircraft's own model (`UnitSpec::look`).
- **Not built yet:** far-from-the-pilot timetables (traffic lives near the pilot only, as before), ground vehicles,
  taxiway edge reservation, and stealing (`fly-stolen-aircraft`).
- **Cost.** Not measured (a cloud session measures nothing). Per fixed step it adds one pass over the traffic list and,
  every `check_s`, a pass over the stands of the fields near the pilot (21 at an airport, 6 at a base).
