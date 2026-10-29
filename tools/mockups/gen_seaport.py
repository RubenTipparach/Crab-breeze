"""Build docs/mockups/seaport.html: a seaport at true size (openspec/changes/seaport-layout), for the owner to approve
before any game code is written (CLAUDE.md section 4; owner, 2026-10-29: "we also need seaports").

    python3 tools/mockups/gen_seaport.py

The page itself is tools/mockups/seaport_src.html. This script inlines into it the harbours change's boat and crane
models (assets/models/boats, the ones the game draws) with their manifest, the Foreman yardstick (gen_civil_aircraft's
yardstick), the small-buildings kit, the building engine, the site buildings and the site kit shared with the airport page: one
implementation of each.
"""
import base64
import json
import os

from gen_civil_aircraft import ROOT, yardstick

SRC = os.path.join(ROOT, "tools", "mockups", "seaport_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "seaport.html")
KIT = os.path.join(ROOT, "docs", "mockups", "small_buildings_kit.js")
SITE_KIT = os.path.join(ROOT, "tools", "mockups", "site_kit.js")
ENGINE = os.path.join(ROOT, "docs", "mockups", "building_engine.js")
SITE_BUILDINGS = os.path.join(ROOT, "docs", "mockups", "site_buildings_kit.js")
BOATS = os.path.join(ROOT, "assets", "models", "boats.json")


def boats():
    """Every model in the boats manifest, with its full-detail glb's bytes."""
    with open(BOATS) as f:
        manifest = json.load(f)["boats"]
    out = {}
    for name, m in manifest.items():
        with open(os.path.join(ROOT, "assets", "models", m["file"]), "rb") as f:
            out[name] = {"glb": base64.b64encode(f.read()).decode("ascii"), "bounds_m": m["bounds_m"]}
    return out


def main():
    with open(SRC) as f:
        page = f.read()
    for marker in ("__BOATS__", "__YARDSTICK__", "__KIT__", "__ENGINE__", "__SITE_BUILDINGS__", "__SITE_KIT__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__BOATS__", json.dumps(boats(), separators=(",", ":")))
    page = page.replace("__YARDSTICK__", json.dumps(yardstick(), separators=(",", ":")))
    with open(KIT) as f:
        page = page.replace("__KIT__", f.read())
    for marker, path in (("__ENGINE__", ENGINE), ("__SITE_BUILDINGS__", SITE_BUILDINGS), ("__SITE_KIT__", SITE_KIT)):
        with open(path) as f:
            page = page.replace(marker, f.read())
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
