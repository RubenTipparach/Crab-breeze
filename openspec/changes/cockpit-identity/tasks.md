# Tasks: cockpit-identity

## 1. Layouts
- [x] 1.1 Octagon, Tile wall, Twin tier rows moved so every seam is clear of the eye line; top MFDs on Cinema, Tile wall, Twin tier; the arcs one wide tile ahead and square tiles round the sides
- [x] 1.2 The four registered as `build_cockpit.py` variants, each with its own trim (paint, accent line, fabric, lamps); quick builds (512 px) looked at from the eye
- [x] 1.3 `tools/blender/measure_view.py`: the share of the fighting band (10 degrees under the eye line to 20 over it, full width) each cockpit hides; the Foreman's Panorama 21.2 %, the owner's most 25 %
- [x] 1.4 Over the budget: the Octagon (30.1 %, its rail and MFDs in the band; now 24.2 %, the rail at 21.6 degrees and the MFDs on the upper row, `mfd_row`) and the Canopy (37.0 %, its screen's top 14 degrees up; now 22.5 % with the top at 1.58 m, code only: the shipped Canopy is the old bake until 2.3)
- [x] 1.5 The Titan at 25.0 %, right at the limit at 85 degrees; 21.2 % at the game's 95 degree view, inside the budget with margin

## 2. Interiors and chests
- [x] 2.1 Driller, Picket, Mule, Sapper chests on their new variants; mechs rebuilt; boarding data regenerated; `cb-tools validate` passes
- [ ] 2.2 Bake the four at 4096 px, 16 occlusion samples (they ship with the 512 px quick maps until then)
- [ ] 2.3 Rebuild and rebake the Canopy with its raised screen; Harrow rebuilt and its boarding regenerated

## 3. Checks
- [x] 3.1 `cargo test --workspace` (the new `no_two_mechs_share_a_cockpit_interior`), `cb-tools validate`, `openspec validate --all`, the dash check
- [ ] 3.2 Headless seat stills of all eight mechs, each different (`docs/screenshots/interiors/`)
- [ ] 3.3 Playtest on the owner's machine: board each of the four, watch the opening, read the six screens
