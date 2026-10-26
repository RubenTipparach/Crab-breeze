# Design: terrain-height-cache

## 1. Where

`cb-core::world::terrain::Heightfield`. `lattice(ix, iz)` stays the one implementation of a corner's height; a new
`cached(ix, iz)` reads it through the cache, and `height_at` and `lattice_at_step(.., 1)` call `cached`.

## 2. The cache

- The world's lattice is `n` x `n` corners (`n = world_size_m / terrain_cell_m + 1`: 5 121 at 40 960 m and 8 m).
- Tiles of `TILE` = 16 x 16 corners: `ceil(n / 16)`, 321 per side, 103 041 tiles. Each is a `OnceLock<Box<[f32]>>`
  (16 bytes empty), so the empty cache is 1.6 MB; a filled tile is 1 KiB.
- A read inside the world finds its tile; an empty tile is filled whole (256 corners through `lattice`) by the first
  thread to reach it (`OnceLock::get_or_init`; others wait for it). Past the world's edge (the open sea) corners are
  worked out directly, as today.
- Filled tiles are kept for the run. Worst case, every tile visited: 103 041 KiB, about 101 MB. A free-roam session
  that crosses the island touches a few thousand tiles (a 2 km wide band: a few MB).
- `Heightfield` is shared by reference across threads (the async meshing tasks read it); `OnceLock` is `Sync`. Clone
  gives an empty cache (`Clone` is implemented by hand), so a clone never shares or copies tiles.

## 3. Determinism

A tile holds what `lattice` returns for its corners, bit for bit; when it was filled cannot change a height. A test
compares `height_at` through the cache with the uncached formula at seeded points over the island, the edge and past it.

## 4. Cost

| | Before | After |
| --- | --- | --- |
| `height_at`, tile filled | 3 x (noise + flats + river carve) | 3 array reads |
| first read of a tile | (same) | 256 lattice corners, about 1 ms on the i7-9700F, once |
| memory | 0 | 1.6 MB, plus 1 KiB per tile touched |

Expected: `look_point` and `fly_one_shot` fall to a small fraction of today's cost; measured with the free-roam-fight
take, old and new interleaved (section 5).

## 5. Measuring

`free-roam-fight` (board, nearest job by mail, drive, fight) three times each for the old and the new build,
interleaved, vsync off, at 1920 x 1080 on the 60 Hz display; the drive's and the fight's p50, p95, p99 and the share
under 60 fps per run, and the `sections:` log lines. Then the full `free-roam-perf` run.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| TH1 | Precompute the whole lattice behind the loading screen instead | no: about 26 million corners, 10 s or more of loading | provisional |
