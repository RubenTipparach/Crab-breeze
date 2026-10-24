"""Build the alien crab mechs, the invaders the human mechs fight, as glb files on the alien rig.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_alien_mechs.py [-- id ...]

Writes assets/models/alien_<id>.glb and assets/models/aliens.json (the measured manifest). Built with
mech_kit.py (the port of Mining Mike's boss mech kit) in that kit's units, then rebuilt at the scale
that makes each one its height in metres, the same two passes as tools/blender/build_mechs.py.

The owner's direction (2026-10-24): "design some potentially crab looking mechs from enemy aliens",
with three reference sheets of hard-surface mechanical crabs. Each design follows one of them:

  skitter  6 m scuttler, after the four-legged spider crab: a rounded hull on four tall arched legs
           with big joint hubs and hydraulic rods, twin missile pods on arms over the hull, a turret
           with a sensor eye, violet hazard bands.
  snapper  12 m main line crab, after the slate-blue crab: a flat armoured hull with a square face,
           twin headlight eyes and a grille, two big hooked claws raised high, two antenna masts, six
           short folded legs on box feet.
  bigboy   30 m siege crab, after "Big Boy": a wide flat lens shell, one giant boxy claw held forward
           with a beam emitter in its jaws, short thick piston legs on toothed splayed feet, two masts.

What makes them alien rather than human: acid green glows (vents, eyes, mast tips), violet reactor
cores and violet livery with alien glyph markings; the human mechs glow amber, cyan and red. The
skeleton is docs/design/alien-rig.md, not the biped contract of docs/design/mech-rig.md.

Coordinates are glTF style, as in the kit: +Y up, the crab faces +Z, +X is its left. The kit is
imported unchanged; everything crab-specific (legs solved to the ground, claws, masts, lens shells,
glyphs) is in this file.
"""
import math
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_kit as K  # noqa: E402
from mech_kit import Kit, Node  # noqa: E402

PI = math.pi
OUT = os.path.join(K.ROOT, "assets", "models")
MANIFEST = os.path.join(OUT, "aliens.json")

GLOW = 0x8cff3a    # acid green: vents, eyes, mast tips
CORE = 0xb45cff    # violet: reactor cores, missile heads
MARK = 0x8a3ad8    # violet livery bands and glyphs


def N(name):
    return Node(name, K.Matrix.Identity(4))


# ----------------------------------------------------------------------------- materials

def alien_mats(prefix, tint, tint2, livery=MARK, glow=GLOW, core=CORE):
    """The kit's makeMats with the pale plate tinted per machine and the alien glows."""
    m, sh = K._mat, K.scale_hex
    return {
        "armor": m(f"{prefix}_armor", color=tint, tex="armorL", normal="armorL_n", orm="armorL_orm"),
        "armor2": m(f"{prefix}_armor2", color=tint2, tex="armorL", normal="armorB_n", orm="armorB_orm"),
        "frame": m(f"{prefix}_frame", tex="frame", normal="frame_n", orm="frame_orm"),
        "red": m(f"{prefix}_livery", color=livery, tex="wear", normal="wear_n", orm="wear_orm"),
        "art": m(f"{prefix}_core", color=core, emit=core, emit_k=3.0, rough=0.25, metal=0.2),
        "ink": m(f"{prefix}_ink", color=0x0e0b14, rough=0.6, metal=0.3),
        "metal": m(f"{prefix}_metal", color=0xa9aeb5, tex="wear", normal="wear_n", rough=0.26, metal=0.95),
        "dark": m(f"{prefix}_dark", color=0x24222b, tex="frame", normal="frame_n", rough=0.7, metal=0.45),
        "rubber": m(f"{prefix}_rubber", color=0x141317, rough=0.92),
        "hazard": m(f"{prefix}_hazard", color=0xc9a6ff, tex="hazard", orm="hazard_orm"),
        "lens": m(f"{prefix}_lens", color=0x050807, emit=glow, emit_k=1.6, rough=0.08, metal=0.6),
        "glow": m(f"{prefix}_glow", color=glow, emit=glow, emit_k=4.0),
        "glowDim": m(f"{prefix}_glowdim", color=sh(glow, 0.45), emit=glow, emit_k=1.3),
        "tip": m(f"{prefix}_tip", color=glow, emit=glow, emit_k=5.0),
    }


# ----------------------------------------------------------------------------- extra primitives

