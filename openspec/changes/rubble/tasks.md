# Tasks: rubble

## 0. Mockup
- [x] 0.1 Heap heightfield with repose, pieces settling on it, pool overflow into the heap, dust
- [ ] 0.2 Owner answers W13

## 1. Core
- [x] 1.1 Positive deltas in the deformation tiles, `max_rubble_m` (test: `rubble_raises_the_ground_under_it`)
- [x] 1.2 Deposit and repose (test: `a_heap_never_stands_steeper_than_its_angle_of_repose`); a tower's heap raises in 0.7 ms (release, M4 Max, indicative)
- [x] 1.3 Determinism (test: `the_same_collapse_lands_the_same_heaps`, in `destruct.rs`)

## 2. App
- [x] 2.0 Heaps drawn with a rubble vertex colour (broken concrete with brick flecks, blended in from 0.2 m to 0.6 m deep) until the material layer exists
- [ ] 2.1 Rubble material layer and instanced dressing
- [ ] 2.2 Dust clouds
- [ ] 2.3 Playtest: climb the warehouse's rubble on foot, stride over it in the mech; on Drive
