"""Checks every furniture model reads its own baked tile of the shared atlas.

Each model's TEXCOORD_0 is sampled in assets/textures/furniture/albedo.png, the image itself, not the generator's
arithmetic: the 2048 px cap once halved every model's UVs against the atlas and the generator's own check, sharing the
mistake, passed it (owner, 2026-10-28: "something got messed up with the furniture textures"). A model fails when its
UVs leave its atlas_rect in assets/models/furniture.json or when too many land on the atlas's unbaked black.

    python tools/build/check_furniture_atlas.py
"""

import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
MAX_BLACK = 0.2  # share of a model's UVs allowed on unbaked atlas (island gutters); a misplaced model is far above it
MIN_COVER = 0.95  # share of the baked atlas the items' and swatches' rects must cover: rects shrunk with the UVs do not


def glb(path):
    data = path.read_bytes()
    n = struct.unpack("<I", data[12:16])[0]
    js = json.loads(data[20:20 + n])
    off = 20 + n
    length = struct.unpack("<I", data[off:off + 4])[0]
    return js, data[off + 8:off + 8 + length]


def uvs(js, binary):
    out = []
    for mesh in js["meshes"]:
        for prim in mesh["primitives"]:
            a = js["accessors"][prim["attributes"]["TEXCOORD_0"]]
            v = js["bufferViews"][a["bufferView"]]
            start = v.get("byteOffset", 0) + a.get("byteOffset", 0)
            out.append(np.frombuffer(binary, np.float32, a["count"] * 2, start).reshape(-1, 2))
    return np.concatenate(out)


def main():
    manifest = json.loads((ROOT / "assets/models/furniture.json").read_text(encoding="utf-8"))
    albedo = np.asarray(Image.open(ROOT / "assets/textures/furniture/albedo.png").convert("RGB")).astype(np.int32)
    h, w, _ = albedo.shape
    bad = []
    for name, item in sorted(manifest["items"].items()):
        uv = uvs(*glb(ROOT / "assets/models" / item["file"]))
        u0, v0, u1, v1 = item["atlas_rect"]
        outside = ((uv[:, 0] < u0) | (uv[:, 0] > u1) | (uv[:, 1] < v0) | (uv[:, 1] > v1)).mean()
        x = np.clip((uv[:, 0] * w).astype(int), 0, w - 1)
        y = np.clip((uv[:, 1] * h).astype(int), 0, h - 1)
        black = (albedo[y, x].sum(axis=1) < 30).mean()
        ok = outside == 0 and black <= MAX_BLACK
        print(f"{'ok ' if ok else 'BAD'} {name}: {outside:.1%} outside its rect, {black:.1%} on unbaked atlas")
        if not ok:
            bad.append(name)
    # The rects themselves must lie on what was baked: every baked texel inside some item's or swatch's rect.
    covered = np.zeros((h, w), bool)
    for u0, v0, u1, v1 in [i["atlas_rect"] for i in manifest["items"].values()] + list(manifest["swatches"].values()):
        covered[int(v0 * h):int(np.ceil(v1 * h)), int(u0 * w):int(np.ceil(u1 * w))] = True
    baked = albedo.sum(axis=2) >= 30
    cover = (baked & covered).sum() / max(1, baked.sum())
    print(f"the rects cover {cover:.1%} of the baked atlas")
    if cover < MIN_COVER:
        bad.append("the manifest's rects")
    if bad:
        print(f"FAIL: {', '.join(bad)} do not read their own tile")
        sys.exit(1)
    print(f"all {len(manifest['items'])} furniture models read their own tile")


if __name__ == "__main__":
    main()
