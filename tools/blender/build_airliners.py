"""The three airliners (openspec change `airliner-models`): a twin radial propliner after the DC-3, an early four-jet
narrowbody after the Boeing 720 and a three-engine wide-body after the L-1011 TriStar, modelled the CSG way in Blender
from the NASA public-domain three-views the owner approved (docs/reference/airliner-blueprints/; owner, 2026-10-29:
"approve all, send it, make me models, and setup rig and materials so I can see planes with their gear down, up, props
rotating, and navigation lights", "make interiors work! no need for light baking").

What each file holds (design sections 1-6):
- a hollow fuselage (the outer loft minus an inner loft a wall's thickness in), its cabin windows cut through the wall
  with a pane of glass in each, the flight deck's windscreen, the gear wells;
- the wing, tailplanes and fin lofted from NACA sections, the ailerons, flaps, elevators and rudder cut out of them on
  their hinge lines;
- turbofans (lathed pods, intakes cut to a spinning fan, pylons unioned on) or radials (cowl, cylinders, three-blade
  propellers);
- the landing gear on pivots, its doors cut out of the very shell they close;
- the lights: navigation, beacons, strobes and landing lights, each with a `LAMP_*` empty;
- the interior: the flight deck, the cabin floor, every seat row, overhead bins, bulkheads and galleys;
- damage zones: every part above is cut at the airframe's joints into `ZONE_<name>` nodes, each cut capped with the
  `torn` material, so the game can break an aircraft apart by moving nodes.

Every mesh is named `<part>.L<n>` for its level of detail (0 within 250 m with the interior, 1 to 1.5 km, 2 beyond)
and hangs under its zone, or under a `RIG_*` pivot when it moves. The manifest (`<kind>.json`) lists the levels'
triangles against budget, the zones (parent, health share, capsules measured off their meshes), the rig (each pivot's
axis and travel), the lamps, the seats and the rest pose; the page (tools/mockups/gen_airliners.py) and the game read
it, never a copy.

Frame: glTF metres, +Y up, the nose toward -Z, +X the right wing, the origin on the fuselage's centre line half way
along it. The jets stand level on their gear with the origin `rest_m` over the ground; the propliner sits tail down,
pitched `rest_pitch_deg`.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_airliners.py [-- propliner ...] [--shots DIR]
"""
import hashlib
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
from aircraft_kit import CSG, clean, empty, g2b, hull, lathe, lin, loft, skin_x  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "aircraft")
K.TEXDIR = os.path.join(ROOT, "assets", "textures", "mech")
CABIN_TEX = os.path.join(ROOT, "assets", "textures", "aircraft")
# The fuselage's cross-section: a superellipse this close to a circle.
EXP = 2.05
# Zone cuts leave this gap between neighbours, so no two caps share a plane (CLAUDE.md section 10: no z-fighting).
GAP = 0.002
LEVELS = (0, 1, 2)
DETAIL = {
    0: dict(round=48, foil=12, nacelle=32, wheel=20, interior=True, windows=True, hollow=True, lights=True),
    1: dict(round=24, foil=9, nacelle=16, wheel=12, interior=False, windows=False, hollow=False, lights=True),
    2: dict(round=10, foil=4, nacelle=8, wheel=6, interior=False, windows=False, hollow=False, lights=False),
}

# ----------------------------------------------------------------------------------------------------------- the kinds
# Sizes are the real aircraft's published ones; shapes from the NASA three-views. Stations: (share of the length from
# the nose, share of the half width, share of the half height, centre height as a share of the half height).
KINDS = {
    "propliner": dict(
        after="Douglas DC-3", blueprint="douglas-c47-dc3.png",
        length=19.7, span=29.0, height=5.16, width=2.35, depth=2.55, wall=0.08,
        stations=[(0.0, 0.06, 0.06, -0.1), (0.02, 0.45, 0.42, -0.08), (0.05, 0.7, 0.66, -0.04), (0.09, 0.86, 0.85, 0.0),
                  (0.14, 0.95, 0.96, 0.02), (0.2, 1.0, 1.0, 0.0), (0.52, 1.0, 1.0, 0.0), (0.62, 0.93, 0.95, 0.04),
                  (0.72, 0.78, 0.82, 0.1), (0.82, 0.58, 0.62, 0.2), (0.9, 0.4, 0.44, 0.3), (0.96, 0.22, 0.26, 0.38),
                  (1.0, 0.07, 0.08, 0.42)],
        # the wing: chord-line height, root leading edge (from the nose), root chord, tip chord, the flat centre
        # section's half span, dihedral outside it, thickness at root and tip, extra stations, the zone split
        wing=dict(y=-0.85, le=5.75, root=4.4, tip=1.45, centre=2.95, dihedral=5.0, t_root=0.16, t_tip=0.1,
                  te_sweep=3.0, split=8.6, stations=[2.95, 8.6]),
        fin=dict(le=-3.15, root=3.0, tip=1.05, top=2.45, sweep=38.0, t=0.11),
        tailplane=dict(le=-2.95, root=2.3, tip=1.05, span=7.9, sweep=12.0, y=0.25, dihedral=0.0, t=0.1),
        engines=[dict(kind="radial", x=2.85, zone="wing_inner")],
        radial=dict(r=0.72, ahead=2.35, behind=0.95, prop_r=1.75, blades=3),
        gear=dict(kind="taildragger", main_r=0.58, main_w=0.36, leg=1.55, tail_r=0.2, tail_z=-1.85),
        rest_pitch=None,  # from the gear
        cabin=dict(floor=-0.45, first=-4.6, pitch=0.86, rows=7, blocks=(2, 1), seat_w=0.46, bins=False,
                   deck_end=-6.15, cabin_end=2.95, galley_aft=1.2, window_h=0.42, window_w=0.5, window_y=0.55,
                   windows_per_row=1),
        zones=dict(nose=-6.2, centre_end=-0.6, aft_end=3.4),
        windscreen=dict(z0=0.055, z1=0.1, panes=4),
        livery=dict(crown=(0.93, 0.93, 0.9), line=(0.12, 0.32, 0.62), belly=(0.62, 0.63, 0.64), seat=(0.18, 0.3, 0.55),
                    carpet=(0.25, 0.22, 0.2)),
        seats=21, budget={0: 30000, 1: 6000, 2: 2000},
    ),
    "airliner": dict(
        after="Boeing 720", blueprint="boeing-720.png",
        length=41.3, span=39.9, height=12.65, width=3.76, depth=4.1, wall=0.12,
        stations=[(0.0, 0.03, 0.03, -0.35), (0.01, 0.32, 0.3, -0.3), (0.025, 0.55, 0.52, -0.22),
                  (0.045, 0.75, 0.72, -0.12), (0.07, 0.9, 0.88, -0.05), (0.1, 0.97, 0.96, -0.01), (0.14, 1.0, 1.0, 0.0),
                  (0.7, 1.0, 1.0, 0.0), (0.76, 0.96, 0.95, 0.04), (0.82, 0.86, 0.85, 0.12), (0.88, 0.7, 0.68, 0.24),
                  (0.93, 0.5, 0.48, 0.36), (0.97, 0.3, 0.28, 0.48), (1.0, 0.08, 0.08, 0.55)],
        wing=dict(y=-1.27, le=14.3, root=10.0, tip=2.3, centre=1.9, dihedral=7.0, t_root=0.13, t_tip=0.09,
                  le_sweep=37.0, split=9.5, stations=[1.9, 6.8, 9.5, 12.4], chords={1.9: 9.4, 6.8: 7.0, 9.5: 5.8, 12.4: 4.6}),
        fin=dict(le=-9.6, root=7.4, tip=2.4, top=8.9, sweep=40.0, t=0.1),
        tailplane=dict(le=-6.9, root=4.8, tip=1.7, span=12.8, sweep=35.0, y=0.5, dihedral=6.0, t=0.09),
        engines=[dict(kind="turbofan", x=6.8, zone="wing_inner"), dict(kind="turbofan", x=12.4, zone="wing_outer")],
        turbofan=dict(r=0.78, intake=0.66, length=5.4, ahead=3.3, drop=1.3),
        gear=dict(kind="tricycle", main_x=3.36, main_r=0.58, main_w=0.38, leg=2.0, bogie=1.4, nose_z=4.6, nose_r=0.45,
                  nose_leg=2.05),
        cabin=dict(floor=-0.55, first=-13.0, pitch=0.84, rows=23, blocks=(3, 3), seat_w=0.46, bins=True,
                   deck_end=-15.4, cabin_end=8.4, galley_aft=1.8, window_h=0.36, window_w=0.26, window_y=1.0,
                   windows_per_row=1),
        zones=dict(nose=-15.5, fwd_end=-6.2, centre_end=0.9, aft_end=10.6),
        windscreen=dict(z0=0.04, z1=0.075, panes=6),
        livery=dict(crown=(0.94, 0.94, 0.93), line=(0.78, 0.18, 0.1), belly=(0.66, 0.67, 0.69), seat=(0.6, 0.2, 0.15),
                    carpet=(0.3, 0.2, 0.18)),
        seats=138, budget={0: 60000, 1: 10000, 2: 2500},
    ),
    "widebody": dict(
        after="Lockheed L-1011 TriStar", blueprint="lockheed-l1011-tristar.png",
        length=54.2, span=47.35, height=16.87, width=5.97, depth=6.0, wall=0.12,
        stations=[(0.0, 0.03, 0.03, -0.3), (0.008, 0.35, 0.32, -0.27), (0.02, 0.6, 0.56, -0.2),
                  (0.035, 0.78, 0.75, -0.12), (0.055, 0.9, 0.89, -0.06), (0.08, 0.97, 0.97, -0.02), (0.11, 1.0, 1.0, 0.0),
                  (0.72, 1.0, 1.0, 0.0), (0.78, 0.95, 0.95, 0.06), (0.84, 0.84, 0.84, 0.15), (0.9, 0.68, 0.68, 0.25),
                  (0.95, 0.5, 0.5, 0.32), (1.0, 0.32, 0.32, 0.38)],
        wing=dict(y=-1.5, le=20.5, root=11.5, tip=2.9, centre=2.9, dihedral=7.5, t_root=0.13, t_tip=0.09,
                  le_sweep=37.0, split=14.5, stations=[2.9, 9.6, 14.5, 18.0], chords={2.9: 11.0, 9.6: 7.6, 14.5: 5.6, 18.0: 4.4}),
        fin=dict(le=-11.4, root=8.6, tip=3.0, top=12.1, sweep=38.0, t=0.1, base=1.55),
        tailplane=dict(le=-10.4, root=6.5, tip=2.2, span=21.8, sweep=35.0, y=0.25, dihedral=3.0, t=0.09),
        engines=[dict(kind="turbofan", x=9.6, zone="wing_inner"), dict(kind="tail", zone="tail")],
        turbofan=dict(r=1.32, intake=1.12, length=6.8, ahead=3.6, drop=1.9),
        gear=dict(kind="tricycle", main_x=5.45, main_r=0.62, main_w=0.42, leg=2.4, bogie=1.6, nose_z=6.3, nose_r=0.5,
                  nose_leg=2.4),
        cabin=dict(floor=-0.7, first=-18.6, pitch=0.81, rows=34, blocks=(2, 5, 2), seat_w=0.46, bins=True,
                   deck_end=-21.0, cabin_end=11.6, galley_aft=2.0, window_h=0.36, window_w=0.27, window_y=1.0,
                   windows_per_row=1),
        zones=dict(nose=-21.1, fwd_end=-9.0, centre_end=1.6, aft_end=13.8),
        windscreen=dict(z0=0.03, z1=0.06, panes=6),
        livery=dict(crown=(0.95, 0.95, 0.94), line=(0.1, 0.45, 0.4), belly=(0.7, 0.71, 0.72), seat=(0.12, 0.38, 0.36),
                    carpet=(0.22, 0.24, 0.26)),
        seats=306, budget={0: 90000, 1: 14000, 2: 3000},
    ),
}

