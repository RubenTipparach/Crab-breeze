"""Real furniture for the small buildings' interiors: the 1980s set a pilot walks past on foot.

The small buildings kit (`docs/mockups/small_buildings_kit.js`, `FURN`) dresses houses and shops with
furniture made of boxes. This generator models each of those items as believable furniture of the game's
setting (alternate-history 1980s: American suburbs, diners, corner stores, garages, a laundromat, a salon),
one glb per item, each filling the space of its block version so the game can swap a block item for the
real one with the same transform (`docs/design/furniture.md`).

Built the CSG way of `blender-hard-surface`: rounded boxes, lathes, swept tubes and extruded profiles,
openings carved with named cutters through Exact booleans (the sink, the cooler's open front, the TV's
screen recess, the washer's door well, the shelf bays of the stand), bevels where light catches, then one
triangulated mesh with UVs in metres.

Conventions the game relies on (`docs/design/furniture.md`):
- local frame in metres, glTF axes: y up, the item's front faces +z, origin on the floor;
- one node `MODEL`, one mesh, one primitive in the one material `furniture` shared by every item, textured by
  one 4096 px atlas (`assets/textures/furniture/`: albedo, normal, ORM) baked here in Cycles from procedural
  shaders, so all furniture is one draw call; the atlas's bottom strip holds a swatch per kit surface key for
  the block furniture;
- COLOR_0 is (1, 1, 1, tint mask): alpha 1 on the surface the placement's tint multiplies into;
- TEXCOORD_0 lies in the item's atlas region; NORMAL is split on hard edges only; TANGENT is written.

Run (Blender 4.4 or later, or the `bpy` module):

    blender --background --factory-startup --python tools/blender/build_furniture.py [-- --only sofa,bed] [--sheet]
    python tools/blender/build_furniture.py [--only sofa,bed] [--sheet] [--no-build]

Writes assets/models/furniture/<item>.glb, assets/textures/furniture/*.png and assets/models/furniture.json
(triangles, bounds, atlas regions, swatches and sha256s, read back off the written files), byte for byte the
same on every run. `--sheet` renders docs/screenshots/furniture/
(sheet.png, every item at true relative scale beside a 1.8 m figure, and one still per item) in Cycles on
the CPU. The build refuses an item over 6,000 triangles or outside its block's box by more than 5 cm.
"""
import hashlib
import json
import math
import os
import random
import struct
import sys

import bpy  # noqa: I001  (bpy first: bmesh and mathutils exist only after it under the pip module)
import bmesh
from mathutils import Matrix, Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "furniture")
MANIFEST = os.path.join(ROOT, "assets", "models", "furniture.json")
SHOTS = os.path.join(ROOT, "docs", "screenshots", "furniture")
GENERATOR = "tools/blender/build_furniture.py"
LIMIT_TRIS = 6000  # the build refuses an item over this (CLAUDE.md section 7)
SLACK_M = 0.05  # how far outside its block's box an item may reach

# The kit's surfaces (`SURFACES` in docs/mockups/small_buildings_kit.js, plus the glass and lamp keys its
# builders use): roughness, metallic and stand-in colour, for the glb's own material and the contact sheet.
SURFACES = {
    "trim": (0.5, 0.0, 0xF4F1EA), "paint": (0.4, 0.0, 0xFFFFFF), "soffit": (0.8, 0.0, 0xF2EFE8),
    "metal": (0.35, 0.7, 0x9AA0A6), "stainless": (0.25, 1.0, 0xD8DCDE), "counter": (0.3, 0.0, 0xE8E2D0),
    "wood": (0.6, 0.0, 0x8A6A46), "fabric": (0.9, 0.0, 0x8A7A6A), "product": (0.5, 0.0, 0xC8A040),
    "rubber": (0.85, 0.0, 0x222222), "concrete": (0.9, 0.0, 0xBAB4AA), "carpet": (0.95, 0.0, 0x8A7A6A),
    "glassDark": (0.05, 0.0, 0x3A4A52), "glassShop": (0.05, 0.0, 0xFFFFFF), "lamp": (0.4, 0.0, 0xFFFFFF),
}
WHITE = 0xFFFFFF


# ----------------------------------------------------------------------------- primitives (glTF frame)

def lin(hexv):
    """An sRGB hex colour as linear RGBA."""
    c = [((hexv >> 16) & 255) / 255, ((hexv >> 8) & 255) / 255, (hexv & 255) / 255]
    return tuple([x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c] + [1.0])


def _samples(a0, a1, r, seg, mid):
    """Sample positions along one axis of a rounded box: dense in the rounded bands, `mid` inside."""
    if r <= 0:
        return [a0] + [a0 + (a1 - a0) * (k + 1) / (mid + 1) for k in range(mid)] + [a1]
    lo = [a0 + r * k / seg for k in range(seg + 1)]
    hi = [a1 - r + r * k / seg for k in range(seg + 1)]
    inner = [a0 + r + (a1 - a0 - 2 * r) * (k + 1) / (mid + 1) for k in range(mid)]
    out = []
    for s in lo + inner + hi:
        if not out or s > out[-1] + 1e-6:
            out.append(s)
    return out


def box(lo, hi, r=0.0, seg=1, mid=(0, 0, 0), bulge=None):
    """A box from `lo` to `hi`, its edges rounded with radius `r` in `seg` steps, `mid` extra samples per
    axis, and `bulge` {"+y": metres, ...} pushing a face out in a pillow (cushions)."""
    lo, hi = Vector(lo), Vector(hi)
    half = (hi - lo) / 2
    if r > 0 and seg == 1 and not bulge and not any(mid):
        # a plain chamfer: one bevel segment on every edge, the hard-surface way
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Translation((lo + hi) / 2) @ Matrix.Diagonal((*(hi - lo), 1)))
        bmesh.ops.bevel(bm, geom=bm.edges[:] + bm.verts[:], offset=min(r, min(half) * 0.95), offset_type="OFFSET",
                        segments=1, profile=0.5, affect="EDGES", clamp_overlap=True)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        return bm
    rv = Vector([min(r, half[i] * 0.98) for i in range(3)])
    rounded = r > 0
    S = [_samples(lo[i], hi[i], rv[i] if rounded else 0.0, seg, mid[i]) for i in range(3)]
    n = [len(s) for s in S]
    bm = bmesh.new()
    verts = {}

    def vert(i, j, k):
        key = (i, j, k)
        if key not in verts:
            verts[key] = bm.verts.new((S[0][i], S[1][j], S[2][k]))
        return verts[key]

    for axis in range(3):
        a, b = [x for x in range(3) if x != axis]
        for side in (0, n[axis] - 1):
            for u in range(n[a] - 1):
                for v in range(n[b] - 1):
                    quad = []
                    for du, dv in ((0, 0), (1, 0), (1, 1), (0, 1)):
                        idx = [0, 0, 0]
                        idx[axis], idx[a], idx[b] = side, u + du, v + dv
                        quad.append(vert(*idx))
                    bm.faces.new(quad)
    c = (lo + hi) / 2
    for v in bm.verts:
        p = v.co.copy()
        if rounded:
            inner = Vector([min(max(p[i], lo[i] + rv[i]), hi[i] - rv[i]) for i in range(3)])
            d = p - inner
            dn = Vector([d[i] / rv[i] if rv[i] > 0 else 0.0 for i in range(3)])
            if dn.length > 1e-9:
                dn.normalize()
                p = inner + Vector([dn[i] * rv[i] for i in range(3)])
        if bulge:
            u = [(v.co[i] - c[i]) / half[i] if half[i] > 0 else 0.0 for i in range(3)]
            for key, amount in bulge.items():
                ax = "xyz".index(key[1])
                s = 1.0 if key[0] == "+" else -1.0
                f = max(0.0, u[ax] * s) ** 6
                for o in range(3):
                    if o != ax:
                        f *= max(0.0, 1.0 - u[o] * u[o])
                p[ax] += s * amount * f
        v.co = p
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def lathe(prof, axis="y", c=(0, 0, 0), seg=16, closed=False, phase=0.0):
    """A surface of revolution: `prof` is (radius, position along the axis from `c`) pairs. Open profiles
    are capped; a zero radius closes to a point. `closed` joins the last point back to the first (a ring)."""
    bm = bmesh.new()
    c = Vector(c)

    def at(r, a, t):
        x, y = r * math.cos(t), r * math.sin(t)
        if axis == "y":
            return c + Vector((x, a, -y))
        if axis == "x":
            return c + Vector((a, x, y))
        return c + Vector((x, y, a))

    rings = []
    for r, a in prof:
        if r < 1e-9 and not closed:
            rings.append([bm.verts.new(at(0, a, 0))])
        else:
            rings.append([bm.verts.new(at(r, a, phase + math.tau * k / seg)) for k in range(seg)])
    pairs = list(range(len(rings) - 1)) + ([len(rings) - 1] if closed else [])
    for i in pairs:
        A, B = rings[i], rings[(i + 1) % len(rings)]
        for k in range(seg):
            k1 = (k + 1) % seg
            if len(A) == 1 and len(B) == 1:
                break
            if len(A) == 1:
                bm.faces.new((A[0], B[k], B[k1]))
            elif len(B) == 1:
                bm.faces.new((A[k], A[k1], B[0]))
            else:
                bm.faces.new((A[k], A[k1], B[k1], B[k]))
    if not closed:
        if len(rings[0]) > 1:
            bm.faces.new(list(reversed(rings[0])))
        if len(rings[-1]) > 1:
            bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def cyl(axis, c, r, a0, a1, seg=16, r1=None):
    """A capped cylinder (a cone with `r1`) along `axis` through `c`, from `a0` to `a1` along it."""
    return lathe([(r, a0), (r if r1 is None else r1, a1)], axis, c, seg)


def torus(axis, c, R, r, seg=24, sides=6):
    return lathe([(R + r * math.cos(math.tau * k / sides), r * math.sin(math.tau * k / sides)) for k in range(sides)],
                 axis, c, seg, closed=True)


def fillet(pts, rad, n=2):
    """A polyline with each corner replaced by a quadratic arc of about `rad`."""
    pts = [Vector(p) for p in pts]
    out = [pts[0]]
    for i in range(1, len(pts) - 1):
        P, A, B = pts[i], pts[i - 1], pts[i + 1]
        u, w = (A - P), (B - P)
        d = min(rad, u.length * 0.45, w.length * 0.45)
        p0, p2 = P + u.normalized() * d, P + w.normalized() * d
        for k in range(n + 1):
            t = k / n
            out.append(p0 * (1 - t) ** 2 + P * 2 * t * (1 - t) + p2 * t * t)
    out.append(pts[-1])
    return out


def tube(pts, r, sides=8, rad=None):
    """A capped tube of radius `r` swept along the polyline `pts` (corners filleted by `rad`)."""
    pts = fillet(pts, rad if rad is not None else r * 2.5) if len(pts) > 2 else [Vector(p) for p in pts]
    bm = bmesh.new()
    rings, normal = [], None
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]).normalized()
        if normal is None:
            ref = Vector((0, 1, 0)) if abs(t.y) < 0.9 else Vector((1, 0, 0))
            normal = (ref - t * ref.dot(t)).normalized()
        else:
            normal = (normal - t * normal.dot(t)).normalized()
        bi = t.cross(normal)
        rings.append([bm.verts.new(p + (normal * math.cos(math.tau * k / sides) + bi * math.sin(math.tau * k / sides)) * r)
                      for k in range(sides)])
    for A, B in zip(rings, rings[1:]):
        for k in range(sides):
            k1 = (k + 1) % sides
            bm.faces.new((A[k], A[k1], B[k1], B[k]))
    bm.faces.new(list(reversed(rings[0])))
    bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def prism(poly, axis, a0, a1, bevel=0.0):
    """The 2D polygon `poly` extruded along `axis` from `a0` to `a1`. Coordinates (u, v) are (z, y) across
    x, (x, z) across y and (x, y) across z."""
    def at(u, v, a):
        return Vector((a, v, u)) if axis == "x" else Vector((u, a, v)) if axis == "y" else Vector((u, v, a))
    bm = bmesh.new()
    A = [bm.verts.new(at(u, v, a0)) for u, v in poly]
    B = [bm.verts.new(at(u, v, a1)) for u, v in poly]
    bm.faces.new(A)
    bm.faces.new(list(reversed(B)))
    for i in range(len(poly)):
        j = (i + 1) % len(poly)
        bm.faces.new((A[i], A[j], B[j], B[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    if bevel > 0:
        edges = [e for e in bm.edges if e.calc_face_angle(0) > math.radians(30)]
        bmesh.ops.bevel(bm, geom=edges, offset=bevel, offset_type="OFFSET", segments=1, profile=0.5,
                        affect="EDGES", clamp_overlap=True)
    return bm


def xf(bm, M):
    bm.transform(M)
    return bm


def move(bm, d):
    return xf(bm, Matrix.Translation(Vector(d)))


def rot(bm, axis, deg, pivot=(0, 0, 0)):
    p = Vector(pivot)
    return xf(bm, Matrix.Translation(p) @ Matrix.Rotation(math.radians(deg), 4, axis.upper()) @ Matrix.Translation(-p))


def taper(bm, axis, a0, a1, s1, centre):
    """Scale the cross-section linearly from 1 at `a0` to `s1` at `a1` along `axis` (a CRT's rear)."""
    ax = "xyz".index(axis)
    c = Vector(centre)
    for v in bm.verts:
        t = min(max((v.co[ax] - a0) / (a1 - a0), 0.0), 1.0)
        s = 1 + (s1 - 1) * t
        for o in range(3):
            if o != ax:
                v.co[o] = c[o] + (v.co[o] - c[o]) * s
    return bm


def _object(bm, name):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    return o


def cut(bm, *cutters):
    """Carve `cutters` out of `bm` with Exact booleans (the CSG way); returns the result."""
    target = _object(bm, "csg_target")
    for i, c in enumerate(cutters):
        tool = _object(c, f"CUT_{i}")
        mod = target.modifiers.new(f"cut{i}", "BOOLEAN")
        mod.operation, mod.solver, mod.object = "DIFFERENCE", "EXACT", tool
    dg = bpy.context.evaluated_depsgraph_get()
    out = bmesh.new()
    out.from_object(target.evaluated_get(dg), dg)
    for o in [target] + [bpy.data.objects[f"CUT_{i}"] for i in range(len(cutters))]:
        me = o.data
        bpy.data.objects.remove(o, do_unlink=True)
        bpy.data.meshes.remove(me)
    bmesh.ops.remove_doubles(out, verts=out.verts, dist=1e-6)
    bmesh.ops.dissolve_degenerate(out, edges=out.edges, dist=1e-6)
    return out


# ----------------------------------------------------------------------------- an item

PAT_NONE, PAT_LABEL, PAT_PEG = 0, 1, 2  # printed patterns the bake draws on a part
CHROME = {0xC8CCD0, 0xC4C8CC, 0xB8BCC0, 0xC0C4C8, 0xB0B4B8, 0x8A8E92, 0x9A9A9A, 0xA0A4A8, 0xE0E0E0, 0xA8ACB0,
          0xB08A40, 0x8A8A8A, 0x6A6E72}
TEXEL_GOODS = 0.3  # packaged goods and books: flat print, so 0.4 of the furniture's texel density
TINT_ALBEDO = 0xE4E2DE  # the light neutral a tinted surface is baked in; the game multiplies the tint into it


class Item:
    """The parts of one piece of furniture: geometry, its surface key, its colour, finish and pattern."""

    def __init__(self, name, tint):
        self.name, self.tint, self.parts = name, tint, []

    def add(self, bm, surf, col=WHITE, soft=False, sharp=None, pattern=PAT_NONE, metal=None, texel=None):
        """`texel` weights the part's share of the atlas (packaged goods get less than the furniture)."""
        assert surf in SURFACES, surf
        if metal is None:
            metal = 1.0 if surf == "stainless" or (surf == "metal" and col in CHROME) else 0.0
        if texel is None:
            texel = TEXEL_GOODS if surf == "product" else 1.0
        thr = sharp if sharp is not None else (70 if soft else 32)
        self.parts.append((bm, surf, col, thr, pattern, metal, texel))
        return bm

    def mesh(self):
        """Every part merged into one triangulated mesh in Blender axes: a material slot per surface (for the
        bake), the part colour in `Color` and its finish in `Finish` (R tint mask, G metallic, B pattern / 10,
        A texel weight)."""
        surfs = []
        for p in self.parts:
            if p[1] not in surfs:
                surfs.append(p[1])
        out = bmesh.new()
        out.loops.layers.uv.new("UVMap")
        colour = out.loops.layers.float_color.new("Color")
        finish = out.loops.layers.float_color.new("Finish")
        C = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))  # glTF to Blender
        for bm, surf, col, thr, pattern, metal, texel in self.parts:
            # flat runs of a rounded box's grid become single faces; floor-contact faces are never seen
            keep = bm.copy()
            area = sum(f.calc_area() for f in bm.faces)
            bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(0.5), verts=bm.verts[:], edges=bm.edges[:])
            bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="FIXED", ngon_method="EAR_CLIP")
            if abs(sum(f.calc_area() for f in bm.faces) - area) > 2e-3 * max(area, 1e-6):
                # round a boolean's opening the dissolve can join the faces into one that wraps the opening, and
                # triangulating it fills the opening (the area grows): keep that part as it was
                bm.free()
                bm = keep
                bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="FIXED", ngon_method="EAR_CLIP")
            else:
                keep.free()
            bm.normal_update()
            down = [f for f in bm.faces if f.normal.y < -0.99 and all(v.co.y < 0.002 for v in f.verts)]
            bmesh.ops.delete(bm, geom=down, context="FACES_ONLY")
            bm.verts.index_update()
            bm.edges.index_update()
            sharp = {e.index for e in bm.edges if not e.is_manifold or e.calc_face_angle(0) > math.radians(thr)}
            if os.environ.get("FURN_DEBUG"):
                print("  part", surf, f"{col:06x}", sum(len(f.verts) - 2 for f in bm.faces))
            vmap = [out.verts.new(C @ v.co) for v in bm.verts]
            tinted = surf == self.tint
            rgba = lin(TINT_ALBEDO if tinted else col)
            fin = (1.0 if tinted else 0.0, metal, pattern / 10, texel)
            for f in bm.faces:
                nf = out.faces.new([vmap[v.index] for v in f.verts])
                nf.material_index = surfs.index(surf)
                nf.smooth = True
                for loop in nf.loops:
                    loop[colour] = rgba
                    loop[finish] = fin
            for e in bm.edges:
                if e.index in sharp:
                    ne = out.edges.get((vmap[e.verts[0].index], vmap[e.verts[1].index]))
                    if ne is not None:
                        ne.smooth = False
            bm.free()
        bmesh.ops.triangulate(out, faces=out.faces, quad_method="FIXED", ngon_method="EAR_CLIP")
        # bmesh's operators may hand back faces in a different order from run to run: put vertices and faces in
        # an order set by their positions alone, so the glb is the same bytes every time
        def canonical(seq, key):
            seq.index_update()
            order = sorted(range(len(seq)), key=lambda i: key(seq[i]))
            rank = [0] * len(seq)
            for r, i in enumerate(order):
                rank[i] = r
            seq.sort(key=lambda e: rank[e.index])
            seq.index_update()
        out.verts.ensure_lookup_table()
        canonical(out.verts, lambda v: (round(v.co.x, 6), round(v.co.y, 6), round(v.co.z, 6)))
        out.faces.ensure_lookup_table()
        canonical(out.faces, lambda f: (f.material_index, sorted(v.index for v in f.verts)))
        return out, surfs


