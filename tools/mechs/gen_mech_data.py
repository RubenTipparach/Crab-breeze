"""Write each detailed mech's game data from its glb (openspec change mech-roster, design section 4).

The owner, 2026-10-24: "Stevedore sucks..all these other mechs 10/10 let's get them integrated with the game". The game
flies a mech from `data/mechs/<id>.ron` and boards it on `data/mechs/<id>/boarding.ron`. Both are written here from the
glb that `tools/blender/build_mechs.py` exported, so nothing about a mech is typed twice:

- measured in the glb's model space at rest: the pelvis (hip height), `EYE_pilot`, the top of the model, the chest's
  half width;
- read from the glb root's extras: the chest's boarding tracks (`boarding.tracks`) and the cockpit interior with the
  interior parts that ride each chest joint (`cockpit.variant`, `cockpit.rides`);
- scaled from the tuned stand-in (`data/mechs/placeholder.ron`, 12 m) by k = height / 12 m where a size is not measured.

Run after build_mechs.py:

    python3 tools/mechs/gen_mech_data.py              # every mech in ROSTER
    python3 tools/mechs/gen_mech_data.py foreman      # one

`cb-tools validate` checks the written eye, hip and height against the glb, so data left stale by a rebuild fails.
"""
import json
import math
import os
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
ROSTER = ["foreman", "ironhorse", "harrow", "roustabout", "driller", "mule", "picket", "sapper"]

# The stand-in's tuning (data/mechs/placeholder.ron), the base every size is scaled from.
STAND_IN_HEIGHT_M = 12.0
STRIDE_PER_HIP = 13.0 / 5.9          # the stand-in's 13 m stride on its 5.9 m hip
# The kneel to be boarded is the mockup's (tools/mockups/mech_pose.js `sitPose`): each leg joint bends by
# `bendLegs` at k = 1.25, as far as the legs physically can, and the torso bows by the waist and chest turns. Owner,
# 2026-10-24: "dont fold the legs all the way down like that, how it was mocked up in the artifact made sense, it
# kneels as far as it 'physically can'" (the stand-in's old 1.8 m of 6.2 m sat the torso on the road).
SIT_BEND = 1.25
SIT_MULT = [0.5, 1.0, 0.8]
SIT_BOW_RAD = 0.28 + 0.12
# The boarding crane reaches out past everything below the hatch on the rope's line (a strip this wide either side of
# the centre line), plus a margin for the knees coming forward in the kneel (owner, 2026-10-24: "extend a crane out
# then lower the rope").
CRANE_STRIP_M = 2.0
CRANE_MARGIN_M = 1.5
# The page's boarding clock (tools/mockups/gen_mech_designs.py): the sit, then the chest a moment after it.
SIT_S, CHEST_AT_S = 3.0, 3.2
# The steam and mist (openspec change boarding-vents), on the page's own windows (gen_mech_designs.py `vent`):
# particles a second from one vent at rate 1, and the legs' rams' throw in the mech's frame (out, a little up and
# forward). The page streams steam at 30 a vent and mist at 9 x 0.7.
STEAM_PER_S, MIST_PER_S = 30.0, 6.3
STEAM_SPEED_M_S, MIST_SPEED_M_S = 3.0, 0.6         # the recipes' own (data/fx/fx.ron vent_steam, vent_mist)
LEG_RAMS = ("shin_L", "shin_R", "hock_L", "hock_R")
LEG_THROW = (1.0, 0.4, 0.3)


def glb_json(path):
    b = open(path, "rb").read()
    n = struct.unpack("<I", b[12:16])[0]
    return json.loads(b[20:20 + n])


