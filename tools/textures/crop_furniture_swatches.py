"""Cut the cabin's texture sets out of the furniture atlas's swatch strip (assets/textures/furniture/, written by
tools/blender/build_furniture.py): each swatch is a tileable 1 m patch baked light and neutral, with its normal and ORM
maps, so the airliners' seats, carpet, panels and rubber wear the furniture's own surfaces (one implementation of how
fabric looks), tinted by the model. Writes assets/textures/aircraft/<key>.png, <key>_n.png, <key>_orm.png.

    python3 tools/textures/crop_furniture_swatches.py
"""
import json
import os

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "assets", "textures", "furniture")
OUT = os.path.join(ROOT, "assets", "textures", "aircraft")
KEYS = ["fabric", "trim", "rubber", "paint"]


def main():
    swatches = json.load(open(os.path.join(ROOT, "assets", "models", "furniture.json")))["swatches"]
    os.makedirs(OUT, exist_ok=True)
    maps = {"": "albedo.png", "_n": "normal.png", "_orm": "orm.png"}
    images = {k: Image.open(os.path.join(SRC, f)) for k, f in maps.items()}
    for key in KEYS:
        u0, v0, u1, v1 = swatches[key]
        for suffix, im in images.items():
            w, h = im.size
            box = (round(u0 * w), round(v0 * h), round(u1 * w), round(v1 * h))
            im.crop(box).save(os.path.join(OUT, f"{key}{suffix}.png"), optimize=False, compress_level=9)
        print(f"{key}: {box}")


main()
