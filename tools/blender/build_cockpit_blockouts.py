"""Low-poly cockpit block-outs, built the CSG way, to choose a layout before any of them is modelled in detail.

The owner, 2026-10-24: "I prefer you make low poly low textured verions before going full detail with these",
"use blender to your advantage, lots of CSG, lots of boolean meshes to get the basic idea across of how each
cockpit will fnction", "I think to make it comfy all mechs should be sitting", "Windows shoul be a closed shape.
What if mech sinks underwater, what if mech goes into the stratos pehjre, you have to keep pilot safe, air tight,
and geometry has to make logical sense when opening and closing, no cliping", "now I want to see more! with more
square panels", "these long thin vertical panels are terrible for combat view".

Every block-out is the same few booleans, so how each one works can be read off the model:

  shell     the cabin is an airtight solid: its plan extruded and grown by WALL_M, minus the plan extruded (Exact)
  screens   square screens are recesses cut 2 cm into the inner wall by cutters carrying SCREEN_allview, so a
            screen is the floor of its recess and the seams between screens are the wall left between cutters;
            the screen straight ahead is one panel with nothing down its middle
  pillars   structural pillars are boxes unioned onto the wall where the plan turns, outside the clear view
  opening   each layout opens its own way (MECH_TEXT): a part is the shell cut by a region (a sector round the
            eye's axis, or a box) with Intersect, and the body keeps the shell minus that region, the part's region
            G smaller so the two never share a face; the cut faces carry GASKET, the seal. A part's motions are a
            chain of nodes, one motion each (pop off the seal, lift, slide back, tilt), so a canopy can rise, run
            back and tip up; clamps on the seams slide clear first, rams join the body to the part, vents puff
            steam as the seal breaks. Every part only ever moves away from the body and the cabin, so nothing
            passes through anything. Each layout is also built entered by the back (<id>_rear.glb: a ramp,
            a hatchback, two doors or a rear visor in the back wall, then the seat swivels round to face it), so the
            front need never move (the owner: "Or back of mech can open")
  inside    a seat with armrests and grips, a dash with four MFDs, three MFDs hanging from the roof; MFD bezels
            are boxes with a recess cut for the display (SCREEN_mfd_<page>)

Everyone sits (the owner: standing "takes a lot of room, and the pilot could get seriously injured in a fight").
Flat colours, no textures. Writes assets/models/cockpit_blockouts/<id>.glb and assets/models/cockpit_blockouts.json
(per layout: name, text, eye, the clear view ahead measured from the eye, the screens' span, the opening's
tracks, rams and vents, triangles). The clear view is the angle between the nearest structural pillars or
opening seams either side of straight ahead that are not themselves screen seams.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_cockpit_blockouts.py [-- id ...]
"""
import json
import math
import os
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "cockpit_blockouts")
MANIFEST = os.path.join(ROOT, "assets", "models", "cockpit_blockouts.json")
G2B = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # glTF frame (y up, z ahead) to Blender
D = math.pi / 180
WALL_M = 0.08        # the shell's wall
RECESS_M = 0.02      # how deep a screen sits in the wall
SEAM_M = 0.012       # the wall left between two screens
SEAM_DEG = 0.6       # a sector part is this much narrower each side than the hole it closes
EYE_Y = 1.20         # seated


def arc(r, a0, a1, n):
    return [(r * math.sin((a0 + (a1 - a0) * i / n) * D), r * math.cos((a0 + (a1 - a0) * i / n) * D)) for i in range(n + 1)]


def mirror(half):
    """A plan given as its left half from the front round to the back: mirrored, the front left as one span."""
    return [(-x, z) for x, z in reversed(half)] + list(half)


LAYOUTS = [
    dict(id="panorama", rear="hatch", mech="clamshell", name="Panorama", rows=[(0.95, 1.80)], plan=mirror([(0.95, 0.85), (1.05, 0.15), (0.95, -0.45)]),
         pillars=[2, 3], mfd=("top", "dash"),
         text="One wide front screen, 1.9 m across, the pillars pushed out to the corners; square side screens to the "
              "shoulders. Armored Core's front."),
    dict(id="tilewall", rear="ramp", mech="petals", name="Tile wall", rows=[(0.80, 1.38), (1.41, 1.99)], plan=arc(1.0, -105, 105, 11), pillars=[0, 11],
         hatch_deg=42, mfd=("dash",),
         text="Two rows of square tiles round 210 degrees, a centimetre of wall between them and no pillar ahead: the "
              "structure is behind the pilot's shoulders."),
    dict(id="allview", rear="visor", mech="visor", name="All-view sphere", dome=dict(r=1.15, segs=16, rings=9), hatch_deg=46, mfd=("arms",),
         text="The Gundam pod: a ball of flat screens all round, under the feet and overhead too; the seat on its arm in "
              "the middle, the MFDs on the armrests."),
    dict(id="titan", rear="ramp", mech="lift", name="Titan", rows=[(0.92, 1.72)], plan=mirror([(0.80, 0.75), (1.00, 0.10), (0.90, -0.40)]), pillars=[2, 3],
         heavy=0.12, mfd=("top", "dash"),
         text="Titanfall's: a front screen 1.6 m across, side screens raked back, a heavy frame but all of it outside 45 "
              "degrees."),
    dict(id="cinema", rear="hatch", mech="door", name="Cinema", rows=[(0.90, 1.95)], plan=mirror([(1.10, 0.95), (1.15, 0.05)]), pillars=[1, 2], mfd=("dash",),
         text="One huge front screen 2.2 m by 1.05 m with no seam at all, square screens beside the knees."),
    dict(id="cab", rear="ramp", mech="ramp", name="Excavator cab", rows=[(0.30, 1.10), (1.13, 1.93)], plan=mirror([(0.80, 0.80), (0.95, 0.10), (0.90, -0.45)]),
         pillars=[2, 3], floor=True, mfd=("top",),
         text="Floor to roof screens like a digger's glass cab, and a screen under the feet looking down at the ground "
              "ahead."),
    dict(id="octagon", rear="hatch", mech="gullwing", name="Octagon", rows=[(0.75, 1.40), (1.43, 2.05)],
         plan=mirror([(0.80, 0.80), (1.05, 0.45), (1.05, -0.20), (0.70, -0.60)]), pillars=[3, 4], mfd=("top", "dash"),
         text="The Stevedore's octagon, seated now and stretched so its front is 1.6 m across: pillars at 45 degrees "
              "instead of 21."),
    dict(id="canopy", rear="barn", mech="canopy_rear", top="hump", name="Canopy", rows=[(0.85, 1.45)], plan=mirror([(0.85, 0.80), (1.00, 0.10), (0.90, -0.40)]), pillars=[2, 3],
         ceiling=True, mfd=("dash",),
         text="A fighter's canopy in flat screens: the front half of the ceiling is screens too, so you see up as well "
              "as out."),
    dict(id="twintier", rear="barn", mech="elevator", top="hump", name="Twin tier", rows=[(0.78, 1.24), (1.30, 1.90)], plan=arc(0.95, -95, 95, 9), pillars=[0, 9],
         hatch_deg=40, mfd=("dash",),
         text="Two rings of square screens round 190 degrees with one thin rail between them under the eye line."),
    dict(id="lowbank", rear="ramp", mech="canopy_front", top="hump", name="Low bank", rows=[(0.80, 1.42)], plan=mirror([(0.75, 1.00), (0.98, 0.50), (0.95, -0.40)]), pillars=[2, 3],
         mfd=("dash",),
         text="The Ironhorse's low screen bank, seated and widened: a front 1.5 m across, the wings swept back past the "
              "knees."),
]

