# Design: dress-off-thread

## 1. The jobs (`cb-app::buildings`)

- `DressJobs`: building id to a `Task<Vec<(Look, Mesh)>>`, in a `BTreeMap` (stable order).
- Each frame `dress_near_player`:
  1. takes the finished jobs, in id order. A building still near, still standing and still without
     a frame gets its parts (`show_parts`) and its plain box hidden. Otherwise the result is dropped
     (a framed building is dressed by `redraw_buildings`, as today).
  2. starts jobs for the nearest undressed buildings within the dressing radius, at most
     `DRESS_JOBS` (4) in flight. The job gets a clone of the building's cells and the dressing
     config; `dressing_meshes` is unchanged and is the one implementation.
  3. drops the jobs of buildings left behind, and undresses as today.
- `DressingSettled` (the stills' and captures' wait) is true only with nothing left to dress and no
  job in flight.

## 2. Cost

| Item | Before | After |
| --- | --- | --- |
| Main thread per dressed building | 12-15 ms mean, 45 ms worst | adding the meshes and materials |
| Worker threads | idle | the same meshing, spread over the pool |
| Memory | | up to 4 buildings' cells cloned while in flight |

Measured with the perf suite, before against after, interleaved, on screen.

## Open questions

None.
