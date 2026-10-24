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


# ----------------------------------------------------------------------------- the cockpit

def build(S):
    M = S.mats
    X, Y, Z = V(1, 0, 0), V(0, 1, 0), V(0, 0, 1)
    I3 = (X, Y, Z)

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
    S.box((-0.78, -0.04, -0.70), (0.78, 1.32, -0.62), "paint")
    S.plate("rear", (0, 0.66, -0.62), Z, Y, t=0.012, group="cabin")
    S.box((-0.78, 1.20, -0.66), (0.78, 1.34, -0.5), "paint_dk", bevel=0.01)

    # --- side consoles: bodies, tops (the console panels), inner faces (breakers), front blocks
    for s, side in ((1, "L"), (-1, "R")):
        x0, x1 = sorted((s * 0.36, s * 0.72))
        S.box((x0, 0.0, -0.42), (x1, 0.645, 0.52), "paint", bevel=0.004)
        S.plate(f"console_{side}", (s * 0.53, 0.657, 0.05), Y, Z, t=0.012)
        S.plate(f"breakers_{side}", (s * 0.36, 0.38, 0.05), -s * X, Y, t=0.01, group="cabin")
        S.box((x0 - (0.02 if s < 0 else 0), 0.0, 0.52), (x1 + (0.02 if s > 0 else 0), 0.62, 0.94), "paint_dk", bevel=0.006)
        # edge trim along the console top, and the floodlight under the sill above it
        S.box((x0, 0.645, -0.43), (x1, 0.663, -0.39), "paint_dk", bevel=0.003)
        S.obox(V(s * 0.66, 0.925, 0.05), I3, (0.03, 0.012, 0.6), "bezel", bevel=0.002)
        S.obox(V(s * 0.66, 0.917, 0.05), I3, (0.016, 0.004, 0.56), "white_lit")
        # cable bundles under the sill, clamped to the wall
        for i, (r, mat) in enumerate(((0.012, "rubber"), (0.009, "red"), (0.01, "rubber"), (0.007, "yellow"))):
            y = 0.905 - i * 0.022
            S.tube([V(s * 0.70, y, -0.66), V(s * 0.70, y, -0.36), V(s * 0.695, y - 0.01, 0.0), V(s * 0.70, y, 0.46),
                    V(s * 0.70, y + 0.02, 0.84)], r, mat)
        for z in (-0.5, -0.3, 0.5, 0.75):
            S.obox(V(s * 0.70, 0.872, z), I3, (0.03, 0.09, 0.025), "steel", bevel=0.003)

    # --- the dash: main panel (tilted back 18 degrees), body, MFD bezels and screens
    a = math.radians(18)
    nd, ud = V(0, math.sin(a), -math.cos(a)), V(0, math.cos(a), math.sin(a))
    cd = V(0, 0.80, 0.70)
    S.plate("dash_main", cd, nd, ud, t=0.014)
    rd, ud2, nd2 = basis(nd, ud)
    S.obox(cd - nd * 0.15, (rd, ud2, nd2), (1.58, 0.47, 0.28), "paint_dk", group="front")
    disp = {"SCREEN_left": "mfd_left", "SCREEN_center": "mfd_center", "SCREEN_right": "mfd_right"}
    for name, px, py, s in CP.MFDS:
        c = cd + rd * px + ud2 * py + nd2 * 0.002
        O, I, t = (s + 0.06) / 2, s / 2, 0.03
        for (cx, cy, w, h) in ((0, (O + I) / 2, 2 * O, O - I), (0, -(O + I) / 2, 2 * O, O - I),
                               ((O + I) / 2, 0, O - I, 2 * I), (-(O + I) / 2, 0, O - I, 2 * I)):
            S.obox(c + rd * cx + ud2 * cy + nd2 * t / 2, (rd, ud2, nd2), (w, h, t), "bezel", group="front", bevel=0.003)
        S.obox(c + nd2 * 0.004, (rd, ud2, nd2), (2 * I, 2 * I, 0.008), "glass", group="front")
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
                S.obox(p + nd2 * (t + 0.005), (rd, ud2, nd2), sz, "paint_lt", group="front", bevel=0.0025)
        for cx in (-1, 1):            # corner rockers
            S.obox(c + rd * cx * (O - (O - I) / 2) + ud2 * (O - (O - I) / 2) + nd2 * (t + 0.004), (rd, ud2, nd2),
                   ((O - I) * 0.55, (O - I) * 0.55, 0.01), "steel", group="front", bevel=0.003)
        S.quad(c + nd2 * 0.0105, nd2, ud2, 2 * I * 0.97, 2 * I * 0.97,
               emission_material(name, os.path.join(DISPLAYS, f"{disp[name]}.png"), 2.5), "screen", name)

    # --- the glare shield over the dash, its floodlight, and the HUD
    S.plate("glare", (0, 1.05, 0.70), Y, Z, t=0.012)
    S.box((-0.79, 1.01, 0.60), (0.79, 1.038, 0.80), "paint_dk", group="front", bevel=0.01)
    S.box((-0.78, 1.0, 0.605), (0.78, 1.012, 0.625), "bezel", group="front", bevel=0.002)
    S.box((-0.60, 0.996, 0.611), (0.60, 1.0, 0.62), "white_lit", group="front")
    S.obox(V(0, 1.085, 0.70), I3, (0.22, 0.06, 0.15), "paint_dk", group="front", bevel=0.008)
    S.obox(V(0, 1.116, 0.69), I3, (0.15, 0.004, 0.09), "glass", group="front")
    hc = V(0, 1.185, 0.63)
    hn = (EYE - hc).normalized()
    hr, hu, _ = basis(hn, Y)
    for sx in (-1, 1):
        S.cyl(V(sx * 0.105, 1.11, 0.66), hc + hr * sx * 0.105 - hu * 0.07, 0.006, "steel", group="front", seg=12)
    S.obox(hc + hu * 0.082, (hr, hu, hn), (0.23, 0.012, 0.012), "paint_dk", group="front", bevel=0.003)
    S.quad(hc, hn, hu, 0.20, 0.15, emission_material("HUD_combiner", os.path.join(DISPLAYS, "hud.png"), 3.0, alpha=True),
           "hud", "HUD_combiner")

    # --- the knee panel and the dash's lower body
    b = math.radians(10)
    nk, uk = V(0, math.sin(b), -math.cos(b)), V(0, math.cos(b), math.sin(b))
    ck = V(0, 0.43, 0.64)
    S.plate("knee", ck, nk, uk, t=0.012)
    rk, uk2, nk2 = basis(nk, uk)
    S.obox(ck - nk2 * 0.13, (rk, uk2, nk2), (1.12, 0.32, 0.25), "paint_dk", group="front")

    # --- the seat: pedestal, cushion, bolsters, back, headrest, harness, eject handle
    S.box((-0.18, 0.0, -0.22), (0.18, 0.34, 0.18), "paint_dk", bevel=0.01)
    S.box((-0.25, 0.34, -0.24), (0.25, 0.45, 0.22), "fabric", bevel=0.03, seg=3)
    for s in (1, -1):
        S.box((s * 0.22 - 0.035, 0.38, -0.24), (s * 0.22 + 0.035, 0.53, 0.2), "fabric", bevel=0.025, seg=3)
    bot, top = V(0, 0.45, -0.22), V(0, 1.14, -0.32)
    ub = (top - bot).normalized()
    nb = ub.cross(X).normalized() * -1      # faces the pilot (+z)
    if nb.z < 0:
        nb = -nb
    rb, ub2, nb2 = basis(nb, ub)
    S.obox((bot + top) / 2, (rb, ub2, nb2), (0.50, 0.70, 0.11), "fabric", bevel=0.03, seg=3)
    S.obox((bot + top) / 2 - nb2 * 0.08, (rb, ub2, nb2), (0.56, 0.76, 0.06), "paint_dk", bevel=0.01)
    for s in (1, -1):
        S.obox((bot + top) / 2 + rb * s * 0.27 + nb2 * 0.03, (rb, ub2, nb2), (0.07, 0.62, 0.13), "fabric", bevel=0.025, seg=3)
    S.obox(top + ub2 * 0.14 + nb2 * 0.01, (rb, ub2, nb2), (0.30, 0.20, 0.10), "fabric", bevel=0.03, seg=3)
    S.obox(top + ub2 * 0.14 - nb2 * 0.06, (rb, ub2, nb2), (0.34, 0.24, 0.05), "paint_dk", bevel=0.008)
    for s in (1, -1):        # shoulder straps over the back and the cushion, lap belt, buckle
        p0 = top + rb * s * 0.09 + nb2 * 0.062
        p1 = bot + rb * s * 0.11 + nb2 * 0.062 + ub2 * 0.05
        S.obox((p0 + p1) / 2, (rb, ub2, nb2), (0.05, (p0 - p1).length, 0.008), "strap")
        S.obox(V(s * 0.13, 0.456, -0.02), I3, (0.17, 0.008, 0.05), "strap")
        S.obox(V(s * 0.21, 0.5, -0.02), (X, Y, Z), (0.008, 0.09, 0.05), "strap")
    S.cyl(V(0, 0.452, -0.02), V(0, 0.468, -0.02), 0.04, "metal", seg=32)
    S.cyl(V(0, 0.468, -0.02), V(0, 0.471, -0.02), 0.022, "red", seg=32)
    loop = [V(0.07 * math.cos(t), 0.36 + 0.05 * math.sin(t), 0.235) for t in [i * 2 * math.pi / 12 for i in range(12)]]
    S.tube(loop, 0.011, "hazard", closed=True)
    S.box((-0.05, 0.33, 0.215), (0.05, 0.36, 0.23), "bezel", bevel=0.004)

    # --- the controls, each its own node with its pivot at the origin
    sp = V(0, 0.30, 0.36)                                # stick pivot
    S.box((-0.08, 0.0, 0.27), (0.08, 0.27, 0.45), "paint_dk", bevel=0.01)
    S.cyl(sp - V(0, 0.03, 0), sp + V(0, 0.05, 0), 0.075, "rubber", r1=0.035, seg=24)
    st = "CTRL_stick"
    S.cyl(sp, sp + V(0, 0.28, 0.02), 0.014, "steel", node=st, group="front", seg=16)
    S.obox(sp + V(0, 0.34, 0.03), (X, V(0, 0.98, 0.17).normalized(), V(0, -0.17, 0.98).normalized()), (0.05, 0.13, 0.065),
           "rubber", node=st, group="front", bevel=0.018, seg=3)
    S.obox(sp + V(0, 0.32, 0.07), I3, (0.02, 0.035, 0.02), "metal", node=st, group="front", bevel=0.006)
    S.cyl(sp + V(0.0, 0.405, 0.03), sp + V(0.0, 0.42, 0.035), 0.009, "steel", node=st, group="front", seg=16)
    S.cyl(sp + V(-0.017, 0.40, 0.0), sp + V(-0.017, 0.413, 0.0), 0.007, "red_lit", node=st, group="front", seg=16)
    S.cyl(sp + V(0.02, 0.385, 0.015), sp + V(0.03, 0.39, 0.015), 0.006, "steel", node=st, group="front", seg=12)

    tp = V(0.4725, 0.66, 0.24)                           # throttle pivot, in the slot
    tn = "CTRL_throttle"
    S.cyl(tp, tp + V(0, 0.15, 0), 0.012, "steel", node=tn, group="front", seg=16)
    S.obox(tp + V(0.015, 0.19, 0.0), (V(0.94, 0.34, 0).normalized(), V(-0.34, 0.94, 0).normalized(), Z), (0.06, 0.09, 0.11),
           "rubber", node=tn, group="front", bevel=0.02, seg=3)
    S.cyl(tp + V(-0.01, 0.235, 0.03), tp + V(-0.012, 0.243, 0.03), 0.008, "red_lit", node=tn, group="front", seg=16)
    S.obox(tp + V(-0.03, 0.21, -0.02), I3, (0.012, 0.02, 0.02), "metal", node=tn, group="front", bevel=0.004)
    S.cyl(tp + V(0.0, 0.225, 0.055), tp + V(0.0, 0.225, 0.062), 0.009, "steel", node=tn, group="front", seg=16)

    wp = V(-0.49, 0.665, 0.29)                           # twist grip pivot
    wn = "CTRL_twist"
    S.cyl(wp, wp + V(0, 0.012, 0), 0.05, "steel", node=wn, group="front", seg=32)
    S.cyl(wp + V(0, 0.012, 0), wp + V(0, 0.09, 0), 0.016, "steel", node=wn, group="front", seg=16)
    S.cyl(wp + V(0, 0.105, -0.07), wp + V(0, 0.105, 0.07), 0.02, "rubber", node=wn, group="front", seg=20)
    for zz in (-0.075, 0.075):
        S.cyl(wp + V(0, 0.105, zz - 0.006), wp + V(0, 0.105, zz + 0.006), 0.024, "paint_dk", node=wn, group="front", seg=20)
    S.cyl(wp + V(0, 0.105, 0.081), wp + V(0, 0.105, 0.086), 0.01, "amber", node=wn, group="front", seg=16)

    for s in (1, -1):        # pedals on arms from the footwell wall
        S.cyl(V(s * 0.15, 0.2, 0.92), V(s * 0.15, 0.16, 0.80), 0.014, "steel", seg=12)
        n_p = V(0, 0.7, -0.71).normalized()
        S.obox(V(s * 0.15, 0.13, 0.78), basis(n_p, Y), (0.11, 0.2, 0.025), "steel", bevel=0.005)
        S.obox(V(s * 0.15, 0.135, 0.775), basis(n_p, Y), (0.1, 0.03, 0.03), "hazard", bevel=0.003)

    # --- the canopy: glass and bows on their own node, hinged at the back
    cc, rad = V(0, 0.96, 0.08), (0.77, 0.78, 0.86)
    S.ellipsoid_cap(cc, rad, 0.965, glass_material(), "glass", node="canopy", name="GLASS_canopy")

    def bow(z0, scale=0.975, n=24):
        k = 1 - ((z0 - cc.z) / rad[2]) ** 2
        pts = []
        for i in range(n + 1):
            t = math.pi * i / n
            pts.append(V(rad[0] * math.sqrt(k) * math.cos(t) * scale, cc.y + rad[1] * math.sqrt(k) * math.sin(t) * scale, z0))
        return pts

    for z0, r in ((0.52, 0.024), (-0.36, 0.03)):
        S.tube(bow(z0), r, "paint_dk", node="canopy")
    ring = [V(rad[0] * 0.995 * math.cos(t), 0.975, cc.z + rad[2] * 0.995 * math.sin(t)) for t in [i * 2 * math.pi / 40 for i in range(40)]]
    S.tube(ring, 0.028, "paint_dk", node="canopy", closed=True)
    top_aft = bow(-0.36)[12]
    S.obox(top_aft + V(0, -0.035, 0.02), I3, (0.12, 0.03, 0.06), "paint_dk", node="canopy", bevel=0.008)
    S.obox(top_aft + V(0, -0.052, 0.02), I3, (0.09, 0.004, 0.04), "white_lit", node="canopy")
    for s in (1, -1):        # canopy handles on the front bow
        p = bow(0.52)[4 if s > 0 else 20]
        S.obox(p + V(-s * 0.03, -0.03, -0.02), I3, (0.02, 0.06, 0.02), "yellow", node="canopy", bevel=0.006)

    # --- the mech outside: the nose and cheeks seen through the glass
    nose = bmesh.new()
    prof = [(0.86, 0.55), (0.86, 0.975), (1.25, 0.945), (1.95, 0.72), (1.95, 0.52)]
    vs = []
    for z, y in prof:
        hw = 0.85 - (z - 0.86) * 0.28
        vs.append((nose.verts.new((hw, y, z)), nose.verts.new((-hw, y, z))))
    for i in range(len(prof)):
        j = (i + 1) % len(prof)
        nose.faces.new((vs[i][0], vs[j][0], vs[j][1], vs[i][1]))
    nose.faces.new([v[0] for v in vs])
    nose.faces.new([v[1] for v in reversed(vs)])
    bmesh.ops.recalc_face_normals(nose, faces=nose.faces)
    S._ob(nose, "paint_lt", "cabin", bevel=0.02, seg=3)
    S.obox(V(0, 0.933, 1.30), basis(V(0, 0.95, 0.31), Z), (1.1, 0.12, 0.004), "hazard")
    S.obox(V(0, 0.829, 1.62), basis(V(0, 0.95, 0.31), Z), (0.95, 0.07, 0.004), "red")
    for s in (1, -1):
        x0, x1 = sorted((s * 0.79, s * 1.3))
        S.box((x0, 0.45, -0.75), (x1, 0.955, 0.95), "paint_lt", bevel=0.03, seg=3)
        S.box((x0 + 0.05, 0.955, -0.6), (x1 - 0.05, 0.965, 0.8), "red")


