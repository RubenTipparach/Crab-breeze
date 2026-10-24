# Design: cockpit-overlay

- `pilot::spawn_camera` adds a child camera: `Camera { order: 1, clear_color: None }`,
  `Camera3d` (its depth is cleared, so it starts empty), the same `Projection`,
  `RenderLayers::layer(1)`, `Msaa::Off` and FXAA like the main camera. The main camera keeps layer
  0. Its transform is identity under the eye camera, so the two can never disagree.
- `mech::cockpit_layer` (Update, after posing): while the pilot is seated, every `Mesh3d` under the
  piloted mech's root carries `RenderLayers::layer(1)`; otherwise none does. It walks the root's
  descendants each frame (a few hundred entities, microseconds), so a glb that loads late is caught.
- Lighting: see `cockpit-lighting` (the sun stays on layer 0; the cockpit pass has its own lights).
- Cost: one more camera pass over the cockpit's meshes only (the stand-in's 39 boxes, a glb
  cockpit's tens of thousands of triangles), and its shadow cascade; measured with `--bench`.

## Open questions

None: the owner described the approach.
