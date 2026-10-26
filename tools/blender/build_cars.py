"""Low-poly block-outs of the seven traffic cars (openspec changes `car-traffic` and `car-driving`), built the CSG
way before any detail (owner, 2026-10-24: "I prefer you make low poly low textured verions before going full
detail", "use blender to your advantage, lots of CSG, lots of boolean meshes").

1980s civilian cars with no real brands (survey J7): sedan, wagon, pickup, van, taxi, bus and semi. Each is a
body box with its wheel arches cut by boolean cylinders, a greenhouse in glass, four wheels as their own nodes
(`WHEEL_FL`, `WHEEL_FR`, `WHEEL_RL`, `WHEEL_RR`) so they can turn, lamps that glow, and a block-out interior for
the driver: a dashboard, the steering wheel (`STEER_wheel`, turned by the game from the steering angle) and the
speedometer's needle (`NEEDLE_speed`, turned by the game from the car's simulated speed, CLAUDE.md 6.1). The
empties `EYE_driver` and `DOOR_driver` are the sockets the pilot uses.

Every size comes from the car's data, `data/cars/<kind>.ron` (size, wheelbase, track and the two sockets), with every
height (body, roof, wheels, ride height, the driver's eye) raised by `car_height_scale` in `data/world/traffic.ron` as
the game raises them on load, so the model and the simulation describe one car. Flat colours, no textures, no LODs: these are stand-ins until the
block-outs are approved and the detailed models with PBR sets and LODs replace them.

The car faces -z in glTF (Blender +y), its middle on the ground at the origin.

    /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/blender/build_cars.py

Writes assets/models/cars/<kind>.glb and assets/models/cars.json (triangles and bounds measured off the meshes).
"""
import math
import os
import re
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_kit as K  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "cars")
MANIFEST = os.path.join(ROOT, "assets", "models", "cars.json")
KINDS = ["sedan", "wagon", "pickup", "van", "taxi", "bus", "semi"]
# a triangle budget per block-out: the build refuses one over it (CLAUDE.md section 7)
BUDGET_TRIS = 4000


def ron(kind):
    """The numbers this model needs from data/cars/<kind>.ron."""
    text = open(os.path.join(ROOT, "data", "cars", f"{kind}.ron")).read()

    def tup(key):
        m = re.search(rf"\b{key}:\s*\(([^)]*)\)", text)
        return [float(x) for x in m.group(1).split(",")]

    def num(key):
        return float(re.search(rf"\b{key}:\s*([-0-9.]+)", text).group(1))

    paints = re.search(r"paints:\s*\[\(([^)]*)\)", text).group(1)
    # every height raised by the traffic's `car_height_scale`, as `CarSpec::raised` does on load (cb-core car.rs)
    k = height_scale()
    w, h, l = tup("size_m")
    ex, ey, ez = tup("eye_driver_m")
    return dict(size=[w, h * k, l], wheelbase=num("wheelbase_m"), track=num("track_m"), wheel=num("wheel_radius_m") * k,
                ride=num("ride_height_m") * k, eye=[ex, ey * k, ez], door=tup("door_driver_m"), k=k,
                paint=[float(x) for x in paints.split(",")], name=re.search(r'name:\s*"([^"]*)"', text).group(1))


def height_scale():
    """`car_height_scale` in data/world/traffic.ron (1 when it is not set): how much taller than its real-world size
    every car is drawn and simulated, its length and width kept."""
    text = open(os.path.join(ROOT, "data", "world", "traffic.ron")).read()
    m = re.search(r"\bcar_height_scale:\s*([-0-9.]+)", text)
    return float(m.group(1)) if m else 1.0


def g2b(x, y, z):
    """glTF (y up, -z ahead) to Blender (z up)."""
    return Vector((x, -z, y))


MATS = {}