# ----------------------------------------------------------------------------- lights

def lights():
    sc = bpy.context.scene
    w = bpy.data.worlds.new("Sky")
    w.use_nodes = True
    nt = w.node_tree
    N, L = nt.nodes, nt.links
    bg = N["Background"]
    tc = N.new("ShaderNodeTexCoord")
    sep = N.new("ShaderNodeSeparateXYZ")
    L.new(tc.outputs["Generated"], sep.inputs[0])
    ramp = N.new("ShaderNodeValToRGB")
    mr = N.new("ShaderNodeMapRange")
    mr.inputs["From Min"].default_value = -1.0
    L.new(sep.outputs[2], mr.inputs["Value"])
    L.new(mr.outputs[0], ramp.inputs[0])
    cr = ramp.color_ramp
    cr.elements[0].position, cr.elements[0].color = 0.0, (0.06, 0.055, 0.05, 1)
    cr.elements[1].position, cr.elements[1].color = 0.5, (0.55, 0.58, 0.62, 1)
    e = cr.elements.new(0.62)
    e.color = (0.45, 0.55, 0.72, 1)
    e = cr.elements.new(1.0)
    e.color = (0.28, 0.40, 0.66, 1)
    L.new(ramp.outputs[0], bg.inputs["Color"])
    bg.inputs["Strength"].default_value = 0.55
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
    light("Dome", "POINT", V(0, 1.63, -0.32), None, 6.0, (1.0, 0.9, 0.75), size=0.04)
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

