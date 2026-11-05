"""The military base's aircraft and the civil airports' helicopter (openspec change `military-base`): a twin-engine,
twin-tail fighter, a high-wing four-turboprop transport with a rear ramp, a utility gunship with stub wings and a
light twin-engine civil helicopter, modelled the CSG way in Blender on the airliners' kit (owner, 2026-10-30: "Work on
military base mockup. Vehicles: helicopter, cargo plane, fighter jets. Facility: dealers choice, but we need same
fidelity as civilian airport"). Original 1980s industrial designs: no real type's shape, name or markings.

Each file holds, per level of detail (`<part>.L<n>`: 0 within 250 m with the interior, 1 to 1.5 km, 2 beyond):
- the airframe cut into damage zones (`ZONE_<name>`), each its own closed meshes, the joints capped `torn`;
- moving parts on `RIG_*` pivots (gear legs, canopy, ramp, rotors, propellers, control surfaces), their travel in
  the manifest's `rig`;
- the lights (`LAMP_*`), the pilot's eye (`EYE_pilot`) and the doors a pilot can use (`DOOR_*`), for stealing;
- at level 0, the cockpit (seats, panel, sticks) and the hold or cabin, seen through the glass or the open ramp.

The manifest (`<kind>.json`) is written in the airliners' shape, so the page and the game read one format.

Frame: glTF metres, +Y up, the nose toward -Z, +X the right wing, the origin on the fuselage centre line half way
along the airframe; the ground under the gear (or skids) is at y = -rest_m.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_military_aircraft.py [-- fighter ...] [--shots DIR]
"""
import json
import math
import os
import sys

import bpy  # noqa: I001 (bpy before bmesh, as the pip module needs)
import bmesh
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_kit as K  # noqa: E402
import aircraft_kit as A  # noqa: E402
from aircraft_kit import (CSG, Model, box_into, boolean, capsules_of, clean, cyl_into, empty, g2b, lathe, lin,  # noqa: E402
                          lod_of, loft, mesh, meshes_under, move_to_pivot, to_gltf, tris, world_verts)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "aircraft")
K.TEXDIR = os.path.join(ROOT, "assets", "textures", "mech")
CABIN_TEX = os.path.join(ROOT, "assets", "textures", "aircraft")
# Neighbouring zone pieces stop this far apart, so no two caps share a plane (CLAUDE.md section 10: no z-fighting).
GAP = 0.004
LEVELS = (0, 1, 2)
DETAIL = {
    0: dict(round=40, foil=12, wheel=18, blade=1, interior=True, cut=True, lights=True),
    1: dict(round=16, foil=9, wheel=10, blade=1, interior=False, cut=True, lights=True),
    2: dict(round=8, foil=4, wheel=6, blade=0, interior=False, cut=False, lights=False),
}

ORDER = ["top", "belly", "trim", "metal", "dark", "rubber", "glass", "torn", "panel", "seat", "floor", "hazard",
         "nav_red", "nav_green", "nav_white", "beacon", "strobe", "landing", "formation"]

# Sizes (design section 2), liveries and budgets. Liveries: the top colour, the belly colour, the trim (fin tips,
# bands), each linear after `lin`.
KINDS = {
    "fighter": dict(length=19.4, span=13.0, height=5.6, rest=1.95, hp=90.0,
                    livery=dict(top=(0.36, 0.39, 0.42), belly=(0.62, 0.65, 0.68), trim=(0.55, 0.16, 0.1)),
                    budget={0: 18000, 1: 5000, 2: 1500}),
    "military_cargo": dict(length=29.8, span=40.4, height=11.7, rest=3.25, hp=220.0,
                           livery=dict(top=(0.24, 0.29, 0.22), belly=(0.4, 0.44, 0.38), trim=(0.55, 0.16, 0.1)),
                           budget={0: 22000, 1: 6000, 2: 2500}),
    "military_helicopter": dict(length=17.6, span=16.4, height=4.4, rest=1.55, hp=70.0,
                                livery=dict(top=(0.26, 0.3, 0.22), belly=(0.35, 0.38, 0.3), trim=(0.55, 0.16, 0.1)),
                                budget={0: 15000, 1: 4000, 2: 1200}),
    "civil_helicopter": dict(length=13.0, span=11.0, height=3.4, rest=1.15, hp=45.0,
                             livery=dict(top=(0.93, 0.93, 0.91), belly=(0.86, 0.38, 0.08), trim=(0.1, 0.12, 0.16)),
                             budget={0: 10000, 1: 3000, 2: 900}),
}

# Health shares of the kind's hp, by zone (each kind's sum to 1).
SHARES = {
    "fighter": dict(centre=0.30, nose=0.12, tail=0.16, wing_inner_l=0.14, wing_inner_r=0.14, engine_1=0.07,
                    engine_2=0.07),
    "military_cargo": dict(centre=0.26, nose=0.10, tail=0.14, wing_inner_l=0.10, wing_inner_r=0.10, wing_outer_l=0.06,
                           wing_outer_r=0.06, engine_1=0.045, engine_2=0.045, engine_3=0.045, engine_4=0.045),
    "military_helicopter": dict(centre=0.30, nose=0.12, tail=0.14, rotor=0.14, tail_rotor=0.08, engine_1=0.10,
                                wing_inner_l=0.06, wing_inner_r=0.06),
    "civil_helicopter": dict(centre=0.34, nose=0.14, tail=0.16, rotor=0.16, tail_rotor=0.08, engine_1=0.12),
}


def materials(kind, k):
    """The kind's materials: every custom surface a full PBR set (CLAUDE.md section 10)."""
    A.MATS.clear()
    A.ORDER[:] = ORDER
    lv = k["livery"]
    m = A.MATS
    m["top"] = K._mat(f"{kind}_top", rgb=lin(lv["top"]), tex="plate", normal="plate_n", orm="plate_orm")
    m["belly"] = K._mat(f"{kind}_belly", rgb=lin(lv["belly"]), tex="plateB", normal="plateB_n", orm="plateB_orm")
    m["trim"] = K._mat(f"{kind}_trim", rgb=lin(lv["trim"]), tex="plate", normal="plate_n", orm="plate_orm")
    m["metal"] = K._mat(f"{kind}_metal", rgb=[0.5, 0.51, 0.53], tex="frameM", normal="frameM_n", orm="frameM_orm")
    m["dark"] = K._mat(f"{kind}_dark", rgb=[0.04, 0.04, 0.045], tex="frameM", normal="frameM_n", orm="frameM_orm")
    m["rubber"] = _cabin_mat(f"{kind}_rubber", "rubber", (0.06, 0.06, 0.06))
    m["glass"] = K._mat(f"{kind}_glass", rgb=[0.03, 0.045, 0.06], rough=0.05, metal=0.1, alpha=0.4)
    m["torn"] = K._mat(f"{kind}_torn", rgb=[0.09, 0.08, 0.07], tex="wear", normal="wear_n", orm="wear_orm")
    m["panel"] = _cabin_mat(f"{kind}_panel", "paint", (0.14, 0.15, 0.16))
    m["seat"] = _cabin_mat(f"{kind}_seat", "fabric", (0.2, 0.22, 0.18) if kind != "civil_helicopter" else (0.5, 0.44, 0.36))
    m["floor"] = _cabin_mat(f"{kind}_floor", "trim", (0.3, 0.31, 0.3))
    m["hazard"] = K._mat(f"{kind}_hazard", rgb=lin((0.85, 0.62, 0.08)), tex="plateB", normal="plateB_n", orm="plateB_orm")
    m["nav_red"] = K._mat(f"{kind}_nav_red", rgb=[0.6, 0.02, 0.02], emit=0xff2010, emit_k=6.0)
    m["nav_green"] = K._mat(f"{kind}_nav_green", rgb=[0.02, 0.5, 0.1], emit=0x20ff50, emit_k=6.0)
    m["nav_white"] = K._mat(f"{kind}_nav_white", rgb=[0.9, 0.9, 0.9], emit=0xffffff, emit_k=6.0)
    m["beacon"] = K._mat(f"{kind}_beacon", rgb=[0.6, 0.02, 0.02], emit=0xff1808, emit_k=8.0)
    m["strobe"] = K._mat(f"{kind}_strobe", rgb=[0.9, 0.9, 0.9], emit=0xffffff, emit_k=8.0)
    m["landing"] = K._mat(f"{kind}_landing", rgb=[0.9, 0.9, 0.85], emit=0xfff4e0, emit_k=5.0)
    m["formation"] = K._mat(f"{kind}_formation", rgb=[0.3, 0.6, 0.3], emit=0x9cff6a, emit_k=2.5)
    for mat in m.values():
        mat.use_backface_culling = True
        b = mat.node_tree.nodes["Principled BSDF"]
        mix = [n for n in mat.node_tree.nodes if n.bl_idname == "ShaderNodeMix"]
        col = mix[0].inputs[7].default_value if mix else b.inputs["Base Color"].default_value
        mat.diffuse_color = (col[0], col[1], col[2], 1.0)


