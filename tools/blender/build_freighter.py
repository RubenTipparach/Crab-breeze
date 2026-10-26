"""The 1980s heavy freighter (survey W48, D8): civil air traffic's freighter and the air drop's cargo aircraft, modelled
the CSG way in Blender (owner, 2026-10-26, survey D8: "Model a real aircraft in Blender now"; W48: "70 m, with a bought
mech slung under it for the air drop").

A high-wing four-engine jet freighter of the early 1980s, between a C-141 and a C-5 in size: a round fuselage whose
belly rises aft into a loading ramp, a swept high wing with anhedral, four turbofans on pylons, a T-tail, main gear
in blisters on the lower sides and a nose gear. Every size is the kind's own in data/world/air_traffic.ron (length,
span, height, fuselage width) and the rest height the game uses (`rest_m`), so the model, the hit capsules and the
flight describe one aircraft.

The way it is made (`.claude/skills/blender-hard-surface`): primitives (lofted rings for the fuselage and lifting
surfaces, a lathe for the nacelles, convex hulls for the blisters, the fairing and the pylons), then Exact booleans:
the blisters and the wing fairing are unioned into the fuselage, the cockpit windows are cut into the nose as
recesses that take the glass material, each intake is cut into its nacelle and takes the dark fan material, and each
pylon is unioned onto its nacelle. Then the result is cleaned (weld, degenerate and limited dissolve), triangulated,
UV'd in metres by box projection, and shaded smooth with sharp edges over 35 degrees.

Materials are full PBR (CLAUDE.md section 10): the skin and the wing wear the modern plating set
(assets/textures/mech/plate*, plateB*: base colour, normal and ORM maps, engraved panel seams, no rivets) tinted by
the kind's livery, the engines and gear the frame metal (frameM*), embedded in the glb as the mechs' are.

Levels of detail: three nodes, `LOD0` (full detail), `LOD1` and `LOD2`, each the same aircraft built with fewer
segments and fewer small parts, each with a triangle budget the build refuses to exceed. Under each, `GEAR_<n>` holds
its landing gear, which the game shows only near the ground. `SLING_hook` is the empty the air drop hangs a bought mech
from; its sling (cables and spreader bar) is its own file, freighter_sling.glb, in the same frame, drawn only by the
air drop.

Frame: glTF, metres, +Y up, the nose toward -Z, +X the right wing, the origin on the fuselage's centre line at its
middle (the block-out's frame: `cb_core::air_traffic::shape`).

    /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/blender/build_freighter.py

Writes assets/models/aircraft/freighter.glb, freighter_sling.glb and freighter.json (triangles per level against
budget, bounds, nodes, the hook, sha256), then shots for the mockup page under docs/screenshots/aircraft/ are made by
the page itself.
"""
import hashlib
import json
import math
import os
import re
import sys

import bpy  # noqa: I001 (bpy before bmesh, as the pip module needs)
import bmesh
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_kit as K  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "aircraft")
KIND = "freighter"

# Triangle budgets per level (CLAUDE.md section 7: every asset has a budget per LOD in its manifest, and the build
# refuses one over budget). Two at most fly near the pilot (W46); LOD0 is drawn within about 400 m.
BUDGET = {0: 12000, 1: 5000, 2: 2000}
# The detail each level is built with: segments round the fuselage, extra fuselage rings, segments round a nacelle,
# airfoil points, and which small parts it has.
DETAIL = {
    0: dict(round=64, rings=1.0, nacelle=40, foil=9, windows=True, lights=True, gear_seg=20, intakes=True, fairings=True),
    1: dict(round=28, rings=0.5, nacelle=18, foil=6, windows=True, lights=True, gear_seg=10, intakes=True, fairings=True),
    2: dict(round=12, rings=0.0, nacelle=8, foil=4, windows=False, lights=False, gear_seg=6, intakes=False, fairings=False),
}
# The plating tiles every this many metres.
TILE_M = 6.0
# The sling: hardpoints on the belly, the spreader bar below them, and the hook a mech's top hangs from.
SLING_BAR_Y = -12.0
SLING_HOOK_Y = -12.9


