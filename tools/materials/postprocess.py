#!/usr/bin/env python3
"""Turn Material Maker's renders into Crab Breeze's effect textures (openspec changes `blast-scorch` and `fx-sprites`).

Reads `data/materials/materials.ron` (the one list of materials), takes each one's maps from the render directory
(`tools/materials/raw/` by default: `<name>_albedo.png`, `_normal.png`, `_orm.png`), resizes them to the listed size with
Lanczos, and writes `assets/textures/fx/<name>/<map>.png`. Material Maker's ORM is already packed the glTF way (R
occlusion, G roughness, B metal), so it is copied as is. Writes a `SOURCE.md` beside the textures and the contact sheet
`docs/screenshots/materials/fx-contact-sheet.png` (each material over grass, sky and a dark ground), and prints a
digest of every file written.

Run after `export_materials.sh` renders, or alone: python3 tools/materials/postprocess.py [render dir]

The houses' surface sets (rows with a `dir`, `export_house_materials.sh`) go through `--houses` instead: maps to
`assets/textures/<dir>/` and their contact sheet `docs/screenshots/house_materials/sheet.png`, each set's albedo tiled
2 x 2, the same lit flat under a light from above, and a lit sphere (the maps read the way the game reads them).

    python3 tools/materials/postprocess.py tools/materials/raw/houses --houses
"""
import hashlib
import os
import re
import sys

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "textures", "fx")
SHEET = os.path.join(ROOT, "docs", "screenshots", "materials", "fx-contact-sheet.png")


def materials():
    """The material rows of materials.ron, each a dict (a small reader for this file's flat tuples)."""
    text = open(os.path.join(ROOT, "data", "materials", "materials.ron")).read()
    text = re.sub(r"//[^\n]*", "", text)
    rows = []
    for body in re.findall(r"\(\s*(name:[^()]*)\)", text):
        row = {}
        for key, val in re.findall(r"(\w+):\s*(\"[^\"]*\"|\[[^\]]*\]|[-\d.]+)", body):
            if val.startswith('"'):
                row[key] = val.strip('"')
            elif val.startswith("["):
                row[key] = re.findall(r"\"([^\"]*)\"", val)
            else:
                row[key] = float(val) if "." in val else int(val)
        rows.append(row)
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    raw = args[0] if args else os.path.join(ROOT, "tools", "materials", "raw")
    rev = os.environ.get("MM_REV", "174f15e")
    if "--houses" in sys.argv:
        return houses(raw, rev)
    tiles = []
    for m in materials():
        if "dir" in m:  # a house surface set: --houses
            continue
        dst = os.path.join(OUT, m["name"])
        os.makedirs(dst, exist_ok=True)
        for kind in m["maps"]:
            img = Image.open(os.path.join(raw, f"{m['name']}_{kind}.png"))
            img = img.convert("RGBA" if kind == "albedo" else "RGB").resize((m["size_px"],) * 2, Image.LANCZOS)
            path = os.path.join(dst, f"{kind}.png")
            img.save(path, optimize=True)
            print(f"{hashlib.sha256(open(path, 'rb').read()).hexdigest()[:12]}  {os.path.relpath(path, ROOT)}")
        with open(os.path.join(dst, "SOURCE.md"), "w") as f:
            f.write(f"# {m['name']}\n\nRendered from `tools/materials/ptex/{m['name']}.ptex` by Material Maker {rev}, headless "
                    f"on lavapipe (`material-maker-headless` skill), then `tools/materials/postprocess.py`: maps "
                    f"{', '.join(m['maps'])} at {m['size_px']} px. Use: {m['use']}.\n")
        albedo = Image.open(os.path.join(dst, "albedo.png")).convert("RGBA").resize((384, 384), Image.LANCZOS)
        row = []
        for bg in ((70, 96, 40), (110, 150, 200), (22, 22, 26)):
            row.append(Image.alpha_composite(Image.new("RGBA", albedo.size, bg + (255,)), albedo))
        if "normal" in m["maps"]:
            row.append(Image.open(os.path.join(dst, "normal.png")).convert("RGBA").resize((384, 384), Image.LANCZOS))
        tiles.append(row)
    os.makedirs(os.path.dirname(SHEET), exist_ok=True)
    sheet = Image.new("RGB", (4 * 384, len(tiles) * 384), (40, 40, 40))
    for y, row in enumerate(tiles):
        for x, t in enumerate(row):
            sheet.paste(t.convert("RGB"), (x * 384, y * 384))
    sheet.save(SHEET, optimize=True)
    print("contact sheet", os.path.relpath(SHEET, ROOT))


