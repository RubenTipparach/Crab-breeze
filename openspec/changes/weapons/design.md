# Design: weapons

| Weapon | Muzzle speed | Interval | Penetration | Damage | Blast |
| --- | --- | --- | --- | --- | --- |
| autocannon 35 mm | 1 100 m/s | 0.12 s | 90 mm | 12 | 0.4 MJ |
| heavy cannon 120 mm | 850 m/s | 2.5 s | 260 mm | 90 | 6 MJ |

A projectile moves `x += v dt; v.y -= g dt` each fixed step and tests the segment it swept against:
mech part capsules (the rig's bones at that step), building cells, tree trunks, then the ground.
Hit parts take damage through `Damage::resolve`; a blast digs, snaps trees and breaks building
cells within its radius. Aim: the guns converge on the point the pilot looks at (the torso aim).
Cost: a few hundred projectiles a step, each a few segment tests through the bucket grids.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G4 | Missiles | lock-on missiles next, after guns | decided by the owner (Ace Combat first) |
