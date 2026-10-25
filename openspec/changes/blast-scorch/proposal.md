# Proposal: blast-scorch

## Why

Owner, 2026-10-25: "rocket explosions on dirt need work, need proper textures made from material maker". A blast on
dirt leaves two things today, and neither is a texture. The dug crater's ground is the grass texture darkened by a
vertex colour (`ground.rs`, `ground_colour`, mixed 85 % toward `(0.10, 0.08, 0.07)`). The `ground-marks` crater shape
is a procedural multiply (a ragged bowl and a darker floor). From the cockpit the result reads as a dark smudge on
grass: no char, no ash, no thrown soil, no radial blast pattern.

## What changes

- **Two Material Maker graphs** (`tools/materials/ptex/`), rendered headless at 2048 px with full PBR maps:
  - `blast_scorch`: one whole blast mark, not tiling. A charred centre with a cracked crust, grey-white ash, a ring of
    churned dark soil, radial streaks of thrown dirt and soot fading out, and a ragged edge in the alpha.
  - `churned_soil`: tiling, 2 m. Broken dark earth and clods, for the crater's bowl and its lip.
- **The scorch is a decal**: each crater on soil gets Bevy's forward decal with the `blast_scorch` maps, sized from
  the crater and turned at random. It is lit live like the ground under it, and the dug bowl takes it too. The
  procedural crater shape stays for paving only.
- **The pipeline Crab Breeze lacks**: `tools/materials/` (graphs, `build_ptex.py`, an export script running the
  `material-maker-headless` skill, a post-process packing ORM the glTF way), `data/materials/materials.ron` listing each
  material, and a contact sheet in `docs/screenshots/materials/`. `terrain-materials` builds on the same pipeline.

## Out of scope

The terrain splat (`terrain-materials`), particle sprites (`fx-sprites`), plumes (`rocket-plumes`), scorch on paving
and buildings (stays procedural), persistent craters past the mark ring's cap.
