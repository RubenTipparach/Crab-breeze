# Proposal: terrain-height-cache

## Why

The owner's free-roam performance run (2026-10-26: "ensure it always stays withing the 60-120 fps range. If it drops
below raise some flags and study the conditions that cause it") found fights and fast skating well under 60 fps on the
RTX 3070: a fight's median frame 15 to 30 ms, its slowest 5 % 50 to 120 ms, with the GPU at 1.4 ms. The CPU is the
limit. Timed sections (`perf_hud::section`, free-roam-fight, 2026-10-26) put it on two callers of the ground's height:

| Section | Per simulation step |
| --- | --- |
| `look_point` (where the guns converge: a 2 m march out to 1.5 km) | 2.0 to 8.5 ms |
| `fly_one_shot` (one shell's flight this step) | 0.42 ms each, about 17 in flight: 7 ms |
| the mech's own step | 0.08 ms |
| an enemy's brain | 0.2 ms |

Both spend their time in `Heightfield::height_at`, which works every height out from scratch: three lattice corners,
each the island's noise (`Shape::height`), the city flats and the river channels' carve (a search for the nearest
river). Nothing is kept. Every system that asks for the ground (movement, shells, aim, trees, AI) pays it again.

## What changes

- **The lattice heights are kept once worked out**, in tiles of 16 x 16 corners filled the first time anything reads
  them. A height read is then two or three memory reads. The heights are exactly the ones worked out today (the same
  function fills the tiles), so nothing in the game moves or changes.
- Nothing else: callers keep calling `height_at`.
