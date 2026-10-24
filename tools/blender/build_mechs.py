"""Build the Crab Breeze mech design candidates as glb files on the rig contract.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_mechs.py [-- id ...]

Writes assets/models/mech_<id>.glb and assets/models/mechs.json (the measured manifest). Built with
mech_kit.py, the port of Mining Mike's boss mech kit, in that kit's units; each mech is built once to
measure it and again at the scale that makes it its height in metres.

  bulwark  Mining Mike's Uber mech "Bulwark", ported: planted and square, breaker cannon, slab shield.
           The back cradle now holds the flight jets; the head is the cockpit, a visor across it.
  ravager  Mining Mike's Uber mech "Ravager", ported: hunched on reverse knees, twin rail lances,
           a crest of blades. The low three-eyed head is the cockpit.
  foreman    the excavator, the wall cockpit in its chamfered prow behind a visor and an apron (the recommendation).
  stevedore  retired (owner, 2026-10-24: "Stevedore sucks"); its generator is in git history.
  ironhorse  the locomotive, the jockey cockpit in a hexagonal prow that swings open whole.
             Original designs in the boss kit's finish (the owner, 2026-10-24: "Follow design philosophy of
             boss mechs from mining mike. The cockpit is part of the mech, inside of it with the armor
             opening up at the chest", then "New mechs are just stolen. Those are not original mechs");
             the chest hardware is mech_boarding.py.
  breeze   retired ("Breeze me h sucks"), still built because the game loads it until the owner picks.

Provenance: RubenTipparach/godot-sandbox godot-sandbox/docs/mockups/tank_bosses_kit.js at 3336b88,
SPECS.bulwark.build and SPECS.ravager.build, transliterated line by line; what changed is marked
"Crab Breeze:". Joint names follow docs/design/mech-rig.md (body is pelvis, waist is torso_twist,
chest is torso, head is cockpit, sh/elb are shoulder/forearm, hip/knee/tar/ank are thigh/shin/hock/foot).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_boarding as MB  # noqa: E402
import mech_kit as K  # noqa: E402
from mech_palettes import PALETTES  # noqa: E402
from mech_kit import Kit, Node, leg_height  # noqa: E402
from mathutils import Vector  # noqa: E402

PI = math.pi
OUT = os.path.join(K.ROOT, "assets", "models")
MANIFEST = os.path.join(OUT, "mechs.json")


def N(name):
    return Node(name, K.Matrix.Identity(4))


def contract_arm(k, chest, S, x, y, z, rx, elb_y, elb_rx):
    """sh -> (upper_arm) -> elb, as the contract wants: the upper arm is a joint at the shoulder."""
    sh = k.J(f"shoulder_{S}", chest, x, y, z, rx)
    ua = k.J(f"upper_arm_{S}", sh)
    elb = k.J(f"forearm_{S}", ua, 0, elb_y, 0, elb_rx)
    return sh, elb


def cockpit_interior(k, head, ex, ey, ez):
    """Crab Breeze: seat, consoles, screens and controls around the pilot's eye (kit units in, metres
    reasoned): everything below the sightline, nothing in front of it."""
    u = 1.0 / k.s if k.s else 1.0
    M = k.M
    k.sock("EYE_pilot", head, ex, ey, ez)
    seat_y, seat_z = ey - 0.78 * u, ez - 0.12 * u
    k.sock("SEAT_pilot", head, ex, seat_y, seat_z)
    k.box(0.62 * u, 0.12 * u, 0.56 * u, M["seat"], head, ex, seat_y, seat_z)
    k.box(0.6 * u, 0.95 * u, 0.12 * u, M["seat"], head, ex, seat_y + 0.5 * u, seat_z - 0.33 * u, -0.12)
    k.box(0.34 * u, 0.26 * u, 0.12 * u, M["seat"], head, ex, seat_y + 1.05 * u, seat_z - 0.42 * u, -0.12)
    k.box(0.5 * u, 0.3 * u, 0.5 * u, M["dark"], head, ex, seat_y - 0.21 * u, seat_z)
    # front console: its top slopes up away from the pilot; three screens face the eye
    cz, cy = ez + 0.62 * u, ey - 0.62 * u
    k.box(1.7 * u, 0.55 * u, 0.42 * u, M["dark"], head, ex, cy - 0.2 * u, cz, -0.45)
    for dx in (-0.52, 0.0, 0.52):
        k.box(0.32 * u, 0.2 * u, 0.02 * u, M["screen"], head, ex + dx * u, cy + 0.06 * u, cz - 0.13 * u, -0.95)
    for sx in (1, -1):
        k.box(0.22 * u, 0.42 * u, 0.7 * u, M["dark"], head, ex + 0.52 * sx * u, seat_y + 0.2 * u, seat_z + 0.3 * u)
        k.box(0.14 * u, 0.02 * u, 0.22 * u, M["screen"], head, ex + 0.52 * sx * u, seat_y + 0.42 * u, seat_z + 0.35 * u)
    for name, dx, dy, dz in (("stick", 0.0, 0.0, 0.42), ("throttle", -0.5, 0.36, 0.2), ("twist", 0.5, 0.36, 0.2)):
        bx, by, bz = ex + dx * u, seat_y + dy * u, seat_z + dz * u
        k.cyl(0.025 * u, 0.025 * u, 0.3 * u, M["metal"], head, bx, by + 0.15 * u, bz)
        k.sph(0.055 * u, M["rubber"], head, bx, by + 0.32 * u, bz)
        k.sock(f"CTRL_{name}", head, bx, by + 0.32 * u, bz)


def ladder(k, x, z_ahead):
    """Crab Breeze: the boarding ladder, its own node `ladder` under root: from the ground to the cockpit
    floor, z_ahead kit units in front of the cockpit. A game stows it when the mech moves."""
    M, u = k.M, 1.0 / k.s
    cp = k.world["cockpit"].translation
    top, z = cp.y * u, cp.z * u + z_ahead
    lad = k.J("ladder", k.root, x, 0, z)
    n = max(2, int(top * k.s / 0.4))
    for i in range(n):
        k.box(0.6 * u, 0.05 * u, 0.06 * u, M["metal"], lad, 0, (0.2 + i * 0.4) * u, 0)
    for dx in (-0.32, 0.32):
        k.box(0.06 * u, top, 0.08 * u, M["dark"], lad, dx * u, top / 2, 0)
    k.sock("LADDER_base", k.root, x, 0, z + 0.6 * u)


def heel_jet(k, g, z, r):
    """Crab Breeze: a small booster behind each heel for the rocket skate (skatePose, survey M4)."""
    M = k.M
    k.cyl(r, r * 1.2, r * 2.4, M["dark"], g, 0, -1, z, PI / 2)
    k.cyl(r * 0.8, r * 0.8, 0.2, M["glowDim"], g, 0, -1, z - r * 1.25, PI / 2)


# ----------------------------------------------------------------------------- BULWARK

def bulwark(k):
    M = k.M
    segs, footH = [[22, -0.26], [23, 0.52]], 5
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(34, 10, 20, M["frame"], body, 0, 0, 0)
    k.cyl(5.5, 5.5, 40, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pl(18, 10, 4, M["armor"], body, 0, -2.5, 11, -0.2)
    k.box(11, 2, 0.5, M["hazard"], body, 0, -6.2, 12.6, -0.2)
    k.core(body, 0, 0, 13.6, 3)
    for s in (1, -1):
        k.pl(5, 13, 16, M["armor2"], body, s * 19.6, -4, 0, 0, 0, s * 0.15)
        k.bolts(body, 3, -4, 3.4, -10.2, 4, 0.6)

    def seg0(k, g, s, L):
        k.cyl(6, 6, 10, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(9, 20, 9, M["frame"], g, 0, -11, 0)
        pl = k.pl(14, 18, 5, M["armor"], g, 0, -10, 5.5, 0.05)
        k.stripes(pl, 12.6, 0, 2.5, 2.8, 0, hb=1.7, gap=0.9)
        k.pl(5, 16, 12, M["armor2"], g, s * 7, -10, 0)
        k.vent(g, 5, 7, s * 9.6, -10, 0, 0, s * PI / 2, 3)

    def seg1(k, g, s, L):
        k.cyl(5, 5, 13, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.pl(13, 10, 6, M["armor2"], g, 0, 1, 6, -0.35)
        k.box(9, 22, 9, M["frame"], g, 0, -11.5, -1)
        gr = k.pl(14, 18, 5, M["armor"], g, 0, -12.5, 5.5, -0.05)
        k.box(11, 2.4, 0.5, M["hazard"], gr, 0, -6.5, 2.6)
        k.pl(10, 14, 6, M["armor2"], g, 0, -11, -6, 0.1)
        k.vent(g, 6, 8, 0, -11, -9.2, 0, PI, 3)

    def foot(k, g, s, S):
        k.sph(4.5, M["dark"], g, 0, 0, 0)
        k.pl(18, 4.5, 24, M["armor"], g, 0, -2.8, 3)
        toe = k.J(f"toe_{S}", g, 0, -5, 9)
        for x in (-6, 0, 6):
            k.px([[0, 0], [6, 0], [6, 1.5], [1.5, 3.8], [0, 3.8]], 5, M["armor2"], toe, x, 0, 5.5)
        k.box(18.6, 1.2, 24.6, M["rubber"], g, 0, -4.6, 3)
        k.pl(9, 4, 6, M["frame"], g, 0, -2.6, -10)
        k.box(14, 1.8, 0.5, M["hazard"], g, 0, -2, 15.3)
        heel_jet(k, g, -13.5, 1.8)  # Crab Breeze
        k.sock(f"WHEEL_{S}", g, 0, -4.6, 3)

    k.legs(body, 15, segs, [seg0, seg1], foot)
    k.piston(N("thigh_L"), [0, -3, -5.5], N("shin_L"), [0, -6, -5.5], 1.8)
    k.piston(N("thigh_R"), [0, -3, -5.5], N("shin_R"), [0, -6, -5.5], 1.8)
    waist = k.J("torso_twist", body, 0, 7, -1)
    k.cyl(10, 11.5, 6, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(3):
        k.tor(11.2, 0.8, M["frame"], waist, 0, -1.8 + i * 1.8, 0, PI / 2)
    chest = k.J("torso", waist, 0, 3.5, 0)
    k.pl(46, 24, 30, M["armor"], chest, 0, 13, 0)
    k.px([[0, 0], [5, 3], [5, 20], [0, 24]], 40, M["armor"], chest, 0, 1, 15)
    k.stripes(chest, 34, 0, 17, 20.4, 0, hb=2, gap=1)
    k.box(9, 3, 0.5, M["hazard"], chest, 12, 5, 20.3)
    k.core(chest, 0, 8, 21, 4.4)
    k.pl(50, 6, 27, M["armor2"], chest, 0, 27, -1)
    k.bolts(chest, 9, -16, 27, 12.9, 4, 0.6)
    for s in (1, -1):
        k.vent(chest, 12, 10, s * 23.3, 12, 1, 0, s * PI / 2, 5)
    jet_cradle(k, chest, -1, 26, -15, 22)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(3, 3.6, 16, M["dark"], chest, s * 19, 34, -11)
        k.cyl(3.9, 3.9, 2, M["frame"], chest, s * 19, 42, -11)
        k.cyl(2.7, 2.7, 0.4, M["glowDim"], chest, s * 19, 43.1, -11)
        k.tor(3.6, 0.4, M["frame"], chest, s * 19, 37, -11, PI / 2)
    k.hose(chest, [[-16, 4, -12], [-21, 8, -6], [-21, 14, 2]], 1.2)
    k.hose(chest, [[16, 4, -12], [21, 8, -6], [21, 14, 2]], 1.2)
    spine = k.J("spine", chest, 0, 30, -9)
    k.pl(14, 9, 12, M["frame"], spine, 0, 4, 0)
    k.box(10, 1.4, 0.5, M["hazard"], spine, 0, 1.5, 6.2)
    k.cyl(1.2, 1.2, 8, M["metal"], spine, 6, 12, 0)
    k.cyl(1.2, 1.2, 8, M["metal"], spine, -6, 12, 0)
    # Crab Breeze: the head is the cockpit. Kit's head was pl(18, 9, 16); this is a hollow shell big enough
    # to sit in, with a visor band where the kit had its glow strip.
    head = k.J("cockpit", chest, 0, 29, 9)
    W, H, D, t = 22, 14, 20, 1.4
    k.pl(W, t, D, M["armor"], head, 0, 0, 0)                        # floor
    k.pl(W, t, D, M["armor"], head, 0, H, 0)                        # roof
    for s in (1, -1):
        k.pl(t, H, D, M["armor"], head, s * (W / 2 - t / 2), H / 2, 0)
    k.pl(W, H, t, M["armor"], head, 0, H / 2, -D / 2 + t / 2)      # back
    k.pl(W, 6.6, t, M["armor"], head, 0, 3.3, D / 2 - t / 2)       # front below the visor
    k.pl(W, 2.6, t, M["armor"], head, 0, H - 1.3, D / 2 - t / 2)   # brow above it
    k.box(W - 2 * t, 4.0, 0.15, M["glass"], head, 0, 8.6, D / 2 - 0.4)
    k.pl(W + 2, 3, 8, M["red"], head, 0, H + 1.5, 3, 0.3)
    k.box(14, 1.2, 0.6, M["glow"], head, 0, 5.8, D / 2 + 0.1)
    k.halo(head, 0, 5.8, D / 2 + 1, 22, 0.6)
    for s in (1, -1):
        k.pl(3, 12, 17, M["armor2"], head, s * (W / 2 + 1.4), H / 2, 0, 0, s * 0.1, 0)
    k.lamp(head, 7, H + 1.2, 6.2, 0.8, 5)
    k.ant(head, -8, H, -6, 11, -0.15, 0.25)
    cockpit_interior(k, head, 0, 8.6, 1.5)
    k.sock("HATCH_entry", head, 0, H + 1, -4)
    ladder(k, 0, D / 2 + 6)
    # Left: the siege fist behind a slab shield.
    shL, elbL = contract_arm(k, chest, "L", 27, 20, 0, -0.08, -19, -0.5)
    k.sph(6.5, M["frame"], shL, 0, 0, 0)
    pL = k.pl(18, 12, 24, M["red"], shL, 3, 5, 0, 0, 0, 0.25)
    k.box(1.5, 10, 20, M["hazard"], pL, 9.2, 0, 0)
    k.box(8, 16, 8, M["frame"], shL, 0, -10, 0)
    k.cyl(4.8, 4.8, 10, M["dark"], elbL, 0, 0, 0, 0, 0, PI / 2)
    k.pl(13, 20, 14, M["armor2"], elbL, 0, -10, 1)
    k.core(elbL, 0, -9, 8.9, 3)
    k.box(15, 9, 15, M["frame"], elbL, 0, -23, 1)
    for i in range(4):
        k.box(3, 4.5, 4.5, M["metal"], elbL, -4.5 + i * 3, -29, 6.5)
    k.sock("MUZZLE_L", elbL, 0, -29, 8, PI / 2)
    shield = k.J("hand_L", elbL, 9, -12, 1)
    k.pl(4, 38, 26, M["armor"], shield, 0, 0, 2, 0, 0, 0, 0.9)
    k.stripes(shield, 24, 2.3, 8, 2, PI / 2, hb=1.8, gap=0.9)
    k.box(0.6, 6, 22, M["hazard"], shield, 2.3, -14, 2)
    k.box(5, 2, 28, M["frame"], shield, 0, 19.5, 2)
    k.box(5, 2, 28, M["frame"], shield, 0, -19.5, 2)
    k.box(1, 1.4, 12, M["glow"], shield, 2.6, 2, 2)
    k.halo(shield, 3.4, 2, 2, 10, 0.5)
    # Right: the breaker cannon.
    shR, elbR = contract_arm(k, chest, "R", -27, 20, 0, -0.1, -17, -1.42)
    k.sph(6.5, M["frame"], shR, 0, 0, 0)
    pR = k.pl(18, 12, 24, M["red"], shR, -3, 5, 0, 0, 0, -0.25)
    k.box(1.5, 10, 20, M["hazard"], pR, -9.2, 0, 0)
    k.box(8, 14, 8, M["frame"], shR, 0, -9, 0)
    k.cyl(5, 5, 11, M["dark"], elbR, 0, 0, 0, 0, 0, PI / 2)
    housing = k.pl(17, 26, 17, M["armor"], elbR, 0, -13, 0)
    k.stripes(housing, 15, 0, 5, 8.7, 0, hb=1.5, gap=0.8)
    k.core(elbR, 0, -13, 9.8, 3.2)
    k.vent(elbR, 7, 9, -8.8, -13, 0, 0, -PI / 2, 3)
    k.vent(elbR, 7, 9, 8.8, -13, 0, 0, PI / 2, 3)
    k.box(20, 4, 20, M["frame"], elbR, 0, -27, 0)
    for i in range(4):
        k.tor(10, 0.7, M["glowDim"], elbR, 0, -4 - i * 5, 0, PI / 2)
    barrel = k.J("hand_R", elbR, 0, -29, 0)
    k.cyl(3.8, 4.3, 32, M["metal"], barrel, 0, -16, 0, 0, 0, 0, 16)
    for y in (-5, -14, -23):
        k.cyl(5, 5, 2.2, M["frame"], barrel, 0, y, 0)
    k.box(10, 8, 10, M["dark"], barrel, 0, -34, 0)
    for yy in (-32.5, -35.5):
        for s in (1, -1):
            k.box(0.5, 1.2, 7, M["ink"], barrel, s * 5.1, yy, 0)
    k.cyl(2.6, 2.6, 0.4, M["ink"], barrel, 0, -38.1, 0)
    k.sock("MUZZLE_R", barrel, 0, -38.5, 0, PI / 2)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.9, A=0.34, lift=0.45, bob=2.4 * k.s, roll=0.035, yaw=0.05, arm=0.08, lean=0.06),
                    idle=dict(scan=0.32, scanPeriod=3.1, breathHz=0.85))


def jet_cradle(k, chest, y0, h, z, w):
    """Crab Breeze: the kit's cradle (where a pilot mech docked into the Uber mech) holds the flight jets
    here (survey M3). Same frame and clamps; two nozzles and the JET sockets inside."""
    M = k.M
    k.pl(w + 6, h + 4, 3, M["frame"], chest, 0, y0 + h / 2, z + 2.5)
    for s in (1, -1):
        k.box(3.6, h + 6, 9, M["frame"], chest, s * (w / 2 + 1.8), y0 + h / 2, z - 2)
    k.box(w + 7, 3.4, 9, M["frame"], chest, 0, y0 + h + 2, z - 2)
    k.box(w + 7, 2.6, 9, M["dark"], chest, 0, y0 - 1, z - 2)
    for S, s in (("L", 1), ("R", -1)):
        x = s * w * 0.24
        k.cyl(4.2, 5.2, h * 0.75, M["dark"], chest, x, y0 + h * 0.55, z - 3)
        k.cyl(5.4, 5.4, 1.4, M["frame"], chest, x, y0 + h * 0.18, z - 3)
        k.cyl(3.8, 3.8, 0.3, M["glow"], chest, x, y0 + h * 0.16 - 0.6, z - 3)
        k.halo(chest, x, y0 + h * 0.16 - 1.5, z - 3, 14, 0.7)
        k.sock(f"JET_{S}", chest, x, y0 + h * 0.16 - 1, z - 3, PI / 2)
        c = k.J(f"clamp_{S}", chest, s * (w / 2 + 1.8), y0 + h + 1, z - 6)
        k.box(4, 3.4, 10, M["metal"], c, 0, 0, -3)
        k.box(4, h * 0.7, 3.2, M["metal"], c, -s * 2, -h * 0.35, -7.5)
        k.box(1.2, h * 0.55, 0.6, M["hazard"], c, -s * 2, -h * 0.35, -9.2)
        k.lamp(c, -s * 2, -h * 0.68, -9.4, 0.9, 6)


# ----------------------------------------------------------------------------- RAVAGER

def ravager(k):
    M = k.M
    segs, footH = [[21, -0.55], [23, 1.05], [13, -0.62]], 4.5
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(30, 9, 22, M["frame"], body, 0, 0, 0)
    k.cyl(5, 5, 36, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pz([[-9, -4], [9, -4], [6, 5], [-6, 5]], 4, M["armor"], body, 0, -2, 12, -0.25)
    k.core(body, 0, -1, 14.4, 2.8)
    for s in (1, -1):
        k.pl(5, 12, 18, M["armor2"], body, s * 17.5, -3, -1, 0, 0, s * 0.2)

    def seg0(k, g, s, L):
        k.cyl(5.5, 5.5, 9, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(8, 20, 8, M["frame"], g, 0, -10.5, 0)
        pl = k.pl(12, 17, 5, M["armor"], g, 0, -9.5, 5, 0.04)
        k.stripes(pl, 10.8, 0, 2, 2.8, 0, hb=1.5, gap=0.8)
        k.pl(4, 15, 12, M["armor2"], g, s * 6.2, -10, 0)

    def seg1(k, g, s, L):
        k.cyl(4.6, 4.6, 11, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(7, 23, 7, M["frame"], g, 0, -11.5, 0)
        k.pl(10, 20, 4, M["armor"], g, 0, -11, -4.8, -0.04)
        k.pz([[0, 0], [3, 0], [0, 9]], 3, M["metal"], g, 0, -2, -6.5, PI)
        k.vent(g, 5, 8, s * 4.2, -12, 0, 0, s * PI / 2, 3)

    def seg2(k, g, s, L):
        k.cyl(3.6, 3.6, 9, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(6, 13, 6, M["frame"], g, 0, -6.5, 0)
        k.pl(8, 10, 3, M["armor2"], g, 0, -6.5, 3.6)
        k.box(6, 1.6, 0.4, M["hazard"], g, 0, -10, 5.2)

    def foot(k, g, s, S):
        k.sph(3.6, M["dark"], g, 0, 0, 0)
        k.pl(12, 3.6, 10, M["armor"], g, 0, -2.3, 2)
        toe_j = k.J(f"toe_{S}", g, 0, -2.4, 6)
        for a in (-0.35, 0.35):
            toe = k.g(toe_j, 0, 0, 0, 0, a, 0)
            k.pl(5, 3.4, 12, M["armor2"], toe, 0, 0, 5)
            k.pz([[0, 0], [4, 0], [0, 5]], 2.4, M["metal"], toe, 0, -1.4, 11, -PI / 2)
        k.pl(4, 3, 9, M["armor2"], g, 0, -2.3, -6.5)
        k.box(12.4, 0.9, 10.4, M["rubber"], g, 0, -4, 2)
        heel_jet(k, g, -10, 1.4)  # Crab Breeze
        k.sock(f"WHEEL_{S}", g, 0, -4, 2)

    k.legs(body, 13.5, segs, [seg0, seg1, seg2], foot)
    k.piston(N("thigh_L"), [0, -2.5, 5], N("shin_L"), [0, -6, 4], 1.5)
    k.piston(N("thigh_R"), [0, -2.5, 5], N("shin_R"), [0, -6, 4], 1.5)
    waist = k.J("torso_twist", body, 0, 6.5, -1)
    k.cyl(9, 10.5, 5.5, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(2):
        k.tor(10.2, 0.8, M["frame"], waist, 0, -1 + i * 2, 0, PI / 2)
    chest = k.J("torso", waist, 0, 3, 2, 0.22)
    k.pl(40, 20, 28, M["armor"], chest, 0, 11, 0)
    k.pz([[-21, 0], [21, 0], [14, 14], [-14, 14]], 10, M["armor"], chest, 0, 18, 9)
    k.px([[0, 0], [7, 4], [7, 16], [0, 20]], 34, M["armor2"], chest, 0, 1, 14)
    k.stripes(chest, 30, 0, 14, 21.3, 0, hb=1.8, gap=0.9)
    k.box(8, 2.6, 0.5, M["hazard"], chest, -10, 5, 21.2)
    k.core(chest, 0, 6, 21.6, 4.2)
    for s in (1, -1):
        k.vent(chest, 11, 9, s * 20.2, 10, 0, 0, s * PI / 2, 4)
    jet_cradle(k, chest, 0, 24, -14, 20)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(2.6, 3.4, 13, M["dark"], chest, s * 15, 26, -12, -0.4)
        k.cyl(3.6, 3.6, 1.8, M["frame"], chest, s * 15, 32, -14.6, -0.4)
    crest = k.J("crest", chest, 0, 24, -6)
    for i in range(5):
        k.px([[0, 0], [6, 0], [2, 9 + 4 * (i % 2)], [0, 8]], 2.6, M["armor2"] if i % 2 else M["red"], crest, 0, -1 + i * 0.3, 8 - i * 5.5)
    k.hose(chest, [[-14, 3, -12], [-19, 6, -5], [-20, 12, 3]], 1.1)
    k.hose(chest, [[14, 3, -12], [19, 6, -5], [20, 12, 3]], 1.1)
    # Crab Breeze: the low head is the cockpit, a hollow wedge; the three eyes ride above the visor.
    head = k.J("cockpit", chest, 0, 18, 15, -0.22)
    W, H, D, t = 22, 13, 18, 1.3
    k.pz([[-W / 2, 0], [W / 2, 0], [W / 2 - 3, t], [-W / 2 + 3, t]], D, M["armor"], head, 0, 0, 2)  # floor
    k.pz([[-W / 2 + 3, H - t], [W / 2 - 3, H - t], [W / 2 - 5, H], [-W / 2 + 5, H]], D, M["armor"], head, 0, 0, 2)
    for s in (1, -1):
        k.pl(t, H, D, M["armor"], head, s * (W / 2 - 2), H / 2, 2, 0, 0, s * 0.18)
    k.pl(W, H, t, M["frame"], head, 0, H / 2, 2 - D / 2 + t / 2)
    k.pl(W - 2, 5.8, t, M["armor"], head, 0, 2.9, 2 + D / 2 - t / 2)
    k.pl(W - 6, 2.4, t, M["armor"], head, 0, H - 1.2, 2 + D / 2 - t / 2)
    k.box(W - 4, 4.2, 0.15, M["glass"], head, 0, 7.9, 2 + D / 2 - 0.4)
    k.pl(17, 2.6, 11, M["red"], head, 0, H + 0.6, 3, 0.2)
    for x, y in ((-3.6, H + 2.4), (3.6, H + 2.4), (0, H + 3.4)):
        k.sph(1.4, M["lens"], head, x, y, 6.5)
        k.halo(head, x, y, 7.5, 7, 0.7)
    for i in range(5):
        k.box(0.6, 2, 0.5, M["ink"], head, -3 + i * 1.5, 1.5, 2 + D / 2 + 0.3)
    k.ant(head, 6, H, -5, 8, -0.3, -0.2)
    cockpit_interior(k, head, 0, 8.0, 2.5)
    k.sock("HATCH_entry", head, 0, H + 1, -4)
    ladder(k, 0, 2 + D / 2 + 5)
    # Both arms: a twin rail lance, coils down the rails, a reactor core in each housing.
    for S, s in (("L", 1), ("R", -1)):
        sh, elb = contract_arm(k, chest, S, s * 25, 17, 1, -0.12, -16, -1.45)
        k.sph(6, M["frame"], sh, 0, 0, 0)
        p = k.pl(16, 11, 22, M["armor"], sh, s * 2.5, 4.5, 0, 0, 0, s * 0.3)
        k.stripes(p, 20, s * 8.25, -0.5, 0, s * PI / 2, hb=1.4, gap=0.7)
        for i in range(3):
            k.cyl(0, 1.5, 5, M["metal"], p, s * -1, 7, -6 + i * 6)
        k.box(7.5, 14, 7.5, M["frame"], sh, 0, -9, 0)
        k.cyl(4.6, 4.6, 10, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        hs = k.pl(14, 22, 13, M["armor2"], elb, 0, -11, 0)
        k.box(10, 1.6, 0.5, M["hazard"], hs, 0, -8, 6.6)
        k.core(elb, 0, -11, 7.8, 2.8)
        k.box(15, 3.5, 14, M["frame"], elb, 0, -23, 0)
        bar = k.J(f"hand_{S}", elb, 0, -24, 0)
        for x in (-2.6, 2.6):
            k.box(2.4, 30, 3.2, M["metal"], bar, x, -15, 0)
            for i in range(6):
                k.box(0.7, 1.2, 4.6, M["glowDim"], bar, x, -4 - i * 4.6, 0)
        k.box(9, 3, 5, M["frame"], bar, 0, -6, 0)
        k.box(9, 3, 5, M["frame"], bar, 0, -22, 0)
        k.box(8, 2.5, 4.6, M["dark"], bar, 0, -30.5, 0)
        k.sock(f"MUZZLE_{S}", bar, 0, -32, 0, PI / 2)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.8, A=0.4, lift=0.55, bob=2.2 * k.s, roll=0.04, yaw=0.07, arm=0.1, lean=0.12, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.7, scanPeriod=1.6, breathHz=1.1))


# ----------------------------------------------------------------------------- BREEZE

def breeze(k):
    """Crab Breeze's own: reverse knees like the Ravager, slimmer; a bubble canopy in the chest (no head),
    swept jet nacelles; a rail lance on the left, a gatling on the right. Pale plate, blue livery."""
    M = k.M
    segs, footH = [[20, -0.5], [22, 1.0], [12, -0.6]], 4
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(26, 8, 18, M["frame"], body, 0, 0, 0)
    k.cyl(4.4, 4.4, 30, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pz([[-8, -4], [8, -4], [5, 4], [-5, 4]], 4, M["armor"], body, 0, -2, 10.5, -0.25)
    k.box(9, 1.6, 0.5, M["hazard"], body, 0, -5.4, 12.2, -0.25)
    for s in (1, -1):
        k.pl(4, 10, 15, M["armor2"], body, s * 15, -3, -1, 0, 0, s * 0.18)

    def seg0(k, g, s, L):
        k.cyl(5, 5, 8, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(7, 19, 7, M["frame"], g, 0, -10, 0)
        pl = k.pl(11, 16, 4.5, M["armor"], g, 0, -9, 4.4, 0.04)
        k.stripes(pl, 10, 0, 2, 2.5, 0, hb=1.3, gap=0.7)
        k.pl(3.5, 14, 10, M["armor2"], g, s * 5.4, -9.5, 0)
        k.vent(g, 4, 6, s * 7.4, -9, 0, 0, s * PI / 2, 3)

    def seg1(k, g, s, L):
        k.cyl(4.2, 4.2, 10, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(6, 22, 6, M["frame"], g, 0, -11, 0)
        k.pl(9, 19, 3.5, M["armor"], g, 0, -10.5, -4.2, -0.04)
        k.pz([[0, 0], [3, 0], [0, 8]], 2.6, M["metal"], g, 0, -2, -5.8, PI)
        k.box(7, 1.6, 0.4, M["hazard"], g, 0, -17, -6.1)

    def seg2(k, g, s, L):
        k.cyl(3.2, 3.2, 8, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(5, 12, 5, M["frame"], g, 0, -6, 0)
        k.pl(7, 9, 3, M["armor2"], g, 0, -6, 3.2)

    def foot(k, g, s, S):
        k.sph(3.2, M["dark"], g, 0, 0, 0)
        k.pl(11, 3.2, 9, M["armor"], g, 0, -2, 2)
        toe_j = k.J(f"toe_{S}", g, 0, -2.2, 5.5)
        for a in (-0.32, 0.32):
            toe = k.g(toe_j, 0, 0, 0, 0, a, 0)
            k.pl(4.4, 3, 11, M["armor2"], toe, 0, 0, 4.5)
            k.pz([[0, 0], [3.6, 0], [0, 4.5]], 2.2, M["metal"], toe, 0, -1.3, 10, -PI / 2)
        k.pl(4, 3, 8, M["armor2"], g, 0, -2.1, -6)
        k.box(11.4, 0.9, 9.4, M["rubber"], g, 0, -3.6, 2)
        # the skate wheel set in the sole, and the heel booster
        k.cyl(2.2, 2.2, 3, M["rubber"], g, 0, -3.3, 2, 0, 0, PI / 2, 20)
        k.cyl(1.0, 1.0, 3.2, M["metal"], g, 0, -3.3, 2, 0, 0, PI / 2, 12)
        heel_jet(k, g, -9.5, 1.3)
        k.sock(f"WHEEL_{S}", g, 0, -3.3, 2)

    k.legs(body, 11.5, segs, [seg0, seg1, seg2], foot)
    k.piston(N("thigh_L"), [0, -2.5, 4.5], N("shin_L"), [0, -5.5, 3.5], 1.3)
    k.piston(N("thigh_R"), [0, -2.5, 4.5], N("shin_R"), [0, -5.5, 3.5], 1.3)
    waist = k.J("torso_twist", body, 0, 6, -1)
    k.cyl(8, 9.5, 5, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(2):
        k.tor(9.2, 0.7, M["frame"], waist, 0, -1 + i * 2, 0, PI / 2)
    chest = k.J("torso", waist, 0, 3, 0, 0.08)
    # the chest: a deep wedge, open at the front for the canopy
    k.pl(34, 18, 20, M["armor"], chest, 0, 11, -4)
    k.pz([[-17, 0], [17, 0], [11, 12], [-11, 12]], 8, M["armor"], chest, 0, 17, -4)
    for s in (1, -1):
        k.pl(9, 18, 14, M["armor"], chest, s * 12.5, 11, 10, 0, s * 0.15, 0)
        k.vent(chest, 9, 8, s * 17.3, 10, -4, 0, s * PI / 2, 4)
    k.stripes(chest, 26, 0, 22.5, 6.2, 0, hb=1.5, gap=0.8)
    k.core(chest, 0, 6, -14.6, 3.6)
    # the cockpit: a sealed pod at the front of the chest (owner, 2026-10-24: "The cockpit should be
    # enclosed fully"). The pod is its own glb (tools/blender/build_cockpit.py, its hatch nodes
    # animated for boarding); this joint and its sockets are where it mounts.
    cp = k.J("cockpit", chest, 0, 6, 9)
    u = 1.0 / k.s if k.s else 1.0
    k.box(16, 12, 4, M["frame"], cp, 0, 4, -8)          # the mounting cradle against the chest
    for x in (-6, 6):
        k.cyl(1.2, 1.2, 10, M["dark"], cp, x, 4, -6, PI / 2)
    k.sock("EYE_pilot", cp, 0, 6.2, 1.8)
    k.sock("SEAT_pilot", cp, 0, 6.2 - 0.78 * u, 1.8 - 0.12 * u)
    k.sock("HATCH_entry", cp, 0, 6.2 - 0.4 * u, 1.8 + 1.2 * u)
    k.sock("LADDER_base", k.root, 0, 0, 30)
    # jet nacelles on the back, swept fins over them
    for S, s in (("L", 1), ("R", -1)):
        x = s * 9
        k.cyl(4.0, 4.8, 22, M["armor"], chest, x, 15, -18)
        k.cyl(5.0, 5.0, 1.4, M["frame"], chest, x, 4.4, -18)
        k.cyl(3.6, 3.6, 0.3, M["glow"], chest, x, 3.6, -18)
        k.halo(chest, x, 2.6, -18, 14, 0.7)
        k.stripes(k.g(chest, x, 18, -18), 7, 0, 0, -4.6, PI, hb=1.1, gap=0.6)
        k.px([[0, 0], [10, 0], [16, 6], [10, 7]], 1.6, M["red"], chest, x + s * 4, 22, -20, 0, 0, s * 0.35)
        k.sock(f"JET_{S}", chest, x, 3.4, -18, PI / 2)
    k.hose(chest, [[-12, 4, -12], [-16, 7, -6], [-17, 12, 2]], 0.9)
    k.hose(chest, [[12, 4, -12], [16, 7, -6], [17, 12, 2]], 0.9)
    k.ant(chest, 10, 23, -8, 9, -0.25, -0.15)
    k.lamp(chest, -12, 21, 4, 0.7, 5)
    # Left: a rail lance. Right: a six barrel gatling.
    for S, s in (("L", 1), ("R", -1)):
        sh, elb = contract_arm(k, chest, S, s * 21, 17, -2, -0.1, -14, -1.45)
        k.sph(5, M["frame"], sh, 0, 0, 0)
        p = k.pl(14, 9, 18, M["red"], sh, s * 2.2, 4, 0, 0, 0, s * 0.3)
        k.box(1.2, 7, 15, M["hazard"], p, s * 7.2, 0, 0)
        k.box(6.5, 12, 6.5, M["frame"], sh, 0, -8, 0)
        k.cyl(4, 4, 9, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        hs = k.pl(12, 19, 11, M["armor"], elb, 0, -10, 0)
        k.box(9, 1.4, 0.5, M["hazard"], hs, 0, -7, 5.6)
        k.box(13, 3, 12, M["frame"], elb, 0, -20.5, 0)
        hand = k.J(f"hand_{S}", elb, 0, -21, 0)
        if s > 0:
            for x in (-2.2, 2.2):
                k.box(2, 28, 2.8, M["metal"], hand, x, -14, 0)
                for i in range(6):
                    k.box(0.6, 1.0, 4, M["glowDim"], hand, x, -4 - i * 4.2, 0)
            k.box(8, 2.6, 4.4, M["frame"], hand, 0, -6, 0)
            k.box(8, 2.6, 4.4, M["frame"], hand, 0, -20, 0)
            k.sock("MUZZLE_L", hand, 0, -29, 0, PI / 2)
        else:
            k.cyl(4.2, 4.2, 9, M["dark"], hand, 0, -4.5, 0, 0, 0, 0, 20)
            for i in range(6):
                a = i / 6 * 2 * PI
                k.cyl(0.75, 0.75, 22, M["metal"], hand, math.cos(a) * 2.1, -14, math.sin(a) * 2.1, 0, 0, 0, 10)
            for y in (-12, -21):
                k.cyl(3.2, 3.2, 1.1, M["frame"], hand, 0, y, 0, 0, 0, 0, 20)
            k.sock("MUZZLE_R", hand, 0, -25, 0, PI / 2)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.95, A=0.4, lift=0.55, bob=2.0 * k.s, roll=0.04, yaw=0.06, arm=0.1, lean=0.1, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.4, scanPeriod=2.2, breathHz=1.2))


# ----------------------------------------------------------------------------- the chest cockpits
#
# The owner, 2026-10-24: "Breeze me h sucks", "Follow design philosophy of boss mechs from mining mike. The
# cockpit is part of the mech, inside of it with the armor opening up at the chest", then, of a first pass
# that reused the bosses' limbs and weapons: "New mechs are just stolen. Those are not original mechs".
#
# So these three take the boss kit's finish (bevelled plates over a frame, livery bands, hazard edges,
# vents, hoses, rams, glowing cores) and nothing else from the bosses: their shapes come from Crab Breeze's
# own setting, an alternate 1980s of industrial machines (survey G1), on digitigrade legs (survey M2).
# Each carries one cockpit interior (build_cockpit.py --variant) in a bay in its chest, closed by armour
# that opens to board (mech_boarding.py).
#
#   foreman    the excavator: hydraulic rams everywhere, track-pad feet, a slewing ring, a cab with a roll
#              cage; a hydraulic breaker and a belt-fed boom cannon. The Panorama clamshell. (panorama)
#   ironhorse  the locomotive: a banded boiler torso whose smokebox front drops as a ramp, drive-wheel knees
#              with coupling rods, cow-catcher shins, a smokestack, a steam cannon and a buffer ram. (cab)


def bearing_x(k, hull, x, y):
    """The bearing (radians) at which the chest's outer face at height y (metres) stands x metres to the side."""
    best = min(range(158), key=lambda i: abs(hull["at"](i * 0.01, y)[0] * k.s - x))
    return best * 0.01


