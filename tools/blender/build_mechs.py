"""Build the Crab Breeze mech design candidates as glb files on the rig contract.

    /opt/blender/blender --background --factory-startup --python tools/blender/build_mechs.py [-- id ...]

Writes assets/models/mech_<id>.glb and assets/models/mechs.json (the manifest). Three candidates
for the owner to choose between (docs/mockups/mech_designs.html):

  breeze   digitigrade striker, bubble canopy, swept jet pods  (survey M2 and C1 recommendations)
  bulwark  heavy, human knee, enclosed armoured cockpit with a vision slit
  hermit   wide and low on splayed back knees, a crab face and one oversized claw

Each is about 12 m (survey M1) and built on the bones of docs/design/mech-rig.md. Design pass:
flat PBR material roles, no Material Maker textures yet; UVs are already in metres for them.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mech_kit as K  # noqa: E402
from mech_kit import Part, Rig  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "models")
MANIFEST = os.path.join(OUT, "mechs.json")


# ----------------------------------------------------------------------------- skeleton

def skeleton(rig, P):
    """The contract's bones at this design's joint positions. P holds glTF points per joint."""
    rig.bone("root", None, (0, 0, 0))
    rig.bone("pelvis", "root", P["pelvis"])
    rig.bone("torso_twist", "pelvis", P["twist"])
    rig.bone("torso", "torso_twist", P["torso"])
    rig.bone("cockpit", "torso", P["cockpit"])
    rig.socket("EYE_pilot", "cockpit", P["eye"])
    rig.socket("SEAT_pilot", "cockpit", P["seat"])
    rig.socket("HATCH_entry", "cockpit", P["hatch"])
    rig.socket("LADDER_base", "root", P["ladder"])
    for s, S in ((1, "L"), (-1, "R")):
        def m(p):
            return (p[0] * s, p[1], p[2])
        rig.bone(f"shoulder_{S}", "torso", m(P["shoulder"]))
        rig.bone(f"upper_arm_{S}", f"shoulder_{S}", m(P["upper_arm"]))
        rig.bone(f"forearm_{S}", f"upper_arm_{S}", m(P["elbow"]))
        rig.bone(f"hand_{S}", f"forearm_{S}", m(P["hand"]))
        rig.socket(f"MUZZLE_{S}", f"hand_{S}", m(P["muzzle"]))
        rig.socket(f"JET_{S}", "torso", m(P["jet"]))
        rig.bone(f"thigh_{S}", "pelvis", m(P["hip"]))
        rig.bone(f"shin_{S}", f"thigh_{S}", m(P["knee"]))
        rig.bone(f"hock_{S}", f"shin_{S}", m(P["hock"]))
        rig.bone(f"foot_{S}", f"hock_{S}", m(P["ankle"]))
        rig.bone(f"toe_{S}", f"foot_{S}", m(P["toe"]))
        rig.socket(f"WHEEL_{S}", f"foot_{S}", m(P["wheel"]))
    for name, p in P.get("controls", {}).items():
        rig.socket(f"CTRL_{name}", "cockpit", p)


def lerp(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def off(p, d):
    return tuple(a + b for a, b in zip(p, d))


# ----------------------------------------------------------------------------- shared parts

def legs(rig, mats, P, style):
    """Both legs. style: thickness of the plates and whether the knee points forward."""
    w = style["leg_w"]
    for s, S in ((1, "L"), (-1, "R")):
        def m(p):
            return (p[0] * s, p[1], p[2])
        hip, knee, hock, ankle, toe = (m(P[k]) for k in ("hip", "knee", "hock", "ankle", "toe"))

        p = Part(rig, f"thigh_{S}", f"thigh_{S}_armour")
        p.cyl("joint", off(hip, (-0.55 * s, 0, 0)), off(hip, (0.55 * s, 0, 0)), 0.55, sides=20)
        p.cyl("metal", off(hip, (-0.62 * s, 0, 0)), off(hip, (0.62 * s, 0, 0)), 0.28, sides=16)
        a, b = off(hip, (0, -0.2, 0)), off(knee, (0, 0.35, 0))
        p.hull("hull", [off(a, (dx * w, 0, dz)) for dx in (-0.5, 0.5) for dz in (-0.55, 0.6)]
               + [off(b, (dx * w * 0.8, 0, dz)) for dx in (-0.5, 0.5) for dz in (-0.45, 0.45)])
        p.hull("trim", [off(lerp(a, b, t), (dx * w * 0.86, 0, 0.62 + 0.04 * t)) for t in (0.15, 0.7)
                        for dx in (-0.5, 0.5)] + [off(lerp(a, b, 0.42), (0, 0, 0.78))])
        p.piston(off(hip, (0.62 * s * w, -0.5, -0.55)), off(knee, (0.55 * s * w, 0.2, -0.45)), 0.13)
        p.finish(mats)

        p = Part(rig, f"shin_{S}", f"shin_{S}_armour")
        p.cyl("joint", off(knee, (-0.5 * s, 0, 0)), off(knee, (0.5 * s, 0, 0)), 0.48, sides=20)
        p.sphere("hull", off(knee, (0, 0.05, 0.25)), 0.55, (0.95 * w, 0.9, 0.8))
        a, b = off(knee, (0, -0.3, 0)), off(hock, (0, 0.25, 0))
        p.hull("hull", [off(a, (dx * w * 0.9, 0, dz)) for dx in (-0.5, 0.5) for dz in (-0.4, 0.55)]
               + [off(b, (dx * w * 0.6, 0, dz)) for dx in (-0.5, 0.5) for dz in (-0.3, 0.35)])
        p.hull("dark", [off(lerp(a, b, t), (dx * w * 0.7, 0, -0.55)) for t in (0.1, 0.85) for dx in (-0.5, 0.5)]
               + [off(lerp(a, b, 0.5), (0, 0, -0.75))], inset=False)
        p.piston(off(knee, (0.45 * s, -0.3, 0.55)), off(hock, (0.4 * s, 0.3, 0.45)), 0.1)
        p.box("hazard", off(lerp(a, b, 0.25), (0, 0, 0.58)), (w * 0.55, 0.16, 0.04), inset=False)
        p.finish(mats)

        p = Part(rig, f"hock_{S}", f"hock_{S}_strut")
        p.cyl("joint", off(hock, (-0.42 * s, 0, 0)), off(hock, (0.42 * s, 0, 0)), 0.38, sides=18)
        p.cyl("dark", hock, ankle, 0.34, 0.26, sides=16)
        p.hull("hull", [off(hock, (dx * w * 0.55, -0.1, dz)) for dx in (-0.5, 0.5) for dz in (-0.35, 0.2)]
               + [off(ankle, (dx * w * 0.5, 0.45, dz)) for dx in (-0.5, 0.5) for dz in (-0.25, 0.3)])
        p.finish(mats)

        p = Part(rig, f"foot_{S}", f"foot_{S}_shoe")
        p.cyl("joint", off(ankle, (-0.36 * s, 0, 0)), off(ankle, (0.36 * s, 0, 0)), 0.32, sides=18)
        fw = style["foot_w"]
        p.hull("hull", [off(ankle, (dx * fw, dy, dz)) for dx in (-0.5, 0.5) for dy, dz in
                        ((-0.45, -1.0), (-0.45, 1.4), (0.15, -0.6), (0.2, 0.7))]
               + [off(toe, (dx * fw * 0.8, dy, 0.4)) for dx in (-0.5, 0.5) for dy in (-0.2, 0.1)])
        p.box("dark", off(ankle, (0, -0.47, 0.2)), (fw * 1.04, 0.06, 2.3), inset=False)
        # the skate wheel, set into the sole (survey M4)
        wc = m(P["wheel"])
        p.cyl("joint", off(wc, (-0.22, 0, 0)), off(wc, (0.22, 0, 0)), 0.34, sides=24)
        p.cyl("metal", off(wc, (-0.26, 0, 0)), off(wc, (0.26, 0, 0)), 0.14, sides=16)
        p.finish(mats)

        p = Part(rig, f"toe_{S}", f"toe_{S}_claw")
        for dx in (-0.32, 0.32):
            base = off(toe, (dx * fw, -0.1, 0))
            p.hull("metal", [off(base, (ex, ey, ez)) for ex in (-0.12, 0.12) for ey, ez in
                             ((-0.12, 0.0), (0.18, 0.0), (-0.12, 0.65))] + [off(base, (0, -0.15, 0.95))], inset=False)
        p.finish(mats, bevel=0.02)


def cockpit_interior(rig, mats, P):
    """Seat, consoles, screens, stick and throttle: what the pilot sees from EYE_pilot."""
    seat, eye = P["seat"], P["eye"]
    p = Part(rig, "cockpit", "cockpit_interior")
    p.box("seat", off(seat, (0, 0, 0)), (0.62, 0.14, 0.6), inset=False)
    p.box("seat", off(seat, (0, 0.5, -0.32)), (0.6, 0.95, 0.14), inset=False)
    p.box("seat", off(seat, (0, 1.02, -0.36)), (0.34, 0.26, 0.12), inset=False)
    p.box("dark", off(seat, (0, -0.2, 0)), (0.5, 0.25, 0.5), inset=False)
    # front console under the sightline: its top slopes up away from the pilot, screens face the eye
    near, far = eye[2] + 0.42, eye[2] + 0.78
    top_n, top_f = eye[1] - 0.62, eye[1] - 0.42
    p.hull("dark", [(x, eye[1] - 1.05, z) for x in (-0.82, 0.82) for z in (near, far)]
           + [(x, top_n, near) for x in (-0.82, 0.82)] + [(x, top_f, far) for x in (-0.82, 0.82)], inset=False)
    for x in (-0.52, 0, 0.52):
        p.box("screen", (x, (top_n + top_f) / 2 + 0.02, (near + far) / 2),
              (0.3, 0.02, 0.2), inset=False, rot=K.Matrix.Rotation(-0.5, 4, "X"))
    # side consoles and the controls
    for s in (1, -1):
        p.box("dark", (0.52 * s, seat[1] + 0.2, seat[2] + 0.3), (0.22, 0.42, 0.7), inset=False)
        p.box("screen", (0.52 * s, seat[1] + 0.42, seat[2] + 0.35), (0.14, 0.02, 0.22), inset=False)
    for name, p0 in P.get("controls", {}).items():
        p.cyl("metal", p0, off(p0, (0, 0.28, 0.03)), 0.025, sides=10)
        p.sphere("joint", off(p0, (0, 0.32, 0.04)), 0.055)
    p.finish(mats, bevel=0.01, segments=1)


def jets(rig, mats, P, size=1.0):
    """The flight pods on the back (survey M3): thrust down at rest, glowing nozzles."""
    p = Part(rig, "torso", "jet_pods")
    for s in (1, -1):
        j = (P["jet"][0] * s, P["jet"][1], P["jet"][2])
        top, bot = off(j, (0, 1.6 * size, 0)), off(j, (0, -0.2, 0))
        p.cyl("hull", top, off(j, (0, 0.2, 0)), 0.42 * size, 0.55 * size, sides=20)
        p.cyl("dark", off(j, (0, 0.2, 0)), bot, 0.52 * size, 0.46 * size, sides=20)
        p.cyl("glow", bot, off(bot, (0, -0.05, 0)), 0.38 * size, 0.38 * size, sides=20)
        p.cyl("metal", off(j, (-0.6 * s * size, 1.0 * size, 0.6)), off(j, (-0.15 * s, 1.1 * size, 0.1)), 0.09)
    p.finish(mats)


def arm(rig, mats, P, S, s, weapon, w=1.0):
    def m(p):
        return (p[0] * s, p[1], p[2])
    sh, ua, el, ha, mu = (m(P[k]) for k in ("shoulder", "upper_arm", "elbow", "hand", "muzzle"))
    p = Part(rig, f"shoulder_{S}", f"shoulder_{S}_mount")
    p.cyl("joint", off(sh, (-0.3 * s, 0, 0)), off(ua, (0.1 * s, 0, 0)), 0.5 * w, sides=20)
    p.hull("trim", [off(sh, (dx, dy, dz)) for dx in (-0.2 * s, 1.1 * s * w) for dy in (-0.2, 0.75 * w) for dz in (-0.9 * w, 0.9 * w)]
           + [off(sh, (0.5 * s * w, 1.0 * w, 0))])
    p.finish(mats)
    p = Part(rig, f"upper_arm_{S}", f"upper_arm_{S}_armour")
    p.hull("hull", [off(ua, (dx * w, -0.2, dz * w)) for dx in (-0.42, 0.42) for dz in (-0.42, 0.42)]
           + [off(el, (dx * w, 0.25, dz * w)) for dx in (-0.36, 0.36) for dz in (-0.36, 0.36)])
    p.piston(off(ua, (0, -0.1, -0.5 * w)), off(el, (0, 0.1, -0.45 * w)), 0.08 * w)
    p.finish(mats)
    p = Part(rig, f"forearm_{S}", f"forearm_{S}_armour")
    p.cyl("joint", off(el, (-0.42 * w, 0, 0)), off(el, (0.42 * w, 0, 0)), 0.38 * w, sides=18)
    p.hull("hull", [off(el, (dx * w, dy * w, -0.25)) for dx in (-0.42, 0.42) for dy in (-0.42, 0.42)]
           + [off(ha, (dx * w, dy * w, -0.2)) for dx in (-0.36, 0.36) for dy in (-0.36, 0.36)])
    p.finish(mats)
    p = Part(rig, f"hand_{S}", f"hand_{S}_{weapon}")
    weapons[weapon](p, ha, mu, w)
    p.finish(mats, bevel=0.025)


def w_rail(p, ha, mu, w):
    p.box("dark", off(ha, (0, 0, 0.6)), (0.62 * w, 0.62 * w, 1.6), inset=True)
    p.box("trim", off(ha, (0, 0.34 * w, 0.6)), (0.4 * w, 0.06, 1.3), inset=False)
    for dx in (-0.14, 0.14):
        p.box("metal", off(ha, (dx * w, 0, (mu[2] - ha[2] + 1.4) / 2)), (0.09, 0.22 * w, mu[2] - ha[2] - 1.4), inset=False)
    p.cyl("glow", off(ha, (0, 0, 1.45)), off(ha, (0, 0, 1.5)), 0.12, sides=12)


def w_gatling(p, ha, mu, w):
    p.cyl("dark", off(ha, (0, 0, -0.1)), off(ha, (0, 0, 1.0)), 0.48 * w, sides=20)
    for i in range(6):
        a = i / 6 * math.tau
        o = (math.cos(a) * 0.22 * w, math.sin(a) * 0.22 * w, 0)
        p.cyl("metal", off(ha, o), off(mu, o), 0.07 * w, sides=10)
    for z in (0.3, 0.75):
        t = (mu[2] - ha[2]) * z
        p.cyl("dark", off(ha, (0, 0, t - 0.06)), off(ha, (0, 0, t + 0.06)), 0.36 * w, sides=20)


def w_cannon(p, ha, mu, w):
    p.box("dark", off(ha, (0, 0, 0.5)), (0.9 * w, 0.9 * w, 1.6))
    p.cyl("metal", off(ha, (0, 0, 1.2)), mu, 0.26 * w, 0.22 * w, sides=20)
    p.cyl("dark", off(mu, (0, 0, -0.6)), mu, 0.34 * w, 0.34 * w, sides=20)
    p.vents("joint", off(ha, (0, 0, 1.1)), 0.7 * w, 0.6 * w, 5, axis="x")


def w_claw(p, ha, mu, w):
    """The hermit's crusher: two jaws on a heavy wrist, the size of a car."""
    p.cyl("joint", off(ha, (0, 0, -0.2)), off(ha, (0, 0, 0.5)), 0.7 * w, sides=24)
    for s, rolec in ((1, "trim"), (-1, "hull")):
        p.hull(rolec, [off(ha, (dx, s * 0.12, 0.4)) for dx in (-0.55 * w, 0.55 * w)]
               + [off(ha, (dx, s * 0.95 * w, 0.9)) for dx in (-0.5 * w, 0.5 * w)]
               + [off(ha, (dx, s * 0.6 * w, 2.4 * w)) for dx in (-0.35 * w, 0.35 * w)]
               + [off(ha, (0, s * 0.15, 2.9 * w))])
    p.cyl("glow", off(ha, (0, 0, 0.45)), off(ha, (0, 0, 0.5)), 0.2, sides=12)


