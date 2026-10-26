"""Block-outs of every boat and the harbour's crane and quay, at true size, built the CSG way.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_boats.py

openspec/changes/harbours (design section 9; HB7 and HB19, pre-approved by the owner, 2026-10-26: "You also forgot
boats, I'll take whatever you recommend for that", "No survey needed qs preapprove all boat choices"). Labelled
stand-ins until a detail pass: flat PBR colours (base colour, roughness, metal; no maps yet), every size the real
1980s one, read from data/world/ships.ron so the model and the game agree:

  freighter     120 m general cargo ship: raised forecastle, three hatches cut into the deck, two derrick masts,
                the bridge house aft with its windows cut through, a funnel
  tanker        150 m product tanker: a flush deck with the pipe rack and manifold, a tall bridge house aft, a funnel
  fishing_boat  22 m trawler: the wheelhouse forward, a stern gantry, a mast
  rowing_boat   4.2 m clinker skiff: its open hull hollowed by a cutter, three thwarts, oars stowed
  launch        8 m motor launch: a cuddy cabin with a windscreen, the cockpit cut into the deck
  ferry         60 m islet ferry: a bow ramp, two passenger decks with window bands cut, the bridge on top
  gunboat       30 m raider patrol boat: a gun turret forward with its barrel, the bridge amidships, a radar mast
  crane         a harbour's luffing crane: a 12 m portal on four legs, the slewing house, the tower and a 35 m jib
  quay          a 40 m section of the quay: the deck on piles, bollards and fenders along the berth face

Frames (glTF, as Bevy reads them): +Y up. A boat's bow is toward -Z (heading 0 in the game faces -Z), its origin at
the middle of its waterline, its keel at -draft. The crane's origin is the middle of its portal's foot, its jib out
toward -Z (out to sea in the harbour's frame). The quay section's origin is the middle of its deck's top, its berth
face toward -Z.

Writes assets/models/boats/<id>.glb (LOD 0) and <id>_lod1.glb (LOD 1: the hull and the main blocks, no masts,
derricks, rails or cut windows), and assets/models/boats.json, the manifest measured off the exports: bounds,
triangles against each LOD's budget. `cb-tools validate` checks every boat's measured size against its data.

The helpers follow tools/blender/build_mech_blockouts.py (box, cylinder, loft and the Exact boolean that transfers
the cutter's material), which runs its own build on import, so they are kept small here.
"""
import json
import math
import os
import re

import bmesh
import bpy
from mathutils import Matrix, Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "boats")
MANIFEST = os.path.join(ROOT, "assets", "models", "boats.json")
SHIPS = os.path.join(ROOT, "data", "world", "ships.ron")
G2B = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # glTF frame (y up, -z ahead) to Blender
V = Vector

# Triangle budgets per LOD (design section 10): ships, small craft, harbour pieces.
BUDGET = {
    "freighter": (6000, 2500), "tanker": (6000, 2500), "ferry": (6000, 2500), "gunboat": (4000, 1500),
    "fishing_boat": (3000, 1200), "launch": (2000, 800), "rowing_boat": (1500, 600),
    "crane": (3000, 1200), "quay": (4000, 1500),
}

