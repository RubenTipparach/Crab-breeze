"""Low-poly mech block-outs at the new size, built the CSG way, before any detail.

The owner, 2026-10-24: "2x mech size please, they dont seem very big right now, redesign mechs, start with
lowpoly first", "I really expect legs on these mechs to be bigger than the torso, just for balance", "I really
like how stevedore is all struts thats great", "the foreman looks good too", and for every model "use blender to
your advantage, lots of CSG, lots of boolean meshes to get the basic idea across".

Three chassis, each about 24 m tall (twice survey M1's 12 m), on digitigrade legs (survey M2) whose volume
outweighs the torso's (the manifest says by how much, measured off the meshes), with a seated cockpit prow on the
chest whose hatch is cut by a boolean groove (gasket orange, as on the cockpit block-outs):

  foreman    the excavator: chamfered armour boxes (box minus corner cutters), a slewing ring, double hydraulic
             rams down thighs and shins, track-pad feet, an engine pack and stack, a breaker and a boom cannon
  stevedore  the dock crane, all struts: lattice-girder legs and torso (longerons and zigzag braces), outrigger
             feet, gantry A-frames over the shoulders, a lattice crane jib and a cargo mortar
  ironhorse  the locomotive: a boiler torso (a cylinder minus its smokebox recess), drive-wheel knees with a
             coupling rod to the hock wheel, cow-catcher shins (a box cut to a wedge), a smokestack and dome,
             a steam cannon and a buffer ram

Flat colours, no textures. Writes assets/models/mech_blockouts/<id>.glb and assets/models/mech_blockouts.json.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_mech_blockouts.py
"""
import json
import math
import os

import bmesh
import bpy
from mathutils import Matrix, Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "mech_blockouts")
MANIFEST = os.path.join(ROOT, "assets", "models", "mech_blockouts.json")
G2B = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # glTF frame (y up, z ahead) to Blender
V = Vector
HEIGHT_M = 24.0

PALETTES = {
    "foreman": dict(BODY=(0.82, 0.80, 0.74), BODY2=(0.62, 0.60, 0.56), FRAME=(0.10, 0.10, 0.12), ACCENT=(0.95, 0.62, 0.10),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.70, 0.2)),
    "stevedore": dict(BODY=(0.12, 0.32, 0.72), BODY2=(0.20, 0.22, 0.26), FRAME=(0.08, 0.08, 0.10), ACCENT=(0.85, 0.16, 0.12),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(0.55, 0.90, 1.0)),
    "ironhorse": dict(BODY=(0.20, 0.17, 0.15), BODY2=(0.32, 0.27, 0.22), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.78, 0.58, 0.25),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.78, 0.38)),
}
MATS = {}
GROUPS = {}          # group name -> [objects]: legs, torso, arms, head, cockpit


def materials(pal):
    for o in list(bpy.data.materials):
        bpy.data.materials.remove(o)
    MATS.clear()
    for name, rgb in pal.items():
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        b = m.node_tree.nodes["Principled BSDF"]
        b.inputs["Base Color"].default_value = (*rgb, 1)
        b.inputs["Roughness"].default_value = 0.55
        b.inputs["Metallic"].default_value = 0.6 if name in ("METAL", "FRAME") else 0.2
        if name == "GLOW":
            b.inputs["Emission Color"].default_value = (*rgb, 1)
            b.inputs["Emission Strength"].default_value = 3.0
        MATS[name] = m
    g = bpy.data.materials.new("GASKET")
    g.use_nodes = True
    g.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.95, 0.42, 0.08, 1)
    MATS["GASKET"] = g


def obj(name, bm, mat, group):
    bmesh.ops.transform(bm, matrix=G2B, verts=bm.verts)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.materials.append(MATS[mat])
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    GROUPS.setdefault(group, []).append(ob)
    return ob


def frame_of(axes):
    axes = [V(a).normalized() for a in axes]
    if axes[0].cross(axes[1]).dot(axes[2]) < 0:      # right-handed, or a boolean with it does nothing
        axes[0] = -axes[0]
    return Matrix([tuple(a) for a in axes]).transposed().to_4x4()


