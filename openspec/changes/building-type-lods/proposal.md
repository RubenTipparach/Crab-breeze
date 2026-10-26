# Proposal: building-type-lods

## Why

Owner, 2026-10-25: "for each building type we're gonna need a custom LOD texture/mesh for them, so we
dont just yse the generic texture we have now".

Today a building more than `dress_radius_m` (250 m) from the pilot is one plain box: every type gets
the same tiled facade texture (`tex.facade`, one 4 m bay by one 3.5 m storey) tinted with one of five
tones picked by its id (`cb-app/src/world.rs`). A brick walk-up, a glass curtain-wall tower and a
brick warehouse look alike past 250 m, and when the pilot walks or skates closer each one pops from
a grey-tinted box into its real skin with different colours and window rhythm. Seed 42 has 3,609
buildings in 13 cities, and from the cockpit nearly all of them are past 250 m: the skyline the
pilot fights in front of is the generic box.

## What changes

- **Each building type gets its own far look**: a mid-distance massing mesh and a far box, both
  wearing that type's own facade texture baked from its own near facade. The five facade kinds in
  the game today: brick walk-up, ribbon-window office, punched-grid concrete office, glass
  curtain-wall tower, brick warehouse.
- **One answer to "what is this building"**: the near dressing's private style and colour pick
  (`dressing::look_of`) becomes a public pure function, so the far look uses the same type, brick,
  concrete and glass colours as the near skin, and the swap between them is a change of detail, not
  of colour or window rhythm.
- **Three distances**: dressed as now within 250 m; the type's massing mesh (parapet, cornice, roof
  props, setbacks, its baked facade) to 1.5 km; the type's box with the same baked facade beyond.
- **Baked facade atlases**: `cb-tools bake-facades` renders each type's ground storey, typical
  storey and crown band face-on from the same dressing pieces and material sets the near skin uses,
  into a small colour, normal and occlusion-roughness-metal set per type (texture array layers), so
  a far facade cannot disagree with its near one.
- **A building type is not done without its far look**: a test fails for any type that lacks a
  baked layer or a massing recipe, so the six types of `building-variety` and the seventeen of
  `small-buildings` each ship theirs when they are built.

## Out of scope

Damaged buildings (they already draw their frame at every distance), merging far boxes per block
(`building-lod` step 4), lit windows at night (no time of day yet), landmarks.