# Health shares of the kind's `hp` (design section 5); the propliner has no forward cabin and no outer engines.
SHARES = {"centre": 0.30, "cabin_fwd": 0.15, "cabin_aft": 0.15, "nose": 0.10, "tail": 0.12, "wing_inner": 0.14,
          "wing_outer": 0.08, "engine": 0.06}

ORDER = ["crown", "line", "belly", "wing", "fin", "nacelle", "metal", "dark", "rubber", "glass", "torn", "cabin_wall",
         "seat", "carpet", "trim", "panel", "nav_red", "nav_green", "nav_white", "beacon", "strobe", "landing"]


def materials(kind, k):
    """The kind's materials, every custom surface a full PBR set (CLAUDE.md section 10)."""
    A.MATS.clear()
    A.ORDER[:] = ORDER
    lv = k["livery"]
    m = A.MATS
    m["crown"] = K._mat(f"{kind}_crown", rgb=lin(lv["crown"]), tex="plate", normal="plate_n", orm="plate_orm")
    m["line"] = K._mat(f"{kind}_line", rgb=lin(lv["line"]), tex="plate", normal="plate_n", orm="plate_orm")
    m["belly"] = K._mat(f"{kind}_belly", rgb=lin(lv["belly"]), tex="plateB", normal="plateB_n", orm="plateB_orm")
    m["wing"] = K._mat(f"{kind}_wing", rgb=lin((0.78, 0.79, 0.8)), tex="plateB", normal="plateB_n", orm="plateB_orm")
    m["fin"] = K._mat(f"{kind}_fin", rgb=lin(lv["line"]), tex="plate", normal="plate_n", orm="plate_orm")
    m["nacelle"] = K._mat(f"{kind}_nacelle", rgb=lin((0.86, 0.86, 0.85)), tex="frameM", normal="frameM_n", orm="frameM_orm")
    m["metal"] = K._mat(f"{kind}_metal", rgb=[0.55, 0.56, 0.58], tex="frameM", normal="frameM_n", orm="frameM_orm")
    m["dark"] = K._mat(f"{kind}_dark", rgb=[0.05, 0.05, 0.06], tex="frameM", normal="frameM_n", orm="frameM_orm")
    m["rubber"] = _cabin_mat(f"{kind}_rubber", "rubber", (0.06, 0.06, 0.06))
    m["glass"] = K._mat(f"{kind}_glass", rgb=[0.02, 0.035, 0.05], rough=0.05, metal=0.1, alpha=0.45)
    m["torn"] = K._mat(f"{kind}_torn", rgb=[0.09, 0.08, 0.07], tex="wear", normal="wear_n", orm="wear_orm")
    m["cabin_wall"] = _cabin_mat(f"{kind}_cabin_wall", "trim", (0.86, 0.84, 0.8))
    m["seat"] = _cabin_mat(f"{kind}_seat", "fabric", lv["seat"])
    m["carpet"] = _cabin_mat(f"{kind}_carpet", "fabric", lv["carpet"])
    m["trim"] = _cabin_mat(f"{kind}_trim", "trim", (0.72, 0.71, 0.68))
    m["panel"] = _cabin_mat(f"{kind}_panel", "paint", (0.16, 0.17, 0.18))
    m["nav_red"] = K._mat(f"{kind}_nav_red", rgb=[0.6, 0.02, 0.02], emit=0xff2010, emit_k=6.0)
    m["nav_green"] = K._mat(f"{kind}_nav_green", rgb=[0.02, 0.5, 0.1], emit=0x20ff50, emit_k=6.0)
    m["nav_white"] = K._mat(f"{kind}_nav_white", rgb=[0.9, 0.9, 0.9], emit=0xffffff, emit_k=6.0)
    m["beacon"] = K._mat(f"{kind}_beacon", rgb=[0.6, 0.02, 0.02], emit=0xff1808, emit_k=8.0)
    m["strobe"] = K._mat(f"{kind}_strobe", rgb=[0.9, 0.9, 0.9], emit=0xffffff, emit_k=8.0)
    m["landing"] = K._mat(f"{kind}_landing", rgb=[0.9, 0.9, 0.85], emit=0xfff4e0, emit_k=5.0)
    for mat in m.values():
        mat.use_backface_culling = True
        b = mat.node_tree.nodes["Principled BSDF"]
        mix = [n for n in mat.node_tree.nodes if n.bl_idname == "ShaderNodeMix"]
        col = mix[0].inputs[7].default_value if mix else b.inputs["Base Color"].default_value
        mat.diffuse_color = (col[0], col[1], col[2], 1.0)  # the look sheet's flat colour only


def _cabin_mat(name, key, rgb):
    """A cabin surface: one of the furniture's swatch sets (tools/textures/crop_furniture_swatches.py) tinted."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    b = nt.nodes["Principled BSDF"]

    def img(suffix):
        path = os.path.join(CABIN_TEX, f"{key}{suffix}.png")
        im = bpy.data.images.get(f"cabin_{key}{suffix}") or bpy.data.images.load(path)
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


# ----------------------------------------------------------------------------------------------------- small geometry

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
    A.boolean(target, others, op, label)
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
    return A.obj(name, bm, mat, parent)


def copy_obj(o, name):
    c = o.copy()
    c.data = o.data.copy()
    c.name = name
    for coll in o.users_collection:
        coll.objects.link(c)
    return c


def slot(key):
    return ORDER.index(key)


def box_obj(name, lo, hi, mat, parent):
    """An axis-aligned glTF box from corner `lo` to `hi`."""
    bm = bmesh.new()
    c = [(a + b) * 0.5 for a, b in zip(lo, hi)]
    box_into(bm, c, [(b - a) * 0.5 for a, b in zip(lo, hi)])
    return mesh(name, bm, mat, parent)


def prism_obj(name, pts, mat, parent):
    return A.obj(name, hull(pts), mat, parent)


def move_to_pivot(o, at):
    """Move a finished object's vertices into the frame of a pivot at glTF `at` (its parent)."""
    o.data.transform(Matrix.Translation(-g2b(*at)))


# ------------------------------------------------------------------------------------------------------- the airframe

class Plane:
    """One aircraft's measurements, derived from its row in `KINDS`."""

    def __init__(self, kind):
        self.kind = kind
        k = self.k = KINDS[kind]
        self.L, self.S = k["length"], k["span"]
        self.hw, self.hh = k["width"] * 0.5, k["depth"] * 0.5
        self.z0 = -self.L * 0.5
        self.st = [(self.z0 + self.L * t, self.hw * w, self.hh * h, self.hh * c) for t, w, h, c in k["stations"]]

    # the fuselage
    def at(self, z):
        return A._station(self.st, z)

    def side(self, z, y):
        return skin_x(self.st, z, y, k=EXP)

    def crown(self, z, x=0.0):
        return A.skin_y(self.st, z, x, k=EXP)

    def belly(self, z):
        hw, hh, cy = self.at(z)
        return cy - hh

    def zs(self, lo=None, hi=None, extra=()):
        zs = sorted(set([s[0] for s in self.st] + list(extra)))
        return [z for z in zs if (lo is None or z >= lo - 1e-6) and (hi is None or z <= hi + 1e-6)]

    # the wing
    def wing_chord(self, x):
        w = self.k["wing"]
        if "chords" in w:
            pts = [(0.0, w["root"])] + sorted(w["chords"].items()) + [(self.S * 0.5, w["tip"])]
            for (x1, c1), (x2, c2) in zip(pts, pts[1:]):
                if x1 <= x <= x2:
                    return c1 + (c2 - c1) * (x - x1) / (x2 - x1)
            return w["tip"]
        return w["root"] + (w["tip"] - w["root"]) * x / (self.S * 0.5)

    def wing_le(self, x):
        w = self.k["wing"]
        z_root = self.z0 + w["le"]
        if "le_sweep" in w:
            return z_root + x * math.tan(math.radians(w["le_sweep"]))
        # the propliner: a nearly straight trailing edge, the leading edge swept back to meet the tip chord
        te = z_root + w["root"] + x * math.tan(math.radians(w["te_sweep"]))
        return te - self.wing_chord(x)

    def wing_y(self, x):
        w = self.k["wing"]
        return w["y"] + max(abs(x) - w["centre"], 0.0) * math.tan(math.radians(w["dihedral"]))

    def wing_t(self, x):
        w = self.k["wing"]
        return w["t_root"] + (w["t_tip"] - w["t_root"]) * abs(x) / (self.S * 0.5)


def fuselage_ring(p, z, n, inset=0.0, levels=()):
    """A fuselage ring at z: the superellipse, with extra points exactly at each height in `levels` on both sides, so
    the livery's lines run along ring edges."""
    hw, hh, cy = p.at(z)
    hw, hh = max(hw - inset, 0.01), max(hh - inset, 0.01)
    angles = [math.tau * i / n for i in range(n)]
    for y in levels:
        s = max(min((y - cy) / hh, 0.999), -0.999)
        # the angle (from the top, clockwise seen from the nose) where the superellipse is at height y
        a = math.acos(math.copysign(abs(s) ** (EXP / 2.0), s))
        angles += [a, math.tau - a]
    angles.sort()
    out = []
    for a in angles:
        c, s = math.sin(a), math.cos(a)
        x = hw * math.copysign(abs(c) ** (2 / EXP), c)
        y = hh * math.copysign(abs(s) ** (2 / EXP), s)
        out.append((x, cy + y, z))
    return out


def livery_levels(p):
    c = p.k["cabin"]
    wy = c["floor"] + c["window_y"]
    return [wy + c["window_h"] * 0.5 + 0.18, wy - c["window_h"] * 0.5 - 0.16, p.k["wing"]["y"] + 0.15]


