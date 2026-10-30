"""Build docs/mockups/island-sites.html: the island as the game generates it (seed 42), with the sites the owner asked for
on 2026-10-30 (openspec/changes/island-sites-layout): two civil airports placed first, two seaports, four military
bases (base 4 of today's five taken away), each runway's approach cones, the ship lane between the ports, and the offshore
limit (openspec/changes/offshore-limit: 15 km off the coast or 1 km inside the world's edge, whichever comes first).

    python3 tools/mockups/gen_island_sites.py

The map is the game's own (`cb-tools map 42`, the hydrology change's picture), and so is every site and base on it
(`cb-tools sites 42`): the page shows where the game places them. The Today view is what the game built before the
change (the TODAY table). The page itself is tools/mockups/island_sites_src.html.
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
# What the game built before the change (cb-tools sites 42 on 2026-10-30, before island-sites-layout): one airport, one
# seaport, five bases. Kept here for the page's Today view; the proposal is now the game's own placement.
TODAY = {
    "sites": [{"kind": "airport", "origin": [-5780, 7686], "turn": 0, "size": [2620, 1520]},
              {"kind": "seaport", "origin": [-11319, 3714], "turn": 3, "size": [930, 2010]}],
    "bases": [{"id": 0, "centre": [-10477, -5624], "half": [350, 250], "runway": [[-11427, -5957], [-9527, -5957]]},
              {"id": 1, "centre": [8582, -4939], "half": [350, 250], "runway": [[7582, -5272], [9182, -5272]]},
              {"id": 2, "centre": [-4515, 10871], "half": [350, 250], "runway": [[-5515, 10539], [-3515, 10539]]},
              {"id": 3, "centre": [-733, -11797], "half": [350, 250], "runway": [[-1733, -11465], [267, -11465]]},
              {"id": 4, "centre": [3731, -8367], "half": [350, 250], "runway": [[2731, -8034], [4331, -8034]]}],
}


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
            bases.append({"id": v[0], "centre": v[1:3], "half": v[3:5], "runway": [v[5:7], v[7:9]]})
        m = re.match(r"city (\d+): centre \((-?\d+), (-?\d+)\)", line)
        if m:
            cities.append([int(m[2]), int(m[3])])
    data = {"seed": SEED, "world_m": WORLD_M, "sites": sites, "bases": bases, "cities": cities, "today": TODAY,
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