def spec():
    """The freighter's numbers from data/world/air_traffic.ron."""
    text = open(os.path.join(ROOT, "data", "world", "air_traffic.ron")).read()
    block = text[text.index(f'"{KIND}"'):]
    block = block[: block.index("),\n        \"") if '),\n        "' in block else len(block)]

    def num(key):
        return float(re.search(rf"\b{key}:\s*([-0-9.]+)", block).group(1))

    tint = [float(x) for x in re.search(r"tint:\s*\(([^)]*)\)", block).group(1).split(",")]
    d = dict(length=num("length_m"), span=num("span_m"), height=num("height_m"), fuselage=num("fuselage_m"), tint=tint)
    # cb_core::air_traffic::rest_m: the origin's height over the ground when it stands on its gear
    d["rest"] = d["fuselage"] * 0.5 + (d["height"] - d["fuselage"]) * 0.15
    return d


def g2b(x, y, z):
    """glTF (y up, nose toward -z) to Blender (z up)."""
    return Vector((x, -z, y))


def lin(c):
    return [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]


MATS = {}
ORDER = ["skin", "wing", "metal", "dark", "glass", "nav_red", "nav_green", "nav_white"]


def materials(tint):
    MATS.clear()
    t = lin(tint)
    MATS["skin"] = K._mat("freighter_skin", rgb=t, tex="plate", normal="plate_n", orm="plate_orm")
    MATS["wing"] = K._mat("freighter_wing", rgb=[c * 0.86 for c in t], tex="plateB", normal="plateB_n", orm="plateB_orm")
    MATS["metal"] = K._mat("freighter_metal", rgb=[0.55, 0.56, 0.58], tex="frameM", normal="frameM_n", orm="frameM_orm")
    MATS["dark"] = K._mat("freighter_dark", rgb=[0.08, 0.08, 0.09], tex="frameM", normal="frameM_n", orm="frameM_orm")
    MATS["glass"] = K._mat("freighter_glass", rgb=[0.02, 0.035, 0.05], rough=0.06, metal=0.1)
    MATS["nav_red"] = K._mat("freighter_nav_red", rgb=[0.6, 0.02, 0.02], emit=0xff2010, emit_k=6.0)
    MATS["nav_green"] = K._mat("freighter_nav_green", rgb=[0.02, 0.5, 0.1], emit=0x20ff50, emit_k=6.0)
    MATS["nav_white"] = K._mat("freighter_nav_white", rgb=[0.9, 0.9, 0.9], emit=0xffffff, emit_k=6.0)
    for m in MATS.values():
        m.use_backface_culling = True


# ----------------------------------------------------------------------------- primitives (glTF frame, metres)

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


def boolean(target, others, op, label):
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


def clean(o):
    """Weld, dissolve degenerate and coplanar splits (material kept), triangulate, UV in metres, shade smooth with
    sharp edges over 35 degrees."""
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
            lp[uv].uv = (u / TILE_M, v / TILE_M)
    for f in bm.faces:
        f.smooth = True
    bm.to_mesh(me)
    bm.free()
    me.set_sharp_from_angle(angle=math.radians(35.0))


# ----------------------------------------------------------------------------- the aircraft

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


def fuselage_stations(d, rings):
    """(z, half width, half height, centre height) from the nose (-z) to the tail: a rounded nose, the long constant
    barrel, and the belly rising aft into the ramp so the tail cone sits high."""
    L, F = d["length"], d["fuselage"]
    r = F * 0.5
    z0 = -L * 0.5
    nose = [(0.0, 0.08, 0.08, -0.6), (0.7, 0.42, 0.36, -0.55), (1.8, 0.62, 0.56, -0.4), (3.4, 0.8, 0.75, -0.22),
            (5.4, 0.93, 0.9, -0.08), (7.6, 0.99, 0.98, -0.01), (9.5, 1.0, 1.0, 0.0)]
    aft = [(0.71, 1.0, 1.0, 0.0), (0.76, 0.98, 0.94, 0.08), (0.81, 0.92, 0.82, 0.22), (0.86, 0.82, 0.68, 0.36),
           (0.9, 0.7, 0.55, 0.48), (0.94, 0.55, 0.42, 0.58), (0.97, 0.38, 0.3, 0.64), (1.0, 0.1, 0.1, 0.68)]
    st = [(z0 + a, r * w, r * h, r * c) for a, w, h, c in nose]
    if rings > 0:
        mid = [0.3, 0.45, 0.6] if rings >= 1 else [0.45]
        st += [(z0 + L * t, r, r, 0.0) for t in mid]
    st += [(z0 + L * t, r * w, r * h, r * c) for t, w, h, c in aft]
    if rings <= 0:  # the far level: every other nose and tail ring
        st = st[::2] + ([st[-1]] if len(st) % 2 == 0 else [])
    if rings >= 1:  # the near level: a ring half way between each pair over the nose and the tail, for a smooth line
        fine = []
        for a, b in zip(st, st[1:]):
            fine.append(a)
            if b[0] - a[0] < L * 0.12:
                fine.append(tuple((x + y) * 0.5 for x, y in zip(a, b)))
        st = fine + [st[-1]]
    return st