# the built interiors' screen corners (build_cockpit.py's numbers), measured the same way for comparison
BUILT = dict(
    wall=dict(eye=(0.0, 1.20, 0.0), plan=mirror([(0.56, 1.00), (0.86, 0.45), (0.86, -0.16)])),
    harness=dict(eye=(0.0, 1.65, 0.0), plan=[((0.76 / math.cos(math.pi / 8)) * math.sin((j - 3.5) * math.pi / 4),
                                              0.07 + (0.76 / math.cos(math.pi / 8)) * math.cos((j - 3.5) * math.pi / 4)) for j in range(8)]),
    jockey=dict(eye=(0.0, 1.10, 0.30), plan=mirror([(0.40, 1.06), (0.77, 0.74)])),
)


def bearing(x, z, ez=0.0):
    return math.atan2(x, z - ez) / D


def clear_deg(angles):
    left = min([a for a in angles if a > 0.5], default=180)
    right = max([a for a in angles if a < -0.5], default=-180)
    return round(min(360, left - right))


# ----------------------------------------------------------------------------- Blender helpers

MATS = {}


def mat(name, rgb, emit=0.0, rough=0.6, metal=0.2):
    if name in MATS:
        return MATS[name]
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*rgb, 1)
    b.inputs["Roughness"].default_value = rough
    b.inputs["Metallic"].default_value = metal
    if emit:
        b.inputs["Emission Color"].default_value = (*rgb, 1)
        b.inputs["Emission Strength"].default_value = emit
    MATS[name] = m
    return m


def materials():
    mat("HULL", (0.20, 0.21, 0.24), rough=0.7, metal=0.3)
    mat("LINING", (0.33, 0.34, 0.37), rough=0.8)
    mat("FRAME", (0.12, 0.12, 0.14), rough=0.5, metal=0.6)
    mat("GASKET", (0.95, 0.42, 0.08), rough=0.9)
    mat("SEAT", (0.10, 0.10, 0.11), rough=0.9)
    mat("CONSOLE", (0.26, 0.27, 0.30), rough=0.6, metal=0.3)
    mat("GRIP", (0.55, 0.56, 0.60), rough=0.4, metal=0.7)
    mat("BEZEL", (0.04, 0.04, 0.05), rough=0.5)
    mat("SCREEN_allview", (0.05, 0.12, 0.10), emit=0.5)
    for k in ("status", "nav", "weapons", "radar"):
        mat(f"SCREEN_mfd_{k}", (0.05, 0.25, 0.12), emit=1.0)


def new_obj(name, bm, material):
    bmesh.ops.transform(bm, matrix=G2B, verts=bm.verts)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.materials.append(MATS[material])
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def prism(name, poly, y0, y1, material):
    """A solid over a convex plan polygon [(x, z)], glTF frame."""
    bm = bmesh.new()
    lo = [bm.verts.new((x, y0, z)) for x, z in poly]
    hi = [bm.verts.new((x, y1, z)) for x, z in poly]
    bm.faces.new(lo)
    bm.faces.new(hi)
    for i in range(len(poly)):
        j = (i + 1) % len(poly)
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return new_obj(name, bm, material)


def box(name, c, axes, size, material):
    """A box centred on c (glTF frame) with axes (r, u, n) and size (w, h, d)."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0, calc_uvs=True)
    axes = [Vector(a) for a in axes]
    if axes[0].cross(axes[1]).dot(axes[2]) < 0:     # a left-handed frame turns the box inside out, and a boolean
        axes[0] = -axes[0]                          # with an inside-out cutter does nothing
    R = Matrix([tuple(a) for a in axes]).transposed().to_4x4()
    bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(Vector(c)) @ R @ Matrix.Diagonal((*size, 1)))
    return new_obj(name, bm, material)


def sphere(name, r, c, segs, rings, material):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=segs, v_segments=rings, radius=r)
    bmesh.ops.rotate(bm, verts=bm.verts, cent=(0, 0, 0), matrix=Matrix.Rotation(math.pi / segs, 3, "Z"))   # a panel ahead
    bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(Vector(c)) @ G2B.inverted())
    return new_obj(name, bm, material)


def boolean(target, cutter, op, keep=False):
    """An Exact boolean with the cutter's materials carried onto the faces it makes; the cutter is removed."""
    for m in cutter.data.materials:
        if m.name not in [s.name for s in target.data.materials]:
            target.data.materials.append(m)
    mod = target.modifiers.new("csg", "BOOLEAN")
    mod.operation = op
    mod.solver = "EXACT"
    mod.object = cutter
    mod.material_mode = "TRANSFER"
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    if not keep:
        bpy.data.objects.remove(cutter)
    return target


def join(objs, name):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.join()
    objs[0].name = name
    return objs[0]


def copy(ob, name):
    c = ob.copy()
    c.data = ob.data.copy()
    c.name = name
    bpy.context.scene.collection.objects.link(c)
    return c


def offset(poly, d):
    """A convex polygon grown by d (cockpit_hull.offset)."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import cockpit_hull
    return cockpit_hull.offset(poly, d)


def facing(c, look):
    """Axes (r, u, n) of a panel at c turned to look at a point (glTF frame)."""
    n = (Vector(look) - Vector(c)).normalized()
    r = Vector((0, 1, 0)).cross(n).normalized()
    u = n.cross(r).normalized()
    return r, u, n


# ----------------------------------------------------------------------------- the shell

def loft(name, keys, material):
    """A closed solid through plan rings [(y, [(x, z), ...])], each with the same number of points (glTF frame)."""
    bm = bmesh.new()
    rings = [[bm.verts.new((x, y, z)) for x, z in poly] for y, poly in keys]
    n = len(rings[0])
    bm.faces.new(rings[0])
    bm.faces.new(rings[-1])
    for r0, r1 in zip(rings, rings[1:]):
        for i in range(n):
            bm.faces.new((r0[i], r0[(i + 1) % n], r1[(i + 1) % n], r1[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return new_obj(name, bm, material)


HUMP = ((0.30, 0.20), (0.48, 0.42))       # a canopy bubble over the walls: (rise, how far in) for each ring


def shell_keys(outline, H, grow, top):
    """The rings of a cabin's solid: straight walls to H, then for a "hump" top a faceted bubble that rises and
    narrows, the way a fighter's canopy does. grow is how far the rings stand outside the cabin."""
    if top != "hump":
        return [(-grow, offset(outline, grow)), (H + grow, offset(outline, grow))]
    return [(-grow, offset(outline, grow)), (H, offset(outline, grow))] + \
        [(H + dy + grow, offset(outline, grow - inset)) for dy, inset in HUMP]


