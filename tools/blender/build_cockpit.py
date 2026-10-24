"""Crab Breeze: the cockpit interiors, built to the pilot's eye, their surfaces baked to PBR maps.

  /opt/blender/blender --background --factory-startup --python tools/blender/build_cockpit.py -- \
      [--variant wall|jockey|harness] [--size 4096] [--samples 16] [--map-samples 4] [--no-denoise] [--no-bake]

Run tools/blender/bake_cockpit_panels.py, tools/textures/gen_cockpit_displays.py and
tools/textures/gen_mech_plating.mjs first: this script reads their maps. It writes one interior as glTF
with its textures beside it (assets/models/cockpit_<variant>/cockpit_<variant>.gltf) and a manifest
(assets/models/cockpit_<variant>.json).

The owner, 2026-10-24: "Design the mech cockpit. Use detailed textures. Use the texturing skill we had
on star crew bridge sim", "The cockpit should be enclosed fully. We can view outside via screens like in
gundam", "I want normal maps, speculat maps and ao for all our custom textures", "Id like a couple
variations of cockpit designs please", "The cockpit is part of the mech, inside of it with the armor
opening up at the chest". So these are interiors only: the mech's chest armour (build_mechs.py) is the
outside and opens first; then the interior's own front opens (the INNER_* nodes) and the pilot steps in.

  wall     seated in a boxy cabin under a belt of flat screens, twin grips (the references)
  jockey   riding forward on a saddle, chest pad and handlebars, a low wide screen bank
  harness  standing in a harness, forearm rests ending in grips, a tall octagon of flat screens

Every variant: metres, glTF frame (+y up, the pilot faces +z, +x is the pilot's left), origin on the
floor under the eye; the screens showing the outside use the material SCREEN_allview; picture screens
are SCREEN_<name>; the panels (cockpit_panels.py) are laid on as real plates; the inside is baked into
two atlases (front: the controls and what moves; cabin: the rest) as base colour, tangent normal, ORM
and emission; real-time lamps export as KHR_lights_punctual; the manifest carries the envelope promised
to the chest, the bounds closed and open, sockets and each moving node's motion.
"""

import argparse
import json
import math
import os
import shutil
import sys
import time

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cockpit_kit as K  # noqa: E402
import cockpit_panels as CP  # noqa: E402
import pbr as PBR  # noqa: E402

ROOT = K.ROOT
PANELS = os.path.join(ROOT, "assets", "textures", "cockpit", "panels")
DISPLAYS = os.path.join(ROOT, "assets", "textures", "cockpit", "displays")
MODEL = MANIFEST = None   # set per variant in main(): assets/models/cockpit_<variant>/ and cockpit_<variant>.json
RAW = os.path.join(ROOT, "output", "cockpit_raw")

G2B = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # glTF frame to Blender
EYE = Vector((0.0, 1.20, 0.0))
TRI_BUDGET = 600_000  # an RTX 3070 (owner, 2026-10-24: "don't worry about 200k triangle count")


def B(v):
    return Vector((v[0], -v[2], v[1]))


def V(*a):
    return Vector(a)


def basis(n, up):
    """Axes of a face that faces n: right (the viewer's right), up, n. glTF frame, right handed."""
    n = Vector(n).normalized()
    r = Vector(up).cross(n).normalized()
    u = n.cross(r).normalized()
    return r, u, n


# ----------------------------------------------------------------------------- the set builder

class Set:
    def __init__(self, mats):
        self.mats = mats
        self.parts = []          # dicts: ob, group (front, cabin, screen, allview, shell), node
        self.n = 0
        self.disp = {}           # picture screens: node name -> display image

    def _ob(self, bm, mat, group, node=None, name=None, bevel=0.0, seg=2, smooth=False, uvs=False):
        if not uvs:
            uvl = bm.loops.layers.uv.get("panel") or bm.loops.layers.uv.new("panel")
            for f in bm.faces:
                for lp in f.loops:
                    lp[uvl].uv = (0.002, 0.002)
        bmesh.ops.transform(bm, matrix=G2B, verts=bm.verts)
        if smooth:
            for f in bm.faces:
                f.smooth = True
        self.n += 1
        nm = name or f"part_{self.n}"
        me = bpy.data.meshes.new(nm)
        bm.to_mesh(me)
        bm.free()
        me.materials.append(mat if isinstance(mat, bpy.types.Material) else self.mats[mat])
        ob = bpy.data.objects.new(nm, me)
        bpy.context.scene.collection.objects.link(ob)
        if bevel:
            m = ob.modifiers.new("bevel", "BEVEL")
            m.width = bevel
            m.segments = seg
            m.limit_method = "ANGLE"
            m.angle_limit = math.radians(35)
            m.harden_normals = False
        self.parts.append(dict(ob=ob, group=group, node=node))
        return ob

    def obox(self, c, axes, size, mat, group="cabin", node=None, bevel=0.0, seg=2, name=None):
        """A box centred on c with axes (r, u, n) and size (w, h, d), glTF frame."""
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        r, u, n = axes
        R = Matrix((r, u, n)).transposed().to_4x4()
        bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(c) @ R @ Matrix.Diagonal((*size, 1)))
        return self._ob(bm, mat, group, node, name, bevel, seg)

    def box(self, lo, hi, mat, group="cabin", node=None, bevel=0.0, seg=2, name=None):
        lo, hi = Vector(lo), Vector(hi)
        return self.obox((lo + hi) / 2, (V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)), hi - lo, mat, group, node, bevel, seg, name)

    def cyl(self, p0, p1, r, mat, group="cabin", node=None, r1=None, seg=24, name=None):
        p0, p1 = Vector(p0), Vector(p1)
        d = p1 - p0
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r1 is None else r1, depth=d.length,
                              matrix=Matrix.Translation((0, 0, d.length / 2)))
        q = Vector((0, 0, 1)).rotation_difference(d.normalized())
        bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(p0) @ q.to_matrix().to_4x4())
        return self._ob(bm, mat, group, node, name, smooth=seg > 8)

    def tube(self, pts, r, mat, group="cabin", node=None, closed=False, res=4):
        """A cable, pipe or loop along points (glTF frame), converted to a mesh."""
        cu = bpy.data.curves.new("tube", "CURVE")
        cu.dimensions = "3D"
        cu.bevel_depth = r
        cu.bevel_resolution = 2
        cu.resolution_u = res
        cu.use_fill_caps = not closed
        sp = cu.splines.new("BEZIER")
        sp.bezier_points.add(len(pts) - 1)
        for bp, p in zip(sp.bezier_points, pts):
            bp.co = B(p)
            bp.handle_left_type = bp.handle_right_type = "AUTO"
        sp.use_cyclic_u = closed
        ob = bpy.data.objects.new("tube", cu)
        bpy.context.scene.collection.objects.link(ob)
        dg = bpy.context.evaluated_depsgraph_get()
        me = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
        bpy.data.objects.remove(ob)
        bpy.data.curves.remove(cu)
        bm = bmesh.new()
        bm.from_mesh(me)
        bpy.data.meshes.remove(me)
        bmesh.ops.transform(bm, matrix=G2B.inverted(), verts=bm.verts)   # back to glTF frame; _ob converts
        for lay in list(bm.loops.layers.uv.values()):
            bm.loops.layers.uv.remove(lay)
        return self._ob(bm, mat, group, node, smooth=True)

    def plate(self, panel, c, n, up, t=0.012, group="front", node=None):
        """A panel's plate: front face through c facing n, its maps UV'd across the front face, the
        edges taking the panel's corner texel."""
        w, h = CP.PANELS[panel][0], CP.PANELS[panel][1]
        r, u, nn = basis(n, up)
        c = Vector(c) + nn * 0.0015          # proud of the face it is laid on: never coplanar (no z-fighting)
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        R = Matrix((r, u, nn)).transposed().to_4x4()
        bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(c - nn * t / 2) @ R @ Matrix.Diagonal((w, h, t, 1)))
        uvl = bm.loops.layers.uv.new("panel")
        bm.normal_update()
        for f in bm.faces:
            front = f.normal.dot(nn) > 0.9
            for lp in f.loops:
                if front:
                    d = lp.vert.co - c
                    lp[uvl].uv = (d.dot(r) / w + 0.5, d.dot(u) / h + 0.5)
                else:
                    lp[uvl].uv = (0.003, 0.003)
        return self._ob(bm, panel_material(panel), group, node, name=f"plate_{panel}", uvs=True)

    def quad(self, c, n, up, w, h, mat, group, name, node=None):
        """A single quad with 0..1 UVs (screens, the HUD combiner)."""
        r, u, nn = basis(n, up)
        c = Vector(c)
        bm = bmesh.new()
        vs = [bm.verts.new(c + r * sx * w / 2 + u * sy * h / 2) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        f = bm.faces.new(vs)
        uvl = bm.loops.layers.uv.new("panel")
        for lp, uv in zip(f.loops, ((0, 0), (1, 0), (1, 1), (0, 1))):
            lp[uvl].uv = uv
        return self._ob(bm, mat, group, node, name=name, uvs=True)

    def ellipsoid_cap(self, c, rad, y_cut, mat, group, node=None, name=None, seg=64, rings=32):
        """The upper part of an ellipsoid above y_cut: the canopy glass."""
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=rings, radius=1.0)
        bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Diagonal((rad[0], rad[2], rad[1], 1)))   # sphere z is up
        bmesh.ops.transform(bm, verts=bm.verts, matrix=G2B.inverted())
        bmesh.ops.translate(bm, verts=bm.verts, vec=Vector(c))
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.y < y_cut - 1e-4], context="VERTS")
        return self._ob(bm, mat, group, node, name, smooth=True)


def panel_material(name):
    """A panel's maps as a Cycles material for the bake: albedo darkened by its own occlusion, the
    relief as a normal map, the backlit buttons as emission (UV map 'panel')."""
    mn = f"panel_{name}"
    if mn in bpy.data.materials:
        return bpy.data.materials[mn]
    mat = bpy.data.materials.new(mn)
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    for x in list(N):
        N.remove(x)
    out = N.new("ShaderNodeOutputMaterial")
    bsdf = N.new("ShaderNodeBsdfPrincipled")
    L.new(bsdf.outputs[0], out.inputs[0])
    uv = N.new("ShaderNodeUVMap")
    uv.uv_map = "panel"

    def tex(kind, colour):
        t = N.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(os.path.join(PANELS, f"{name}_{kind}.png"), check_existing=True)
        t.image.colorspace_settings.name = "sRGB" if colour else "Non-Color"
        t.interpolation = "Cubic"
        L.new(uv.outputs[0], t.inputs[0])
        return t

    alb, orm, nor, emi = tex("albedo", True), tex("orm", False), tex("normal", False), tex("emit", True)
    sep = N.new("ShaderNodeSeparateColor")
    L.new(orm.outputs[0], sep.inputs[0])
    L.new(alb.outputs[0], bsdf.inputs["Base Color"])
    L.new(sep.outputs[1], bsdf.inputs["Roughness"])
    # the panel's own occlusion and metalness, read by the PBR bake through an emission swap
    for nm, out in (("AO_SRC", 0), ("METAL_SRC", 2)):
        m = N.new("ShaderNodeMath")
        m.name = nm
        m.operation = "ADD"
        m.inputs[1].default_value = 0.0
        L.new(sep.outputs[out], m.inputs[0])
    nm = N.new("ShaderNodeNormalMap")
    nm.uv_map = "panel"
    L.new(nor.outputs[0], nm.inputs["Color"])
    L.new(nm.outputs[0], bsdf.inputs["Normal"])
    L.new(emi.outputs[0], bsdf.inputs["Emission Color"])
    bsdf.inputs["Emission Strength"].default_value = 1.0
    return mat


def emission_material(name, image_path, strength, alpha=False):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    for x in list(N):
        N.remove(x)
    out = N.new("ShaderNodeOutputMaterial")
    t = N.new("ShaderNodeTexImage")
    t.image = bpy.data.images.load(image_path, check_existing=True)
    em = N.new("ShaderNodeEmission")
    em.inputs["Strength"].default_value = strength
    L.new(t.outputs[0], em.inputs["Color"])
    if alpha:
        tr = N.new("ShaderNodeBsdfTransparent")
        mix = N.new("ShaderNodeMixShader")
        L.new(t.outputs["Alpha"], mix.inputs[0])
        L.new(tr.outputs[0], mix.inputs[1])
        L.new(em.outputs[0], mix.inputs[2])
        L.new(mix.outputs[0], out.inputs[0])
    else:
        L.new(em.outputs[0], out.inputs[0])
    return mat


def glass_material():
    mat = bpy.data.materials.new("GLASS_canopy")
    mat.use_nodes = True
    nt = mat.node_tree
    for x in list(nt.nodes):
        nt.nodes.remove(x)
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    tr = nt.nodes.new("ShaderNodeBsdfTransparent")
    tr.inputs["Color"].default_value = (0.9, 0.94, 0.97, 1.0)
    nt.links.new(tr.outputs[0], out.inputs[0])
    return mat