def mat_trs(nd):
    """A glTF node's local matrix (row-major 4x4 lists)."""
    if "matrix" in nd:
        m = nd["matrix"]
        return [[m[c * 4 + r] for c in range(4)] for r in range(4)]
    tx, ty, tz = nd.get("translation", [0, 0, 0])
    x, y, z, w = nd.get("rotation", [0, 0, 0, 1])
    sx, sy, sz = nd.get("scale", [1, 1, 1])
    R = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
         [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
         [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    return [[R[0][0] * sx, R[0][1] * sy, R[0][2] * sz, tx],
            [R[1][0] * sx, R[1][1] * sy, R[1][2] * sz, ty],
            [R[2][0] * sx, R[2][1] * sy, R[2][2] * sz, tz],
            [0, 0, 0, 1]]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def apply(m, p):
    return [m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3] for i in range(3)]


def extra(x):
    """Root extras are written as JSON strings by the kit; accept either form."""
    return json.loads(x) if isinstance(x, str) else x


class Model:
    """A glb's node tree at rest, in model space (the scene root's frame: +y up, +z forward)."""

    def __init__(self, path):
        self.j = glb_json(path)
        self.nodes = self.j["nodes"]
        self.parent = {}
        for i, nd in enumerate(self.nodes):
            for c in nd.get("children", []):
                self.parent[c] = i
        self.index = {nd.get("name"): i for i, nd in enumerate(self.nodes)}
        self.world = {}
        for r in self.j["scenes"][0]["nodes"]:
            self._walk(r, [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]])

    def _walk(self, i, up):
        w = mul(up, mat_trs(self.nodes[i]))
        self.world[i] = w
        for c in self.nodes[i].get("children", []):
            self._walk(c, w)

    def at(self, name):
        return apply(self.world[self.index[name]], (0, 0, 0))

    def under(self, i, names):
        """Is node i in the subtree of a node with one of these names?"""
        while i is not None:
            if self.nodes[i].get("name") in names:
                return True
            i = self.parent.get(i)
        return False

    def corners(self, keep=lambda i: True):
        """The bounding-box corners of every mesh the filter keeps, in model space."""
        out = []
        for i, nd in enumerate(self.nodes):
            if "mesh" not in nd or not keep(i):
                continue
            for p in self.j["meshes"][nd["mesh"]]["primitives"]:
                a = self.j["accessors"][p["attributes"]["POSITION"]]
                lo, hi = a["min"], a["max"]
                for cx in (lo[0], hi[0]):
                    for cy in (lo[1], hi[1]):
                        for cz in (lo[2], hi[2]):
                            out.append(apply(self.world[i], (cx, cy, cz)))
        return out

    def root_extras(self):
        return self.nodes[self.index["root"]].get("extras", {})

    def rotation(self, name):
        return self.nodes[self.index[name]].get("rotation", [0, 0, 0, 1])


def rotate_inv(q, v):
    """v turned by the inverse of the unit quaternion q (x, y, z, w)."""
    x, y, z, w = q
    return rotate((-x, -y, -z, w), v)


def rotate(q, v):
    x, y, z, w = q
    ux, uy, uz = v
    # v' = v + 2w(q x v) + 2 q x (q x v)
    cx, cy, cz = y * uz - z * uy, z * ux - x * uz, x * uy - y * ux
    ccx, ccy, ccz = y * cz - z * cy, z * cx - x * cz, x * cy - y * cx
    return (ux + 2 * (w * cx + ccx), uy + 2 * (w * cy + ccy), uz + 2 * (w * cz + ccz))


def f(v, nd=2):
    """A number for RON: always with a decimal point."""
    s = f"{v:.{nd}f}"
    return "0.0" if s in ("-0.00", "-0.0") else s


def measure(m):
    """What the game reads of the mech's size, measured."""
    top = max(c[1] for c in m.corners())
    arms_legs = {f"{b}_{s}" for b in ("shoulder", "thigh") for s in ("L", "R")}
    body = m.corners(lambda i: not m.under(i, arms_legs))
    half_width = max(abs(c[0]) for c in body)
    return dict(hip=m.at("pelvis")[1], eye=m.at("EYE_pilot")[1], top=top, half_width=half_width,
                hatch=m.at("HATCH_entry"))


