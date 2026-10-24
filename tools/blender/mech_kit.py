"""Shared kit for Crab Breeze mech generators (run inside Blender, headless).

A port of Mining Mike's boss mech builder (RubenTipparach/godot-sandbox
godot-sandbox/docs/mockups/tank_bosses_kit.js at 3336b88, the `Kit` class, `rigLegs`, `makeMats`
and the box-projected UVs), so a mech here is built with the same vocabulary as Mining Mike's Uber
mechs and bosses (owner, 2026-10-24: "The mech boss and Uber mech stuff are some of the best in
mining mike"). The API keeps the kit's names and argument order: J, g, box, pl, pz, px, cyl, sph,
tor, oct, stripes, vent, bolts, hose, lamp, ant, piston, sock. What changes:

- it writes Blender objects and exports a glb on the bone names of docs/design/mech-rig.md;
- builders work in the kit's own units and `s` turns them into metres (two passes: measure, rescale);
- a halo is an empty `HALO_*` with its size and colour in glTF extras, drawn by whoever loads it;
- a piston is a sleeve on one joint and a rod on the other, so it telescopes with no runtime code;
- a reactor core is its own node `CORE_*`, so a viewer can spin it.

Coordinates are glTF style: +Y up, the mech faces +Z, +X is its left. Each joint's geometry is merged
into one object per material.
"""
import json
import math
import os
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pbr as PBR  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
TEXDIR = os.path.join(ROOT, "assets", "textures", "mech")

C = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))  # glTF to Blender
CI = C.inverted()
TAU = math.tau


def eul(rx=0.0, ry=0.0, rz=0.0):
    """three.js Euler order XYZ."""
    return Matrix.Rotation(rx, 4, "X") @ Matrix.Rotation(ry, 4, "Y") @ Matrix.Rotation(rz, 4, "Z")


def hash1(i):
    s = math.sin(i * 127.1 + 311.7) * 43758.5453
    return s - math.floor(s)


def clear_scene():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    for coll in (bpy.data.meshes, bpy.data.materials, bpy.data.images):
        for item in list(coll):
            coll.remove(item)


# ----------------------------------------------------------------------------- materials

def srgb(hexv):
    c = [((hexv >> 16) & 255) / 255, ((hexv >> 8) & 255) / 255, (hexv & 255) / 255]
    return [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]


def _image(name):
    path = os.path.join(TEXDIR, name + ".png")
    img = bpy.data.images.get(name) or bpy.data.images.load(path)
    img.name = name
    return img