def w_missiles(p, ha, mu, w):
    p.box("trim", off(ha, (0, 0, 0.45)), (1.0 * w, 0.8 * w, 1.3))
    for ix in (-1, 0, 1):
        for iy in (-0.5, 0.5):
            p.cyl("dark", off(ha, (ix * 0.3 * w, iy * 0.34 * w, 1.06)), off(ha, (ix * 0.3 * w, iy * 0.34 * w, 1.12)), 0.11 * w, sides=12)


weapons = {"rail": w_rail, "gatling": w_gatling, "cannon": w_cannon, "claw": w_claw, "missiles": w_missiles}


def ladder(rig, mats, P):
    """Rungs from LADDER_base up the leg toward the hatch: boarding is climbed (the law)."""
    p = Part(rig, "cockpit", "ladder")
    base = P["ladder"]
    top = P["ladder_top"]
    n = int((top[1] - base[1]) / 0.4)
    for i in range(n):
        y = base[1] + 0.2 + i * 0.4
        p.box("metal", (base[0], y, base[2]), (0.6, 0.05, 0.06), inset=False)
    for dx in (-0.32, 0.32):
        p.box("dark", (base[0] + dx, (base[1] + top[1]) / 2, base[2]), (0.06, top[1] - base[1], 0.08), inset=False)
    p.finish(mats, bevel=0)


