---
name: combined-lowpoly-vehicle
description: Build a game-ready low-poly vehicle (car, truck, aircraft, or any other) in Blender with headless bpy/bmesh from orthographic blueprints or reference images. Fuses three views into station cross-sections, box-models the body as one mirrored shell, cuts real wheel wells with liners, recesses glass and lights with material-transferring cutters, lathes spoked wheels, and gates every phase with silhouette and look renders. Use for any car, truck or vehicle modelling request where meshes must be built, not generated.
metadata:
  version: "1.3"
  built_from: "A youtube-lowpoly-car (stations, mirror shell, arches from existing verts, every-other-face spokes, inset/extrude details) · B vehicle-artist (proportions first, object split, cutter collections) · C blender-image-to-3d (measure then model, gated phases, self-check silhouettes, axle origins, glass as its own object) · D blender-mcp-master (budget first, modifier order, 4.x API care, bevel sizing) · E Crab Breeze hard-surface (named cutters, material transfer, clean/check/refuse, deterministic builds) · F blender-mcp (look render after every build) · G control (three-view clamped section loft). Lessons from all seven runs of the 2026-10-28 bake-off, and v1.1 fixes from round 2 (CC0 estate): E's three-profile intersection and swept seams, D's and F's validators and look sheets, H's bevel, sliver and glass costs."
---

## In Crab Breeze
Use this skill for the city's **vehicles**: traffic cars, vans, trucks, the tracked and wheeled
support vehicles, and aircraft. It is a low-poly, game-ready method (2k to 8k tris per vehicle).
The mechs and the cockpit stay with `blender-hard-surface` and the mech kit (section 10). Hero
vehicles that the pilot sees up close follow this repository's high-detail rules: LODs, PBR
materials from `material-maker`, and budgets in the asset manifest. Use this skill's methods for
LOD1 and lower, and its hero-car pointers for LOD0. The scorer it mentions (silhouette IoU against
blueprint masks) lives in the owner's separate `vehicle-skills` workspace, not in this repository.


# Low-poly vehicle, combined method

Rule zero: **no generated meshes.** No Meshy, Tripo, Hunyuan3D, Rodin, Hyper3D, image-to-mesh, asset
libraries or downloads. Every polygon comes from your own bpy/bmesh code, in one rerunnable `build.py`.

## Best traits, in one line each (v1.2)
- **C, measure first:** every number goes in the brief before any geometry, plus your own silhouette
  gate, so scored runs are only spent on a build that already passes.
- **A, stations and lean budgets:** decide where the shape changes before modelling, and build
  nothing in between. A built its car in under half the budget.
- **G and H, the clamped section loft** for curved bodies (cars, fuselages, nacelles).
- **E, the three-profile EXACT intersection** for slab-sided bodies (trucks, cabs, boxes, tanks).
  It also brings:
  - named convex cutters with material transfer;
  - one swept solid per seam;
  - checks that refuse to save a bad mesh;
  - deterministic rebuilds.
- **H, finish:**
  - lined wells and recessed glass on one plane per window;
  - spokes from every other rim face;
  - a dark rocker band;
  - the low-angle and 128 px look gate.
- **D and F, verify:** run a validator, then look at a material-shaded sheet from six angles. It
  catches what the numbers miss, such as every face collapsing to material 0 or open wheel geometry.
- **B, organise:**
  - one object per system (body, wheels, glass, lights);
  - cutters in a hidden collection;
  - pivots where parts move.

## Hero cars and cabins (v1.2)
For a showcase car, build the body with the `lowpoly-car-surface-shell` skill (an open shell grown
from the roof, with sharp-marked crease loops and framed openings, about 10k to 15k tris) and keep
this skill's wheels, lights and gates. If the cabin shows through the glass, add the
`lowpoly-car-interior` skill. Both come from the Praize3d series.

