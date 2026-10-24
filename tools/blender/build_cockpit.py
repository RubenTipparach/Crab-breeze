"""Crab Breeze: the Breeze cockpit set, built to the pilot's eye, its light and occlusion baked.

  /opt/blender/blender --background --factory-startup --python tools/blender/build_cockpit.py -- \
      [--size 4096] [--samples 96] [--no-denoise]

Run tools/blender/bake_cockpit_panels.py and tools/textures/gen_cockpit_displays.py first: this
script reads their maps. It writes assets/models/cockpit_breeze.glb, its baked atlases
(assets/textures/cockpit/baked_{front,cabin}.jpg) and a manifest (cockpit_breeze.json).

What it does (the owner, 2026-10-24: "Design the mech cockpit. Use detailed textures. Use the
texturing skill we had on star crew bridge sim. Baked lighting with AO. Beautiful panels, warning
labels, buttons, circuitry"):

  1. Builds the set in metres, glTF frame (+y up, the pilot faces +z, +x is the pilot's left),
     origin on the tub floor under the eye: the tub, the dash with three MFD bezels and the HUD,
     the glare shield, the knee panel, two side consoles, the seat with its harness and eject
     handle, the stick, throttle and twist grip, pedals, cable runs, the bubble canopy and its
     bows, and the mech's nose and cheeks seen through the glass.
  2. Lays the panel maps (cockpit_panels.py, modelled and baked the Star Crew way) on the console
     faces as real plates, with their albedo, occlusion, normal map and backlit buttons.
  3. Lights the cabin: the sky through the canopy, floodlights under the glare shield and the
     canopy sill, the dome lamp, a red footwell lamp, and the glow of the screens and buttons.
  4. Bakes that light, with occlusion, in Cycles into two atlases (front: the dash, consoles and
     controls; cabin: everything else), denoises them, and multiplies a contact occlusion pass in.
  5. Exports a glb whose baked surfaces are unlit (KHR_materials_unlit), the screens and HUD as
     their own unlit nodes (SCREEN_*, HUD_combiner) for the game to draw live, the canopy glass
     as a transparent node, the controls as CTRL_* nodes with their pivots at the origin, and the
     sockets EYE_pilot and SEAT_pilot (docs/design/mech-rig.md).
"""

import argparse
import json
import math
import os
import struct
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

ROOT = K.ROOT
PANELS = os.path.join(ROOT, "assets", "textures", "cockpit", "panels")
DISPLAYS = os.path.join(ROOT, "assets", "textures", "cockpit", "displays")
BAKED = os.path.join(ROOT, "assets", "textures", "cockpit")
MODEL = os.path.join(ROOT, "assets", "models", "cockpit_breeze.glb")
RAW = os.path.join(ROOT, "output", "cockpit_raw")

G2B = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # glTF frame to Blender
EYE = Vector((0.0, 1.20, 0.0))
TRI_BUDGET = 200_000


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
        self.parts = []          # dicts: ob, group (front, cabin, screen, hud, glass), node
        self.n = 0

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
        c = Vector(c)
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
    aom = N.new("ShaderNodeMapRange")          # the panel's own occlusion, 85 percent strength
    aom.inputs["To Min"].default_value = 0.15
    L.new(sep.outputs[0], aom.inputs["Value"])
    mul = N.new("ShaderNodeMix")
    mul.data_type = "RGBA"
    mul.blend_type = "MULTIPLY"
    mul.inputs["Factor"].default_value = 1.0
    L.new(alb.outputs[0], mul.inputs[6])
    L.new(aom.outputs[0], mul.inputs[7])
    L.new(mul.outputs[2], bsdf.inputs["Base Color"])
    nm = N.new("ShaderNodeNormalMap")
    nm.uv_map = "panel"
    L.new(nor.outputs[0], nm.inputs["Color"])
    L.new(nm.outputs[0], bsdf.inputs["Normal"])
    L.new(emi.outputs[0], bsdf.inputs["Emission Color"])
    bsdf.inputs["Emission Strength"].default_value = 4.0
    bsdf.inputs["Roughness"].default_value = 0.6
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

# The all-view screen is an ellipsoid round the cabin: centre, radii (x, y, z), metres. E() is a
# point on it at azimuth az (0 ahead, positive to the pilot's left) and elevation el (radians),
# scaled by s (1 is the screen surface, larger is the armour outside it).
POD_C, POD_R = V(0.0, 0.96, 0.08), (0.77, 0.78, 0.86)
D = math.radians
# The layout of the screen wall (owner's references, 2026-10-24: Armored Core, Titanfall, the
# Gundam pods): one big front screen, an angled screen each side, a smaller one each side behind
# the shoulders, heavy pillars between them, a dark overhead with small monitors hanging from it.
# (name, az0, az1, el0, el1, node): degrees; the front rides on the visor, the sides on the doors.
SCREENS = [("front", -50, 50, 0, 45, "HATCH_upper"), ("side_L", 53, 104, 0, 45, "DOOR_L"), ("side_R", -104, -53, 0, 45, "DOOR_R"),
           ("rear_L", 113, 150, 4, 38, "pod"), ("rear_R", -150, -113, 4, 38, "pod")]
