## ADDED Requirements

### Requirement: A perf_probe build answers what its last frames cost and why, while it plays
A build with the `perf_probe` feature SHALL answer HTTP on `127.0.0.1:7878` (or `CB_PROBE_PORT`) with its last frames'
lengths, the systems timed in the slow ones, the game events on those frames and a census of the world, and
`tools/perf/probe_mcp.py` SHALL serve the same as MCP tools.

#### Scenario: A building falls during a run
- **WHEN** a missile brings down a building in a `perf_probe` build
- **THEN** `/events` lists the blast with the buildings broken and chunks made, and the debris spawned, each with its
  frame's milliseconds, and `/summary` lists that frame among the worst if it ran over 16.7 ms