def box(name, c, size, mat, group, axes=((1, 0, 0), (0, 1, 0), (0, 0, 1))):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(V(c)) @ frame_of(axes) @ Matrix.Diagonal((*size, 1)))
    return obj(name, bm, mat, group)


def between(a, b):
    """Axes for a member running from a to b: (side, along, other)."""
    d = (V(b) - V(a)).normalized()
    side = V((0, 1, 0)).cross(d)
    if side.length < 1e-4:
        side = V((1, 0, 0))
    side.normalize()
    return side, d, side.cross(d)


def strut(name, a, b, w, mat, group, d=None):
    a, b = V(a), V(b)
    s, u, n = between(a, b)
    return box(name, (a + b) / 2, (w, (b - a).length, d or w), mat, group, (s, u, n))


def cyl(name, a, b, r, mat, group, seg=16, r2=None):
    a, b = V(a), V(b)
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r2 is None else r2, depth=(b - a).length)
    q = V((0, 0, 1)).rotation_difference((b - a).normalized())
    bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation((a + b) / 2) @ q.to_matrix().to_4x4())
    return obj(name, bm, mat, group)


def loft(name, keys, mat, group):
    """A solid through plan sections [(y, [(x, z), ...])] with the same number of points each."""
    bm = bmesh.new()
    rings = [[bm.verts.new((x, y, z)) for x, z in poly] for y, poly in keys]
    n = len(rings[0])
    bm.faces.new(rings[0])
    bm.faces.new(rings[-1])
    for r0, r1 in zip(rings, rings[1:]):
        for i in range(n):
            bm.faces.new((r0[i], r0[(i + 1) % n], r1[(i + 1) % n], r1[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return obj(name, bm, mat, group)


def boolean(target, cutter, op):
    for m in cutter.data.materials:
        if m.name not in [s.name for s in target.data.materials]:
            target.data.materials.append(m)
    mod = target.modifiers.new("csg", "BOOLEAN")
    mod.operation, mod.solver, mod.object, mod.material_mode = op, "EXACT", cutter, "TRANSFER"
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    for g in GROUPS.values():
        if cutter in g:
            g.remove(cutter)
    bpy.data.objects.remove(cutter)
    return target


def chamfered(name, c, size, ch, mat, group, axes=((1, 0, 0), (0, 1, 0), (0, 0, 1)), top=True):
    """A box with its vertical edges (and its top edges) cut off by cutter boxes turned 45 degrees: CSG."""
    b = box(name, c, size, mat, group, axes)
    ax = [V(a).normalized() for a in axes]
    w, h, d = size
    k = 0
    for sx in (1, -1):
        for sz in (1, -1):
            p = V(c) + ax[0] * sx * w / 2 + ax[2] * sz * d / 2
            r = (ax[0] * sx + ax[2] * sz).normalized()
            boolean(b, box(f"{name}_c{k}", p, (ch * 1.414, h * 1.2, ch * 1.414), mat, group, (r, ax[1], r.cross(ax[1]))), "DIFFERENCE")
            k += 1
    if top:
        for s_, a_ in ((1, ax[0]), (-1, ax[0]), (1, ax[2]), (-1, ax[2])):
            span = d if a_ is ax[0] else w
            p = V(c) + ax[1] * h / 2 + a_ * s_ * (w / 2 if a_ is ax[0] else d / 2)
            r = (ax[1] + a_ * s_).normalized()
            other = ax[2] if a_ is ax[0] else ax[0]
            boolean(b, box(f"{name}_t{k}", p, (ch * 1.414, ch * 1.414, span * 1.2), mat, group, (r, r.cross(other), other)), "DIFFERENCE")
            k += 1
    return b


def lattice(name, a, b, half, mat, group, bays=4, w=0.22):
    """A lattice girder from a to b: four longerons at the corners of a square section 2*half across, and zigzag
    braces on each face. The Stevedore's struts."""
    a, b = V(a), V(b)
    s, u, n = between(a, b)
    corners = [s * sx * half + n * sz * half for sx, sz in ((1, 1), (-1, 1), (-1, -1), (1, -1))]
    for i, c in enumerate(corners):
        strut(f"{name}_l{i}", a + c, b + c, w, mat, group)
    L = (b - a).length
    for f in range(4):
        c0, c1 = corners[f], corners[(f + 1) % 4]
        for k in range(bays):
            p0 = a + u * (L * k / bays) + (c0 if k % 2 == 0 else c1)
            p1 = a + u * (L * (k + 1) / bays) + (c1 if k % 2 == 0 else c0)
            strut(f"{name}_b{f}{k}", p0, p1, w * 0.7, mat, group)


def cockpit_prow(name, c, w, h, d, group="cockpit"):
    """The seated cockpit's prow on the chest: a chamfered box with the hatch's outline cut as a groove."""
    p = chamfered(name, c, (w, h, d), 0.35, "BODY2", group)
    front = V(c) + V((0, 0, d / 2))
    for i, (pc, sz) in enumerate((((0, h * 0.42 - 0.05, 0), (w * 0.84, 0.08, 0.16)), ((0, -h * 0.42 + 0.05, 0), (w * 0.84, 0.08, 0.16)),
                                  ((w * 0.42, 0, 0), (0.08, h * 0.84, 0.16)), ((-w * 0.42, 0, 0), (0.08, h * 0.84, 0.16)))):
        boolean(p, box(f"{name}_g{i}", front + V(pc), sz, "GASKET", group), "DIFFERENCE")
    box(f"{name}_lamp", front + V((0, h * 0.30, 0.03)), (w * 0.5, 0.12, 0.08), "GLOW", group)
    return p


def mirror_x(fn):
    for s in (1, -1):
        fn(s)


# ----------------------------------------------------------------------------- the chassis
# Digitigrade legs, side view (z ahead): hip, knee forward and down, hock back and down, the foot on the ground.
def leg_points(hip_y, scale=1.0):
    return dict(hip=V((0, hip_y, 0)), knee=V((0, hip_y * 0.66, 2.8 * scale)), hock=V((0, hip_y * 0.30, -1.9 * scale)),
                ball=V((0, 1.0, 0.8 * scale)))


def foreman():
    hip_y, hx = 13.0, 3.4
    for s in (1, -1):
        P = {k: v + V((s * hx, 0, 0)) for k, v in leg_points(hip_y).items()}
        chamfered(f"thigh{s}", (P["hip"] + P["knee"]) / 2, (3.8, (P["knee"] - P["hip"]).length + 0.6, 3.4), 0.5, "BODY", "legs",
                  between(P["hip"], P["knee"]))
        chamfered(f"shin{s}", (P["knee"] + P["hock"]) / 2, (3.2, (P["hock"] - P["knee"]).length + 0.4, 2.9), 0.45, "BODY", "legs",
                  between(P["knee"], P["hock"]))
        chamfered(f"meta{s}", (P["hock"] + P["ball"]) / 2, (2.6, (P["ball"] - P["hock"]).length + 0.3, 2.4), 0.4, "BODY2", "legs",
                  between(P["hock"], P["ball"]))
        cyl(f"knee{s}", P["knee"] - V((s * 2.1, 0, 0)), P["knee"] + V((s * 2.1, 0, 0)), 1.5, "FRAME", "legs")
        cyl(f"hock{s}", P["hock"] - V((s * 1.7, 0, 0)), P["hock"] + V((s * 1.7, 0, 0)), 1.2, "FRAME", "legs")
        for k, (a, b) in enumerate(((P["hip"], P["knee"]), (P["knee"], P["hock"]))):       # the double rams, in front
            off = V((0, 0, 1.9 if k == 0 else -1.7))
            for dx in (-0.8, 0.8):
                cyl(f"ram{s}{k}{dx}", a + off * 0.6 + V((dx, 0, 0)), a.lerp(b, 0.62) + off + V((dx, 0, 0)), 0.42, "FRAME", "legs", 10)
                cyl(f"rod{s}{k}{dx}", a.lerp(b, 0.62) + off + V((dx, 0, 0)), b + off * 0.6 + V((dx, 0, 0)), 0.22, "METAL", "legs", 8)
        # the track-pad foot: a chamfered sole and rubber ribs
        chamfered(f"foot{s}", P["ball"] + V((0, -0.35, 0.6)), (4.8, 1.3, 7.2), 0.4, "FRAME", "legs")
        for i in range(8):
            box(f"rib{s}{i}", P["ball"] + V((0, -0.9, -2.6 + i * 0.75)), (4.9, 0.25, 0.35), "RUBBER", "legs")
    box("hips", (0, hip_y + 0.4, 0), (2 * hx + 1.0, 2.6, 3.6), "FRAME", "legs")
    cyl("slew", (0, hip_y + 1.7, 0), (0, hip_y + 2.6, 0), 3.4, "FRAME", "torso", 24)
    # the cab torso: compact, chamfered, the cockpit prow out front
    chamfered("torso", (0, hip_y + 5.3, -0.4), (6.8, 5.4, 5.6), 0.8, "BODY", "torso")
    box("hazard", (0, hip_y + 2.75, 2.42), (5.6, 0.35, 0.2), "ACCENT", "torso")
    cockpit_prow("prow", (0, hip_y + 5.4, 3.0), 3.2, 3.0, 1.6)
    chamfered("head", (0, hip_y + 8.5, 0.6), (2.6, 1.2, 2.0), 0.3, "BODY2", "head")
    for x in (-0.6, 0.6):
        cyl(f"lens{x}", (x, hip_y + 8.5, 1.55), (x, hip_y + 8.5, 1.7), 0.32, "GLOW", "head", 12)
    chamfered("engine", (0, hip_y + 5.0, -4.3), (5.2, 4.6, 2.4), 0.5, "BODY2", "torso")
    cyl("stack", (1.6, hip_y + 7.0, -4.3), (1.6, 23.6, -4.3), 0.45, "FRAME", "torso", 12)
    for s in (1, -1):
        box(f"beacon{s}", (s * 2.8, hip_y + 8.1, 1.4), (0.5, 0.4, 0.5), "ACCENT", "torso")
        # arms: a shoulder block, a boom, rams, and the tool
        sh = V((s * 4.6, hip_y + 6.6, 0.2))
        chamfered(f"shoulder{s}", sh, (2.2, 2.4, 3.0), 0.4, "BODY", "arms")
        el = sh + V((s * 0.6, -3.6, 2.0))
        strut(f"upper{s}", sh, el, 1.3, "BODY2", "arms")
        cyl(f"armram{s}", sh + V((0, 0.4, 1.2)), el + V((0, 0.6, 0.6)), 0.3, "FRAME", "arms", 8)
        hand = el + V((0, -0.8, 3.6))
        if s < 0:
            cyl("breaker", el, hand, 0.75, "BODY", "arms", 12)
            cyl("chisel", hand, hand + V((0, -0.3, 2.0)), 0.3, "METAL", "arms", 8, r2=0.05)
        else:
            strut("receiver", el, el + V((0, -0.2, 1.4)), 1.1, "FRAME", "arms")
            cyl("cannon", el + V((0, -0.1, 1.2)), hand + V((0, 0, 1.4)), 0.32, "METAL", "arms", 10)


def stevedore():
    hip_y, hx = 14.0, 3.0
    for s in (1, -1):
        P = {k: v + V((s * hx, 0, 0)) for k, v in leg_points(hip_y, 1.05).items()}
        lattice(f"thigh{s}", P["hip"], P["knee"], 1.6, "BODY", "legs", bays=4, w=0.34)
        lattice(f"shin{s}", P["knee"], P["hock"], 1.4, "BODY", "legs", bays=4, w=0.3)
        lattice(f"meta{s}", P["hock"], P["ball"], 1.2, "BODY", "legs", bays=3, w=0.28)
        for j in ("knee", "hock"):
            cyl(f"{j}{s}", P[j] - V((s * 2.0, 0, 0)), P[j] + V((s * 2.0, 0, 0)), 0.9, "FRAME", "legs", 12)
        # outrigger foot: four struts out to pads
        hub = P["ball"]
        box(f"hub{s}", hub, (1.6, 1.0, 1.6), "FRAME", "legs")
        for k, (dx, dz) in enumerate(((2.6, 2.8), (-2.6, 2.8), (2.6, -2.4), (-2.6, -2.4))):
            pad = hub + V((dx, -0.8, dz))
            strut(f"out{s}{k}", hub, pad + V((0, 0.4, 0)), 0.45, "ACCENT", "legs")
            cyl(f"pad{s}{k}", pad + V((0, -0.2, 0)), pad + V((0, 0.2, 0)), 0.9, "FRAME", "legs", 10)
    box("hips", (0, hip_y + 0.5, 0), (2 * hx + 1.4, 1.6, 2.4), "FRAME", "legs")
    # the torso: a lattice cage, the cockpit container in its front
    y0, y1, hw, z0, z1 = hip_y + 1.6, hip_y + 7.6, 3.2, -2.6, 2.0
    C = [V((sx * hw, y, z)) for y in (y0, y1) for sx in (1, -1) for z in (z0, z1)]
    edges = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)]
    for i, (a, b) in enumerate(edges):
        strut(f"cage{i}", C[a], C[b], 0.45, "BODY", "torso")
    for i, (a, b, c_, d) in enumerate(((0, 5, 1, 4), (2, 7, 3, 6), (0, 6, 2, 4), (1, 7, 3, 5))):   # X braces on the faces
        strut(f"x{i}a", C[a], C[b], 0.28, "BODY", "torso")
        strut(f"x{i}b", C[c_], C[d], 0.28, "BODY", "torso")
    box("core", (0, (y0 + y1) / 2, z0 + 1.6), (4.0, 4.6, 2.6), "BODY2", "torso")
    cockpit_prow("prow", (0, (y0 + y1) / 2 + 0.3, z1 + 0.2), 3.0, 3.2, 2.0)
    for s in (1, -1):                                   # gantry A-frames over the shoulders
        top = V((s * (hw - 0.6), y1 + 3.4, (z0 + z1) / 2))
        strut(f"gant{s}a", V((s * (hw - 0.6), y1, z0)), top, 0.35, "ACCENT", "torso")
        strut(f"gant{s}b", V((s * (hw - 0.6), y1, z1)), top, 0.35, "ACCENT", "torso")
    strut("gantbar", V((hw - 0.6, y1 + 3.4, -0.3)), V((-(hw - 0.6), y1 + 3.4, -0.3)), 0.4, "FRAME", "torso")
    box("nest", (0, y1 + 3.9, 0.2), (1.8, 1.0, 1.4), "BODY2", "head")
    cyl("eye", (0, y1 + 3.9, 0.9), (0, y1 + 3.9, 1.05), 0.3, "GLOW", "head", 12)
    for s in (1, -1):
        sh = V((s * (hw + 1.0), y1 - 1.0, -0.2))
        box(f"shoulder{s}", sh, (1.4, 1.6, 1.6), "FRAME", "arms")
        el = sh + V((s * 0.4, -3.4, 1.6))
        lattice(f"upper{s}", sh, el, 0.55, "BODY", "arms", bays=3, w=0.16)
        if s < 0:                                       # the crane jib: a long lattice, a cable and a grapple
            tip = el + V((0, 1.0, 7.0))
            lattice("jib", el, tip, 0.5, "ACCENT", "arms", bays=6, w=0.15)
            cyl("cable", tip, tip + V((0, -3.0, 0)), 0.06, "METAL", "arms", 6)
            box("grapple", tip + V((0, -3.4, 0)), (0.9, 0.8, 0.9), "FRAME", "arms")
        else:                                           # the cargo mortar
            box("mortar", el + V((0, -0.2, 1.0)), (1.6, 1.6, 2.2), "BODY2", "arms")
            for dx in (-0.4, 0.4):
                cyl(f"tube{dx}", el + V((dx, 0.3, 1.0)), el + V((dx, 1.4, 2.4)), 0.3, "FRAME", "arms", 10)


def ironhorse():
    hip_y, hx = 12.6, 3.2
    for s in (1, -1):
        P = {k: v + V((s * hx, 0, 0)) for k, v in leg_points(hip_y, 1.1).items()}
        chamfered(f"thigh{s}", (P["hip"] + P["knee"]) / 2, (3.6, (P["knee"] - P["hip"]).length + 0.5, 3.6), 0.6, "BODY", "legs",
                  between(P["hip"], P["knee"]))
        # the shin with its cow-catcher: a box cut to a wedge by a cutter turned across its front
        sh = chamfered(f"shin{s}", (P["knee"] + P["hock"]) / 2, (3.2, (P["hock"] - P["knee"]).length + 0.3, 3.0), 0.5, "BODY", "legs",
                       between(P["knee"], P["hock"]))
        cc = box(f"catcher{s}", (P["knee"] + P["hock"]) / 2 + V((0, -1.2, 1.4)), (3.4, 3.4, 1.6), "ACCENT", "legs",
                 ((1, 0, 0), (0, 1, 1), (0, -1, 1)))
        boolean(sh, cc, "UNION")
        chamfered(f"meta{s}", (P["hock"] + P["ball"]) / 2, (2.6, (P["ball"] - P["hock"]).length + 0.3, 2.6), 0.4, "BODY2", "legs",
                  between(P["hock"], P["ball"]))
        out = V((s * 2.2, 0, 0))
        cyl(f"drive{s}", P["knee"] + out * 0.7, P["knee"] + out * 1.2, 2.3, "ACCENT", "legs", 20)       # drive wheel at the knee
        cyl(f"hockw{s}", P["hock"] + out * 0.6, P["hock"] + out * 1.05, 1.6, "ACCENT", "legs", 16)
        strut(f"coupling{s}", P["knee"] + out * 1.3 + V((0, -1.4, 0)), P["hock"] + out * 1.15 + V((0, 0.9, 0)), 0.45, "METAL", "legs")
        chamfered(f"foot{s}", P["ball"] + V((0, -0.4, 0.5)), (3.8, 1.2, 6.0), 0.4, "FRAME", "legs")
    box("hips", (0, hip_y + 0.4, 0), (2 * hx + 1.2, 2.4, 3.2), "FRAME", "legs")
    # the boiler torso: a cylinder along z, its smokebox recess cut where the cockpit prow sits
    cz, cy, R = 0.0, hip_y + 4.6, 3.0
    boiler = cyl("boiler", (0, cy, cz - 3.4), (0, cy, cz + 2.8), R, "BODY", "torso", 20)
    boolean(boiler, box("smokebox_cut", (0, cy + 0.3, cz + 2.8), (3.6, 3.4, 1.4), "BODY2", "torso"), "DIFFERENCE")
    for z in (-2.6, -0.6, 1.4):
        cyl(f"band{z}", (0, cy, z - 0.15), (0, cy, z + 0.15), R + 0.12, "ACCENT", "torso", 20)
    cockpit_prow("prow", (0, cy + 0.3, cz + 2.75), 3.0, 2.8, 1.3)
    cyl("stack", (0, cy + R - 0.3, 1.6), (0, cy + R + 3.6, 1.6), 0.8, "FRAME", "torso", 14, r2=1.1)
    cyl("dome", (0, cy + R - 0.2, -0.8), (0, cy + R + 1.0, -0.8), 1.1, "BODY2", "torso", 14, r2=0.7)
    box("cab_head", (0, cy + R + 0.6, -2.4), (2.2, 1.2, 1.6), "BODY2", "head")
    cyl("headlamp", (0, cy + R + 0.6, -1.6), (0, cy + R + 0.6, -1.45), 0.4, "GLOW", "head", 12)
    cyl("firebox", (0, cy - 0.4, cz - 3.4), (0, cy - 0.4, cz - 5.6), 2.2, "BODY2", "torso", 16)
    for s in (1, -1):
        sh = V((s * (R + 1.2), cy + 1.0, -0.2))
        chamfered(f"shoulder{s}", sh, (2.0, 2.2, 2.6), 0.4, "BODY", "arms")
        el = sh + V((s * 0.4, -3.2, 1.8))
        strut(f"upper{s}", sh, el, 1.2, "BODY2", "arms")
        hand = el + V((0, -0.4, 4.2))
        if s < 0:                                       # the steam cannon: banded barrel
            cyl("cannon", el, hand, 0.55, "FRAME", "arms", 12)
            for i in range(4):
                p = el.lerp(hand, 0.2 + 0.2 * i)
                cyl(f"cband{i}", p - V((0, 0, 0.12)), p + V((0, 0, 0.12)), 0.7, "ACCENT", "arms", 12)
        else:                                           # the buffer ram
            cyl("piston", el, hand, 0.35, "METAL", "arms", 10)
            cyl("buffer", hand, hand + V((0, 0, 0.5)), 1.2, "BODY", "arms", 16)


CHASSIS = {"foreman": (foreman, "Foreman", "the excavator"), "stevedore": (stevedore, "Stevedore", "the dock crane, all struts"),
           "ironhorse": (ironhorse, "Ironhorse", "the locomotive")}


def volume(objs):
    v = 0.0
    for o in objs:
        bm = bmesh.new()
        bm.from_mesh(o.data)
        v += abs(bm.calc_volume(signed=False))
        bm.free()
    return v


def main():
    os.makedirs(OUT, exist_ok=True)
    rows = []
    for key, (fn, name, what) in CHASSIS.items():
        for o in list(bpy.data.objects):
            bpy.data.objects.remove(o)
        GROUPS.clear()
        materials(PALETTES[key])
        fn()
        bpy.context.view_layer.update()
        pts = [o.matrix_world @ v.co for o in bpy.data.objects if o.type == "MESH" for v in o.data.vertices]
        lo, hi = [min(p[i] for p in pts) for i in range(3)], [max(p[i] for p in pts) for i in range(3)]
        tris = 0
        for o in bpy.data.objects:
            bm = bmesh.new()
            bm.from_mesh(o.data)
            bmesh.ops.triangulate(bm, faces=bm.faces)
            bm.to_mesh(o.data)
            tris += len(o.data.polygons)
            bm.free()
        legs, torso = volume(GROUPS.get("legs", [])), volume(GROUPS.get("torso", []) + GROUPS.get("cockpit", []))
        root = bpy.data.objects.new(key, None)
        bpy.context.scene.collection.objects.link(root)
        for o in bpy.data.objects:
            if o.type == "MESH":
                o.parent = root
        path = os.path.join(OUT, f"{key}.glb")
        bpy.ops.object.select_all(action="SELECT")
        bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=True, export_yup=True)
        row = dict(id=key, name=name, what=what, file=f"mech_blockouts/{key}.glb", height_m=round(hi[2] - lo[2], 2),
                   width_m=round(hi[0] - lo[0], 2), length_m=round(hi[1] - lo[1], 2), tris=tris,
                   legs_m3=round(legs, 1), torso_m3=round(torso, 1), legs_over_torso=round(legs / torso, 2) if torso else None)
        rows.append(row)
        print("BUILT", row, flush=True)
    with open(MANIFEST, "w") as f:
        json.dump(dict(generator="tools/blender/build_mech_blockouts.py", target_height_m=HEIGHT_M, mechs=rows), f, indent=1)


main()