def ray_poly(poly, b_deg):
    """Where a ray from the eye's axis at bearing b meets a polygon: the point, the edge's tangent, its normal out."""
    d = Vector((math.sin(b_deg * D), math.cos(b_deg * D)))
    for i in range(len(poly)):
        p, q = Vector(poly[i]), Vector(poly[(i + 1) % len(poly)])
        e = q - p
        den = d.x * (-e.y) - d.y * (-e.x)
        if abs(den) < 1e-9:
            continue
        t = (p.x * (-e.y) - p.y * (-e.x)) / den
        s = (d.x * p.y - d.y * p.x) / den
        if t > 0 and -1e-6 <= s <= 1 + 1e-6:
            tg = e.normalized()
            n = Vector((tg.y, -tg.x))
            if n.dot(p + e / 2) < 0:
                n = -n
            return d * t, tg, n
    raise ValueError(b_deg)


def cross_x(poly, z):
    """Where a plan polygon crosses the line at depth z: the x of each crossing."""
    xs = []
    for i in range(len(poly)):
        (x0, z0), (x1, z1) = poly[i], poly[(i + 1) % len(poly)]
        if (z0 - z) * (z1 - z) <= 0 and z0 != z1:
            xs.append(x0 + (x1 - x0) * (z - z0) / (z1 - z0))
    return xs


def wall_z(poly, x):
    """How far ahead the cabin's wall is at a given x: the furthest crossing of the plan with the line at x."""
    zs = []
    for i in range(len(poly)):
        (x0, z0), (x1, z1) = poly[i], poly[(i + 1) % len(poly)]
        if (x0 - x) * (x1 - x) <= 0 and x0 != x1:
            zs.append(z0 + (z1 - z0) * (x - x0) / (x1 - x0))
    return max(zs)


# ----------------------------------------------------------------------------- the mechanisms
# A part is a region of the shell cut free (a sector round the eye's axis, or a box), the motions that open it (each a
# slide along an axis or a turn about an axis through a pivot, with its start and length in seconds; each motion rides
# on the ones before it, so a canopy can lift, then slide back, then tilt), and its hardware: hydraulic rams from the
# body to the part, clamps on the seam that slide clear before anything moves, and vents that puff as the seal breaks.
G = 0.006          # a part's region is this much smaller all round than the hole cut for it: the seam
CLAMP = (0.08, 0.06, 0.04)

MECH_TEXT = dict(
    clamshell="the front splits at the screens' foot: clamps slide off the seams, both halves pop off their seals, the "
              "bottom drops to the deck as a step, then the top swings up about the roof edge carrying the dash",
    petals="three petals: the two side ones swing out on their outer edges like doors, then the middle one pops and "
           "swings up carrying the dash; the seams fall on the screens' own seams",
    visor="a curved visor plate, outside the ball, turns up over the top about the pilot's eye; then the seat runs "
          "forward on its rail to the opening",
    lift="the top half rises clear of the roof line, then tilts back; the bottom half drops as a step",
    door="a heavy door: clamps slide off the free edge, it pops off its seal, then swings out on three hinges at its "
         "right edge with the dash on it",
    ramp="the whole front, floor to roof, unclamps and drops forward on its bottom edge to the deck as a ramp",
    gullwing="gull-wing doors: each side, roof edge and all, turns up about a hinge on the roof, and the front drops "
             "forward as a step with the dash on it",
    canopy_rear="a fighter's canopy: everything above the sill, screens, roof and bubble, lifts off its seal and "
                "tilts up on its back edge on two rams",
    canopy_front="a fighter's canopy hinged at the front: the whole bubble above the sill lifts and swings up and "
                 "forward over the nose on two rams",
    elevator="the upper ring of screens and the bubble rise straight up on four rams and tilt back, then the front of "
             "the lower ring drops as a step",
    rear_ramp="by the back: the back wall unclamps and drops back to the deck as a ramp, then the seat swivels round to "
              "face it; the screens ahead never move",
    rear_hatch="by the back: a hatchback, the back wall and the back of the roof swinging up on two rams, then the seat "
               "swivels round to face it; the screens ahead never move",
    rear_barn="by the back: two doors swing out on hinges at the back corners, then the seat swivels round to face them; "
              "the screens ahead never move",
    rear_visor="by the back: a visor plate behind the ball turns up over the top about the pilot's eye, then the seat "
               "swivels round to face the opening",
)


def stage(kind, axis, v, t0, d, pivot=None):
    return dict(kind=kind, axis=axis, v=v, t0=t0, d=d, pivot=pivot)