def _cabin_mat(name, key, rgb):
    """A cabin surface: one of the furniture swatch sets (tools/textures/crop_furniture_swatches.py), tinted."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    b = nt.nodes["Principled BSDF"]

    def img(suffix):
        im = bpy.data.images.get(f"cabin_{key}{suffix}") or bpy.data.images.load(os.path.join(CABIN_TEX, f"{key}{suffix}.png"))
        im.name = f"cabin_{key}{suffix}"
        return im

    it = nt.nodes.new("ShaderNodeTexImage")
    it.image = img("")
    mix = nt.nodes.new("ShaderNodeMix")
    mix.data_type, mix.blend_type = "RGBA", "MULTIPLY"
    mix.inputs["Factor"].default_value = 1.0
    nt.links.new(it.outputs["Color"], mix.inputs[6])
    mix.inputs[7].default_value = list(lin(rgb)) + [1.0]
    nt.links.new(mix.outputs[2], b.inputs["Base Color"])
    K.PBR.normal(nt, b, img("_n"))
    K.PBR.orm(nt, b, img("_orm"))
    return mat


# --------------------------------------------------------------------------------------------------------- geometry

def station(st, z):
    """(half width, half height, centre height) of a body at z, between its listed stations (z, hw, hh, cy)."""
    return A._station(st, z)


def body_piece(name, st, za, zb, n, mat, parent, k=2.3, caps=(True, True)):
    """The part of a lofted body between za and zb, closed at both ends; an end that meets a neighbour (caps) takes
    the `torn` material, so a break shows the inside."""
    zs = [za] + [s[0] for s in st if za + 1e-4 < s[0] < zb - 1e-4] + [zb]
    rings = [A.section(z, *station(st, z), n, k) for z in zs]
    bm = loft(rings)
    bm.normal_update()
    torn = ORDER.index("torn")
    keep = ORDER.index(mat)
    for f in bm.faces:
        f.material_index = keep
    for f in bm.faces:
        if len(f.verts) == 3:
            zc = -sum(v.co.y for v in f.verts) / 3.0  # glTF z from Blender y
            if (caps[0] and abs(zc - za) < 1e-3) or (caps[1] and abs(zc - zb) < 1e-3):
                if abs(f.normal.y) > 0.99:
                    f.material_index = torn
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for key in ORDER:
        me.materials.append(A.MATS[key])
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    o.parent = parent
    return o


def hollow(piece, st, za, zb, wall, n, parent, k=2.3):
    """Hollow a body piece: everything a wall's thickness in from the skin between za and zb is cut away."""
    inner = [(z, hw - wall, hh - wall, cy) for z, hw, hh, cy in st if hw > wall * 2 and hh > wall * 2]
    core = body_piece("cut_core", inner, za, zb, n, "floor", parent, k)
    boolean(piece, [core], "DIFFERENCE", "hollow", hollowing=True)


def panes(name, st, za, zb, n, boxes, wall, parent, k=2.3):
    """Glass in the middle of a hollow wall, where the window cutters `boxes` (centre, half) made holes: a thin shell
    from 0.35 to 0.65 of the wall in from the skin, kept only inside the boxes."""
    mid = lambda f: [(z, hw - wall * f, hh - wall * f, cy) for z, hw, hh, cy in st if hw > wall * 2]  # noqa: E731
    o = body_piece(name, mid(0.35), za, zb, n, "glass", parent, k, caps=(False, False))
    boolean(o, [body_piece("cut_pane_core", mid(0.65), za - 0.01, zb + 0.01, n, "glass", parent, k)], "DIFFERENCE",
            "pane shell", hollowing=True)
    boolean(o, [boxes_obj("cut_pane_boxes", boxes, "glass", parent)], "INTERSECT", "panes")
    clean(o)
    return o


def paint_belly(o, below_y, mat="belly"):
    """Faces whose middle is under `below_y` (glTF) and face down take the belly colour (counter-shading)."""
    i, top = ORDER.index(mat), ORDER.index("top")
    for p in o.data.polygons:
        if p.material_index == top and p.center.z < below_y and p.normal.z < 0.2:
            p.material_index = i


def foil_loft(secs, n, mat, parent, name, side=1.0, vertical=False, xf=None, cap_torn=False):
    """A lifting surface through `secs` (span station, leading edge z, chord, height, thickness share): the kit's
    NACA rings, optionally mapped by `xf` (a function of a glTF point), lofted and closed. With `cap_torn` the root
    end takes the torn material."""
    rings = []
    for s in secs:
        r = A.airfoil(s[0], s[1], s[2], s[3], s[4], n, vertical, side)
        rings.append([xf(q) if xf else q for q in r])
    bm = loft(rings)
    o = A.obj(name, bm, mat, parent)
    if cap_torn:
        root_x = rings[0][0][1] if vertical else rings[0][0][0]
        for p in o.data.polygons:
            c = p.center
            v = c.z if vertical else c.x
            if abs(v - root_x) < 1e-3 and (abs(p.normal.z) > 0.95 if vertical else abs(p.normal.x) > 0.95):
                p.material_index = ORDER.index("torn")
    return o


def planform(x, root_x, tip_x, root_le, root_c, tip_c, sweep_deg):
    """Leading edge and chord at span x of a straight-tapered planform."""
    t = (x - root_x) / max(tip_x - root_x, 1e-6)
    le = root_le + (x - root_x) * math.tan(math.radians(sweep_deg))
    return le, root_c + (tip_c - root_c) * t


def wing_with_surface(m, base, zone, side, xs, plan, y_of, t, n, lod, mat, surf=None, cap_root=False):
    """A wing panel from span xs[0] to xs[-1] (positive, mirrored by `side`), with an optional moving trailing
    surface `surf` = (role, x_a, x_b, hinge share, travel dict). The panel in front of the surface is cut back to the
    hinge; the surface hangs on a `RIG_` pivot along the hinge line, a GAP clear of the panel at both ends."""
    s = "l" if side < 0 else "r"
    pieces = []
    if surf is None:
        secs = [(x, *plan(x)[:1], plan(x)[1], y_of(x), t) for x in xs]
        pieces.append(foil_loft(secs, n, mat, m.zones[zone], f"{base}_{s}.L{lod}", side, cap_torn=cap_root))
        return pieces
    role, xa, xb, h, travel = surf
    inner = [x for x in xs if x < xa - 1e-6] + [xa - GAP]
    outer = [xb + GAP] + [x for x in xs if x > xb + 1e-6]
    sec = lambda x, share=1.0, off=0.0: (x, plan(x)[0] + off, plan(x)[1] * share, y_of(x), t)  # noqa: E731
    if len(inner) > 1:
        pieces.append(foil_loft([sec(x) for x in inner], n, mat, m.zones[zone], f"{base}_in_{s}.L{lod}", side,
                                cap_torn=cap_root))
    pieces.append(foil_loft([sec(x, h) for x in (xa, xb)], n, mat, m.zones[zone], f"{base}_fwd_{s}.L{lod}", side))
    if len(outer) > 1 and outer[-1] > outer[0]:
        pieces.append(foil_loft([sec(x) for x in outer], n, mat, m.zones[zone], f"{base}_out_{s}.L{lod}", side))
    # the moving surface, behind the hinge
    ha = (side * (xa + GAP), y_of(xa + GAP), plan(xa + GAP)[0] + plan(xa + GAP)[1] * h + GAP)
    hb = (side * (xb - GAP), y_of(xb - GAP), plan(xb - GAP)[0] + plan(xb - GAP)[1] * h + GAP)
    at = tuple((a + b) * 0.5 for a, b in zip(ha, hb))
    name = f"{role}_{s}"
    if lod == 0:
        m.pivot(name, at, zone, [b - a for a, b in zip(ha, hb)], role=role, **travel)
    node = bpy.data.objects[f"RIG_{name}"]
    rear = [(x, plan(x)[0] + plan(x)[1] * h + GAP, plan(x)[1] * (1.0 - h) - GAP, y_of(x), t * 0.55) for x in
            (xa + GAP, xb - GAP)]
    o = foil_loft(rear, n, mat, node, f"{name}.L{lod}", side)
    move_to_pivot(o, at)
    pieces.append(o)
    return pieces