# ----------------------------------------------------------------------------- the pod's shape

D = math.radians
MID = V(0.0, 1.2, 0.1)          # a point inside the cabin: faces and frame members turn toward it (set per variant)
# The cabin is a box, the way the owner's references are (Titanfall, Armored Core, the Gundam pods;
# owner, 2026-10-24: "That's not a mech cockpit", "The curved ones are just fish eye pov"). Above the
# consoles a belt of flat screens runs round it: the front screen, two chamfered screens at the
# front corners, a screen on each side wall, heavy pillars at the corners, a header beam over
# them and a flat ceiling with three monitors hanging from it. Plan corners (x, z) at the screens'
# foot, y = BELT[0]; the front and chamfer screens lean in LEAN metres at the top.
BELT = (1.00, 1.64)
CEIL = 1.86
LEAN = 0.10
CORNERS = {"F": (0.56, 1.00), "S": (0.86, 0.45), "R": (0.86, -0.16)}


def plan(c, s):
    """A plan corner (x, z) on side s (+1 the pilot's left), at the screens' foot."""
    x, z = CORNERS[c]
    return V(s * x, BELT[0], z)


def inward(a, b):
    """The inward normal of the wall from a to b (plan), toward the cabin's middle."""
    d = (b - a)
    n = V(-d.z, 0, d.x).normalized()
    if n.dot(MID - (a + b) / 2) < 0:
        n = -n
    return n


def quad_ob(S, pts, mat, group, node=None, name=None, uvs=False):
    """A flat quad (or a strip of quads) facing the cabin's middle, UVs 0..1 if uvs."""
    bm = bmesh.new()
    vs = [bm.verts.new(p) for p in pts]
    f = bm.faces.new(vs)
    bm.normal_update()
    mid = sum(pts, Vector()) / len(pts)
    if f.normal.dot(MID - mid) < 0:
        bmesh.ops.reverse_faces(bm, faces=[f])
    if uvs:
        uvl = bm.loops.layers.uv.new("panel")
        for lp, uv in zip(f.loops, ((0, 0), (1, 0), (1, 1), (0, 1))):
            lp[uvl].uv = uv
    return S._ob(bm, mat, group, node, name, uvs=uvs)


def bar(S, p0, p1, w, d, mat, group, node=None, bevel=0.004, n=None):
    """A frame member from p0 to p1, w wide, d deep, its face toward the cabin's middle (or n)."""
    p0, p1 = Vector(p0), Vector(p1)
    m = (p0 + p1) / 2
    a = (p1 - p0).normalized()
    n = Vector(n) if n is not None else (MID - m)
    u = n.cross(a).normalized()
    n = a.cross(u).normalized()
    return S.obox(m, (a, u, n), ((p1 - p0).length + w, w, d), mat, group, node, bevel=bevel)


def slab(S, pts, t, mat, group, node=None, bevel=0.01):
    """Armour: the quad pts (in order) pushed out by t, away from the cabin's middle."""
    pts = [Vector(p) for p in pts]
    mid = sum(pts, Vector()) / 4
    n = (pts[1] - pts[0]).cross(pts[3] - pts[0]).normalized()
    if n.dot(mid - MID) < 0:
        n = -n
    bm = bmesh.new()
    lo = [bm.verts.new(p) for p in pts]
    hi = [bm.verts.new(p + n * t) for p in pts]
    bm.faces.new(lo)
    bm.faces.new(list(reversed(hi)))
    for i in range(4):
        j = (i + 1) % 4
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return S._ob(bm, mat, group, node, bevel=bevel)


def flat_screen(S, name, c, node, strip=True):
    """One flat screen on the corners c (foot left, foot right, top right, top left), showing the
    outside world (SCREEN_allview), its back plate behind it, a heavy bezel round it and a cyan strip
    under it."""
    n = (c[1] - c[0]).cross(c[3] - c[0]).normalized()
    mid = sum(c, Vector()) / 4
    if n.dot(MID - mid) < 0:
        n = -n
    quad_ob(S, c, "screen_wall", "allview", node, name=f"SCREEN_wall_{name}")
    back = [p - n * 0.02 for p in c]
    quad_ob(S, list(reversed(back)), "paint_dk", "cabin", node)
    for k in range(4):
        bar(S, c[k] + n * 0.02, c[(k + 1) % 4] + n * 0.02, 0.05, 0.05, "bezel", "cabin", node, n=n)
    if strip:
        lo0, lo1 = c[0].lerp(c[1], 0.1) + n * 0.05 - (c[3] - c[0]).normalized() * 0.03, c[0].lerp(c[1], 0.9) + n * 0.05 - (c[3] - c[0]).normalized() * 0.03
        bar(S, lo0, lo1, 0.008, 0.008, "cyan", "cabin", node, bevel=0.0, n=n)
    return c


def screen(S, name, b0, b1, lean, node, height=BELT[1] - BELT[0]):
    """A flat screen whose foot runs b0 to b1, leaning in by lean at its top (flat_screen)."""
    n = inward(b0, b1)
    t0, t1 = b0 + n * lean + V(0, height, 0), b1 + n * lean + V(0, height, 0)
    return flat_screen(S, name, [b0, b1, t1, t0], node)


# The nodes the game animates: name -> (parent, pivot in the cockpit frame). Every mesh hangs off
# one of these, its pivot at the node's origin, so turning the node about its origin is the motion.
NODES = {}      # set per variant: name -> (parent, pivot in the cockpit frame)
WALL_NODES = {
    "pod": (None, V(0, 0, 0)),
    "INNER_front": ("pod", V(0, CEIL + 0.12, 0.40)),
    "INNER_side_L": ("pod", V(0.95, 1.30, 0.45)),
    "INNER_side_R": ("pod", V(-0.95, 1.30, 0.45)),
    "INNER_sill": ("pod", V(0, 0.0, 0.98)),
    "CONSOLE_front": ("INNER_front", V(0, 1.04, 0.80)),     # the dash rides up with the front unit
    "SCREEN_left": ("CONSOLE_front", None),
    "SCREEN_center": ("CONSOLE_front", None),
    "SCREEN_right": ("CONSOLE_front", None),
    "CTRL_stick": ("pod", V(-0.49, 0.665, 0.30)),
    "CTRL_throttle": ("pod", V(0.4725, 0.66, 0.24)),
}


# ----------------------------------------------------------------------------- the cockpit