def mechanism(L, c):
    """The parts that open this layout, from its mechanism and the cabin's measures c."""
    m, H, F, B, top, deg, W = c["mech"], c["H"], c["front"], c["back"], c["top"], c["deg"], WALL_M
    y_lo, y_hi = -W - 0.2, top + 0.2
    out = lambda b, y: (b, y, "out")      # noqa: E731   a clamp on a bearing seam, sliding away from the part
    down = lambda b, y: (b, y, "down")    # noqa: E731   a clamp on a horizontal seam, sliding down onto the body
    if m == "clamshell":
        ys = L["rows"][0][0] - 0.06
        return [dict(name="UPPER", region=("sector", -deg, deg, ys, y_hi), keep_inner=True,
                     stages=[stage("pos", "z", 0.04, 0.35, 0.35), stage("rot", "x", -1.45, 1.4, 1.5, (0, top, F + W))],
                     clamps=[out(s * deg, (ys + H) / 2) for s in (1, -1)], dash=True,
                     vents=[((s * 0.5, ys + 0.02, F + W), (s * 0.3, -0.3, 1)) for s in (1, -1)]),
                dict(name="LOWER", region=("sector", -deg, deg, y_lo, ys - 0.02), keep_inner=True,
                     hole=("sector", -deg, deg, y_lo, ys + 0.005),
                     stages=[stage("pos", "z", 0.04, 0.35, 0.3), stage("rot", "x", 1.42, 0.6, 1.4, (0, -W, F + W))],
                     clamps=[out(s * deg, ys / 2) for s in (1, -1)])]
    if m == "petals":
        a, b = c["vertex"](28), c["vertex"](67)
        parts = [dict(name="PETAL_M", region=("sector", -a, a, y_lo, y_hi), keep_inner=True,
                      stages=[stage("pos", "z", 0.04, 1.5, 0.3), stage("rot", "x", -1.5, 1.8, 1.4, (0, top, F + W))],
                      dash=True, vents=[((0, H * 0.5, F + W), (0, 0.2, 1))])]
        for sg, nm in ((1, "PETAL_L"), (-1, "PETAL_R")):
            p, _, _ = ray_poly(c["outer"], sg * b)
            parts.append(dict(name=nm, region=("sector", a, b, y_lo, y_hi) if sg > 0 else ("sector", -b, -a, y_lo, y_hi),
                              keep_inner=True, stages=[stage("rot", "y", sg * 1.75, 0.3, 1.5, (p.x, 0, p.y))],
                              vents=[((p.x - sg * 0.02, H * 0.5, p.y + 0.02), (sg, 0.1, 0.4))]))
        return parts
    swivel = dict(name="SEAT", region="seat", stages=[stage("rot", "y", math.pi, 2.5, 1.6, (0, 0, -0.12))])
    if m in ("rear_ramp", "rear_hatch", "rear_barn"):     # by the back, so nothing ahead of the pilot ever moves
        xe, zb = c["xb"] - 0.12, B - W
        side = lambda y: [dict(at=(s * xe, y, zb - 0.022), axes=((1, 0, 0), (0, 1, 0), (0, 0, -1)), slide=(s, 0, 0))  # noqa: E731
                          for s in (1, -1)]
        if m == "rear_ramp":
            return [dict(name="RAMP", region=("box", -xe, xe, 0.04, H - 0.04, zb - 0.3, B + 0.03),
                         stages=[stage("pos", "z", -0.04, 0.35, 0.3), stage("rot", "x", -1.5, 0.75, 2.0, (0, 0.04, zb))],
                         clamps=side(0.5) + side(H - 0.5),
                         vents=[((0, H - 0.1, zb), (0, 0.3, -1))] + [((s * xe, 0.3, zb), (s * 0.3, 0, -1)) for s in (1, -1)]),
                    swivel]
        if m == "rear_hatch":
            rx = min(0.5, xe - 0.12)
            return [dict(name="HATCH", region=("box", -xe, xe, 0.30, top + 0.2, zb - 0.3, B + 0.45),
                         stages=[stage("pos", "y", 0.03, 0.35, 0.3), stage("rot", "x", 1.6, 0.7, 2.0, (0, top, B + 0.45))],
                         rams=[((s * rx, top + 0.04, B + 0.85), (s * rx, top + 0.04, B + 0.2)) for s in (1, -1)],
                         clamps=side(0.7) + side(H - 0.4),
                         vents=[((0, top, B + 0.45), (0, 1, 0.2)), ((0, 0.3, zb), (0, -0.2, -1))]),
                    swivel]
        doors = []
        for sg, nm in ((1, "DOOR_L"), (-1, "DOOR_R")):
            doors.append(dict(name=nm, region=("box", 0.0, xe, 0.04, H - 0.04, zb - 0.3, B + 0.03) if sg > 0 else
                              ("box", -xe, 0.0, 0.04, H - 0.04, zb - 0.3, B + 0.03),
                              stages=[stage("pos", "z", -0.03, 0.35, 0.3),
                                      stage("rot", "y", -sg * 1.65, 0.7 + (0.15 if sg < 0 else 0), 1.8, (sg * xe, 0, zb))],
                              clamps=[dict(at=(sg * 0.3, y, zb - 0.022), axes=((1, 0, 0), (0, 1, 0), (0, 0, -1)), slide=(0, d, 0))
                                      for y, d in ((H - 0.04, 1), (0.04, -1))],
                              vents=[((sg * 0.02, H * 0.5, zb), (sg * 0.3, 0, -1))]))
        return doors + [swivel]
    if m == "rear_visor":
        return [dict(name="VISOR", region="visor_rear", stages=[stage("rot", "x", 1.95, 0.5, 2.0, (0, EYE_Y, 0))],
                     clamps="dome_back", vents=[((s * 0.95, EYE_Y, -0.75), (s, 0.1, -0.6)) for s in (1, -1)]),
                swivel]
    if m == "visor":
        return [dict(name="VISOR", region="visor", stages=[stage("rot", "x", -1.95, 0.5, 2.0, (0, EYE_Y, 0))],
                     clamps="dome", vents=[((s * 0.95, EYE_Y, 0.75), (s, 0.1, 0.6)) for s in (1, -1)]),
                dict(name="SEAT", region="seat", stages=[stage("pos", "z", 0.6, 2.6, 1.4)])]
    if m == "lift":
        ys = L["rows"][0][0] - 0.06
        return [dict(name="UPPER", region=("sector", -deg, deg, ys, y_hi), keep_inner=True,
                     stages=[stage("pos", "z", 0.04, 0.3, 0.3), stage("pos", "y", top - ys + 0.08, 0.7, 1.4),
                             stage("rot", "x", -0.55, 2.2, 0.9, (0, ys, F - 0.1))],
                     clamps=[out(s * deg, (ys + H) / 2) for s in (1, -1)],
                     vents=[((s * 0.55, ys + 0.02, F + W), (s * 0.3, -0.2, 1)) for s in (1, -1)]),
                dict(name="LOWER", region=("sector", -deg, deg, y_lo, ys - 0.02), keep_inner=True, dash=True,
                     hole=("sector", -deg, deg, y_lo, ys + 0.005),
                     stages=[stage("pos", "z", 0.04, 1.6, 0.3), stage("rot", "x", 1.42, 1.9, 1.3, (0, -W, F + W))],
                     clamps=[out(s * deg, ys / 2) for s in (1, -1)])]
    if m == "door":
        wd = (F + W) * math.tan(deg * D)
        return [dict(name="DOOR", region=("sector", -deg, deg, y_lo, y_hi), keep_inner=True, dash=True,
                     stages=[stage("pos", "z", 0.04, 0.35, 0.3), stage("rot", "y", -1.75, 0.75, 2.2, (-wd, 0, F + W))],
                     clamps=[out(deg, y) for y in (0.45, H * 0.5, H - 0.45)],
                     hinges=[(-wd, y, F + W) for y in (0.35, H * 0.5, H - 0.35)],
                     vents=[((s * wd, H * 0.5, F + W), (s * 0.5, 0, 1)) for s in (1, -1)])]
    if m == "ramp":
        return [dict(name="RAMP", region=("sector", -deg, deg, y_lo, y_hi), keep_inner=True,
                     stages=[stage("pos", "z", 0.04, 0.35, 0.3), stage("rot", "x", 1.5, 0.75, 2.0, (0, -W, F + W))],
                     clamps=[out(s * deg, y) for s in (1, -1) for y in (0.5, H - 0.5)],
                     vents=[((0, top - 0.05, F + W), (0, 0.4, 1))])]
    if m == "gullwing":
        b0, b1 = c["seam_after_pillar"], c["vertex"](101) - 0.5
        parts = []
        for sg, nm in ((1, "WING_L"), (-1, "WING_R")):
            hx = sg * c["hinge_x"]
            parts.append(dict(name=nm, region=("sector", b0, b1, 0.06, y_hi) if sg > 0 else ("sector", -b1, -b0, 0.06, y_hi),
                              keep_inner=True,
                              stages=[stage("pos", "y", 0.03, 0.3, 0.3), stage("rot", "z", sg * 1.75, 0.65, 1.8, (hx, top, 0))],
                              vents=[((hx, top, 0.2), (sg * 0.3, 1, 0.1)), ((sg * 1.1, 0.1, 0.2), (sg, -0.2, 0.2))]))
        return parts + [dict(name="FRONT", region=("sector", -deg, deg, y_lo, y_hi), keep_inner=True, dash=True,
                             stages=[stage("pos", "z", 0.04, 0.35, 0.3), stage("rot", "x", 1.5, 0.9, 2.0, (0, -W, F + W))],
                             vents=[((0, top - 0.05, F + W), (0, 0.4, 1))])]
        # the owner: "octagon front I think still needs to open"
    if m == "canopy_rear":
        ys = L["rows"][0][0] - 0.05
        rams = []
        for s in (1, -1):
            p, _, n = ray_poly(c["outer"], s * 95)
            q = p + n * 0.09
            rams.append(((q.x, ys - 0.6, q.y), (q.x, ys + 0.15, q.y)))
        return [dict(name="CANOPY", region=("box", -3, 3, ys, y_hi, -3, 3),
                     stages=[stage("pos", "y", 0.05, 0.35, 0.3), stage("rot", "x", -1.0, 0.75, 2.0, (0, ys, B - W))],
                     rams=rams, clamps=[down(s * b, ys) for s in (1, -1) for b in (85, 110)],
                     vents=[((s * 1.0, ys, z), (s, 0.2, 0)) for s in (1, -1) for z in (0.4, -0.3)])]
    if m == "canopy_front":
        ys = L["rows"][0][0] - 0.05
        return [dict(name="CANOPY", region=("box", -3, 3, ys, y_hi, -3, 3),
                     stages=[stage("pos", "y", 0.05, 0.35, 0.3), stage("rot", "x", 1.25, 0.75, 2.0, (0, ys, F + W))],
                     rams=[((s * 1.13, ys - 0.55, 0.55), (s * 1.13, ys + 0.15, -0.2)) for s in (1, -1)],
                     clamps=[down(s * b, ys) for s in (1, -1) for b in (75, 105)],
                     vents=[((s * 1.0, ys, z), (s, 0.2, 0)) for s in (1, -1) for z in (0.3, -0.3)])]
    if m == "elevator":
        ys = L["rows"][1][0] - 0.03
        lo = c["vertex"](53)
        return [dict(name="TOP", region=("box", -3, 3, ys, y_hi, -3, 3),
                     stages=[stage("pos", "y", 0.7, 0.4, 1.6), stage("rot", "x", -0.45, 2.1, 1.0, (0, ys, B - W))],
                     rams=[((sx * 1.08, ys - 0.6, z), (sx * 1.08, ys + 0.2, z)) for sx in (1, -1) for z in (-0.25, -0.6)],
                     clamps=[down(s * b, ys) for s in (1, -1) for b in (70, 112)],
                     vents=[((s * 0.95, ys, z), (s, 0.1, 0)) for s in (1, -1) for z in (0.3, -0.4)]),
                dict(name="LOWER", region=("sector", -lo, lo, y_lo, ys), keep_inner=True, dash=True,
                     hole=("sector", -lo, lo, y_lo, ys + 0.005),
                     stages=[stage("pos", "z", 0.04, 1.8, 0.3), stage("rot", "x", 1.42, 2.1, 1.4, (0, -W, F + W))])]
    raise ValueError(m)