# ----------------------------------------------------------------------------- the items
# Each builder models its item in the block's local frame: metres, y up, front toward +z.

def sofa(I):
    wood = 0x4A3020
    for x in (-0.92, 0.92):
        for z in (-0.37, 0.37):
            I.add(lathe([(0.02, 0.0), (0.028, 0.085), (0.0, 0.085)], "y", (x, 0, z), seg=10), "wood", wood, sharp=50)
    for s in (-1, 1):  # rolled, padded arms
        x0, x1 = (0.80, 1.0) if s > 0 else (-1.0, -0.80)
        I.add(box((x0, 0.08, -0.45), (x1, 0.56, 0.45), 0.05, 2, (1, 2, 4)), "fabric", soft=True)
        I.add(box((x0 - 0.01, 0.50, -0.45), (x1 + 0.0, 0.64, 0.44), 0.065, 3, (1, 0, 4), {"+y": 0.01}), "fabric", soft=True)
    I.add(box((-0.81, 0.08, -0.40), (0.81, 0.31, 0.43), 0.03, 2, (4, 1, 1)), "fabric", soft=True)  # base, skirted
    I.add(box((-0.81, 0.08, -0.45), (0.81, 0.80, -0.29), 0.05, 2, (4, 2, 0)), "fabric", soft=True)  # back frame
    for x0, x1 in ((-0.80, -0.004), (0.004, 0.80)):
        I.add(box((x0, 0.30, -0.30), (x1, 0.455, 0.445), 0.05, 2, (3, 0, 3), {"+y": 0.018, "+z": 0.008}), "fabric", soft=True)
        b = box((x0 + 0.005, 0.42, -0.33), (x1 - 0.005, 0.83, -0.15), 0.06, 2, (3, 3, 0), {"+z": 0.022})
        I.add(rot(b, "x", -9, (0, 0.42, -0.24)), "fabric", soft=True)
        for k in range(3):  # tufting buttons on the back cushions
            bx = x0 + (x1 - x0) * (k + 1) / 4
            bt = lathe([(0.012, 0.0), (0.01, 0.006), (0.0, 0.008)], "z", (bx, 0.66, -0.13), seg=8)
            I.add(rot(bt, "x", -9, (0, 0.42, -0.24)), "fabric", soft=True)


def bed(I):
    wood, dark, linen = 0x6A4428, 0x5A3820, 0xF2EFE8
    for s in (-1, 1):  # headboard posts with turned finials
        I.add(lathe([(0.034, 0.0), (0.034, 0.93), (0.044, 0.94), (0.044, 0.97), (0.028, 0.985), (0.03, 1.0),
                     (0.04, 1.02), (0.03, 1.04), (0.0, 1.05)], "y", (s * 0.758, 0, -1.026), seg=12), "wood", wood, sharp=50)
        I.add(box((s * 0.72 - 0.04, 0.0, 0.92), (s * 0.72 + 0.04, 0.50, 1.0), 0.008, 1), "wood", wood)  # foot posts
        I.add(lathe([(0.03, 0.5), (0.034, 0.515), (0.0, 0.535)], "y", (s * 0.72, 0, 0.96), seg=10), "wood", wood, sharp=50)
        I.add(box((s * 0.76 - 0.02, 0.20, -1.0), (s * 0.76 + 0.02, 0.36, 0.93), 0.006, 1), "wood", dark)  # side rails
    I.add(box((-0.73, 0.88, -1.05), (0.73, 0.97, -1.0), 0.008, 1), "wood", wood)  # top rail
    I.add(box((-0.73, 0.30, -1.05), (0.73, 0.40, -1.0), 0.008, 1), "wood", wood)  # bottom rail
    I.add(box((-0.73, 0.39, -1.045), (0.73, 0.89, -1.015)), "wood", dark)  # panel field
    for k in range(3):  # raised panels
        x0 = -0.70 + k * 0.47
        I.add(box((x0, 0.44, -1.02), (x0 + 0.44, 0.84, -1.004), 0.012, 2), "wood", wood)
    I.add(box((-0.73, 0.22, 0.935), (0.73, 0.42, 0.985), 0.008, 1), "wood", wood)  # footboard
    I.add(box((-0.79, 0.25, -0.99), (0.79, 0.59, 0.93), 0.07, 3, (3, 1, 5), {"+y": 0.012}), "fabric", soft=True)  # spread
    I.add(box((-0.795, 0.555, -0.66), (0.795, 0.618, -0.48), 0.028, 2, (3, 0, 0)), "soffit", linen, soft=True)  # turn-down
    for x0, x1 in ((-0.72, -0.03), (0.03, 0.72)):
        p = box((x0, 0.56, -0.98), (x1, 0.67, -0.66), 0.05, 2, (2, 0, 2), {"+y": 0.03})
        I.add(rot(p, "x", -8, (0, 0.56, -0.98)), "soffit", linen, soft=True)


def table(I):
    top, legs, chairs = 0x8A5A36, 0x6A4428, 0x7A5232
    I.add(box((-0.60, 0.72, -0.45), (0.60, 0.765, 0.45), 0.012, 2, (2, 0, 2)), "wood", top)
    for x in (-0.51, 0.51):
        I.add(box((x - 0.012, 0.62, -0.34), (x + 0.012, 0.725, 0.34)), "wood", legs)
    for z in (-0.365, 0.365):
        I.add(box((-0.50, 0.62, z - 0.012), (0.50, 0.725, z + 0.012)), "wood", legs)
    for x in (-0.52, 0.52):
        for z in (-0.375, 0.375):
            I.add(lathe([(0.022, 0.0), (0.028, 0.02), (0.02, 0.1), (0.026, 0.34), (0.018, 0.44), (0.028, 0.53),
                         (0.031, 0.6), (0.031, 0.725)], "y", (x, 0, z), seg=10), "wood", legs, sharp=50)
    for flip in (False, True):  # a chair at each end, facing the table
        parts = []
        parts.append(box((-0.20, 0.43, 0.55), (0.20, 0.47, 0.84), 0.012, 2))
        for x in (-0.175, 0.175):
            parts.append(box((x - 0.016, 0.0, 0.565), (x + 0.016, 0.435, 0.597)))
            parts.append(box((x - 0.016, 0.0, 0.84), (x + 0.016, 0.95, 0.872), 0.005, 1))
            parts.append(box((x - 0.008, 0.14, 0.597), (x + 0.008, 0.16, 0.84)))
        parts.append(box((-0.16, 0.14, 0.70), (0.16, 0.16, 0.716)))
        parts.append(box((-0.19, 0.80, 0.838), (0.19, 0.93, 0.874), 0.01, 2))
        parts.append(box((-0.19, 0.52, 0.842), (0.19, 0.56, 0.870), 0.006, 1))
        for x in (-0.09, 0.0, 0.09):
            parts.append(cyl("y", (x, 0, 0.856), 0.011, 0.555, 0.805, seg=8))
        for p in parts:
            I.add(rot(p, "y", 180) if flip else p, "wood", chairs, sharp=50)


def kitchen(I):
    chrome, top, steel, almond, black = 0xC4C8CC, 0xE8E2D0, 0xD8DCDE, 0xD9D6CC, 0x2A2A2A
    I.add(box((-1.80, 0.0, -0.30), (1.06, 0.10, 0.22)), "trim")  # toe kick
    I.add(box((-1.80, 0.095, -0.30), (1.06, 0.885, 0.27)), "trim")  # base carcass
    w = 2.86 / 6
    for i in range(6):
        x0, x1 = -1.80 + i * w + 0.005, -1.80 + (i + 1) * w - 0.005
        xc = (x0 + x1) / 2
        sink = i in (2, 3)
        I.add(box((x0, 0.71, 0.268), (x1, 0.865, 0.288), 0.006, 1), "trim")  # drawer or false front
        I.add(box((x0 + 0.04, 0.735, 0.284), (x1 - 0.04, 0.84, 0.294), 0.006, 1), "trim")
        if not sink:
            I.add(tube([(xc - 0.06, 0.787, 0.29), (xc - 0.06, 0.787, 0.31), (xc + 0.06, 0.787, 0.31), (xc + 0.06, 0.787, 0.29)],
                       0.005, 6, 0.01), "metal", chrome, sharp=60)
        I.add(box((x0, 0.115, 0.268), (x1, 0.695, 0.288), 0.006, 1), "trim")  # door
        I.add(box((x0 + 0.05, 0.17, 0.284), (x1 - 0.05, 0.64, 0.296), 0.008, 1), "trim")  # raised panel
        hx = x1 - 0.03 if i % 2 == 0 else x0 + 0.03
        I.add(tube([(hx, 0.56, 0.288), (hx, 0.56, 0.305), (hx, 0.66, 0.305), (hx, 0.66, 0.288)], 0.005, 6, 0.01), "metal", chrome, sharp=60)
    slab = box((-1.82, 0.88, -0.32), (1.07, 0.92, 0.32), 0.006, 2)
    I.add(cut(slab, box((-0.555, 0.85, -0.205), (-0.045, 0.95, 0.165), 0.03, 2)), "counter", top)
    I.add(box((-1.82, 0.915, -0.32), (1.07, 1.02, -0.30), 0.004, 1), "counter", top)  # backsplash
    basin = cut(box((-0.57, 0.72, -0.22), (-0.03, 0.93, 0.18), 0.012, 2),
                box((-0.55, 0.74, -0.20), (-0.31, 1.0, 0.16), 0.03, 2),
                box((-0.29, 0.74, -0.20), (-0.05, 1.0, 0.16), 0.03, 2))
    I.add(basin, "stainless", steel)
    for x in (-0.43, -0.17):
        I.add(cyl("y", (x, 0, -0.02), 0.03, 0.738, 0.745, seg=12), "stainless", 0x8A8E92)
    I.add(box((-0.36, 0.925, -0.29), (-0.24, 0.95, -0.21), 0.008, 2), "stainless", steel)  # faucet deck
    I.add(tube([(-0.30, 0.95, -0.25), (-0.30, 1.12, -0.25), (-0.30, 1.12, -0.10), (-0.30, 1.08, -0.10)], 0.011, 8, 0.05), "stainless", steel, sharp=60)
    for x in (-0.41, -0.19):
        I.add(lathe([(0.018, 0.925), (0.016, 0.95), (0.022, 0.965), (0.0, 0.975)], "y", (x, 0, -0.25), seg=10), "stainless", steel, sharp=60)
    I.add(box((-1.80, 1.45, -0.30), (1.00, 2.15, -0.002)), "trim")  # upper carcass
    w = 2.80 / 6
    for i in range(6):
        x0, x1 = -1.80 + i * w + 0.005, -1.80 + (i + 1) * w - 0.005
        I.add(box((x0, 1.465, -0.005), (x1, 2.135, 0.015), 0.006, 1), "trim")
        I.add(box((x0 + 0.05, 1.52, 0.011), (x1 - 0.05, 2.08, 0.023), 0.008, 1), "trim")
        hx = x1 - 0.03 if i % 2 == 0 else x0 + 0.03
        I.add(lathe([(0.008, 0.0), (0.012, 0.012), (0.0, 0.02)], "z", (hx, 1.50, 0.015), seg=8), "metal", chrome, sharp=60)
    # the fridge: almond, freezer on top, chrome pulls on the left (hinged right)
    I.add(box((1.10, 0.0, -0.32), (1.80, 1.79, 0.285), 0.03, 2), "metal", almond)
    I.add(box((1.112, 1.225, 0.27), (1.788, 1.775, 0.315), 0.016, 2), "metal", almond)
    I.add(box((1.112, 0.10, 0.27), (1.788, 1.205, 0.315), 0.016, 2), "metal", almond)
    I.add(tube([(1.16, 1.30, 0.31), (1.16, 1.30, 0.34), (1.16, 1.62, 0.34), (1.16, 1.62, 0.31)], 0.011, 8, 0.03), "metal", chrome, sharp=60)
    I.add(tube([(1.16, 0.72, 0.31), (1.16, 0.72, 0.34), (1.16, 1.12, 0.34), (1.16, 1.12, 0.31)], 0.011, 8, 0.03), "metal", chrome, sharp=60)
    I.add(box((1.12, 0.012, 0.25), (1.78, 0.09, 0.29), 0.004, 1), "metal", black)  # kick grille
    for k in range(7):
        x = 1.17 + k * 0.09
        I.add(box((x, 0.025, 0.288), (x + 0.06, 0.078, 0.296)), "metal", 0x55585A)


