# Proposal: dress-off-thread

## Why

Walking or flying through a city hitches. The 2026-10-25 benchmark on the owner's RTX 3070 (in the
window, vsync on, 60 Hz) missed 9-11 % of refreshes, with p95 at 31 ms on the world and fight takes.
The game's own slow-system log names the cause: `dress_near_player`, which builds a nearby building's
detail (window frames, ledges, parapets, roof props, glass) on the main thread, one building a frame,
took over 4 ms on 120-240 frames a take, 12-15 ms on average and up to 45 ms.

## What changes

- A building's dressing is meshed on the async compute pool, as the terrain's chunks already are.
  The frame that asks for it no longer waits for it; the finished meshes are added when they are
  ready, nearest building first.
- Nothing the player sees changes except when: the plain box shows until the dressed building is
  ready, a few frames later.
- A building damaged while its dressing is being built is handed to the damaged-building path as
  today, and the finished meshes are dropped.