def chest_bay(k, chest, variant, at, mode, torso, swing=None, wrap_z=None, timing=None):
    """The cockpit and its lofted chest (mech_boarding.pod): `at` is the interior's floor centre in the chest
    joint (kit units), `mode` how the front opens (lift, doors, side, inner). Adds the mist that rolls out over the
    sill. Returns cp, cav, out, hull as MB.pod does."""
    cp, cav, out, hull = MB.pod(k, chest, variant, at, mode, torso, swing=swing, wrap_z=wrap_z, timing=timing)
    chest_markings(k, cp, hull)
    u = 1.0 / k.s
    # no chain ladder (owner, 2026-10-24: "They shouldnt have a ... ladder"): the mech sits, the step drops
    MB.vent(k, cp, 0, cav["y0"] + 0.3 * u, cav["z1"] - 0.2 * u, (0, -0.25, 1), "sill", "mist", 1.4)
    for s in (1, -1):
        MB.vent(k, cp, s * cav["hw"] * 0.6, cav["y0"] + 0.2 * u, cav["z1"] - 0.3 * u, (s * 0.3, -0.2, 1), "sill", "mist", 1.0)
    return cp, cav, out, hull


def _mk(k, kind):
    """The decal variant that reads on this mech: dark markings on a light scheme, white on a dark one."""
    return f"{kind}_{'d' if k.mark['dark'] else 'w'}"