def mech_ron(mid, name, size, cockpit):
    k = size["top"] / STAND_IN_HEIGHT_M
    hip, eye, top = size["hip"], size["eye"], size["top"]
    rides = sorted(extra(cockpit.get("rides") or {}).items())
    v = cockpit["variant"]
    ride_lines = ", ".join(f'(part: "{p}", joint: "{j}")' for p, j in rides)
    return f"""// {name}: generated by tools/mechs/gen_mech_data.py from assets/models/mech_{mid}.glb (openspec change
// mech-roster). Edit the generator or the mech, not this file: `cb-tools validate` checks it against the glb.
// Sizes are measured off the glb ({f(top, 1)} m tall, hips at {f(hip, 1)} m, eyes at {f(eye, 1)} m); the rest is the
// stand-in's tuning (placeholder.ron) scaled by k = {f(k, 3)}.
(
    name: "{name}",
    model: Some("models/mech_{mid}.glb"),
    interior: Some((
        model: "models/cockpit_{v}/cockpit_{v}.gltf",
        rides: [{ride_lines}],
    )),
    locomotion: (
        walk_speed_max_m_s: 12.0,
        throttle_min: -0.4,
        throttle_rate_per_s: 1.5,
        accel_m_s2: 6.0,
        brake_m_s2: 9.0,
        turn_rate_deg_s: 60.0,
        twist_max_deg: 100.0,
        twist_rate_deg_s: 160.0,
        pitch_limits_deg: (-20.0, 25.0),
        pitch_rate_deg_s: 100.0,
        body_radius_m: {f(size["half_width"])},
        brush_radius_m: {f(5.5 * k)},
        body_height_m: {f(top)},
        stopped_speed_m_s: 0.2,
        hip_height_m: {f(hip)},
        max_wade_depth_m: {f(eye - 1.2)},
        wade_slowdown: 0.6,
        mass_kg: {f(50000.0 * k ** 3, 1)},
        limp_speed_ratio: 0.22,
    ),
    gait: (
        stride_max_m: {f(STRIDE_PER_HIP * hip)},
        stride_min_m: {f(2.0 * k)},
        step_height_m: {f(0.9 * k)},
        bob_m: {f(0.25 * k)},
        sway_m: {f(0.15 * k)},
        turn_step_radius_m: {f(2.5 * k)},
        settle_speed_m_s: 0.6,
    ),
    body: (
        eye_height_m: {f(eye)},
        torso_hit_m: ({f(0.5 * k)}, {f(3.9 * k)}),
        torso_hit_radius_m: {f(2.4 * k)},
        arm_hit_radius_m: {f(1.0 * k)},
        leg_hit_radius_m: {f(1.1 * k)},
        head_hit_radius_m: {f(1.1 * k)},
        wreck_centre_m: {f(10.0 * k)},
        blast_height_m: {f(9.0 * k)},
        smash_height_m: {f(5.0 * k)},
        cut_box_m: ({f(1.6 * k)}, {f(3.0 * k)}, {f(1.6 * k)}),
    ),
)
"""


AXES = {"x": (1.0, 0.0, 0.0), "y": (0.0, 1.0, 0.0), "z": (0.0, 0.0, 1.0)}


def kneel(m):
    """The mockup's kneel for this mech: (pelvis height, hock angle back from straight down in degrees), from the
    legs' own segments (root extras `legSegs`: length and bend of hip, knee and hock) exactly as `bendLegs` bends
    them, and the hock's rest angle measured off the rig."""
    ex = m.root_extras()
    segs = ex["legSegs"]
    if isinstance(segs, str):
        segs = json.loads(segs)
    foot = ex.get("footH", 0.0)
    a = a2 = 0.0
    h = h2 = foot
    for i, (length, rest) in enumerate(segs):
        bend = SIT_BEND * math.copysign(1.0, rest) * SIT_MULT[i] * 0.9
        a += rest
        a2 += rest + bend
        h += length * math.cos(a)
        h2 += length * math.cos(a2)
    pelvis = m.at("pelvis")[1] * h2 / h
    # The hock (the last segment) turns by the change in its absolute angle; a two-segment leg has no hock to turn.
    turn = -(a2 - a) if len(segs) >= 3 else 0.0
    hock, ankle = m.at("hock_L"), m.at("foot_L")
    rest_back = math.atan2(ankle[2] - hock[2], -(ankle[1] - hock[1]))
    return pelvis, math.degrees(rest_back + turn)


def crane_reach(m):
    """How far out in front of the hatch's sill the crane must reach: past the furthest point of any mesh below the sill
    that crosses the centre strip, measured at rest, plus the margin."""
    hatch = m.at("HATCH_entry")
    furthest = hatch[2]
    for i, nd in enumerate(m.nodes):
        if "mesh" not in nd:
            continue
        pts = []
        for p in m.j["meshes"][nd["mesh"]]["primitives"]:
            a = m.j["accessors"][p["attributes"]["POSITION"]]
            lo, hi = a["min"], a["max"]
            for cx in (lo[0], hi[0]):
                for cy in (lo[1], hi[1]):
                    for cz in (lo[2], hi[2]):
                        pts.append(apply(m.world[i], (cx, cy, cz)))
        xs, ys = [q[0] for q in pts], [q[1] for q in pts]
        if min(xs) < CRANE_STRIP_M and max(xs) > -CRANE_STRIP_M and min(ys) < hatch[1] - 0.5:
            furthest = max(furthest, max(q[2] for q in pts))
    return furthest - hatch[2] + CRANE_MARGIN_M


