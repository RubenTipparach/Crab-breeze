"""Crab Breeze: the relief kit for cockpit panels, and the bake that turns a panel into texture maps.

Star Crew's panel-textures method (`star-crew-bridge-simulator/tools/blender/build_wall_panels.py`
at 8b0bbf1), retargeted to Crab Breeze's high-detail PBR look: a panel (a console face, a wall
plate, a breaker board) is MODELLED in Blender out of real pieces (plates with seams, screws,
buttons with backlit caps, toggles with guards, rotary knobs, dials, keypads, stencilled placards,
hazard stripes, circuit boards with traces and chips) and rendered face-on with an orthographic
camera in Cycles. One render writes every map the cockpit needs from that panel:

  albedo     the Diffuse Color pass (base colour with crevice grime and edge wear in it)
  normal     the Normal pass (the relief, bevels included), tangent space because the panel faces +z
  orm        occlusion (the AO pass), roughness and metalness (two shader AOVs)
  emission   the Emit pass (backlit caps, lamps, lenses)
  beauty     the panel lit by a fixed key light, for the contact sheet only

Panel space is Blender's own: the panel lies in the xy plane facing +z, x across, y up, metres.
The cockpit set (build_cockpit.py) puts these maps on its console faces and bakes the cabin's
light and occlusion over them.
"""

import math
import os
import random

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
FONT_SANS = os.path.join(ROOT, "tools", "fonts", "DejaVuSans-Bold.ttf")
FONT_MONO = os.path.join(ROOT, "tools", "fonts", "DejaVuSansMono-Bold.ttf")


def srgb(h):
    """A hex colour as linear RGB."""
    h = h.lstrip("#")
    c = [int(h[i:i + 2], 16) / 255.0 for i in (0, 2, 4)]
    return tuple(x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c)


# Every surface a panel is made of. colour, roughness, metalness, then what the shader adds:
# grime (crevice darkening), wear (edges worn to bare metal), emit (a backlit colour), stripes
# (yellow and black hazard bands), weave (fabric). An interior style (build_cockpit STYLES) adds: wood (a varnished
# grain between two colours), panels (riveted wall plates with hatches, grilles, circuits and placards, and
# grime streaks: a lived-in wall), burl (a lacquered burl veneer between two colours), quilt (diamond tufted padding, its pitch in metres, darker stitched seams), grain
# (a leather's pebbled bump) and stipple (moulded plastic's fine texture).
ROLES = {
    "paint":    dict(c="#3b424a", r=0.58, m=0.0, grime=1.0, wear=1.0),
    "paint_lt": dict(c="#5d6670", r=0.55, m=0.0, grime=1.0, wear=1.0),
    "paint_dk": dict(c="#323840", r=0.6, m=0.0, grime=0.8, wear=0.8, panels=dict()),
    "bezel":    dict(c="#16181b", r=0.42, m=0.0, grime=0.5, wear=0.5),
    "rubber":   dict(c="#0d0d0e", r=0.92, m=0.0, grime=0.3),
    "metal":    dict(c="#a9aeb5", r=0.3, m=1.0, grime=0.6),
    "steel":    dict(c="#6e747b", r=0.42, m=1.0, grime=0.8),
    "brass":    dict(c="#b08a4a", r=0.35, m=1.0, grime=0.5),
    "stencil":  dict(c="#e9e6dc", r=0.6, m=0.0),
    "ink":      dict(c="#121314", r=0.6, m=0.0),
    "red":      dict(c="#a8191d", r=0.5, m=0.0, grime=0.6, wear=0.6),
    "yellow":   dict(c="#e0ac16", r=0.5, m=0.0, grime=0.6, wear=0.6),
    "hazard":   dict(c="#e0ac16", r=0.55, m=0.0, grime=0.8, wear=0.8, stripes=True),
    "placard":  dict(c="#d9d4c3", r=0.45, m=0.0, grime=0.5),
    "warnplate": dict(c="#b01e20", r=0.45, m=0.0, grime=0.5),
    "cautplate": dict(c="#e0ac16", r=0.45, m=0.0, grime=0.5),
    "pcb":      dict(c="#1c4d2e", r=0.3, m=0.0, grime=0.3),
    "copper":   dict(c="#c88743", r=0.3, m=1.0),
    "gold":     dict(c="#d8ab45", r=0.25, m=1.0),
    "solder":   dict(c="#c9cbcf", r=0.25, m=1.0),
    "chip":     dict(c="#1b1c1f", r=0.45, m=0.0),
    "ceramic":  dict(c="#c9b48a", r=0.5, m=0.0),
    "capblue":  dict(c="#28508f", r=0.35, m=0.0),
    "glass":    dict(c="#0b1013", r=0.08, m=0.0),
    "lens_off": dict(c="#2a2d30", r=0.2, m=0.0),
    "amber":    dict(c="#5a3a08", r=0.2, m=0.0, emit="#ffae2e"),
    "green":    dict(c="#0f3f1c", r=0.2, m=0.0, emit="#46ff7c"),
    "red_lit":  dict(c="#4a0a0a", r=0.2, m=0.0, emit="#ff3326"),
    "white_lit": dict(c="#5a5a55", r=0.2, m=0.0, emit="#fff4dc"),
    "cyan":     dict(c="#0b3a44", r=0.2, m=0.0, emit="#3fe6ff"),
    "fabric":   dict(c="#2c3036", r=0.9, m=0.0, grime=0.6, weave=True),
    # the sealed pod: its screen wall (a dim glow in the bake: the outside world lights the cabin)
    # and the outside armour, which the game lights live (its maps are the mech plating)
    "screen_wall": dict(c="#0a0d10", r=0.3, m=0.0, emit="#5f6f80"),
    "ext_armor": dict(c="#c3c7cd", r=0.5, m=0.0, grime=0.6, wear=0.6),
    "ext_frame": dict(c="#4b5059", r=0.5, m=0.3, grime=0.7, wear=0.5),
    "ext_dark": dict(c="#23272c", r=0.55, m=0.2, grime=0.6),
    "strap":    dict(c="#4b4f3f", r=0.85, m=0.0, grime=0.4, weave=True),
    # the cockpit-styles roles: the rails and edges an interior style dresses (the paint's colour unless a style or a
    # trim sets it), grab handles, and the safety kit's colours
    "trim":     dict(c="#3b424a", r=0.58, m=0.0, grime=1.0, wear=1.0),
    "handle":   dict(c="#8a9098", r=0.35, m=1.0, grime=0.4),
    "kit_white": dict(c="#e3dfd5", r=0.5, m=0.0, grime=0.5),
    "kit_orange": dict(c="#e2621a", r=0.7, m=0.0, grime=0.5),
    "kit_green": dict(c="#1f6b3a", r=0.5, m=0.0, grime=0.4),
}


PI = 3.141592653589793


def _nodes(mat):
    mat.use_nodes = True
    nt = mat.node_tree
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    return nt


