"""Build docs/mockups/crab_fortress.html: the crab air fortress (openspec/changes/crab-airship), a block-out.

    python3 tools/mockups/gen_crab_fortress.py

The page itself is tools/mockups/crab_fortress_src.html. This script inlines into it the block-outs it
carries at true scale: the deck guard and the dropped crabs (Shrike, Mite, Scuttler from
tools/blender/build_mech_blockouts.py, manifest assets/models/mech_crabs.json) and our 24 m Foreman
(assets/models/mech_blockouts.json) as the yardstick, so the page needs nothing but three.js r147 from
the CDN. Sizes on the page are read off the manifests, never typed (Mining Mike's rule: orbit it, at
true relative scale). The block-outs carry no textures, so GLTFLoader parses them from the inlined bytes
without fetching anything (the artifact host refuses fetch() of data: and blob: URLs).

The fortress is built in the page from primitives: it is the low-poly block-out the owner picks from
before the Blender model is made (CLAUDE.md section 1). It is painted the mechs' way: the plating kit
(tools/textures/gen_mech_plating.mjs, assets/textures/mech), its panelling laid out by each piece's shape (carapace
rings and sectors round the shell, drum panels round cylinders, slab modules on flat faces; colour, normal, ORM,
emission and a livery mask each), the slab's normal laid over everything small as close-up detail, tinted by the
`hermit` palette in tools/blender/mech_palettes.py, blue and orange. The maps are inlined at 512 px as JPEG to keep the page small; the game loads the 1024 px PNGs.
Two friendly mechs fly round it (Stevedore and Ironhorse block-outs) and draw its fire.
"""
import base64
import io
import json
import os
import sys

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
SRC = os.path.join(ROOT, "tools", "mockups", "crab_fortress_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "crab_fortress.html")
CRABS = ["shrike", "mite", "scuttler"]   # deck guard (Shrike, Mite) and drops (Mite, Scuttler): design section 4
YARDSTICK = "foreman"
ALLIES = ["stevedore", "ironhorse"]          # friendly mechs flying round the fortress
# the panelling laid out by shape: carapace (rings and sectors round the shell), drum (a cylinder's wrap), slab (a flat
# face's module), each with its normal, ORM, emission and livery mask
TEXTURES = [f"{n}{k}" for n in ("carapace", "drum", "slab") for k in ("", "_n", "_orm", "_e", "_m")]
TEX_PX = 512
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))
from mech_palettes import PALETTES  # noqa: E402


def srgb_hex(c):
    """A linear RGB triple from mech_palettes.py as an sRGB hex colour for the page."""
    enc = [12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055 for v in c]
    return "#" + "".join(f"{round(max(0, min(1, v)) * 255):02x}" for v in enc)


def texture(name):
    """One map, downscaled to half (keeping its shape) and inlined as a JPEG data URL."""
    im = Image.open(os.path.join(ROOT, "assets", "textures", "mech", name + ".png")).convert("RGB")
    im = im.resize((im.width * TEX_PX // 1024, im.height * TEX_PX // 1024), Image.LANCZOS)   # 4096x1024 carapace, 1024x2048 drum and slab
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=90, optimize=True)
    return "data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode("ascii")


def entry(meta):
    """One model for the page: its manifest numbers and its glb bytes."""
    with open(os.path.join(MODELS, meta["file"]), "rb") as f:
        glb = base64.b64encode(f.read()).decode("ascii")
    keep = {k: meta[k] for k in ("id", "name", "what", "height_m", "width_m", "length_m", "tris")}
    keep["limbs"] = [n for n in meta.get("limbs", []) if not n.endswith("_foot")]
    keep["glb"] = glb
    return keep


def main():
    with open(os.path.join(MODELS, "mech_crabs.json")) as f:
        crabs = {m["id"]: m for m in json.load(f)["mechs"]}
    with open(os.path.join(MODELS, "mech_blockouts.json")) as f:
        humans = {m["id"]: m for m in json.load(f)["mechs"]}
    models = {c: entry(crabs[c]) for c in CRABS}
    models[YARDSTICK] = entry(humans[YARDSTICK])
    for a in ALLIES:
        models[a] = entry(humans[a])
    tex = {t: texture(t) for t in TEXTURES}
    palette = {k: srgb_hex(v) for k, v in PALETTES["hermit"].items()}
    with open(SRC) as f:
        page = f.read()
    if "__MODELS__" not in page:
        raise SystemExit(f"{SRC}: no __MODELS__ marker")
    for marker in ("__MODELS__", "__TEXTURES__", "__PALETTE__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__MODELS__", json.dumps(models, separators=(",", ":")))
    page = page.replace("__TEXTURES__", json.dumps(tex, separators=(",", ":")))
    page = page.replace("__PALETTE__", json.dumps(palette))
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