def unit(v):
    n = math.sqrt(sum(c * c for c in v)) or 1.0
    return tuple(c / n for c in v)


def vent_line(node, kind, duration, rate, d, hold=False):
    """One vent for the timeline: `kind` "steam" or "mist", `d` its throw in the node's own frame."""
    k = "Mist" if kind == "mist" else "Steam"
    per_s, speed = (MIST_PER_S, MIST_SPEED_M_S) if kind == "mist" else (STEAM_PER_S, STEAM_SPEED_M_S)
    d = unit(d)
    tail = ", hold: true" if hold else ""
    return (f'(node: "{node}", kind: {k}, duration_s: {f(duration)}, rate_per_s: {f(per_s * rate, 1)}, '
            f"speed_m_s: {f(speed, 1)}, dir: ({f(d[0], 3)}, {f(d[1], 3)}, {f(d[2], 3)}){tail})")


def vent_empties(nodes):
    """(name, kind, stage, rate, throw in the node's own frame) of every `VENT_*` empty in a glTF node list. The
    extras' `dir` is in the parent's frame; turned into the empty's own (they are written unrotated, so it is the
    same unless that changes)."""
    out = []
    for nd in nodes:
        name = nd.get("name", "")
        if not name.startswith("VENT_"):
            continue
        ex = nd.get("extras") or {}
        d = extra(ex.get("dir", [0, 0, 1]))
        d = rotate_inv(nd.get("rotation", [0, 0, 0, 1]), tuple(d))
        out.append((name, ex.get("kind", "steam"), ex.get("stage", ""), float(ex.get("rate", 1.0)), d))
    return sorted(out)


def interior_nodes(cockpit):
    v = cockpit.get("variant")
    if not v:
        return []
    path = os.path.join(ROOT, "assets", "models", f"cockpit_{v}", f"cockpit_{v}.gltf")
    return json.load(open(path))["nodes"] if os.path.exists(path) else []


def vent_stages(m, cockpit, tracks, ready_s):
    """The boarding's steam and mist (openspec change boarding-vents) as timeline stages (start, text), on the page's
    windows: the legs' rams through the sit, the seals as the chest breaks, the hinges as it swings, the mist over
    the sill and out of the cabin once it is open, held while it stands open."""
    end = max((t["t0"] + t["d"] for t in tracks), default=0.0)
    chest = vent_empties(m.nodes)
    cabin = vent_empties(interior_nodes(cockpit))
    stages = []

    def stage(name, start, duration, lines):
        if not lines:
            return
        body = "\n".join(f"                {ln}," for ln in lines)
        stages.append((start, f'        (name: "{name}", start_s: {f(start)}, duration_s: {f(duration)},\n'
                              f"            vents: [\n{body}\n            ]),"))

    # The legs' rams: the page's throw (out, up, forward) in the mech's frame, turned into each bone's rest frame.
    legs = []
    for b in LEG_RAMS:
        if b not in m.index:
            continue
        w = m.world[m.index[b]]
        side = 1.0 if b.endswith("_L") else -1.0
        t = (side * LEG_THROW[0], LEG_THROW[1], LEG_THROW[2])
        local = tuple(sum(w[r][c] * t[r] for r in range(3)) for c in range(3))     # R^T t (the scale cancels in unit)
        legs.append(vent_line(b, "steam", SIT_S, 1.0, local))
    stage("the legs' rams work", 0.0, SIT_S, legs)
    seals = [vent_line(n, k, 1.3, r, d) for n, k, st, r, d in chest if st == "seam"]
    stage("the seals break", CHEST_AT_S - 0.1, 1.3, seals)
    if end > 0.35:
        hinges = [vent_line(n, k, end - 0.3, r, d) for n, k, st, r, d in chest if st == "open"]
        stage("the hinges steam", CHEST_AT_S + 0.3, end - 0.3, hinges)
    # Held vents run to just past ready_s, so they keep going while the chest stands open.
    s0 = CHEST_AT_S + 0.6 * end
    sill = [vent_line(n, k, ready_s - s0 + 0.05, r, d, hold=True) for n, k, st, r, d in chest if st == "sill"]
    stage("mist rolls over the sill", s0, ready_s - s0, sill)
    s1 = CHEST_AT_S + 0.8 * end
    inner = [vent_line(n, k, ready_s - s1 + 0.05, r, d, hold=True) if k == "mist"
             else vent_line(n, k, min(1.5, ready_s - s1), r, d) for n, k, st, r, d in cabin]
    stage("the cabin breathes out", s1, ready_s - s1, inner)
    return stages


