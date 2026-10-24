"""The chest cockpit and its opening armour, shared by every mech built with mech_kit.py.

The owner, 2026-10-24: "Follow design philosophy of boss mechs from mining mike. The cockpit is part of
the mech, inside of it with the armor opening up at the chest", after "Show me mech designs with the
animation of the cockpit opening up" and "Id like a couple variations of cockpit designs please".

A chassis calls these in its builder (build_mechs.py):

  bay()        the cockpit bay: a lined cavity in the chest sized to the interior's envelope (the cockpit
               variants of tools/blender/build_cockpit.py) plus 0.1 m, the `cockpit` joint on the tub floor
               under the eye, and the pilot sockets. Its numbers come from the interior's own manifest
               (assets/models/cockpit_<variant>.json) when it exists, so the bay always fits what goes in.
  shell()      the chest built around the bay, with the front left open for the hatch plates.
  hatch()      an armour plate on its own joint, with the motion that opens it (a track the page plays).
  ram()        a hydraulic ram between two joints, as nodes the page aims every frame (RAM_<n>, its rod,
               its tip), so it stays attached however far the plate swings.
  vent()       a steam or mist vent: an empty with its kind, stage, rate and direction.
  ladder()     a chain ladder stowed under the sill that drops to the ground once the chest is open.

Everything is in kit units except the bay, which is metres turned into kit units (u = 1 / scale). The
timeline is written to the root's extras as `boarding` (tracks, vent stages, the ladder) and played by
tools/mockups/gen_mech_designs.py; the game reads the same extras (openspec/changes/mech-boarding).
"""
import json
import math
import os

from mathutils import Vector

import mech_kit as K

PI = math.pi

# The interiors' envelopes in metres (x half width, y range, z range), the eye, and the doorway. These
# are the contract with build_cockpit.py; a built interior's manifest overrides them.
COCKPITS = {
    "wall": dict(x=1.05, y=(0.0, 2.1), z=(-0.8, 1.25), eye=(0.0, 1.20, 0.0), door=(0.9, 1.6)),
    "jockey": dict(x=0.85, y=(0.0, 1.75), z=(-1.0, 1.3), eye=(0.0, 1.10, 0.30), door=(0.9, 1.3)),
    "harness": dict(x=1.05, y=(0.0, 2.4), z=(-0.75, 0.95), eye=(0.0, 1.65, 0.0), door=(0.9, 1.6)),
}
MARGIN_M = 0.1      # clearance between the interior and the bay's lining
LINING_M = 0.12     # the lining's thickness


def cockpit_env(variant):
    """The interior's envelope: its manifest when it has been built, the contract otherwise."""
    e = dict(COCKPITS.get(variant, COCKPITS["wall"]))
    path = os.path.join(K.ROOT, "assets", "models", f"cockpit_{variant}.json")
    if os.path.exists(path):
        man = json.load(open(path))
        dw = man.get("doorway_m")
        if isinstance(dw, dict):
            e["door"] = (dw["width"], dw["height"])
        env = man.get("envelope_m")
        if env:
            e.update(x=env["x"][1] if isinstance(env["x"], list) else env["x"], y=tuple(env["y"]), z=tuple(env["z"]))
        if man.get("eye_m"):
            e["eye"] = tuple(man["eye_m"])
    return e


def bay(k, chest, variant, x, y, z):
    """The cockpit bay in `chest`, its floor centre at (x, y, z) kit units. Returns the cockpit node and
    the cavity in kit units: dict(hw, y0, y1, z0, z1) in the cockpit joint's frame."""
    M, u = k.M, (1.0 / k.s if k.s else 1.0)
    E = cockpit_env(variant)
    cp = k.J("cockpit", chest, x, y, z)
    hw = (E["x"] + MARGIN_M) * u
    y0, y1 = (E["y"][0] - MARGIN_M) * u, (E["y"][1] + MARGIN_M) * u
    z0, z1 = (E["z"][0] - MARGIN_M) * u, (E["z"][1] + MARGIN_M) * u
    t = LINING_M * u
    W, H, D = 2 * hw, y1 - y0, z1 - z0
    yc, zc = (y0 + y1) / 2, (z0 + z1) / 2
    k.box(W + 2 * t, t, D, M["dark"], cp, 0, y0 - t / 2, zc)              # floor
    k.box(W + 2 * t, t, D, M["dark"], cp, 0, y1 + t / 2, zc)              # roof
    k.box(W + 2 * t, H, t, M["dark"], cp, 0, yc, z0 - t / 2)              # back
    for s in (1, -1):
        k.box(t, H, D, M["dark"], cp, s * (hw + t / 2), yc, zc)          # sides
        for i in range(4):                                               # ribs along the walls
            k.box(t * 0.8, H * 0.96, t * 0.8, M["frame"], cp, s * (hw - t * 0.4), yc, z0 + D * (0.15 + 0.23 * i))
        k.lamp(cp, s * (hw - t), y1 - t, z1 - t * 2, 0.08 * u, 0.6 * u)
    k.box(W * 0.9, 0.04 * u, 0.06 * u, M["hazard"], cp, 0, y0 + 0.02 * u, z1 - 0.05 * u)   # the sill's hazard edge
    E_eye = E["eye"]
    k.sock("EYE_pilot", cp, E_eye[0] * u, E_eye[1] * u, E_eye[2] * u)
    k.sock("SEAT_pilot", cp, 0, 0.45 * u, E_eye[2] * u - 0.02 * u)
    k.sock("HATCH_entry", cp, 0, y0, z1)
    k.extras["cockpit"] = dict(variant=variant, eye_m=list(E_eye), envelope_m=dict(x=E["x"], y=list(E["y"]), z=list(E["z"])),
                               door_m=list(E["door"]))
    return cp, dict(hw=hw, y0=y0, y1=y1, z0=z0, z1=z1)


