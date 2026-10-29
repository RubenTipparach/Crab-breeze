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

**The owner's own run** (bin/plain17.exe, free roam continued from a save the scripted runs had written, then the
clock to 00:00): 83 pipelines compiled after `GAME_READY`, 35 of them in its first second (the 2.1 s first frame; the
warm-up misses them, day or night) and 16 in the 30 s after the clock went to midnight (night-only variants: the warm-up
runs in daylight).

## 4. Next

- Warm the night's pipelines too: the warm-up renders its copies once by day and once by night (moon shadows, lit
  windows, lamp lights), and the 35 first-frame pipelines are found and added.
- Measure where the owner was: the run they continued was most likely a suburb (`suburb-house`), which is already the
  slower scene; the same noon and midnight variants there, three rounds, with the sitting's drift checked by a noon
  run first and last.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| NF1 | How many real lamp lights the night keeps | decided by the numbers in section 3 | open |