def tv(I):
    wood, plastic, dark = 0x5A3A22, 0x2A2A2A, 0x141414
    stand = box((-0.55, 0.045, -0.25), (0.55, 0.55, 0.25), 0.01, 2)
    I.add(cut(stand, box((-0.51, 0.085, -0.21), (0.51, 0.29, 0.40)), box((-0.51, 0.33, -0.21), (0.51, 0.51, 0.40))), "wood", wood)
    for x in (-0.5, 0.5):
        for z in (-0.2, 0.2):
            I.add(lathe([(0.02, 0.0), (0.025, 0.035), (0.03, 0.048)], "y", (x, 0, z), seg=10), "metal", 0xB08A40, sharp=60)
    # a VCR on the upper shelf
    I.add(box((-0.22, 0.33, -0.17), (0.22, 0.415, 0.17), 0.006, 1), "metal", 0x303030)
    I.add(box((-0.17, 0.355, 0.165), (0.05, 0.395, 0.176), 0.003, 1), "metal", dark)
    I.add(box((0.08, 0.38, 0.166), (0.19, 0.402, 0.175)), "glassDark", 0x203a34)
    for k in range(5):
        x = 0.08 + k * 0.023
        I.add(box((x, 0.352, 0.168), (x + 0.017, 0.364, 0.178)), "metal", 0x8A8A8A)
    # tapes on the lower shelf
    rnd = random.Random(7)
    x = -0.46
    for k in range(7):
        I.add(box((x, 0.086, -0.10), (x + 0.028, 0.29 - 0.08, 0.09)), "product", [0x1A1A1A, 0x22303A, 0xE8E4D8][rnd.randrange(3)])
        x += 0.031
    I.add(box((0.10, 0.086, -0.12), (0.40, 0.15, 0.14), 0.006, 1), "metal", 0x3A3A3A)  # a game console
    I.add(box((0.14, 0.15, 0.06), (0.24, 0.158, 0.12)), "metal", 0x1A1A1A)
    # the set: a bezel with the screen recessed in it, a tapered rear, dials and a speaker grille
    bezel = box((-0.38, 0.55, -0.03), (0.38, 1.07, 0.20), 0.03, 2)
    I.add(cut(bezel, box((-0.335, 0.615, 0.165), (0.175, 1.005, 0.30), 0.035, 2)), "metal", plastic)
    rear = box((-0.33, 0.58, -0.25), (0.33, 1.03, -0.02), 0.04, 2, (1, 1, 2))
    I.add(taper(rear, "z", -0.03, -0.25, 0.62, (0, 0.80, 0)), "metal", plastic)
    I.add(box((-0.335, 0.615, 0.145), (0.175, 1.005, 0.172), 0.035, 2, (4, 4, 0), {"+z": 0.016}), "glassDark", 0x2E3C44, soft=True)
    for y, r in ((0.95, 0.034), (0.865, 0.028)):
        I.add(lathe([(r, 0.20), (r, 0.214), (r * 0.85, 0.228), (r * 0.55, 0.232), (0.0, 0.233)], "z", (0.285, y, 0), seg=16), "metal", 0x9A9A9A, sharp=50)
        I.add(box((0.282, y - 0.006, 0.228), (0.288, y + r * 0.7, 0.236)), "metal", 0xE0E0E0)
    for k in range(7):
        y = 0.625 + k * 0.022
        I.add(box((0.215, y, 0.197), (0.355, y + 0.011, 0.205)), "metal", dark)
    I.add(box((0.23, 0.79, 0.198), (0.27, 0.81, 0.21), 0.003, 1), "metal", 0xC8C8C8)
    I.add(box((0.31, 0.795, 0.199), (0.32, 0.805, 0.206)), "lamp", 0xFF3020)


def shelf(I):
    wood, back = 0x7A5232, 0x5A3A22
    for s in (-1, 1):
        I.add(box((s * 0.89 - 0.011, 0.0, -0.18), (s * 0.89 + 0.011, 1.885, 0.175), 0.004, 1), "wood", wood)
    I.add(box((-0.905, 1.88, -0.18), (0.905, 1.905, 0.185), 0.006, 2), "wood", wood)
    I.add(box((-0.895, 1.835, 0.15), (0.895, 1.882, 0.178), 0.006, 1), "wood", wood)  # crown
    I.add(box((-0.88, 0.0, 0.135), (0.88, 0.085, 0.16)), "wood", back)  # plinth
    I.add(box((-0.883, 0.08, -0.18), (0.883, 1.885, -0.165)), "wood", back)
    levels = [0.08, 0.44, 0.80, 1.16, 1.52]
    for y in levels:
        I.add(box((-0.883, y, -0.166), (0.883, y + 0.022, 0.17), 0.004, 1), "wood", wood)
    cloth = [0x6A1E22, 0x1E2E5A, 0x2E4A2A, 0xC8A040, 0xE8E0C8, 0x5A3A22, 0x1A1A1A, 0x8A3A1A, 0x3A5A6A, 0xB0503A]
    rnd = random.Random(1983)
    for li, y0 in enumerate(levels):
        y0 += 0.022
        room = (1.835 if li == 4 else levels[li + 1]) - y0 - 0.02
        x, x_end = -0.875, 0.875
        if li == 1:  # a vase and a photo frame share this shelf
            I.add(lathe([(0.0, 0.0), (0.045, 0.0), (0.06, 0.06), (0.05, 0.14), (0.025, 0.2), (0.03, 0.23), (0.0, 0.22)],
                        "y", (0.66, y0, 0.0), seg=14), "product", 0x2F6F6A, sharp=60)
            I.add(rot(box((0.41, y0, -0.02), (0.55, y0 + 0.18, 0.0), 0.004, 1), "x", -10, (0, y0, 0)), "wood", 0x2A1A10)
            x_end = 0.36
        if li == 3:  # a stack lying flat
            yy = y0
            for k in range(5):
                h = rnd.uniform(0.025, 0.04)
                I.add(box((0.45 + rnd.uniform(-0.01, 0.01), yy, -0.14), (0.75 + rnd.uniform(-0.01, 0.01), yy + h, 0.06)), "product", cloth[rnd.randrange(10)])
                yy += h
            x_end = 0.42
        while x < x_end - 0.02:
            wb = rnd.uniform(0.022, 0.05)
            hb = min(room, rnd.uniform(0.19, 0.30))
            db = rnd.uniform(0.15, 0.22)
            if x + wb > x_end:
                break
            I.add(box((x, y0, -0.16), (x + wb, y0 + hb, -0.16 + db)), "product", cloth[rnd.randrange(10)])
            x += wb + rnd.choice([0.0, 0.0, 0.0, 0.002])
        if li in (0, 2) and x < 0.80:  # the last one leans
            b = box((0, 0, -0.16), (0.03, 0.26, 0.04))
            rot(b, "z", -20, (0, 0, 0))
            I.add(move(b, (x + 0.005, y0, 0)), "product", cloth[rnd.randrange(10)])


def desk(I):
    top, body, brass, beige, keys = 0x6A4A30, 0x5A3A22, 0xB08A40, 0xD8D0C0, 0xE8E0D0
    I.add(box((-0.75, 0.72, -0.40), (0.75, 0.76, 0.40), 0.008, 2, (3, 0, 1)), "wood", top)
    I.add(box((-0.75, 0.0, -0.38), (-0.30, 0.722, 0.38), 0.004, 1), "wood", body)  # pedestal
    for y0, y1 in ((0.47, 0.70), (0.25, 0.455), (0.03, 0.235)):
        I.add(box((-0.74, y0, 0.375), (-0.31, y1, 0.393), 0.005, 1), "wood", top)
        yc = (y0 + y1) / 2 + 0.03
        I.add(tube([(-0.585, yc, 0.39), (-0.585, yc, 0.41), (-0.465, yc, 0.41), (-0.465, yc, 0.39)], 0.005, 6, 0.01), "metal", brass, sharp=60)
    I.add(box((0.70, 0.0, -0.38), (0.74, 0.722, 0.38), 0.004, 1), "wood", body)  # end panel
    I.add(box((-0.30, 0.30, -0.38), (0.70, 0.722, -0.36), 0.003, 1), "wood", body)  # modesty panel
    I.add(box((-0.28, 0.655, 0.36), (0.68, 0.715, 0.38), 0.004, 1), "wood", top)  # pencil drawer
    # an 80s home computer: system unit, monitor, keyboard
    I.add(box((-0.20, 0.76, -0.32), (0.26, 0.86, 0.04), 0.008, 2), "metal", beige)
    for y in (0.775, 0.81):
        I.add(box((0.02, y, 0.034), (0.22, y + 0.018, 0.044), 0.002, 1), "metal", 0x2A2A2A)
        I.add(box((0.17, y + 0.004, 0.04), (0.19, y + 0.014, 0.048)), "metal", 0x8A8478)
    I.add(box((-0.17, 0.79, 0.038), (-0.15, 0.80, 0.046)), "lamp", 0xFF3020)
    bezel = box((-0.18, 0.865, -0.20), (0.18, 1.14, 0.03), 0.018, 2)
    I.add(cut(bezel, box((-0.14, 0.90, 0.005), (0.14, 1.105, 0.10), 0.02, 2)), "metal", beige)
    rear = box((-0.16, 0.875, -0.31), (0.16, 1.12, -0.19), 0.02, 1)
    I.add(taper(rear, "z", -0.19, -0.31, 0.7, (0, 1.0, 0)), "metal", beige)
    I.add(box((-0.14, 0.90, -0.01), (0.14, 1.105, 0.008), 0.02, 2, (3, 3, 0), {"+z": 0.01}), "glassDark", 0x22342A, soft=True)
    I.add(box((-0.22, 0.76, 0.08), (0.22, 0.786, 0.26), 0.006, 1), "metal", 0xC8BFAE)
    for row in range(4):
        z = 0.105 + row * 0.033
        for k in range(14):
            x = -0.205 + k * 0.0295
            I.add(box((x, 0.785, z), (x + 0.025, 0.797, z + 0.026)), "metal", 0x8A8478 if k in (0, 13) else keys)
    I.add(box((-0.10, 0.785, 0.237), (0.10, 0.796, 0.256)), "metal", keys)
    I.add(lathe([(0.0, 0.0), (0.038, 0.0), (0.04, 0.09), (0.036, 0.09), (0.034, 0.008), (0.0, 0.008)], "y", (0.5, 0.76, 0.1), seg=14),
          "product", 0xC23B30, sharp=60)  # a mug
    # the office chair, facing the desk
    cz = 0.72
    for k in range(5):
        a = 36 + k * 72
        arm = box((-0.016, 0.055, 0.02), (0.016, 0.085, 0.205), 0.006, 1)
        caster = cyl("x", (0, 0.026, 0.19), 0.026, -0.012, 0.012, seg=10)
        I.add(move(rot(arm, "y", a), (0, 0, cz)), "metal", 0x1E1E1E)
        I.add(move(rot(caster, "y", a), (0, 0, cz)), "rubber", 0x1A1A1A, sharp=60)
    I.add(lathe([(0.0, 0.05), (0.05, 0.05), (0.05, 0.095), (0.026, 0.11), (0.026, 0.33), (0.0, 0.33)], "y", (0, 0, cz), seg=14),
          "metal", 0x2A2A2A, sharp=60)
    I.add(cyl("y", (0, 0, cz), 0.02, 0.33, 0.40, seg=12), "metal", 0xC8CCD0, sharp=60)
    I.add(box((-0.10, 0.38, cz - 0.10), (0.10, 0.415, cz + 0.12), 0.006, 1), "metal", 0x2A2A2A)
    I.add(box((-0.24, 0.40, 0.50), (0.24, 0.49, 0.92), 0.035, 2, (2, 0, 2), {"+y": 0.012}), "fabric", 0x3A3A3A, soft=True)
    I.add(tube([(0, 0.40, 0.84), (0, 0.42, 0.935), (0, 0.62, 0.935)], 0.018, 8, 0.05), "metal", 0x2A2A2A, sharp=60)
    back = box((-0.21, 0.56, 0.875), (0.21, 0.92, 0.94), 0.03, 2, (2, 2, 0), {"-z": 0.012})
    I.add(rot(back, "x", 6, (0, 0.56, 0.9)), "fabric", 0x3A3A3A, soft=True)