def paint_fuselage(o, p):
    """Crown, cheat line and belly on the outer skin by height (the ring has edges exactly on these lines). The cheat
    line and the belly stop where the nose and the tail cone narrow past them, so neither runs out as a zig-zag."""
    top, bot, bel = livery_levels(p)
    for f in o.data.polygons:
        c = f.center  # Blender frame: glTF y is Blender z, glTF z is -Blender y
        y, z = c.z, -c.y
        hw, hh, cy = p.at(z)
        band = abs(top - cy) < 0.9 * hh and abs(bot - cy) < 0.9 * hh
        belly = abs(bel - cy) < 0.9 * hh
        if band and bot - 1e-3 < y < top + 1e-3:
            f.material_index = slot("line")
        elif belly and y < bel - 1e-3:
            f.material_index = slot("belly")
        else:
            f.material_index = slot("crown")


def outer_loft(p, n, name, parent, levels):
    rings = [fuselage_ring(p, z, n, 0.0, levels) for z in p.zs()]
    return A.obj(name, loft(rings), "crown", parent)


def inner_loft(p, n, name, parent):
    """The cabin's cavity: from just behind the nose cap to the rear pressure bulkhead, a wall's thickness in."""
    k = p.k
    lo = p.z0 + p.L * k["stations"][2][0]
    hi = p.z0 + p.L * 0.94
    zs = [lo] + [z for z in p.zs(lo, hi) if lo + 0.05 < z < hi - 0.05] + [hi]
    rings = [fuselage_ring(p, z, n, k["wall"], ()) for z in zs]
    return A.obj(name, loft(rings), "cabin_wall", parent)


# ------------------------------------------------------------------------------------------- lifting surfaces, cuts

def wing_rings(p, side, n):
    w = p.k["wing"]
    xs = [0.0] + list(w["stations"]) + [p.S * 0.5]
    rings = []
    for x in xs:
        rings.append(A.airfoil(x, p.wing_le(x), p.wing_chord(x), p.wing_y(x), p.wing_t(x), n, side=side))
    return rings


def wing_surface_box(p, x_a, x_b, share, side, behind=True, grow=0.0, gap=0.0):
    """A cutter behind (or, with behind False, nothing but) the hinge line at `share` of the chord between span x_a
    and x_b on one side: a prism following the wing's sweep and dihedral."""
    pts = []
    for x in (x_a, x_b):
        le, ch, y = p.wing_le(x), p.wing_chord(x), p.wing_y(x)
        zh = le + ch * share + gap
        for yy in (y - 1.2, y + 1.2):
            pts += [(side * x, yy, zh), (side * x, yy, le + ch + 1.0 + grow)]
    return pts


def hinge_of(p, x_a, x_b, share, side):
    a = Vector((side * x_a, p.wing_y(x_a), p.wing_le(x_a) + p.wing_chord(x_a) * share))
    b = Vector((side * x_b, p.wing_y(x_b), p.wing_le(x_b) + p.wing_chord(x_b) * share))
    return a, b


# ----------------------------------------------------------------------------------------------------- the model

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


def split_by_z(m, src, lod, name_of, ranges, cap="torn"):
    """Cut `src` (a finished, clean object) into the fuselage zones: one copy intersected with a box per z range.
    Each cut face takes `cap`. Returns {zone: object}; `src` is removed."""
    out = {}
    big = 40.0
    for zone, (za, zb) in ranges.items():
        piece = copy_obj(src, f"{name_of}_{zone}.L{lod}")
        piece.parent = m.zones[zone]
        cutter = box_obj(f"cut_{name_of}_{zone}_{lod}", (-big, -big, za + GAP), (big, big, zb - GAP), cap, m.zones[zone])
        boolean(piece, [cutter], "INTERSECT", "zone")
        if len(piece.data.polygons) == 0:
            bpy.data.objects.remove(piece)
            continue
        out[zone] = piece
    bpy.data.objects.remove(src)
    return out


def split_by_x(m, src, lod, name_of, ranges, cap="torn"):
    """As `split_by_z`, across the span: `ranges` {zone: (x_a, x_b)} in glTF x."""
    out = {}
    big = 80.0
    for zone, (xa, xb) in ranges.items():
        piece = copy_obj(src, f"{name_of}_{zone}.L{lod}")
        piece.parent = m.zones[zone]
        cutter = box_obj(f"cut_{name_of}_{zone}_{lod}", (xa + GAP, -big, -big), (xb - GAP, big, big), cap, m.zones[zone])
        boolean(piece, [cutter], "INTERSECT", "zone")
        if len(piece.data.polygons) == 0:
            bpy.data.objects.remove(piece)
            continue
        out[zone] = piece
    bpy.data.objects.remove(src)
    return out


def multi_box_obj(name, boxes, mat, parent):
    """One object holding many boxes (each (lo, hi) in glTF), so a boolean takes them as one operand."""
    bm = bmesh.new()
    for lo, hi in boxes:
        box_into(bm, [(a + b) * 0.5 for a, b in zip(lo, hi)], [(b - a) * 0.5 for a, b in zip(lo, hi)])
    return mesh(name, bm, mat, parent)


def snap_between_rows(p, z):
    """Move a zone boundary out of any seat row into the gap after the row it falls in."""
    c = p.k["cabin"]
    rows = [c["first"] + i * c["pitch"] for i in range(c["rows"])]
    for i, r in enumerate(rows):
        if r - 0.05 <= z <= r + SEAT_DEPTH + 0.02:
            nxt = rows[i + 1] if i + 1 < len(rows) else r + SEAT_DEPTH + 0.4
            return (r + SEAT_DEPTH + nxt) * 0.5
    return z


SEAT_DEPTH = 0.64


def fuselage_zones(p):
    """{zone: (z_a, z_b)} along the fuselage, the boundaries between seat rows."""
    zk = p.k["zones"]
    big = p.L
    if "fwd_end" in zk:
        cuts = [zk["nose"], snap_between_rows(p, zk["fwd_end"]), snap_between_rows(p, zk["centre_end"]), zk["aft_end"]]
        names = ["nose", "cabin_fwd", "centre", "cabin_aft", "tail"]
    else:
        cuts = [zk["nose"], snap_between_rows(p, zk["centre_end"]), zk["aft_end"]]
        names = ["nose", "centre", "cabin_aft", "tail"]
    edges = [-big] + cuts + [big]
    return {n: (a, b) for n, a, b in zip(names, edges, edges[1:])}


def make_zones(m, p):
    """The part tree (design section 5)."""
    m.zone("centre")
    fz = fuselage_zones(p)
    if "cabin_fwd" in fz:
        m.zone("cabin_fwd", "centre")
        m.zone("nose", "cabin_fwd")
    else:
        m.zone("nose", "centre")
    m.zone("cabin_aft", "centre")
    m.zone("tail", "cabin_aft")
    for s in ("l", "r"):
        m.zone(f"wing_inner_{s}", "centre")
        m.zone(f"wing_outer_{s}", f"wing_inner_{s}")
    for n, (e, side) in engine_list(p).items():
        parent = e["zone"] if e["zone"] == "tail" else f"{e['zone']}_{side}"
        m.zone(f"engine_{n}", parent)
    return fz


def engine_list(p):
    """The engines numbered as on the real aircraft: from the left wing tip to the right; the tail engine between."""
    wing = sorted(e["x"] for e in p.k["engines"] if e["kind"] != "tail")
    tail = [e for e in p.k["engines"] if e["kind"] == "tail"]
    out = []
    for x in reversed(wing):
        out.append((dict(next(e for e in p.k["engines"] if e.get("x") == x), side_x=-x), "l"))
    if tail:
        out.append((dict(tail[0], side_x=0.0), "c"))
    for x in wing:
        out.append((dict(next(e for e in p.k["engines"] if e.get("x") == x), side_x=x), "r"))
    return {i + 1: v for i, v in enumerate(out)}


def zone_of_z(fz, z):
    for n, (a, b) in fz.items():
        if a <= z < b:
            return n
    return "centre"


def fus_side(p):
    """Where the wing's root zone ends: just inside the fuselage's skin at the wing."""
    w = p.k["wing"]
    z = p.wing_le(0.0) + w["root"] * 0.5
    return p.side(z, p.wing_y(0.0)) - p.k["wall"] * 0.5


# ------------------------------------------------------------------------------------------------------ the fuselage

def deck_eye(p):
    """The pilots' eye: 1.35 m over the floor (seated, 1.2 m, plus the cushion), at the first station along the nose with
    0.65 m of headroom over it (less in a fuselage too low for that, as the DC-3's), half a metre further back.
    Returns (z, y)."""
    y = p.k["cabin"]["floor"] + 1.35
    top = max(cy + hh for _, hw, hh, cy in p.st)
    need = min(y + 0.65, top - 0.12)
    z = p.z0
    while p.crown(z) < need and z < 0.0:
        z += 0.05
    return z + 0.5, y


def nose_surface_z(p, x, y):
    """Where the nose's skin is at height y and across x, marching back from the tip."""
    z = p.z0
    while z < 0.0:
        hw, hh, cy = p.at(z)
        if abs(x) < hw and abs(y - cy) < hh and (abs(x) / hw) ** EXP + (abs(y - cy) / hh) ** EXP < 1.0:
            return z
        z += 0.02
    return z


def windscreen_boxes(p, depth_in, grow=0.0):
    """The flight deck's window cutters, in front of and beside the pilots' eyes as a real windscreen is: the front
    panes cut back through the sloping nose in a band from just under the eye to half a metre over it, the side panes
    across beside the seats. `depth_in` is how far behind the skin a cutter reaches (through the wall, or a recess)."""
    z_eye, y_eye = deck_eye(p)
    y0, y1 = y_eye - 0.12, min(y_eye + 0.48, p.crown(z_eye) - 0.15)
    out = []
    n_front = 2 if p.k["windscreen"]["panes"] <= 4 else 4
    hw_front = p.at(z_eye)[0] * 0.5
    pane_w = 2 * hw_front / n_front - 0.1
    for i in range(n_front):
        x = -hw_front + (i + 0.5) * 2 * hw_front / n_front
        zs = max(nose_surface_z(p, x + d, y) for d in (-pane_w / 2, pane_w / 2) for y in (y0, y1))
        out.append(((x - pane_w / 2 - grow, y0 - grow, zs - 2.0), (x + pane_w / 2 + grow, y1 + grow, zs + depth_in + grow)))
    za, zb = z_eye - 0.75, z_eye + 0.25
    for sd in (-1, 1):
        xo = min(p.side(z, (y0 + y1) * 0.5) for z in (za, zb))
        # never inboard of the front panes: one cutter object must not overlap itself
        xi = max(xo - depth_in, hw_front + 0.08)
        lo = (min(sd * xi, sd * (xo + 0.8)) - grow, y0 - grow, za - grow)
        hi = (max(sd * xi, sd * (xo + 0.8)) + grow, y1 + grow, zb + grow)
        out.append((lo, hi))
    return out