def wing_planform(d):
    """The wing's root and tip: (x, leading edge z, chord, height of the chord line, thickness share)."""
    s = d["span"] * 0.5
    root = (0.0, -9.5, 10.5, d["fuselage"] * 0.5 - 0.45, 0.13)
    sweep = math.tan(math.radians(25.0))
    tip_x = s
    tip_chord = 3.6
    tip_le = root[1] + (tip_x - 2.0) * sweep + (root[2] - tip_chord) * 0.25
    tip = (tip_x, tip_le, tip_chord, root[3] - (tip_x - 2.0) * math.tan(math.radians(3.0)), 0.09)
    return root, tip


def airfoil(x, le, chord, y, t, n, vertical=False, side=1.0):
    """An airfoil ring at span station x: n points round, upper then lower, a rounded nose and a sharp tail."""
    ts = {9: [0.0, 0.025, 0.08, 0.18, 0.32, 0.5, 0.7, 0.86, 1.0], 6: [0.0, 0.05, 0.2, 0.45, 0.75, 1.0],
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


def wing_at(d, x):
    """The wing's leading edge z, chord and chord-line height at span x."""
    root, tip = wing_planform(d)
    t = (x - root[0]) / (tip[0] - root[0])
    return root[1] + (tip[1] - root[1]) * t, root[2] + (tip[2] - root[2]) * t, root[3] + (tip[3] - root[3]) * t


def nacelle(d, lod, x, parent, det):
    """A turbofan: a lathed pod, its intake cut hollow to a dark fan face, its exhaust cone, and its pylon unioned on."""
    le, chord, wy = wing_at(d, abs(x))
    zf = le - 3.2
    cy = wy - 2.55
    prof = [(0.0, 1.02), (0.25, 1.2), (1.2, 1.28), (3.6, 1.24), (5.4, 1.06), (6.6, 0.82)]
    if det["nacelle"] <= 8:
        prof = [(0.0, 1.1), (1.2, 1.28), (5.0, 1.1), (6.6, 0.82)]
    pod = obj(f"engine_{lod}_{x:+.0f}", lathe(prof, det["nacelle"], (x, cy, zf)), "metal", parent)
    if det["intakes"]:
        intake = obj(f"intake_{lod}_{x:+.0f}", lathe([(-0.4, 0.9), (0.9, 0.9)], det["nacelle"], (x, cy, zf)), "dark", parent)
        boolean(pod, [intake], "DIFFERENCE", "intake")
    cone = lathe([(6.4, 0.62), (7.5, 0.05)], max(det["nacelle"] // 2, 6), (x, cy, zf))
    pylon_pts = []
    for zz, top in ((zf + 1.0, False), (zf + 5.6, False)):
        for dx in (-0.22, 0.22):
            pylon_pts.append((x + dx, cy + 0.8, zz))
    for zz in (le + 0.6, le + chord * 0.75):
        for dx in (-0.2, 0.2):
            pylon_pts.append((x + dx, wy + 0.2, zz))
    pylon = obj(f"pylon_{lod}_{x:+.0f}", hull(pylon_pts), "skin", parent)
    boolean(pod, [pylon], "UNION", "pylon")
    obj(f"exhaust_{lod}_{x:+.0f}", cone, "dark", parent)
    return pod


def gear(d, lod, parent, det):
    """The landing gear under `GEAR_<lod>`: a nose pair and three main pairs each side, each pair on a strut up into
    the fuselage or its blister."""
    g = empty(f"GEAR_{lod}", (0, 0, 0), parent)
    ground = -d["rest"]
    wr, ww = 0.62, 0.42
    seg = det["gear_seg"]
    r = d["fuselage"] * 0.5
    axles = [(0.0, -d["length"] * 0.5 + 7.5, -r + 0.6)] + [(sx * 3.5, z, -2.6) for sx in (-1, 1) for z in (-1.5, 1.2, 3.9)]
    for i, (x, z, top) in enumerate(axles):
        for dx in (-0.34, 0.34):
            w = lathe([(-ww * 0.5, wr), (ww * 0.5, wr)], seg, (x + dx, ground + wr, z), axis="x")
            obj(f"wheel_{lod}_{i}_{dx:+.2f}", w, "dark", g)
        strut = hull([(x + a, ground + wr, z + b) for a in (-0.14, 0.14) for b in (-0.14, 0.14)]
                     + [(x + a, top, z + b) for a in (-0.14, 0.14) for b in (-0.14, 0.14)])
        obj(f"strut_{lod}_{i}", strut, "metal", g)
    return g


def light(name, at, mat, parent, r=0.18):
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=r, matrix=Matrix.Translation(g2b(*at)))
    return obj(name, bm, mat, parent)


def build_level(d, lod, root):
    det = DETAIL[lod]
    node = empty(f"LOD{lod}", (0, 0, 0), root)
    L, S, F = d["length"], d["span"], d["fuselage"]
    r = F * 0.5
    top = d["height"] - d["rest"]
    # the fuselage
    rings = [section(z, hw, hh, cy, det["round"]) for z, hw, hh, cy in fuselage_stations(d, det["rings"])]
    body = obj(f"fuselage_{lod}", loft(rings), "skin", node)
    # the main gear blisters and the wing's fairing over the fuselage, unioned in
    pieces = []
    for sx in (-1, 1):
        pts = [(sx * x, y, z) for z in (-6.5, 7.5) for x, y in ((r * 0.55, -r * 0.98), (r * 1.18, -r * 0.92), (r * 1.18, -r * 0.3), (r * 0.6, -r * 0.2))]
        pts += [(sx * r * 0.6, -r * 0.6, -9.5), (sx * r * 0.6, -r * 0.6, 10.5)]
        pieces.append(obj(f"blister_{lod}_{sx:+d}", hull(pts), "skin", node))
    root_le, root_chord, root_y = wing_at(d, 0.0)
    # the fairing: a rounded hump over the wing's root, lofted from the fuselage's own sections, tapering at both ends
    z_a, z_b = root_le - 4.0, root_le + root_chord + 3.5
    hump = []
    for t, k in ((0.0, 0.25), (0.12, 0.7), (0.3, 1.0), (0.7, 1.0), (0.88, 0.7), (1.0, 0.25)):
        z = z_a + (z_b - z_a) * t
        hump.append(section(z, r * 0.78 * k, (root_y + 0.85 - r * 0.45) * k, r * 0.45, det["round"] // 2))
    pieces.append(obj(f"fairing_{lod}", loft(hump), "skin", node))
    boolean(body, pieces, "UNION", "blisters and fairing")
    # the cockpit windows: recesses 6 cm deep cut through the nose's skin; each recess's floor takes the glass
    if det["windows"]:
        st = fuselage_stations(d, 1.0)
        cut = []
        z0 = -L * 0.5
        for i, x in enumerate((-1.5, -0.5, 0.5, 1.5)):
            za, zb = z0 + 3.0, z0 + 4.3
            pts = []
            for z in (za, zb):
                for xx in (x - 0.42, x + 0.42):
                    y = skin_y(st, z, xx) - 0.06
                    pts += [(xx, y, z), (xx, y + 1.5, z)]
            cut.append(obj(f"window_{lod}_{i}", hull(pts), "glass", node))
        for i, sx in enumerate((-1, 1)):
            za, zb = z0 + 4.6, z0 + 5.6
            pts = []
            for z in (za, zb):
                for y in (1.2, 1.8):
                    x = skin_x(st, z, y) - 0.06
                    pts += [(sx * x, y, z), (sx * (x + 1.5), y, z)]
            cut.append(obj(f"window_{lod}_side{i}", hull(pts), "glass", node))
        boolean(body, cut, "DIFFERENCE", "windows")
    clean(body)
    # the wing, both halves, lofted root to tip with a mid ring at full detail
    root, tip = wing_planform(d)
    for side in (-1.0, 1.0):
        w = lifting(f"wing_{lod}_{side:+.0f}", root, tip, det["foil"], "wing", node, side=side, mid=lod == 0)
        clean(w)
    # the flap tracks' fairings under the trailing edge, four a side
    if det["fairings"]:
        for side in (-1.0, 1.0):
            for x in (6.5, 10.0, 16.5, 26.0):
                le, chord, wy = wing_at(d, x)
                te = le + chord
                pts = [(side * (x + a), wy + b, z) for a in (-0.28, 0.28) for b, z in ((0.1, te - chord * 0.35), (-0.25, te - chord * 0.2))]
                pts += [(side * x, wy - 0.75, te - 0.2), (side * x, wy - 0.3, te + 1.6), (side * (x - 0.2), wy - 0.55, te + 0.8),
                        (side * (x + 0.2), wy - 0.55, te + 0.8)]
                clean(obj(f"flap_track_{lod}_{side:+.0f}_{x:.0f}", hull(pts), "wing", node))
    # four turbofans
    for x in (-S * 0.33, -S * 0.19, S * 0.19, S * 0.33):
        clean(nacelle(d, lod, x, node, det))
    for o in [o for o in node.children if o.name.startswith("exhaust_")]:
        clean(o)
    # the fin and the T tail
    fin_root = (r * 0.55, L * 0.5 - 13.5, 9.5, 0.0, 0.11)
    fin_tip = (top - 0.6, L * 0.5 - 6.0, 5.0, 0.0, 0.09)
    fin = lifting(f"fin_{lod}", fin_root, fin_tip, det["foil"], "wing", node, vertical=True)
    clean(fin)
    tail_y = top - 0.9
    tp_root = (0.0, L * 0.5 - 10.0, 6.2, tail_y, 0.1)
    tp_tip = (S * 0.19, L * 0.5 - 10.0 + S * 0.19 * math.tan(math.radians(25.0)) + 0.9, 2.8, tail_y + 0.4, 0.08)
    for side in (-1.0, 1.0):
        clean(lifting(f"tailplane_{lod}_{side:+.0f}", tp_root, tp_tip, det["foil"], "wing", node, side=side))
    # navigation lights: red on the left wing tip, green on the right, white on the tail
    if det["lights"]:
        tle, tchord, ty = wing_at(d, S * 0.5)
        light(f"nav_left_{lod}", (-S * 0.5 + 0.2, ty, tle + tchord * 0.3), "nav_red", node)
        light(f"nav_right_{lod}", (S * 0.5 - 0.2, ty, tle + tchord * 0.3), "nav_green", node)
        light(f"nav_tail_{lod}", (0.0, top - 0.2, L * 0.5 - 2.4), "nav_white", node)
    for o in node.children:
        if o.name.startswith("nav_"):
            clean(o)
    g = gear(d, lod, node, det)
    for o in g.children:
        clean(o)
    return node


def sling(d, root):
    """The air drop's sling, its own file: four cables from two belly hardpoints to a spreader bar, and a hook block.
    Same frame as the aircraft, so it is a child with no offset."""
    belly = -d["fuselage"] * 0.5 + 0.1
    parts = []
    for z in (-6.0, 6.0):
        for x in (-1.2, 1.2):
            a, b = Vector((x, belly, z)), Vector((0.0, SLING_BAR_Y, z * 1.05))
            n = 8
            ring_a = [(a.x + 0.07 * math.cos(math.tau * k / n), a.y, a.z + 0.07 * math.sin(math.tau * k / n)) for k in range(n)]
            ring_b = [(b.x + 0.07 * math.cos(math.tau * k / n), b.y, b.z + 0.07 * math.sin(math.tau * k / n)) for k in range(n)]
            parts.append(obj(f"cable_{x:+.1f}_{z:+.0f}", loft([ring_a, ring_b]), "dark", root))
        pad = hull([(x, y, z + c) for x in (-1.5, 1.5) for y in (belly - 0.4, belly + 0.6) for c in (-0.5, 0.5)])
        parts.append(obj(f"hardpoint_{z:+.0f}", pad, "metal", root))
    bar = hull([(x, y, z) for x in (-0.25, 0.25) for y in (SLING_BAR_Y - 0.25, SLING_BAR_Y + 0.25) for z in (-6.8, 6.8)])
    parts.append(obj("spreader_bar", bar, "metal", root))
    hook = hull([(x, y, z) for x in (-0.45, 0.45) for y in (SLING_HOOK_Y, SLING_BAR_Y - 0.2) for z in (-0.6, 0.6)])
    parts.append(obj("hook", hook, "metal", root))
    for p in parts:
        clean(p)


# ----------------------------------------------------------------------------- measure and export

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


def main():
    d = spec()
    K.clear_scene()
    for c in list(bpy.data.collections):
        bpy.data.collections.remove(c)
    materials(d["tint"])
    os.makedirs(OUT, exist_ok=True)
    root = empty(KIND, (0, 0, 0), None)
    levels = {}
    for lod in (0, 1, 2):
        node = build_level(d, lod, root)
        levels[lod] = (tris_under(node), node)
    hook = empty("SLING_hook", (0.0, SLING_HOOK_Y, 0.0), root)
    over = [f"LOD{k}: {t} triangles, budget {BUDGET[k]}" for k, (t, _) in levels.items() if t > BUDGET[k]]
    if over:
        raise SystemExit("freighter over budget, nothing written: " + "; ".join(over))
    bounds = bounds_under(levels[0][1])
    glb = os.path.join(OUT, f"{KIND}.glb")
    sha = export(glb, [root] + list(root.children_recursive))
    # the sling, its own file
    for o in [root] + list(root.children_recursive):
        o.select_set(False)
    sroot = empty(f"{KIND}_sling", (0, 0, 0), None)
    sling(d, sroot)
    sling_glb = os.path.join(OUT, f"{KIND}_sling.glb")
    sling_sha = export(sling_glb, [sroot] + list(sroot.children_recursive))
    names = sorted(o.name for o in [root] + list(root.children_recursive) if o.name.startswith(("LOD", "GEAR_", "SLING_")))
    manifest = {
        "_": "The 1980s heavy freighter (survey W48, D8), built by tools/blender/build_freighter.py: written by the "
             "build, never by hand. Frame: glTF metres, +Y up, nose toward -Z, +X the right wing, origin on the "
             "fuselage's centre line at its middle; the ground under its gear is at y = -rest_m.",
        "id": KIND,
        "file": f"{KIND}.glb",
        "sling_file": f"{KIND}_sling.glb",
        "generator": "tools/blender/build_freighter.py",
        "blender": bpy.app.version_string,
        "size_m": {"length": d["length"], "span": d["span"], "height": d["height"], "fuselage": d["fuselage"]},
        "rest_m": round(d["rest"], 3),
        "bounds_m": bounds,
        "lods": [{"lod": k, "triangles": t, "budget_triangles": BUDGET[k]} for k, (t, _) in sorted(levels.items())],
        "nodes": names,
        "sling_hook_m": [0.0, SLING_HOOK_Y, 0.0],
        "materials": [MATS[k].name for k in ORDER],
        "textures": ["plate", "plate_n", "plate_orm", "plateB", "plateB_n", "plateB_orm", "frameM", "frameM_n", "frameM_orm"],
        "csg": CSG,
        "sha256": {f"{KIND}.glb": sha, f"{KIND}_sling.glb": sling_sha},
        "bytes": os.path.getsize(glb),
    }
    with open(os.path.join(OUT, f"{KIND}.json"), "w") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")
    print("EXPORTED freighter: " + ", ".join(f"LOD{k} {t} tris" for k, (t, _) in sorted(levels.items()))
          + f"; bounds {bounds}; {manifest['bytes'] / 1e6:.1f} MB")


main()