VISOR_AZ, VISOR_EL, DOOR_AZ1, DOOR_EL = D(51.5), D(76), D(106), D(47)
ARMOUR = (1.06, 1.13)


def E(az, el, s=1.0):
    return V(POD_R[0] * s * math.cos(el) * math.sin(az), POD_C.y + POD_R[1] * s * math.sin(el),
             POD_C.z + POD_R[2] * s * math.cos(el) * math.cos(az))


def patch(S, az0, az1, el0, el1, s, mat, group, node=None, name=None, inward=True, nu=None, nv=None):
    """A piece of the ellipsoid surface between two azimuths and two elevations, facing in or out."""
    nu = nu or max(2, int(abs(az1 - az0) / math.radians(4)))
    nv = nv or max(2, int(abs(el1 - el0) / math.radians(4)))
    bm = bmesh.new()
    grid = [[bm.verts.new(E(az0 + (az1 - az0) * i / nu, el0 + (el1 - el0) * j / nv, s)) for j in range(nv + 1)] for i in range(nu + 1)]
    for i in range(nu):
        for j in range(nv):
            q = (grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1])
            bm.faces.new(q if not inward else tuple(reversed(q)))
    bm.normal_update()
    c = sum((f.calc_center_median() for f in bm.faces), Vector()) / len(bm.faces)
    if (sum((f.normal for f in bm.faces), Vector()).dot(c - POD_C) > 0) == inward:
        bmesh.ops.reverse_faces(bm, faces=bm.faces)
    return S._ob(bm, mat, group, node, name, smooth=True)


