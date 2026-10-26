"""Packs downloaded ambientCG sets into the game's PBR layout (openspec changes `building-surfaces`,
`road-surfaces`). The output folder's name picks the list: `building` or `road`.

For each set: the colour map is scaled per channel so it averages `TARGET` (sRGB), so a vertex
colour sets the hue and the map adds the detail, as the ground's grass; the OpenGL normal map is
copied as PNG; the ORM map is packed the glTF way (R ambient occlusion, or white without one, G
roughness, B metalness, or black without one). Also writes a 2 x 2 tiling sheet per set for the
seam check (CLAUDE.md section 13).

    python3 tools/textures/pack_cc0_sets.py <folder of unzipped sets> assets/textures/building <sheet dir>
    python3 tools/textures/pack_cc0_sets.py <folder of unzipped sets> assets/textures/road <sheet dir>
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

TARGET = 0.8
# output folder -> surface name in the game -> ambientCG set id
SET_LISTS = {
    "building": {
        "concrete": "Concrete034",
        "plaster": "Plaster001",
        "wood": "Wood094",
        "fabric": "Fabric061",
        "metal": "Metal027",
        "plastic": "Plastic010",
        "cardboard": "Cardboard002",
        "leaves": "ScatteredLeaves009",
    },
    # city streets; arterials, highways and runways; pavements
    "road": {
        "asphalt": "Asphalt023L",
        "asphalt_worn": "Asphalt022",
        "paving": "PavingStones133",
    },
}
# Sets whose colour varies so much that the tint would come out speckled: grey them by this much.
DESATURATE = {"leaves": 0.8}


def map_of(src: Path, set_id: str, kind: str):
    p = src / set_id / f"{set_id}_1K-JPG_{kind}.jpg"
    return Image.open(p) if p.exists() else None


def main(src: Path, out: Path, sheets: Path):
    sheets.mkdir(parents=True, exist_ok=True)
    for name, set_id in SET_LISTS[out.name].items():
        dest = out / name
        dest.mkdir(parents=True, exist_ok=True)
        color = np.asarray(map_of(src, set_id, "Color").convert("RGB"), dtype=np.float32) / 255.0
        scale = TARGET / np.maximum(color.reshape(-1, 3).mean(axis=0), 1e-3)
        albedo = np.clip(color * scale, 0.0, 1.0)
        grey = DESATURATE.get(name, 0.0)
        if grey:
            luma = albedo @ np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)
            albedo = albedo * (1.0 - grey) + luma[..., None] * grey
        Image.fromarray((albedo * 255 + 0.5).astype(np.uint8)).save(dest / "albedo.jpg", quality=92)
        map_of(src, set_id, "NormalGL").convert("RGB").save(dest / "normal_gl.png")
        size = albedo.shape[1], albedo.shape[0]
        ao = map_of(src, set_id, "AmbientOcclusion")
        metal = map_of(src, set_id, "Metalness")
        r = ao.convert("L") if ao else Image.new("L", size, 255)
        g = map_of(src, set_id, "Roughness").convert("L")
        b = metal.convert("L") if metal else Image.new("L", size, 0)
        Image.merge("RGB", (r, g, b)).save(dest / "orm.png")
        tile = Image.open(dest / "albedo.jpg")
        sheet = Image.new("RGB", (tile.width * 2, tile.height * 2))
        for x in (0, tile.width):
            for y in (0, tile.height):
                sheet.paste(tile, (x, y))
        sheet.resize((512, 512)).save(sheets / f"{name}_2x2.png")
        print(f"{name}: {set_id}, colour scale {scale.round(2).tolist()}, ao {'yes' if ao else 'no'}, metal {'yes' if metal else 'no'}")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))
