"""Crab Breeze: bake every cockpit panel in cockpit_panels.PANELS to its texture maps.

  /opt/blender/blender --background --factory-startup --python tools/blender/bake_cockpit_panels.py -- \
      [--only dash_main,knee] [--scale 0.25] [--samples 32]

Writes assets/textures/cockpit/panels/<name>_{albedo,normal,orm,emit,beauty}.png and
panels.json (each panel's size in metres and pixels). Raw EXRs go to output/cockpit_raw/
(gitignored). --scale lowers the texel density for a quick look; a committed bake is scale 1.
"""

import argparse
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cockpit_kit as K  # noqa: E402
import cockpit_panels as CP  # noqa: E402


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="")
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--samples", type=int, default=32)
    a = ap.parse_args(argv)
    out = os.path.join(K.ROOT, "assets", "textures", "cockpit", "panels")
    raw = os.path.join(K.ROOT, "output", "cockpit_raw")
    os.makedirs(out, exist_ok=True)
    os.makedirs(raw, exist_ok=True)
    man_path = os.path.join(out, "panels.json")
    man = json.load(open(man_path)) if os.path.exists(man_path) else {}
    names = [n for n in CP.PANELS if not a.only or n in a.only.split(",")]
    for n in names:
        w, h, ppm, fn = CP.PANELS[n]
        t = time.time()
        wp, hp = K.bake_panel(fn, n, w, h, ppm * a.scale, a.samples, out, raw)
        man[n] = {"width_m": w, "height_m": h, "px_per_m": ppm * a.scale, "width_px": wp, "height_px": hp}
        print(f"PANEL {n}: {wp} x {hp} px in {time.time() - t:.1f} s", flush=True)
    with open(man_path, "w") as f:
        json.dump(dict(sorted(man.items())), f, indent=2)
        f.write("\n")


main()