## Vehicle types (v1.2)
Pick the body method by the shape, then reuse the phases below.
- **Car or van:** clamped loft (curved) or three-profile intersection (boxy). Phases 2 to 7 as written.
- **Truck (cab plus bed or box):**
  - **Build the parts as separate solids,** each one by three-profile intersection: cab, hood,
    fenders and cargo bed. Union them only where the shells really meet.
  - **Fenders:** extrude the fender's side outline and curve it over the wheel. Use a swept
    profile or bend existing verts, not a box.
  - **Cargo bed:** a floor, side and front boards, a tailgate, and stake pockets cut in as recesses.
    Canvas bows are thin swept tubes. Leave them out of the silhouette if the drawing dashes them.
  - **Separate parts:** spare wheel, grille as a cutter grid, bumper beams, winch, step plates and
    mirrors on arms.
  - **Wheels:** big off-road tyres, 12 to 16 sides, with tread blocks as alternate-face extrusions on
    the tread ring, which costs the same as the spoke trick.
- **Aircraft:**
  - **Fuselage:** a clamped loft of round or oval sections along its length. The side view gives the
    top and bottom lines, and the top view gives the half width. Sixteen to 24 points per section.
  - **Wings and tail surfaces:** a thin airfoil section (6 to 8 points: a round leading edge and a
    sharp trailing edge) lofted root to tip. Take the chord and sweep from the top view, the dihedral
    from the front view, and the thickness from the side view. Mirror for the other wing.
  - **Control surfaces:** an inset line on the wing, a V-groove seam (E) set back from the trailing
    edge.
  - **Engine nacelles:** lathe the side-view profile and blend them into the wing with a short loft.
  - **Propellers:** 2 to 4 twisted blades, each a 4-point section lofted 3 times, plus a lathed
    spinner. Keep them separate, with the pivot on the axis.
  - **Canopy:** a recessed glass loft with frame bars as V-grooves or thin trim strips.
  - **Landing gear:** struts are 6-sided tubes, wheels use the wheel recipe, and the gear doors
    are thin plates. Rest the aircraft on its gear at Z = 0.
  - **Budget guide:** fuselage 30%, wings 25%, tail 10%, nacelles and props 20%, gear 10%, detail 5%.
- **Tracked vehicles:**
  - **Hull and turret:** three-profile intersection for both.
  - **Tracks:** a swept closed band around the road wheels, with tread as alternate-face extrusions.
  - **Road wheels:** the wheel recipe.
  - **Gun barrel:** lathed.

## Round 3 lessons (v1.3: roadster, truck, aircraft)
- **Budget the hero wheel.** The sector-array wheel costs about 540 tris once trimmed (no
  per-sector lug pockets, 16-sided tyre, 20-sided rim). On a 5k car, keep one centre nut. Put a dark
  brake drum behind open spokes, or daylight shows through.
- **Missing views.** When there is no top view, intersect two profiles (side and front) and take the
  widths from the front and rear views.
- **Disagreeing views.** When the front and rear (or any two views of the same projection) disagree,
  overlay their masks first. Their mutual IoU caps the score, so record the cap and stop chasing it.
- **Open bodies (roadsters, cabs).** Cut the cockpit as a rounded-rectangle tub cutter. Build a raked
  windscreen as a ruled surface between a base curve and a top curve, with a swept frame. Roll bars,
  side pipes and bumper bars are swept tubes.
- **An overlapping island inside the body is silently dropped by an EXACT boolean** (round 3 lost a
  bonnet scoop this way). Make add-ons such as scoops their own objects.
- **Disc wheels:** use the every-other-face trick on a narrow lug-nut band, not the whole disc,
  which turns spoked. Off-road tread: two staggered rows of alternate-face extrusions.
- **Racks and slats** that are solid in the mask: posts and slats over a recessed dark backing
  board.
- **Ink thickness.** Masks include the drawing's line weight. Model thin parts (wings, tails, frames)
  about 1.5 cm fat on each edge.
- **Propellers.** The masks have no blades, so park one blade straight down to keep the front-view
  red to a minimum. Use at least 10 to 12 airfoil points when hinge grooves are needed.
