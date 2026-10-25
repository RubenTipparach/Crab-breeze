# Design: blast-scorch

## 1. What a blast on soil leaves today

| Part | Where | What it looks like |
| --- | --- | --- |
| The dug bowl | `ground.rs`: crater tiles and deltas from `Live::dug` | The grass texture under a vertex colour mixed 85 % toward dark brown |
| The scorch | `ground_marks.wesl`, shape 2 | A multiply: a ragged bowl with a lip, the floor darkened toward brown |
| The blast | `fx.ron` `explosion_ground` | Soft-disc particles (`fx-sprites`) |

## 2. The graphs

Both in `tools/materials/ptex/`, written by `tools/materials/build_ptex.py` (Star Crew's `Graph` helper) and rendered
by the `material-maker-headless` skill at 2048 px. Ports: albedo, normal (OpenGL, green up), ORM (R occlusion, G
roughness, B metal 0), height; `blast_scorch` also writes alpha.

- **`blast_scorch`** (one blast, 2048 px over the decal's square):
  - radial distance `r` from the centre, warped by fbm so no edge is round;
  - centre (`r < 0.25`): black char crust, Voronoi cracks with paler ash in them, roughness 0.95;
  - ash ring (`0.2 < r < 0.45`): grey-white powder, patchy (fbm threshold), roughness 1.0;
  - churned ring (`0.35 < r < 0.7`): dark wet-looking soil and clods, height up for the lip, roughness 0.8;
  - streaks (`0.4 < r < 1.0`): 40 to 60 radial rays of soot and thrown dirt, thinning outwards;
  - alpha: 1 inside `r < 0.6`, falling off with the streaks to 0 at the ragged edge.
- **`churned_soil`** (tiles every 2 m): broken earth, clods by Voronoi, darker in the gaps.

## 3. The decal

- Bevy 0.20's forward decal (`ForwardDecal`, `ForwardDecalMaterial<StandardMaterial>`) needs the camera's depth
  prepass. The pilot camera already has one (`water_render.rs` requires `DepthPrepass`).
- Material: base colour with the albedo and alpha, normal map, ORM as occlusion plus metallic-roughness, alpha
  blended, `depth_fade_factor` 1.5 m. One shared material, so every scorch batches into one draw.
- Each new crater on soil (`Live::dug`, surface `Dirt` from `WorldPlan::surface_at`) spawns a decal:
  - centred on the blast;
  - square side `scorch_per_radius * crater radius`, 2.6 by default;
  - projection depth: the crater's depth plus 2 m;
  - a random yaw from the blast's id;
  - held for the crater's life (`ground_marks` craters, 240 s), the oldest despawned past `max_scorches` (64).
- The `ground-marks` crater on soil is not laid any more; on paving it stays.
- Numbers in `data/fx/battle_fx.ron` (`scorch`: `scorch_per_radius`, `max_scorches`, `depth_fade_m` 6 m to reach the bowl, `life_s`), validated.

## 4. The pipeline

| File | Does |
| --- | --- |
| `tools/materials/build_ptex.py` | Writes the graphs (`Graph`: nodes, links, `material()`), from Star Crew |
| `tools/materials/export_materials.sh` | Renders every graph with the headless skill into `tools/materials/raw/` (gitignored), then runs the post-process |
| `tools/materials/postprocess.py` | Packs ORM, writes `assets/textures/materials/<name>/{albedo,normal,orm,height}.png`, the contact sheet, a `SOURCE.md` with the Material Maker revision |
| `data/materials/materials.ron` | Every material: graph, size, tile metres, use |

## 5. Costs

| What | Cost |
| --- | --- |
| Draws | 1 for every scorch, batched (one mesh, one material) |
| Fragments | One decal pass over the pixels a scorch covers. A 15 m crater 50 m away covers about 40 000 px at 1080p |
| Texture memory | `blast_scorch` 2048 px, 4 maps as RGBA8 with mips: about 85 MiB. `churned_soil` 1024 px: about 21 MiB. Compression (BC7 in KTX2) would cut it by four, later |
| CPU | One spawn per crater; nothing a frame |

Estimates. A cloud session measures nothing (CLAUDE.md section 7).

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| X1 | Does the scorch stay for the whole mission, or fade after 4 minutes? | Fade after 4 minutes, 64 at most |
| X2 | Does a rocket on paving get a textured scorch too? | Later: soot on concrete is its own graph |