def _mat(name, color=0xffffff, tex=None, normal=None, rough=0.5, metal=0.0, emit=None, emit_k=0.0, alpha=1.0, orm=None):
    """A PBR material. `orm` names a plating ORM map (occlusion, roughness, metal), which then decides
    roughness and metal in place of `rough` and `metal` (those stay for untextured parts)."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    b = nt.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = srgb(color) + [1.0]
    b.inputs["Roughness"].default_value = rough
    b.inputs["Metallic"].default_value = metal
    if tex:
        it = nt.nodes.new("ShaderNodeTexImage")
        it.image = _image(tex)
        if color != 0xffffff:  # a tinted map: multiply, which glTF exports as baseColorFactor
            mix = nt.nodes.new("ShaderNodeMix")
            mix.data_type = "RGBA"
            mix.blend_type = "MULTIPLY"
            mix.inputs["Factor"].default_value = 1.0
            nt.links.new(it.outputs["Color"], mix.inputs[6])
            mix.inputs[7].default_value = srgb(color) + [1.0]
            nt.links.new(mix.outputs[2], b.inputs["Base Color"])
        else:
            nt.links.new(it.outputs["Color"], b.inputs["Base Color"])
    if normal:
        PBR.normal(nt, b, _image(normal))
    if orm:
        PBR.orm(nt, b, _image(orm))
    if emit is not None:
        b.inputs["Emission Color"].default_value = srgb(emit) + [1.0]
        b.inputs["Emission Strength"].default_value = emit_k
    if alpha < 1:
        b.inputs["Alpha"].default_value = alpha
        m.surface_render_method = "BLENDED"
    return m


def scale_hex(hexv, k):
    r, g, b = (hexv >> 16) & 255, (hexv >> 8) & 255, hexv & 255
    return (min(255, int(r * k)) << 16) | (min(255, int(g * k)) << 8) | min(255, int(b * k))


def make_mats(prefix, glow, livery, core, light=False):
    """makeMats from the kit, one set per mech. `light` swaps the dark armour for the pale plate."""
    a = "armorL" if light else "armor"
    return {
        "armor": _mat(f"{prefix}_armor", tex=a, normal=a + "_n", orm=a + "_orm"),
        "armor2": _mat(f"{prefix}_armor2", tex="armorB", normal="armorB_n", orm="armorB_orm"),
        "frame": _mat(f"{prefix}_frame", tex="frame", normal="frame_n", orm="frame_orm"),
        "red": _mat(f"{prefix}_livery", color=livery, tex="wear", normal="wear_n", orm="wear_orm"),
        "art": _mat(f"{prefix}_core", color=core, emit=core, emit_k=2.5, rough=0.25, metal=0.2),
        "ink": _mat(f"{prefix}_ink", color=0x170d0f, rough=0.6, metal=0.3),
        "metal": _mat(f"{prefix}_metal", color=0xa9aeb5, tex="wear", normal="wear_n", rough=0.26, metal=0.95),
        "dark": _mat(f"{prefix}_dark", color=0x232126, tex="frame", normal="frame_n", rough=0.7, metal=0.45),
        "rubber": _mat(f"{prefix}_rubber", color=0x141313, rough=0.92),
        "hazard": _mat(f"{prefix}_hazard", tex="hazard", orm="hazard_orm"),
        "lens": _mat(f"{prefix}_lens", color=0x060608, emit=glow, emit_k=1.6, rough=0.08, metal=0.6),
        "glow": _mat(f"{prefix}_glow", color=glow, emit=glow, emit_k=4.0),
        "glowDim": _mat(f"{prefix}_glowdim", color=scale_hex(glow, 0.45), emit=glow, emit_k=1.2),
        "tip": _mat(f"{prefix}_tip", color=0xff2a20, emit=0xff2a20, emit_k=4.0),
        "glass": _mat(f"{prefix}_glass", color=0x5a8aa8, rough=0.05, alpha=0.22),
        "seat": _mat(f"{prefix}_seat", color=0x2a2420, rough=0.85),
        "screen": _mat(f"{prefix}_screen", color=0x0c3a2c, emit=0x2fd18e, emit_k=0.9, rough=0.3),
    }


# ----------------------------------------------------------------------------- primitives (local, metres)

def _poly_prism(pts, depth, bv):
    """ExtrudeGeometry: the polygon in XY, extruded along Z, centred, edges bevelled by bv."""
    bm = bmesh.new()
    vs = [bm.verts.new((x, y, -depth / 2)) for x, y in pts]
    f = bm.faces.new(vs)
    res = bmesh.ops.extrude_face_region(bm, geom=[f])
    top = [g for g in res["geom"] if isinstance(g, bmesh.types.BMVert)]
    bmesh.ops.translate(bm, verts=top, vec=(0, 0, depth))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    if bv > 0:
        bmesh.ops.bevel(bm, geom=list(bm.edges), offset=bv, offset_type="OFFSET", segments=1, profile=0.5,
                        affect="EDGES", clamp_overlap=True)
    return bm


def _box(w, h, d, bv=0.0):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Diagonal((w, h, d, 1)))
    if bv > 0:
        bmesh.ops.bevel(bm, geom=list(bm.edges), offset=bv, offset_type="OFFSET", segments=1, profile=0.5,
                        affect="EDGES", clamp_overlap=True)
    return bm


def _cyl(rt, rb, h, seg):
    """CylinderGeometry: along +Y, radius rt at the top."""
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=seg, radius1=rb, radius2=rt, depth=h,
                          matrix=Matrix.Rotation(-math.pi / 2, 4, "X"))
    return bm


def _sph(r, seg):
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=max(6, seg // 2), radius=r)
    return bm


def _tor(R0, r, arc, seg, tube=6):
    """TorusGeometry: a ring in the XY plane around Z."""
    bm = bmesh.new()
    full = abs(arc - TAU) < 1e-6
    n = seg if full else seg + 1
    rings = []
    for i in range(n):
        a = arc * i / seg
        c, s = math.cos(a), math.sin(a)
        ring = []
        for j in range(tube):
            b = TAU * j / tube
            rr = R0 + r * math.cos(b)
            ring.append(bm.verts.new((rr * c, rr * s, r * math.sin(b))))
        rings.append(ring)
    for i in range(seg):
        r0, r1 = rings[i], rings[(i + 1) % n]
        for j in range(tube):
            bm.faces.new((r0[j], r1[j], r1[(j + 1) % tube], r0[(j + 1) % tube]))
    return bm


def _oct(r):
    bm = bmesh.new()
    v = [bm.verts.new(p) for p in ((r, 0, 0), (-r, 0, 0), (0, r, 0), (0, -r, 0), (0, 0, r), (0, 0, -r))]
    for a, b, c in ((0, 2, 4), (2, 1, 4), (1, 3, 4), (3, 0, 4), (2, 0, 5), (1, 2, 5), (3, 1, 5), (0, 3, 5)):
        bm.faces.new((v[a], v[b], v[c]))
    return bm


def _tube(points, r, segs=20, sides=7):
    """TubeGeometry along a Catmull-Rom curve through points."""
    P = [Vector(p) for p in points]
    ext = [P[0] * 2 - P[1]] + P + [P[-1] * 2 - P[-2]]
    path = []
    for i in range(segs + 1):
        t = i / segs * (len(P) - 1)
        k = min(int(t), len(P) - 2)
        u = t - k
        p0, p1, p2, p3 = ext[k], ext[k + 1], ext[k + 2], ext[k + 3]
        path.append(0.5 * ((2 * p1) + (-p0 + p2) * u + (2 * p0 - 5 * p1 + 4 * p2 - p3) * u * u
                           + (-p0 + 3 * p1 - 3 * p2 + p3) * u ** 3))
    bm = bmesh.new()
    rings = []
    for i, p in enumerate(path):
        tgt = (path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]).normalized()
        nrm = tgt.cross(Vector((0, 1, 0)) if abs(tgt.y) < 0.9 else Vector((1, 0, 0))).normalized()
        bi = tgt.cross(nrm)
        rings.append([bm.verts.new(p + r * (math.cos(TAU * j / sides) * nrm + math.sin(TAU * j / sides) * bi))
                      for j in range(sides)])
    for a, b in zip(rings, rings[1:]):
        for j in range(sides):
            bm.faces.new((a[j], b[j], b[(j + 1) % sides], a[(j + 1) % sides]))
    return bm


# ----------------------------------------------------------------------------- the kit

class Node:
    """A place to hang things: a joint, and a transform inside that joint's frame (metres)."""

    def __init__(self, joint, M):
        self.joint, self.M = joint, M


