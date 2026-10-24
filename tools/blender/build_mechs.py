"""Build the Crab Breeze mech design candidates as glb files on the rig contract.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_mechs.py [-- id ...]

Writes assets/models/mech_<id>.glb and assets/models/mechs.json (the measured manifest). Built with
mech_kit.py, the port of Mining Mike's boss mech kit, in that kit's units; each mech is built once to
measure it and again at the scale that makes it its height in metres.

  bulwark  Mining Mike's Uber mech "Bulwark", ported: planted and square, breaker cannon, slab shield.
           The back cradle now holds the flight jets; the head is the cockpit, a visor across it.
  ravager  Mining Mike's Uber mech "Ravager", ported: hunched on reverse knees, twin rail lances,
           a crest of blades. The low three-eyed head is the cockpit.
  warden   heavy assault biped, the wall cockpit in its chest behind a clamshell (the recommendation).
  lancer   fast reverse-knee skirmisher, the jockey cockpit behind petal doors.
  atlas    tall siege gunner, the harness cockpit behind a drawbridge front.
           The three are the owner's "Follow design philosophy of boss mechs from mining mike. The cockpit is
           part of the mech, inside of it with the armor opening up at the chest" (2026-10-24); the chest
           hardware is mech_boarding.py.
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
from mech_kit import Kit, Node, leg_height  # noqa: E402

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
# The owner, 2026-10-24: "Breeze me h sucks", "Follow design philosophy of boss mechs from mining mike.
# The cockpit is part of the mech, inside of it with the armor opening up at the chest". Three chassis in
# the boss kit's vocabulary (a readable role silhouette, plated armour over a frame, livery bands, hazard
# edges, vents, hoses, rams, a glowing core, weapons for hands), each carrying one of the cockpit interiors
# (build_cockpit.py --variant) in a bay in its chest, closed by armour that opens to board
# (mech_boarding.py).


def cannon_arm(k, elb, S, s, scale=1.0):
    """Skarn's breach cannon, scaled: a housing with a core and vents, then the barrel on hand_<S>."""
    M, c = k.M, scale
    k.cyl(5 * c, 5 * c, 11 * c, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
    hs = k.pl(17 * c, 26 * c, 17 * c, M["armor"], elb, 0, -13 * c, 0)
    k.stripes(hs, 15 * c, 0, 5 * c, 8.7 * c, 0, hb=1.5 * c, gap=0.8 * c)
    k.core(elb, 0, -13 * c, 9.8 * c, 3.2 * c)
    for t in (1, -1):
        k.vent(elb, 7 * c, 9 * c, t * 8.8 * c, -13 * c, 0, 0, t * PI / 2, 3)
    k.box(20 * c, 4 * c, 20 * c, M["frame"], elb, 0, -27 * c, 0)
    for i in range(4):
        k.tor(10 * c, 0.7 * c, M["glowDim"], elb, 0, (-4 - i * 5) * c, 0, PI / 2)
    barrel = k.J(f"hand_{S}", elb, 0, -29 * c, 0)
    k.cyl(3.8 * c, 4.3 * c, 32 * c, M["metal"], barrel, 0, -16 * c, 0, 0, 0, 0, 16)
    for y in (-5, -14, -23):
        k.cyl(5 * c, 5 * c, 2.2 * c, M["frame"], barrel, 0, y * c, 0)
    k.box(10 * c, 8 * c, 10 * c, M["dark"], barrel, 0, -34 * c, 0)
    for yy in (-32.5, -35.5):
        for t in (1, -1):
            k.box(0.5 * c, 1.2 * c, 7 * c, M["ink"], barrel, t * 5.1 * c, yy * c, 0)
    k.cyl(2.6 * c, 2.6 * c, 0.4 * c, M["ink"], barrel, 0, -38.1 * c, 0)
    k.sock(f"MUZZLE_{S}", barrel, 0, -38.5 * c, 0, PI / 2)


def shield_arm(k, elb, S, s):
    """Skarn's tower shield on the forearm (hand_<S> carries it)."""
    M = k.M
    k.cyl(4.8, 4.8, 10, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
    k.pl(13, 20, 14, M["armor2"], elb, 0, -10, 1)
    k.box(15, 9, 15, M["frame"], elb, 0, -23, 1)
    sh = k.J(f"hand_{S}", elb, s * 9, -12, 1)
    slab = k.pl(4, 40, 28, M["armor"], sh, 0, 0, 2, 0, 0, 0, 0.9)
    k.stripes(slab, 26, 2.3, 8, 2, PI / 2, hb=1.8, gap=0.9)
    k.box(0.6, 6, 24, M["hazard"], sh, s * 2.3, -15, 2)
    for t in (1, -1):
        k.box(5, 2, 30, M["frame"], sh, 0, t * 20.5, 2)
    k.box(1, 1.4, 12, M["glow"], sh, s * 2.6, 2, 2)
    k.halo(sh, s * 3.4, 2, 2, 10, 0.5)
    k.bolts(sh, 4, -0.5, -6, 9.1, 2, 0.5)
    for z in (-8, 2, 12):
        k.cyl(0, 1.8, 6, M["metal"], sh, 0, -23.5, z, PI)
    k.sock(f"MUZZLE_{S}", sh, 0, 0, 16, 0)


def missile_box(k, p, x, y, z, rows, cols, ry=0.0, name="POD"):
    """Rook's launcher box: a plated box with a grid of tubes facing +z and a hazard lip."""
    M = k.M
    g = k.g(p, x, y, z, 0, ry, 0)
    W, H = cols * 3.2 + 2, rows * 3.2 + 2
    k.pl(W, H, 9, M["armor2"], g, 0, 0, 0)
    k.box(W + 0.6, 1.2, 9.4, M["frame"], g, 0, H / 2 + 0.3, 0)
    for r in range(rows):
        for c in range(cols):
            tx, ty = -W / 2 + 2.6 + c * 3.2, -H / 2 + 2.6 + r * 3.2
            k.cyl(1.2, 1.2, 0.6, M["ink"], g, tx, ty, 4.6, PI / 2, seg=10)
            k.cyl(0.55, 0.55, 0.5, M["tip"], g, tx, ty, 4.75, PI / 2, seg=8)
    k.box(W * 0.8, 1.0, 0.4, M["hazard"], g, 0, -H / 2 - 0.2, 4.4)
    k.sock(f"{name}_muzzle", g, 0, 0, 5.5, 0)


def head_sensor(k, p, x, y, z, w=12, wide=True):
    """The sensor head on the chest: the mech's eyes, which the cockpit's screens show (survey M8)."""
    M = k.M
    h = k.J("head", p, x, y, z)
    k.pl(w, 6.5, w * 0.8, M["armor"], h, 0, 3, 0)
    k.pl(w + 1, 2.2, 5, M["armor2"], h, 0, 6.2, 1.6, 0.3)
    if wide:
        k.box(w * 0.72, 1.3, 0.6, M["glow"], h, 0, 3.2, w * 0.4 + 0.3)
        k.halo(h, 0, 3.2, w * 0.4 + 1.0, 12, 0.6)
    else:
        for xx in (-2.2, 2.2):
            k.sph(1.3, M["lens"], h, xx, 3.4, w * 0.4)
            k.halo(h, xx, 3.4, w * 0.4 + 1, 6, 0.7)
    for t in (1, -1):
        k.pl(2.2, 6, w * 0.7, M["armor2"], h, t * (w / 2 + 0.6), 2.6, 0.4, 0, t * 0.12, 0)
    k.box(w * 0.5, 2, 1.5, M["frame"], h, 0, 0.4, w * 0.4)
    k.lamp(h, w * 0.32, 5.2, w * 0.34, 0.55, 4)
    k.ant(h, -w * 0.36, 6.4, -2.5, 7, -0.15, 0.25)
    k.sock("SENSOR_head", h, 0, 3.2, w * 0.4 + 0.6)
    return h


def chest_bay(k, chest, variant, at, out_min):
    """The bay and the shell for a chest: `at` is the bay floor centre in the chest joint (kit units),
    `out_min` the smallest outer box (hw, y0, y1, z0, z1) in the cockpit joint's frame; the shell grows to
    keep at least 6 units of armour round the bay."""
    cp, cav = MB.bay(k, chest, variant, *at)
    out = dict(hw=max(out_min["hw"], cav["hw"] + 6), y0=min(out_min["y0"], cav["y0"] - 5),
               y1=max(out_min["y1"], cav["y1"] + 5), z0=min(out_min["z0"], cav["z0"] - 6), z1=max(out_min["z1"], cav["z1"]))
    MB.shell(k, cp, cav, out)
    MB.ladder(k, cp, cav)
    MB.vent(k, cp, 0, cav["y0"] + 0.3 / k.s, cav["z1"] - 0.2 / k.s, (0, -0.25, 1), "sill", "mist", 1.4)
    for s in (1, -1):
        MB.vent(k, cp, s * cav["hw"] * 0.6, cav["y0"] + 0.2 / k.s, cav["z1"] - 0.3 / k.s, (s * 0.3, -0.2, 1), "sill", "mist", 1.0)
    return cp, cav, out


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


# ----------------------------------------------------------------------------- WARDEN

def warden(k):
    """Heavy assault biped in Skarn's and Bulwark's line: planted on human knees, a breach cannon on the
    right, a tower shield and a shoulder missile box on the left, flight jets in a back cradle. The chest
    is the cockpit bay (wall: seated, a flat screen wall); it opens like a clamshell: the cheek guards
    swing out, the upper plate lifts on two rams with the visor slit on it, the lower plate drops forward
    into a platform, and the chain ladder falls to the ground."""
    M = k.M
    segs, footH = [[24, -0.26], [25, 0.52]], 5.5
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(36, 10, 22, M["frame"], body, 0, 0, 0)
    k.cyl(5.8, 5.8, 42, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pl(20, 10, 4, M["armor"], body, 0, -2.5, 12, -0.2)
    k.box(12, 2, 0.5, M["hazard"], body, 0, -6.2, 13.7, -0.2)
    k.core(body, 0, 0, 14.6, 3.2)
    for s in (1, -1):
        k.pl(5, 13, 17, M["armor2"], body, s * 20.6, -4, 0, 0, 0, s * 0.15)
        k.bolts(body, 3, -4, 3.4, -11.2, 4, 0.6)

    def seg0(k, g, s, L):
        k.cyl(6.4, 6.4, 11, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(10, 22, 10, M["frame"], g, 0, -12, 0)
        pl = k.pl(15, 19, 5, M["armor"], g, 0, -11, 6, 0.05)
        k.stripes(pl, 13, 0, 2.5, 2.8, 0, hb=1.7, gap=0.9)
        k.pl(5, 17, 13, M["armor2"], g, s * 7.5, -11, 0)
        k.vent(g, 5, 7, s * 10.1, -11, 0, 0, s * PI / 2, 3)

    def seg1(k, g, s, L):
        k.cyl(5.4, 5.4, 14, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.pl(14, 11, 6, M["armor2"], g, 0, 1, 6.5, -0.35)
        k.box(10, 24, 10, M["frame"], g, 0, -12.5, -1)
        gr = k.pl(15, 19, 5, M["armor"], g, 0, -13.5, 6, -0.05)
        k.box(12, 2.4, 0.5, M["hazard"], gr, 0, -7, 2.6)
        k.pl(11, 15, 6, M["armor2"], g, 0, -12, -6.5, 0.1)
        k.vent(g, 6, 8, 0, -12, -9.8, 0, PI, 3)

    def foot(k, g, s, S):
        k.sph(4.8, M["dark"], g, 0, 0, 0)
        k.pl(19, 4.8, 25, M["armor"], g, 0, -3.0, 3)
        toe = k.J(f"toe_{S}", g, 0, -5.3, 9.5)
        for x in (-6.4, 0, 6.4):
            k.px([[0, 0], [6, 0], [6, 1.5], [1.5, 3.8], [0, 3.8]], 5.4, M["armor2"], toe, x, 0, 5.5)
        k.box(19.6, 1.2, 25.6, M["rubber"], g, 0, -4.9, 3)
        k.pl(9, 4, 6, M["frame"], g, 0, -2.8, -10.5)
        k.box(15, 1.8, 0.5, M["hazard"], g, 0, -2.2, 16)
        heel_jet(k, g, -14, 1.9)
        k.sock(f"WHEEL_{S}", g, 0, -4.9, 3)

    k.legs(body, 16, segs, [seg0, seg1], foot)
    k.piston(N("thigh_L"), [0, -3, -6], N("shin_L"), [0, -6, -6], 1.9)
    k.piston(N("thigh_R"), [0, -3, -6], N("shin_R"), [0, -6, -6], 1.9)
    waist = k.J("torso_twist", body, 0, 7.5, -1)
    k.cyl(11, 12.5, 6, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(3):
        k.tor(12.2, 0.8, M["frame"], waist, 0, -1.8 + i * 1.8, 0, PI / 2)
    chest = k.J("torso", waist, 0, 3.5, 0)
    # the bay: floor 5 units up the chest, its front at the chest's face
    cp, cav, out = chest_bay(k, chest, "wall", (0, 5, 2), dict(hw=24, y0=-6, y1=27, z0=-21, z1=12))
    hw, y0, y1, z1 = cav["hw"], cav["y0"], cav["y1"], out["z1"]
    k.pl(2 * out["hw"] + 6, 5, out["z1"] - out["z0"] + 2, M["armor2"], cp, 0, out["y1"] + 2.5, (out["z0"] + out["z1"]) / 2 - 1)
    k.bolts(cp, 9, -out["hw"] * 0.8, out["y1"] + 2.5, z1 + 0.6, out["hw"] * 0.2, 0.6)
    for s in (1, -1):
        k.vent(cp, 12, 12, s * (out["hw"] + 0.3), (y0 + y1) / 2, (out["z0"] + z1) / 2, 0, s * PI / 2, 5)
        k.stripes(cp, 14, s * (out["hw"] - 9), out["y1"] - 3, z1 + 0.4, 0, hb=1.6, gap=0.8)
    # the clamshell: the split line 42 % of the way up the bay
    ysplit = y0 + 0.42 * (y1 - y0)
    up = MB.hatch(k, "CHEST_upper", cp, 0, y1 + 1.6, z1 + 2.2, ("x", -1.95), 1.2, 1.7)
    plate_face(k, up, 2 * hw + 5, y1 + 1.6 - ysplit, -1, 0, glow_slit=True)
    k.pz([[-hw - 2.5, 0], [hw + 2.5, 0], [hw - 1, 4], [-hw + 1, 4]], 4, M["armor2"], up, 0, -1, 3.6, -PI / 2)
    lo = MB.hatch(k, "CHEST_lower", cp, 0, y0 - 1.6, z1 + 2.2, ("x", 1.62), 1.6, 1.4)
    plate_face(k, lo, 2 * hw + 5, ysplit - (y0 - 1.6), 1, 0, livery=False)
    k.box((2 * hw + 4) * 0.8, 0.6, 3, M["rubber"], lo, 0, (ysplit - y0) * 0.5, 4.2)       # the tread it becomes
    for S, s in (("L", 1), ("R", -1)):
        j = MB.hatch(k, f"CHEST_side_{S}", cp, s * (hw + 4.5), (y0 + y1) / 2, z1 + 5.6, ("y", s * 1.35), 0.6, 1.1)
        k.pl(7, y1 - y0 + 4, 2.6, M["armor"], j, -s * 3.2, 0, 0)
        k.box(1.0, y1 - y0, 0.4, M["hazard"], j, -s * 6.2, 0, 1.4)
        k.cyl(1.2, 1.2, y1 - y0 + 4, M["frame"], j, 0, 0, 0)
        MB.ram(k, f"upper_{S}", N("cockpit"), [s * hw * 0.75, y1 + 3.5, z1 - 6], up, [s * hw * 0.75, -6, 1.5], 1.0)
        MB.ram(k, f"lower_{S}", N("cockpit"), [s * hw * 0.62, y0 - 3.5, z1 - 5], lo, [s * hw * 0.62, 6, 1.5], 0.9)
        for f in (0.25, 0.75):
            MB.vent(k, cp, s * (hw + 1.5), y0 + f * (y1 - y0), z1 + 1.5, (s, 0, 0.6), "seam")
        MB.vent(k, cp, s * hw * 0.5, ysplit, z1 + 2.5, (s * 0.4, 0.1, 1), "seam", rate=1.5)
        MB.vent(k, cp, s * hw * 0.85, y1 + 1.8, z1 + 1.0, (s * 0.2, 1, 0.3), "open", rate=1.2)
    head_sensor(k, chest, 0, 5 + out["y1"] + 5, 2 + (out["z0"] + z1) / 2 + 6, 13)
    jet_cradle(k, chest, 3, 26, 2 + out["z0"] - 2, 26)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(3, 3.6, 16, M["dark"], chest, s * 21, 5 + out["y1"] + 8, 2 + out["z0"] + 4)
        k.cyl(3.9, 3.9, 2, M["frame"], chest, s * 21, 5 + out["y1"] + 16, 2 + out["z0"] + 4)
        k.cyl(2.7, 2.7, 0.4, M["glowDim"], chest, s * 21, 5 + out["y1"] + 17.1, 2 + out["z0"] + 4)
    k.hose(chest, [[-18, 6, -16], [-26, 10, -8], [-27, 16, 2]], 1.2)
    k.hose(chest, [[18, 6, -16], [26, 10, -8], [27, 16, 2]], 1.2)
    sy = 5 + out["y1"] - 6
    # left: the tower shield, the missile box on the pauldron; right: the breach cannon
    for S, s in (("L", 1), ("R", -1)):
        sh, elb = contract_arm(k, chest, S, s * (out["hw"] + 6), sy, 2, -0.08, -19, -0.5 if s > 0 else -1.42)
        k.sph(7, M["frame"], sh, 0, 0, 0)
        p = k.pl(19, 13, 26, M["red"], sh, s * 3, 5.5, 0, 0, 0, s * 0.25)
        k.box(1.5, 11, 22, M["hazard"], p, s * 9.7, 0, 0)
        k.box(8.5, 17, 8.5, M["frame"], sh, 0, -10, 0)
        if s > 0:
            shield_arm(k, elb, S, s)
            missile_box(k, sh, s * 3, 15, -2, 3, 4, 0.0, "POD_L")
        else:
            cannon_arm(k, elb, S, s)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.85, A=0.34, lift=0.45, bob=2.4 * k.s, roll=0.035, yaw=0.05, arm=0.08, lean=0.06),
                    idle=dict(scan=0.32, scanPeriod=3.1, breathHz=0.85))


# ----------------------------------------------------------------------------- LANCER

def lancer(k):
    """Fast skirmisher in Brisk's and Ravager's line: reverse knees, a rail lance on the left, twin SMGs
    on the right, swept blade fins, big jet nacelles. The chest is a forward wedge round a slim bay (jockey:
    the pilot leans forward on a saddle). It opens in petals: the brow lifts, the two front doors swing
    apart, the step slides out and drops, the ladder falls."""
    M = k.M
    segs, footH = [[20, -0.55], [22, 1.05], [12, -0.62]], 4.2
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(28, 8, 20, M["frame"], body, 0, 0, 0)
    k.cyl(4.6, 4.6, 32, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pz([[-8, -4], [8, -4], [5, 4], [-5, 4]], 4, M["armor"], body, 0, -2, 11, -0.25)
    k.core(body, 0, -1.5, 13.2, 2.4)
    for s in (1, -1):
        k.pl(4, 10, 16, M["armor2"], body, s * 16, -3, -1, 0, 0, s * 0.2)

    def seg0(k, g, s, L):
        k.cyl(5, 5, 8.5, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(7, 19, 7, M["frame"], g, 0, -10, 0)
        pl = k.pl(11, 16, 4.5, M["armor"], g, 0, -9, 4.6, 0.04)
        k.stripes(pl, 10, 0, 2, 2.5, 0, hb=1.3, gap=0.7)
        k.pl(3.5, 14, 10, M["armor2"], g, s * 5.6, -9.5, 0)

    def seg1(k, g, s, L):
        k.cyl(4.2, 4.2, 10, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(6, 22, 6, M["frame"], g, 0, -11, 0)
        k.pl(9, 19, 3.5, M["armor"], g, 0, -10.5, -4.2, -0.04)
        k.pz([[0, 0], [3, 0], [0, 9]], 2.8, M["metal"], g, 0, -2, -6, PI)
        k.vent(g, 4.5, 7, s * 3.8, -12, 0, 0, s * PI / 2, 3)

    def seg2(k, g, s, L):
        k.cyl(3.2, 3.2, 8, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(5, 12, 5, M["frame"], g, 0, -6, 0)
        k.pl(7, 9, 3, M["armor2"], g, 0, -6, 3.2)
        k.box(5.5, 1.4, 0.4, M["hazard"], g, 0, -9.5, 4.8)

    def foot(k, g, s, S):
        k.sph(3.2, M["dark"], g, 0, 0, 0)
        k.pl(11, 3.2, 9, M["armor"], g, 0, -2, 2)
        toe_j = k.J(f"toe_{S}", g, 0, -2.2, 5.5)
        for a in (-0.34, 0.34):
            toe = k.g(toe_j, 0, 0, 0, 0, a, 0)
            k.pl(4.4, 3, 11, M["armor2"], toe, 0, 0, 4.5)
            k.pz([[0, 0], [3.6, 0], [0, 4.5]], 2.2, M["metal"], toe, 0, -1.3, 10, -PI / 2)
        k.pl(4, 3, 8, M["armor2"], g, 0, -2.1, -6)
        k.box(11.4, 0.9, 9.4, M["rubber"], g, 0, -3.6, 2)
        k.cyl(2.2, 2.2, 3, M["rubber"], g, 0, -3.3, 2, 0, 0, PI / 2, 20)
        heel_jet(k, g, -9.5, 1.3)
        k.sock(f"WHEEL_{S}", g, 0, -3.3, 2)

    k.legs(body, 12, segs, [seg0, seg1, seg2], foot)
    k.piston(N("thigh_L"), [0, -2.5, 4.5], N("shin_L"), [0, -5.5, 3.5], 1.3)
    k.piston(N("thigh_R"), [0, -2.5, 4.5], N("shin_R"), [0, -5.5, 3.5], 1.3)
    waist = k.J("torso_twist", body, 0, 6, -1)
    k.cyl(8.5, 10, 5, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(2):
        k.tor(9.7, 0.7, M["frame"], waist, 0, -1 + i * 2, 0, PI / 2)
    chest = k.J("torso", waist, 0, 3, 1, 0.12)
    cp, cav, out = chest_bay(k, chest, "jockey", (0, 4, 4), dict(hw=17, y0=-5, y1=22, z0=-20, z1=11))
    hw, y0, y1, z1 = cav["hw"], cav["y0"], cav["y1"], out["z1"]
    # the wedge: a sloped roof plate running forward over the bay, the beak at its nose
    k.pz([[-out["hw"], 0], [out["hw"], 0], [out["hw"] - 5, 7], [-out["hw"] + 5, 7]], out["z1"] - out["z0"], M["armor"], cp,
         0, out["y1"], (out["z0"] + out["z1"]) / 2)
    for s in (1, -1):
        k.vent(cp, 9, 9, s * (out["hw"] + 0.3), (y0 + y1) / 2 - 2, (out["z0"] + z1) / 2, 0, s * PI / 2, 4)
        k.px([[0, 0], [12, 0], [20, 9], [12, 10]], 1.8, M["red"], cp, s * (out["hw"] - 2), out["y1"] + 5, out["z0"] + 2, 0, 0, s * 0.45)
    k.stripes(cp, 2 * out["hw"] - 10, 0, out["y1"] + 3.5, (out["z0"] + z1) / 2 + 2, 0, hb=1.3, gap=0.7, rx=-0.55)
    brow = MB.hatch(k, "CHEST_brow", cp, 0, y1 + 1.6, z1 - 1.0, ("x", -0.75), 0.4, 0.9)
    k.pl(2 * hw + 6, 3, 9, M["armor2"], brow, 0, -0.5, 3.5, 0.25)
    k.box(2 * hw * 0.7, 1.1, 0.5, M["glow"], brow, 0, -1.2, 8.2)
    k.halo(brow, 0, -1.2, 9, 14, 0.7)
    ystep = y0 + 0.3 * (y1 - y0)
    for S, s in (("L", 1), ("R", -1)):
        d = MB.hatch(k, f"CHEST_door_{S}", cp, s * (hw + 1.8), (ystep + y1) / 2, z1 + 2.2, ("y", s * 1.5), 1.0, 1.3)
        k.pl(hw + 1.8, y1 - ystep + 2, 3, M["armor"], d, -s * (hw + 1.8) / 2, 0, 0)
        k.stripes(d, hw * 0.8, -s * (hw + 1.8) / 2, (y1 - ystep) * 0.25, 1.8, 0, hb=1.2, gap=0.6)
        k.box(0.8, y1 - ystep, 0.4, M["hazard"], d, -s * (hw + 1.4), 0, 1.6)
        k.cyl(1.0, 1.0, y1 - ystep + 2, M["frame"], d, 0, 0, 0)
        k.bolts(d, 3, -s * 3, -(y1 - ystep) / 2 + 2, 1.6, -s * 4, 0.35)
        MB.ram(k, f"door_{S}", N("cockpit"), [s * (hw + 3), y1 - 2, z1 - 8], d, [-s * 4, (y1 - ystep) / 2 - 3, 1.5], 0.8)
        MB.vent(k, cp, s * 1.0, (ystep + y1) / 2, z1 + 3.5, (s * 0.2, 0.2, 1), "seam", rate=1.6)
        MB.vent(k, cp, s * (hw + 2.5), y1, z1 + 1, (s * 0.6, 0.7, 0.3), "open")
    step = MB.hatch(k, "CHEST_step", cp, 0, y0 - 1.0, z1 + 1.8, ("pos_z", 0.6), 1.9, 0.6)
    k.pl(2 * hw + 2, ystep - y0 + 2, 3.2, M["armor"], step, 0, (ystep - y0) / 2, 0)
    k.box(2 * hw, 1, 0.4, M["hazard"], step, 0, ystep - y0 + 0.4, 1.7)
    drop = MB.hatch(k, "CHEST_step_fold", step, 0, 0, 1.6, ("x", 1.5), 2.4, 0.8)
    k.box(2 * hw - 2, 0.6, 2.4, M["rubber"], drop, 0, (ystep - y0) * 0.5, 0.6)
    head_sensor(k, chest, 0, 4 + out["y1"] + 6, 4 + out["z0"] + 8, 10, wide=False)
    for S, s in (("L", 1), ("R", -1)):
        x = s * 10
        k.cyl(4.2, 5.0, 24, M["armor"], chest, x, 18, 4 + out["z0"] - 4)
        k.cyl(5.2, 5.2, 1.4, M["frame"], chest, x, 6.2, 4 + out["z0"] - 4)
        k.cyl(3.8, 3.8, 0.3, M["glow"], chest, x, 5.4, 4 + out["z0"] - 4)
        k.halo(chest, x, 4.4, 4 + out["z0"] - 4, 14, 0.7)
        k.stripes(k.g(chest, x, 22, 4 + out["z0"] - 4), 7, 0, 0, -4.8, PI, hb=1.1, gap=0.6)
        k.sock(f"JET_{S}", chest, x, 4.2, 4 + out["z0"] - 4, PI / 2)
    k.hose(chest, [[-13, 5, -12], [-19, 8, -5], [-20, 13, 3]], 0.9)
    k.hose(chest, [[13, 5, -12], [19, 8, -5], [20, 13, 3]], 0.9)
    sy = 4 + out["y1"] - 5
    for S, s in (("L", 1), ("R", -1)):
        sh, elb = contract_arm(k, chest, S, s * (out["hw"] + 5), sy, 1, -0.1, -14, -1.45)
        k.sph(5, M["frame"], sh, 0, 0, 0)
        p = k.pl(14, 9, 18, M["red"], sh, s * 2.2, 4, 0, 0, 0, s * 0.3)
        k.box(1.2, 7, 15, M["hazard"], p, s * 7.2, 0, 0)
        for i in range(3):
            k.px([[0, 0], [5, 0], [1.5, 8 + 2 * (i % 2)], [0, 7]], 1.6, M["armor2"], p, 0, 4.5, -6 + i * 6)
        k.box(6.5, 12, 6.5, M["frame"], sh, 0, -8, 0)
        k.cyl(4, 4, 9, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        hs = k.pl(12, 19, 11, M["armor"], elb, 0, -10, 0)
        k.box(9, 1.4, 0.5, M["hazard"], hs, 0, -7, 5.6)
        k.core(elb, 0, -10, 6.4, 2.2)
        k.box(13, 3, 12, M["frame"], elb, 0, -20.5, 0)
        hand = k.J(f"hand_{S}", elb, 0, -21, 0)
        if s > 0:   # the rail lance
            for x in (-2.2, 2.2):
                k.box(2, 30, 2.8, M["metal"], hand, x, -15, 0)
                for i in range(6):
                    k.box(0.6, 1.0, 4, M["glowDim"], hand, x, -4 - i * 4.4, 0)
            k.box(8, 2.6, 4.4, M["frame"], hand, 0, -6, 0)
            k.box(8, 2.6, 4.4, M["frame"], hand, 0, -21, 0)
            k.sock("MUZZLE_L", hand, 0, -31, 0, PI / 2)
        else:       # twin SMGs, Brisk's
            for x in (-2.8, 2.8):
                k.box(4, 13, 5, M["armor2"], hand, x, -6.5, 0)
                k.cyl(0.9, 0.9, 12, M["metal"], hand, x, -18, 0, seg=10)
                k.cyl(1.4, 1.4, 1.6, M["frame"], hand, x, -14, 0, seg=10)
                k.box(0.6, 7, 3, M["ink"], hand, x, -6, 2.6)
                k.sock(f"MUZZLE_R{'a' if x < 0 else 'b'}", hand, x, -24.5, 0, PI / 2)
            k.box(9, 3, 7, M["frame"], hand, 0, -1.5, -1)
            k.box(6, 6, 4, M["dark"], hand, 0, -6, -4.5)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=1.0, A=0.42, lift=0.55, bob=2.0 * k.s, roll=0.04, yaw=0.06, arm=0.1, lean=0.12, liftK=[0.5, 1, 0.9]),
                    idle=dict(scan=0.5, scanPeriod=1.8, breathHz=1.2))


# ----------------------------------------------------------------------------- ATLAS

def atlas(k):
    """Siege gunner in Vanta's and Hex's line: tall and upright on long straight legs, a rail cannon over
    the right shoulder, a missile battery for a left forearm, a reactor ring on its back. The chest is a
    tall bay (harness: the pilot stands). It opens like a drawbridge: the collar plates fold up and out,
    the whole front lowers forward on two heavy rams into a ramp, the ladder falls from its lip."""
    M = k.M
    segs, footH = [[26, -0.2], [28, 0.4]], 6
    hipY = leg_height(segs, footH)
    body = k.J("pelvis", k.root, 0, hipY, 0)
    k.pl(32, 10, 20, M["frame"], body, 0, 0, 0)
    k.cyl(5.4, 5.4, 38, M["dark"], body, 0, -1, 0, 0, 0, PI / 2)
    k.pl(16, 11, 4, M["armor"], body, 0, -3, 11, -0.15)
    k.core(body, 0, -1, 13.4, 2.8)
    for s in (1, -1):
        k.pl(5, 14, 16, M["armor2"], body, s * 19, -4, 0, 0, 0, s * 0.12)

    def seg0(k, g, s, L):
        k.cyl(5.8, 5.8, 10, M["frame"], g, 0, 0, 0, 0, 0, PI / 2)
        k.box(8.5, 24, 8.5, M["frame"], g, 0, -13, 0)
        pl = k.pl(12, 22, 4.5, M["armor"], g, 0, -12, 5, 0.03)
        k.stripes(pl, 10.5, 0, 4, 2.5, 0, hb=1.5, gap=0.8)
        k.pl(4, 20, 11, M["armor2"], g, s * 6.4, -12, 0)
        k.vent(g, 4.5, 9, s * 8.6, -13, 0, 0, s * PI / 2, 4)

    def seg1(k, g, s, L):
        k.cyl(5, 5, 12, M["dark"], g, 0, 0, 0, 0, 0, PI / 2)
        k.pl(12, 9, 5, M["armor2"], g, 0, 1, 6, -0.3)
        k.box(8, 27, 8, M["frame"], g, 0, -14, -0.5)
        gr = k.pl(12, 22, 4.5, M["armor"], g, 0, -15, 5, -0.03)
        k.box(10, 2.2, 0.5, M["hazard"], gr, 0, -9, 2.4)
        k.cyl(1.4, 1.4, 22, M["metal"], g, s * 5.5, -14, -4.5, seg=8)
        k.pl(9, 18, 5, M["armor2"], g, 0, -13, -5.5, 0.06)

    def foot(k, g, s, S):
        k.sph(4.4, M["dark"], g, 0, 0, 0)
        k.pl(16, 4.6, 22, M["armor"], g, 0, -3.2, 2.5)
        toe = k.J(f"toe_{S}", g, 0, -5.4, 9)
        for x in (-5, 5):
            k.px([[0, 0], [6.5, 0], [6.5, 1.5], [1.5, 4.2], [0, 4.2]], 6.4, M["armor2"], toe, x, 0, 5)
        for a in (-1, 1):
            k.pl(3, 3.2, 10, M["frame"], g, a * 8.4, -4.4, -3, 0, a * 0.5, 0)
        k.box(16.6, 1.2, 22.6, M["rubber"], g, 0, -5.4, 2.5)
        k.box(12, 1.8, 0.5, M["hazard"], g, 0, -2.4, 13.8)
        heel_jet(k, g, -12, 1.7)
        k.sock(f"WHEEL_{S}", g, 0, -5.4, 2.5)

    k.legs(body, 14, segs, [seg0, seg1], foot)
    k.piston(N("thigh_L"), [0, -3, -5], N("shin_L"), [0, -7, -5.2], 1.6)
    k.piston(N("thigh_R"), [0, -3, -5], N("shin_R"), [0, -7, -5.2], 1.6)
    waist = k.J("torso_twist", body, 0, 7.5, -0.5)
    k.cyl(9.5, 11, 7, M["dark"], waist, 0, 0, 0, 0, 0, 0, 24)
    for i in range(4):
        k.tor(10.8, 0.7, M["frame"], waist, 0, -2.4 + i * 1.6, 0, PI / 2)
    chest = k.J("torso", waist, 0, 4.5, 0)
    cp, cav, out = chest_bay(k, chest, "harness", (0, 4, 3), dict(hw=21, y0=-5, y1=32, z0=-18, z1=10))
    hw, y0, y1, z1 = cav["hw"], cav["y0"], cav["y1"], out["z1"]
    k.pl(2 * out["hw"] + 4, 4, out["z1"] - out["z0"] + 2, M["armor2"], cp, 0, out["y1"] + 2, (out["z0"] + out["z1"]) / 2)
    for s in (1, -1):
        k.vent(cp, 10, 16, s * (out["hw"] + 0.3), (y0 + y1) / 2, (out["z0"] + z1) / 2 - 2, 0, s * PI / 2, 6)
        k.pl(4, y1 - y0, 6, M["frame"], cp, s * (out["hw"] - 1), (y0 + y1) / 2, z1 + 1)
    # the drawbridge: the whole front, hinged at the sill, on two heavy rams; its face carries the sensor bar
    fr = MB.hatch(k, "CHEST_front", cp, 0, y0 - 1.0, z1 + 2.4, ("x", 1.42), 1.4, 2.2)
    plate_face(k, fr, 2 * hw + 6, y1 + 2 - (y0 - 1.0), 1, 0, glow_slit=True)
    for i in range(5):
        k.box(2 * hw * 0.8, 0.5, 1.2, M["frame"], fr, 0, 3 + i * (y1 - y0) / 6, -1.6)          # the ramp's ribs inside
    for S, s in (("L", 1), ("R", -1)):
        MB.ram(k, f"front_{S}", N("cockpit"), [s * (hw + 2), y0 + 3, z1 - 8], fr, [s * (hw + 1.5), (y1 - y0) * 0.55, -1.2], 1.4)
        c = MB.hatch(k, f"CHEST_collar_{S}", cp, s * (hw - 2), out["y1"] + 3.5, z1 - 4, ("z", -s * 1.0), 0.6, 1.0)
        k.pl(out["hw"] - hw + 6, 4, 14, M["red"], c, s * (out["hw"] - hw + 6) / 2, 1.5, 0, 0, 0, s * 0.1)
        k.box(1.2, 3, 12, M["hazard"], c, s * (out["hw"] - hw + 5), 1.5, 0)
        for f in (0.2, 0.5, 0.8):
            MB.vent(k, cp, s * (hw + 1.6), y0 + f * (y1 - y0), z1 + 1.8, (s, 0.1, 0.5), "seam")
        MB.vent(k, cp, s * (hw - 3), out["y1"] + 2, z1 - 2, (s * 0.3, 1, 0.2), "open", rate=1.4)
    MB.vent(k, cp, 0, y1 + 1.5, z1 + 2, (0, 0.4, 1), "seam", rate=2.0)
    head_sensor(k, chest, 0, 3 + out["y1"] + 6, 3 + (out["z0"] + z1) / 2 + 2, 11)
    # the reactor ring on the back, Hex's halo made solid
    ring = k.g(chest, 0, 3 + (out["y1"] + y0) / 2 + 4, 3 + out["z0"] - 5)
    k.tor(16, 1.6, M["frame"], ring, 0, 0, 0)
    k.tor(13.5, 0.7, M["glowDim"], ring, 0, 0, 0)
    for i in range(6):
        a = i / 6 * 2 * PI
        k.box(3, 5, 4, M["armor2"], ring, math.cos(a) * 16, math.sin(a) * 16, 0, 0, 0, a)
    k.core(ring, 0, 0, 0, 4)
    for S, s in (("L", 1), ("R", -1)):
        k.cyl(3.4, 4.4, 14, M["dark"], chest, s * 12, 3 + y0 + 4, 3 + out["z0"] - 3)
        k.cyl(3.0, 3.0, 0.3, M["glowDim"], chest, s * 12, 3 + y0 - 3.2, 3 + out["z0"] - 3)
        k.sock(f"JET_{S}", chest, s * 12, 3 + y0 - 3.6, 3 + out["z0"] - 3, PI / 2)
    k.hose(chest, [[-14, 6, -14], [-22, 10, -8], [-24, 18, 2]], 1.1)
    k.hose(chest, [[14, 6, -14], [22, 10, -8], [24, 18, 2]], 1.1)
    sy = 3 + out["y1"] - 7
    for S, s in (("L", 1), ("R", -1)):
        sh, elb = contract_arm(k, chest, S, s * (out["hw"] + 6), sy, 0, -0.06, -18, -0.9 if s > 0 else -0.35)
        k.sph(6.5, M["frame"], sh, 0, 0, 0)
        p = k.pl(17, 12, 24, M["armor"], sh, s * 2.8, 5, 0, 0, 0, s * 0.22)
        k.stripes(p, 20, s * 8.75, -0.5, 0, s * PI / 2, hb=1.4, gap=0.7)
        k.box(8, 16, 8, M["frame"], sh, 0, -9.5, 0)
        k.cyl(4.6, 4.6, 10, M["dark"], elb, 0, 0, 0, 0, 0, PI / 2)
        if s > 0:   # the missile battery forearm
            k.pl(13, 16, 13, M["armor2"], elb, 0, -9, 0)
            hand = k.J("hand_L", elb, 0, -19, 2)
            missile_box(k, hand, 0, -3, 2, 4, 3, 0.0, "MISSILE_L")
            k.sock("MUZZLE_L", hand, 0, -3, 8, 0)
        else:       # holds the rail cannon's grip; the rail rides the shoulder
            k.pl(12, 18, 12, M["armor2"], elb, 0, -10, 0)
            k.J("hand_R", elb, 0, -20, 0)
    rail = k.J("rail_R", chest, -(out["hw"] + 2), sy + 12, -2)
    k.box(8, 7, 14, M["frame"], rail, 0, 0, 0)
    k.cyl(3, 3, 8, M["dark"], rail, 0, -4, 0)
    k.core(rail, 0, 3, 0, 2.4)
    for x in (-1.8, 1.8):
        k.box(2.2, 2.6, 52, M["metal"], rail, x, 1.5, 24)
        for i in range(8):
            k.box(0.6, 3.2, 1.0, M["glowDim"], rail, x, 1.5, 6 + i * 5.6)
    for zz in (6, 22, 40):
        k.box(7, 4.5, 2.5, M["frame"], rail, 0, 1.5, zz)
    k.box(6.5, 3.6, 3, M["dark"], rail, 0, 1.5, 50.5)
    k.sock("MUZZLE_R", rail, 0, 1.5, 52.5, 0)
    k.extras.update(footH=footH * k.s, hipY=hipY * k.s,
                    walk=dict(hz=0.75, A=0.32, lift=0.42, bob=2.6 * k.s, roll=0.03, yaw=0.04, arm=0.06, lean=0.04),
                    idle=dict(scan=0.25, scanPeriod=3.6, breathHz=0.7))


# ----------------------------------------------------------------------------- run

DESIGNS = {
    # id: (builder, height m, glow, livery, core, light plate, name, card)
    "warden": (warden, 12.5, 0x59f2ff, 0x2f6fe0, 0xffb347, False, "Warden",
               dict(legs="human knee, two segments", cockpit="wall: seated in a flat screen wall, in the chest",
                    opening="clamshell: cheek guards swing out, the upper plate lifts on rams, the lower drops into a platform",
                    weapons="tower shield and shoulder missile box (L), breach cannon (R)",
                    note="Heavy assault, after Skarn and Bulwark. Recommended: the classic seated cockpit.")),
    "lancer": (lancer, 11.0, 0x59f2ff, 0x2f6fe0, 0x59f2ff, True, "Lancer",
               dict(legs="reverse knee, three segments", cockpit="jockey: leaning forward on a saddle, in a slim chest",
                    opening="petals: the brow lifts, two front doors swing apart, the step slides out and folds down",
                    weapons="rail lance (L), twin SMGs (R)", note="Fast skirmisher, after Brisk and Ravager. Big jets, heel skates.")),
    "atlas": (atlas, 13.5, 0x8fe3ff, 0x2f6fe0, 0x8fb6ff, False, "Atlas",
              dict(legs="human knee, long and straight", cockpit="harness: standing upright, a tall bay",
                   opening="drawbridge: the collars fold up, the whole front lowers on two rams into a ramp",
                   weapons="missile battery forearm (L), shoulder rail cannon (R)", note="Siege gunner, after Vanta and Hex. Reactor ring on its back.")),
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

def build(key, s):
    fn, height, glow, livery, core, light, name, card = DESIGNS[key]
    K.clear_scene()
    mats = K.make_mats(f"mech_{key}", glow, livery, core, light)
    k = Kit(mats, s, key)
    k.extras.update(glow=glow, core=core)
    fn(k)
    K.finish(k)
    return k


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    os.makedirs(OUT, exist_ok=True)
    for key in argv or list(DESIGNS):
        fn, height, glow, livery, core, light, name, card = DESIGNS[key]
        build(key, 1.0)
        _, b = K.measure()
        s = height / (b["y"][1] - b["y"][0])
        k = build(key, s)
        extras = dict(k.extras, rest={n: r for n, r in k.rest.items()}, scale=s)
        meta = dict(id=key, name=name, generator="tools/blender/build_mechs.py", status="design candidate",
                    style="Mining Mike boss kit (tank_bosses_kit.js at 3336b88)",
                    cockpit_variant=k.extras.get("cockpit", {}).get("variant"), **card)
        K.export(os.path.join(OUT, f"mech_{key}.glb"), MANIFEST, meta, extras)


main()