# --- the houses' surface sets -------------------------------------------------------------------------------------

HOUSE_SHEET = os.path.join(ROOT, "docs", "screenshots", "house_materials", "sheet.png")


def lit(albedo, normal, orm, n_world, t_world, b_world, light):
    """Shade texels with the maps the way the game reads them: sRGB albedo, a tangent-space normal (OpenGL: green toward
    the image top, which in the game is the -v side), ORM packed the glTF way. n/t/b_world are the surface frame per
    texel (t along +u, b along +v); Lambert plus a GGX highlight from one light and a dim sky."""
    import numpy as np
    a = (albedo[..., :3] / 255.0) ** 2.2
    nt = normal[..., :3] / 127.5 - 1.0
    occ, rough, metal = orm[..., 0:1] / 255.0, np.clip(orm[..., 1:2] / 255.0, 0.05, 1.0), orm[..., 2:3] / 255.0
    n = t_world * nt[..., 0:1] - b_world * nt[..., 1:2] + n_world * nt[..., 2:3]
    n /= np.linalg.norm(n, axis=-1, keepdims=True) + 1e-6
    v = np.array([0.0, 0.0, 1.0])
    l = np.array(light) / np.linalg.norm(light)
    h = (l + v) / np.linalg.norm(l + v)
    nl = np.clip((n * l).sum(-1, keepdims=True), 0.0, 1.0)
    nh = np.clip((n * h).sum(-1, keepdims=True), 0.0, 1.0)
    al = rough ** 4
    d = al / (np.pi * (nh * nh * (al - 1.0) + 1.0) ** 2)
    f0 = 0.04 * (1.0 - metal) + a * metal
    spec = d * f0 * 0.25 * nl
    col = (a * (1.0 - metal) * nl * 1.5 + spec * 1.5 + (a * (1.0 - metal) * 0.3 + f0 * 0.55) * occ) * occ
    col = np.clip(col, 0.0, 1.0) ** (1.0 / 2.2)
    return (col * 255.0).astype(np.uint8)


