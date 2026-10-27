# Handoff: the Mac session, paused 2026-10-26

The owner, 2026-10-26: "pause everything we'll pick up again later, document what needs to be done".
Every agent is stopped. No game, build or bake is running. The Mac lock (`~/.cb-mac-lock`) is free.
Nothing below is pushed unless it says so.

## Where the work lives

Each helper worked in its own git worktree of `~/Crab-breeze`. They are all merged into one session
branch, `li-verify`, and only that branch is pushed (CLAUDE.md: one session, one PR).

| Worktree | Branch | Last commit | State |
| --- | --- | --- | --- |
| `~/cb-li-verify` | `li-verify` | 5a2b537 | **Mid-merge of `li-roads`, one conflict:** `crates/cb-app/src/capture.rs`. 20 commits ahead of `origin/li-verify` (9d4332e, the last push). PR #77 from this branch is already merged, so the next push needs a new PR. |
| `~/cb-li-cars` | `li-cars` | 616b1d0 | Clean. Not yet merged into li-verify since 0812ac1. |
| `~/cb-li-vat` | `li-vat` | a83e919 | Clean. Not yet merged into li-verify. |
| `~/cb-li-roads` | `li-roads` | 011f5ea | 5 files uncommitted (the tyre-track triangle cut, half done). |
| `~/cb-li-perf60` | `li-perf60` | c1b1713 | Clean. Not yet merged into li-verify. |
| `~/cb-li-traffic` | `li-traffic` | 82ae1df (main) | `crates/cb-core/src/traffic.rs` touched, nothing committed. |
| `~/cb-li-fov` | `li-fov` | this file | The FOV change, the 60 fps rule and `tools/perf/mac_lock.sh`; merged everywhere. |

## Main is red

PR #84 (owner: "car should be 1000m") set `spawn_within_m` 1000 and `despawn_beyond_m` 1200 in
`data/world/traffic.ron` without updating cb-core's tests. Four fail on main itself:

1. `the_shipped_traffic_data_is_the_seven_1980s_kinds`: still asserts 1500. Update to the owner's 1000.
2. `the_lanes_fill_to_one_car_per_60_m_in_town_and_300_m_on_the_highway`: 4 highway cars for 8.1
   expected. Find out whether the fill or the test is wrong; do not just widen the tolerance.
3. `with_lights_on_the_arterials_traffic_never_deadlocks`: car 455 stood 174 s on lane 49 (seed 9).
   Treat as a real deadlock. Prime suspect: spawn and despawn are now 200 m apart (was 500), so a car
   can vanish while it holds a junction grant (`grant_turns` in `crates/cb-core/src/traffic.rs`).
4. `seed_11_drives_the_golden_traffic`: update the golden only after 2 and 3 are fixed.

The full checks on main (82ae1df) were asked for by the desktop session and **never ran**.

## To finish, in order

1. **Main's traffic tests** (above), on `li-traffic`; merge into li-verify.
2. **Finish the li-verify merge of li-roads**: resolve `capture.rs` (both sides added stills
   instruments: keep both), gates, commit.
3. **Roads** (`li-roads`): finish the uncommitted tyre-track cut (198,048 triangles at seed 42, target
   under 30,000 by merging pieces along straight runs) and the highway patches (`patch_chance` 0.04,
   varied lengths; they read as an 8 m grid from the cockpit). Re-shoot the cockpit and highway stills.
4. **Cars** (`li-cars`), still open from the owner's asks:
   - done: taller cars (`car_height_scale` 1.18), cars on the road surface, third-person driving
     (CLAUDE.md's LAW has the exception), people and the pilot standing on the road and pavement tops,
     getting in only offered on a stopped car (616b1d0);
   - open: a car pulled from the kerb runs into the car parked ahead, and the logs show "driving at
     0.0 m/s": find why and fix; the HUD speed readout while driving (DC2, provisional); a still of
     the camera drifting back behind the car (DC1).
5. **People** (`li-vat`): the rebake is done (lower poly near about 3,800 and 3,300 triangles, crowd
   about 1,000 and 700, hairstyles, a real wave, closed mouths, flat clothes with a clear hem, both
   faces drawn at collars and cuffs). Open: the windowed A/B with the full crowd, which must show
   every frame under 16.7 ms. The viewer is published at
   https://claude.ai/artifact/58Q4zQai6UWyymW7ebn9RM (republish from `~/cb-li-vat/output/people_vat/people.html`).
6. **60 fps** (`li-perf60`): the gate exists (`--perf-gate`, `PERF_GATE` line, exit 3,
   `tools/perf/perf_gate.py`). Next: the release baseline on `streaming-route` and `free-roam-fight`,
   every frame over 16.7 ms listed by cause, then fix biggest first: mesh uploads (a byte budget per
   frame), pipeline compiles moved into the loading screen, audio starts off the main thread, the GPU
   waits. Report to `docs/benchmarks/2026-10-26-60fps-mac/`. Then the desktop's 3070 release rerun
   (it agreed not to touch mesh upload code).
7. **Verification** (`li-verify`): the owner's "No breaks until all items verified". Still to play and
   report in `docs/playtests/`: the civilians and cars on foot, aircraft and harbours, the cockpit and
   weapons items, the Stratostacean deck, far buildings, music, and rockets at the ground. Found and
   fixed so far: terminal clicks in a career, a crash on Continue in a bought mech, the scripted jet
   leg, wheel transforms (1,596 writes a frame to under 1).
8. **Merge everything into li-verify**, run all gates, merge origin/main again, push, open the new PR.

## Open questions for the owner

- **V1** in the survey (https://claude.ai/artifact/EbeWFXQeCbcM7ok3tsp64h): how the field of view is
  measured. Built provisionally: on foot and driving 75 degrees across (about 47 tall on 16:9), the
  cockpit 95 tall. `on_foot_fov_across` in `data/pilot/pilot.ron` switches it back.
- Not yet in the survey: DC1 (the chase camera drifts back after 2 s), DC2 (speed readout), R2 (a
  grass set for parks), R3 (a tint on runways).

## Rules learned this session

- Every frame above 60 fps in every perf run, and one game, bake or release build on the Mac at a time
  through `tools/perf/mac_lock.sh` (CLAUDE.md section 7).
- Baked VAT files are about 22 MB in all and go straight into git: commit a rebake once, when its look
  is settled.
