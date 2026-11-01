"""The aircraft kit: the primitives every aircraft generator builds with, in the glTF frame and in metres (the
freighter's, `build_freighter.py`, and the airliners', `build_airliners.py`), so lofting a fuselage, an airfoil, a
lathe, a boolean step, the clean-up and the export have one implementation (CLAUDE.md section 6.1).

A generator fills `MATS` (name to Blender material) and `ORDER` (the material slots, in order) before it builds; `obj`
gives every mesh those slots and `CSG` records every boolean step for the manifest.
"""
import hashlib
import math
import os

import bpy  # noqa: I001 (bpy before bmesh, as the pip module needs)
import bmesh
from mathutils import Matrix, Vector

MATS = {}
ORDER = []
# The plating tiles every this many metres (the box projection in `clean`).
TILE_M = 6.0


def g2b(x, y, z):
    """glTF (y up, nose toward -z) to Blender (z up)."""
    return Vector((x, -z, y))


def lin(c):
    return [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]


def new_bm():
    bm = bmesh.new()
    return bm


def set_mat(bm, key):
    i = ORDER.index(key)
    for f in bm.faces:
        f.material_index = i


def loft(rings, cap=True):
    """A closed solid through `rings` (each a list of glTF points, the same count, in order round), capped at both
    ends with a fan to the ring's centre."""
    bm = bmesh.new()
    vs = [[bm.verts.new(g2b(*p)) for p in r] for r in rings]
    n = len(rings[0])
    for a, b in zip(vs, vs[1:]):
        for i in range(n):
            j = (i + 1) % n
            bm.faces.new((a[i], a[j], b[j], b[i]))
    if cap:
        for r, flip in ((vs[0], True), (vs[-1], False)):
            c = sum((v.co for v in r), Vector()) / n
            cv = bm.verts.new(c)
            for i in range(n):
                j = (i + 1) % n
                bm.faces.new((r[j], r[i], cv) if flip else (r[i], r[j], cv))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def hull(points):
    """The convex hull of glTF points."""
    bm = bmesh.new()
    vs = [bm.verts.new(g2b(*p)) for p in points]
    res = bmesh.ops.convex_hull(bm, input=vs)
    junk = [g for g in res["geom_interior"] + res["geom_unused"] if isinstance(g, bmesh.types.BMVert)]
    bmesh.ops.delete(bm, geom=junk, context="VERTS")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def lathe(profile, n, at, axis="z"):
    """A solid of revolution round a glTF axis through `at`: `profile` is (along, radius) pairs."""
    rings = []
    for s, r in profile:
        ring = []
        for k in range(n):
            a = math.tau * k / n
            c, d = math.cos(a) * max(r, 1e-3), math.sin(a) * max(r, 1e-3)
            if axis == "z":
                ring.append((at[0] + c, at[1] + d, at[2] + s))
            else:  # across, along x (a wheel)
                ring.append((at[0] + s, at[1] + c, at[2] + d))
        rings.append(ring)
    return loft(rings)


def obj(name, bm, mat, parent):
    set_mat(bm, mat)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for k in ORDER:
        me.materials.append(MATS[k])
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    o.parent = parent
    return o


def empty(name, at, parent):
    o = bpy.data.objects.new(name, None)
    o.empty_display_type = "PLAIN_AXES"
    bpy.context.collection.objects.link(o)
    o.parent = parent
    o.location = g2b(*at)
    return o


CSG = []


def _boolean_step(target, others, op, label):
    """One Exact boolean step, applied at once: `others` are the cutters (DIFFERENCE) or pieces (UNION), removed after.
    Materials transfer, so a cutter's faces keep the cutter's material (glass windows, dark intakes)."""
    coll = bpy.data.collections.new(f"{target.name}.{label}")
    bpy.context.scene.collection.children.link(coll)
    for o in others:
        for c in o.users_collection:
            c.objects.unlink(o)
        coll.objects.link(o)
    mod = target.modifiers.new(label, "BOOLEAN")
    mod.operation, mod.solver, mod.operand_type = op, "EXACT", "COLLECTION"
    mod.collection = coll
    mod.material_mode = "TRANSFER"
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    for o in list(coll.objects):
        bpy.data.objects.remove(o)
    bpy.data.collections.remove(coll)
    CSG.append(f"{target.name}: {label} ({op.lower()}, {len(others)})")


