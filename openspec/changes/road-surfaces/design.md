# Design: road-surfaces

## 1. Sets

| Surface | Set | Real size of one tile | Drawn tile | Used by |
|---|---|---|---|---|
| `asphalt` | ambientCG Asphalt023L | 2.5 m | 2.5 m | city streets |
| `asphalt_worn` | ambientCG Asphalt022 | 1.85 m | see `road_surfaces.worn_tile_m` | arterials, highways, runways, taxiways |
| `paving` | ambientCG PavingStones133 | 4 x 4 slabs | 2.4 m (600 mm slabs) | pavements, kerb faces, park blocks under their tint |

Every set is the 1K JPG download. The colour map is scaled per channel to average 0.8 (sRGB), as the
building surfaces are, so the road's vertex colour keeps setting its tone (street 0.16, arterial
0.13, pavement 0.55) and the map adds only the grain. Normal maps are the OpenGL ones as they came;
ORM is packed the glTF way (R occlusion, G roughness, B metal 0). Each set was looked at tiled 2 x 2
and at full resolution across the seam; none shows one. Asphalt022's dark patches repeat every
1.85 m in the 2 x 2 sheet, which is why it carries the tint below.

## 2. Large-scale variation

One texture sample, no custom shader: the road meshes are cut into cells of `tint_cell_m` (8 m) and
each vertex's colour is the road's tone times `1 + tint_amount * perlin(x / tint_scale_m, z /
tint_scale_m)`, the island's own gradient noise. Neighbouring cells share their corner colours, so
the tint is continuous and changes over tens of metres: patches of fresher and older surface that
break the tile's repeat from the cockpit. The pavement takes no tint.

## 3. Markings, kerbs and craters

Lane dashes stay where they were, 4 cm over the road on their own vertex-coloured material, so they
stay crisp over the new asphalt. Kerbs are the block's box: its top is the pavement and its sides
show the slab set. Craters never dig a road (`terrain-deformation`, survey W7): a shell on a street
leaves its scorch and smoke on the asphalt, which the stills check.

## 4. Cost

Texture memory: three 1024 x 1024 sets of three RGBA8 maps with full mips, 4 MiB x 4/3 per map,
nine maps: about 48 MiB of VRAM. The removed stand-ins were two 256 x 256 maps (about 0.5 MiB).
Draw calls: one more mesh (the worn roads beside the city streets). Triangles: the cut into 8 m
cells adds about two triangles per 8 m of street length per lane of cells. Frame time is not
measured here (no same-session A/B was run); sampling a normal and an ORM map on the roads is the
same work the buildings' surfaces already do.

## Open questions

| # | Question | Recommendation | Status |
|---|---|---|---|
| R1 | Arterials on the worn set, or the clean one? | worn: they carry the heavy traffic | provisional |
| R2 | Parks on their own grass set instead of tinted paving? | yes, in a later change | open |

## Seen in the game, 2026-10-26

`CB_STILLS_ROADS=1 --stills`, seed 42, on the Mac, before and after in `docs/screenshots/roads/`
(`before-*.jpg`, `after-*.jpg`). On foot the street shows fine aggregate at real scale and the
pavement 600 mm slabs with dark joints; from the cockpit's eye height (18.4 m) neither the street's
nor the highway's tile repeats visibly. The lane dashes stay crisp. A 4 MJ shell on the street
leaves its scorch on the asphalt and no hole, as `terrain-deformation` keeps roads level. The
streets' blue cast is the scene's sky light on a dark tone, the same before and after.