def shell(k, cp, cav, out, mat=None, frame_mat=None):
    """The chest around the bay, open at the front. `out` is the outer box in the cockpit joint's frame,
    kit units: dict(hw, y0, y1, z0, z1) with z1 the chest's front face. The aperture is the cavity's
    front, framed by a bolted ring; the hatch plates close it."""
    M = k.M
    mat, fr = mat or M["armor"], frame_mat or M["frame"]
    zc, D = (out["z0"] + out["z1"]) / 2, out["z1"] - out["z0"]
    W = 2 * out["hw"]
    k.pl(W, cav["y0"] - out["y0"], D, mat, cp, 0, (out["y0"] + cav["y0"]) / 2, zc)                 # below
    k.pl(W, out["y1"] - cav["y1"], D, mat, cp, 0, (out["y1"] + cav["y1"]) / 2, zc)                 # above
    k.pl(W, cav["y1"] - cav["y0"], cav["z0"] - out["z0"], mat, cp, 0, (cav["y0"] + cav["y1"]) / 2,
         (out["z0"] + cav["z0"]) / 2)                                                              # behind
    for s in (1, -1):
        w = out["hw"] - cav["hw"]
        k.pl(w, cav["y1"] - cav["y0"], out["z1"] - cav["z0"], mat, cp, s * (cav["hw"] + w / 2),
             (cav["y0"] + cav["y1"]) / 2, (cav["z0"] + out["z1"]) / 2)                             # beside
    # the aperture's frame: a ring of bars standing proud of the face, bolted
    zf, b = out["z1"] + 0.6, 1.6
    hw, y0, y1 = cav["hw"], cav["y0"], cav["y1"]
    k.box(2 * hw + 2 * b, b, 1.2, fr, cp, 0, y1 + b / 2, zf)
    k.box(2 * hw + 2 * b, b, 1.2, fr, cp, 0, y0 - b / 2, zf)
    for s in (1, -1):
        k.box(b, y1 - y0, 1.2, fr, cp, s * (hw + b / 2), (y0 + y1) / 2, zf)
        k.bolts(cp, 4, s * (hw + b / 2), y0 + 2, zf + 0.6, 0, 0.35)
    k.bolts(cp, 6, -hw + 1, y1 + b / 2, zf + 0.6, (2 * hw - 2) / 5, 0.35)


def hatch(k, name, parent, x, y, z, motion, t0, d, rx=0, ry=0, rz=0):
    """A plate's joint (the hinge or slide origin) and the track that opens it. motion: ("x"|"y"|"z", angle)
    for a hinge, or ("pos_x"|"pos_y"|"pos_z", metres) for a slide. t0 and d are seconds after the chest
    starts opening."""
    j = k.J(name, parent, x, y, z, rx, ry, rz)
    axis, v = motion
    tr = dict(node=name, t0=t0, d=d)
    if axis.startswith("pos_"):
        tr.update(pos=axis[4:], v=v)
    else:
        tr.update(axis=axis, v=v)
    k.extras.setdefault("boarding", {}).setdefault("tracks", []).append(tr)
    return j


def ram(k, name, A, a, B, b, r):
    """A hydraulic ram from point a on node A to point b on node B (kit units in each node's frame). The
    base joint RAM_<name> points its +z at the tip at rest; the rod RAM_<name>_rod slides along that z;
    RAM_<name>_tip rides on B. The page re-aims the base at the tip and slides the rod every frame."""
    M, s = k.M, k.s
    WA, WB = k.world[A.joint] @ A.M, k.world[B.joint] @ B.M
    pa = Vector(a) * s
    pb = WA.inverted() @ (WB @ (Vector(b) * s))
    d = pb - pa
    L = d.length / s
    e = Vector((0, 0, 1)).rotation_difference(d.normalized()).to_euler("ZYX")   # matrix Rx @ Ry @ Rz, as eul()
    base = k.J(f"RAM_{name}", A, a[0], a[1], a[2], e.x, e.y, e.z)
    sl, rl = L * 0.58, L * 0.62
    k.cyl(r, r, sl, M["dark"], base, 0, 0, sl / 2, PI / 2, seg=12)
    k.cyl(r * 1.15, r * 1.15, sl * 0.12, M["frame"], base, 0, 0, sl - sl * 0.06, PI / 2, seg=12)
    k.sph(r * 1.3, M["frame"], base, 0, 0, 0, seg=8)
    rod = k.J(f"RAM_{name}_rod", base, 0, 0, L - rl)
    k.cyl(r * 0.55, r * 0.55, rl, M["metal"], rod, 0, 0, rl / 2, PI / 2, seg=10)
    k.sock(f"RAM_{name}_tip", B, *b)
    k.sph(r * 1.2, M["frame"], B, *b, seg=8)
    k.extras.setdefault("boarding", {}).setdefault("rams", []).append(dict(base=f"RAM_{name}", rod=f"RAM_{name}_rod",
                                                                            tip=f"RAM_{name}_tip", rod_len=rl * s))
    return base


