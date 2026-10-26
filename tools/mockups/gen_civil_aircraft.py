"""Build docs/mockups/civil_aircraft.html: the civil aircraft at true size (openspec/changes/civil-air-traffic): the
modelled freighter (survey D8, W48) and the three approved block-out stand-ins.

    python3 tools/mockups/gen_civil_aircraft.py

The page itself is tools/mockups/civil_aircraft_src.html. This script inlines into it:
- __AIRCRAFT__: every kind's size and block-out boxes, printed by `cargo run -p cb-tools -- aircraft`, the very table the
  game draws (cb_core::air_traffic::shape::blockout), so the page and the game cannot disagree; a modelled kind (the
  freighter, tools/blender/build_freighter.py) carries its glb, its sling's glb and its manifest, the files the game
  draws;
- __YARDSTICK__: the Foreman block-out (assets/models/mech_blockouts.json and its glb), the 24 m yardstick, sized by its
  manifest (Mining Mike's rule: orbit it, at true relative scale).
The page loads three.js r147 from the CDN, as the other mockup pages here do. The owner approved the stand-ins on
2026-10-26 ("Approved all of the stand ins"), chose the 70 m freighter with a slung mech (W48) and asked for it to be
modelled in Blender now (D8).
"""
import base64
import json
import os
import subprocess

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "tools", "mockups", "civil_aircraft_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "civil_aircraft.html")
MODELS = os.path.join(ROOT, "assets", "models")


def b64(path):
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode("ascii")


def aircraft():
    """The block-out table, from the game's own code; each modelled kind with its glb, sling and manifest."""
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "cb-tools", "--", "aircraft", "data"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    table = json.loads(out)
    for k in table["kinds"]:
        m = k.get("model")
        if not m:
            continue
        scene = os.path.join(ROOT, "assets", m["scene"])
        m["glb"] = b64(scene)
        m["sling_glb"] = b64(os.path.join(ROOT, "assets", m["sling"])) if m.get("sling") else None
        with open(os.path.splitext(scene)[0] + ".json") as f:
            man = json.load(f)
        m["lods"] = man["lods"]
        m["generator"] = man["generator"]
    return table


def yardstick():
    """The Foreman block-out: its manifest numbers and its glb bytes."""
    with open(os.path.join(MODELS, "mech_blockouts.json")) as f:
        meta = {m["id"]: m for m in json.load(f)["mechs"]}["foreman"]
    with open(os.path.join(MODELS, meta["file"]), "rb") as f:
        glb = base64.b64encode(f.read()).decode("ascii")
    keep = {k: meta[k] for k in ("id", "name", "height_m", "width_m", "length_m")}
    keep["glb"] = glb
    return keep


def main():
    with open(SRC) as f:
        page = f.read()
    for marker in ("__AIRCRAFT__", "__YARDSTICK__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__AIRCRAFT__", json.dumps(aircraft(), separators=(",", ":")))
    page = page.replace("__YARDSTICK__", json.dumps(yardstick(), separators=(",", ":")))
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
