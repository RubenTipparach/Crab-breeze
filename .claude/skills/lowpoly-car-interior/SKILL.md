---
name: lowpoly-car-interior
description: Build a game-ready low-poly car interior (door aperture flanges, dashboard, centre console, door cards, pillar trim, headliner, front sport seats, rear bench) in Blender with headless bpy/bmesh, fitted to an existing body shell and detailed by what shows through the windows. Profile-then-extrude construction, mirrored halves, Mark-Sharp seams on smooth shading, no subdivision. Use after the exterior is built, when a car's cabin will be seen in game.
metadata:
  version: "1.0"
  source: "Method studied privately from the Praize3d 'Low poly car modeling in blender' interior videos (9 to 16; https://www.youtube.com/playlist?list=PLzSbzd1_gUxQIZq1bsQBlEh-zzPOgAyoI), read from frame sheets since the videos have no narration. This is an independent write-up of the technique."
---

## In Crab Breeze
Use this for vehicle cabins that show through the glass (parked or wrecked cars in the streets).
It is not for the mech cockpit, which follows section 10 and `panel-textures`.


# Low-poly car interior

Use this skill only once the exterior shell exists. The cabin is modelled to fit that shell and is
judged from outside the car, through the glass.

## Rules
- **Build order.** Shell openings first, then the big volumes, then details. The volumes are the
  dash, console, door cards, pillars and headliner, in that order. The seats come last because
  they are the most visible and the most expensive.
- **Mirror everything that crosses the centreline:** dash, console, headliner, seats, pillar strips.
  Build only X >= 0, with the Mirror modifier set to clip and merge at 0.001. Apply it at export.
- **Profile, then extrude.** Most parts start as a plane or one edge strip, not a cube. Draw the
  section from the blueprint or the body's own edge, then extrude it along the run.
- **No Subdivision, no Bevel modifier.** Softness comes from a few hand-placed quad rows:
  - 4 to 8 rows across the dash top;
  - 6 to 10 loops round a seat back.
- **Seams are shading, not geometry.** Shade the part smooth and Mark Sharp every seam and panel
  break. A 1 to 3 mm push along the normal on a seam row turns it into a stitched groove.
- **Detail by visibility.** Spend polygons where the glass shows them:
  - window-line trim, the dash top, the upper half of the seats, the door-card tops.
  Leave plain or omit:
  - the lower dash, footwells and under-seat areas;
  - the backs of panels, which can be single-sided.
  To decide in code, cast rays from points outside the windows and refine only the faces they hit.

## Parts, in build order (with bmesh recipes)
1. **Aperture flanges.**
   - Take each door or window opening's boundary loop on the body.
   - `extrude_edge_only` it, move the new loop inward along the local inward normal by 4 to 8 cm,
     then extrude once more for a lip.
   - This is what makes the shell read as having depth through the glass.
   - Add A, B and C pillar inner trim as mirrored strips following the window opening, plus sill
     trim on the body floor. Reuse the body's own floor; don't build a separate tub.
2. **Dashboard.**
   - A thin strip profile from A pillar to A pillar under the windscreen base, extruded rearward
     into a slab.
   - The top is a curved quad grid, 4 to 8 rows, following the windscreen base.
   - Then the instrument-cluster hood, the centre stack, two console rails, the transmission tunnel
     and the lower knee panel.
   - **Details**, each one an inset with a push of one or two faces, or a separate small box:
     - screen: a flat inset panel;
     - vents: inset boxes;
     - glovebox: an inset handle slot.
   - About 1.5k to 2k tris.
3. **Centre console.** Extend the tunnel back to the rear seats, with the tray as one inset recess
   and the armrest as a box with two hand loops. The gear selector sits in the console as a small
   lathe or box.
4. **Door cards.**
   - A flat card following the door outline: `contextual_create` or `triangle_fill`, then an
     `inset_region` border.
   - Press it onto the inner door surface. Either use Shrinkwrap (nearest surface point, on surface,
     offset 0 to 5 mm) or, in code, a `BVHTree.find_nearest` on the body per vert, plus an offset
     along the normal.
   - Then add the armrest ridge (extrude), the grab-handle recess and pull cup (inset and push in),
     the window-switch pod (inset and pull out), and the speaker (inset circle).
5. **Headliner.** One mirrored grid:
   - an outer loop from the roof's inner edge points;
   - `grid_fill` or fan rows to the centreline;
   - each vert set just under the roof shell (BVH nearest minus 1 to 2 cm).
6. **Front sport seat.** Separate objects (headrest, backrest, cushion, base) parented together,
   about 100 to 350 tris each and about 340 for the whole seat:
   - **Headrest:** the front outline as a plane, extruded for depth, with the corners chamfered by
     hand.
   - **Backrest:** draw the side S-curve as an edge chain and extrude it sideways for the centre
     strip. Add two `inset_region` rings for the inner and outer bolsters, keeping a visible step
     between rings, which is the seam. Push the outer ring forward by a bolster depth that grows
     towards the shoulders.
   - **Cushion:** a side profile extruded across, with the outer loops raised for thigh bolsters
     and split lengthwise into three panels with one raised seam ridge.
   - **Base:** the closed side outline extruded across, with a switch pod (an inset recess holding
     two small panels).
7. **Rear bench.**
   - **Back:** one upright plane cut into vertical bands, with alternate columns pushed in or out to
     make flutes. Mark the column edges sharp.
   - **Seat shells:** two mirrored outboard shells whose seams curve towards the waist.
   - **Cushion:** a strip extruded into the cushion, with the front edge rolled down into a nose and
     the side bolsters pulled up towards the doors.
   - **Armrest:** a box between the cushions.
   - **Seams:** cross-cushion seam rings, marked sharp and pushed in.
8. **Small items last:** steering wheel (a lathed rim with 3 spokes), seat-belt buckles, mirror
   housing inside. A steering wheel and pedals may be simple, because they are rarely seen.

## Materials and shading
- Use region material slots by face group (`face.material_index`): inner shell, floor or carpet,
  trim, seat leather, accent, screen (emissive, dim).
- Smooth shading throughout, with sharp edges on seams and creases (Blender 4.1 and later need no
  auto smooth).
- Keep a "trash" collection for discarded attempts, hidden and excluded from export.

## Budget
- Whole car 35k to 38k tris, of which interior about 15k (hero level).
- For background or game-ready cars, scale everything down: keep the seats' upper halves, the dash
  top and the window-line trim, and drop the footwells and lower-dash detail first.

## Checks
- Look sheet from outside, through each window, at gameplay size.
- Check for z-fighting between trim and shell: keep at least a 3 mm offset.
- Check that no interior part pokes through the body; use BVH ray tests along the normals.