def vent(k, p, x, y, z, direction, stage, kind="steam", rate=1.0):
    """A steam or mist vent on node p: a grille where it can be seen, and the empty the page emits from.
    stage: seam (pressure equalising), open (as the plates part), sill (mist rolling out)."""
    n = len([1 for nm in k.joints if nm.startswith("VENT_")])
    d = Vector(direction).normalized()
    return k.sock(f"VENT_{stage}_{n:02d}", p, x, y, z,
                  extras={"kind": kind, "stage": stage, "rate": rate, "dir": json.dumps([round(c, 4) for c in d])})


def ladder(k, cp, cav, segs=10, seg_m=0.8):
    """A chain ladder stowed in a box under the sill. LADDER_<i> drop one after another to the ground."""
    M, u = k.M, 1.0 / k.s
    sl = seg_m * u
    lad = k.J("LADDER", cp, 0, cav["y0"] - 0.35 * u, cav["z1"] + 0.25 * u)
    k.box(1.0 * u, 0.3 * u, 0.4 * u, M["frame"], lad, 0, 0.05 * u, 0)
    k.box(0.9 * u, 0.04 * u, 0.02 * u, M["hazard"], lad, 0, 0.05 * u, 0.21 * u)
    for i in range(segs):
        g = k.J(f"LADDER_{i + 1}", lad, 0, -0.1 * u, 0)
        for dx in (-0.3, 0.3):
            k.box(0.04 * u, sl, 0.04 * u, M["metal"], g, dx * u, -sl / 2, 0)
        for f in (0.25, 0.75):                                           # two rungs a segment, 0.4 m apart
            k.cyl(0.025 * u, 0.025 * u, 0.6 * u, M["dark"], g, 0, -f * sl, 0, 0, 0, PI / 2, 6)
    k.extras.setdefault("boarding", {})["ladder"] = dict(node="LADDER", segs=segs, seg_m=seg_m)


# ----------------------------------------------------------------------------- the lofted chest
# The owner, 2026-10-24: "Cockpit doesn't have the cockpit shape we just made. It's a square box", "The
# geometry of the cockpit must match the interiors that we just designed". pod() replaces bay() and
# shell(): the chest is lofted round the interior's own outline, measured off its glTF by cockpit_hull.py,
# in three bands (under the screens, the screen belt, the roof). Inside, a dark lining 0.05 m off the
# interior; outside, armour POD_ARMOUR_M thick that stands POD_BELT_M prouder round the screen belt, so the
# shape of the screens inside shows on the chest: the Foreman's chamfered cab, the Stevedore's octagon, the
# Ironhorse's hexagonal visor bank. Everything ahead of the cut (z_cut, just behind the pivots of the
# interior's own opening parts) opens; everything behind it, the floor and the roof stay.
#
#   mode "lift"   the front splits at the screens' foot: the upper part hinges up over the roof, the lower
#                 part drops forward into a step (the Foreman)
#   mode "doors"  the front splits down the middle into two doors hinged at the sides (the Stevedore)
#   mode "side"   the whole front is one door hinged on the pilot's right
#   mode "inner"  the chest opens on the interior's own parts, as one body with them (_inner, below): the
#                 Foreman's Panorama clamshell, the Ironhorse's Excavator cab ramp

import bmesh                     # noqa: E402

import cockpit_hull as CH        # noqa: E402

POD_MARGIN_M = 0.05     # the lining stands this far off the interior's outline
POD_ARMOUR_M = 0.30     # the armour's thickness outside the lining
POD_BELT_M = 0.06       # how much prouder the armour stands round the screen belt
POD_CHAMFER_M = 0.08    # the chamfer round the floor and roof slabs


def _angles(polys, centre, n=36):
    """The loft's angles in [-pi/2, 3pi/2): uniform ones, every polygon corner, and the four cut angles
    (-pi/2, 0, pi/2, pi) exactly, so the pieces split on a ring of vertices."""
    cx, cz = centre
    forced = [-PI / 2, 0.0, PI / 2, PI]
    cand = [-PI / 2 + 2 * PI * i / n for i in range(n)]
    for p in polys:
        for x, z in p:
            cand.append(math.atan2(x - cx, z - cz))
    out = list(forced)
    for a in cand:
        a = (a + PI / 2) % (2 * PI) - PI / 2
        if all(abs(a - b) > 0.025 for b in out):
            out.append(a)
    return sorted(out)


def _radii(poly, angles, centre):
    cx, cz = centre
    return [math.hypot(x - cx, z - cz) for x, z in CH.ring(poly, angles, centre)]


