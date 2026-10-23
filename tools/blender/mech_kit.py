"""Shared kit for Crab Breeze mech generators (run inside Blender, headless).

Owns: the glTF to Blender axis helper, the part builder (boxes, tapered hulls, cylinders, pistons,
panel insets), the material roles, the node skeleton of docs/design/mech-rig.md, world-space UVs in
metres, the manifest and the glb export. Every mech builder imports it; never copy it into one.

Borrowed from Mining Mike (RubenTipparach/godot-sandbox `mechs/tools/mech_chassis.py` at 3336b88):
the `bl()` axis helper, the PartBuilder idea (one bmesh per part, joined, bevelled) and building parts
on a named node hierarchy that the game drives. Rewritten for high detail: bevels with segments,
panel insets, convex hull armour, PBR material roles and UVs in metres instead of a pixel atlas.

Coordinates in this file are glTF style: +Y up, the mech faces +Z, +X is its left.
"""
import json
import math
import os

import bmesh
import bpy
from mathutils import Matrix, Vector

UV_TILE_M = 2.0  # one texture tile per 2 m of surface, the same density on every part


def bl(v):
    """glTF (x, y, z) to Blender (x, -z, y)."""
    return Vector((v[0], -v[2], v[1]))


def clear_scene():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    for coll in (bpy.data.meshes, bpy.data.materials, bpy.data.images):
        for item in list(coll):
            coll.remove(item)


# ----------------------------------------------------------------------------- materials

ROLE_DEFAULTS = {
    # role: (base colour sRGB, metallic, roughness, emission strength, alpha)
    "hull": ((0.80, 0.82, 0.84), 0.15, 0.45, 0.0, 1.0),
    "trim": ((0.15, 0.30, 0.62), 0.15, 0.40, 0.0, 1.0),
    "dark": ((0.11, 0.12, 0.14), 0.30, 0.55, 0.0, 1.0),
    "metal": ((0.52, 0.53, 0.55), 1.00, 0.32, 0.0, 1.0),
    "joint": ((0.07, 0.07, 0.08), 0.10, 0.80, 0.0, 1.0),
    "hazard": ((0.95, 0.70, 0.08), 0.00, 0.50, 0.0, 1.0),
    "glass": ((0.35, 0.55, 0.70), 0.00, 0.05, 0.0, 0.28),
    "glow": ((0.30, 0.90, 1.00), 0.00, 0.30, 3.0, 1.0),
    "screen": ((0.10, 0.55, 0.40), 0.00, 0.30, 1.2, 1.0),
    "seat": ((0.18, 0.16, 0.14), 0.00, 0.85, 0.0, 1.0),
}