def unit_number(k, p, x, y, z, ry, size):
    """The unit number, two stencil digits side by side on a face turned by ry (0 faces ahead)."""
    pre = "dd" if k.mark["dark"] else "dw"
    g = k.g(p, x, y, z, 0, ry, 0)
    for i, ch in enumerate(k.mark["number"]):
        k.decal(f"{pre}{ch}", g, (i - 0.5) * size * 0.78, 0, i * 0.04, width=size)    # the second a hair proud: no shared plane


def chest_markings(k, cp, hull):
    """A mech's markings on its chest, as a kit builder lays them from a decal sheet (owner, 2026-10-24: "instead of
    \"danger\" plastered everywhere it should say \"no step\", company logo, \"warning flamable\" etc", with decal sheets and
    finished kits for reference): the unit number on both flanks, the maker's logo across the back, a data plate and
    edge lines by the hatch, EJECT by the cockpit, WARNING FLAMMABLE and KEEP CLEAR over the jets. Each sits 0.06 m off
    the armour."""
    if not getattr(k, "mark", None):
        return
    U = 1.0 / k.s
    Yb, Yt = hull["Yb"], hull["Yt"]
    ym = (Yb + Yt) / 2

    u = 1.0 / k.s
    taken = []            # (bearing from, to, height from, to) of the markings laid so far

    def put(a, y, w):
        """The node, point and bearing for a marking w metres wide at bearing a and height y (metres). It goes on the
        armour piece that holds it, so it rides that piece when the chest opens rather than hanging where the armour
        was, and it slides back along the hull until all of it is on one piece: one straddling a seam has an end over
        the doorway once the door swings."""
        piece = hull["piece_of"]
        sg = 1 if math.sin(a) >= 0 else -1
        for i in range(40):
            b = a + sg * 0.05 * i
            x, _, z = hull["at"](b, y)
            h = (w / 2) / max(0.3, math.hypot(x / u, z / u - hull["z_cut"]))
            ends = {piece(b + d, y + e) for d in (-h, 0, h) for e in (-w * 0.35, w * 0.35)}
            box = (b - h, b + h, y - w * 0.35, y + w * 0.35)
            clear = all(box[1] < t[0] or box[0] > t[1] or box[3] < t[2] or box[2] > t[3] for t in taken)
            if len(ends) == 1 and clear:          # all on one piece, and over no other marking
                taken.append(box)
                return (*MB.on(k, hull, ends.pop(), hull["at"](b, y, 0.06)), b)
        return (*MB.on(k, hull, piece(a, y), hull["at"](a, y, 0.06 + 0.02 * len(taken))), a)

    for s in (1, -1):
        n, q, b = put(s * 1.0, ym + 0.3, 1.4 * 1.8)          # two digits 0.78 of a digit apart
        unit_number(k, n, *q, b, 1.4 * U)
        for kind, a, y, w in ((_mk(k, "lines"), s * 1.25, Yt + 0.05, 3.0), ("marks", s * 1.8, Yb + 0.3, 1.0)):
            n, q, b = put(a, y, w)
            k.decal(kind, n, *q, 0, b, 0, w * U)
    for kind, a, y, w in ((_mk(k, "logo"), PI, Yt - 0.1, 4.0), (_mk(k, "data"), 1.2, Yb + 0.35, 2.0),
                          ("eject", -1.2, Yt - 0.25, 0.9), ("flammable", PI - 0.55, Yb + 0.3, 1.6),
                          ("keepclear", PI + 0.55, Yb + 0.3, 1.6)):
        n, q, b = put(a, y, w)
        k.decal(kind, n, *q, 0, b, 0, w * U)


def chamfered(w, zf, zb, c=0.25):
    """A plan section (metres): a rectangle w half wide from zb to zf with its corners cut by c."""
    return [(w - c, zf), (w, zf - c), (w, zb + c), (w - c, zb), (-w + c, zb), (-w, zb + c), (-w, zf - c), (-w + c, zf)]


def rounded(w, zf, zb, n=10):
    """A plan section (metres): an ellipse w half wide from zb to zf."""
    zm, rz = (zf + zb) / 2, (zf - zb) / 2
    return [(w * math.sin(2 * PI * i / n), zm + rz * math.cos(2 * PI * i / n)) for i in range(n)]


def pack(k, node, keys_m, mat, cz):
    """A lofted pack on node through plan sections [(y, section)] in metres (the cockpit's frame)."""
    u = 1.0 / k.s
    MB.solid(k, node, [(y * u, [(x * u, z * u) for x, z in p]) for y, p in keys_m], mat, (0.0, cz * u))


# The torsos (mech_boarding.section, metres from the interior's floor, the interior's frame): broad shoulders
# behind the cut, a waist that sits in the slewing ring or the hip drum, a collar for the head, and ahead
# of the cut only the cockpit's prow (owner, 2026-10-24: "I would want a mech more mech shape, this looks
# like 2 boxes and trashcan. the limbs look cool though").
FOREMAN_TORSO = [
    (-0.80, 1.25, 0.90, -0.70, dict(wcut=0.9, sh=0.05, cb=0.30)),
    (-0.45, 1.60, 1.00, -0.95, dict(wcut=1.0, sh=0.25, cb=0.35)),
    (0.30, 2.00, 1.00, -1.10, dict(wcut=1.0, sh=0.40, cb=0.40)),
    (1.30, 2.35, 0.90, -1.20, dict(wcut=1.0, sh=0.50, cb=0.45)),
    (2.05, 2.35, 0.70, -1.20, dict(wcut=0.9, sh=0.50, cb=0.45)),
    (2.45, 1.90, 0.50, -1.05, dict(wcut=0.7, sh=0.40, cb=0.40)),
    (2.75, 1.00, 0.38, -0.80, dict(wcut=0.5, sh=0.20, cb=0.30)),
]
def doubled(torso, k=2.0):
    """A torso of the first round at 24 m: its height, width, back and shoulder flare scaled round the true-size cockpit,
    its prow (zf, wcut) kept, as the cockpit inside it does not grow."""
    return [(y * k, w * k, zf, zb * k, dict(kw, sh=kw.get("sh", 0.3) * k, **({"cb": kw["cb"] * k} if "cb" in kw else {})))
            for y, w, zf, zb, kw in torso]


IRONHORSE_TORSO = [
    (-0.60, 1.20, 1.00, -0.90, dict(wcut=0.9, sh=0.05, round_back=4)),
    (-0.25, 1.60, 1.10, -1.30, dict(wcut=0.95, sh=0.25, round_back=5)),
    (0.60, 1.95, 1.00, -1.60, dict(wcut=0.95, sh=0.40, round_back=6)),
    (1.40, 2.10, 0.90, -1.70, dict(wcut=0.95, sh=0.45, round_back=6)),
    (2.00, 1.80, 0.75, -1.60, dict(wcut=0.8, sh=0.40, round_back=6)),
    (2.40, 1.10, 0.72, -1.20, dict(wcut=0.6, sh=0.30, round_back=5)),
]


def ram_pair(k, g, a, b, r, rod_mat=None):
    """A static hydraulic cylinder from a to b on one node (the excavator's boom rams): barrel, chrome rod,
    clevis ends. For rams that must follow a moving part, use mech_boarding.ram."""
    M = k.M
    A, B = Vector(a), Vector(b)
    d = B - A
    L = d.length
    e = Vector((0, 1, 0)).rotation_difference(d.normalized()).to_euler("ZYX")
    bl = L * 0.6
    k.cyl(r, r, bl, M["dark"], g, *(A + d.normalized() * bl / 2), e.x, e.y, e.z, seg=12)
    k.cyl(r * 1.18, r * 1.18, bl * 0.1, M["frame"], g, *(A + d.normalized() * bl * 0.93), e.x, e.y, e.z, seg=12)   # short of the barrel's end: no shared cap
    k.cyl(r * 0.55, r * 0.55, L * 0.45, rod_mat or M["metal"], g, *(A + d.normalized() * L * 0.77), e.x, e.y, e.z, seg=10)
    for p in (A, B):
        k.box(r * 2.2, r * 2.2, r * 1.6, M["frame"], g, *p)


def lattice(k, g, w, d, L, mat, n=4, y0=0.0):
    """A truss girder hanging down from g's origin by L: four corner chords, rungs and X bracing on the
    front and sides. w x d is its section."""
    M = k.M
    for sx in (1, -1):
        for sz in (1, -1):
            k.box(1.1, L, 1.1, mat, g, sx * w / 2, y0 - L / 2, sz * d / 2)
    step = L / n
    for i in range(n + 1):
        y = y0 - i * step
        k.box(w, 0.9, 0.9, M["frame"], g, 0, y, d / 2)
        k.box(w, 0.9, 0.9, M["frame"], g, 0, y, -d / 2)
        for sx in (1, -1):
            k.box(0.76, 0.84, d - 0.2, M["frame"], g, sx * w / 2, y, 0)      # inside the front and back rungs: no shared faces
    diag_f = math.atan2(w, step)
    diag_s = math.atan2(d, step)
    for i in range(n):
        y = y0 - (i + 0.5) * step
        lf, ls = math.hypot(w, step), math.hypot(d, step)
        sg = 1 if i % 2 else -1
        k.box(0.6, lf, 0.6, mat, g, 0, y, d / 2 + 0.2, 0, 0, sg * diag_f)     # 0.05 clear of every rung and chord face
        k.box(0.6, lf, 0.6, mat, g, 0, y, -d / 2 - 0.2, 0, 0, -sg * diag_f)
        for sx in (1, -1):
            k.box(0.6, ls, 0.6, mat, g, sx * (w / 2 + 0.15), y, 0, sg * diag_s, 0, 0)   # clear of the side rungs and a joint drum's cap


