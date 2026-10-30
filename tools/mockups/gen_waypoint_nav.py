#!/usr/bin/env python3
"""Builds the waypoint-nav mockup page (openspec change `waypoint-nav`): docs/artifacts/waypoint-nav/page.src.html with
the real island's map data inlined (docs/artifacts/waypoint-nav/map_data.json, from `cb-tools mapdata 42 256`), written
to docs/artifacts/waypoint-nav/index.html, the page that is published."""
from pathlib import Path

DIR = Path(__file__).resolve().parents[2] / "docs" / "artifacts" / "waypoint-nav"
src = (DIR / "page.src.html").read_text()
data = (DIR / "map_data.json").read_text().strip()
assert "/*MAP_DATA*/null" in src
(DIR / "index.html").write_text(src.replace("/*MAP_DATA*/null", data))
print("wrote", DIR / "index.html")
