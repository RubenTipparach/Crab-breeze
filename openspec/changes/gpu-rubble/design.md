# Design: gpu-rubble

## 1. The layer

`Layer::Chunks { count, size_m: [min, max], speed_m_s, spread, life_s, rest_s, colour }` in
`cb-core::fx`, validated like the other layers. A recipe `"rubble_shed"` in `data/fx/fx.ron` holds
one `Chunks` layer (and the existing dust); the caller scales `count` by the chunk's volume with
`Fx::dense` (`count_per_m3`, capped by `max_count`).

## 2. On the GPU (`cb-app::particles`)

- Mesh particles: `EffectMesh` on the effect entity with one rough box mesh (a cube with jittered
  corners, made once at startup); each particle's size is random in `size_m` and its orientation
  spins at a random rate (hanabi's `OrientModifier` with a rotation attribute advanced per step).
- Gravity 9.81 m/s2 and drag 0.2 per s, as `Rubble`.
- Landing: the effect carries the ground height under the burst as a property (`ground_y`, read
  from the terrain and crater heights when the effect is spawned). An update expression keeps
  `position.y >= ground_y`; on reaching it the velocity drops to zero (scaled by
  `step(ground_y + 0.01, position.y)`), so a piece stops where it lands.
- Fading: alpha and size hold until `life_s - rest_s`, then shrink to nothing over `rest_s`.

## 3. Who sheds

`physics::spawn_chunks` writes `Fx::new("rubble_shed", centre).dense(volume_m3 * count_per_m3)`
for every chunk, near or far, one per building per frame as the far puffs are (the cap keeps a
whole storey from throwing more than `max_count`).

## 4. Cost

GPU: hanabi simulates and draws up to `max_count` pieces per burst (planned 120) as instanced
meshes of 12 triangles; a collapse of 40 chunks is under 5,000 pieces, 60,000 triangles. CPU: one
effect entity per burst, as the dust. Measured with the demolition and city combat benchmarks.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Lit pieces | unlit, tinted concrete, until a take shows they read wrong | provisional |
| Pieces landing on rubble heaps and slabs | ground and crater height only | provisional |