def _surface(keys, angles, centre):
    """keys [(y, poly)] rising: returns r(i, y), the surface's distance from the axis at angle i, height y."""
    R = [(y, _radii(p, angles, centre)) for y, p in keys]

    def r(i, y):
        if y <= R[0][0]:
            return R[0][1][i]
        for (ya, ra), (yb, rb) in zip(R, R[1:]):
            if y <= yb:
                t = 0.0 if yb - ya < 1e-9 else (y - ya) / (yb - ya)
                return ra[i] + (rb[i] - ra[i]) * t
        return R[-1][1][i]
    return r


def _newell(pts):
    n = Vector((0, 0, 0))
    for a, b in zip(pts, pts[1:] + pts[:1]):
        n += Vector(((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y)))
    return n


class _Faces:
    """Faces sorted by material, each wound so its normal agrees with the direction it was asked for."""
    def __init__(self):
        self.by = {}

    def add(self, role, pts, want):
        pts = [Vector(p) for p in pts]
        if _newell(pts).dot(Vector(want)) < 0:
            pts.reverse()
        self.by.setdefault(role, []).append(pts)

    def put(self, k, joint, origin, mats):
        """Into the kit on `joint`, the points (metres, cockpit frame) taken relative to `origin`."""
        o = Vector(origin)
        for role, faces in self.by.items():
            bm = bmesh.new()
            vs = {}
            for f in faces:
                loop = []
                for p in f:
                    q = p - o
                    key = (round(q.x, 5), round(q.y, 5), round(q.z, 5))
                    if key not in vs:
                        vs[key] = bm.verts.new(q)
                    if not loop or loop[-1] is not vs[key]:
                        loop.append(vs[key])
                if len(loop) >= 3 and len(set(loop)) == len(loop):
                    try:
                        bm.faces.new(loop)
                    except ValueError:
                        pass
            k._put(bm, mats[role], joint)


def section(w, zf, zb, z_cut, wcut=None, sh=0.3, cb=0.3, round_back=0):
    """A torso cross-section in plan (metres, the cockpit's frame): half width w, front zf, back zb. Ahead of
    the cut plane z_cut it is no wider than wcut, so the front of the chest is the cockpit's prow and only the
    prow opens; behind the cut it flares to w over sh metres (the shoulders' front face), and its back corners
    are chamfered by cb, or rounded with round_back points (the Ironhorse's boiler)."""
    wcut = w if wcut is None else min(w, wcut)
    zf = max(zf, z_cut + 0.04)
    half = [(0.0, zf), (wcut * 0.8, zf), (wcut, z_cut), (w, z_cut - sh)]
    if round_back:
        zm, rz = (z_cut - sh + zb) / 2, (z_cut - sh - zb) / 2
        for q in range(1, round_back + 1):
            t = q / (round_back + 1) * PI / 2
            half.append((w * math.cos(t), zm - rz * math.sin(t)))
        half.append((0.0, zb))
    else:
        half += [(w, zb + cb), (w - cb, zb), (0.0, zb)]
    left = [(x, z) for x, z in half]
    right = [(-x, z) for x, z in reversed(half[1:-1])]
    return left + right


