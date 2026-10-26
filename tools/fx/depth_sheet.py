#!/usr/bin/env python3
"""Stitch the particle depth bench's stills into one sheet (openspec change `particle-depth`).

Usage: depth_sheet.py OUT.png LABEL=DIR [LABEL=DIR ...]

Each DIR holds `depth-<test>-<shot>.png` from a `CB_STILLS_FX=depth` run of one variant. Rows are the
tests and shots; columns are the variants in the order given, labelled. A missing still is left grey.
"""
import sys
from pathlib import Path

from PIL import Image, ImageDraw

TESTS = ["ground-near", "ground-far", "trail", "skate", "sky"]
SHOTS = ["a", "b", "turned"]
W, H = 480, 300


def main() -> None:
    out = Path(sys.argv[1])
    cols = [a.split("=", 1) for a in sys.argv[2:]]
    rows = [(t, s) for t in TESTS for s in SHOTS]
    sheet = Image.new("RGB", (160 + W * len(cols), 30 + H * len(rows)), (40, 40, 40))
    draw = ImageDraw.Draw(sheet)
    for c, (label, _) in enumerate(cols):
        draw.text((160 + c * W + 8, 8), label, fill=(255, 255, 255))
    for r, (test, shot) in enumerate(rows):
        draw.text((8, 30 + r * H + H // 2), f"{test} {shot}", fill=(255, 255, 255))
        for c, (_, d) in enumerate(cols):
            p = Path(d) / f"depth-{test}-{shot}.png"
            if p.exists():
                sheet.paste(Image.open(p).convert("RGB").resize((W, H)), (160 + c * W, 30 + r * H))
    sheet.save(out)
    print(out)


if __name__ == "__main__":
    main()
