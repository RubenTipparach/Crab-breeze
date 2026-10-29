# Design: night-frame

## 1. Method

- Release build, on screen in its window (CLAUDE.md section 7), vsync off (`CB_UNCAPPED=1 CB_NATIVE_1080=1`), game
  audio muted (`CB_MUTE=1`), nothing else run beside it. RTX 3070, i7-9700F, 60 Hz display.
- The scripted spot: `city-stand` (free roam, board, look round the capital from the cockpit for 40 s): the densest
  scene, and the one with the most lamps and windows in reach. The clock held by `CB_HOUR` (12 noon, 0 midnight).
- Variants interleaved A, B, then B, A, two runs each; the frame logs (`CB_FRAME_LOG`) give p50, p95, p99 and the share
  over 16.7 ms; the HUD's `perf:` lines give the GPU's own time.

## 2. The suspects and what each instrument switches off

| Variant | Switch | What it leaves out |
| --- | --- | --- |
| noon | `CB_HOUR=12` | the night |
| midnight | `CB_HOUR=0` | nothing |
| midnight, no lamp lights | `CB_HOUR=0 CB_NO_LAMPS=1` | the lamps' real spot lights (the heads still glow) |
| midnight, no lit windows | `CB_HOUR=0 CB_NO_WINDOW_LIGHTS=1` | the windows' night light |
| midnight, no shadows | `CB_HOUR=0 CB_NO_SHADOWS=1` | the moon's shadows (and every other) |
| midnight, no effect lights | `CB_HOUR=0 CB_NO_FX_LIGHTS=1` | the effects' point lights |

## 3. Results (2026-10-29, bin/night1.exe, `output/perf/night`, `tools/perf/night_report.py`)

`city-stand`'s 40 s look round from the cockpit, two rounds interleaved (round 2 in reverse order):

| Variant | p50 ms (r1, r2) | p95 ms (r1, r2) | over 16.7 ms (r1, r2) | GPU ms (r1, r2) |
| --- | --- | --- | --- | --- |
| noon | 13.5, 16.7 | 16.8, 21.8 | 5.2%, 49.2% | 2.4, 2.5 |
| midnight | 13.7, 14.5 | 16.8, 18.8 | 5.3%, 18.0% | 7.1, 2.4 |
| no lamp lights | 14.0 (disturbed), 15.1 | 20.8, 19.1 | 12.6%, 24.7% | 2.4, 2.4 |
| no lit windows | 13.6, 14.4 | 16.6, 20.9 | 4.6%, 18.4% | 8.0, 2.4 |
| no shadows | 14.0, 12.2 | 18.6, 15.0 | 13.3%, 1.6% | 1.4, 1.5 |
| no effect lights | 15.4, 13.3 | 20.3, 15.4 | 33.5%, 1.3% | 3.0, 2.4 |

- **No result.** The spread between a variant's two runs (noon: 13.5 against 16.7 ms p50) is larger than any
  difference between variants, so nothing here is a measured night cost at this spot. The frame is CPU-paced at about
  14 ms whatever the GPU does.
- **The sitting drifted:** in both rounds the later runs were slower, whatever their variant.
- **The lamps' GPU cost did not repeat:** 7.1 against 2.4 ms in round 1, 2.4 against 2.4 in round 2. A lead, not a
  result.
- `no-lamps` round 1 overlapped a 10 s lint (a broken rule): its frame numbers are not used.
- A build-then-batch job stopped before these runs may have carried on beside them (a stopped job's inner loop
  survives on this machine): the city runs may have shared the machine, one more reason they are no result.

**The owner's own run** (bin/plain17.exe, free roam continued from a save the scripted runs had written, then the
clock to 00:00): 83 pipelines compiled after `GAME_READY`, 35 of them in its first second (the 2.1 s first frame; the
warm-up misses them, day or night) and 16 in the 30 s after the clock went to midnight. Those 16 are not night
variants: 14 particle pipelines (hanabi init, update and render), the rocket plume and a forward decal, compiled at the
first shots after the switch. They are the effects warm-up's misses (`fx::warm_up_streams`), day or night.

## 4. Next