def clean(o, tile_m=None):
    """Weld, dissolve degenerate and coplanar splits (material kept), triangulate, UV in metres (a texture tile every
    `tile_m`, `TILE_M` unless given), shade smooth with sharp edges over 35 degrees."""
    tile = tile_m or TILE_M
    me = o.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
    bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=1e-4)
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(1.0), verts=bm.verts, edges=bm.edges, delimit={"MATERIAL"})
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bmesh.ops.triangulate(bm, faces=bm.faces, quad_method="BEAUTY", ngon_method="BEAUTY")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    uv = bm.loops.layers.uv.verify()
    bm.normal_update()
    for f in bm.faces:
        n = f.normal
        a = [abs(n.x), abs(n.y), abs(n.z)]
        for lp in f.loops:
            p = lp.vert.co
            if a[2] >= a[0] and a[2] >= a[1]:
                u, v = p.x, p.y
            elif a[0] >= a[1]:
                u, v = p.y, p.z
            else:
                u, v = p.x, p.z
            lp[uv].uv = (u / tile, v / tile)
    for f in bm.faces:
        f.smooth = True
    bm.to_mesh(me)
    bm.free()
    me.set_sharp_from_angle(angle=math.radians(35.0))


def section(z, hw, hh, cy, n, k=2.3):
    """A fuselage ring at station z: a superellipse (exponent k, a little squarer than a circle) of half width hw and
    half height hh round the height cy, n points, starting at the top."""
    out = []
    for i in range(n):
        a = math.tau * i / n
        c, s = math.sin(a), math.cos(a)
        x = hw * math.copysign(abs(c) ** (2 / k), c)
        y = hh * math.copysign(abs(s) ** (2 / k), s)
        out.append((x, cy + y, z))
    return out


def _station(st, z):
    """The fuselage's ring (half width, half height, centre height) at station z, between the listed stations."""
    for (z1, w1, h1, c1), (z2, w2, h2, c2) in zip(st, st[1:]):
        if z1 <= z <= z2:
            t = (z - z1) / max(z2 - z1, 1e-6)
            return w1 + (w2 - w1) * t, h1 + (h2 - h1) * t, c1 + (c2 - c1) * t
    return st[-1][1], st[-1][2], st[-1][3]


def skin_y(st, z, x, k=2.3):
    """The top of the skin over (x, z)."""
    hw, hh, cy = _station(st, z)
    return cy + hh * max(1.0 - (abs(x) / hw) ** k, 0.0) ** (1.0 / k)


def skin_x(st, z, y, k=2.3):
    """The side of the skin at height y over station z (positive x)."""
    hw, hh, cy = _station(st, z)
    return hw * max(1.0 - (abs(y - cy) / hh) ** k, 0.0) ** (1.0 / k)


def airfoil(x, le, chord, y, t, n, vertical=False, side=1.0):
    """An airfoil ring at span station x: n points round, upper then lower, a rounded nose and a sharp tail."""
    ts = {12: [0.0, 0.01, 0.03, 0.07, 0.13, 0.21, 0.32, 0.45, 0.6, 0.75, 0.88, 1.0],
          9: [0.0, 0.025, 0.08, 0.18, 0.32, 0.5, 0.7, 0.86, 1.0], 6: [0.0, 0.05, 0.2, 0.45, 0.75, 1.0],
          4: [0.0, 0.15, 0.55, 1.0]}[n]

    def thick(u):
        return 2.97 * math.sqrt(u) - 1.26 * u - 3.52 * u * u + 2.84 * u ** 3 - 1.0 * u ** 4  # NACA 4-digit, /0.6

    up = [(u, max(thick(u), 0.0) * t * 0.5 * chord) for u in ts]
    pts = [(le + u * chord, h) for u, h in up] + [(le + u * chord, -h * 0.75) for u, h in reversed(up[1:-1])]
    if vertical:  # a fin: the "span" is height, the thickness across x
        return [(h * side, x, z) for z, h in pts]
    return [(x * side, y + h, z) for z, h in pts]


def lifting(name, root, tip, n, mat, parent, side=1.0, vertical=False, mid=False):
    """A wing, tailplane or fin lofted from root to tip (and a ring half way at full detail)."""
    secs = [root, tip]
    if mid:
        secs = [root, tuple((a + b) * 0.5 for a, b in zip(root, tip)), tip]
    rings = [airfoil(s[0], s[1], s[2], s[3], s[4], n, vertical, side) for s in secs]
    return obj(name, loft(rings), mat, parent)


