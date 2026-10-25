# Design: building-surfaces

## 1. The sets

All from ambientCG (CC0 1.0), 1K JPG sets, downloaded 2026-10-24, each folder under
`assets/textures/building/<surface>/` with `albedo.jpg`, `normal_gl.png`, `orm.png` and `SOURCE.md`
(URL, licence, every change). As the grass: the colour map is scaled so it averages near white, so
the vertex colour (the kit's or the building's tone) sets the hue and the map adds the detail.

| surface | set | tile | used for |
| --- | --- | ---: | --- |
| concrete | Concrete034 | 2.4 m | columns, slabs, core walls, falling chunks |
| plaster | Plaster001 | 2.4 m | room partitions |
| wood | Wood094 | 1.0 m | desks, shelves, beds, wardrobes, tables, tills |
| fabric | Fabric061 | 0.6 m | seats, sofas, bedding |
| metal | Metal027 (painted) | 1.0 m | filing cabinets, shelving gondolas, fridge, till drawer |
| plastic | Plastic010 | 1.0 m | monitors, TV, bath, worktops |
| cardboard | Cardboard002 | 1.0 m | crates |
| leaves | ScatteredLeaves009 | 0.5 m | plant foliage (the pot is plastic) |

## 2. Drawing

`MeshBuilder` already writes UVs in metres; each surface gets its own builder at its tile size,
with tangents for the normal map. A band of a damaged building goes from 2 meshes (facade, bare)
to at most 9 (facade and 8 surfaces); a dressed whole building's trim mesh splits the same way.
One `StandardMaterial` per surface is shared by every building, so meshes of a surface batch.

## 3. Cost

- Texture memory: 8 sets x 3 maps x 1024 x 1024 x 4 bytes x 1.33 (mips) = about 134 MB.
- Draw calls: up to 7 more per damaged band and per dressed building, near the player only
  (dressing within 250 m). Measured with `perf_suite.py --scenarios demolition,defence` against
  the 2026-10-24 Mac report.
- CPU: none per frame; the meshes are rebuilt on the same events as before.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Material Maker graphs instead of CC0 sets | CC0 now (owner asked for textures now; section 13 exception), graphs later if the look needs it | provisional |
