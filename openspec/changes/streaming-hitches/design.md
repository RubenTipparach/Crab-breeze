# Design: streaming-hitches

The measurement is `docs/benchmarks/2026-10-26-streaming-mac/report.md`.

## 1. Ambience beds: the skip off the main thread

`ambience::BedLoop` is a `Decodable` asset made per bed start from the loaded `AudioSource` (an `Arc` of its bytes, no
copy) and the start point. Its source, `LateSkip`, gives one silent sample for every 16 it reads and throws away until
it reaches the start (`SKIP_PER_SAMPLE`), then the recording's samples, and builds a new decoder at the end to go
round again from the beginning (Bevy's `skip_duration(start).repeat_infinite()` looped from the start point instead).
The bed is spawned with `PlaybackSettings::ONCE`, as it loops itself.

Cost: the decoder is built on the main thread as before (under 1 ms, measured 0.2 to 0.8 ms for these files); the skip
runs on the audio thread at 16 times real time, about 1.9 s of silence for a 30 s start. A bed starts silent and eases
in over `ease_s`, so the silent lead-in is not heard.

## 2. Trees: three rings

Nearest first, per fixed step: every wanted tile within `must_radius_m` (60 m, about one to four 264 m tiles), then at
most `urgent_tiles_per_step` (4) within `urgent_radius_m` (400 m), then at most `tiles_per_step` (2) to the keep radius.
At 100 m/s a body crosses 1.7 m a step: the 60 m ring holds everything it can touch for a second, and the 400 m ring
(about 16 tiles) completes in 3 to 4 steps. Deterministic: the budget counts fixed steps and the order is by distance
then key, as before.

Cost: the worst step for a body new to a place, 4.1 to 7.8 ms before, 1.4 to 4.0 ms after (M4 Max, headless).

## 3. Cars: change detection

`draw_cars` builds the car's `Transform` and sets it with `set_if_neq`; `spin_wheels` writes a wheel's rotation only
when it differs. A parked car, or one waiting at a light, is then not walked by the transform propagation. Cost: none;
transforms written a frame on the route 362 to 377 after, 503 to 586 before.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| S1 | How far is the trees' must-have ring? | 60 m (a second at top speed, with room) | provisional, recommended (owner: "Use recommended for survey") |
| S2 | How fast does a bed catch up to its start? | 16 samples skipped a sample (under 2 s for 30 s, inaudible under the fade-in) | provisional, recommended |
