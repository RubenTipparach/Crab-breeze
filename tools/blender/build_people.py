"""Crab Breeze's people: MPFB 2 bodies with Universal Animation Library clips, baked into vertex animation
textures (openspec/changes/people-vat, design sections 1-3).

It owns the people's models and their bakes. From tools/blender/people.json it writes, per body:
  - assets/models/people/<id>.glb: the body at both levels of detail (meshes `<id>_near` and `<id>_crowd`),
    skinned to MPFB's 53-bone game_engine rig, with every clip as an animation (for the review page and the
    manifest check; the game draws the bakes, not this);
  - assets/models/people/<id>_<lod>.vat: the level's triangles, each vertex's colour region and shade, and every
    clip's frames, each vertex's position (16 bits a component over the level's bounds) and normal
    (octahedral, 8 bits a component), one row of the texture per frame;
and assets/models/people/people_vat.json, the manifest: the clips (row, frames, rate, loop, the speed a moving
clip walks at), each body's levels with their counts, bounds and triangle budgets, and every file's SHA-256.

How a body is made (the humanoid pipeline, `blender-humanoid-characters`, after Undercity's build_npcs.py at
fps-game-demo 1363650):
  - MPFB's HumanService builds the human from the row (macro sliders, the low-poly proxy, CC0 clothes, hair and
    eyes), its height fitted to height_m by bisection on the height slider; the clothes' delete groups are applied;
  - everything is joined into one mesh; every vertex takes a colour region (skin, top, bottom, shoes, hair) by the
    piece it came from and the bones that move it, and a shade from its own texture's brightness there, so the
    folds, seams, lips and brows of the CC0 textures survive as vertex shading under the run-time palette;
  - each level is the full body through a Decimate collapse at its ratio (1 keeps it whole), its regions and
    shades taken from the nearest full vertex;
  - the clips are UAL's (CC0) retargeted bone by bone onto the game_engine rig: every mapped bone takes the source
    bone's rotation from its rest pose in world space, after its own rest direction is turned onto the source's
    (UAL rests in a T-pose, MPFB in an A-pose); the hips move by the source's hips, scaled by the hip heights.
    The wave is authored here over the idle: the right arm raised, the forearm swinging.
The build refuses an asset outside the pinned packs and a level over its triangle budget (read back from the file it
wrote), naming the body. The same table and packs give the same bytes (`--verify`).

Run:
  python3 tools/deps/fetch_character_tools.py
  /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/blender/build_people.py
  (-- --verify rebuilds and fails unless every file matches the committed one, writing nothing)
The script restarts Blender once with BLENDER_USER_RESOURCES in the dependency cache, so MPFB never touches the
user's own Blender settings.

Axes: MPFB and UAL build with the body facing -Y, Z up. The bakes are in the game's person space, +Y up and the body
facing -Z (`(x, y, z)` in Blender is `(-x, z, y)` there); the glb is the glTF exporter's +Y up (the body faces +Z).
"""
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
import traceback

import bpy
import numpy as np

# numpy on macOS (Accelerate) flags spurious divide and overflow warnings in plain matrix products.
np.seterr(all="ignore")
from mathutils import Matrix, Quaternion, Vector
from mathutils.kdtree import KDTree

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools", "deps"))
import fetch_character_tools as deps  # noqa: E402

TABLE = os.path.join(HERE, "people.json")
MPFB_PACK = "mpfb"
ASSET_PACK = "makehuman_system_assets_cc0"
UAL_PACK = "universal_animation_library_standard"
MPFB_MODULE = "bl_ext.user_default.mpfb"
VAT_MAGIC = b"CBVAT001"
REGIONS = ("skin", "top", "bottom", "shoes", "hair")
SKIN, TOP, BOTTOM = 0, 1, 2


class BuildError(RuntimeError):
    """A body that can't be built as the table asks. The message names the body."""


# ----------------------------------------------------------------------------- environment

def private_blender_dir():
    return os.path.join(deps.cache_dir(), "blender", "%d.%d" % bpy.app.version[:2])


def ensure_private_blender():
    """Restart Blender once with the private user folder, so MPFB never lands in the user's own."""
    want = private_blender_dir()
    if os.path.realpath(os.environ.get("BLENDER_USER_RESOURCES", "")) == os.path.realpath(want):
        return
    os.makedirs(want, exist_ok=True)
    env = dict(os.environ, BLENDER_USER_RESOURCES=want, PYTHONHASHSEED="0")
    sys.exit(subprocess.run([bpy.app.binary_path] + sys.argv[1:], env=env).returncode)


