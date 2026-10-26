# Design: cockpit-styles

## 1. Styles

`build_cockpit.STYLES`, applied after the cockpit's trim (`apply_style`), so a style can be set on any cockpit.

| Style | Walls (`paint`) | Trims (`paint_lt`) | Structure (`paint_dk`) | Seat (`fabric`) | Strips, lamps |
| --- | --- | --- | --- | --- | --- |
| wood | brown leather, pebbled grain | walnut, rings and fine grain, roughness 0.32 | dark brown | tan leather, quilted 90 mm | amber, warm |
| airliner | off-white plastic, stipple | light grey plastic | mid grey plastic | blue cloth weave | soft white, neutral |
| padded | charcoal leather quilted 120 mm, grain | brushed steel, metal 1 | near-black leather | oxblood leather quilted 70 mm | red-orange, warm |

## 2. Material features (`cockpit_kit.make_material`)

- `wood=(dark, light)`: a Rings wave in object space stretched 14 times across the piece's run, distorted by noise, plus
  fine noise; mixes the two tones; a 0.4 mm grain bump.
- `quilt=pitch_m`: on each face's own plane (weighted by the normal's components to the fourth power), the pillow
  sin(pi frac(u)) sin(pi frac(v)) on the diagonals of a pitch grid; height 12 mm in the bump; seams darkened.
- `grain`: Voronoi distance at scale 900 as a 0.3 mm bump (leather).
- `stipple`: noise at scale 700 as a 0.3 mm bump (moulded plastic).

All in object space, so the patterns bake into the atlases as the existing `weave` does: the albedo, normal and ORM
maps carry them; roughness and metal stay per role (the ORM's G and B).

## 3. Mockups

`--preview PATH` builds the cabin, skips the bake and the export, and renders `PATH_front.png` and `PATH_side.png` in
Cycles (48 samples, denoised, 1280 x 720) from `EYE_pilot` at the game's view angle. About 4 minutes a cockpit and style
on the cloud CPU.

## 4. Cost

No triangles. The maps stay 4096 px; the features only change what the bake writes. A style change costs one rebake
(about 50 minutes a cockpit).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CS1 | Which cockpits get which style | Decided by the owner from the mockups | open |

## 5. The mockup

`docs/mockups/cockpit_styles.html` (published; the owner asked for quick JS mockups, not renders): a three.js block-out
of the Iron Horse's cab at true scale with a style switcher, seat and orbit views. It also proposes the wall dressing
(owner: "the wall is kinda empoty, need electrical panels, vents, rivets, buttons") and the wire runs (owner: "tidy up
the wires they go no where", "place wires along the struts"): breaker panels with placards, louvred vents, backlit
button blocks and rivet rows on the lower side walls; the cable bundle from each side console down to the floor, along
it to the corner pillar, up the pillar in clips every 0.22 m, and along the roof beam into a junction box over the front
screen. The textures there are drawn in the page; the builder's are the material features above.
