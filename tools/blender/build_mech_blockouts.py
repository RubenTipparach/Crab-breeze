"""Low-poly mech block-outs at the new size, built the CSG way, before any detail.

The owner, 2026-10-24: "2x mech size please, they dont seem very big right now, redesign mechs, start with
lowpoly first", "I really expect legs on these mechs to be bigger than the torso, just for balance", "I really
like how stevedore is all struts thats great", "the foreman looks good too", and for every model "use blender to
your advantage, lots of CSG, lots of boolean meshes to get the basic idea across".

Thirteen chassis, each about 24 m tall (twice survey M1's 12 m), on digitigrade legs (survey M2) whose volume
outweighs the torso's (the manifest says by how much, measured off the meshes), with a seated cockpit prow on the
chest whose hatch is cut by a boolean groove (gasket orange, as on the cockpit block-outs):

  foreman    the excavator: chamfered armour boxes (box minus corner cutters), a slewing ring, double hydraulic
             rams down thighs and shins, track-pad feet, an engine pack and stack, a breaker and a boom cannon
  stevedore  the dock crane, all struts: lattice-girder legs and torso (longerons and zigzag braces), outrigger
             feet, gantry A-frames over the shoulders, a lattice crane jib and a cargo mortar
  ironhorse  the locomotive: a boiler torso (a cylinder minus its smokebox recess), drive-wheel knees with a
             coupling rod to the hock wheel, cow-catcher shins (a box cut to a wedge), a smokestack and dome,
             a steam cannon and a buffer ram
  harrow     new, the Mad Cat pattern (docs/design/mech-references.md): one hull slung between the hips, its
             nose cut to a slope with the Canopy cockpit set in it, box missile racks on the shoulders, gun-pod
             arms, wheel feet, a camera mast for a head; olive drab
  roustabout new, the Scopedog pattern: a boxy upright chest with the Titan cockpit in its middle, the upper
             chest and its camera turret the part that lifts, a backpack with vents, a box-magazine rifle and a
             pipe grab, wheel feet; gunmetal and orange
  grader, logger, lineman, driller, bowser, mule, pumper, ploughman: eight more after working machines (a road
             dozer, a forestry harvester, a bucket truck, a drill rig, a fuel tanker, a haul truck, a fire engine,
             a tractor), each with a different cockpit block-out, so all ten cockpits have a chassis

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
    "harrow": dict(BODY=(0.30, 0.33, 0.20), BODY2=(0.24, 0.25, 0.26), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.88, 0.70, 0.12),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.52, 0.53, 0.55), GLOW=(0.55, 1.0, 0.6)),
    "roustabout": dict(BODY=(0.36, 0.38, 0.40), BODY2=(0.22, 0.23, 0.24), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.92, 0.42, 0.10),
                       RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.75, 0.3)),
    "grader": dict(BODY=(0.86, 0.64, 0.10), BODY2=(0.20, 0.20, 0.21), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.08, 0.08, 0.08),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.65, 0.2)),
    "logger": dict(BODY=(0.17, 0.30, 0.20), BODY2=(0.26, 0.24, 0.20), FRAME=(0.07, 0.08, 0.07), ACCENT=(0.92, 0.48, 0.10),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.8, 0.4)),
    "lineman": dict(BODY=(0.30, 0.40, 0.52), BODY2=(0.22, 0.24, 0.27), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.95, 0.55, 0.10),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.85, 0.4)),
    "driller": dict(BODY=(0.48, 0.20, 0.12), BODY2=(0.30, 0.29, 0.28), FRAME=(0.08, 0.07, 0.07), ACCENT=(0.90, 0.78, 0.15),
                    RUBBER=(0.06, 0.06, 0.06), METAL=(0.58, 0.58, 0.60), GLOW=(0.5, 0.9, 1.0)),
    "bowser": dict(BODY=(0.62, 0.14, 0.12), BODY2=(0.58, 0.59, 0.60), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.95, 0.80, 0.15),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.62, 0.63, 0.66), GLOW=(1.0, 0.75, 0.3)),
    "mule": dict(BODY=(0.90, 0.72, 0.16), BODY2=(0.28, 0.27, 0.25), FRAME=(0.08, 0.08, 0.08), ACCENT=(0.80, 0.16, 0.10),
                 RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.7, 0.3)),
    "pumper": dict(BODY=(0.70, 0.10, 0.08), BODY2=(0.24, 0.24, 0.26), FRAME=(0.08, 0.08, 0.09), ACCENT=(0.92, 0.88, 0.70),
                   RUBBER=(0.06, 0.06, 0.06), METAL=(0.62, 0.63, 0.66), GLOW=(0.4, 0.6, 1.0)),
    "ploughman": dict(BODY=(0.16, 0.30, 0.55), BODY2=(0.24, 0.24, 0.25), FRAME=(0.07, 0.07, 0.08), ACCENT=(0.90, 0.75, 0.20),
                      RUBBER=(0.06, 0.06, 0.06), METAL=(0.55, 0.56, 0.60), GLOW=(1.0, 0.85, 0.5)),
    # Gundams keep their colours (white, blue, red, yellow): they are what makes one read as a Gundam. Weathered, not show
    # paint: an off-white with grime in it, as the Ground Type wears in the jungle.
    "picket": dict(BODY=(0.74, 0.74, 0.70), BODY2=(0.16, 0.26, 0.56), FRAME=(0.16, 0.16, 0.18), ACCENT=(0.72, 0.12, 0.10),
                   YELLOW=(0.92, 0.70, 0.10), RUBBER=(0.06, 0.06, 0.06), METAL=(0.45, 0.46, 0.48), GLOW=(0.3, 1.0, 0.45)),
    "sapper": dict(BODY=(0.72, 0.72, 0.68), BODY2=(0.20, 0.27, 0.42), FRAME=(0.15, 0.15, 0.16), ACCENT=(0.68, 0.14, 0.10),
                   YELLOW=(0.90, 0.68, 0.12), ARMOR=(0.30, 0.34, 0.30), RUBBER=(0.06, 0.06, 0.06), METAL=(0.45, 0.46, 0.48),
                   GLOW=(0.3, 1.0, 0.45)),
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


# ----------------------------------------------------------------------------- the new two, from the reference study
# docs/design/mech-references.md: chicken legs heavier than the body, the cockpit in the middle of the chest or in the
# nose of one hull slung between the hips, practical slab armour in olive drab and gunmetal, weapons that look like
# equipment, wheels in the feet, a sensor for a head. Each carries its cockpit block-out (tools/blender/
# build_cockpit_blockouts.py) at true size where the pilot sits, so the integration is measured, not drawn.

def prism_x(name, profile, hw, mat, group):
    """A solid from a side profile [(z, y), ...] (counter-clockwise seen from +x) extruded from -hw to hw across."""
    bm = bmesh.new()
    L = [bm.verts.new((-hw, y, z)) for z, y in profile]
    R = [bm.verts.new((hw, y, z)) for z, y in profile]
    n = len(profile)
    bm.faces.new(L)
    bm.faces.new(list(reversed(R)))
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((L[i], L[j], R[j], R[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return obj(name, bm, mat, group)


def pod_in(layout, at):
    """The cockpit block-out `layout` placed with its floor centre at `at` (metres, glTF frame), facing ahead."""
    path = os.path.join(ROOT, "assets", "models", "cockpit_blockouts", f"{layout}.glb")
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    for o in set(bpy.data.objects) - before:
        if o.parent is None:
            o.location = o.location + G2B.to_3x3() @ V(at)
            STATE["pods"].append(o)
        if o.type == "MESH":
            GROUPS.setdefault("pod", []).append(o)


def wheel_foot(name, ball, s, w=4.2, l=6.6, r=1.15):
    """A foot with its skate wheel (survey M4) showing: a sole, a toe wedge, and the powered wheel in a housing
    at the heel, folded up so it clears the ground until the pilot drops it."""
    chamfered(f"{name}_sole", ball + V((0, -0.5, 0.7)), (w, 1.2, l), 0.4, "FRAME", "legs")
    box(f"{name}_toe", ball + V((0, -0.35, 0.7 + l / 2 + 0.3)), (w * 0.8, 0.9, 1.4), "BODY2", "legs",
        ((1, 0, 0), (0, 1, 0.5), (0, -0.5, 1)))
    hub = ball + V((0, 0.0, 0.7 - l / 2 + 0.4))
    box(f"{name}_housing", hub + V((0, 0.35, 0)), (w * 0.62, 1.5, 2.6), "BODY2", "legs")
    cyl(f"{name}_wheel", hub - V((w * 0.28, 0, 0)), hub + V((w * 0.28, 0, 0)), r, "RUBBER", "legs", 16)
    cyl(f"{name}_hub", hub - V((w * 0.31, 0, 0)), hub + V((w * 0.31, 0, 0)), r * 0.45, "ACCENT", "legs", 10)


# Each mech's own proportions (owner, 2026-10-24: "All 10 mechs have roughly the same ratios. Id like more variation
# in arm size torso size and leg shape"). legs: the knee's height (share of the hip's) and how far forward it is, the
# hock's height and how far back, how thick the leg is, and what the thigh and shin are made of (box slabs, round
# pistons, a lattice). torso: the upper body scaled about the top of the waist (the cockpit moves with it but keeps its
# true size). arms: each arm scaled about its shoulder.
VARIETY = {
    "harrow": dict(torso=1.1, arms=1.0, legs=dict(kh=0.62, kf=3.6, hh=0.34, hb=2.6, thick=1.2)),
    "roustabout": dict(torso=1.08, arms=1.4, legs=dict(kh=0.72, kf=2.0, hh=0.30, hb=1.3, thick=1.35, shin="cyl")),
    "grader": dict(torso=0.85, arms=1.0, legs=dict(kh=0.64, kf=2.4, hh=0.26, hb=1.6, thick=1.45)),
    "logger": dict(torso=0.9, arms=1.1, legs=dict(kh=0.58, kf=4.4, hh=0.42, hb=3.4, thick=0.7, shin="cyl")),
    "lineman": dict(torso=0.85, arms=0.7, legs=dict(kh=0.68, kf=2.6, hh=0.36, hb=2.0, thick=0.72, thigh="cyl", shin="cyl")),
    "driller": dict(torso=1.3, arms=1.5, legs=dict(kh=0.66, kf=2.8, hh=0.30, hb=1.9, thick=1.0, shin="lattice")),
    "bowser": dict(torso=1.1, arms=1.0, legs=dict(kh=0.74, kf=2.0, hh=0.24, hb=1.3, thick=1.3)),
    "mule": dict(torso=1.0, arms=1.3, legs=dict(kh=0.60, kf=3.0, hh=0.32, hb=2.2, thick=1.5)),
    "pumper": dict(torso=0.9, arms=1.3, legs=dict(kh=0.64, kf=3.4, hh=0.38, hb=2.6, thick=0.82, shin="cyl")),
    "ploughman": dict(torso=1.05, arms=0.65, legs=dict(kh=0.72, kf=2.6, hh=0.22, hb=1.4, thick=1.1, thigh="cyl")),
}
STATE = dict(key=None, anchor=None, pods=[])

LEG_SLIM = (0.72, 0.8)    # owner, 2026-10-24: "legs a little less thick" (thigh and shin width, depth)
WAIST_LIFT_M = 2.0        # owner, 2026-10-24: "arms clip through the leg. Perhaps torso a little higher"


def waist(hip_y, hx):
    """The torso stood WAIST_LIFT_M higher on a waist block over the hips, so the arms hang clear of the thighs.
    Returns the height the torso is built from."""
    box("waist", (0, hip_y + 1.2 + WAIST_LIFT_M / 2, -0.2), (max(3.0, hx * 1.1), WAIST_LIFT_M + 0.6, 2.6), "FRAME", "legs")
    STATE["anchor"] = V((0, hip_y + 1.5 + WAIST_LIFT_M, -0.2))
    return hip_y + WAIST_LIFT_M


def chicken_legs(hip_y, hx, scale, thigh, shin, body_mat="BODY"):
    """Reverse-jointed legs with wheel feet, shaped by the mech's VARIETY entry: the knee and hock placed by it, the
    thigh and shin as chamfered slabs, round pistons (a sleeve with a rod out of it) or an open lattice."""
    L = VARIETY.get(STATE["key"], {}).get("legs", {})
    t = L.get("thick", 1.0)
    thigh = (thigh[0] * LEG_SLIM[0] * t, thigh[1] * LEG_SLIM[1] * t)
    shin = (shin[0] * LEG_SLIM[0] * t, shin[1] * LEG_SLIM[1] * t)
    pts = dict(hip=V((0, hip_y, 0)), knee=V((0, hip_y * L.get("kh", 0.66), L.get("kf", 2.8) * scale)),
               hock=V((0, hip_y * L.get("hh", 0.30), -L.get("hb", 1.9) * scale)), ball=V((0, 1.0, 0.8 * scale)))

    def member(name, a, b, size, kind, mat):
        if kind == "cyl":
            r = min(size) / 2
            cyl(name, a, a.lerp(b, 0.62), r, mat, "legs", 14)
            cyl(name + "_rod", a.lerp(b, 0.5), b, r * 0.55, "METAL", "legs", 10)
        elif kind == "lattice":
            lattice(name, a, b, min(size) / 2, mat, "legs", bays=4, w=0.3)
        else:
            chamfered(name, (a + b) / 2, (size[0], (b - a).length + 0.5, size[1]), 0.5, mat, "legs", between(a, b))

    for s in (1, -1):
        P = {k: v + V((s * hx, 0, 0)) for k, v in pts.items()}
        member(f"thigh{s}", P["hip"], P["knee"], thigh, L.get("thigh", "box"), body_mat)
        member(f"shin{s}", P["knee"], P["hock"], shin, L.get("shin", "box"), body_mat)
        chamfered(f"meta{s}", (P["hock"] + P["ball"]) / 2, (shin[0] * 0.8, (P["ball"] - P["hock"]).length + 0.3, shin[1] * 0.8),
                  0.4, "BODY2", "legs", between(P["hock"], P["ball"]))
        rk = 1.5 * t ** 0.5
        kw, hw_ = thigh[0] * 0.5 + 0.3, shin[0] * 0.5 + 0.3          # the drums stand just past the leg's own sides
        cyl(f"knee{s}", P["knee"] - V((kw, 0, 0)), P["knee"] + V((kw, 0, 0)), rk, "FRAME", "legs")
        cyl(f"hock{s}", P["hock"] - V((hw_, 0, 0)), P["hock"] + V((hw_, 0, 0)), rk * 0.77, "FRAME", "legs")
        fr = thigh[1] / 2 + 0.35
        cyl(f"ram{s}", P["hip"] + V((0, -0.6, fr)), P["hip"].lerp(P["knee"], 0.6) + V((0, 0, fr + 0.1)), 0.45, "FRAME", "legs", 10)
        cyl(f"rod{s}", P["hip"].lerp(P["knee"], 0.6) + V((0, 0, fr + 0.1)), P["knee"] + V((0, 0.6, fr - 0.3)), 0.24, "METAL", "legs", 8)
        if L.get("shin", "box") == "box":
            cyl(f"sram{s}", P["knee"] + V((0, -0.4, -shin[1] / 2 - 0.3)), P["knee"].lerp(P["hock"], 0.6) + V((0, 0, -shin[1] / 2 - 0.4)),
                0.4, "FRAME", "legs", 10)
        if L.get("thigh", "box") == "box":
            box(f"stencil{s}", (P["hip"] + P["knee"]) / 2 + V((s * (thigh[0] / 2 + 0.03), 0, 0)), (0.06, 1.6, 2.2), "ACCENT", "legs",
                between(P["hip"], P["knee"]))
        wheel_foot(f"foot{s}", P["ball"], s)


def _scale_about(objs, pivot, k):
    """Scale meshes (built in world space, identity matrices) about a point in glTF space."""
    pb = G2B @ V(pivot) if len(pivot) == 3 and not hasattr(pivot, "blender") else pivot
    M = Matrix.Translation(pb) @ Matrix.Scale(k, 4) @ Matrix.Translation(-pb)
    for o in objs:
        o.data.transform(M)


def arm_leg_clips():
    """Measured, not judged: every pair of an arm piece and a leg piece whose surfaces intersect (BVH overlap)."""
    from mathutils.bvhtree import BVHTree
    trees = {}

    def tree(o):
        if o.name not in trees:
            bm = bmesh.new()
            bm.from_mesh(o.data)
            bm.transform(o.matrix_world)
            trees[o.name] = BVHTree.FromBMesh(bm)
            bm.free()
        return trees[o.name]
    legs = [o for o in GROUPS.get("legs", []) if not o.name.startswith(("waist", "hips"))]
    out = []
    for a in GROUPS.get("arms", []) + GROUPS.get("frame", []):     # arms, and tool frames hung off the hips
        for g in legs:
            if tree(a).overlap(tree(g)):
                out.append(f"{a.name}/{g.name}")
    return out


def apply_variety(key):
    """The mech's torso and arm sizes (VARIETY), after its builder has run."""
    v = VARIETY.get(key)
    if not v or STATE["anchor"] is None:
        return
    a = STATE["anchor"]
    up = [o for g in ("torso", "head", "arms") for o in GROUPS.get(g, [])]
    _scale_about(up, a, v["torso"])
    ab = G2B @ a
    for o in STATE["pods"]:                     # the cockpit rides the torso and keeps its true size
        o.location = ab + (o.location - ab) * v["torso"]
    for side in (1, -1):
        arms = [o for o in GROUPS.get("arms", []) if side * (sum((vv.co.x for vv in o.data.vertices), 0.0) / max(1, len(o.data.vertices))) > 0.3]
        if not arms:
            continue
        vs = [vv.co for o in arms for vv in o.data.vertices]
        pb = V((side * min(abs(c.x) for c in vs), sum(c.y for c in vs) / len(vs), max(c.z for c in vs)))
        M = Matrix.Translation(pb) @ Matrix.Scale(v["arms"], 4) @ Matrix.Translation(-pb)
        for o in arms:
            o.data.transform(M)