def wheel(k, g, x, y, z, r, w, spokes=8):
    """A locomotive drive wheel facing sideways (axis x): tyre, rim, spokes, hub and a counterweight."""
    M = k.M
    k.cyl(r, r, w, M["dark"], g, x, y, z, 0, 0, PI / 2, 28)
    k.tor(r * 0.86, r * 0.07, M["frame"], g, x + w / 2 + 0.1, y, z, 0, PI / 2, 0)
    for i in range(spokes):
        a = i / spokes * 2 * PI
        # from the hub out, so neighbouring spokes never meet (they share their side planes)
        k.box(w * 0.5, r * 0.6, r * 0.12, M["frame"], g, x + w / 2 + 0.2, y + math.cos(a) * r * 0.55, z + math.sin(a) * r * 0.55, a, 0, 0)
    k.cyl(r * 0.22, r * 0.22, w + 1.2, M["metal"], g, x, y, z, 0, 0, PI / 2, 14)
    k.pz([[-r * 0.5, 0], [r * 0.5, 0], [r * 0.3, r * 0.42], [-r * 0.3, r * 0.42]], 0.8, M["armor2"], g, x + w / 2 + 0.4, y - r * 0.82, z, 0, PI / 2, 0)


def beacon(k, p, x, y, z, r=0.9):
    """An amber rotating beacon on a short plinth (the works vehicle's)."""
    M = k.M
    k.cyl(r * 1.2, r * 1.3, 0.8, M["frame"], p, x, y, z)
    k.cyl(r, r * 0.9, r * 1.6, M["glowDim"], p, x, y + 0.8 + r * 0.8, z)
    k.sph(r * 0.7, M["glow"], p, x, y + 1.6 + r * 1.4, z)
    k.halo(p, x, y + 0.8 + r * 0.8, z, r * 7, 0.75, 0xffa020)


def floodlight(k, p, x, y, z, rx=0.0, ry=0.0, r=1.1):
    M = k.M
    g = k.g(p, x, y, z, rx, ry, 0)
    k.box(r * 2.4, r * 1.8, r * 1.4, M["dark"], g, 0, 0, 0)
    k.box(r * 2.0, r * 1.4, 0.3, M["glow"], g, 0, 0, r * 0.75)
    k.halo(g, 0, 0, r, r * 6, 0.6)
    k.box(0.6, r * 1.2, 0.6, M["frame"], g, 0, -r * 1.4, 0)


# ----------------------------------------------------------------------------- FOREMAN