def boarding_ron(m, name, size, tracks, cockpit):
    stages = []
    for t in sorted(tracks, key=lambda t: (t["t0"], t["node"])):
        start, d = CHEST_AT_S + t["t0"], t["d"]
        if "pos" in t:
            # the page slides in the parent's frame; the game slides in the node's own, so turn the slide into it
            mv = rotate_inv(m.rotation(t["node"]), tuple(c * t["v"] for c in AXES[t["pos"]]))
            motion = f'(node: "{t["node"]}", translate_m: ({f(mv[0], 3)}, {f(mv[1], 3)}, {f(mv[2], 3)}))'
            what = "slides off its seal"
        else:
            ax = AXES[t["axis"]]
            motion = (f'(node: "{t["node"]}", axis: ({f(ax[0], 1)}, {f(ax[1], 1)}, {f(ax[2], 1)}), '
                      f'angle_deg: {f(math.degrees(t["v"]), 1)})')
            what = "swings open"
        stages.append((start, f'        (name: "{t["node"]} {what}", start_s: {f(start)}, duration_s: {f(d)},\n'
                              f"            motions: [{motion}]),"))
    ready = max([s for s, _ in stages] + [SIT_S]) if not tracks else max(CHEST_AT_S + t["t0"] + t["d"] for t in tracks)
    ready_s = float(f(ready + 0.3))
    stages += vent_stages(m, cockpit, tracks, ready_s)
    body = "\n".join(s for _, s in sorted(stages, key=lambda s: s[0]))
    pelvis, hock_deg = kneel(m)
    return f"""// {name}'s boarding: generated by tools/mechs/gen_mech_data.py from the glb's own chest tracks (the root extras
// `boarding.tracks`, written by tools/blender/mech_boarding.py), on the page's clock: the sit over {f(SIT_S, 1)} s, the chest
// from {f(CHEST_AT_S, 1)} s. The cockpit's opening parts ride their chest joints (the mech file's `interior.rides`), so they
// open with these stages. The steam and mist are the glb's and the interior's own `VENT_*` empties (openspec change
// boarding-vents). Edit the generator or the mech, not this file.
(
    sit: (duration_s: {f(SIT_S, 1)}, pelvis_height_m: {f(pelvis)}, hock_angle_deg: {f(hock_deg, 1)}, torso_bow_deg: {f(math.degrees(SIT_BOW_RAD), 1)}),
    stages: [
{body}
    ],
    ready_s: {f(ready_s)},
    clear_m: 3.0,
    crane_reach_m: {f(crane_reach(m))},
    screen_power_up_s: 2.5,
    screen_power_down_s: 0.6,
)
"""


def generate(mid):
    path = os.path.join(ROOT, "assets", "models", f"mech_{mid}.glb")
    m = Model(path)
    ex = m.root_extras()
    cockpit = extra(ex.get("cockpit") or {})
    tracks = extra(ex.get("boarding") or {}).get("tracks", [])
    meta = json.load(open(os.path.join(ROOT, "assets", "models", "mechs.json")))[mid]
    size = measure(m)
    out = os.path.join(ROOT, "data", "mechs")
    os.makedirs(os.path.join(out, mid), exist_ok=True)
    with open(os.path.join(out, f"{mid}.ron"), "w") as fh:
        fh.write(mech_ron(mid, meta["name"], size, cockpit))
    with open(os.path.join(out, mid, "boarding.ron"), "w") as fh:
        fh.write(boarding_ron(m, meta["name"], size, tracks, cockpit))
    print(f"{mid}: {size['top']:.2f} m tall, hips {size['hip']:.2f} m, eyes {size['eye']:.2f} m, "
          f"half width {size['half_width']:.2f} m, {len(tracks)} chest tracks, interior {cockpit.get('variant')}")


if __name__ == "__main__":
    for mid in (sys.argv[1:] or ROSTER):
        generate(mid)