def aisle(I):
    frame, shelfc, red = 0xD8D8D8, 0xCFCFCF, 0xB02A24
    I.add(box((-2.0, 0.0, -0.45), (2.0, 0.12, 0.45), 0.006, 1), "metal", frame)
    I.add(box((-1.99, 0.0, -0.44), (1.99, 0.025, 0.44)), "metal", 0x3A3A3A)  # toe shadow strip
    I.add(box((-1.98, 0.118, -0.035), (1.98, 1.55, 0.035)), "metal", shelfc)  # back panels
    for k in range(13):  # pegboard ribs
        y = 0.25 + k * 0.1
        for s in (-1, 1):
            I.add(box((-1.98, y, s * 0.035 - 0.003), (1.98, y + 0.008, s * 0.035 + 0.003)), "metal", 0xBDBDBD)
    I.add(box((-2.0, 1.545, -0.05), (2.0, 1.60, 0.05), 0.008, 2), "metal", red)  # top cap
    for x in (-1.98, -0.66, 0.66, 1.98):
        I.add(box((x - 0.022, 0.118, -0.048), (x + 0.022, 1.548, 0.048)), "metal", 0x9A9EA2)
    palette = [0xC23B30, 0xE8B23A, 0x3B67C4, 0x2F8A48, 0xF2F0EA, 0xE07A2A, 0x6A2A6A, 0x1E3A6A]
    rnd = random.Random(1984)
    for lv in (0.15, 0.60, 1.05):
        for s in (-1, 1):
            I.add(box((-1.978, lv - 0.03, min(s * 0.036, s * 0.44)), (1.978, lv, max(s * 0.036, s * 0.44))), "metal", shelfc)
            z0, z1 = sorted((s * 0.437, s * 0.452))
            I.add(box((-1.978, lv - 0.055, z0), (1.978, lv + 0.012, z1), 0.003, 1), "metal", 0xF2F2F2)  # price channel
            for x in (-1.98, -0.66, 0.66, 1.98):  # brackets
                I.add(prism([(0.04 * s, lv - 0.03), (0.40 * s, lv - 0.03), (0.04 * s, lv - 0.13)], "x", x - 0.004, x + 0.004), "metal", 0x9A9EA2)
            cap = 0.38 if lv < 1.0 else 0.42
            x = -1.95
            while x < 1.90:
                kind = rnd.choices(["box", "cans", "bottles", "bags"], [0.55, 0.17, 0.12, 0.16])[0]
                c = palette[rnd.randrange(len(palette))]
                fz = s * 0.415  # the front face of the stock
                inner = s * 0.06

                def zr(a, b):
                    return (min(a, b), max(a, b))
                if kind == "box":
                    w, h = rnd.uniform(0.17, 0.25), rnd.uniform(0.22, cap)
                    if x + w > 1.95:
                        break
                    za, zb = zr(fz, inner)
                    I.add(box((x, lv, za), (x + w, lv + h, zb)), "product", c, pattern=PAT_LABEL)
                    x += w + 0.012
                elif kind == "cans":
                    w = 0.21
                    if x + w > 1.95:
                        break
                    for k in range(3):
                        cx = x + 0.035 + k * 0.07
                        I.add(cyl("y", (cx, 0, fz - s * 0.035), 0.033, lv, lv + 0.12, seg=8), "product", c, sharp=50, pattern=PAT_LABEL)
                    za, zb = zr(fz - s * 0.07, inner)
                    I.add(box((x, lv, za), (x + w, lv + 0.12, zb)), "product", c)
                    x += w + 0.012
                elif kind == "bottles":
                    w = 0.22
                    if x + w > 1.95:
                        break
                    for k in range(3):
                        I.add(lathe([(0.034, 0.0), (0.034, 0.17), (0.014, 0.23), (0.013, 0.27), (0.0, 0.27)], "y",
                                    (x + 0.037 + k * 0.073, lv, fz - s * 0.04), seg=8), "product", c, sharp=50, pattern=PAT_LABEL)
                    za, zb = zr(fz - s * 0.08, inner)
                    I.add(box((x, lv, za), (x + w, lv + 0.17, zb)), "product", c)
                    x += w + 0.012
                else:
                    w = rnd.uniform(0.22, 0.30)
                    if x + w > 1.95:
                        break
                    za, zb = zr(fz, inner + s * 0.05)
                    I.add(box((x, lv, za), (x + w, lv + min(cap, 0.32), zb), 0.03, 1), "product", c, soft=True, pattern=PAT_LABEL)
                    x += w + 0.012


def cooler(I):
    white, alu, chrome = 0xE8E8E8, 0xB8BCC0, 0xC8CCD0
    shell = box((-1.5, 0.0, -0.40), (1.5, 2.10, 0.375), 0.01, 1)
    I.add(cut(shell, box((-1.44, 0.27, -0.34), (1.44, 1.93, 0.50))), "metal", white)
    for y in (0.30, 0.70, 1.10, 1.50):
        I.add(box((-1.44, y, -0.335), (1.44, y + 0.015, 0.31)), "metal", 0xA8ACB0)
    I.add(cyl("x", (0, 1.90, 0.25), 0.016, -1.40, 1.40, seg=8), "lamp", 0xFFFFFF, sharp=50)
    I.add(box((-1.46, 1.955, 0.37), (1.46, 2.08, 0.392), 0.004, 1), "lamp", 0xFFF2DC)  # lit header sign
    I.add(box((-1.48, 0.02, 0.35), (1.48, 0.245, 0.38)), "metal", 0x3A3A3A)  # base grille
    for k in range(8):
        y = 0.04 + k * 0.025
        I.add(box((-1.46, y, 0.378), (1.46, y + 0.012, 0.388)), "metal", 0x6A6A6A)
    rnd = random.Random(1985)
    drinks = [0x3A1A10, 0x2F8A48, 0xE07A2A, 0xC23B30, 0x3B67C4, 0xE8B23A, 0xF2F0EA]
    for i in range(3):
        x0, x1 = -1.44 + i * 0.96 + 0.004, -1.44 + (i + 1) * 0.96 - 0.004
        frame = box((x0, 0.26, 0.375), (x1, 1.94, 0.41), 0.006, 1)
        I.add(cut(frame, box((x0 + 0.045, 0.31, 0.30), (x1 - 0.045, 1.89, 0.50))), "metal", alu)
        xh = x1 - 0.07 if i < 2 else x0 + 0.07
        I.add(tube([(xh, 0.80, 0.41), (xh, 0.80, 0.44), (xh, 1.40, 0.44), (xh, 1.40, 0.41)], 0.012, 8, 0.03), "metal", chrome, sharp=60)
        for y in (0.315, 0.715, 1.115, 1.515):
            x = x0 + 0.05
            while x < x1 - 0.06:
                kind = rnd.choices(["bottles", "carton", "cans"], [0.45, 0.3, 0.25])[0]
                c = drinks[rnd.randrange(len(drinks))]
                if kind == "bottles":
                    if x + 0.11 > x1 - 0.05:
                        break
                    I.add(lathe([(0.05, 0.0), (0.05, 0.22), (0.02, 0.30), (0.016, 0.33), (0.0, 0.33)], "y",
                                (x + 0.055, y, 0.22), seg=8), "product", c, sharp=50, pattern=PAT_LABEL)
                    I.add(box((x + 0.005, y, -0.30), (x + 0.105, y + 0.30, 0.16)), "product", c)
                    x += 0.115
                elif kind == "carton":
                    if x + 0.14 > x1 - 0.05:
                        break
                    I.add(box((x, y, -0.25), (x + 0.135, y + 0.245, 0.27), 0.004, 1), "product", c, pattern=PAT_LABEL)
                    x += 0.145
                else:
                    if x + 0.14 > x1 - 0.05:
                        break
                    for k in range(2):
                        I.add(cyl("y", (x + 0.034 + k * 0.068, 0, 0.235), 0.032, y, y + 0.12, seg=8), "product", c, sharp=50, pattern=PAT_LABEL)
                    I.add(box((x, y, -0.30), (x + 0.136, y + 0.24, 0.20)), "product", c)
                    x += 0.145


def counter(I):
    chrome, top, black = 0xC8CCD0, 0xD8CDB0, 0x2A2A2A
    body = box((-1.50, 0.08, -0.35), (1.50, 1.0, 0.35))
    I.add(cut(body, box((-1.44, 0.12, -0.45), (1.44, 0.93, -0.06))), "trim")  # the clerk's open shelves
    I.add(box((-1.44, 0.50, -0.35), (1.44, 0.52, -0.06)), "trim")
    I.add(box((-1.48, 0.0, -0.33), (1.48, 0.085, 0.31)), "metal", black)  # kick
    for k in range(3):
        x0 = -1.44 + k * 0.96 + 0.01
        I.add(box((x0, 0.15, 0.345), (x0 + 0.94, 0.91, 0.362), 0.006, 1), "trim")
        I.add(box((x0 + 0.06, 0.21, 0.358), (x0 + 0.88, 0.85, 0.372), 0.01, 1), "trim")
    I.add(box((-1.505, 0.92, 0.345), (1.505, 0.965, 0.375), 0.008, 2), "trim")  # moulding
    I.add(box((-1.555, 0.995, -0.405), (1.555, 1.035, 0.405), 0.004, 1), "metal", chrome)  # edge band
    I.add(box((-1.55, 1.0, -0.40), (1.55, 1.045, 0.40), 0.004, 1), "counter", top)
    # the cash register: drawer base, sloped keyboard, customer display, receipt printer
    reg = 0x3A3A3A
    I.add(box((0.40, 1.04, -0.20), (0.85, 1.15, 0.15), 0.008, 2), "metal", reg)
    I.add(box((0.42, 1.055, -0.212), (0.83, 1.135, -0.195), 0.004, 1), "metal", 0x2A2A2A)
    I.add(cyl("z", (0.625, 1.115, 0), 0.008, -0.218, -0.21, seg=8), "metal", chrome, sharp=50)
    I.add(prism([(-0.20, 1.148), (0.10, 1.148), (0.10, 1.27), (-0.20, 1.19)], "x", 0.40, 0.85, bevel=0.004), "metal", reg)
    slope = math.atan2(0.08, 0.30)
    cols = {0: 0xE8E2D0, 1: 0xE8E2D0, 2: 0xE8E2D0, 3: 0xC23B30, 4: 0x2F8A48, 5: 0x8A8A8A}
    for row in range(5):
        for col in range(6):
            key = box((-0.026, 0.0, -0.022), (0.026, 0.014, 0.022), 0.003, 1) if col == 5 else box((-0.022, 0.0, -0.022), (0.022, 0.014, 0.022), 0.003, 1)
            rot(key, "x", -math.degrees(slope))
            zc = -0.17 + row * 0.057
            yc = 1.19 + (zc + 0.20) * 0.08 / 0.30
            xc = 0.44 + col * 0.06 + (0.008 if col == 5 else 0)
            I.add(move(key, (xc, yc - 0.003, zc)), "metal", cols[col] if row > 0 or col < 3 else 0xE8B23A)
    I.add(box((0.60, 1.26, 0.04), (0.66, 1.29, 0.10)), "metal", reg)
    I.add(box((0.51, 1.285, 0.03), (0.75, 1.35, 0.15), 0.006, 1), "metal", reg)
    for z, zz in ((0.149, 0.156), (0.024, 0.031)):
        I.add(box((0.53, 1.298, z), (0.73, 1.337, zz)), "glassDark", 0x1E3A2A)
    I.add(box((0.70, 1.148, -0.10), (0.84, 1.205, 0.10), 0.006, 1), "metal", 0x4A4A4A)
    I.add(box((0.72, 1.20, -0.09), (0.82, 1.215, -0.07)), "soffit", 0xF4F2EC)  # receipt paper


def booth(I):
    chrome, top = 0xC8CCD0, 0xE8E2D0
    for flip in (False, True):
        parts = [
            ("metal", chrome, False, box((-0.90, 0.0, -1.04), (0.90, 0.06, -0.60), 0.004, 1)),
            ("fabric", WHITE, True, box((-0.90, 0.055, -1.05), (0.90, 0.36, -0.58), 0.02, 1)),
            ("fabric", WHITE, True, box((-0.895, 0.34, -0.93), (0.895, 0.465, -0.55), 0.04, 2, (3, 0, 1), {"+y": 0.012})),
            ("fabric", WHITE, True, tube([(-0.88, 0.452, -0.552), (0.88, 0.452, -0.552)], 0.008, 6)),
            ("fabric", WHITE, True, box((-0.90, 0.36, -1.05), (0.90, 1.088, -0.955), 0.02, 1)),
            ("metal", chrome, False, box((-0.905, 1.082, -1.055), (0.905, 1.10, -0.95), 0.004, 1)),
        ]
        w = 1.78 / 5
        for k in range(5):  # channel-tufted back rolls
            x0 = -0.89 + k * w
            parts.append(("fabric", WHITE, True, box((x0 + 0.003, 0.46, -0.97), (x0 + w - 0.003, 1.075, -0.88), 0.045, 2, (0, 2, 0), {"+z": 0.018})))
        for surf, col, soft, bm in parts:
            I.add(rot(bm, "y", 180) if flip else bm, surf, col, soft=soft)
    I.add(box((-0.855, 0.712, -0.405), (0.855, 0.748, 0.405), 0.004, 1), "metal", chrome)
    I.add(box((-0.85, 0.715, -0.40), (0.85, 0.756, 0.40), 0.004, 1), "counter", top)
    I.add(lathe([(0.0, 0.0), (0.25, 0.0), (0.25, 0.012), (0.20, 0.02), (0.06, 0.045), (0.04, 0.07), (0.04, 0.66),
                 (0.06, 0.68), (0.14, 0.70), (0.14, 0.713), (0.0, 0.713)], "y", (0, 0, 0), seg=24), "metal", 0xB8BCC0, sharp=50)
    # the table's things, at the wall end
    I.add(box((0.62, 0.756, -0.05), (0.76, 0.87, 0.05), 0.008, 2), "metal", chrome)  # napkin dispenser
    I.add(box((0.63, 0.80, -0.052), (0.75, 0.86, 0.052)), "soffit", 0xF8F6F0)
    for z, c in ((0.12, 0xC23B30), (-0.12, 0xE8B23A)):
        I.add(lathe([(0.0, 0.0), (0.027, 0.0), (0.028, 0.13), (0.012, 0.16), (0.008, 0.19), (0.0, 0.19)], "y", (0.68, 0.756, z), seg=10), "product", c, sharp=50)
    for z, c in ((0.22, 0xF2F0EA), (-0.22, 0x2A2A2A)):
        I.add(lathe([(0.0, 0.0), (0.018, 0.0), (0.018, 0.05), (0.016, 0.06), (0.0, 0.07)], "y", (0.72, 0.756, z), seg=10), "product", c, sharp=50)
    I.add(lathe([(0.0, 0.0), (0.035, 0.0), (0.038, 0.09), (0.03, 0.095), (0.03, 0.12), (0.006, 0.135), (0.0, 0.135)], "y", (0.56, 0.756, 0.0), seg=12), "stainless", 0xD8DCDE, sharp=50)


def stool(I):
    chrome = 0xC8CCD0
    I.add(lathe([(0.0, 0.0), (0.19, 0.0), (0.19, 0.012), (0.17, 0.024), (0.07, 0.045), (0.045, 0.07), (0.035, 0.09),
                 (0.035, 0.665), (0.05, 0.675), (0.0, 0.676)], "y", (0, 0, 0), seg=24), "metal", chrome, sharp=50)
    I.add(torus("y", (0, 0.30, 0), 0.165, 0.011, 24, 6), "metal", chrome, sharp=60)
    for k in range(4):
        a = math.radians(45 + 90 * k)
        d = Vector((math.cos(a), 0, math.sin(a)))
        I.add(tube([Vector((0, 0.30, 0)) + d * 0.03, Vector((0, 0.30, 0)) + d * 0.16], 0.008, 6), "metal", chrome, sharp=60)
    I.add(lathe([(0.0, 0.672), (0.17, 0.672), (0.19, 0.69), (0.19, 0.712), (0.0, 0.712)], "y", (0, 0, 0), seg=24), "metal", chrome, sharp=50)
    I.add(lathe([(0.0, 0.708), (0.188, 0.708), (0.197, 0.72), (0.199, 0.736), (0.195, 0.754), (0.18, 0.77), (0.14, 0.777),
                 (0.08, 0.780), (0.0, 0.781)], "y", (0, 0, 0), seg=24), "fabric", soft=True)
    I.add(torus("y", (0, 0.737, 0), 0.197, 0.006, 24, 5), "fabric", soft=True)