- Warm the pipelines the warm-up misses: the 35 of the first frame and the effects' (every recipe's particle
  pipelines, the plume, the decals), so the first second and the first shots do not stall. Not a night cost, but in
  the owner's night run.
- Measure where the owner was: the run they continued was most likely a suburb (`suburb-house`), which is already the
  slower scene; the same noon and midnight variants there, three rounds, with the sitting's drift checked by a noon
  run first and last.

## 5. The suburbs at night: the moon's shadows

`suburb-stand` (free roam, board, jet to the nearest houses, land, look round from the cockpit 40 s, beat 6),
`output/perf/night/suburb-stand`, three rounds:

| Variant | p50 ms | over 16.7 ms | GPU ms |
| --- | --- | --- | --- |
| noon | 13.7, 14.7, 14.8 | 5.2%, 11.4%, 10.7% | 1.5 |
| midnight | 16.2, 15.2, 14.8 | 41%, 20%, 10% | 1.6 |
| no lamp lights | 15.1, 16.6, 17.9 | 17%, 47%, 76% | 1.6 |
| no shadows (midnight) | 13.5, 13.3, 13.4 | 6.0%, 5.1%, 4.2% | 3.8, 6.5, 1.7 |

- Shadows are the one steady cost: switched off at midnight they gave the fastest runs of every round (13.4 ms p50,
  4 to 6% over 16.7 ms), faster than noon too, so about 1.5 ms of the frame on the CPU at any hour (the GPU's time does
  not move).
- Midnight against noon is not a result: 2.5 ms slower in round 1, 0.5 in round 2, level in round 3.
- The lamps' real lights switched off were slower, not faster (15.1 to 17.9 ms): not chased; they are not the cost.
- The moon casts with the sun's own cascades (3, to 700 m). Only one of the two casts at a time. What differs is the
  angle: at midnight the moon is low, and a directional light's cascade boxes run back toward the light, so a low one
  takes in far more casters (every house, tree and patch of grass along its line) than the noon sun's short boxes.
  Each caster is extracted, culled and queued on the CPU per cascade.

**Plan** (provisional, `NF2`):
- The moon's shadows from data, apart from the sun's: `moon_cascades` 2 and `moon_far_m` 250 (moonlight is 35% of the noon
  sun's on screen, `moon_full_share`; its shadows matter most near the eye).
- If a low sun costs the same at dawn and dusk (a `dusk` variant, `CB_HOUR` at the sun's last 10 degrees, is the
  check), the sun's cascades shorten as it nears the horizon by the same rule.
- Measured old against new at midnight in the suburbs and the capital, three rounds.

## 6. The moon's own cascades, measured (old against new)

`tools/perf/ab_night.sh`, `suburb-stand` at midnight, three rounds interleaved, the old build (bin/night1.exe, the
moon on the sun's 3 cascades to 700 m) from a worktree of its commit with its own data (`CB_ROOT`), the new (bin/night2.exe,
`moon_cascades` 2 to `moon_shadow_far_m` 250), one game at a time (`output/perf/night/ab-suburb-stand`):

| Round | p50 ms old, new | p95 ms old, new | over 16.7 ms old, new |
| --- | --- | --- | --- |
| 1 | 14.65, 13.88 | 17.46, 16.91 | 9.8%, 5.4% |
| 2 | 13.77, 13.77 | 16.64, 16.42 | 4.7%, 4.1% |
| 3 | 13.53, 12.91 | 16.37, 15.27 | 4.0%, 1.4% |

The new build is as fast or faster in every round and has fewer frames over 16.7 ms in every round, but the gain (0
to 0.8 ms of p50) is about the spread between rounds: a small gain in a steady direction, not a proven one. Kept: the
moon's shadows end at 250 m, which reads the same from the cockpit. The night still has frames over 16.7 ms (1.4 to
5.4%): the rest of the shadow cost is the sun's and the moon's alike (section 5) and is `city-frame`'s work.

(An earlier attempt at this A/B is discarded: a stopped batch's loop survived on this machine and ran beside it.)

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| NF1 | How many real lamp lights the night keeps | keep 96: their CPU share did not repeat (section 5) | provisional |
| NF2 | How far and in how many cascades the moon casts | 2 cascades to 250 m (built; section 6) | provisional |