def window_boxes(p, depth_in, grow=0.0):
    """One cabin window per seat row each side, its middle beside the passenger's head."""
    c = p.k["cabin"]
    y = c["floor"] + c["window_y"]
    out = []
    for i in range(c["rows"]):
        z = c["first"] + i * c["pitch"] + 0.18
        xo = p.side(z, y)
        for s in (-1, 1):
            a, b = s * (xo - depth_in), s * (xo + 0.3)
            out.append(((min(a, b) - grow, y - c["window_h"] / 2 - grow, z - c["window_w"] / 2 - grow),
                        (max(a, b) + grow, y + c["window_h"] / 2 + grow, z + c["window_w"] / 2 + grow)))
    return out


def window_band(p, depth_in):
    """The far levels' windows: one recessed strip along each side."""
    c = p.k["cabin"]
    y = c["floor"] + c["window_y"]
    za, zb = c["first"], c["first"] + (c["rows"] - 1) * c["pitch"] + 0.4
    xo = min(p.side(z, y) for z in (za, zb, (za + zb) / 2))
    out = []
    for s in (-1, 1):
        a, b = s * (xo - depth_in), s * (xo + 0.6)
        out.append(((min(a, b), y - c["window_h"] / 2, za), (max(a, b), y + c["window_h"] / 2, zb)))
    return out


def nose_well(p):
    g = p.k["gear"]
    zn = p.z0 + g["nose_z"]
    b = p.belly(zn)
    lo, hi = zn - g["nose_leg"] - g["nose_r"] - 0.2, zn + 0.45
    return (-0.45, b - 0.6, lo), (0.45, b + 1.0, hi)


def build_fuselage(m, p, lod, fz):
    det = DETAIL[lod]
    n = det["round"]
    tmp = m.root
    shell = outer_loft(p, n, f"fuselage_{lod}", tmp, livery_levels(p))
    paint_fuselage(shell, p)
    doors = []
    if det["hollow"]:
        boolean(shell, [inner_loft(p, n, f"cabin_{lod}", tmp)], "DIFFERENCE", "cabin", hollowing=True)
        cutters = windscreen_boxes(p, p.k["wall"] + 0.3) + window_boxes(p, p.k["wall"] + 0.25)
        glass_src = copy_obj(shell, f"glass_src_{lod}")
        panes = windscreen_boxes(p, p.k["wall"] + 0.3, grow=-0.012) + window_boxes(p, p.k["wall"] + 0.25, grow=-0.012)
        boolean(glass_src, [multi_box_obj(f"glass_cut_{lod}", panes, "glass", tmp)], "INTERSECT", "glass")
        for f in glass_src.data.polygons:
            f.material_index = slot("glass")
        if p.k["gear"]["kind"] == "tricycle":
            lo, hi = nose_well(p)
            for s, name in ((-1, "l"), (1, "r")):
                door = copy_obj(shell, f"door_nose_{name}_{lod}")
                dlo = (min(0.0, s * hi[0]) + (0.006 if s > 0 else 0.0), lo[1], lo[2] + 0.01)
                dhi = (max(0.0, s * hi[0]) - (0.006 if s < 0 else 0.0), hi[1], hi[2] - 0.01)
                dlo = (dlo[0] + 0.004, dlo[1], dlo[2])
                dhi = (dhi[0] - 0.004, dhi[1], dhi[2])
                boolean(door, [box_obj(f"door_cut_{name}_{lod}", dlo, dhi, "dark", tmp)], "INTERSECT", "door")
                doors.append((name, door))
            cutters.append((lo, hi))
        boolean(shell, [multi_box_obj(f"window_cut_{lod}", cutters, "trim", tmp)], "DIFFERENCE", "windows and wells")
    else:
        cut = windscreen_boxes(p, 0.05) + window_band(p, 0.05)
        boolean(shell, [multi_box_obj(f"window_cut_{lod}", cut, "glass", tmp)], "DIFFERENCE", "windows")
        glass_src = None
    clean(shell)
    pieces = split_by_z(m, shell, lod, "fuselage", fz)
    for o in pieces.values():
        clean(o)
    if glass_src is not None:
        clean(glass_src)
        for o in split_by_z(m, glass_src, lod, "glass", fz).values():
            clean(o)
    for name, door in doors:
        clean(door)
        g = p.k["gear"]
        zn = p.z0 + g["nose_z"]
        lo, hi = nose_well(p)
        s = -1 if name == "l" else 1
        hinge = (s * hi[0], p.belly(zn) + 0.05, (lo[2] + hi[2]) * 0.5)
        piv = m.pivot(f"door_nose_{name}", hinge, zone_of_z(fz, zn), (0, 0, 1), angle_deg=85.0 * s,
                      keys=[[0.0, 1.0], [0.8, 1.0], [1.0, 0.0]], role="gear_door")
        door.name = f"door_nose_{name}.L{lod}"
        door.parent = piv
        move_to_pivot(door, hinge)


# ------------------------------------------------------------------------------------------------------------ wings

def wing_well(p, side):
    """The main gear's pocket in the wing root's underside (jets): where the bogie lies when it is up."""
    g = p.k["gear"]
    zg = main_z(p)
    x0, x1 = 0.45, g["main_x"] + 0.35
    zlo, zhi = zg - g["bogie"] * 0.5 - g["main_r"] - 0.2, zg + g["bogie"] * 0.5 + g["main_r"] + 0.2
    # under the wing's upper skin everywhere along the pocket, so it never shows from above
    top = min(p.wing_y(x) + p.wing_t(x) * p.wing_chord(x) * 0.3 for x in (x0, x1)) - 0.08
    lo = (min(side * x0, side * x1), -20.0, zlo)
    hi = (max(side * x0, side * x1), top, zhi)
    return lo, hi


def clear_of(w, x):
    """A cut's span station moved at least 8 cm off any of the wing's own stations: an Exact boolean whose cutter face
    passes through a ring of the loft can fail and take the wing with it."""
    for st in w["stations"]:
        if abs(x - st) < 0.08:
            return st + (0.08 if x >= st else -0.08)
    return x


def main_z(p):
    g = p.k["gear"]
    # half way along the chord, or further forward if the pocket would reach the flap's hinge line (72 % of the chord)
    # where the flap begins: an Exact boolean whose cutters overlap can take the whole wing with it
    half = g["bogie"] * 0.5 + g["main_r"] + 0.2
    xf = fus_side(p) + 0.35
    hinge = p.wing_le(xf) + p.wing_chord(xf) * 0.72
    return min(p.wing_le(g["main_x"]) + p.wing_chord(g["main_x"]) * 0.5, hinge - 0.25 - half)


def build_wings(m, p, lod):
    det = DETAIL[lod]
    w = p.k["wing"]
    root_side = fus_side(p)
    tip = p.S * 0.5
    for side, s in ((-1.0, "l"), (1.0, "r")):
        wing = A.obj(f"wing_{s}_{lod}", loft(wing_rings(p, side, det["foil"])), "wing", m.root)
        if lod == 0 and p.k["gear"]["kind"] == "tricycle":
            lo, hi = wing_well(p, side)
            door = copy_obj(wing, f"door_main_{s}_{lod}")
            # the wing's lower skin under the pocket (an airfoil's lower surface is 0.375 t c under its chord line)
            lower = [p.wing_y(x) - 0.375 * p.wing_t(x) * p.wing_chord(x) for x in (0.45, p.k["gear"]["main_x"] + 0.35)]
            wb = min(lower)
            dlo = (lo[0] + 0.01, -20.0, lo[2] + 0.01)
            dhi = (hi[0] - 0.01, max(lower) + 0.12, hi[2] - 0.01)
            boolean(door, [box_obj(f"door_cut_{s}_{lod}", dlo, dhi, "dark", m.root)], "INTERSECT", "door")
            boolean(wing, [box_obj(f"well_{s}_{lod}", lo, hi, "dark", m.root)], "DIFFERENCE", "well")
            clean(door)
            inner_x = 0.45
            hinge = (side * inner_x, wb + 0.04, (lo[2] + hi[2]) * 0.5)
            piv = m.pivot(f"door_main_{s}", hinge, "centre", (0, 0, 1), angle_deg=-side * 85.0,
                          keys=[[0.0, 1.0], [0.8, 1.0], [1.0, 0.0]], role="gear_door")
            door.name = f"door_main_{s}.L{lod}"
            door.parent = piv
            move_to_pivot(door, hinge)
        surfaces = []
        if lod < 2:
            # the aileron outboard of the split, the flap inboard of it, each cut on its hinge line
            for what, xa, xb, share in (("aileron", clear_of(w, w["split"] + 0.3), clear_of(w, tip - 0.8), 0.74),
                                        ("flap", clear_of(w, root_side + 0.35), clear_of(w, w["split"] - 0.3), 0.72)):
                piece = copy_obj(wing, f"{what}_{s}.L{lod}")
                boolean(piece, [prism_obj(f"{what}_cut_{s}", wing_surface_box(p, xa, xb, share, side, gap=0.03), "wing",
                                          m.root)], "INTERSECT", what)
                boolean(wing, [prism_obj(f"{what}_hole_{s}", wing_surface_box(p, xa, xb, share, side), "wing", m.root)],
                        "DIFFERENCE", what)
                a, b = hinge_of(p, xa, xb, share, side)
                zone = f"wing_outer_{s}" if what == "aileron" else f"wing_inner_{s}"
                mid = tuple((a + b) * 0.5)
                axis = tuple((b - a).normalized() * side)
                travel = dict(range_deg=20.0, role="aileron") if what == "aileron" else dict(detents_deg=[0.0, 15.0, 30.0], role="flap")
                piv = m.pivot(f"{what}_{s}", mid, zone, axis, **travel)
                surfaces.append((piece, piv, mid))
        clean(wing)
        ranges = {"centre": (-root_side, root_side) if side > 0 else (-root_side, root_side),
                  f"wing_inner_{s}": (root_side, w["split"]) if side > 0 else (-w["split"], -root_side),
                  f"wing_outer_{s}": (w["split"], tip + 2) if side > 0 else (-tip - 2, -w["split"])}
        for zone, o in split_by_x(m, wing, lod, f"wing_{s}", ranges).items():
            o.name = f"wing_{s}_{zone.split('_')[1] if zone != 'centre' else 'root'}.L{lod}"
            clean(o)
        for piece, piv, mid in surfaces:
            clean(piece)
            piece.parent = piv
            move_to_pivot(piece, mid)


# ------------------------------------------------------------------------------------------------------------- tail

def fin_geom(p):
    f = p.k["fin"]
    le_root = p.L * 0.5 + f["le"]
    z_mid = le_root + f["root"] * 0.5
    h_root = p.crown(z_mid) - 0.25 + f.get("base", 0.0)
    return le_root, h_root