def pod(k, chest, variant, at, mode, torso, armour_m=POD_ARMOUR_M, timing=None, swing=None):
    """The cockpit and the sculpted torso round it in `chest`, the interior's floor centre at `at` (kit units).

    torso: [(y, w, zf, zb, dict(wcut, sh, cb, round_back))] rising from the waist to the collar, heights in
    metres from the interior's floor: the chest's own silhouette (section()). Between the lining's floor and
    roof the armour is the larger of that silhouette and the lining grown by armour_m (POD_BELT_M more round the
    screen belt), so the prow ahead of the cut keeps the interior's shape: the Foreman's chamfers, the
    Stevedore's octagon, the Ironhorse's hexagonal visor bank. The prow, from just under the lining's floor to
    just over its roof and ahead of z_cut, is the part that opens (mode lift, doors or side).

    Returns (cp, cav, out, hull): the cockpit node; the lining's box and the torso's box in kit units in the
    cockpit joint's frame (hw, y0, y1, z0, z1); and the hull: its moving joints, `at(a, y, d)` a point on the
    armour's outer face at bearing a (radians, 0 ahead, pi/2 the pilot's left) and height y (metres), d metres
    proud of it, in kit units, and its numbers in metres. timing: {joint: (t0, d)} overrides the opening. swing: {interior
    node: value} overrides how far a stage of a coupled part (mode inner) turns or slides in the mech, where the chest's
    own size asks for more than the interior alone does."""
    M, u = k.M, 1.0 / k.s
    E = cockpit_env(variant)
    m = CH.measure(variant)
    cp = k.J("cockpit", chest, *at)
    centre = m["centre"]
    cx, cz = centre
    (y0, sb, P0), (_, st, P1), (_, y1, P2) = m["bands"]
    mg, T, bt = POD_MARGIN_M, armour_m, POD_BELT_M
    Yb, Yt = y0 - mg, y1 + mg
    eps = 0.004
    inner_keys = [(Yb, CH.offset(P0, mg)), (sb - 0.015, CH.offset(P0, mg)), (sb + 0.015, CH.offset(P1, mg)),
                  (st - 0.015, CH.offset(P1, mg)), (st + 0.015, CH.offset(P2, mg)), (Yt, CH.offset(P2, mg))]
    cock_keys = [(Yb, CH.offset(P0, mg + T)), (sb - 0.06, CH.offset(P0, mg + T)), (sb + 0.06, CH.offset(P1, mg + T + bt)),
                 (st - 0.06, CH.offset(P1, mg + T + bt)), (st + 0.06, CH.offset(P2, mg + T)), (Yt, CH.offset(P2, mg + T))]
    design = [(y, section(w, zf, zb, cz, **kw)) for y, w, zf, zb, kw in torso]
    A = _angles([p for _, p in inner_keys + cock_keys + design], centre, n=40)
    n = len(A)
    r_in, r_ck, r_ds = _surface(inner_keys, A, centre), _surface(cock_keys, A, centre), _surface(design, A, centre)
    y_lo, y_hi = Yb - 0.5 * T, Yt + 0.5 * T            # the opening prow, top and bottom
    ysplit = sb
    PT = CH.parts(variant) if mode == "inner" else {}
    if PT:                                               # the seam between a part from the floor and one above it
        lows = [d["front_y"][1] for d in PT.values() if d["front_y"][0] <= y0 + 0.12 and d["front_y"][1] < y1 - 0.15]
        wrap = [d["stages"][0][1]["pivot_m"][1] for d in PT.values() if d["z"][0] < cz]
        ysplit = max(lows) if lows else (wrap[0] if wrap else sb)
        if any(abs(ysplit - y) < 0.004 for y in (sb, st, Yb, Yt)):
            ysplit += 0.006
    Ybot, Ytop = design[0][0], design[-1][0]

    def ri(i, y):                       # the lining; solid (radius 0) outside it
        return r_in(i, y) if Yb <= y <= Yt else 0.0

    def ro(i, y):
        r = r_ds(i, y)
        if Yb - T <= y <= Yt + T:       # the cockpit's own armour, with its floor and roof slabs
            r = max(r, r_ck(i, min(max(y, Yb), Yt)) - (0.0 if Yb <= y <= Yt else 0.06 * abs(y - min(max(y, Yb), Yt)) / T))
        return r

    H = sorted({y for y, _ in inner_keys + cock_keys + design} | {Yb - eps, Yt + eps, Yb - T, Yt + T, y_lo, y_hi, ysplit})
    H = [y for y in H if Ybot <= y <= Ytop]

    def P(i, y, r):
        a = A[i]
        return (cx + math.sin(a) * r, y, cz + math.cos(a) * r)

    def radial(q):
        c = [sum(v[t] for v in q) / len(q) for t in range(3)]
        return (c[0] - cx, 0.0, c[2] - cz)

    def tangent(i):
        return (math.cos(A[i]), 0.0, -math.sin(A[i]))

    def wall(F, idx, ja, jb, loop=False):
        """The solid between the lining and the armour at angles idx (in order) and heights H[ja..jb], closed
        top, bottom and at its cut ends. Where there is no lining the inner face shrinks to the axis."""
        segs = list(zip(idx, idx[1:] + idx[:1])) if loop else list(zip(idx, idx[1:]))
        for j in range(ja, jb):
            ya, yb = H[j], H[j + 1]
            for i, i2 in segs:
                q = [P(i, ya, ro(i, ya)), P(i2, ya, ro(i2, ya)), P(i2, yb, ro(i2, yb)), P(i, yb, ro(i, yb))]
                F.add("armor", q, radial(q))
                if ri(i, ya) + ri(i2, ya) + ri(i, yb) + ri(i2, yb) > 0:
                    q = [P(i, ya, ri(i, ya)), P(i2, ya, ri(i2, ya)), P(i2, yb, ri(i2, yb)), P(i, yb, ri(i, yb))]
                    w = radial(q)
                    F.add("dark", q, (-w[0], -1e-3 if yb <= Yb + eps else (1e-3 if ya >= Yt - eps else 0), -w[2]))
        for j, up in ((ja, -1), (jb, 1)):
            y = H[j]
            role = "armor" if y in (Ybot, Ytop) else "frame"
            for i, i2 in segs:
                F.add(role, [P(i, y, ri(i, y)), P(i2, y, ri(i2, y)), P(i2, y, ro(i2, y)), P(i, y, ro(i, y))], (0, up, 0))
        if not loop:
            for i, sg in ((idx[0], -1), (idx[-1], 1)):
                t = tangent(i)
                for j in range(ja, jb):
                    ya, yb = H[j], H[j + 1]
                    F.add("frame", [P(i, ya, ri(i, ya)), P(i, ya, ro(i, ya)), P(i, yb, ro(i, yb)), P(i, yb, ri(i, yb))],
                          (sg * t[0], 0, sg * t[2]))

    mats = dict(armor=M["armor"], dark=M["dark"], frame=M["frame"])
    i_neg, i_0, i_pos = A.index(-PI / 2), A.index(0.0), A.index(PI / 2)
    front = list(range(i_neg, i_pos + 1))
    back = list(range(i_pos, n)) + [i_neg]
    allr = list(range(n))
    j_bot, j_top, j_lo, j_hi, j_s = 0, len(H) - 1, H.index(y_lo), H.index(y_hi), H.index(ysplit)
    solids = dict(back=(back, j_lo, j_hi, False), low=(allr, j_bot, j_lo, True), high=(allr, j_hi, j_top, True))
    if mode != "inner":
        F = _Faces()
        for idx, ja, jb, lp in solids.values():
            wall(F, idx, ja, jb, loop=lp)
        F.put(k, cp, (0, 0, 0), mats)

    def zfront(y):
        return cz + ro(i_0, y)

    def xside(sg):
        i = i_pos if sg > 0 else i_neg
        return max(ro(i, y) for y in H if y_lo <= y <= y_hi) * sg

    timing = timing or {}
    joints = {}

    def piece(name, idx, ja, jb, hinge, motion, t0, d):
        t0, d = timing.get(name, (t0, d))
        jn = hatch(k, name, cp, hinge[0] * u, hinge[1] * u, hinge[2] * u, motion, t0, d)
        Fp = _Faces()
        wall(Fp, idx, ja, jb)
        Fp.put(k, jn, hinge, mats)
        joints[name] = (jn, hinge)
        return jn

    if mode == "lift":
        piece("CHEST_visor", front, j_s, j_hi, (0.0, y_hi, cz), ("x", -1.6), 0.5, 1.4)
        piece("CHEST_apron", front, j_lo, j_s, (0.0, y_lo, zfront(y_lo)), ("x", 1.5), 1.6, 1.4)
    elif mode == "doors":
        piece("CHEST_door_L", list(range(i_0, i_pos + 1)), j_lo, j_hi, (xside(1), y_lo, cz), ("y", 1.6), 1.1, 1.5)
        piece("CHEST_door_R", list(range(i_neg, i_0 + 1)), j_lo, j_hi, (xside(-1), y_lo, cz), ("y", -1.6), 1.1, 1.5)
    elif mode == "side":
        piece("CHEST_door", front, j_lo, j_hi, (xside(-1), y_lo, cz), ("y", -1.6), 0.8, 1.6)
    elif mode == "inner":
        _inner(k, cp, PT, solids, front, wall, mats, dict(cz=cz, y0=y0, y1=y1, y_lo=y_lo, y_hi=y_hi, Ytop=Ytop,
                                                          split=ysplit, back_r=lambda y: ro(A.index(PI), y),
                                                          Yb=Yb, Yt=Yt, T=T,
                                                          zfront=zfront, u=u, j_lo=j_lo, j_hi=j_hi), joints, timing,
               swing or {})
    else:
        raise ValueError(mode)

    # sockets and the interior's contract, as bay() wrote them
    E_eye = E["eye"]
    k.sock("EYE_pilot", cp, E_eye[0] * u, E_eye[1] * u, E_eye[2] * u)
    k.sock("SEAT_pilot", cp, 0, 0.45 * u, E_eye[2] * u - 0.02 * u)
    k.sock("HATCH_entry", cp, 0, Yb * u, (cz + r_in(i_0, Yb)) * u)
    k.extras["cockpit"] = dict(variant=variant, eye_m=list(E_eye), envelope_m=dict(x=E["x"], y=list(E["y"]), z=list(E["z"])),
                               door_m=list(E["door"]), hull=dict(mode=mode, z_cut_m=round(cz, 4), armour_m=T, belt_m=bt,
                                                                 margin_m=mg, lining_y_m=[round(Yb, 4), round(Yt, 4)],
                                                                 prow_y_m=[round(y_lo, 4), round(y_hi, 4)]))
    if mode == "inner":     # which interior part rides which chest joint: the two open as one body
        k.extras["cockpit"]["rides"] = {part: joints[_chest_name(part)][0].joint for part in PT}

    def bbox(r, lo, hi, ys):
        xs, zs = [], []
        for y in ys:
            for i in range(n):
                q = P(i, y, r(i, y))
                xs.append(abs(q[0]))
                zs.append(q[2])
        return dict(hw=max(xs) * u, y0=lo * u, y1=hi * u, z0=min(zs) * u, z1=max(zs) * u)

    cav = bbox(r_in, Yb, Yt, [y for y in H if Yb <= y <= Yt])
    out = bbox(ro, Ybot, Ytop, H)

    def at(a, y, d=0.0):
        a = (a + PI / 2) % (2 * PI) - PI / 2
        i = min(range(n), key=lambda q: abs(A[q] - a))
        r = ro(i, y) + d
        return (cx + math.sin(A[i]) * r) * u, y * u, (cz + math.cos(A[i]) * r) * u

    hull = dict(cp=cp, joints=joints, at=at, Yb=Yb, Yt=Yt, y_lo=y_lo, y_hi=y_hi, Ybot=Ybot, Ytop=Ytop, z_cut=cz, sb=sb,
                st=st, ysplit=ysplit, zfront=zfront, xside=xside, ro=ro, angles=A)
    return cp, cav, out, hull