def fist(name, el, k=1.0, ahead=2.4):
    """A punching arm's business end (owner, 2026-10-24: "larger arms on some for punching"): a thick forearm with a
    piston down it, from the elbow el forward, ending in a fist block with a knuckle plate and a hazard band."""
    wr = el + V((0, -0.2 * k, ahead * k))
    strut(f"{name}_fore", el, wr, 1.5 * k, "BODY2", "arms")
    cyl(f"{name}_piston", el + V((0, 0.9 * k, 0.2)), wr + V((0, 0.9 * k, -0.4)), 0.3 * k, "METAL", "arms", 8)
    box(f"{name}", wr + V((0, 0, 1.2 * k)), (2.3 * k, 2.2 * k, 2.4 * k), "BODY", "arms")
    box(f"{name}_knuckles", wr + V((0, 0, 2.5 * k)), (2.4 * k, 1.5 * k, 0.4 * k), "METAL", "arms")
    box(f"{name}_band", wr + V((0, 0, 0.1)), (2.4 * k, 2.3 * k, 0.3 * k), "ACCENT", "arms")


def harrow():
    """The Mad Cat pattern: one hull slung between the hips, its nose sloped, the Canopy cockpit set in the nose so
    its bubble rises out of the slope; box missile racks on the shoulders, gun pods for arms, a camera mast."""
    hip_y, hx = 15.0, 3.9
    chicken_legs(hip_y, hx, 1.12, (4.6, 4.2), (3.6, 3.3))
    box("hips", (0, hip_y + 0.2, -0.4), (2 * hx + 1.2, 2.4, 3.8), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    # the hull: its side profile extruded across, the nose sloping down to a chin; the nose's corners cut away (CSG)
    yc, zc = hip_y + 3.2, 0.6
    hull = prism_x("hull", [(zc - 4.8, yc - 2.0), (zc - 4.8, yc + 2.0), (zc + 0.2, yc + 2.4), (zc + 5.2, yc - 0.1),
                            (zc + 6.2, yc - 0.7), (zc + 5.4, yc - 2.1)], 3.3, "BODY", "torso")
    for s_ in (1, -1):
        boolean(hull, box(f"nose_corner{s_}", (s_ * 3.4, yc, zc + 6.0), (2.4, 6.0, 2.4), "BODY", "torso",
                          ((1, 0, s_ * 1), (0, 1, 0), (-s_ * 1, 0, 1))), "DIFFERENCE")
    # the Canopy sits in a well in the slope, its screens and bubble standing out of it, as the Mad Cat's does
    floor = (0, yc + 0.2, zc + 2.4)
    boolean(hull, box("well", V(floor) + V((0, 1.6, 0.0)), (2.5, 3.2, 2.0), "BODY2", "torso"), "DIFFERENCE")
    pod_in("canopy", floor)
    box("hull_band", (0, yc - 1.75, zc + 0.6), (6.7, 0.4, 8.6), "ACCENT", "torso")
    for s in (1, -1):
        box(f"skirt{s}", (s * 3.45, yc - 1.4, zc + 0.6), (0.4, 1.8, 7.6), "BODY2", "torso")
    # behind: the power pack, a radiator grille, two exhaust stacks
    chamfered("pack", (0, yc + 0.6, zc - 6.4), (5.6, 3.8, 3.2), 0.5, "BODY2", "torso")
    for i in range(5):
        box(f"fin{i}", (0, yc - 0.8 + i * 0.7, zc - 8.05), (4.6, 0.18, 0.15), "FRAME", "torso")
    for s in (1, -1):
        cyl(f"stack{s}", (s * 1.6, yc + 2.4, zc - 6.8), (s * 1.6, yc + 5.0, zc - 6.8), 0.45, "FRAME", "torso", 10)
        # shoulders: a pivot drum, a missile rack box on top with its tube grid cut in (CSG)
        sh = V((s * 4.6, yc + 1.0, zc - 0.8))
        cyl(f"drum{s}", sh - V((s * 0.9, 0, 0)), sh + V((s * 0.9, 0, 0)), 1.5, "FRAME", "arms", 14)
        rack = chamfered(f"rack{s}", sh + V((0, 2.6, 0.4)), (2.6, 2.2, 3.6), 0.3, "BODY", "arms")
        for i in range(3):
            for j in range(2):
                p = sh + V((-0.75 + i * 0.75, 2.2 + j * 0.8, 2.3))
                boolean(rack, cyl(f"tube{s}{i}{j}", p - V((0, 0, 0.6)), p + V((0, 0, 0.6)), 0.28, "FRAME", "arms", 8), "DIFFERENCE")
        box(f"hatch{s}", sh + V((0, 3.75, 0.4)), (2.3, 0.12, 3.2), "ACCENT", "arms")
        # the arm hangs low as a gun pod: upper strut, the pod with its magazine and barrel
        el = sh + V((s * 1.2, -2.0, 1.0))
        strut(f"upper{s}", sh, el, 1.6, "BODY2", "arms")
        pod = el + V((0, -0.6, 1.8))
        # bigger guns (owner, 2026-10-24: "larger arms for bigger guns"): a long autocannon pod and a heavy cannon
        chamfered(f"gunpod{s}", pod, (2.4, 2.4, 5.4), 0.3, "BODY", "arms")
        box(f"mag{s}", pod + V((s * 1.9, 0.2, -0.8)), (1.4, 1.6, 2.0), "FRAME", "arms")     # fed from the outside
        cyl(f"barrel{s}", pod + V((0, 0.3, 2.6)), pod + V((0, 0.3, 8.2)), 0.38 if s > 0 else 0.62, "METAL", "arms", 12)
        cyl(f"brake{s}", pod + V((0, 0.3, 7.6)), pod + V((0, 0.3, 8.6)), 0.55 if s > 0 else 0.85, "FRAME", "arms", 12)
    # the sensor, not a face: a camera mast beside the canopy, its lens looking ahead
    cyl("mast", (-1.9, yc + 2.0, zc + 0.6), (-1.9, yc + 3.6, zc + 0.6), 0.25, "FRAME", "head", 8)
    box("camera", (-1.9, yc + 3.9, zc + 0.8), (0.9, 0.7, 1.2), "BODY2", "head")
    cyl("lens", (-1.9, yc + 3.9, zc + 1.4), (-1.9, yc + 3.9, zc + 1.55), 0.25, "GLOW", "head", 10)


def roustabout():
    """The Scopedog pattern, at 24 m: a boxy upright chest on chicken legs, the Titan cockpit in its middle; the
    upper chest with the camera turret on it is the Titan's top half (it rises and tilts back), the lower chest
    front is its step. A box-magazine rifle, a pipe grab, a backpack with vents."""
    hip_y, hx = 14.4, 3.5
    chicken_legs(hip_y, hx, 1.0, (4.2, 4.0), (3.4, 3.2))
    box("hips", (0, hip_y + 0.3, -0.2), (2 * hx + 1.0, 2.6, 3.4), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yb = hip_y + 1.6
    chamfered("waist", (0, yb + 0.6, -0.2), (5.0, 1.6, 4.4), 0.4, "BODY2", "torso")
    floor = (0, yb + 1.6, 1.4)
    lower = chamfered("chest_lower", (0, yb + 2.5, -0.2), (7.0, 2.6, 5.8), 0.6, "BODY", "torso")
    upper = chamfered("chest_upper", (0, yb + 5.0, -0.4), (7.4, 2.4, 6.0), 0.6, "BODY", "torso")
    box("seam", (0, yb + 3.8, -0.2), (7.1, 0.12, 5.9), "GASKET", "torso")
    for o in (lower, upper):
        boolean(o, box(f"well_{o.name}", V(floor) + V((0, 1.05, 1.6)), (2.4, 2.25, 3.4), "BODY2", "torso"), "DIFFERENCE")
    pod_in("titan", floor)
    for s in (1, -1):
        box(f"cheek{s}", (s * 2.6, yb + 3.3, 2.75), (1.8, 4.4, 0.5), "BODY2", "torso")
    box("hazard", (0, yb + 1.25, 2.7), (5.6, 0.3, 0.2), "ACCENT", "torso")
    # the camera turret on the upper chest (it goes up with it): a drum with three lenses on a slide
    cyl("turret", (0, yb + 6.2, 0.4), (0, yb + 7.6, 0.4), 1.4, "BODY2", "head", 14)
    for i, (dx, r) in enumerate(((-0.55, 0.32), (0.35, 0.22), (0.9, 0.16))):
        cyl(f"lens{i}", (dx, yb + 6.9, 1.6), (dx, yb + 6.9, 2.0), r, "GLOW", "head", 10)
    box("visor_slot", (0, yb + 6.9, 1.62), (2.6, 0.7, 0.2), "FRAME", "head")
    # the backpack: an engine box, two vent grilles, a jet nozzle under
    chamfered("backpack", (0, yb + 4.0, -4.6), (5.6, 5.0, 2.8), 0.5, "BODY2", "torso")
    for s in (1, -1):
        for i in range(4):
            box(f"vent{s}{i}", (s * 1.4, yb + 3.0 + i * 0.6, -6.05), (1.8, 0.16, 0.12), "FRAME", "torso")
        cyl(f"jet{s}", (s * 1.6, yb + 1.0, -4.6), (s * 1.6, yb + 0.0, -4.9), 0.7, "FRAME", "torso", 12, r2=0.9)
        sh = V((s * 4.6, yb + 5.2, -0.4))
        chamfered(f"shoulder{s}", sh, (2.2, 2.6, 3.0), 0.4, "BODY", "arms")
        box(f"shstripe{s}", sh + V((s * 1.12, 0, 0)), (0.06, 1.0, 2.6), "ACCENT", "arms")
        el = sh + V((s * 0.5, -3.6, 1.0))
        strut(f"upper{s}", sh, el, 1.3, "BODY2", "arms")
        hand = el + V((0, -0.4, 2.6))
        if s > 0:
            strut(f"fore{s}", el, hand, 1.1, "BODY", "arms")
        if s > 0:                                   # the rifle: receiver, box magazine, barrel, carried in the hand
            rec = hand + V((0, -0.3, 1.2))
            box("receiver", rec, (1.3, 1.6, 4.2), "FRAME", "arms")
            box("magazine", rec + V((0, -1.7, 0.4)), (1.0, 2.0, 1.2), "BODY2", "arms")
            cyl("rifle_barrel", rec + V((0, 0.2, 2.1)), rec + V((0, 0.2, 6.8)), 0.32, "METAL", "arms", 10)
            box("rifle_sight", rec + V((0, 1.0, 0.2)), (0.4, 0.4, 1.4), "FRAME", "arms")
        else:                                       # the pipe grab is now a piston fist, for punching
            fist("punch", el, 1.0, ahead=1.2)


# ----------------------------------------------------------------------------- eight more (owner, 2026-10-24: "I need
# like 10 mech mock ups", "it's not like something we need highres either"): low-poly, each after a 1980s working
# machine on chicken legs, each carrying a different cockpit block-out at true size in its body, so all ten cockpits
# have a chassis to be judged in.

def grader():
    """A road dozer: a squat chest, the Panorama in its front, a dozer blade carried on push arms from the hips,
    a ripper behind, a beacon bar for a head."""
    hip_y, hx = 14.2, 3.8
    chicken_legs(hip_y, hx, 1.05, (4.6, 4.2), (3.8, 3.4))
    box("hips", (0, hip_y + 0.2, 0), (2 * hx + 1.2, 2.6, 3.6), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 3.6
    chamfered("torso", (0, yc, -0.6), (7.0, 4.6, 5.6), 0.7, "BODY", "torso")
    pod_in("panorama", (0, yc - 1.6, 2.4))
    for s in (1, -1):
        # the push arms run outside the thighs, from the waist's sides down to the blade's ends
        strut(f"push{s}", (s * 7.0, hip_y + 0.6, 0.6), (s * 7.0, hip_y - 4.6, 6.4), 0.8, "BODY2", "frame")
        strut(f"push_mount{s}", (s * 1.4, hip_y + 0.6, 0.6), (s * 7.3, hip_y + 0.6, 0.6), 0.7, "FRAME", "frame")
        cyl(f"tilt{s}", (s * 3.0, hip_y + 2.6, 2.0), (s * 3.0, hip_y - 2.8, 6.6), 0.35, "METAL", "frame", 8)
    box("blade", (0, hip_y - 4.4, 6.9), (15.2, 3.2, 0.5), "BODY", "frame", ((1, 0, 0), (0, 1, 0.25), (0, -0.25, 1)))
    box("blade_edge", (0, hip_y - 6.0, 7.3), (15.4, 0.4, 0.6), "METAL", "frame")
    box("ripper_beam", (0, hip_y - 0.6, -3.6), (5.0, 0.8, 1.0), "FRAME", "arms")
    for x in (-1.6, 0, 1.6):
        strut(f"tooth{x}", (x, hip_y - 0.6, -3.8), (x, hip_y - 4.4, -4.6), 0.5, "METAL", "arms")
    chamfered("engine", (0, yc + 0.2, -4.4), (5.0, 3.6, 2.6), 0.4, "BODY2", "torso")
    cyl("stack", (1.8, yc + 2.0, -4.4), (1.8, yc + 5.4, -4.4), 0.4, "FRAME", "torso", 10)
    box("beacons", (0, yc + 2.6, 0.6), (3.4, 0.5, 0.8), "BODY2", "head")
    for x in (-1.2, 1.2):
        box(f"beacon{x}", (x, yc + 3.05, 0.6), (0.6, 0.4, 0.6), "GLOW", "head")
    for s in (1, -1):
        box(f"stripe{s}", (s * 3.52, yc - 1.4, -0.6), (0.06, 0.6, 4.6), "ACCENT", "torso")


def logger():
    """A forestry harvester: a low wide hull hung between the hips with the Low bank in its nose, a crane boom on
    its back with a log grapple, a felling head with a saw for the right hand."""
    hip_y, hx = 13.0, 4.2
    chicken_legs(hip_y, hx, 1.2, (4.4, 4.4), (3.6, 3.4))
    box("hips", (0, hip_y + 0.2, 0), (2 * hx + 1.0, 2.4, 3.4), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc, zc = hip_y + 2.2, 0.4
    prism_x("hull", [(zc - 5.0, yc - 1.8), (zc - 5.0, yc + 1.8), (zc + 2.6, yc + 1.8), (zc + 5.4, yc - 0.2),
                     (zc + 5.0, yc - 1.8)], 3.6, "BODY", "torso")
    pod_in("lowbank", (0, yc - 0.6, zc + 3.4))
    box("hull_band", (0, yc - 1.2, zc), (7.3, 0.4, 9.6), "ACCENT", "torso")
    base = V((0, yc + 1.8, zc - 3.0))
    cyl("slew", base, base + V((0, 1.0, 0)), 1.4, "FRAME", "arms", 14)
    knuckle = base + V((0, 7.0, 2.0))
    strut("boom1", base + V((0, 1.0, 0)), knuckle, 1.0, "BODY", "arms")
    tip = knuckle + V((0, -2.0, 8.0))
    strut("boom2", knuckle, tip, 0.8, "BODY", "arms")
    cyl("boom_ram", base + V((0, 1.4, 1.0)), knuckle + V((0, -1.6, 0.6)), 0.3, "METAL", "arms", 8)
    for dx in (-0.7, 0.7):
        strut(f"grapple{dx}", tip, tip + V((dx, -2.6, 0)), 0.35, "FRAME", "arms")
    for s in (1, -1):
        sh = V((s * 4.6, yc + 0.6, zc + 0.6))
        cyl(f"drum{s}", sh - V((s * 0.8, 0, 0)), sh + V((s * 0.8, 0, 0)), 1.2, "FRAME", "arms", 12)
        el = sh + V((s * 0.9, -1.6, 2.6))
        strut(f"upper{s}", sh, el, 1.0, "BODY2", "arms")
        if s < 0:
            box("felling_head", el + V((0, -0.4, 1.6)), (1.6, 2.0, 1.4), "BODY", "arms")
            cyl("saw", el + V((0, -1.4, 2.4)) - V((0.15, 0, 0)), el + V((0, -1.4, 2.4)) + V((0.15, 0, 0)), 1.1, "METAL", "arms", 16)
        else:
            strut("fore", el, el + V((0, -0.4, 2.6)), 0.8, "BODY", "arms")
            box("claw", el + V((0, -0.4, 3.2)), (1.2, 1.0, 1.0), "ACCENT", "arms")


def lineman():
    """A utility bucket truck: an upright torso with the Cinema door in its chest, a telescoping boom from the left
    shoulder with a work bucket on its end, a cable drum on the back, insulator posts on the shoulders."""
    hip_y, hx = 14.0, 3.4
    chicken_legs(hip_y, hx, 1.0, (4.0, 3.8), (3.2, 3.0))
    box("hips", (0, hip_y + 0.3, 0), (2 * hx + 1.0, 2.4, 3.2), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 4.0
    chamfered("torso", (0, yc, -0.4), (6.4, 5.4, 5.0), 0.6, "BODY", "torso")
    pod_in("cinema", (0, yc - 2.0, 1.9))
    cyl("drum", (-2.2, yc + 0.2, -4.0), (2.2, yc + 0.2, -4.0), 1.8, "BODY2", "torso", 18)
    cyl("cable", (-2.0, yc + 0.2, -4.0), (2.0, yc + 0.2, -4.0), 1.95, "ACCENT", "torso", 18)
    turret = V((3.4, yc + 2.9, -0.6))
    cyl("turret", turret, turret + V((0, 0.8, 0)), 1.1, "FRAME", "arms", 12)
    b1 = turret + V((0.4, 6.0, 4.0))
    strut("boom_a", turret + V((0, 0.8, 0)), b1, 0.9, "BODY2", "arms")
    b2 = b1 + V((0, 3.0, 5.0))
    strut("boom_b", b1, b2, 0.6, "ACCENT", "arms")
    box("bucket", b2 + V((0, -1.0, 0.8)), (1.4, 1.4, 1.4), "ACCENT", "arms")
    for s in (1, -1):
        for k in range(3):
            cyl(f"ins{s}{k}", (s * 2.4, yc + 2.7, -1.6 + k * 1.0), (s * 2.4, yc + 3.5, -1.6 + k * 1.0), 0.18, "GLOW", "head", 8)
        sh = V((s * 4.0, yc + 1.4, -0.4))
        chamfered(f"shoulder{s}", sh, (1.8, 2.2, 2.6), 0.3, "BODY", "arms")
        el = sh + V((s * 0.4, -3.4, 1.4))
        strut(f"upper{s}", sh, el, 1.0, "BODY2", "arms")
        strut(f"fore{s}", el, el + V((0, -0.4, 2.8)), 0.9, "BODY", "arms")
    box("lamp_bar", (0, yc + 2.9, 1.4), (3.0, 0.4, 0.6), "GLOW", "head")


def driller():
    """A mine drill rig: the All-view sphere held in a ring frame at the front of the chest, an auger drill for the
    right arm, a core grab for the left, a lattice derrick mast standing on its back."""
    hip_y, hx = 14.4, 3.6
    chicken_legs(hip_y, hx, 1.05, (4.4, 4.0), (3.6, 3.2))
    box("hips", (0, hip_y + 0.3, -0.4), (2 * hx + 1.0, 2.4, 3.6), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 3.8
    chamfered("torso", (0, yc, -1.4), (6.6, 5.0, 4.4), 0.6, "BODY", "torso")
    pod_in("allview", (0, yc - 1.8, 1.6))
    for k in range(8):                      # the ring frame round the ball
        a0, a1 = k / 8 * 2 * math.pi, (k + 1) / 8 * 2 * math.pi
        c = V((0, yc - 0.6, 1.6))
        strut(f"ring{k}", c + V((math.cos(a0) * 1.9, math.sin(a0) * 1.9, -0.6)), c + V((math.cos(a1) * 1.9, math.sin(a1) * 1.9, -0.6)),
              0.35, "ACCENT", "torso")
    lattice("derrick", (0, yc + 2.5, -2.4), (0, yc + 9.5, -3.0), 0.9, "ACCENT", "torso", bays=5, w=0.2)
    for s in (1, -1):
        sh = V((s * 4.2, yc + 1.0, -1.0))
        chamfered(f"shoulder{s}", sh, (2.0, 2.4, 2.8), 0.4, "BODY", "arms")
        el = sh + V((s * 0.5, -3.4, 1.6))
        strut(f"upper{s}", sh, el, 1.2, "BODY2", "arms")
        if s > 0:
            cyl("auger_motor", el, el + V((0, -0.2, 1.6)), 0.9, "BODY2", "arms", 12)
            cyl("auger", el + V((0, -0.2, 1.6)), el + V((0, -0.6, 6.2)), 0.7, "METAL", "arms", 10, r2=0.1)
            for i in range(5):
                p = el + V((0, -0.2 - 0.08 * i, 2.2 + i * 0.8))
                cyl(f"flight{i}", p - V((0, 0, 0.08)), p + V((0, 0, 0.08)), 0.95 - i * 0.12, "ACCENT", "arms", 10)
        else:
            strut("core_arm", el, el + V((0, -0.6, 2.6)), 0.8, "BODY", "arms")
            cyl("core", el + V((0, -0.6, 2.6)), el + V((0, -2.4, 2.8)), 0.45, "METAL", "arms", 10)


def bowser():
    """A fuel tanker: a long cylindrical tank on its back over the hips, the Octagon cab low in front with its
    gull-wing doors, a hose reel arm and a filler boom, hazard diamonds on the tank."""
    hip_y, hx = 13.6, 3.8
    chicken_legs(hip_y, hx, 1.1, (4.4, 4.2), (3.6, 3.3))
    box("hips", (0, hip_y + 0.2, -0.6), (2 * hx + 1.2, 2.4, 3.8), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 2.8
    chamfered("chassis", (0, yc, 0.4), (6.0, 3.0, 6.4), 0.5, "BODY", "torso")
    pod_in("octagon", (0, yc - 1.0, 3.2))
    cyl("tank", (0, yc + 3.6, -6.0), (0, yc + 3.6, 1.6), 2.8, "BODY2", "torso", 20)
    for z in (-5.6, 1.2):
        cyl(f"tank_end{z}", (0, yc + 3.6, z - 0.2), (0, yc + 3.6, z + 0.2), 2.95, "BODY", "torso", 20)
    for s in (1, -1):
        box(f"diamond{s}", (s * 2.85, yc + 3.6, -2.2), (0.08, 1.4, 1.4), "ACCENT", "torso", ((1, 0, 0), (0, 1, 1), (0, -1, 1)))
        sh = V((s * 4.0, yc + 0.8, 0.6))
        cyl(f"drum{s}", sh - V((s * 0.7, 0, 0)), sh + V((s * 0.7, 0, 0)), 1.1, "FRAME", "arms", 12)
        el = sh + V((s * 0.8, -2.2, 1.6))
        strut(f"upper{s}", sh, el, 1.0 if s > 0 else 1.5, "BODY", "arms")
        if s > 0:
            cyl("reel", el - V((0.6, 0, 0)), el + V((0.6, 0, 0)), 1.2, "ACCENT", "arms", 14)
            cyl("nozzle", el + V((0, 0, 0.4)), el + V((0, -0.4, 3.6)), 0.3, "METAL", "arms", 8)
        else:
            strut("fore", el, el + V((0, -0.4, 3.0)), 1.5, "BODY2", "arms")
            box("ac_body", el + V((0, -0.4, 3.6)), (1.8, 1.8, 3.0), "FRAME", "arms")
            cyl("ac_drum", el + V((-0.9, -0.6, 3.0)), el + V((-2.0, -0.6, 3.0)), 1.3, "BODY", "arms", 16)
            for dx in (-0.35, 0.35):
                cyl(f"ac_barrel{dx}", el + V((dx, -0.2, 5.0)), el + V((dx, -0.2, 9.6)), 0.3, "METAL", "arms", 10)
    strut("filler_boom", (0, yc + 6.4, -4.0), (0, yc + 7.4, -0.6), 0.4, "FRAME", "torso")
    box("beacon", (0, yc + 6.6, -4.4), (0.6, 0.5, 0.6), "GLOW", "head")


def mule():
    """A mine haul truck: the heaviest legs here, a tall cab front with the Twin tier in it under a canopy roof,
    a dump bed tipped on its back, the radiator grille at the waist."""
    hip_y, hx = 13.4, 4.0
    chicken_legs(hip_y, hx, 1.15, (5.2, 4.8), (4.2, 3.8))
    box("hips", (0, hip_y + 0.2, 0), (2 * hx + 1.4, 2.8, 4.0), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 3.4
    chamfered("torso", (0, yc, -0.4), (6.6, 4.2, 5.4), 0.6, "BODY", "torso")
    pod_in("twintier", (0, yc - 1.6, 2.3))
    box("canopy_roof", (0, yc + 3.4, 1.4), (7.4, 0.4, 5.0), "BODY", "torso")
    for s in (1, -1):
        strut(f"roof_post{s}", (s * 3.3, yc + 2.0, 3.6), (s * 3.3, yc + 3.3, 3.6), 0.4, "FRAME", "torso")
    box("grille", (0, yc - 1.6, 2.35), (4.6, 1.0, 0.2), "FRAME", "torso")
    bed = box("bed", (0, yc + 3.0, -4.4), (7.6, 3.4, 7.0), "BODY", "torso", ((1, 0, 0), (0, 1, -0.35), (0, 0.35, 1)))
    boolean(bed, box("bed_cut", (0, yc + 3.9, -4.6), (6.8, 3.0, 6.4), "BODY", "torso", ((1, 0, 0), (0, 1, -0.35), (0, 0.35, 1))),
            "DIFFERENCE")
    for s in (1, -1):
        cyl(f"hoist{s}", (s * 2.4, yc - 1.0, -2.6), (s * 2.4, yc + 1.8, -3.4), 0.4, "METAL", "torso", 8)
        sh = V((s * 4.2, yc + 0.8, 0.0))
        chamfered(f"shoulder{s}", sh, (2.0, 2.2, 2.6), 0.3, "BODY2", "arms")
        el = sh + V((s * 1.0, -1.4, 1.6))
        strut(f"upper{s}", sh, el, 1.6, "BODY2", "arms")
        fist(f"fist{s}", el, 1.25, ahead=1.6)


def pumper():
    """A fire engine: the Tile wall in its chest, water monitors on both shoulders, hose reels at the hips, an
    extension ladder along its back, a light bar for a head."""
    hip_y, hx = 14.0, 3.5
    chicken_legs(hip_y, hx, 1.0, (4.2, 4.0), (3.4, 3.2))
    box("hips", (0, hip_y + 0.3, 0), (2 * hx + 1.0, 2.4, 3.2), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 3.8
    chamfered("torso", (0, yc, -0.6), (6.8, 5.0, 5.2), 0.6, "BODY", "torso")
    pod_in("tilewall", (0, yc - 1.8, 2.0))
    box("chrome_band", (0, yc - 2.0, -0.6), (6.9, 0.4, 5.3), "ACCENT", "torso")
    for k in range(2):
        strut(f"ladder_rail{k}", (-0.6 + k * 1.2, yc + 2.6, -3.6), (-0.6 + k * 1.2, yc - 2.0, -3.6), 0.25, "ACCENT", "torso")
    for i in range(6):
        strut(f"rung{i}", (-0.6, yc + 2.2 - i * 0.75, -3.6), (0.6, yc + 2.2 - i * 0.75, -3.6), 0.12, "ACCENT", "torso")
    box("lightbar", (0, yc + 2.7, 0.6), (3.6, 0.4, 0.8), "GLOW", "head")
    for s in (1, -1):
        cyl(f"reel{s}", (s * 0.4, hip_y - 0.4, -2.4), (s * 1.5, hip_y - 0.4, -2.4), 1.1, "ACCENT", "torso", 14)   # behind the waist
        sh = V((s * 4.1, yc + 1.4, -0.4))
        chamfered(f"shoulder{s}", sh, (1.8, 2.2, 2.6), 0.3, "BODY", "arms")
        mon = sh + V((0, 1.6, 0.4))
        cyl(f"monitor{s}", mon, mon + V((0, 0.4, 2.4)), 0.4, "METAL", "arms", 10, r2=0.25)
        el = sh + V((s * 0.4, -3.4, 1.4))
        strut(f"upper{s}", sh, el, 1.0, "BODY2", "arms")
        if s > 0:
            box("axe_head", el + V((0, -0.6, 2.0)), (0.4, 2.2, 1.8), "METAL", "arms")
        else:
            strut("hook", el, el + V((0, 0.4, 4.0)), 0.3, "METAL", "arms")


def ploughman():
    """A farm tractor: the Excavator cab on top of a long bonnet, its exhaust stack up front, big wheel hubs for
    hips, a plough share on one arm and a three-point hitch behind."""
    hip_y, hx = 14.2, 3.8
    chicken_legs(hip_y, hx, 1.05, (4.4, 4.0), (3.6, 3.3))
    for s in (1, -1):
        cyl(f"hub{s}", (s * (hx + 0.2), hip_y, 0), (s * (hx + 1.6), hip_y, 0), 3.0, "RUBBER", "legs", 20)
        cyl(f"rim{s}", (s * (hx + 1.5), hip_y, 0), (s * (hx + 1.7), hip_y, 0), 1.8, "ACCENT", "legs", 16)
    box("hips", (0, hip_y + 0.2, 0), (2 * hx, 2.4, 3.4), "FRAME", "legs")
    hip_y = waist(hip_y, hx)
    yc = hip_y + 2.8
    box("bonnet", (0, yc, 2.6), (3.8, 2.4, 6.0), "BODY", "torso")
    for i in range(6):
        box(f"grille{i}", (0, yc - 0.9 + i * 0.35, 5.62), (3.0, 0.12, 0.1), "FRAME", "torso")
    cyl("stack", (1.2, yc + 1.2, 4.0), (1.2, yc + 4.4, 4.0), 0.35, "FRAME", "torso", 10)
    chamfered("cab_base", (0, yc + 0.6, -1.6), (5.4, 3.6, 4.4), 0.5, "BODY", "torso")
    pod_in("cab", (0, yc + 2.4, -1.4))
    box("cab_roof", (0, yc + 5.0, -1.4), (3.2, 0.3, 2.6), "BODY", "torso")
    box("hitch", (0, hip_y - 0.6, -3.6), (3.0, 0.6, 1.4), "FRAME", "arms")
    for x in (-1.2, 1.2):
        strut(f"link{x}", (x, hip_y - 0.6, -3.6), (x * 0.6, hip_y - 2.6, -5.2), 0.35, "METAL", "arms")
    for s in (1, -1):
        sh = V((s * 3.4, yc + 1.4, -1.6))
        chamfered(f"shoulder{s}", sh, (1.6, 2.0, 2.4), 0.3, "BODY2", "arms")
        el = sh + V((s * 0.6, -3.0, 1.8))
        strut(f"upper{s}", sh, el, 0.9, "BODY2", "arms")
        if s > 0:
            box("share", el + V((0, -1.0, 1.8)), (0.5, 1.6, 3.0), "METAL", "arms", ((1, 0.6, 0), (-0.6, 1, 0), (0, 0, 1)))
        else:
            box("bale_fork", el + V((0, -0.6, 1.8)), (1.6, 0.3, 2.6), "METAL", "arms")


# ----------------------------------------------------------------------------- two Gundams (owner, 2026-10-24: "Make a
# couple Gundam variant", after "if we do gundams, I'd want something practical looking like this" of the RX-79[G]
# Ground Type; then "None of those look like gundams"). What makes one read as a Gundam, all of it here: a small helmet
# head with a big yellow V fin, two green eyes and a red chin; a broad blue chest with yellow slatted vents either side
# over a red abdomen and a white waist with a V buckle; front skirts; big rounded shoulder pads; long legs, knee pads,
# shins flaring wide at the ankle; two beam saber hilts over the shoulders; white, blue, red and yellow.

def octa(cx, cz, w, d, c):
    """A plan octagon (x, z): a w by d rectangle centred on (cx, cz) with its corners cut by c."""
    hw, hd = w / 2, d / 2
    return [(cx + hw - c, cz + hd), (cx + hw, cz + hd - c), (cx + hw, cz - hd + c), (cx + hw - c, cz - hd),
            (cx - hw + c, cz - hd), (cx - hw, cz - hd + c), (cx - hw, cz + hd - c), (cx - hw + c, cz + hd)]


def gundam(layout, bulk=1.0):
    """A Gundam, about 24 m to the tip of its V fin. bulk thickens the limbs. Returns useful points for its kit."""
    hx = 1.95 * bulk
    for s in (1, -1):
        x = s * hx
        # the foot: a long sole, a toe wedge, the heel, the ankle guard (red)
        chamfered(f"foot{s}", (x, 0.6, 0.8), (3.1 * bulk, 1.2, 6.6), 0.35, "ACCENT", "legs")
        box(f"toe{s}", (x, 0.85, 4.4), (2.8 * bulk, 0.9, 1.4), "ACCENT", "legs", ((1, 0, 0), (0, 1, 0.7), (0, -0.7, 1)))
        chamfered(f"ankle{s}", (x, 1.7, 0.4), (2.2 * bulk, 1.4, 2.4), 0.3, "FRAME", "legs")
        # the shin: lofted, flaring wide over the ankle, narrower under the knee (white), a red-trimmed front plate
        loft(f"shin{s}", [(1.4, octa(x, 0.2, 4.4 * bulk, 5.0, 0.8)), (2.4, octa(x, 0.2, 4.3 * bulk, 4.8, 0.8)),
                          (4.6, octa(x, 0.1, 3.3 * bulk, 3.7, 0.6)), (7.0, octa(x, 0.0, 2.7 * bulk, 3.0, 0.5))], "BODY", "legs")
        box(f"shin_vent{s}", (x, 2.6, -2.25), (1.6 * bulk, 1.4, 0.2), "FRAME", "legs")
        # the knee: joint, then the big pad standing proud of the knee's front
        cyl(f"knee{s}", (x - 1.3 * bulk, 7.3, 0.0), (x + 1.3 * bulk, 7.3, 0.0), 1.1 * bulk, "FRAME", "legs", 14)
        chamfered(f"kneepad{s}", (x, 7.6, 1.6), (2.9 * bulk, 3.2, 1.6), 0.4, "BODY", "legs")
        box(f"kneebar{s}", (x, 7.6, 2.42), (1.6 * bulk, 0.4, 0.1), "ACCENT", "legs")
        # the thigh, white, and the hip ball
        chamfered(f"thigh{s}", (x, 9.8, 0.0), (2.3 * bulk, 4.6, 2.6 * bulk), 0.35, "BODY", "legs")
        cyl(f"hipjoint{s}", (x - 1.0, 12.0, 0.0), (x + 1.0, 12.0, 0.0), 1.15 * bulk, "FRAME", "legs", 14)
    # the waist: the crotch block (red), the white waist band and its yellow V buckle, the skirts
    box("hips", (0, 12.1, 0.0), (2 * hx + 0.4, 1.6, 2.8), "FRAME", "legs")
    box("crotch", (0, 11.6, 0.9), (1.7, 1.9, 1.6), "ACCENT", "torso", ((1, 0, 0), (0, 1, -0.2), (0, 0.2, 1)))
    chamfered("waist", (0, 13.3, 0.0), (5.2 * bulk, 1.3, 3.6), 0.3, "BODY", "torso")
    for s in (1, -1):
        strut(f"buckle{s}", (0, 12.9, 1.85), (s * 0.9, 13.75, 1.85), 0.35, "YELLOW", "torso")
        box(f"fskirt{s}", (s * 1.45, 11.9, 1.75), (2.3 * bulk, 2.6, 0.35), "BODY", "torso", ((1, 0, 0), (0, 1, -0.22), (0, 0.22, 1)))
        box(f"fskirt_trim{s}", (s * 1.45, 10.8, 2.0), (2.0 * bulk, 0.35, 0.1), "ACCENT", "torso")
        box(f"sskirt{s}", (s * (2.6 * bulk + 0.5), 12.2, 0.0), (0.35, 2.6, 3.0), "BODY", "torso", ((1, 0, 0), (0.18 * s, 1, 0), (0, 0, 1)))
    box("rskirt", (0, 12.3, -1.95), (4.4 * bulk, 2.4, 0.35), "BODY", "torso")
    # the abdomen (red) and the chest (blue), the cockpit in the chest's middle, the yellow slatted vents either side
    chamfered("abdomen", (0, 14.6, 0.1), (3.8 * bulk, 1.6, 3.2), 0.3, "ACCENT", "torso")
    yc = 17.2
    chest = chamfered("chest", (0, yc, -0.1), (7.4 * bulk, 3.6, 4.6), 0.6, "BODY2", "torso")
    boolean(chest, box("cockpit_well", (0, yc - 0.5, 2.0), (2.5, 2.4, 1.8), "BODY2", "torso"), "DIFFERENCE")
    pod_in(layout, (0, yc - 1.6, 1.15))
    for s in (1, -1):
        vx = s * 2.45 * bulk
        box(f"vent{s}", (vx, yc + 0.6, 2.2), (1.7, 1.5, 0.3), "YELLOW", "torso")
        for i in range(4):
            box(f"slat{s}{i}", (vx, yc + 0.08 + i * 0.36, 2.38), (1.5, 0.12, 0.08), "FRAME", "torso")
    chamfered("collar", (0, yc + 2.05, -0.3), (3.6, 0.7, 3.0), 0.2, "BODY", "torso")
    # the head: neck, white helmet, face, eyes, red chin, vulcans, the yellow V fin and its red crest
    hy = yc + 3.4
    cyl("neck", (0, yc + 2.2, -0.2), (0, hy - 0.6, -0.2), 0.55, "FRAME", "head", 10)
    hy += 0.2
    chamfered("helmet", (0, hy, -0.15), (2.4, 2.2, 2.6), 0.45, "BODY", "head")
    box("face", (0, hy - 0.4, 1.1), (1.4, 1.2, 0.4), "BODY", "head")
    box("chin", (0, hy - 1.05, 1.2), (0.75, 0.42, 0.4), "ACCENT", "head")
    for s in (1, -1):
        box(f"eye{s}", (s * 0.37, hy - 0.08, 1.33), (0.52, 0.2, 0.08), "GLOW", "head", ((1, -0.18 * s, 0), (0.18 * s, 1, 0), (0, 0, 1)))
        cyl(f"vulcan{s}", (s * 0.95, hy + 0.45, 0.9), (s * 0.95, hy + 0.45, 1.3), 0.15, "FRAME", "head", 8)
        strut(f"vfin{s}", (s * 0.15, hy + 0.7, 1.3), (s * 1.9, hy + 2.8, 1.4), 0.24, "YELLOW", "head", d=0.14)
    box("crest", (0, hy + 0.7, 1.3), (0.5, 0.6, 0.3), "ACCENT", "head")
    # the backpack, its two thrusters, the beam saber hilts over the shoulders
    chamfered("backpack", (0, yc + 0.2, -3.2), (4.6, 3.8, 1.9), 0.4, "BODY", "torso")
    for s in (1, -1):
        cyl(f"thruster{s}", (s * 1.2, yc - 1.6, -3.5), (s * 1.2, yc - 3.0, -4.0), 0.55, "FRAME", "torso", 12, r2=0.85)
        cyl(f"saber{s}", (s * 1.5, yc + 1.6, -3.2), (s * 2.2, yc + 3.8, -3.4), 0.2, "BODY", "torso", 10)
    # the arms: a big rounded shoulder pad, the upper arm, the elbow, a white forearm, a grey fist
    hands = {}
    for s in (1, -1):
        sh = V((s * (3.7 * bulk + 1.3), yc + 0.9, -0.2))
        chamfered(f"shoulder{s}", sh + V((s * 0.3, 0.4, 0)), (2.8 * bulk, 2.8, 3.4), 0.7, "BODY", "arms")
        box(f"shoulder_band{s}", sh + V((s * 0.3, -0.95, 0)), (2.9 * bulk, 0.3, 3.5), "BODY2", "arms")
        el = sh + V((s * 0.3, -3.5, 0.6))
        strut(f"upper{s}", sh, el, 1.3 * bulk, "FRAME", "arms")
        cyl(f"elbow{s}", el - V((0.8, 0, 0)), el + V((0.8, 0, 0)), 0.75 * bulk, "FRAME", "arms", 12)
        wr = el + V((0, -0.5, 3.0))
        chamfered(f"forearm{s}", (el + wr) / 2, (2.0 * bulk, (wr - el).length + 0.5, 2.1 * bulk), 0.35, "BODY", "arms", between(el, wr))
        box(f"hand{s}", wr + V((0, -0.1, 0.6)), (1.1, 1.3, 1.2), "FRAME", "arms")
        hands[s] = wr + V((0, -0.1, 0.6))
    return dict(hands=hands, yc=yc, hy=hy)


def picket():
    """The RX-79[G] Ground Type, the owner's "practical" Gundam: weathered white, blue and red, a 100 mm machine gun
    with a box magazine in the right hand, the tall shield on the left forearm, a radio antenna on the backpack, the
    Panorama in its chest (the upper hatch flips up and the lower folds down, as the 08th MS Team's does)."""
    g = gundam("panorama")
    h = g["hands"][1]
    box("mg_receiver", h + V((0, 0.3, 1.0)), (0.9, 1.2, 3.8), "FRAME", "arms")
    box("mg_magazine", h + V((0, -1.25, 0.9)), (0.7, 1.7, 1.2), "BODY2", "arms")
    cyl("mg_barrel", h + V((0, 0.45, 2.9)), h + V((0, 0.45, 6.6)), 0.25, "METAL", "arms", 10)
    box("mg_shroud", h + V((0, 0.45, 3.5)), (0.6, 0.6, 1.4), "FRAME", "arms")
    l = g["hands"][-1]
    box("shield", l + V((-1.4, 0.2, -1.6)), (0.35, 6.6, 3.2), "BODY", "arms")
    box("shield_band", l + V((-1.6, 1.6, -1.6)), (0.06, 0.6, 3.0), "ACCENT", "arms")
    box("shield_mark", l + V((-1.6, -0.4, -1.6)), (0.06, 1.4, 1.4), "BODY2", "arms")
    box("radio", (1.5, g["yc"] + 1.8, -3.9), (0.8, 1.2, 0.8), "FRAME", "torso")
    cyl("radio_whip", (1.5, g["yc"] + 2.4, -3.9), (1.6, g["yc"] + 6.6, -4.2), 0.05, "METAL", "torso", 6)


def sapper():
    """A heavy Gundam, Full Armor style: the same Gundam under bolt-on armour plates (chest, shoulders, thighs, shins),
    a long cannon over the right shoulder, a missile pod on the left forearm, a beam rifle in the right hand, the Titan
    in its chest."""
    g = gundam("titan", bulk=1.12)
    yc = g["yc"]
    for nm, c, sz in (("armor_chest", (0, yc - 0.4, 2.5), (6.4, 1.4, 0.35)),
                      ("armor_belly", (0, 14.5, 1.85), (3.6, 1.2, 0.3))):
        box(nm, c, sz, "ARMOR", "torso")
    for s in (1, -1):
        x = s * 1.95 * 1.12
        box(f"armor_thigh{s}", (x, 9.8, 1.75), (2.6, 3.6, 0.35), "ARMOR", "legs")
        box(f"armor_shin{s}", (x, 3.8, 2.45), (3.0, 3.2, 0.35), "ARMOR", "legs", ((1, 0, 0), (0, 1, 0.12), (0, -0.12, 1)))
        box(f"armor_pauldron{s}", (s * (3.7 * 1.12 + 1.6), yc + 2.6, -0.2), (3.2, 0.5, 3.6), "ARMOR", "arms")
    mount = V((2.4, yc + 2.6, -2.8))
    box("cannon_mount", mount, (1.4, 1.2, 2.2), "FRAME", "torso")
    cyl("cannon", mount + V((0, 0.7, -1.0)), mount + V((0, 1.5, 9.4)), 0.5, "METAL", "torso", 12)
    cyl("cannon_brake", mount + V((0, 1.42, 8.8)), mount + V((0, 1.55, 10.0)), 0.75, "FRAME", "torso", 12)
    l = g["hands"][-1]
    pod = box("missile_pod", l + V((-1.4, 0.8, -1.6)), (1.2, 1.8, 3.2), "ARMOR", "arms")
    for i in range(3):
        p = l + V((-1.4, 0.3 + i * 0.5, 0.0))
        boolean(pod, cyl(f"cell{i}", p - V((0, 0, 0.4)), p + V((0, 0, 0.4)), 0.18, "FRAME", "arms", 8), "DIFFERENCE")
    h = g["hands"][1]
    box("rifle_body", h + V((0, 0.3, 1.2)), (0.8, 1.1, 4.4), "FRAME", "arms")
    cyl("rifle_barrel", h + V((0, 0.45, 3.4)), h + V((0, 0.45, 7.4)), 0.3, "BODY", "arms", 10)
    box("rifle_scope", h + V((0, 1.15, 1.4)), (0.4, 0.4, 1.6), "BODY2", "arms")

CHASSIS = {"foreman": (foreman, "Foreman", "the excavator"), "stevedore": (stevedore, "Stevedore", "the dock crane, all struts"),
           "ironhorse": (ironhorse, "Ironhorse", "the locomotive"),
           "harrow": (harrow, "Harrow", "new: the Mad Cat pattern, one hull between the hips, the Canopy in its nose"),
           "roustabout": (roustabout, "Roustabout", "new: the Scopedog pattern, the Titan in its chest, the upper chest lifts"),
           "grader": (grader, "Grader", "new: a road dozer, the Panorama, a blade on push arms"),
           "logger": (logger, "Logger", "new: a forestry harvester, the Low bank in a low hull, a crane boom"),
           "lineman": (lineman, "Lineman", "new: a utility bucket truck, the Cinema door, a bucket boom"),
           "driller": (driller, "Driller", "new: a mine drill rig, the All-view sphere in a ring, an auger"),
           "bowser": (bowser, "Bowser", "new: a fuel tanker, the Octagon under a tank"),
           "mule": (mule, "Mule", "new: a mine haul truck, the Twin tier under a canopy roof, a dump bed"),
           "pumper": (pumper, "Pumper", "new: a fire engine, the Tile wall, water monitors on the shoulders"),
           "ploughman": (ploughman, "Ploughman", "new: a farm tractor, the Excavator cab over a bonnet"),
           "picket": (picket, "Picket", "Gundam: the RX-79[G] Ground Type, the Panorama, machine gun and shield"),
           "sapper": (sapper, "Sapper", "Gundam: Full Armor style, the Titan, shoulder cannon, missile pod, beam rifle")}


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
        STATE.update(key=key, anchor=None, pods=[])
        fn()
        apply_variety(key)
        clips = arm_leg_clips()
        print("CLIPS", key, clips, flush=True)
        bpy.context.view_layer.update()
        pts = [o.matrix_world @ v.co for o in bpy.data.objects if o.type == "MESH" for v in o.data.vertices]
        lo, hi = [min(p[i] for p in pts) for i in range(3)], [max(p[i] for p in pts) for i in range(3)]
        tris = 0
        for o in bpy.data.objects:
            if o.type != "MESH":
                continue
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
            if o is not root and o.parent is None:      # a cockpit block-out keeps its own nodes under its root
                o.parent = root
        path = os.path.join(OUT, f"{key}.glb")
        bpy.ops.object.select_all(action="SELECT")
        bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=True, export_yup=True)
        row = dict(id=key, name=name, what=what, file=f"mech_blockouts/{key}.glb", height_m=round(hi[2] - lo[2], 2),
                   width_m=round(hi[0] - lo[0], 2), length_m=round(hi[1] - lo[1], 2), tris=tris,
                   legs_m3=round(legs, 1), torso_m3=round(torso, 1), legs_over_torso=round(legs / torso, 2) if torso else None,
                   arm_leg_clips=clips)
        rows.append(row)
        print("BUILT", row, flush=True)
    with open(MANIFEST, "w") as f:
        json.dump(dict(generator="tools/blender/build_mech_blockouts.py", target_height_m=HEIGHT_M, mechs=rows), f, indent=1)


main()
