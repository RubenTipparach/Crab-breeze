"""Inverted faces in the shipped models (openspec change `city-frame` section 7; owner, 2026-10-28: "verify that there
are no inverted faces, I saw some on the center wood beam on the house").

    python tools/build/check_winding.py                  # every .glb and .gltf under assets/
    python tools/build/check_winding.py assets/models/small_buildings --list 20

A triangle is inverted when its winding (glTF's front face is counter-clockwise) points against the normals stored on
its corners: lit as if it faced one way, culled as if it faced the other. A piece is inside out when it is closed (every
edge shared by two of its triangles) and its signed volume is negative, or, open or closed, when most of its area faces
in towards its own centre (a box, a beam or a plank missing its ends; a curved or concave piece can trip it, so it is
reported apart and does not fail the check). A node mirrored by a negative scale flips the
winding, as glTF says, and is counted so. Exits 1 when any model has an inverted triangle or an inside-out piece.
"""

import json
import os
import struct
import sys
from collections import defaultdict

import numpy as np

COMPONENT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def load(path):
    """The glTF JSON and its binary buffers."""
    if path.endswith(".glb"):
        data = open(path, "rb").read()
        jl = struct.unpack("<I", data[12:16])[0]
        j = json.loads(data[20 : 20 + jl])
        off = 20 + jl
        bl = struct.unpack("<I", data[off : off + 4])[0]
        return j, [data[off + 8 : off + 8 + bl]]
    j = json.load(open(path, encoding="utf-8"))
    base = os.path.dirname(path)
    return j, [open(os.path.join(base, b["uri"]), "rb").read() for b in j.get("buffers", [])]


def accessor(j, bufs, i):
    a = j["accessors"][i]
    v = j["bufferViews"][a["bufferView"]]
    dt = np.dtype(COMPONENT[a["componentType"]])
    w = WIDTH[a["type"]]
    start = v.get("byteOffset", 0) + a.get("byteOffset", 0)
    stride = v.get("byteStride", dt.itemsize * w)
    raw = bufs[v["buffer"]]
    n = a["count"]
    if stride == dt.itemsize * w:
        out = np.frombuffer(raw, dt, n * w, start).reshape(n, w)
    else:
        rows = [np.frombuffer(raw, dt, w, start + k * stride) for k in range(n)]
        out = np.array(rows)
    return out.astype(np.float64) if dt == np.float32 else out.astype(np.int64)


def node_matrix(n):
    if "matrix" in n:
        return np.array(n["matrix"], dtype=np.float64).reshape(4, 4).T
    t = np.array(n.get("translation", [0, 0, 0]), dtype=np.float64)
    x, y, z, w = n.get("rotation", [0, 0, 0, 1])
    s = np.array(n.get("scale", [1, 1, 1]), dtype=np.float64)
    r = np.array(
        [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ]
    )
    m = np.eye(4)
    m[:3, :3] = r * s
    m[:3, 3] = t
    return m


def mesh_nodes(j):
    """(node name, mesh index, world matrix) for every node with a mesh."""
    out = []
    nodes = j.get("nodes", [])
    roots = set(range(len(nodes))) - {c for n in nodes for c in n.get("children", [])}

    def walk(i, parent):
        m = parent @ node_matrix(nodes[i])
        if "mesh" in nodes[i]:
            out.append((nodes[i].get("name", f"node{i}"), nodes[i]["mesh"], m))
        for c in nodes[i].get("children", []):
            walk(c, m)

    for r in sorted(roots):
        walk(r, np.eye(4))
    return out


def pieces(tris):
    """Groups triangles into pieces joined by shared vertices (union-find on vertex indices)."""
    parent = {}

    def find(a):
        while parent.setdefault(a, a) != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for t in tris:
        a = find(t[0])
        for v in t[1:]:
            b = find(v)
            if a != b:
                parent[b] = a
    groups = defaultdict(list)
    for k, t in enumerate(tris):
        groups[find(t[0])].append(k)
    return list(groups.values())