def build_wall(S):
    """The seated cabin of flat screens (the owner's references): a belt of screens over the consoles."""
    X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
    I3 = (X, Y, Z)
    S.vents = []

    def vent(node, p, d, kind, stage, rate=1.0):
        S.vents.append(dict(node=node, p=V(*p), d=V(*d).normalized(), kind=kind, stage=stage, rate=rate))

    # --- the tub: floor, footwell, walls, rear bulkhead, sill rails
    S.plate("floor", (0, 0.0, 0.05), Y, Z, t=0.02, group="cabin")
    S.box((-0.72, -0.04, 0.70), (0.72, 0.0, 0.94), "steel", bevel=0.004)
    S.plate("footwell", (0, 0.27, 0.92), -Z, Y, t=0.02, group="cabin", node="INNER_sill")
    S.box((-0.76, 0.0, 0.92), (0.76, 0.62, 0.98), "paint", node="INNER_sill")
    S.box((-0.70, 0.56, 0.975), (0.70, 0.60, 0.99), "hazard", node="INNER_sill")       # its outer face: the step
    S.cyl(V(-0.7, 0.0, 0.98), V(0.7, 0.0, 0.98), 0.025, "steel", seg=16)
    for s in (1, -1):
        S.box((s * 0.72 if s > 0 else -0.78, -0.04, -0.66), (0.78 if s > 0 else -0.72, 0.96, 0.98), "paint")
        S.plate(f"wall_{'L' if s > 0 else 'R'}", (s * 0.72, 0.80, 0.05), -s * X, Y, t=0.012, group="cabin")
        S.box((s * 0.69 - 0.04, 0.935, -0.66) if s > 0 else (-0.73, 0.935, -0.66),
              (0.73, 0.99, 0.86) if s > 0 else (-0.69 + 0.04, 0.99, 0.86), "paint_dk", bevel=0.008)
    S.box((-0.78, -0.04, -0.70), (0.78, 1.22, -0.62), "paint")
    S.plate("rear", (0, 0.66, -0.62), Z, Y, t=0.012, group="cabin")
    S.box((-0.6, 1.14, -0.64), (0.6, 1.20, -0.58), "bezel", bevel=0.004)
    S.box((-0.5, 1.13, -0.585), (0.5, 1.137, -0.578), "white_lit")

    # --- side consoles: bodies, tops (the console panels), inner faces (breakers), front blocks
    for s, side in ((1, "L"), (-1, "R")):
        x0, x1 = sorted((s * 0.36, s * 0.72))
        S.box((x0, 0.0, -0.42), (x1, 0.645, 0.52), "paint", bevel=0.004)
        S.plate(f"console_{side}", (s * 0.53, 0.657, 0.05), Y, Z, t=0.012)
        S.plate(f"breakers_{side}", (s * 0.36, 0.38, 0.05), -s * X, Y, t=0.01, group="cabin")
        S.box((x0 - (0.02 if s < 0 else 0), 0.0, 0.52), (x1 + (0.02 if s > 0 else 0), 0.62, 0.94), "paint_dk", bevel=0.006)
        S.box((x0, 0.645, -0.43), (x1, 0.663, -0.39), "paint_dk", bevel=0.003)
        S.obox(V(s * 0.66, 0.925, 0.05), I3, (0.03, 0.012, 0.6), "bezel", bevel=0.002)
        S.obox(V(s * 0.66, 0.917, 0.05), I3, (0.016, 0.004, 0.56), "white_lit")
        for i, (r, mat) in enumerate(((0.012, "rubber"), (0.009, "red"), (0.01, "rubber"), (0.007, "yellow"))):
            y = 0.905 - i * 0.022
            S.tube([V(s * 0.70, y, -0.66), V(s * 0.70, y, -0.36), V(s * 0.695, y - 0.01, 0.0), V(s * 0.70, y, 0.46),
                    V(s * 0.70, y + 0.02, 0.84)], r, mat)
        for z in (-0.5, -0.3, 0.5, 0.75):
            S.obox(V(s * 0.70, 0.872, z), I3, (0.03, 0.09, 0.025), "steel", bevel=0.003)

    # --- the front console: dash, MFDs, glare shield, knee panel, on one node that swings clear
    cn = "CONSOLE_front"
    a = math.radians(18)
    nd, ud = V(0, math.sin(a), -math.cos(a)), V(0, math.cos(a), math.sin(a))
    cd = V(0, 0.80, 0.70)
    S.plate("dash_main", cd, nd, ud, t=0.014, node=cn)
    rd, ud2, nd2 = basis(nd, ud)
    S.obox(cd - nd * 0.15, (rd, ud2, nd2), (1.58, 0.47, 0.28), "paint_dk", group="front", node=cn)
    disp = {"SCREEN_left": "mfd_left", "SCREEN_center": "mfd_center", "SCREEN_right": "mfd_right"}
    for name, px, py, s in CP.MFDS:
        c = cd + rd * px + ud2 * py + nd2 * 0.002
        O, I, t = (s + 0.06) / 2, s / 2, 0.03
        for (cx, cy, w, h) in ((0, (O + I) / 2, 2 * O, O - I), (0, -(O + I) / 2, 2 * O, O - I),
                               ((O + I) / 2, 0, O - I, 2 * I), (-(O + I) / 2, 0, O - I, 2 * I)):
            S.obox(c + rd * cx + ud2 * cy + nd2 * t / 2, (rd, ud2, nd2), (w, h, t), "bezel", group="front", node=cn, bevel=0.003)
        S.obox(c + nd2 * 0.004, (rd, ud2, nd2), (2 * I, 2 * I, 0.008), "glass", group="front", node=cn)
        for side in range(4):
            k = 5 if side < 2 else 4
            for i in range(k):
                tt = (i + 0.5) / k * 2 - 1
                if side < 2:
                    p = c + rd * (tt * I * 0.92) + ud2 * ((1 if side == 0 else -1) * (O + I) / 2)
                    sz = (s * 0.11, (O - I) * 0.5, 0.012)
                else:
                    p = c + ud2 * (tt * I * 0.85) + rd * ((1 if side == 2 else -1) * (O + I) / 2)
                    sz = ((O - I) * 0.5, s * 0.1, 0.012)
                S.obox(p + nd2 * (t + 0.005), (rd, ud2, nd2), sz, "paint_lt", group="front", node=cn, bevel=0.0025)
        for cx in (-1, 1):
            S.obox(c + rd * cx * (O - (O - I) / 2) + ud2 * (O - (O - I) / 2) + nd2 * (t + 0.004), (rd, ud2, nd2),
                   ((O - I) * 0.55, (O - I) * 0.55, 0.01), "steel", group="front", node=cn, bevel=0.003)
        S.quad(c + nd2 * 0.0105, nd2, ud2, 2 * I * 0.97, 2 * I * 0.97,
               emission_material(name, os.path.join(DISPLAYS, f"{disp[name]}.png"), 2.5), "screen", name, node=name)
        S.disp[name] = disp[name]
    S.plate("glare", (0, 1.05, 0.70), Y, Z, t=0.012, node=cn)
    S.box((-0.79, 1.01, 0.60), (0.79, 1.038, 0.80), "paint_dk", group="front", node=cn, bevel=0.01)
    S.box((-0.78, 1.0, 0.605), (0.78, 1.012, 0.625), "bezel", group="front", node=cn, bevel=0.002)
    S.box((-0.60, 0.996, 0.611), (0.60, 1.0, 0.62), "white_lit", group="front", node=cn)
    b = math.radians(10)
    nk, uk = V(0, math.sin(b), -math.cos(b)), V(0, math.cos(b), math.sin(b))
    ck = V(0, 0.43, 0.64)
    S.plate("knee", ck, nk, uk, t=0.012, node=cn)
    rk, uk2, nk2 = basis(nk, uk)
    S.obox(ck - nk2 * 0.13, (rk, uk2, nk2), (1.12, 0.32, 0.25), "paint_dk", group="front", node=cn)
    for s in (1, -1):        # the console's swing arms and their rams
        S.obox(V(s * 0.70, 0.86, 0.86), I3, (0.04, 0.38, 0.06), "steel", group="front", node=cn, bevel=0.006)
        S.cyl(V(s * 0.70, 1.04, 0.80), V(s * 0.74, 1.04, 0.80), 0.04, "metal", group="front", node=cn)

    # --- the seat: pedestal, cushion, bolsters, back, headrest, harness, eject handle
    S.box((-0.18, 0.0, -0.22), (0.18, 0.34, 0.18), "paint_dk", bevel=0.01)
    S.box((-0.25, 0.34, -0.24), (0.25, 0.45, 0.22), "fabric", bevel=0.03, seg=3)
    for s in (1, -1):
        S.box((s * 0.22 - 0.035, 0.38, -0.24), (s * 0.22 + 0.035, 0.53, 0.2), "fabric", bevel=0.025, seg=3)
    bot, top = V(0, 0.45, -0.22), V(0, 1.14, -0.32)
    ub = (top - bot).normalized()
    nb = X.cross(ub).normalized()
    rb, ub2, nb2 = basis(nb, ub)
    S.obox((bot + top) / 2, (rb, ub2, nb2), (0.50, 0.70, 0.11), "fabric", bevel=0.03, seg=3)
    S.obox((bot + top) / 2 - nb2 * 0.08, (rb, ub2, nb2), (0.56, 0.76, 0.06), "paint_dk", bevel=0.01)
    for s in (1, -1):
        S.obox((bot + top) / 2 + rb * s * 0.27 + nb2 * 0.03, (rb, ub2, nb2), (0.07, 0.62, 0.13), "fabric", bevel=0.025, seg=3)
    S.obox(top + ub2 * 0.14 + nb2 * 0.01, (rb, ub2, nb2), (0.30, 0.20, 0.10), "fabric", bevel=0.03, seg=3)
    S.obox(top + ub2 * 0.14 - nb2 * 0.06, (rb, ub2, nb2), (0.34, 0.24, 0.05), "paint_dk", bevel=0.008)
    for s in (1, -1):
        p0 = top + rb * s * 0.09 + nb2 * 0.062
        p1 = bot + rb * s * 0.11 + nb2 * 0.062 + ub2 * 0.05
        S.obox((p0 + p1) / 2, (rb, ub2, nb2), (0.05, (p0 - p1).length, 0.008), "strap")
        S.obox(V(s * 0.13, 0.456, -0.02), I3, (0.17, 0.008, 0.05), "strap")
        S.obox(V(s * 0.21, 0.5, -0.02), I3, (0.008, 0.09, 0.05), "strap")
    S.cyl(V(0, 0.452, -0.02), V(0, 0.468, -0.02), 0.04, "metal", seg=32)
    S.cyl(V(0, 0.468, -0.02), V(0, 0.471, -0.02), 0.022, "red", seg=32)
    loop = [V(0.07 * math.cos(t), 0.36 + 0.05 * math.sin(t), 0.235) for t in [i * 2 * math.pi / 12 for i in range(12)]]
    S.tube(loop, 0.011, "hazard", closed=True)
    S.box((-0.05, 0.33, 0.215), (0.05, 0.36, 0.23), "bezel", bevel=0.004)

    # --- the controls: a grip at the end of each console (the references' twin side sticks).
    # The right grip flies the mech and turns the torso (it twists); the left is the throttle,
    # riding its quadrant through idle, walk, run, skate and the jet gate.
    for node, side in (("CTRL_stick", -1), ("CTRL_throttle", 1)):
        gp = NODES[node][1]
        if node == "CTRL_stick":
            S.cyl(gp - V(0, 0.01, 0), gp + V(0, 0.045, 0), 0.06, "rubber", r1=0.03, seg=24)
        S.cyl(gp, gp + V(0, 0.15, 0.01), 0.014, "steel", node=node, group="front", seg=16)
        up, fw = V(0, 0.96, 0.27).normalized(), V(0, -0.27, 0.96).normalized()
        S.obox(gp + V(0, 0.22, 0.03), (V(1, 0, 0), up, fw), (0.055, 0.14, 0.07), "rubber", node=node, group="front", bevel=0.02, seg=3)
        S.obox(gp + V(0, 0.21, 0.075), (V(1, 0, 0), up, fw), (0.022, 0.04, 0.02), "metal", node=node, group="front", bevel=0.006)
        S.obox(gp + V(side * 0.03, 0.235, 0.02), (V(1, 0, 0), up, fw), (0.012, 0.05, 0.04), "paint_dk", node=node, group="front", bevel=0.004)
        top = gp + V(0, 0.295, 0.05)
        S.cyl(top, top + V(0, 0.012, 0.004), 0.011, "steel", node=node, group="front", seg=16)
        S.cyl(top + V(-side * 0.018, -0.006, -0.02), top + V(-side * 0.018, 0.006, -0.02), 0.008, "red_lit" if node == "CTRL_stick" else "amber",
              node=node, group="front", seg=16)
        S.cyl(top + V(side * 0.02, -0.01, -0.005), top + V(side * 0.024, 0.0, -0.005), 0.006, "green", node=node, group="front", seg=12)
    for s in (1, -1):
        S.cyl(V(s * 0.15, 0.2, 0.92), V(s * 0.15, 0.16, 0.80), 0.014, "steel", seg=12)
        n_p = V(0, 0.7, -0.71).normalized()
        S.obox(V(s * 0.15, 0.13, 0.78), basis(n_p, Y), (0.11, 0.2, 0.025), "steel", bevel=0.005)
        S.obox(V(s * 0.15, 0.135, 0.775), basis(n_p, Y), (0.1, 0.03, 0.03), "hazard", bevel=0.003)
        # cyan accent strips along the consoles' inner edges
        S.obox(V(s * 0.365, 0.664, 0.05), I3, (0.006, 0.004, 0.86), "cyan")

    # --- the screen belt: front screen on the visor, chamfer screens on the doors, side screens
    hn = "INNER_front"
    cnr = {}
    for s in (1, -1):
        cnr[s] = {k: plan(k, s) for k in CORNERS}
    inset = lambda a, b, d: (a + (b - a).normalized() * d, b + (a - b).normalized() * d)
    f0, f1 = inset(cnr[1]["F"], cnr[-1]["F"], 0.06)
    screen(S, "front", f0, f1, LEAN, hn)
    for s, door in ((1, "INNER_side_L"), (-1, "INNER_side_R")):
        a, b = inset(cnr[s]["S"], cnr[s]["F"], 0.06)
        screen(S, f"chamfer_{'L' if s > 0 else 'R'}", a, b, LEAN, door)
        a, b = inset(cnr[s]["R"], cnr[s]["S"], 0.06)
        screen(S, f"side_{'L' if s > 0 else 'R'}", a, b, 0.0, None, height=0.56)
    # pillars at the corners, leaning with the screens, rivets and a cyan line down each
    top_of = {}
    for s in (1, -1):
        nF = inward(cnr[1]["F"], cnr[-1]["F"])
        nC = inward(cnr[s]["S"], cnr[s]["F"])
        for k, lean_n in (("F", (nF + nC).normalized() * LEAN), ("S", nC * LEAN * 0.5), ("R", V(0, 0, 0))):
            b = cnr[s][k] - V(0, 0.04, 0)
            t = cnr[s][k] + lean_n + V(0, BELT[1] - BELT[0] + 0.1, 0)
            top_of[(s, k)] = t
            node = None
            bar(S, b, t, 0.11, 0.10, "paint", "cabin", node, bevel=0.012)
            n_in = (V(0, 1.3, 0.1) - (b + t) / 2)
            n_in.y = 0
            n_in.normalize()
            bar(S, b + n_in * 0.06, t + n_in * 0.06, 0.012, 0.012, "cyan", "cabin", node, bevel=0.0)
            for i in range(7):
                q = b.lerp(t, (i + 0.5) / 7) + n_in * 0.055
                S.cyl(q - n_in * 0.01, q + n_in * 0.006, 0.009, "steel", "cabin", node, seg=10)
    # the header beam over the screens, the sill under them, the shelf out to the tub walls
    ring = [top_of[(1, "R")], top_of[(1, "S")], top_of[(1, "F")], top_of[(-1, "F")], top_of[(-1, "S")], top_of[(-1, "R")]]
    for i, (p0, p1) in enumerate(zip(ring, ring[1:])):
        node = hn if i == 2 else None
        bar(S, p0 - V(0, 0.03, 0), p1 - V(0, 0.03, 0), 0.10, 0.09, "paint", "cabin", node, bevel=0.012)
    floor_ring = [cnr[1]["R"], cnr[1]["S"], cnr[1]["F"], cnr[-1]["F"], cnr[-1]["S"], cnr[-1]["R"]]
    sill_node = [None, "INNER_side_L", hn, "INNER_side_R", None]     # the sill under each screen rides it
    for i, (p0, p1) in enumerate(zip(floor_ring, floor_ring[1:])):
        bar(S, p0 - V(0, 0.05, 0), p1 - V(0, 0.05, 0), 0.10, 0.06, "paint_dk", "cabin", sill_node[i], bevel=0.008, n=V(0, 1, 0))
    for s in (1, -1):
        x0, x1 = sorted((s * 0.72, s * 0.90))
        S.box((x0, 0.93, -0.72), (x1, 0.97, 0.47), "paint_dk", bevel=0.004)
        # the wall behind the side screen, aft of it: ribbed, with a vent and conduit
        x0, x1 = sorted((s * 0.86, s * 0.90))
        S.box((x0, 0.96, -0.74), (x1, CEIL, -0.16), "paint_dk")
        for z in (-0.30, -0.45, -0.60):
            S.box((s * 0.86 - 0.03 if s > 0 else -0.86, 0.98, z - 0.02), (s * 0.86 if s > 0 else -0.83, CEIL - 0.02, z + 0.02), "paint", bevel=0.006)
        for k, y in enumerate((1.25, 1.31, 1.37)):
            S.tube([V(s * 0.83, y, -0.72), V(s * 0.83, y, -0.18)], 0.012, ("red", "rubber", "yellow")[k])
    S.box((-0.90, 1.20, -0.76), (0.90, CEIL, -0.70), "paint_dk")
    # the ceiling: plate, beams across, conduits, round vents, a grab handle, a lamp
    S.box((-0.92, CEIL, -0.76), (0.92, CEIL + 0.03, 0.395), "paint_dk")
    S.box((-0.92, CEIL, 0.405), (0.92, CEIL + 0.03, 1.02), "paint_dk", node=hn)
    for z in (0.62, 0.20, -0.25, -0.62):
        S.box((-0.88, CEIL - 0.07, z - 0.04), (0.88, CEIL, z + 0.04), "paint", node=hn if z > 0.4 else None, bevel=0.01)
    for x in (-0.3, 0.3):
        S.tube([V(x, CEIL - 0.04, -0.7), V(x, CEIL - 0.04, 0.37)], 0.02, "steel")
        S.tube([V(x, CEIL - 0.04, 0.43), V(x, CEIL - 0.04, 0.95)], 0.02, "steel", node=hn)
    for x, z in ((0.55, 0.0), (-0.55, 0.0), (0.0, -0.45)):
        for k in range(3):
            S.cyl(V(x, CEIL - 0.004 * (k + 1), z), V(x, CEIL - 0.004 * k, z), 0.09 - 0.025 * k, ("bezel", "paint_dk", "steel")[k], seg=32)
    S.cyl(V(-0.18, CEIL - 0.1, 0.0), V(0.18, CEIL - 0.1, 0.0), 0.012, "yellow", seg=12)
    for x in (-0.18, 0.18):
        S.cyl(V(x, CEIL - 0.1, 0.0), V(x, CEIL, 0.0), 0.012, "steel", seg=12)
    S.box((-0.25, CEIL - 0.03, -0.62), (0.25, CEIL, -0.56), "bezel", bevel=0.004)
    S.box((-0.22, CEIL - 0.034, -0.61), (0.22, CEIL - 0.03, -0.57), "white_lit")
    # three monitors hanging from the front header, tilted at the eye
    for name, img, x, y, z in (("OVH_left", "mfd_right", 0.40, 1.74, 0.70), ("OVH_center", "mfd_center", 0.0, 1.76, 0.78),
                               ("OVH_right", "mfd_left", -0.40, 1.74, 0.70)):
        c = V(x, y, z)
        nn = (EYE - c).normalized()
        r, u, nn = basis(nn, V(0, 1, 0))
        S.obox(c - nn * 0.02, (r, u, nn), (0.24, 0.17, 0.04), "bezel", "cabin", hn, bevel=0.006)
        S.cyl(c - nn * 0.03 + u * 0.05, V(x, CEIL, z - 0.05), 0.012, "steel", "cabin", hn, seg=12)
        S.obox(c + nn * 0.001 - u * 0.072, (r, u, nn), (0.2, 0.006, 0.004), "cyan", "cabin", hn)
        S.quad(c + nn * 0.0015, nn, u, 0.205, 0.145, emission_material(name, os.path.join(DISPLAYS, f"{img}.png"), 2.0), "screen", name, node=name)
        NODES[name] = (hn, None)
        S.disp[name] = img

    # --- the shell: plain structure closing the cabin, seen only through the opened chest (the
    # mech's chest armour is the outside, tools/blender/build_mechs.py)
    for s in (1, -1):
        x0, x1 = sorted((s * 0.98, s * 1.00))
        S.box((x0, -0.06, -0.78), (x1, 1.97, 0.40), "ext_dark", "shell")
    S.box((-1.0, -0.06, -0.80), (1.0, 1.97, -0.78), "ext_dark", "shell")
    S.box((-1.0, 1.95, -0.78), (1.0, 1.97, 0.40), "ext_dark", "shell")
    S.box((-1.0, -0.08, -0.80), (1.0, -0.06, 0.98), "ext_dark", "shell")
    for s in (1, -1):                                    # posts tying the screen belt to the shell
        S.box((s * 0.90 - 0.04, 0.90, -0.18), (s * 0.90 + 0.04, 1.95, -0.10), "ext_frame", "shell")
    # cold mist pouring out over the floor once the sill drops
    for i in range(6):
        vent("pod", V(-0.5 + 0.2 * i, 0.15, 0.95), (0, -0.2, 1), "mist", "inner_open", 1.0)