# --- mode "inner": the chest opens on the interior's own parts
# The owner, 2026-10-24, on the Foreman: "it's cockpit has a problem when opening. It needs to follow the titan
# style or panorama style". The chest and the interior used to open twice, on different hinges, and their
# pieces met. Now each part of the interior that opens (its INNER_ nodes, measured by cockpit_hull.parts) takes
# the armour in front of it: the chest is cut with a box round the part (Exact booleans, the CSG way), that
# piece rides a chain of joints with the part's own stages, hinged on the armour's outer edge nearest the
# part's own hinge, and the interior part is carried on the last joint (extras cockpit.rides). One body, one
# opening. A box's sides are planes across the hinge, so a piece swinging on it never passes through what stays.

BIG = 50.0      # metres: a box face that is "no limit"


def ride_w_of(d):
    return max(abs(d["x"][0]), abs(d["x"][1])) + POD_MARGIN_M


def ride_w(variant, part):
    """The half width (metres) of the chest piece cut round an interior part: its seam down each side."""
    return ride_w_of(CH.parts(variant)[part])


def _chest_name(part):
    return "CHEST_" + part[len("INNER_"):]


def _solid_obj(F, name):
    """A Blender object from faces sorted by role, one material slot a role (armor, dark, frame)."""
    import bpy
    roles = ("armor", "dark", "frame")
    bm = bmesh.new()
    vs = {}
    for ri, role in enumerate(roles):
        for f in F.by.get(role, []):
            loop = []
            for p in f:
                key = (round(p.x, 5), round(p.y, 5), round(p.z, 5))
                if key not in vs:
                    vs[key] = bm.verts.new(p)
                if not loop or loop[-1] is not vs[key]:
                    loop.append(vs[key])
            if len(loop) >= 3 and len(set(loop)) == len(loop):
                try:
                    bm.faces.new(loop).material_index = ri
                except ValueError:
                    pass
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for role in roles:
        me.materials.append(_role_mat(role))
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def _role_mat(role):
    import bpy
    nm = f"__pod_{role}"
    return bpy.data.materials.get(nm) or bpy.data.materials.new(nm)


