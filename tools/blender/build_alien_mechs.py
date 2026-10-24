"""Build the alien crab mechs, the invaders the human mechs fight, as glb files on the alien rig.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_alien_mechs.py [-- id ...]

Writes assets/models/alien_<id>.glb and assets/models/aliens.json (the measured manifest). Built with
mech_kit.py (the port of Mining Mike's boss mech kit) in that kit's units, then rebuilt at the scale
that makes each one its height in metres, the same two passes as tools/blender/build_mechs.py.

The owner's direction (2026-10-24): "design some potentially crab looking mechs from enemy aliens",
with three reference sheets of hard-surface mechanical crabs. Each design follows one of them:

  skitter  6 m scuttler, after the four-legged spider crab: a rounded hull on four tall arched legs
           with big joint hubs and hydraulic rods, twin SMG arms at the front, twin missile pods on arms
           over the hull, a turret with a sensor eye, violet hazard bands.
  snapper  12 m main line crab, after the slate-blue crab: a flat armoured hull with a square face,
           twin headlight eyes and a grille, a heavy cannon arm and a sword arm raised high, two
           antenna masts, six short folded legs on box feet.
  bigboy   30 m siege crab, after "Big Boy": a wide flat lens shell, a giant boxy siege cannon held
           forward and a missile battery, short thick piston legs on toothed splayed feet, two masts.

No claws (owner, 2026-10-24: "crab should not have big claw features. It's claws are guns or swords or
missile launchers", and "Follow design philosophy of boss mechs from mining mike"). As on Mining Mike's
bosses (SKARN, BRISK, SHIVER, ROOK, BULWARK in tank_bosses_kit.js at 3336b88), every limb that would be
a hand is a weapon with a role you can read from its silhouette: the cannon is a housing with a
reactor core and cooling rings round the barrel (Skarn's breach cannon, Bulwark's breaker), the SMGs
are Brisk's twin barrels with a drum, the sword is Shiver's blade with a glowing edge, the missile
rack is Rook's pod of tubes with violet heads.

What makes them alien rather than human: acid green glows (vents, eyes, mast tips), violet reactor
cores and violet livery with alien glyph markings; the human mechs glow amber, cyan and red. The
skeleton is docs/design/alien-rig.md, not the biped contract of docs/design/mech-rig.md.

Coordinates are glTF style, as in the kit: +Y up, the crab faces +Z, +X is its left. The kit is
imported unchanged; everything crab-specific (legs solved to the ground, weapon arms, masts, lens
shells, glyphs) is in this file.
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


def weapon_arm(k, body, S, at, yaw, raise_, arm_len, wrist_bend, wrist_fold):
    """arm_<S> (shoulder: yaw and raise at the hull) and wrist_<S> (elbow: bend about Z, fold toward the
    centre line about Y). Both run along their joint's +X. The weapon joint goes on the wrist's end."""
    s = 1 if S == "L" else -1
    ry_l = -PI / 2 + yaw
    arm = k.J(f"arm_{S}", body, s * at[0], at[1], at[2], 0, ry_l if s > 0 else PI - ry_l, raise_)
    wrist = k.J(f"wrist_{S}", arm, arm_len, 0, 0, 0, -s * wrist_fold, wrist_bend)
    k.extras.setdefault("arms", []).append(S)
    return arm, wrist


def register(k, joint, kind, muzzles, recoil=0.0, flash=4.0, period=2.0, axis=(1, 0, 0)):
    """Name a weapon for the engine and the review page: its joint, its kind (cannon, smg, blade, rack),
    its fire points, how far it recoils (kit units in, metres out), its flash size and fire period."""
    k.extras.setdefault("weapons", []).append(dict(
        joint=joint, kind=kind, muzzles=muzzles, recoil_m=round(recoil * k.s, 4), flash_m=round(flash * k.s, 4),
        period_s=period, axis=list(axis)))


def cannon(k, j, S, L, r, cores=1, name=None):
    """A heavy cannon along the joint's +X: a trunnion hub, an armoured breech housing with a reactor
    core on top and livery down both flanks, a barrel ringed with glowing cooling rings and collars, a
    muzzle brake with a glowing bore. MUZZLE_<name> fires along its +Z (the joint's +X)."""
    M = k.M
    name = name or f"gun_{S}"
    hub(k, j, r * 1.25, r * 2.5, M["armor2"])
    x0, hl = r * 0.4, L * 0.42
    k.pl(hl, r * 2.8, r * 2.6, M["armor"], j, x0 + hl / 2, 0, 0)
    k.pl(hl * 0.8, r * 0.5, r * 2.75, M["armor2"], j, x0 + hl * 0.45, r * 1.45, 0)
    for zz, ry in ((r * 1.3 + 0.05, 0), (-(r * 1.3 + 0.05), PI)):
        k.stripes(j, hl * 0.7, x0 + hl * 0.5, -r * 0.35, zz, ry, hb=r * 0.32, gap=r * 0.16, d=0.4)
    for i in range(cores):
        k.core(j, x0 + hl * (0.3 + 0.4 * i) if cores > 1 else x0 + hl * 0.55, r * 1.75, 0, r * 0.55)
    k.vent(j, r * 1.4, r * 0.9, x0 + hl * 0.18, r * 1.42, r * 0.6, -PI / 2, 0, 3)
    k.bolts(j, 5, x0 + hl * 0.15, -r * 1.42, r * 1.1, hl * 0.16, r * 0.12, PI / 2)
    k.hose(j, [[x0 + hl * 0.1, -r * 1.2, -r * 1.1], [x0 + hl * 0.5, -r * 1.7, -r * 1.2], [x0 + hl * 0.9, -r * 1.2, -r * 1.1]], r * 0.16)
    xs = x0 + hl
    k.pl(r * 1.1, r * 2.2, r * 2.2, M["frame"], j, xs + r * 0.5, 0, 0)
    bl = L - hl - r * 2.2
    xb = xs + r * 1.0
    k.cyl(r * 0.62, r * 0.72, bl, M["metal"], j, xb + bl / 2, 0, 0, 0, 0, -PI / 2, seg=18)
    for f in (0.12, 0.3, 0.48):
        k.tor(r * 0.92, r * 0.12, M["glowDim"], j, xb + bl * f, 0, 0, 0, PI / 2, 0, seg=24)
    for f in (0.2, 0.39, 0.62):
        k.cyl(r * 0.88, r * 0.88, r * 0.35, M["frame"], j, xb + bl * f, 0, 0, 0, 0, -PI / 2, seg=18)
    xe = xb + bl
    k.pl(r * 1.4, r * 1.7, r * 1.7, M["dark"], j, xe + r * 0.55, 0, 0)
    for yy in (-r * 0.5, r * 0.5):
        k.box(r * 0.9, r * 0.22, r * 1.8, M["ink"], j, xe + r * 0.6, yy, 0)
    k.cyl(r * 0.5, r * 0.5, 0.3, M["ink"], j, xe + r * 1.27, 0, 0, 0, 0, -PI / 2, seg=16)
    k.cyl(r * 0.34, r * 0.34, 0.3, M["glow"], j, xe + r * 1.3, 0, 0, 0, 0, -PI / 2, seg=16)
    k.halo(j, xe + r * 1.5, 0, 0, r * 2.6, 0.7)
    m = f"MUZZLE_{name}"
    k.sock(m, j, xe + r * 1.45, 0, 0, 0, PI / 2)
    return [m]


def smg(k, j, S, L, r, name=None):
    """Twin-barrel SMG after Brisk's: a hub, a receiver with livery, a drum under it, two barrels in a
    shroud, glowing barrel tips. One MUZZLE_<name> between the barrels."""
    M = k.M
    name = name or f"gun_{S}"
    hub(k, j, r * 1.3, r * 2.2, M["armor2"])
    k.pl(L * 0.5, r * 2.3, r * 2.0, M["armor"], j, L * 0.3, 0, 0)
    k.stripes(j, L * 0.36, L * 0.3, r * 0.2, r * 1.0 + 0.05, 0, hb=r * 0.3, gap=r * 0.15, d=0.3)
    k.stripes(j, L * 0.36, L * 0.3, r * 0.2, -(r * 1.0 + 0.05), PI, hb=r * 0.3, gap=r * 0.15, d=0.3)
    k.cyl(r * 1.15, r * 1.15, r * 1.3, M["dark"], j, L * 0.3, -r * 1.6, 0, PI / 2, seg=18)
    k.cyl(r * 0.6, r * 0.6, r * 1.4, M["frame"], j, L * 0.3, -r * 1.6, 0, PI / 2, seg=12)
    k.pl(L * 0.22, r * 1.9, r * 1.7, M["frame"], j, L * 0.62, 0, 0)
    for dy in (-r * 0.42, r * 0.42):
        k.cyl(r * 0.3, r * 0.3, L * 0.42, M["metal"], j, L * 0.75, dy, 0, 0, 0, -PI / 2, seg=10)
        k.cyl(r * 0.36, r * 0.36, r * 0.5, M["glowDim"], j, L * 0.94, dy, 0, 0, 0, -PI / 2, seg=10)
    k.box(r * 0.7, r * 0.35, r * 0.35, M["glow"], j, L * 0.35, r * 1.25, 0)
    m = f"MUZZLE_{name}"
    k.sock(m, j, L * 0.97, 0, 0, 0, PI / 2)
    k.sock(f"EJECT_{name}", j, L * 0.38, 0, r * 1.1)
    return [m]


def blade(k, j, S, L, w):
    """A sword arm after Shiver's blade: a hub and guard at the wrist, a long blade along +X in the XY
    plane (so it swings about Z), a glowing edge on its lower side, a spine ridge along the top.
    BLADE_tip_<S> and BLADE_mid_<S> mark the tip and the middle of the edge."""
    M = k.M
    hub(k, j, w * 0.5, w * 0.85, M["armor2"])
    k.pl(w * 0.45, w * 1.25, w * 0.75, M["frame"], j, w * 0.55, 0, 0)
    k.box(w * 0.2, w * 1.3, w * 0.8, M["red"], j, w * 0.4, 0, 0)
    x0 = w * 0.75
    body_pts = [[x0, -w * 0.42], [x0 + L * 0.4, -w * 0.54], [x0 + L * 0.78, -w * 0.4], [x0 + L, w * 0.04],
                [x0 + L * 0.8, w * 0.3], [x0 + L * 0.35, w * 0.38], [x0, w * 0.3]]
    k.pz(body_pts, w * 0.18, M["metal"], j, 0, 0, 0, bv=0.15)
    edge = [[x0 + L * 0.04, -w * 0.33], [x0 + L * 0.4, -w * 0.45], [x0 + L * 0.78, -w * 0.32], [x0 + L * 0.95, w * 0.0],
            [x0 + L * 0.9, -w * 0.02], [x0 + L * 0.75, -w * 0.22], [x0 + L * 0.4, -w * 0.33], [x0 + L * 0.04, -w * 0.24]]
    k.pz(edge, w * 0.26, M["glow"], j, 0, 0, 0, bv=0.05)
    spine = [[x0, w * 0.3], [x0 + L * 0.35, w * 0.38], [x0 + L * 0.8, w * 0.3], [x0 + L * 0.78, w * 0.2],
             [x0 + L * 0.35, w * 0.26], [x0, w * 0.18]]
    k.pz(spine, w * 0.32, M["armor2"], j, 0, 0, 0, bv=0.1)
    for f in (0.3, 0.6):
        k.halo(j, x0 + L * f, -w * 0.42, 0, w * 1.1, 0.35)
    k.sock(f"BLADE_tip_{S}", j, x0 + L, 0, 0, 0, PI / 2)
    k.sock(f"BLADE_mid_{S}", j, x0 + L * 0.5, -w * 0.45, 0, 0, PI / 2)


def rack(k, j, S, rows, cols, cell, depth, name=None):
    """A missile battery after Rook's pods along the joint's +X: an armoured box on a yoke, livery down
    its flanks, a front frame of rows x cols tubes with violet warheads, a hazard blast plate behind.
    One MUZZLE_<name><i> per tube, numbered from 1."""
    M = k.M
    name = name or f"rack_{S}"
    W, H = cols * cell + cell * 0.8, rows * cell + cell * 0.8
    hub(k, j, cell * 0.9, cell * 1.6, M["armor2"])
    k.pl(cell * 1.6, H * 0.5, W * 0.4, M["frame"], j, cell * 0.8, 0, 0)
    xc = cell * 1.5 + depth / 2
    k.pl(depth, H, W, M["armor"], j, xc, 0, 0)
    for zz, ry in ((W / 2 + 0.05, 0), (-(W / 2 + 0.05), PI)):
        k.stripes(j, depth * 0.75, xc, -H * 0.15, zz, ry, hb=cell * 0.4, gap=cell * 0.2, d=0.4)
    k.pl(depth * 0.9, cell * 0.45, W * 1.04, M["armor2"], j, xc, H / 2 + cell * 0.2, 0)
    xf = cell * 1.5 + depth
    k.box(cell * 0.4, H * 0.96, W * 0.96, M["frame"], j, xf + cell * 0.15, 0, 0)
    k.box(0.4, H * 0.6, W * 0.5, M["hazard"], j, cell * 1.5 - 0.15, 0, 0)
    muz = []
    i = 0
    for r_ in range(rows):
        for c in range(cols):
            i += 1
            y = -H / 2 + cell * 0.4 + cell * (r_ + 0.5)
            z = -W / 2 + cell * 0.4 + cell * (c + 0.5)
            k.cyl(cell * 0.38, cell * 0.38, cell * 0.25, M["ink"], j, xf + cell * 0.4, y, z, 0, 0, -PI / 2, seg=12)
            k.cyl(0, cell * 0.28, cell * 0.55, M["art"], j, xf + cell * 0.4, y, z, 0, 0, -PI / 2, seg=10)
            m = f"MUZZLE_{name}{i}"
            k.sock(m, j, xf + cell * 0.7, y, z, 0, PI / 2)
            muz.append(m)
    return muz


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
    """Four tall arched legs with big hubs and rods, a rounded hull, twin SMG arms at the front, two
    missile pods on arms over it, a sensor turret on top and a single big eye in a pale face plate."""
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

    # twin SMG arms under the face, where a crab carries its claws
    for S, sx in (("L", 1), ("R", -1)):
        arm, wrist = weapon_arm(k, body, S, (12, -4, 15), 0.4, 0.1, 10, -0.1, 0.4)
        k.sph(2.8, M["frame"], arm, 0, 0, 0, seg=12)
        k.pl(10, 3.4, 3.8, M["armor2"], arm, 5, 0, 0)
        k.box(5, 3.6, 4.0, M["red"], arm, 5.5, 0, 0)
        hub(k, wrist, 2.4, 4.2)
        k.pl(7, 3, 3.4, M["frame"], wrist, 3.5, 0, 0)
        g = k.J(f"gun_{S}", wrist, 7, 0, 0, 0, 0, 0.0)
        register(k, f"gun_{S}", "smg", smg(k, g, S, 22, 2.4), recoil=1.2, flash=11, period=2.4)
    register(k, "pod_L", "rack", [f"MUZZLE_pod_L{i}" for i in range(1, 5)], recoil=0.8, flash=12, period=3.2, axis=(0, 0, 1))
    register(k, "pod_R", "rack", [f"MUZZLE_pod_R{i}" for i in range(1, 5)], recoil=0.8, flash=12, period=3.2, axis=(0, 0, 1))
    register(k, "turret_1", "smg", ["MUZZLE_turret_L", "MUZZLE_turret_R"], recoil=0, flash=6, period=2.4, axis=(0, 0, 1))

    k.extras.update(gait=dict(hz=1.9, swing=0.32, lift=0.42, scuttle_m=2.2, bob=0.12),
                    idle=dict(scan=0.9, scanPeriod=1.4))


# ----------------------------------------------------------------------------- SNAPPER (reference 2)

def snapper(k):
    """A flat armoured hull with a square face, twin headlight eyes and a grille, a heavy cannon arm and
    a sword arm raised high, two antenna masts at the back, six short folded legs on box feet."""
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

    # the weapon arms, raised high where the reference carries its claws: a heavy cannon on the left,
    # a sword on the right
    for S, sx, raise_, bend, fold in (("L", 1, 0.75, -0.55, 0.35), ("R", -1, 0.95, -0.25, 0.3)):
        arm, wrist = weapon_arm(k, body, S, (23, 8, 20), 0.4, raise_, 22, bend, fold)
        k.sph(6, M["frame"], arm, 0, 0, 0, seg=14)
        k.pl(22, 8, 9, M["armor"], arm, 11, 0, 0)
        k.pl(16, 2.2, 10, M["armor2"], arm, 11, 4.4, 0)
        k.stripes(arm, 10, 10, 0, 4.8, PI / 2, hb=1.1, gap=0.6, d=0.4)
        hub(k, wrist, 4.6, 10, M["armor2"])
        k.pl(14, 7.4, 8, M["armor2"], wrist, 7, 0, 0)
        k.pl(10, 2, 9, M["armor"], wrist, 8, 3.8, 0)
        k.piston(N("body"), [sx * 21, 0, 20], N(f"arm_{S}"), [12, -4.5, 0], 1.2)
        k.piston(N(f"arm_{S}"), [8, 4.5, 0], N(f"wrist_{S}"), [8, 4, 0], 1.0)
    g = k.J("gun_L", N("wrist_L"), 14, 0, 0, 0, 0, -0.2)
    register(k, "gun_L", "cannon", cannon(k, g, "L", 40, 3.8), recoil=4, flash=16, period=2.2)
    bl = k.J("blade_R", N("wrist_R"), 14, 0, 0, 0, 0, 0.0)
    blade(k, bl, "R", 44, 11)
    register(k, "blade_R", "blade", [], period=2.0)

    k.extras.update(gait=dict(hz=1.1, swing=0.26, lift=0.32, scuttle_m=2.5, bob=0.08),
                    idle=dict(scan=0.4, scanPeriod=2.4))


# ----------------------------------------------------------------------------- BIG BOY (reference 1)

def bigboy(k):
    """A wide flat lens shell over a heavy chassis, a giant boxy siege cannon held forward on the left
    and a twelve-tube missile battery on the right, short thick piston legs on toothed feet, two antenna
    masts and weak points (violet cores, tanks, the cannon's breech) for lock-on."""
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

    # the siege cannon on the left, a giant boxy gun held forward where the reference holds its claw
    arm, wrist = weapon_arm(k, body, "L", (40, -4, 44), 0.5, 0.6, 40, -0.45, 0.6)
    k.sph(12, M["frame"], arm, 0, 0, 0, seg=16)
    k.pl(40, 16, 18, M["armor"], arm, 20, 0, 0)
    k.pl(30, 3, 19, M["armor2"], arm, 20, 9, 0)
    k.piston(N("body"), [42, -14, 34], N("arm_L"), [24, -9, 0], 2.6)
    hub(k, wrist, 11, 21, M["armor2"])
    k.pl(26, 18, 20, M["armor2"], wrist, 13, 0, 0)
    k.piston(N("arm_L"), [26, 9, 0], N("wrist_L"), [16, 10, 0], 2.2)
    g = k.J("gun_L", wrist, 26, 0, 0, 0, 0, -0.1)
    register(k, "gun_L", "cannon", cannon(k, g, "L", 124, 16, cores=2), recoil=10, flash=48, period=3.0)
    for zz, ry, seed in ((20.95, 0, 77), (-20.95, PI, 78)):
        glyphs(k, g, 6.4 + 26, 10, zz, 5, 4, seed, 0, ry, 0)
    k.sock("WEAK_breech", g, 30, 26, 0)
    # the missile battery on the right, on a shorter arm, angled up
    arm, wrist = weapon_arm(k, body, "R", (38, -6, 44), 0.45, 0.12, 38, 0.18, 0.45)
    k.sph(8, M["frame"], arm, 0, 0, 0, seg=14)
    k.pl(38, 11, 12, M["armor"], arm, 19, 0, 0)
    k.pl(28, 2.5, 12.6, M["armor2"], arm, 19, 6.2, 0)
    k.piston(N("body"), [-40, -14, 36], N("arm_R"), [16, -6, 0], 1.8)
    hub(k, wrist, 7, 13, M["armor2"])
    k.pl(16, 10, 11, M["armor2"], wrist, 8, 0, 0)
    rk = k.J("rack_R", wrist, 16, 0, 0, 0, 0, 0.0)
    register(k, "rack_R", "rack", rack(k, rk, "R", 3, 4, 7, 30), recoil=2, flash=10, period=3.4)

    k.extras.update(gait=dict(hz=0.42, swing=0.16, lift=0.2, scuttle_m=3.5, bob=0.3),
                    idle=dict(scan=0.25, scanPeriod=4.0))


# ----------------------------------------------------------------------------- run

DESIGNS = {
    # id: (builder, height m, plate tint, second plate tint, name, card)
    "skitter": (skitter, 6.0, 0xc4cad6, 0xeef0f4, "Skitter",
                dict(role="fast scuttler and missile skirmisher",
                     reference="the four-legged spider crab sheet",
                     legs="four tall arched legs, big joint hubs, hydraulic rods",
                     weapons="twin SMG arms, twin four-tube missile pods, twin turret guns",
                     note="Light and quick: it runs sideways in packs, hosing a mech with the two SMGs it "
                          "carries where a crab has claws, then looses its two pods of four missiles and is "
                          "gone again. One big eye in the pale face plate and one in the turret; the long thin "
                          "legs break first.")),
    "snapper": (snapper, 12.0, 0x7c8bb4, 0x5d6a8e, "Snapper",
                dict(role="main line assault crab",
                     reference="the slate-blue crab sheet",
                     legs="six short folded legs on box feet",
                     weapons="heavy cannon arm (L), sword arm (R)",
                     note="The invaders' line machine, the human mechs' size: a flat armoured hull on six "
                          "stubby legs and a square face with twin headlight eyes over a grille. It carries its "
                          "two arms high: a heavy cannon with a reactor core and cooling rings, and a long sword "
                          "with a glowing edge for when it closes in.")),
    "bigboy": (bigboy, 30.0, 0xd89a88, 0xb87a6c, "Big Boy",
               dict(role="siege crab, a set-piece boss",
                    reference="the Big Boy sheet",
                    legs="six short thick piston legs on toothed splayed feet",
                    weapons="siege cannon (L), twelve-tube missile battery (R)",
                    note="A walking fortress for an Ace Combat style set piece: a 30 m lens-shelled crab "
                         "that carries a giant boxy siege cannon forward and a missile battery angled up. Its "
                         "weak points are named for lock-on: the violet core on its back, the two glowing tanks "
                         "behind it, the core under its belly and the cannon's breech.")),
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
