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
  hatch     the front is split off the shell by a wedge cutter (Intersect for the hatch, Difference for the body,
            the hatch's wedge SEAM_DEG narrower so the two never share a face); the cut faces carry GASKET, the
            seal the hatch closes on. The hatch first pops POP_M straight out, then swings up about its top front
            edge: every point of it stays ahead of that edge and below it, so it never passes through the cabin
  inside    a seat with armrests and grips, a dash carried on the hatch with four MFDs, three MFDs hanging from the
            roof; MFD bezels are boxes with a recess cut for the display (SCREEN_mfd_<page>)

Everyone sits (the owner: standing "takes a lot of room, and the pilot could get seriously injured in a fight").
Flat colours, no textures. Writes assets/models/cockpit_blockouts/<id>.glb and assets/models/cockpit_blockouts.json
(per layout: name, text, eye, the clear view ahead measured from the eye, the screens' span, the hatch's motion,
triangles). The clear view is the angle between the nearest structural pillars or hatch seams either side of
straight ahead.

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
SEAM_DEG = 0.6       # the hatch's wedge is this much narrower each side than the hole it closes
POP_M = 0.04         # the hatch first moves straight out by this, off its seal
SWING = -1.6         # then swings up about its top front edge (radians about x)
EYE_Y = 1.20         # seated


def arc(r, a0, a1, n):
    return [(r * math.sin((a0 + (a1 - a0) * i / n) * D), r * math.cos((a0 + (a1 - a0) * i / n) * D)) for i in range(n + 1)]


def mirror(half):
    """A plan given as its left half from the front round to the back: mirrored, the front left as one span."""
    return [(-x, z) for x, z in reversed(half)] + list(half)


LAYOUTS = [
    dict(id="panorama", name="Panorama", rows=[(0.95, 1.80)], plan=mirror([(0.95, 0.85), (1.05, 0.15), (0.95, -0.45)]),
         pillars=[2, 3], mfd=("top", "dash"),
         text="One wide front screen, 1.9 m across, the pillars pushed out to the corners; square side screens to the "
              "shoulders. Armored Core's front."),
    dict(id="tilewall", name="Tile wall", rows=[(0.80, 1.38), (1.41, 1.99)], plan=arc(1.0, -105, 105, 11), pillars=[0, 11],
         hatch_deg=42, mfd=("dash",),
         text="Two rows of square tiles round 210 degrees, a centimetre of wall between them and no pillar ahead: the "
              "structure is behind the pilot's shoulders."),
    dict(id="allview", name="All-view sphere", dome=dict(r=1.15, segs=16, rings=9), hatch_deg=46, mfd=("arms",),
         text="The Gundam pod: a ball of flat screens all round, under the feet and overhead too; the seat on its arm in "
              "the middle, the MFDs on the armrests."),
    dict(id="titan", name="Titan", rows=[(0.92, 1.72)], plan=mirror([(0.80, 0.75), (1.00, 0.10), (0.90, -0.40)]), pillars=[2, 3],
         heavy=0.12, mfd=("top", "dash"),
         text="Titanfall's: a front screen 1.6 m across, side screens raked back, a heavy frame but all of it outside 45 "
              "degrees."),
    dict(id="cinema", name="Cinema", rows=[(0.90, 1.95)], plan=mirror([(1.10, 0.95), (1.15, 0.05)]), pillars=[1, 2], mfd=("dash",),
         text="One huge front screen 2.2 m by 1.05 m with no seam at all, square screens beside the knees."),
    dict(id="cab", name="Excavator cab", rows=[(0.30, 1.10), (1.13, 1.93)], plan=mirror([(0.80, 0.80), (0.95, 0.10), (0.90, -0.45)]),
         pillars=[2, 3], floor=True, mfd=("top",),
         text="Floor to roof screens like a digger's glass cab, and a screen under the feet looking down at the ground "
              "ahead."),
    dict(id="octagon", name="Octagon", rows=[(0.75, 1.40), (1.43, 2.05)],
         plan=mirror([(0.80, 0.80), (1.05, 0.45), (1.05, -0.20), (0.70, -0.60)]), pillars=[3, 4], mfd=("top", "dash"),
         text="The Stevedore's octagon, seated now and stretched so its front is 1.6 m across: pillars at 45 degrees "
              "instead of 21."),
    dict(id="canopy", name="Canopy", rows=[(0.85, 1.45)], plan=mirror([(0.85, 0.80), (1.00, 0.10), (0.90, -0.40)]), pillars=[2, 3],
         ceiling=True, mfd=("dash",),
         text="A fighter's canopy in flat screens: the front half of the ceiling is screens too, so you see up as well "
              "as out."),
    dict(id="twintier", name="Twin tier", rows=[(0.78, 1.24), (1.30, 1.90)], plan=arc(0.95, -95, 95, 9), pillars=[0, 9],
         hatch_deg=40, mfd=("dash",),
         text="Two rings of square screens round 190 degrees with one thin rail between them under the eye line."),
    dict(id="lowbank", name="Low bank", rows=[(0.80, 1.42)], plan=mirror([(0.75, 1.00), (0.98, 0.50), (0.95, -0.40)]), pillars=[2, 3],
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


def wedge(a_deg, ez, y0, y1, r=6.0):
    """The region within a_deg of straight ahead from the eye's axis, as a prism."""
    pts = [(0.0, ez)] + [(r * math.sin(a * D), ez + r * math.cos(a * D)) for a in (-a_deg, -a_deg / 2, 0, a_deg / 2, a_deg)]
    return pts, y0, y1


def facing(c, look):
    """Axes (r, u, n) of a panel at c turned to look at a point (glTF frame)."""
    n = (Vector(look) - Vector(c)).normalized()
    r = Vector((0, 1, 0)).cross(n).normalized()
    u = n.cross(r).normalized()
    return r, u, n


# ----------------------------------------------------------------------------- one layout

def build(L):
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    ez, eye = 0.0, Vector((0.0, EYE_Y, 0.0))
    is_dome = "dome" in L
    parts = []                        # (object, node) to export: node "pod" or "HATCH"
    if is_dome:
        dm = L["dome"]
        r, segs, rings = dm["r"], dm["segs"], dm["rings"]
        shell = sphere("shell", r + WALL_M, eye, segs, rings, "HULL")
        boolean(shell, sphere("inner", r, eye, segs, rings, "SCREEN_allview"), "DIFFERENCE")
        top_y, bot_y, front_z = EYE_Y + r + WALL_M, EYE_Y - r - WALL_M, r + WALL_M
        hatch_deg = L["hatch_deg"]
        cut_y = (EYE_Y - 0.62, EYE_Y + 0.75)
        floor_y = EYE_Y - r
        pillar_angles = []
    else:
        rows, plan = L["rows"], L["plan"]
        H = rows[-1][1] + 0.12
        back = min(-0.75, min(z for _, z in plan) - 0.25)
        outline = [plan[0]] + list(plan[1:]) + [(plan[-1][0], back), (plan[0][0], back)]
        # make the outline counter-clockwise from above for offset()
        area = sum(outline[i][0] * outline[(i + 1) % len(outline)][1] - outline[(i + 1) % len(outline)][0] * outline[i][1]
                   for i in range(len(outline)))
        if area < 0:
            outline = outline[::-1]
        shell = prism("shell", offset(outline, WALL_M), -WALL_M, H + WALL_M, "HULL")
        boolean(shell, prism("cabin", outline, 0.0, H, "LINING"), "DIFFERENCE")
        # the screens: recesses cut into the inner wall, square, the one straight ahead whole
        cutters = []
        for y0, y1 in rows:
            h = y1 - y0
            for i in range(len(plan) - 1):
                a, b = Vector((plan[i][0], 0, plan[i][1])), Vector((plan[i + 1][0], 0, plan[i + 1][1]))
                ahead = (bearing(a.x, a.z) < 0) != (bearing(b.x, b.z) < 0) and a.z + b.z > 0
                n = 1 if ahead else max(1, round((b - a).length / h))
                t = (b - a).normalized()
                inward = Vector((-t.z, 0, t.x))
                if inward.dot(Vector((0, 0, 0)) - (a + b) / 2) < 0:
                    inward = -inward
                for k in range(n):
                    c = a.lerp(b, (k + 0.5) / n)
                    w = (b - a).length / n - SEAM_M
                    cc = Vector((c.x, (y0 + y1) / 2, c.z)) - inward * 0.0
                    cutters.append(box(f"rec{len(cutters)}", cc, (t, Vector((0, 1, 0)), inward), (w, h, 2 * RECESS_M), "SCREEN_allview"))
        if L.get("ceiling"):           # the canopy: the front half of the ceiling is screens
            for k, (x0, x1) in enumerate(((-0.75, -0.02), (0.02, 0.75))):
                cutters.append(box(f"ceil{k}", ((x0 + x1) / 2, H, 0.35), (Vector((1, 0, 0)), Vector((0, 0, 1)), Vector((0, -1, 0))),
                                   (x1 - x0, 0.75, 2 * RECESS_M), "SCREEN_allview"))
        if L.get("floor"):             # the cab: a screen under the feet, ahead
            cutters.append(box("floorscreen", (0, 0, 0.5), (Vector((1, 0, 0)), Vector((0, 0, 1)), Vector((0, 1, 0))),
                               (1.3, 0.55, 2 * RECESS_M), "SCREEN_allview"))
        boolean(shell, join(cutters, "recesses"), "DIFFERENCE")
        # structural pillars where the plan turns, outside the clear view
        pil = []
        for i in L.get("pillars", []):
            x, z = plan[i]
            p = Vector((x, H / 2, z))
            inward = (Vector((0, H / 2, 0)) - p)
            inward.y = 0
            inward.normalize()
            w = L.get("heavy", 0.08)
            r_, u_, n_ = Vector((0, 1, 0)).cross(inward).normalized(), Vector((0, 1, 0)), inward
            pil.append(box(f"pillar{i}", p + inward * 0.04, (r_, u_, n_), (w, H, 0.12), "FRAME"))
        if pil:
            boolean(shell, join(pil, "pillars"), "UNION")
        pillar_angles = [bearing(*plan[i]) for i in L.get("pillars", [])]
        hatch_deg = L.get("hatch_deg") or (min(abs(a) for a in pillar_angles) - 1.5)
        top_y, bot_y = H + WALL_M, -WALL_M
        front_z = max(z for _, z in plan) + WALL_M
        cut_y = (-WALL_M - 0.1, H + WALL_M + 0.1)
        floor_y = 0.0
    # the hatch: the front of the shell inside the wedge, its own wedge a hair narrower; the cabin's inside is not cut
    inner_keep = None
    if not is_dome:
        inner_keep = offset(outline, -0.03)
    def wedge_cutter(name, a_deg):
        pts, y0, y1 = wedge(a_deg, ez, *cut_y)
        c = prism(name, pts, y0, y1, "GASKET")
        if inner_keep:
            boolean(c, prism(name + "_in", inner_keep, -1.0, 9.0, "GASKET"), "DIFFERENCE")
        else:
            boolean(c, sphere(name + "_in", 0.6, eye, 12, 6, "GASKET"), "DIFFERENCE")
        return c
    hatch = copy(shell, "hatch")
    boolean(shell, wedge_cutter("cutA", hatch_deg), "DIFFERENCE")
    boolean(hatch, wedge_cutter("cutB", hatch_deg - SEAM_DEG), "INTERSECT")
    parts += [(shell, "pod"), (hatch, "HATCH")]
    # the hinge: the hatch's top front edge
    hv = [hatch.matrix_world @ v.co for v in hatch.data.vertices]
    hinge_b = Vector((0.0, min(v.y for v in hv), max(v.z for v in hv)))           # Blender: -y ahead, z up
    hinge = (0.0, hinge_b.z, -hinge_b.y)                                         # glTF
    # inside: the seat with armrests and grips
    seat = [box("seat_pan", (0, floor_y + 0.45, -0.10), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))), (0.52, 0.12, 0.52), "SEAT"),
            box("seat_back", (0, floor_y + 0.92, -0.40), facing((0, floor_y + 0.92, -0.40), (0, floor_y + 1.0, 1.0)), (0.52, 0.85, 0.12), "SEAT"),
            box("seat_post", (0, floor_y + 0.2, -0.12), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))), (0.18, 0.42, 0.18), "CONSOLE")]
    for s in (1, -1):
        seat.append(box(f"arm{s}", (s * 0.34, floor_y + 0.62, 0.0), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))), (0.12, 0.08, 0.62), "CONSOLE"))
        seat.append(box(f"grip{s}", (s * 0.34, floor_y + 0.72, 0.26), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))), (0.05, 0.16, 0.06), "GRIP"))
    seat_o = join(seat, "seat")
    if is_dome:
        boolean(seat_o, box("boom", (0, EYE_Y - 0.9, -0.75), facing((0, EYE_Y - 0.9, -0.75), (0, EYE_Y - 0.3, 0.6)), (0.16, 0.16, 1.0), "CONSOLE"), "UNION")
    parts.append((seat_o, "pod"))
    # MFDs: a bezel box with a recess cut for the display
    kinds = ["status", "nav", "weapons", "radar"]
    def mfd(name, c, size, kind):
        ax = facing(c, eye)
        b_ = box(name, c, ax, (size + 0.05, size + 0.05, 0.05), "BEZEL")
        boolean(b_, box(name + "_rec", Vector(c) + ax[2] * 0.025, ax, (size, size, 0.016), "BEZEL"), "DIFFERENCE")
        # the display: a quad with its own UVs a millimetre proud of the recess floor
        bm = bmesh.new()
        r_, u_, n_ = ax
        cc = Vector(c) + n_ * (0.025 - 0.008 + 0.001)
        vs = [bm.verts.new(cc + r_ * sx * size / 2 + u_ * sy * size / 2) for sx, sy in ((1, -1), (-1, -1), (-1, 1), (1, 1))]
        f = bm.faces.new(vs)
        uv = bm.loops.layers.uv.new("UVMap")
        for lp, t in zip(f.loops, ((1, 1), (0, 1), (0, 0), (1, 0))):      # glTF's v runs down the image
            lp[uv].uv = t
        bm.normal_update()
        if f.normal.dot(n_) < 0:
            f.normal_flip()
        q = new_obj(name + "_screen", bm, f"SCREEN_mfd_{kind}")
        return [b_, q]
    where = L["mfd"]
    if "dash" in where:                # the dash rides on the hatch
        y0 = L["rows"][0][0]
        zf = max(z for _, z in L["plan"]) - 0.30
        dash = box("dash", (0, y0 - 0.16, zf), facing((0, y0 - 0.16, zf), (0, y0 + 0.6, -0.4)), (1.5, 0.30, 0.10), "CONSOLE")
        strut = [box(f"dash_strut{s}", (s * 0.6, (y0 - 0.16) / 2, zf + 0.1), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))),
                     (0.06, y0 - 0.16, 0.06), "FRAME") for s in (1, -1)]
        d = join([dash] + strut, "dash")
        parts.append((d, "HATCH"))
        for i, x in enumerate((-0.51, -0.17, 0.17, 0.51)):
            for o in mfd(f"mfd_dash{i}", (x, y0 - 0.08, zf - 0.08), 0.24, kinds[i]):
                parts.append((o, "HATCH"))
    if "top" in where:                 # hanging from the roof, in the upper edge of the view
        y1 = L["rows"][-1][1]
        zf = max(z for _, z in L["plan"]) - 0.45
        for i, x in enumerate((-0.34, 0.0, 0.34)):
            for o in mfd(f"mfd_top{i}", (x, y1 - 0.02, zf), 0.26, kinds[(i + 2) % 4 if i != 1 else 3]):
                parts.append((o, "pod"))
        parts.append((box("mfd_rail", (0, y1 + 0.13, zf - 0.02), (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))), (1.1, 0.04, 0.05), "FRAME"), "pod"))
    if "arms" in where:                # on the armrests
        for i, (x, z) in enumerate(((0.34, 0.16), (-0.34, 0.16), (0.34, 0.42), (-0.34, 0.42))):
            for o in mfd(f"mfd_arm{i}", (x, floor_y + 0.78 + 0.05 * (i // 2), z), 0.2, kinds[i]):
                parts.append((o, "pod"))
    # nodes: the pod and the hatch (its origin on the hinge), then export
    pod = bpy.data.objects.new("pod", None)
    bpy.context.scene.collection.objects.link(pod)
    hn = bpy.data.objects.new("HATCH", None)
    bpy.context.scene.collection.objects.link(hn)
    hn.parent = pod
    hn.location = G2B @ Vector(hinge)
    bpy.context.view_layer.update()          # the hinge's world matrix before anything is parented to it
    for ob, node in parts:
        mw = ob.matrix_world.copy()
        ob.parent = pod if node == "pod" else hn
        ob.matrix_world = mw
    sock = bpy.data.objects.new("EYE_pilot", None)
    bpy.context.scene.collection.objects.link(sock)
    sock.parent = pod
    sock.location = G2B @ eye
    # every boolean result triangulated, so the count is honest
    tris = 0
    for ob, _ in parts:
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        bmesh.ops.triangulate(bm, faces=bm.faces)
        bm.to_mesh(ob.data)
        tris += len(ob.data.polygons)
        bm.free()
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, f"{L['id']}.glb")
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=True, export_yup=True,
                              export_extras=True)
    if is_dome:
        angles, span = [], 360
    else:
        angles = pillar_angles + [hatch_deg, -hatch_deg]
        b = [bearing(x, z) for x, z in L["plan"]]
        span = round(max(b) - min(b))
    return dict(id=L["id"], name=L["name"], text=L["text"], file=f"cockpit_blockouts/{L['id']}.glb", pose="seated",
                eye_m=list(eye), clear_deg=360 if is_dome else clear_deg(angles), span_deg=span, tris=tris,
                hatch=dict(node="HATCH", hinge_m=[round(v, 4) for v in hinge], pop_m=POP_M, swing_rad=SWING,
                           tracks=[dict(node="HATCH", pos="z", v=POP_M, t0=0.0, d=0.4),
                                   dict(node="HATCH", axis="x", v=SWING, t0=0.4, d=1.6)]),
                mfd=list(L["mfd"]))


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    materials()
    old = json.load(open(MANIFEST)) if os.path.exists(MANIFEST) else {}
    done = {d["id"]: d for d in old.get("layouts", [])}
    for L in LAYOUTS:
        if argv and L["id"] not in argv:
            continue
        done[L["id"]] = build(L)
        print("BUILT", L["id"], done[L["id"]]["tris"], "tris, clear", done[L["id"]]["clear_deg"], flush=True)
    built = {k: dict(clear_deg=clear_deg([bearing(x, z, v["eye"][2]) for x, z in v["plan"]]),
                     span_deg=round(max(bearing(x, z, v["eye"][2]) for x, z in v["plan"]) - min(bearing(x, z, v["eye"][2]) for x, z in v["plan"])))
             for k, v in BUILT.items()}
    with open(MANIFEST, "w") as f:
        json.dump(dict(generator="tools/blender/build_cockpit_blockouts.py", seated=True,
                       layouts=[done[L["id"]] for L in LAYOUTS if L["id"] in done], built=built), f, indent=1)


main()
