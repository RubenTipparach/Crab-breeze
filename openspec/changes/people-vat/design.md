# Design: people-vat

## 1. The body (`tools/blender/build_people.py`, table `tools/blender/people.json`)

The humanoid pipeline (`blender-humanoid-characters`), after Undercity's `build_npcs.py` (fps-game-demo 1363650):

- **Packs**, pinned by SHA-256 in `tools/deps/character_packs.json` and fetched by `tools/deps/fetch_character_tools.py`
  (borrowed from Undercity) into `~/.cache/crab-breeze/deps`, never the repository:

  | Pack | Licence | Size |
  | --- | --- | --- |
  | MPFB 2.0.17 (extensions.blender.org) | code GPL-3.0-or-later, runs at build time only and never ships; its data CC0; output unclaimed | 45 MB |
  | MakeHuman system assets, CC0 pack | CC0 1.0 | 281 MB |
  | Universal Animation Library, Standard (Quaternius), the 2025-03-25 build's glb as Undercity ships it in its public repository at 1363650 | CC0 1.0 | 6.7 MB |

  Quaternius' own page serves only a newer build now, so the UAL pin is Undercity's committed copy at a fixed commit.
- **Two bodies**, `man` and `woman`, each MPFB's macro sliders, its low-poly proxy (`male1591`, `female1605`), CC0
  clothes (a casual suit and shoes) and the low-poly eyes, with the 53-bone `game_engine` rig. Height is fitted to
  1.75 m by bisection on MPFB's height slider (built: 1.747 m and 1.746 m); the game scales each person to their own
  1.55-1.95 m. The pilot is 1.8 m: a person at the table's height stands 5 cm shorter.
- **Hair styles, several a body** (owner, 2026-10-26: "need a few different hairs"): the man `short02`, `short01`,
  `short04` or bald, the woman `ponytail01`, `braid01`, `long01` or `short02`, all CC0, none over the face (the bob,
  `bob01` and `bob02`, hung over an eye and was dropped). Every style is baked into the body's own texture as its own
  vertices, numbered in each vertex (`hairvar`), and a person draws one: the shader drops the others. Chosen over a
  body per hair (four times the texture) and over rigid hair on the head bone (the long hair and the ponytail swing
  with the neck and shoulders).
- **Colour by region, not by texture.** Every vertex carries a region (skin, top, bottom, shoes, hair, eyes) and a
  shade; the colour is the person's palette at run time (section 4). A garment's loose parts (a shirt, a pair of
  trousers) each take one region, by the bones that move most of their vertices.
  **Why the shirt blended into the trousers** (owner, 2026-10-26: "why their shirt blend in with pants"): the region
  was picked per vertex by the bone that moves it most, so the shirt's lowest vertices, moved by the pelvis, were
  coloured as trousers, and the 89 to 146 triangles along each hem that had both kinds of vertex faded from one
  colour to the other. Now a whole loose part is one region (no triangle mixes the two), the top's open edges (hem,
  cuffs, collar) are a darker band, and a person's top and bottom always differ in luminance by
  `draw.min_top_bottom_contrast` (0.08).
  **Why the trousers had blotches**: the shade was the CC0 texture's brightness under each vertex, so the jeans'
  printed wear and stains became dark patches the size of a triangle. Clothes, shoes and hair are now flat (shade 1)
  but for the hem band; only skin keeps the texture's shade, at half its contrast and smoothed, and the eyes their
  iris.
- **The mouth closed**: the proxy's lips and the mouth tube behind them were open loops with nothing behind (the
  build adds no teeth or tongue), so a face showed a hole; the build fills the open loops on the face's middle line.
  The eyes are the low-poly eyeballs in their sockets, coloured by the eyes region (`draw.palette.eyes`).
- **Levels of detail** (owner, 2026-10-26: "lower poly characters"; the crowd's look kept: "crowd is great"): each part
  goes through a Decimate collapse at its kind's ratio with its open edges held (`hold_edges`), so a sleeve's cuff stays
  on the arm and the skin under it, and each hair style to about `hair_triangles`:

  | Level | Ratios (skin, eyes, clothes, shoes) | Hair | Man drawn / in file | Woman drawn / in file | Budget |
  | --- | --- | --- | --- | --- | --- |
  | near | 0.6, 1.0, 0.25, 0.1 | 400 | 3,770 / 4,570 | 3,282 / 4,479 | 4,000 |
  | crowd | 0.1, 0.25, 0.05, 0.03 | 140 | 1,010 / 1,290 | 706 / 1,124 | 1,100 |

  "Drawn" is one person (the body and its largest hair style); the budget is held to it, read back from the written
  file; the manifest carries it and a test refuses it again (section 6). The crowd level is at about the triangles the
  owner liked; the held cuffs and hems keep the clothes from going lower.

## 2. The clips

UAL's clips retargeted onto the `game_engine` rig in the build: for every mapped bone (the map is Undercity's, both
rigs to Godot's humanoid profile), the target's rest direction is first turned onto the source's (UAL rests in a
T-pose, MPFB in an A-pose), then takes the source bone's rotation from its rest in world space; the hips move by the
source hips' motion, scaled by the two hip heights. A bone not mapped keeps its rest relative to its parent.