def wheel(m, name, node, at, c, r, w, n, lod):
    """A tyre and hub at glTF `c`, axle across x, under pivot `node` at `at`."""
    tyre = lathe([(-w * 0.5, r * 0.7), (-w * 0.5, r * 0.95), (-w * 0.35, r), (w * 0.35, r), (w * 0.5, r * 0.95),
                  (w * 0.5, r * 0.7)], n, c, axis="x")
    o = A.obj(f"{name}_tyre.L{lod}", tyre, "rubber", node)
    move_to_pivot(o, at)
    hub = lathe([(-w * 0.52, 0.01), (-w * 0.52, r * 0.62), (w * 0.52, r * 0.62), (w * 0.52, 0.01)], max(n // 2, 6), c,
                axis="x")
    o2 = A.obj(f"{name}_hub.L{lod}", hub, "metal", node)
    move_to_pivot(o2, at)
    return [o, o2]


def strut(name, a, b, r, n, mat, node, at):
    bm = bmesh.new()
    cyl_into(bm, a, b, r, n)
    o = mesh(name, bm, mat, node, at=at)
    return o


def leg(m, lod, name, zone, top, ground_y, wheels, r, w, retract, n, legr=0.09, keys=None):
    """A gear leg: a pivot at `top` (glTF), a strut down to the axle `r` over `ground_y`, its wheels at x offsets
    `wheels` from the strut; `retract` = (axis, angle_deg). Level 0 records the pivot."""
    axle = (top[0], ground_y + r, top[2])
    nm = f"gear_{name}"
    if lod == 0:
        m.pivot(nm, top, zone, retract[0], role="gear_leg", angle_deg=retract[1], seq="gear",
                keys=keys or [[0.0, 0.0], [0.15, 0.0], [0.85, 1.0], [1.0, 1.0]])
    node = bpy.data.objects[f"RIG_{nm}"]
    out = [strut(f"{nm}_strut.L{lod}", top, axle, legr, max(n // 2, 6), "metal", node, top)]
    for dx in wheels:
        out += wheel(m, f"{nm}_{'ab'[wheels.index(dx)]}", node, top, (axle[0] + dx, axle[1], axle[2]), r, w, n, lod)
    return out


def blades(name, centre, axis_up, count, radius, chord, root, node, lod, mat="dark", droop=0.0, twist_deg=8.0,
           along=None):
    """`count` blades from `root` out to `radius` round `centre`, in the plane normal to `axis_up` (a glTF unit
    vector); each blade a thin box pitched `twist_deg`. A hub is the caller's."""
    bm = bmesh.new()
    up = Vector(axis_up).normalized()
    a0 = Vector(along) if along else (Vector((1, 0, 0)) if abs(up.x) < 0.9 else Vector((0, 0, 1)))
    a0 = (a0 - up * a0.dot(up)).normalized()
    for i in range(count):
        ang = math.tau * i / count
        d = (Matrix.Rotation(ang, 3, up) @ a0).normalized()
        c = d.cross(up).normalized()
        tw = math.radians(twist_deg)
        cc = (c * math.cos(tw) + up * math.sin(tw)).normalized()
        th = d.cross(cc).normalized()
        mid = Vector(centre) + d * ((root + radius) * 0.5) - up * droop
        box_into(bm, tuple(mid), ((radius - root) * 0.5, chord * 0.5, max(chord * 0.06, 0.03)), axes=(tuple(d), tuple(cc), tuple(th)))
    return mesh(f"{name}.L{lod}", bm, mat, node, at=centre)


def hub_lathe(name, centre, r, length, n, mat, node, axis="y"):
    """A short drum (a rotor hub, a spinner) round glTF `centre`, along y or z."""
    if axis == "z":
        prof = [(-length * 0.5, 0.01), (-length * 0.5, r), (length * 0.35, r * 0.85), (length * 0.5, 0.01)]
        bm = lathe(prof, n, centre, axis="z")
    else:
        bm = bmesh.new()
        cyl_into(bm, (centre[0], centre[1] - length * 0.5, centre[2]), (centre[0], centre[1] + length * 0.5, centre[2]),
                 r, n)
    return mesh(f"{name}", bm, mat, node, at=centre)


def light(m, lod, name, at, zone, mat, colour, pattern, r=0.1):
    if not DETAIL[lod]["lights"]:
        return
    clean(A.light(f"light_{name}.L{lod}", at, mat, m.zones[zone], r=r))
    if lod == 0:
        m.lamp(name, at, zone, colour, pattern)


def boxes_obj(name, boxes, mat, parent):
    """One object of many glTF boxes (centre, half)."""
    bm = bmesh.new()
    for c, h in boxes:
        box_into(bm, c, h)
    return mesh(name, bm, mat, parent)


def seat(boxes_seat, boxes_frame, x, z, floor, w=0.5, back=1.0, facing=-1.0):
    """A seat facing `facing` (-1: toward the nose) at (x, z) on `floor`: cushion, back and a frame under it."""
    boxes_seat.append(((x, floor + 0.45, z), (w * 0.45, 0.07, 0.24)))
    boxes_seat.append(((x, floor + 0.45 + back * 0.4, z - facing * 0.24), (w * 0.45, back * 0.36, 0.06)))
    boxes_frame.append(((x, floor + 0.2, z), (w * 0.35, 0.19, 0.18)))


# ------------------------------------------------------------------------------------------------------- the fighter
#
# A 1980s air superiority fighter of the island's own make: a long nose radome, a bubble canopy over one seat, two
# boxy side intakes, a clipped delta wing with leading-edge extensions, twin fins canted out 15 degrees, all-moving
# tailplanes and two afterburning engines side by side.

FIGHTER_ST = [(-9.7, 0.02, 0.02, 0.05), (-9.2, 0.22, 0.22, 0.05), (-8.2, 0.42, 0.42, 0.08), (-7.0, 0.55, 0.58, 0.12),
              (-5.6, 0.62, 0.72, 0.18), (-4.0, 0.72, 0.78, 0.2), (-2.6, 1.05, 0.78, 0.15), (-1.0, 1.62, 0.74, 0.08),
              (1.5, 1.85, 0.72, 0.05), (5.0, 1.82, 0.7, 0.05), (7.6, 1.6, 0.62, 0.05), (8.6, 1.4, 0.55, 0.05),
              (9.0, 1.35, 0.52, 0.05)]


def build_fighter(m, lod):
    det = DETAIL[lod]
    n, nf = det["round"], det["foil"]
    st = FIGHTER_ST
    gy = -m.kind_rest  # the ground
    # the fuselage in three zones
    nose = body_piece(f"fus_nose.L{lod}", st, -9.7, -3.0 - GAP, n, "top", m.zones["nose"], caps=(False, True))
    mid = body_piece(f"fus_mid.L{lod}", st, -3.0 + GAP, 4.6 - GAP, n, "top", m.zones["centre"])
    aft = body_piece(f"fus_aft.L{lod}", st, 4.6 + GAP, 9.0, n, "top", m.zones["tail"], caps=(True, False))
    if det["cut"]:
        # the cockpit tub under the canopy (level 0 only: its seat is the interior)
        if det["interior"]:
            tub = boxes_obj(f"cut_tub_{lod}", [((0.0, 0.62, -5.0), (0.46, 0.5, 1.15))], "dark", m.zones["nose"])
            boolean(nose, [tub], "DIFFERENCE", "cockpit tub")
            tub2 = boxes_obj(f"cut_tub2_{lod}", [((0.0, 0.62, -3.2), (0.46, 0.5, 0.35))], "dark", m.zones["centre"])
            boolean(mid, [tub2], "DIFFERENCE", "cockpit tub")
    # intakes: boxy ducts down each side, their mouths cut dark
    for side, s in ((-1, "l"), (1, "r")):
        x0 = side * 1.02
        pts = []
        for z, hw, top, bot in ((-3.9, 0.42, 0.25, -0.75), (-2.6, 0.44, 0.3, -0.72), (-0.6, 0.5, 0.28, -0.62),
                                (1.2, 0.4, 0.15, -0.5)):
            for xx in (x0 - side * 0.3, x0 + side * hw * 1.6):
                for yy in (top, bot):
                    pts.append((xx, yy, z))
        duct = A.obj(f"intake_{s}.L{lod}", A.hull(pts), "top", m.zones["centre"])
        if det["cut"]:
            mouth = boxes_obj(f"cut_mouth_{s}_{lod}", [((x0 + side * 0.36, -0.24, -3.9), (0.36, 0.42, 0.6))], "dark",
                              m.zones["centre"])
            boolean(duct, [mouth], "DIFFERENCE", "intake mouth")
        clean(duct)
        paint_belly(duct, -0.1)
    for o in (nose, mid, aft):
        clean(o)
        paint_belly(o, -0.15)
    # the canopy, hinged at its back
    hinge = (0.0, 0.92, -2.85)
    if lod == 0:
        m.pivot("canopy", hinge, "nose", (1.0, 0.0, 0.0), role="canopy", angle_deg=48.0, seq="canopy",
                keys=[[0.0, 0.0], [1.0, 1.0]])
    cnode = bpy.data.objects["RIG_canopy"]
    rings = []
    for z, hw, hh in ((-6.6, 0.05, 0.03), (-6.2, 0.4, 0.4), (-5.4, 0.52, 0.66), (-4.2, 0.54, 0.72), (-3.3, 0.5, 0.56),
                      (-2.9, 0.44, 0.4)):
        base = station(st, z)
        sill = base[2] + base[1] * 0.92
        ring = []
        k = max(n // 2, 6)
        for i in range(k + 1):
            a = math.pi * i / k
            ring.append((math.cos(a) * hw, sill + math.sin(a) * hh, z))
        rings.append(ring)
    can = A.obj(f"canopy.L{lod}", loft(rings), "glass", cnode)
    move_to_pivot(can, hinge)
    frame = boxes_obj(f"canopy_frame.L{lod}", [((0.0, 0.94, -2.95), (0.4, 0.05, 0.06)), ((0.0, 1.3, -4.6), (0.03, 0.05, 0.06))],
                      "metal", cnode)
    move_to_pivot(frame, hinge)
    # the wing: leading-edge extension blended to a 45 degree clipped delta, a flaperon on each side
    root_x, tip_x = 1.4, 6.2
    plan = lambda x: planform(x, root_x, tip_x, -1.4, 6.6, 1.55, 42.0)  # noqa: E731
    y_of = lambda x: 0.05 - (x - root_x) * 0.03  # noqa: E731
    for side, s in ((-1, "l"), (1, "r")):
        zone = f"wing_inner_{s}"
        surf = ("aileron", 3.1, 6.0, 0.72, dict(range_deg=20.0))
        for o in wing_with_surface(m, "wing", zone, side, [root_x, 3.1, 6.0, tip_x], plan, y_of, 0.045, nf, lod, "top",
                                   surf, cap_root=True):
            clean(o)
        lerx = [(side * 0.9, 0.2, -6.0), (side * 0.95, 0.12, -6.0), (side * 1.6, 0.12, -1.6), (side * 1.6, 0.0, -1.6),
                (side * 0.9, 0.0, -2.0), (side * 1.4, 0.22, -1.0), (side * 1.4, -0.04, -1.0)]
        clean(A.obj(f"lerx_{s}.L{lod}", A.hull(lerx), "top", m.zones["centre"]))
        # tip rail with a missile
        tipz = plan(tip_x)[0]
        clean(boxes_obj(f"tip_rail_{s}.L{lod}", [((side * (tip_x + 0.06), y_of(tip_x), tipz + 0.75), (0.05, 0.05, 0.55))],
                        "metal", m.zones[zone]))
        if lod < 2:
            msl(m, lod, f"aam_tip_{s}", (side * (tip_x + 0.13), y_of(tip_x) - 0.05, tipz + 0.6), 0.075, 2.9, zone)
            msl(m, lod, f"aam_mid_{s}", (side * 4.0, y_of(4.0) - 0.5, plan(4.0)[0] + 1.2), 0.1, 3.6, zone, pylon=0.36)
    # twin fins canted out, rudders
    for side, s in ((-1, "l"), (1, "r")):
        base = (side * 1.15, 0.62, 0.0)
        cant = math.radians(15.0) * side

        def xf(q, b=base, c=cant):
            x, y, z = q[0], q[1] - b[1], q[2]
            return (b[0] + x * math.cos(c) + y * math.sin(c), b[1] + y * math.cos(c) - x * math.sin(c), z)
        # vertical: (thickness x, height y, z); fin from y 0 to 3.25 over its root
        fin_plan = lambda h: (4.3 + h * math.tan(math.radians(40.0)), 3.4 - h * 0.62)  # noqa: E731
        lo_h, hi_h = 0.0, 3.25
        secs = [(h, fin_plan(h)[0], fin_plan(h)[1] * (0.74 if 0.5 < h < 2.9 else 1.0), 0.0, 0.05) for h in (lo_h, 0.5, 2.9, hi_h)]
        rings = [[xf((p[0], p[1] + base[1], p[2])) for p in A.airfoil(sc[0], sc[1], sc[2], sc[3], sc[4], nf, True, 1.0)] for sc in secs]
        clean(A.obj(f"fin_{s}.L{lod}", loft(rings), "top", m.zones["tail"]))
        ra, rb = xf((0.0, base[1] + 0.5 + GAP, fin_plan(0.5)[0] + fin_plan(0.5)[1] * 0.74)), \
            xf((0.0, base[1] + 2.9 - GAP, fin_plan(2.9)[0] + fin_plan(2.9)[1] * 0.74))
        at = tuple((a + b) * 0.5 for a, b in zip(ra, rb))
        if lod == 0:
            m.pivot(f"rudder_{s}", at, "tail", [b - a for a, b in zip(ra, rb)], role="rudder", range_deg=25.0)
        rnode = bpy.data.objects[f"RIG_rudder_{s}"]
        rsecs = [(h, fin_plan(h)[0] + fin_plan(h)[1] * 0.74 + GAP, fin_plan(h)[1] * 0.26 - GAP, 0.0, 0.03)
                 for h in (0.5 + GAP, 2.9 - GAP)]
        rr = [[xf((p[0], p[1] + base[1], p[2])) for p in A.airfoil(sc[0], sc[1], sc[2], sc[3], sc[4], nf, True, 1.0)] for sc in rsecs]
        ro = A.obj(f"rudder_{s}.L{lod}", loft(rr), "trim", rnode)
        move_to_pivot(ro, at)
        clean(ro)
    # all-moving tailplanes on their pivots
    for side, s in ((-1, "l"), (1, "r")):
        at = (side * 1.85, 0.0, 6.9)
        if lod == 0:
            m.pivot(f"stab_{s}", at, "tail", (1.0, 0.0, 0.0), role="elevator", range_deg=18.0)
        snode = bpy.data.objects[f"RIG_stab_{s}"]
        sp = lambda x: planform(x, 1.75, 5.2, 5.6, 2.9, 1.0, 40.0)  # noqa: E731
        secs = [(x, sp(x)[0], sp(x)[1], -0.02, 0.04) for x in (1.75, 5.2)]
        so = foil_loft(secs, nf, "top", snode, f"stab_{s}.L{lod}", side)
        move_to_pivot(so, at)
        clean(so)
    # engines: afterburner nozzles in their own zones (losing one leaves the other)
    for i, side in enumerate((-1, 1)):
        zone = f"engine_{i + 1}"
        c = (side * 0.72, 0.0, 9.0)
        prof = [(-0.6, 0.6), (0.0, 0.58), (0.9, 0.5), (1.15, 0.48)]
        noz = A.obj(f"nozzle_{i + 1}.L{lod}", lathe(prof, n, c, axis="z"), "metal", m.zones[zone])
        if det["cut"]:
            cut = bmesh.new()
            cyl_into(cut, (c[0], c[1], c[2] + 0.5), (c[0], c[1], c[2] + 1.4), 0.4, n)
            boolean(noz, [A.obj(f"cut_noz_{i}_{lod}", cut, "dark", m.zones[zone])], "DIFFERENCE", "exhaust")
        clean(noz)
        if lod == 0:
            empty(f"FX_afterburner_{i + 1}", (c[0], c[1], c[2] + 1.15), m.zones[zone])
    # gear: nose leg folds forward, mains fold forward into the intake ducts
    leg(m, lod, "nose", "nose", (0.0, -0.45, -6.2), gy, [0.0], 0.3, 0.2, ((1.0, 0.0, 0.0), -100.0), det["wheel"])
    for side, s in ((-1, "l"), (1, "r")):
        leg(m, lod, f"main_{s}", "centre", (side * 1.25, -0.55, 1.4), gy, [side * 0.18], 0.4, 0.26,
            ((1.0, 0.0, 0.0), -95.0), det["wheel"], legr=0.11)
    # lights
    light(m, lod, "nav_left", (-6.15, y_of(6.1) + 0.05, plan(6.1)[0] + 0.3), "wing_inner_l", "nav_red", "red", "steady", 0.08)
    light(m, lod, "nav_right", (6.15, y_of(6.1) + 0.05, plan(6.1)[0] + 0.3), "wing_inner_r", "nav_green", "green", "steady", 0.08)
    light(m, lod, "nav_tail", (0.0, 0.05, 9.25), "tail", "nav_white", "white", "steady", 0.07)
    light(m, lod, "beacon_bottom", (0.0, -0.78, 0.5), "centre", "beacon", "red", "beacon", 0.09)
    light(m, lod, "strobe_tail_l", (-1.98, 3.66, 7.4), "tail", "strobe", "white", "strobe", 0.06)
    light(m, lod, "strobe_tail_r", (1.98, 3.66, 7.4), "tail", "strobe", "white", "strobe", 0.06)
    light(m, lod, "landing", (0.0, -0.95, -6.0), "nose", "landing", "white", "landing", 0.08)
    if DETAIL[lod]["lights"]:
        strips = [((s * 1.05, 0.5, -3.4), (0.012, 0.09, 0.32)) for s in (-1, 1)] + \
                 [((s * 1.9, 0.75, 4.6), (0.012, 0.32, 0.08)) for s in (-1, 1)]
        clean(boxes_obj(f"formation_strips.L{lod}", strips, "formation", m.zones["centre"]))
        if lod == 0:
            m.lamp("formation", (0.0, 0.6, 0.0), "centre", "green", "steady")
    if det["interior"]:
        fighter_cockpit(m)
    return dict(eye=(0.0, 1.02, -4.8), doors={"DOOR_ladder": (-1.1, gy, -5.0)})


def msl(m, lod, name, nose_at, r, length, zone, pylon=0.0):
    """A missile pointing toward -z with its nose at `nose_at`, four tail fins; on a pylon `pylon` m tall above it."""
    c = (nose_at[0], nose_at[1], nose_at[2] + length * 0.5)
    prof = [(-length * 0.5, 0.01), (-length * 0.5 + r * 2.5, r), (length * 0.5, r), (length * 0.5, 0.01)]
    n = 10 if lod == 0 else 6
    clean(A.obj(f"{name}.L{lod}", lathe(prof, n, c, axis="z"), "belly", m.zones[zone]))
    fins = []
    for a in range(4):
        ang = math.pi / 4 + a * math.pi / 2
        d = (math.cos(ang), math.sin(ang))
        fins.append(((c[0] + d[0] * r * 2.0, c[1] + d[1] * r * 2.0, c[2] + length * 0.42),
                     (abs(d[0]) * r * 1.2 + 0.01, abs(d[1]) * r * 1.2 + 0.01, length * 0.06)))
    clean(boxes_obj(f"{name}_fins.L{lod}", fins, "metal", m.zones[zone]))
    if pylon:
        clean(boxes_obj(f"{name}_pylon.L{lod}", [((c[0], nose_at[1] + r + pylon * 0.5, c[2]), (0.05, pylon * 0.5, length * 0.3))],
                        "top", m.zones[zone]))


def fighter_cockpit(m):
    """The ejection seat, the panel with its hood and the head-up glass, a stick and throttle."""
    z = m.zones["nose"]
    fl = 0.2
    seat_b, frame_b = [], []
    seat(seat_b, frame_b, 0.0, -4.45, fl, w=0.6, back=1.25)
    clean(boxes_obj("int_seat.L0", seat_b, "seat", z))
    clean(boxes_obj("int_seat_frame.L0", frame_b + [((0.0, fl + 1.32, -4.12), (0.2, 0.08, 0.08))], "hazard", z))
    clean(boxes_obj("int_panel.L0", [((0.0, 0.78, -5.95), (0.42, 0.24, 0.12)), ((0.0, 1.02, -5.85), (0.36, 0.04, 0.2)),
                                     ((-0.4, 0.62, -5.0), (0.05, 0.12, 0.6)), ((0.4, 0.62, -5.0), (0.05, 0.12, 0.6))],
                    "panel", z))
    clean(boxes_obj("int_hud.L0", [((0.0, 1.17, -5.75), (0.12, 0.09, 0.01))], "glass", z))
    clean(boxes_obj("int_stick.L0", [((0.0, 0.55, -4.85), (0.03, 0.22, 0.03)), ((-0.33, 0.68, -4.65), (0.04, 0.06, 0.08))],
                    "dark", z))
    m.interior += ["int_seat", "int_panel", "int_hud", "int_stick"]


# ------------------------------------------------------------------------------------------------- the cargo plane
#
# A high-wing tactical transport: a round fuselage on the ground, the wing on top, four turboprops with four-blade
# propellers, gear in side sponsons, a tall fin with the tailplane at its foot, and an upswept tail whose underside is
# the loading ramp. Level 0 is hollow: its hold, troop seats and flight deck are seen through the open ramp.

CARGO_ST = [(-14.9, 0.05, 0.05, -0.3), (-14.5, 0.75, 0.7, -0.25), (-13.6, 1.45, 1.35, -0.15), (-12.4, 1.95, 1.85, -0.06),
            (-10.8, 2.2, 2.15, 0.0), (5.0, 2.2, 2.15, 0.0), (8.0, 2.05, 1.85, 0.32), (11.0, 1.6, 1.35, 0.9),
            (13.4, 1.0, 0.8, 1.45), (14.9, 0.3, 0.3, 1.8)]
RAMP_Y = -0.4  # the ramp is the upswept tail's underside below this height
CARGO_WALL = 0.1


def build_cargo(m, lod):
    det = DETAIL[lod]
    n, nf = det["round"], det["foil"]
    st = CARGO_ST
    gy = -m.kind_rest
    nose = body_piece(f"fus_nose.L{lod}", st, -14.9, -9.0 - GAP, n, "top", m.zones["nose"], caps=(False, True))
    mid = body_piece(f"fus_mid.L{lod}", st, -9.0 + GAP, 5.0 - GAP, n, "top", m.zones["centre"])
    aft = body_piece(f"fus_aft.L{lod}", st, 5.0 + GAP, 14.9, n, "top", m.zones["tail"], caps=(True, False))
    ramp = None
    hinge = (0.0, -2.0 + CARGO_WALL, 5.3)
    if lod == 0:
        m.pivot("ramp", hinge, "tail", (1.0, 0.0, 0.0), role="ramp", angle_deg=28.0, seq="ramp",
                keys=[[0.0, 0.0], [1.0, 1.0]])
    rnode = bpy.data.objects["RIG_ramp"]
    if det["interior"]:
        # hollow: a wall's thickness in from the skin, flight deck to the tail
        for piece, za, zb, zone in ((nose, -13.4, -9.0, "nose"), (mid, -9.0, 5.0, "centre"), (aft, 5.0, 13.2, "tail")):
            hollow(piece, st, za - 0.01, zb + 0.01, CARGO_WALL, n, m.zones[zone])
        # the flight deck's bulkhead stays open; the ramp is cut from the aft shell and hangs on its hinge
        box = [((0.0, RAMP_Y - 3.0, 9.3), (1.85, 3.0, 4.0))]
        ramp = A.obj(f"ramp.L{lod}", bmesh.new(), "top", m.zones["tail"])
        ramp.data = aft.data.copy()
        boolean(ramp, [boxes_obj("cut_ramp2", box, "hazard", m.zones["tail"])], "INTERSECT", "ramp")
        boolean(aft, [boxes_obj("cut_ramp", box, "hazard", m.zones["tail"])], "DIFFERENCE", "ramp opening", hollowing=True)
        ramp.parent = rnode
        move_to_pivot(ramp, hinge)
    else:
        # the ramp a skin just outside the shut tail's underside
        pts = []
        for z in (5.4, 8.0, 10.5, 12.6):
            hw, hh, cy = station(st, z)
            for x in (-hw * 0.82, hw * 0.82):
                y = cy - hh * 0.97 - 0.02
                if y < RAMP_Y:
                    pts += [(x, y, z), (x, y + 0.08, z)]
        if len(pts) >= 8:
            ramp = A.obj(f"ramp.L{lod}", A.hull(pts), "top", rnode)
            move_to_pivot(ramp, hinge)
    # windscreen: recessed glass in the nose
    if det["cut"]:
        ws = []
        for x in (-0.95, -0.35, 0.35, 0.95):
            ws.append(((x, 0.95, -13.15 + abs(x) * 0.25), (0.26, 0.24, 0.5)))
        for x in (-1.75, 1.75):
            ws.append(((x, 0.75, -12.2), (0.4, 0.22, 0.32)))
        frame = "metal" if lod == 0 else "glass"
        boolean(nose, [boxes_obj(f"cut_ws_{lod}", ws, frame, m.zones["nose"])], "DIFFERENCE", "windscreen")
        if lod == 0:
            panes(f"windscreen.L{lod}", st, -14.2, -11.4, n, ws, CARGO_WALL, m.zones["nose"])
            port = [((s * 2.18, 0.3, z), (0.15, 0.2, 0.2)) for s in (-1, 1) for z in (-7.5, -3.0, 1.5)]
            boolean(mid, [boxes_obj("cut_ports", port, "metal", m.zones["centre"])], "DIFFERENCE", "portholes")
            panes(f"portholes.L{lod}", st, -8.0, 2.0, n, port, CARGO_WALL, m.zones["centre"])
    for o in (nose, mid, aft) + ((ramp,) if ramp else ()):
        clean(o)
        paint_belly(o, -1.0)
    # sponsons for the main gear
    for side, s in ((-1, "l"), (1, "r")):
        pts = [(side * x, y, z) for x in (1.7, 2.75) for y in (-0.6, -2.05) for z in (-2.6, 3.6)]
        pts += [(side * 1.9, -1.2, -3.6), (side * 1.9, -1.2, 4.6)]
        sp = A.obj(f"sponson_{s}.L{lod}", A.hull(pts), "belly", m.zones["centre"])
        clean(sp)
    # the high wing, its flaps and ailerons
    root_x, tip_x = 0.0, 20.2
    plan = lambda x: planform(x, root_x, tip_x, -3.2, 4.4, 2.3, 3.0)  # noqa: E731
    y_of = lambda x: 2.35 - max(x - 6.0, 0.0) * 0.04  # noqa: E731 (a little anhedral outboard, as high wings have)
    for side, s in ((-1, "l"), (1, "r")):
        for o in wing_with_surface(m, "wing", f"wing_inner_{s}", side, [root_x, 2.4, 8.6, 9.0], plan, y_of, 0.16, nf,
                                   lod, "top", ("flap", 2.4, 8.6, 0.72, dict(detents_deg=[0.0, 15.0, 35.0]))):
            clean(o)
        for o in wing_with_surface(m, "wing_o", f"wing_outer_{s}", side, [9.0 + GAP, 12.5, 19.0, tip_x], plan, y_of,
                                   0.14, nf, lod, "top", ("aileron", 12.5, 19.0, 0.74, dict(range_deg=18.0)),
                                   cap_root=True):
            clean(o)
    # four turboprops
    for i, (side, x) in enumerate(((-1, 10.6), (-1, 5.2), (1, 5.2), (1, 10.6))):
        zone = f"engine_{i + 1}"
        xx = side * x
        le = plan(x)[0]
        c = (xx, y_of(x) - 0.3, le + 1.0)
        prof = [(-3.3, 0.18), (-3.1, 0.55), (-2.4, 0.68), (0.5, 0.66), (2.6, 0.45), (3.4, 0.12)]
        clean(A.obj(f"nacelle_{i + 1}.L{lod}", lathe(prof, max(n // 2, 8), c, axis="z"), "top", m.zones[zone]))
        hub = (xx, c[1], c[2] - 3.45)
        if lod == 0:
            m.pivot(f"prop_{i + 1}", hub, zone, (0.0, 0.0, -1.0), role="prop", rpm=1020.0)
        pnode = bpy.data.objects[f"RIG_prop_{i + 1}"]
        sp = A.obj(f"spinner_{i + 1}.L{lod}", lathe([(-0.55, 0.01), (-0.3, 0.3), (0.2, 0.34), (0.25, 0.01)], max(n // 2, 8),
                                                   hub, axis="z"), "metal", pnode)
        move_to_pivot(sp, hub)
        clean(sp)
        if det["blade"]:
            clean(blades(f"prop_blades_{i + 1}", hub, (0.0, 0.0, -1.0), 4, 2.05, 0.32, 0.3, pnode, lod, twist_deg=28.0,
                         along=(0.0, 1.0, 0.0)))
        else:
            disc = bmesh.new()
            cyl_into(disc, (hub[0], hub[1], hub[2] - 0.02), (hub[0], hub[1], hub[2] + 0.02), 2.0, 8)
            o = mesh(f"prop_disc_{i + 1}.L{lod}", disc, "dark", pnode, at=hub)
            clean(o)
    # the tail: a tall fin and the tailplane at its foot
    fin_plan = lambda h: (8.6 + h * math.tan(math.radians(30.0)), 6.0 - h * 0.5)  # noqa: E731
    fy = 2.0
    secs = [(fy + h, fin_plan(h)[0], fin_plan(h)[1] * (0.7 if 0.8 < h < 5.4 else 1.0), 0.0, 0.11) for h in (0.0, 0.8, 5.4, 5.9)]
    clean(foil_loft(secs, nf, "top", m.zones["tail"], f"fin.L{lod}", vertical=True))
    ra = (0.0, fy + 0.8 + GAP, fin_plan(0.8)[0] + fin_plan(0.8)[1] * 0.7 + GAP)
    rb = (0.0, fy + 5.4 - GAP, fin_plan(5.4)[0] + fin_plan(5.4)[1] * 0.7 + GAP)
    at = tuple((a + b) * 0.5 for a, b in zip(ra, rb))
    if lod == 0:
        m.pivot("rudder", at, "tail", [b - a for a, b in zip(ra, rb)], role="rudder", range_deg=25.0)
    rn = bpy.data.objects["RIG_rudder"]
    rs = [(fy + h, fin_plan(h)[0] + fin_plan(h)[1] * 0.7 + GAP, fin_plan(h)[1] * 0.3 - GAP, 0.0, 0.07) for h in (0.8 + GAP, 5.4 - GAP)]
    ro = foil_loft(rs, nf, "trim", rn, f"rudder.L{lod}", vertical=True)
    move_to_pivot(ro, at)
    clean(ro)
    tp = lambda x: planform(x, 0.4, 7.9, 9.6, 4.1, 2.0, 8.0)  # noqa: E731
    ty = lambda x: 2.1  # noqa: E731
    for side, s in ((-1, "l"), (1, "r")):
        for o in wing_with_surface(m, "tailplane", "tail", side, [0.4, 1.4, 7.4, 7.9], tp, ty, 0.1, nf, lod, "top",
                                   ("elevator", 1.4, 7.4, 0.7, dict(range_deg=20.0))):
            clean(o)
    # gear: nose twin wheels fold forward, mains (two in tandem a side) rise into the sponsons
    leg(m, lod, "nose", "nose", (0.0, -1.7, -11.6), gy, [-0.24, 0.24], 0.48, 0.3, ((1.0, 0.0, 0.0), -90.0), det["wheel"],
        legr=0.12)
    for side, s in ((-1, "l"), (1, "r")):
        for j, z in enumerate((-1.0, 1.6)):
            leg(m, lod, f"main_{s}{j}", "centre", (side * 2.25, -1.55, z), gy, [0.0], 0.62, 0.42, ((1.0, 0.0, 0.0), -90.0),
                det["wheel"], legr=0.14, keys=[[0.0, 0.0], [0.2, 0.0], [1.0, 1.0]])
    # lights
    tz = plan(tip_x)[0]
    light(m, lod, "nav_left", (-tip_x + 0.05, y_of(tip_x), tz + 0.6), "wing_outer_l", "nav_red", "red", "steady", 0.14)
    light(m, lod, "nav_right", (tip_x - 0.05, y_of(tip_x), tz + 0.6), "wing_outer_r", "nav_green", "green", "steady", 0.14)
    light(m, lod, "strobe_left", (-tip_x + 0.05, y_of(tip_x), tz + 1.6), "wing_outer_l", "strobe", "white", "strobe", 0.1)
    light(m, lod, "strobe_right", (tip_x - 0.05, y_of(tip_x), tz + 1.6), "wing_outer_r", "strobe", "white", "strobe", 0.1)
    light(m, lod, "nav_tail", (0.0, 1.85, 14.95), "tail", "nav_white", "white", "steady", 0.12)
    light(m, lod, "beacon_top", (0.0, fy + 5.95, fin_plan(5.9)[0] + 1.0), "tail", "beacon", "red", "beacon", 0.15)
    light(m, lod, "beacon_bottom", (0.0, -2.18, -4.0), "centre", "beacon", "red", "beacon", 0.15)
    for side, s in ((-1, "l"), (1, "r")):
        light(m, lod, f"landing_{s}", (side * 2.6, y_of(2.6) - 0.15, plan(2.6)[0] + 0.1), f"wing_inner_{s}", "landing",
              "white", "landing", 0.15)
    if det["interior"]:
        cargo_hold(m)
    return dict(eye=(-0.55, 0.95, -12.3), doors={"DOOR_crew_l": (-2.2, gy, -10.0), "DOOR_ramp": (0.0, gy, 13.0)})


def cargo_hold(m):
    """The flight deck (two seats, the panel), the hold's floor with its roller tracks and the troop seats along both
    walls, facing in."""
    fl = -1.55
    zf, zc = m.zones["nose"], m.zones["centre"]
    clean(boxes_obj("int_floor_deck.L0", [((0.0, -0.05, -11.6), (1.5, 0.05, 0.9))], "floor", zf))
    fw = min(A.skin_x(CARGO_ST, z, fl) for z in (-8.8, 4.8)) - CARGO_WALL - 0.03
    clean(boxes_obj("int_floor.L0", [((0.0, fl, -1.9), (fw, 0.05, 6.9))], "floor", zc))
    clean(boxes_obj("int_tracks.L0", [((x, fl + 0.07, -1.9), (0.05, 0.03, 6.9)) for x in (-1.2, -0.4, 0.4, 1.2)], "metal", zc))
    s_b, f_b = [], []
    for x in (-0.55, 0.55):
        seat(s_b, f_b, x, -11.45, -0.05, w=0.6, back=1.2)
    clean(boxes_obj("int_deck_seats.L0", s_b, "seat", zf))
    clean(boxes_obj("int_deck_frames.L0", f_b, "metal", zf))
    clean(boxes_obj("int_panel.L0", [((0.0, 0.65, -12.55), (1.15, 0.28, 0.15)), ((0.0, 0.42, -12.0), (0.18, 0.3, 0.35))],
                    "panel", zf))
    tb, tf = [], []
    for side in (-1, 1):
        for i in range(14):
            z = -8.2 + i * 0.95
            tb.append(((side * 1.52, fl + 0.48, z), (0.24, 0.05, 0.22)))
            tb.append(((side * 1.8, fl + 0.95, z), (0.04, 0.4, 0.22)))
            tf.append(((side * 1.42, fl + 0.22, z), (0.03, 0.2, 0.03)))
    clean(boxes_obj("int_troop_seats.L0", tb, "seat", zc))
    clean(boxes_obj("int_troop_frames.L0", tf, "metal", zc))
    m.interior += ["int_floor", "int_tracks", "int_deck_seats", "int_panel", "int_troop_seats"]


# --------------------------------------------------------------------------------------------------- the helicopters

HELI_WALL = 0.07


def heli_st(kind):
    if kind == "military_helicopter":
        return [(-6.6, 0.12, 0.15, -0.1), (-6.2, 0.62, 0.62, -0.05), (-5.3, 0.95, 0.9, 0.05), (-3.8, 1.1, 1.05, 0.12),
                (1.4, 1.1, 1.05, 0.12), (2.6, 0.75, 0.75, 0.32), (3.6, 0.38, 0.42, 0.55), (9.6, 0.2, 0.26, 0.75),
                (10.2, 0.16, 0.2, 0.78)]
    return [(-5.4, 0.12, 0.15, -0.15), (-5.0, 0.6, 0.6, -0.05), (-4.2, 0.82, 0.82, 0.05), (-2.8, 0.88, 0.88, 0.1),
            (0.6, 0.86, 0.86, 0.1), (1.6, 0.55, 0.6, 0.3), (2.4, 0.26, 0.3, 0.5), (7.0, 0.15, 0.18, 0.62),
            (7.5, 0.13, 0.16, 0.64)]


def build_heli(m, lod, kind):
    det = DETAIL[lod]
    n, nf = det["round"], det["foil"]
    st = heli_st(kind)
    mil = kind == "military_helicopter"
    gy = -m.kind_rest
    z_nose, z_cab, z_boom, z_end = st[0][0], st[2][0] + 0.9, st[5][0], st[-1][0]
    # the glazed nose and the cabin
    nose_st = [(z, hw, hh, cy) for z, hw, hh, cy in st]
    nose = body_piece(f"canopy.L{lod}", nose_st, z_nose, z_cab - GAP, n, "glass", m.zones["nose"], caps=(False, True))
    for p in nose.data.polygons:
        c = nose.matrix_world @ p.center
        if c.z < st[2][3] - st[2][2] * 0.35 or p.material_index == ORDER.index("torn"):
            p.material_index = ORDER.index("top" if c.z >= st[2][3] - st[2][2] * 0.35 else "belly")
    cab = body_piece(f"cabin.L{lod}", st, z_cab + GAP, z_boom - GAP, n, "top", m.zones["centre"])
    boom = body_piece(f"boom.L{lod}", st, z_boom + GAP, z_end, max(n // 2, 8), "top", m.zones["tail"], caps=(True, False))
    if det["cut"]:
        win = [((s * st[3][1], 0.35, z_cab + 1.3), (0.3, 0.34, 0.65)) for s in (-1, 1)]
        if mil:
            win += [((s * st[3][1], 0.35, z_cab + 2.9), (0.3, 0.34, 0.5)) for s in (-1, 1)]
        if det["interior"]:
            hollow(cab, st, z_cab + 0.1, z_boom - 0.1, HELI_WALL, n, m.zones["centre"])
        boolean(cab, [boxes_obj(f"cut_win_{lod}", win, "metal" if det["interior"] else "glass", m.zones["centre"])],
                "DIFFERENCE", "cabin windows")
        if det["interior"]:
            panes(f"cabin_windows.L{lod}", st, z_cab + 0.3, z_boom - 0.3, n, win, HELI_WALL, m.zones["centre"])
    for o in (nose, cab, boom):
        clean(o)
        paint_belly(o, -0.25)
    # the engine housing on the roof, the mast and the main rotor
    top_y = st[3][3] + st[3][2]
    eng = [(s * w, top_y - 0.1, z) for s in (-1, 1) for w, z in ((0.55, z_cab + 0.2), (0.6, z_boom - 0.2))]
    eng += [(s * 0.45, top_y + 0.62, z) for s in (-1, 1) for z in (z_cab + 0.6, z_boom - 0.6)]
    eng += [(0.0, top_y + 0.3, z_boom + 0.6)]
    ehull = A.obj(f"engine_housing.L{lod}", A.hull(eng), "top", m.zones["engine_1"])
    if det["cut"]:
        ex = bmesh.new()
        for s in (-1, 1):
            cyl_into(ex, (s * 0.4, top_y + 0.35, z_boom - 1.0), (s * 0.4, top_y + 0.35, z_boom + 0.9), 0.16, 8)
        boolean(ehull, [A.obj(f"cut_ex_{lod}", ex, "dark", m.zones["engine_1"])], "DIFFERENCE", "exhausts")
    clean(ehull)
    mast = (0.0, top_y + 0.95, (z_cab + z_boom) * 0.5 - 0.3)
    r_rotor = m.k["span"] * 0.5
    if lod == 0:
        m.pivot("rotor", mast, "rotor", (0.0, 1.0, 0.0), role="rotor", rpm=290.0 if mil else 380.0)
    rnode = bpy.data.objects["RIG_rotor"]
    clean(hub_lathe(f"rotor_hub.L{lod}", mast, 0.32, 0.5, max(n // 2, 8), "metal", rnode))
    mast_bm = bmesh.new()
    cyl_into(mast_bm, (mast[0], top_y + 0.4, mast[2]), (mast[0], mast[1], mast[2]), 0.12, 8)
    clean(mesh(f"mast.L{lod}", mast_bm, "metal", m.zones["rotor"]))
    if det["blade"]:
        clean(blades("rotor_blades", mast, (0.0, 1.0, 0.0), 4, r_rotor, 0.55 if mil else 0.38, 0.35, rnode, lod, droop=0.05,
                     twist_deg=6.0))
    else:
        d = bmesh.new()
        cyl_into(d, (mast[0], mast[1] - 0.02, mast[2]), (mast[0], mast[1] + 0.02, mast[2]), r_rotor, 10)
        clean(mesh(f"rotor_disc.L{lod}", d, "dark", rnode, at=mast))
    # the tail: a fin, a stabiliser and the tail rotor on the left of the fin
    fin_secs = [(0.4, z_end - 1.5, 1.4, 0.0, 0.12), (2.0 if mil else 1.5, z_end - 0.5, 0.9, 0.0, 0.1)]
    clean(foil_loft(fin_secs, nf, "top", m.zones["tail"], f"fin.L{lod}", vertical=True))
    for side in (-1, 1):
        tsec = [(0.15, z_end - 2.3, 0.9, 0.55, 0.12), (1.3 if mil else 1.0, z_end - 2.1, 0.6, 0.55, 0.1)]
        clean(foil_loft(tsec, nf, "top", m.zones["tail"], f"stabiliser_{'lr'[side > 0]}.L{lod}", side))
    tr_at = (-0.32, 1.35 if mil else 1.05, z_end - 0.6)
    tr_r = 1.4 if mil else 0.95
    if lod == 0:
        m.pivot("tail_rotor", tr_at, "tail_rotor", (1.0, 0.0, 0.0), role="tail_rotor", rpm=1500.0 if mil else 2100.0)
    tnode = bpy.data.objects["RIG_tail_rotor"]
    if det["blade"]:
        clean(blades("tail_rotor_blades", tr_at, (1.0, 0.0, 0.0), 4 if mil else 2, tr_r, 0.22, 0.1, tnode, lod, twist_deg=10.0,
                     along=(0.0, 1.0, 0.0)))
    else:
        d = bmesh.new()
        cyl_into(d, (tr_at[0] - 0.02, tr_at[1], tr_at[2]), (tr_at[0] + 0.02, tr_at[1], tr_at[2]), tr_r, 8)
        clean(mesh(f"tail_rotor_disc.L{lod}", d, "dark", tnode, at=tr_at))
    clean(hub_lathe(f"tail_rotor_hub.L{lod}", tr_at, 0.12, 0.25, 8, "metal", tnode))
    # landing gear: the gunship stands on fixed wheels, the civil helicopter on skids
    if mil:
        for side in (-1, 1):
            a = (side * 0.9, st[3][3] - st[3][2] + 0.2, z_cab + 0.6)
            ax = (side * 1.35, gy + 0.36, z_cab + 0.6)
            bm = bmesh.new()
            cyl_into(bm, a, ax, 0.08, 8)
            clean(mesh(f"gear_strut_{'lr'[side > 0]}.L{lod}", bm, "metal", m.zones["centre"]))
            node = m.zones["centre"]
            for o in wheel(m, f"gear_main_{'lr'[side > 0]}", node, (0.0, 0.0, 0.0), ax, 0.36, 0.22, det["wheel"], lod):
                clean(o)
        tw = (0.0, gy + 0.2, z_end - 1.2)
        bm = bmesh.new()
        cyl_into(bm, (0.0, st[-2][3] - st[-2][2], z_end - 1.2), tw, 0.05, 6)
        clean(mesh(f"gear_tail_strut.L{lod}", bm, "metal", m.zones["tail"]))
        for o in wheel(m, "gear_tail", m.zones["tail"], (0.0, 0.0, 0.0), tw, 0.2, 0.12, det["wheel"], lod):
            clean(o)
        # stub wings with rocket pods and a chin gun
        for side, s in ((-1, "l"), (1, "r")):
            zone = f"wing_inner_{s}"
            secs = [(0.85, z_cab + 1.6, 1.3, 0.0, 0.14), (2.9, z_cab + 1.8, 0.95, -0.1, 0.12)]
            clean(foil_loft(secs, nf, "top", m.zones[zone], f"stub_wing_{s}.L{lod}", side, cap_torn=True))
            for x in (1.7, 2.6):
                c = (side * x, -0.45, z_cab + 2.1)
                prof = [(-0.9, 0.08), (-0.8, 0.22), (0.9, 0.22), (0.95, 0.05)]
                pod = A.obj(f"pod_{s}{int(x * 10)}.L{lod}", lathe(prof, max(n // 3, 6), c, axis="z"), "belly", m.zones[zone])
                if det["cut"] and lod == 0:
                    tubes = bmesh.new()
                    for k in range(5):
                        a = math.tau * k / 5
                        cyl_into(tubes, (c[0] + math.cos(a) * 0.12, c[1] + math.sin(a) * 0.12, c[2] - 1.2),
                                 (c[0] + math.cos(a) * 0.12, c[1] + math.sin(a) * 0.12, c[2] - 0.5), 0.04, 6)
                    boolean(pod, [A.obj(f"cut_tubes_{s}{x}", tubes, "dark", m.zones[zone])], "DIFFERENCE", "tubes")
                clean(pod)
        gun = bmesh.new()
        cyl_into(gun, (0.0, st[2][3] - st[2][2] - 0.05, -5.0), (0.0, st[2][3] - st[2][2] - 0.05, -6.2), 0.05, 8)
        box_into(gun, (0.0, st[2][3] - st[2][2] + 0.05, -5.0), (0.2, 0.15, 0.25))
        clean(mesh(f"chin_gun.L{lod}", gun, "dark", m.zones["nose"]))
    else:
        for side in (-1, 1):
            sk = bmesh.new()
            x = side * 1.1
            cyl_into(sk, (x, gy + 0.06, -3.4), (x, gy + 0.06, 1.2), 0.06, 8)
            cyl_into(sk, (x, gy + 0.06, -3.4), (x, gy + 0.32, -3.9), 0.06, 8)
            for z in (-2.4, 0.4):
                cyl_into(sk, (x, gy + 0.06, z), (side * 0.6, st[3][3] - st[3][2] + 0.1, z), 0.05, 8)
            clean(mesh(f"skid_{'lr'[side > 0]}.L{lod}", sk, "metal", m.zones["centre"]))
    # lights
    light(m, lod, "nav_left", (-st[3][1] - 0.02, st[3][3] + 0.3, z_cab + 1.0), "centre", "nav_red", "red", "steady", 0.07)
    light(m, lod, "nav_right", (st[3][1] + 0.02, st[3][3] + 0.3, z_cab + 1.0), "centre", "nav_green", "green", "steady", 0.07)
    light(m, lod, "nav_tail", (0.0, st[-1][3], z_end + 0.05), "tail", "nav_white", "white", "steady", 0.06)
    light(m, lod, "beacon_top", (0.0, top_y + 0.66, z_boom - 0.3), "engine_1", "beacon", "red", "beacon", 0.09)
    light(m, lod, "beacon_bottom", (0.0, st[3][3] - st[3][2] - 0.02, z_cab + 1.5), "centre", "beacon", "red", "beacon", 0.09)
    light(m, lod, "landing", (0.0, st[2][3] - st[2][2] + 0.05, z_nose + 1.0), "nose", "landing", "white", "landing", 0.1)
    if det["interior"]:
        heli_cabin(m, st, mil, z_cab, z_boom)
    door_y = gy
    return dict(eye=(-0.42 if not mil else 0.0, st[2][3] + 0.25, z_nose + 1.55 + (0.0 if not mil else 0.2)),
                doors={"DOOR_cabin_l": (-st[3][1] - 0.6, door_y, z_cab + 1.2), "DOOR_cabin_r": (st[3][1] + 0.6, door_y, z_cab + 1.2)})


def heli_cabin(m, st, mil, z_cab, z_boom):
    fl = st[3][3] - st[3][2] * 0.62
    zn, zc = m.zones["nose"], m.zones["centre"]
    s_b, f_b = [], []
    if mil:  # tandem: gunner ahead and low, pilot behind and high
        seat(s_b, f_b, 0.0, st[0][0] + 1.6, fl, w=0.55, back=1.1)
        seat(s_b, f_b, 0.0, st[0][0] + 2.6, fl + 0.3, w=0.55, back=1.1)
    else:
        for x in (-0.42, 0.42):
            seat(s_b, f_b, x, st[0][0] + 1.75, fl, w=0.55)
    clean(boxes_obj("int_crew_seats.L0", s_b, "seat", zn))
    clean(boxes_obj("int_crew_frames.L0", f_b, "metal", zn))
    clean(boxes_obj("int_panel.L0", [((0.0, fl + 0.75, st[0][0] + 0.95), (0.6, 0.18, 0.12))], "panel", zn))
    cb, cf = [], []
    rows = (2, 3) if mil else (3, 3)
    for r in range(2):
        z = z_cab + 1.1 + r * 1.0
        for i in range(rows[r]):
            x = (i - (rows[r] - 1) * 0.5) * 0.55
            seat(cb, cf, x, z, fl, w=0.5, facing=-1.0 if r == 1 else 1.0)
    fw = min(A.skin_x(st, z, fl) for z in (z_cab + 0.2, z_boom - 0.2)) - HELI_WALL - 0.03
    clean(boxes_obj("int_floor.L0", [((0.0, fl, (z_cab + z_boom) * 0.5), (fw, 0.03, (z_boom - z_cab) * 0.5 - 0.1))],
                    "floor", zc))
    clean(boxes_obj("int_cabin_seats.L0", cb, "seat", zc))
    clean(boxes_obj("int_cabin_frames.L0", cf, "metal", zc))
    m.interior += ["int_crew_seats", "int_panel", "int_floor", "int_cabin_seats"]


# ------------------------------------------------------------------------------------------------------ the build

ZONE_TREES = {
    "fighter": [("centre", None), ("nose", "centre"), ("tail", "centre"), ("wing_inner_l", "centre"),
                ("wing_inner_r", "centre"), ("engine_1", "tail"), ("engine_2", "tail")],
    "military_cargo": [("centre", None), ("nose", "centre"), ("tail", "centre"), ("wing_inner_l", "centre"),
                       ("wing_inner_r", "centre"), ("wing_outer_l", "wing_inner_l"), ("wing_outer_r", "wing_inner_r"),
                       ("engine_1", "wing_outer_l"), ("engine_2", "wing_inner_l"), ("engine_3", "wing_inner_r"),
                       ("engine_4", "wing_outer_r")],
    "military_helicopter": [("centre", None), ("nose", "centre"), ("tail", "centre"), ("engine_1", "centre"),
                            ("rotor", "engine_1"), ("tail_rotor", "tail"), ("wing_inner_l", "centre"),
                            ("wing_inner_r", "centre")],
    "civil_helicopter": [("centre", None), ("nose", "centre"), ("tail", "centre"), ("engine_1", "centre"),
                         ("rotor", "engine_1"), ("tail_rotor", "tail")],
}


class Plan:
    """The bits of a kind `Model` needs: its kind name (the root node)."""

    def __init__(self, kind):
        self.kind = kind


def zone_manifest(m, kind):
    out = []
    for name, node in m.zones.items():
        own = [o for o in meshes_under(node) if lod_of(o) == 1 and _zone_of(o) == name]
        pts = [to_gltf(v) for o in own for v in world_verts(o)]
        out.append(dict(name=name, node=f"ZONE_{name}", parent=m.parents[name], share=SHARES[kind][name],
                        capsules=capsules_of(pts) if pts else []))
    return out


def _under(o, name):
    n = o.parent
    while n is not None:
        if n.name == name:
            return True
        n = n.parent
    return False


def _zone_of(o):
    n = o.parent
    while n is not None:
        if n.name.startswith("ZONE_"):
            return n.name[5:]
        n = n.parent
    return None


def build(kind, shots=None):
    k = KINDS[kind]
    K.clear_scene()
    for coll in list(bpy.data.collections):
        bpy.data.collections.remove(coll)
    CSG.clear()
    materials(kind, k)
    m = Model(Plan(kind))
    m.k = k
    m.kind_rest = k["rest"]
    for name, parent in ZONE_TREES[kind]:
        m.zone(name, parent)
    marks = None
    for lod in LEVELS:
        if kind == "fighter":
            marks = build_fighter(m, lod)
        elif kind == "military_cargo":
            marks = build_cargo(m, lod)
        else:
            marks = build_heli(m, lod, kind)
    empty("EYE_pilot", marks["eye"], m.zones["nose"])
    for name, at in marks["doors"].items():
        empty(name, at, m.zones["centre"])
    for o in [o for o in bpy.data.objects if o.name.startswith("cut_")]:
        bpy.data.objects.remove(o)
    bpy.context.view_layer.update()
    stray = [o.name for o in m.root.children if not o.name.startswith("ZONE_")]
    if stray:
        raise SystemExit(f"{kind}: stray nodes under the root: {stray}")
    objs = meshes_under(m.root)
    empty_objs = [o.name for o in objs if not o.data.polygons]
    if empty_objs:
        raise SystemExit(f"{kind}: empty meshes: {empty_objs}")
    counts = {lod: tris([o for o in objs if lod_of(o) == lod]) for lod in LEVELS}
    over = [f"LOD{i}: {t} triangles, budget {k['budget'][i]}" for i, t in counts.items() if t > k["budget"][i]]
    if over:
        raise SystemExit(f"{kind} over budget, nothing written: " + "; ".join(over) + f" (all: {counts})")
    unnamed = [o.name for o in objs if lod_of(o) is None]
    if unnamed:
        raise SystemExit(f"{kind}: meshes with no level: {unnamed[:8]}")
    if abs(sum(SHARES[kind].values()) - 1.0) > 1e-6 or set(SHARES[kind]) != set(m.zones):
        raise SystemExit(f"{kind}: the zone shares do not cover the zones or do not sum to 1")
    pts = [to_gltf(v) for o in objs if lod_of(o) == 1 for v in world_verts(o)]
    lo = [min(q[i] for q in pts) for i in range(3)]
    hi = [max(q[i] for q in pts) for i in range(3)]
    rest = k["rest"]
    # a helicopter's length is its airframe's, nose to tail, without the main rotor's disc (its span)
    body = [to_gltf(v) for o in objs if lod_of(o) == 1 and not _under(o, "RIG_rotor") for v in world_verts(o)]
    blo, bhi = min(q[2] for q in body), max(q[2] for q in body)
    measured = {"length": round(bhi - blo, 3), "span": round(hi[0] - lo[0], 3), "height_at_rest": round(hi[1] + rest, 3),
                "ground_clearance": round(lo[1] + rest, 3)}
    if abs(lo[1] + rest) > 0.03:
        raise SystemExit(f"{kind}: the lowest point is {lo[1] + rest:.3f} m off the ground at rest {rest} m")
    for key, real in (("length", k["length"]), ("span", k["span"]), ("height_at_rest", k["height"])):
        if abs(measured[key] - real) > real * 0.03:
            raise SystemExit(f"{kind}: {key} {measured[key]} m against the design's {real} m (more than 3 % off)")
    for o in objs:
        o.data.validate(clean_customdata=False)
    os.makedirs(OUT, exist_ok=True)
    glb = os.path.join(OUT, f"{kind}.glb")
    sha = A.export(glb, [m.root] + list(m.root.children_recursive))
    eyes = {o.name: [round(v, 3) for v in to_gltf(o.matrix_world.translation)] for o in bpy.data.objects
            if o.name.startswith(("EYE_", "DOOR_", "FX_"))}
    manifest = {
        "_": "One of the military base's aircraft (openspec change military-base), built by "
             "tools/blender/build_military_aircraft.py: written by the build, never by hand. Frame: glTF metres, +Y up, "
             "nose toward -Z, +X the right wing, origin on the fuselage centre line half way along; the ground is at "
             "y = -rest_m. Meshes are <part>.L<level> under ZONE_* nodes or RIG_* pivots; level 0 carries the "
             "interior (int_*). EYE_pilot is the pilot's eye, DOOR_* where a pilot on foot climbs in.",
        "id": kind,
        "file": f"{kind}.glb",
        "generator": "tools/blender/build_military_aircraft.py",
        "blender": bpy.app.version_string,
        "size_m": {"length": k["length"], "span": k["span"], "height": k["height"]},
        "measured_m": measured,
        "rest_m": rest,
        "rest_pitch_deg": 0.0,
        "hp": k["hp"],
        "bounds_m": {"x": [round(lo[0], 3), round(hi[0], 3)], "y": [round(lo[1], 3), round(hi[1], 3)],
                     "z": [round(lo[2], 3), round(hi[2], 3)]},
        "lods": [{"lod": i, "triangles": counts[i], "budget_triangles": k["budget"][i], "within_m": [250, 1500, None][i]}
                 for i in LEVELS],
        "interior": sorted(set(m.interior)),
        "zones": zone_manifest(m, kind),
        "rig": m.rig,
        "sequences_s": {"gear": 8.0, "canopy": 4.0, "ramp": 10.0},
        "lamps": m.lamps,
        "light_patterns": {"steady": {}, "beacon": {"period_s": 1.0, "on_s": 0.1},
                           "strobe": {"period_s": 1.2, "on_s": 0.05, "gap_s": 0.15, "flashes": 2},
                           "landing": {"below_agl_m": 600}},
        "marks": eyes,
        "materials": [A.MATS[key].name for key in ORDER],
        "csg_steps": len(CSG),
        "sha256": sha,
        "bytes": os.path.getsize(glb),
    }
    with open(os.path.join(OUT, f"{kind}.json"), "w") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"EXPORTED {kind}: " + ", ".join(f"LOD{i} {t} tris" for i, t in counts.items())
          + f"; {len(m.zones)} zones, {len(m.rig)} pivots, {len(m.lamps)} lamps; measured {measured}; "
          + f"{manifest['bytes'] / 1e6:.1f} MB")
    if shots:
        look_sheet(m, kind, k, shots)


def look_sheet(m, kind, k, out_dir):
    """Workbench shots of level 0 from six sides, for the eye (not shipped)."""
    os.makedirs(out_dir, exist_ok=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.render.resolution_x, scene.render.resolution_y = 900, 600
    scene.world = bpy.data.worlds.new("w")
    for o in meshes_under(m.root):
        o.hide_render = lod_of(o) != 0
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scene.collection.objects.link(cam)
    scene.camera = cam
    size = max(k["length"], k["span"])
    views = {"front34": (-0.9, 0.45, -1.0), "rear34": (0.9, 0.55, 1.0), "side": (-1.0, 0.02, 0.0), "top": (0.0, 1.0, 0.001),
             "front": (0.0, 0.08, -1.0), "below": (-0.5, -0.6, -0.6)}
    for name, d in views.items():
        d = Vector(d).normalized()
        cam.location = g2b(*(d * size * 1.3))
        look = g2b(0.0, 0.0, 0.0) - cam.location
        cam.rotation_euler = look.to_track_quat("-Z", "Y").to_euler()
        scene.render.filepath = os.path.join(out_dir, f"{kind}_{name}.png")
        bpy.ops.render.render(write_still=True)


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    shots = None
    if "--shots" in argv:
        i = argv.index("--shots")
        shots = argv[i + 1]
        argv = argv[:i] + argv[i + 2:]
    for kind in argv or list(KINDS):
        build(kind, shots)


main()