- **Aircraft paint.** Split the upper and lower camouflage by face normal (wing, tail, nacelles) and
  by profile segment (fuselage). Recalculate normals FIRST.
- **Determinism.** `bmesh.ops.solidify` scrambles the GLB index order. Build thin shells (glass) by
  hand when rebuilds must be byte-identical.
- **Paint defaults by type:** red for cars, olive drab for military trucks, two-tone grey-green for
  aircraft. Never add markings, insignia or numbers.

## Phase 0. Measure before modelling (C, B, D)
Write a short brief block before any geometry:
- AXES and ORIGIN (e.g. +Z up, front -Y, origin on the ground between the axles).
- SCALE: length, body width, height, wheelbase, axle Ys, wheel diameter, tyre width, track, ground
  clearance, arch radius, roof width, mirror span.
- **STATIONS** (A): the Y positions where the profile changes. Bumper tips, each wheel's front edge,
  axle and rear edge, windshield base, roof front and rear, rear-window base, deck end, tail.
- BUDGET (D): the triangle cap, split before you start. A rough guide for 4k is body 1.2k, 4 wheels
  at 220 each (0.9k), glass 0.2k, lights, grille and mirrors 0.4k, and the rest kept in reserve.
- CENTRELINES (v1.1): find each view's centre from symmetric features (tyre outer edges, body
  sides away from the mirrors), never from a bounding box that includes mirrors, aerials or
  callouts. In round 2 that mistake put the masks 2.8 cm off centre.
- CONFLICTS: where views disagree (for example, the mirrors are in the front view but not the side,
  or the nose tips reach past the stated length). Decide once, write it down, and don't fight the
  scorer over it later.

## Phase 1. Blockout from three views (G, C, A)
The body is ONE closed, mirrored half shell lofted through the stations:
- At each station, take the **front-view half profile** (a fixed set of 8 to 12 points from the
  centreline, over the roof and shoulder, down to the sill) and **scale its X** by the top view's
  half width at that Y.
- **Clamp each point's Z** between the side view's floor line and roof line at that Y. Because the
  clamp does the work, one fixed point count gives the hood, cabin, deck and arches, and no
  station ever needs special topology.
- Add 3 to 4 extra stations over each wheel so the arch can bend (step 2).
- **Alternative body (E, v1.1):** extrude the side, plan and front outlines as three prisms and
  intersect them with EXACT booleans. It gives one closed body in three operations and scored
  best in round 2. Use it when the views are clean outlines. Use the clamped loft when you need
  rounded sections (tumblehome, crowned hood).
- Bridge consecutive rings, cap both ends, and keep X = 0 verts exactly on the centreline. Mirror on
  X with clipping (or build both halves in bmesh: same result).
- Wheels at this stage are 16-sided cylinders, each its own object with its **origin at the axle**.
- **Gate 1**: render orthographic Workbench silhouettes (flat black on white, framed like the
  blueprints) and compare them with the blueprint masks. Aim for IoU of at least 0.95 per view, and
  check proportions to within 2 percent. Fix proportions now. No detail goes on a wrong blockout.

## Phase 2. Wheel arches and wells (A, D, plus the fix nobody had)
- **The arch comes from existing verts** (A): the station rings over a wheel carry verts at
  arch-top and rocker heights. Move those onto the arch circle. Put the middle arch verts halfway
  between the circle and its outer tangent, not exactly on it: four faces exactly on the circle cut
  into the opening, and the halfway point scores better.
- **Wells with liners** (none of the seven contenders did this, and every one of them left
  see-through tunnels):
  1. Cut each opening with a cylinder cutter that reaches only from the body side to the inner
     tyre face plus 2 cm, not through the whole width.
  2. Close the cut with a liner: a half cylinder (8 segments) in dark trim, inset from the arch lip
     by about 2 cm. This keeps the side silhouette open behind the tyre and stops the car being
     see-through from low angles.
