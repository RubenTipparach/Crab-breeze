"""Crab Breeze: the sky's cube map, cached from a Poly Haven HDRI (openspec change `glass-reflections`).

  python3 tools/textures/gen_sky_cube.py [--face 512]

Downloads one CC0 HDRI from Poly Haven (https://polyhaven.com/hdris; pinned below by name, size and
SHA-256, cached in output/hdri/), and bakes it into the six faces of a cube map, stacked top to
bottom (+X, -X, +Y, -Y, +Z, -Z) the way Bevy reinterprets a stacked image as a cube. The game loads
it once and filters it on the GPU into the world's reflections and sky light
(`GeneratedEnvironmentMapLight` on the pilot's eye camera), so glass, water and metal reflect a sky.

Two things are changed from the photograph, both because the game already draws them itself:
- the sun is clamped out (the game's own sun is a directional light and draws its own highlight;
  a second sun in the reflections would point the wrong way);
- below the horizon is a flat ground tone (the photograph's lower half is a stand-in; the city,
  the terrain and the water are the game's own).

Writes assets/textures/sky/sky_cube.hdr (Radiance RGBE, linear) and prints its luminance figures,
which data/world/world.ron's `sky_light` section is tuned against.
"""
import argparse
import hashlib
import os
import urllib.request

import numpy as np

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
OUT = os.path.join(ROOT, "assets", "textures", "sky", "sky_cube.hdr")
CACHE = os.path.join(ROOT, "output", "hdri")

# The HDRI: midday, partly cloudy, sky only (CC0, Poly Haven, by Greg Zaal).
HDRI = "kloofendal_48d_partly_cloudy_puresky"
HDRI_RES = "2k"
HDRI_URL = f"https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/{HDRI_RES}/{HDRI}_{HDRI_RES}.hdr"
HDRI_SHA256 = "5244534e9cf5b606f2ff513aa00ddb161b0a4826ffd88a0d3bd03ac29247d198"

# The sun: any texel brighter than this many times the sky's 99th percentile is scaled down to it.
SUN_CLAMP = 1.5
# The ground below the horizon: this fraction of the sky's mean radiance, in a grey-brown, blended
# in over this many degrees under the horizon.
GROUND_FRACTION = 0.22
GROUND_TINT = np.array([1.0, 0.95, 0.88])
HORIZON_BLEND_DEG = 4.0


def fetch():
    os.makedirs(CACHE, exist_ok=True)
    path = os.path.join(CACHE, os.path.basename(HDRI_URL))
    if not os.path.exists(path):
        print("downloading", HDRI_URL)
        urllib.request.urlretrieve(HDRI_URL, path)
    digest = hashlib.sha256(open(path, "rb").read()).hexdigest()
    if HDRI_SHA256 and digest != HDRI_SHA256:
        raise SystemExit(f"{path}: sha256 {digest}, expected {HDRI_SHA256}")
    print(f"{os.path.basename(path)} sha256 {digest}")
    return path


def read_hdr(path):
    """A Radiance RGBE file (flat or new-style run-length) as float32 RGB, rows top to bottom."""
    data = open(path, "rb").read()
    pos = 0
    while True:  # header lines up to the blank one
        end = data.index(b"\n", pos)
        line = data[pos:end]
        pos = end + 1
        if line == b"":
            break
    end = data.index(b"\n", pos)
    dims = data[pos:end].split()
    pos = end + 1
    assert dims[0] == b"-Y" and dims[2] == b"+X", dims
    h, w = int(dims[1]), int(dims[3])
    buf = np.frombuffer(data, dtype=np.uint8, offset=pos)
    rgbe = np.empty((h, w, 4), dtype=np.uint8)
    i = 0
    for y in range(h):
        if buf[i] == 2 and buf[i + 1] == 2 and buf[i + 2] < 128:
            i += 4
            for c in range(4):
                x = 0
                while x < w:
                    n = int(buf[i])
                    i += 1
                    if n > 128:
                        rgbe[y, x : x + n - 128, c] = buf[i]
                        x += n - 128
                        i += 1
                    else:
                        rgbe[y, x : x + n, c] = buf[i : i + n]
                        x += n
                        i += n
        else:
            rgbe[y] = buf[i : i + 4 * w].reshape(w, 4)
            i += 4 * w
    e = rgbe[..., 3].astype(np.int32)
    scale = np.where(e > 0, np.ldexp(1.0, e - 136), 0.0)
    return rgbe[..., :3].astype(np.float32) * scale[..., None].astype(np.float32)


