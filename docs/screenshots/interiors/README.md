# Every mech's cockpit from the seat, 2026-10-25 (renewed 2026-10-26)

Headless stills from a cloud session (Mesa lavapipe under Xvfb, no GPU), taken by the stills instrument
(`--stills`) after the pilot walks up, rides the rope and seats, with the cabin sealed. **Not a playtest and no
frame times** (CLAUDE.md sections 7 and 9). Main at `c5639a8` (PR #37) with PR #41's smoke puffs.

| Mech | Cockpit | Seat | Free look on the dash's centre screen |
| --- | --- | --- | --- |
| Foreman | Panorama | `foreman-seat.jpg` | `foreman-outline.jpg` |
| Driller | Octagon (amber) | `driller-seat.jpg` | `driller-outline.jpg` |
| Picket | Cinema (ice blue) | `picket-seat.jpg` | `picket-outline.jpg` |
| Ironhorse | Cab | `ironhorse-seat.jpg` | `ironhorse-outline.jpg` |
| Mule | Tile wall (yellow) | `mule-seat.jpg` | `mule-outline.jpg` |
| Harrow | Canopy (rebaked, screen raised) | `harrow-seat.jpg` | `harrow-outline.jpg` |
| Roustabout | Titan | `roustabout-seat.jpg` | `roustabout-outline.jpg` |
| Sapper | Twin tier (red) | `sapper-seat.jpg` | `sapper-outline.jpg` |

`all-seats.jpg` is the eight seat stills on one sheet. Since 2026-10-26 (`cockpit-identity`) every roster mech has a
cockpit of its own, all baked at 4096 px, each hiding at most 25 % of the fighting band (`tools/blender/measure_view.py`).
Shot at the 95 degree seated view, level. Seen here: in the Octagon, the Tile wall and the Twin tier the hung MFDs and
the dash's screens sit at the top and bottom edges of the level view, read by looking up or down; in the other five
they are in view at a glance.