def light(name, at, mat, parent, r=0.18):
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=r, matrix=Matrix.Translation(g2b(*at)))
    return obj(name, bm, mat, parent)


def tris_under(node):
    dg = bpy.context.evaluated_depsgraph_get()
    n = 0
    for o in [node] + list(node.children_recursive):
        if o.type == "MESH":
            me = o.evaluated_get(dg).to_mesh()
            me.calc_loop_triangles()
            n += len(me.loop_triangles)
            o.evaluated_get(dg).to_mesh_clear()
    return n


def bounds_under(node):
    lo, hi = Vector((1e9,) * 3), Vector((-1e9,) * 3)
    for o in [node] + list(node.children_recursive):
        if o.type != "MESH":
            continue
        for v in o.data.vertices:
            w = o.matrix_world @ v.co
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
    # Blender (x, y, z) back to glTF (x, z, -y)
    return {"x": [round(lo.x, 3), round(hi.x, 3)], "y": [round(lo.z, 3), round(hi.z, 3)], "z": [round(-hi.y, 3), round(-lo.y, 3)]}


def export(path, objs):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=True, export_yup=True,
                              export_extras=False, export_image_format="JPEG", export_jpeg_quality=88, export_cameras=False,
                              export_lights=False, export_animations=False, export_skins=False, export_morph=False)
    return hashlib.sha256(open(path, "rb").read()).hexdigest()




# ------------------------------------------------- shared by the aircraft generators (moved from build_airliners.py)

def volume(o):
    bm = bmesh.new()
    bm.from_mesh(o.data)
    v = abs(bm.calc_volume(signed=True))
    bm.free()
    return v



def boolean(target, others, op, label, hollowing=False):
    """The kit's boolean step, refusing one that leaves nothing (an Exact boolean on bad input fails silently)."""
    before = len(target.data.polygons)
    vol0 = volume(target)
    _boolean_step(target, others, op, label)
    if before and op != "INTERSECT" and not hollowing and volume(target) < vol0 * 0.5:
        raise SystemExit(f"{target.name}: the {label} {op.lower()} lost half the solid ({vol0:.2f} to {volume(target):.2f} m3)")
    if os.environ.get("CB_DEBUG_BOOL"):
        print("BOOL", target.name, label, op, before, "->", len(target.data.polygons))



def box_into(bm, c, half, axes=None):
    """A box into `bm`: centre `c` (glTF), half sizes along `axes` (three glTF vectors, the world's by default)."""
    ax = axes or ((1, 0, 0), (0, 1, 0), (0, 0, 1))
    ax = [Vector(a) for a in ax]
    c = Vector(c)
    vs = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                p = c + ax[0] * (sx * half[0]) + ax[1] * (sy * half[1]) + ax[2] * (sz * half[2])
                vs.append(bm.verts.new(g2b(*p)))
    for f in ((0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)):
        bm.faces.new([vs[i] for i in f])



def cyl_into(bm, a, b, r, n):
    """A capped cylinder from glTF `a` to `b`."""
    a, b = Vector(a), Vector(b)
    d = (b - a).normalized()
    u = d.orthogonal().normalized()
    v = d.cross(u)
    rings = []
    for p in (a, b):
        rings.append([bm.verts.new(g2b(*(p + (u * math.cos(math.tau * i / n) + v * math.sin(math.tau * i / n)) * r)))
                      for i in range(n)])
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((rings[0][i], rings[0][j], rings[1][j], rings[1][i]))
    bm.faces.new(list(reversed(rings[0])))
    bm.faces.new(rings[1])



def mesh(name, bm, mat, parent, at=None):
    """An object from `bm` with every face `mat` (a slot name), under `parent`; with `at` (the parent pivot's glTF
    position) its vertices are moved into the pivot's frame."""
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    if at is not None:
        bmesh.ops.translate(bm, verts=bm.verts, vec=-g2b(*at))
    return obj(name, bm, mat, parent)



def copy_obj(o, name):
    c = o.copy()
    c.data = o.data.copy()
    c.name = name
    for coll in o.users_collection:
        coll.objects.link(c)
    return c