def washer(I):
    enamel, steel, dark = 0xF0EEE8, 0xD8DCDE, 0x2A2A2A
    body = box((-0.34, 0.06, -0.34), (0.34, 0.885, 0.33), 0.02, 2)
    I.add(cut(body, cyl("z", (0, 0.48, 0), 0.215, 0.30, 0.40, seg=32)), "metal", enamel)
    I.add(box((-0.345, 0.88, -0.345), (0.345, 0.90, 0.345), 0.004, 1), "metal", enamel)  # top
    I.add(box((-0.33, 0.0, -0.33), (0.33, 0.065, 0.30)), "metal", 0x6A6A6A)  # kick
    I.add(box((-0.32, 0.74, 0.325), (0.32, 0.875, 0.345), 0.006, 1), "stainless", steel)  # console
    I.add(box((0.06, 0.765, 0.34), (0.28, 0.85, 0.37), 0.006, 2), "stainless", 0xA0A4A8)  # coin slide
    for k in range(4):
        x = 0.085 + k * 0.05
        I.add(box((x, 0.78, 0.368), (x + 0.025, 0.835, 0.374)), "metal", dark)
    I.add(lathe([(0.032, 0.345), (0.03, 0.36), (0.0, 0.365)], "z", (-0.20, 0.81, 0), seg=16), "metal", dark, sharp=50)
    I.add(box((-0.205, 0.81, 0.36), (-0.195, 0.84, 0.368)), "metal", 0xE0E0E0)
    for k, c in enumerate((0x30D040, 0xFFB020, 0xFF3020)):
        I.add(box((-0.10 + k * 0.03, 0.80, 0.344), (-0.085 + k * 0.03, 0.815, 0.35)), "lamp", c)
    I.add(box((-0.30, 0.69, 0.33), (-0.10, 0.73, 0.345), 0.004, 1), "stainless", steel)  # soap drawer
    I.add(box((0.10, 0.68, 0.331), (0.30, 0.715, 0.336)), "product", 0x2A4A8A)  # maker's plate
    # the porthole door: a chrome ring with a domed glass in the round well
    I.add(lathe([(0.168, 0.31), (0.21, 0.31), (0.214, 0.33), (0.206, 0.355), (0.178, 0.362), (0.166, 0.345)], "z", (0, 0.48, 0), seg=32, closed=True),
          "metal", 0xC8CCD0, sharp=50)
    I.add(lathe([(0.0, 0.315), (0.17, 0.315), (0.17, 0.34), (0.12, 0.36), (0.06, 0.37), (0.0, 0.373)], "z", (0, 0.48, 0), seg=32),
          "glassDark", 0x3A4A52, soft=True)
    I.add(box((0.19, 0.43, 0.335), (0.235, 0.53, 0.372), 0.008, 2), "metal", dark)
    for y in (0.40, 0.56):
        I.add(box((-0.235, y - 0.025, 0.325), (-0.20, y + 0.025, 0.35), 0.004, 1), "metal", 0x8A8E92)


def chair(I):
    chrome = 0xC8CCD0
    I.add(lathe([(0.0, 0.0), (0.28, 0.0), (0.28, 0.014), (0.26, 0.03), (0.11, 0.06), (0.08, 0.085), (0.07, 0.10),
                 (0.07, 0.30), (0.055, 0.315), (0.05, 0.42), (0.0, 0.42)], "y", (0, 0, 0), seg=32), "metal", chrome, sharp=50)
    I.add(lathe([(0.075, 0.12), (0.075, 0.28), (0.0, 0.28)], "y", (0, 0, 0), seg=20), "rubber", 0x1A1A1A, sharp=50)  # boot
    I.add(tube([(0.05, 0.14, 0.04), (0.10, 0.08, 0.18), (0.10, 0.07, 0.27)], 0.012, 8, 0.05), "metal", chrome, sharp=60)  # pump
    I.add(box((0.05, 0.055, 0.24), (0.16, 0.08, 0.32), 0.008, 1), "rubber", 0x1A1A1A)
    I.add(box((-0.24, 0.40, -0.24), (0.24, 0.462, 0.25), 0.012, 2), "metal", chrome)  # seat pan
    I.add(box((-0.26, 0.45, -0.24), (0.26, 0.585, 0.29), 0.05, 2, (2, 0, 2), {"+y": 0.015}), "fabric", soft=True)
    I.add(tube([(0, 0.43, -0.20), (0, 0.47, -0.30), (0, 0.66, -0.30)], 0.02, 8, 0.06), "metal", chrome, sharp=60)
    back = box((-0.25, 0.57, -0.32), (0.25, 1.18, -0.21), 0.07, 3, (2, 3, 0), {"+z": 0.012})
    I.add(rot(back, "x", -3, (0, 0.57, -0.26)), "fabric", soft=True)
    for k in range(3):  # channel rolls across the back
        y0 = 0.64 + k * 0.165
        b = box((-0.215, y0, -0.215), (0.215, y0 + 0.155, -0.175), 0.02, 2, (2, 1, 0), {"+z": 0.012})
        I.add(rot(b, "x", -3, (0, 0.57, -0.26)), "fabric", soft=True)
    for s in (-1, 1):
        for z in (-0.16, 0.17):
            I.add(tube([(s * 0.25, 0.44, z), (s * 0.325, 0.47, z), (s * 0.325, 0.69, z)], 0.012, 8, 0.04), "metal", chrome, sharp=60)
        I.add(box((s * 0.325 - 0.055, 0.68, -0.22), (s * 0.325 + 0.055, 0.74, 0.25), 0.026, 2, (0, 0, 2), {"+y": 0.008}), "fabric", soft=True)
    I.add(tube([(-0.17, 0.44, 0.22), (-0.17, 0.21, 0.44), (0.17, 0.21, 0.44), (0.17, 0.44, 0.22)], 0.013, 8, 0.05), "metal", chrome, sharp=60)
    I.add(box((-0.20, 0.17, 0.38), (0.20, 0.20, 0.53), 0.008, 1), "rubber", 0x1A1A1A)  # footrest


def workbench(I):
    steel, board = 0x55585A, 0x8D5A3A
    for x in (-1.15, 1.15):
        for z in (-0.30, 0.30):
            sx, sz = (1 if x < 0 else -1), (1 if z < 0 else -1)
            I.add(box((min(x, x + sx * 0.05), 0.0, min(z, z + sz * 0.005)), (max(x, x + sx * 0.05), 0.85, max(z, z + sz * 0.005))), "metal", steel)
            I.add(box((min(x, x + sx * 0.005), 0.0, min(z, z + sz * 0.05)), (max(x, x + sx * 0.005), 0.85, max(z, z + sz * 0.05))), "metal", steel)
    for z in (-0.30, 0.295):
        I.add(box((-1.15, 0.80, z), (1.15, 0.85, z + 0.005)), "metal", steel)
        I.add(box((-1.15, 0.14, z), (1.15, 0.17, z + 0.005)), "metal", steel)
    tones = [0x8A6A46, 0x7E6040, 0x947250]
    for k in range(7):
        z0 = -0.35 + k * 0.1
        I.add(box((-1.2, 0.85, z0), (1.2, 0.90, z0 + 0.1), 0.004, 1), "wood", tones[k % 3])
    for k in range(3):
        z0 = -0.295 + k * 0.197
        I.add(box((-1.145, 0.17, z0), (1.145, 0.19, z0 + 0.195), 0.003, 1), "wood", tones[(k + 1) % 3])
    I.add(box((0.35, 0.68, -0.29), (1.05, 0.80, 0.30), 0.004, 1), "metal", 0x6A6E72)  # drawer
    I.add(box((0.36, 0.69, 0.30), (1.04, 0.79, 0.312), 0.004, 1), "metal", 0x6A6E72)
    I.add(tube([(0.62, 0.74, 0.31), (0.62, 0.74, 0.33), (0.78, 0.74, 0.33), (0.78, 0.74, 0.31)], 0.006, 6, 0.01), "metal", 0xC8CCD0, sharp=60)
    # the pegboard, framed, with tools on it
    I.add(box((-1.2, 0.90, -0.35), (1.2, 1.90, -0.335)), "wood", board, pattern=PAT_PEG)
    for (a, b) in (((-1.2, 0.90, -0.34), (1.2, 0.93, -0.325)), ((-1.2, 1.87, -0.34), (1.2, 1.90, -0.325)),
                   ((-1.2, 0.90, -0.34), (-1.17, 1.90, -0.325)), ((1.17, 0.90, -0.34), (1.2, 1.90, -0.325))):
        I.add(box(a, b, 0.003, 1), "wood", 0x6A4A30)
    red = 0xC23B30
    I.add(box((-0.9, 1.20, -0.335), (-0.40, 1.50, -0.12), 0.006, 2), "metal", red)  # wall cabinet
    I.add(box((-0.652, 1.21, -0.12), (-0.648, 1.49, -0.117)), "metal", 0x6A1A14)
    for x in (-0.67, -0.63):
        I.add(box((x - 0.006, 1.32, -0.12), (x + 0.006, 1.38, -0.11)), "metal", 0xC8CCD0)
    I.add(box((-0.20, 1.25, -0.333), (-0.17, 1.55, -0.30), 0.006, 1), "wood", 0xC8A070)  # hammer
    I.add(box((-0.25, 1.53, -0.33), (-0.10, 1.575, -0.30), 0.006, 1), "metal", 0x3A3A3A)
    for k in range(6):  # wrenches, smallest first
        x, ln = 0.0 + k * 0.06, 0.13 + k * 0.022
        I.add(box((x - 0.008 - k * 0.001, 1.60 - ln, -0.333), (x + 0.008 + k * 0.001, 1.60, -0.325), 0.002, 1), "metal", 0xB0B4B8)
        I.add(cyl("z", (x, 1.60, 0), 0.018 + k * 0.002, -0.333, -0.325, seg=10), "metal", 0xB0B4B8, sharp=60)
        I.add(cyl("z", (x, 1.60 - ln, 0), 0.016 + k * 0.002, -0.333, -0.325, seg=10), "metal", 0xB0B4B8, sharp=60)
    for k, c in enumerate((0xC23B30, 0xE8B23A, 0x1A1A1A, 0x3B67C4, 0xC23B30)):  # screwdrivers
        x = 0.42 + k * 0.05
        I.add(lathe([(0.0, 1.42), (0.012, 1.425), (0.014, 1.48), (0.011, 1.52), (0.0, 1.525)], "y", (x, 0, -0.31), seg=8), "product", c, sharp=50)
        I.add(cyl("y", (x, 0, -0.31), 0.003, 1.28 + k * 0.01, 1.42, seg=6), "metal", 0xC8CCD0, sharp=60)
    I.add(prism([(0.72, 1.30), (1.10, 1.36), (1.10, 1.46), (0.72, 1.46)], "z", -0.333, -0.33), "metal", 0xC0C4C8)  # saw blade
    I.add(box((0.62, 1.30, -0.334), (0.73, 1.47, -0.31), 0.008, 2), "wood", 0x6A2A1A)
    # a bench vise on the front left, a toolbox and a coffee can of bolts on the top
    vise = 0x3A5A7A
    I.add(box((-1.05, 0.90, 0.16), (-0.85, 0.96, 0.32), 0.006, 1), "metal", vise)
    I.add(box((-1.03, 0.96, 0.18), (-0.87, 1.04, 0.24), 0.006, 1), "metal", vise)
    I.add(box((-1.03, 0.96, 0.27), (-0.87, 1.04, 0.31), 0.006, 1), "metal", vise)
    I.add(cyl("z", (-0.95, 0.985, 0), 0.012, 0.31, 0.385, seg=10), "metal", 0xC8CCD0, sharp=60)
    I.add(cyl("x", (0, 0.985, 0.38), 0.007, -1.03, -0.87, seg=8), "metal", 0xC8CCD0, sharp=60)
    I.add(box((0.45, 0.90, -0.25), (0.95, 1.08, 0.02), 0.01, 2), "metal", red)
    I.add(tube([(0.60, 1.08, -0.115), (0.60, 1.12, -0.115), (0.80, 1.12, -0.115), (0.80, 1.08, -0.115)], 0.01, 8, 0.02), "metal", 0x2A2A2A, sharp=60)
    I.add(box((0.448, 1.0, -0.252), (0.952, 1.006, 0.022)), "metal", 0x8A2A20)
    I.add(lathe([(0.0, 0.90), (0.055, 0.90), (0.055, 1.02), (0.05, 1.025), (0.0, 1.025)], "y", (0.0, 0, 0.0), seg=14), "product", 0x8A2A20, sharp=50)


def lift(I):
    blue, grey, pad = 0x2F5F9A, 0x55585A, 0x1A1A1A
    for s in (-1, 1):
        x0, x1 = (-1.6, -1.3) if s < 0 else (1.3, 1.6)
        xi = x1 if s < 0 else x0  # the inner face
        post = box((x0, 0.02, -0.20), (x1, 3.05, 0.20), 0.008, 1)
        cx0, cx1 = sorted((xi - s * 0.10, xi + s * 0.10))
        I.add(cut(post, box((cx0, 0.12, -0.13), (cx1, 2.95, 0.13))), "metal", blue)
        I.add(box((x0 - 0.0 if s < 0 else x0 - 0.02, 0.0, -0.30), (x1 + 0.02 if s < 0 else x1, 0.022, 0.30), 0.004, 1), "metal", 0x3A3A3A)
        for dx in (0.06, 0.24):
            for z in (-0.25, 0.25):
                I.add(lathe([(0.018, 0.022), (0.018, 0.035), (0.0, 0.04)], "y", (x0 + dx, 0, z), seg=6), "metal", 0x8A8E92, sharp=60)
        cr0, cr1 = sorted((xi - s * 0.08, xi + s * 0.07))
        I.add(box((cr0, 1.40, -0.16), (cr1, 1.80, 0.16), 0.01, 2), "metal", 0x3A4048)  # carriage
        I.add(cyl("y", (xi - s * 0.10, 0, 0), 0.035, 0.12, 2.6, seg=12), "metal", 0xC8CCD0, sharp=60)  # ram
        for k in range(12):  # safety lock ladder
            y = 0.5 + k * 0.18
            I.add(box((xi - s * 0.13 - 0.01, y, 0.09), (xi - s * 0.13 + 0.01, y + 0.04, 0.12)), "metal", 0x8A8E92)
        for zs in (-1, 1):  # the arms: outer box, telescoping inner box, screw pad
            parts = [("metal", grey, box((-0.05, 1.60, 0.0), (0.05, 1.70, 0.74), 0.006, 1)),
                     ("metal", 0x6A6E72, box((-0.036, 1.612, 0.70), (0.036, 1.688, 1.06), 0.004, 1)),
                     ("metal", 0x8A8E92, cyl("y", (0, 0, 1.025), 0.035, 1.688, 1.73, seg=12)),
                     ("rubber", pad, cyl("y", (0, 0, 1.025), 0.065, 1.73, 1.755, seg=16)),
                     ("metal", grey, cyl("y", (0, 0, 0.0), 0.06, 1.58, 1.72, seg=14))]
            ang = math.degrees(math.atan2(0.50, 0.96)) * (-s)
            for surf, col, bm in parts:
                if zs < 0:
                    rot(bm, "y", 180)
                    rot(bm, "y", ang)
                    bm.transform(Matrix.Scale(-1, 4, Vector((1, 0, 0))))
                else:
                    rot(bm, "y", ang)
                bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
                move(bm, (xi + s * 0.02, 0, zs * 0.11))
                I.add(bm, surf, col, sharp=50 if surf == "rubber" else None)
    I.add(box((-1.6, 3.05, -0.12), (1.6, 3.20, 0.12), 0.008, 2), "metal", blue)  # overhead beam
    I.add(box((-1.25, 2.93, -0.025), (1.25, 2.98, 0.025), 0.01, 2), "metal", 0xE8B23A)  # shut-off bar
    I.add(cyl("y", (-1.45, 0, 0.30), 0.07, 1.0, 1.32, seg=16), "metal", 0x2A2A2A, sharp=50)  # power unit
    I.add(box((-1.56, 0.84, 0.205), (-1.34, 1.0, 0.39), 0.008, 1), "metal", 0x1E1E1E)
    I.add(box((-1.36, 1.10, 0.20), (-1.30, 1.20, 0.26), 0.004, 1), "metal", 0xE8E8E8)
    I.add(cyl("z", (-1.33, 1.15, 0), 0.015, 0.26, 0.275, seg=10), "lamp", 0x30D040, sharp=50)
    I.add(tube([(-1.45, 0.86, 0.30), (-1.45, 0.06, 0.30), (-1.45, 0.06, 0.0), (1.45, 0.06, 0.0), (1.45, 0.10, 0.0)], 0.012, 6, 0.06), "rubber", pad, sharp=60)