| Clip | Source | Frames | Loops | Notes |
| --- | --- | --- | --- | --- |
| idle | `Idle_Loop` | 60 | yes | waiting at the kerb, standing about |
| walk | `Walk_Loop` | 32 | yes | walks 0.95 m/s (man) at its own rate |
| run | `Sprint_Loop` | 16 | yes | 3.71 m/s |
| sit | `Sitting_Idle_Loop` | 40 | yes | pelvis 0.55 m up |
| wave | `Idle_Loop` with the right arm authored | 48 | no | eased in and out of the idle (0.35 s); upper arm raised 110 degrees out to the side, elbow bent near a right angle with the forearm 38 degrees in from upright, swinging 20 degrees either side at 2 Hz; the hand open (fingers straight), palm to the front, the wrist following 0.06 s late; weight 2.5 cm onto the left foot. UAL has no wave. Contact sheet `docs/screenshots/people_vat/wave-sheet.png` |
| talk | `Idle_Talking_Loop` | 70 | yes | the answer after the wave |
| fall | `Death01` | 58 | no | falls backwards and holds the last frame lying |

The speed a moving clip walks at is measured in the build (the planted foot's backward speed, averaged over the
loop) and written to the manifest per body, so the cadence rule never uses a typed-in number.

## 3. The bake

Every clip is sampled at its source's 24 frames a second, evenly over its length (a loop's last sample is not its
first again), one texture row per frame: 326 rows. Per level, a `.vat` file (`cb_core::people_vat`, which reads and
checks it): the triangles, each vertex's region and shade, and per row per vertex four `u16`.

- **Quantised with bounds, not float.** Positions are 16 bits a component over the level's bounds (about 2.4 m across
  at most: 0.04 mm a step, finer than half floats' 1 mm at 2 m), and the normal is octahedral, 8 bits a component, in
  the fourth: one `Rgba16Uint` texel (8 bytes) a vertex a frame, one fetch for position and normal, half the memory of
  separate half-float position and normal textures. `textureLoad` only (no filtering is wanted across vertices); the
  shader interpolates between frames itself.
- Sizes: near 3,257 x 326 x 8 B = 8.3 MB (man), 8.1 MB (woman); crowd 3.0 and 2.6 MB; 22 MB in all, every hair
  style included (the first bake, the whole body with one hair, was 21 MB a body). Committed; the generator's
  `--verify` rebuilds every file and fails unless it matches byte for byte.
- `assets/models/people/people_vat.json` is the manifest: each clip's row, frames, exact rate, loop and per-body speed;
  each body's levels with counts, bounds, bytes, SHA-256 and budget; the sitting and standing pelvis heights. The glb
  (`<id>.glb`, both levels skinned with every clip) is the source the review page shows and the test checks the
  manifest against.

## 4. The renderer (`crates/cb-app/src/people_vat.rs`, shaders `assets/shaders/people_vat*.wesl`)

- **One material per body and level** (four): `StandardMaterial` extended by `PeopleVat` (the level's texture, its
  bounds, the palette tables and the people buffer), whose vertex shader (main pass) and prepass vertex shader (depth,
  normals, shadows) read the VAT; the fragment is Bevy's PBR, unchanged, so people are lit, fogged and shadowed like
  everything else.
- **Packed 32 to an entity.** A level's draw mesh is 32 copies of the body, each vertex carrying its vertex number and
  copy in its UV and its shade and region in its colour. Each (body, level) has a fixed run of chunk entities
  sharing that mesh and material, each with a `MeshTag` naming its first slot in **one storage buffer of every
  person**: place, heading, scale, the clip playing and its frame, the clip fading out and its frame, the blend weight,
  the palette. The vertex shader reads the person by tag plus copy, the two frames of each clip, interpolates between
  frames and crossfades between clips, scales, turns and places the vertex. A chunk with nobody in it is hidden; an
  empty copy in a used chunk collapses to nothing. All chunks of a level batch into one draw: four draws in the main
  pass, four in the prepass and two per shadow cascade, whatever the crowd.