# ----------------------------------------------------------------------------- shared hardware
# Mining Mike's boss-kit vocabulary, inside the cabin: bolt rows, hoses with collars, hazard bands,
# glowing core lamps (godot-sandbox tank_bosses_kit.js: bolts, hose, stripes, core).

def bolt_row(S, p0, p1, n, nrm, node=None, group="cabin", r=0.009):
    """n bolt heads from p0 to p1 standing out of a face along nrm."""
    p0, p1, nrm = Vector(p0), Vector(p1), Vector(nrm).normalized()
    for i in range(n):
        q = p0.lerp(p1, (i + 0.5) / n)
        S.cyl(q - nrm * 0.004, q + nrm * 0.007, r, "steel", group, node, seg=10)


def hose(S, pts, r, node=None, mat="rubber"):
    """A hose along pts with a steel collar at each end."""
    pts = [Vector(p) for p in pts]
    S.tube(pts, r, mat, "cabin", node)
    for a, b in ((pts[0], pts[1]), (pts[-1], pts[-2])):
        d = (b - a).normalized()
        S.cyl(a - d * 0.01, a + d * 0.035, r * 1.45, "steel", "cabin", node, seg=16)


def core_lamp(S, c, n, r, node=None, mat="cyan"):
    """A round lamp in a stepped bezel facing n (the kit's glowing cores)."""
    c, n = Vector(c), Vector(n).normalized()
    S.cyl(c - n * 0.02, c + n * 0.012, r * 1.35, "bezel", "cabin", node, seg=32)
    S.cyl(c, c + n * 0.016, r * 1.1, "steel", "cabin", node, seg=32)
    S.cyl(c, c + n * 0.02, r, mat, "cabin", node, seg=32)


def monitor(S, name, img, c, look_at, w, h, node, rod_to=None):
    """A small picture monitor (an MFD or an overhead monitor) at c, turned to look_at."""
    c = Vector(c)
    nn = (Vector(look_at) - c).normalized()
    r, u, nn = basis(nn, V(0, 1, 0))
    S.obox(c - nn * 0.02, (r, u, nn), (w + 0.035, h + 0.025, 0.04), "bezel", "cabin", node, bevel=0.006)
    S.obox(c + nn * 0.001 - u * (h / 2 + 0.002), (r, u, nn), (w * 0.85, 0.006, 0.004), "cyan", "cabin", node)
    if rod_to is not None:
        S.cyl(c - nn * 0.03 + u * 0.03, Vector(rod_to), 0.012, "steel", "cabin", node, seg=12)
    S.quad(c + nn * 0.0015, nn, u, w, h, emission_material(name, os.path.join(DISPLAYS, f"{img}.png"), 2.0), "screen", name, node=name)
    NODES[name] = (node or "pod", None)
    S.disp[name] = img


def grip(S, node, gp, axis, side):
    """A hand grip on its node at gp whose barrel runs along axis, with its buttons and switches.
    side +1 is the pilot's left hand."""
    a = Vector(axis).normalized()
    up = V(0, 1, 0) if abs(a.y) < 0.9 else V(0, 0, 1)
    r, u, n = basis(a, up)
    S.cyl(gp - a * 0.02, gp + a * 0.05, 0.026, "steel", "front", node, seg=16)
    S.obox(gp + a * 0.12, (r, u, a), (0.055, 0.06, 0.14), "rubber", "front", node, bevel=0.02, seg=3)
    S.obox(gp + a * 0.19 + u * 0.03, (r, u, a), (0.03, 0.02, 0.025), "red_lit" if side < 0 else "amber", "front", node, bevel=0.004)
    S.obox(gp + a * 0.16 + u * 0.035, (r, u, a), (0.012, 0.012, 0.012), "green", "front", node, bevel=0.003)
    S.obox(gp + a * 0.09 - u * 0.035, (r, u, a), (0.03, 0.02, 0.04), "steel", "front", node, bevel=0.005)


def walls(S, x, y0, y1, z0, z1, mat="paint_dk", t=0.03, group="cabin"):
    """Side walls at +-x (inner face), from y0 to y1 and z0 to z1, t thick, outward."""
    for s in (1, -1):
        a, b = sorted((s * x, s * (x + t)))
        S.box((a, y0, z0), (b, y1, z1), mat, group)


# ----------------------------------------------------------------------------- the jockey

JOCKEY_NODES = {
    "pod": (None, V(0, 0, 0)),
    "INNER_front": ("pod", V(0, 1.70, 0.74)),
    "INNER_yoke": ("pod", V(0, 0.78, 0.75)),
    "CONSOLE_front": ("pod", V(0, 0.02, 1.16)),
    "CTRL_stick": ("INNER_yoke", V(-0.40, 0.86, 0.44)),
    "CTRL_throttle": ("INNER_yoke", V(0.40, 0.86, 0.44)),
}


