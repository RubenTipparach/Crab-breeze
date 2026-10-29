# Design: perf-probe

## 1. What it measures

| Path | Answers |
| --- | --- |
| `/summary?s=10` | the last `s` seconds: frames, fps, p50/p95/p99/max ms, frames over 16.7 and 11.1 ms, the ten worst frames (each with its systems over 0.2 ms, counts, game events, eye and speed, and the next frame's length, since a main-world stall shows in the next frame with pipelined rendering), the systems most often the largest cost of a slow frame, and the census |
| `/frames?since=N&limit=M` | `[n, t_s, ms, main_ms, x, y, z, speed]` for frames after `N` |
| `/events?since=N` | each game event after frame `N` with its frame's and the next frame's ms |
| `/census` | entities by the first word of their `Name`, physics bodies dynamic, kinematic and static, colliders, meshes drawn and in view, mesh assets, point and spot lights (once a second) |

Systems are named from `perf_hud`'s tally: every system in a `hitch_spans` build, only the `slow` and `section` guards
otherwise. The probe is built with `hitch_spans` for cause-finding runs; its frame times are then the instrumented
build's (about 3 ms a frame slower on the desktop) and rank causes only. Frame-time results come from a release build
without either feature.

## 2. Cost

Outside a `perf_probe` build: one atomic load per `probe_event!` and per frame in `perf_hud::frame_start`. In one:
a frame record (under 1 KB) pushed to a ring of 8 000; the census walks every named entity, body, collider and mesh
once a second (a few ms on the second it runs, reported as `census` in the frame's systems); the server thread
answers on demand.

## 3. The pilot's new beats

- `FireAtNearest("building" | "house", seconds, n)`: picks the nearest building (the collision grid) or house lot not
  fired at yet, 60 to 800 m from the mech, aims at it (a building at 35 % of its height, a house 3 m up), holds the
  primary trigger and fires the missile key every 2.5 s, `seconds` each, `n` in all; prints `PLAYTEST_TARGET`.
- `SkateToNearest` / `JetToNearest` destinations `house` (the nearest lot more than 300 m off) and `farthest-city`.

## Open questions

None.
