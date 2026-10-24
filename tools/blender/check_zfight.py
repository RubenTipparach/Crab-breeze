"""Find z-fighting on a mech glb: faces of different pieces that lie in the same plane, face the same way and overlap.

    /opt/blender/blender --background --factory-startup --python tools/blender/check_zfight.py -- assets/models/mech_<id>.glb ...

CLAUDE.md section 10: "No z-fighting, ever". Two surfaces closer than TOL metres along their shared normal, facing the
same way, whose outlines overlap by more than AREA square metres, are reported with where they are, so a generator can
be fixed rather than a screenshot argued over. Pieces are the connected islands of each mesh (the kit merges a joint's
parts into one mesh per material, so islands are what the generator placed). Exits 1 if anything is found.
"""
import sys
import bmesh
import bpy
from mathutils import Vector

TOL, AREA = 0.006, 0.01


def _clip(subject, clip):
    """Sutherland-Hodgman: the convex polygon subject clipped by the convex polygon clip (both counter-clockwise)."""
    out = subject
    for i in range(len(clip)):
        a, b = clip[i - 1], clip[i]
        inp, out = out, []
        if not inp:
            break
        side = lambda p: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        for j in range(len(inp)):
            p, q = inp[j - 1], inp[j]
            sp, sq = side(p), side(q)
            if sq >= 0:
                if sp < 0:
                    t = sp / (sp - sq)
                    out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
                out.append(q)
            elif sp >= 0:
                t = sp / (sp - sq)
                out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def _area(P):
    return 0.5 * sum(P[i - 1][0] * P[i][1] - P[i][0] * P[i - 1][1] for i in range(len(P)))


def _ccw(P):
    return P if _area(P) > 0 else list(reversed(P))


def islands(bm):
    seen, out = set(), []
    for f in bm.faces:
        if f.index in seen:
            continue
        stack, isl = [f], []
        seen.add(f.index)
        while stack:
            g = stack.pop()
            isl.append(g)
            for e in g.edges:
                for h in e.link_faces:
                    if h.index not in seen:
                        seen.add(h.index)
                        stack.append(h)
        out.append(isl)
    return out


def check(path):
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    bpy.ops.import_scene.gltf(filepath=path)
    bpy.context.view_layer.update()
    faces = []     # (normal, offset, verts2d basis, piece id, obj name, material)
    pid = 0
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        bm = bmesh.new()
        bm.from_mesh(o.data)
        bm.transform(o.matrix_world)
        bm.normal_update()
        bm.faces.ensure_lookup_table()
        for isl in islands(bm):
            pid += 1
            for f in isl:
                if f.calc_area() < 1e-4:
                    continue
                n = f.normal.copy()
                mat = o.material_slots[f.material_index].material.name if o.material_slots else ""
                faces.append((n, n.dot(f.verts[0].co), [v.co.copy() for v in f.verts], pid, o.name, mat))
        bm.free()
    # bucket by plane: quantised normal and offset
    buckets = {}
    for i, (n, d, vs, p, on, m) in enumerate(faces):
        key = (round(n.x, 2), round(n.y, 2), round(n.z, 2), round(d / TOL))
        for dd in (-1, 0, 1):
            buckets.setdefault((key[0], key[1], key[2], key[3] + dd), []).append(i)
    hits = {}
    for key, idx in buckets.items():
        if len(idx) < 2:
            continue
        for a in range(len(idx)):
            for b in range(a + 1, len(idx)):
                i, j = idx[a], idx[b]
                if i > j:
                    i, j = j, i
                if (i, j) in hits:
                    continue
                fi, fj = faces[i], faces[j]
                if fi[3] == fj[3] or fi[0].dot(fj[0]) < 0.999 or abs(fi[1] - fj[1]) > TOL:
                    continue
                n = fi[0]
                u = (Vector((1, 0, 0)) if abs(n.x) < 0.9 else Vector((0, 1, 0))).cross(n).normalized()
                w = n.cross(u)
                A = [(v.dot(u), v.dot(w)) for v in fi[2]]
                B = [(v.dot(u), v.dot(w)) for v in fj[2]]
                ox = min(max(x for x, _ in A), max(x for x, _ in B)) - max(min(x for x, _ in A), min(x for x, _ in B))
                oy = min(max(y for _, y in A), max(y for _, y in B)) - max(min(y for _, y in A), min(y for _, y in B))
                if ox <= 0 or oy <= 0 or ox * oy <= AREA:
                    continue
                inter = _clip(_ccw(A), _ccw(B))
                ar = abs(_area(inter)) if len(inter) > 2 else 0.0
                if ar > AREA:
                    c = sum(fi[2], Vector()) / len(fi[2])
                    hits[(i, j)] = (ar, fi[4], fi[5], fj[4], fj[5], c, n)
    print("ZFIGHT", path, len(hits))
    worst = sorted(hits.values(), key=lambda h: -h[0])[:40]
    for area, o1, m1, o2, m2, c, n in worst:
        print(f"  {area:6.2f} m2  {o1}[{m1}] / {o2}[{m2}]  at glTF ({c.x:.2f}, {c.z:.2f}, {-c.y:.2f})"
              f"  facing ({n.x:.2f}, {n.z:.2f}, {-n.y:.2f})")
    return len(hits)


argv = sys.argv[sys.argv.index("--") + 1:]
total = sum(check(p) for p in argv)
sys.exit(1 if total else 0)