def shell(S, az0, az1, el0, el1, s0, s1, mat, group, node=None, bevel=0.0):
    """A solid curved plate: the ellipsoid between two scales, closed at its edges."""
    nu = max(2, int(abs(az1 - az0) / math.radians(4)))
    nv = max(2, int(abs(el1 - el0) / math.radians(4)))
    bm = bmesh.new()
    G = {}
    for k, s in enumerate((s0, s1)):
        G[k] = [[bm.verts.new(E(az0 + (az1 - az0) * i / nu, el0 + (el1 - el0) * j / nv, s)) for j in range(nv + 1)] for i in range(nu + 1)]
    for i in range(nu):
        for j in range(nv):
            bm.faces.new((G[1][i][j], G[1][i + 1][j], G[1][i + 1][j + 1], G[1][i][j + 1]))
            bm.faces.new((G[0][i][j + 1], G[0][i + 1][j + 1], G[0][i + 1][j], G[0][i][j]))
    for i in range(nu):
        for j in (0, nv):
            bm.faces.new((G[0][i][j], G[0][i + 1][j], G[1][i + 1][j], G[1][i][j]))
    for i in (0, nu):
        for j in range(nv):
            bm.faces.new((G[0][i][j], G[0][i][j + 1], G[1][i][j + 1], G[1][i][j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return S._ob(bm, mat, group, node, bevel=bevel)


def bar(S, p0, p1, w, d, mat, group, node=None, bevel=0.004, out=0.0):
    """A straight frame member from p0 to p1 on the inside of the shell: w wide along the surface,
    d deep, its face turned toward the pod's centre; out pushes it outward."""
    p0, p1 = Vector(p0), Vector(p1)
    m = (p0 + p1) / 2
    n = (POD_C - m).normalized()
    a = (p1 - p0).normalized()
    u = n.cross(a).normalized()
    n = a.cross(u).normalized()
    if n.dot(POD_C - m) < 0:
        n = -n
    return S.obox(m - n * out, (a, u, n), ((p1 - p0).length + w, w, d), mat, group, node, bevel=bevel)


def screen(S, name, az0, az1, el0, el1, node):
    """One flat screen between four points on the shell, in a heavy bezel, with a cyan glow strip
    along its lower edge."""
    a0, a1, e0, e1 = D(az0), D(az1), D(el0), D(el1)
    c = [E(a0, e0, 0.985), E(a1, e0, 0.985), E(a1, e1, 0.985), E(a0, e1, 0.985)]
    bm = bmesh.new()
    n = 8
    g = [[bm.verts.new(c[0].lerp(c[1], i / n).lerp(c[3].lerp(c[2], i / n), j / n)) for j in range(n + 1)] for i in range(n + 1)]
    for i in range(n):
        for j in range(n):
            bm.faces.new((g[i][j], g[i + 1][j], g[i + 1][j + 1], g[i][j + 1]))
    bm.normal_update()
    mid = sum(c, Vector()) / 4
    if sum((f.normal for f in bm.faces), Vector()).dot(POD_C - mid) < 0:
        bmesh.ops.reverse_faces(bm, faces=bm.faces)
    S._ob(bm, "screen_wall", "allview", None if node == "pod" else node, name=f"SCREEN_wall_{name}")
    grp = "cabin"
    nd = None if node == "pod" else node
    for k in range(4):
        bar(S, c[k], c[(k + 1) % 4], 0.05, 0.05, "bezel", grp, nd)
    for p in c:
        S.obox(p + (POD_C - p).normalized() * 0.02, (V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)), (0.06, 0.06, 0.06), "paint_dk", grp, nd, bevel=0.01)
    lo0, lo1 = c[0].lerp(c[1], 0.08), c[0].lerp(c[1], 0.92)
    bar(S, lo0 + (POD_C - mid).normalized() * 0.03, lo1 + (POD_C - mid).normalized() * 0.03, 0.008, 0.008, "cyan", grp, nd, bevel=0.0)


# The nodes the game animates: name -> (parent, pivot in the cockpit frame). Every mesh hangs off
# one of these, its pivot at the node's origin, so turning the node about its origin is the motion.
NODES = {
    "pod": (None, V(0, 0, 0)),
    "HATCH_upper": ("pod", E(0, VISOR_EL, ARMOUR[1])),
    "DOOR_L": ("pod", E(DOOR_AZ1, D(22), 1.06)),
    "DOOR_R": ("pod", E(-DOOR_AZ1, D(22), 1.06)),
    "FLAP_L": ("pod", V(0.88, 0.45, 0.55)),
    "FLAP_R": ("pod", V(-0.88, 0.45, 0.55)),
    "HATCH_lower": ("pod", V(0, 0.58, 1.08)),
    "STAIR_1": ("pod", V(0, 0.56, 1.14)),
    "STAIR_2": ("STAIR_1", V(0, -0.06, 1.14)),
    "STAIR_3": ("STAIR_2", V(0, -0.06, 1.18)),
    "CONSOLE_front": ("pod", V(0, 1.04, 0.80)),
    "SCREEN_left": ("CONSOLE_front", None),
    "SCREEN_center": ("CONSOLE_front", None),
    "SCREEN_right": ("CONSOLE_front", None),
    "CTRL_stick": ("pod", V(-0.49, 0.665, 0.30)),
    "CTRL_throttle": ("pod", V(0.4725, 0.66, 0.24)),
    "STRUT_L": ("pod", E(D(62), D(58), ARMOUR[1] + 0.02)),
    "STRUT_R": ("pod", E(-D(62), D(58), ARMOUR[1] + 0.02)),
    "STRUT_L_rod": ("STRUT_L", None),
    "STRUT_R_rod": ("STRUT_R", None),
    "STRUT_L_tip": ("HATCH_upper", E(D(32), D(40), ARMOUR[1] + 0.02)),
    "STRUT_R_tip": ("HATCH_upper", E(-D(32), D(40), ARMOUR[1] + 0.02)),
}
LATCHES = [(-38, -1), (-13, -1), (13, -1), (38, -1), (80, 47), (-80, 47)]


# ----------------------------------------------------------------------------- the cockpit

def build(S):
    X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
    I3 = (X, Y, Z)
    S.vents = []

    def vent(node, p, d, kind, stage, rate=1.0):
        S.vents.append(dict(node=node, p=V(*p), d=V(*d).normalized(), kind=kind, stage=stage, rate=rate))

    # --- the tub: floor, footwell, walls, rear bulkhead, sill rails
    S.plate("floor", (0, 0.0, 0.05), Y, Z, t=0.02, group="cabin")
    S.box((-0.72, -0.04, 0.70), (0.72, 0.0, 0.94), "steel", bevel=0.004)
    S.plate("footwell", (0, 0.27, 0.92), -Z, Y, t=0.02, group="cabin")
    S.box((-0.76, 0.0, 0.92), (0.76, 0.62, 0.98), "paint")
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

    # --- the screen wall: five screens in heavy bezels, the pillars between them, the overhead
    for name, az0, az1, el0, el1, node in SCREENS:
        screen(S, name, az0, az1, el0, el1, node)
    # the inner faces behind the moving screens ride with them; the rest of the shell is fixed
    hn = "HATCH_upper"
    shell(S, -VISOR_AZ, VISOR_AZ, D(-2), DOOR_EL, 1.0, 1.06, "paint_dk", "cabin", node=hn)
    for door, a0, a1 in (("DOOR_L", VISOR_AZ, DOOR_AZ1), ("DOOR_R", -DOOR_AZ1, -VISOR_AZ)):
        shell(S, a0, a1, D(-2), DOOR_EL, 1.0, 1.06, "paint_dk", "cabin", node=door)
        shell(S, a0, a1, D(-2), DOOR_EL, *ARMOUR, "ext_armor", "ext", node=door, bevel=0.008)
        for el in (D(12), D(30)):
            S.tube([E(a0 + (a1 - a0) * t / 8, el, ARMOUR[1] + 0.012) for t in range(9)], 0.012, "ext_dark", "ext", door)
        front = a0 if door == "DOOR_L" else a1
        S.tube([E(front, el, ARMOUR[0] - 0.005) for el in [DOOR_EL * t / 8 for t in range(9)]], 0.012, "rubber", "ext", door)
        for k in range(5):
            vent(door, E(front, DOOR_EL * (k + 0.5) / 5, ARMOUR[1]), (0, 0.2, 1), "steam", "doors", 0.7)
        vent(door, E((a0 + a1) / 2, DOOR_EL, ARMOUR[1]), (0, 1, 0), "steam", "doors", 0.7)
    for (a0, a1, e0, e1) in ((DOOR_AZ1, 2 * math.pi - DOOR_AZ1, D(-2), D(89)), (VISOR_AZ, DOOR_AZ1, DOOR_EL, D(89)),
                             (-DOOR_AZ1, -VISOR_AZ, DOOR_EL, D(89)), (-VISOR_AZ, VISOR_AZ, DOOR_EL, D(89))):
        shell(S, a0, a1, e0, e1, 1.0, 1.06, "paint_dk", "cabin")
    # pillars: the frame between the screens, with cyan light down their inner edges
    for az in (51.5, -51.5, 108.5, -108.5):
        p0, p1 = E(D(az), D(-2), 0.97), E(D(az), D(DOOR_EL / D(1) - 1), 0.97)
        bar(S, p0, p1, 0.09, 0.08, "paint", "cabin", None if abs(az) > 100 else ("DOOR_L" if az > 0 else "DOOR_R"), bevel=0.01)
        bar(S, p0, p1, 0.012, 0.012, "cyan", "cabin", None if abs(az) > 100 else ("DOOR_L" if az > 0 else "DOOR_R"), bevel=0.0, out=-0.05)
        for k in range(6):
            q = p0.lerp(p1, (k + 0.5) / 6)
            S.cyl(q, q + (POD_C - q).normalized() * 0.05, 0.008, "steel", "cabin", None if abs(az) > 100 else ("DOOR_L" if az > 0 else "DOOR_R"), seg=10)
    # the overhead: a frame ring at the screens' top, ribs, round vents, and three hanging monitors
    ring_pts = [E(D(a), DOOR_EL, 0.975) for a in range(-180, 181, 10)]
    for p0, p1 in zip(ring_pts, ring_pts[1:]):
        bar(S, p0, p1, 0.07, 0.06, "paint", "cabin", bevel=0.006)
    for a in range(0, 360, 45):
        S.tube([E(D(a), DOOR_EL + D(2), 0.97), E(D(a), D(70), 0.96), E(D(a), D(86), 0.96)], 0.018, "paint", "cabin")
    for a, el in ((150, 62), (-150, 62), (90, 64), (-90, 64), (0, 80)):
        c = E(D(a), D(el), 0.995)
        n = (POD_C - c).normalized()
        r, u, nn = basis(n, V(0, 0, 1) if abs(n.y) > 0.9 else V(0, 1, 0))
        for k in range(3):
            S.obox(c + n * (0.004 + k * 0.004), (r, u, nn), (0.16 - 0.04 * k, 0.16 - 0.04 * k, 0.006), ("bezel", "paint_dk", "steel")[k], "cabin", bevel=0.004)
    disp = {"OVH_left": ("mfd_right", 30, 56), "OVH_center": ("mfd_center", 0, 60), "OVH_right": ("mfd_left", -30, 56)}
    for name, (img, a, el) in disp.items():
        c = E(D(a), D(el), 0.86)
        n = (EYE - c).normalized()
        r, u, nn = basis(n, V(0, 1, 0))
        S.obox(c - nn * 0.02, (r, u, nn), (0.24, 0.18, 0.04), "bezel", "cabin", bevel=0.006)
        S.cyl(c - nn * 0.03, E(D(a), D(el + 6), 0.97), 0.012, "steel", "cabin", seg=12)
        S.obox(c + nn * 0.001 - u * 0.075, (r, u, nn), (0.2, 0.008, 0.004), "cyan", "cabin")
        S.quad(c + nn * 0.0015, nn, u, 0.21, 0.155, emission_material(name, os.path.join(DISPLAYS, f"{img}.png"), 2.0), "screen", name, node=name)
        NODES[name] = ("pod", None)

    # --- the pod's armour: the fixed rear and top, the visor, the flaps, the lower plate
    shell(S, DOOR_AZ1, 2 * math.pi - DOOR_AZ1, D(-3), D(89.5), *ARMOUR, "ext_armor", "ext", bevel=0.008)
    for (a0, a1, e0) in ((VISOR_AZ, DOOR_AZ1, DOOR_EL), (-DOOR_AZ1, -VISOR_AZ, DOOR_EL), (-VISOR_AZ, VISOR_AZ, VISOR_EL)):
        shell(S, a0, a1, e0 + D(0.5), D(89.5), *ARMOUR, "ext_armor", "ext", bevel=0.008)
    shell(S, -VISOR_AZ + D(0.5), VISOR_AZ - D(0.5), D(-3), VISOR_EL, *ARMOUR, "ext_armor", "ext", node=hn, bevel=0.008)
    shell(S, D(-7), D(7), D(3), VISOR_EL - D(2), ARMOUR[1], ARMOUR[1] + 0.035, "ext_frame", "ext", node=hn, bevel=0.006)
    shell(S, -VISOR_AZ + D(3), VISOR_AZ - D(3), D(-3), D(3), ARMOUR[1], ARMOUR[1] + 0.02, "hazard", "ext", node=hn)
    for s in (1, -1):
        shell(S, s * D(20) - D(7), s * D(20) + D(7), D(20), D(36), ARMOUR[1], ARMOUR[1] + 0.025, "ext_dark", "ext", node=hn, bevel=0.004)
        for k in range(5):
            el = D(22) + k * D(3)
            S.tube([E(s * D(20) - D(6), el, ARMOUR[1] + 0.03), E(s * D(20) + D(6), el, ARMOUR[1] + 0.03)], 0.006, "steel", "ext", hn)
        p = E(s * D(11), VISOR_EL - D(6), ARMOUR[1] + 0.03)
        S.cyl(p, p + (p - POD_C).normalized() * 0.03, 0.022, "amber", "ext", hn, seg=16)
    S.cyl(E(D(-17), VISOR_EL, ARMOUR[1] + 0.01), E(D(17), VISOR_EL, ARMOUR[1] + 0.01), 0.035, "steel", "ext")
    for s in (1, -1):
        vent("pod", E(s * D(14), VISOR_EL, ARMOUR[1] + 0.04), (0, 1, -0.3), "steam", "visor", 1.0)
    for i in range(11):
        az = -VISOR_AZ + 2 * VISOR_AZ * (i + 0.5) / 11
        vent("pod", E(az, D(-2), ARMOUR[1] + 0.01), (math.sin(az), -0.3, math.cos(az)), "steam", "seam", 0.6)
    # latches round the visor and doors
    for i, (az, el) in enumerate(LATCHES):
        p = E(math.radians(az), math.radians(el), ARMOUR[1])
        d = (p - POD_C).normalized()
        S.cyl(p - d * 0.01, p + d * 0.05, 0.028, "steel", "ext", node=f"LATCH_{i + 1}", seg=16)
        S.cyl(p + d * 0.05, p + d * 0.065, 0.014, "amber", "ext", node=f"LATCH_{i + 1}", seg=16)
        NODES[f"LATCH_{i + 1}"] = ("pod", p)
        vent("pod", p + d * 0.06, tuple(d), "steam", "latch", 0.5)
    # struts that lift the visor
    for S_, s in (("L", 1), ("R", -1)):
        b, t = NODES[f"STRUT_{S_}"][1], NODES[f"STRUT_{S_}_tip"][1]
        L = (t - b).length
        S.cyl(b, b + V(0, 0, L * 0.55), 0.035, "ext_dark", "ext", node=f"STRUT_{S_}", seg=16)
        rod0 = b + V(0, 0, L * 0.45)
        NODES[f"STRUT_{S_}_rod"] = (f"STRUT_{S_}", rod0)
        S.cyl(rod0, rod0 + V(0, 0, L * 0.55), 0.018, "metal", "ext", node=f"STRUT_{S_}_rod", seg=16)
        S.cyl(b - V(0.04 * s, 0, 0), b + V(0.04 * s, 0, 0), 0.03, "steel", "ext", seg=16)
        vent("pod", b, (s * 0.5, 0.3, 0.2), "steam", "visor", 0.6)
    # the sides, rear and belly of the pod; the flaps at its front corners
    for s, side in ((1, "L"), (-1, "R")):
        x0, x1 = sorted((s * 0.80, s * 0.88))
        S.box((x0, -0.12, -0.80), (x1, 0.96, 0.55), "ext_armor", "ext", bevel=0.01)
        fn = f"FLAP_{side}"
        S.box((x0, -0.12, 0.55), (x1, 0.96, 1.10), "ext_armor", "ext", node=fn, bevel=0.01)
        fx0, fx1 = (x1, x1 + 0.025) if s > 0 else (x0 - 0.025, x0)
        S.box((fx0, 0.10, 0.62), (fx1, 0.80, 1.04), "ext_frame", "ext", node=fn, bevel=0.006)
        S.box((x0 - 0.01, 0.84, 0.58), (x1 + 0.01, 0.9, 1.08), "hazard", "ext", node=fn)
        S.cyl(V(s * 0.88, -0.1, 0.55), V(s * 0.88, 0.94, 0.55), 0.03, "steel", "ext", seg=16)
        for k in range(4):
            vent("pod", V(s * 0.89, 0.1 + 0.25 * k, 1.09), (s, 0.1, 0.3), "steam", "flaps", 0.6)
    S.box((-0.88, -0.18, -0.86), (0.88, 0.96, -0.80), "ext_armor", "ext", bevel=0.01)
    S.box((-0.88, -0.18, -0.80), (0.88, -0.12, 1.10), "ext_frame", "ext", bevel=0.01)
    # the lower front: fixed armour under the lower plate, the plate itself, the stair stowed on it
    S.box((-0.80, -0.12, 0.98), (0.80, 0.58, 1.10), "ext_armor", "ext", bevel=0.01)
    ln = "HATCH_lower"
    S.box((-0.80, 0.585, 0.98), (0.80, 0.97, 1.08), "ext_armor", "ext", node=ln, bevel=0.01)
    S.box((-0.74, 0.62, 0.975), (0.74, 0.94, 0.985), "steel", "ext", node=ln)          # its inner face: the step plate
    S.box((-0.74, 0.92, 0.97), (0.74, 0.95, 0.98), "hazard", "ext", node=ln)
    S.box((-0.6, 0.66, 1.08), (0.6, 0.70, 1.10), "red", "ext", node=ln)
    S.cyl(V(-0.7, 0.58, 1.08), V(0.7, 0.58, 1.08), 0.03, "steel", "ext", seg=16)
    for st_, (y0, y1, z0, zr) in (("STAIR_1", (-0.06, 0.56, 1.11, 1.17)), ("STAIR_2", (-0.06, 0.52, 1.17, 1.23)), ("STAIR_3", (-0.02, 0.50, 1.18, 1.22))):
        w = 0.5 if st_ != "STAIR_3" else 0.44
        for sx in (-1, 1):
            S.box((sx * w / 2 - 0.025, y0, z0), (sx * w / 2 + 0.025, y1, zr), "ext_dark", "ext", node=st_, bevel=0.004)
        n = 4
        for k in range(n):
            y = y0 + 0.05 + (y1 - y0 - 0.1) * k / (n - 1)
            S.box((-w / 2, y - 0.012, z0 + 0.005), (w / 2, y + 0.012, zr - 0.005), "steel", "ext", node=st_)
            S.box((-w / 2, y + 0.008, zr - 0.012), (w / 2, y + 0.014, zr - 0.004), "hazard", "ext", node=st_)
    # mist: cold air pouring out over the sill
    for i in range(6):
        vent("pod", V(-0.5 + 0.2 * i, 0.70, 0.98), (0, -0.2, 1), "mist", "open", 1.0)


# ----------------------------------------------------------------------------- lights

def lights():
    """The sealed cabin's light: no sky. The screen wall glows (role screen_wall: the outside seen
    through it lights the cabin), floodlights under the glare shield and the screen's lower edge,
    a lamp strip on the rear bulkhead, a red footwell lamp, and the MFDs and lit buttons."""
    sc = bpy.context.scene
    w = bpy.data.worlds.new("Dark")
    w.use_nodes = True
    w.node_tree.nodes["Background"].inputs["Color"].default_value = (0.005, 0.005, 0.006, 1)
    w.light_settings.distance = 0.06
    sc.world = w

    def light(name, kind, loc, rot_to, energy, colour, size=0.05, sx=None, sy=None):
        ld = bpy.data.lights.new(name, kind)
        ld.energy = energy
        ld.color = colour
        if kind == "AREA":
            ld.shape = "RECTANGLE"
            ld.size, ld.size_y = sx, sy
        else:
            ld.shadow_soft_size = size
        ob = bpy.data.objects.new(name, ld)
        ob.location = B(loc)
        if rot_to is not None:
            d = (B(rot_to) - B(loc)).normalized()
            ob.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
        sc.collection.objects.link(ob)

    warm = (1.0, 0.86, 0.68)
    light("Flood_glare", "AREA", V(0, 0.99, 0.616), V(0, 0.4, 0.66), 7.0, warm, sx=1.2, sy=0.02)
    for s in (1, -1):
        light(f"Flood_sill_{s}", "AREA", V(s * 0.66, 0.912, 0.05), V(s * 0.5, 0.4, 0.05), 3.5, warm, sx=0.02, sy=0.56)
    light("Rear_strip", "AREA", V(0, 1.125, -0.575), V(0, 0.5, 0.3), 5.0, (1.0, 0.9, 0.78), sx=1.0, sy=0.02)
    light("Footwell", "POINT", V(0, 0.25, 0.80), None, 1.5, (1.0, 0.18, 0.12), size=0.05)


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
    bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=0.0015, area_weight=0.0, scale_to_bounds=False)
    bpy.ops.uv.select_all(action="SELECT")
    bpy.ops.uv.pack_islands(rotate=True, margin=0.0015, shape_method="AABB")
    bpy.ops.object.mode_set(mode="OBJECT")


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


def tonemap(x, exposure):
    """Linear radiance to display: exposure, a soft shoulder (white at 6), sRGB."""
    x = np.maximum(x * exposure, 0.0)
    y = x * (1.0 + x / 36.0) / (1.0 + x)
    return K.to_srgb(y)


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
    """Mark the baked, screen and HUD materials KHR_materials_unlit in the exported glb."""
    data = open(path, "rb").read()
    jlen = struct.unpack_from("<I", data, 12)[0]
    js = json.loads(data[20:20 + jlen])
    rest = data[20 + jlen:]
    for m in js.get("materials", []):
        if m["name"].startswith(("BAKED_", "SCREEN_", "HUD_")):
            m.setdefault("extensions", {})["KHR_materials_unlit"] = {}
    used = js.setdefault("extensionsUsed", [])
    if "KHR_materials_unlit" not in used:
        used.append("KHR_materials_unlit")
    jb = json.dumps(js, separators=(",", ":")).encode()
    jb += b" " * ((4 - len(jb) % 4) % 4)
    out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(jb) + len(rest)) + struct.pack("<II", len(jb), 0x4E4F534A) + jb + rest
    open(path, "wb").write(out)


