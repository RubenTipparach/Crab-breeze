# Design: civil-air-traffic

The aircraft are points with a heading, a speed and a climb rate, flown at the fixed step by pure code
in `cb-core::air_traffic`. `cb-app::air_traffic` feeds them the pilot, the enemies and the ground, draws
them, and turns a crash into an explosion and a wreck. Every number below is in
`data/world/air_traffic.ron` unless it says otherwise.

## 1. Kinds (`air_traffic.ron` `kinds`)

| Kind | Stands in for | Length x span x height | Cruise | Approach | Climb / descent | Turn | Height | Most near you |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `airliner` | 1980s twin-jet, between a 737-300 and a 757 | 40 x 35 x 12 m | 120 m/s | 70 m/s | 12 / 9 m/s | 3 deg/s | 1 100 m over the ground | 3 |
| `freighter` | 1980s four-engine heavy freighter (W48) | 70 x 68 x 19 m | 110 m/s | 65 m/s | 9 / 8 m/s | 2.5 deg/s | 900 m | 2 |
| `light_plane` | four-seat high-wing light plane | 8.3 x 11 x 2.7 m | 55 m/s | | 4 / 5 m/s | 8 deg/s | 300 m, circling 700-1 100 m out | 2 |
| `helicopter` | light turbine news or police helicopter | 12 x 10.2 (rotor) x 3.4 m | 35 m/s | | 6 / 6 m/s | 12 deg/s | 180 m, circling 350-600 m out | 2 |

Speeds and heights are the island's, not the real aircraft's: the routes are 4-30 km long, so a real
airliner's 230 m/s at 10 km would never level off. Each kind's `hp`, `armour_mm` and `crash_mj` are in
the file; a validation refuses a route kind that cannot descend as steeply as the glide path from cruise.

## 2. Routes between the bases (`flight.rs`)

**The bases have no runway.** `air-bases` draws a 700 x 500 m apron, three hangars at the back, three
40 m pads at the front and a tower in the front corner. A strip is laid along the apron's long side
instead: on the line 60 m in front of the centre, from 40 m inside one end to 40 m inside the other
(620 m), clear of the pads, the tower and the hangars, and flown either way.

A flight, in order:

1. **Roll.** From the strip's start it speeds up at `approach^2 / (2 x 0.9 x length)`, which lifts it
   off before the strip ends (4.4 m/s2 for the airliner, about twice a real one: W45).
2. **Climb** straight ahead at its climb rate to 150 m over the ground.
3. **Enroute** to the approach's entry: on the destination strip's centre line, `final_m` (4 km)
   plus two turn radii at cruise out. It holds its cruise height over the ground, never under the glide
   path to the strip, never under 120 m over the ground.
4. **Intercept:** at the entry it turns onto the centre line with a localizer (turn back toward the
   line over 0.4 of a turn radius), slowing to approach speed, down toward the glide path.
5. **Final** from the fix: down the 3 degree glide path on the centre line.
6. **Touchdown** at the threshold if within 20 m of the strip's height, 40 m of its centre line and
   20 degrees of its heading; otherwise it goes round again (back to 3).
7. **Rollout,** braking to stop before the strip ends, then **parked** for 30 s, then it leaves.

The approach end is chosen once per destination: the one whose 4 km final keeps 60 m of ground under
its glide path, the nearer if both do. On the seed-42 island this matters: base 1 sits under hills to
its west.

**Ground.** The island has a 1 900 m peak. Every second a route aircraft reads the ground under it, 12 s
ahead, and every 6 s of flight for the next 120 s along its heading and its way, and flies no lower
than what a full-rate climb from here clears by 120 m. Every 5 s it reads the highest ground on the
straight way to its next point (every 400 m). When that ground is higher than a climb at its best
climb speed (1.2 x approach) can reach in time, allowing a circle's width, it circles where it is,
climbing, until it can. Over the approach's entry 150 m above the glide path it circles down before
turning in.

**Shaped for the career loop.** A route may carry `via` points: it overflies each at cruise, in order,
before its approach, and reports each as it passes (`Happened::Passed(i)`). `Traffic::launch(kind,
from, to, via)` sends one at once, wherever the pilot is. The career loop's mech air drop (mech-shop, on
main) flies the `freighter`'s stand-in and engine, but not a launched `Traffic` freighter: its release
comes `delay_s` after the order on the career's clock and is saved with the career, and a launched
freighter would arrive when its climb, cruise and turns bring it, which would make the drop's timing the
flight model's. `launch` and `Passed` stay for the day the drop is re-timed to the flight (or for
`airfields`' landed deliveries).

## 3. City traffic

