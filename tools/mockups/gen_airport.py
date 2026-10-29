"""Build docs/mockups/airport.html: one air base and its proposed airfield at true size (openspec/changes/airport-layout),
for the owner to approve before any game code is written (owner, 2026-10-28: "Please make mockups before editing game
code", "Mockups should b 3d in js").

    python3 tools/mockups/gen_airport.py

The page itself is tools/mockups/airport_src.html. This script inlines into it the small-buildings kit (its support rule breaks every building), the civil aircraft table (`cb-tools
aircraft`, the very block-outs and freighter model the game draws) and the Foreman yardstick, through the functions
tools/mockups/gen_civil_aircraft.py already has: one implementation of each.
"""
import json
import os

from gen_civil_aircraft import ROOT, aircraft, yardstick

SRC = os.path.join(ROOT, "tools", "mockups", "airport_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "airport.html")
KIT = os.path.join(ROOT, "docs", "mockups", "small_buildings_kit.js")


def main():
    with open(SRC) as f:
        page = f.read()
    for marker in ("__AIRCRAFT__", "__YARDSTICK__", "__KIT__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    table = aircraft()
    # The parked freighter shows its full-detail level only, and needs no sling.
    for k in table["kinds"]:
        if k.get("model"):
            k["model"].pop("sling_glb", None)
    page = page.replace("__AIRCRAFT__", json.dumps(table, separators=(",", ":")))
    page = page.replace("__YARDSTICK__", json.dumps(yardstick(), separators=(",", ":")))
    # The small-buildings kit's support rule decides what stands once a building is hit: one implementation of it.
    with open(KIT) as f:
        page = page.replace("__KIT__", f.read())
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