def tires(I):
    tread = [(0.214, -0.07), (0.232, -0.086), (0.27, -0.097), (0.31, -0.098), (0.338, -0.088), (0.349, -0.068),
             (0.35, -0.04), (0.337, -0.036), (0.337, -0.016), (0.35, -0.012), (0.35, 0.012), (0.337, 0.016),
             (0.337, 0.036), (0.35, 0.04), (0.349, 0.068), (0.338, 0.088), (0.31, 0.098), (0.27, 0.097),
             (0.232, 0.086), (0.214, 0.07), (0.208, 0.04), (0.208, -0.04)]
    rnd = random.Random(14)
    for i in range(4):
        c = (rnd.uniform(-0.008, 0.008), i * 0.22 + 0.10, rnd.uniform(-0.008, 0.008))
        prof = [(r, -a) for r, a in tread]  # ordered so the lathe faces outward
        I.add(lathe(prof, "y", c, seg=26, closed=True, phase=rnd.uniform(0, 0.2)), "rubber", 0x222222, sharp=45)


ITEMS = {
    "sofa": ((-1.00, 0.00, -0.45), (1.00, 0.85, 0.45), "fabric", sofa, "Sofa: skirted base, rolled arms, two seat and two back cushions, wooden feet"),
    "bed": ((-0.80, 0.00, -1.06), (0.80, 1.05, 1.00), "fabric", bed, "Double bed: panelled headboard with turned posts, footboard, bedspread, turned-down sheet, two pillows"),
    "table": ((-0.60, 0.00, -0.88), (0.60, 0.95, 0.88), None, table, "Dining table with turned legs and a spindle-back chair at each end"),
    "kitchen": ((-1.82, 0.00, -0.32), (1.82, 2.15, 0.33), "trim", kitchen, "Kitchen run: raised-panel cabinets, laminate top, double steel sink and faucet, almond fridge"),
    "tv": ((-0.55, 0.00, -0.25), (0.55, 1.08, 0.25), None, tv, "TV stand with a VCR, tapes and a console, and a CRT set with dials"),
    "shelf": ((-0.90, 0.00, -0.18), (0.90, 1.90, 0.18), None, shelf, "Bookcase with crown and plinth, five shelves of books, a vase and a photo"),
    "desk": ((-0.75, 0.00, -0.40), (0.75, 1.15, 0.95), None, desk, "Pedestal desk, beige home computer with monitor and keyboard, office chair"),
    "aisle": ((-2.00, 0.00, -0.45), (2.00, 1.60, 0.45), None, aisle, "Store gondola: three shelves a side of boxes, cans, bottles and bags"),
    "cooler": ((-1.50, 0.00, -0.40), (1.50, 2.10, 0.41), None, cooler, "Three-door glass drinks cooler with lit header, shelves of bottles, cartons and cans"),
    "counter": ((-1.55, 0.00, -0.40), (1.55, 1.35, 0.40), "trim", counter, "Shop counter, panelled front, open shelves behind, chrome-edged top, cash register"),
    "booth": ((-0.90, 0.00, -1.05), (0.90, 1.10, 1.05), "fabric", booth, "Diner booth: channel-tufted benches, chrome-banded table on a pedestal, condiments"),
    "stool": ((-0.20, 0.00, -0.20), (0.20, 0.78, 0.20), "fabric", stool, "Diner counter stool: chrome pedestal and foot ring, piped vinyl seat"),
    "washer": ((-0.34, 0.00, -0.34), (0.34, 0.90, 0.34), None, washer, "Laundromat front-loader: coin slide, dial, lamps, chrome porthole door"),
    "chair": ((-0.40, 0.00, -0.32), (0.40, 1.20, 0.55), "fabric", chair, "Salon hydraulic chair: chrome base and pump, channelled back, arms, footrest"),
    "workbench": ((-1.20, 0.00, -0.35), (1.20, 1.90, 0.35), None, workbench, "Garage bench: angle-iron frame, plank top, pegboard with tools, cabinet, vise, toolbox"),
    "lift": ((-1.60, 0.00, -1.20), (1.60, 3.20, 1.20), None, lift, "Two-post car lift: channel posts, carriages, telescoping arms with pads, overhead beam, power unit"),
    "tires": ((-0.35, 0.00, -0.35), (0.35, 0.86, 0.35), None, tires, "Stack of four tyres with tread grooves"),
}


# ----------------------------------------------------------------------------- the bake materials
# One procedural shader per surface key: the look a part of that surface is baked with. Coordinates are object
# space in metres (Blender axes), so grain, weave and print hold one scale on every item and every swatch.

ATLAS = 4096  # px, square
SWATCH = 256  # px, one swatch of a kit surface
SWATCH_KEYS = ["fabric", "wood", "metal", "counter", "trim", "paint", "glassDark", "glassShop", "lamp", "product",
               "rubber", "stainless", "concrete"]
PAD = 4  # px between items' regions
# Large, repetitive shop and garage fixtures take a lower texel density than the furniture a pilot sits on, so the
# one atlas holds them all (a fraction of the planned density).
ITEM_DENSITY = {"aisle": 0.55, "cooler": 0.6, "lift": 0.7, "shelf": 0.8, "workbench": 0.85, "counter": 0.85}
TEX = os.path.join(ROOT, "assets", "textures", "furniture")


class Nodes:
    """A small node-graph writer, so each surface's shader reads as a few lines."""

    def __init__(self, mat):
        self.nt = mat.node_tree
        self.nt.nodes.clear()

    def n(self, kind, **props):
        node = self.nt.nodes.new(kind)
        for k, v in props.items():
            if k.startswith("in_"):
                node.inputs[int(k[3:]) if k[3:].isdigit() else k[3:].replace("_", " ")].default_value = v
            else:
                setattr(node, k, v)
        return node

    def link(self, a, b):
        self.nt.links.new(a, b)

    def math(self, op, a, b=None, clamp=False):
        m = self.n("ShaderNodeMath", operation=op, use_clamp=clamp)
        for i, v in enumerate((a, b)):
            if v is None:
                continue
            if isinstance(v, (int, float)):
                m.inputs[i].default_value = v
            else:
                self.link(v, m.inputs[i])
        return m.outputs[0]

    def mix(self, a, b, fac, blend="MIX"):
        m = self.n("ShaderNodeMix", data_type="RGBA", blend_type=blend)
        for sock, v in ((m.inputs[0], fac), (m.inputs[6], a), (m.inputs[7], b)):
            if isinstance(v, (int, float)):
                sock.default_value = v
            elif isinstance(v, tuple):
                sock.default_value = v
            else:
                self.link(v, sock)
        return m.outputs[2]

    def noise(self, vec, scale, detail=4.0, rough=0.5, out="Fac"):
        t = self.n("ShaderNodeTexNoise", in_Scale=scale, in_Detail=detail, in_Roughness=rough)
        self.link(vec, t.inputs["Vector"])
        return t.outputs[out]

    def scaled(self, vec, sx, sy, sz):
        m = self.n("ShaderNodeVectorMath", operation="MULTIPLY")
        m.inputs[1].default_value = (sx, sy, sz)
        self.link(vec, m.inputs[0])
        return m.outputs[0]


def bake_material(surf):
    """The procedural shader a surface is baked with (one per key, shared by every item)."""
    rough, _, _ = SURFACES[surf]
    m = bpy.data.materials.new(f"bake_{surf}")
    m.use_nodes = True
    N = Nodes(m)
    out = N.n("ShaderNodeOutputMaterial")
    p = N.n("ShaderNodeBsdfPrincipled")
    N.link(p.outputs[0], out.inputs["Surface"])
    co = N.n("ShaderNodeTexCoord").outputs["Object"]
    col = N.n("ShaderNodeAttribute", attribute_name="Color").outputs["Color"]
    fin = N.n("ShaderNodeSeparateColor")
    N.link(N.n("ShaderNodeAttribute", attribute_name="Finish").outputs["Color"], fin.inputs[0])
    metal = fin.outputs[1]
    pattern = fin.outputs[2]
    N.link(metal, p.inputs["Metallic"])
    p.inputs["Emission Strength"].default_value = 1.0
    shade, height, r = 1.0, None, rough  # colour factor, bump height, roughness (sockets or numbers)
    bump = 0.0
    if surf in ("fabric", "soffit", "carpet"):
        w1 = N.n("ShaderNodeTexWave", wave_type="BANDS", bands_direction="X", in_Scale=180.0)
        w2 = N.n("ShaderNodeTexWave", wave_type="BANDS", bands_direction="Z", in_Scale=180.0)
        N.link(co, w1.inputs["Vector"])
        N.link(co, w2.inputs["Vector"])
        weave = N.math("MULTIPLY", w1.outputs["Fac"], w2.outputs["Fac"])
        mottle = N.noise(co, 18.0)
        shade = N.math("ADD", N.math("MULTIPLY", mottle, 0.12), N.math("MULTIPLY", weave, 0.08))
        shade = N.math("ADD", shade, 0.84)
        height, bump = N.math("ADD", weave, N.math("MULTIPLY", N.noise(co, 60.0), 0.5)), 0.25
        r = N.math("ADD", N.math("MULTIPLY", mottle, 0.08), rough - 0.04)
    elif surf in ("wood",):
        v = N.scaled(co, 0.12, 1.0, 1.0)
        w = N.n("ShaderNodeTexWave", wave_type="RINGS", in_Scale=9.0, in_Distortion=7.0, in_Detail=3.0, in_Detail_Scale=1.5)
        N.link(v, w.inputs["Vector"])
        fine = N.noise(N.scaled(co, 2.0, 60.0, 60.0), 4.0)
        grain = N.math("ADD", N.math("MULTIPLY", w.outputs["Fac"], 0.75), N.math("MULTIPLY", fine, 0.25))
        shade = N.math("ADD", N.math("MULTIPLY", grain, 0.42), 0.68)
        height, bump = grain, 0.12
        r = N.math("ADD", N.math("MULTIPLY", grain, 0.15), rough - 0.08)
    elif surf in ("metal",):
        orange = N.noise(co, 220.0, 2.0)
        scuff = N.noise(co, 9.0, 6.0, 0.6)
        shade = N.math("ADD", N.math("MULTIPLY", scuff, 0.08), 0.94)
        height, bump = orange, 0.05
        # chrome (metallic 1) is glossier than paint
        r = N.math("ADD", N.math("MULTIPLY", scuff, 0.12), N.math("SUBTRACT", 0.4, N.math("MULTIPLY", metal, 0.26)))
    elif surf in ("stainless",):
        brushed = N.noise(N.scaled(co, 4.0, 4.0, 900.0), 3.0, 2.0)
        shade = N.math("ADD", N.math("MULTIPLY", brushed, 0.1), 0.9)
        height, bump = brushed, 0.04
        r = N.math("ADD", N.math("MULTIPLY", brushed, 0.12), 0.18)
    elif surf in ("counter",):
        speck = N.noise(co, 380.0, 1.0)
        dots = N.math("GREATER_THAN", speck, 0.68)
        mottle = N.noise(co, 14.0)
        shade = N.math("SUBTRACT", N.math("ADD", 0.93, N.math("MULTIPLY", mottle, 0.07)), N.math("MULTIPLY", dots, 0.35))
        r = N.math("ADD", N.math("MULTIPLY", mottle, 0.08), 0.24)
        height, bump = mottle, 0.02
    elif surf in ("trim", "paint"):
        peel = N.noise(co, 160.0, 2.0)
        mottle = N.noise(co, 10.0)
        shade = N.math("ADD", N.math("MULTIPLY", mottle, 0.05), 0.95)
        height, bump = peel, 0.04
        r = N.math("ADD", N.math("MULTIPLY", mottle, 0.1), rough - 0.05)
    elif surf in ("rubber",):
        grit = N.noise(co, 140.0, 3.0)
        mottle = N.noise(co, 12.0)
        shade = N.math("ADD", N.math("MULTIPLY", mottle, 0.2), 0.85)
        height, bump = grit, 0.2
        r = N.math("ADD", N.math("MULTIPLY", grit, 0.1), 0.8)
    elif surf in ("concrete",):
        grit = N.noise(co, 90.0, 6.0, 0.65)
        mottle = N.noise(co, 4.0, 4.0)
        shade = N.math("ADD", N.math("MULTIPLY", mottle, 0.25), N.math("ADD", N.math("MULTIPLY", grit, 0.12), 0.72))
        height, bump = grit, 0.35
        r = 0.92
    elif surf in ("glassDark", "glassShop"):
        smudge = N.noise(co, 6.0, 3.0)
        shade = N.math("ADD", N.math("MULTIPLY", smudge, 0.12), 0.88)
        r = N.math("ADD", N.math("MULTIPLY", smudge, 0.08), 0.03)
    elif surf in ("lamp",):
        shade = 1.0
    elif surf in ("product",):
        mottle = N.noise(co, 30.0)
        shade = N.math("ADD", N.math("MULTIPLY", mottle, 0.06), 0.94)
        r = N.math("ADD", N.math("MULTIPLY", mottle, 0.1), 0.38)
    base = N.mix(col, (1, 1, 1, 1), 0.0)
    if isinstance(shade, float):
        tinted = N.mix(base, (shade, shade, shade, 1.0), 1.0, "MULTIPLY")
    else:
        sc = N.n("ShaderNodeCombineColor")
        for i in range(3):
            N.link(shade, sc.inputs[i])
        tinted = N.mix(base, sc.outputs[0], 1.0, "MULTIPLY")
    # printed packaging: a white band and a darker block of print, where the part asks for a label
    is_label = N.math("COMPARE", pattern, 0.1)
    N.nt.nodes[-1].inputs[2].default_value = 0.02
    band = N.n("ShaderNodeTexWave", wave_type="BANDS", bands_direction="Z", wave_profile="SAW", in_Scale=9.0)
    N.link(co, band.inputs["Vector"])
    stripe = N.math("MULTIPLY", N.math("GREATER_THAN", band.outputs["Fac"], 0.55), N.math("LESS_THAN", band.outputs["Fac"], 0.8))
    blocks = N.n("ShaderNodeTexBrick", in_Scale=14.0, in_Mortar_Size=0.0)
    N.link(co, blocks.inputs["Vector"])
    printed = N.mix(tinted, (0.95, 0.94, 0.9, 1.0), N.math("MULTIPLY", stripe, is_label))
    blk = N.math("MULTIPLY", N.math("GREATER_THAN", blocks.outputs["Fac"], 0.7), N.math("MULTIPLY", is_label, N.math("SUBTRACT", 1.0, stripe)))
    printed = N.mix(printed, N.mix(tinted, (0.25, 0.25, 0.25, 1.0), 1.0, "MULTIPLY"), blk)
    # pegboard: a hole every 25 mm, dark
    is_peg = N.math("COMPARE", pattern, 0.2)
    N.nt.nodes[-1].inputs[2].default_value = 0.02
    grid = N.n("ShaderNodeVectorMath", operation="FRACTION")
    N.link(N.scaled(co, 40.0, 40.0, 40.0), grid.inputs[0])
    centred = N.n("ShaderNodeVectorMath", operation="SUBTRACT")
    centred.inputs[1].default_value = (0.5, 0.5, 0.5)
    N.link(grid.outputs[0], centred.inputs[0])
    sepv = N.n("ShaderNodeSeparateXYZ")
    N.link(centred.outputs[0], sepv.inputs[0])
    dist = N.math("SQRT", N.math("ADD", N.math("POWER", sepv.outputs[0], 2.0), N.math("POWER", sepv.outputs[2], 2.0)))
    hole = N.math("MULTIPLY", N.math("LESS_THAN", dist, 0.2), is_peg)
    final = N.mix(printed, (0.06, 0.04, 0.03, 1.0), hole)
    N.link(final, p.inputs["Base Color"])
    # albedo and metal are both baked as emission (Cycles' diffuse colour pass is darkened by metallic):
    # the PASS value picks which one the emission carries
    which = N.n("ShaderNodeValue", name="PASS")
    N.link(N.mix(metal, final, which.outputs[0]), p.inputs["Emission Color"])
    if isinstance(r, float):
        p.inputs["Roughness"].default_value = r
    else:
        N.link(r, p.inputs["Roughness"])
    hh = N.math("SUBTRACT", height if height is not None else 0.0, N.math("MULTIPLY", hole, 2.0))
    bn = N.n("ShaderNodeBump", in_Strength=max(bump, 0.3) if True else bump, in_Distance=0.002)
    bn.inputs["Strength"].default_value = bump if height is not None else 0.0
    N.link(hh, bn.inputs["Height"])
    N.link(bn.outputs[0], p.inputs["Normal"])
    img = N.n("ShaderNodeTexImage", name="BAKE")
    N.nt.nodes.active = img
    return m