def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser()
    ap.add_argument("--size", type=int, default=4096)
    ap.add_argument("--samples", type=int, default=96)
    ap.add_argument("--exposure", type=float, default=1.6)
    ap.add_argument("--no-denoise", action="store_true")
    a = ap.parse_args(argv)
    t0 = time.time()
    os.makedirs(RAW, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    mats = K.make_materials()
    S = Set(mats)
    build(S)
    lights()

    # realize, then join: the static front, the static cabin, the canopy frame, each control
    for p in S.parts:
        realize(p["ob"])
    by = {}
    for p in S.parts:
        by.setdefault((p["group"], p["node"]), []).append(p["ob"])
    pivots = {"CTRL_stick": V(0, 0.30, 0.36), "CTRL_throttle": V(0.4725, 0.66, 0.24), "CTRL_twist": V(-0.49, 0.665, 0.29)}
    hinge = V(0, 0.975, -0.70)
    nodes = {}
    for (group, node), obs in by.items():
        if group in ("screen", "hud", "glass"):
            for ob in obs:
                nodes[ob.name] = (ob, group, node)
            continue
        if node is None:
            name = f"COCKPIT_{group}"
        elif node == "canopy":
            name = "CANOPY_frame"
        else:
            name = node
        ob = join(obs, name, pivots.get(node, hinge if node == "canopy" else None))
        nodes[name] = (ob, group, node)
    atlases = {g: [ob for ob, gg, _ in nodes.values() if gg == g] for g in ("front", "cabin")}
    for g, obs in atlases.items():
        lightmap_uvs(obs)
    print(f"BUILT in {time.time() - t0:.0f} s: " + ", ".join(f"{k} {tris(v[0])}" for k, v in nodes.items()), flush=True)

    # bake each atlas: combined (diffuse light, direct and indirect, and emission), then occlusion
    sc.render.engine = "CYCLES"
    cy = sc.cycles
    cy.device = "CPU"
    cy.samples = a.samples
    cy.use_denoising = False
    cy.max_bounces = 4
    cy.diffuse_bounces = 3
    cy.glossy_bounces = 0
    cy.transmission_bounces = 2
    cy.transparent_max_bounces = 6
    cy.sample_clamp_indirect = 3.0
    cy.seed = 0
    bk = sc.render.bake
    bk.margin = 12
    bk.margin_type = "EXTEND"
    bk.use_pass_direct = bk.use_pass_indirect = True
    bk.use_pass_diffuse = bk.use_pass_emit = True
    bk.use_pass_glossy = bk.use_pass_transmission = False
    results = {}
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
        final = tonemap(rgb * (0.55 + 0.45 * np.clip(ao, 0, 1)), a.exposure)
        path = os.path.join(BAKED, f"baked_{g}.jpg")
        save_jpeg(path, final)
        np.save(os.path.join(RAW, f"baked_{g}_linear.npy"), (rgb * (0.55 + 0.45 * np.clip(ao, 0, 1))).astype(np.float16))
        results[g] = path

    # export: baked surfaces unlit on their light UV, screens and HUD unlit, glass transparent
    for name, (ob, group, node) in nodes.items():
        if group in ("front", "cabin"):
            only_uv(ob, "light")
            ob.data.materials.clear()
            ob.data.materials.append(bpy.data.materials.get(f"BAKED_{group}") or export_material(f"BAKED_{group}", results[group]))
        elif group == "screen":
            only_uv(ob, "panel")
            ob.data.materials.clear()
            disp = {"SCREEN_left": "mfd_left", "SCREEN_center": "mfd_center", "SCREEN_right": "mfd_right"}[name]
            ob.data.materials.append(export_material(name, os.path.join(DISPLAYS, f"{disp}.png")))
        elif group == "hud":
            only_uv(ob, "panel")
            ob.data.materials.clear()
            ob.data.materials.append(export_material("HUD_combiner", os.path.join(DISPLAYS, "hud.png"), alpha=True))
        elif group == "glass":
            only_uv(ob, "panel")
            m = bpy.data.materials.new("GLASS_canopy_export")
            m.use_nodes = True
            b = m.node_tree.nodes["Principled BSDF"]
            b.inputs["Base Color"].default_value = (0.72, 0.84, 0.95, 1)
            b.inputs["Alpha"].default_value = 0.12
            b.inputs["Roughness"].default_value = 0.05
            m.blend_method = "BLEND"
            ob.data.materials.clear()
            ob.data.materials.append(m)
    for o in [o for o in sc.objects if o.type == "LIGHT"]:
        bpy.data.objects.remove(o)
    root = bpy.data.objects.new("cockpit", None)
    sc.collection.objects.link(root)
    canopy = bpy.data.objects.new("canopy", None)
    canopy.location = B(hinge)
    sc.collection.objects.link(canopy)
    canopy.parent = root
    for name, (ob, group, node) in nodes.items():
        par = canopy if node == "canopy" else root
        mw = ob.matrix_world.copy()
        ob.parent = par
        ob.matrix_world = mw
    socks = {"EYE_pilot": EYE, "SEAT_pilot": V(0, 0.45, -0.02), "HATCH_entry": V(0, 1.9, -0.1)}
    for name, p in socks.items():
        e = bpy.data.objects.new(name, None)
        e.location = B(p)
        sc.collection.objects.link(e)
        e.parent = root
    root["eye_m"] = list(EYE)
    root["canopy_hinge_m"] = list(hinge)
    bpy.context.view_layer.update()
    os.makedirs(os.path.dirname(MODEL), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=MODEL, export_format="GLB", export_extras=True, export_yup=True, export_apply=False,
                              export_image_format="AUTO", export_texcoords=True, export_normals=True, export_lights=False,
                              export_cameras=False, use_selection=False)
    patch_unlit(MODEL)
    total = sum(tris(ob) for ob, _, _ in nodes.values())
    man = {
        "model": "assets/models/cockpit_breeze.glb",
        "frame": "glTF: +y up, the pilot faces +z, +x is the pilot's left; metres; origin on the tub floor under the eye",
        "eye_m": list(EYE), "canopy_hinge_m": list(hinge),
        "nodes": {k: {"tris": tris(v[0]), "atlas": v[1] if v[1] in ("front", "cabin") else None} for k, v in sorted(nodes.items())},
        "tris": total, "tri_budget": TRI_BUDGET,
        "draw_calls": sum(len(ob.data.materials) for ob, _, _ in nodes.values()),
        "atlases": {g: {"file": os.path.relpath(p, ROOT), "px": a.size} for g, p in results.items()},
        "bake": {"samples": a.samples, "exposure": a.exposure, "denoised": not a.no_denoise,
                 "lights": "sky through the canopy, glare shield and sill floodlights, dome lamp, red footwell lamp, screen and button glow"},
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