Light planes and helicopters circle a city's centre at their radius, steering back toward the circle in
proportion to how far off it they are (the invasion helicopter's orbit rule, `units.rs`), at their
height over the ground, one way round or the other by seed.

## 4. Near the pilot only (`mod.rs`, `Traffic`)

| Rule | Value |
| --- | --- |
| Spawn anywhere unseen | beyond 10 km (an airliner is a few pixels there) |
| Spawn behind you | beyond 3 km and more than 75 degrees off where you look |
| Removed | beyond 14 km, or parked 30 s, or crashed (its wreck stays as debris) |
| Look for room | every 4 s, one per kind below its `max_near` |
| At most | 3 + 2 + 2 + 2 = 9 aircraft |

A route aircraft starts rolling at a base out of sight, or cruising at a quarter, half or three
quarters of the way between two bases out of sight. A city aircraft starts on its circle out of sight,
or 3.6 km out from a city within 11 km of you, inbound. Every choice is from a `SimRng` forked from the
world seed and the check's count, so a seed flies the same traffic.

## 5. Keeping clear of a fight

A fight is a 2.5 km circle round every live enemy (the hostiles list: mechs, vehicles, the fortress's
parts). Every aircraft's way is bent by one rule, `steer_clear`: inside a circle, straight out of it;
heading into one, along its near tangent plus 8.6 degrees; else unchanged. A route whose destination
base is in a fight diverts to the nearest quiet base. A city aircraft whose city is in a fight leaves
for good. No aircraft is spawned in a fight, at a base in one or to circle a city in one.

## 6. Shapes, hits and wrecks (`shape.rs`)

**One table of boxes per kind**, `blockout(spec)`, from the measured size: fuselage, wing, engines, fin
and tailplane; a propeller and gear; a cabin, boom, mast, crossed rotor bars and skids. The game draws
it (`cb-app::air_traffic`), and `cb-tools aircraft` prints it for the mockup page
(`tools/mockups/gen_civil_aircraft.py`, writing `docs/mockups/civil_aircraft.html`), so the two cannot
disagree. A test checks each table is its kind's length, span and height. These are **stand-ins**, approved
by the owner on 2026-10-26 ("Approved all of the stand ins"); the real models come from Blender next (CLAUDE.md section 10).

**Hit shapes:** a capsule down the fuselage and one across the wing (a helicopter: its cabin only),
posed by the aircraft's heading, pitch and bank. They join the shot sweep's part list under ids from
800 000 (`CIVIL_ID_BASE`), and a hit goes through `units::armour_hit`, the one armour rule the
invasion's vehicles use.

**Downed:** it falls under gravity, its speed over the ground decaying at 0.15 per second, turning at 25
deg/s (a helicopter 75). On the ground it explodes (`blast::Explosion` at its `crash_mj`: crater, trees,
building cells, splash on mechs) and its wreck is laid down as `fall.wreck_pieces` (6) boxes, the
fuselage in lengths and the wing in halves, strewn along its heading by its id, pushed into
`Live::new_chunks`: a rigid body near you, a dust puff far away (`debris-physics`).

**Crabs shoot them.** An invasion vehicle whose target (you) is not its shot takes a shot of
opportunity at the nearest civil aircraft in its guns' range and cone, through the same aiming rule
(`Unit::aim_at`). Crab mechs on foot do not (their guns do not elevate to aircraft yet).

## 7. Standing and the cockpit

**Your hits.** Every shot or blow that lands on a civil aircraft sends `CivilAirHit { aircraft, at,
base, downed, by_pilot }`, where `base` is the air base nearest the hit. A hit by the pilot is added to
`CivilStanding`, a tally per base of hits and downings. The career loop's per-base standing reads
either when it lands; this change charges nothing.

**Neutral on the screens.** Civil aircraft are listed in `Neutrals`, never in `Hostiles` (the music,
the defence count and the scripted pilot see only enemies). Targeting takes both, marking civil ones
neutral: the pick nearest the crosshair and the cycle key skip neutrals, so a missile never locks one
by itself. Pressing the cycle key with the crosshair within 3 degrees of one selects it on purpose
(`targeting.ron` `neutral_pick_cone_deg`, W47). Its box is white with `CIV` and its range, red only
once locked; the TARGET page names it `CIVIL AIRLINER`.

**Engine sound.** Jets carry `engine_pitch`; within 4 km of your head each plays the `airliner_jet`
loop riding it: the mech's rocket roar (`rocket_loop.ogg`) pitched to 0.55 or 0.45. A stand-in until a
turbofan, propeller and rotor are cut from CC0 recordings (W49).

## 8. Cost

| Item | Cost |
| --- | --- |
| Simulation, seed 42, 9 aircraft, release | 22 us a fixed step on average, worst 0.5 ms (a step where several aircraft read the ground at once), measured by `measure_traffic_step` on this Mac, not the owner's PC |
| Why | the ground is noise read point by point at about 3 us a read; an aircraft reads 1 point a step at most and about 45 a second |
| Drawing | 9 aircraft x 6-8 boxes: at most 72 draws, about 900 triangles; 2 materials per kind |
| Hits | 2 capsules per aircraft added to the shot sweep (18 at most) |
| Sound | at most 3 engine voices |
| Memory | a few kilobytes |

Frame time in the game is not measured here: this session is not allowed to run the game (CLAUDE.md
section 8). It is to be measured on the 3070 in the cockpit at the start and over a city.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W32 | People, cars and aircraft: in what order | All three at once, in parallel | decided (survey W32) |
| W33 | What civil aircraft, where | Airliners and freighters between the bases; light planes and helicopters over cities; clear of fights; crabs can shoot them | decided (survey W33) |
| W34 | What hitting one costs | Standing with the nearest base (the career loop's) | decided (survey W34); this change reports the hits |
| W45 | The bases have no runway | Add a runway to each base | answered by the owner, 2026-10-26 (survey): "Add a real runway, about 1.5 to 2 km, to each base"; building |
| W46 | How much traffic | 9 near you at most (3 airliners, 2 freighters, 2 light planes, 2 helicopters) | decided: the recommendation, accepted by the owner, 2026-10-26 (survey) |
| W47 | Selecting a civil aircraft on purpose | The cycle key with the crosshair held on it | answered by the owner, 2026-10-26 (survey): "Never: only unguided fire can hit one". Q and the auto-pick skip them and no missile locks one; building |
| W48 | The freighter's size | 70 m, slinging a bought mech under it for the air drop | decided: the recommendation, accepted by the owner, 2026-10-26 (survey) |
| W49 | Engine sound | Cut a turbofan, a propeller and a rotor from CC0 recordings | answered by the owner, 2026-10-26 (survey): the recommendation; building |