def tub(p, role, xh, y0, y1, z0, z1, cowl, t=0.1):
    """An open cockpit tub: floor, side walls, back wall to y1, a front cowl only `cowl` m high."""
    p.box(role, (0, y0, (z0 + z1) / 2), (2 * xh, t, z1 - z0))
    for s in (1, -1):
        p.box(role, (s * (xh - t / 2), (y0 + y1) / 2, (z0 + z1) / 2), (t, y1 - y0, z1 - z0))
    p.box(role, (0, (y0 + y1) / 2, z0 + t / 2), (2 * xh, y1 - y0, t))
    p.box(role, (0, y0 + cowl / 2, z1 - t / 2), (2 * xh, cowl, t))


# ----------------------------------------------------------------------------- designs

def breeze(rig, mats):
    P = dict(pelvis=(0, 6.4, 0), twist=(0, 7.0, 0), torso=(0, 7.4, 0), cockpit=(0, 9.2, 1.2),
             eye=(0, 9.62, 1.55), seat=(0, 8.95, 1.35), hatch=(0.0, 10.4, 1.4),
             shoulder=(2.25, 9.9, -0.2), upper_arm=(2.65, 9.75, -0.2), elbow=(2.75, 8.15, 0.05),
             hand=(2.75, 7.9, 1.25), muzzle=(2.75, 7.9, 4.2), jet=(1.35, 9.6, -1.95),
             hip=(1.05, 6.2, 0), knee=(1.1, 3.95, 1.15), hock=(1.1, 1.75, -0.35), ankle=(1.1, 0.52, 0.05),
             toe=(1.1, 0.35, 1.35), wheel=(1.1, 0.34, 0.2), ladder=(0.0, 0.0, 2.35), ladder_top=(0.0, 8.35, 2.35),
             controls={"stick": (0.0, 8.55, 1.75), "throttle": (-0.52, 8.95, 1.45), "twist": (0.52, 8.95, 1.45)})
    skeleton(rig, P)
    legs(rig, mats, P, dict(leg_w=1.0, foot_w=1.1))

    p = Part(rig, "pelvis", "pelvis_block")
    p.hull("dark", [(x, y, z) for x in (-1.15, 1.15) for y in (5.85, 6.95) for z in (-0.8, 0.7)])
    p.mirror_hull("hull", [(0.5, 6.95, 0.75), (1.3, 6.8, 0.6), (0.5, 6.0, 0.95), (1.2, 6.1, 0.8), (0.9, 6.5, 1.05)])
    p.cyl("metal", (0, 6.95, 0), (0, 7.25, 0), 0.75, sides=24)
    p.finish(mats)

    p = Part(rig, "torso", "torso_shell")
    # the chest: wide at the shoulders, narrow at the waist, cut away in front for the cockpit pod
    p.hull("hull", [(x, 7.4, z) for x in (-1.0, 1.0) for z in (-1.2, 0.4)]
           + [(x, 10.6, z) for x in (-1.95, 1.95) for z in (-1.6, 0.2)]
           + [(x, 11.05, z) for x in (-1.3, 1.3) for z in (-1.2, -0.1)] + [(0, 9.0, 0.6)])
    p.mirror_hull("trim", [(1.0, 10.65, -1.5), (1.9, 10.6, -1.3), (1.0, 11.1, -1.1), (1.3, 11.05, -0.2), (0.4, 11.15, -0.6)])
    p.hull("dark", [(x, y, z) for x in (-1.2, 1.2) for y in (8.0, 10.4) for z in (-2.0, -1.4)])  # reactor housing
    p.vents("joint", (0, 9.2, -2.05), 1.8, 1.6, 7)
    p.box("hazard", (0, 10.75, 0.22), (1.6, 0.12, 0.04), inset=False)
    # swept fins over the jets, the Ace Combat silhouette
    p.mirror_hull("trim", [(1.2, 10.9, -1.6), (1.4, 10.9, -2.2), (2.9, 11.6, -3.4), (2.7, 11.75, -3.2), (1.3, 11.05, -1.5)])
    p.cyl("dark", (0.9, 11.1, -0.9), (0.9, 12.05, -1.1), 0.04, sides=8)  # antenna
    p.sphere("glow", (0.9, 12.07, -1.1), 0.06)
    p.finish(mats)

    p = Part(rig, "cockpit", "cockpit_pod")
    # a tub the pilot sits in, open on top under a bubble canopy
    tub(p, "hull", 1.0, 8.4, 9.3, 0.35, 2.7, 0.62)
    p.hull("hull", [(x, 8.4, z) for x in (-0.9, 0.9) for z in (2.3, 2.7)] + [(0, 8.0, 3.0), (0, 8.9, 2.95)])  # nose
    p.sphere("glass", (0, 9.4, 1.55), 1.0, (0.92, 0.85, 1.32), seg=32)
    for t in (-0.5, 0.0, 0.5):  # canopy frame ribs
        x = 0.9 * t
        p.box("dark", (x, 9.6 + 0.5 * math.cos(t), 1.55), (0.05, 0.05, 2.2), inset=False)
    p.cyl("dark", (-0.92, 9.4, 1.55), (0.92, 9.4, 1.55), 0.04, sides=8)
    p.box("glow", (0, 8.65, 2.62), (0.5, 0.06, 0.03), inset=False)
    p.finish(mats)
    cockpit_interior(rig, mats, P)
    jets(rig, mats, P, 1.0)
    for s, S, wpn in ((1, "L", "rail"), (-1, "R", "gatling")):
        arm(rig, mats, P, S, s, wpn)
    ladder(rig, mats, P)
    return dict(id="breeze", name="Breeze", legs="digitigrade", cockpit="bubble canopy",
                weapons="rail cannon (L), gatling (R)", note="Recommended: survey M2 and C1 picks.")