# ----------------------------------------------------------------------------- a flat-walled cabin's measures

def cabin(L, mech):
    """A flat-walled layout's measures, pure Python (no Blender): its outline (the plan closed round the back,
    counter-clockwise), the pillars' bearings, the opening's half-angle and the measures c the mechanisms
    read. The block-out and the detailed interior (build_cockpit.py) both build from this."""
    rows, plan = L["rows"], L["plan"]
    H = rows[-1][1] + 0.12
    back = min(-0.75, min(z for _, z in plan) - 0.25)
    outline = [plan[0]] + list(plan[1:]) + [(plan[-1][0], back), (plan[0][0], back)]
    area = sum(outline[i][0] * outline[(i + 1) % len(outline)][1] - outline[(i + 1) % len(outline)][0] * outline[i][1]
               for i in range(len(outline)))
    if area < 0:
        outline = outline[::-1]
    tp = L.get("top", "flat")
    pillar_angles = [bearing(*plan[i]) for i in L.get("pillars", [])]
    deg = L.get("hatch_deg") or (min(abs(a) for a in pillar_angles) - 1.5)
    verts_b = sorted(abs(bearing(x, z)) for x, z in plan)
    c = dict(mech=mech, H=H, front=max(z for _, z in plan), back=back, deg=deg, outer=offset(outline, WALL_M),
             top=H + WALL_M + (HUMP[-1][0] if tp == "hump" else 0.0),
             vertex=lambda a_: min(verts_b, key=lambda v: abs(v - a_)),
             seam_after_pillar=(min(abs(a) for a in pillar_angles) + 2.5) if pillar_angles else deg,
             hinge_x=0.84, xb=min(abs(x) for x, z in outline if abs(z - back) < 1e-6))
    return outline, pillar_angles, deg, c


# ----------------------------------------------------------------------------- one layout