def check_primitive(pos, nrm, idx, flip):
    """(inverted triangles, triangles, inside-out pieces, closed pieces)."""
    tris = idx.reshape(-1, 3)
    if flip:
        tris = tris[:, [0, 2, 1]]
    a, b, c = pos[tris[:, 0]], pos[tris[:, 1]], pos[tris[:, 2]]
    g = np.cross(b - a, c - a)
    area2 = np.linalg.norm(g, axis=1)
    live = area2 > 1e-10
    inverted = 0
    if nrm is not None:
        nv = nrm[tris[:, 0]] + nrm[tris[:, 1]] + nrm[tris[:, 2]]
        inverted = int(np.sum((np.einsum("ij,ij->i", g, nv) < 0) & live))
    # Welded by position, so a piece split for its UVs or normals still closes.
    key = {}
    weld = np.array([key.setdefault(tuple(np.round(p, 5)), len(key)) for p in pos])
    wt = weld[tris]
    inside_out = closed = inward = 0
    for group in pieces([tuple(t) for t in wt]):
        if len(group) >= 4:
            centre = (a[group] + b[group] + c[group]).mean(axis=0) / 3.0
            mid = (a[group] + b[group] + c[group]) / 3.0
            facing = np.einsum("ij,ij->i", g[group], mid - centre)
            if np.sum(area2[group][facing < 0]) > 0.6 * np.sum(area2[group]):
                inward += 1
        edges = defaultdict(int)
        for k in group:
            t = wt[k]
            for e in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                edges[(min(e), max(e))] += 1
        if any(n != 2 for n in edges.values()):
            continue
        closed += 1
        vol = np.sum(np.einsum("ij,ij->i", a[group], np.cross(b[group], c[group]))) / 6.0
        if vol < -1e-9:
            inside_out += 1
    return inverted, len(tris), inside_out, closed, inward


def check(path, listed):
    j, bufs = load(path)
    bad = []
    for name, mi, m in mesh_nodes(j):
        flip = np.linalg.det(m[:3, :3]) < 0
        mesh = j["meshes"][mi]
        for pi, p in enumerate(mesh["primitives"]):
            if p.get("mode", 4) != 4 or "POSITION" not in p["attributes"]:
                continue
            pos = accessor(j, bufs, p["attributes"]["POSITION"])
            nrm = accessor(j, bufs, p["attributes"]["NORMAL"]) if "NORMAL" in p["attributes"] else None
            idx = accessor(j, bufs, p["indices"]).ravel() if "indices" in p else np.arange(len(pos))
            inv, n, io, closed, inward = check_primitive(pos, nrm, idx, flip)
            if inv or io or inward:
                mat = j["materials"][p["material"]].get("name", "?") if "material" in p else "-"
                bad.append((name, pi, mat, inv, n, io, closed, inward))
    for b in bad[:listed]:
        print(
            f"  {b[0]} prim {b[1]} ({b[2]}): {b[3]} of {b[4]} triangles inverted, {b[5]} of {b[6]} closed pieces inside"
            f" out, {b[7]} pieces facing inward"
        )
    if len(bad) > listed:
        print(f"  ... and {len(bad) - listed} more")
    return bad


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listed = int(sys.argv[sys.argv.index("--list") + 1]) if "--list" in sys.argv else 5
    if "--list" in sys.argv:
        args = [a for a in args if a != str(listed)]
    roots = args or ["assets"]
    files = []
    for r in roots:
        if os.path.isfile(r):
            files.append(r)
        for d, _, names in os.walk(r):
            files += [os.path.join(d, n) for n in sorted(names) if n.endswith((".glb", ".gltf"))]
    failed = 0
    for f in files:
        bad = check(f, 0)
        if any(b[3] or b[5] for b in bad):
            failed += 1
            inv = sum(b[3] for b in bad)
            io = sum(b[5] for b in bad)
            print(f"{f}: {inv} inverted triangles, {io} inside-out pieces, in {len(bad)} primitives")
            check(f, listed)
        elif bad:
            print(f"{f}: {sum(b[7] for b in bad)} pieces facing inward (to look at)")
            check(f, listed)
    print(f"{failed} of {len(files)} models with inverted faces")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