- Keep the arch's lip flush (the low-poly look), or extrude the lip faces out 1 to 2 cm for a flare.

## Phase 3. Forms by moving edges (A)
Shape by moving existing verts and edges. Add at most one loop per feature:
- Round the roof and upper-side corners by pulling every OTHER edge inward (tumblehome, the roof
  at about 65 to 70 percent of body width). Drop the hood's outer edges. Pull the nose's middle
  edges forward and keep the outer ones back.
- Pull a door line onto existing edges so a seam can run along it in phase 4.
- Tuck the bottom edges and the tail in slightly. Never leave a perfectly boxy lower body.

## Phase 4. Recessed details with cutters (E, B, A)
Details are recesses, not stickers:
- Keep every cutter in a `CUTTERS` collection that is hidden from render and export (B).
- Cutters are named for their part (`CUT_headlight_R`), are convex, and overlap the body by at
  least 5 mm on every side. **Extending past the body only works on convex faces** (E's lesson): at
  a concave crease, split the cutter into two convex pieces instead.
- Use EXACT booleans with **material transfer**: the cut floor takes the cutter's material, so
  glass, lamps and grille need no face picking (E). Apply the modifiers in a fixed order, then
  delete the cutters' references.
- In plain bmesh (no booleans), do the same with **inset, then extrude inward** (A): headlights
  inset 1 cm and pushed in 2 cm. For the grille, extrude and scale down, then extrude in (no inset:
  an inset leaves an extra centre face). Side windows are inset, pushed in 1.5 cm, with their
  bottom edge on the beltline.
- **Glass** (C, A): a separate object, either the recessed faces split off and solidified to 5 mm
  or a thin shell sitting 2 to 3 mm inside the opening, never proud of the body. Use a dark, mostly
  opaque material (about 0.85 alpha or opaque dark blue-grey). Very clear glass shows an empty car.
- **Mirrors**: an arm and a housing, the housing turned towards the driver, the root sunk into the
  body.
- **Recesses on curved panels (v1.1):** ray cast the outline onto the body, fit one plane through
  the hits, and put the cutter floor 15 to 25 mm behind that plane. Flat-panel recipes go uneven on
  a curved nose.
- **Glass budget (v1.1):** make one plane per window from the cutter's floor outline, sitting 3 mm
  inside, then solidify. Solidifying the split-off boolean floor faces cost 924 triangles against 104.
- **Bumpers and roof rails (v1.1):** a bumper is a shell lofted from the body's own end stations,
  12 to 20 mm proud of the body, with a dark lower lip and the plate recess cut into it. Roof rails
  are a ray-cast bar plus two or three feet, sunk into the roof so their tops match the side line.
- **Seams (E, v1.1):** make one closed V-section solid per seam, swept along a surface-snapped path.
  Per-segment groove cutters that share planes break the mesh.
- Optional seams for character: one shallow, 1-segment chamfered groove along the door line (E's
  weighted chamfer), costing about 40 triangles.

## Phase 5. Wheels (A, C)
For each wheel:
1. Lathe a 16 to 20-sided profile: tyre tread with a slight bulge on the sidewall, then the rim lip,
   the rim face and the hub. Cap the inner ring.
2. **Spokes from one extrusion** (A): on the rim-face ring, select every other face, extrude, scale
   towards the hub and push 1.5 cm inward. You get 8 spokes, or 10 on a 20-sided ring.
3. Make the rim face a lighter metal than the hub recess, so the spokes read at gameplay size.
   A flat disc with no recess reads as a hubcap: avoid it.
4. Put a vertex at the bottom of the tyre so the car sits on Z = 0. Duplicate the wheel as linked
   copies, origins at the axles.
- Budget: 180 to 260 triangles per wheel.

**Hero wheels (v1.2, sector array, from the Praize3d rim study):** for spokes of any shape:
1. Model ONE half-sector in polar coordinates, with half-angle pi/N.
2. Shape the spokes as quad strips that dish back towards the hub: z = dish * (1 - r/R).
3. Add a chamfer ridge on the spoke faces (inset plus slide, or a 1-segment bevel), a hex
   lug nut, and a 12-gon lug pocket in the sector.
4. Mirror the sector across its centreline. Then repeat it N times, either with an Array modifier
   (fixed count N, object offset = an empty rotated 2*pi/N, merge on) or in bmesh (duplicate,
   rotate, remove doubles).
5. Lathe the lip, barrel and tyre separately (barrel 32, tyre 24 to 30, bore 20 segments).

The spoke count is then independent of the rim's segment count. Use it for hero cars. Keep the
every-other-face trick for background vehicles (the 1.5k to 3k tris here, against 250).
**Lights (v1.2):** cut the light opening by deleting the faces inside a surface-snapped outline and
extruding the hole's rim inward into a socket. Then build the light as its own object:
- **Bezel:** an inset ring, slightly proud.
- **Lens:** pushed back.
- **Light bars:** thin quad strips.
- **Grooves:** inset-and-push borders round each element.

Give each layer its own material (housing, lens, emissive bars, reflector). Mark sharp on every
step and bezel edge and shade smooth by angle: crisp at low poly, with no subdivision.

## Phase 6. Topology, modifiers and shading (D, E)
- **Modifier order** (D): Mirror, then Boolean (cutters), then Bevel (angle limit 30 degrees,
  1 segment, **5 to 15 mm**), then Weighted Normal. The "0.5 to 2 percent of size" rule is far too
  big for a low-poly car.
- **Bevel cost (v1.1):** a 30-degree bevel applied after the boolean recesses added about 70% to
  the body's triangles in round 2. Bevel only edges between two paint faces (bevel weights), keep
  recess and boolean edges crisp, and budget the bevel before cutting details.
- **Slivers (v1.1):** keep cutter edges at least 1.5 cm from station creases, or the bevel turns
  them non-manifold.
- Then clean (merge by distance, dissolve degenerate faces, recalculate normals). Shading is flat,
  or Weighted Normal for the slightly polished look.
- **bmesh traps** (A, C):
  - `extrude_face_region` keeps the original faces: delete them, or you get non-manifold edges.
  - Extruding a face that touches the mirror plane leaves a wall on the plane: remove it.
  - `obj.data.materials.clear()` resets the material indices on every face: reassign them after.
    This hit two contenders. Check it with a material-shaded look render, not with numbers.
  - `inset_region` re-links the faces next to the inset: gather every detail face group before
    the first inset.
  - Use absolute paths everywhere. A relative output path resolves against Blender's working
    directory, which may be `C:\`.
- **Check, then refuse** (E): before saving, fail the build if there are non-manifold edges, if the
  triangle count is over budget, if a required material is missing, if min Z is not 0, or if a
  visible helper is left over. Builds must be deterministic, so two runs give the same file.

## Phase 7. Materials (A)
Use material slots, assigned by face or through the cutter transfer:
- **Paint**: red, roughness 0.35, specular 0.4.
- **Glass**: dark blue-grey, roughness 0.1.
- **Trim**: near black, roughness 0.6. Use it for the grille, rocker band and well liners.
- **Rubber**: roughness 0.87.
- **Rim metal**: light grey, roughness 0.3, metallic 0.6.
- **Headlights**: emission, cool white, strength 5.
- **Tail lights**: emission, red, strength 4.
- A dark **rocker band** (A) between the arches makes the car look lower and is free: assign trim
  to the bottom-most side faces.

## Phase 8. Look gate and delivery (F, C)
- After every build, render a **look sheet**: clay and material renders from the front three
  quarter, rear three quarter, side, front, top and a low angle, plus a 128 px gameplay-size strip.
  Look at it. The low angle catches see-through wells, and the 128 px strip shows whether the
  wheels, lights and glass read at game size.
- Spend scorer or harness runs only after your own silhouette and look gates pass.
- Export GLB with modifiers applied and the cutters excluded, then reimport it and check the counts.