def _box_obj(name, lo, hi):
    """A cutter box from lo to hi (metres), all its faces the frame role: what a cut leaves is the seam's face."""
    import bpy
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector([lo[i] if v.co[i] < 0 else hi[i] for i in range(3)])
    for f in bm.faces:
        f.material_index = 2
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for role in ("armor", "dark", "frame"):
        me.materials.append(_role_mat(role))
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def _boolean(ob, cutter, op):
    """A new object: ob op cutter (INTERSECT or DIFFERENCE), Exact, materials kept by slot."""
    import bpy
    res = bpy.data.objects.new(ob.name + "_" + op[:3].lower(), ob.data.copy())
    bpy.context.scene.collection.objects.link(res)
    md = res.modifiers.new("b", "BOOLEAN")
    md.operation, md.solver, md.object = op, "EXACT", cutter
    md.material_mode = "INDEX"
    dg = bpy.context.evaluated_depsgraph_get()
    me = bpy.data.meshes.new_from_object(res.evaluated_get(dg))
    res.modifiers.clear()
    old = res.data
    res.data = me
    bpy.data.meshes.remove(old)
    return res


def _drop(ob):
    import bpy
    me = ob.data
    bpy.data.objects.remove(ob)
    if me.users == 0:
        bpy.data.meshes.remove(me)


def _put_obj(k, ob, joint, origin, mats):
    """An object's faces into the kit on `joint`, by role, its points taken relative to `origin` (metres)."""
    roles = ("armor", "dark", "frame")
    o = Vector(origin)
    me = ob.data
    for ri, role in enumerate(roles):
        faces = [f for f in me.polygons if f.material_index == ri]
        if not faces:
            continue
        bm = bmesh.new()
        vs = {}
        for f in faces:
            loop = []
            for vi in f.vertices:
                if vi not in vs:
                    vs[vi] = bm.verts.new(me.vertices[vi].co - o)
                loop.append(vs[vi])
            try:
                bm.faces.new(loop)
            except ValueError:
                pass
        k._put(bm, mats[role], joint)


