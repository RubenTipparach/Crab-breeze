"""Crab Breeze: the contact sheet of the cockpit panels, at one scale, to be looked at before a
bake is called done (CLAUDE.md 10). Each panel's beauty render, and beside it the albedo, normal
and occlusion maps the cockpit actually uses.

  python3 tools/textures/cockpit_contact_sheet.py [--map beauty|albedo|normal|orm] [--ppm 700]
"""
import argparse
import json
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
PAN = os.path.join(ROOT, "assets", "textures", "cockpit", "panels")
FONT = os.path.join(ROOT, "tools", "fonts", "DejaVuSans-Bold.ttf")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--map", default="beauty")
    ap.add_argument("--ppm", type=float, default=700)
    ap.add_argument("--out", default=os.path.join(ROOT, "docs", "screenshots", "cockpit", "panels-contact-sheet.png"))
    a = ap.parse_args()
    man = json.load(open(os.path.join(PAN, "panels.json")))
    font = ImageFont.truetype(FONT, 18)
    tiles = []
    for name, m in man.items():
        im = Image.open(os.path.join(PAN, f"{name}_{a.map}.png")).convert("RGB")
        w, h = int(m["width_m"] * a.ppm), int(m["height_m"] * a.ppm)
        tiles.append((name, m, im.resize((w, h), Image.LANCZOS)))
    W = 2400
    pad = 30
    x, y, row_h, place = pad, pad, 0, []
    for name, m, im in sorted(tiles, key=lambda t: -t[2].size[1]):
        if x + im.size[0] > W - pad:
            x, y = pad, y + row_h + 40
            row_h = 0
        place.append((x, y, name, m, im))
        x += im.size[0] + pad
        row_h = max(row_h, im.size[1])
    H = y + row_h + 40 + pad
    sheet = Image.new("RGB", (W, H), (22, 24, 27))
    d = ImageDraw.Draw(sheet)
    for x, y, name, m, im in place:
        sheet.paste(im, (x, y + 26))
        d.text((x, y), f"{name}  {m['width_m']:.2f} x {m['height_m']:.2f} m  {m['px_per_m']:.0f} px/m", fill=(220, 220, 210), font=font)
    sheet.save(a.out)
    print(a.out, sheet.size)


main()