def bulwark(rig, mats):
    P = dict(pelvis=(0, 6.0, 0), twist=(0, 6.7, 0), torso=(0, 7.0, 0), cockpit=(0, 9.6, 1.0),
             eye=(0, 10.0, 1.05), seat=(0, 9.38, 0.85), hatch=(0, 10.7, 1.0),
             shoulder=(2.45, 10.1, 0), upper_arm=(2.95, 9.9, 0), elbow=(3.05, 8.0, 0.1),
             hand=(3.05, 7.7, 1.3), muzzle=(3.05, 7.7, 4.6), jet=(1.6, 9.0, -2.2),
             hip=(1.3, 5.8, 0), knee=(1.3, 3.25, 0.35), hock=(1.3, 1.05, -0.05), ankle=(1.3, 0.6, 0.05),
             toe=(1.3, 0.4, 1.5), wheel=(1.3, 0.34, 0.25), ladder=(0.0, 0.0, 1.75), ladder_top=(0.0, 8.8, 1.75),
             controls={"stick": (0.0, 8.95, 1.25), "throttle": (-0.52, 9.38, 0.95), "twist": (0.52, 9.38, 0.95)})
    skeleton(rig, P)
    legs(rig, mats, P, dict(leg_w=1.35, foot_w=1.5))

    p = Part(rig, "pelvis", "pelvis_block")
    p.hull("dark", [(x, y, z) for x in (-1.45, 1.45) for y in (5.4, 6.7) for z in (-1.0, 0.9)])
    p.mirror_hull("hull", [(0.6, 6.7, 0.95), (1.7, 6.6, 0.8), (0.6, 5.2, 1.2), (1.6, 5.3, 1.0), (1.3, 6.0, 1.3)])
    p.cyl("metal", (0, 6.7, 0), (0, 7.0, 0), 0.95, sides=24)
    p.finish(mats)

    p = Part(rig, "torso", "torso_shell")
    p.hull("hull", [(x, 7.0, z) for x in (-1.6, 1.6) for z in (-1.4, 1.0)] + [(x, 9.2, z) for x in (-2.2, 2.2) for z in (-1.8, 1.15)]
           + [(x, 10.9, z) for x in (-2.2, 2.2) for z in (-1.8, 0.3)] + [(x, 11.3, z) for x in (-1.6, 1.6) for z in (-1.4, 0.0)])
    p.box("hazard", (0, 7.5, 1.05), (2.4, 0.18, 0.05), inset=False)
    p.vents("joint", (0, 8.6, 1.05), 2.0, 1.2, 6)
    p.hull("dark", [(x, y, z) for x in (-1.5, 1.5) for y in (8.0, 10.8) for z in (-2.3, -1.7)])
    p.box("trim", (0, 11.55, -0.6), (0.8, 0.5, 1.2))  # sensor mast
    p.sphere("glow", (0, 11.75, 0.02), 0.12)
    p.finish(mats)

    # the armoured head that IS the cockpit: a hollow shell, one vision slit in front
    p = Part(rig, "cockpit", "cockpit_head")
    xh, y0, y1, z0, z1, t = 1.05, 9.25, 10.65, 0.2, 2.05, 0.14
    tub(p, "trim", xh, y0, y1, z0, z1, 0.62, t)
    p.box("trim", (0, y1, (z0 + z1) / 2), (2 * xh, t, z1 - z0))            # roof
    p.box("trim", (0, (10.14 + y1) / 2, z1 - t / 2), (2 * xh, y1 - 10.14, t))  # brow over the slit
    for s in (1, -1):
        p.box("trim", (s * 0.88, (9.87 + 10.14) / 2, z1 - t / 2), (0.34, 0.27, t), inset=False)
    p.box("glass", (0, 10.0, z1 - 0.05), (1.4, 0.27, 0.03), inset=False)
    p.box("dark", (0, 10.2, z1 + 0.05), (1.9, 0.1, 0.18), inset=False)
    p.hull("trim", [(x, 9.25, z) for x in (-1.0, 1.0) for z in (1.6, 2.1)] + [(x, 8.8, z) for x in (-0.7, 0.7) for z in (1.3, 1.9)])
    p.finish(mats)
    cockpit_interior(rig, mats, P)
    jets(rig, mats, P, 1.15)
    for s, S, wpn in ((1, "L", "missiles"), (-1, "R", "cannon")):
        arm(rig, mats, P, S, s, wpn, w=1.25)
    ladder(rig, mats, P)
    return dict(id="bulwark", name="Bulwark", legs="human knee", cockpit="enclosed, vision slit and screens",
                weapons="missile box (L), heavy cannon (R)", note="Survey C1 option 2: armour over view.")