def _inner(k, cp, PT, solids, front, wall, mats, g, joints, timing, swing):
    """The chest cut round each opening part of the interior; see the note above."""
    cz, split, u = g["cz"], g["split"], g["u"]
    # the prow is its own solid, so a part's box can take it without taking the torso behind the cut
    idx, ja, jb, lp = solids["back"]
    solids = dict(solids, prow=(front, ja, jb, False))
    objs = {}
    for nm, (idx, ja, jb, lp) in solids.items():
        F = _Faces()
        wall(F, idx, ja, jb, loop=lp)
        objs[nm] = _solid_obj(F, f"__pod_{nm}")
    rest = dict(objs)
    for part, d in sorted(PT.items()):
        pv = d["stages"][0][1]["pivot_m"]
        slides_up = any(m.get("slide") == "y" for _, m in d["stages"])
        if d["z"][0] < cz:                                   # it wraps round the cabin: all of the chest above the seam
            cuts = {nm: ((-BIG, split, -BIG), (BIG, BIG, BIG)) for nm in objs}
            hinge = (0.0, split, cz - g["back_r"](split))
        else:
            W = ride_w_of(d)
            low = d["front_y"][0] <= g["y0"] + 0.12
            high = d["front_y"][1] >= g["y1"] - 0.15
            ya = -BIG if low else split
            yb = BIG if high else split
            cuts = {"prow": ((-W, ya, -BIG), (W, yb, BIG))}
            # the slabs under the floor and over the roof, ahead of the cabin's front wall, go with the part too (owner,
            # 2026-10-24: "pieces of geometry that block the pilot from exiting, that part shouldve opened WITH the
            # dooor"): left behind they make a sill to climb over and a lintel to duck under
            z_in = d["z"][1] - 0.12
            if low:
                cuts["low"] = ((-W, g["Yb"] - g["T"] - 0.02, z_in), (W, BIG, BIG))
            if high and slides_up:                        # it rises: the roof ahead of the cut goes with it
                cuts["high"] = ((-W, ya, cz), (W, BIG, BIG))
            elif high:
                cuts["high"] = ((-W, -BIG, z_in), (W, g["Yt"] + g["T"] + 0.02, BIG))
            ends = [g["Yb"] - g["T"] if low else split,
                    (g["Ytop"] if slides_up else g["Yt"] + g["T"]) if high else split]
            yh = min(ends, key=lambda y: abs(y - pv[1]))
            hinge = (0.0, yh, g["zfront"](yh))
        pieces = []
        for nm, (lo, hi) in cuts.items():
            box = _box_obj(f"__cut_{part}_{nm}", lo, hi)
            pieces.append(_boolean(rest[nm], box, "INTERSECT"))
            left = _boolean(rest[nm], box, "DIFFERENCE")
            if rest[nm] is not objs[nm]:
                _drop(rest[nm])
            rest[nm] = left
            _drop(box)
        # the joints: one a stage, as the interior moves the part, on the armour's hinge
        name = _chest_name(part)
        parent, jn = cp, None
        for si, (node, mo) in enumerate(d["stages"]):
            t0, dd = timing.get(node, (mo["t0"], mo["d"]))
            motion = (("pos_" + mo["slide"]) if "slide" in mo else mo["axis"], swing.get(node, mo["value"]))
            nm_j = name if si == 0 else f"{name}_{si}"
            at = (hinge[0] * u, hinge[1] * u, hinge[2] * u) if si == 0 else (0, 0, 0)
            jn = hatch(k, nm_j, parent, *at, motion, t0, dd)
            parent = jn
        for pc in pieces:
            _put_obj(k, pc, jn, hinge, mats)
            _drop(pc)
        joints[name] = (jn, hinge)
    for nm, ob in rest.items():
        _put_obj(k, ob, cp, (0, 0, 0), mats)
        if ob is not objs[nm]:
            _drop(ob)
    for ob in objs.values():
        _drop(ob)


def solid(k, node, keys, mat, centre=(0.0, 0.0), n=24):
    """A closed loft through plan sections at rising heights, [(y, [(x, z), ...])] in kit units in the node's
    frame, each star-shaped round centre: backpacks, pauldrons, skirts that follow a designed silhouette."""
    s = k.s
    keys = [(y * s, [(x * s, z * s) for x, z in poly]) for y, poly in keys]
    c = (centre[0] * s, centre[1] * s)
    A = _angles([p for _, p in keys], c, n=n)
    R = [(y, _radii(p, A, c)) for y, p in keys]
    F = _Faces()

    def P(i, y, r):
        return (c[0] + math.sin(A[i]) * r, y, c[1] + math.cos(A[i]) * r)
    m = len(A)
    for (ya, ra), (yb, rb) in zip(R, R[1:]):
        for i in range(m):
            i2 = (i + 1) % m
            q = [P(i, ya, ra[i]), P(i2, ya, ra[i2]), P(i2, yb, rb[i2]), P(i, yb, rb[i])]
            cc = [sum(v[t] for v in q) / 4 for t in range(3)]
            F.add("m", q, (cc[0] - c[0], 0, cc[2] - c[1]))
    F.add("m", [P(i, R[0][0], R[0][1][i]) for i in range(m)], (0, -1, 0))
    F.add("m", [P(i, R[-1][0], R[-1][1][i]) for i in range(m)], (0, 1, 0))
    F.put(k, node, (0, 0, 0), dict(m=mat))


# --- dressing on the lofted chest: everything a chassis hangs on it follows the hull's own surface

def on(k, hull, name, p):
    """The node a dressing goes on (a moving piece by name, or the cockpit for None) and the point p
    (kit units, cockpit frame) in that node's frame."""
    if name:
        j, h = hull["joints"][name]
        u = 1.0 / k.s
        return j, (p[0] - h[0] * u, p[1] - h[1] * u, p[2] - h[2] * u)
    return hull["cp"], tuple(p)


def contour(k, hull, name, a0, a1, y, d, r, mat, steps=24):
    """A bar r metres thick round the armour at height y (metres), from bearing a0 to a1, d metres proud."""
    pts = [on(k, hull, name, hull["at"](a0 + (a1 - a0) * i / steps, y, d))[1] for i in range(steps + 1)]
    k.hose(on(k, hull, name, (0, 0, 0))[0], pts, r / k.s, mat)


def rib(k, hull, name, a, ya, yb, d, r, mat, steps=8):
    """A bar r metres thick up the armour at bearing a, from ya to yb (metres), d metres proud."""
    pts = [on(k, hull, name, hull["at"](a, ya + (yb - ya) * i / steps, d))[1] for i in range(steps + 1)]
    k.hose(on(k, hull, name, (0, 0, 0))[0], pts, r / k.s, mat)
