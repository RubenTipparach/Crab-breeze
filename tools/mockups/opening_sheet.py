"""Stitch each chassis's opening frames (shot by tools/mockups/shoot.mjs as <mech>_seq1..6.png) into one
contact sheet, docs/screenshots/mechs/<mech>_opening.png: the sequence at a glance, left to right.

    python3 tools/mockups/opening_sheet.py docs/screenshots/mechs
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

LABELS = ["Standing", "Kneeling", "Latches, steam", "Plates opening", "Opening", "Open, ladder down"]
FONT = os.path.join(os.path.dirname(__file__), "..", "fonts", "DejaVuSans-Bold.ttf")


def main():
    d = sys.argv[1] if len(sys.argv) > 1 else "docs/screenshots/mechs"
    for m in ("foreman", "stevedore", "ironhorse"):
        frames = [os.path.join(d, f"{m}_seq{i}.png") for i in range(1, 7)]
        if not all(os.path.exists(f) for f in frames):
            continue
        ims = [Image.open(f).convert("RGB") for f in frames]
        w, h = 640, int(ims[0].height * 640 / ims[0].width)
        sheet = Image.new("RGB", (w * 3 + 16, h * 2 + 12), (20, 18, 24))
        font = ImageFont.truetype(FONT, 22)
        for i, im in enumerate(ims):
            x, y = 4 + (i % 3) * (w + 4), 4 + (i // 3) * (h + 4)
            sheet.paste(im.resize((w, h), Image.LANCZOS), (x, y))
            ImageDraw.Draw(sheet).text((x + 12, y + h - 36), f"{i + 1}. {LABELS[i]}", font=font, fill=(255, 210, 138))
        out = os.path.join(d, f"{m}_opening.png")
        sheet.save(out, optimize=True)
        for f in frames:
            os.remove(f)
        print(out)


if __name__ == "__main__":
    main()