PALETTE = {
    "HULL": ((0.20, 0.26, 0.34), 0.55, 0.4),
    "BOTTOM": ((0.45, 0.10, 0.08), 0.7, 0.1),
    "DECK": ((0.32, 0.30, 0.27), 0.8, 0.1),
    "HOUSE": ((0.88, 0.87, 0.82), 0.6, 0.1),
    "GLASS": ((0.04, 0.06, 0.08), 0.1, 0.6),
    "FUNNEL": ((0.75, 0.16, 0.10), 0.5, 0.2),
    "BLACK": ((0.06, 0.06, 0.07), 0.6, 0.3),
    "METAL": ((0.55, 0.56, 0.58), 0.35, 0.9),
    "YELLOW": ((0.92, 0.72, 0.12), 0.55, 0.2),
    "WOOD": ((0.50, 0.33, 0.18), 0.75, 0.0),
    "WHITE": ((0.90, 0.90, 0.88), 0.5, 0.1),
    "GREY": ((0.34, 0.36, 0.34), 0.6, 0.5),
    "CONCRETE": ((0.46, 0.45, 0.42), 0.9, 0.0),
    "RUBBER": ((0.05, 0.05, 0.05), 0.95, 0.0),
    "TANKER": ((0.40, 0.14, 0.12), 0.55, 0.4),
    "TRAWLER": ((0.70, 0.22, 0.16), 0.6, 0.3),
}
MATS = {}
PARTS = []          # (object, lod1)


def ships_data():
    """Each boat kind's size, draught and tint from ships.ron (the one source of the numbers)."""
    text = open(SHIPS).read()
    out = {}
    for m in re.finditer(r'"(\w+)": \(\s*size_m: \(([\d.]+), ([\d.]+), ([\d.]+)\), draft_m: ([\d.]+)', text):
        out[m.group(1)] = dict(beam=float(m.group(2)), height=float(m.group(3)), length=float(m.group(4)),
                               draft=float(m.group(5)))
    crane = re.search(r"crane_m: \(([\d.]+), ([\d.]+), ([\d.]+)\)", text)
    out["crane"] = dict(portal=float(crane.group(1)), height=float(crane.group(2)), jib=float(crane.group(3)))
    return out


def materials():
    for name, (rgb, rough, metal) in PALETTE.items():
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        b = m.node_tree.nodes["Principled BSDF"]
        b.inputs["Base Color"].default_value = (*rgb, 1)
        b.inputs["Roughness"].default_value = rough
        b.inputs["Metallic"].default_value = metal
        MATS[name] = m


def obj(name, bm, mat, lod1=True):
    bmesh.ops.transform(bm, matrix=G2B, verts=bm.verts)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.materials.append(MATS[mat])
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    PARTS.append((ob, lod1))
    return ob