def build_jockey(S):
    """The racing cabin: the pilot rides it like a motorbike, leaning onto a chest pad, knees in
    pads, hands on handlebar grips, looking down a low wide bank of flat screens with a strip of
    screen overhead. A slim cabin for a slim chest."""
    X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
    I3 = (X, Y, Z)
    S.vents = []
    W, RO, ZB, ZF = 0.82, 1.68, -0.95, 1.16
    fn, yk, cn = "INNER_front", "INNER_yoke", "CONSOLE_front"
    # --- the tub: floor, walls, rear bulkhead, roof
    S.plate("floor", (0, 0.0, -0.05), Y, Z, t=0.02, group="cabin")
    S.box((-W, -0.06, ZB), (W, -0.025, ZF), "paint_dk")
    walls(S, W, -0.06, 0.80, ZB, ZF)
    walls(S, W, 0.80, RO + 0.04, ZB, 0.74)
    S.box((-W - 0.03, -0.06, ZB - 0.03), (W + 0.03, RO + 0.04, ZB), "paint")
    S.plate("rear", (0, 0.80, ZB), Z, Y, t=0.012, group="cabin")
    S.box((-W - 0.03, RO, ZB), (W + 0.03, RO + 0.04, 0.735), "paint_dk")
    for z in (-0.70, -0.30, 0.10, 0.50):
        S.box((-W, RO - 0.06, z - 0.035), (W, RO, z + 0.035), "paint", bevel=0.008)
    for s in (1, -1):
        hose(S, [V(s * 0.70, RO - 0.05, ZB + 0.02), V(s * 0.72, RO - 0.10, -0.40), V(s * 0.74, RO - 0.06, 0.20), V(s * 0.70, RO - 0.12, 0.68)], 0.022)
        hose(S, [V(s * 0.62, RO - 0.04, ZB + 0.02), V(s * 0.62, RO - 0.05, 0.66)], 0.014, mat="red")
        S.plate(f"wall_{'L' if s > 0 else 'R'}", (s * W, 1.02, -0.30), -s * X, Y, t=0.012, group="cabin")
        bolt_row(S, V(s * W, 1.30, ZB + 0.05), V(s * W, 1.30, 0.70), 14, -s * X)
        bolt_row(S, V(s * W, 0.75, ZB + 0.05), V(s * W, 0.75, 0.70), 14, -s * X)
    S.box((-0.30, RO - 0.03, -0.62), (0.30, RO, -0.56), "bezel", bevel=0.004)
    S.box((-0.27, RO - 0.034, -0.61), (0.27, RO - 0.03, -0.57), "white_lit")
    core_lamp(S, V(0, 1.42, ZB), Z, 0.07)
    S.box((-0.40, 1.30, ZB), (0.40, 1.33, ZB + 0.008), "hazard")

    # --- the saddle on its pedestal, the foot pegs and the knee pads
    S.box((-0.13, 0.0, -0.58), (0.13, 0.60, -0.18), "paint_dk", bevel=0.01)
    bolt_row(S, V(0.135, 0.08, -0.56), V(0.135, 0.08, -0.20), 6, X)
    bolt_row(S, V(-0.135, 0.08, -0.56), V(-0.135, 0.08, -0.20), 6, -X)
    n_s = V(0, 0.99, 0.12).normalized()
    rs, us, ns = basis(n_s, Z)
    S.obox(V(0, 0.66, -0.38), (rs, us, ns), (0.30, 0.46, 0.10), "fabric", bevel=0.035, seg=3)
    S.obox(V(0, 0.75, -0.62), I3, (0.30, 0.10, 0.10), "fabric", bevel=0.035, seg=3)       # the rear hump
    S.obox(V(0, 0.60, -0.38), I3, (0.32, 0.03, 0.50), "paint_dk", bevel=0.006)
    for s in (1, -1):
        # foot pegs with pedal plates, behind and under the knees
        S.cyl(V(s * 0.12, 0.16, -0.24), V(s * 0.30, 0.16, -0.24), 0.02, "steel", seg=12)
        S.obox(V(s * 0.24, 0.17, -0.22), I3, (0.10, 0.02, 0.22), "steel", bevel=0.004)
        S.obox(V(s * 0.24, 0.185, -0.22), I3, (0.09, 0.006, 0.05), "hazard")
        # knee pads on a frame from the floor, facing back at the knees
        nk = V(0, 0.35, -0.94).normalized()
        S.obox(V(s * 0.20, 0.46, 0.14), basis(nk, Y), (0.14, 0.20, 0.07), "fabric", bevel=0.03, seg=3)
        S.obox(V(s * 0.20, 0.46, 0.18), basis(nk, Y), (0.16, 0.22, 0.03), "paint_dk", bevel=0.006)
        S.cyl(V(s * 0.20, 0.0, 0.26), V(s * 0.20, 0.40, 0.20), 0.025, "steel", seg=16)
    # --- side consoles: the console panels on top, breakers on their inner faces, MFDs at their fronts
    for s, side in ((1, "L"), (-1, "R")):
        x0, x1 = sorted((s * 0.50, s * 0.80))
        S.box((x0, 0.0, -0.55), (x1, 0.55, 0.45), "paint", bevel=0.004)
        S.plate(f"console_{side}", (s * 0.65, 0.562, -0.05), Y, Z, t=0.012)
        S.plate(f"breakers_{side}", (s * 0.50, 0.30, -0.05), -s * X, Y, t=0.01, group="cabin")
        S.obox(V(s * 0.507, 0.556, -0.05), I3, (0.006, 0.004, 0.92), "cyan")
        monitor(S, f"SCREEN_{'left' if s > 0 else 'right'}", "mfd_left" if s > 0 else "mfd_right",
                V(s * 0.64, 0.72, 0.38), EYE, 0.21, 0.15, None)
        S.obox(V(s * 0.64, 0.60, 0.40), I3, (0.06, 0.10, 0.06), "steel", bevel=0.006)
    # --- the yoke: a carriage on two columns behind the dash carrying the chest pad and handlebars.
    # It rises clear for boarding.
    for s in (1, -1):
        S.box((s * 0.50 - 0.03, 0.0, 0.72), (s * 0.50 + 0.03, RO, 0.78), "steel", bevel=0.006)
        S.box((s * 0.50 - 0.045, 0.70, 0.705), (s * 0.50 + 0.045, 0.73, 0.795), "hazard")
    S.box((-0.53, 0.74, 0.71), (0.53, 0.82, 0.79), "paint", group="front", node=yk, bevel=0.01)
    bolt_row(S, V(-0.45, 0.78, 0.79), V(0.45, 0.78, 0.79), 10, Z, node=yk, group="front")
    for s in (1, -1):
        bar(S, V(s * 0.46, 0.79, 0.73), V(s * 0.20, 0.93, 0.16), 0.06, 0.05, "paint", "front", yk, n=V(0, 1, 0))
        S.cyl(V(s * 0.40, 0.84, 0.55), V(s * 0.40, 0.87, 0.47), 0.02, "steel", "front", yk, seg=12)
    n_p = V(0, 0.55, -0.83).normalized()
    rp, up_, np_ = basis(n_p, Y)
    S.obox(V(0, 0.98, 0.06), (rp, up_, np_), (0.36, 0.36, 0.08), "fabric", "front", yk, bevel=0.035, seg=3)
    S.obox(V(0, 0.98, 0.06) - np_ * 0.06, (rp, up_, np_), (0.40, 0.40, 0.05), "paint_dk", "front", yk, bevel=0.008)
    S.cyl(V(-0.42, 0.86, 0.44), V(0.42, 0.86, 0.44), 0.018, "steel", "front", yk, seg=16)
    grip(S, "CTRL_stick", V(-0.40, 0.86, 0.44), -X, -1)
    grip(S, "CTRL_throttle", V(0.40, 0.86, 0.44), X, 1)
    hose(S, [V(0.47, 0.70, 0.74), V(0.62, 0.62, 0.70), V(0.74, 0.58, 0.60)], 0.016)
    # --- the dash: the knee panel tilted at the pilot, on a block that folds down as the step
    S.box((-0.78, 0.05, 0.86), (0.78, 0.68, 1.14), "paint_dk", group="front", node=cn, bevel=0.01)
    nd = V(0, 0.80, -0.60).normalized()
    rd, ud, nd = basis(nd, Y)
    S.obox(V(0, 0.72, 0.99) - nd * 0.05, (rd, ud, nd), (1.16, 0.36, 0.10), "paint_dk", group="front", node=cn, bevel=0.006)
    S.plate("knee", V(0, 0.72, 0.99), nd, Y, t=0.012, node=cn)
    S.box((-0.76, 0.66, 1.14), (0.76, 0.68, 1.16), "hazard", group="front", node=cn)
    S.cyl(V(-0.78, 0.02, 1.16), V(0.78, 0.02, 1.16), 0.025, "steel", seg=16)
    # --- the screen bank: a low wide front screen and two wings, pillars and a header, on the
    # front unit that lifts like a visor; the overhead strip stays
    yb, hb, lean = 0.80, 0.50, 0.12
    c0 = screen(S, "front", V(0.40, yb, 1.06), V(-0.40, yb, 1.06), lean, fn, height=hb)
    cL = screen(S, "wing_L", V(0.77, yb, 0.74), V(0.40, yb, 1.06), lean, fn, height=hb)
    cR = screen(S, "wing_R", V(-0.40, yb, 1.06), V(-0.77, yb, 0.74), lean, fn, height=hb)
    for b, t in ((cL[0], cL[3]), (c0[0], c0[3]), (c0[1], c0[2]), (cR[1], cR[2])):
        bar(S, b - V(0, 0.04, 0), t + V(0, 0.08, 0), 0.08, 0.08, "paint", "cabin", fn, bevel=0.01)
    for a, b in ((cL[3], cL[2]), (c0[3], c0[2]), (cR[3], cR[2])):
        bar(S, a + V(0, 0.07, 0), b + V(0, 0.07, 0), 0.08, 0.07, "paint", "cabin", fn, bevel=0.01)
    for a, b in ((cL[0], cL[1]), (c0[0], c0[1]), (cR[0], cR[1])):
        bar(S, a - V(0, 0.03, 0), b - V(0, 0.03, 0), 0.07, 0.06, "paint_dk", "cabin", fn, bevel=0.008, n=V(0, 1, 0))
    S.box((-W - 0.03, RO, 0.745), (W + 0.03, RO + 0.04, ZF), "paint_dk", node=fn)
    for s in (1, -1):        # the front unit's cheeks: from the wing screens to the side walls
        x0, x1 = sorted((s * 0.75, s * 0.845))
        S.box((x0, yb - 0.04, 0.745), (x1, RO, 0.80), "paint", node=fn)
        S.box((s * 0.40 - 0.05, RO - 0.30, 0.95), (s * 0.40 + 0.05, RO, 1.08), "paint", node=fn)
        S.cyl(V(s * 0.20, RO - 0.01, 0.76), V(s * 0.20, RO - 0.01, 1.10), 0.012, "steel", "cabin", fn, seg=10)
    S.box((-0.70, 1.33, 1.00), (0.70, 1.36, 1.16), "paint_dk", node=fn)
    # overhead strip, hanging from the fixed roof: tilted down at the eye
    strip = [V(0.42, 1.38, 0.70), V(-0.42, 1.38, 0.70), V(-0.42, 1.60, 0.36), V(0.42, 1.60, 0.36)]
    flat_screen(S, "overhead", strip, None, strip=False)
    for s in (1, -1):
        S.cyl(V(s * 0.36, 1.62, 0.40), V(s * 0.36, RO, 0.40), 0.014, "steel", seg=10)
    for i in range(5):
        S.vents.append(dict(node="pod", p=V(-0.4 + 0.2 * i, 0.12, 1.10), d=V(0, -0.2, 1).normalized(), kind="mist", stage="inner_open", rate=1.0))


# ----------------------------------------------------------------------------- the harness

HC, HA, HY = 0.07, 0.76, (0.55, 2.10)       # the octagon's centre z, its apothem, the screens' foot and top
HROOF = 2.25
HARNESS_NODES = {
    "pod": (None, V(0, 0, 0)),
    "INNER_front": ("pod", V(0, HROOF - 0.04, HC + HA)),
    "INNER_side_L": ("pod", V(0.80, 1.30, HC + 0.33)),
    "INNER_side_R": ("pod", V(-0.80, 1.30, HC + 0.33)),
    "CONSOLE_front": ("pod", V(0, 0.02, HC + HA + 0.02)),
    "INNER_arm_L": ("pod", V(0.30, 1.10, -0.22)),
    "INNER_arm_R": ("pod", V(-0.30, 1.10, -0.22)),
    "CTRL_stick": ("INNER_arm_R", V(-0.30, 1.13, 0.30)),
    "CTRL_throttle": ("INNER_arm_L", V(0.30, 1.13, 0.30)),
}


def oct_vertex(phi):
    """The octagon's vertex at bearing phi (radians from +z toward +x), on the screens' foot plane."""
    R = HA / math.cos(math.pi / 8)
    return V(R * math.sin(phi), 0.0, HC + R * math.cos(phi))


