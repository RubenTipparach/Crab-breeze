---
name: lowpoly-car-surface-shell
description: Model a hero-quality low-poly car body in Blender (headless bpy/bmesh) as an open SURFACE shell grown from the roof outward, fitted to all four blueprint views, with every styling crease as a sharp-marked edge loop, openings as framed holes (no booleans), and the car split into many separately mirrored parts (bumpers, doors, glass, lights, handles, grilles). Use for a showcase car where a lofted or boxed body looks too generic; budget about 10k to 15k tris for the exterior.
metadata:
  version: "1.0"
  source: "Method studied privately from Praize3d, 'Low poly car modeling in blender' parts 1 to 6 (https://www.youtube.com/playlist?list=PLzSbzd1_gUxQIZq1bsQBlEh-zzPOgAyoI), read from frame sheets since the videos have no narration. This is an independent write-up of the technique."
---

## In Crab Breeze
This is the hero-vehicle body method (LOD0 for a car the pilot sees close up, about 10k to 15k
tris). Pair it with `combined-lowpoly-vehicle` for wheels, lights and checks, and with
`material-maker` for PBR maps.


# Low-poly car body as a surface shell

The other vehicle skills close a volume first (a loft or a three-profile intersection) and cut into
it. This one never makes a closed volume. It grows panels edge by edge from the roof and snaps
each vertex to every view. That costs more steps, but you control exactly where every line goes.

## References
- **Load the views as data.** Use a polyline per view: side silhouette, top plan, front section,
  rear section, plus the styling lines (shoulder, hood creases, door cuts, window frames) and each
  wheel's centre and radius.
- **Use real dimensions** for the scale (length, width, height, wheelbase).
- **Keep photos to hand** for the 3D forms that no orthographic line shows.

## Build order
1. **Roof.**
   - Start with one plane on the roof centreline, keeping only x >= 0.
   - Add a Mirror modifier: X, clipping on, merge 0.001. Keep centreline verts at x = 0 exactly.
   - Grow a grid of about 3 columns by 7 rows. Take each vert's length position and height from the
     side roofline, its width from the top plan, and add a crown term so the roof drops towards the
     outer edge.
2. **Glasshouse as holes.** Extrude the roof's outer edge down into the A, B and C pillar strips and
   the window-frame strips. The windows stay OPEN holes, framed by those strips. Glass and frame trim
   are separate objects added later.
3. **Hood.**
   - Extrude the cowl edge forward along the top-view outline, then bend it down to the nose in the
     side view.
   - Use about 4 to 5 columns by 8 to 10 rows.
   - Put a vertex row on each hood crease, so the crease is a real loop, and mark it sharp.
   - Shape the nose around the grille notch.
4. **Fenders and sides.** Extrude the hood's outer boundary down and out into the fender top and
   shoulder line, then keep extruding the side down to the sill. The shoulder line is a sharp loop.
5. **Wheel arches.**
   - Use a wheel-sized cylinder as a guide only.
   - Make the side panel's lower edge an arc of about 9 points round the wheel centre, at the tyre
     radius plus the gap. Bridge it to the row above.
   - Extrude that arc 3 to 4 cm outward, and slightly outward radially, to make the flared lip.
     Mark the lip sharp.
6. **Doors and sills.** Extrude the doors back from the fender's rear edge and fit them to the door
   lines. The side skirt is a band extruded along the bottom edge between the arches.
7. **Rear quarter and tail.**
   - Extrude the C pillar down from the roof's rear edge and fill the quarter panel between the
     pillar, the arch and the tail. One triangle is fine where the panel narrows.
   - Extrude the trunk lid off the rear window edge.
   - Make a **ducktail** by extruding the lid's rear loop back and up into a thin sharp lip.
   - Extrude the rear fascia down from the lid edge.
8. **Bumpers.** Separate mirrored objects, built as quad rings that FRAME each opening (intakes,
   air curtains, grille recess, exhausts).
   - Extrude each inner loop inward along the normal for a short depth wall.
   - Make the **splitter** by extruding the lower loop forward and out.
   - Make the **diffuser fins** as short strips extruded down at even spacing along the rear lower
     loop.
9. **Panel gaps.** Split the doors, hood and trunk off along their cut loops. Keep a loop on each
   side of every cut so the gap reads, and mark both sharp.
10. **Details**, each its own mirrored object:
    - **Door handles:** an oval recess (inset and push in 1 cm) holding a 5-segment bar with
      chamfered ends.
    - **Headlights:** a hole in the body, with:
      - a recessed housing extruded into it;
      - an eyelid strip along the top rim;
      - L-shaped running-light guides as planes with Solidify (0.007 m, offset -1, rim fill);
      - hex projectors made from 6-gons: fill, inset, extrude.
    - **Honeycomb grille:** tile 6-gons on a flat hex lattice and give each one an
      `inset_individual` frame with its centre deleted, so the cells are open. Then bend the grid onto
      the curved bumper, either with a Lattice or by projecting each vert onto the bumper surface in
      code.
    - **Tail lights and rims:** see the layered-light and sector-array wheel recipes in the combined
      skill.

## Rules
- Every visible styling line carries its own edge loop, marked sharp (`edge.smooth = False`). Big
  flat areas get long quads. Accept a triangle or an n-gon wherever a quad would cost a whole
  extra loop.
- No Subdivision, no Boolean. Openings are framed holes with short walls.
- Shade smooth by angle, which respects the marked sharps.
- Clean up often with `bmesh.ops.remove_doubles(dist=0.001)` and recalculate normals.

## Budget
- Body half: about 650 tris. Whole exterior: about 10k to 12k tris with about 48 objects, most of
  them small mirrored details.
- The honeycomb grille is the biggest single cost (thousands of tris). For game LODs, swap it for a
  dark recess with an emissive-free texture.

## When to use which skill
- **Background or traffic car:** combined skill, clamped loft or three-profile body (2k to 5k tris).
- **Hero or showcase car:** this skill for the body, plus the combined skill's wheel, light and
  verification phases, plus the interior skill if the cabin is seen.