def box(name, c, size, mat, lod1=True, rot_x=0.0, rot_y=0.0):
    """A box of `size` (x, y, z) centred on `c`, turned `rot_x` about x then `rot_y` about y (radians)."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    m = Matrix.Translation(V(c)) @ Matrix.Rotation(rot_y, 4, "Y") @ Matrix.Rotation(rot_x, 4, "X") @ Matrix.Diagonal((*size, 1))
    bmesh.ops.transform(bm, verts=bm.verts, matrix=m)
    return obj(name, bm, mat, lod1)


def cyl(name, a, b, r, mat, lod1=True, seg=12, r2=None):
    a, b = V(a), V(b)
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r2 is None else r2, depth=(b - a).length)
    q = V((0, 0, 1)).rotation_difference((b - a).normalized())
    bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation((a + b) / 2) @ q.to_matrix().to_4x4())
    return obj(name, bm, mat, lod1)


def strut(name, a, b, w, mat, lod1=False):
    """A square member from a to b."""
    a, b = V(a), V(b)
    d = (b - a)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    q = V((0, 0, 1)).rotation_difference(d.normalized())
    bmesh.ops.transform(bm, verts=bm.verts,
                        matrix=Matrix.Translation((a + b) / 2) @ q.to_matrix().to_4x4() @ Matrix.Diagonal((w, w, d.length, 1)))
    return obj(name, bm, mat, lod1)


def loft_z(name, keys, mat, lod1=True):
    """A closed solid through sections across z: [(z, [(x, y), ...])], each with the same number of points."""
    bm = bmesh.new()
    rings = [[bm.verts.new((x, y, z)) for x, y in poly] for z, poly in keys]
    n = len(rings[0])
    bm.faces.new(rings[0])
    bm.faces.new(rings[-1])
    for r0, r1 in zip(rings, rings[1:]):
        for i in range(n):
            bm.faces.new((r0[i], r0[(i + 1) % n], r1[(i + 1) % n], r1[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return obj(name, bm, mat, lod1)


def boolean(target, cutter, op="DIFFERENCE"):
    for m in cutter.data.materials:
        if m.name not in [s.name for s in target.data.materials]:
            target.data.materials.append(m)
    mod = target.modifiers.new("csg", "BOOLEAN")
    mod.operation, mod.solver, mod.object, mod.material_mode = op, "EXACT", cutter, "TRANSFER"
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    PARTS[:] = [p for p in PARTS if p[0] is not cutter]
    bpy.data.objects.remove(cutter)
    return target


def hull(name, length, beam, draft, deck, mat, sheer=1.0, transom=0.85, stem=0.04, flat_bow=False):
    """A ship's hull lofted from stern (+z) to bow (-z): full sections amidships, a transom aft, the bow fined to a
    stem with a rising forefoot, the deck rising by `sheer` to the bow. Below 0.4 m over the waterline the faces are
    the antifouling red."""
    L, b, d = length / 2, beam / 2, draft

    def sec(z, w, keel, top):
        return (z, [(-w, top), (-w, -d * 0.35), (-w * 0.55, keel), (w * 0.55, keel), (w, -d * 0.35), (w, top)])

    bow_w = b * (0.55 if flat_bow else stem)
    keys = [
        sec(L, b * transom, -d * 0.55, deck),
        sec(L * 0.6, b, -d, deck),
        sec(-L * 0.35, b, -d, deck + sheer * 0.2),
        sec(-L * 0.75, b * 0.62, -d * 0.85, deck + sheer * 0.6),
        sec(-L, bow_w, -d * 0.25, deck + sheer),
    ]
    h = loft_z(name, keys, mat)
    # The waterline band: split the faces at 0.4 m and paint those below it.
    bm = bmesh.new()
    bm.from_mesh(h.data)
    geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
    bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, 0.4), plane_no=(0, 0, 1))
    h.data.materials.append(MATS["BOTTOM"])
    for f in bm.faces:
        if f.calc_center_median().z < 0.4:
            f.material_index = 1
    bm.to_mesh(h.data)
    bm.free()
    return h


def windows(target, name, x0, x1, y, z, rows=1, h=1.0):
    """Window bands from x0 to x1 cut into a house's face at `z` with dark cutters (the cutter's material lines the
    openings), `rows` of them 2.8 m apart from height `y`."""
    for r in range(rows):
        boolean(target, box(f"{name}_w{r}", ((x0 + x1) / 2, y + r * 2.8, z), (x1 - x0, h, 0.6), "GLASS", lod1=False))


def side_windows(target, name, half_w, y, z, length, h=1.0):
    """A window band along both sides of a house `half_w` from its middle, `length` long about `z`."""
    for sx in (-1, 1):
        boolean(target, box(f"{name}_s{sx}", (sx * half_w, y, z), (0.6, h, length), "GLASS", lod1=False))


# ----------------------------------------------------------------------------------------------------- the boats

def freighter(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 3.0
    hl = hull("freighter_hull", L, B, d, deck, "HULL", sheer=2.0)
    # Three hatches: recesses cut into the deck, then coamings and covers set in them.
    for i, z in enumerate((-30.0, -8.0, 14.0)):
        boolean(hl, box(f"hatch_cut{i}", (0, deck, z), (B * 0.6, 1.0, 16.0), "DECK"))
        box(f"hatch{i}", (0, deck + 0.4, z), (B * 0.56, 1.4, 15.0), "DECK")
    # The forecastle.
    box("fo'c'sle", (0, deck + 1.5, -L / 2 + 10.0), (B * 0.5, 3.0, 14.0), "HULL")
    # The bridge house aft, windows cut, wings, a funnel.
    house = box("house", (0, deck + 4.2, L / 2 - 16.0), (B * 0.9, 8.4, 14.0), "HOUSE")
    windows(house, "house", -B * 0.4, B * 0.4, deck + 3.0, L / 2 - 23.0, rows=3)
    box("bridge_wings", (0, deck + 7.6, L / 2 - 21.5), (B, 1.2, 3.0), "HOUSE", lod1=False)
    cyl("funnel", (0, deck + 8.4, L / 2 - 12.0), (0, deck + 13.0, L / 2 - 12.0), 2.2, "FUNNEL", seg=12)
    box("funnel_cap", (0, deck + 13.3, L / 2 - 12.0), (4.6, 0.6, 4.6), "BLACK", lod1=False)
    # Two derrick masts between the hatches, rigged to the masthead height.
    top = H - d
    for i, z in enumerate((-19.0, 3.0)):
        cyl(f"mast{i}", (0, deck, z), (0, top, z), 0.45, "WHITE", lod1=False, seg=8)
        for sx in (-1, 1):
            strut(f"derrick{i}{sx}", (sx * 1.0, deck + 2.0, z), (sx * 5.0, deck + 12.0, z - 7.0), 0.3, "WHITE")
    # Rails along the deck edge.
    for sx in (-1, 1):
        box(f"rail{sx}", (sx * (B / 2 - 0.1), deck + 0.55, -2.0), (0.12, 1.1, L * 0.8), "WHITE", lod1=False)


def tanker(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 3.0
    hl = hull("tanker_hull", L, B, d, deck, "TANKER", sheer=1.5)
    boolean(hl, box("manifold_well", (0, deck, -6.0), (B * 0.3, 0.8, 6.0), "DECK"))
    box("pipe_rack", (0, deck + 1.0, -16.0), (2.0, 1.6, L * 0.62), "METAL", lod1=False)
    for sx in (-1, 1):
        cyl(f"manifold{sx}", (0, deck + 1.0, -6.0), (sx * (B / 2 - 1.0), deck + 1.0, -6.0), 0.5, "METAL", lod1=False, seg=8)
    box("catwalk", (0, deck + 2.3, -16.0), (1.2, 0.2, L * 0.62), "YELLOW", lod1=False)
    house = box("house", (0, deck + 5.5, L / 2 - 16.0), (B * 0.85, 11.0, 14.0), "HOUSE")
    windows(house, "house", -B * 0.38, B * 0.38, deck + 3.0, L / 2 - 23.0, rows=4)
    cyl("funnel", (0, deck + 11.0, L / 2 - 10.0), (0, deck + 15.0, L / 2 - 10.0), 2.6, "BLACK", seg=12)
    top = H - d
    cyl("mast", (0, deck + 11.0, L / 2 - 19.0), (0, top, L / 2 - 19.0), 0.35, "WHITE", lod1=False, seg=8)
    cyl("fore_mast", (0, deck, -L / 2 + 8.0), (0, deck + 9.0, -L / 2 + 8.0), 0.3, "WHITE", lod1=False, seg=8)


def fishing_boat(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 1.5
    hl = hull("trawler_hull", L, B, d, deck, "TRAWLER", sheer=1.2)
    boolean(hl, box("fish_hold", (0, deck, 4.0), (B * 0.5, 0.8, 4.0), "DECK"))
    house = box("wheelhouse", (0, deck + 1.5, -2.5), (B * 0.7, 3.0, 5.0), "HOUSE")
    windows(house, "wheelhouse", -B * 0.3, B * 0.3, deck + 2.0, -5.05, rows=1, h=0.9)
    # The stern gantry (an A-frame) and the mast.
    for sx in (-1, 1):
        strut(f"gantry{sx}", (sx * (B / 2 - 0.4), deck, L / 2 - 1.0), (sx * 1.0, deck + 4.5, L / 2 - 2.0), 0.3, "YELLOW")
    box("gantry_bar", (0, deck + 4.5, L / 2 - 2.0), (2.4, 0.3, 0.3), "YELLOW", lod1=False)
    cyl("mast", (0, deck + 3.0, -1.0), (0, H - d, -1.0), 0.18, "WHITE", lod1=False, seg=8)
    cyl("drum", (-1.2, deck + 0.6, 6.0), (1.2, deck + 0.6, 6.0), 0.55, "METAL", lod1=False, seg=10)


def rowing_boat(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    top = H - d
    hl = hull("skiff_hull", L, B, d, top, "WOOD", sheer=0.0, transom=0.7, stem=0.05)
    # Hollow it out: a cutter inside the gunwales, leaving 4 cm walls and a floor.
    cutter = hull("skiff_cut", L - 0.2, B - 0.12, d - 0.08, top + 0.5, "WOOD", sheer=0.0, transom=0.7, stem=0.05)
    cutter.location = (0, 0, 0.12)
    bpy.context.view_layer.update()
    bpy.context.view_layer.objects.active = cutter
    cutter.select_set(True)
    bpy.ops.object.transform_apply(location=True)
    boolean(hl, cutter)
    for i, z in enumerate((-1.0, 0.1, 1.2)):
        box(f"thwart{i}", (0, top - 0.25, z), (B - 0.2, 0.05, 0.22), "WOOD")
    for sx in (-1, 1):
        box(f"oar{sx}", (sx * 0.25, top - 0.15, 0.0), (0.06, 0.04, 3.2), "WOOD", lod1=False)


def launch(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 0.7
    hl = hull("launch_hull", L, B, d, deck, "WHITE", sheer=0.3, transom=0.9)
    boolean(hl, box("cockpit", (0, deck, 1.6), (B * 0.75, 0.9, 3.0), "DECK"))
    cabin = box("cabin", (0, deck + 0.65, -1.2), (B * 0.72, 1.3, 2.6), "HOUSE")
    windows(cabin, "cabin", -B * 0.3, B * 0.3, deck + 0.8, -2.5, rows=1, h=0.45)
    box("windscreen", (0, deck + 0.95, 0.2), (B * 0.7, 0.45, 0.08), "GLASS", lod1=False, rot_x=0.35)
    cyl("mast", (0, deck + 1.3, -0.6), (0, H - d, -0.6), 0.04, "METAL", lod1=False, seg=6)


def ferry(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 2.8
    hl = hull("ferry_hull", L, B, d, deck, "HULL", sheer=0.6, flat_bow=True)
    # The bow ramp, raised, and the car deck's opening cut behind it.
    boolean(hl, box("car_deck", (0, deck, -L / 2 + 4.0), (B * 0.5, 1.2, 6.0), "DECK"))
    box("ramp", (0, deck + 1.4, -L / 2 + 1.2), (B * 0.5, 2.8, 0.4), "METAL")
    d1 = box("deck1", (0, deck + 1.5, 4.0), (B * 0.92, 3.0, L * 0.62), "WHITE")
    side_windows(d1, "deck1", B * 0.46, deck + 1.6, 4.0, L * 0.5, h=1.0)
    d2 = box("deck2", (0, deck + 4.3, 6.0), (B * 0.8, 2.6, L * 0.45), "WHITE")
    side_windows(d2, "deck2", B * 0.4, deck + 4.4, 6.0, L * 0.38, h=0.9)
    bridge = box("bridge", (0, deck + 6.6, -1.0), (B * 0.6, 2.0, 5.0), "HOUSE")
    windows(bridge, "bridge", -B * 0.28, B * 0.28, deck + 6.7, -3.55, rows=1, h=0.9)
    cyl("funnel", (0, deck + 5.6, 14.0), (0, deck + 9.0, 14.0), 1.4, "FUNNEL", seg=10)
    cyl("mast", (0, deck + 7.6, -1.0), (0, H - d, -1.0), 0.2, "WHITE", lod1=False, seg=8)


def gunboat(s):
    L, B, d, H = s["length"], s["beam"], s["draft"], s["height"]
    deck = 2.0
    hull("gunboat_hull", L, B, d, deck, "GREY", sheer=0.8)
    bridge = box("bridge", (0, deck + 1.5, 1.0), (B * 0.6, 3.0, 7.0), "GREY")
    windows(bridge, "bridge", -B * 0.26, B * 0.26, deck + 2.2, -2.55, rows=1, h=0.7)
    turret = cyl("turret", (0, deck, -8.0), (0, deck + 1.6, -8.0), 1.4, "GREY", seg=12)
    boolean(turret, box("gun_port", (0, deck + 0.9, -9.3), (0.8, 0.7, 0.6), "BLACK"))
    cyl("barrel", (0, deck + 0.9, -8.6), (0, deck + 0.9, -12.6), 0.12, "BLACK", lod1=False, seg=8)
    cyl("radar_mast", (0, deck + 3.0, 2.0), (0, H - d, 2.0), 0.15, "METAL", lod1=False, seg=8)
    box("radar", (0, H - d - 0.5, 2.0), (1.8, 0.2, 0.4), "BLACK", lod1=False)
    box("aft_gun", (0, deck + 0.6, 10.0), (0.9, 1.2, 0.9), "GREY", lod1=False)


def crane(s):
    portal, height, jib = s["portal"], s["height"], s["jib"]
    p = portal / 2
    # The portal: four legs and the girders over them, the slewing house, the tower and the jib out over -z.
    for sx in (-1, 1):
        for sz in (-1, 1):
            box(f"leg{sx}{sz}", (sx * (p - 0.6), 5.0, sz * (p - 0.6)), (1.2, 10.0, 1.2), "YELLOW")
    gird = box("portal_top", (0, 10.6, 0), (portal, 1.2, portal), "YELLOW")
    boolean(gird, box("portal_hole", (0, 10.6, 0), (portal - 3.0, 2.0, portal - 3.0), "YELLOW"))
    house = box("house", (0, 14.0, 1.0), (6.0, 5.6, 8.0), "YELLOW")
    windows(house, "house", -2.0, 2.0, 14.2, -3.05, rows=1, h=1.0)
    cyl("tower", (0, 16.8, 0), (0, height, 0), 1.2, "YELLOW", seg=10)
    a = math.radians(28.0)
    tip = (0, height - 3.0 + math.sin(a) * jib, -math.cos(a) * jib)
    strut("jib", (0, height - 3.0, 0), tip, 1.2, "YELLOW", lod1=True)
    strut("jib_tie", (0, height, 0), tip, 0.3, "METAL")
    cyl("hook", (0, tip[1] - 0.5, tip[2]), (0, tip[1] - 10.0, tip[2]), 0.05, "METAL", lod1=False, seg=6)
    box("counterweight", (0, height - 2.0, 4.0), (3.0, 2.5, 3.0), "BLACK")


def quay(s):
    w, depth, bed = 40.0, 30.0, -14.0
    box("deck", (0, -0.6, 0), (w, 1.2, depth), "CONCRETE")
    for x in range(-18, 19, 12):
        for z in range(-13, 14, 12):
            box(f"pile{x}_{z}", (x, (bed - 1.2) / 2, z), (1.0, -bed - 1.2, 1.0), "CONCRETE", lod1=(z == -13))
    for i, x in enumerate((-15.0, 0.0, 15.0)):
        cyl(f"bollard{i}", (x, 0, -depth / 2 + 1.0), (x, 0.8, -depth / 2 + 1.0), 0.3, "BLACK", lod1=False, seg=10)
        box(f"fender{i}", (x, -1.4, -depth / 2 - 0.25), (1.6, 2.0, 0.5), "RUBBER", lod1=False)
    box("kerb", (0, 0.1, -depth / 2 + 0.25), (w, 0.2, 0.5), "YELLOW", lod1=False)


BUILDERS = dict(freighter=freighter, tanker=tanker, fishing_boat=fishing_boat, rowing_boat=rowing_boat, launch=launch,
                ferry=ferry, gunboat=gunboat, crane=crane, quay=quay)
LABEL = dict(freighter="Freighter", tanker="Tanker", fishing_boat="Fishing boat", rowing_boat="Rowing boat",
             launch="Launch", ferry="Ferry", gunboat="Gunboat", crane="Harbour crane", quay="Quay section")


def clear():
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    for m in list(bpy.data.meshes):
        bpy.data.meshes.remove(m)
    PARTS.clear()


def join(name, objs):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    ob.name = name
    ob.data.name = name
    # Triangulate, so the count is what the game draws and the export is stable.
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    bm.to_mesh(ob.data)
    bm.free()
    return ob


def export(ob, path):
    bpy.ops.object.select_all(action="DESELECT")
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_apply=True, export_yup=True)


def measure(ob):
    """Bounds in the glTF frame (x, y up, z) and the triangle count."""
    pts = [ob.matrix_world @ v.co for v in ob.data.vertices]
    g = [(p.x, p.z, -p.y) for p in pts]
    lo = [round(min(c[i] for c in g), 3) for i in range(3)]
    hi = [round(max(c[i] for c in g), 3) for i in range(3)]
    return lo, hi, len(ob.data.polygons)


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    materials()
    os.makedirs(OUT, exist_ok=True)
    data = ships_data()
    rows = {}
    for key, build in BUILDERS.items():
        clear()
        build(data.get(key, data["crane"]) if key != "quay" else {})
        objs = [p[0] for p in PARTS]
        lod1_objs = [p[0] for p in PARTS if p[1]]
        # LOD 1 first, from copies of its parts, so LOD 0 keeps them all.
        copies = []
        for o in lod1_objs:
            c = o.copy()
            c.data = o.data.copy()
            bpy.context.scene.collection.objects.link(c)
            copies.append(c)
        lod1 = join(f"{key}_lod1", copies)
        export(lod1, os.path.join(OUT, f"{key}_lod1.glb"))
        _, _, tris1 = measure(lod1)
        bpy.data.objects.remove(lod1)
        lod0 = join(key, objs)
        export(lod0, os.path.join(OUT, f"{key}.glb"))
        lo, hi, tris0 = measure(lod0)
        b0, b1 = BUDGET[key]
        if tris0 > b0 or tris1 > b1:
            raise SystemExit(f"{key}: over budget, LOD 0 {tris0}/{b0}, LOD 1 {tris1}/{b1}")
        rows[key] = dict(
            label=LABEL[key],
            file=f"boats/{key}.glb",
            lod1=f"boats/{key}_lod1.glb",
            stand_in=True,
            bounds_m=dict(min=lo, max=hi),
            size_m=[round(hi[i] - lo[i], 2) for i in range(3)],
            lods=[dict(lod=0, triangles=tris0, budget_triangles=b0), dict(lod=1, triangles=tris1, budget_triangles=b1)],
        )
        print("BUILT", key, rows[key]["size_m"], tris0, tris1, flush=True)
    with open(MANIFEST, "w") as f:
        json.dump(dict(
            _="Boat, crane and quay block-outs from tools/blender/build_boats.py (openspec/changes/harbours section 9): "
              "labelled stand-ins at true size. glTF frame: metres, +Y up; a boat's bow toward -Z, its origin the "
              "middle of its waterline; the crane's jib and the quay's berth face toward -Z.",
            generator="tools/blender/build_boats.py",
            blender=bpy.app.version_string,
            boats=rows,
        ), f, indent=1, sort_keys=True)
        f.write("\n")


main()