BAKE_MATS = {}


def bake_mat(surf):
    if surf not in BAKE_MATS:
        BAKE_MATS[surf] = bake_material(surf)
    return BAKE_MATS[surf]


# ----------------------------------------------------------------------------- build, unwrap, lay out, bake

def clear():
    for coll in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.images, bpy.data.cameras,
                 bpy.data.lights, bpy.data.worlds):
        for item in list(coll):
            coll.remove(item)
    BAKE_MATS.clear()


def make(name):
    """The item's object: built, merged, unwrapped and packed into its own unit square."""
    lo, hi, tint, build, _ = ITEMS[name]
    I = Item(name, tint)
    build(I)
    bm, surfs = I.mesh()
    me = bpy.data.meshes.new(f"mesh_{name}")
    bm.to_mesh(me)
    bm.free()
    for s in surfs:
        me.materials.append(bake_mat(s))
    o = bpy.data.objects.new(f"item_{name}", me)
    o.location = (list(ITEMS).index(name) * 10.0, 0, 0)  # apart, so no item occludes another in the bake
    bpy.context.scene.collection.objects.link(o)

    return o, unwrap(me), len(me.polygons), surfs


ISLAND_GAP_M = 0.03  # gap between islands, in metres at full texel weight (about 5 px at the planned density)


def unwrap(me):
    """Deterministic UVs: each face projected along the axis it faces most, in metres scaled by its part's texel
    weight; faces that share an edge and face the same way form an island; islands are shelf-packed (long side
    across) into a square and the square scaled to the unit square. Returns UV units per metre at full weight."""
    weight = [d.color[3] for d in me.color_attributes["Finish"].data]
    polys = list(me.polygons)
    key = []
    for p in polys:
        n = p.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        key.append((ax, n[ax] >= 0))
    parent = list(range(len(polys)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i
    edge_faces = {}
    for p in polys:
        for ek in p.edge_keys:
            edge_faces.setdefault(ek, []).append(p.index)
    for fs in edge_faces.values():
        if len(fs) == 2 and key[fs[0]] == key[fs[1]]:
            ra, rb = find(fs[0]), find(fs[1])
            if ra != rb:
                parent[max(ra, rb)] = min(ra, rb)
    islands = {}
    for p in polys:
        islands.setdefault(find(p.index), []).append(p.index)
    verts = me.vertices
    loops = me.loops
    uv = {}
    rects = []
    for root in sorted(islands):
        faces = islands[root]
        ax, pos = key[root]
        w = weight[polys[root].loop_indices[0]]
        pts = {}
        for fi in faces:
            for li in polys[fi].loop_indices:
                c = verts[loops[li].vertex_index].co
                if ax == 0:
                    u, v = (c.y if pos else -c.y), c.z
                elif ax == 1:
                    u, v = (-c.x if pos else c.x), c.z
                else:
                    u, v = c.x, (c.y if pos else -c.y)
                pts[li] = (u * w, v * w)
        us = [q[0] for q in pts.values()]
        vs = [q[1] for q in pts.values()]
        u0, v0, wd, ht = min(us), min(vs), max(us) - min(us), max(vs) - min(vs)
        turn = ht > wd
        for li, (u, v) in pts.items():
            u, v = u - u0, v - v0
            uv[li] = (v, wd - u) if turn else (u, v)
        rects.append((max(wd, ht), min(wd, ht), root, list(pts)))

    def shelf(width):
        x, y, row, place = 0.0, 0.0, 0.0, {}
        for w_, h_, root, _ in sorted(rects, key=lambda r: (-r[1], -r[0], r[2])):
            if x + w_ > width and x > 0:
                x, y, row = 0.0, y + row + ISLAND_GAP_M, 0.0
            place[root] = (x, y)
            x += w_ + ISLAND_GAP_M
            row = max(row, h_)
        return place, max(width, y + row)
    widest = max(r[0] for r in rects)
    lo, hi = widest, sum(r[0] + ISLAND_GAP_M for r in rects)
    for _ in range(40):  # the narrowest shelf width whose packing is no taller than it is wide
        mid = (lo + hi) / 2
        if shelf(mid)[1] <= mid:
            hi = mid
        else:
            lo = mid
    place, side = shelf(hi)
    side += ISLAND_GAP_M
    layer = me.uv_layers[0].data
    for _, _, root, lis in rects:
        ox, oy = place[root]
        for li in lis:
            u, v = uv[li]
            layer[li].uv = ((ox + u + ISLAND_GAP_M / 2) / side, (oy + v + ISLAND_GAP_M / 2) / side)
    return 1.0 / side


def layout(scales):
    """Square regions per item at one texel density, shelf-packed above the swatch strip; the highest density
    that fits. Returns (px per metre, {name: (x, y, side)})."""
    def pack(density):
        sides = {n: int(math.ceil(density * ITEM_DENSITY.get(n, 1.0) / s)) for n, s in scales.items()}
        order = sorted(sides, key=lambda n: (-sides[n], n))
        x, y, row_h, out = 0, SWATCH + PAD, 0, {}
        for n in order:
            side = sides[n]
            if x + side > ATLAS:
                x, y, row_h = 0, y + row_h + PAD, 0
            if y + side > ATLAS:
                return None
            out[n] = (x, y, side)
            x += side + PAD
            row_h = max(row_h, side)
        return out
    lo, hi = 10.0, 2000.0
    for _ in range(40):
        mid = (lo + hi) / 2
        if pack(mid) is None:
            hi = mid
        else:
            lo = mid
    density = math.floor(lo)
    return density, pack(density)


def set_bake_image(img, which):
    for m in BAKE_MATS.values():
        node = m.node_tree.nodes["BAKE"]
        node.image = img
        m.node_tree.nodes.active = node
        m.node_tree.nodes["PASS"].outputs[0].default_value = which


def bake_passes(objs, side, samples_ao=48):
    """Bake the selected objects into side x side images: albedo, normal, roughness, metal, occlusion.
    Returns numpy arrays (rows bottom-up, RGBA float)."""
    import numpy as np
    sc = bpy.context.scene
    for o in sc.objects:
        o.select_set(o in objs)
        o.hide_render = o not in objs
    bpy.context.view_layer.objects.active = objs[0]
    out = {}
    passes = [("albedo", "EMIT", 4, 1.0), ("normal", "NORMAL", 4, 0.0), ("rough", "ROUGHNESS", 4, 0.0),
              ("metal", "EMIT", 1, 0.0), ("ao", "AO", samples_ao, 0.0)]
    for key, kind, spp, which in passes:
        img = bpy.data.images.new(f"bake_{key}", side, side, alpha=False, float_buffer=True)
        img.colorspace_settings.name = "Non-Color"
        set_bake_image(img, which)
        sc.cycles.samples = spp
        sc.cycles.seed = 0
        bpy.ops.object.bake(type=kind, margin=PAD, margin_type="EXTEND", use_clear=True, target="IMAGE_TEXTURES",
                            normal_space="TANGENT")
        a = np.empty(side * side * 4, dtype=np.float32)
        img.pixels.foreach_get(a)
        out[key] = a.reshape(side, side, 4)
        bpy.data.images.remove(img)
    return out


def write_png(path, rgb):
    """An 8-bit RGB PNG from a float array with rows bottom-up (Blender's order), written byte for byte the
    same every time."""
    import zlib
    import numpy as np
    data = np.clip(np.round(rgb[::-1, :, :3] * 255.0), 0, 255).astype(np.uint8)
    h, w = data.shape[:2]
    raw = np.concatenate([np.zeros((h, 1), dtype=np.uint8), data.reshape(h, w * 3)], axis=1).tobytes()

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def to_srgb(a):
    import numpy as np
    a = np.clip(a, 0.0, 1.0)
    return np.where(a <= 0.0031308, a * 12.92, 1.055 * np.power(a, 1 / 2.4) - 0.055)


def seamless(a):
    """Blend a square patch with itself shifted by half, weighted so the seams of each copy are hidden by the
    other: the patch then tiles."""
    import numpy as np
    n = a.shape[0]
    t = np.abs(np.linspace(-1.0, 1.0, n))
    w1 = np.clip(1.0 - t, 0.0, 1.0)
    w = np.minimum(w1[:, None], w1[None, :])[:, :, None]
    w = np.clip(w * 2.5, 0.0, 1.0)
    shifted = np.roll(np.roll(a, n // 2, axis=0), n // 2, axis=1)
    return a * w + shifted * (1.0 - w)


def swatch_object(surf):
    """A 1 m square facing up, its UVs the unit square, a light neutral part of `surf`."""
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=0.5)
    uv = bm.loops.layers.uv.new("UVMap")
    colour = bm.loops.layers.float_color.new("Color")
    finish = bm.loops.layers.float_color.new("Finish")
    neutral = {"glassDark": 0xF0F0F0, "rubber": 0xE8E8E8, "lamp": WHITE}.get(surf, 0xF2F2F2)
    metal = 1.0 if surf == "stainless" else (0.7 if surf == "metal" else 0.0)
    for f in bm.faces:
        for loop in f.loops:
            loop[uv].uv = (loop.vert.co.x + 0.5, loop.vert.co.y + 0.5)
            loop[colour] = lin(neutral)
            loop[finish] = (0.0, metal, 0.0, 1.0)
    me = bpy.data.meshes.new(f"swatch_{surf}")
    bm.to_mesh(me)
    bm.free()
    me.materials.append(bake_mat(surf))
    o = bpy.data.objects.new(f"swatch_{surf}", me)
    o.location = (0, 0, -50)
    bpy.context.scene.collection.objects.link(o)
    return o


def furniture_material():
    m = bpy.data.materials.new("furniture")
    m.use_nodes = True
    m["atlas"] = json.dumps({"albedo": "textures/furniture/albedo.png", "normal": "textures/furniture/normal.png",
                             "orm": "textures/furniture/orm.png"})
    return m


def finalise_and_export(name, o, region, mat):
    """Move the item's UVs into its atlas region, set COLOR_0 to (1, 1, 1, tint mask), give it the one shared
    material and export it as MODEL."""
    x, y, side = region
    me = o.data
    for l in me.uv_layers[0].data:
        l.uv = ((x + l.uv.x * side) / ATLAS, (y + l.uv.y * side) / ATLAS)
    fin = me.color_attributes["Finish"]
    mask = [d.color[0] for d in fin.data]
    me.color_attributes.remove(me.color_attributes["Color"])
    col = me.color_attributes.new("COLOR", "FLOAT_COLOR", "CORNER")
    for d, m in zip(col.data, mask):
        d.color = (1.0, 1.0, 1.0, 1.0 if m > 0.5 else 0.0)
    me.color_attributes.remove(me.color_attributes["Finish"])
    me.color_attributes.active_color = me.color_attributes["COLOR"]
    me.color_attributes.render_color_index = 0
    me.materials.clear()
    me.materials.append(mat)
    for poly in me.polygons:
        poly.material_index = 0
    o.location = (0, 0, 0)
    o.name, me.name = "MODEL", "MODEL"
    for ob in bpy.context.scene.objects:
        ob.select_set(ob == o)
    path = os.path.join(OUT, f"{name}.glb")
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=False,
                              export_yup=True, export_texcoords=True, export_normals=True, export_tangents=True,
                              export_vertex_color="ACTIVE", export_all_vertex_colors=False, export_materials="EXPORT",
                              export_extras=True, export_cameras=False, export_lights=False, export_animations=False,
                              export_skins=False, export_morph=False, export_image_format="NONE")
    o.name, me.name = f"done_{name}", f"done_{name}"
    return path


# ----------------------------------------------------------------------------- read-back

def read_glb(path):
    """The glb's JSON and binary chunk."""
    data = open(path, "rb").read()
    n = struct.unpack_from("<I", data, 12)[0]
    js = json.loads(data[20:20 + n])
    return js, data[20 + n + 8:]


def accessor(js, binary, idx):
    a = js["accessors"][idx]
    bv = js["bufferViews"][a["bufferView"]]
    comps = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[a["type"]]
    fmt, size, norm = {5126: ("f", 4, 1.0), 5123: ("H", 2, 65535.0), 5121: ("B", 1, 255.0)}[a["componentType"]]
    stride = bv.get("byteStride", comps * size)
    off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    out = []
    for i in range(a["count"]):
        vals = struct.unpack_from("<" + fmt * comps, binary, off + i * stride)
        out.append([v / norm for v in vals] if a.get("normalized") else list(vals))
    return out


def verify(name, path, region):
    """Read the written glb back: one MODEL node and mesh with one primitive in the material `furniture`, with
    NORMAL, TEXCOORD_0 inside the item's atlas region and COLOR_0 (1, 1, 1, mask), the mask set where the item
    is tinted; triangles under the limit and bounds inside the block's box."""
    lo, hi, tint, _, _ = ITEMS[name]
    js, binary = read_glb(path)
    errs = []
    if len(js["meshes"]) != 1 or len(js["nodes"]) != 1 or js["nodes"][0].get("name") != "MODEL":
        errs.append("not one mesh under one node MODEL")
    prims = js["meshes"][0]["primitives"]
    if len(prims) != 1:
        errs.append(f"{len(prims)} primitives, not one")
    prim = prims[0]
    if js["materials"][prim["material"]]["name"] != "furniture":
        errs.append("material is not furniture")
    for att in ("POSITION", "NORMAL", "TEXCOORD_0", "COLOR_0"):
        if att not in prim["attributes"]:
            errs.append(f"no {att}")
    tris = js["accessors"][prim["indices"]]["count"] // 3
    pa = js["accessors"][prim["attributes"]["POSITION"]]
    blo, bhi = pa["min"], pa["max"]
    ca = js["accessors"][prim["attributes"]["COLOR_0"]]
    if ca["type"] != "VEC4":
        errs.append("COLOR_0 is not VEC4")
    cols = accessor(js, binary, prim["attributes"]["COLOR_0"])
    if any(abs(c[i] - 1.0) > 1e-3 for c in cols for i in range(3)):
        errs.append("COLOR_0 rgb is not 1")
    tinted = sum(1 for c in cols if c[3] > 0.5)
    if bool(tint) != (tinted > 0):
        errs.append(f"tint mask covers {tinted} vertices, tint surface {tint}")
    uvs = accessor(js, binary, prim["attributes"]["TEXCOORD_0"])
    x, y, side = region
    u0, u1 = x / ATLAS - 1e-4, (x + side) / ATLAS + 1e-4
    v0, v1 = 1 - (y + side) / ATLAS - 1e-4, 1 - y / ATLAS + 1e-4
    if any(not (u0 <= u <= u1 and v0 <= v <= v1) for u, v in uvs):
        errs.append("UVs leave the item's atlas region")
    for i, ax in enumerate("xyz"):
        if blo[i] < lo[i] - SLACK_M or bhi[i] > hi[i] + SLACK_M:
            errs.append(f"{ax} bounds {blo[i]:.3f}..{bhi[i]:.3f} outside {lo[i]}..{hi[i]} by more than {SLACK_M} m")
    if tris > LIMIT_TRIS:
        errs.append(f"{tris} triangles over the {LIMIT_TRIS} limit")
    return dict(triangles=tris, bounds=(blo, bhi), tinted_vertices=tinted), errs


def rect(x, y, w, h):
    """An atlas rectangle in glTF UV terms: [u0, v0, u1, v1], v down from the top."""
    return [round(x / ATLAS, 6), round(1 - (y + h) / ATLAS, 6), round((x + w) / ATLAS, 6), round(1 - y / ATLAS, 6)]


def build_all(quick):
    import numpy as np
    clear()
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.use_denoising = False
    sc.render.threads_mode = "FIXED"
    sc.render.threads = 4
    w = bpy.data.worlds.new("bake")
    sc.world = w
    w.light_settings.distance = 0.4  # AO reach, metres
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(TEX, exist_ok=True)
    objs, scales, info = {}, {}, {}
    for name in ITEMS:
        o, s, faces, surfs = make(name)
        objs[name], scales[name] = o, s
        info[name] = surfs
        print(f"BUILT {name}: {faces} triangles, surfaces {', '.join(surfs)}")
    density, regions = layout(scales)
    real = {name: round(scales[name] * side) for name, (x, y, side) in regions.items()}
    print("TEXEL", real)
    if quick:
        density, regions = density // 4, {n: (x // 4, y // 4, s // 4) for n, (x, y, s) in regions.items()}
    size = ATLAS // 4 if quick else ATLAS
    print(f"LAYOUT {density} px/m")
    atlas = {k: np.zeros((size, size, 4), dtype=np.float32) for k in ("albedo", "normal", "rough", "metal", "ao")}
    atlas["normal"][:, :] = (0.5, 0.5, 1.0, 1.0)
    atlas["ao"][:, :] = 1.0
    for name, (x, y, side) in regions.items():
        baked = bake_passes([objs[name]], side, 12 if quick else 48)
        for k, a in baked.items():
            atlas[k][y:y + side, x:x + side] = a
        print(f"BAKED {name} at {x},{y} side {side} px")
    swatches = {}
    sw = SWATCH // 4 if quick else SWATCH
    for i, surf in enumerate(SWATCH_KEYS):
        o = swatch_object(surf)
        baked = bake_passes([o], sw, 4)
        for k, a in baked.items():
            a = seamless(a)
            if k == "normal":
                n = a[:, :, :3] * 2 - 1
                n /= np.linalg.norm(n, axis=2, keepdims=True)
                a[:, :, :3] = n * 0.5 + 0.5
            atlas[k][0:sw, i * sw:(i + 1) * sw] = a
        swatches[surf] = rect(i * SWATCH, 0, SWATCH, SWATCH)
        bpy.data.objects.remove(o, do_unlink=True)
        print(f"SWATCH {surf}")
    orm = np.stack([atlas["ao"][:, :, 0], atlas["rough"][:, :, 0], atlas["metal"][:, :, 0]], axis=2)
    if quick:
        return
    write_png(os.path.join(TEX, "albedo.png"), to_srgb(atlas["albedo"][:, :, :3]))
    write_png(os.path.join(TEX, "normal.png"), atlas["normal"][:, :, :3])
    write_png(os.path.join(TEX, "orm.png"), orm)
    mat = furniture_material()
    items, failed = {}, []
    for name in ITEMS:
        lo, hi, tint, _, what = ITEMS[name]
        path = finalise_and_export(name, objs[name], regions[name], mat)
        got, errs = verify(name, path, regions[name])
        blo, bhi = got["bounds"]
        x, y, side = regions[name]
        items[name] = {
            "file": f"furniture/{name}.glb",
            "what": what,
            "box_m": {"lo": list(lo), "hi": list(hi)},
            "bounds_m": {"lo": [round(v, 3) for v in blo], "hi": [round(v, 3) for v in bhi]},
            "baked_surfaces": info[name],
            "tint_surface": tint,
            "atlas_rect": rect(x, y, side, side),
            "atlas_px": [x, ATLAS - y - side, side, side],
            "triangles": got["triangles"],
            "budget_tris": LIMIT_TRIS,
            "bytes": os.path.getsize(path),
            "sha256": hashlib.sha256(open(path, "rb").read()).hexdigest(),
        }
        print(f"FURNITURE {name}: {got['triangles']} triangles, {got['tinted_vertices']} tinted vertices")
        for e in errs:
            print(f"  REFUSED {name}: {e}")
            failed.append(f"{name}: {e}")
    manifest = {
        "format": "crabbreeze.furniture/2",
        "generator": GENERATOR,
        "blender": bpy.app.version_string,
        "frame": "metres, glTF axes: y up, front toward +z, origin on the floor at the block item's local origin",
        "material": "furniture: one material and one atlas for every item and the block swatches (one draw call)",
        "atlas": {"size_px": ATLAS, "albedo": "textures/furniture/albedo.png (sRGB)",
                  "normal": "textures/furniture/normal.png (tangent space, OpenGL, +Y up)",
                  "orm": "textures/furniture/orm.png (R occlusion, G roughness, B metal)",
                  "texel_density_px_per_m": {"planned": density, "items": real,
                                             "goods_weight": TEXEL_GOODS},
                  "sha256": {k: hashlib.sha256(open(os.path.join(TEX, f"{k}.png"), "rb").read()).hexdigest()
                             for k in ("albedo", "normal", "orm")}},
        "colour": "COLOR_0 VEC4: rgb 1, alpha 1 on the tinted surface (the placement's tint multiplies the albedo) and 0 elsewhere",
        "uv": "TEXCOORD_0 atlas UVs inside the item's atlas_rect; rects are [u0, v0, u1, v1] in glTF UV (v down from the top)",
        "swatch_px": SWATCH,
        "swatches": swatches,
        "slack_m": SLACK_M,
        "items": items,
    }
    with open(MANIFEST, "w") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")
    if failed:
        raise SystemExit("furniture build refused: " + "; ".join(failed))


# ----------------------------------------------------------------------------- contact sheet

SHEET_TINTS = {"sofa": 0x6A7A4A, "bed": 0x8A3A3A, "kitchen": 0xE8DCC0, "counter": 0xB8A07A, "booth": 0xB02A2A,
               "stool": 0xB02A2A, "chair": 0x3A3A3A}


def atlas_material(tint):
    """What the game draws: the atlas, the tint multiplied in where COLOR_0's alpha says."""
    m = bpy.data.materials.new(f"atlas_{tint:06x}")
    m.use_nodes = True
    N = Nodes(m)
    out = N.n("ShaderNodeOutputMaterial")
    p = N.n("ShaderNodeBsdfPrincipled")
    N.link(p.outputs[0], out.inputs["Surface"])
    uv = N.n("ShaderNodeUVMap", uv_map="UVMap").outputs[0]
    imgs = {}
    for k in ("albedo", "normal", "orm"):
        im = bpy.data.images.get(f"{k}.png") or bpy.data.images.load(os.path.join(TEX, f"{k}.png"))
        im.colorspace_settings.name = "sRGB" if k == "albedo" else "Non-Color"
        t = N.n("ShaderNodeTexImage", image=im, interpolation="Cubic")
        N.link(uv, t.inputs[0])
        imgs[k] = t
    attr = N.n("ShaderNodeVertexColor")
    mask = attr.outputs["Alpha"]
    tinted = N.mix(imgs["albedo"].outputs[0], N.mix(imgs["albedo"].outputs[0], lin(tint), 1.0, "MULTIPLY"), mask)
    orm = N.n("ShaderNodeSeparateColor")
    N.link(imgs["orm"].outputs[0], orm.inputs[0])
    ao = N.n("ShaderNodeCombineColor")
    for i in range(3):
        N.link(orm.outputs[0], ao.inputs[i])
    N.link(N.mix(tinted, ao.outputs[0], 1.0, "MULTIPLY"), p.inputs["Base Color"])
    N.link(orm.outputs[1], p.inputs["Roughness"])
    N.link(orm.outputs[2], p.inputs["Metallic"])
    nm = N.n("ShaderNodeNormalMap", uv_map="UVMap")
    N.link(imgs["normal"].outputs[0], nm.inputs["Color"])
    N.link(nm.outputs[0], p.inputs["Normal"])
    return m


def figure(x, z):
    """A 1.8 m figure for scale, standing at (x, z) glTF, built in Blender axes."""
    mat = bpy.data.materials.new("figure")
    mat.use_nodes = True
    mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.25, 0.3, 0.35, 1)
    parts = [((0, 0, 1.68), (0.11, 0.11, 0.12)), ((0, 0, 1.25), (0.21, 0.12, 0.3)), ((-0.1, 0, 0.47), (0.075, 0.075, 0.47)),
             ((0.1, 0, 0.47), (0.075, 0.075, 0.47)), ((-0.27, 0, 1.2), (0.05, 0.05, 0.33)), ((0.27, 0, 1.2), (0.05, 0.05, 0.33))]
    for (px, py, pz), (sx, sy, sz) in parts:
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=16, v_segments=10, radius=1.0)
        bm.transform(Matrix.Translation((x + px, -z + py, pz)) @ Matrix.Diagonal((sx, sy, sz, 1)))
        o = _object(bm, "figure")
        bm.free()
        o.data.materials.append(mat)
        o.data.shade_smooth()


