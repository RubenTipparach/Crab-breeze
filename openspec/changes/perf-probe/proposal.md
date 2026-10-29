# Proposal: perf-probe

## Why

The owner (2026-10-27): "our target is constant 60fps through the whole time", "city destructions can cause massive
frame drops, as well as subrban housing destruction", "develop an mcp server if you have to to get perf in real time,
and analyze events in side the game caausing bottle necks". The `hitch:` log lines name the systems in a slow frame
after the run; nothing says, while the game plays, what happened in the game on that frame (a building falling, a
blast, debris spawned) or what the world holds (physics bodies, meshes, entities).

## What changes

A measurement instrument only (CLAUDE.md section 4); no game behaviour changes.

1. **The probe** (`crates/cb-app/src/perf_probe.rs`, cargo feature `perf_probe`): the game keeps its last 8 000
   frames (length, main-world share, the systems timed in it, the counts, the game events, the eye's position and
   speed) and answers plain HTTP on `127.0.0.1:7878` (`CB_PROBE_PORT`): `/summary?s=N`, `/frames`, `/events`,
   `/census`.
2. **Game events** written with `probe_event!` (formatted only in a probe build): each blast (energy, place, buildings
   and houses broken, chunks), each step whose buildings changed or shed chunks, each frame debris spawned (chunks
   offered, bodies made, far puffs, bodies moving and resting).
3. **An MCP server** (`tools/perf/probe_mcp.py`, stdio JSON-RPC, no dependencies): `perf_summary`, `perf_events`,
   `perf_frames`, `perf_census`, so a session asks a running game directly.
4. **The scripted pilot** gains `FireAtNearest(kind, seconds, n)` (brings down the nearest buildings or houses) and
   `house` and `farthest-city` destinations; three scripts: `city-destruction`, `suburb-destruction`, `fast-flight`.