def _lathe(profile, seg):
    """A solid of revolution about +Y from (radius, y) pairs, top to bottom; a zero radius is a pole."""
    bm = bmesh.new()
    rings = []
    for r, y in profile:
        if r < 1e-6:
            rings.append([bm.verts.new((0, y, 0))])
        else:
            rings.append([bm.verts.new((r * math.cos(K.TAU * j / seg), y, r * math.sin(K.TAU * j / seg)))
                          for j in range(seg)])
    for a, b in zip(rings, rings[1:]):
        if len(a) == 1 and len(b) == 1:
            continue
        for j in range(seg):
            if len(a) == 1:
                bm.faces.new((a[0], b[j], b[(j + 1) % seg]))
            elif len(b) == 1:
                bm.faces.new((a[j], b[0], a[(j + 1) % seg]))
            else:
                bm.faces.new((a[j], b[j], b[(j + 1) % seg], a[(j + 1) % seg]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def lathe(k, profile, mat, p, x=0, y=0, z=0, rx=0, ry=0, rz=0, seg=24, scale=None, smooth=True):
    s = k.s
    k._put(_lathe([(r * s, h * s) for r, h in profile], seg), mat, p, x, y, z, rx, ry, rz,
           scale=scale, smooth=smooth)


def ring(k, R, r, mat, p, x=0, y=0, z=0, zscale=1.0, seg=40):
    """A horizontal torus (the kit's tor laid flat), optionally stretched along Z into an ellipse."""
    s = k.s
    k._put(K._tor(R * s, r * s, K.TAU, seg), mat, p, x, y, z, PI / 2, 0, 0, scale=(1, zscale, 1), smooth=True)


def hook(R, t0, arc, curl=1, n=12):
    """A tapered hooked finger as a polygon in XY: it leaves (0, 0) along +X and curls toward +Y (curl 1)
    or -Y (curl -1), thinning to a point. For pz, which extrudes it along Z."""
    outer, inner = [], []
    for i in range(n + 1):
        u = i / n
        a = u * arc
        cx, cy = R * math.sin(a), curl * (R - R * math.cos(a))
        nx, ny = math.sin(a), -curl * math.cos(a)  # the normal on the outside of the curl
        t = t0 * (1 - u) ** 0.85 + 0.25
        outer.append((cx + nx * t / 2, cy + ny * t / 2))
        inner.append((cx - nx * t / 2, cy - ny * t / 2))
    pts = outer + inner[::-1]
    if curl < 0:
        pts = pts[::-1]
    return [[round(a, 4), round(b, 4)] for a, b in pts]


# ----------------------------------------------------------------------------- crab parts

def solve_raise(Lf, Lt, a2, target, prefer):
    """The femur raise a1 that puts the foot joint `target` units below the coxa (negative is down),
    given the knee bend a2: Lf sin a1 + Lt sin(a1 + a2) = target. The root nearest `prefer`."""
    best, prev = None, None
    lo, hi, n = -1.3, 1.7, 1200
    for i in range(n + 1):
        a = lo + (hi - lo) * i / n
        f = Lf * math.sin(a) + Lt * math.sin(a + a2) - target
        if prev is not None and (f == 0 or (f > 0) != (prev[1] > 0)):
            a0, f0 = prev
            r = a0 + (a - a0) * f0 / (f0 - f)
            if best is None or abs(r - prefer) < abs(best - prefer):
                best = r
        prev = (a, f)
    if best is None:
        raise ValueError(f"leg cannot reach the ground: Lf {Lf} Lt {Lt} a2 {a2} target {target}")
    return best


def leg(k, body, body_y, S, n, at, yaw, Lc, Lf, Lt, Ld, a2, prefer, style):
    """One walking leg on the alien rig: leg_<S><n>_coxa (yaw at the hull), _femur (raise), _tibia
    (knee), _foot (kept level, so the sole is flat), and TIP_<S><n> on the ground under the sole.
    Lengths in kit units; the femur raise is solved so the sole lands exactly on y = 0."""
    s = 1 if S == "L" else -1
    x, y, z = at
    ry = -yaw if s > 0 else PI + yaw
    pre = f"leg_{S}{n}"
    cox = k.J(pre + "_coxa", body, s * x, y, z, 0, ry, 0)
    a1 = solve_raise(Lf, Lt, a2, -(body_y + y) + Ld, prefer)
    fem = k.J(pre + "_femur", cox, Lc, 0, 0, 0, 0, a1)
    tib = k.J(pre + "_tibia", fem, Lf, 0, 0, 0, 0, a2)
    foot = k.J(pre + "_foot", tib, Lt, 0, 0, 0, 0, -(a1 + a2))
    k.sock(f"TIP_{S}{n}", foot, 0, -Ld, 0)
    style(k, cox, fem, tib, foot, s, Lc, Lf, Lt, Ld)
    k.extras.setdefault("legs", []).append(dict(id=f"{S}{n}", side=s, group=(n + (0 if s > 0 else 1)) % 2))


def claw_chain(k, body, S, at, yaw, raise_, arm_len, wrist_len, wrist_bend, wrist_fold, claw_bend):
    """arm_<S> (shoulder: yaw and raise at the hull), wrist_<S> (elbow), claw_<S> (the hand with the
    fixed finger). Every segment runs along its joint's +X. The pincer is added by the caller."""
    s = 1 if S == "L" else -1
    ry_l = -PI / 2 + yaw
    arm = k.J(f"arm_{S}", body, s * at[0], at[1], at[2], 0, ry_l if s > 0 else PI - ry_l, raise_)
    wrist = k.J(f"wrist_{S}", arm, arm_len, 0, 0, 0, -s * wrist_fold, wrist_bend)
    claw = k.J(f"claw_{S}", wrist, wrist_len, 0, 0, 0, 0, claw_bend)
    k.extras.setdefault("claws", []).append(S)
    return arm, wrist, claw


def hub(k, p, r, w, cap=None, x=0, y=0, z=0):
    """A big round joint hub on the hinge axis (local Z): a drum, bright caps on both faces and a dark
    boss through the middle, as the reference legs' joints."""
    M = k.M
    k.cyl(r, r, w, M["frame"], p, x, y, z, PI / 2, seg=24)
    k.cyl(r * 0.84, r * 0.84, w + 0.7, cap or M["metal"], p, x, y, z, PI / 2, seg=24)
    k.cyl(r * 0.5, r * 0.5, w + 1.2, M["dark"], p, x, y, z, PI / 2, seg=16)
    k.cyl(r * 0.22, r * 0.22, w + 1.8, M["metal"], p, x, y, z, PI / 2, seg=10)


def headlight(k, p, x, y, z, r, name, halo=True):
    """A round eye in a frame ring, looking +Z: SENSOR_<name> at its centre."""
    M = k.M
    k.cyl(r * 1.35, r * 1.35, r * 0.9, M["frame"], p, x, y, z, PI / 2, seg=20)
    k.cyl(r * 1.05, r * 1.05, r * 0.95, M["ink"], p, x, y, z + 0.05, PI / 2, seg=20)
    k.sph(r, M["lens"], p, x, y, z + r * 0.2, 1, 1, 0.55, seg=16)
    k.sph(r * 0.42, M["glow"], p, x, y, z + r * 0.62, 1, 1, 0.5, seg=12)
    if halo:
        k.halo(p, x, y, z + r * 0.9, r * 6, 0.8)
    k.sock(f"SENSOR_{name}", p, x, y, z + r * 0.7)


def mast(k, p, name, x, y, z, length, rx=0, rz=0, r=1.0):
    """An antenna mast: a footed pole with collars and a glowing tip. Its own joint mast_<name>."""
    M = k.M
    j = k.J(f"mast_{name}", p, x, y, z, rx, 0, rz)
    k.pl(4 * r, 4 * r, 4 * r, M["frame"], j, 0, 2 * r, 0)
    k.cyl(0.45 * r, 0.9 * r, length, M["metal"], j, 0, 4 * r + length / 2, 0, seg=8)
    for f in (0.3, 0.62, 0.86):
        k.cyl(1.1 * r, 1.1 * r, 0.9 * r, M["dark"], j, 0, 4 * r + length * f, 0, seg=8)
    k.cyl(0.25 * r, 0.25 * r, 3 * r, M["metal"], j, 0, 4 * r + length + 1.5 * r, 0, seg=6)
    k.sph(1.0 * r, M["tip"], j, 0, 4 * r + length + 3 * r, 0, seg=10)
    k.halo(j, 0, 4 * r + length + 3 * r, 0, 5 * r, 0.9)
    k.extras.setdefault("masts", []).append(name)
    return j


def glyphs(k, p, x, y, z, size, count, seed, rx=0, ry=0, rz=0, mat=None):
    """A line of alien glyphs: each two to four short strokes in a cell, laid on a surface facing +Z of
    the group. Deterministic from the seed. Strokes stand 0.35 kit units proud so nothing z-fights."""
    g = k.g(p, x, y, z, rx, ry, rz)
    mat = mat or k.M["red"]
    h = K.hash1
    for c in range(count):
        cx = (c - (count - 1) / 2) * size * 1.25
        strokes = 2 + int(h(seed * 31 + c) * 3)
        for i in range(strokes):
            u = h(seed * 7 + c * 13 + i * 3)
            v = h(seed * 11 + c * 17 + i * 5)
            ang = [0, PI / 2, PI / 4, -PI / 4][int(h(seed + c * 19 + i * 23) * 4)]
            ln = size * (0.45 + 0.5 * h(seed * 3 + c + i * 29))
            k.box(ln, size * 0.14, 0.35, mat, g, cx + (u - 0.5) * size * 0.6, (v - 0.5) * size * 0.8, 0, 0, 0, ang)


# ----------------------------------------------------------------------------- SKITTER (reference 3)

def skitter(k):
    """Four tall arched legs with big hubs and rods, a rounded hull, two missile pods on arms over it,
    a sensor turret on top and a single big eye in a pale face plate."""
    M = k.M
    body_y = 30
    body = k.J("body", k.root, 0, body_y, 0)
    # the rounded hull, an egg stretched front to back
    prof = [(0, 13), (7, 12.4), (13, 10), (17.5, 5.5), (19, 0), (18, -5), (14, -9.5), (8, -11.6), (0, -12)]
    lathe(k, prof, M["armor"], body, 0, 0, 0, seg=28, scale=(1, 1, 1.3))
    ring(k, 19.4, 1.1, M["red"], body, 0, 1.2, 0, zscale=1.3)
    ring(k, 18.6, 0.6, M["red"], body, 0, -2.6, 0, zscale=1.3)
    ring(k, 17.6, 1.4, M["frame"], body, 0, -6.4, 0, zscale=1.3)
    # the face: a pale armour plate across the front, one big eye in it
    face = k.g(body, 0, 0.5, 22.6, -0.08)
    k.pl(22, 19, 4, M["armor2"], face, 0, 0, 0)
    k.pl(24, 3, 5, M["frame"], face, 0, 10.5, -0.5)
    k.pl(24, 3, 5, M["frame"], face, 0, -10.5, -0.5)
    headlight(k, face, 0, 0.5, 2.0, 5.2, "main")
    for sx in (-1, 1):
        k.box(3, 1.2, 0.6, M["glowDim"], face, sx * 8, -6.5, 2.1)
        k.cyl(0.9, 0.9, 1.2, M["metal"], face, sx * 8.6, 6.8, 2.0, PI / 2, seg=8)
    glyphs(k, body, -10.5, 8.5, 15.5, 2.4, 3, 3, -0.55, -0.5, 0)
    # exhaust stacks along the top left, as the reference's row of pipes
    for i in range(4):
        k.cyl(1.0, 1.15, 5, M["metal"], body, 9 - i * 3.2, 11.5, 9.5, -0.15, 0, -0.35, seg=10)
        k.cyl(0.65, 0.65, 0.4, M["ink"], body, 9 - i * 3.2 - 0.85, 13.85, 9.9, -0.15, 0, -0.35, seg=10)
    k.vent(body, 8, 4, 0, 4, -23.5, 0, PI, 3)
    k.bolts(body, 5, -6, -4, 22.4, 3, 0.45)
    # the sensor turret on top
    tur = k.J("turret_1", body, 0, 12.2, -3)
    k.cyl(7.5, 8.2, 3, M["frame"], tur, 0, 1.2, 0, seg=24)
    lathe(k, [(0, 7.5), (4, 7), (6.6, 5), (7, 2.6)], M["armor"], tur, 0, 0, 0, seg=24)
    k.pl(7, 5, 7, M["armor2"], tur, 0, 8.5, 1.5)
    headlight(k, tur, 0, 8.6, 5.2, 1.7, "turret", halo=False)
    for sx in (-1, 1):
        k.cyl(0.7, 0.7, 11, M["metal"], tur, sx * 4.4, 8.0, 5.5, PI / 2, seg=8)
        k.cyl(1.1, 1.1, 2.2, M["dark"], tur, sx * 4.4, 8.0, 1.8, PI / 2, seg=8)
        k.sock(f"MUZZLE_turret_{'L' if sx > 0 else 'R'}", tur, sx * 4.4, 8.0, 11.2)
    # two missile pods on arms over the hull, out to the sides
    for S, sx in (("L", 1), ("R", -1)):
        pod = k.J(f"pod_{S}", body, sx * 11, 9.5, -11)
        k.sph(2.8, M["frame"], pod, 0, 0, 0, seg=12)
        k.box(3.2, 4, 4, M["frame"], pod, 0, 1.5, 0)
        for dz in (-1.8, 1.8):
            k.box(1.7, 1.7, 26, M["metal"], pod, sx * 2.6, 9.5 + dz * 0.3, 10 + dz, -0.62)
        k.pl(4, 4, 6, M["frame"], pod, sx * 2.6, 17.3, 19)
        pg = k.g(pod, sx * 4.5, 20.5, 21)
        k.pl(16, 12, 17, M["armor"], pg, 0, 0, 0)
        k.pl(16.6, 3, 17.6, M["red"], pg, 0, -1.5, 0)
        k.pl(13.5, 9.5, 2, M["frame"], pg, 0, 0, 8.6)
        for i, (dx, dy) in enumerate(((-3.1, 2.5), (3.1, 2.5), (-3.1, -2.5), (3.1, -2.5))):
            k.cyl(2.0, 2.0, 1.4, M["ink"], pg, dx, dy, 9.5, PI / 2, seg=14)
            k.sph(1.3, M["art"], pg, dx, dy, 9.8, 1, 1, 0.6, seg=10)
            k.sock(f"MUZZLE_pod_{S}{i + 1}", pg, dx, dy, 10.4)
        k.box(1.2, 7, 8, M["armor2"], pg, sx * 8.6, 0, -3)
        k.vent(pg, 9, 4, 0, 0, -8.8, 0, PI, 3)
        k.hose(pod, [[sx * 1.5, 1, -1], [sx * 4.5, 10, 4], [sx * 4.5, 15, 13]], 0.6)

    def style(k, cox, fem, tib, foot, s, Lc, Lf, Lt, Ld):
        k.cyl(4.6, 4.6, 7, M["frame"], cox, 0, 0, 0, seg=16)
        k.pl(Lc + 5, 6, 7, M["armor2"], cox, Lc / 2, 0, 0)
        hub(k, fem, 6.4, 8.6)
        for zz in (-2.6, 2.6):
            k.pl(Lf - 2, 3, 2, M["frame"], fem, Lf / 2, 0, zz)
        k.pl(Lf * 0.62, 5.5, 6.6, M["armor"], fem, Lf * 0.52, 1.4, 0)
        k.box(Lf * 0.5, 1.2, 6.8, M["red"], fem, Lf * 0.52, 1.4, 0)
        hub(k, tib, 5.6, 9.6)
        k.pl(Lt - 4, 4.6, 5.2, M["armor"], tib, Lt / 2, 0, 0)
        k.pl(Lt * 0.36, 5.4, 6.0, M["armor2"], tib, Lt * 0.62, -0.2, 0)
        k.box(2.2, 5.8, 6.2, M["red"], tib, Lt * 0.36, -0.2, 0)
        k.box(2.2, 5.8, 6.2, M["red"], tib, Lt * 0.86, -0.2, 0)
        k.hose(fem, [[Lf * 0.3, 3.4, 3.6], [Lf * 0.85, 5.8, 3.6], [Lf + 3, 2, 4.2]], 0.55)
        k.sph(2.4, M["dark"], foot, 0, 0, 0, seg=10)
        k.pl(3.4, Ld - 2.4, 3.4, M["frame"], foot, 0, -(Ld - 2.4) / 2, 0)
        k.pz([[-3, 2.6], [3.4, 2.6], [7, 0.6], [6, 0], [-3, 0]], 4.2, M["armor2"], foot, 0, -Ld, 0, bv=0.3)

    for z, yaw in ((11, 0.8), (-11, -0.8)):
        n = 1 if z > 0 else 2
        for S in ("L", "R"):
            leg(k, body, body_y, S, n, (15, -1, z), yaw, 5, 30, 40, 6, -2.45, 0.8, style)
        k.piston(N(f"leg_L{n}_femur"), [7, -3.5, 0], N(f"leg_L{n}_tibia"), [10, -3.2, 0], 1.0)
        k.piston(N(f"leg_R{n}_femur"), [7, -3.5, 0], N(f"leg_R{n}_tibia"), [10, -3.2, 0], 1.0)

    # two small grabbers under the face
    for S, sx in (("L", 1), ("R", -1)):
        arm, wrist, claw = claw_chain(k, body, S, (6, -8, 18), 0.25, -0.45, 7, 5, 0.5, 0.2, 0.3)
        k.sph(1.8, M["frame"], arm, 0, 0, 0, seg=10)
        k.pl(7, 2.4, 2.6, M["armor2"], arm, 3.5, 0, 0)
        k.pl(5, 2.2, 2.4, M["frame"], wrist, 2.5, 0, 0)
        k.pl(3, 3, 3, M["armor"], claw, 1, 0, 0)
        k.pz([[0, -0.6], [4, -0.6], [5, 0.6], [0, 0.6]], 1.6, M["metal"], claw, 2, -0.9, 0, bv=0.15)
        pin = k.J(f"pincer_{S}", claw, 2.2, 0.9, 0, 0, 0, 0.25)
        k.pz([[0, -0.5], [4, -0.4], [5, -1.2], [0, 0.6]], 1.4, M["metal"], pin, 0, 0, 0, bv=0.15)

    k.extras.update(gait=dict(hz=1.9, swing=0.32, lift=0.42, scuttle_m=2.2, bob=0.12),
                    idle=dict(scan=0.9, scanPeriod=1.4), snap=dict(hz=1.4, open=0.35, dir=1))


# ----------------------------------------------------------------------------- SNAPPER (reference 2)

def snapper(k):
    """A flat armoured hull with a square face, twin headlight eyes and a grille, two big hooked claws
    raised high on long arms, two antenna masts at the back, six short folded legs on box feet."""
    M = k.M
    body_y = 30
    body = k.J("body", k.root, 0, body_y, 0)
    k.pl(58, 14, 48, M["frame"], body, 0, -3, 0)
    k.pl(66, 11, 54, M["armor"], body, 0, 5, -2)
    k.px([[-27, 0], [22, 0], [16, 7], [-22, 7]], 56, M["armor2"], body, 0, 10.5, -2)
    k.pl(30, 3, 20, M["armor"], body, 0, 18.5, -6)
    k.stripes(body, 50, 0, 7.5, 25.3, 0, hb=1.3, gap=0.7)
    for sx in (-1, 1):
        k.pl(4, 14, 46, M["armor2"], body, sx * 34, 1, -2, 0, 0, sx * 0.12)
        glyphs(k, body, sx * 36.4, 2, -6, 3.2, 4, 7 + sx, 0, sx * PI / 2, sx * 0.12)
        k.bolts(body, 6, -16, -5.5, 0, 6, 0.5)
    # the square face: twin headlight eyes, a grille, a heavy brow and a toothed lower jaw
    face = k.g(body, 0, 2, 27)
    k.pl(44, 17, 6, M["armor"], face, 0, 0, 0)
    k.pl(48, 3.4, 9, M["armor2"], face, 0, 9.4, 0.5, 0.2)
    for sx, S in ((1, "L"), (-1, "R")):
        k.pl(12, 11, 2, M["frame"], face, sx * 12.5, 1.5, 3.2)
        headlight(k, face, sx * 12.5, 1.5, 4.3, 3.6, S)
    k.vent(face, 11, 7, 0, 1, 3.4, 0, 0, 4)
    k.pl(30, 4.4, 7, M["frame"], face, 0, -9.5, -0.5)
    for i in range(6):
        k.pz([[-1.2, 0], [1.2, 0], [0, -2.6]], 2.4, M["metal"], face, -9 + i * 3.6, -11.7, 1.5, bv=0.1)
    for S, sx in (("L", 1), ("R", -1)):
        mand = k.J(f"mandible_{S}", face, sx * 5, -11, 3)
        k.px([[0, 0], [5, 0], [6, -3], [0, -1.5]], 2, M["metal"], mand, sx * 1.6, 0, 0, bv=0.1)
    # the hatch, a reactor core and the engine block at the back
    k.cyl(7, 7.6, 2.6, M["armor2"], body, 0, 21, -7, seg=20)
    k.cyl(5, 5, 0.6, M["frame"], body, 0, 22.5, -7, seg=20)
    k.core(body, 0, 21.5, -20, 3.4)
    k.pl(42, 13, 10, M["frame"], body, 0, 2, -31)
    for sx in (-1, 1):
        k.vent(body, 10, 7, sx * 11.5, 2, -36.4, 0, PI, 3)
    k.hose(body, [[-16, 8, -30], [-24, 12, -22], [-26, 9, -12]], 1.0)
    k.hose(body, [[16, 8, -30], [24, 12, -22], [26, 9, -12]], 1.0)
    mast(k, body, "L", 13, 19, -15, 30, -0.22, -0.16)
    mast(k, body, "R", -13, 19, -15, 30, -0.22, 0.16)
    k.lamp(body, 22, 17.5, -20, 0.9, 5)

    def style(k, cox, fem, tib, foot, s, Lc, Lf, Lt, Ld):
        k.cyl(5, 5, 8, M["frame"], cox, 0, 0, 0, seg=16)
        k.pl(Lc + 4, 8, 9, M["armor2"], cox, Lc / 2, 0, 0)
        hub(k, fem, 5.4, 10, M["armor2"])
        k.pl(Lf, 7, 8, M["armor"], fem, Lf / 2, 0, 0)
        k.pl(Lf * 0.8, 2, 9, M["armor2"], fem, Lf / 2, 3.8, 0)
        hub(k, tib, 5, 10.4, M["armor2"])
        k.pl(Lt, 6.4, 7.4, M["armor"], tib, Lt / 2, 0, 0)
        k.pl(Lt * 0.55, 7.2, 8.4, M["armor2"], tib, Lt * 0.4, 0.2, 0)
        k.box(2, 7.4, 8.6, M["red"], tib, Lt * 0.72, 0.2, 0)
        k.sph(3.4, M["dark"], foot, 0, 0, 0, seg=12)
        k.pl(5, Ld - 3, 5, M["frame"], foot, 0, -(Ld - 3) / 2, 0)
        k.pl(12, 3.6, 10, M["armor2"], foot, 1.5, -Ld + 2.6, 0)
        k.box(12.4, 0.8, 10.4, M["rubber"], foot, 1.5, -Ld + 0.4, 0)

    for n, (z, yaw) in enumerate(((17, 0.5), (0, 0.05), (-17, -0.45)), 1):
        for S in ("L", "R"):
            leg(k, body, body_y, S, n, (30, -6, z), yaw, 7, 18, 24, 9, -2.0, 0.6, style)
            k.piston(N(f"leg_{S}{n}_femur"), [4, -3.6, 4.6], N(f"leg_{S}{n}_tibia"), [9, -2, 4.4], 1.0)

    # the claws: long arms raised high, a hooked pincer on each; the left one a size bigger
    for S, sx, f in (("L", 1, 1.15), ("R", -1, 1.0)):
        arm, wrist, claw = claw_chain(k, body, S, (23, 8, 22), 0.45, 0.85, 26 * f, 18 * f, 0.5, 0.22, 0.2)
        k.sph(6, M["frame"], arm, 0, 0, 0, seg=14)
        k.pl(26 * f, 8 * f, 9 * f, M["armor"], arm, 13 * f, 0, 0)
        k.pl(18 * f, 2.2, 10 * f, M["armor2"], arm, 13 * f, 4.4 * f, 0)
        k.stripes(arm, 10 * f, 9 * f, 0, 4.8 * f, PI / 2, hb=1.1, gap=0.6, d=0.4)
        k.cyl(4.6 * f, 4.6 * f, 10 * f, M["dark"], wrist, 0, 0, 0, PI / 2, seg=16)
        k.pl(18 * f, 7.4 * f, 8 * f, M["armor2"], wrist, 9 * f, 0, 0)
        k.pl(12 * f, 6, 9 * f, M["armor"], wrist, 10 * f, 1, 0)
        k.piston(N("body"), [sx * 21, 0, 20], N(f"arm_{S}"), [12 * f, -4.5 * f, 0], 1.2)
        k.piston(N(f"arm_{S}"), [8 * f, 4.5 * f, 0], N(f"wrist_{S}"), [8 * f, 4 * f, 0], 1.0)
        k.cyl(4.8 * f, 4.8 * f, 10.5 * f, M["frame"], claw, 0, 0, 0, PI / 2, seg=16)
        k.pl(14 * f, 16 * f, 12 * f, M["armor"], claw, 5 * f, 0, 0)
        k.box(12 * f, 1.4, 12.6 * f, M["red"], claw, 5 * f, 3 * f, 0)
        k.pz(hook(15 * f, 11 * f, 2.4, -1), 10 * f, M["armor"], claw, 10 * f, 3.5 * f, 0, bv=0.6)
        k.pz(hook(15 * f, 4 * f, 2.2, -1), 11 * f, M["frame"], claw, 10.5 * f, 3.2 * f, 0, bv=0.3)
        k.sph(1.5 * f, M["glow"], claw, 12 * f, -1 * f, 0, seg=10)
        k.halo(claw, 12.5 * f, -1 * f, 0, 6 * f, 0.7)
        k.sock(f"MUZZLE_claw_{S}", claw, 12.5 * f, -1 * f, 0, 0, PI / 2)
        pin = k.J(f"pincer_{S}", claw, 11 * f, -5 * f, 0, 0, 0, -0.35)
        k.pz(hook(10 * f, 8 * f, 1.9, 1), 8 * f, M["armor2"], pin, 0, 0, 0, bv=0.5)

    k.extras.update(gait=dict(hz=1.1, swing=0.26, lift=0.32, scuttle_m=2.5, bob=0.08),
                    idle=dict(scan=0.4, scanPeriod=2.4), snap=dict(hz=0.7, open=0.45, dir=-1))


# ----------------------------------------------------------------------------- BIG BOY (reference 1)

def bigboy(k):
    """A wide flat lens shell over a heavy chassis, one giant boxy claw held forward with a beam
    emitter between its jaws, a small claw on the other side, short thick piston legs on toothed feet,
    two antenna masts and weak points (violet core and tanks) for lock-on."""
    M = k.M
    body_y = 64
    body = k.J("body", k.root, 0, body_y, 0)
    k.pl(108, 30, 92, M["frame"], body, 0, 0, 0)
    k.pl(96, 12, 80, M["dark"], body, 0, -18, 0)
    for sx in (-1, 1):
        for z in (38, 0, -38):
            k.pl(20, 26, 22, M["armor2"], body, sx * 56, -4, z)
    k.vent(body, 30, 10, 0, -2, 46.2, 0, 0, 4)
    k.core(body, 0, -20, 0, 7)
    k.sock("WEAK_belly", body, 0, -20, 0)
    # the lens shell, tipped forward a little
    sh = k.J("shell", body, 0, 22, 0, 0.07)
    R, ht, hb = 96, 26, 12
    prof = [(R * i / 10, ht * (1 - (i / 10) ** 2)) for i in range(11)]
    prof += [(R * 0.92, -hb * 0.45), (R * 0.72, -hb * 0.85), (R * 0.4, -hb), (0, -hb)]
    prof[0] = (0, ht)
    lathe(k, prof, M["armor"], sh, 0, 0, 0, seg=48, scale=(1, 1, 0.86))
    ring(k, R + 1, 3.2, M["frame"], sh, 0, 0, 0, zscale=0.86, seg=56)
    ring(k, R * 0.62, 1.4, M["red"], sh, 0, ht * (1 - 0.62 ** 2) + 0.3, 0, zscale=0.86, seg=48)

    def on_top(rr, ang):
        """A point on the shell's top at radius fraction rr and bearing ang, with the surface tilt."""
        x, z = rr * R * math.sin(ang), rr * R * math.cos(ang) * 0.86
        y = ht * (1 - rr * rr)
        slope = 2 * ht * rr / R
        return x, y, z, math.atan(slope) * math.cos(ang), -math.atan(slope) * math.sin(ang)

    k.cyl(17, 19, 4, M["armor2"], sh, 0, ht + 1, -8, seg=28)
    k.cyl(12, 12, 1.2, M["frame"], sh, 0, ht + 3.4, -8, seg=28)
    k.bolts(sh, 6, -10, ht + 3.2, 4, 4, 0.7)
    for i, ang in enumerate((-0.6, -0.3, 0.3, 0.6, 2.6, 3.0, 3.3, 3.7)):
        x, y, z, rx, rz = on_top(0.8, ang)
        k.pl(14, 2.2, 18, M["armor2"], sh, x, y + 1.1, z, rx, ang, 0)
    # an inner ring of plates, radial seams between the outer ones and three hatches
    for ang in (-1.25, -1.85, 1.25, 1.85, 2.35, -2.35):
        x, y, z, rx, rz = on_top(0.46, ang)
        k.pl(12, 2.4, 15, M["armor2"], sh, x, y + 1.0, z, rx, ang, 0)
    for i in range(16):
        ang = (i + 0.5) / 16 * K.TAU
        x, y, z, rx, rz = on_top(0.83, ang)
        k.box(0.8, 0.9, 22, M["ink"], sh, x, y + 0.25, z, rx * 0.9, ang, 0)
    for ang in (0.95, -0.95, PI):
        x, y, z, rx, rz = on_top(0.62, ang)
        k.cyl(4.2, 4.6, 1.6, M["frame"], sh, x, y + 0.6, z, rx, ang, 0, seg=16)
        k.cyl(2.6, 2.6, 0.6, M["glowDim"], sh, x, y + 1.5, z, rx, ang, 0, seg=16)
    # glyph markings across the front left of the shell, as the reference's lettering
    for row, (rr, ang, cnt) in enumerate(((0.55, 0.42, 4), (0.4, 0.62, 3))):
        x, y, z, rx, rz = on_top(rr, ang)
        glyphs(k, sh, x, y + 0.3, z, 6, cnt, 40 + row, rx - PI / 2, ang, 0)
    # armour tags hanging over the front rim, with glyphs
    for sx in (-1, 1):
        tag = k.g(sh, sx * 44, -12, 74, 0.18, sx * 0.45, 0)
        k.pl(18, 34, 3.5, M["armor2"], tag, 0, 0, 0)
        k.box(18.6, 3, 4, M["red"], tag, 0, 12, 0)
        glyphs(k, tag, 0, -4, 1.9, 4, 3, 60 + sx, 0, 0, PI / 2)
    # the sensor cluster under the front rim, and vents along it
    for i, (x, y) in enumerate(((-15, -9), (0, -11), (15, -9))):
        headlight(k, sh, x, y, 70, 4.2, str(i + 1))
    for sx in (-1, 1):
        k.vent(sh, 18, 6, sx * 34, -7, 66, 0, 0, 3)
    # the back: two great tanks with glowing bands (weak points) and the reactor
    for S, sx in (("L", 1), ("R", -1)):
        tk = k.g(sh, sx * 36, 16, -58, PI / 2)
        lathe(k, [(0, 30), (12, 27), (19, 18), (21, 0), (19, -18), (12, -27), (0, -30)], M["armor"], tk, seg=28)
        for yy in (-14, 0, 14):
            k.cyl(21.6, 21.6, 2.2, M["frame"], tk, 0, yy, 0, seg=28)
        k.cyl(21.4, 21.4, 4, M["glowDim"], tk, 0, 7, 0, seg=28)
        k.sock(f"WEAK_tank_{S}", sh, sx * 36, 38, -58)
    k.core(sh, 0, 22, -34, 8)
    k.sock("WEAK_core", sh, 0, 22, -34)
    mast(k, sh, "L", 28, ht * (1 - 0.29 ** 2) - 1, 6, 70, -0.04, -0.03, r=1.6)
    mast(k, sh, "R", -12, ht - 1, 18, 58, -0.06, 0.04, r=1.6)

    def style(k, cox, fem, tib, foot, s, Lc, Lf, Lt, Ld):
        k.cyl(12, 12, 18, M["frame"], cox, 0, 0, 0, seg=20)
        k.pl(Lc + 8, 18, 20, M["armor2"], cox, Lc / 2, 0, 0)
        hub(k, fem, 11, 20, M["armor2"])
        k.pl(Lf, 15, 17, M["armor"], fem, Lf / 2, 0, 0)
        k.pl(Lf * 0.7, 3, 18.4, M["armor2"], fem, Lf * 0.5, 8, 0)
        k.stripes(fem, 14, Lf * 0.5, 0, 8.8, 0, hb=2, gap=1, d=0.5)
        hub(k, tib, 10, 21, M["armor2"])
        k.pl(Lt, 13, 15, M["armor"], tib, Lt / 2, 0, 0)
        k.pl(6, 17, 18, M["frame"], tib, Lt * 0.45, 0, 0)
        k.pl(Lt * 0.3, 15, 17, M["armor2"], tib, Lt * 0.78, 0, 0)
        k.vent(tib, 8, 6, Lt * 0.25, 0, 7.6, 0, 0, 3)
        k.sph(8, M["dark"], foot, 0, 0, 0, seg=14)
        k.pl(11, Ld - 7, 11, M["frame"], foot, 0, -(Ld - 7) / 2, 0)
        k.pl(24, 7, 22, M["armor2"], foot, 1, -Ld + 4.5, 0)
        for a in (-0.55, -0.18, 0.18, 0.55):
            toe = k.g(foot, 0, -Ld, 0, 0, a, 0)
            k.pz([[0, 0], [17, 0], [17, 1.6], [4, 7], [0, 7]], 5, M["armor"], toe, 8, 0, 0, bv=0.4)
        k.pz([[0, 0], [-9, 0], [-9, 1.4], [0, 6]], 6, M["armor"], foot, -9, -Ld, 0, bv=0.4)

    for n, (z, yaw) in enumerate(((38, 0.55), (0, 0.05), (-38, -0.5)), 1):
        for S in ("L", "R"):
            leg(k, body, body_y, S, n, (58, -10, z), yaw, 14, 40, 46, 20, -1.9, 0.35, style)
            for zz in (-11.5, 11.5):
                k.piston(N(f"leg_{S}{n}_femur"), [8, -8, zz], N(f"leg_{S}{n}_tibia"), [14, -6, zz], 2.2)
            k.piston(N(f"leg_{S}{n}_coxa"), [2, 10, 0], N(f"leg_{S}{n}_femur"), [26, 8, 0], 2.0)

    # the giant claw on the left, held forward and low
    arm, wrist, claw = claw_chain(k, body, "L", (40, -4, 44), 0.5, 0.75, 40, 26, -0.55, 0.7, -0.7)
    k.sph(12, M["frame"], arm, 0, 0, 0, seg=16)
    k.pl(40, 16, 18, M["armor"], arm, 20, 0, 0)
    k.pl(30, 3, 19, M["armor2"], arm, 20, 9, 0)
    k.piston(N("body"), [42, -14, 34], N("arm_L"), [24, -9, 0], 2.6)
    k.cyl(11, 11, 21, M["dark"], wrist, 0, 0, 0, PI / 2, seg=20)
    k.pl(26, 18, 20, M["armor2"], wrist, 13, 0, 0)
    k.piston(N("arm_L"), [26, 9, 0], N("wrist_L"), [16, 10, 0], 2.2)
    k.cyl(13, 13, 24, M["frame"], claw, 0, 0, 0, PI / 2, seg=22)
    k.pl(84, 46, 32, M["armor"], claw, 42, 0, 0)
    for zz in (-16.4, 16.4):
        k.stripes(claw, 50, 40, 8, zz, 0 if zz > 0 else PI, hb=2.4, gap=1.2, d=0.6)
        glyphs(k, claw, 40, -8, zz + (0.35 if zz > 0 else -0.35), 6, 4, 77 if zz > 0 else 78, 0, 0 if zz > 0 else PI, 0)
    k.vent(claw, 22, 6, 30, 23.2, 0, -PI / 2, 0, 4)
    k.bolts(claw, 8, 6, 23.2, 12, 9, 0.9, -PI / 2)
    k.pl(10, 50, 36, M["frame"], claw, 82, 0, 0)
    k.pz([[0, -11], [38, -11], [48, -2], [38, 9], [0, 11]], 28, M["armor2"], claw, 86, -13, 0, bv=0.9)
    for i in range(4):
        k.pz([[0, 0], [5, 0], [2.5, 4.5]], 22, M["metal"], claw, 90 + i * 9, -2, 0, bv=0.2)
    k.cyl(9, 9, 2, M["ink"], claw, 87.5, 4, 0, 0, 0, -PI / 2, seg=20)
    k.cyl(6, 6, 2, M["glow"], claw, 88.2, 4, 0, 0, 0, -PI / 2, seg=20)
    k.halo(claw, 90, 4, 0, 30, 0.85)
    k.sock("MUZZLE_claw_L", claw, 89, 4, 0, 0, PI / 2)
    pin = k.J("pincer_L", claw, 86, 15, 0, 0, 0, 0.18)
    k.pz([[0, -10], [36, -7], [46, -1], [36, 7], [0, 10]], 26, M["armor2"], pin, 0, 0, 0, bv=0.9)
    for i in range(4):
        k.pz([[0, 0], [5, 0], [2.5, -4.5]], 20, M["metal"], pin, 6 + i * 9, -9, 0, bv=0.2)
    # the small claw on the right, folded under the rim
    arm, wrist, claw = claw_chain(k, body, "R", (38, -10, 44), 0.45, -0.1, 24, 16, -0.4, 0.6, -0.2)
    k.sph(7, M["frame"], arm, 0, 0, 0, seg=14)
    k.pl(24, 9, 10, M["armor"], arm, 12, 0, 0)
    k.cyl(6, 6, 11, M["dark"], wrist, 0, 0, 0, PI / 2, seg=16)
    k.pl(16, 9, 10, M["armor2"], wrist, 8, 0, 0)
    k.pl(20, 13, 10, M["armor"], claw, 10, 0, 0)
    k.pz([[0, -3], [16, -3], [20, 0], [0, 3]], 8, M["armor2"], claw, 20, -3.5, 0, bv=0.4)
    k.sock("MUZZLE_claw_R", claw, 21, 1, 0, 0, PI / 2)
    pin = k.J("pincer_R", claw, 20, 4, 0, 0, 0, 0.2)
    k.pz([[0, -3], [15, -2], [19, 0.5], [0, 3]], 7, M["armor2"], pin, 0, 0, 0, bv=0.4)

    k.extras.update(gait=dict(hz=0.42, swing=0.16, lift=0.2, scuttle_m=3.5, bob=0.3),
                    idle=dict(scan=0.25, scanPeriod=4.0), snap=dict(hz=0.25, open=0.3, dir=1))


# ----------------------------------------------------------------------------- run

DESIGNS = {
    # id: (builder, height m, plate tint, second plate tint, name, card)
    "skitter": (skitter, 6.0, 0xc4cad6, 0xeef0f4, "Skitter",
                dict(role="fast scuttler and missile skirmisher",
                     reference="the four-legged spider crab sheet",
                     legs="four tall arched legs, big joint hubs, hydraulic rods",
                     weapons="twin four-tube missile pods, twin turret guns",
                     note="Light and quick: it runs sideways in packs, stops to loose its two pods of four "
                          "missiles and is gone again. Its one big eye in the pale face plate and the eye in "
                          "the turret are what it sees with; the legs are long and thin and break first.")),
    "snapper": (snapper, 12.0, 0x7c8bb4, 0x5d6a8e, "Snapper",
                dict(role="main line assault crab",
                     reference="the slate-blue crab sheet",
                     legs="six short folded legs on box feet",
                     weapons="two hooked claws with short-range emitters in the palms",
                     note="The invaders' line machine, the human mechs' size: a flat armoured hull on six "
                          "stubby legs, a square face with twin headlight eyes over a grille, and two big hooked "
                          "claws carried high to crush a mech's torso. The left claw is a size bigger.")),
    "bigboy": (bigboy, 30.0, 0xd89a88, 0xb87a6c, "Big Boy",
               dict(role="siege crab, a set-piece boss",
                    reference="the Big Boy sheet",
                    legs="six short thick piston legs on toothed splayed feet",
                    weapons="beam emitter between the jaws of the giant claw, small claw",
                    note="A walking fortress for an Ace Combat style set piece: a 30 m lens-shelled crab "
                         "that drags one giant boxy claw with a beam weapon in its jaws. Its weak points are "
                         "named for lock-on: the violet core on its back, the two glowing tanks behind it and "
                         "the core under its belly.")),
}


def build(key, s):
    fn, height, tint, tint2, name, card = DESIGNS[key]
    K.clear_scene()
    mats = alien_mats(f"alien_{key}", tint, tint2)
    k = Kit(mats, s, key)
    k.extras.update(glow=GLOW, core=CORE)
    fn(k)
    K.finish(k)
    return k


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    os.makedirs(OUT, exist_ok=True)
    for key in argv or list(DESIGNS):
        fn, height, tint, tint2, name, card = DESIGNS[key]
        build(key, 1.0)
        _, b = K.measure()
        s = height / (b["y"][1] - b["y"][0])
        k = build(key, s)
        extras = dict(k.extras, rest={n: r for n, r in k.rest.items()}, scale=s, rig="alien-crab")
        meta = dict(id=key, name=name, generator="tools/blender/build_alien_mechs.py", status="design candidate",
                    rig="docs/design/alien-rig.md", target_height_m=height,
                    style="Mining Mike boss kit (tank_bosses_kit.js at 3336b88), alien glows", **card)
        meta = K.export(os.path.join(OUT, f"alien_{key}.glb"), MANIFEST, meta, extras)
        print(f"BOUNDS {key}: {meta['bounds_m']}")


main()