def foreman(k):
    """The excavator. Heavy and low on digitigrade legs driven by exposed double rams, standing on track-pad
    feet; a slewing ring for a waist; the chest is the operator's cab inside a roll cage, beacons and work
    lights on it, the engine bay and a counterweight behind. Right: a hydraulic breaker on a boom. Left: a
    belt-fed cannon on an articulated boom, its ammunition drum on the back. Boarding: the visor lifts
    on two rams, the apron under it drops forward into a step. Cockpit: Panorama, opening as one body with the chest."""
    M = k.M
    segs, footH = [[17, -0.62], [19, 1.15], [10, -0.53]], 4.6
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    # the axle housing: a wide drum with a differential bulge, and the hip motors
    k.cyl(5.4, 5.4, 34, M["dark"], body, 0, 0, 0, 0, 0, PI / 2, 20)
    k.sph(7.2, M["armor2"], body, 0, -0.5, 0, 1, 0.8, 0.9)
    for s in (1, -1):
        k.cyl(6.4, 6.4, 3, M["frame"], body, s * 11, 0, 0, 0, 0, PI / 2, 20)
        k.bolts(body, 6, -5, 6.2, 0, 2, 0.5)
    k.pl(16, 6, 3, M["armor"], body, 0, -3, 7.5, -0.3)
    k.box(13, 1.4, 0.5, M["hazard"], body, 0, -5.4, 8.6, -0.3)

    def seg0(k, g, s, L):            # thigh: a box girder, a double ram up its front
        k.cyl(4.8, 4.8, 7.6, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)            # short of the axle's end: no shared cap
        k.box(6, L, 6, M["frame"], g, 0, -L / 2, -1)
        k.pl(4, L * 0.8, 9, M["armor"], g, s * 4.4, -L * 0.48, -1)
        k.stripes(k.g(g, s * 6.5, -L * 0.48, -1, 0, PI / 2, 0), 6, 0, 0, 0, 0, hb=1.0, gap=0.5)
        for x in (-1.6, 1.6):
            ram_pair(k, g, (x, -2, 4.6), (x, -L + 2.5, 3.4), 1.05)
        k.hose(g, [[s * 3, -1, -4.5], [s * 4, -L * 0.5, -5], [s * 3, -L + 1, -4]], 0.6)

    def seg1(k, g, s, L):            # shin: a deep box with a mud guard, ribs and its own ram behind
        k.cyl(4.2, 4.2, 9, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.pl(8, L * 0.85, 7, M["armor"], g, 0, -L * 0.47, 0)
        for i in range(4):
            k.box(8.6, 0.8, 7.6, M["frame"], g, 0, -3 - i * L * 0.2, 0)
        k.pz([[-5, 0], [5, 0], [4, 5], [-4, 5]], 2, M["armor2"], g, 0, -L * 0.25, 4.6, -PI / 2)
        k.box(7, 1.2, 0.4, M["hazard"], g, 0, -L * 0.82, 3.8)
        ram_pair(k, g, (0, -2, -4.6), (0, -L + 2, -4.2), 0.9)

    def seg2(k, g, s, L):            # hock: a coil-over damper
        k.cyl(3.2, 3.2, 7, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(4.5, L, 4.5, M["frame"], g, 0, -L / 2, 0)
        for i in range(6):
            k.tor(2.6, 0.35, M["metal"], g, 0, -1.5 - i * (L - 3) / 6, 2.9, PI / 2)

    def foot(k, g, s, S):            # a track pad: idlers fore and aft, cleats round it
        k.sph(3, M["dark"], g, 0, 0, 0)
        k.box(9, 3.4, 17, M["frame"], g, 0, -2.2, 2)
        for z in (-6.5, 10.5):
            k.cyl(2.2, 2.2, 9.6, M["dark"], g, 0, -2.6, z, 0, 0, PI / 2, 18)
        for i in range(9):
            z = -6.5 + i * 2.125
            k.box(9.8, 0.6, 1.2, M["rubber"], g, 0, -4.6, z)
            k.box(9.8, 0.6, 1.2, M["rubber"], g, 0, -0.6, z)
        k.pl(10, 1.6, 12, M["armor"], g, 0, 0.6, 2)
        toe = k.J(f"toe_{S}", g, 0, -3, 12.5)
        k.pz([[-4.5, 0], [4.5, 0], [3, 3], [-3, 3]], 2.4, M["armor2"], toe, 0, 0, 0.6, -PI / 2)
        heel_jet(k, g, -9, 1.3)
        k.sock(f"WHEEL_{S}", g, 0, -4.6, 2)

    k.legs(body, 13, segs, [seg0, seg1, seg2], foot)
    waist = k.J("torso_twist", body, 0, 6.4, -1)
    # the slewing ring: a toothed ring under the cab
    k.cyl(11, 11, 3, M["dark"], waist, 0, 0, 0, 0, 0, 0, 32)
    k.tor(11.2, 0.9, M["frame"], waist, 0, 1.4, 0, PI / 2, seg=32)
    for i in range(32):
        a = i / 32 * 2 * PI
        k.box(0.8, 1.6, 1.0, M["metal"], waist, math.cos(a) * 11.9, -0.4, math.sin(a) * 11.9, 0, -a, 0)
    chest = k.J("torso", waist, 0, 2.5, 0)
    # in the mech the upper half swings on past level, so the dash it carries clears the doorway instead of hanging
    # at chest height in front of it, and the step lies flat (owner, 2026-10-24: "pieces of geometry that block the
    # pilot from exiting")
    cp, cav, out, hull = chest_bay(k, chest, "panorama", (0, 5, 3), "inner", doubled(FOREMAN_TORSO),
                                   swing={"INNER_upper_1": -2.75, "INNER_lower_1": 2.7})
    hw, y0, y1, z1 = cav["hw"], cav["y0"], cav["y1"], out["z1"]
    U = 1.0 / k.s
    Yt = hull["Yt"]
    # the shoulders: a hazard band round each, an amber beacon on top, work lights on the hood
    for S, s in (("L", 1), ("R", -1)):
        MB.contour(k, hull, None, s * (PI / 2 + 0.08), s * (PI - 0.35), 1.75, 0.03, 0.06, M["hazard"])
        MB.contour(k, hull, None, s * (PI / 2 + 0.08), s * (PI - 0.35), 0.45, 0.03, 0.05, M["frame"])
        beacon(k, cp, *hull["at"](s * (PI / 2 + 0.75), 2.45, 0.02))
        floodlight(k, cp, *hull["at"](s * 0.55, Yt + 0.30, 0.12), -0.25, s * 0.15)
    MB.contour(k, hull, None, -1.3, 1.3, Yt + 0.22, 0.02, 0.05, M["hazard"])     # the hood's edge over the visor
    # the head on the collar: a low armoured block, two lenses and a rangefinder bar
    top = doubled(FOREMAN_TORSO)[-1]
    mast = k.J("head", chest, 0, 5 + (top[0] - 0.05) * U, 3 + (top[2] + top[3]) / 2 * U)
    k.cyl(2.6, 3.2, 5.0, M["frame"], mast, 0, -0.7, 0)     # the neck, sunk into the hood
    k.pz([[-3.6, 0], [3.6, 0], [3.0, 3.4], [-3.0, 3.4]], 5, M["armor"], mast, 0, 1.4, 0, 0, PI / 2, 0)
    for x in (-1.8, 1.8):
        k.cyl(1.0, 1.0, 1.2, M["dark"], mast, x, 3.0, 2.4, PI / 2)
        k.sph(0.75, M["lens"], mast, x, 3.0, 3.0)
    k.box(4.6, 0.6, 0.4, M["glow"], mast, 0, 4.2, 2.45)      # narrower than the block: its ends not on its sides
    k.sock("SENSOR_head", mast, 0, 3, 3.2)
    k.sock("CAM_main", mast, 0, 3, 3.2)
    # behind: the engine pack lofted onto the back, its radiator, the exhaust stack, the jets under it
    pack(k, cp, [(0.10, chamfered(1.25, -0.80, -1.65)), (0.90, chamfered(1.55, -0.80, -2.00, 0.35)),
                 (1.90, chamfered(1.55, -0.80, -1.95, 0.35)), (2.45, chamfered(1.05, -0.80, -1.55))], M["armor2"], -1.25)
    k.vent(cp, 2.0 * U, 0.8 * U, 0, 1.4 * U, -2.0 * U - 0.2, 0, PI, 7)
    k.stripes(cp, 2.2 * U, 0, 0.35 * U, -1.66 * U - 0.2, PI, hb=1.6, gap=0.8)
    bz = 3 + out["z0"]
    k.cyl(1.6, 1.8, 12, M["dark"], cp, 1.0 * U, 2.45 * U + 5, -1.3 * U)
    k.tor(1.7, 0.3, M["frame"], cp, 1.0 * U, 2.45 * U + 8, -1.3 * U, PI / 2)
    k.sock("ex_stack", cp, 1.0 * U, 2.45 * U + 11.4, -1.3 * U)
    k.pl(4.4, 0.4, 4.4, M["frame"], cp, 1.0 * U, 2.45 * U + 12, -1.3 * U, 0.3)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(2.4, 3.0, 6, M["dark"], cp, s * 1.5 * U, 0.2 * U, -1.7 * U - 2, PI / 2)       # apart: no overlapping caps
        k.cyl(2.6, 2.6, 0.3, M["glowDim"], cp, s * 1.5 * U, 0.2 * U, -1.7 * U - 5.1, PI / 2)
        k.sock(f"JET_{S}", cp, s * 1.5 * U, 0.2 * U, -1.7 * U - 5.4, PI)
    # the opening is the Panorama's own clamshell (owner, 2026-10-24: the Foreman's opening "needs to follow the
    # titan style or panorama style"): the front under the screens drops forward into a step, then the front
    # over them swings up and out on two rams, the dash riding it; the cockpit's own halves ride these plates
    Yb, Yt, ys = hull["Yb"], hull["Yt"], hull["ysplit"]
    # the upper plate is clean with a hazard band (owner, 2026-10-24: no grilles)
    MB.contour(k, hull, "CHEST_upper", -0.5, 0.5, ys + 0.16, 0.11, 0.03, M["hazard"])
    # the lower plate is clean too, with its hazard edge, which becomes the step's nose
    MB.contour(k, hull, "CHEST_lower", -0.55, 0.55, Yb + 0.05, 0.05, 0.04, M["hazard"])
    MB.contour(k, hull, "CHEST_lower", -0.45, 0.45, ys - 0.12, 0.05, 0.03, M["red"])
    ap, (ax, ay, az) = MB.on(k, hull, "CHEST_lower", hull["at"](0, (Yb + ys) / 2, 0.05))
    k.box(5, 3, 0.8, M["glow"], ap, ax, ay, az)
    k.halo(ap, ax, ay, az + 0.8, 8, 0.5)
    MB.contour(k, hull, None, -1.2, 1.2, Yb - 0.22, 0.02, 0.05, M["hazard"])     # the sill's edge under the apron
    for S, s in (("L", 1), ("R", -1)):
        base = hull["at"](s * 1.05, Yt + 0.05, 0.10)
        vis, tip = MB.on(k, hull, "CHEST_upper", hull["at"](s * 0.55, (ys + Yt) / 2, 0.10))
        MB.ram(k, f"visor_{S}", cp, list(base), vis, list(tip), 0.9)
        MB.vent(k, cp, *hull["at"](s * 1.2, Yt + 0.05, 0.05), (s * 0.4, 1, 0.4), "open", rate=1.4)
        for f in (0.3, 0.7):
            MB.vent(k, cp, *hull["at"](s * PI / 2, Yb + f * (Yt - Yb), 0.03), (s, 0.1, 0.6), "seam")
    MB.vent(k, cp, *hull["at"](0, ys, 0.06), (0, -0.2, 1), "seam", rate=2.0)
    # arms: shoulders are boom feet with their rams
    sy = 5 + out["y1"] - 9
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb = contract_arm(k, chest, S, s * (out["hw"] + 5.5), sy, 2, -0.05, -15, -1.2)
        k.cyl(5, 5, 7, M["frame"], sh_, 0, 0, 0, 0, 0, PI / 2, 18)
        k.pl(9, 7, 13, M["armor"], sh_, s * 2, 4, 0, 0, 0, s * 0.2)
        k.box(1.0, 5, 11, M["hazard"], sh_, s * 6.6, 4, 0)
        k.box(6, 15, 6, M["frame"], sh_, 0, -7.5, 0)
        ram_pair(k, sh_, (0, 3, -4.4), (0, -13, -4.0), 0.9)
        k.cyl(3.6, 3.6, 7.5, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        hand = k.J(f"hand_{S}", elb, 0, -16, 0)
        if s < 0:   # the hydraulic breaker: a long body, its ram and accumulator, the chisel
            k.box(6.5, 16, 6.5, M["frame"], elb, 0, -8, 0)
            ram_pair(k, elb, (0, -2, 4.2), (0, -15, 4.0), 0.85)
            k.cyl(4.2, 4.2, 20, M["armor"], hand, 0, -10, 0, seg=18)
            k.stripes(k.g(hand, 0, -6, 0), 8.6, 0, 0, 4.3, 0, hb=1.2, gap=0.6)
            k.cyl(2.4, 2.4, 6, M["dark"], hand, 3.8, -6, 0, seg=14)
            k.sph(2.6, M["dark"], hand, 3.8, -3, 0)
            k.cyl(4.6, 4.6, 1.6, M["frame"], hand, 0, -19.5, 0, seg=18)
            k.cyl(1.6, 1.6, 9, M["metal"], hand, 0, -24.5, 0, seg=12)
            k.cyl(0, 1.6, 3, M["metal"], hand, 0, -30.5, 0, PI, seg=12)
            k.sock("MUZZLE_R", hand, 0, -32, 0, PI / 2)
        else:       # the boom cannon: a receiver, a ribbed barrel, a feed belt to the drum on the back
            k.box(6.5, 14, 6.5, M["frame"], elb, 0, -7, 0)
            ram_pair(k, elb, (0, -2, -4.2), (0, -13, -4.0), 0.8)
            k.pl(8, 10, 9, M["armor2"], hand, 0, -4, 0)
            k.box(9, 1.1, 0.4, M["hazard"], hand, 0, -8.6, 4.7)
            k.cyl(1.7, 1.9, 22, M["metal"], hand, 0, -20, 1, seg=14)
            for i in range(7):
                k.cyl(2.3, 2.3, 0.7, M["frame"], hand, 0, -12 - i * 2.2, 1, seg=14)
            k.box(4.4, 4, 4.4, M["dark"], hand, 0, -32, 1)
            k.sock("MUZZLE_L", hand, 0, -34.5, 1, PI / 2)
            k.hose(chest, [[s * (out["hw"] - 2), 4, bz - 9], [s * (out["hw"] + 8), 0, bz - 2], [s * (out["hw"] + 7), sy - 18, 4]], 1.4,
                   M["metal"])
            k.cyl(5, 5, 7, M["armor2"], chest, s * (out["hw"] - 2), 8, bz - 9, 0, 0, PI / 2, 20)
            k.tor(5, 0.4, M["frame"], chest, s * (out["hw"] + 1.6), 8, bz - 9, 0, PI / 2, 0)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.8, A=0.36, lift=0.45, bob=2.4 * k.s, roll=0.035, yaw=0.05, arm=0.07, lean=0.06, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.3, scanPeriod=3.0, breathHz=0.8))


# ----------------------------------------------------------------------------- IRONHORSE

def ironhorse(k):
    """The locomotive. Leaning forward on digitigrade legs whose knees and hocks are drive wheels joined by
    a coupling rod, cow-catcher ploughs on the shins; the torso is a banded boiler, a
    smokestack, a steam dome and a whistle on its collar, the headlamp over the prow; a firebox pack behind. Right: a
    steam cannon fed by pipes from the boiler, gauges on its breech. Left: a buffer ram. Boarding: the dogs
    round the prow's seam swing off, the smokebox front drops as a ramp with the cab's front on it.
    Cockpit: jockey (the driver leans into it)."""
    M = k.M
    segs, footH = [[19, -0.66], [21, 1.2], [11, -0.58]], 4.4
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(24, 7, 16, M["frame"], body, 0, 0, 0)
    k.cyl(4.6, 4.6, 30, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    for s in (1, -1):
        wheel(k, body, s * 15.5, -1, 0, 6, 2.2, 10)

    def seg0(k, g, s, L):
        k.box(5.5, L, 5.5, M["frame"], g, 0, -L / 2, 0)
        k.pl(8, L * 0.75, 4, M["armor"], g, 0, -L * 0.45, 3.6)
        k.stripes(g, 7, 0, -L * 0.3, 5.7, 0, hb=1.1, gap=0.6)
        k.box(1.2, L, 2, M["metal"], g, s * 6.6, -L / 2, 0)          # the coupling rod to the knee wheel

    def seg1(k, g, s, L):
        wheel(k, g, s * 1.0, 0, 0, 5, 2, 10)
        k.box(5, L, 5, M["frame"], g, 0, -L / 2, 0)
        cc = k.g(g, 0, -L * 0.55, 4)                                 # the cow-catcher
        for i in range(6):
            k.box(0.9, 9, 0.9, M["armor2"], cc, -3.75 + i * 1.5, 0, 0, -0.45, 0, 0)
        k.box(8.5, 1, 2.5, M["frame"], cc, 0, 4.2, -1.7)
        k.box(8.5, 1, 2.5, M["frame"], cc, 0, -4, 1.8)
        k.box(8.5, 0.8, 0.4, M["hazard"], cc, 0, -4.4, 3.2)
        k.box(1.2, L * 0.9, 2, M["metal"], g, s * 3.6, -L / 2, -1.5)

    def seg2(k, g, s, L):
        wheel(k, g, s * 0.8, 0, 0, 3.6, 1.8, 8)
        k.box(4, L, 4, M["frame"], g, 0, -L / 2, 0)
        k.pl(5, L * 0.7, 2.6, M["armor"], g, 0, -L / 2, 2.6)

    def foot(k, g, s, S):
        k.sph(2.8, M["dark"], g, 0, 0, 0)
        k.pl(9, 3, 12, M["armor"], g, 0, -1.8, 2.5)
        toe = k.J(f"toe_{S}", g, 0, -1.9, 8.6)
        k.pz([[-4.3, 0], [4.3, 0], [2.4, 4], [-2.4, 4]], 2.7, M["armor2"], toe, 0, 0, 0, -PI / 2)
        k.box(9.4, 0.9, 13, M["rubber"], g, 0, -3.4, 2.5)
        k.box(8, 1, 0.4, M["hazard"], g, 0, -1.2, 8.7)
        heel_jet(k, g, -6.5, 1.2)
        k.sock(f"WHEEL_{S}", g, 0, -3.4, 2.5)

    k.legs(body, 11.5, segs, [seg0, seg1, seg2], foot)
    waist = k.J("torso_twist", body, 0, 5.5, -1)
    k.cyl(7.5, 8.5, 4, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    chest = k.J("torso", waist, 0, 2.5, 0, 0.18)
    cp, cav, out, hull = chest_bay(k, chest, "cab", (0, 4, 4), "inner", doubled(IRONHORSE_TORSO), swing={"INNER_ramp_1": 2.65})
    hw, y0, y1, z1 = cav["hw"], cav["y0"], cav["y1"], out["z1"]
    # the boiler: the Excavator cab's hull (owner, 2026-10-24: "Iron horse has no seat still"; the cab is seated),
    # banded like a boiler; its front is the smokebox door, now a ramp: it drops forward on a hinge along its foot,
    # the cab's own front riding it, held shut by dogs round the seam
    Yb, Yt = hull["Yb"], hull["Yt"]
    cy = (out["y0"] + out["y1"]) / 2
    R = max(out["hw"], (out["y1"] - out["y0"]) / 2)
    U = 1.0 / k.s
    Lb = 1.0 * U                 # the collar's fittings: the stack forward, the dome and whistle behind it
    zc = -0.55 * U
    fz = hull["at"](0, Yt + 0.25, 0)[2]
    W = MB.ride_w("cab", "INNER_ramp")          # the ramp's half width: its seam down each side
    aw = {y: bearing_x(k, hull, W, y) for y in (Yb + 0.12, hull["sb"] - 0.10, Yt - 0.10, (Yb + Yt) / 2)}
    for y in (Yb + 0.12, hull["sb"] - 0.10, Yt - 0.10):
        MB.contour(k, hull, None, PI / 2 + 0.03, 3 * PI / 2 - 0.03, y, 0.04, 0.05, M["frame"])
        MB.contour(k, hull, "CHEST_ramp", -aw[y] + 0.06, aw[y] - 0.06, y, 0.04, 0.05, M["frame"])
    k.stripes(k.g(cp, *hull["at"](PI, (Yb + Yt) / 2, 0.02)), 2 * R * 0.5, 0, 0, 0, PI, hb=1.3, gap=0.7)
    D = hull["joints"]["CHEST_ramp"][0]
    _, (x, yy, zz) = MB.on(k, hull, "CHEST_ramp", hull["at"](0, (Yb + hull["sb"]) / 2, 0.04))
    k.tor(3.2, 0.5, M["metal"], D, x, yy, zz + 0.6, seg=24)                       # the handwheel
    for i in range(3):
        k.box(6.2, 0.5, 0.5, M["metal"], D, x, yy, zz + 0.6, 0, 0, i * PI / 3)
    k.cyl(1.2, 1.2, 1.2, M["frame"], D, x, yy, zz + 0.7, PI / 2)
    k.box(7, 1.8, 0.4, M["red"], D, x, yy + 5, zz + 0.3)                         # the number plate
    for f in (-0.6, 0.0, 0.6):
        k.cyl(0.9, 0.9, 1.8, M["frame"], D, f * W * U, 0, 0, 0, 0, PI / 2, seg=10)   # the hinge barrels, along its foot
    am = aw[(Yb + Yt) / 2]
    dogs = [(sg * (am + 0.05), Yb + f * (Yt - Yb), sg * PI / 2) for sg in (1, -1) for f in (0.2, 0.5, 0.8)]
    dogs += [(0, hull["y_hi"] + 0.05, 0)]
    for i, (a, y, ry) in enumerate(dogs):
        x, yy, zz = hull["at"](a, y, 0.03)
        dog = MB.hatch(k, f"CHEST_dog_{i + 1}", cp, x, yy, zz, ("z", 1.2), 0.0, 0.5, 0, ry, 0)
        k.box(1.0, 2.6, 0.8, M["metal"], dog, 0, -1.0, 0.3)
        k.cyl(0.55, 0.55, 1.2, M["frame"], dog, 0, 0, 0, PI / 2, seg=8)
    # the headlamp on the smokebox, the stack, the dome, the whistle
    ty = out["y1"]
    hy = (Yt + 0.3) * U + 1.6
    k.cyl(2.2, 2.6, 3, M["dark"], cp, 0, hy, fz - 2.5, PI / 2, seg=18)
    k.cyl(1.9, 1.9, 0.4, M["glow"], cp, 0, hy, fz - 1.0, PI / 2, seg=18)
    k.halo(cp, 0, hy, fz - 0.2, 14, 0.8)
    k.cyl(2.8, 2.2, 8, M["dark"], cp, 0, ty + 4, zc + Lb * 0.25)
    k.cyl(3.4, 2.6, 2.2, M["frame"], cp, 0, ty + 8.6, zc + Lb * 0.25)
    k.sock("ex_stack", cp, 0, ty + 9.8, zc + Lb * 0.25)
    k.sph(3, M["armor2"], cp, 0, ty + 1.2, zc - Lb * 0.05, 1, 0.8, 1)
    k.cyl(0.4, 0.5, 3, M["metal"], cp, 2, ty + 2, zc - Lb * 0.2, seg=8)
    k.cyl(0.7, 0.7, 1.2, M["metal"], cp, 2, ty + 3.8, zc - Lb * 0.2, seg=10)
    # the sensor: a cab-roof turret with twin lenses, ahead of the dome
    head = k.J("head", chest, 0, 4 + ty + 0.4, 4 + 0.35 * U)
    k.cyl(2.0, 2.4, 3.4, M["frame"], head, 0, -0.8, 0, seg=16)            # its neck, down into the roof
    k.box(6, 3, 4.5, M["armor"], head, 0, 1.5, 0)
    for x in (-1.4, 1.4):
        k.sph(0.8, M["lens"], head, x, 1.6, 2.3)
        k.halo(head, x, 1.6, 2.8, 4.5, 0.6)
    k.sock("SENSOR_head", head, 0, 1.6, 2.6)
    k.sock("CAM_main", head, 0, 1.6, 2.6)
    for S, s in (("L", 1), ("R", -1)):
        MB.vent(k, cp, *hull["at"](s * PI / 2, (Yb + Yt) / 2, 0.06), (s, 0.1, 0.5), "seam", rate=1.8)
        MB.vent(k, cp, *hull["at"](s * 0.8, Yt - 0.05, 0.06), (s * 0.5, 0.7, 0.5), "seam")
        MB.vent(k, cp, s * R * 0.9, cy - R * 0.3, zc, (s, -0.3, 0), "open", rate=1.6)    # the cylinder cocks
    MB.vent(k, cp, 0, ty + 9.6, zc + Lb * 0.25, (0, 1, 0), "open", rate=2.2)
    # the firebox behind: a rounded pack on the boiler's back, banded, the jets under it
    pack(k, cp, [(-0.10, rounded(1.10, -1.20, -2.20)), (0.70, rounded(1.40, -1.20, -2.55)), (1.50, rounded(1.30, -1.20, -2.45)),
                 (1.95, rounded(0.80, -1.20, -2.05))], M["armor2"], -1.75)
    k.stripes(cp, 1.4 * U, 0, 0.75 * U, -2.55 * U - 0.3, PI, hb=1.3, gap=0.7)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(2.4, 3.2, 5, M["dark"], cp, s * 1.4 * U, 0.0, -2.2 * U - 2, PI / 2)          # apart: no overlapping caps
        k.cyl(2.6, 2.6, 0.3, M["glowDim"], cp, s * 1.4 * U, 0.0, -2.2 * U - 4.7, PI / 2)
        k.sock(f"JET_{S}", cp, s * 1.4 * U, 0.0, -2.2 * U - 5, PI)
    sy = 4 + cy + R * 0.35
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb = contract_arm(k, chest, S, s * (R + 4.5), sy, 3, -0.08, -13, -1.35)
        k.sph(4.4, M["frame"], sh_, 0, 0, 0)
        k.pl(8, 6, 11, M["armor"], sh_, s * 1.8, 3.6, 0, 0, 0, s * 0.25)
        k.box(0.8, 4, 9, M["hazard"], sh_, s * 5.8, 3.6, 0)
        k.box(5, 13, 5, M["frame"], sh_, 0, -6.5, 0)
        k.cyl(3.4, 3.4, 7, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        hand = k.J(f"hand_{S}", elb, 0, -14, 0)
        if s < 0:   # the steam cannon: a breech with gauges, steam pipes from the boiler, a banded barrel
            k.pl(8, 13, 8, M["armor2"], elb, 0, -7, 0)
            for i, (yy, zz) in enumerate(((-4, 4.2), (-8, 4.2))):
                k.cyl(1.4, 1.4, 0.6, M["frame"], elb, 2, yy, zz, PI / 2, seg=16)
                k.cyl(1.1, 1.1, 0.3, M["glowDim"], elb, 2, yy, zz + 0.4, PI / 2, seg=16)
            k.tor(1.6, 0.25, M["metal"], elb, -2.2, -6, 4.3, seg=16)
            k.cyl(2.6, 2.9, 24, M["dark"], hand, 0, -12, 0, seg=18)
            for i in range(5):
                k.cyl(3.2, 3.2, 1.0, M["metal"], hand, 0, -3 - i * 4.6, 0, seg=18)
            k.cyl(3.6, 3.0, 3, M["frame"], hand, 0, -25.5, 0, seg=18)
            k.sock("MUZZLE_R", hand, 0, -27.5, 0, PI / 2)
            k.hose(chest, [[s * (R - 3), sy - 6, zc - 2], [s * (R + 5), sy - 10, 4], [s * (R + 4.5), sy - 18, 6]], 0.8, M["metal"])
        else:       # the buffer ram: a spring piston and a round buffer face
            k.pl(8, 12, 8, M["armor2"], elb, 0, -6.5, 0)
            k.cyl(1.4, 1.4, 10, M["metal"], hand, 0, -5, 0, seg=12)
            for i in range(7):
                k.tor(1.9, 0.35, M["frame"], hand, 0, -1 - i * 1.3, 0, PI / 2)
            k.cyl(4.6, 4.6, 1.6, M["armor"], hand, 0, -11, 0, seg=28)
            k.cyl(3.6, 3.6, 0.6, M["metal"], hand, 0, -12, 0, seg=28)
            k.box(7, 0.8, 0.5, M["hazard"], hand, 0, -11, 4.7)
            k.sock("MUZZLE_L", hand, 0, -12.5, 0, PI / 2)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.95, A=0.42, lift=0.55, bob=2.0 * k.s, roll=0.04, yaw=0.06, arm=0.09, lean=0.12, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.45, scanPeriod=2.0, breathHz=1.2))


# ----------------------------------------------------------------------------- the 24 m round, in detail
# The owner, 2026-10-24, of the block-outs: "1,2,6,8,11,12 start building them. Any cockpit style should do", then "I
# want combat mechs with guns". So the Harrow, the Roustabout, the Driller, the Mule, the Picket and the Sapper are built
# in the kit's finish at 24 m, each from its block-out's silhouette, with a detailed seated cockpit whose chest opens as
# one body with it (mode inner), and every one armed. The parts they share are one implementation each, below.

def walker_legs(k, body, hip_w, segs, look):
    """Legs for the 24 m round, on the contract's joints. look: t (thickness), shin ("slab", "piston" or "lattice"),
    rams (hydraulic rams down the thigh), knee ("pad" for a humanoid's knee guard). The foot carries the skate wheel at
    its heel (survey M4) and a heel jet."""
    M = k.M
    t = look.get("t", 1.0)

    def thigh(k, g, s, L):
        k.cyl(2.6 * t, 2.6 * t, 6.0 * t, M["frame"], g, 0, 0, 0, 0, 0, PI / 2, 18)
        k.box(4.2 * t, L, 4.4 * t, M["frame"], g, 0, -L / 2, 0)
        k.pl(5.0 * t, L * 0.78, 2.0, M["armor"], g, 0, -L * 0.48, 2.8 * t)
        k.pl(1.8, L * 0.72, 5.2 * t, M["armor2"], g, s * 3.0 * t, -L * 0.48, 0)
        k.stripes(k.g(g, s * 3.95 * t, -L * 0.45, 0, 0, PI / 2, 0), 4.0 * t, 0, 0, 0, 0, hb=0.8, gap=0.4)
        if getattr(k, "mark", None):                 # chevrons and a hydraulics plate on the thigh's front, the squadron outside
            k.decal(_mk(k, "chevron"), g, 0, -L * 0.3, 2.8 * t + 1.08, 0, 0, 0, 2.6 * t)
            k.decal("caution_hyd", g, 0, -L * 0.62, 2.8 * t + 1.08, 0, 0, 0, 4.6 * t)
            if s > 0:
                k.decal(_mk(k, "emblem"), g, s * (3.0 * t + 0.98), -L * 0.62, 0, 0, PI / 2, 0, 3.6 * t)
        if look.get("rams", True):
            ram_pair(k, g, (0, -1.5, -3.2 * t), (0, -L + 2, -2.9 * t), 0.75 * t)

    def shin(k, g, s, L):
        k.cyl(2.3 * t, 2.3 * t, 6.2 * t, M["dark"], g, 0, 0, 0, 0, 0, PI / 2, 18)
        kind = look.get("shin", "slab")
        if kind == "lattice":
            lattice(k, g, 3.9 * t, 3.9 * t, L - 1.2, M["armor2"], n=4, y0=-0.6)   # narrower than the thigh: no shared sides
        elif kind == "piston":
            k.cyl(2.3 * t, 2.5 * t, L * 0.62, M["armor"], g, 0, -L * 0.33, 0, seg=18)
            k.cyl(1.25 * t, 1.25 * t, L * 0.5, M["metal"], g, 0, -L * 0.72, 0, seg=12)
            for i in range(3):
                k.tor(2.6 * t, 0.3, M["frame"], g, 0, -2 - i * L * 0.17, 0, PI / 2)
        else:
            k.pl(4.6 * t, L * 0.86, 4.2 * t, M["armor"], g, 0, -L * 0.47, 0)
            for i in range(3):
                k.box(5.0 * t, 0.6, 4.6 * t, M["frame"], g, 0, -3 - i * L * 0.25, 0)
            k.pz([[-2.8 * t, 0], [2.8 * t, 0], [2.2 * t, 3], [-2.2 * t, 3]], 1.4, M["armor2"], g, 0, -L * 0.3, 2.5 * t, -PI / 2)
        if look.get("knee") == "pad":
            k.pl(5.4 * t, 5.2, 2.2, M["armor"], g, 0, -1.2, 3.4 * t, 0.15)
            k.box(3.2 * t, 0.6, 0.3, M["hazard"], g, 0, -1.0, 4.6 * t)
        k.box(4.0 * t, 0.9, 0.3, M["hazard"], g, 0, -L * 0.82, 2.45 * t)
        k.cyl(1.0 * t, 1.3 * t, 2.6 * t, M["dark"], g, 0, -L * 0.45, -2.8 * t, PI / 2 + 0.5)        # the calf booster
        k.cyl(1.0 * t, 1.0 * t, 0.2, M["glowDim"], g, 0, -L * 0.45 + 0.6, -4.0 * t, PI / 2 + 0.5)

    def hock(k, g, s, L):
        k.cyl(1.9 * t, 1.9 * t, 5.6 * t, M["dark"], g, 0, 0, 0, 0, 0, PI / 2, 16)
        k.pl(3.6 * t, L, 3.8 * t, M["armor2"], g, 0, -L / 2, 0)
        k.pl(4.0 * t, L * 0.5, 1.0, M["armor"], g, 0, -L * 0.45, 2.2 * t)
        ram_pair(k, g, (0, -0.8, -2.4 * t), (0, -L + 0.8, -2.2 * t), 0.6 * t)

    def foot(k, g, s, S):
        k.sph(1.7 * t, M["dark"], g, 0, 0, 0)
        k.pl(6.4 * t, 2.2, 10.5 * t, M["armor"], g, 0, -1.4, 1.8 * t)
        toe = k.J(f"toe_{S}", g, 0, -2.0, 7.0 * t)
        k.pz([[-3.2 * t, 0], [3.2 * t, 0], [2.2 * t, 3], [-2.2 * t, 3]], 2.0, M["armor2"], toe, 0, 0, 0.5, -PI / 2)
        k.box(6.8 * t, 0.8, 10.8 * t, M["rubber"], g, 0, -2.9, 1.8 * t)
        k.box(6.0 * t, 0.7, 0.3, M["hazard"], g, 0, -1.0, 7.1 * t)
        k.cyl(1.7 * t, 1.7 * t, 3.6 * t, M["rubber"], g, 0, -2.0, -3.2 * t, 0, 0, PI / 2, 18)          # the skate wheel
        k.cyl(0.8 * t, 0.8 * t, 3.9 * t, M["metal"], g, 0, -2.0, -3.2 * t, 0, 0, PI / 2, 10)
        heel_jet(k, g, -5.6 * t, 0.9 * t)
        k.sock(f"WHEEL_{S}", g, 0, -2.9, -3.2 * t)

    k.legs(body, hip_w, segs, [thigh, shin] + ([hock] if len(segs) == 3 else []), foot)


def walker_arm(k, chest, S, s, x, y, z, a=1.0, upper=9.0, elb_rx=-1.3):
    """An arm for the 24 m round: shoulder ball, a raked pauldron with its hazard edge, the upper arm, the elbow drum,
    an armoured forearm. Returns (shoulder, elbow, hand): weapons hang on the hand along its -y, +z up."""
    M = k.M
    sh, elb = contract_arm(k, chest, S, x, y, z, -0.05, -upper * a, elb_rx)
    k.sph(3.0 * a, M["frame"], sh, 0, 0, 0)
    k.pl(6.2 * a, 4.4 * a, 7.4 * a, M["armor"], sh, s * 1.6 * a, 2.2 * a, 0, 0, 0, s * 0.2)
    k.box(0.6, 3.2 * a, 6.2 * a, M["hazard"], sh, s * 4.75 * a, 1.9 * a, 0, 0, 0, s * 0.2)
    if getattr(k, "mark", None):                     # the unit number on the pauldron, NO STEP on its top, lines on the forearm
        pg = k.g(sh, s * 1.6 * a, 2.2 * a, 0, 0, 0, s * 0.2)
        unit_number(k, pg, 0, 0.1 * a, 3.7 * a + 0.08, 0, 3.0 * a)
        k.decal(_mk(k, "nostep"), pg, 0, 2.2 * a + 0.08, 0, -PI / 2, 0, 0, 4.0 * a)
    k.box(3.4 * a, upper * a, 3.4 * a, M["frame"], sh, 0, -upper * a / 2, 0)
    k.cyl(2.4 * a, 2.4 * a, 5.4 * a, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2, 16)
    k.pl(4.8 * a, 8 * a, 5.0 * a, M["armor2"], elb, 0, -4.4 * a, 0)
    k.box(5.0 * a, 0.7, 0.3, M["hazard"], elb, 0, -7.6 * a, 2.55 * a)
    if getattr(k, "mark", None):
        k.decal(_mk(k, "lines"), elb, s * (2.4 * a + 0.08), -4.4 * a, 0, 0, s * PI / 2, PI / 2, 5.6 * a)
    hand = k.J(f"hand_{S}", elb, 0, -8.8 * a, 0)
    return sh, elb, hand


def autocannon(k, hand, S, L=16.0, r=1.0, mag=True):
    """A belt-fed autocannon on the hand: receiver, ribbed barrel, muzzle brake, a box magazine under it."""
    M = k.M
    k.pl(4.4 * r, 7, 4.6 * r, M["armor"], hand, 0, -3.2, 0)
    k.box(4.8 * r, 0.7, 0.3, M["hazard"], hand, 0, -6.2, 2.4 * r)
    k.cyl(0.85 * r, 1.0 * r, L, M["metal"], hand, 0, -6 - L / 2, 0.5, seg=14)
    for i in range(int(L / 3)):
        k.cyl(1.3 * r, 1.3 * r, 0.6, M["frame"], hand, 0, -8 - i * 3, 0.5, seg=14)
    k.box(2.6 * r, 2.2 * r, 2.4 * r, M["dark"], hand, 0, -6.6 - L, 0.5)
    k.sock(f"MUZZLE_{S}", hand, 0, -8 - L, 0.5, PI / 2)
    if mag:
        k.pl(3.0 * r, 3.6 * r, 3.8 * r, M["armor2"], hand, 0, -3.0, -4.0 * r)


def rifle(k, hand, S, L=14.0, r=1.0):
    """A rifle held in the hand: receiver, a curved box magazine, a stock, a barrel with a shroud, a sight."""
    M = k.M
    k.box(2.6 * r, 9 * r, 3.2 * r, M["frame"], hand, 0, -3.5 * r, 0.4)
    k.pl(2.0 * r, 3.0 * r, 4.4 * r, M["armor2"], hand, 0, -4.2 * r, -3.4 * r, 0.25)
    k.box(2.2 * r, 4 * r, 2.4 * r, M["dark"], hand, 0, 2.2 * r, 0.6)
    k.cyl(0.55 * r, 0.6 * r, L, M["metal"], hand, 0, -8 * r - L / 2, 0.7, seg=12)
    k.box(1.8 * r, 4 * r, 1.8 * r, M["frame"], hand, 0, -9.5 * r, 0.7)
    k.box(1.0 * r, 3.2 * r, 1.0 * r, M["armor"], hand, 0, -2.5 * r, 2.4 * r)
    k.cyl(0.4 * r, 0.4 * r, 0.3, M["lens"], hand, 0, -4.2 * r, 2.4 * r, PI / 2)
    k.sock(f"MUZZLE_{S}", hand, 0, -8 * r - L - 0.5, 0.7, PI / 2)


def fist(k, elb, hand, S, a=1.0):
    """A punching fist: a heavy forearm with a ram down it, the fist block with a knuckle plate and a hazard band."""
    M = k.M
    k.pl(6.4 * a, 9 * a, 6.6 * a, M["armor"], elb, 0, -5 * a, 0)
    ram_pair(k, elb, (0, -1, 3.8 * a), (0, -9 * a, 3.6 * a), 0.7 * a)
    k.pl(6.0 * a, 5.6 * a, 6.2 * a, M["armor2"], hand, 0, -3.0 * a, 0)
    k.box(6.2 * a, 1.6 * a, 6.4 * a, M["metal"], hand, 0, -6.2 * a, 0)
    for i in range(4):
        k.box(1.2 * a, 1.0 * a, 1.6 * a, M["frame"], hand, -2.25 * a + i * 1.5 * a, -7.2 * a, 1.6 * a)
    k.box(6.3 * a, 0.8, 6.5 * a, M["hazard"], hand, 0, -0.4, 0)
    k.sock(f"MUZZLE_{S}", hand, 0, -7.6 * a, 0, PI / 2)


def missile_rack(k, p, x, y, z, cols, rows, cell=1.8, rx=0.0, ry=0.0, name=None):
    """A box of missile cells facing ahead (+z), red tips in dark tubes, a hazard band over them; its socket fires."""
    M = k.M
    g = k.g(p, x, y, z, rx, ry, 0)
    W, H, D = cols * cell + 1.4, rows * cell + 1.4, 7.0
    k.pl(W, H, D, M["armor"], g, 0, 0, 0)
    k.box(W + 0.4, H * 0.3, D * 0.6, M["frame"], g, 0, -H * 0.2, -0.6)
    for i in range(cols):
        for j in range(rows):
            cx, cy = (i - (cols - 1) / 2) * cell, (j - (rows - 1) / 2) * cell
            k.cyl(cell * 0.4, cell * 0.4, 0.6, M["ink"], g, cx, cy, D / 2 + 0.05, PI / 2, seg=10)
            k.cyl(cell * 0.22, cell * 0.22, 0.4, M["tip"], g, cx, cy, D / 2 + 0.2, PI / 2, seg=8)
    k.box(W * 0.9, 0.7, 0.3, M["hazard"], g, 0, H / 2 + 0.2, D / 2 - 0.4)
    if name:
        k.sock(name, g, 0, 0, D / 2 + 0.6, PI / 2)


def shield(k, elb, s, a=1.0):
    """A tall shield on the forearm's outside, raked, with its livery band and a hazard edge."""
    M = k.M
    g = k.g(elb, s * 3.4 * a, -5 * a, 1.0, 0, 0, s * 0.08)
    k.pl(1.4, 24 * a, 13 * a, M["armor"], g, 0, 0, 0)
    k.pl(0.8, 20 * a, 10 * a, M["armor2"], g, s * 0.9, 0, 0)
    k.stripes(k.g(g, s * 1.4, 3 * a, 0, 0, s * PI / 2, 0), 8 * a, 0, 0, 0, 0, hb=1.2, gap=0.6)
    k.box(1.6, 1.0, 12.6 * a, M["hazard"], g, 0, -11.6 * a, 0)
    if getattr(k, "mark", None):                     # the squadron's emblem and the unit number on the shield's face
        k.decal(_mk(k, "emblem"), g, s * 1.36, -2.6 * a, 0, 0, s * PI / 2, 0, 7.4 * a)
        unit_number(k, g, s * 1.36, -8.0 * a, 0, s * PI / 2, 3.4 * a)


def walker_head(k, chest, x, y, z, kind, a=1.0):
    """The sensor head: "visor" (a raked helmet with one lit band and a swept antenna), "turret" (a drum with three
    lenses on a slide) or "mast" (a camera on a post)."""
    M = k.M
    head = k.J("head", chest, x, y, z)
    # the neck runs down into the torso, so the head is seated on it whatever the collar's shape (owner, 2026-10-24:
    # "mech heads tend to not be attached")
    k.cyl(1.6 * a, 2.0 * a, 4.0 * a, M["frame"], head, 0, -1.2 * a, 0, seg=16)
    k.tor(2.1 * a, 0.35 * a, M["dark"], head, 0, -0.2 * a, 0, PI / 2, seg=20)
    if kind == "visor":
        k.cyl(1.8 * a, 2.2 * a, 2.0 * a, M["frame"], head, 0, 1.0 * a, 0)
        k.pz([[-3.2 * a, 0], [3.2 * a, 0], [2.6 * a, 4.6 * a], [-2.6 * a, 4.6 * a]], 5.6 * a, M["armor"], head, 0, 1.8 * a, -0.2, 0, PI / 2, 0)
        k.box(4.6 * a, 0.9 * a, 0.5, M["glow"], head, 0, 4.0 * a, 2.85 * a)
        k.halo(head, 0, 4.0 * a, 3.4 * a, 10 * a, 0.7)
        k.pl(5.2 * a, 1.0 * a, 3.0 * a, M["armor2"], head, 0, 5.2 * a, 1.8 * a, 0.3)
        k.ant(head, -2.0 * a, 6.2 * a, -1.2 * a, 6 * a, -0.6, 0.2)
        k.sock("SENSOR_head", head, 0, 4.0 * a, 3.0 * a)
    elif kind == "turret":
        k.cyl(3.4 * a, 3.6 * a, 3.6 * a, M["armor"], head, 0, 1.8 * a, 0, seg=20)
        k.box(6.6 * a, 1.8 * a, 2.0 * a, M["frame"], head, 0, 2.0 * a, 3.0 * a)
        for i, (dx, r) in enumerate(((-1.8, 0.9), (0.6, 0.65), (2.2, 0.45))):
            k.cyl(r * a, r * a, 1.6 * a, M["dark"], head, dx * a, 2.0 * a, 3.8 * a, PI / 2)
            k.sph(r * 0.8 * a, M["lens"], head, dx * a, 2.0 * a, 4.6 * a)
            k.halo(head, dx * a, 2.0 * a, 5.0 * a, 4 * a, 0.6)
        k.sock("SENSOR_head", head, -1.8 * a, 2.0 * a, 4.6 * a)
    else:
        k.cyl(0.6 * a, 0.7 * a, 6 * a, M["frame"], head, 0, 3 * a, 0, seg=10)
        k.pl(3.4 * a, 2.6 * a, 4.4 * a, M["armor2"], head, 0, 6.6 * a, 0.6 * a)
        k.cyl(0.9 * a, 0.9 * a, 1.2 * a, M["dark"], head, 0, 6.6 * a, 3.0 * a, PI / 2)
        k.sph(0.7 * a, M["lens"], head, 0, 6.6 * a, 3.6 * a)
        k.sock("SENSOR_head", head, 0, 6.6 * a, 3.6 * a)
    k.sock("CAM_main", head, 0, 3.0 * a, 2.0 * a)
    return head


def back_jets(k, cp, xs, y, z, r=2.4):
    """The jets under a back pack (survey M3), pointing down and back, their sockets for the game."""
    M = k.M
    for S, x in zip(("L", "R"), xs):
        k.cyl(r, r * 1.3, r * 2.2, M["dark"], cp, x, y, z, PI / 2 + 0.6)
        k.cyl(r * 1.05, r * 1.05, 0.3, M["glowDim"], cp, x, y - r * 0.9, z - r * 0.6, PI / 2 + 0.6)
        # the contract hangs the jets on the torso (cb_core::mech::rig::SOCKETS), so the socket is placed in the torso's
        # frame at the point it marks in the cockpit's
        k.sock(f"JET_{S}", K.Node("torso", k.world["torso"].inverted() @ k.world[cp.joint] @ cp.M), x, y - r, z - r * 0.7, PI)


def walker_extras(k, hipY, footH, hz=0.8, lean=0.08):
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=hz, A=0.36, lift=0.45, bob=2.0 * k.s, roll=0.035, yaw=0.05, arm=0.07, lean=lean, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.3, scanPeriod=3.0, breathHz=0.8))


