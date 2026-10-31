# The nuclear cloud's draw cost, 2026-10-30

What the Stratostacean's mushroom cloud (`crates/cb-app/src/nuke.rs`) costs to draw, and why.

- **Machine:** the owner's MacBook Pro, Apple M4 Max, 120 Hz Retina display. It is not the target machine (an RTX 3070).
- **Build:** release, branch `fortress-nuke`.
- **Scene:** the nuke test scene (`CB_STILLS_NUKE=1 CB_NO_TREES=1`), 2.5 km from the blast. Every frame was logged
  (`CB_HITCH_MS=0`), with no OBS. The scene's own stills ran too, so a few frames carry a screenshot.
- **Variants:** set by the cloud's measurement switches (`CB_NUKE_PUFF_STRIDE`, `CB_NUKE_SIZE_SHARE`).
- **Repeats:** one run per variant, not interleaved, so the spread between runs is unknown. A difference of a
  millisecond or two is not a result.

## What the cloud is

- **Sprites:** 1880 billboards (ball 900, ring 240, column 500, skirt 240), placed on the CPU each frame.
- **Draw calls:** the sprites are sorted with the other soft sprites into one mesh. That is one mesh and one draw call.
- **CPU cost:** the main world takes 2.6-3.1 ms a frame with or without the cloud, so the cloud's CPU cost is small.
- **GPU cost:** the cost is fill. Each sprite is a large transparent quad: a ball puff is 3.6 times the ball's
  radius across, so hundreds of them overlap every pixel of the cloud. Each of those fragments samples three sheets,
  reads depth for the soft edge, and works out the fog and haze between it and the eye.

## 1920 x 1080 (`CB_NATIVE_1080=1`)

Frame times in milliseconds, while the cloud is up.

| Variant | Before the blast p50 | With the cloud p50 | p95 | p99 |
|---|---|---|---|---|
| No cloud drawn | 20.3 | 19.1 | 27.7 | 32.9 |
| Full cloud | 19.6 | 24.4 | 40.0 | 41.1 |
| Every 4th puff | 20.3 | 20.2 | 30.3 | 32.8 |
| Puffs at half size | 19.9 | 21.0 | 32.2 | 33.5 |

## At the Retina window's own size (3840 x 2160)

| Variant | With the cloud p50 | p95 | p99 |
|---|---|---|---|
| No cloud drawn | 30.8 | 36.7 | 41.3 |
| Full cloud | 41.9 | 58.6 | 66.3 |
| Every 4th puff | 30.4 | 46.4 | 47.6 |
| Puffs at half size | 31.0 | 47.0 | 49.4 |

## Findings

- **On the Mac the scene is over budget before the blast:** about 20 ms a frame at 1080p. The take recordings drew
  at the Retina window's own size, four times as many pixels, which is where their 42 ms came from.
- **The full cloud adds about 5 ms at p50 and 12 ms at p95 at 1080p.** The cost scales with puff count and puff
  area, as overdraw does.
- **Every 4th puff (470 puffs) costs about a millisecond at p50 and 3 ms at p95.**

## Next

- **Fewer puffs:** draw about a quarter as many puffs at the 4x size, then check on stills that no gaps come back.
- **Cheaper fragments:** work out the fog and haze once per sprite instead of once per fragment.
- **Confirm on the RTX 3070:** measure there.

## After the fix: a quarter of the puffs, fog once per sprite

The cloud now has 470 puffs (ball 225, ring 60, column 125, skirt 60) at the same 4x size, each a little more opaque.
The sprite shader works out the air between the eye and a sprite once, at the sprite's centre, in the vertex
shader, instead of once per fragment. Interleaved runs at 1920 x 1080, no OBS:

| Run | With the cloud p50 | p95 | p99 |
|---|---|---|---|
| No cloud drawn | 19.2 | 28.4 | 33.9 |
| New cloud | 19.8 | 29.9 | 33.0 |
| No cloud drawn, again | 18.8 | 29.1 | 33.1 |
| New cloud, again | 19.5 | 28.4 | 32.9 |

The cloud now costs about 0.6 ms at p50. The two no-cloud runs differ by 0.4 ms, so this is close to the spread. The
frame time no longer climbs as the smoke spreads: the last third of the cloud's life runs at 18.6-18.7 ms, the same
as with no cloud. The stills (`docs/screenshots/fortress-down/2026-10-30-nuke-quarter/`) show no gaps.