- **Per frame on the CPU**: the clip rule and phase step for each person (section 5), and the buffer (48 bytes a
  person, 96 KB at 2,000) handed to the GPU. No mesh is rebuilt, no transform written, no entity spawned per person.
- **Levels**: within `draw.near_m` (60 m) of the eye, at most `near_max` (200) people at the near level, everyone else
  at the crowd level. The near level casts shadows; the crowd level does not (at 60 m and beyond a person's shadow is
  a few pixels, and 2,000 crowd bodies in four cascades would cost about 7 M vertices a frame for it).
- **Colours and hair**: a person's `look` picks a body (man or woman), a hair style, and an index into each palette
  list: eight skin tones, eight hair colours, sixteen tops, twelve bottoms (stepped on until it contrasts with the
  top) and eight shoes (1980s workwear and street clothes, survey G1). Linear RGB, in the material's uniform; the
  eyes' whites fixed.
- **Drivers**: a driver who leaves a car runs as a person (the run clip) instead of the capsule.
- `CB_NO_PEOPLE=1` still leaves the plugin out (the people's simulation and drawing) for the benchmark's comparison.

## 5. Clips from the simulation (`cb_core::civilians::anim`, pure)

| The simulation says | Clip |
| --- | --- |
| walking (pace above 0.2 m/s), crossing | walk, at the pace over the clip's speed, within 0.5-1.8 times its rate |
| walking but still, waiting at the kerb, standing a while | idle |
| sitting on a bench | sit |
| greeted and answering | wave for the first 2 s (`draw.wave_s`), then talk |
| fleeing | run, cadence as the walk |
| struck down | fall, holding its last frame until cleared |

A change of clip crossfades over 0.25 s (`draw.crossfade_s`); walk to run keeps the feet's place in the stride.
Each person starts a loop at a point set by their id (a Weyl sequence, never the global RNG), so a crowd is not in
step. Tests: `civilians::anim::tests`.

## 6. Checks

- `people_vat::tests::the_shipped_bake_matches_its_manifest_and_its_glbs`: the manifest's rows and budgets, every
  `.vat` file's counts, rows and bounds against the manifest, each level's triangles against the glb's mesh of that
  name, every clip in the glb with a key a frame, and both bodies at 1.75 m within 2 cm.
- `a_manifest_within_its_budgets_passes_and_one_triangle_over_is_refused`.
- The shaders compile against Bevy 0.20's `bevy_pbr` in `cargo test` (`shader_check`).

## 7. Cost

| | Stand-ins (before) | VAT (after) |
| --- | --- | --- |
| Near bodies | up to 200 entities, 120 triangles each, posed each frame | up to 200 people at 3,300-3,800 triangles drawn in 7 chunk entities a body |
| Past 60 m | one mesh rebuilt 30 times a second on the CPU (0.59 ms p50 for 2,000) and uploaded | up to 2,048 a body at 765-867 triangles, 64 chunk entities a body |
| CPU a frame | the rebuild, 200 transforms | the clip rule and 96 KB buffer for 2,000 people |
| GPU memory | about 4 MB of crowd mesh | 22 MB of textures, 2 x 32-copy near meshes (about 4 MB) and crowd meshes |
| Draws | 1 crowd + batched near | 4 main, 4 prepass, 2 per shadow cascade |

Measured numbers go in section 8 when the A/B runs.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| PV1 | How people look: two bodies (a man, a woman) coloured by region from palettes, or more bodies, or textured clothes | two bodies and palettes now; more bodies (age, build, outfit cuts) are a row each in the table and a draw each | provisional |
| PV2 | Clothing variety | 16 tops, 12 bottoms, 8 shoes, 8 hair colours, 8 skin tones from the 1980s street and workwear palette, a top and bottom always at least 0.08 apart in luminance | provisional |
| PV3 | Clips beyond the set (phone, carrying, cowering, pointing at a mech) | none yet; each is a row in the table and 30-70 rows of texture | provisional |
| PV4 | The crowd level casts no shadow | no shadow past 60 m | provisional |
| PV5 | The near level's detail | about 3,300 to 3,800 triangles drawn (owner, 2026-10-26: "lower poly characters") | decided by the owner |
| PV6 | Hair styles | three and bald for men, four for women, CC0, none over the face | provisional |
| PV7 | Crowd level triangles | about 700 to 1,000 drawn: the held cuffs and hems keep it above the owner's 500 to 800 | provisional |
