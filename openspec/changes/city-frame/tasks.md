# Tasks: city-frame

- [x] 1.1 Measured: `city-stand` with each part switched off (probe build, 3070)
- [x] 1.2 Grass `shadow_lods`, with validation
- [x] 1.3 Cars: `CarDetail` ranges and shadows, `lod_fade::out_to`, tests
- [x] 2.1 Buildings: `rooms_radius_m`, `dress_radius_m`, `live_far_m` tried on `city-stand`; dressing to 150 m chosen, interiors kept at 250 m (F2)
- [x] 2.2 Release builds old against new interleaved, `city-stand` and `city-cruise`; `docs/benchmarks/2026-10-28-city/`
- [x] 2.3 Found: the game spills 870 MB into shared memory (5.0 GB dedicated beside 2.5 GB of other programs); census of images and meshes
- [ ] 3.1 Video memory: compress or shrink the cockpit's 4096 px maps and the largest sets (write-up first; the owner's 4096 px rule)
- [ ] 3.2 Then the CPU: render world 15 ms (draws, mesh uploads), traffic 4 ms, the screens cube