def sheet(only):
    os.makedirs(SHOTS, exist_ok=True)
    rows = [["lift", "cooler", "aisle", "kitchen", "workbench"],
            ["shelf", "counter", "booth", "bed", "table", "sofa"],
            ["desk", "tv", "chair", "washer", "tires", "stool"]]
    names = [n for r in rows for n in r if not only or n in only]

    def setup_world(sc):
        sc.render.engine = "CYCLES"
        sc.cycles.device = "CPU"
        sc.cycles.samples = 24
        sc.cycles.use_denoising = True
        sc.view_settings.view_transform = "AgX"
        w = bpy.data.worlds.new("w")
        w.use_nodes = True
        w.node_tree.nodes["Background"].inputs["Color"].default_value = (0.55, 0.6, 0.68, 1)
        w.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.7
        sc.world = w
        sun = bpy.data.lights.new("sun", "SUN")
        sun.energy = 3.5
        sun.angle = math.radians(8)
        so = bpy.data.objects.new("sun", sun)
        so.rotation_euler = (math.radians(50), math.radians(10), math.radians(-35))
        sc.collection.objects.link(so)
        fl = bpy.data.meshes.new("floor")
        fbm = bmesh.new()
        bmesh.ops.create_grid(fbm, x_segments=1, y_segments=1, size=40)
        fbm.to_mesh(fl)
        fbm.free()
        fo = bpy.data.objects.new("floor", fl)
        fm = bpy.data.materials.new("floor")
        fm.use_nodes = True
        fm.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.42, 0.42, 0.4, 1)
        fl.materials.append(fm)
        sc.collection.objects.link(fo)

    mats = {}

    def load(name, x, z):
        bpy.ops.import_scene.gltf(filepath=os.path.join(OUT, f"{name}.glb"))
        o = bpy.data.objects["MODEL"]
        o.name = f"item_{name}"
        o.location = (x, -z, 0)
        tint = SHEET_TINTS.get(name, WHITE)
        if tint not in mats:
            mats[tint] = atlas_material(tint)
        uvl = o.data.uv_layers[0]
        uvl.name = "UVMap"
        o.data.materials[0] = mats[tint]
        return o

    def camera(sc, target, dist, yaw, pitch, lens=40):
        cam = bpy.data.cameras.new("cam")
        cam.lens = lens
        co = bpy.data.objects.new("cam", cam)
        sc.collection.objects.link(co)
        t = Vector(target)
        d = Vector((math.sin(math.radians(yaw)) * math.cos(math.radians(pitch)), -math.cos(math.radians(yaw)) * math.cos(math.radians(pitch)), math.sin(math.radians(pitch))))
        co.location = t + d * dist
        co.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
        sc.camera = co

    for name in names:  # one still per item, from its front three-quarter
        clear()
        mats.clear()
        sc = bpy.context.scene
        setup_world(sc)
        load(name, 0, 0)
        lo, hi = ITEMS[name][0], ITEMS[name][1]
        size = max(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2])
        camera(sc, ((lo[0] + hi[0]) / 2, -(lo[2] + hi[2]) / 2, hi[1] * 0.45), size * 1.7 + 0.5, 30, 22)
        sc.render.resolution_x, sc.render.resolution_y = 800, 600
        sc.render.filepath = os.path.join(SHOTS, f"{name}.png")
        bpy.ops.render.render(write_still=True)
        print(f"SHOT {name}")
    if only:
        return
    clear()
    mats.clear()
    sc = bpy.context.scene
    setup_world(sc)
    zrow = [-5.0, -1.6, 1.4]
    for r, row in enumerate(rows):
        widths = [ITEMS[n][1][0] - ITEMS[n][0][0] for n in row]
        total = sum(widths) + 0.6 * (len(row) - 1)
        x = -total / 2
        for n, w in zip(row, widths):
            load(n, x + w / 2 - (ITEMS[n][0][0] + ITEMS[n][1][0]) / 2, zrow[r])
            x += w + 0.6
    figure(6.2, 1.4)
    camera(sc, (0.2, 1.2, 0.6), 17.5, 12, 24, lens=35)
    sc.render.resolution_x, sc.render.resolution_y = 1800, 1100
    sc.cycles.samples = 32
    sc.render.filepath = os.path.join(SHOTS, "sheet.png")
    bpy.ops.render.render(write_still=True)
    print("SHOT sheet")


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    only = set()
    if "--only" in argv:
        only = set(argv[argv.index("--only") + 1].split(","))
        unknown = only - set(ITEMS)
        if unknown:
            raise SystemExit(f"unknown items: {', '.join(sorted(unknown))}")
    if "--no-build" not in argv:
        build_all("--quick" in argv)
    if "--sheet" in argv:
        sheet(only)


main()