def build_harness(S):
    """The standing cabin: the pilot stands in a harness against a backplate, forearms on two rests
    that end in grips, feet in stirrups, inside a tall octagon of flat screens with a ring of
    monitors overhead."""
    X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
    I3 = (X, Y, Z)
    S.vents = []
    fn, cn = "INNER_front", "CONSOLE_front"
    y0, y1 = HY
    side_of = {1: "INNER_side_L", -1: "INNER_side_R"}
    # --- facets: k = 0 front, +-1 front diagonals, +-2 sides, +-3 rear diagonals (the rear is the spine)
    tops = {}
    for k in (-3, -2, -1, 0, 1, 2, 3):
        a0, a1 = (k - 0.5) * math.pi / 4, (k + 0.5) * math.pi / 4
        p0, p1 = oct_vertex(a1), oct_vertex(a0)                  # foot left, foot right (seen from inside)
        d = (p1 - p0).normalized()
        p0, p1 = p0 + d * 0.05, p1 - d * 0.05
        node = fn if k == 0 else (side_of[1] if k == 1 else side_of[-1] if k == -1 else None)
        c = [p0 + V(0, y0, 0), p1 + V(0, y0, 0), p1 + V(0, y1, 0), p0 + V(0, y1, 0)]
        flat_screen(S, f"facet_{k:+d}".replace("+", "p").replace("-", "m"), c, node)
        # the low wall under each facet
        n = inward(c[0], c[1])
        q0, q1 = oct_vertex(a1), oct_vertex(a0)
        lo = [q0, q1, q1 + V(0, y0 - 0.03, 0), q0 + V(0, y0 - 0.03, 0)]
        quad_ob(S, lo, "paint", "cabin", node if k != 0 else cn)
        tops[k] = (c, node)
    # pillars at the vertices that do not move, rivets and cyan lines; the header and sill rings
    for j in range(-3, 5):
        phi = (j - 0.5) * math.pi / 4
        q = oct_vertex(phi)
        node = side_of[1] if j == 1 else side_of[-1] if j == 0 else None   # the front corners swing with the diagonals
        b, t = q + V(0, y0 - 0.05, 0), q + V(0, y1 + 0.05, 0)
        bar(S, b, t, 0.07, 0.08, "paint", "cabin", node, bevel=0.01)
        n_in = (V(0, 1.3, HC) - q)
        n_in.y = 0
        n_in.normalize()
        bar(S, b + n_in * 0.05, t + n_in * 0.05, 0.012, 0.012, "cyan", "cabin", node, bevel=0.0)
        bolt_row(S, b + n_in * 0.045, t + n_in * 0.045, 9, n_in, node=node)
    for k, (c, node) in tops.items():
        bar(S, c[3] + V(0, 0.08, 0), c[2] + V(0, 0.08, 0), 0.08, 0.08, "paint", "cabin", node, bevel=0.01)
        bar(S, c[0] - V(0, 0.05, 0), c[1] - V(0, 0.05, 0), 0.07, 0.06, "paint_dk", "cabin", node, bevel=0.008, n=V(0, 1, 0))
    # --- the rear: a bulkhead between the rear diagonals, the spine column and the harness on it
    qa, qb = oct_vertex(math.pi * 3.5 / 4), oct_vertex(-math.pi * 3.5 / 4)
    S.box((qb.x - 0.05, -0.04, qa.z), (qa.x + 0.05, HROOF, qa.z + 0.06), "paint")
    S.plate("wall_L", (0, 1.95, qa.z + 0.06), Z, Y, t=0.012, group="cabin")
    core_lamp(S, V(0, 0.45, qa.z + 0.06), Z, 0.08)
    S.cyl(V(0, 0.0, -0.55), V(0, HROOF, -0.55), 0.07, "steel", seg=24)
    S.box((-0.10, 0.85, -0.52), (0.10, 1.70, -0.44), "paint_dk", bevel=0.01)            # the carriage on the spine
    for x in (-0.06, 0.06):
        S.cyl(V(x, 0.10, -0.50), V(x, 0.84, -0.50), 0.025, "metal", seg=16)              # its rams
        S.cyl(V(x, 0.0, -0.50), V(x, 0.45, -0.50), 0.04, "paint", seg=16)
    S.obox(V(0, 1.30, -0.36), I3, (0.42, 0.66, 0.06), "paint", bevel=0.012)            # the backplate
    S.obox(V(0, 1.30, -0.32), I3, (0.36, 0.60, 0.04), "fabric", bevel=0.02, seg=3)
    S.obox(V(0, 1.68, -0.33), I3, (0.24, 0.14, 0.06), "fabric", bevel=0.025, seg=3)    # the head pad
    S.obox(V(0, 1.68, -0.37), I3, (0.28, 0.17, 0.03), "paint_dk", bevel=0.006)
    bolt_row(S, V(-0.19, 1.62, -0.33), V(-0.19, 0.99, -0.33), 6, Z)
    bolt_row(S, V(0.19, 1.62, -0.33), V(0.19, 0.99, -0.33), 6, Z)
    for s in (1, -1):        # shoulder straps over the top, the waist belt's clasps
        S.tube([V(s * 0.12, 1.56, -0.30), V(s * 0.13, 1.60, -0.22), V(s * 0.13, 1.52, -0.15)], 0.012, "strap")
        S.obox(V(s * 0.13, 1.50, -0.15), I3, (0.05, 0.05, 0.015), "steel", bevel=0.004)
        S.obox(V(s * 0.20, 1.00, -0.28), I3, (0.08, 0.06, 0.06), "steel", bevel=0.006)
        S.obox(V(s * 0.20, 1.00, -0.25), I3, (0.04, 0.03, 0.01), "red")
        hose(S, [V(s * 0.08, 1.60, -0.50), V(s * 0.25, 1.80, -0.55), V(s * 0.45, 2.15, -0.55)], 0.018)
    # --- the forearm rests: a padded rest on each side ending in a vertical grip; they lift clear
    for s, side in ((1, "L"), (-1, "R")):
        an = f"INNER_arm_{side}"
        piv = HARNESS_NODES[an][1]
        S.cyl(piv - V(s * 0.05, 0, 0), piv + V(s * 0.05, 0, 0), 0.04, "steel", "front", an, seg=16)
        S.obox(V(s * 0.30, 1.08, 0.04), I3, (0.10, 0.04, 0.48), "paint", "front", an, bevel=0.008)
        S.obox(V(s * 0.30, 1.115, 0.02), I3, (0.09, 0.03, 0.36), "fabric", "front", an, bevel=0.012, seg=3)
        S.obox(V(s * 0.355, 1.06, 0.06), I3, (0.006, 0.02, 0.30), "cyan", "front", an)
        bar(S, V(s * 0.30, 1.10, -0.18), V(s * 0.16, 1.18, -0.38), 0.05, 0.05, "paint_dk", "cabin", None, n=Y)
        grip(S, "CTRL_throttle" if s > 0 else "CTRL_stick", V(s * 0.30, 1.10, 0.30), Y, s)
    # --- the stirrups on a raised foot plate, the floor
    S.plate("floor", (0, 0.0, HC), Y, Z, t=0.02, group="cabin")
    S.box((-0.82, -0.06, -0.75), (0.82, -0.025, 0.90), "paint_dk")
    S.box((-0.30, 0.0, -0.18), (0.30, 0.05, 0.22), "steel", bevel=0.006)
    for s in (1, -1):
        S.obox(V(s * 0.14, 0.075, 0.04), I3, (0.12, 0.03, 0.30), "steel", bevel=0.006)
        S.obox(V(s * 0.14, 0.092, 0.04), I3, (0.10, 0.006, 0.22), "hazard")
        S.tube([V(s * 0.20, 0.09, 0.10), V(s * 0.14, 0.17, 0.10), V(s * 0.08, 0.09, 0.10)], 0.012, "strap")
        S.cyl(V(s * 0.14, 0.05, -0.12), V(s * 0.14, 0.12, -0.10), 0.03, "metal", seg=12)
    # --- side pods under the side facets: console panels on top, wall panels on their inner faces
    for s, side in ((1, "L"), (-1, "R")):
        x0, x1 = sorted((s * 0.46, s * 0.77))
        S.box((x0, 0.0, -0.37), (x1, 0.48, 0.47), "paint", bevel=0.004)
        S.plate(f"console_{side}", (s * 0.615, 0.492, 0.05), Y, Z, t=0.012)
        S.plate(f"breakers_{side}", (s * 0.46, 0.24, 0.05), -s * X, Y, t=0.01, group="cabin")
    # --- the low front console: the knee panel tilted up at the pilot, folding down as the step
    nk = V(0, 0.75, -0.66).normalized()
    rk, uk, nk = basis(nk, Y)
    ck = V(0, 0.46, HC + HA - 0.20)
    S.box((-0.34, 0.02, HC + HA - 0.30), (0.34, 0.40, HC + HA - 0.02), "paint_dk", group="front", node=cn, bevel=0.008)
    S.obox(ck - nk * 0.06, (rk, uk, nk), (1.14, 0.34, 0.10), "paint_dk", group="front", node=cn, bevel=0.006)
    S.plate("knee", ck, nk, Y, t=0.012, node=cn)
    S.cyl(V(-0.34, 0.02, HC + HA + 0.02), V(0.34, 0.02, HC + HA + 0.02), 0.025, "steel", seg=16)
    # --- the roof and its ring of monitors
    S.box((-0.85, HROOF, -0.75), (0.85, HROOF + 0.04, HC + HA + 0.05), "paint_dk")
    ring = []
    for i, deg in enumerate((-75, -45, -15, 15, 45, 75)):
        a = math.radians(deg)
        p = V(0.58 * math.sin(a), 2.00, 0.0 + 0.58 * math.cos(a))
        ring.append(p)
        monitor(S, f"OVH_{i + 1}", ("mfd_right", "mfd_center", "mfd_left")[i % 3], p, EYE, 0.20, 0.145, None, rod_to=V(p.x, HROOF, p.z))
    S.tube([V(0.62 * math.sin(math.radians(d)), 2.16, 0.62 * math.cos(math.radians(d))) for d in range(-90, 91, 15)], 0.018, "steel")
    S.box((-0.25, HROOF - 0.03, -0.30), (0.25, HROOF, -0.24), "bezel", bevel=0.004)
    S.box((-0.22, HROOF - 0.034, -0.29), (0.22, HROOF - 0.03, -0.25), "white_lit")
    for i in range(5):
        S.vents.append(dict(node="pod", p=V(-0.3 + 0.15 * i, 0.10, HC + HA), d=V(0, -0.2, 1).normalized(), kind="mist",
                            stage="inner_open", rate=1.0))


# ----------------------------------------------------------------------------- the variants
# Each variant: its builder, nodes, the pilot's eye, the cabin middle, the envelope it promises the
# chest, sockets, the inner opening (each moving node's motion within a 3 s opening, after the
# mech's chest armour has opened) and its real-time lamps (name, kind, at, aim, watts, colour,
# radius, cone in degrees).

WARM, COOL = (1.0, 0.86, 0.68), (0.62, 0.78, 1.0)
VARIANTS = {
    "wall": dict(
        build="build_wall", nodes=WALL_NODES, eye=V(0.0, 1.20, 0.0), mid=V(0.0, 1.2, 0.1),
        envelope=((-1.05, 1.05), (0.0, 2.1), (-0.8, 1.25)), doorway=(0.9, 1.6),
        sockets={"SEAT_pilot": V(0, 0.45, -0.02), "HATCH_entry": V(0, 0.02, 1.02)},
        motions={"INNER_front": dict(axis="x", value=-1.65, t0=0.0, d=1.6),
                 "INNER_side_L": dict(axis="y", value=0.8, t0=0.5, d=1.2),
                 "INNER_side_R": dict(axis="y", value=-0.8, t0=0.5, d=1.2),
                 "INNER_sill": dict(axis="x", value=1.45, t0=1.6, d=1.0)},
        lights=[("LIGHT_screen", "SPOT", V(0, 1.30, 0.90), V(0, 0.9, -0.2), 30.0, COOL, 0.4, 120),
                ("LIGHT_glare", "SPOT", V(0, 0.99, 0.62), V(0, 0.4, 0.66), 6.0, WARM, 0.1, 110),
                ("LIGHT_sill_L", "SPOT", V(0.70, 1.00, 0.10), V(0.48, 0.55, 0.10), 4.0, WARM, 0.05, 100),
                ("LIGHT_sill_R", "SPOT", V(-0.70, 1.00, 0.10), V(-0.48, 0.55, 0.10), 4.0, WARM, 0.05, 100),
                ("LIGHT_rear", "POINT", V(0, 1.30, -0.50), None, 5.0, (1.0, 0.9, 0.78), 0.05, None),
                ("LIGHT_footwell", "POINT", V(0, 0.25, 0.80), None, 1.2, (1.0, 0.18, 0.12), 0.03, None)],
        front_screen="SCREEN_wall_front", front_screen_c=V(0, 1.32, 1.05)),
    "jockey": dict(
        build="build_jockey", nodes=JOCKEY_NODES, eye=V(0.0, 1.10, 0.30), mid=V(0.0, 1.0, 0.3),
        envelope=((-0.85, 0.85), (0.0, 1.75), (-1.0, 1.3)), doorway=(0.9, 1.3),
        sockets={"SEAT_pilot": V(0, 0.72, -0.38), "HATCH_entry": V(0, 0.02, 1.16)},
        motions={"INNER_front": dict(axis="x", value=-1.5, t0=0.0, d=1.4),
                 "INNER_yoke": dict(slide="y", value=0.45, t0=0.9, d=1.2),
                 "CONSOLE_front": dict(axis="x", value=1.35, t0=1.5, d=1.2)},
        lights=[("LIGHT_screen", "SPOT", V(0, 1.25, 1.00), V(0, 0.8, -0.3), 18.0, COOL, 0.3, 120),
                ("LIGHT_overhead", "SPOT", V(0, 1.62, 0.20), V(0, 0.6, 0.0), 5.0, WARM, 0.1, 110),
                ("LIGHT_sill_L", "SPOT", V(0.78, 1.10, -0.10), V(0.55, 0.55, -0.10), 3.0, WARM, 0.05, 100),
                ("LIGHT_sill_R", "SPOT", V(-0.78, 1.10, -0.10), V(-0.55, 0.55, -0.10), 3.0, WARM, 0.05, 100),
                ("LIGHT_rear", "POINT", V(0, 1.45, -0.80), None, 3.5, (1.0, 0.9, 0.78), 0.05, None),
                ("LIGHT_footwell", "POINT", V(0, 0.20, 0.55), None, 1.0, (1.0, 0.18, 0.12), 0.03, None)],
        front_screen="SCREEN_wall_front", front_screen_c=V(0, 1.05, 1.0)),
    "harness": dict(
        build="build_harness", nodes=HARNESS_NODES, eye=V(0.0, 1.65, 0.0), mid=V(0.0, 1.4, 0.05),
        envelope=((-1.05, 1.05), (0.0, 2.4), (-0.75, 0.95)), doorway=(0.9, 1.6),
        sockets={"STAND_pilot": V(0, 0.09, 0.04), "HATCH_entry": V(0, 0.02, HC + HA + 0.05)},
        motions={"INNER_front": dict(axis="x", value=-1.5, t0=0.0, d=1.5),
                 "INNER_side_L": dict(axis="y", value=0.9, t0=0.4, d=1.2),
                 "INNER_side_R": dict(axis="y", value=-0.9, t0=0.4, d=1.2),
                 "INNER_arm_L": dict(axis="x", value=-1.3, t0=1.0, d=1.0),
                 "INNER_arm_R": dict(axis="x", value=-1.3, t0=1.0, d=1.0),
                 "CONSOLE_front": dict(axis="x", value=1.3, t0=1.6, d=1.2)},
        lights=[("LIGHT_screen", "SPOT", V(0, 1.70, 0.80), V(0, 1.2, -0.3), 22.0, COOL, 0.4, 120),
                ("LIGHT_ring", "POINT", V(0, 2.12, 0.0), None, 6.0, WARM, 0.1, None),
                ("LIGHT_sill_L", "SPOT", V(0.70, 0.62, 0.0), V(0.30, 1.2, 0.0), 3.0, WARM, 0.05, 100),
                ("LIGHT_sill_R", "SPOT", V(-0.70, 0.62, 0.0), V(-0.30, 1.2, 0.0), 3.0, WARM, 0.05, 100),
                ("LIGHT_rear", "POINT", V(0, 1.90, -0.62), None, 3.0, (1.0, 0.9, 0.78), 0.05, None),
                ("LIGHT_footwell", "POINT", V(0, 0.15, 0.30), None, 1.0, (1.0, 0.18, 0.12), 0.03, None)],
        front_screen="SCREEN_wall_facet_p0", front_screen_c=V(0, 1.32, HC + HA)),
}


# ----------------------------------------------------------------------------- lights

def lights(spec):
    """The cabin's real-time lamps, exported as glTF punctual lights (Bevy and the review page light
    the cockpit with them). The world is dark: the cabin is sealed. Lights are not baked; the bake
    keeps only the maps and the contact occlusion."""
    sc = bpy.context.scene
    w = bpy.data.worlds.new("Dark")
    w.use_nodes = True
    w.node_tree.nodes["Background"].inputs["Color"].default_value = (0.005, 0.005, 0.006, 1)
    w.light_settings.distance = 0.08
    sc.world = w
    out = []
    for name, kind, loc, aim, watts, colour, radius, cone in spec:
        ld = bpy.data.lights.new(name, kind)
        ld.energy = watts
        ld.color = colour
        ld.shadow_soft_size = radius
        if kind == "SPOT":
            ld.spot_size = math.radians(cone)
            ld.spot_blend = 0.6
        ob = bpy.data.objects.new(name, ld)
        ob.location = B(loc)
        if aim is not None:
            d = (B(aim) - B(loc)).normalized()
            ob.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
        sc.collection.objects.link(ob)
        out.append((ob, loc))
    return out


