# Design: particle-steps

## 1. Sizes (`cb-core::fx::burst_count`, pure)

`step = ceil(count / 4)`, at least 1. The burst asked for is `round(count * density)`; it is rounded
to the nearest whole number of steps, at least one step while any particle is asked for, then cut to
the whole steps that fit the room left under `max_particles`. A density from 0 to 2 gives at most
nine sizes per layer.

## 2. Warm-up (`cb-app::fx`)

From frame 30 behind the loading screen, one density a frame: 0.25, 0.5 ... 2.0. While the game is
not ready, `play` ignores the particle budget and the burst range, so every size of every recipe is
built under the ground before `GAME_READY`.

## 3. Cost

| Item | Cost |
| --- | --- |
| GPU effects built | 70 burst layers x up to 9 sizes, all behind the loading screen |
| Warm-up particles | about 17,000 over eight frames, under the ground, before play |
| In play | nothing new: the same bursts, rounded |

Measured with the perf suite, before against after, on screen.

## Open questions

None.
