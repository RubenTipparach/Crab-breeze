# Design: fx-sprites

## 1. Today

`particles.rs` builds one bevy_hanabi `EffectAsset` per particle kind and recipe, each sampling texture slot 0 with
`ImageSampleMapping::Modulate`; slot 0 is always the soft disc (`set_sprite`), copied for blended explosion bursts so
they sort apart. Colour comes from `colour_over_life(kind)`.

## 2. The sheets

Material Maker graphs in `tools/materials/ptex/`, each a 4 x 4 grid of cells rendered at 2048 px and shipped at 1024, rendered as RGBA (premultiplied
off; colour in RGB, coverage in A):

| Sheet | Cells | How the graph makes a cell |
| --- | --- | --- |
| `smoke_puffs` | 16 variants | A cluster of 5 to 9 soft spheres (sdf circles with fbm-warped edges, each cell seeded apart); the height of the cluster lit from 50 degrees above the top of the cell for the shading; alpha the coverage with torn fbm edges |
| `fire_flipbook` | 16 frames | A flame tongue: a vertical gradient warped by fbm scrolling up with the frame index, white-yellow core to deep orange edge, breaking into wisps by the last frames; alpha the brightness |
| `dust_wisps` | 16 variants | Stretched, thin fbm clouds, light and torn |
| `dirt_clods` | 16 variants | Voronoi-chipped stones and soil lumps, hard alpha, a bevel lit from the top |

Colour over life still tints them (smoke dark grey to light grey, fire's sheet multiplied by its emissive ramp), so
the recipes in `fx.ron` keep their meaning.

## 3. The engine

- `ParticleKind` gets a sheet (`sheet(kind) -> Option<&str>`): one table, so a new kind names its sheet in one place.
- A random cell at birth: `SetAttributeModifier(SPRITE_INDEX, rand(0..16))` with `FlipbookModifier { sprite_grid_size:
  (4, 4) }`. Fire and flame instead step `SPRITE_INDEX = floor(16 * age / lifetime)`.
- Smoke's spin is cut to a range of plus or minus 0.3 rad, so the lit side stays mostly up.
- Explosion smoke keeps its per-burst copies for sorting, now copies of the smoke sheet.

## 4. Costs

| What | Cost |
| --- | --- |
| Draws | Unchanged: one per effect asset and texture, as now |
| Texture memory | Four 1024 px RGBA8 sheets (256 px cells; a particle rarely covers more on screen) with mips: about 5.3 MiB each, 21 MiB together, plus up to 8 copies of `smoke_puffs` for the explosion bursts' sorting, 43 MiB |
| Fragments | One texture sample a pixel, as now; larger alpha holes make the sheets slightly cheaper to blend |
| CPU | None |

Estimates. A cloud session measures nothing.

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| X3 | Smoke colour: grey (as now) or brown-black near the fire turning grey? | Brown-black turning grey |