def frame_contours(k, hull, Yb, Yt):
    """The bands every chest in this round wears: a hazard band round the back at the floor, frame ribs at the roof."""
    M = k.M
    MB.contour(k, hull, None, PI / 2 + 0.05, 3 * PI / 2 - 0.05, Yb + 0.05, 0.03, 0.06, M["hazard"])
    MB.contour(k, hull, None, PI / 2 + 0.05, 3 * PI / 2 - 0.05, Yt + 0.15, 0.03, 0.05, M["frame"])
    for s in (1, -1):
        for f in (0.3, 0.7):
            MB.vent(k, hull["cp"], *hull["at"](s * PI / 2, Yb + f * (Yt - Yb), 0.04), (s, 0.1, 0.6), "seam")


# the torsos (mech_boarding.section, metres from the interior's floor): twice the Foreman's, round a true-size cockpit
HARROW_TORSO = [
    (-1.40, 2.00, 1.20, -3.60, dict(wcut=1.10, sh=0.20, cb=0.50)),
    (-0.80, 2.80, 1.50, -4.60, dict(wcut=1.20, sh=0.50, cb=0.60)),
    (0.40, 3.10, 1.40, -5.00, dict(wcut=1.25, sh=0.70, cb=0.70)),
    (1.60, 3.00, 1.10, -4.90, dict(wcut=1.25, sh=0.70, cb=0.70)),
    (2.40, 2.50, 0.60, -4.40, dict(wcut=1.10, sh=0.60, cb=0.60)),
    (3.00, 1.60, 0.35, -3.60, dict(wcut=0.90, sh=0.40, cb=0.50)),
]
ROUSTABOUT_TORSO = [
    (-1.60, 2.20, 1.00, -2.00, dict(wcut=1.10, sh=0.20, cb=0.50)),
    (-0.90, 3.00, 1.20, -2.60, dict(wcut=1.15, sh=0.50, cb=0.50)),
    (0.60, 3.40, 1.20, -2.90, dict(wcut=1.15, sh=0.70, cb=0.50)),
    (2.00, 3.60, 1.10, -3.00, dict(wcut=1.15, sh=0.80, cb=0.50)),
    (3.20, 3.40, 0.80, -2.90, dict(wcut=1.10, sh=0.80, cb=0.50)),
    (4.00, 2.60, 0.50, -2.40, dict(wcut=0.90, sh=0.60, cb=0.40)),
    (4.50, 1.60, 0.40, -1.80, dict(wcut=0.70, sh=0.30, cb=0.30)),
]
DRILLER_TORSO = [
    (-1.60, 2.40, 1.10, -2.40, dict(wcut=1.05, sh=0.20, cb=0.60)),
    (-0.80, 3.40, 1.30, -3.00, dict(wcut=1.10, sh=0.60, cb=0.70)),
    (0.80, 3.90, 1.30, -3.40, dict(wcut=1.15, sh=0.90, cb=0.80)),
    (2.20, 4.00, 1.20, -3.40, dict(wcut=1.15, sh=1.00, cb=0.80)),
    (3.40, 3.60, 0.90, -3.20, dict(wcut=1.05, sh=0.90, cb=0.70)),
    (4.20, 2.60, 0.50, -2.60, dict(wcut=0.85, sh=0.60, cb=0.50)),
    (4.70, 1.50, 0.40, -1.80, dict(wcut=0.60, sh=0.30, cb=0.30)),
]
MULE_TORSO = [
    (-1.60, 2.60, 1.20, -2.60, dict(wcut=1.00, sh=0.30, cb=0.40)),
    (-0.60, 3.40, 1.40, -3.00, dict(wcut=1.05, sh=0.60, cb=0.50)),
    (1.00, 3.60, 1.30, -3.20, dict(wcut=1.05, sh=0.70, cb=0.50)),
    (2.40, 3.50, 1.20, -3.20, dict(wcut=1.05, sh=0.70, cb=0.50)),
    (3.20, 3.00, 0.90, -3.00, dict(wcut=0.95, sh=0.60, cb=0.40)),
    (3.80, 2.00, 0.50, -2.40, dict(wcut=0.80, sh=0.40, cb=0.30)),
]
PICKET_TORSO = [
    (-1.40, 1.60, 1.00, -1.60, dict(wcut=1.00, sh=0.10, cb=0.40)),
    (-0.60, 2.40, 1.20, -2.00, dict(wcut=1.10, sh=0.40, cb=0.50)),
    (0.80, 3.40, 1.30, -2.40, dict(wcut=1.15, sh=0.80, cb=0.60)),
    (2.20, 3.60, 1.20, -2.50, dict(wcut=1.15, sh=0.90, cb=0.60)),
    (3.00, 3.20, 0.90, -2.30, dict(wcut=1.00, sh=0.80, cb=0.50)),
    (3.60, 2.00, 0.50, -1.80, dict(wcut=0.80, sh=0.40, cb=0.40)),
]
SAPPER_TORSO = [(y, w * 1.1, zf, zb * 1.1, dict(kw, wcut=kw["wcut"])) for y, w, zf, zb, kw in PICKET_TORSO]


