"""The outer shape of a cockpit interior, measured off its glTF, for the chest that houses it.

The owner, 2026-10-24: "Cockpit doesn't have the cockpit shape we just made. It's a square box", "The
geometry of the cockpit must match the interiors that we just designed". So the mech's chest is not a box
round the interior's envelope: it is a shell lofted round the interior's own outline, measured here.

measure(variant) reads assets/models/cockpit_<variant>/cockpit_<variant>.gltf in its closed pose (every node
at rest), and returns, in metres in the interior's frame (+y up, the pilot faces +z, origin on the tub floor
under the eye):

  y0, y1        the floor and the top of the interior
  sb, st        the bottom and top of its screen wall
  bands         three convex plan outlines (x, z), under, through and above the screen wall
  z_cut         where the opening starts: just behind the pivots of the inner front and side units
                (INNER_front, INNER_side_*), so everything that swings out to let the pilot in is ahead of it
  centre        (0, z_cut): rays from here sample every band, so the cut is exactly the angles -90 to 90

Pure Python and numpy, no Blender: the glTF is read with its own accessors and node transforms, so this is
the real artifact, not a copy of its numbers (CLAUDE.md 6.5). ring(), offset() and clip_z() are the plan
geometry the chest builder lofts with.
"""
import json
import math
import os

import numpy as np

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
CTYPE = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
NCOMP = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def _matrix(n):
    if "matrix" in n:
        return np.array(n["matrix"], dtype=float).reshape(4, 4).T
    t = n.get("translation", [0, 0, 0])
    x, y, z, w = n.get("rotation", [0, 0, 0, 1])
    s = n.get("scale", [1, 1, 1])
    R = np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                  [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                  [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])
    M = np.eye(4)
    M[:3, :3] = R * np.array(s)
    M[:3, 3] = t
    return M


def _points(path):
    """Every vertex of the interior at rest, in its frame, and which material it carries."""
    js = json.load(open(path))
    base = os.path.dirname(path)
    bufs = [open(os.path.join(base, b["uri"]), "rb").read() for b in js["buffers"]]

    def acc(i):
        a = js["accessors"][i]
        bv = js["bufferViews"][a["bufferView"]]
        dt, nc = np.dtype(CTYPE[a["componentType"]]), NCOMP[a["type"]]
        off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        stride = bv.get("byteStride", 0) or dt.itemsize * nc
        if stride == dt.itemsize * nc:
            return np.frombuffer(bufs[bv["buffer"]], dtype=dt, count=a["count"] * nc, offset=off).reshape(-1, nc).astype(float)
        return np.array([np.frombuffer(bufs[bv["buffer"]], dtype=dt, count=nc, offset=off + k * stride)
                         for k in range(a["count"])], dtype=float)

    out = []

    def walk(i, M):
        n = js["nodes"][i]
        W = M @ _matrix(n)
        if "mesh" in n:
            for p in js["meshes"][n["mesh"]]["primitives"]:
                P = acc(p["attributes"]["POSITION"])
                mat = js["materials"][p["material"]]["name"] if "material" in p else ""
                out.append(((np.c_[P, np.ones(len(P))] @ W.T)[:, :3], mat))
        for c in n.get("children", []):
            walk(c, W)

    for r in js["scenes"][js.get("scene", 0)]["nodes"]:
        walk(r, np.eye(4))
    return out


def hull(xz):
    """The convex hull of plan points, counter-clockwise seen from above (+y), monotone chain."""
    P = sorted(set((round(float(x), 4), round(float(z), 4)) for x, z in xz))
    if len(P) < 3:
        return P

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lo, hi = [], []
    for p in P:
        while len(lo) >= 2 and cross(lo[-2], lo[-1], p) <= 1e-9:
            lo.pop()
        lo.append(p)
    for p in reversed(P):
        while len(hi) >= 2 and cross(hi[-2], hi[-1], p) <= 1e-9:
            hi.pop()
        hi.append(p)
    h = lo[:-1] + hi[:-1]
    return simplify(h)


def simplify(poly, tol=0.015):
    """Drop vertices that sit within tol metres of the line through their neighbours."""
    out = list(poly)
    changed = True
    while changed and len(out) > 4:
        changed = False
        for i in range(len(out)):
            a, b, c = out[i - 1], out[i], out[(i + 1) % len(out)]
            ab = math.hypot(c[0] - a[0], c[1] - a[1]) or 1e-9
            d = abs((c[0] - a[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (c[1] - a[1])) / ab
            if d < tol:
                out.pop(i)
                changed = True
                break
    return out


def offset(poly, d):
    """A convex polygon grown outward by d metres (every edge moved along its normal, corners re-met)."""
    n = len(poly)
    area = sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n))
    sgn = 1 if area > 0 else -1
    lines = []
    for i in range(n):
        (x0, z0), (x1, z1) = poly[i], poly[(i + 1) % n]
        ex, ez = x1 - x0, z1 - z0
        L = math.hypot(ex, ez) or 1e-9
        nx, nz = (ez / L, -ex / L) if sgn > 0 else (-ez / L, ex / L)
        lines.append(((x0 + nx * d, z0 + nz * d), (ex, ez)))
    out = []
    for i in range(n):
        (p, r), (q, s) = lines[i - 1], lines[i]
        den = r[0] * s[1] - r[1] * s[0]
        if abs(den) < 1e-9:
            out.append(q)
            continue
        t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / den
        out.append((p[0] + r[0] * t, p[1] + r[1] * t))
    return out