class Kit:
    def __init__(self, mats, s=1.0, prefix="mech"):
        self.M, self.s, self.prefix = mats, s, prefix
        self.joints, self.world, self.rest = {}, {}, {}
        self.acc = {}          # (joint, material) -> bmesh
        self.own = []          # separate nodes: (name, joint, local matrix, bmesh, material)
        self.halos = 0
        self.extras = {}
        root = self._empty("root", None, Matrix.Identity(4))
        self.root = Node("root", Matrix.Identity(4))
        self.world["root"] = Matrix.Identity(4)
        del root

    # --- joints and groups
    def _empty(self, name, parent, T):
        o = bpy.data.objects.new(name, None)
        o.empty_display_type = "PLAIN_AXES"
        o.empty_display_size = 0.25
        bpy.context.collection.objects.link(o)
        if parent:
            o.parent = self.joints[parent]
        o.matrix_parent_inverse = Matrix.Identity(4)
        o.matrix_basis = C @ T @ CI
        self.joints[name] = o
        return o

    def _T(self, p, x, y, z, rx, ry, rz):
        s = self.s
        return p.M @ Matrix.Translation((x * s, y * s, z * s)) @ eul(rx, ry, rz)

    def J(self, name, p, x=0, y=0, z=0, rx=0, ry=0, rz=0):
        T = self._T(p, x, y, z, rx, ry, rz)
        self._empty(name, p.joint, T)
        self.world[name] = self.world[p.joint] @ T
        self.rest[name] = [rx, ry, rz]
        return Node(name, Matrix.Identity(4))

    def g(self, p, x=0, y=0, z=0, rx=0, ry=0, rz=0):
        return Node(p.joint, self._T(p, x, y, z, rx, ry, rz))

    def sock(self, name, p, x=0, y=0, z=0, rx=0, ry=0, extras=None):
        n = self.J(name, p, x, y, z, rx, ry, 0)
        if extras:
            for k, v in extras.items():
                self.joints[name][k] = v
        return n

    # --- geometry
    def _uv(self, bm):
        """boxUV: box projection in kit units over a 40 unit tile, offset by a hash of the vertex count."""
        uv = bm.loops.layers.uv.verify()
        S = 40 * self.s
        n = len(bm.verts)
        ox, oy = hash1(n) * 40 * self.s, hash1(n + 7) * 40 * self.s
        bm.normal_update()
        for f in bm.faces:
            a = [abs(c) for c in f.normal]
            for lp in f.loops:
                p = lp.vert.co
                if a[0] >= a[1] and a[0] >= a[2]:
                    u, v = p.z, p.y
                elif a[1] >= a[2]:
                    u, v = p.x, p.z
                else:
                    u, v = p.x, p.y
                lp[uv].uv = ((u + ox) / S, (v + oy) / S)

    def _put(self, bm, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, scale=None, smooth=False, own=None):
        self._uv(bm)
        for f in bm.faces:
            f.smooth = smooth
        L = self._T(p, x, y, z, rx, ry, rz)
        if scale:
            L = L @ Matrix.Diagonal((scale[0], scale[1], scale[2], 1))
        if own:
            self.own.append((own, p.joint, L, bm, mat))
            return
        bm.transform(L)
        key = (p.joint, mat)
        if key not in self.acc:
            self.acc[key] = bmesh.new()
            self.acc[key].loops.layers.uv.verify()
        me = bpy.data.meshes.new("tmp")
        bm.to_mesh(me)
        bm.free()
        self.acc[key].from_mesh(me)
        bpy.data.meshes.remove(me)

    def box(self, w, h, d, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0):
        s = self.s
        self._put(_box(w * s, h * s, d * s), mat, p, x, y, z, rx, ry, rz)

    def pl(self, w, h, d, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, bv=None):
        s = self.s
        bv = min(0.9, min(w, h, d) * 0.22) if bv is None else bv
        bv = max(0.05, min(bv, w / 2 - 0.05, h / 2 - 0.05, d / 2 - 0.05))
        self._put(_box(w * s, h * s, d * s, bv * s), mat, p, x, y, z, rx, ry, rz)
        return self.g(p, x, y, z, rx, ry, rz)

    def pz(self, pts, depth, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, bv=0.35):
        s = self.s
        self._put(_poly_prism([(a * s, b * s) for a, b in pts], depth * s, bv * s), mat, p, x, y, z, rx, ry, rz)

    def px(self, pts, width, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, bv=0.35):
        s = self.s
        bm = _poly_prism([(a * s, b * s) for a, b in pts], width * s, bv * s)
        bm.transform(Matrix.Rotation(-math.pi / 2, 4, "Y"))
        self._put(bm, mat, p, x, y, z, rx, ry, rz)

    def cyl(self, rt, rb, h, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, seg=16):
        s = self.s
        self._put(_cyl(rt * s, rb * s, h * s, seg), mat, p, x, y, z, rx, ry, rz, smooth=True)

    def sph(self, r, mat, p, x=0, y=0, z=0, sx=1, sy=None, sz=None, seg=14):
        s = self.s
        sy = sx if sy is None else sy
        sz = sx if sz is None else sz
        self._put(_sph(r * s, seg), mat, p, x, y, z, 0, 0, 0, scale=(sx, sy, sz), smooth=True)

    def tor(self, R0, r, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, arc=TAU, seg=24):
        s = self.s
        self._put(_tor(R0 * s, r * s, arc, seg), mat, p, x, y, z, rx, ry, rz, smooth=True)

    def oct(self, r, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, sy=1, own=None):
        s = self.s
        self._put(_oct(r * s), mat, p, x, y, z, rx, ry, rz, scale=(1, sy, 1), own=own)

    def hose(self, p, pts, r, mat=None):
        s = self.s
        self._put(_tube([(a * s, b * s, c * s) for a, b, c in pts], r * s), mat or self.M["rubber"], p, 0, 0, 0, 0, 0, 0,
                  smooth=True)

    # --- compositions, as in the kit
    def stripes(self, p, w, x, y, z, ry=0, hb=1.4, gap=0.75, d=0.45, rx=0, rz=0):
        g = self.g(p, x, y, z, rx, ry, rz)
        self.box(w, hb, d, self.M["red"], g, 0, (hb + gap) / 2, 0)
        self.box(w, hb, d, self.M["red"], g, 0, -(hb + gap) / 2, 0)
        self.box(w * 0.98, gap, d * 0.7, self.M["ink"], g, 0, 0, -0.05)
        return g

    def vent(self, p, w, h, x, y, z, rx=0, ry=0, n=4):
        g, M = self.g(p, x, y, z, rx, ry, 0), self.M
        self.box(w, h, 0.6, M["ink"], g, 0, 0, 0)
        self.box(w * 0.86, h * 0.8, 0.2, M["glowDim"], g, 0, 0, 0.35)
        for i in range(n):
            self.box(w * 0.94, (h / n) * 0.42, 0.7, M["frame"], g, 0, -h / 2 + (i + 0.5) * h / n, 0.7, -0.55)
        self.box(w + 0.8, 0.6, 1.2, M["frame"], g, 0, h / 2 + 0.2, 0.3)
        self.box(w + 0.8, 0.6, 1.2, M["frame"], g, 0, -h / 2 - 0.2, 0.3)
        return g

    def halo(self, p, x, y, z, size, op=0.8, color=None):
        self.halos += 1
        return self.sock(f"HALO_{self.halos}", p, x, y, z,
                         extras={"halo_size_m": size * self.s, "halo_opacity": op, "halo_color": color or self.extras.get("glow", 0xffffff)})

    def lamp(self, p, x, y, z, r, hs=None, red=False):
        self.sph(r, self.M["tip"] if red else self.M["glow"], p, x, y, z)
        self.halo(p, x, y, z, hs or r * 7, 0.7, 0xff3020 if red else None)

    def ant(self, p, x, y, z, length, rx=0, rz=0):
        g, M = self.g(p, x, y, z, rx, 0, rz), self.M
        self.cyl(0.75, 0.95, 1.4, M["frame"], g, 0, 0.7, 0)
        self.cyl(0.16, 0.3, length, M["metal"], g, 0, 1.4 + length / 2, 0, seg=6)
        self.cyl(0.5, 0.5, 0.5, M["dark"], g, 0, 1.4 + length * 0.45, 0, seg=8)
        self.sph(0.42, M["tip"], g, 0, 1.6 + length, 0)
        self.halo(g, 0, 1.6 + length, 0, 3.2, 0.9, 0xff3020)

    def bolts(self, p, n, x0, y, z, dx, r, rx=0):
        for i in range(n):
            self.cyl(r, r, 0.5, self.M["metal"], p, x0 + i * dx, y, z, rx + math.pi / 2, 0, 0, seg=6)

    def core(self, p, x, y, z, r):
        """artCore: a spinning octahedron in a frame ring, with a halo; the reactor here."""
        self.tor(r * 1.35, r * 0.2, self.M["frame"], p, x, y, z, math.pi / 2)
        n = sum(1 for o in self.own if o[0].startswith("CORE_")) + 1
        self.oct(r, self.M["art"], p, x, y, z, own=f"CORE_{n}")
        self.halo(p, x, y, z, r * 7, 0.75, self.extras.get("core"))

    def piston(self, gA, a, gB, b, r):
        """A ram from a point on joint gA to a point on joint gB: sleeve on A, rod on B, so it telescopes."""
        WA, WB = self.world[gA.joint] @ gA.M, self.world[gB.joint] @ gB.M
        s = self.s
        A, B = Vector(a) * s, Vector(b) * s
        B_in_A = WA.inverted() @ (WB @ B)
        A_in_B = WB.inverted() @ (WA @ A)
        L = (B_in_A - A).length / s
        for node, p0, p1, rad, mat, frac in ((gA, A, B_in_A, r, self.M["dark"], 0.58), (gB, B, A_in_B, r * 0.55, self.M["metal"], 0.62)):
            d = (p1 - p0).normalized()
            q = Vector((0, 1, 0)).rotation_difference(d).to_matrix().to_4x4()
            bm = _cyl(rad * s, rad * s, L * frac * s, 10)
            bm.transform(Matrix.Translation(d * L * frac * s / 2) @ q)
            self._put(bm, mat, node, p0.x / s, p0.y / s, p0.z / s, smooth=True)
            self.sph(rad * 1.25, self.M["frame"], node, p0.x / s, p0.y / s, p0.z / s, seg=8)

    # --- legs: rigLegs, on the contract's names
    def legs(self, body, hip_w, segs, seg_fns, foot_fn, hip_dy=0):
        """segs: [[length, rest angle]] in kit units. Joints: thigh, shin, (hock), foot, toe; a two
        segment leg gets a zero length hock so every mech has the contract's bones."""
        names = ["thigh", "shin", "hock"]
        for s in (1, -1):
            S = "L" if s > 0 else "R"
            parent, total = body, 0.0
            for i, (length, rest) in enumerate(segs):
                j = self.J(f"{names[i]}_{S}", parent, s * hip_w if i == 0 else 0,
                           hip_dy if i == 0 else -segs[i - 1][0], 0, rest)
                total += rest
                if i < len(seg_fns) and seg_fns[i]:
                    seg_fns[i](self, j, s, length)
                parent = j
            if len(segs) == 2:
                parent = self.J(f"hock_{S}", parent, 0, -segs[-1][0], 0, 0)
                ank = self.J(f"foot_{S}", parent, 0, 0, 0, -total)
            else:
                ank = self.J(f"foot_{S}", parent, 0, -segs[-1][0], 0, -total)
            foot_fn(self, ank, s, S)
        self.extras["legSegs"] = [[length * self.s, rest] for length, rest in segs]