def materials(paint):
    MATS.clear()
    MATS["PAINT"] = K._mat("PAINT", rgb=paint, rough=0.35, metal=0.3)
    MATS["GLASS"] = K._mat("GLASS", rgb=[0.03, 0.05, 0.07], rough=0.05, metal=0.0)
    MATS["TYRE"] = K._mat("TYRE", rgb=[0.02, 0.02, 0.02], rough=0.9)
    MATS["TRIM"] = K._mat("TRIM", rgb=[0.6, 0.6, 0.62], rough=0.25, metal=1.0)
    MATS["DASH"] = K._mat("DASH", rgb=[0.06, 0.06, 0.07], rough=0.7)
    MATS["DIAL"] = K._mat("DIAL", rgb=[0.85, 0.85, 0.8], rough=0.5, emit=0xd8f0ff, emit_k=0.4)
    MATS["NEEDLE"] = K._mat("NEEDLE", rgb=[0.9, 0.2, 0.05], rough=0.4, emit=0xff4010, emit_k=1.5)
    MATS["HEAD"] = K._mat("HEAD", rgb=[1.0, 0.95, 0.8], emit=0xfff2cc, emit_k=4.0)
    MATS["TAIL"] = K._mat("TAIL", rgb=[0.6, 0.02, 0.02], emit=0xff1a0a, emit_k=2.5)
    MATS["SIGN"] = K._mat("SIGN", rgb=[0.95, 0.85, 0.3], emit=0xffe080, emit_k=1.0)
    # single-sided, so from the driver's seat the body and the glass round the eye are not drawn and the road shows
    for m in MATS.values():
        m.use_backface_culling = True


def mesh(name, bm, mat, parent, at=None):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.materials.append(MATS[mat])
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    o.parent = parent
    if at is not None:
        o.location = at
    return o


def box(name, c, size, mat, parent):
    """A box centred at glTF point `c`, `size` (x, y, z) in glTF axes."""
    bm = K._box(size[0], size[2], size[1])
    bm.transform(Matrix.Translation(g2b(*c)))
    return mesh(name, bm, mat, parent)


def wheel_cylinder(name, c, r, w, mat, parent, seg=12):
    """A cylinder across the car (along x) at glTF point `c`, its origin at its middle."""
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r, depth=w,
                          matrix=Matrix.Rotation(math.pi / 2, 4, "Y"))
    return mesh(name, bm, mat, parent, at=g2b(*c))


def boolean(target, cutter):
    mod = target.modifiers.new("csg", "BOOLEAN")
    mod.operation, mod.solver, mod.object = "DIFFERENCE", "EXACT", cutter
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    bpy.data.objects.remove(cutter)


def empty(name, c, parent):
    o = bpy.data.objects.new(name, None)
    o.empty_display_type = "PLAIN_AXES"
    o.empty_display_size = 0.2
    bpy.context.collection.objects.link(o)
    o.parent = parent
    o.location = g2b(*c)
    return o