def hermit(rig, mats):
    P = dict(pelvis=(0, 5.4, 0), twist=(0, 5.9, 0), torso=(0, 6.2, 0), cockpit=(0, 7.4, 1.8),
             eye=(0, 7.75, 2.25), seat=(0, 7.1, 2.0), hatch=(0, 8.9, 0.8),
             shoulder=(2.6, 7.4, 0.6), upper_arm=(3.0, 7.2, 0.8), elbow=(3.2, 6.0, 1.8),
             hand=(3.2, 6.1, 3.0), muzzle=(3.2, 6.1, 5.4), jet=(1.9, 7.6, -2.0),
             hip=(1.8, 5.2, 0), knee=(2.5, 3.6, 1.0), hock=(2.6, 1.6, -0.6), ankle=(2.6, 0.5, 0.0),
             toe=(2.6, 0.32, 1.3), wheel=(2.6, 0.34, 0.2), ladder=(0.0, 0.0, 3.2), ladder_top=(0.0, 6.7, 3.2),
             controls={"stick": (0.0, 6.7, 2.45), "throttle": (-0.52, 7.1, 2.15), "twist": (0.52, 7.1, 2.15)})
    skeleton(rig, P)
    legs(rig, mats, P, dict(leg_w=1.1, foot_w=1.25))

    p = Part(rig, "pelvis", "pelvis_block")
    p.hull("dark", [(x, y, z) for x in (-2.0, 2.0) for y in (4.8, 5.9) for z in (-0.9, 0.9)])
    p.cyl("metal", (0, 5.9, 0), (0, 6.2, 0), 1.0, sides=24)
    p.finish(mats)

    p = Part(rig, "torso", "carapace")
    # one wide shell, flat on top, like a crab seen from the front
    p.hull("hull", [(x, 6.2, z) for x in (-2.3, 2.3) for z in (-1.8, 1.2)] + [(x, 8.4, z) for x in (-3.0, 3.0) for z in (-2.2, 1.0)]
           + [(x, 8.9, z) for x in (-2.2, 2.2) for z in (-1.6, 0.4)])
    p.mirror_hull("trim", [(1.4, 8.92, -1.5), (2.9, 8.45, -2.0), (2.95, 8.5, 0.8), (1.5, 8.95, 0.3), (2.2, 9.15, -0.6)])
    for s in (1, -1):  # the two eyes either side of the canopy
        p.cyl("dark", (1.15 * s, 8.6, 0.9), (1.15 * s, 9.3, 1.25), 0.12, sides=12)
        p.sphere("glow", (1.15 * s, 9.38, 1.3), 0.2)
    p.vents("joint", (0, 7.4, -1.85), 3.0, 1.2, 6)
    p.finish(mats)

    p = Part(rig, "cockpit", "cockpit_face")
    tub(p, "dark", 1.1, 6.75, 7.45, 0.9, 3.0, 0.5)
    p.sphere("glass", (0, 7.55, 2.3), 0.95, (1.1, 0.75, 0.95), seg=32)
    p.cyl("dark", (-1.05, 7.55, 2.3), (1.05, 7.55, 2.3), 0.05, sides=8)
    p.finish(mats)
    cockpit_interior(rig, mats, P)
    jets(rig, mats, P, 0.9)
    arm(rig, mats, P, "L", 1, "claw", w=1.35)
    arm(rig, mats, P, "R", -1, "gatling", w=0.85)
    ladder(rig, mats, P)
    return dict(id="hermit", name="Hermit", legs="splayed digitigrade", cockpit="low crab face canopy",
                weapons="crusher claw (L), gatling (R)", note="The name's crab: low, wide, asymmetric.")


DESIGNS = {
    "breeze": (breeze, {"hull": (0.86, 0.87, 0.88), "trim": (0.16, 0.34, 0.70)}),
    "bulwark": (bulwark, {"hull": (0.42, 0.45, 0.33), "trim": (0.30, 0.32, 0.24), "hazard": (0.92, 0.62, 0.10)}),
    "hermit": (hermit, {"hull": (0.78, 0.38, 0.16), "trim": (0.30, 0.18, 0.12), "glow": (1.0, 0.55, 0.20)}),
}


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    os.makedirs(OUT, exist_ok=True)
    for key in argv or list(DESIGNS):
        fn, colours = DESIGNS[key]
        K.clear_scene()
        mats = K.make_materials(f"mech_{key}", colours)
        meta = fn(Rig(), mats)
        meta["generator"] = "tools/blender/build_mechs.py"
        meta["status"] = "design candidate"
        K.export(os.path.join(OUT, f"mech_{key}.glb"), MANIFEST, meta)


main()