def fin_le(p, h):
    f = p.k["fin"]
    le_root, h_root = fin_geom(p)
    return le_root + (h - h_root) * math.tan(math.radians(f["sweep"]))


def fin_chord(p, h):
    f = p.k["fin"]
    le_root, h_root = fin_geom(p)
    t = (h - h_root) / (f["top"] - h_root)
    return f["root"] + (f["tip"] - f["root"]) * t


def tp_geom(p):
    t = p.k["tailplane"]
    le = p.L * 0.5 + t["le"]
    y = p.at(le + t["root"] * 0.5)[2] + t["y"]
    return le, y


def tp_le(p, x):
    t = p.k["tailplane"]
    le, y = tp_geom(p)
    return le + abs(x) * math.tan(math.radians(t["sweep"]))


def tp_chord(p, x):
    t = p.k["tailplane"]
    return t["root"] + (t["tip"] - t["root"]) * abs(x) / (t["span"] * 0.5)


def tp_y(p, x):
    t = p.k["tailplane"]
    le, y = tp_geom(p)
    return y + abs(x) * math.tan(math.radians(t["dihedral"]))


def build_tail(m, p, lod):
    det = DETAIL[lod]
    f, t = p.k["fin"], p.k["tailplane"]
    zone = m.zones["tail"]
    le_root, h_root = fin_geom(p)
    root = (h_root, le_root, f["root"], 0.0, f["t"])
    tip = (f["top"], fin_le(p, f["top"]), f["tip"], 0.0, f["t"] * 0.8)
    fin = A.lifting(f"fin.L{lod}", root, tip, det["foil"], "fin", zone, vertical=True)
    if lod < 2:
        ha, hb = h_root + 0.35 + (p.crown(le_root + f["root"]) - p.crown(le_root)) * 0.0 + 0.4, f["top"] - 0.3
        share = 0.68

        def prism(gap):
            pts = []
            for h in (ha, hb):
                le, ch = fin_le(p, h), fin_chord(p, h)
                for x in (-1.0, 1.0):
                    pts += [(x, h, le + ch * share + gap), (x, h, le + ch + 1.0)]
            return pts

        rudder = copy_obj(fin, f"rudder.L{lod}")
        boolean(rudder, [prism_obj("rudder_cut", prism(0.03), "fin", zone)], "INTERSECT", "rudder")
        boolean(fin, [prism_obj("rudder_hole", prism(0.0), "fin", zone)], "DIFFERENCE", "rudder")
        a = Vector((0.0, ha, fin_le(p, ha) + fin_chord(p, ha) * share))
        b = Vector((0.0, hb, fin_le(p, hb) + fin_chord(p, hb) * share))
        mid = tuple((a + b) * 0.5)
        piv = m.pivot("rudder", mid, "tail", tuple((b - a).normalized()), range_deg=25.0, role="rudder")
        clean(rudder)
        rudder.parent = piv
        move_to_pivot(rudder, mid)
    clean(fin)
    tle, ty = tp_geom(p)
    half = t["span"] * 0.5
    for side, s in ((-1.0, "l"), (1.0, "r")):
        root = (0.0, tle, t["root"], ty, t["t"])
        tipr = (half, tp_le(p, half), t["tip"], tp_y(p, half), t["t"] * 0.8)
        tp = A.lifting(f"tailplane_{s}.L{lod}", root, tipr, det["foil"], "wing", zone, side=side)
        if lod < 2:
            xa = p.at(tle + t["root"] * 0.7)[0] + 0.15
            xb = half - 0.25
            share = 0.7

            def prism(gap):
                pts = []
                for x in (xa, xb):
                    le, ch, y = tp_le(p, x), tp_chord(p, x), tp_y(p, x)
                    for yy in (y - 1.0, y + 1.0):
                        pts += [(side * x, yy, le + ch * share + gap), (side * x, yy, le + ch + 1.0)]
                return pts

            el = copy_obj(tp, f"elevator_{s}.L{lod}")
            boolean(el, [prism_obj("elevator_cut", prism(0.03), "wing", zone)], "INTERSECT", "elevator")
            boolean(tp, [prism_obj("elevator_hole", prism(0.0), "wing", zone)], "DIFFERENCE", "elevator")
            a = Vector((side * xa, tp_y(p, xa), tp_le(p, xa) + tp_chord(p, xa) * share))
            b = Vector((side * xb, tp_y(p, xb), tp_le(p, xb) + tp_chord(p, xb) * share))
            mid = tuple((a + b) * 0.5)
            piv = m.pivot(f"elevator_{s}", mid, "tail", tuple((b - a).normalized() * side), range_deg=20.0,
                          role="elevator")
            clean(el)
            el.parent = piv
            move_to_pivot(el, mid)
        clean(tp)


# ---------------------------------------------------------------------------------------------------------- engines