def build(L, door="front"):
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    eye = Vector((0.0, EYE_Y, 0.0))
    is_dome = "dome" in L
    statics = []                       # meshes that stay on the pod
    c = dict(mech=L["mech"] if door == "front" else "rear_" + L["rear"])
    oid = L["id"] if door == "front" else L["id"] + "_rear"
    if is_dome:
        dm = L["dome"]
        r, segs, rings = dm["r"], dm["segs"], dm["rings"]
        shell = sphere("shell", r + WALL_M, eye, segs, rings, "HULL")
        boolean(shell, sphere("inner", r, eye, segs, rings, "SCREEN_allview"), "DIFFERENCE")
        deg = L["hatch_deg"]
        floor_y, inner_keep, pillar_angles = EYE_Y - r, None, []
        c.update(H=EYE_Y + r, front=r, back=-r, top=EYE_Y + r + WALL_M, deg=deg)
    else:
        rows, plan = L["rows"], L["plan"]
        outline, pillar_angles, deg, cm = cabin(L, c["mech"])
        c.update(cm)
        H, tp = c["H"], L.get("top", "flat")
        shell = loft("shell", shell_keys(outline, H, WALL_M, tp), "HULL")
        boolean(shell, loft("cabin", shell_keys(outline, H, 0.0, tp), "LINING"), "DIFFERENCE")
        cutters = []
        for y0, y1 in rows:
            h = y1 - y0
            for i in range(len(plan) - 1):
                a, b = Vector((plan[i][0], 0, plan[i][1])), Vector((plan[i + 1][0], 0, plan[i + 1][1]))
                ahead = (bearing(a.x, a.z) < 0) != (bearing(b.x, b.z) < 0) and a.z + b.z > 0
                n = 1 if ahead else max(1, round((b - a).length / h))
                t = (b - a).normalized()
                inward = Vector((-t.z, 0, t.x))
                if inward.dot(-(a + b) / 2) < 0:
                    inward = -inward
                for k in range(n):
                    cc = a.lerp(b, (k + 0.5) / n)
                    w = (b - a).length / n - SEAM_M
                    cutters.append(box(f"rec{len(cutters)}", (cc.x, (y0 + y1) / 2, cc.z), (t, Vector((0, 1, 0)), inward),
                                       (w, h, 2 * RECESS_M), "SCREEN_allview"))
        if L.get("ceiling"):           # screens overhead, in the bubble's lid or the flat roof
            hy = H + (HUMP[-1][0] if tp == "hump" else 0.0)
            for k, (x0, x1) in enumerate(((-0.52, -0.02), (0.02, 0.52))):
                cutters.append(box(f"ceil{k}", ((x0 + x1) / 2, hy, 0.02), (Vector((1, 0, 0)), Vector((0, 0, 1)), Vector((0, -1, 0))),
                                   (x1 - x0, 0.55, 2 * RECESS_M), "SCREEN_allview"))
        if L.get("floor"):             # the cab: a screen under the feet, ahead
            cutters.append(box("floorscreen", (0, 0, 0.45), (Vector((1, 0, 0)), Vector((0, 0, 1)), Vector((0, 1, 0))),
                               (1.3, 0.5, 2 * RECESS_M), "SCREEN_allview"))
        boolean(shell, join(cutters, "recesses"), "DIFFERENCE")
        pil = []
        for i in L.get("pillars", []):
            x, z = plan[i]
            p = Vector((x, H / 2, z))
            inward = Vector((-x, 0, -z)).normalized()
            pil.append(box(f"pillar{i}", p + inward * 0.04, (Vector((0, 1, 0)).cross(inward), Vector((0, 1, 0)), inward),
                           (L.get("heavy", 0.08), H, 0.12), "FRAME"))
        if pil:
            boolean(shell, join(pil, "pillars"), "UNION")
        floor_y = 0.0
        inner_keep = offset(outline, -0.03)

    def region_solid(name, reg, shrink, keep_inner):
        if reg[0] == "sector":
            _, a0, a1, y0, y1 = reg
            da = SEAM_DEG * shrink / G
            a0, a1, y0, y1 = a0 + da, a1 - da, y0 + shrink, y1 - shrink
            pts = [(0.0, 0.0)] + [(6.0 * math.sin(a * D), 6.0 * math.cos(a * D)) for a in
                                  (a0, a0 + (a1 - a0) / 4, (a0 + a1) / 2, a1 - (a1 - a0) / 4, a1)]
            cut = prism(name, pts, y0, y1, "GASKET")
            if keep_inner and inner_keep:
                boolean(cut, prism(name + "_in", inner_keep, -1.0, 9.0, "GASKET"), "DIFFERENCE")
            elif is_dome:
                boolean(cut, sphere(name + "_in", 0.6, eye, 12, 6, "GASKET"), "DIFFERENCE")
            return cut
        _, x0, x1, y0, y1, z0, z1 = reg
        return box(name, ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), ((1, 0, 0), (0, 1, 0), (0, 0, 1)),
                   (x1 - x0 - 2 * shrink, y1 - y0 - 2 * shrink, z1 - z0 - 2 * shrink), "GASKET")

    parts = mechanism(L, c)
    objs = {P["name"]: [] for P in parts}
    for P in parts:                     # every part is cut from the whole shell before any hole is made in it
        if P["region"] in ("visor", "visor_rear"):
            rr, sg_, rg_ = L["dome"]["r"], L["dome"]["segs"], L["dome"]["rings"]
            cb_, lo_ = (180, 0.80) if P["region"] == "visor_rear" else (0, 0.88)     # behind, clear of the seat's boom
            plate = sphere("visor", rr + 2 * WALL_M + 0.01, eye, sg_, rg_, "HULL")
            boolean(plate, sphere("visor_in", rr + WALL_M + 0.01, eye, sg_, rg_, "SCREEN_allview"), "DIFFERENCE")   # screens inside
            boolean(plate, region_solid("visor_cut", ("sector", cb_ - deg - 5, cb_ + deg + 5, EYE_Y - lo_, EYE_Y + 0.83), 0, False), "INTERSECT")
            objs["VISOR"].append(plate)
        elif P["region"] != "seat":
            piece = copy(shell, P["name"].lower())
            boolean(piece, region_solid(P["name"] + "_cut", P["region"], G, P.get("keep_inner")), "INTERSECT")
            objs[P["name"]].append(piece)
    for P in parts:
        if P["region"] in ("visor", "visor_rear"):
            cb_, lo_ = (180, 0.72) if P["region"] == "visor_rear" else (0, 0.8)
            boolean(shell, region_solid("hole", ("sector", cb_ - deg, cb_ + deg, EYE_Y - lo_, EYE_Y + 0.75), 0, False), "DIFFERENCE")
        elif P["region"] != "seat":
            boolean(shell, region_solid(P["name"] + "_hole", P.get("hole", P["region"]), 0, P.get("keep_inner")), "DIFFERENCE")
    statics.append(shell)
    # inside: the seat with armrests and grips, on its post (in the sphere, a boom from the back of the ball)
    I3 = ((1, 0, 0), (0, 1, 0), (0, 0, 1))
    seat = [box("seat_pan", (0, floor_y + 0.45, -0.10), I3, (0.52, 0.12, 0.52), "SEAT"),
            box("seat_back", (0, floor_y + 0.92, -0.40), facing((0, floor_y + 0.92, -0.40), (0, floor_y + 1.0, 1.0)), (0.52, 0.85, 0.12), "SEAT")]
    for sg in (1, -1):
        seat.append(box(f"arm{sg}", (sg * 0.34, floor_y + 0.62, 0.0), I3, (0.12, 0.08, 0.62), "CONSOLE"))
        seat.append(box(f"grip{sg}", (sg * 0.34, floor_y + 0.72, 0.26), I3, (0.05, 0.16, 0.06), "GRIP"))
    seat_o = join(seat, "seat")
    post_top = floor_y + 0.388          # 2 mm under the pan, so the seat can run off it
    post = box("seat_post", (0, (floor_y + post_top) / 2, -0.12), I3, (0.18, post_top - floor_y, 0.18), "CONSOLE")
    if is_dome:
        boolean(post, box("boom", (0, 0.24, -0.75), facing((0, 0.24, -0.75), (0, 0.33, -0.25)), (0.16, 0.16, 1.05), "CONSOLE"), "UNION")
        statics.append(box("seat_rail", (0, post_top - 0.015, 0.25), I3, (0.10, 0.03, 0.95), "FRAME"))
    statics.append(post)
    (objs["SEAT"] if "SEAT" in objs else statics).append(seat_o)
    # the MFDs: a bezel box with a recess cut for the display, which is a quad with its own UVs
    kinds = ["status", "nav", "weapons", "radar"]

    def mfd(name, cc, size, kind):
        ax = facing(cc, eye)
        b_ = box(name, cc, ax, (size + 0.05, size + 0.05, 0.05), "BEZEL")
        boolean(b_, box(name + "_rec", Vector(cc) + ax[2] * 0.025, ax, (size, size, 0.016), "BEZEL"), "DIFFERENCE")
        bm = bmesh.new()
        r_, u_, n_ = ax
        q0 = Vector(cc) + n_ * (0.025 - 0.008 + 0.001)
        vs = [bm.verts.new(q0 + r_ * sx * size / 2 + u_ * sy * size / 2) for sx, sy in ((1, -1), (-1, -1), (-1, 1), (1, 1))]
        f = bm.faces.new(vs)
        uv = bm.loops.layers.uv.new("UVMap")
        for lp, t in zip(f.loops, ((1, 1), (0, 1), (0, 0), (1, 0))):      # glTF's v runs down the image
            lp[uv].uv = t
        bm.normal_update()
        if f.normal.dot(n_) < 0:
            f.normal_flip()
        return [b_, new_obj(name + "_screen", bm, f"SCREEN_mfd_{kind}")]
    where = L["mfd"]
    if "dash" in where:
        y0 = L["rows"][0][0]
        zf = c["front"] - 0.30
        carrier = next((P for P in parts if P.get("dash")), None)
        # never wider than the cabin where it stands, nor than the doorway it rides out through (the owner: the
        # control panel was in the way of climbing out, "you had control panel attacked to the door earlier")
        half = min(abs(x) for x in cross_x(outline, zf + 0.1)) - 0.08
        if carrier and carrier["region"][0] == "sector" and carrier["name"] != "PETAL_M":
            half = min(half, c["front"] * math.tan(max(abs(carrier["region"][1]), abs(carrier["region"][2])) * D) - 0.17)
        w = min(1.5, 2 * half)
        step = (w - 0.06) / 4
        size = min(0.24, step - 0.06)
        dash = box("dash", (0, y0 - 0.16, zf), facing((0, y0 - 0.16, zf), (0, y0 + 0.6, -0.4)), (w, 0.30, 0.10), "CONSOLE")
        if carrier:                      # brackets from the dash to the opening part's own wall
            legs = [box(f"dash_bracket{sg}", (sg * w * 0.3, y0 - 0.2, (zf + wall_z(outline, sg * w * 0.3)) / 2), I3,
                        (0.06, 0.06, wall_z(outline, sg * w * 0.3) - zf + 0.01), "FRAME") for sg in (1, -1)]
        else:
            legs = [box(f"dash_strut{sg}", (sg * w * 0.4, (y0 - 0.16) / 2, zf - 0.05), I3, (0.06, y0 - 0.16, 0.06), "FRAME") for sg in (1, -1)]
        home = objs[carrier["name"]] if carrier else statics
        home.append(join([dash] + legs, "dash"))
        for i in range(4):
            home.extend(mfd(f"mfd_dash{i}", ((i - 1.5) * step, y0 - 0.08, zf - 0.08), size, kinds[i]))
    if "top" in where:                  # on the viewscreen itself, along the top of the front screen, as a HUD (the owner:
        y1 = L["rows"][-1][1]           # "this upper mfd, should take up some realestate on teh viewscreen")
        size = 0.30                     # about 20 degrees over the eye line, kept inside one row of screens
        row = next((r for r in L["rows"] if r[0] <= EYE_Y + 0.3 <= r[1]), L["rows"][-1])
        yc = min(max(EYE_Y + 0.3, row[0] + size / 2 + 0.03), row[1] - size / 2 - 0.03)
        zs = c["front"] + RECESS_M      # the floor of the front screen's recess
        home = next((objs[P["name"]] for P in parts if P["region"][0] == "sector" and P["region"][1] < 0 < P["region"][2]
                     and P["region"][3] <= yc <= P["region"][4]), statics)
        for i, x in enumerate((-0.36, 0.0, 0.36)):
            for nm, sz, dz, mt in (("frame", size + 0.03, 0.0015, "BEZEL"), ("screen", size, 0.003, f"SCREEN_mfd_{kinds[(i + 2) % 4 if i != 1 else 3]}")):
                bm = bmesh.new()
                # the panel's right is the pilot's right, -x (as facing() gives for a panel looking back at the eye)
                vs = [bm.verts.new((x - sx * sz / 2, yc + sy * sz / 2, zs - dz)) for sx, sy in ((1, -1), (-1, -1), (-1, 1), (1, 1))]
                f = bm.faces.new(vs)
                uv = bm.loops.layers.uv.new("UVMap")
                for lp, t in zip(f.loops, ((1, 1), (0, 1), (0, 0), (1, 0))):
                    lp[uv].uv = t
                bm.normal_update()
                if f.normal.z > 0:
                    f.normal_flip()
                home.append(new_obj(f"mfd_inset_{nm}{i}", bm, mt))
    if "arms" in where:
        for i, (x, z) in enumerate(((0.34, 0.16), (-0.34, 0.16), (0.34, 0.42), (-0.34, 0.42))):
            (objs["SEAT"] if "SEAT" in objs else statics).extend(mfd(f"mfd_arm{i}", (x, floor_y + 0.78 + 0.05 * (i // 2), z), 0.2, kinds[i]))
    for P in parts:                     # a door's hinge knuckles: a pin on the body, a knuckle above and below it on the door
        for k, (x, y, z) in enumerate(P.get("hinges", [])):
            statics.append(box(f"hinge_pin{k}", (x - 0.02, y, z + 0.035), I3, (0.05, 0.12, 0.07), "FRAME"))
            for dy in (-0.13, 0.13):
                objs[P["name"]].append(box(f"hinge_knuckle{k}{dy > 0}", (x + 0.035, y + dy, z + 0.035), I3, (0.07, 0.12, 0.07), "FRAME"))
    # the clamps: blocks on the seam, outside, that slide clear before the part moves
    clamps = []                         # (name, centre, axes, slide direction, start)
    for P in parts:
        t_first = min(st["t0"] for st in P["stages"])
        spec = P.get("clamps") or []
        if spec in ("dome", "dome_back"):
            rr = L["dome"]["r"] + WALL_M + 0.002 + CLAMP[2] / 2
            back_ = spec == "dome_back"
            for k, s in enumerate((1, -1)):
                b = (180 * back_ + s * (deg + 8)) * D
                n = Vector((math.sin(b), 0, math.cos(b)))
                tg = Vector((math.cos(b), 0, -math.sin(b))) * s
                clamps.append((f"CLAMP_{P['name']}_{k}", eye + n * rr, (tg, Vector((0, 1, 0)), n), tg, t_first))
            e_ = -47 * D
            zs = -1 if back_ else 1
            n = Vector((0, math.sin(e_), zs * math.cos(e_)))
            tg = Vector((0, math.cos(e_), -zs * math.sin(e_)))
            clamps.append((f"CLAMP_{P['name']}_2", eye + n * rr, (Vector((1, 0, 0)), tg, n), -tg, t_first))
            continue
        for k, item in enumerate(spec):
            if isinstance(item, dict):           # given outright: centre, axes (along, up, out) and the way it slides
                ax_ = tuple(Vector(a_) for a_ in item["axes"])
                clamps.append((f"CLAMP_{P['name']}_{k}", Vector(item["at"]), ax_, Vector(item["slide"]), t_first))
                continue
            b, y, how = item
            p, tg, n = ray_poly(c["outer"], b)
            tg = Vector((tg.x, 0, tg.y))
            n = Vector((n.x, 0, n.y))
            grow = Vector((math.cos(b * D), 0, -math.sin(b * D))) * (1 if b > 0 else -1)
            if tg.dot(grow) < 0:
                tg = -tg
            at = Vector((p.x, y, p.y)) + n * (0.002 + CLAMP[2] / 2)
            clamps.append((f"CLAMP_{P['name']}_{k}", at, (tg, Vector((0, 1, 0)), n),
                           tg if how == "out" else Vector((0, -1, 0)), t_first))
    # the node tree: the pod; each part a chain of nodes, one motion each, its meshes on the last
    pod = bpy.data.objects.new("pod", None)
    bpy.context.scene.collection.objects.link(pod)

    def empty(name, parent, at):
        e = bpy.data.objects.new(name, None)
        bpy.context.scene.collection.objects.link(e)
        e.parent = parent
        bpy.context.view_layer.update()
        e.matrix_world = Matrix.Translation(G2B @ Vector(at))
        bpy.context.view_layer.update()
        return e

    def adopt(ob, parent):
        mw = ob.matrix_world.copy()
        ob.parent = parent
        bpy.context.view_layer.update()
        ob.matrix_world = mw

    tracks, rams, vents, last = [], [], [], {}
    for P in parts:
        parent, at = pod, Vector((0, 0, 0))
        for k, st in enumerate(P["stages"]):
            at = Vector(st["pivot"]) if st.get("pivot") else at
            node = f"{P['name']}_{k}"
            parent = empty(node, parent, at)
            tr = dict(node=node, t0=st["t0"], d=st["d"], v=st["v"])
            tr.update({"pos": st["axis"]} if st["kind"] == "pos" else {"axis": st["axis"]})
            tracks.append(tr)
        last[P["name"]] = parent
        for ob in objs[P["name"]]:
            adopt(ob, parent)
        for k, (base, tip) in enumerate(P.get("rams", [])):
            empty(f"RAMB_{P['name']}_{k}", pod, base)
            empty(f"RAMT_{P['name']}_{k}", parent, tip)
            rams.append(dict(base=f"RAMB_{P['name']}_{k}", tip=f"RAMT_{P['name']}_{k}"))
        t_first = min(st["t0"] for st in P["stages"])
        for k, (pt, dirv) in enumerate(P.get("vents", [])):
            empty(f"VENT_{P['name']}_{k}", pod, pt)
            vents.append(dict(node=f"VENT_{P['name']}_{k}", dir=list(dirv), t0=t_first, d=0.9))
    for name, at, axes, slide, t_first in clamps:
        e = empty(name, pod, at)
        cb = box(name.lower(), at, axes, CLAMP, "GASKET")
        adopt(cb, e)
        tracks.append(dict(node=name, dir=[round(v, 4) for v in slide], v=0.06, t0=max(0.0, t_first - 0.35), d=0.3))
    for ob in statics:
        adopt(ob, pod)
    empty("EYE_pilot", last.get("SEAT", pod), eye)      # the eye rides with the seat where the seat moves
    tris = 0
    for ob in [o for o in bpy.data.objects if o.type == "MESH"]:
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        bmesh.ops.triangulate(bm, faces=bm.faces)
        bm.to_mesh(ob.data)
        tris += len(ob.data.polygons)
        bm.free()
    os.makedirs(OUT, exist_ok=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(filepath=os.path.join(OUT, f"{oid}.glb"), export_format="GLB", use_selection=True,
                              export_apply=True, export_yup=True, export_extras=True)
    # the clear view ahead: between the nearest pillars, or seams of the opening that do not fall on screen seams
    if is_dome:
        clear, span = 360, 360
    else:
        on_seam = lambda a_: any(abs(abs(a_) - abs(bearing(x, z))) < 0.6 for x, z in L["plan"])   # noqa: E731
        seams = [a_ for P in parts if P["region"][0] == "sector" for a_ in P["region"][1:3] if not on_seam(a_)]
        b = [bearing(x, z) for x, z in L["plan"]]
        clear, span = clear_deg(pillar_angles + seams), round(max(b) - min(b))
    return dict(id=L["id"], name=L["name"], text=L["text"], file=f"cockpit_blockouts/{oid}.glb", pose="seated",
                eye_m=list(eye), clear_deg=clear, span_deg=span, tris=tris, mech=c["mech"], opening=MECH_TEXT[c["mech"]],
                open_s=round(max(t["t0"] + t["d"] for t in tracks), 2),
                parts=[P["name"] for P in parts], tracks=tracks, rams=rams, vents=vents, mfd=list(L["mfd"]))


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    materials()
    old = json.load(open(MANIFEST)) if os.path.exists(MANIFEST) else {}
    done = {d["id"]: d for d in old.get("layouts", [])}
    for L in LAYOUTS:
        if argv and L["id"] not in argv:
            continue
        done[L["id"]] = build(L)
        rear = build(L, "back")                  # the same cabin entered by the back (owner, 2026-10-24: "Or back of mech can open")
        done[L["id"]]["rear"] = {k: rear[k] for k in ("file", "clear_deg", "tris", "mech", "opening", "open_s", "parts", "tracks", "rams", "vents")}
        print("BUILT", L["id"], done[L["id"]]["tris"], "tris, clear", done[L["id"]]["clear_deg"], "| back door", rear["clear_deg"], flush=True)
    built = {k: dict(clear_deg=clear_deg([bearing(x, z, v["eye"][2]) for x, z in v["plan"]]),
                     span_deg=round(max(bearing(x, z, v["eye"][2]) for x, z in v["plan"]) - min(bearing(x, z, v["eye"][2]) for x, z in v["plan"])))
             for k, v in BUILT.items()}
    with open(MANIFEST, "w") as f:
        json.dump(dict(generator="tools/blender/build_cockpit_blockouts.py", seated=True,
                       layouts=[done[L["id"]] for L in LAYOUTS if L["id"] in done], built=built), f, indent=1)


if __name__ == "__main__":       # build_cockpit.py imports the layouts and their mechanisms
    main()
