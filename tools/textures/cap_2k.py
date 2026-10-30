"""No texture over 2048 px (CLAUDE.md section 7; owner, 2026-10-28: "No 4k textures 2k only. Compress all 4k").

    python tools/textures/cap_2k.py            # shrink every PNG and JPEG under assets/ that is over 2048 px
    python tools/textures/cap_2k.py --check    # list every image over 2048 px, files and those inside .glb; exit 1 if any

An image is scaled so its longer side is 2048 px, keeping its aspect (a 4096 x 1024 sheet becomes 2048 x 512), with a
Lanczos filter; a normal map (its name holds "normal" or ends "_n") is averaged as vectors and renormalised, so its slopes keep
their length. A JPEG is written back at quality 95, a PNG losslessly in its own mode. UVs are fractions of the image,
so every model that uses one draws it the same, at a quarter of the memory. The generators make their maps at
2048 px themselves (build_cockpit.py --size, build_furniture.py MAX_MAP_PX, gen_mech_plating.mjs MAX_MAP_PX,
export_fortress_glb.mjs maxTextureSize); this is for maps already made, and the check keeps them so.
"""

import io
import json
import os
import struct
import sys

import numpy as np
from PIL import Image

MAX_PX = 2048
ROOT = "assets"


def shrink(im, normal):
    w, h = im.size
    k = MAX_PX / max(w, h)
    size = (max(1, round(w * k)), max(1, round(h * k)))
    if not normal:
        return im.resize(size, Image.LANCZOS)
    rgb = im.convert("RGB")
    v = np.asarray(rgb, dtype=np.float32) / 127.5 - 1.0
    chans = [Image.fromarray(v[..., c]).resize(size, Image.BOX) for c in range(3)]
    n = np.stack([np.asarray(c, dtype=np.float32) for c in chans], axis=-1)
    n /= np.maximum(np.linalg.norm(n, axis=-1, keepdims=True), 1e-6)
    out = Image.fromarray(np.clip((n + 1.0) * 127.5 + 0.5, 0, 255).astype(np.uint8), "RGB")
    if im.mode == "RGBA":
        out.putalpha(im.getchannel("A").resize(size, Image.LANCZOS))
    return out


def files():
    for root, _, names in os.walk(ROOT):
        for n in sorted(names):
            if n.lower().endswith((".png", ".jpg", ".jpeg")):
                yield os.path.join(root, n)


def glb_images(path):
    data = open(path, "rb").read()
    jl = struct.unpack("<I", data[12:16])[0]
    j = json.loads(data[20 : 20 + jl])
    off = 20 + jl
    bl = struct.unpack("<I", data[off : off + 4])[0]
    b = data[off + 8 : off + 8 + bl]
    for i, im in enumerate(j.get("images", [])):
        if "bufferView" in im:
            bv = j["bufferViews"][im["bufferView"]]
            s = bv.get("byteOffset", 0)
            yield i, Image.open(io.BytesIO(b[s : s + bv["byteLength"]])).size


def check():
    over = []
    for p in files():
        with Image.open(p) as im:
            if max(im.size) > MAX_PX:
                over.append(f"{p} {im.size[0]}x{im.size[1]}")
    for root, _, names in os.walk(ROOT):
        for n in sorted(names):
            if n.lower().endswith(".glb"):
                p = os.path.join(root, n)
                for i, (w, h) in glb_images(p):
                    if max(w, h) > MAX_PX:
                        over.append(f"{p} image {i} {w}x{h}")
    for o in over:
        print("over 2048 px:", o)
    print(f"{len(over)} images over {MAX_PX} px")
    return 1 if over else 0


def cap_glb(path):
    """Shrinks the images inside one .glb over 2048 px and writes it back: the binary chunk rebuilt with every buffer
    view in its order, each on a 4-byte boundary as glTF wants, the JSON's offsets and lengths moved to match."""
    data = open(path, "rb").read()
    jl = struct.unpack("<I", data[12:16])[0]
    j = json.loads(data[20 : 20 + jl])
    off = 20 + jl
    bl = struct.unpack("<I", data[off : off + 4])[0]
    b = data[off + 8 : off + 8 + bl]
    views = j["bufferViews"]
    blobs = [b[v.get("byteOffset", 0) : v.get("byteOffset", 0) + v["byteLength"]] for v in views]
    changed = []
    for i, im in enumerate(j.get("images", [])):
        if "bufferView" not in im:
            continue
        k = im["bufferView"]
        img = Image.open(io.BytesIO(blobs[k]))
        img.load()
        if max(img.size) <= MAX_PX:
            continue
        name = (im.get("name") or "").lower()
        out = shrink(img, "normal" in name or name.endswith("_n"))
        if out.mode != img.mode:
            out = out.convert(img.mode)
        buf = io.BytesIO()
        if im.get("mimeType") == "image/jpeg":
            out.convert("RGB").save(buf, "JPEG", quality=95, subsampling=0)
        else:
            out.save(buf, "PNG", optimize=True)
        blobs[k] = buf.getvalue()
        changed.append(f"image {i} {img.size[0]}x{img.size[1]} -> {out.size[0]}x{out.size[1]}")
    if not changed:
        return []
    body = bytearray()
    for v, blob in zip(views, blobs):
        while len(body) % 4:
            body.append(0)
        v["byteOffset"] = len(body)
        v["byteLength"] = len(blob)
        body += blob
    while len(body) % 4:
        body.append(0)
    j["buffers"][0]["byteLength"] = len(body)
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    total = 12 + 8 + len(js) + 8 + len(body)
    with open(path, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, total))
        f.write(struct.pack("<II", len(js), 0x4E4F534A) + js)
        f.write(struct.pack("<II", len(body), 0x004E4942) + bytes(body))
    return changed


def cap():
    done = 0
    for root, _, names in os.walk(ROOT):
        for n in sorted(names):
            if n.lower().endswith(".glb"):
                p = os.path.join(root, n)
                for c in cap_glb(p):
                    print(f"{p}: {c}")
                    done += 1
    for p in files():
        with Image.open(p) as im:
            im.load()
            if max(im.size) <= MAX_PX:
                continue
            before = im.size
            stem = os.path.splitext(os.path.basename(p).lower())[0]
            out = shrink(im, "normal" in stem or stem.endswith("_n"))
            if out.mode != im.mode and im.mode in ("L", "LA", "RGB", "RGBA", "I;16"):
                out = out.convert(im.mode)
        if p.lower().endswith((".jpg", ".jpeg")):
            out.convert("RGB").save(p, quality=95, subsampling=0)
        else:
            out.save(p, optimize=True)
        done += 1
        print(f"{p}: {before[0]}x{before[1]} -> {out.size[0]}x{out.size[1]}")
    print(f"{done} images shrunk to {MAX_PX} px")


if __name__ == "__main__":
    sys.exit(check() if "--check" in sys.argv else cap())
