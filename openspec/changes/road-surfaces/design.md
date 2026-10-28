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

## 2. Tone and large-scale variation

All numbers are `road_surfaces:` in `data/world/world.ron`.

- **Tone.** Clean asphalt is a warm grey-black (linear 0.20, 0.19, 0.175), worn asphalt lighter
  (0.25, 0.24, 0.22). The packer raises the asphalt photos' contrast (x2.2 clean, x1.6 worn),
  steepens their normal maps (x1.8, x1.5) and moves their roughness to 0.88 and 0.86 with more of its
  variation, so the aggregate shows under a dark tone.
- **Tint.** Every corner's tone is times `1 + tint_amount * perlin(x / tint_scale_m, z /
  tint_scale_m)`, the island's gradient noise: 22% over about 45 m, continuous across faces.
- **Tyre tracks.** Lanes are laid out `lane_m` (3.5 m) from the centre line; every lane has two
  tracks 0.9 m either side of its middle, each a strip 1 m wide of black blended over the asphalt, 0
  at its edges and `track_dark` (0.22) at its middle, 2 cm over the road and under the 4 cm lane
  dashes. One draw for every track on the island. Blended rather than cut into the road's vertex
  colours: the cut first tried put 2.3 million triangles into the roads.
- **Patch repairs.** About one lane cell in sixteen (`patch_chance`, one lane by 8 m, a stable hash of
  the place) is drawn in the other set at `patch_tone` (0.75): fresh dark patches in a street, clean
  ones in a worn highway. The cell is cut at its lanes and the patch lies beside the road's other
  lanes, never over them, so nothing z-fights. A cell with no patch stays one quad across.
- The pavement takes no tint.

### Why the first version read blue

The street stills stand in the buildings' shade, where the light is the blue sky's. The first tone
(0.16, 0.16, 0.17) was a cold near-black, and the photo's roughness (0.69) mirrored the sky at a
glancing view, so in shade the road measured (42, 50, 75) in sRGB. With the warm tone and the rough,
higher-contrast maps it measures about (36, 41, 58) in the same shade, reading as grey asphalt in
sky light; in sunlight the highway reads neutral grey.

## 3. Markings, kerbs and craters

Lane dashes stay where they were, 4 cm over the road on their own vertex-coloured material, so they
stay crisp over the new asphalt. Kerbs are the block's box: its top is the pavement and its sides
show the slab set. Craters never dig a road (`terrain-deformation`, survey W7): a shell on a street
leaves its scorch and smoke on the asphalt, which the stills check.

## 4. Cost

Texture memory: three 1024 x 1024 sets of three RGBA8 maps with full mips, 4 MiB x 4/3 per map,
nine maps: about 48 MiB of VRAM. The removed stand-ins were two 256 x 256 maps (about 0.5 MiB).
Draw calls: two more meshes (the worn roads, the tyre tracks) and one runway mesh per air base.
Triangles, seed 42: 73,732 street, 124,108 worn, 198,048 tyre track (blended, no shadow), 4,250
block. Frame time is not
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
after stills were retaken after the owner's review: the street now reads grey with darker tyre
tracks, the odd dark patch, and tone drifting along it; the highway shows its patches from the
cockpit height.
