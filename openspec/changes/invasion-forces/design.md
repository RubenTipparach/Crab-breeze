# Design: invasion-forces

All numbers in `data/world/units.ron` and `data/missions/defence.ron`; the code holds none.

## 1. Units (`cb-core::units`, pure)

| Unit | Size | Speed | HP / armour | Weapon | Brain |
| --- | --- | --- | --- | --- | --- |
| tank | 7 x 3.6 x 2.6 m | 12 m/s | 220 / 200 mm front | 120 mm, 900 m/s, every 4 s | close to 450 m, stop, fire; move when hit |
| helicopter | 14 m rotor | 45 m/s | 120 / 20 mm | 4 rockets, 220 m/s, every 7 s | circle at 300 m, 70-110 m up, salvo when nosed on |
| fighter jet | 15 m long | 180-250 m/s, turn 8 deg/s | 80 / 10 mm | 25 mm cannon; 2 missiles (the `targeting` seeker) | attack runs: dive, strafe, climb out, loop back |
| cargo plane | 40 m span | 90 m/s | 400 / 20 mm | none | fly the drop line at 400 m, drop, leave |
| crab mech | per `crab-enemies` | | | | the AI pilot |
| crab airship | 300 x 80 x 60 m | 15 m/s | engines 4 x 600, turrets 10 x 150, bays 2 x 300 | flak (12 rounds/s, 1 MJ), beam turrets | hold over the attacked city at 600-900 m, launch fighters every 20 s, drop crab mechs every 45 s |

Every unit is hit through the one `weapons::fly` (capsules or boxes per part), damaged through the
one `Damage::resolve`, and its weapons are `WeaponSpec`s. Aircraft move as points with a speed, a
turn rate and a climb rate (no aerodynamics yet), pure and tested.

## 2. Drops

`Drop { body, released_at, chute_open_s: 2.0, descent_m_s: 8.0, swing_deg: 12 }`: free fall under
gravity until the chute opens, then descend at `descent_m_s` with a pendulum swing, land on
`height_at`, the chute folds and the unit's brain starts. A cargo plane releases its load along its
line over 3 s; an airship drops from its bays. Tests: `a_dropped_tank_lands_where_it_was_released_plus_drift`,
`a_unit_killed_under_its_chute_never_lands`, `a_downed_cargo_plane_takes_its_cargo_with_it`.

## 3. Placeholders

Boxes, cylinders and spheres in each unit's size and a faction colour (rust for the invaders' human
forces, violet for the crabs), drawn as instanced meshes. Chutes are hemispheres. Each is replaced by
its Blender model when it has one; the hit shapes and sockets stay.

## 4. Cost

Up to 30 ground units, 12 aircraft and one airship: each unit a few microseconds a step; drawing
about 200 placeholder meshes. The airship's 16 parts are one entity each. Measured with
`--bench defence` (CPU-bound today, so entity and system counts matter more than triangles).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G15 | The crab airship's size and how it dies | 300 m long, 600-900 m up; killed by its 4 engines, turrets and bays lockable parts on the way | provisional (survey G15) |
| G16 | What each defence wave brings | Wave 1: cargo drops of tanks and enemy mechs, helicopters; wave 2: plus fighters; wave 3: the crab airship with its fighters and crab mechs | provisional (survey G16) |