def build(kind, d):
    w, h, l = d["size"]
    wb, tr, ride = d["wheelbase"], d["track"], d["ride"]
    big = kind in ("bus", "semi")
    r = d["wheel"]
    root = empty("root", (0, 0, 0), None)
    floor = ride
    # the body: one box, wheel arches cut out by boolean cylinders
    body_top = {"pickup": 1.15 * d["k"], "van": h, "bus": h, "semi": h * 0.62}.get(kind, h * 0.58)
    if kind == "semi":
        cab_len = 2.4
        body = box("body", (0, (floor + body_top) / 2, -l / 2 + cab_len / 2), (w * 0.98, body_top - floor, cab_len), "PAINT", root)
        trailer = box("trailer", (0, (floor + 0.9 + h) / 2, l / 2 - (l - cab_len - 0.4) / 2), (w, h - floor - 0.9, l - cab_len - 0.4), "TRIM", root)
        trailer.data.materials[0] = MATS["PAINT"]
    else:
        body = box("body", (0, (floor + body_top) / 2, 0), (w, body_top - floor, l), "PAINT", root)
    axle_f, axle_r = -wb / 2, wb / 2
    if kind == "semi":
        axle_f, axle_r = -l / 2 + 1.3, -l / 2 + 1.3 + wb
    for z in (axle_f, axle_r):
        cut = wheel_cylinder("arch", (0, r, z), r + 0.08, w * 1.2, "PAINT", None)
        target = body if (kind != "semi" or z == axle_f) else trailer
        boolean(target, cut)
    # the greenhouse: glass over the body, set in from its sides
    if kind in ("sedan", "taxi", "wagon"):
        back = 0.9 if kind == "wagon" else 0.6
        front = -0.55
        box("cabin", (0, (body_top + h) / 2, (front + back) / 2 * l * 0.5), (w * 0.86, h - body_top, l * (back - front) * 0.5), "GLASS", root)
    elif kind == "pickup":
        box("cabin", (0, (body_top + h) / 2, -0.25), (w * 0.9, h - body_top, 1.7), "GLASS", root)
        # the bed: a box with its inside cut away
        bed = box("bed", (0, body_top + 0.25, l * 0.25), (w, 0.5, l * 0.45), "PAINT", root)
        boolean(bed, box("bed_cut", (0, body_top + 0.4, l * 0.25), (w * 0.85, 0.5, l * 0.42), "PAINT", None))
    elif kind == "van":
        box("windscreen", (0, h * 0.72, -l / 2 + 0.02), (w * 0.86, h * 0.36, 0.06), "GLASS", root)
        box("side_glass_l", (-w / 2 - 0.01, h * 0.72, -l / 2 + 0.9), (0.04, h * 0.32, 1.4), "GLASS", root)
    elif kind == "bus":
        box("windows_l", (-w / 2 - 0.01, h * 0.68, 0), (0.04, h * 0.32, l * 0.9), "GLASS", root)
        box("windows_r", (w / 2 + 0.01, h * 0.68, 0), (0.04, h * 0.32, l * 0.9), "GLASS", root)
        box("windscreen", (0, h * 0.62, -l / 2 - 0.01), (w * 0.9, h * 0.45, 0.04), "GLASS", root)
    elif kind == "semi":
        box("cab_top", (0, (body_top + h * 0.95) / 2, -l / 2 + 1.2), (w * 0.96, h * 0.95 - body_top, 2.4), "PAINT", root)
        box("windscreen", (0, body_top + 0.45, -l / 2 - 0.01), (w * 0.86, 0.7, 0.04), "GLASS", root)
    if kind == "taxi":
        box("sign", (0, h + 0.12, 0.1), (0.9, 0.24, 0.3), "SIGN", root)
    # wheels: their own nodes, so they can turn
    for name, x, z in (("WHEEL_FL", -tr / 2, axle_f), ("WHEEL_FR", tr / 2, axle_f), ("WHEEL_RL", -tr / 2, axle_r), ("WHEEL_RR", tr / 2, axle_r)):
        wheel_cylinder(name, (x, r, z), r, 0.24 if not big else 0.32, "TYRE", root)
    # lamps
    for sx in (-1, 1):
        box("headlamp", (sx * w * 0.36, floor + 0.35, -l / 2 - 0.02), (0.28, 0.16, 0.05), "HEAD", root)
        box("taillamp", (sx * w * 0.4, floor + 0.4, l / 2 + 0.02), (0.22, 0.14, 0.05), "TAIL", root)
    box("bumper_f", (0, floor + 0.1, -l / 2 - 0.05), (w * 1.02, 0.16, 0.1), "TRIM", root)
    box("bumper_r", (0, floor + 0.1, l / 2 + 0.05), (w * 1.02, 0.16, 0.1), "TRIM", root)
    # the driver's block-out interior: dashboard, wheel, speedometer; seen from EYE_driver
    ex, ey, ez = d["eye"]
    dash_z = ez - 0.75
    box("dashboard", (0, ey - 0.42, dash_z), (w * 0.9, 0.3, 0.35), "DASH", root)
    box("speedo_dial", (ex, ey - 0.3, dash_z + 0.18), (0.18, 0.18, 0.02), "DIAL", root)
    needle = box("NEEDLE_speed", (0, 0.04, 0), (0.012, 0.08, 0.01), "NEEDLE", root)
    needle.location = g2b(ex, ey - 0.3, dash_z + 0.195)
    steer = wheel_cylinder("STEER_wheel", (ex, ey - 0.38, ez - 0.42), 0.19, 0.03, "DASH", root, seg=16)
    steer.rotation_euler = (0, 0, math.pi / 2)  # its disc faces the driver
    empty("EYE_driver", (ex, ey, ez), root)
    empty("DOOR_driver", tuple(d["door"]), root)
    return root


def main():
    os.makedirs(OUT, exist_ok=True)
    if os.path.exists(MANIFEST):
        os.remove(MANIFEST)
    over = []
    for kind in KINDS:
        K.clear_scene()
        d = ron(kind)
        materials(d["paint"])
        build(kind, d)
        meta = dict(id=kind, name=d["name"], status="block-out stand-in, approved (survey W36)",
                    generator="tools/blender/build_cars.py", source=f"data/cars/{kind}.ron", size_m=d["size"],
                    height_scale=d["k"], budget_tris=BUDGET_TRIS)
        m = K.export(os.path.join(OUT, f"{kind}.glb"), MANIFEST, meta, {"car": kind})
        if m["triangles"] > BUDGET_TRIS:
            over.append(f"{kind}: {m['triangles']} triangles over the {BUDGET_TRIS} budget")
    if over:
        raise SystemExit("; ".join(over))


main()