def ring(poly, angles, centre):
    """The polygon sampled along rays from centre at the given angles (0 is +z, pi/2 is +x): every ring of a
    loft shares the angles, so their quads line up; a polygon's own corners are among the angles."""
    cx, cz = centre
    out = []
    for a in angles:
        dx, dz = math.sin(a), math.cos(a)
        best = None
        for i in range(len(poly)):
            (x0, z0), (x1, z1) = poly[i], poly[(i + 1) % len(poly)]
            ex, ez = x1 - x0, z1 - z0
            den = dx * ez - dz * ex
            if abs(den) < 1e-12:
                continue
            t = ((x0 - cx) * ez - (z0 - cz) * ex) / den
            u = ((x0 - cx) * dz - (z0 - cz) * dx) / den
            if t > 0 and -1e-9 <= u <= 1 + 1e-9 and (best is None or t < best):
                best = t
        out.append((cx + dx * best, cz + dz * best))
    return out


def angles_for(polys, centre, n=24):
    """Uniform angles plus every corner of every polygon, sorted, so corners stay sharp in the loft."""
    cx, cz = centre
    a = {round(2 * math.pi * i / n, 6) for i in range(n)}
    for p in polys:
        for x, z in p:
            a.add(round(math.atan2(x - cx, z - cz) % (2 * math.pi), 6))
    a = sorted(a)
    keep = [a[0]]
    for v in a[1:]:
        if v - keep[-1] > 0.01:
            keep.append(v)
    return keep


SCREEN_FRAME_M = 0.07      # the screens' bezels stand this far proud of the glass


def measure(variant):
    """The interior's outline in three bands, in metres in its frame:
    dict(y0, y1, sb, st, bands=[(y_lo, y_hi, plan polygon), ...], z_cut, centre).
    Band 0 is under the screens (floor, consoles), band 1 the screen wall (the outside-view screens and
    their frames, and whatever stands beside them), band 2 above it (the roof). Each plan is convex,
    counter-clockwise from above. The plain plates of an older shell (EXT_* materials) are left out:
    the chest is the shell now."""
    path = os.path.join(ROOT, "assets", "models", f"cockpit_{variant}", f"cockpit_{variant}.gltf")
    man = json.load(open(os.path.join(ROOT, "assets", "models", f"cockpit_{variant}.json")))
    parts = [(P, m) for P, m in _points(path) if not m.startswith("EXT_")]
    P = np.vstack([q for q, _ in parts])
    A = np.vstack([q for q, m in parts if m == "SCREEN_allview"])
    y0, y1 = float(P[:, 1].min()), float(P[:, 1].max())
    sb, st = float(A[:, 1].min()), float(A[:, 1].max())
    glass = offset(hull(A[:, [0, 2]]), SCREEN_FRAME_M)
    mid = P[(P[:, 1] > sb + 0.02) & (P[:, 1] < st - 0.02)][:, [0, 2]]
    bands = [(y0, sb, hull(P[P[:, 1] <= sb + 0.02][:, [0, 2]])),
             (sb, st, hull(np.vstack([np.array(glass), mid]))),
             (st, y1, hull(P[P[:, 1] >= st - 0.02][:, [0, 2]]))]
    piv = [m["pivot_m"][2] for n, m in man.get("motions", {}).items()
           if n.startswith(("INNER_front", "INNER_side")) and "pivot_m" in m]
    z_cut = (min(piv) - 0.06) if piv else 0.3
    return dict(y0=y0, y1=y1, sb=sb, st=st, bands=bands, z_cut=z_cut, centre=(0.0, z_cut))


if __name__ == "__main__":
    for v in ("wall", "jockey", "harness"):
        m = measure(v)
        print(v, f"y {m['y0']:.2f}..{m['y1']:.2f}, screens {m['sb']:.2f}..{m['st']:.2f}, z_cut {m['z_cut']:.2f}")
        for lo, hi, poly in m["bands"]:
            print(f"  {lo:.2f}..{hi:.2f}", [(round(x, 2), round(z, 2)) for x, z in poly])