def install_packs():
    """Verify every pack, unpack MPFB and the CC0 assets, enable MPFB; the allowlist of pack files."""
    import addon_utils
    packs = deps.load_all()
    for p in packs:
        deps.verify(p)
    repo = next(r for r in bpy.context.preferences.extensions.repos if r.module == "user_default")
    mpfb_dir = deps.unpack(deps.pack_by_id(MPFB_PACK, packs), os.path.join(repo.directory, "mpfb"))
    addon_utils.enable(MPFB_MODULE, default_set=True)
    from bl_ext.user_default.mpfb.services import AssetService, LocationService
    data_dir = LocationService.get_user_data()
    deps.unpack(deps.pack_by_id(ASSET_PACK, packs), data_dir)
    AssetService.rescan_pack_metadata()
    AssetService.update_all_asset_lists()
    allow = deps.Allowlist({MPFB_PACK: mpfb_dir, ASSET_PACK: data_dir}, [p for p in packs if p["id"] != UAL_PACK])
    return allow, deps.zip_path(deps.pack_by_id(UAL_PACK, packs))


def services():
    from bl_ext.user_default.mpfb.entities.objectproperties import GeneralObjectProperties, HumanObjectProperties
    from bl_ext.user_default.mpfb.services import HumanService, TargetService
    return HumanService, TargetService, HumanObjectProperties, GeneralObjectProperties


# ----------------------------------------------------------------------------- the body

def clean_scene():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o, do_unlink=True)
    for coll in (bpy.data.meshes, bpy.data.armatures, bpy.data.materials, bpy.data.images,
                 bpy.data.node_groups, bpy.data.actions):
        for d in list(coll):
            coll.remove(d)
    bpy.data.orphans_purge(do_local_ids=True, do_linked_ids=True, do_recursive=True)


def coords(obj, evaluated=False):
    """Vertex positions in world space, (n, 3) float64."""
    if evaluated:
        ev = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
        me = ev.to_mesh()
    else:
        me = obj.data
    co = np.empty(len(me.vertices) * 3, np.float64)
    me.vertices.foreach_get("co", co)
    if evaluated:
        ev.to_mesh_clear()
    co = co.reshape(-1, 3)
    mw = np.array(obj.matrix_world, np.float64)
    return co @ mw[:3, :3].T + mw[:3, 3]


def fit_height(body, hs, ts, props):
    """MPFB's height slider that gives height_m, by bisection on a bare base mesh."""
    macro = dict(body["macro"], gender=1.0 if body["sex"] == "male" else 0.0, height=0.5)
    macro["race"] = dict(body["macro"]["race"])
    base = hs.create_human(macro_detail_dict=macro)

    def at(v):
        props.set_value("height", v, entity_reference=base)
        ts.reapply_macro_details(base)
        bpy.context.view_layer.update()
        z = coords(base, evaluated=True)[:, 2]
        return float(z.max() - z.min())

    lo, hi, best = 0.0, 1.0, None
    for _ in range(30):
        v = 0.5 * (lo + hi)
        h = at(v)
        if best is None or abs(h - body["height_m"]) < abs(best[1] - body["height_m"]):
            best = (v, h)
        if abs(h - body["height_m"]) <= 0.005:
            break
        lo, hi = (v, hi) if h < body["height_m"] else (lo, v)
    if abs(best[1] - body["height_m"]) > 0.005:
        raise BuildError(f"{body['id']}: height fit ended at {best[1]:.4f} m for {body['height_m']} m")
    mesh = base.data
    bpy.data.objects.remove(base, do_unlink=True)
    bpy.data.meshes.remove(mesh)
    return round(best[0], 6), best[1], macro


def build_human(body, table, hs, macro):
    info = hs._create_default_human_info_dict()
    info["name"] = body["id"]
    info["phenotype"] = macro
    info["rig"] = table["rig"]
    info["proxy"] = "{0}/{0}.proxy".format(body["proxy"])
    info["eyes"] = "low-poly/low-poly.mhclo"
    info["eyebrows"] = ""
    info["eyelashes"] = ""
    info["hair"] = "{0}/{0}.mhclo".format(body["hair"]) if body["hair"] else ""
    info["skin_mhmat"] = "{0}/{0}.mhmat".format(body["skin"])
    info["skin_material_type"] = "GAMEENGINE"
    info["clothes"] = ["{0}/{0}.mhclo".format(c) for c in body["clothes"]]
    settings = hs.get_default_deserialization_settings()
    settings["subdiv_levels"] = 0
    settings["override_clothes_model"] = "GAMEENGINE"
    settings["override_eyes_model"] = "GAMEENGINE"
    return hs.deserialize_from_dict(info, settings)


