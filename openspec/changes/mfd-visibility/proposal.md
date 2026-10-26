# Proposal: mfd-visibility

## Why

Owner, 2026-10-26, from the Harrow's seat: "harrow MFD cockput is unacceptable, first the mfds are not visible entirely
in the view so we need to fix that by moving them up, secondly they are not rendered with the same aspect as all other
MFDs", "get screenshots of other MFDs and confirm if they are all entirely visible on screen", "these mfds can stick
above the cockpit line, think like a bulge over the dash", "see how this dashboard curves up?" (a car's dash whose
centre display stands up on top of it), "iron horse, driller, sapper all the same problem".

Measured (`tools/blender/measure_mfds.py`, from `EYE_pilot` at the 95 degree seated view, level): no cockpit shows all
six screens whole. The Tile wall's and the Twin tier's dash screens are 0 % visible, the Octagon's six 22 %, the
Cab's hung ones 44 %, the Canopy's dash ones 44-56 %; even the Panorama's are 89-100 %. Every screen looks squashed,
0.64 to 0.87 high for 1 wide (the pages are square): the dash leans back 18 degrees whatever the angle it is seen at.

## What changes

- **Every MFD is placed by its angle from the eye**, the same in every cockpit: the three dash screens centred 25
  degrees under the eye line, the three hung ones 30 degrees over it, 23 to 26 degrees apart side to side, each turned to
  face the eye, and brought nearer (and smaller by the same share) where a cabin is too shallow to hold them. All six then lie wholly inside the 95 degree view and outside the fighting band (10 degrees under the
  eye line to 20 over, `cockpit-identity`), and each looks square.
- **The dash screens stand on a pod above a lower dash**, the owner's "bulge over the dash": the dash body drops
  below the screens, so its top and the glare shield no longer cover them; a hood curves over each screen.
- **The hung screens hang on stalks from the roof** at their angle, in front of the front screen.
- **Checked by the instrument**: every screen 100 % visible, its aspect within 0.9-1.2, in all eight cockpits; and the
  fighting band still 25 % hidden at most.
- All eight interiors rebuilt and baked at 4096 px.