# ----------------------------------------------------------------------------- finishing

def realize(ob):
    if not ob.modifiers:
        return
    dg = bpy.context.evaluated_depsgraph_get()
    me = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
    old = ob.data
    ob.modifiers.clear()
    ob.data = me
    if old.users == 0:
        bpy.data.meshes.remove(old)


def join(obs, name, origin=None):
    me = bpy.data.meshes.new(name)
    t = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(t)
    if origin is not None:
        t.location = B(origin)
    with bpy.context.temp_override(active_object=t, selected_editable_objects=[t] + obs, selected_objects=[t] + obs):
        bpy.ops.object.join()
    return t


UV_ANGLE = 60      # degrees: smart project splits islands at sharper creases than this
UV_SHAPE = "AABB"  # pack shape; CONCAVE measured no better on this set
UV_LONG = 0.25     # thin islands longer than this (in pre-pack UV units) are shrunk before packing


def lightmap_uvs(obs):
    """A 'light' UV layer on every object of an atlas: smart projected, then packed together."""
    for ob in obs:
        uv = ob.data.uv_layers.get("light") or ob.data.uv_layers.new(name="light")
        ob.data.uv_layers.active = uv
    bpy.ops.object.select_all(action="DESELECT")
    for ob in obs:
        ob.select_set(True)
    bpy.context.view_layer.objects.active = obs[0]
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(UV_ANGLE), island_margin=0.0015, area_weight=0.0, scale_to_bounds=False)
    # Long hair-thin islands (trim strips a few millimetres wide and 1.5 m long) cap how far the packer
    # can scale everything else up, since none may exceed the unit square. Shrink just those: they are
    # baked into this UV, so they only lose texels along a length nobody inspects.
    bpy.ops.object.mode_set(mode="OBJECT")
    from bpy_extras.bmesh_utils import bmesh_linked_uv_islands
    shrunk = 0
    for ob in obs:
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        lay = bm.loops.layers.uv["light"]
        for isl in bmesh_linked_uv_islands(bm, lay):
            loops = [l for f in isl for l in f.loops]
            uvs = np.array([l[lay].uv[:] for l in loops])
            lo, hi = uvs.min(0), uvs.max(0)
            ext = hi - lo
            if ext.max() > UV_LONG and ext.min() < ext.max() * 0.15:
                c, k = (lo + hi) / 2, UV_LONG / ext.max()
                for l in loops:
                    l[lay].uv = Vector(c + (np.array(l[lay].uv[:]) - c) * k)
                shrunk += 1
        bm.to_mesh(ob.data)
        bm.free()
    print(f"UV shrank {shrunk} thin islands", flush=True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.uv.select_all(action="SELECT")
    bpy.ops.uv.pack_islands(rotate=True, margin=0.0015, shape_method=UV_SHAPE)
    bpy.ops.object.mode_set(mode="OBJECT")
    u = np.concatenate([np.array([d.uv[:] for d in ob.data.uv_layers["light"].data]) for ob in obs])
    print(f"UV PACK {obs[0].name}..: u {u[:, 0].min():.3f}-{u[:, 0].max():.3f} v {u[:, 1].min():.3f}-{u[:, 1].max():.3f}", flush=True)


def bake_target(img):
    for m in bpy.data.materials:
        if not m.use_nodes:
            continue
        n = m.node_tree.nodes.get("BAKE_TARGET")
        if n is None:
            n = m.node_tree.nodes.new("ShaderNodeTexImage")
            n.name = "BAKE_TARGET"
        n.image = img
        m.node_tree.nodes.active = n


def pixels(img):
    w, h = img.size
    a = np.empty(w * h * 4, np.float32)
    img.pixels.foreach_get(a)
    return a.reshape(h, w, 4)


def denoise(rgb):
    """OIDN through the compositor (Blender has no denoiser for bakes): the baked image in, the
    denoised image out. The atlas margins keep island edges from bleeding."""
    sc = bpy.context.scene
    h, w = rgb.shape[:2]
    src = bpy.data.images.new("dn_src", w, h, float_buffer=True, alpha=False)
    a = np.ones((h, w, 4), np.float32)
    a[:, :, :3] = rgb
    src.pixels.foreach_set(a.ravel())
    engine, res = sc.render.engine, (sc.render.resolution_x, sc.render.resolution_y)
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.render.resolution_x, sc.render.resolution_y = w, h
    hidden = [o for o in sc.objects if not o.hide_render]
    for o in hidden:
        o.hide_render = True
    sc.use_nodes = True
    sc.render.use_compositing = True
    nt = sc.node_tree
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    ni = nt.nodes.new("CompositorNodeImage")
    ni.image = src
    dn = nt.nodes.new("CompositorNodeDenoise")
    dn.prefilter = "ACCURATE"
    comp = nt.nodes.new("CompositorNodeComposite")
    nt.links.new(ni.outputs[0], dn.inputs["Image"])
    nt.links.new(dn.outputs[0], comp.inputs[0])
    sc.render.image_settings.file_format = "OPEN_EXR"
    sc.render.image_settings.color_depth = "32"
    path = os.path.join(RAW, "denoise.exr")
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)
    out = bpy.data.images.load(path, check_existing=False)
    r = pixels(out)[:, :, :3].copy()
    bpy.data.images.remove(out)
    bpy.data.images.remove(src)
    for o in hidden:
        o.hide_render = False
    sc.render.engine = engine
    sc.render.resolution_x, sc.render.resolution_y = res
    sc.use_nodes = False
    return r


def save_jpeg(path, rgb, quality=90):
    h, w = rgb.shape[:2]
    img = bpy.data.images.new(os.path.basename(path), w, h, alpha=False)
    img.colorspace_settings.name = "Non-Color"
    a = np.ones((h, w, 4), np.float32)
    a[:, :, :3] = np.clip(rgb, 0, 1)
    img.pixels.foreach_set(a.ravel())
    img.filepath_raw = path
    img.file_format = "JPEG"
    bpy.context.scene.render.image_settings.quality = quality
    img.save(quality=quality)
    bpy.data.images.remove(img)