def house_cells(dst, cell):
    """One set's three contact-sheet cells: albedo 2 x 2, the same lit flat (world up at the top), a lit sphere."""
    import numpy as np
    maps = {k: np.asarray(Image.open(os.path.join(dst, f"{k}.png")).convert("RGB"), dtype=np.float64)
            for k in ("albedo", "normal", "orm")}
    half = cell // 2
    tile = {k: np.asarray(Image.fromarray(v.astype(np.uint8)).resize((half, half), Image.LANCZOS), dtype=np.float64)
            for k, v in maps.items()}
    tiled = {k: np.tile(v, (2, 2, 1)) for k, v in tile.items()}
    albedo = Image.fromarray(tiled["albedo"].astype(np.uint8))
    # The game's v grows up the wall and its texture rows grow with v, so flip the rows to put world up at the top.
    flat = {k: v[::-1] for k, v in tiled.items()}
    one = np.ones((cell, cell, 1))
    swatch = lit(flat["albedo"], flat["normal"], flat["orm"], one * [0, 0, 1], one * [1, 0, 0], one * [0, 1, 0],
                 [-0.35, 0.75, 0.55])
    # A sphere 1 tile across, so each tile wraps it twice around and twice from pole to pole.
    ys, xs = np.mgrid[0:cell, 0:cell]
    x = (xs + 0.5) / cell * 2.0 - 1.0
    y = 1.0 - (ys + 0.5) / cell * 2.0
    inside = x * x + y * y < 1.0
    z = np.sqrt(np.clip(1.0 - x * x - y * y, 0.0, 1.0))
    phi, theta = np.arctan2(x, z), np.arcsin(np.clip(y, -1.0, 1.0))
    u, v = phi / np.pi, theta / np.pi * 2.0
    size = maps["albedo"].shape[0]
    cols = (np.floor(u * size) % size).astype(int)
    rows = (np.floor(v * size) % size).astype(int)
    s = {k: m[rows, cols] for k, m in maps.items()}
    n = np.stack([x, y, z], -1)
    t = np.stack([np.cos(phi), np.zeros_like(phi), -np.sin(phi)], -1)
    b = np.stack([-np.sin(theta) * np.sin(phi), np.cos(theta), -np.sin(theta) * np.cos(phi)], -1)
    sphere = lit(s["albedo"], s["normal"], s["orm"], n, t, b, [-0.45, 0.6, 0.65])
    sphere[~inside] = (34, 34, 38)
    return [albedo, Image.fromarray(swatch), Image.fromarray(sphere)]


def houses(raw, rev):
    """Write every house surface set and the houses' contact sheet."""
    from PIL import ImageDraw
    sets = [m for m in materials() if "dir" in m]
    cell, label, per_row = 288, 22, 3
    cells = []
    for m in sets:
        dst = os.path.join(ROOT, "assets", "textures", m["dir"])
        os.makedirs(dst, exist_ok=True)
        for kind in m["maps"]:
            img = Image.open(os.path.join(raw, f"{m['name']}_{kind}.png")).convert("RGB")
            img = img.resize((m["size_px"],) * 2, Image.LANCZOS)
            path = os.path.join(dst, f"{kind}.png")
            img.save(path, optimize=True)
            print(f"{hashlib.sha256(open(path, 'rb').read()).hexdigest()[:12]}  {os.path.relpath(path, ROOT)}")
        with open(os.path.join(dst, "SOURCE.md"), "w") as f:
            f.write(f"# {m['dir']}\n\nRendered from `tools/materials/ptex/{m['name']}.ptex` (written by "
                    f"`tools/materials/build_house_ptex.py`) by Material Maker {rev}, headless on lavapipe "
                    f"(`material-maker-headless` skill), at 2048 px, then `tools/materials/postprocess.py --houses`: "
                    f"maps {', '.join(m['maps'])} at {m['size_px']} px, one tile {m['tile_m']} m. Surfaces: "
                    f"{', '.join(m['surfaces'])}. {m['use']}.\n")
        cells.append((m, house_cells(dst, cell)))
    rows = (len(cells) + per_row - 1) // per_row
    w = per_row * (3 * cell + 24) + 24
    sheet = Image.new("RGB", (w, rows * (cell + label + 16) + 16), (24, 24, 28))
    draw = ImageDraw.Draw(sheet)
    for i, (m, imgs) in enumerate(cells):
        x0 = 24 + (i % per_row) * (3 * cell + 24)
        y0 = 16 + (i // per_row) * (cell + label + 16)
        draw.text((x0, y0 + 4), f"{m['dir'].split('/')[-1]}  {m['tile_m']} m  ({', '.join(m['surfaces'])})", fill=(230, 230, 230))
        for k, img in enumerate(imgs):
            sheet.paste(img.convert("RGB"), (x0 + k * cell, y0 + label))
    os.makedirs(os.path.dirname(HOUSE_SHEET), exist_ok=True)
    sheet.save(HOUSE_SHEET, optimize=True)
    print("contact sheet", os.path.relpath(HOUSE_SHEET, ROOT))


if __name__ == "__main__":
    main()