def tris(ob):
    return sum(len(p.vertices) - 2 for p in ob.data.polygons)


# ----------------------------------------------------------------------------- main

def ext_material(role, cache={}):
    """The pod's outside as a lit PBR material: the mech's plating maps where the role has them."""
    if role in cache:
        return cache[role]
    tex = {"ext_armor": "armorL", "ext_frame": "frame", "hazard": "hazard"}.get(role)
    spec = K.ROLES[role]
    mat = bpy.data.materials.new(f"EXT_{role}")
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    bsdf = N["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = spec["r"]
    bsdf.inputs["Metallic"].default_value = spec["m"]
    bsdf.inputs["Base Color"].default_value = list(K.srgb(spec["c"])) + [1.0]
    if spec.get("emit"):
        bsdf.inputs["Emission Color"].default_value = list(K.srgb(spec["emit"])) + [1.0]
        bsdf.inputs["Emission Strength"].default_value = 3.0
    if tex:
        uv = N.new("ShaderNodeUVMap")
        uv.uv_map = "UVMap"
        t = N.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(os.path.join(ROOT, "assets", "textures", "mech", f"{tex}.png"), check_existing=True)
        L.new(uv.outputs[0], t.inputs[0])
        L.new(t.outputs[0], bsdf.inputs["Base Color"])
        npath = os.path.join(ROOT, "assets", "textures", "mech", f"{tex}_n.png")
        if os.path.exists(npath):
            tn = N.new("ShaderNodeTexImage")
            tn.image = bpy.data.images.load(npath, check_existing=True)
            tn.image.colorspace_settings.name = "Non-Color"
            L.new(uv.outputs[0], tn.inputs[0])
            nm = N.new("ShaderNodeNormalMap")
            L.new(tn.outputs[0], nm.inputs["Color"])
            L.new(nm.outputs[0], bsdf.inputs["Normal"])
    cache[role] = mat
    return mat


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
    ap.add_argument("--size", type=int, default=4096)
    ap.add_argument("--samples", type=int, default=96)
    ap.add_argument("--exposure", type=float, default=1.6)
    ap.add_argument("--no-denoise", action="store_true")
    ap.add_argument("--no-bake", action="store_true", help="geometry only: skip the bake, keep the last atlases")
    a = ap.parse_args(argv)
    t0 = time.time()
    os.makedirs(RAW, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    mats = K.make_materials()
    S = Set(mats)
    build(S)
    lights()

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

    results = {g: os.path.join(BAKED, f"baked_{g}.jpg") for g in atlases}
    if not a.no_bake:
        sc.render.engine = "CYCLES"
        cy = sc.cycles
        cy.device = "CPU"
        cy.samples = a.samples
        cy.use_denoising = False
        cy.max_bounces = 4
        cy.diffuse_bounces = 3
        cy.glossy_bounces = 0
        cy.transmission_bounces = 0
        cy.transparent_max_bounces = 2
        cy.sample_clamp_indirect = 3.0
        cy.seed = 0
        bk = sc.render.bake
        bk.margin = 12
        bk.margin_type = "EXTEND"
        bk.use_pass_direct = bk.use_pass_indirect = True
        bk.use_pass_diffuse = bk.use_pass_emit = True
        bk.use_pass_glossy = bk.use_pass_transmission = False
        for g, obs in atlases.items():
            bpy.ops.object.select_all(action="DESELECT")
            for ob in obs:
                ob.select_set(True)
                ob.data.uv_layers.active = ob.data.uv_layers["light"]
            bpy.context.view_layer.objects.active = obs[0]
            imgs = {}
            for kind in ("COMBINED", "AO"):
                t = time.time()
                img = bpy.data.images.new(f"bake_{g}_{kind}", a.size, a.size, float_buffer=True, alpha=False)
                bake_target(img)
                if kind == "COMBINED":
                    bpy.ops.object.bake(type="COMBINED", pass_filter={"DIRECT", "INDIRECT", "DIFFUSE", "EMIT"}, margin=12,
                                        use_clear=True)
                else:
                    sc.world.light_settings.distance = 0.06
                    cy.samples = max(16, a.samples // 3)
                    bpy.ops.object.bake(type="AO", margin=12, use_clear=True)
                    cy.samples = a.samples
                imgs[kind] = pixels(img)[:, :, :3].copy()
                print(f"BAKED {g} {kind} in {time.time() - t:.0f} s", flush=True)
            rgb, ao = imgs["COMBINED"], imgs["AO"][:, :, :1]
            if not a.no_denoise:
                t = time.time()
                rgb = denoise(rgb)
                ao = denoise(np.repeat(ao, 3, axis=2))[:, :, :1]
                print(f"DENOISED {g} in {time.time() - t:.0f} s", flush=True)
            lit = rgb * (0.55 + 0.45 * np.clip(ao, 0, 1))
            save_jpeg(results[g], tonemap(lit, a.exposure))
            np.save(os.path.join(RAW, f"baked_{g}_linear.npy"), lit.astype(np.float16))

    # export materials: baked surfaces unlit on their light UV, the screens unlit, the outside lit
    disp = {"SCREEN_left": "mfd_left", "SCREEN_center": "mfd_center", "SCREEN_right": "mfd_right",
            "OVH_left": "mfd_right", "OVH_center": "mfd_center", "OVH_right": "mfd_left"}
    for name, (ob, group, node) in meshes.items():
        if group in ("front", "cabin"):
            only_uv(ob, "light")
            ob.data.materials.clear()
            ob.data.materials.append(bpy.data.materials.get(f"BAKED_{group}") or export_material(f"BAKED_{group}", results[group]))
        elif group == "screen":
            only_uv(ob, "panel")
            ob.data.materials.clear()
            ob.data.materials.append(export_material(f"SCREEN_{node.split('_', 1)[1].lower()}" if node.startswith("SCREEN") else f"SCREEN_{node.lower()}",
                                                     os.path.join(DISPLAYS, f"{disp[node]}.png")))
        elif group == "allview":
            for l in [l.name for l in ob.data.uv_layers if not l.name.startswith(".")]:
                ob.data.uv_layers.remove(ob.data.uv_layers[l])
            m = bpy.data.materials.get("SCREEN_allview") or bpy.data.materials.new("SCREEN_allview")
            m.use_nodes = True
            m.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.01, 0.012, 0.014, 1)
            ob.data.materials.clear()
            ob.data.materials.append(m)
        elif group == "ext":
            for l in [l.name for l in ob.data.uv_layers if not l.name.startswith(".")]:
                ob.data.uv_layers.remove(ob.data.uv_layers[l])
            cube_uvs(ob)
            for i, slot in enumerate(ob.material_slots):
                role = slot.material.name.split(".")[0] if slot.material else "ext_dark"
                ob.material_slots[i].material = ext_material(role if role in K.ROLES else "ext_dark")
    for o in [o for o in sc.objects if o.type == "LIGHT"]:
        bpy.data.objects.remove(o)

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
    for name, p in {"EYE_pilot": EYE, "SEAT_pilot": V(0, 0.45, -0.02), "HATCH_entry": V(0, 0.80, 1.20),
                    "LADDER_base": V(0, -1.9, 2.3)}.items():
        e = bpy.data.objects.new(name, None)
        e.location = B(p)
        sc.collection.objects.link(e)
        e.parent = root
    root["eye_m"] = list(EYE)
    bpy.context.view_layer.update()
    os.makedirs(os.path.dirname(MODEL), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=MODEL, export_format="GLB", export_extras=True, export_yup=True, export_apply=False,
                              export_image_format="AUTO", export_texcoords=True, export_normals=True, export_lights=False,
                              export_cameras=False, use_selection=False)
    patch_unlit(MODEL)
    total = sum(tris(ob) for ob, _, _ in meshes.values())
    man = {
        "model": "assets/models/cockpit_breeze.glb",
        "frame": "glTF: +y up, the pilot faces +z, +x is the pilot's left; metres; origin on the tub floor under the eye",
        "eye_m": list(EYE),
        "nodes": {n: {"parent": NODES[n][0], "pivot_m": [round(c, 4) for c in pivot(n)]} for n in NODES},
        "meshes": {k: {"tris": tris(v[0]), "group": v[1], "node": v[2]} for k, v in sorted(meshes.items())},
        "vents": len(S.vents),
        "tris": total, "tri_budget": TRI_BUDGET,
        "draw_calls": sum(len(ob.data.materials) for ob, _, _ in meshes.values()),
        "atlases": {g: {"file": os.path.relpath(p, ROOT), "px": a.size} for g, p in results.items()},
        "bake": {"samples": a.samples, "exposure": a.exposure, "denoised": not a.no_denoise,
                 "lights": "the screen wall's glow, glare shield and sill floodlights, the rear lamp strip, a red footwell lamp, the MFDs and lit buttons"},
        "panels": sorted(CP.PANELS),
        "glb_bytes": os.path.getsize(MODEL),
    }
    with open(MODEL.replace(".glb", ".json"), "w") as f:
        json.dump(man, f, indent=2)
        f.write("\n")
    if total > TRI_BUDGET:
        raise SystemExit(f"cockpit is {total} triangles, over the budget of {TRI_BUDGET}")
    print(f"EXPORTED {MODEL}: {total} triangles, {os.path.getsize(MODEL) / 1e6:.1f} MB in {time.time() - t0:.0f} s", flush=True)


main()
