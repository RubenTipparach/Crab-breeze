"""Build docs/mockups/civil_aircraft.html: the civil aircraft block-out stand-ins (openspec/changes/civil-air-traffic).

    python3 tools/mockups/gen_civil_aircraft.py

The page itself is tools/mockups/civil_aircraft_src.html. This script inlines into it:
- __AIRCRAFT__: every kind's size and block-out boxes, printed by `cargo run -p cb-tools -- aircraft`, the very table the
  game draws (cb_core::air_traffic::shape::blockout), so the page and the game cannot disagree;
- __YARDSTICK__: the Foreman block-out (assets/models/mech_blockouts.json and its glb), the 24 m yardstick, sized by its
  manifest (Mining Mike's rule: orbit it, at true relative scale).
The page loads three.js r147 from the CDN, as the other mockup pages here do. It is not published: the stand-ins need the
owner's approval first (CLAUDE.md section 10).
"""
import base64
import json
import os
import subprocess

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "tools", "mockups", "civil_aircraft_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "civil_aircraft.html")
MODELS = os.path.join(ROOT, "assets", "models")


def aircraft():
    """The block-out table, from the game's own code."""
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "cb-tools", "--", "aircraft", "data"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)


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
