# Design: combined-arms

All numbers in `data/world/units.ron` (defaults in `cb-core::units`). Counts and placement in
`data/world/encounters.ron` (`units: [(kind, count, offset_m, spread_m)]`).

| Kind | Speed | HP | Armour | Weapon | Range | Hit capsule |
| --- | --- | --- | --- | --- | --- | --- |
| tank | 12 m/s | 220 | 200 mm | 120 mm, 900 m/s, every 4 s, 140 mm pen, 35 dmg, 2 MJ | 600 m | 7 m long, r 2.2 |
| helicopter | 45 m/s | 120 | 20 mm | 4 rockets, 220 m/s, every 7 s, 110 mm, 22 dmg, 1.5 MJ | 500 m | 12 m long, r 2.0 |
| technical | 22 m/s | 50 | 0 | 12.7 mm MG, 900 m/s, 0.1 s, 25 mm, 1 dmg | 300 m | 5 m long, r 1.4 |
| rocket team | 1.5 m/s | 10 | 0 | rocket, 160 m/s, every 6 s, 150 mm, 40 dmg, 2.5 MJ | 450 m | r 1.0 |
| infantry (5) | 1.8 m/s | 4 each | 0 | rifles, 900 m/s, 0.4 s, 5 mm, 0.2 dmg | 250 m | r 0.6 each |

Projectile gains its own penetration, damage, blast and a rocket flag (rockets: smoke trail,
no drop), so units and mechs share one `fly`. A hit that beats armour takes `damage`; one that
does not takes 5 %. A blast damages units within its radius by `30 * MJ^(1/3)` falling off with
distance (infantry and technicals die to almost any blast).

Brains (each a few lines, every 0.3 s): tank closes to 450 m then stops and fires, moves again when
hit; helicopter circles the target at 300 m and 70-110 m up, firing a salvo when its nose points at
it; technical circles at 180 m at speed; rocket team walks toward the nearest building to the
target, stops, fires every 6 s; infantry squads walk toward the target to 200 m, fire, scatter
when a blast lands near.

Cost: up to 60 units, each a few microseconds a step; drawing ~200 small meshes.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G6 | First fight | 2 enemy mechs, 3 tanks, 2 helicopters, 3 technicals, 3 rocket teams, 4 squads | provisional (survey G6) |