def with_obj(obj, fn, **kw):
    with bpy.context.temp_override(object=obj, active_object=obj, selected_objects=[obj],
                                   selected_editable_objects=[obj]):
        return fn(**kw)


def apply_masks(obj):
    for mod in list(obj.modifiers):
        if mod.type == 'MASK':
            with_obj(obj, bpy.ops.object.modifier_move_to_index, modifier=mod.name, index=0)
            with_obj(obj, bpy.ops.object.modifier_apply, modifier=mod.name)
        elif mod.type == 'SUBSURF':
            obj.modifiers.remove(mod)


def texture_of(obj):
    """The colour texture of the object's material as an (h, w, 4) array, or None."""
    mat = obj.active_material
    if not mat or not mat.node_tree:
        return None
    for name in ("DiffuseTexture", "diffuseTexture"):
        n = mat.node_tree.nodes.get(name)
        if n and n.image:
            img = n.image
            px = np.empty(img.size[0] * img.size[1] * 4, np.float32)
            img.pixels.foreach_get(px)
            return px.reshape(img.size[1], img.size[0], 4)
    for n in mat.node_tree.nodes:
        if n.type == 'TEX_IMAGE' and n.image and "normal" not in n.image.name.lower():
            img = n.image
            px = np.empty(img.size[0] * img.size[1] * 4, np.float32)
            img.pixels.foreach_get(px)
            return px.reshape(img.size[1], img.size[0], 4)
    return None


def vertex_shades(obj):
    """Each vertex's shade: its texture's brightness under its UVs over the piece's median, 0.55-1.45."""
    n = len(obj.data.vertices)
    tex = texture_of(obj)
    if tex is None or not obj.data.uv_layers:
        return np.ones(n)
    uv = np.empty(len(obj.data.loops) * 2, np.float64)
    obj.data.uv_layers.active.data.foreach_get("uv", uv)
    uv = uv.reshape(-1, 2)
    vi = np.empty(len(obj.data.loops), np.int64)
    obj.data.loops.foreach_get("vertex_index", vi)
    h, w = tex.shape[:2]
    x = np.clip((uv[:, 0] % 1.0) * w, 0, w - 1).astype(np.int64)
    y = np.clip((uv[:, 1] % 1.0) * h, 0, h - 1).astype(np.int64)
    lum = tex[y, x, :3] @ np.array([0.2126, 0.7152, 0.0722])
    acc, cnt = np.zeros(n), np.zeros(n)
    np.add.at(acc, vi, lum)
    np.add.at(cnt, vi, 1.0)
    lum_v = np.where(cnt > 0, acc / np.maximum(cnt, 1), np.nan)
    med = float(np.nanmedian(lum_v)) if np.isfinite(lum_v).any() else 1.0
    return np.clip(np.nan_to_num(lum_v / max(med, 1e-3), nan=1.0), 0.55, 1.45)


def dominant_bone(obj, bone_names):
    names = {g.index: g.name for g in obj.vertex_groups if g.name in bone_names}
    out = []
    for v in obj.data.vertices:
        best, bw = "", 0.0
        for g in v.groups:
            if g.group in names and g.weight > bw:
                best, bw = names[g.group], g.weight
        out.append(best)
    return out


