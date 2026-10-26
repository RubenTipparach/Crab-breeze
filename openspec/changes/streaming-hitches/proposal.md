# Proposal: streaming-hitches

## Why

Owner, 2026-10-26: "Let's focus on getting no frames under 60fps, then we'll focus on 90 afterwards. The biggest
hitches is loading and unloading sections of the map it seems." The desktop's free-roam run on the RTX 3070 (#75) found
one-frame spikes of 20 to 50 ms while skating at 80 to 105 m/s, with no one system over the 4 ms `slow` line, and tree
stalls up to 36 ms.

Measured on the M4 Max along a fixed route across the west of the island (`data/playtests/streaming-route.ron`, every
system timed, `docs/benchmarks/2026-10-26-streaming-mac/report.md`), the main world's streaming (ground, trees, grass,
building dressing) stays under 2 ms a frame and causes no frame over 16.7 ms on this machine; the render world's upload
of new meshes does, on 1 to 9 frames a run (up to 26 ms). Three things it does name,
each a worst case that grows on a slower CPU:

| Cause | Worst on the M4 Max | When |
| --- | ---: | --- |
| An ambience bed starting 0 to 30 s into its loop: Bevy decodes and throws away everything before it, on the main thread (`play_queued_audio_system`) | 43 ms | a bed fades in (a new district, the forest, height) |
| A body new to a place (an enemy brought in, a mech dropped) grows its whole 400 m ring of trees in one step | 4 to 8 ms (16 tiles) | a job's enemies arriving |
| Every car's transform written every frame, parked or not, so the transform propagation walks every car's scene | 500 to 590 cars a frame | in and near a city |

## What changes

- **Ambience beds skip to their start on the audio thread** (`ambience::BedLoop`): silence for a sixteenth of the
  skipped time while the bed is still fading in from nothing, then the same samples Bevy's start position gave, looped
  from the recording's beginning.
- **Trees near a body grow at once, the rest of its ring a few a step**: tiles within `must_radius_m` (60 m) of a body
  at once, `urgent_tiles_per_step` (4) more out to `urgent_radius_m` (400 m), then `tiles_per_step` (2) to the keep
  radius. Data in `data/world/world.ron`.
- **A car's transform is written only when it moved** (`draw_cars`, `spin_wheels`).

## Not changed, and why

- Pipeline compiles (15 to 83 ms, about seven a run) are synchronous on macOS only: Bevy compiles pipelines on worker
  threads on Windows and Linux. Not a hitch on the owner's machine.
- Waits for the next swapchain image (`prepare_windows`, 10 to 30 ms, about twenty a run) are the M4 Max's GPU running
  over on a frame; the RTX 3070 draws a frame in about 1 ms.
- No time-slicing, hysteresis or look-ahead was added to the ground, grass or dressing streams: they already build off
  the main thread with per-frame caps and keep-margins, and none of them reached 2 ms in a frame on the route.
- The render world's upload of new meshes (`allocate_and_free_meshes`, up to 26 ms on the Mac) is left for the desktop
  rerun to size: on Vulkan and the 3070 it may cost far less, and capping it means holding finished meshes back a
  frame, which the streams' per-frame caps already partly do.
