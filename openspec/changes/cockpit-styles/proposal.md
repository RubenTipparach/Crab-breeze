# Proposal: cockpit-styles

## Why

The owner, 2026-10-26: "can we mess with the textures a bit to give them a different interior style? wood trims,
passenger plane interiors, soft interior paddings? make a mockups of these interiors before baking them. this is only
texture/normal mpas work and some material changes".

Every cockpit is painted steel in a colour of its mech's (`cockpit-identity`'s trims). The rooms differ in shape and
colour but not in what they are made of.

## What changes

- **Interior styles**, material changes only (no geometry): each re-dresses the kit's four big roles, the walls
  (`paint`), the trims (`paint_lt`), the structure (`paint_dk`) and the seat (`fabric`), and tints the accent strips
  and the lamps:
  - **wood**: varnished walnut trims with a grain in the normal map, cream-brown leather walls, a tan quilted leather
    seat, warm lamps;
  - **airliner**: off-white moulded plastic with a fine stipple, light and mid greys, blue seat cloth, soft white
    strips, a cabin's neutral light;
  - **padded**: diamond-quilted charcoal leather walls and seat (tufted in the normal map, seams stitched dark),
    brushed steel trims, red-orange strips.
- **New material features** in `tools/blender/cockpit_kit.py`: `wood`, `quilt`, `grain`, `stipple`, baked into the
  atlases' albedo and normal maps like the existing `weave` and `stripes`.
- **Mockups before any bake**: `build_cockpit.py --style <name> --preview <png>` renders the built, unbaked cabin from
  the seat (ahead, and turned to the left wall), so styles are judged before six hours of baking.
- **Which mech gets which style** is the owner's pick from the mockups (open question CS1).

## Capabilities

- New: `cockpit-styles`.