def leg_height(segs, foot_h):
    """legH from the kit."""
    a, h = 0.0, foot_h
    for length, rest in segs:
        a += rest
        h += length * math.cos(a)
    return h


# ----------------------------------------------------------------------------- output

def finish(kit):
    """Turn the accumulated geometry into objects under their joints."""
    for (joint, mat), bm in kit.acc.items():
        bm.transform(C)
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-6)
        me = bpy.data.meshes.new(f"{joint}_{mat.name}")
        bm.to_mesh(me)
        bm.free()
        o = bpy.data.objects.new(f"{joint}_{mat.name.split('_', 1)[-1]}", me)
        bpy.context.collection.objects.link(o)
        o.parent = kit.joints[joint]
        o.matrix_parent_inverse = Matrix.Identity(4)
        me.materials.append(mat)
    for name, joint, L, bm, mat in kit.own:
        bm.transform(C)
        me = bpy.data.meshes.new(name)
        bm.to_mesh(me)
        bm.free()
        o = bpy.data.objects.new(name, me)
        bpy.context.collection.objects.link(o)
        o.parent = kit.joints[joint]
        o.matrix_parent_inverse = Matrix.Identity(4)
        o.matrix_basis = C @ L @ CI
        me.materials.append(mat)
    kit.acc, kit.own = {}, []


