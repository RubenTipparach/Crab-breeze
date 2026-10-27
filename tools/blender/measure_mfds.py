"""How well the pilot sees each MFD from the seat (a measurement instrument, CLAUDE.md section 4; openspec change
`mfd-visibility`). From EYE_pilot, at the game's seated eye camera (data/pilot/pilot.ron `view_fov_deg` vertical, 16:9,
level, straight ahead), for each of the six screens (`HUD_mfd_0..2_screen` hung, `SCREEN_left/center/right_screen` on
the dash): the share of a 9 x 9 grid of points on the screen that is inside the picture and not hidden by anything
nearer (a ray from the eye meets the screen first), and how square it looks (its height over its width as drawn; 1 is
square, the pages are square). It also reads the screen's true shape in the model: its width and height in metres, how
far its corners are from right angles, and how far its face is turned from the eye (0: square on). Prints one
`MFD <file> <screen> visible <%> aspect <h/w> size <w>x<h> m corners <deg> facing <deg>` line each.

Usage: blender --background --factory-startup --python tools/blender/measure_mfds.py -- <interior.gltf>"""
import math
import os
import re
import sys

import bpy
from mathutils import Vector

_PILOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "data", "pilot", "pilot.ron")
FOV_V_DEG = float(re.search(r"view_fov_deg:\s*([0-9.]+)", open(_PILOT).read()).group(1))
ASPECT = 16 / 9
SCREENS = ["HUD_mfd_0_screen", "HUD_mfd_1_screen", "HUD_mfd_2_screen",
           "SCREEN_left_screen", "SCREEN_center_screen", "SCREEN_right_screen"]
N = 9
BEZEL_M = 0.035       # the bezel's depth in front of its screen, metres

argv = sys.argv[sys.argv.index("--") + 1:]
src = argv[0]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
sc = bpy.context.scene
dg = bpy.context.evaluated_depsgraph_get()
eye = bpy.data.objects["EYE_pilot"].matrix_world.translation.copy()
# glTF +Z ahead is Blender -Y; Blender +Z is up.
fwd, up = Vector((0, -1, 0)), Vector((0, 0, 1))
right = fwd.cross(up)
tv = math.tan(math.radians(FOV_V_DEG) / 2)
th = tv * ASPECT


def to_screen(p):
    """The point p on the picture, (-1..1, -1..1) inside it, or None behind the eye."""
    d = p - eye
    z = d.dot(fwd)
    if z <= 1e-6:
        return None
    return (d.dot(right) / z / th, d.dot(up) / z / tv)


def mesh_of(name):
    ob = bpy.data.objects.get(name)
    if ob and ob.type == "MESH":
        return ob
    if ob:
        for c in ob.children_recursive:
            if c.type == "MESH":
                return c
    return None


for name in SCREENS:
    ob = mesh_of(name)
    if ob is None:
        print(f"MFD {src} {name} missing", flush=True)
        continue
    pts = [ob.matrix_world @ v.co for v in ob.data.vertices]
    # The quad's corners: the extreme points along its two edge directions.
    c = sum(pts, Vector()) / len(pts)
    a = (pts[1] - pts[0]).normalized()
    n = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalized()
    b = n.cross(a)
    us = [(p - c).dot(a) for p in pts]
    vs = [(p - c).dot(b) for p in pts]
    u0, u1, v0, v1 = min(us), max(us), min(vs), max(vs)
    seen = 0
    for i in range(N):
        for j in range(N):
            p = c + a * (u0 + (u1 - u0) * (i + 0.5) / N) + b * (v0 + (v1 - v0) * (j + 0.5) / N)
            s = to_screen(p)
            if s is None or abs(s[0]) > 1 or abs(s[1]) > 1:
                continue
            d = p - eye
            hit, loc, _, _, hob, _ = sc.ray_cast(dg, eye, d.normalized(), distance=d.length + 0.01)
            # Seen when the ray meets the screen first, or meets only its own bezel's rim: anything within BEZEL_M in front
            # of the screen is the frame round it, which stands 3 cm proud of the glass and covers a few millimetres of
            # its edge from an angle, as any monitor's does. Anything nearer the eye hides it.
            if not hit or hob.name == ob.name or (loc - p).length < BEZEL_M:
                seen += 1
            elif os.environ.get("MFD_DEBUG"):
                print(f"  hidden {name} ({i},{j}) by {hob.name if hit else None} at {loc if hit else None}")
    corners = [c + a * u + b * v for u, v in ((u0, v0), (u1, v0), (u1, v1), (u0, v1))]
    sp = [to_screen(q) for q in corners]
    if all(sp):
        px = [(x * th, y * tv) for x, y in sp]       # tangent-plane units, the same scale both ways
        w = ((Vector(px[1]) - Vector(px[0])).length + (Vector(px[2]) - Vector(px[3])).length) / 2
        h = ((Vector(px[3]) - Vector(px[0])).length + (Vector(px[2]) - Vector(px[1])).length) / 2
        ar = h / w if w > 0 else 0.0
        # the quad's edge order sets which of its axes runs up the screen: height is the one nearer the world's up
        if abs(a.dot(up)) > abs(b.dot(up)) and ar > 0:
            ar = 1 / ar
    else:
        ar = 0.0
    # the true shape: the quad's own corners in order round its centre, their angles, its size along its edges
    ring = sorted(set((round((p - c).dot(a), 5), round((p - c).dot(b), 5)) for p in pts), key=lambda q: math.atan2(q[1], q[0]))
    off = 0.0
    for k in range(len(ring)):
        p0, p1, p2 = (Vector(ring[k - 1]), Vector(ring[k]), Vector(ring[(k + 1) % len(ring)]))
        off = max(off, abs(math.degrees((p0 - p1).angle(p2 - p1)) - 90.0))
    wide, tall = (u1 - u0, v1 - v0) if abs(a.dot(up)) <= abs(b.dot(up)) else (v1 - v0, u1 - u0)
    facing = math.degrees(abs(n.dot((eye - c).normalized())) and math.acos(min(1.0, abs(n.dot((eye - c).normalized())))))
    print(f"MFD {os.path.basename(src)} {name} visible {100 * seen / (N * N):.0f} aspect {ar:.2f} size {wide:.3f}x{tall:.3f} m "
          f"corners {off:.1f} facing {facing:.1f}", flush=True)
