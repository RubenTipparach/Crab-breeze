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
before the Blender model is made (CLAUDE.md section 1).
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
SRC = os.path.join(ROOT, "tools", "mockups", "crab_fortress_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "crab_fortress.html")
CRABS = ["shrike", "mite", "scuttler"]   # deck guard (Shrike, Mite) and drops (Mite, Scuttler): design section 4
YARDSTICK = "foreman"


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
    with open(SRC) as f:
        page = f.read()
    if "__MODELS__" not in page:
        raise SystemExit(f"{SRC}: no __MODELS__ marker")
    page = page.replace("__MODELS__", json.dumps(models, separators=(",", ":")))
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