def measure():
    """Triangle count and glTF bounds of every mesh, after modifiers."""
    bpy.context.view_layer.update()
    dg = bpy.context.evaluated_depsgraph_get()
    tris, lo, hi = 0, Vector((1e9,) * 3), Vector((-1e9,) * 3)
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        ev = obj.evaluated_get(dg)
        me = ev.to_mesh()
        me.calc_loop_triangles()
        tris += len(me.loop_triangles)
        mw = obj.matrix_world
        for v in me.vertices:
            w = mw @ v.co
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
        ev.to_mesh_clear()
    return tris, {"x": [lo.x, hi.x], "y": [lo.z, hi.z], "z": [-hi.y, -lo.y]}


def export(path, manifest_path, meta, root_extras):
    tris, bounds = measure()
    root = bpy.data.objects["root"]
    for k, v in root_extras.items():
        root[k] = json.dumps(v) if isinstance(v, (list, dict)) else v
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_apply=True, export_yup=True,
                              export_extras=True, export_image_format="JPEG", export_jpeg_quality=88)
    meta = dict(meta, triangles=tris, bounds_m={k: [round(a, 3), round(b, 3)] for k, (a, b) in bounds.items()},
                file=os.path.basename(path), bytes=os.path.getsize(path))
    data = json.load(open(manifest_path)) if os.path.exists(manifest_path) else {}
    data[meta["id"]] = meta
    with open(manifest_path, "w") as f:
        json.dump(data, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"EXPORTED {meta['id']}: {tris} tris, height {bounds['y'][1]:.2f} m, {meta['bytes'] / 1e6:.1f} MB")
    return meta