def make_material(name, spec):
    """One role as a Cycles material: Principled BSDF with metallic left at 0 (the bake is diffuse
    and view independent: metalness is written to an AOV instead), grime from an AO node, worn
    edges from a Bevel node, bevelled shading normals, and AOV outputs `rough` and `metal`."""
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    nt = _nodes(mat)
    N, L = nt.nodes, nt.links
    out = N.new("ShaderNodeOutputMaterial")
    bsdf = N.new("ShaderNodeBsdfPrincipled")
    L.new(bsdf.outputs[0], out.inputs[0])
    base = list(srgb(spec["c"])) + [1.0]
    rough = spec["r"]
    col = N.new("ShaderNodeRGB")
    col.outputs[0].default_value = base
    colour = col.outputs[0]
    bev = N.new("ShaderNodeBevel")
    bev.samples = 8
    bev.inputs["Radius"].default_value = 0.0012
    L.new(bev.outputs[0], bsdf.inputs["Normal"])
    if spec.get("stripes"):
        # 45 degree yellow and black bands, 40 mm wide, in object space so each stripe plate
        # starts its own pattern
        tc = N.new("ShaderNodeTexCoord")
        sep = N.new("ShaderNodeSeparateXYZ")
        L.new(tc.outputs["Object"], sep.inputs[0])
        add = N.new("ShaderNodeMath"); add.operation = "ADD"
        L.new(sep.outputs[0], add.inputs[0]); L.new(sep.outputs[1], add.inputs[1])
        mul = N.new("ShaderNodeMath"); mul.operation = "MULTIPLY"; mul.inputs[1].default_value = 1.0 / 0.056
        L.new(add.outputs[0], mul.inputs[0])
        fr = N.new("ShaderNodeMath"); fr.operation = "FRACT"
        L.new(mul.outputs[0], fr.inputs[0])
        st = N.new("ShaderNodeMath"); st.operation = "GREATER_THAN"; st.inputs[1].default_value = 0.5
        L.new(fr.outputs[0], st.inputs[0])
        mix = N.new("ShaderNodeMix"); mix.data_type = "RGBA"
        L.new(st.outputs[0], mix.inputs["Factor"])
        mix.inputs[6].default_value = base
        mix.inputs[7].default_value = list(srgb("#141414")) + [1.0]
        colour = mix.outputs[2]
    if spec.get("weave"):
        # a fine fabric weave as a bump, 3 mm pitch
        wv = N.new("ShaderNodeTexWave"); wv.wave_type = "BANDS"; wv.inputs["Scale"].default_value = 330
        wv2 = N.new("ShaderNodeTexWave"); wv2.wave_type = "BANDS"; wv2.bands_direction = "Y"
        wv2.inputs["Scale"].default_value = 330
        wtc = N.new("ShaderNodeTexCoord")        # object space, metres, so a joined mesh keeps the pitch
        L.new(wtc.outputs["Object"], wv.inputs["Vector"]); L.new(wtc.outputs["Object"], wv2.inputs["Vector"])
        m = N.new("ShaderNodeMath"); m.operation = "MULTIPLY"
        L.new(wv.outputs[1], m.inputs[0]); L.new(wv2.outputs[1], m.inputs[1])
        bump = N.new("ShaderNodeBump"); bump.inputs["Strength"].default_value = 0.35
        bump.inputs["Distance"].default_value = 0.0006
        L.new(m.outputs[0], bump.inputs["Height"])
        L.new(bev.outputs[0], bump.inputs["Normal"])
        L.new(bump.outputs[0], bsdf.inputs["Normal"])
    normal = bump.outputs[0] if spec.get("weave") else bev.outputs[0]

    def add_bump(height, strength, distance):
        nonlocal normal
        b = N.new("ShaderNodeBump")
        b.inputs["Strength"].default_value = strength
        b.inputs["Distance"].default_value = distance
        L.new(height, b.inputs["Height"])
        L.new(normal, b.inputs["Normal"])
        L.new(b.outputs[0], bsdf.inputs["Normal"])
        normal = b.outputs[0]

    def op(kind, a, b=None):
        m = N.new("ShaderNodeMath")
        m.operation = kind
        for k, v in enumerate((a, b)):
            if v is None:
                continue
            if isinstance(v, (int, float)):
                m.inputs[k].default_value = v
            else:
                L.new(v, m.inputs[k])
        return m.outputs[0]

    def mix_col(fac, a, b):
        m = N.new("ShaderNodeMix")
        m.data_type = "RGBA"
        L.new(fac, m.inputs["Factor"])
        for sock, v in ((6, a), (7, b)):
            if isinstance(v, str):
                m.inputs[sock].default_value = list(srgb(v)) + [1.0]
            else:
                L.new(v, m.inputs[sock])
        return m.outputs[2]

    if spec.get("wood"):
        # Varnished wood: rings along the piece's longest run (object x), stretched, wavered by noise, between a dark
        # and a light tone, with a faint grain bump. Object space, metres.
        tc = N.new("ShaderNodeTexCoord")
        mp = N.new("ShaderNodeMapping")
        mp.inputs["Scale"].default_value = (1.0, 14.0, 14.0)
        L.new(tc.outputs["Object"], mp.inputs[0])
        wv = N.new("ShaderNodeTexWave")
        wv.wave_type = "RINGS"
        wv.inputs["Scale"].default_value = 1.6
        wv.inputs["Distortion"].default_value = 7.0
        wv.inputs["Detail"].default_value = 3.0
        wv.inputs["Detail Scale"].default_value = 1.5
        L.new(mp.outputs[0], wv.inputs["Vector"])
        fine = N.new("ShaderNodeTexNoise")
        fine.inputs["Scale"].default_value = 160.0
        fine.inputs["Detail"].default_value = 4.0
        L.new(mp.outputs[0], fine.inputs["Vector"])
        f = op("MULTIPLY", op("ADD", wv.outputs["Fac"], op("MULTIPLY", fine.outputs["Fac"], 0.35)), 0.75)
        colour = mix_col(f, spec["wood"][0], spec["wood"][1])
        add_bump(fine.outputs["Fac"], 0.08, 0.0004)
    if spec.get("burl"):
        # Burl veneer: a domain-warped noise figure between a dark and a light tone with dark knots (Voronoi), under a
        # lacquer (the role's low roughness). Object space, metres.
        tc = N.new("ShaderNodeTexCoord")
        warp = N.new("ShaderNodeTexNoise")
        warp.inputs["Scale"].default_value = 9.0
        warp.inputs["Detail"].default_value = 3.0
        L.new(tc.outputs["Object"], warp.inputs["Vector"])
        fig = N.new("ShaderNodeTexNoise")
        fig.inputs["Scale"].default_value = 14.0
        fig.inputs["Detail"].default_value = 6.0
        fig.inputs["Distortion"].default_value = 4.0
        L.new(warp.outputs["Color"], fig.inputs["Vector"])
        knot = N.new("ShaderNodeTexVoronoi")
        knot.inputs["Scale"].default_value = 40.0
        L.new(tc.outputs["Object"], knot.inputs["Vector"])
        k = op("MULTIPLY", op("SUBTRACT", 0.08, op("MINIMUM", knot.outputs["Distance"], 0.08)), 6.0)
        f = op("MAXIMUM", op("SUBTRACT", fig.outputs["Fac"], k), 0.0)
        colour = mix_col(f, spec["burl"][0], spec["burl"][1])
        add_bump(fig.outputs["Fac"], 0.04, 0.0003)
    if spec.get("quilt"):
        # Diamond tufting: on each face's own plane (picked by its normal, so walls, roof and seat all quilt), a
        # pillow sin(pi frac(u)) sin(pi frac(v)) on the diagonals u, v of a `quilt` metre grid, zero at the stitched
        # seams, which are darker.
        tc = N.new("ShaderNodeTexCoord")
        geo = N.new("ShaderNodeNewGeometry")
        sx = N.new("ShaderNodeSeparateXYZ")
        L.new(tc.outputs["Object"], sx.inputs[0])
        sn = N.new("ShaderNodeSeparateXYZ")
        L.new(geo.outputs["Normal"], sn.inputs[0])
        pitch = spec["quilt"]
        coords = (sx.outputs[0], sx.outputs[1], sx.outputs[2])
        height = None
        for ax in range(3):
            a_, b_ = [coords[k] for k in range(3) if k != ax]
            u = op("DIVIDE", op("ADD", a_, b_), pitch)
            v = op("DIVIDE", op("SUBTRACT", a_, b_), pitch)
            pu = op("SINE", op("MULTIPLY", op("FRACT", u), PI))
            pv = op("SINE", op("MULTIPLY", op("FRACT", v), PI))
            pillow = op("POWER", op("MULTIPLY", pu, pv), 0.45)
            w = op("POWER", op("ABSOLUTE", sn.outputs[ax]), 4.0)
            term = op("MULTIPLY", pillow, w)
            wsum = w if height is None else op("ADD", wsum, w)
            height = term if height is None else op("ADD", height, term)
        height = op("DIVIDE", height, op("MAXIMUM", wsum, 1e-4))
        seam = op("SUBTRACT", 1.0, op("MINIMUM", op("MULTIPLY", height, 4.0), 1.0))
        colour = mix_col(op("MULTIPLY", seam, 0.55), colour, "#0c0a09")
        add_bump(height, 0.9, 0.012)
    if spec.get("panels") is not None:
        # Lived-in wall panelling (owner, 2026-10-27: "more details baked into the wall panels so they aren't just flat
        # textures. Paneling - circuits, rivets, little small details"). On each face's own plane (weighted by its
        # normal, as the quilt), plates `w` x `h` metres with engraved seams and domed rivets along them; each plate
        # picks by a random number: an inset hatch, a louvred grille, an exposed circuit board with copper traces, a
        # stencilled placard, or plain. Heights go to the bump (the baked normal map), masks to the colour.
        P = spec["panels"]
        tw, th = P.get("w", 0.42), P.get("h", 0.30)
        rivets, circuits = P.get("rivets", True), P.get("circuits", True)
        tc = N.new("ShaderNodeTexCoord")
        geo = N.new("ShaderNodeNewGeometry")
        sx = N.new("ShaderNodeSeparateXYZ")
        L.new(tc.outputs["Object"], sx.inputs[0])
        sn = N.new("ShaderNodeSeparateXYZ")
        L.new(geo.outputs["Normal"], sn.inputs[0])
        coords = (sx.outputs[0], sx.outputs[1], sx.outputs[2])
        acc, wsum = {}, None

        def gt(x, y):
            return op("GREATER_THAN", x, y)

        def lt(x, y):
            return op("LESS_THAN", x, y)

        def band(x, lo, hi):
            return op("MULTIPLY", gt(x, lo), lt(x, hi))

        def mul(*xs):
            out = xs[0]
            for x in xs[1:]:
                out = op("MULTIPLY", out, x)
            return out

        for ax in range(3):
            a_, b_ = [coords[k] for k in range(3) if k != ax]      # b_ is Blender's up on a wall (its world Z)
            u, v = op("DIVIDE", a_, tw), op("DIVIDE", b_, th)
            fu, fv = op("FRACT", u), op("FRACT", v)
            du = op("MULTIPLY", op("MINIMUM", fu, op("SUBTRACT", 1.0, fu)), tw)
            dv = op("MULTIPLY", op("MINIMUM", fv, op("SUBTRACT", 1.0, fv)), th)
            dmin = op("MINIMUM", du, dv)
            seam = lt(dmin, 0.0022)
            cid = N.new("ShaderNodeCombineXYZ")
            L.new(op("FLOOR", u), cid.inputs[0])
            L.new(op("FLOOR", v), cid.inputs[1])
            cid.inputs[2].default_value = ax * 7.0
            wn = N.new("ShaderNodeTexWhiteNoise")
            wn.noise_dimensions = "3D"
            L.new(cid.outputs[0], wn.inputs["Vector"])
            rnd = wn.outputs["Value"]
            inner = mul(gt(du, 0.05), gt(dv, 0.05))
            # screw holes (owner, 2026-10-27: "Not like large rivets but like screw holes"): 5 mm recessed heads with a
            # cross slot, 12 mm in from each seam, every 100 mm along it
            if rivets:
                pa = op("MULTIPLY", op("SUBTRACT", op("FRACT", op("DIVIDE", a_, 0.1)), 0.5), 0.1)
                pb = op("MULTIPLY", op("SUBTRACT", op("FRACT", op("DIVIDE", b_, 0.1)), 0.5), 0.1)
                cu = op("SUBTRACT", du, 0.012)
                cv = op("SUBTRACT", dv, 0.012)
                d1 = op("SQRT", op("ADD", op("POWER", cu, 2.0), op("POWER", pb, 2.0)))
                d2 = op("SQRT", op("ADD", op("POWER", cv, 2.0), op("POWER", pa, 2.0)))
                hole1, hole2 = lt(d1, 0.0026), lt(d2, 0.0026)
                cross1 = op("MULTIPLY", hole1, lt(op("MINIMUM", op("ABSOLUTE", cu), op("ABSOLUTE", pb)), 0.00045))
                cross2 = op("MULTIPLY", hole2, lt(op("MINIMUM", op("ABSOLUTE", cv), op("ABSOLUTE", pa)), 0.00045))
                rivet = op("MAXIMUM", hole1, hole2)                     # the screw's recess
                slotx = op("MAXIMUM", cross1, cross2)                   # its cross slot, deeper
            else:
                rivet = slotx = op("MULTIPLY", seam, 0.0)
            hatch = mul(lt(rnd, 0.22), lt(op("ABSOLUTE", op("SUBTRACT", dmin, 0.05)), 0.0018))
            slots = mul(band(rnd, 0.22, 0.35), inner, lt(op("FRACT", op("DIVIDE", b_, 0.014)), 0.45))
            edge = mul(gt(dmin, 0.0022), lt(dmin, 0.0045))          # the plate's bevelled edge, catching the light
            if circuits:
                board = mul(band(rnd, 0.35, 0.47), gt(du, 0.04), gt(dv, 0.04))
                tn = N.new("ShaderNodeTexNoise")
                tn.inputs["Scale"].default_value = 30.0
                L.new(tc.outputs["Object"], tn.inputs["Vector"])
                lines = op("MAXIMUM", lt(op("FRACT", op("MULTIPLY", a_, 40.0)), 0.16), lt(op("FRACT", op("MULTIPLY", b_, 40.0)), 0.16))
                traces = mul(board, lines, gt(tn.outputs["Fac"], 0.48))
            else:
                board = traces = op("MULTIPLY", seam, 0.0)
            placard = mul(band(rnd, 0.47, 0.6), band(fv, 0.72, 0.92), gt(du, 0.05))
            tn2 = N.new("ShaderNodeTexNoise")
            tn2.inputs["Scale"].default_value = 90.0
            L.new(tc.outputs["Object"], tn2.inputs["Vector"])
            ink = mul(placard, band(fv, 0.77, 0.87), lt(op("FRACT", op("MULTIPLY", a_, 70.0)), 0.5), gt(tn2.outputs["Fac"], 0.45))
            height = op("ADD", op("SUBTRACT", op("SUBTRACT", op("SUBTRACT", 0.5, op("MULTIPLY", seam, 0.5)),
                                                          op("MULTIPLY", hatch, 0.4)), op("MULTIPLY", slots, 0.35)),
                         op("ADD", op("MULTIPLY", rivet, 0.12), op("ADD", op("MULTIPLY", slotx, -0.45),
                                                                    op("ADD", op("MULTIPLY", traces, 0.15), op("MULTIPLY", placard, 0.08)))))
            w = op("POWER", op("ABSOLUTE", sn.outputs[ax]), 4.0)
            for key, val in (("h", height), ("seam", op("MAXIMUM", seam, hatch)), ("slots", slots), ("rivet", rivet), ("edge", edge), ("slotx", slotx),
                             ("board", board), ("traces", traces), ("placard", placard), ("ink", ink)):
                term = op("MULTIPLY", val, w)
                acc[key] = term if key not in acc else op("ADD", acc[key], term)
            wsum = w if wsum is None else op("ADD", wsum, w)
        ws = op("MAXIMUM", wsum, 1e-4)
        m = {k: op("DIVIDE", v, ws) for k, v in acc.items()}
        colour = mix_col(m["board"], colour, "#16301f")
        colour = mix_col(m["traces"], colour, "#b8813f")
        colour = mix_col(m["placard"], colour, "#d6d0bd")
        colour = mix_col(m["ink"], colour, "#161616")
        colour = mix_col(op("MULTIPLY", m["seam"], 0.85), colour, "#070707")
        colour = mix_col(op("MULTIPLY", m["slots"], 0.7), colour, "#0a0a0a")
        colour = mix_col(op("MULTIPLY", m["edge"], 0.35), colour, "#8d949b")
        colour = mix_col(op("MULTIPLY", m["rivet"], 0.85), colour, "#7c838b")      # brushed steel screw heads
        colour = mix_col(m["slotx"], colour, "#0d0d0d")                           # their cross slots
        # grime streaks running down the wall: noise stretched along the up axis, darkening
        sm = N.new("ShaderNodeMapping")
        sm.inputs["Scale"].default_value = (14.0, 14.0, 0.7)
        L.new(tc.outputs["Object"], sm.inputs[0])
        st = N.new("ShaderNodeTexNoise")
        st.inputs["Scale"].default_value = 3.0
        st.inputs["Detail"].default_value = 6.0
        L.new(sm.outputs[0], st.inputs["Vector"])
        streak = op("MULTIPLY", op("MAXIMUM", op("SUBTRACT", st.outputs["Fac"], 0.55), 0.0), 1.4 * P.get("streaks", 1.0))
        colour = mix_col(streak, colour, "#1a1712")
        add_bump(m["h"], 1.0, 0.0022)
    if spec.get("grain"):
        # a leather's pebbled grain
        gv = N.new("ShaderNodeTexVoronoi")
        gv.inputs["Scale"].default_value = 900.0
        add_bump(gv.outputs["Distance"], 0.12, 0.0003)
    if spec.get("stipple"):
        # moulded plastic's fine stipple
        sp = N.new("ShaderNodeTexNoise")
        sp.inputs["Scale"].default_value = 700.0
        sp.inputs["Detail"].default_value = 2.0
        add_bump(sp.outputs["Fac"], 0.08 * spec["stipple"], 0.0003)
    noise = N.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 60.0
    noise.inputs["Detail"].default_value = 6.0
    if spec.get("wear"):
        # Edge wear: where the bevelled normal leaves the true normal, chipped paint shows the
        # steel under it, broken up by noise.
        geo = N.new("ShaderNodeNewGeometry")
        dot = N.new("ShaderNodeVectorMath"); dot.operation = "DOT_PRODUCT"
        L.new(bev.outputs[0], dot.inputs[0]); L.new(geo.outputs["Normal"], dot.inputs[1])
        e = N.new("ShaderNodeMapRange")
        e.inputs["From Min"].default_value = 0.998
        e.inputs["From Max"].default_value = 0.96
        L.new(dot.outputs["Value"], e.inputs["Value"])
        nz = N.new("ShaderNodeMapRange")
        nz.inputs["From Min"].default_value = 0.36
        nz.inputs["From Max"].default_value = 0.56
        L.new(noise.outputs["Fac"], nz.inputs["Value"])
        w = N.new("ShaderNodeMath"); w.operation = "MULTIPLY"
        L.new(e.outputs[0], w.inputs[0]); L.new(nz.outputs[0], w.inputs[1])
        w2 = N.new("ShaderNodeMath"); w2.operation = "MULTIPLY"; w2.inputs[1].default_value = spec["wear"]
        L.new(w.outputs[0], w2.inputs[0])
        mix = N.new("ShaderNodeMix"); mix.data_type = "RGBA"
        L.new(w2.outputs[0], mix.inputs["Factor"])
        L.new(colour, mix.inputs[6])
        mix.inputs[7].default_value = list(srgb("#8d9298")) + [1.0]
        colour = mix.outputs[2]
    if spec.get("grime"):
        # Crevice grime: occluded within 12 mm darkens toward a dirty brown-grey, plus a faint
        # mottle so a big plate is not one flat colour.
        ao = N.new("ShaderNodeAmbientOcclusion")
        ao.samples = 12
        ao.inputs["Distance"].default_value = 0.012
        g = N.new("ShaderNodeMapRange")
        g.inputs["From Min"].default_value = 1.0
        g.inputs["From Max"].default_value = 0.5
        g.inputs["To Max"].default_value = 0.9 * spec["grime"]
        L.new(ao.outputs["AO"], g.inputs["Value"])
        mix = N.new("ShaderNodeMix"); mix.data_type = "RGBA"
        L.new(g.outputs[0], mix.inputs["Factor"])
        L.new(colour, mix.inputs[6])
        mix.inputs[7].default_value = list(srgb("#17140f")) + [1.0]
        mot = N.new("ShaderNodeMapRange")
        mot.inputs["To Min"].default_value = 0.0
        mot.inputs["To Max"].default_value = 0.24 * spec["grime"]
        n2 = N.new("ShaderNodeTexNoise"); n2.inputs["Scale"].default_value = 9.0
        n2.inputs["Detail"].default_value = 4.0
        L.new(n2.outputs["Fac"], mot.inputs["Value"])
        mix2 = N.new("ShaderNodeMix"); mix2.data_type = "RGBA"
        L.new(mot.outputs[0], mix2.inputs["Factor"])
        L.new(mix.outputs[2], mix2.inputs[6])
        mix2.inputs[7].default_value = list(srgb("#2a2620")) + [1.0]
        colour = mix2.outputs[2]
    if spec.get("wear"):
        # fine scuffs: a noise stretched along x, thresholded into faint light streaks
        tc = N.new("ShaderNodeTexCoord")
        mp = N.new("ShaderNodeMapping")
        mp.inputs["Scale"].default_value = (4.0, 90.0, 4.0)
        L.new(tc.outputs["Object"], mp.inputs[0])
        sn = N.new("ShaderNodeTexNoise"); sn.inputs["Scale"].default_value = 6.0; sn.inputs["Detail"].default_value = 8.0
        L.new(mp.outputs[0], sn.inputs["Vector"])
        sr = N.new("ShaderNodeMapRange")
        sr.inputs["From Min"].default_value = 0.66
        sr.inputs["From Max"].default_value = 0.74
        sr.inputs["To Max"].default_value = 0.35 * spec["wear"]
        L.new(sn.outputs["Fac"], sr.inputs["Value"])
        mix = N.new("ShaderNodeMix"); mix.data_type = "RGBA"
        L.new(sr.outputs[0], mix.inputs["Factor"])
        L.new(colour, mix.inputs[6])
        mix.inputs[7].default_value = list(srgb("#9aa0a7")) + [1.0]
        colour = mix.outputs[2]
    L.new(colour, bsdf.inputs["Base Color"])
    bsdf.inputs["Metallic"].default_value = 0.0
    bsdf.inputs["Roughness"].default_value = rough
    if spec.get("emit"):
        bsdf.inputs["Emission Color"].default_value = list(srgb(spec["emit"])) + [1.0]
        bsdf.inputs["Emission Strength"].default_value = 1.0
    for aov, v in (("rough", rough), ("metal", spec["m"])):
        o = N.new("ShaderNodeOutputAOV")
        o.aov_name = aov
        o.inputs["Value"].default_value = v
    # sources the PBR bake reads through an emission swap (build_cockpit.bake_swap)
    mv = N.new("ShaderNodeValue")
    mv.name = "METAL_SRC"
    mv.outputs[0].default_value = spec["m"]
    av = N.new("ShaderNodeValue")
    av.name = "AO_SRC"
    av.outputs[0].default_value = 1.0
    return mat