def export_material(name, image_path, alpha=False):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    bsdf = N["Principled BSDF"]
    uv = N.new("ShaderNodeUVMap")
    uv.uv_map = "UVMap"
    t = N.new("ShaderNodeTexImage")
    t.image = bpy.data.images.load(image_path, check_existing=True)
    L.new(uv.outputs[0], t.inputs[0])
    L.new(t.outputs[0], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 1.0
    bsdf.inputs["Metallic"].default_value = 0.0
    if alpha:
        L.new(t.outputs["Alpha"], bsdf.inputs["Alpha"])
        mat.blend_method = "BLEND"
    return mat


def only_uv(ob, keep):
    me = ob.data
    for name in [l.name for l in me.uv_layers if l.name != keep and not l.name.startswith(".")]:
        me.uv_layers.remove(me.uv_layers[name])
    me.uv_layers[keep].name = "UVMap"


def patch_unlit(path):
    """Mark the screen materials KHR_materials_unlit in the exported glTF: the game draws them live."""
    js = json.load(open(path))
    for m in js.get("materials", []):
        if m["name"].startswith(("SCREEN_", "HUD_")):
            m.setdefault("extensions", {})["KHR_materials_unlit"] = {}
    used = js.setdefault("extensionsUsed", [])
    if "KHR_materials_unlit" not in used:
        used.append("KHR_materials_unlit")
    with open(path, "w") as f:
        json.dump(js, f, indent=1)


def tris(ob):
    return sum(len(p.vertices) - 2 for p in ob.data.polygons)


# ----------------------------------------------------------------------------- main

def ext_material(role, cache={}):
    """The pod's outside as a lit PBR material: the mech's plating maps (colour, normal, ORM)."""
    if role in cache:
        return cache[role]
    tex = {"ext_armor": "armorL", "ext_frame": "frame", "hazard": "hazard"}.get(role)
    spec = K.ROLES[role]
    mat = bpy.data.materials.new(f"EXT_{role}")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = spec["r"]
    bsdf.inputs["Metallic"].default_value = spec["m"]
    bsdf.inputs["Base Color"].default_value = list(K.srgb(spec["c"])) + [1.0]
    if spec.get("emit"):
        bsdf.inputs["Emission Color"].default_value = list(K.srgb(spec["emit"])) + [1.0]
        bsdf.inputs["Emission Strength"].default_value = 3.0
    if tex:
        mech = os.path.join(ROOT, "assets", "textures", "mech")
        load = lambda n: bpy.data.images.load(os.path.join(mech, f"{n}.png"), check_existing=True)  # noqa: E731
        nt.links.new(PBR.image(nt, load(tex), colour=True).outputs[0], bsdf.inputs["Base Color"])
        if os.path.exists(os.path.join(mech, f"{tex}_n.png")):
            PBR.normal(nt, bsdf, load(f"{tex}_n"))
        PBR.orm(nt, bsdf, load(f"{tex}_orm"))
    elif role not in ("rubber",) and not spec.get("emit"):
        # flat painted and bare parts still carry the plating's scratches in their relief
        mech = os.path.join(ROOT, "assets", "textures", "mech")
        PBR.normal(nt, bsdf, bpy.data.images.load(os.path.join(mech, "wear_n.png"), check_existing=True), strength=0.6)
    cache[role] = mat
    return mat


def atlas_material(g, maps, emit_k):
    """An atlas's baked maps as one lit PBR material on its UV map."""
    mat = bpy.data.materials.new(f"COCKPIT_{g}")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    load = lambda k: bpy.data.images.load(maps[k], check_existing=True)  # noqa: E731
    nt.links.new(PBR.image(nt, load("albedo"), colour=True).outputs[0], bsdf.inputs["Base Color"])
    PBR.normal(nt, bsdf, load("normal"))
    PBR.orm(nt, bsdf, load("orm"))
    if emit_k > 0:
        nt.links.new(PBR.image(nt, load("emissive"), colour=True).outputs[0], bsdf.inputs["Emission Color"])
        bsdf.inputs["Emission Strength"].default_value = emit_k
    return mat


class swap_surface:
    """Bake a material value through emission: each material's output shows the named node's value
    as emission (or `default` where the material has no such node), and is put back after."""

    def __init__(self, mats, node_name, default):
        self.mats, self.node_name, self.default, self.saved = mats, node_name, default, []

    def __enter__(self):
        for m in self.mats:
            nt = m.node_tree
            out = next((n for n in nt.nodes if n.type == "OUTPUT_MATERIAL" and n.is_active_output), None)
            if out is None:
                continue
            old = out.inputs["Surface"].links[0].from_socket if out.inputs["Surface"].links else None
            em = nt.nodes.new("ShaderNodeEmission")
            src = nt.nodes.get(self.node_name)
            if src is not None:
                nt.links.new(src.outputs[0], em.inputs["Color"])
            else:
                em.inputs["Color"].default_value = (self.default,) * 3 + (1.0,)
            nt.links.new(em.outputs[0], out.inputs["Surface"])
            self.saved.append((nt, out, old, em))
        return self

    def __exit__(self, *exc):
        for nt, out, old, em in self.saved:
            if old is not None:
                nt.links.new(old, out.inputs["Surface"])
            nt.nodes.remove(em)


def cube_uvs(ob, size=1.2):
    bpy.ops.object.select_all(action="DESELECT")
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
    uv = ob.data.uv_layers.get("UVMap") or ob.data.uv_layers.new(name="UVMap")
    ob.data.uv_layers.active = uv
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.cube_project(cube_size=size)
    bpy.ops.object.mode_set(mode="OBJECT")


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", choices=sorted(VARIANTS), default="wall")
    ap.add_argument("--size", type=int, default=4096)
    ap.add_argument("--samples", type=int, default=16, help="contact occlusion samples (denoised after)")
    ap.add_argument("--map-samples", type=int, default=4, help="samples for the colour, normal, roughness, metal and emission passes")
    ap.add_argument("--no-denoise", action="store_true")
    ap.add_argument("--no-bake", action="store_true", help="geometry only: skip the bake, keep the last atlases")
    a = ap.parse_args(argv)
    t0 = time.time()
    global EYE, MID, MODEL, MANIFEST
    VAR = VARIANTS[a.variant]
    EYE, MID = VAR["eye"], VAR["mid"]
    NODES.clear()
    NODES.update(VAR["nodes"])
    MODEL = os.path.join(ROOT, "assets", "models", f"cockpit_{a.variant}", f"cockpit_{a.variant}.gltf")
    MANIFEST = os.path.join(ROOT, "assets", "models", f"cockpit_{a.variant}.json")
    os.makedirs(RAW, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    mats = K.make_materials()
    S = Set(mats)
    globals()[VAR["build"]](S)
    rig = lights(VAR["lights"])

    # realize, then join: one mesh per node and group, its origin at the node's pivot
    for p in S.parts:
        realize(p["ob"])
    by = {}
    for p in S.parts:
        by.setdefault((p["group"], p["node"] or "pod"), []).append(p["ob"])

    def pivot(node):
        return NODES[node][1] if NODES[node][1] is not None else pivot(NODES[node][0])

    meshes = {}
    for (group, node), obs in by.items():
        name = f"{node}_{group}" if group != "screen" else f"{node}_screen"
        meshes[name] = (join(obs, name, pivot(node)), group, node)
    atlases = {g: [ob for ob, gg, _ in meshes.values() if gg == g] for g in ("front", "cabin")}
    for g, obs in atlases.items():
        lightmap_uvs(obs)
    print(f"BUILT in {time.time() - t0:.0f} s: " + ", ".join(f"{k} {tris(v[0])}" for k, v in sorted(meshes.items())), flush=True)

    tex_dir = os.path.join(RAW, f"pbr_{a.variant}")
    os.makedirs(tex_dir, exist_ok=True)
    maps = {g: {k: os.path.join(tex_dir, f"cockpit_{a.variant}_{g}_{k}.jpg") for k in ("albedo", "normal", "orm", "emissive")}
            for g in atlases}
    emit_k = {}
    scale_file = os.path.join(tex_dir, "emissive_scale.json")
    if os.path.exists(scale_file):
        emit_k = json.load(open(scale_file))
    if not a.no_bake:
        sc.render.engine = "CYCLES"
        cy = sc.cycles
        cy.device = "CPU"
        cy.use_denoising = False
        cy.seed = 0
        bk = sc.render.bake
        bk.margin = 16
        bk.margin_type = "EXTEND"
        for g, obs in atlases.items():
            bpy.ops.object.select_all(action="DESELECT")
            for ob in obs:
                ob.select_set(True)
                ob.data.uv_layers.active = ob.data.uv_layers["light"]
                ob.data.uv_layers["light"].active_render = True     # the tangent frame the normal map is baked in
            bpy.context.view_layer.objects.active = obs[0]
            mats = list({s.material for ob in obs for s in ob.material_slots if s.material and s.material.use_nodes})

            def bake(kind, samples, **kw):
                t = time.time()
                img = bpy.data.images.new(f"bake_{g}_{kind}", a.size, a.size, float_buffer=True, alpha=False)
                bake_target(img)
                cy.samples = samples
                bpy.ops.object.bake(type=kind, margin=16, use_clear=True, **kw)
                px = pixels(img)[:, :, :3].copy()
                bpy.data.images.remove(img)
                print(f"BAKED {g} {kind} in {time.time() - t:.0f} s", flush=True)
                return px

            albedo = bake("DIFFUSE", a.map_samples, pass_filter={"COLOR"})
            normal = bake("NORMAL", a.map_samples, normal_space="TANGENT")
            rough = bake("ROUGHNESS", a.map_samples)[:, :, 0]
            emit = bake("EMIT", a.map_samples)
            with swap_surface(mats, "METAL_SRC", 0.0):
                metal = bake("EMIT", a.map_samples)[:, :, 0]
            with swap_surface(mats, "AO_SRC", 1.0):
                ao_src = bake("EMIT", a.map_samples)[:, :, 0]
            sc.world.light_settings.distance = 0.08
            ao = bake("AO", a.samples)
            if not a.no_denoise:
                ao = denoise(ao)
            occl = np.clip(ao[:, :, 0], 0, 1) * np.clip(ao_src, 0, 1)
            k = float(max(1.0, np.percentile(emit.max(axis=2), 99.95)))
            emit_k[g] = round(k, 3)
            save_jpeg(maps[g]["albedo"], K.to_srgb(np.clip(albedo, 0, 1)), 92)
            save_jpeg(maps[g]["normal"], normal, 95)
            save_jpeg(maps[g]["orm"], np.dstack([occl, rough, metal]), 92)
            save_jpeg(maps[g]["emissive"], K.to_srgb(np.clip(emit / k, 0, 1)), 90)
            print(f"MAPS {g}: occlusion mean {occl.mean():.2f}, roughness {rough.mean():.2f}, metal {metal.mean():.2f}, "
                  f"emission scale {k:.2f}", flush=True)
        json.dump(emit_k, open(scale_file, "w"))

    # export materials: the baked atlases lit PBR on their light UV, the screens unlit, the outside lit PBR
    disp = S.disp
    for name, (ob, group, node) in meshes.items():
        if group in ("front", "cabin"):
            only_uv(ob, "light")
            ob.data.materials.clear()
            ob.data.materials.append(bpy.data.materials.get(f"COCKPIT_{group}") or atlas_material(group, maps[group], emit_k.get(group, 1.0)))
        elif group == "screen":
            only_uv(ob, "panel")
            ob.data.materials.clear()
            mname = f"SCREEN_{node.split('_', 1)[1].lower()}" if node.startswith("SCREEN") else f"SCREEN_{node.lower()}"
            old = bpy.data.materials.get(mname)
            if old is not None:                          # the bake's emission material had the name
                old.name = "bake_" + mname
            ob.data.materials.append(export_material(mname, os.path.join(DISPLAYS, f"{disp[node]}.png")))
        elif group == "allview":
            for l in [l.name for l in ob.data.uv_layers if not l.name.startswith(".")]:
                ob.data.uv_layers.remove(ob.data.uv_layers[l])
            m = bpy.data.materials.get("SCREEN_allview") or bpy.data.materials.new("SCREEN_allview")
            m.use_nodes = True
            m.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.01, 0.012, 0.014, 1)
            ob.data.materials.clear()
            ob.data.materials.append(m)
        elif group in ("ext", "shell"):
            for l in [l.name for l in ob.data.uv_layers if not l.name.startswith(".")]:
                ob.data.uv_layers.remove(ob.data.uv_layers[l])
            cube_uvs(ob)
            for i, slot in enumerate(ob.material_slots):
                role = slot.material.name.split(".")[0] if slot.material else "ext_dark"
                ob.material_slots[i].material = ext_material(role if role in K.ROLES else "ext_dark")

    # the envelope check: every vertex of every mesh, closed and open (glTF frame, metres)
    def motion_matrix(node):
        m = Matrix.Identity(4)
        while node is not None:
            mo = VAR["motions"].get(node)
            if mo:
                pv = pivot(node)
                if "axis" in mo:
                    R = Matrix.Rotation(mo["value"], 4, mo["axis"].upper())
                    m = Matrix.Translation(pv) @ R @ Matrix.Translation(-pv) @ m
                else:
                    off = V(0, 0, 0)
                    setattr(off, mo["slide"], mo["value"])
                    m = Matrix.Translation(off) @ m
            node = NODES[node][0]
        return m

    def bounds(open_):
        lo, hi = V(1e9, 1e9, 1e9), V(-1e9, -1e9, -1e9)
        for ob, group, node in meshes.values():
            M = motion_matrix(node) if open_ else Matrix.Identity(4)
            for v in ob.data.vertices:
                w = ob.matrix_world @ v.co
                g = M @ Vector((w.x, w.z, -w.y))
                for i in range(3):
                    lo[i], hi[i] = min(lo[i], g[i]), max(hi[i], g[i])
        return [[round(lo[i], 3), round(hi[i], 3)] for i in range(3)]

    bpy.context.view_layer.update()
    closed_b, open_b = bounds(False), bounds(True)
    env = VAR["envelope"]
    over = [("xyz"[i], closed_b[i], env[i]) for i in range(3) if closed_b[i][0] < env[i][0] - 1e-3 or closed_b[i][1] > env[i][1] + 1e-3]
    print(f"BOUNDS closed {closed_b} open {open_b} envelope {env}" + (f" OVER {over}" if over else ""), flush=True)

    # the node tree: an empty at each node's pivot, the meshes under their nodes
    empties = {}

    def empty(node):
        if node in empties:
            return empties[node]
        e = bpy.data.objects.new(node if node != "pod" else "cockpit", None)
        sc.collection.objects.link(e)
        par = NODES[node][0]
        if par is not None:
            e.parent = empty(par)
            e.location = B(pivot(node) - pivot(par))
        else:
            e.location = B(pivot(node))
        empties[node] = e
        return e

    for n in NODES:
        empty(n)
    for name, (ob, group, node) in meshes.items():
        ob.parent = empty(node)
        ob.location = (0.0, 0.0, 0.0)
    for i, v in enumerate(S.vents):
        e = bpy.data.objects.new(f"VENT_{v['stage']}_{i:02d}", None)
        sc.collection.objects.link(e)
        e.parent = empty(v["node"])
        e.location = B(v["p"] - pivot(v["node"]))
        e["kind"], e["stage"], e["rate"] = v["kind"], v["stage"], v["rate"]
        e["dir"] = [round(c, 4) for c in v["d"]]
    root = empties["pod"]
    for name, p in dict({"EYE_pilot": EYE}, **VAR["sockets"]).items():
        e = bpy.data.objects.new(name, None)
        e.location = B(p)
        sc.collection.objects.link(e)
        e.parent = root
    for ob, loc in rig:                                   # the light rig rides the pod
        ob.parent = root
        ob.location = B(loc)
    root["eye_m"] = list(EYE)
    bpy.context.view_layer.update()
    out_dir = os.path.dirname(MODEL)
    shutil.rmtree(out_dir, ignore_errors=True)          # no stale textures beside a new export
    os.makedirs(out_dir, exist_ok=True)
    for ob, _, _ in meshes.values():                     # triangles, so the exporter can write tangents
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        bm.to_mesh(ob.data)
        bm.free()
    bpy.ops.export_scene.gltf(filepath=MODEL, export_format="GLTF_SEPARATE", export_texture_dir="textures",
                              export_extras=True, export_yup=True, export_apply=False, export_image_format="AUTO",
                              export_texcoords=True, export_normals=True, export_tangents=True, export_lights=True,
                              export_cameras=False, use_selection=False)
    patch_unlit(MODEL)
    total = sum(tris(ob) for ob, _, _ in meshes.values())
    front_dir = [round(c, 3) for c in (VAR["front_screen_c"] - EYE).normalized()]   # where the HUD screen is
    man = {
        "variant": a.variant,
        "model": f"assets/models/cockpit_{a.variant}/cockpit_{a.variant}.gltf",
        "frame": "glTF: +y up, the pilot faces +z, +x is the pilot's left; metres; origin on the tub floor under the eye",
        "eye_m": [round(c, 4) for c in EYE],
        "envelope_m": {"x": list(env[0]), "y": list(env[1]), "z": list(env[2])},
        "bounds_closed_m": dict(zip("xyz", closed_b)),
        "bounds_open_m": dict(zip("xyz", open_b)),
        "doorway_m": {"width": VAR["doorway"][0], "height": VAR["doorway"][1], "through": "the front face (+z)"},
        "sockets": {k: [round(c, 4) for c in v] for k, v in dict({"EYE_pilot": EYE}, **VAR["sockets"]).items()},
        "motions": {n: dict(m, pivot_m=[round(c, 4) for c in pivot(n)]) for n, m in VAR["motions"].items()},
        "inner_opening_s": 3.0,
        "front_screen_dir": front_dir,
        "allview_material": "SCREEN_allview",
        "nodes": {n: {"parent": NODES[n][0], "pivot_m": [round(c, 4) for c in pivot(n)]} for n in NODES},
        "meshes": {k: {"tris": tris(v[0]), "group": v[1], "node": v[2]} for k, v in sorted(meshes.items())},
        "vents": len(S.vents),
        "lights": len(rig),
        "tris": total, "tri_budget": TRI_BUDGET,
        "draw_calls": sum(len(ob.data.materials) for ob, _, _ in meshes.values()),
        "atlases": {g: {"maps": sorted(maps[g]), "px": a.size, "emissive_strength": emit_k.get(g, 1.0)} for g in atlases},
        "bake": {"ao_samples": a.samples, "denoised": not a.no_denoise,
                 "maps": "base colour, tangent normal, ORM (R occlusion, G roughness, B metal), emission; light is real time"},
        "panels": sorted(CP.PANELS),
        "bytes": sum(os.path.getsize(os.path.join(dp, f)) for dp, _, fs in os.walk(out_dir) for f in fs),
    }
    with open(MANIFEST, "w") as f:
        json.dump(man, f, indent=2)
        f.write("\n")
    if total > TRI_BUDGET:
        raise SystemExit(f"cockpit is {total} triangles, over the budget of {TRI_BUDGET}")
    print(f"EXPORTED {MODEL}: {total} triangles, {man['bytes'] / 1e6:.1f} MB in {time.time() - t0:.0f} s", flush=True)


main()