def make_body(body, table, allow):
    """The fitted body joined into one mesh with `region` and `shade` per vertex, and its rig."""
    hs, ts, props, gop = services()
    clean_scene()
    slider, fitted, macro = fit_height(body, hs, ts, props)
    macro["height"] = slider
    basemesh = build_human(body, table, hs, macro)
    rig = basemesh.parent
    for img in bpy.data.images:
        if img.filepath and allow.pack_of(bpy.path.abspath(img.filepath)) is None:
            raise BuildError(f"{body['id']}: texture {img.filepath} is not a file of a pinned pack; refused")
    parts = sorted([o for o in bpy.data.objects if o.type == 'MESH' and o is not basemesh], key=lambda o: o.name)
    kinds = {o.name: str(gop.get_value("object_type", entity_reference=o)) for o in parts}
    bpy.data.objects.remove(basemesh, do_unlink=True)
    bones = {b.name for b in rig.data.bones}
    bottom = set(table["bottom_bones"])
    by_asset = table["region_of_asset"]
    for o in parts:
        apply_masks(o)
        asset = o.name.split(".")[-1]
        if asset in table["decimate"]:
            dec = o.modifiers.new("Decimate", 'DECIMATE')
            dec.ratio = table["decimate"][asset]
            dec.use_symmetry = True
            with_obj(o, bpy.ops.object.modifier_move_to_index, modifier=dec.name, index=0)
            with_obj(o, bpy.ops.object.modifier_apply, modifier=dec.name)
        kind = kinds[o.name]
        n = len(o.data.vertices)
        if asset in by_asset:
            region = [by_asset[asset]] * n
        elif kind == "Clothes":
            region = [BOTTOM if b in bottom else TOP for b in dominant_bone(o, bones)]
        elif kind == "Hair":
            region = [4] * n
        else:
            region = [SKIN] * n
        shade = vertex_shades(o)
        a = o.data.attributes.new("region", 'INT', 'POINT')
        a.data.foreach_set("value", region)
        s = o.data.attributes.new("shade", 'FLOAT', 'POINT')
        s.data.foreach_set("value", shade.astype(np.float32))
    skin = next(o for o in parts if kinds[o.name] == "Proxymeshes")
    with bpy.context.temp_override(object=skin, active_object=skin, selected_objects=parts,
                                   selected_editable_objects=parts):
        bpy.ops.object.join()
    skin.name = skin.data.name = body["id"] + "_full"
    rig.name = body["id"]
    for g in list(skin.vertex_groups):
        if g.name not in bones:
            skin.vertex_groups.remove(g)
    for mod in skin.modifiers:
        if mod.type == 'ARMATURE':
            mod.object = rig
    skin.data.materials.clear()
    with_obj(skin, bpy.ops.object.shade_smooth)
    return skin, rig, fitted


def make_crowd_lod(near, ratio, name):
    """A decimated level: a copy of the full body through a Decimate collapse, regions and shades from the nearest
    full vertex."""
    me = near.data.copy()
    obj = bpy.data.objects.new(name, me)
    me.name = name
    bpy.context.scene.collection.objects.link(obj)
    obj.parent = near.parent
    obj.matrix_world = near.matrix_world.copy()
    for mod in list(obj.modifiers):
        obj.modifiers.remove(mod)
    dec = obj.modifiers.new("Decimate", 'DECIMATE')
    dec.ratio = ratio
    dec.use_symmetry = True
    with_obj(obj, bpy.ops.object.modifier_apply, modifier=dec.name)
    arm = obj.modifiers.new("Armature", 'ARMATURE')
    arm.object = near.parent
    src = coords(near)
    tree = KDTree(len(src))
    for i, p in enumerate(src):
        tree.insert(p, i)
    tree.balance()
    reg = np.empty(len(near.data.vertices), np.int32)
    near.data.attributes["region"].data.foreach_get("value", reg)
    sh = np.empty(len(near.data.vertices), np.float32)
    near.data.attributes["shade"].data.foreach_get("value", sh)
    dst = coords(obj)
    nearest = [tree.find(p)[1] for p in dst]
    for name_, arr in (("region", reg), ("shade", sh)):
        if name_ in obj.data.attributes:
            obj.data.attributes.remove(obj.data.attributes[name_])
    a = obj.data.attributes.new("region", 'INT', 'POINT')
    a.data.foreach_set("value", reg[nearest])
    s = obj.data.attributes.new("shade", 'FLOAT', 'POINT')
    s.data.foreach_set("value", sh[nearest])
    return obj