def make_materials():
    return {k: make_material(k, v) for k, v in ROLES.items()}


# ----------------------------------------------------------------------------- the panel

class Panel:
    """A panel being modelled: every piece is an object in this panel's collection, made of
    roles from ROLES. Units are metres; the face of the panel is z = 0."""

    def __init__(self, name, w, h, mats, seed=1):
        self.name, self.w, self.h, self.mats = name, w, h, mats
        self.coll = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(self.coll)
        self.n = 0
        self.rng = random.Random(seed)
        self.font_sans = bpy.data.fonts.load(FONT_SANS, check_existing=True)
        self.font_mono = bpy.data.fonts.load(FONT_MONO, check_existing=True)

    def _ob(self, bm, role, bevel=0.0, smooth=False):
        self.n += 1
        me = bpy.data.meshes.new(f"{self.name}_{self.n}")
        if smooth:
            for f in bm.faces:
                f.smooth = True
        bm.to_mesh(me)
        bm.free()
        me.materials.append(self.mats[role])
        ob = bpy.data.objects.new(me.name, me)
        self.coll.objects.link(ob)
        if bevel:
            m = ob.modifiers.new("bevel", "BEVEL")
            m.width = bevel
            m.segments = 2
            m.limit_method = "ANGLE"
            m.angle_limit = math.radians(40)
        return ob

    # --- primitives

    def box(self, x0, y0, x1, y1, z0, z1, role, bevel=0.0, rot=0.0):
        """An axis box in panel space; rot turns it about z through its centre (radians)."""
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        cx, cy, cz = (x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2
        m = Matrix.Translation((cx, cy, cz)) @ Matrix.Rotation(rot, 4, "Z") @ Matrix.Diagonal(
            (abs(x1 - x0), abs(y1 - y0), abs(z1 - z0), 1.0))
        bmesh.ops.transform(bm, matrix=m, verts=bm.verts)
        return self._ob(bm, role, bevel)

    def rect(self, cx, cy, w, h, z0, z1, role, bevel=0.0, rot=0.0):
        return self.box(cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2, z0, z1, role, bevel, rot)

    def cyl(self, x, y, r, z0, z1, role, seg=32, r1=None, bevel=0.0):
        """A cylinder (or a frustum with top radius r1) standing on the panel."""
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=seg, radius1=r,
                              radius2=r if r1 is None else r1, depth=z1 - z0,
                              matrix=Matrix.Translation((x, y, (z0 + z1) / 2)))
        return self._ob(bm, role, bevel, smooth=seg > 8)

    def poly(self, pts, z0, z1, role, bevel=0.0):
        """A flat profile (x, y), counter clockwise, extruded from z0 to z1."""
        bm = bmesh.new()
        lo = [bm.verts.new((p[0], p[1], z0)) for p in pts]
        hi = [bm.verts.new((p[0], p[1], z1)) for p in pts]
        bm.faces.new(list(reversed(lo)))
        bm.faces.new(hi)
        n = len(pts)
        for i in range(n):
            j = (i + 1) % n
            bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        return self._ob(bm, role, bevel)

    def ring(self, x, y, r_out, r_in, z0, z1, role, seg=48):
        pts_o = [(x + r_out * math.cos(2 * math.pi * i / seg), y + r_out * math.sin(2 * math.pi * i / seg)) for i in range(seg)]
        pts_i = [(x + r_in * math.cos(2 * math.pi * i / seg), y + r_in * math.sin(2 * math.pi * i / seg)) for i in range(seg)]
        bm = bmesh.new()
        lo_o = [bm.verts.new((p[0], p[1], z0)) for p in pts_o]
        hi_o = [bm.verts.new((p[0], p[1], z1)) for p in pts_o]
        lo_i = [bm.verts.new((p[0], p[1], z0)) for p in pts_i]
        hi_i = [bm.verts.new((p[0], p[1], z1)) for p in pts_i]
        for i in range(seg):
            j = (i + 1) % seg
            bm.faces.new((hi_o[i], hi_o[j], hi_i[j], hi_i[i]))
            bm.faces.new((lo_o[i], lo_i[i], lo_i[j], lo_o[j]))
            bm.faces.new((lo_o[i], lo_o[j], hi_o[j], hi_o[i]))
            bm.faces.new((lo_i[i], hi_i[i], hi_i[j], lo_i[j]))
        return self._ob(bm, role, smooth=True)

    def tube(self, pts, r, role, seg=10):
        """A cable or pipe along points (x, y, z), as a bevelled curve converted to a mesh."""
        self.n += 1
        cu = bpy.data.curves.new(f"{self.name}_c{self.n}", "CURVE")
        cu.dimensions = "3D"
        cu.bevel_depth = r
        cu.bevel_resolution = 3
        cu.use_fill_caps = True
        sp = cu.splines.new("BEZIER")
        sp.bezier_points.add(len(pts) - 1)
        for bp, p in zip(sp.bezier_points, pts):
            bp.co = p
            bp.handle_left_type = bp.handle_right_type = "AUTO"
        cu.materials.append(self.mats[role])
        ob = bpy.data.objects.new(cu.name, cu)
        self.coll.objects.link(ob)
        return ob

    def text(self, body, x, y, size, role="stencil", z=0.0, align="CENTER", mono=False, rot=0.0, depth=0.0004):
        """Stencilled or engraved lettering laid on a face at height z."""
        self.n += 1
        cu = bpy.data.curves.new(f"{self.name}_t{self.n}", "FONT")
        cu.body = body
        cu.font = self.font_mono if mono else self.font_sans
        cu.size = size
        cu.align_x = align
        cu.align_y = "CENTER"
        cu.extrude = depth / 2
        cu.materials.append(self.mats[role])
        ob = bpy.data.objects.new(cu.name, cu)
        ob.location = (x, y, z + depth / 2 + 0.0002)
        ob.rotation_euler = (0.0, 0.0, rot)
        self.coll.objects.link(ob)
        return ob

    # --- furniture

    def plates(self, rects, z=0.0, t=0.006, gap=0.0035, role="paint", bevel=0.0015):
        """The panel face cut into plates with seams between them: each rect (x0, y0, x1, y1) is
        one plate, shrunk by half the gap, standing on a dark backing so every seam is a real
        groove the light and the occlusion find."""
        self.box(-self.w / 2, -self.h / 2, self.w / 2, self.h / 2, z - t - 0.01, z - t + 0.0005, "paint_dk")
        for (x0, y0, x1, y1) in rects:
            g = gap / 2
            self.box(x0 + g, y0 + g, x1 - g, y1 - g, z - t, z, role, bevel)

    def screw(self, x, y, z=0.0, r=0.0042, role="steel"):
        self.cyl(x, y, r, z, z + 0.0018, role, seg=16, r1=r * 0.9)
        a = self.rng.uniform(0, math.pi)
        self.rect(x, y, r * 1.6, r * 0.34, z + 0.0016, z + 0.0021, "ink", rot=a)

    def frame(self, x0, y0, x1, y1, w, z0, z1, role, bevel=0.0):
        """A rectangular frame of width w round (x0, y0, x1, y1), outside it."""
        self.box(x0 - w, y1, x1 + w, y1 + w, z0, z1, role, bevel)
        self.box(x0 - w, y0 - w, x1 + w, y0, z0, z1, role, bevel)
        self.box(x0 - w, y0, x0, y1, z0, z1, role, bevel)
        self.box(x1, y0, x1 + w, y1, z0, z1, role, bevel)

    def window(self, x0, y0, x1, y1, seed, density=1.0):
        """An inspection window onto a live circuit board: the board on the plate, a rubber gasket
        and a painted frame round it."""
        self.pcb(x0, y0, x1, y1, z=-0.004, density=density, seed=seed)
        self.frame(x0, y0, x1, y1, 0.004, 0.0, 0.006, "rubber")
        self.frame(x0 - 0.004, y0 - 0.004, x1 + 0.004, y1 + 0.004, 0.014, 0.0, 0.009, "paint_dk", bevel=0.0015)

    def code(self, body, x, y, size=0.0058, z=0.0, align="LEFT"):
        """A small stencilled panel number or part code."""
        return self.text(body, x, y, size, "stencil", z=z, align=align, mono=True)

    def screw_row(self, x0, y0, x1, y1, n, z=0.0):
        for i in range(n):
            t = i / max(1, n - 1)
            self.screw(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z, r=0.0032)

    def screws(self, x0, y0, x1, y1, z=0.0, inset=0.012):
        for x, y in ((x0 + inset, y0 + inset), (x1 - inset, y0 + inset), (x0 + inset, y1 - inset), (x1 - inset, y1 - inset)):
            self.screw(x, y, z)

    def button(self, x, y, w, h, cap="lens_off", z=0.0, label=None, label_size=0.0075, legend=None):
        """A square push button: a black bezel, a raised cap (a lit lens if cap is an emissive
        role), an optional legend on the cap and a label under it."""
        self.rect(x, y, w + 0.006, h + 0.006, z, z + 0.004, "bezel", bevel=0.0012)
        self.rect(x, y, w, h, z + 0.002, z + 0.0085, cap, bevel=0.0012)
        if legend:
            self.text(legend, x, y, min(h * 0.32, 0.0085), "ink" if ROLES[cap].get("emit") else "stencil",
                      z=z + 0.0085, mono=True)
        if label:
            self.text(label, x, y - h / 2 - 0.0085, label_size, "stencil", z=z)

    def toggle(self, x, y, z=0.0, up=True, guard=None, label=None, top=None, bottom=None, lever_role="metal"):
        """A bat-handle toggle switch in a hex nut, lever thrown up or down; guard 'red' or
        'yellow' adds a hinged cover frame round it."""
        self.cyl(x, y, 0.0075, z, z + 0.004, "steel", seg=6)
        self.cyl(x, y, 0.0045, z + 0.004, z + 0.008, "metal", seg=20)
        tilt = 0.45 if up else -0.45
        L = 0.022
        self.n += 1
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=16, radius1=0.0022, radius2=0.0032, depth=L,
                              matrix=Matrix.Translation((0, 0, L / 2)))
        bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation((x, y, z + 0.007)) @ Matrix.Rotation(-tilt, 4, "X"))
        self._ob(bm, lever_role, smooth=True)
        if guard:
            gw, gh = 0.026, 0.05
            for sx in (-1, 1):
                self.box(x + sx * gw / 2 - 0.002, y - gh / 2, x + sx * gw / 2 + 0.002, y + gh / 2, z, z + 0.028, guard, bevel=0.0012)
            self.box(x - gw / 2 - 0.002, y + gh / 2 - 0.004, x + gw / 2 + 0.002, y + gh / 2, z, z + 0.028, guard, bevel=0.0012)
            self.box(x - gw / 2 - 0.002, y - gh / 2, x + gw / 2 + 0.002, y - gh / 2 + 0.003, z, z + 0.006, guard, bevel=0.001)
        if label:
            self.text(label, x, y - 0.036, 0.0075, "stencil", z=z)
        if top:
            self.text(top, x, y + 0.03, 0.0062, "stencil", z=z)
        if bottom:
            self.text(bottom, x, y - 0.025, 0.0062, "stencil", z=z)

    def knob(self, x, y, r=0.014, z=0.0, label=None, ticks=None, pointer=0.0, cap="bezel"):
        """A rotary knob: skirt, knurled cap, a white pointer line, and engraved detents round it."""
        self.cyl(x, y, r * 1.25, z, z + 0.003, "steel", seg=40)
        n = 28
        pts = []
        for i in range(n * 2):
            a = math.pi * i / n
            rr = r if i % 2 == 0 else r * 0.93
            pts.append((x + rr * math.cos(a), y + rr * math.sin(a)))
        self.poly(pts, z + 0.003, z + 0.016, cap, bevel=0.0008)
        self.cyl(x, y, r * 0.75, z + 0.016, z + 0.0175, cap, seg=32)
        self.rect(x + math.sin(pointer) * r * 0.45, y + math.cos(pointer) * r * 0.45, 0.0016, r * 0.75,
                  z + 0.0172, z + 0.0182, "stencil", rot=-pointer)
        if ticks:
            k = len(ticks)
            for i, t in enumerate(ticks):
                a = -0.9 * math.pi / 2 + 0.9 * math.pi * i / max(1, k - 1)
                tx, ty = x + math.sin(a) * r * 1.75, y + math.cos(a) * r * 1.75
                self.rect(tx, ty, 0.0012, 0.004, z, z + 0.0004, "stencil", rot=-a)
                if t:
                    self.text(t, x + math.sin(a) * r * 2.35, y + math.cos(a) * r * 2.35, 0.0055, "stencil", z=z)
        if label:
            self.text(label, x, y - r * 2.6, 0.0075, "stencil", z=z)

    def dial(self, x, y, r, z=0.0, label="", lo=0, hi=10, needle=0.3, red_from=None, units=""):
        """A round analogue gauge: bezel ring, black face, ticks and numbers, a red band, a needle."""
        self.ring(x, y, r * 1.18, r * 0.98, z, z + 0.008, "bezel")
        self.cyl(x, y, r, z, z + 0.002, "ink", seg=48)
        for i in range(41):
            t = i / 40
            a = -0.75 * math.pi + 1.5 * math.pi * t
            major = i % 5 == 0
            L = r * (0.16 if major else 0.08)
            rr = r * 0.9 - L / 2
            col = "red_lit" if red_from is not None and t >= red_from else "stencil"
            self.rect(x + math.sin(a) * rr, y + math.cos(a) * rr, 0.0011 if major else 0.0007, L, z + 0.002, z + 0.0026, col, rot=-a)
            if major and i % 10 == 0:
                v = lo + (hi - lo) * t
                self.text(f"{v:g}", x + math.sin(a) * r * 0.6, y + math.cos(a) * r * 0.6, r * 0.17, "stencil", z=z + 0.002)
        self.text(label, x, y - r * 0.38, r * 0.15, "stencil", z=z + 0.002)
        if units:
            self.text(units, x, y + r * 0.3, r * 0.12, "stencil", z=z + 0.002)
        a = -0.75 * math.pi + 1.5 * math.pi * needle
        self.rect(x + math.sin(a) * r * 0.4, y + math.cos(a) * r * 0.4, 0.0018, r * 0.85, z + 0.003, z + 0.0042, "white_lit", rot=-a)
        self.cyl(x, y, r * 0.09, z + 0.002, z + 0.0055, "steel", seg=20)

    def keypad(self, x0, y0, cols, rows, pitch, legends, z=0.0, lit=()):
        for r in range(rows):
            for c in range(cols):
                i = r * cols + c
                lg = legends[i] if i < len(legends) else ""
                cap = "amber" if (c, r) in lit else "lens_off"
                self.button(x0 + c * pitch, y0 - r * pitch, pitch * 0.72, pitch * 0.72, cap, z=z, legend=lg)

    def placard(self, x, y, w, h, lines, role="placard", ink="ink", z=0.0, size=None, frame=None):
        """A riveted plate with lettering: a warning (role warnplate, ink stencil), a caution
        (cautplate) or an instruction (placard)."""
        self.rect(x, y, w, h, z, z + 0.0012, role, bevel=0.0006)
        if frame:
            for (a, b, c, d) in ((x - w / 2 + 0.002, y - h / 2 + 0.002, x + w / 2 - 0.002, y - h / 2 + 0.0035),
                                 (x - w / 2 + 0.002, y + h / 2 - 0.0035, x + w / 2 - 0.002, y + h / 2 - 0.002)):
                self.box(a, b, c, d, z + 0.0012, z + 0.0016, frame)
        s = size or min(h / (len(lines) + 1) * 0.8, 0.012)
        for i, ln in enumerate(lines):
            yy = y + (len(lines) - 1) / 2 * s * 1.35 - i * s * 1.35
            self.text(ln, x, yy, s, ink, z=z + 0.0012)
        for sx in (-1, 1):
            self.cyl(x + sx * (w / 2 - 0.004), y, 0.0016, z + 0.0012, z + 0.0022, "steel", seg=10)

    def hazard(self, x0, y0, x1, y1, z=0.0, t=0.0015):
        return self.box(x0, y0, x1, y1, z, z + t, "hazard", bevel=0.0006)

    def annunciator(self, x0, y1, cols, rows, cw, ch, cells, z=0.0):
        """A caution and warning panel: a grid of lenses, each with its legend; cells maps (c, r)
        to (text, lit role or None)."""
        self.box(x0 - 0.004, y1 - rows * ch - 0.004, x0 + cols * cw + 0.004, y1 + 0.004, z, z + 0.003, "bezel", bevel=0.001)
        for r in range(rows):
            for c in range(cols):
                txt, lit = cells.get((c, r), ("", None))
                cx, cy = x0 + (c + 0.5) * cw, y1 - (r + 0.5) * ch
                self.rect(cx, cy, cw - 0.003, ch - 0.003, z + 0.003, z + 0.0055, lit or "lens_off", bevel=0.0006)
                if txt:
                    for i, part in enumerate(txt.split("\n")):
                        k = len(txt.split("\n"))
                        self.text(part, cx, cy + ((k - 1) / 2 - i) * ch * 0.3, ch * 0.24, "ink" if lit else "stencil",
                                  z=z + 0.0055, mono=True)

    def vent(self, x0, y0, x1, y1, n, z=0.0):
        """Louvres: a dark well with slats laid at an angle."""
        self.box(x0, y0, x1, y1, z - 0.004, z + 0.0015, "paint_dk")
        h = (y1 - y0) / n
        for i in range(n):
            yy = y0 + (i + 0.5) * h
            self.n += 1
            bm = bmesh.new()
            bmesh.ops.create_cube(bm, size=1.0)
            bmesh.ops.transform(bm, verts=bm.verts, matrix=Matrix.Translation(((x0 + x1) / 2, yy, z + 0.002))
                                @ Matrix.Rotation(0.6, 4, "X") @ Matrix.Diagonal((x1 - x0 - 0.004, h * 0.9, 0.0016, 1)))
            self._ob(bm, "paint")

    # --- circuitry

    def pcb(self, x0, y0, x1, y1, z=0.0, density=1.0, seed=7):
        """An exposed circuit board: green board on standoffs, chips with pins, capacitors,
        resistors, copper traces routed in 45 degree runs between pads, vias, silkscreen."""
        rng = random.Random(seed)
        zb = z + 0.006
        self.box(x0, y0, x1, y1, z + 0.004, zb, "pcb", bevel=0.0006)
        for (a, b) in ((x0 + 0.008, y0 + 0.008), (x1 - 0.008, y0 + 0.008), (x0 + 0.008, y1 - 0.008), (x1 - 0.008, y1 - 0.008)):
            self.cyl(a, b, 0.004, zb, zb + 0.0012, "solder", seg=16)
            self.cyl(a, b, 0.0022, zb + 0.0012, zb + 0.0024, "steel", seg=12)
        W, H = x1 - x0, y1 - y0
        parts = []

        def free(r):
            for q in parts:
                if r[0] < q[2] + 0.006 and r[2] > q[0] - 0.006 and r[1] < q[3] + 0.006 and r[3] > q[1] - 0.006:
                    return False
            return x0 + 0.016 < r[0] and r[2] < x1 - 0.016 and y0 + 0.016 < r[1] and r[3] < y1 - 0.016

        pads = []
        # chips
        for _ in range(int(14 * density * W * H / 0.04)):
            cw, ch = rng.choice(((0.03, 0.03), (0.022, 0.014), (0.04, 0.018), (0.016, 0.016)))
            if rng.random() < 0.5:
                cw, ch = ch, cw
            cx, cy = rng.uniform(x0, x1), rng.uniform(y0, y1)
            r = (cx - cw / 2, cy - ch / 2, cx + cw / 2, cy + ch / 2)
            if not free(r):
                continue
            parts.append(r)
            self.box(*r[:2], *r[2:], zb, zb + 0.003, "chip", bevel=0.0004)
            self.cyl(r[0] + 0.0035, r[3] - 0.0035, 0.001, zb + 0.003, zb + 0.0032, "steel", seg=10)
            pitch = 0.0024
            nx = max(2, int((cw - 0.004) / pitch))
            for i in range(nx):
                px = r[0] + 0.002 + i * (cw - 0.004) / max(1, nx - 1)
                for sy, yy in ((-1, r[1]), (1, r[3])):
                    self.box(px - 0.0006, yy - 0.0004 if sy > 0 else yy - 0.0022, px + 0.0006, yy + 0.0022 if sy > 0 else yy + 0.0004,
                             zb, zb + 0.0009, "solder")
                    pads.append((px, yy + sy * 0.0035))
            if rng.random() < 0.6:
                self.text(rng.choice(("U7", "U12", "IC3", "MX-4", "REG", "U21", "CPU")), (r[0] + r[2]) / 2, (r[1] + r[3]) / 2,
                          min(cw, ch) * 0.28, "stencil", z=zb + 0.003, mono=True, depth=0.0002)
        # capacitors and resistors
        for _ in range(int(30 * density * W * H / 0.04)):
            cx, cy = rng.uniform(x0, x1), rng.uniform(y0, y1)
            if rng.random() < 0.35:
                rr = rng.choice((0.004, 0.0055, 0.007))
                r = (cx - rr, cy - rr, cx + rr, cy + rr)
                if not free(r):
                    continue
                parts.append(r)
                self.cyl(cx, cy, rr, zb, zb + rr * 2.4, rng.choice(("capblue", "chip", "capblue")), seg=24)
                self.cyl(cx, cy, rr * 0.85, zb + rr * 2.4, zb + rr * 2.45, "solder", seg=24)
                pads.append((cx + rr + 0.002, cy))
            else:
                horiz = rng.random() < 0.5
                lw, lh = (0.009, 0.0035) if horiz else (0.0035, 0.009)
                r = (cx - lw / 2, cy - lh / 2, cx + lw / 2, cy + lh / 2)
                if not free(r):
                    continue
                parts.append(r)
                self.box(*r[:2], *r[2:], zb, zb + 0.0018, rng.choice(("ceramic", "chip")), bevel=0.0003)
                for s in (-1, 1):
                    px = cx + s * lw / 2 if horiz else cx
                    py = cy if horiz else cy + s * lh / 2
                    self.rect(px, py, 0.0022, 0.0022, zb, zb + 0.0008, "solder")
                    pads.append((px, py))
        def run(pts, w):
            for (a, b) in zip(pts, pts[1:]):
                dx, dy = b[0] - a[0], b[1] - a[1]
                L = math.hypot(dx, dy)
                if L > 1e-4:
                    self.rect((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, L + w, w, zb, zb + 0.0004, "copper", rot=math.atan2(dy, dx))

        # buses: parallel traces between neighbouring chips, straight out, a 45 degree jog, in
        chips = sorted((q for q in parts if q[2] - q[0] > 0.012 and q[3] - q[1] > 0.012), key=lambda q: q[0])
        for a, b in zip(chips, chips[1:]):
            k = rng.randint(4, 8)
            pitch = 0.0022
            ya = (a[1] + a[3]) / 2 - (k - 1) * pitch / 2
            yb = (b[1] + b[3]) / 2 - (k - 1) * pitch / 2
            sx, ex = a[2], b[0]
            if ex - sx < 0.02:
                continue
            for i in range(k):
                y0_, y1_ = ya + i * pitch, yb + i * pitch
                d = abs(y1_ - y0_)
                mid = (sx + ex) / 2 + (i - k / 2) * pitch * (1 if y1_ > y0_ else -1)
                pts = [(sx, y0_), (mid - d / 2, y0_), (mid + d / 2, y1_), (ex, y1_)]
                run(pts, 0.0009)
        # traces: from some pads, a run straight out then a 45 degree jog, then straight on
        for (px, py) in pads:
            if rng.random() < 0.6:
                continue
            pts = [(px, py)]
            ang = rng.choice((0, math.pi / 2, math.pi, -math.pi / 2))
            for _ in range(rng.randint(2, 4)):
                L = rng.uniform(0.01, 0.05)
                nx_, ny_ = pts[-1][0] + math.cos(ang) * L, pts[-1][1] + math.sin(ang) * L
                nx_ = min(max(nx_, x0 + 0.006), x1 - 0.006)
                ny_ = min(max(ny_, y0 + 0.006), y1 - 0.006)
                pts.append((nx_, ny_))
                ang += rng.choice((math.pi / 4, -math.pi / 4))
            run(pts, rng.choice((0.0009, 0.0009, 0.0014, 0.0022)))
            ex, ey = pts[-1]
            self.ring(ex, ey, 0.0018, 0.0008, zb, zb + 0.0006, "gold", seg=12)
        # silkscreen board id
        self.text(f"CB-{seed:02d}  REV C", x0 + 0.03, y0 + 0.01, 0.006, "stencil", z=zb, mono=True, align="LEFT", depth=0.0002)


# ----------------------------------------------------------------------------- render

def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def setup_render(w_px, h_px, samples):
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    cy = sc.cycles
    cy.device = "CPU"
    cy.samples = samples
    cy.use_adaptive_sampling = False
    cy.use_denoising = False
    cy.seed = 0
    cy.max_bounces = 3
    cy.diffuse_bounces = 2
    cy.glossy_bounces = 1
    cy.transmission_bounces = 0
    cy.transparent_max_bounces = 0
    cy.sample_clamp_indirect = 2.0
    cy.filter_width = 1.2
    sc.render.resolution_x, sc.render.resolution_y = w_px, h_px
    sc.render.resolution_percentage = 100
    sc.render.threads_mode = "AUTO"
    sc.view_settings.view_transform = "Standard"
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    bg = world.node_tree.nodes["Background"]
    bg.inputs["Color"].default_value = (0.18, 0.19, 0.21, 1.0)
    bg.inputs["Strength"].default_value = 1.0
    world.light_settings.distance = 0.02
    sc.world = world
    vl = sc.view_layers[0]
    vl.use_pass_diffuse_color = True
    vl.use_pass_normal = True
    vl.use_pass_ambient_occlusion = True
    vl.use_pass_emit = True
    for name in ("rough", "metal"):
        a = vl.aovs.add()
        a.name = name
        a.type = "VALUE"
    # a key light from the upper left for the beauty pass (the contact sheet only)
    ld = bpy.data.lights.new("Key", "SUN")
    ld.energy = 2.2
    ld.angle = math.radians(12)
    ob = bpy.data.objects.new("Key", ld)
    ob.rotation_euler = (math.radians(40), math.radians(-30), 0.0)
    sc.collection.objects.link(ob)
    return sc


def camera(w, h):
    cd = bpy.data.cameras.new("Cam")
    cd.type = "ORTHO"
    cd.ortho_scale = max(w, h)
    cd.clip_start = 0.01
    cd.clip_end = 2.0
    cam = bpy.data.objects.new("Cam", cd)
    cam.location = (0.0, 0.0, 1.0)
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam


def compositor_outputs(outdir, stem):
    """File Output nodes writing every pass as a 32 bit EXR named <stem>_<pass>0001.exr."""
    sc = bpy.context.scene
    sc.use_nodes = True
    sc.render.use_compositing = True
    nt = sc.node_tree
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    rl = nt.nodes.new("CompositorNodeRLayers")
    fo = nt.nodes.new("CompositorNodeOutputFile")
    fo.base_path = outdir
    fo.format.file_format = "OPEN_EXR"
    fo.format.color_depth = "32"
    fo.format.exr_codec = "ZIP"
    fo.file_slots.clear()
    for sock, name in (("Image", "beauty"), ("DiffCol", "albedo"), ("Normal", "normal"), ("AO", "ao"),
                       ("Emit", "emit"), ("rough", "rough"), ("metal", "metal")):
        fo.file_slots.new(f"{stem}_{name}")
        nt.links.new(rl.outputs[sock], fo.inputs[-1])
    comp = nt.nodes.new("CompositorNodeComposite")
    nt.links.new(rl.outputs["Image"], comp.inputs[0])


def read_exr(path):
    img = bpy.data.images.load(path, check_existing=False)
    w, h = img.size
    px = np.empty(w * h * 4, np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    return px.reshape(h, w, 4)[:, :, :3].copy()     # bottom row first, as Blender keeps it


def to_srgb(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def save_png(path, rgb):
    """rgb: float array, bottom row first, already encoded (0..1). Written as 8 bit PNG."""
    h, w = rgb.shape[:2]
    img = bpy.data.images.new(os.path.basename(path), w, h, alpha=False)
    img.colorspace_settings.name = "Non-Color"
    a = np.ones((h, w, 4), np.float32)
    a[:, :, :3] = np.clip(rgb, 0, 1)
    img.pixels.foreach_set(a.ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


def bake_panel(build, name, w, h, ppm, samples, outdir, rawdir):
    """Model one panel with build(P) and write its maps: <name>_albedo, _normal, _orm, _emit and
    _beauty PNGs in outdir. Returns the pixel size."""
    reset_scene()
    w_px, h_px = int(round(w * ppm)), int(round(h * ppm))
    setup_render(w_px, h_px, samples)
    mats = make_materials()
    P = Panel(name, w, h, mats)
    build(P)
    camera(w, h)
    compositor_outputs(rawdir, name)
    bpy.ops.render.render(write_still=False)
    get = lambda p: read_exr(os.path.join(rawdir, f"{name}_{p}0001.exr"))
    alb, nor, ao, emit, rough, metal, beauty = (get(p) for p in ("albedo", "normal", "ao", "emit", "rough", "metal", "beauty"))
    save_png(os.path.join(outdir, f"{name}_albedo.png"), to_srgb(alb))
    n = nor / np.maximum(np.linalg.norm(nor, axis=2, keepdims=True), 1e-6)
    save_png(os.path.join(outdir, f"{name}_normal.png"), n * 0.5 + 0.5)
    orm = np.dstack([ao[:, :, 0], rough[:, :, 0], metal[:, :, 0]])
    save_png(os.path.join(outdir, f"{name}_orm.png"), orm)
    save_png(os.path.join(outdir, f"{name}_emit.png"), to_srgb(emit))
    save_png(os.path.join(outdir, f"{name}_beauty.png"), to_srgb(beauty * 1.1))
    return w_px, h_px
