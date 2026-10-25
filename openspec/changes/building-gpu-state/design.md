# Design: building-gpu-state

## 1. Element index per vertex

`MeshBuilder` already writes a vertex colour (a tint, alpha unused and 1). For building meshes the
alpha carries `slot`: 0 means always shown, `n > 0` means element `n - 1` of the building's frame.

| piece | slot |
| --- | --- |
| column, slab, wall (frame element `id`) | `id + 1` |
| partition or furniture resting on slab `(k, i, j)` | that slab's id + 1 |
| partition or furniture on the ground | 0 |
| a skin piece anchored to an element | that element's id + 1 |
| a skin piece on the whole building, or any piece of a building with no frame yet | 0 |

An f32 holds integers exactly to 2^24, far beyond a tower's element count (about 3,000-6,000).

## 2. The state buffer (as built: one for every building)

As built, every damaged building has a range of one shared `ShaderBuffer` (`BuildingStates`,
64 Ki words to start, doubled when full); a slot is the range start plus the element id plus 1, and
a change is written into the GPU buffer in place from the render world (`RenderQueue::write_buffer`),
so no asset and no material changes. A first version with a buffer and materials per building was
three times slower in combat (its draws could not batch): `docs/benchmarks/2026-10-24-gpu-state-mac/`.


`ShaderBuffer` per framed building: `array<u32>`, element `id` at index `id`, 1 standing, 0 not.
Rewritten from `Frame::elements` when the building is dirty (the same events that rebuilt bands):
a 60-storey tower is about 20 KB. Buildings with no frame share one buffer of a single 1.

## 3. The material

`BuildingMaterial = ExtendedMaterial<StandardMaterial, ElementState>`, `ElementState` holding the
buffer at binding 100. Its `vertex_shader` (`assets/shaders/building_state.wesl`) and
`prepass_vertex_shader` (`building_state_prepass.wesl`) are Bevy 0.20's `mesh.wesl` and
`prepass.wesl` vertex functions cut to what building meshes use (positions, normals, UVs, tangents,
colours; no skinning, morphs or motion vectors), plus: read the slot from the colour's alpha, and
if its element is down, place the vertex outside the clip volume (all three corners of a broken
triangle land on one point, so it rasterises nothing); the colour's alpha is set back to 1. The
fragment stays Bevy's PBR. One material per look (facade tone, skin material, surface), shared by every building; the base
`StandardMaterial`s are the existing ones.

## 4. Lifetime

- Dressed, no frame (a whole building near the player): skin and rooms meshed with slot 0, shared
  materials with the shared all-standing buffer, as now.
- First damage (the building gains a frame): frame, rooms and furniture meshed once per surface with
  slots, the facade panels as their own mesh (hidden while the building is dressed), the skin
  re-meshed once with slots, the building's buffer created and its materials made.
- Every later change: the buffer rewritten. Nothing else.

## 5. Cost

- CPU: a damaged building's meshes are built once, on its first damage (as the bands were on
  theirs); every later hit writes one buffer. Expected: `redraw_buildings` and `dress_near_player`
  slow frames in combat close to 0.
- GPU: broken elements' vertices still run the vertex shader (one buffer read each); nothing
  rasterises. A building's mesh is about as big as its bands were.
- Draw calls: one per building per look, about as many as its bands had.
- Measured with the city combat benchmark against `docs/benchmarks/2026-10-24-surfaces-mac/`.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Mesh the first-damage build off the main thread | only if a trace shows it | provisional |
| One global buffer so buildings share materials and batch | later, with `gpu-instancing` | provisional |