def _body(k, segs, footH, look, hip_w, axle=20.0):
    """Pelvis on its axle and the legs; returns (pelvis, hipY)."""
    M = k.M
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.cyl(3.4 * look.get("t", 1), 3.4 * look.get("t", 1), axle, M["dark"], body, 0, 0, 0, 0, 0, PI / 2, 20)
    k.pl(axle * 0.55, 4.4, 8.4, M["frame"], body, 0, 1.2, -0.6)
    k.box(axle * 0.45, 1.0, 0.4, M["hazard"], body, 0, -0.4, 3.7)
    walker_legs(k, body, hip_w, segs, look)
    return body, hipY


def _waist(k, body, r=6.0):
    M = k.M
    waist = k.J("torso_twist", body, 0, 3.2, -0.4)
    k.cyl(r, r + 0.4, 2.2, M["dark"], waist, 0, 0, 0, 0, 0, 0, 28)
    k.tor(r + 0.2, 0.5, M["frame"], waist, 0, 1.0, 0, PI / 2, seg=28)
    return k.J("torso", waist, 0, 1.4, 0)


# ----------------------------------------------------------------------------- 1 HARROW

def harrow(k):
    """Mech 1, after the block-out (the Mad Cat pattern): one long armoured hull slung low between heavy digitigrade
    legs, the Canopy cockpit in its nose (the canopy and the hull's nose over it lift back as one), a power pack and
    two stacks behind; on each shoulder a six-cell missile rack, the arms hung low as gun pods, an autocannon left and a
    heavy cannon right; a camera mast beside the canopy."""
    M = k.M
    segs, footH = [[15, -0.7], [17, 1.2], [8, -0.5]], 3.2
    body, hipY = _body(k, segs, footH, dict(t=1.05), 9.5, 22)
    chest = _waist(k, body)
    at = (0, 3.0, 1.0)
    # the canopy and the nose over it rise 2.6 m, clear of the hull behind, then tilt back 57 degrees on their back edge
    cp, cav, out, hull = chest_bay(k, chest, "canopy", at, "inner", HARROW_TORSO, wrap_z=-1.4,
                                   swing={"INNER_canopy": 2.6, "INNER_canopy_1": -1.0},
                                   timing={"INNER_canopy": (0.35, 1.3), "INNER_canopy_1": (1.7, 1.4)})
    hull["cp"] = cp
    U, Yb, Yt = 1.0 / k.s, hull["Yb"], hull["Yt"]
    frame_contours(k, hull, Yb, Yt)
    for s in (1, -1):                                          # livery down the flanks, a chine band
        k.stripes(k.g(cp, *hull["at"](s * 1.9, 0.9, 0.03)), 3.0 * U, 0, 0, 0, s * 1.9, hb=1.1, gap=0.6)
    k.box(3.4, 1.0, 0.6, M["glow"], cp, *hull["at"](0, -0.4, 0.05))
    k.halo(cp, *hull["at"](0, -0.4, 0.3), 9, 0.6)
    pack(k, cp, [(-0.6, chamfered(1.8, -4.6, -6.0)), (0.8, chamfered(2.1, -4.6, -6.5, 0.4)), (2.2, chamfered(1.7, -4.6, -6.1))],
         M["armor2"], -5.3)
    k.vent(cp, 2.8 * U, 1.0 * U, 0, 0.8 * U, -6.5 * U - 0.2, 0, PI, 7)
    for s in (1, -1):
        k.cyl(1.0, 1.2, 9, M["dark"], cp, s * 1.1 * U, 2.6 * U + 4, -5.6 * U)
        k.tor(1.15, 0.3, M["frame"], cp, s * 1.1 * U, 2.6 * U + 7, -5.6 * U, PI / 2)
    back_jets(k, cp, (1.2 * U, -1.2 * U), -1.2 * U, -5.6 * U, 2.0)
    walker_head(k, chest, at[0] - 1.9 * U, at[1] + Yt * U + 0.4, at[2] - 1.6 * U, "mast", 1.1)
    sy, sz = at[1] + 1.2 * U, at[2] - 1.4 * U
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb, hand = walker_arm(k, chest, S, s, s * (out["hw"] + 3.0), sy, sz, 0.95, 7.0, -1.45)
        k.cyl(4.2, 4.2, 5, M["frame"], sh_, 0, 0, 0, 0, 0, PI / 2, 20)
        missile_rack(k, sh_, s * 0.6, 7.8, 0.4, 3, 2, 2.0, -0.25, 0, f"MISSILE_{S}")
        # the gun pod is the forearm: a long armoured housing fed from the outside
        k.pl(6.4, 14, 6.8, M["armor"], elb, 0, -6, 0)
        k.pl(3.2, 6.0, 5.0, M["armor2"], elb, s * 4.4, -5, -0.6)
        k.stripes(k.g(elb, s * 3.3, -6, 0, 0, s * PI / 2, 0), 5, 0, 0, 0, 0, hb=1.0, gap=0.5)
        autocannon(k, hand, S, 16 if s > 0 else 12, 1.0 if s > 0 else 1.5, mag=False)
    walker_extras(k, hipY, footH, 0.85, 0.06)


# ----------------------------------------------------------------------------- 2 ROUSTABOUT

def roustabout(k):
    """Mech 2, after the block-out (the Scopedog pattern): a boxy upright chest on stocky legs with piston shins, the
    Titan cockpit in its middle (the front over the screens rises and tilts back, the front under them drops), a
    three-lens camera turret on top, a backpack with vents and jets; a heavy rifle with a box magazine in the right
    hand and a piston fist on the left."""
    M = k.M
    segs, footH = [[14, -0.6], [15, 1.1], [7, -0.5]], 3.0
    body, hipY = _body(k, segs, footH, dict(t=1.15, shin="piston"), 8.6, 20)
    chest = _waist(k, body)
    at = (0, 3.4, 0.6)
    cp, cav, out, hull = chest_bay(k, chest, "titan", at, "inner", ROUSTABOUT_TORSO, swing={"INNER_lower_1": 2.7})
    hull["cp"] = cp
    U, Yb, Yt = 1.0 / k.s, hull["Yb"], hull["Yt"]
    frame_contours(k, hull, Yb, Yt)
    for s in (1, -1):
        k.stripes(k.g(cp, *hull["at"](s * 1.6, 2.6, 0.03)), 3.0 * U, 0, 0, 0, s * 1.6, hb=1.1, gap=0.6)
        MB.contour(k, hull, None, s * (PI / 2 + 0.1), s * (PI - 0.3), 3.6, 0.03, 0.05, M["hazard"])
    walker_head(k, chest, 0, at[1] + 4.35 * U, at[2] - 0.9 * U, "turret", 1.15)
    pack(k, cp, [(-0.6, chamfered(2.2, -2.6, -4.0)), (1.2, chamfered(2.6, -2.6, -4.4, 0.4)), (3.4, chamfered(2.3, -2.6, -4.2))],
         M["armor2"], -3.3)
    for s in (1, -1):
        k.vent(cp, 1.4 * U, 1.8 * U, s * 1.2 * U, 1.6 * U, -4.4 * U - 0.2, 0, PI, 6)
    back_jets(k, cp, (1.4 * U, -1.4 * U), -1.4 * U, -3.6 * U, 2.2)
    sy, sz = at[1] + 2.4 * U, at[2] - 0.6 * U
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb, hand = walker_arm(k, chest, S, s, s * (out["hw"] + 3.2), sy, sz, 1.05, 8.0)
        if s < 0:
            rifle(k, hand, S, 16, 1.1)
        else:
            fist(k, elb, hand, S, 1.1)
    walker_extras(k, hipY, footH, 0.9, 0.08)


# ----------------------------------------------------------------------------- 6 DRILLER

def driller(k):
    """Mech 6, after the block-out (a mine drill rig, armed): a big chest on lattice shins, the Panorama in it (the
    front under the screens folds down, the hood swings up), a lattice derrick standing on its back with a drill
    string, big arms: an auger drill on the right and a heavy autocannon on the left, a rock-bolt magazine on the back."""
    M = k.M
    segs, footH = [[14.5, -0.65], [16, 1.15], [7.5, -0.5]], 3.0
    body, hipY = _body(k, segs, footH, dict(t=1.0, shin="lattice"), 8.8, 20)
    chest = _waist(k, body)
    at = (0, 3.4, 0.8)
    cp, cav, out, hull = chest_bay(k, chest, "panorama", at, "inner", DRILLER_TORSO,
                                   swing={"INNER_upper_1": -2.75, "INNER_lower_1": 2.7})
    hull["cp"] = cp
    U, Yb, Yt = 1.0 / k.s, hull["Yb"], hull["Yt"]
    frame_contours(k, hull, Yb, Yt)
    for s in (1, -1):
        k.stripes(k.g(cp, *hull["at"](s * 1.5, 3.2, 0.03)), 3.2 * U, 0, 0, 0, s * 1.5, hb=1.2, gap=0.6)
        beacon(k, cp, *hull["at"](s * (PI / 2 + 0.6), 4.3, 0.02))
    walker_head(k, chest, 0, at[1] + 4.55 * U, at[2] - 0.6 * U, "visor", 1.0)
    # the derrick: a lattice standing on the back, the drill string down its middle, a crown block on top
    der = k.g(cp, 0, 3.6 * U, -3.6 * U, PI, 0, 0)
    lattice(k, der, 4.0, 4.0, 22, M["red"], n=6, y0=0)
    k.cyl(0.6, 0.6, 20, M["metal"], cp, 0, 3.6 * U + 11, -3.6 * U)
    k.pl(6.4, 2, 6.4, M["frame"], cp, 0, 3.6 * U + 23.2, -3.6 * U)
    k.lamp(cp, 0, 3.6 * U + 24, -3.6 * U, 0.7, red=True)
    pack(k, cp, [(-0.8, rounded(1.6, -3.0, -4.6)), (1.6, rounded(1.9, -3.0, -4.9)), (3.0, rounded(1.4, -3.0, -4.5))], M["armor2"], -3.8)
    back_jets(k, cp, (1.5 * U, -1.5 * U), -1.4 * U, -4.0 * U, 2.2)
    sy, sz = at[1] + 2.6 * U, at[2] - 0.8 * U
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb, hand = walker_arm(k, chest, S, s, s * (out["hw"] + 3.4), sy, sz, 1.15, 8.0)
        if s < 0:               # the auger: a motor, the tapering screw and its flights
            k.pl(6, 9, 6, M["armor"], elb, 0, -5, 0)
            k.cyl(3.0, 3.0, 4, M["dark"], hand, 0, -2, 0, seg=18)
            k.cyl(1.6, 0.2, 16, M["metal"], hand, 0, -12, 0, seg=14)
            for i in range(6):
                k.tor(2.6 - i * 0.35, 0.35, M["armor2"], hand, 0, -5.5 - i * 2.2, 0, PI / 2 + 0.25)
            k.sock(f"MUZZLE_{S}", hand, 0, -20, 0, PI / 2)
        else:
            autocannon(k, hand, S, 15, 1.35)
    walker_extras(k, hipY, footH, 0.8, 0.06)


# ----------------------------------------------------------------------------- 8 MULE

def mule(k):
    """Mech 8, after the block-out (a mine haul truck, armed): the heaviest legs of the round, a tall cab front with
    the Excavator cab in it (its front drops as a ramp below the sill) under a canopy roof on posts, a dump bed tipped
    on its back on two hoist rams, a rocket pod on each shoulder, two piston fists for punching."""
    M = k.M
    segs, footH = [[14, -0.62], [15, 1.12], [7, -0.5]], 3.2
    body, hipY = _body(k, segs, footH, dict(t=1.3), 9.4, 22)
    chest = _waist(k, body, 6.6)
    at = (0, 3.4, 0.8)
    cp, cav, out, hull = chest_bay(k, chest, "cab", at, "inner", MULE_TORSO, swing={"INNER_ramp_1": 2.65})
    hull["cp"] = cp
    U, Yb, Yt = 1.0 / k.s, hull["Yb"], hull["Yt"]
    frame_contours(k, hull, Yb, Yt)
    ry = (hull["Ytop"] + 0.5) * U
    k.pl(9.0 * U, 0.6 * U, 5.4 * U, M["armor"], cp, 0, ry, -0.4 * U)            # the canopy roof, over the cab
    for s in (1, -1):
        k.box(0.5 * U, 0.9 * U, 0.5 * U, M["frame"], cp, s * 3.6 * U, ry - 0.7 * U, 1.6 * U)
        k.box(0.5 * U, 0.9 * U, 0.5 * U, M["frame"], cp, s * 3.6 * U, ry - 0.7 * U, -2.4 * U)
        floodlight(k, cp, s * 2.6 * U, ry - 0.6 * U, 2.0 * U, -0.2)
    k.box(8.4 * U, 0.3 * U, 0.2 * U, M["hazard"], cp, 0, ry - 0.2 * U, 2.3 * U)
    # the dump bed, tipped on its back: floor, two sides and a headboard, on two hoist rams
    bed = k.g(cp, 0, (hull["Ytop"] - 0.6) * U, -3.4 * U, -0.35, 0, 0)
    k.pl(8.0 * U, 0.4 * U, 7.0 * U, M["red"], bed, 0, 0, -3.6 * U)          # its front edge behind the headboard's face
    for s in (1, -1):
        k.pl(0.4 * U, 2.8 * U, 7.2 * U, M["red"], bed, s * 4.1 * U, 1.4 * U, -3.4 * U)
    k.pl(8.0 * U, 3.4 * U, 0.4 * U, M["red"], bed, 0, 1.7 * U, 0)
    for s in (1, -1):
        ram_pair(k, cp, (s * 2.6 * U, -1.0 * U, -3.0 * U), (s * 2.6 * U, (hull["Ytop"] - 1.2) * U, -6.0 * U), 1.2)
    back_jets(k, cp, (1.6 * U, -1.6 * U), -1.2 * U, -3.4 * U, 2.4)
    walker_head(k, chest, 2.2 * U, at[1] + ry, at[2] - 0.4 * U, "mast", 1.0)
    sy, sz = at[1] + 2.0 * U, at[2] - 0.6 * U
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb, hand = walker_arm(k, chest, S, s, s * (out["hw"] + 3.4), sy, sz, 1.2, 7.0)
        fist(k, elb, hand, S, 1.2)
        rk = k.g(sh_, s * 0.6, 8.4, -1.0)                               # the rocket pod: a drum of tubes
        k.cyl(3.0, 3.0, 7, M["armor"], rk, 0, 0, 0, PI / 2, seg=18)
        for i in range(7):
            a_ = i / 6 * 2 * PI
            dx, dy = (math.cos(a_) * 1.7, math.sin(a_) * 1.7) if i < 6 else (0, 0)
            k.cyl(0.6, 0.6, 0.4, M["ink"], rk, dx, dy, 3.6, PI / 2, seg=8)
            k.cyl(0.35, 0.35, 0.3, M["tip"], rk, dx, dy, 3.75, PI / 2, seg=6)
        k.box(6.2, 0.8, 1.0, M["hazard"], rk, 0, 3.0, 0)
        k.sock(f"ROCKET_{S}", rk, 0, 0, 4.0, PI / 2)
    walker_extras(k, hipY, footH, 0.75, 0.05)