def tri_count(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


# ----------------------------------------------------------------------------- clips

def import_ual(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    src = next(o for o in new if o.type == 'ARMATURE')
    for o in new:
        if o.type == 'MESH':
            bpy.data.objects.remove(o, do_unlink=True)
    actions = {a.name.split("|")[-1]: a for a in bpy.data.actions}
    return src, actions


def set_action(arm, action):
    if arm.animation_data is None:
        arm.animation_data_create()
    arm.animation_data.action = action
    if hasattr(arm.animation_data, "action_slot") and action is not None and len(action.slots):
        arm.animation_data.action_slot = action.slots[0]


def hierarchy(arm):
    out = []

    def walk(b):
        out.append(b)
        for c in sorted(b.children, key=lambda c: c.name):
            walk(c)
    for b in sorted([b for b in arm.data.bones if b.parent is None], key=lambda b: b.name):
        walk(b)
    return out


def world_dir(m):
    """A bone's direction (its Y axis) from its world matrix."""
    return (m.to_3x3() @ Vector((0.0, 1.0, 0.0))).normalized()


class Retarget:
    """Poses the target rig from the source rig's current pose (module docstring)."""

    def __init__(self, src, tgt, bone_map):
        self.src, self.tgt, self.map = src, tgt, bone_map
        self.order = hierarchy(tgt)
        sw, tw = src.matrix_world, tgt.matrix_world
        self.rest_s = {b.name: sw @ b.matrix_local for b in src.data.bones}
        self.rest_t = {b.name: tw @ b.matrix_local for b in tgt.data.bones}
        # the target's rest turned onto the source's rest direction, bone by bone
        self.ref_t = {}
        for t, s in bone_map.items():
            if t not in self.rest_t or s not in self.rest_s:
                raise BuildError(f"bone map: {t} -> {s} names a bone missing from a rig")
            align = world_dir(self.rest_t[t]).rotation_difference(world_dir(self.rest_s[s]))
            self.ref_t[t] = align @ self.rest_t[t].to_quaternion()
        self.hip_scale = self.rest_t["pelvis"].translation.z / self.rest_s[bone_map["pelvis"]].translation.z

    def bases(self):
        """Each target bone's matrix_basis for the source's current pose."""
        tw_inv = self.tgt.matrix_world.inverted()
        ws = {pb.name: self.src.matrix_world @ pb.matrix for pb in self.src.pose.bones}
        pose, out = {}, {}
        for b in self.order:
            rest = b.matrix_local
            c = pose[b.parent.name] @ b.parent.matrix_local.inverted() @ rest if b.parent else rest.copy()
            s = self.map.get(b.name)
            if s:
                delta = ws[s].to_quaternion() @ self.rest_s[s].to_quaternion().inverted()
                q = tw_inv.to_quaternion() @ (delta @ self.ref_t[b.name])
                loc = c.translation
                if b.name == "pelvis":
                    move = ws[s].translation - self.rest_s[s].translation
                    loc = (tw_inv @ (self.rest_t["pelvis"].translation + move * self.hip_scale))
                want = Matrix.Translation(loc) @ q.to_matrix().to_4x4()
            else:
                want = c
            out[b.name] = c.inverted() @ want
            pose[b.name] = want
        return out


def pose_from_bases(tgt, order, bases, override):
    """Armature-space pose matrices from bases, with `override(name, chain) -> basis or None` replacing some."""
    pose = {}
    for b in order:
        c = pose[b.parent.name] @ b.parent.matrix_local.inverted() @ b.matrix_local if b.parent else b.matrix_local
        o = override(b.name, c)
        if o is not None:
            bases[b.name] = o
        pose[b.name] = c @ bases[b.name]
    return pose


def aim(c, want_dir):
    """The basis that points a bone with chain matrix `c` (armature space) along want_dir."""
    have = world_dir(c)
    q = have.rotation_difference(want_dir.normalized()) @ c.to_quaternion()
    want = Matrix.Translation(c.translation) @ q.to_matrix().to_4x4()
    return c.inverted() @ want


def wave_override(cfg, t):
    """The wave over the idle: the right upper arm raised and out, the forearm swinging (module docstring)."""
    r, out = math.radians(cfg["raise_deg"]), math.radians(cfg["out_deg"])
    upper = Vector((-math.sin(r), -math.sin(out), -math.cos(r)))
    swing = math.radians(cfg["forearm_deg"]) + math.radians(cfg["swing_deg"]) * math.sin(2 * math.pi * cfg["hz"] * t)
    fore = Vector((-math.sin(swing), -0.25, math.cos(swing)))

    def override(name, c):
        if name == "upperarm_r":
            return aim(c, upper)
        if name == "lowerarm_r":
            return aim(c, fore)
        return None
    return override


def clip_samples(clip, src_actions, cfg_fps):
    """(source action, sample times in source frames, rate in samples a second) of a clip."""
    act = src_actions.get(clip["source"])
    if act is None:
        raise BuildError(f"clip {clip['name']}: source {clip['source']!r} is not in the UAL pack")
    f0, f1 = act.frame_range
    length = clip.get("seconds", (f1 - f0) / 24.0) * 24.0
    n = max(2, int(round(length / 24.0 * cfg_fps)))
    if clip["loop"]:
        times = [f0 + i * length / n for i in range(n)]
    else:
        times = [f0 + i * length / (n - 1) for i in range(n)]
    rate = n / (length / 24.0) if clip["loop"] else (n - 1) / (length / 24.0)
    return act, times, rate


def key_clip(tgt, retarget, order, clip, src_actions, cfg, scene):
    """The clip as an action on the target rig, keyed at the bake's sample times; returns (action, times, rate)."""
    act, times, rate = clip_samples(clip, src_actions, cfg["bake_fps"])
    set_action(retarget.src, act)
    action = bpy.data.actions.new(clip["name"])
    action.use_fake_user = True
    set_action(tgt, action)
    for pb in tgt.pose.bones:
        pb.rotation_mode = 'QUATERNION'
    for i, t in enumerate(times):
        scene.frame_set(int(math.floor(t)), subframe=t - math.floor(t))
        bases = retarget.bases()
        if clip.get("author") == "wave":
            pose_from_bases(tgt, order, bases, wave_override(cfg["wave"], i / rate))
        for b in order:
            pb = tgt.pose.bones[b.name]
            m = bases[b.name]
            pb.location = m.translation
            pb.rotation_quaternion = m.to_quaternion()
            pb.keyframe_insert("location", frame=i)
            pb.keyframe_insert("rotation_quaternion", frame=i)
    if clip["loop"]:  # the closing key, the first pose again, so the exported loop closes
        scene.frame_set(int(math.floor(times[0])), subframe=times[0] - math.floor(times[0]))
        bases = retarget.bases()
        if clip.get("author") == "wave":
            pose_from_bases(tgt, order, bases, wave_override(cfg["wave"], 0.0))
        for b in order:
            pb = tgt.pose.bones[b.name]
            m = bases[b.name]
            pb.location = m.translation
            pb.rotation_quaternion = m.to_quaternion()
            pb.keyframe_insert("location", frame=len(times))
            pb.keyframe_insert("rotation_quaternion", frame=len(times))
    for fc in action.fcurves:
        for k in fc.keyframe_points:
            k.interpolation = 'LINEAR'
    return action, len(times), rate


# ----------------------------------------------------------------------------- the bake

def to_person(a):
    """Blender (x, y, z) to the game's person space (-x, z, y): +Y up, the body facing -Z."""
    return np.stack([-a[:, 0], a[:, 2], a[:, 1]], axis=1)


def evaluated(obj):
    ev = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    me = ev.to_mesh()
    n = len(me.vertices)
    co = np.empty(n * 3, np.float32)
    me.vertices.foreach_get("co", co)
    nor = np.empty(n * 3, np.float32)
    me.vertex_normals.foreach_get("vector", nor)
    ev.to_mesh_clear()
    mw = np.array(obj.matrix_world, np.float64)
    co = co.reshape(-1, 3) @ mw[:3, :3].T + mw[:3, 3]
    nor = nor.reshape(-1, 3) @ mw[:3, :3].T
    return to_person(co), to_person(nor)


def oct_encode(n):
    """Octahedral normals to two bytes each."""
    n = n / np.maximum(np.abs(n).sum(axis=1, keepdims=True), 1e-9)
    x, y, z = n[:, 0], n[:, 1], n[:, 2]
    ox = np.where(z >= 0, x, (1 - np.abs(y)) * np.sign(x + 1e-30))
    oy = np.where(z >= 0, y, (1 - np.abs(x)) * np.sign(y + 1e-30))
    q = lambda v: np.clip(np.round((v * 0.5 + 0.5) * 255.0), 0, 255).astype(np.uint16)  # noqa: E731
    return q(ox) | (q(oy) << 8)


def triangles(obj):
    me = obj.data
    me.calc_loop_triangles()
    tv = np.empty(len(me.loop_triangles) * 3, np.uint32)
    me.loop_triangles.foreach_get("vertices", tv)
    # wound for +Y up after the axis swap (the swap is a proper rotation, so the order stands)
    return tv


def bake_level(obj, rig, clips, scene):
    """Every clip's frames of one level: (positions (rows, n, 3), normals (rows, n, 3))."""
    pos, nor = [], []
    for c in clips:
        set_action(rig, c["action"])
        for i in range(c["frames"]):
            scene.frame_set(i)
            p, n = evaluated(obj)
            pos.append(p)
            nor.append(n)
    return np.stack(pos), np.stack(nor)


def write_vat(path, obj, pos, nor):
    """The level's .vat file (design section 3): header, triangles, regions and shades, frames."""
    n = len(obj.data.vertices)
    tris = triangles(obj)
    reg = np.empty(n, np.int32)
    obj.data.attributes["region"].data.foreach_get("value", reg)
    sh = np.empty(n, np.float32)
    obj.data.attributes["shade"].data.foreach_get("value", sh)
    lo = pos.reshape(-1, 3).min(axis=0) - 0.001
    hi = pos.reshape(-1, 3).max(axis=0) + 0.001
    qpos = np.round((pos - lo) / (hi - lo) * 65535.0).clip(0, 65535).astype(np.uint16)
    qn = oct_encode(nor.reshape(-1, 3)).reshape(pos.shape[0], n, 1)
    texels = np.concatenate([qpos, qn], axis=2).astype("<u2")
    vtx = np.zeros((n, 4), np.uint8)
    vtx[:, 0] = reg.astype(np.uint8)
    vtx[:, 1] = np.round(np.clip(sh, 0.0, 2.0) / 2.0 * 255.0).astype(np.uint8)
    head = VAT_MAGIC + struct.pack("<4I", n, len(tris), pos.shape[0], 0) + struct.pack("<6f", *lo, *hi)
    blob = head + tris.astype("<u4").tobytes() + vtx.tobytes() + texels.tobytes()
    with open(path, "wb") as f:
        f.write(blob)
    return {"vertices": n, "triangles": len(tris) // 3, "rows": int(pos.shape[0]),
            "bounds_min": [round(float(v), 5) for v in lo], "bounds_max": [round(float(v), 5) for v in hi],
            "bytes": len(blob), "sha256": hashlib.sha256(blob).hexdigest()}


def walk_speed(rig, action, frames, rate, scene):
    """The ground speed a moving clip walks at: the planted (lower) foot's backward speed, averaged."""
    set_action(rig, action)
    pts = []
    for i in range(frames + 1):
        scene.frame_set(i)
        pts.append([rig.matrix_world @ rig.pose.bones[b].head for b in ("ball_l", "ball_r")])
    v = []
    for i in range(frames):
        a, b = pts[i], pts[i + 1]
        k = 0 if a[0].z < a[1].z else 1
        v.append((b[k].y - a[k].y) * rate)  # facing -Y: a planted foot slides back, +Y
    return round(float(np.mean(v)), 4)


def pelvis_height(rig, action, scene):
    set_action(rig, action)
    scene.frame_set(0)
    return round(float((rig.matrix_world @ rig.pose.bones["pelvis"].head).z), 4)


# ----------------------------------------------------------------------------- one body, every file

def build_body(body, table, allow, ual_path, out_dir, verify):
    scene = bpy.context.scene
    scene.render.fps = 24
    near, rig, fitted = make_body(body, table, allow)
    lods = {}
    for lod in table["lods"]:
        obj = near if lod["ratio"] >= 1.0 else make_crowd_lod(near, lod["ratio"], f"{body['id']}_{lod['name']}")
        if lod["ratio"] >= 1.0:
            obj.name = obj.data.name = f"{body['id']}_{lod['name']}"
        lods[lod["name"]] = obj
    src, src_actions = import_ual(ual_path)
    retarget = Retarget(src, rig, table["bone_map"])
    order = retarget.order
    clips = []
    for c in table["clips"]:
        action, frames, rate = key_clip(rig, retarget, order, c, src_actions, table, scene)
        clips.append(dict(c, action=action, frames=frames, rate=rate))
    bpy.data.objects.remove(src, do_unlink=True)
    for a in list(bpy.data.actions):
        if a.name not in {c["name"] for c in table["clips"]}:
            bpy.data.actions.remove(a)
    files, levels = {}, []
    for lod in table["lods"]:
        obj = lods[lod["name"]]
        pos, nor = bake_level(obj, rig, clips, scene)
        name = f"{body['id']}_{lod['name']}.vat"
        tmp = os.path.join(out_dir, name + ".tmp")
        stats = write_vat(tmp, obj, pos, nor)
        if stats["triangles"] > lod["budget_triangles"]:
            os.remove(tmp)
            raise BuildError(f"{body['id']}: level {lod['name']} is over budget: {stats['triangles']} triangles, "
                             f"budget {lod['budget_triangles']}")
        files[name] = tmp
        levels.append(dict(stats, name=lod["name"], file=name, budget_triangles=lod["budget_triangles"]))
    # the glb: both levels skinned, every clip, with each vertex's region and shade as the custom attributes
    # `_REGION` and `_SHADE` (the review page colours the body from the palettes with them, as the game does)
    set_action(rig, None)
    for obj in lods.values():
        for src, dst, kind in (("region", "_REGION", 'INT'), ("shade", "_SHADE", 'FLOAT')):
            n = len(obj.data.vertices)
            arr = np.empty(n, np.int32 if kind == 'INT' else np.float32)
            obj.data.attributes[src].data.foreach_get("value", arr)
            a = obj.data.attributes.new(dst, 'FLOAT', 'POINT')
            a.data.foreach_set("value", arr.astype(np.float32))
    for o in bpy.data.objects:
        o.select_set(o in (rig, *lods.values()))
    bpy.context.view_layer.objects.active = rig
    for pb in rig.pose.bones:
        pb.matrix_basis = Matrix.Identity(4)
    glb_name = body["id"] + ".glb"
    tmp = os.path.join(out_dir, glb_name + ".tmp.glb")
    bpy.ops.export_scene.gltf(filepath=tmp, export_format='GLB', use_selection=True, export_yup=True,
                              export_skins=True, export_animations=True, export_animation_mode='ACTIONS',
                              export_morph=False, export_materials='NONE', export_attributes=True,
                              export_rest_position_armature=True, export_def_bones=False,
                              export_force_sampling=False, export_frame_range=False)
    files[glb_name] = tmp
    clip_rows = []
    row = 0
    for c in clips:
        info = {"name": c["name"], "row": row, "frames": c["frames"], "fps": round(c["rate"], 6), "loop": c["loop"]}
        if c.get("moves"):
            info["speed_m_s"] = walk_speed(rig, c["action"], c["frames"], c["rate"], scene)
        clip_rows.append(info)
        row += c["frames"]
    sit = next(c for c in clips if c["name"] == "sit")
    entry = {"id": body["id"], "sex": body["sex"], "height_m": round(fitted, 4), "glb": glb_name,
             "glb_sha256": hashlib.sha256(open(tmp, "rb").read()).hexdigest(),
             "sitting_pelvis_m": pelvis_height(rig, sit["action"], scene),
             "standing_pelvis_m": round(float((rig.matrix_world @ rig.data.bones["pelvis"].head_local).z), 4),
             "levels": levels}
    return entry, clip_rows, files


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    verify = "--verify" in argv
    try:
        ensure_private_blender()
        with open(TABLE, encoding="utf-8") as f:
            table = json.load(f)
        allow, ual_path = install_packs()
        out_dir = os.path.join(ROOT, table["out_dir"])
        os.makedirs(out_dir, exist_ok=True)
        bodies, clip_rows, files = [], None, {}
        for body in table["bodies"]:
            entry, rows, fs = build_body(body, table, allow, ual_path, out_dir, verify)
            if clip_rows is not None and [(r["name"], r["row"], r["frames"]) for r in rows] != \
                    [(r["name"], r["row"], r["frames"]) for r in clip_rows]:
                raise BuildError(f"{body['id']}: its clips' rows differ from the first body's")
            clip_rows = clip_rows or rows
            for r, mine in zip(clip_rows, rows):  # a moving clip's speed is each body's own
                if "speed_m_s" in mine:
                    r.setdefault("speed_m_s_by_body", {})[body["id"]] = mine["speed_m_s"]
                    r.pop("speed_m_s", None)
            bodies.append(entry)
            files.update(fs)
            print(f"[people] {body['id']}: " + ", ".join(
                f"{lv['name']} {lv['triangles']} tris {lv['vertices']} verts {lv['bytes'] // 1024} KB"
                for lv in entry["levels"]), flush=True)
        manifest = {"_comment": "Written by tools/blender/build_people.py from tools/blender/people.json (openspec/"
                                "changes/people-vat); never edited by hand. Rows are rows of each level's texture.",
                    "bake_fps": table["bake_fps"], "regions": list(REGIONS), "clips": clip_rows, "bodies": bodies}
        text = json.dumps(manifest, indent=1, sort_keys=True) + "\n"
        mpath = os.path.join(out_dir, "people_vat.json")
        if verify:
            bad = []
            for name, tmp in sorted(files.items()):
                with open(tmp, "rb") as a, open(os.path.join(out_dir, name), "rb") as b:
                    if a.read() != b.read():
                        bad.append(name)
                os.remove(tmp)
            if open(mpath, encoding="utf-8").read() != text:
                bad.append("people_vat.json")
            if bad:
                raise BuildError(f"the rebuild differs from the committed files: {', '.join(bad)}")
        else:
            for name, tmp in files.items():
                os.replace(tmp, os.path.join(out_dir, name))
            with open(mpath, "w", encoding="utf-8") as f:
                f.write(text)
    except (BuildError, deps.PackError) as e:
        print("FAIL", e, flush=True)
        sys.exit(1)
    except Exception:  # Blender would print the traceback and still exit 0
        traceback.print_exc()
        print("FAIL the build stopped on the error above", flush=True)
        sys.exit(1)
    print("OK people " + ("match their committed files" if verify else "written to " + table["out_dir"]), flush=True)


if __name__ == "__main__":
    main()
