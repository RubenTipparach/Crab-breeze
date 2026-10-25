"""How much of the outside a cockpit hides (openspec change cockpit-identity; a measurement instrument, CLAUDE.md
section 4). Renders the interior from EYE_pilot at the game's eye camera (85 degrees vertical, 16:9, straight ahead and
level, data/pilot/pilot.ron `view_fov_deg`), the outside screens (faces of the screen wall's material, `SCREEN_allview`) white and everything else black
(frame, dividers, hung MFDs, dash), and prints the share of the fighting band that is not outside: the full width of the
view (113 degrees), from 10 degrees under the eye line to 20 degrees over it. The dash under the knees and the roof
overhead are outside the band. On this measure the Foreman's Panorama hides 21.2 % (owner, 2026-10-25: "lets assume
forman cockput obscures about 20 percent of the view", "lets say 25% is abosolute max").

Usage: blender --background --factory-startup --python tools/blender/measure_view.py -- <interior.gltf> [out.png]
Prints `VIEW <file> hidden <percent> (whole picture <percent>)`."""
import math
import sys

import bpy
import numpy as np
from mathutils import Euler, Vector

FOV_V_DEG = 85.0          # the game's eye camera, vertical
W, H = 480, 270
BAND_DEG = (-10.0, 20.0)  # the fighting band, degrees from the eye line
BUDGET = 25.0             # the most of the band a cockpit may hide, percent

argv = sys.argv[sys.argv.index("--") + 1:]
src = argv[0]
out = argv[1] if len(argv) > 1 else None
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
sc = bpy.context.scene


def flat(name, rgb):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    e = nt.nodes.new("ShaderNodeEmission")
    e.inputs["Color"].default_value = (*rgb, 1)
    o = nt.nodes.new("ShaderNodeOutputMaterial")
    nt.links.new(e.outputs[0], o.inputs[0])
    return m


white, black = flat("outside", (1, 1, 1)), flat("cabin", (0, 0, 0))


n_out = 0
for ob in sc.objects:
    if ob.type != "MESH":
        continue
    for slot in ob.material_slots:            # the outside is the screen wall's material (SCREEN_allview)
        is_out = bool(slot.material) and slot.material.name.startswith("SCREEN_allview")
        n_out += is_out
        slot.material = white if is_out else black
for ob in list(sc.objects):
    if ob.type == "LIGHT":
        bpy.data.objects.remove(ob)
eye = bpy.data.objects.get("EYE_pilot")
assert eye, "no EYE_pilot"
assert n_out, "no SCREEN_allview faces"
cam = bpy.data.cameras.new("eye")
cam.sensor_fit = "VERTICAL"
cam.angle_y = math.radians(FOV_V_DEG)
cam.clip_start = 0.02
co = bpy.data.objects.new("eye", cam)
sc.collection.objects.link(co)
co.location = eye.matrix_world.translation
co.rotation_euler = Euler((math.radians(90), 0, math.radians(180)))   # level, looking ahead: glTF +Z, Blender -Y
sc.camera = co
w = bpy.data.worlds.new("void")
w.use_nodes = True
w.node_tree.nodes["Background"].inputs["Color"].default_value = (0, 0, 0, 1)
sc.world = w
sc.render.engine = "CYCLES"
sc.cycles.samples = 1
sc.cycles.use_denoising = False
sc.cycles.max_bounces = 0
sc.render.filter_size = 0.01
sc.render.resolution_x, sc.render.resolution_y, sc.render.resolution_percentage = W, H, 100
sc.view_settings.view_transform = "Standard"
path = out or "/tmp/measure_view.png"
sc.render.filepath = path
bpy.ops.render.render(write_still=True)
img = bpy.data.images.load(path)
px = np.array(img.pixels[:]).reshape(H, W, 4)[::-1, :, 0]      # Blender's rows run bottom up
rows = np.arange(H)
el = np.degrees(np.arctan((H / 2 - rows - 0.5) / (H / 2) * math.tan(math.radians(FOV_V_DEG / 2))))
band = (el >= BAND_DEG[0]) & (el <= BAND_DEG[1])
hidden = 100.0 * float((px[band] < 0.5).mean())
whole = 100.0 * float((px < 0.5).mean())
print(f"VIEW {src} hidden {hidden:.1f} (whole picture {whole:.1f}){' OVER BUDGET' if hidden > BUDGET else ''}", flush=True)