# ----------------------------------------------------------------------------- 11 PICKET and 12 SAPPER

def humanoid(k, variant, torso, swing, look_t=1.0, a=1.0):
    """The two humanoids' shared body (owner, 2026-10-24: a Gundam's build, none of its badges): long legs on a human
    knee with knee guards, a narrow waist with front and side skirts, a broad chest with the cockpit in it and slatted
    intakes either side, a backpack with two thrusters and exhaust stacks, a visor helmet with one swept antenna.
    Returns what the two dress differently."""
    M = k.M
    segs, footH = [[16, -0.18], [17, 0.42]], 3.4
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.cyl(2.8 * look_t, 2.8 * look_t, 14 * look_t, M["dark"], body, 0, 0, 0, 0, 0, PI / 2, 18)
    k.pl(9 * look_t, 5.0, 6.6, M["frame"], body, 0, 1.0, 0)
    for s in (1, -1):                                                  # front and side skirts
        k.pl(5.0 * look_t, 6.4, 0.9, M["armor"], body, s * 2.7 * look_t, -1.2, 3.8, -0.22)
        k.box(4.4 * look_t, 0.6, 0.3, M["hazard"], body, s * 2.7 * look_t, -3.9, 4.6, -0.22)
        k.pl(0.9, 6.0, 6.2, M["armor"], body, s * 6.6 * look_t, -0.6, 0, 0, 0, s * 0.18)
    k.pl(9 * look_t, 5.6, 0.9, M["armor"], body, 0, -0.8, -3.8, 0.2)
    walker_legs(k, body, 4.2 * look_t, segs, dict(t=look_t, shin="slab", knee="pad", rams=False))
    waist = k.J("torso_twist", body, 0, 3.4, 0)
    k.pl(7 * look_t, 3, 5.4, M["frame"], waist, 0, 0.6, 0)
    chest = k.J("torso", waist, 0, 2.4, 0)
    at = (0, 3.2, 0.4)
    cp, cav, out, hull = chest_bay(k, chest, variant, at, "inner", torso, swing=swing)
    hull["cp"] = cp
    U, Yb, Yt = 1.0 / k.s, hull["Yb"], hull["Yt"]
    frame_contours(k, hull, Yb, Yt)
    for s in (1, -1):                                                  # the slatted intakes either side of the cockpit
        k.vent(cp, 1.4 * U, 1.2 * U, *hull["at"](s * 0.95, 2.6, 0.05), 0, s * 0.95, 4)
    walker_head(k, chest, 0, at[1] + (hull["Ytop"] - 0.1) * U, at[2] - 0.5 * U, "visor", 0.9)
    pack(k, cp, [(0.0, chamfered(1.8, -2.4, -3.6)), (1.6, chamfered(2.1, -2.4, -3.9, 0.35)), (3.0, chamfered(1.7, -2.4, -3.6))],
         M["armor2"], -3.0)
    back_jets(k, cp, (1.2 * U, -1.2 * U), 0.2 * U, -3.4 * U, 2.0)
    for s in (1, -1):
        k.cyl(0.8, 0.9, 6, M["dark"], cp, s * 1.5 * U, 3.2 * U + 2.6, -3.2 * U)
    sy, sz = at[1] + 2.4 * U, at[2] - 0.6 * U
    arms = {}
    for S, s in (("L", 1), ("R", -1)):
        sh_, elb, hand = walker_arm(k, chest, S, s, s * (out["hw"] + 3.2 * a), sy, sz, a, 8.5)
        k.sph(5.6 * a, M["armor"], sh_, s * 1.0 * a, 2.0 * a, 0, 1.1, 0.9, 1.15)   # the big rounded shoulder pad
        k.box(6.0 * a, 0.8, 7.0 * a, M["armor2"], sh_, s * 1.0 * a, -2.4 * a, 0)
        arms[s] = (sh_, elb, hand, S)
    return hipY, footH, cp, hull, out, arms, chest, at


def picket(k):
    """Mech 11, after the block-out (the RX-79[G]'s build, in field colours): the Panorama in its chest, a 100 mm
    machine gun with a box magazine in the right hand, a tall shield on the left forearm, a radio whip on the backpack."""
    hipY, footH, cp, hull, out, arms, chest, at = humanoid(k, "panorama", PICKET_TORSO, {"INNER_upper_1": -2.75, "INNER_lower_1": 2.7})
    U = 1.0 / k.s
    sh_, elb, hand, S = arms[-1]
    rifle(k, hand, S, 15, 1.1)
    sh_, elb, hand, S = arms[1]
    shield(k, elb, 1, 1.0)
    k.box(1.6, 3.6, 1.6, k.M["frame"], cp, 1.5 * U, 3.4 * U, -3.9 * U)
    k.ant(cp, 1.5 * U, 3.4 * U + 1.8, -3.9 * U, 16, -0.15, 0.05)
    walker_extras(k, hipY, footH, 0.9, 0.05)


def sapper(k):
    """Mech 12, after the block-out (the full-armour idea): the same humanoid, heavier, the Titan in its chest, bolt-on
    armour on chest and shoulders, a long cannon over the right shoulder, a missile pod on the left forearm and a rifle
    in the right hand."""
    hipY, footH, cp, hull, out, arms, chest, at = humanoid(k, "titan", SAPPER_TORSO, {"INNER_lower_1": 2.7}, 1.15, 1.12)
    M, U = k.M, 1.0 / k.s
    for s in (1, -1):                                                  # bolt-on plates on the chest's flanks
        x, y, z = hull["at"](s * 2.0, 1.2, 0.08)
        k.pl(1.0, 4.4, 5.2, M["armor2"], cp, x, y, z, 0, s * 2.0, 0)
    sh_, elb, hand, S = arms[-1]
    rifle(k, hand, S, 18, 1.2)
    k.pl(7.4, 1.2, 8.4, M["armor2"], sh_, -1.2, 5.2, 0)                 # a pauldron plate over the pad
    sh_, elb, hand, S = arms[1]
    missile_rack(k, elb, 3.8, -4.0, -0.8, 2, 3, 1.8, -PI / 2, 0, "MISSILE_L")
    k.pl(7.4, 1.2, 8.4, M["armor2"], sh_, 1.2, 5.2, 0)
    # the shoulder cannon: a breech on the back, the barrel over the right shoulder
    g = k.g(cp, -2.2 * U, 3.6 * U, -3.0 * U)
    k.pl(4.2, 4.0, 7, M["armor"], g, 0, 0, 0)
    k.cyl(1.0, 1.1, 30, M["metal"], g, 0, 1.6, 15, PI / 2 - 0.05, seg=14)
    k.cyl(1.7, 1.7, 9, M["dark"], g, 0, 1.4, 4.9, PI / 2 - 0.05, seg=14)
    k.cyl(1.9, 1.5, 3, M["frame"], g, 0, 2.4, 30, PI / 2 - 0.05, seg=14)
    k.sock("CANNON_R", g, 0, 2.5, 32, PI / 2)
    walker_extras(k, hipY, footH, 0.8, 0.05)


def plate_face(k, j, W, H, y_dir, z_off, glow_slit=False, livery=True):
    """A hatch plate's face hanging from its hinge along y_dir (-1 down, +1 up), with Mining Mike's
    dressing: livery bands, a hazard edge, bolts, a vent grille, and optionally the visor slit."""
    M = k.M
    yc = y_dir * H / 2
    k.pl(W, H, 3.2, M["armor"], j, 0, yc, z_off)
    k.pl(W * 0.86, H * 0.8, 1.2, M["armor2"], j, 0, yc, z_off + 2.0)
    if livery:
        k.stripes(j, W * 0.7, 0, yc + y_dir * H * 0.2, z_off + 2.8, 0, hb=1.4, gap=0.7)
    k.box(W * 0.9, 1.0, 0.4, M["hazard"], j, 0, y_dir * (H - 0.8), z_off + 1.8)
    k.bolts(j, 5, -W * 0.4, y_dir * (H - 2.2), z_off + 2.8, W * 0.2, 0.4)
    if glow_slit:
        k.box(W * 0.62, 1.2, 0.6, M["glow"], j, 0, yc - y_dir * H * 0.12, z_off + 2.9)
        k.halo(j, 0, yc - y_dir * H * 0.12, z_off + 3.6, 16, 0.7)


# ----------------------------------------------------------------------------- run

DESIGNS = {
    # id: (builder, height m, glow, livery, core, light plate, name, card)
    "foreman": (foreman, 24.0, 0xffb02e, 0x2f6fe0, 0xffb347, False, "Foreman",
                dict(legs="digitigrade, three segments, double hydraulic rams, track-pad feet",
                     cockpit="Panorama: seated behind one wide screen and square side screens, in the chamfered prow",
                     opening="the Panorama clamshell, chest and cockpit as one: the front under the screens drops into a step, then the front over them swings up and out on two rams with the dash",
                     weapons="belt-fed boom cannon (L), hydraulic breaker (R)",
                     note="Original, after an excavator: slewing ring, broad shoulders with beacons, an engine pack and its stack. Recommended.")),
    "ironhorse": (ironhorse, 22.0, 0xffc860, 0x2f6fe0, 0xff8a3a, False, "Ironhorse",
                  dict(legs="digitigrade, drive-wheel knees with coupling rods, cow-catcher shins",
                       cockpit="Excavator cab: seated, floor-to-roof screens in two rows and one in the floor",
                       opening="the dogs swing off, the smokebox front drops forward as a ramp with the cab's front on it",
                       weapons="buffer ram (L), steam cannon (R)",
                       note="Original, after a steam locomotive: a banded boiler torso, smokestack, dome, headlamp, firebox pack.")),
    "harrow": (harrow, 24.0, 0x9cff8a, 0x2f6fe0, 0x8affc0, False, "Harrow",
               dict(legs="digitigrade, heavy slabs and rams, wheel feet", cockpit="Canopy: seated under a bubble in the hull's nose",
                    opening="the canopy and the hull's nose over it lift back as one",
                    weapons="autocannon pod (L), heavy cannon pod (R), a six-cell missile rack on each shoulder",
                    note="Block-out 1, built in detail at 24 m (owner, 2026-10-24: \"1,2,6,8,11,12 start building them\").")),
    "roustabout": (roustabout, 24.0, 0xffb84a, 0x2f6fe0, 0xffb347, False, "Roustabout",
                   dict(legs="digitigrade, stocky, piston shins", cockpit="Titan: seated, the upper front rises and tilts back",
                        opening="the chest over the screens rises and tilts back with the turret's collar, the front under them drops below the sill",
                        weapons="piston fist (L), heavy rifle with a box magazine (R)",
                        note="Block-out 2, built in detail at 24 m.")),
    "driller": (driller, 24.0, 0x7fd8ff, 0x2f6fe0, 0x8fd0ff, False, "Driller",
                dict(legs="digitigrade, lattice shins", cockpit="Panorama, as the Foreman's",
                     opening="the Panorama clamshell, chest and cockpit as one: the lower flap folds down below the sill, the hood swings up",
                     weapons="heavy autocannon (L), auger drill (R)",
                     note="Block-out 6, built in detail at 24 m, armed: a derrick on its back.")),
    "mule": (mule, 24.0, 0xffc040, 0x2f6fe0, 0xff8a3a, False, "Mule",
             dict(legs="digitigrade, the heaviest slabs", cockpit="Excavator cab, under a canopy roof",
                  opening="the cab's front drops below the sill as a flap",
                  weapons="two piston fists, a rocket pod on each shoulder",
                  note="Block-out 8, built in detail at 24 m: a haul truck's dump bed on its back.")),
    "picket": (picket, 24.0, 0xffa040, 0x2f6fe0, 0xffb347, False, "Picket",
               dict(legs="human knee, knee guards, skirts", cockpit="Panorama, in the chest",
                    opening="the Panorama clamshell: the lower flap folds down, the hood swings up",
                    weapons="machine gun with a box magazine (R), tall shield (L)",
                    note="Block-out 11, built in detail at 24 m: a Gundam's build, none of its badges.")),
    "sapper": (sapper, 24.0, 0x70e0ff, 0x2f6fe0, 0x70e0ff, False, "Sapper",
               dict(legs="human knee, heavier, knee guards, skirts", cockpit="Titan, in the chest",
                    opening="the chest over the screens rises and tilts back, the front under them drops below the sill",
                    weapons="rifle (R), missile pod on the forearm (L), cannon over the right shoulder",
                    note="Block-out 12, built in detail at 24 m: bolt-on armour.")),
    "breeze": (breeze, 12.0, 0x59f2ff, 0x2f6fe0, 0x59f2ff, True, "Breeze",
               dict(legs="reverse knee, three segments", cockpit="the pod at the front of the chest",
                    weapons="rail lance (L), gatling (R)", note="Retired (owner, 2026-10-24: \"Breeze me h sucks\"); kept while the game still loads it.")),
    "bulwark": (bulwark, 12.0, 0x59f2ff, 0x3478f0, 0xffb347, False, "Bulwark",
                dict(legs="human knee, two segments", cockpit="armoured head, visor band",
                     weapons="slab shield and siege fist (L), breaker cannon (R)", note="Mining Mike's Uber mech, ported: the reference.")),
    "ravager": (ravager, 12.5, 0xff5a2a, 0xcc1a1f, 0xff8a3a, False, "Ravager",
                dict(legs="reverse knee, three segments", cockpit="low head, visor under three eyes",
                     weapons="twin rail lances (L and R)", note="Mining Mike's rival Uber mech, ported: the reference.")),
}

UNIT_NUMBERS = dict(harrow="01", roustabout="02", driller="06", mule="08", picket="11", sapper="12",
                    foreman="31", ironhorse="33")
PAINTED = ("foreman", "ironhorse", "harrow", "roustabout", "driller", "mule", "picket", "sapper")


def build(key, s):
    fn, height, glow, livery, core, light, name, card = DESIGNS[key]
    K.clear_scene()
    # the round picked from the block-outs wears their schemes (owner, 2026-10-24: "use the cool color scheme from your mockup")
    mats = K.make_mats(f"mech_{key}", glow, livery, core, light, PALETTES.get(key) if key in PAINTED else None)
    k = Kit(mats, s, key)
    k.extras.update(glow=glow, core=core)
    if key in PAINTED:                               # its markings: the unit number, light or dark for its scheme
        P = PALETTES[key]["BODY"]
        k.mark = dict(number=UNIT_NUMBERS[key], dark=(0.3 * P[0] + 0.59 * P[1] + 0.11 * P[2]) > 0.28)
    fn(k)
    K.finish(k)
    return k


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    os.makedirs(OUT, exist_ok=True)
    for key in argv or list(DESIGNS):
        fn, height, glow, livery, core, light, name, card = DESIGNS[key]
        s = 1.0
        for _ in range(3):      # the bay is in metres, the rest in kit units: iterate the scale to the height
            build(key, s)
            _, b = K.measure()
            s *= height / (b["y"][1] - b["y"][0])
        k = build(key, s)
        extras = dict(k.extras, rest={n: r for n, r in k.rest.items()}, scale=s)
        meta = dict(id=key, name=name, generator="tools/blender/build_mechs.py", status="design candidate",
                    style="Mining Mike boss kit (tank_bosses_kit.js at 3336b88)",
                    cockpit_variant=k.extras.get("cockpit", {}).get("variant"), **card)
        K.export(os.path.join(OUT, f"mech_{key}.glb"), MANIFEST, meta, extras)


main()