def srgb_to_lin(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def make_materials(prefix, overrides):
    """One Principled material per role; `overrides` maps role to a base colour."""
    mats = {}
    for role, (col, metal, rough, emit, alpha) in ROLE_DEFAULTS.items():
        col = overrides.get(role, col)
        m = bpy.data.materials.new(f"{prefix}_{role}")
        m.use_nodes = True
        b = m.node_tree.nodes["Principled BSDF"]
        lin = tuple(srgb_to_lin(c) for c in col) + (1.0,)
        b.inputs["Base Color"].default_value = lin
        b.inputs["Metallic"].default_value = metal
        b.inputs["Roughness"].default_value = rough
        if emit > 0:
            b.inputs["Emission Color"].default_value = lin
            b.inputs["Emission Strength"].default_value = emit
        if alpha < 1:
            b.inputs["Alpha"].default_value = alpha
            m.surface_render_method = "BLENDED"
        mats[role] = m
    return mats


# ----------------------------------------------------------------------------- skeleton

class Rig:
    """The node skeleton. Each bone is an empty at its joint; parts hang under it."""

    def __init__(self):
        self.nodes = {}

    def bone(self, name, parent, world_pos):
        obj = bpy.data.objects.new(name, None)
        obj.empty_display_type = "PLAIN_AXES"
        obj.empty_display_size = 0.3
        bpy.context.collection.objects.link(obj)
        if parent:
            p = self.nodes[parent]
            obj.parent = p
            obj.location = bl(world_pos) - p.matrix_world.translation
        else:
            obj.location = bl(world_pos)
        bpy.context.view_layer.update()
        self.nodes[name] = obj
        return obj

    def world(self, name):
        bpy.context.view_layer.update()
        return self.nodes[name].matrix_world.translation.copy()

    def socket(self, name, parent, world_pos, rot_x_deg=0.0):
        obj = self.bone(name, parent, world_pos)
        obj.rotation_euler = (math.radians(rot_x_deg), 0, 0)
        return obj


# ----------------------------------------------------------------------------- part builder

class Part:
    """One bmesh per material role, built in world glTF coordinates, attached to a bone."""

    def __init__(self, rig, bone, name):
        self.rig, self.bone, self.name = rig, bone, name
        self.roles = {}

    def _bm(self, role):
        if role not in self.roles:
            self.roles[role] = bmesh.new()
        return self.roles[role]

    def box(self, role, c, s, inset=True, rot=None):
        """Axis aligned box at centre c with size s (glTF), panel-inset on its big faces."""
        bm = self._bm(role)
        before = set(bm.faces)
        M = Matrix.Translation(bl(c))
        if rot:
            M = M @ rot
        M = M @ Matrix.Diagonal(Vector((s[0], s[2], s[1], 1.0)))
        bmesh.ops.create_cube(bm, size=1.0, matrix=M)
        if inset:
            self._panel(bm, [f for f in bm.faces if f not in before])

    def hull(self, role, points, inset=True):
        """Convex hull of glTF points: the tapered armour plates."""
        bm = self._bm(role)
        before = set(bm.faces)
        verts = [bm.verts.new(bl(p)) for p in points]
        res = bmesh.ops.convex_hull(bm, input=verts)
        bmesh.ops.delete(bm, geom=[g for g in res["geom_interior"]], context="VERTS")
        bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(2), verts=bm.verts[:], edges=bm.edges[:])
        if inset:
            self._panel(bm, [f for f in bm.faces if f not in before and f.is_valid])

    def mirror_hull(self, role, points, inset=True):
        """Hull plus its mirror across X: plates come in pairs."""
        self.hull(role, points, inset)
        self.hull(role, [(-x, y, z) for x, y, z in points], inset)

    def cyl(self, role, p0, p1, r0, r1=None, sides=16, cap=True):
        """Cylinder or cone from p0 to p1 (glTF), radius r0 at p0 and r1 at p1."""
        r1 = r0 if r1 is None else r1
        a, b = bl(p0), bl(p1)
        d = b - a
        length = d.length
        rot = d.to_track_quat("Z", "Y").to_matrix().to_4x4()
        M = Matrix.Translation((a + b) / 2) @ rot
        bmesh.ops.create_cone(self._bm(role), cap_ends=cap, segments=sides, radius1=r0, radius2=r1,
                              depth=length, matrix=M)

    def piston(self, p0, p1, r):
        """A hydraulic ram: a dark sleeve over the first 55%, a polished rod for the rest."""
        a, b = Vector(p0), Vector(p1)
        mid = a.lerp(b, 0.55)
        self.cyl("dark", a, mid, r, r, 12)
        self.cyl("metal", mid, b, r * 0.55, r * 0.55, 10)
        self.cyl("joint", a, a.lerp(b, 0.06), r * 1.25, r * 1.25, 12)
        self.cyl("joint", b.lerp(a, 0.06), b, r * 0.9, r * 0.9, 12)

    def vents(self, role, c, w, h, n, depth=0.06, axis="z"):
        """A grille: n slats across a face. axis is the face normal (glTF)."""
        for i in range(n):
            t = (i + 0.5) / n - 0.5
            if axis == "z":
                self.box(role, (c[0], c[1] + t * h, c[2]), (w, h / n * 0.45, depth), inset=False)
            else:
                self.box(role, (c[0], c[1] + t * h, c[2]), (depth, h / n * 0.45, w), inset=False)

    def sphere(self, role, c, r, scale=(1, 1, 1), seg=24):
        M = Matrix.Translation(bl(c)) @ Matrix.Diagonal(Vector((scale[0], scale[2], scale[1], 1.0)))
        bmesh.ops.create_uvsphere(self._bm(role), u_segments=seg, v_segments=seg // 2, radius=r, matrix=M)

    @staticmethod
    def _panel(bm, faces):
        """Inset big faces and push the panel in 15 mm: the panel seam that reads at distance."""
        big = [f for f in faces if f.is_valid and f.calc_area() > 0.8]
        if not big:
            return
        res = bmesh.ops.inset_individual(bm, faces=big, thickness=0.07, depth=-0.015)
        del res

    def finish(self, mats, bevel=0.04, segments=2):
        """Write one object per role under the bone, with world-space UVs and a bevel."""
        bone = self.rig.nodes[self.bone]
        bpy.context.view_layer.update()
        inv = bone.matrix_world.inverted()
        objs = []
        for role, bm in self.roles.items():
            bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
            bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
            uv = bm.loops.layers.uv.new("UVMap")
            for f in bm.faces:
                n = f.normal
                ax = max(range(3), key=lambda k: abs(n[k]))
                for loop in f.loops:
                    co = loop.vert.co
                    u, v = [(co.y, co.z), (co.x, co.z), (co.x, co.y)][ax]
                    loop[uv].uv = (u / UV_TILE_M, v / UV_TILE_M)
            bm.transform(inv)
            me = bpy.data.meshes.new(f"{self.name}_{role}")
            bm.to_mesh(me)
            bm.free()
            obj = bpy.data.objects.new(f"{self.name}_{role}", me)
            bpy.context.collection.objects.link(obj)
            obj.parent = bone
            obj.data.materials.append(mats[role])
            for poly in me.polygons:
                poly.use_smooth = True
            if bevel > 0 and role not in ("glass", "glow", "screen"):
                m = obj.modifiers.new("bevel", "BEVEL")
                m.width = bevel
                m.segments = segments
                m.limit_method = "ANGLE"
                m.angle_limit = math.radians(35)
                m.harden_normals = True
                obj.modifiers.new("wn", "WEIGHTED_NORMAL").keep_sharp = True
            objs.append(obj)
        self.roles = {}
        return objs


# ----------------------------------------------------------------------------- output

def measure():
    """Triangle count and world bounds of everything that will be exported, after modifiers."""
    dg = bpy.context.evaluated_depsgraph_get()
    tris, lo, hi = 0, Vector((1e9,) * 3), Vector((-1e9,) * 3)
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        ev = obj.evaluated_get(dg)
        me = ev.to_mesh()
        me.calc_loop_triangles()
        tris += len(me.loop_triangles)
        for v in me.vertices:
            w = obj.matrix_world @ v.co
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
        ev.to_mesh_clear()
    # back to glTF: x, y = blender z, z = -blender y
    return tris, {"x": [lo.x, hi.x], "y": [lo.z, hi.z], "z": [-hi.y, -lo.y]}


def export(path, manifest_path, meta):
    tris, bounds = measure()
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_apply=True,
                              export_yup=True, export_extras=True)
    meta = dict(meta, triangles=tris, bounds_m={k: [round(a, 3), round(b, 3)] for k, (a, b) in bounds.items()},
                file=os.path.basename(path))
    data = {}
    if os.path.exists(manifest_path):
        data = json.load(open(manifest_path))
    data[meta["id"]] = meta
    with open(manifest_path, "w") as f:
        json.dump(data, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"EXPORTED {meta['id']}: {tris} tris, height {bounds['y'][1]:.2f} m")
    return meta