def build_turbofan(m, p, lod, n, x):
    det = DETAIL[lod]
    e = p.k["turbofan"]
    zone = m.zones[f"engine_{n}"]
    ax = abs(x)
    le, wy = p.wing_le(ax), p.wing_y(ax)
    zf, cy = le - e["ahead"], wy - e["drop"]
    rr, ll = e["r"] / 1.28, e["length"] / 6.6
    prof = [(0.0, 1.02), (0.25, 1.2), (1.2, 1.28), (3.6, 1.24), (5.4, 1.06), (6.6, 0.82)]
    if det["nacelle"] <= 8:
        prof = [(0.0, 1.1), (1.2, 1.28), (5.0, 1.1), (6.6, 0.82)]
    pod = A.obj(f"nacelle_{n}.L{lod}", lathe([(a * ll, r * rr) for a, r in prof], det["nacelle"], (x, cy, zf)), "nacelle", zone)
    depth = 0.9 * ll
    if lod < 2:
        boolean(pod, [A.obj("intake", lathe([(-0.4, e["intake"]), (depth, e["intake"])], det["nacelle"], (x, cy, zf)), "dark",
                            zone)], "DIFFERENCE", "intake")
    pts = []
    for zz in (zf + 1.0 * ll, zf + 5.6 * ll):
        for dx in (-0.2, 0.2):
            pts.append((x + dx, cy + e["r"] * 0.6, zz))
    for zz in (le + 0.4, le + p.wing_chord(ax) * 0.7):
        for dx in (-0.18, 0.18):
            pts.append((x + dx, wy + 0.15, zz))
    boolean(pod, [prism_obj("pylon", pts, "nacelle", zone)], "UNION", "pylon")
    clean(pod)
    cone = A.obj(f"exhaust_{n}.L{lod}", lathe([(6.4 * ll, 0.62 * rr), (7.5 * ll, 0.05)], max(det["nacelle"] // 2, 6), (x, cy, zf)),
                 "dark", zone)
    clean(cone)
    fan_at = (x, cy, zf + depth - 0.06)
    fan(m, p, lod, f"fan_{n}", fan_at, e["intake"] - 0.04, f"engine_{n}")


def fan(m, p, lod, name, at, r, zone):
    """A fan face on its own pivot: a disc, a spinner and (near) its blades."""
    det = DETAIL[lod]
    piv = m.pivot(name, at, zone, (0, 0, 1), rpm=3000.0, role="fan")
    x, y, z = at
    bm = lathe([(-0.03, r), (0.03, r)], det["nacelle"], (x, y, z))
    o = mesh(f"{name}_disc.L{lod}", bm, "dark", piv, at=at)
    clean(o)
    sp = lathe([(-0.55 * r, 0.02), (-0.3 * r, 0.2 * r), (-0.05, 0.28 * r), (0.0, 0.28 * r)], max(det["nacelle"] // 2, 6), (x, y, z))
    o = mesh(f"{name}_spinner.L{lod}", sp, "metal", piv, at=at)
    clean(o)
    if lod == 0:
        bm = bmesh.new()
        for i in range(20):
            a = math.tau * i / 20
            d = Vector((math.cos(a), math.sin(a), 0.0))
            u = Vector((-math.sin(a), math.cos(a), 0.0))
            for rr0, rr1 in ((0.3 * r, 0.95 * r),):
                c = Vector(at) + d * ((rr0 + rr1) * 0.5) + Vector((0, 0, -0.08))
                tw = 0.6
                box_into(bm, c, ((rr1 - rr0) * 0.5, 0.012, 0.09),
                         (d, u * math.cos(tw) + Vector((0, 0, 1)) * math.sin(tw), -u * math.sin(tw) + Vector((0, 0, 1)) * math.cos(tw)))
        o = mesh(f"{name}_blades.L{lod}", bm, "metal", piv, at=at)
        clean(o)


def build_tail_engine(m, p, lod, n):
    """The wide-body's centre engine: an intake hump on the crown ahead of the fin, an S-duct to the engine in the tail
    cone, its nozzle at the very tail."""
    det = DETAIL[lod]
    zone = m.zones[f"engine_{n}"]
    tail = m.zones["tail"]
    f = p.k["fin"]
    le_root, h_root = fin_geom(p)
    za, zb = le_root - 4.0, le_root + f["root"] * 0.9
    rings = []
    seg = max(det["round"] // 2, 8)
    for t, k in ((0.0, 0.8), (0.08, 1.0), (0.6, 1.0), (0.85, 0.8), (1.0, 0.45)):
        z = za + (zb - za) * t
        cy = p.crown(z) + 0.45 * k
        ring = []
        for i in range(seg):
            a = math.tau * i / seg
            ring.append((math.sin(a) * 1.0 * k, cy + math.cos(a) * 0.85 * k, z))
        rings.append(ring)
    hump = A.obj(f"intake_hump.L{lod}", loft(rings), "crown", tail)
    if lod < 2:
        cy0 = p.crown(za) + 0.45 * 0.8
        cut = lathe([(-0.5, 0.62), (1.6, 0.62)], seg, (0.0, cy0 + 0.05, za))
        boolean(hump, [A.obj("hump_intake", cut, "dark", tail)], "DIFFERENCE", "intake")
    clean(hump)
    z_end = p.L * 0.5
    hw, hh, cy = p.at(z_end - 0.01)
    noz = A.obj(f"nozzle_{n}.L{lod}", lathe([(-1.2, hw * 1.02), (0.0, hw * 1.02), (0.5, hw * 0.92)], det["nacelle"],
                                            (0.0, cy, z_end - 0.4)), "nacelle", zone)
    if lod < 2:
        boolean(noz, [A.obj("nozzle_hole", lathe([(-0.2, hw * 0.7), (1.0, hw * 0.7)], det["nacelle"], (0.0, cy, z_end - 0.4)),
                            "dark", zone)], "DIFFERENCE", "nozzle")
    clean(noz)
    cy0 = p.crown(za) + 0.45 * 0.8
    fan(m, p, lod, f"fan_{n}", (0.0, cy0 + 0.05, za + 1.5), 0.58, f"engine_{n}")


def build_radial(m, p, lod, n, x):
    """A radial engine's nacelle (cowl ahead of the wing, a fairing tapering behind it that holds the main gear) and
    its three-blade propeller."""
    det = DETAIL[lod]
    e = p.k["radial"]
    zone = m.zones[f"engine_{n}"]
    ax = abs(x)
    le, wy, ch = p.wing_le(ax), p.wing_y(ax), p.wing_chord(ax)
    r = e["r"]
    cy = wy + 0.05
    zf = le - e["ahead"]
    prof = [(0.0, 0.9 * r), (0.12, r), (1.0, r), (e["ahead"], 0.96 * r), (e["ahead"] + ch * 0.45, 0.8 * r),
            (e["ahead"] + ch * e["behind"], 0.12 * r)]
    if det["nacelle"] <= 8:
        prof = [prof[0], prof[2], prof[4], prof[5]]
    pod = A.obj(f"nacelle_{n}.L{lod}", lathe(prof, det["nacelle"], (x, cy, zf)), "nacelle", zone)
    if lod < 2:
        boolean(pod, [A.obj("cowl", lathe([(-0.3, 0.66 * r), (0.4, 0.66 * r)], det["nacelle"], (x, cy, zf)), "dark", zone)],
                "DIFFERENCE", "cowl")
    if lod == 0:
        g = p.k["gear"]
        pz = le + 0.35
        # just the stowed wheel's slot in the nacelle's underside: the wheel rides half out, as the DC-3's does
        zw = pz - g["leg"]
        lo = (x - g["main_w"] * 0.5 - 0.06, cy - r - 0.5, zw - g["main_r"] - 0.08)
        hi = (x + g["main_w"] * 0.5 + 0.06, cy + 0.15, zw + g["main_r"] + 0.08)
        boolean(pod, [box_obj("well", lo, hi, "dark", zone)], "DIFFERENCE", "well")
    clean(pod)
    if lod == 0:
        bm = bmesh.new()
        for i in range(9):
            a = math.tau * (i + 0.5) / 9
            c = (x + math.cos(a) * 0.45 * r, cy + math.sin(a) * 0.45 * r, zf + 0.42)
            cyl_into(bm, (c[0], c[1], c[2] - 0.02), (c[0], c[1], c[2] + 0.18), 0.1, 8)
        o = mesh(f"cylinders_{n}.L0", bm, "metal", zone)
        clean(o)
    hub = (x, cy, zf - 0.32)
    piv = m.pivot(f"prop_{n}", hub, f"engine_{n}", (0, 0, 1), rpm=1200.0, role="prop")
    sp = lathe([(-0.55, 0.02), (-0.4, 0.2), (-0.1, 0.32), (0.3, 0.34)], max(det["nacelle"] // 2, 6), hub)
    o = mesh(f"spinner_{n}.L{lod}", sp, "metal", piv, at=hub)
    clean(o)
    bm = bmesh.new()
    for b in range(e["blades"]):
        a = math.tau * b / e["blades"] - math.pi / 2  # one blade straight down, as the drawings park it
        d = Vector((math.cos(a), math.sin(a), 0.0))
        u = Vector((-math.sin(a), math.cos(a), 0.0))
        secs = [(0.2, 0.2, 0.8), (0.9, 0.3, 0.45), (e["prop_r"], 0.16, 0.25)]
        rings = []
        for rad, chord, twist in secs:
            c = Vector(hub) + d * rad
            cu = u * math.cos(twist) + Vector((0, 0, 1)) * math.sin(twist)
            cn = -u * math.sin(twist) + Vector((0, 0, 1)) * math.cos(twist)
            th = chord * 0.12
            ring = [c + cu * chord * 0.5, c + cn * th, c - cu * chord * 0.5, c - cn * th * 0.6]
            rings.append([bm.verts.new(g2b(*q)) for q in ring])
        for ra, rb in zip(rings, rings[1:]):
            for i in range(4):
                j = (i + 1) % 4
                bm.faces.new((ra[i], ra[j], rb[j], rb[i]))
        bm.faces.new(list(reversed(rings[0])))
        bm.faces.new(rings[-1])
    o = mesh(f"blades_{n}.L{lod}", bm, "dark", piv, at=hub)
    clean(o)
    return cy, le


# ------------------------------------------------------------------------------------------------------------- gear

def rest_of(p):
    """The origin's height over the ground when the jet stands on its gear."""
    g = p.k["gear"]
    return -(p.wing_y(g["main_x"]) + 0.1 - g["leg"] - g["main_r"])


def wheel_pair(bm_t, bm_h, c, r, w, n, hub=True):
    """Two wheels on an axle along x through `c`."""
    for dx in (-(w * 0.5 + 0.08), w * 0.5 + 0.08):
        at = (c[0] + dx, c[1], c[2])
        prof = [(-w * 0.5, r * 0.62), (-w * 0.5, r * 0.88), (-w * 0.42, r), (w * 0.42, r), (w * 0.5, r * 0.88), (w * 0.5, r * 0.62)]
        t = lathe(prof, n, at, axis="x")
        t.to_mesh(TMP_ME)
        t.free()
        bm_t.from_mesh(TMP_ME)
        if hub:
            h = lathe([(-w * 0.46, r * 0.64), (w * 0.46, r * 0.64)], max(n // 2, 6), at, axis="x")
            h.to_mesh(TMP_ME)
            h.free()
            bm_h.from_mesh(TMP_ME)


TMP_ME = None


def build_gear(m, p, lod, fz):
    global TMP_ME
    TMP_ME = bpy.data.meshes.get("tmp_wheels") or bpy.data.meshes.new("tmp_wheels")
    det = DETAIL[lod]
    g = p.k["gear"]
    n = det["wheel"]
    up_keys = [[0.0, 0.0], [0.12, 0.0], [0.8, 1.0], [1.0, 1.0]]
    if g["kind"] == "tricycle":
        rest = rest_of(p)
        ground = -rest
        zg = main_z(p)
        for side, s in ((-1.0, "l"), (1.0, "r")):
            P = (side * g["main_x"], p.wing_y(g["main_x"]) + 0.1, zg)
            piv = m.pivot(f"gear_main_{s}", P, "centre", (0, 0, 1), angle_deg=-side * 90.0, keys=up_keys, role="gear_leg")
            B = (P[0], ground + g["main_r"], zg)
            leg, tyres, hubs = bmesh.new(), bmesh.new(), bmesh.new()
            cyl_into(leg, P, (B[0], B[1] + 0.05, B[2]), 0.17 if lod < 2 else 0.25, 8 if lod < 2 else 4)
            box_into(leg, B, (0.12, 0.12, g["bogie"] * 0.5 + 0.1))
            for dz in (-g["bogie"] * 0.42, g["bogie"] * 0.42):
                wheel_pair(tyres, hubs, (B[0], B[1], B[2] + dz), g["main_r"], g["main_w"], n, hub=lod < 2)
            clean(mesh(f"leg_main_{s}.L{lod}", leg, "metal", piv, at=P))
            clean(mesh(f"tyres_main_{s}.L{lod}", tyres, "rubber", piv, at=P))
            if lod < 2:
                clean(mesh(f"hubs_main_{s}.L{lod}", hubs, "metal", piv, at=P))
            else:
                hubs.free()
        zn = p.z0 + g["nose_z"]
        Pn = (0.0, p.belly(zn) + 0.75, zn)
        piv = m.pivot("gear_nose", Pn, zone_of_z(fz, zn), (1, 0, 0), angle_deg=90.0, keys=up_keys, role="gear_leg")
        Bn = (0.0, ground + g["nose_r"], zn)
        leg, tyres, hubs = bmesh.new(), bmesh.new(), bmesh.new()
        cyl_into(leg, Pn, Bn, 0.12 if lod < 2 else 0.2, 8 if lod < 2 else 4)
        wheel_pair(tyres, hubs, Bn, g["nose_r"], 0.3, n, hub=lod < 2)
        clean(mesh(f"leg_nose.L{lod}", leg, "metal", piv, at=Pn))
        clean(mesh(f"tyres_nose.L{lod}", tyres, "rubber", piv, at=Pn))
        if lod < 2:
            clean(mesh(f"hubs_nose.L{lod}", hubs, "metal", piv, at=Pn))
        else:
            hubs.free()
        return
    # the taildragger: each main leg folds forward into its nacelle, wheel half out, as the DC-3's does
    e = p.k["radial"]
    for n_eng, (eng, side) in engine_list(p).items():
        x = eng["side_x"]
        ax = abs(x)
        cy = p.wing_y(ax) + 0.05
        pz = p.wing_le(ax) + 0.35
        P = (x, cy - 0.45, pz)
        piv = m.pivot(f"gear_main_{side}", P, f"engine_{n_eng}", (1, 0, 0), angle_deg=90.0, keys=up_keys, role="gear_leg")
        B = (x, P[1] - g["leg"], pz)
        leg, tyres, hubs = bmesh.new(), bmesh.new(), bmesh.new()
        for dx in (-0.22, 0.22):
            cyl_into(leg, (x + dx, P[1], pz), (x + dx, B[1], pz), 0.07 if lod < 2 else 0.12, 6 if lod < 2 else 4)
        prof = [(-g["main_w"] * 0.5, g["main_r"] * 0.6), (-g["main_w"] * 0.5, g["main_r"] * 0.85), (-g["main_w"] * 0.4, g["main_r"]),
                (g["main_w"] * 0.4, g["main_r"]), (g["main_w"] * 0.5, g["main_r"] * 0.85), (g["main_w"] * 0.5, g["main_r"] * 0.6)]
        t = lathe(prof, n, B, axis="x")
        t.to_mesh(TMP_ME)
        t.free()
        tyres.from_mesh(TMP_ME)
        clean(mesh(f"leg_main_{side}.L{lod}", leg, "metal", piv, at=P))
        clean(mesh(f"tyres_main_{side}.L{lod}", tyres, "rubber", piv, at=P))
        if lod < 2:
            h = lathe([(-g["main_w"] * 0.46, g["main_r"] * 0.62), (g["main_w"] * 0.46, g["main_r"] * 0.62)], max(n // 2, 6), B, axis="x")
            clean(mesh(f"hubs_main_{side}.L{lod}", h, "metal", piv, at=P))
        hubs.free()
    # the tail wheel, fixed
    zt = p.L * 0.5 + g["tail_z"]
    yb = p.belly(zt)
    leg, tyres = bmesh.new(), bmesh.new()
    cyl_into(leg, (0.0, yb + 0.15, zt - 0.25), (0.0, yb - 0.35, zt), 0.06, 6)
    t = lathe([(-0.07, g["tail_r"] * 0.6), (-0.06, g["tail_r"]), (0.06, g["tail_r"]), (0.07, g["tail_r"] * 0.6)], n, (0.0, yb - 0.35, zt), axis="x")
    t.to_mesh(TMP_ME)
    t.free()
    tyres.from_mesh(TMP_ME)
    clean(mesh(f"tail_leg.L{lod}", leg, "metal", m.zones["tail"]))
    clean(mesh(f"tail_wheel.L{lod}", tyres, "rubber", m.zones["tail"]))


def taildragger_rest(p):
    """The propliner's rest pose: (pitch up in degrees, the origin's height over the ground) with its main wheels and
    tail wheel on the ground."""
    g = p.k["gear"]
    x = p.k["engines"][0]["x"]
    cy = p.wing_y(x) + 0.05
    my, mz = cy - 0.45 - g["leg"] - g["main_r"], p.wing_le(x) + 0.35
    zt = p.L * 0.5 + g["tail_z"]
    ty = p.belly(zt) - 0.35 - g["tail_r"]
    pitch = math.atan2(ty - my, zt - mz)
    # the origin's height over the ground line through both contact points, once pitched level
    height = -(my * math.cos(pitch) - mz * math.sin(pitch))
    return math.degrees(pitch), height


# ----------------------------------------------------------------------------------------------------------- lights

def build_lights(m, p, lod):
    det = DETAIL[lod]
    if not det["lights"]:
        return
    tip = p.S * 0.5
    tle, tch, ty = p.wing_le(tip - 0.1), p.wing_chord(tip - 0.1), p.wing_y(tip - 0.1)
    jets = p.k["gear"]["kind"] == "tricycle"
    lights = [
        ("nav_left", (-tip + 0.05, ty, tle + tch * 0.3), "wing_outer_l", "nav_red", "red", "steady", 0.14),
        ("nav_right", (tip - 0.05, ty, tle + tch * 0.3), "wing_outer_r", "nav_green", "green", "steady", 0.14),
        ("strobe_left", (-tip + 0.05, ty, tle + tch * 0.75), "wing_outer_l", "strobe", "white", "strobe", 0.1),
        ("strobe_right", (tip - 0.05, ty, tle + tch * 0.75), "wing_outer_r", "strobe", "white", "strobe", 0.1),
    ]
    z_end = p.L * 0.5
    hw, hh, cy = p.at(z_end - 0.3)
    lights.append(("nav_tail", (0.0, cy, z_end + 0.05), "tail", "nav_white", "white", "steady", 0.12))
    zc = p.wing_le(0.0) + p.k["wing"]["root"] * 0.45
    lights.append(("beacon_top", (0.0, p.crown(zc) + 0.1, zc), "centre", "beacon", "red", "beacon", 0.16))
    lights.append(("beacon_bottom", (0.0, p.belly(zc) - 0.1, zc + 2.0), "centre", "beacon", "red", "beacon", 0.16))
    if jets:
        lights.append(("strobe_tail", (0.0, cy + 0.15, z_end + 0.02), "tail", "strobe", "white", "strobe", 0.08))
        xl = fus_side(p) + 1.0
        for side, s in ((-1, "l"), (1, "r")):
            lights.append((f"landing_{s}", (side * xl, p.wing_y(xl), p.wing_le(xl) + 0.05), f"wing_inner_{s}", "landing",
                           "white", "landing", 0.16))
    else:
        for side, s in ((-1, "l"), (1, "r")):
            xl = 5.2
            lights.append((f"landing_{s}", (side * xl, p.wing_y(xl), p.wing_le(xl) + 0.03), f"wing_inner_{s}", "landing",
                           "white", "landing", 0.13))
    for name, at, zone, mat, colour, pattern, r in lights:
        clean(A.light(f"light_{name}.L{lod}", at, mat, m.zones[zone], r=r))
        if lod == 0:
            m.lamp(name, at, zone, colour, pattern)


# --------------------------------------------------------------------------------------------------------- interior

def cabin_half(p, z, y, inset=0.1):
    return p.side(z, y) - p.k["wall"] - inset


def seat_into(bm_seat, bm_frame, x, z, floor, w, big=False):
    """One seat facing the nose: the cushion, the back leaning 12 degrees, and (as one) its frame."""
    d = 0.3 if big else 0.24
    back_h = 0.42 if big else 0.36
    hw = w * 0.5 - 0.025
    box_into(bm_seat, (x, floor + 0.42, z + d), (hw, 0.07, d))
    a = math.radians(12.0)
    box_into(bm_seat, (x, floor + 0.49 + back_h * math.cos(a), z + 2 * d + math.sin(a) * back_h - 0.02), (hw, back_h, 0.065),
             ((1, 0, 0), (0, math.cos(a), math.sin(a)), (0, -math.sin(a), math.cos(a))))
    if bm_frame is not None:
        box_into(bm_frame, (x, floor + 0.17, z + d), (hw * 0.7, 0.17, d * 0.6))


def seat_xs(p, z):
    """Seat centres across a row at z: the blocks against the walls (and one in the middle on the wide-body)."""
    c = p.k["cabin"]
    w = c["seat_w"]
    half = cabin_half(p, z, c["floor"] + 0.6)
    blocks = c["blocks"]
    xs = []
    left = -half
    for i in range(blocks[0]):
        xs.append(left + w * (i + 0.5))
    right = half
    for i in range(blocks[-1]):
        xs.append(right - w * (i + 0.5))
    if len(blocks) == 3:
        mid = blocks[1]
        for i in range(mid):
            xs.append(-mid * w * 0.5 + w * (i + 0.5))
    return sorted(xs)


def build_interior(m, p, fz):
    """The flight deck, the cabin floor, every seat row, the bins, the bulkheads and the galleys (LOD0 only)."""
    c = p.k["cabin"]
    tmp = m.root
    floor = c["floor"]
    wall = p.k["wall"]
    za_deck = deck_eye(p)[0] - 1.6
    z_back = p.z0 + p.L * 0.94 - 0.05
    out = []
    # the floor, a slab whose edges follow the wall
    zs = [za_deck] + [z for z in p.zs(za_deck, z_back) if za_deck < z < z_back] + [z_back]
    rings = []
    for z in zs:
        h = cabin_half(p, z, floor, 0.01)
        rings.append([(-h, floor, z), (h, floor, z), (h, floor - 0.08, z), (-h, floor - 0.08, z)])
    out.append(("floor", A.obj("int_floor", loft(rings), "carpet", tmp), 1.0))
    # seats
    seats, frames = bmesh.new(), bmesh.new()
    count = 0
    for i in range(c["rows"]):
        z = c["first"] + i * c["pitch"]
        for x in seat_xs(p, z + 0.3):
            seat_into(seats, frames, x, z, floor, c["seat_w"])
            count += 1
    m.seats = count
    out.append(("seats", mesh("int_seats", seats, "seat", tmp), 1.0))
    out.append(("seat_frames", mesh("int_seat_frames", frames, "dark", tmp), 1.0))
    # overhead bins along both walls (and the middle of the wide-body)
    if c["bins"]:
        za, zb = c["first"] - 0.3, c["first"] + (c["rows"] - 1) * c["pitch"] + 0.7
        bins = bmesh.new()
        y0, y1 = floor + 1.55, floor + 1.92
        xo = min(cabin_half(p, z, y1, 0.04) for z in (za, zb, (za + zb) * 0.5))
        for s in (-1, 1):
            box_into(bins, (s * (xo - 0.27), (y0 + y1) * 0.5, (za + zb) * 0.5), (0.27, (y1 - y0) * 0.5, (zb - za) * 0.5))
        if len(c["blocks"]) == 3:
            box_into(bins, (0.0, floor + 1.78, (za + zb) * 0.5), (0.75, 0.16, (zb - za) * 0.5))
        out.append(("bins", mesh("int_bins", bins, "trim", tmp), 1.0))
    # the bulkheads: behind the flight deck and behind the last row, each with a doorway
    for name, z in (("deck_wall", c["deck_end"]), ("aft_wall", c["cabin_end"])):
        r1 = fuselage_ring(p, z, 32, wall + 0.01)
        r2 = fuselage_ring(p, z + 0.06, 32, wall + 0.01)
        bh = A.obj(f"int_{name}", loft([r1, r2]), "trim", tmp)
        boolean(bh, [box_obj(f"{name}_door", (-0.38, floor - 0.1, z - 0.5), (0.38, floor + 1.95, z + 0.6), "trim", tmp)],
                "DIFFERENCE", "doorway")
        lo = box_obj(f"{name}_below", (-20, -20, z - 1), (20, floor - 0.08, z + 1), "trim", tmp)
        boolean(bh, [lo], "DIFFERENCE", "under the floor")
        out.append((name, bh, 1.0))
    # galleys: worktops against the walls behind the flight deck and behind the last row
    gal = bmesh.new()
    for za, zb in ((c["deck_end"] + 0.1, c["first"] - 0.25), (c["first"] + c["rows"] * c["pitch"] - 0.1, c["cabin_end"] - 0.05)):
        if zb - za < 0.4:
            continue
        zm = (za + zb) * 0.5
        h = cabin_half(p, zm, floor + 1.05, 0.03)
        for s in (-1, 1):
            box_into(gal, (s * (h - 0.3), floor + 0.52, zm), (0.3, 0.52, (zb - za) * 0.5 - 0.02))
    out.append(("galleys", mesh("int_galleys", gal, "trim", tmp), 1.0))
    # the flight deck: seats, the panel, the glare shield, the pedestal and the control columns
    jets = p.k["gear"]["kind"] == "tricycle"
    z_eye, _ = deck_eye(p)
    zp = z_eye - 0.85  # the panel's face, under the windscreen
    deck_seats, deck_frames, panel = bmesh.new(), bmesh.new(), bmesh.new()
    for x in (-0.5, 0.5):
        seat_into(deck_seats, deck_frames, x, z_eye - 0.35, floor, 0.56, big=True)
    if jets:
        seat_into(deck_seats, deck_frames, 0.75, z_eye + 0.6, floor, 0.52, big=True)
    hp = cabin_half(p, zp, floor + 0.9, 0.05)
    box_into(panel, (0.0, floor + 0.72, zp), (hp, 0.24, 0.16))
    box_into(panel, (0.0, floor + 0.99, zp - 0.02), (hp * 0.9, 0.03, 0.24))
    box_into(panel, (0.0, floor + 0.38, zp + 0.55), (0.16, 0.38, 0.4))
    for x in (-0.5, 0.5):
        cyl_into(panel, (x, floor + 0.15, zp + 0.5), (x, floor + 0.75, zp + 0.42), 0.035, 6)
        box_into(panel, (x, floor + 0.78, zp + 0.4), (0.14, 0.05, 0.02))
    out.append(("deck_seats", mesh("int_deck_seats", deck_seats, "seat", tmp), 1.0))
    out.append(("deck_frames", mesh("int_deck_frames", deck_frames, "dark", tmp), 1.0))
    out.append(("deck_panel", mesh("int_deck_panel", panel, "panel", tmp), 1.0))
    for name, o, tile in out:
        clean(o, tile_m=tile)
        for zone, piece in split_by_z(m, o, 0, f"int_{name}", fz).items():
            clean(piece, tile_m=tile)
            m.interior.append(piece.name)


# ----------------------------------------------------------------------------------------------- measure and export

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


def zone_manifest(m, p):
    out = []
    for name, node in m.zones.items():
        objs = [o for o in meshes_under(node) if lod_of(o) == 1 and not any(
            o in meshes_under(z) for zn, z in m.zones.items() if zn != name and m.parents.get(zn) is not None
            and _is_desc(m, zn, name))]
        # only this zone's own meshes, not its child zones'
        own = [o for o in objs if _zone_of_obj(m, o) == name]
        pts = [to_gltf(v) for o in own for v in world_verts(o)]
        key = "engine" if name.startswith("engine") else name.rsplit("_", 1)[0] if name.startswith("wing") else name
        out.append(dict(name=name, node=f"ZONE_{name}", parent=m.parents[name], share=SHARES[key],
                        capsules=capsules_of(pts) if pts else []))
    return out


def _is_desc(m, child, ancestor):
    z = m.parents.get(child)
    while z:
        if z == ancestor:
            return True
        z = m.parents.get(z)
    return False


def _zone_of_obj(m, o):
    n = o.parent
    while n is not None:
        if n.name.startswith("ZONE_"):
            return n.name[5:]
        n = n.parent
    return None


def cabin_view(p):
    """Where the page (and a playtest) stands to look down the aisle and out of the flight deck: eye height 1.6 m over
    the floor standing, 1.2 m seated."""
    c = p.k["cabin"]
    floor = c["floor"]
    last = c["first"] + (c["rows"] - 1) * c["pitch"]
    xs = seat_xs(p, last + 0.3)
    gaps = sorted(((b - a, (a + b) * 0.5) for a, b in zip(xs, xs[1:])), reverse=True)
    ax = gaps[0][1]
    z_eye, y_eye = deck_eye(p)
    # standing in the aisle, under the ceiling
    y_aisle = min(floor + 1.6, p.crown(last + 0.9, ax) - p.k["wall"] - 0.3)
    r = lambda v: [round(t, 3) for t in v]
    return {"aisle": r((ax, y_aisle, last + 0.9)), "aisle_look": r((ax, y_aisle - 0.25, c["first"] - 3.0)),
            "deck": r((-0.5, y_eye, z_eye)), "deck_look": r((-0.5, y_eye + 0.1, z_eye - 12.0))}


def cabin_lights(p):
    """Three points down the cabin's ceiling where the page hangs a warm light at night."""
    c = p.k["cabin"]
    za, zb = c["first"], c["first"] + (c["rows"] - 1) * c["pitch"]
    return [[0.0, round(c["floor"] + 1.95, 3), round(za + (zb - za) * t, 3)] for t in (0.1, 0.5, 0.9)]


def export(path, root):
    objs = [root] + list(root.children_recursive)
    return A.export(path, objs)


def build(kind, shots=None):
    p = Plane(kind)
    K.clear_scene()
    for coll in list(bpy.data.collections):
        bpy.data.collections.remove(coll)
    CSG.clear()
    materials(kind, p.k)
    m = Model(p)
    fz = make_zones(m, p)
    for lod in LEVELS:
        build_fuselage(m, p, lod, fz)
        build_wings(m, p, lod)
        build_tail(m, p, lod)
        for n, (e, side) in engine_list(p).items():
            if e["kind"] == "turbofan":
                build_turbofan(m, p, lod, n, e["side_x"])
            elif e["kind"] == "tail":
                build_tail_engine(m, p, lod, n)
            else:
                build_radial(m, p, lod, n, e["side_x"])
        build_gear(m, p, lod, fz)
        build_lights(m, p, lod)
    build_interior(m, p, fz)
    # nothing but the zones' nodes may hang off the root
    stray = [o.name for o in m.root.children if not o.name.startswith("ZONE_")]
    if stray:
        raise SystemExit(f"{kind}: stray nodes under the root: {stray}")
    objs = meshes_under(m.root)
    counts = {lod: tris([o for o in objs if lod_of(o) == lod]) for lod in LEVELS}
    over = [f"LOD{k}: {t} triangles, budget {p.k['budget'][k]}" for k, t in counts.items() if t > p.k["budget"][k]]
    if over:
        raise SystemExit(f"{kind} over budget, nothing written: " + "; ".join(over) + f" (all: {counts})")
    if m.seats != p.k["seats"]:
        raise SystemExit(f"{kind}: {m.seats} seats built, the design says {p.k['seats']}")
    unnamed = [o.name for o in objs if lod_of(o) is None]
    if unnamed:
        raise SystemExit(f"{kind}: meshes with no level: {unnamed[:8]}")
    # bounds and rest pose
    pts = [to_gltf(v) for o in objs if lod_of(o) == 1 for v in world_verts(o)]
    lo = [min(q[i] for q in pts) for i in range(3)]
    hi = [max(q[i] for q in pts) for i in range(3)]
    if p.k["gear"]["kind"] == "tricycle":
        pitch, rest = 0.0, rest_of(p)
    else:
        pitch, rest = taildragger_rest(p)
    a = math.radians(pitch)
    height = max(y * math.cos(a) + z * (-math.sin(a)) for _, y, z in pts) + rest
    for o in objs:
        o.data.validate(clean_customdata=False)
        if os.environ.get("CB_DEBUG_BOOL"):
            bm = bmesh.new()
            bm.from_mesh(o.data)
            v = bm.calc_volume(signed=True)
            bm.free()
            if o.name.startswith(("wing_", "flap", "aileron")):
                print("PIECE", o.name, o.parent.name, len(o.data.polygons), round(v, 3))
            if v < 0 or not o.data.polygons:
                print("INSIDE-OUT or EMPTY", o.name, round(v, 3), len(o.data.polygons))
    os.makedirs(OUT, exist_ok=True)
    glb = os.path.join(OUT, f"{kind}.glb")
    sha = export(glb, m.root)
    manifest = {
        "_": "One of the three airliners (openspec change airliner-models), built by tools/blender/build_airliners.py: "
             "written by the build, never by hand. Frame: glTF metres, +Y up, nose toward -Z, +X the right wing, origin "
             "on the fuselage's centre line half way along it. Meshes are <part>.L<level> under ZONE_* nodes or RIG_* "
             "pivots; level 0 carries the interior (int_*).",
        "id": kind,
        "after": p.k["after"],
        "blueprint": f"docs/reference/airliner-blueprints/{p.k['blueprint']}",
        "file": f"{kind}.glb",
        "generator": "tools/blender/build_airliners.py",
        "blender": bpy.app.version_string,
        "size_m": {"length": p.L, "span": p.S, "height": p.k["height"], "fuselage": p.k["width"]},
        "measured_m": {"length": round(hi[2] - lo[2], 3), "span": round(hi[0] - lo[0], 3), "height_at_rest": round(height, 3)},
        "rest_m": round(rest, 3),
        "rest_pitch_deg": round(pitch, 2),
        "bounds_m": {"x": [round(lo[0], 3), round(hi[0], 3)], "y": [round(lo[1], 3), round(hi[1], 3)],
                     "z": [round(lo[2], 3), round(hi[2], 3)]},
        "lods": [{"lod": k, "triangles": counts[k], "budget_triangles": p.k["budget"][k],
                  "within_m": [250, 1500, None][k]} for k in LEVELS],
        "seats": m.seats,
        "interior": sorted(m.interior),
        "zones": zone_manifest(m, p),
        "rig": m.rig,
        "gear_sequence_s": 8.0,
        "lamps": m.lamps,
        "light_patterns": {"steady": {}, "beacon": {"period_s": 1.0, "on_s": 0.1},
                           "strobe": {"period_s": 1.2, "on_s": 0.05, "gap_s": 0.15, "flashes": 2},
                           "landing": {"below_agl_m": 600}},
        "cabin_view": cabin_view(p),
        "cabin_lights": cabin_lights(p),
        "materials": [A.MATS[k].name for k in ORDER],
        "csg_steps": len(CSG),
        "sha256": sha,
        "bytes": os.path.getsize(glb),
    }
    for key, real in (("length", p.L), ("span", p.S)):
        got = manifest["measured_m"][key]
        if abs(got - real) > real * 0.02:
            ext = sorted(((max(to_gltf(v)[2] for v in world_verts(o)), min(to_gltf(v)[2] for v in world_verts(o)), o.name)
                          for o in objs if lod_of(o) == 1), key=lambda t: max(t[0], -t[1]))[-4:]
            raise SystemExit(f"{kind}: {key} {got} m against the real {real} m (more than 2 % off); longest: {ext}")
    with open(os.path.join(OUT, f"{kind}.json"), "w") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")
    print(f"EXPORTED {kind}: " + ", ".join(f"LOD{k} {t} tris" for k, t in counts.items())
          + f"; {m.seats} seats, {len(m.zones)} zones, {len(m.rig)} pivots, {len(m.lamps)} lamps; measured "
          + f"{manifest['measured_m']}; rest {rest:.2f} m pitch {pitch:.1f}; {manifest['bytes'] / 1e6:.1f} MB")
    if shots:
        look_sheet(m, p, kind, shots)


def look_sheet(m, p, kind, out_dir):
    """Workbench shots of level 0 from six sides, for the eye (not shipped)."""
    os.makedirs(out_dir, exist_ok=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.render.resolution_x, scene.render.resolution_y = 1000, 640
    scene.render.film_transparent = False
    world = bpy.data.worlds.new("w")
    scene.world = world
    for o in meshes_under(m.root):
        o.hide_render = lod_of(o) != 0
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    scene.collection.objects.link(cam)
    scene.camera = cam
    L = p.L
    views = {"front34": ((-0.9, 0.5, -1.0), 1.0), "rear34": ((0.9, 0.6, 1.0), 1.0), "side": ((-1.0, 0.0, 0.0), 0.9),
             "top": ((0.0, 1.0, 0.001), 0.8), "front": ((0.0, 0.05, -1.0), 0.75), "below": ((-0.5, -0.6, -0.6), 1.0)}
    for name, (d, k) in views.items():
        d = Vector(d).normalized()
        dist = max(L, p.S) * 1.25 * k
        target = Vector((0.0, 0.0, 0.0))
        pos = target + d * dist
        cam.location = g2b(*pos)
        look = g2b(*target) - cam.location
        cam.rotation_euler = look.to_track_quat("-Z", "Y").to_euler()
        cam.data.lens = 50
        scene.render.filepath = os.path.join(out_dir, f"{kind}_{name}.png")
        bpy.ops.render.render(write_still=True)


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    shots = None
    if "--shots" in argv:
        i = argv.index("--shots")
        shots = argv[i + 1]
        argv = argv[:i] + argv[i + 2:]
    kinds = argv or list(KINDS)
    for kind in kinds:
        build(kind, shots)


main()
