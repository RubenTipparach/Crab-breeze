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
    e = dict(COCKPITS[variant])
    path = os.path.join(K.ROOT, "assets", "models", f"cockpit_{variant}.json")
    if os.path.exists(path):
        man = json.load(open(path))
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
