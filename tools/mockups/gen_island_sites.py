"""Build docs/mockups/island-sites.html: the island as the game generates it (seed 42), with the sites the owner asked for
on 2026-10-30 (openspec/changes/island-sites-layout): two civil airports placed first, two seaports, three military
bases (two of today's five taken away), each runway's approach cones, the ship lane between the ports, and the offshore
limit (openspec/changes/offshore-limit: 15 km off the coast or 1 km inside the world's edge, whichever comes first).

    python3 tools/mockups/gen_island_sites.py

The map is the game's own (`cb-tools map 42`, the hydrology change's picture) and today's sites are the game's own
(`cb-tools sites 42`). The proposed second airport and second seaport stand where this script's PROPOSED table puts
them, by eye on the map: the generator will find their places by its rules, so the page shows the plan, not the
result. The page itself is tools/mockups/island_sites_src.html.
"""
import base64
import io
import json
import os
import re
import subprocess

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "tools", "mockups", "island_sites_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "island-sites.html")
SEED = 42
WORLD_M = 40960.0
# Placed by eye on the generated map (see the docstring): centre (x, z) in metres, x east, z south.
PROPOSED = [
    {"id": "airport_2", "kind": "airport", "label": "Civil airport 2 (proposed)", "centre": [7600, 8000], "size": [2620, 1520], "turn": 0},
    {"id": "seaport_2", "kind": "seaport", "label": "Seaport 2 (proposed)", "centre": [12250, -500], "size": [930, 2010], "turn": 1},
]
KEEP_BASES = [0, 1, 3]   # the three farthest apart once the civil sites are down; 2 is 3.4 km from airport 1


def run(*args):
    return subprocess.run(["cargo", "run", "-q", "-p", "cb-tools", "--", *args], cwd=ROOT, check=True,
                          capture_output=True, text=True).stdout


def main():
    tmp = os.path.join(ROOT, "target", "island_map.ppm")
    run("map", str(SEED), tmp, "2048")
    im = Image.open(tmp).convert("RGB")
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=88)
    map_uri = "data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode("ascii")
    sites, bases, cities = [], [], []
    for line in run("sites", str(SEED)).splitlines():
        m = re.match(r"(Airport|Seaport): origin \((-?\d+), (-?\d+)\), turn (\d), .*footprint (\d+) x (\d+) m", line)
        if m:
            sites.append({"kind": m[1].lower(), "origin": [int(m[2]), int(m[3])], "turn": int(m[4]), "size": [int(m[5]), int(m[6])]})
        m = re.match(r"AirBase (\d+): centre \((-?\d+), (-?\d+)\), half \((\d+), (\d+)\), runway \((-?\d+), (-?\d+)\) to \((-?\d+), (-?\d+)\)", line)
        if m:
            v = [int(g) for g in m.groups()]
            bases.append({"id": v[0], "centre": v[1:3], "half": v[3:5], "runway": [v[5:7], v[7:9]], "keep": v[0] in KEEP_BASES})
        m = re.match(r"city (\d+): centre \((-?\d+), (-?\d+)\)", line)
        if m:
            cities.append([int(m[2]), int(m[3])])
    data = {"seed": SEED, "world_m": WORLD_M, "sites": sites, "bases": bases, "cities": cities, "proposed": PROPOSED,
            "limit": {"offshore_m": 15000, "edge_margin_m": 1000, "warn_m": 2000}}
    with open(SRC) as f:
        page = f.read()
    for marker in ("__DATA__", "__MAP__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__DATA__", json.dumps(data, separators=(",", ":"))).replace("__MAP__", map_uri)
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