def box_obj(name, lo, hi, mat, parent):
    """An axis-aligned glTF box from corner `lo` to `hi`."""
    bm = bmesh.new()
    c = [(a + b) * 0.5 for a, b in zip(lo, hi)]
    box_into(bm, c, [(b - a) * 0.5 for a, b in zip(lo, hi)])
    return mesh(name, bm, mat, parent)



def prism_obj(name, pts, mat, parent):
    return obj(name, hull(pts), mat, parent)



def move_to_pivot(o, at):
    """Move a finished object's vertices into the frame of a pivot at glTF `at` (its parent)."""
    o.data.transform(Matrix.Translation(-g2b(*at)))



def meshes_under(node):
    return [o for o in [node] + list(node.children_recursive) if o.type == "MESH"]



def tris(objs):
    n = 0
    for o in objs:
        o.data.calc_loop_triangles()
        n += len(o.data.loop_triangles)
    return n



def lod_of(o):
    return int(o.name.rsplit(".L", 1)[1][0]) if ".L" in o.name else None



def world_verts(o):
    return [o.matrix_world @ v.co for v in o.data.vertices]



def to_gltf(v):
    return (v.x, v.z, -v.y)



def capsules_of(points, n_max=3):
    """Capsules round a cloud of glTF points: along its long axis, split across its second axis while that is much
    wider than the third (a wing gets two or three, a fuselage one). Each (a, b, radius)."""
    import numpy as np

    pts = np.array(points)
    c = pts.mean(axis=0)
    u, s, vt = np.linalg.svd(pts - c, full_matrices=False)
    ax = vt
    proj = (pts - c) @ ax.T
    ext = proj.max(axis=0) - proj.min(axis=0)
    k = 1
    if ext[1] > 2.5 * max(ext[2], 0.2):
        k = int(min(n_max, max(2, round(ext[1] / max(ext[2] * 2.0, 0.5)))))
    out = []
    lo1, hi1 = proj[:, 1].min(), proj[:, 1].max()
    for i in range(k):
        a1, b1 = lo1 + (hi1 - lo1) * i / k, lo1 + (hi1 - lo1) * (i + 1) / k
        sel = proj[(proj[:, 1] >= a1 - 1e-6) & (proj[:, 1] <= b1 + 1e-6)]
        if len(sel) < 2:
            continue
        mid1 = (a1 + b1) * 0.5
        d = np.sqrt((sel[:, 1] - mid1) ** 2 + (sel[:, 2] - np.median(sel[:, 2])) ** 2)
        r = float(np.percentile(d, 90))
        lo0, hi0 = sel[:, 0].min() + r, sel[:, 0].max() - r
        if lo0 > hi0:
            lo0 = hi0 = (lo0 + hi0) * 0.5
        mid2 = float(np.median(sel[:, 2]))
        a = c + ax[0] * lo0 + ax[1] * mid1 + ax[2] * mid2
        b = c + ax[0] * hi0 + ax[1] * mid1 + ax[2] * mid2
        out.append(dict(a=[round(float(v), 3) for v in a], b=[round(float(v), 3) for v in b], radius_m=round(r, 3)))
    return out



class Model:
    """Collects one kind's nodes, zones, rig and lamps while it is built."""

    def __init__(self, p):
        self.p = p
        self.root = empty(p.kind, (0, 0, 0), None)
        self.zones = {}
        self.parents = {}
        self.rig = []
        self.lamps = []
        self.seats = 0
        self.interior = []

    def zone(self, name, parent=None):
        if name not in self.zones:
            node = empty(f"ZONE_{name}", (0, 0, 0), self.zones[parent] if parent else self.root)
            self.zones[name] = node
            self.parents[name] = parent
        return self.zones[name]

    def pivot(self, name, at, zone, axis, **travel):
        node = empty(f"RIG_{name}", at, self.zones[zone])
        self.rig.append(dict(node=f"RIG_{name}", zone=zone, at=[round(v, 4) for v in at],
                             axis=[round(v, 5) for v in Vector(axis).normalized()], **travel))
        return node

    def lamp(self, name, at, zone, colour, pattern):
        empty(f"LAMP_{name}", at, self.zones[zone])
        self.lamps.append(dict(node=f"LAMP_{name}", zone=zone, at=[round(v, 4) for v in at], colour=colour,
                               pattern=pattern))


