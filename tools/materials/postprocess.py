#!/usr/bin/env python3
"""Turn Material Maker's renders into Crab Breeze's effect textures (openspec changes `blast-scorch` and `fx-sprites`).

Reads `data/materials/materials.ron` (the one list of materials), takes each one's maps from the render directory
(`tools/materials/raw/` by default: `<name>_albedo.png`, `_normal.png`, `_orm.png`), resizes them to the listed size with
Lanczos, and writes `assets/textures/fx/<name>/<map>.png`. Material Maker's ORM is already packed the glTF way (R
occlusion, G roughness, B metal), so it is copied as is. Writes a `SOURCE.md` beside the textures and the contact sheet
`docs/screenshots/materials/fx-contact-sheet.png` (each material over grass, sky and a dark ground), and prints a
digest of every file written.

Run after `export_materials.sh` renders, or alone: python3 tools/materials/postprocess.py [render dir]
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
    raw = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "tools", "materials", "raw")
    rev = os.environ.get("MM_REV", "174f15e")
    tiles = []
    for m in materials():
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


if __name__ == "__main__":
    main()