def write_hdr(path, rgb):
    """Float RGB as a flat (uncompressed) Radiance RGBE file, rows top to bottom."""
    h, w, _ = rgb.shape
    m = rgb.max(axis=2)
    mant, exp = np.frexp(m)
    rgbe = np.zeros((h, w, 4), dtype=np.uint8)
    ok = m > 1e-32
    k = np.where(ok, mant * 256.0 / np.where(ok, m, 1.0), 0.0)
    rgbe[..., :3] = np.clip(rgb * k[..., None], 0, 255).astype(np.uint8)
    rgbe[..., 3] = np.where(ok, exp + 128, 0).astype(np.uint8)
    # A flat scanline starting 2, 2, <128 would read as run-length: nudge its red.
    first = rgbe[:, 0]
    clash = (first[:, 0] == 2) & (first[:, 1] == 2) & (first[:, 2] < 128)
    rgbe[clash, 0, 0] = 3
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n")
        f.write(f"-Y {h} +X {w}\n".encode())
        f.write(rgbe.tobytes())


def face_dirs(face, n):
    """World directions through each texel of cube face `face` (wgpu order +X -X +Y -Y +Z -Z)."""
    c = (np.arange(n) + 0.5) / n * 2.0 - 1.0
    u, v = np.meshgrid(c, c)  # u left to right, v top to bottom
    one = np.ones_like(u)
    d = [
        (one, -v, -u),
        (-one, -v, u),
        (u, one, v),
        (u, -one, -v),
        (u, -v, one),
        (-u, -v, -one),
    ][face]
    d = np.stack(d, axis=-1)
    return d / np.linalg.norm(d, axis=-1, keepdims=True)


def sample(eq, d):
    """Bilinear lookup of the equirectangular image along directions `d` (y up, -z at the centre)."""
    h, w, _ = eq.shape
    lon = np.arctan2(d[..., 0], -d[..., 2])
    lat = np.arcsin(np.clip(d[..., 1], -1.0, 1.0))
    x = (lon / (2 * np.pi) + 0.5) * w - 0.5
    y = (0.5 - lat / np.pi) * h - 0.5
    x0 = np.floor(x).astype(int)
    y0 = np.clip(np.floor(y).astype(int), 0, h - 2)
    fx = (x - x0)[..., None]
    fy = np.clip(y - y0, 0, 1)[..., None]
    x0 %= w
    x1 = (x0 + 1) % w
    top = eq[y0, x0] * (1 - fx) + eq[y0, x1] * fx
    bot = eq[y0 + 1, x0] * (1 - fx) + eq[y0 + 1, x1] * fx
    return top * (1 - fy) + bot * fy


def lum(rgb):
    return rgb @ np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--face", type=int, default=512, help="cube face size in pixels (a power of two)")
    args = ap.parse_args()
    assert args.face & (args.face - 1) == 0, "the face size must be a power of two"
    eq = read_hdr(fetch())
    h = eq.shape[0]
    sky = eq[: h // 2]
    p99 = float(np.percentile(lum(sky), 99.0))
    cap = p99 * SUN_CLAMP
    l = lum(eq)
    over = l > cap
    eq[over] *= (cap / l[over])[:, None]
    sky_mean = eq[: h // 2].reshape(-1, 3).mean(axis=0)
    ground = sky_mean.mean() * GROUND_FRACTION * GROUND_TINT
    faces = []
    for f in range(6):
        d = face_dirs(f, args.face)
        rgb = sample(eq, d)
        below = np.clip(-np.degrees(np.arcsin(np.clip(d[..., 1], -1, 1))) / HORIZON_BLEND_DEG, 0.0, 1.0)[..., None]
        faces.append(rgb * (1 - below) + ground * below)
    cube = np.concatenate(faces, axis=0).astype(np.float32)
    write_hdr(OUT, cube)
    print(
        f"wrote {os.path.relpath(OUT, ROOT)}: {args.face} px faces, sky mean luminance {lum(sky_mean):.3f}, "
        f"p99 {p99:.3f}, sun clamped at {cap:.3f} ({int(over.sum())} texels), ground {lum(ground):.3f}"
    )


if __name__ == "__main__":
    main()
