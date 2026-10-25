# Design: cockpit-identity

## 1. The layouts, tuned

From the seat (eye 1.20 m over the floor). A seam is "clear" when it is at least 15 degrees above the eye line at the
front screen's depth (`cockpit-parity`): y >= 1.20 + depth x tan 15 degrees.

| Layout | Front depth | Rows now | Seam now | Rows built | Seam | MFDs |
| --- | --- | --- | --- | --- | --- | --- |
| Octagon | 1.05 m | (0.75, 1.40), (1.43, 2.05) | 11.6 degrees up | (0.75, 1.50), (1.53, 2.05) | 16.7 degrees up | top, dash |
| Cinema | 1.15 m | (0.90, 1.95) | none | unchanged | none | **top**, dash |
| Tile wall | 1.00 m | (0.80, 1.38), (1.41, 1.99) | 10.8 degrees up | (0.80, 1.48), (1.51, 1.99) | 16.4 degrees up | **top**, dash |
| Twin tier | 0.95 m | (0.78, 1.24), (1.30, 1.90) | 3.6 degrees up | (0.78, 1.46), (1.49, 1.90) | 16.2 degrees up | **top**, dash |

The arcs already centre a tile straight ahead (the Tile wall's 11 tiles of 19.1 degrees, the Twin tier's 9 of 21.1), so
no seam lies ahead.

Pillars stay where the block-outs put them: Octagon 87 degrees clear ahead, Cinema 95, Tile wall 210 (no pillar ahead
at all), Twin tier 190. Low bank (74 degrees, against the owner's 40-degree rule) and the All-view sphere (no rows, a
seat on an arm: a builder of its own) are not used.

## 1b. How much of the view a cockpit hides

Owner, 2026-10-25: "some of these, the dividers obsucre view too much", "lets assume forman cockput obscures about 20
percent of the view", "lets say 25% is abosolute max". `tools/blender/measure_view.py` renders each cabin from
`EYE_pilot` at the game's eye camera (85 degrees vertical, 16:9, level, straight ahead), the outside screens white and
everything else black, and counts the black in the **fighting band**: the full width, from 10 degrees under the eye
line to 20 degrees over it (the dash under the knees and the roof overhead lie outside it). On that measure the
Foreman's Panorama hides 21.2 %, the owner's "about 20". **No cockpit may hide more than 25 %.**

Measured at the 85 degree view of the time, then again at 95 degrees after the game widened its view (main, 2026-10-25;
the instrument now reads `view_fov_deg` from `data/pilot/pilot.ron`):

| Cockpit | Mech | Hidden at 85 degrees | at 95 degrees | |
| --- | --- | --- | --- | --- |
| Tile wall | Mule | 16.8 % | 14.4 % | |
| Cinema | Picket | 18.3 % | 15.4 % | |
| Twin tier | Sapper | 19.2 % | 15.7 % | |
| Panorama | Foreman | 21.2 % | 17.9 % | the yardstick |
| Cab | Ironhorse | 22.1 % | 18.8 % | |
| Octagon | Driller | 30.1 % then 24.2 % | 20.0 % | its rail at 21.6 degrees, its MFDs hung from the upper row |
| Titan | Roustabout | 25.0 % | 21.2 % | |
| Canopy | Harrow | 37.0 %, 22.5 % in the builder | 31.2 % (the shipped bake) | its screen's top raised from 1.45 m to 1.58 m, rebake pending |

## 2. The opening

Each layout keeps the block-out's opening (`BO.mechanism`): the Octagon's gull wings and dropping front, the Cinema's
door on three hinges, the Tile wall's three petals, the Twin tier's top rising on four rams and its lower front
dropping as the step. The dash rides the part the pilot leaves through, as in every built cockpit.

## 3. The chests

`chest_bay(k, chest, variant, at, "inner", torso, swing=...)` lofts the mech's torso round the interior the variant
builds, so a chest takes a new cockpit by its variant name. A `swing` override is added only where the chest's own size
asks a stage to turn further than the interior alone needs (as the Panorama's `INNER_upper_1` in the Picket). The
torsos (`DRILLER_TORSO`, `PICKET_TORSO`, `MULE_TORSO`, `SAPPER_TORSO`) are not changed.

## 4. Cost

- **Triangles:** about 25 000 to 30 000 a cabin, as the four built (the Canopy 25 352); one cabin drawn at a time.
- **Textures:** four PBR atlases at 4096 px each, about 15 MB a cabin on disk, the Canopy's figure; one loaded at a time.
- **Bake:** about 1.5 h each on a 4-core cloud machine, the Canopy's 5 245 s.
- **Game:** nothing; the game binds any interior by its node names (`cockpit-parity`'s checks).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C16 | Which block-out for the Driller, Picket, Mule and Sapper | Octagon, Cinema, Tile wall, Twin tier, as above | provisional (survey C16) |
