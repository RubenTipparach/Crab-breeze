"""Build docs/mockups/military-base.html: one of the island's four military airfields at true size, with the fighter,
the military cargo plane and both helicopters the game will fly (openspec/changes/military-base), for the owner to
approve before any game code is written (owner, 2026-10-30: "Work on military base mockup. Vehicles: helicopter, cargo
plane, fighter jets. Facility: dealers choice, but we need same fidelity as civilian airport").

    python3 tools/mockups/gen_military_base.py

The page itself is tools/mockups/military_base_src.html. This script inlines into it the small-buildings kit, the
building engine and the site buildings (as the airport's page: every building drawn on the kit and broken by its rule),
the site kit and the airfield kit (the airport's runways, taxiways and lights: one implementation of each), the glTF
JSON loader, and each aircraft's manifest as tools/blender/build_military_aircraft.py wrote it. The models go beside the
page as glTF JSON (docs/mockups/military-base/<kind>.gltf.json, foreman.gltf.json), each glb unchanged inside, the files
the artifact host serves. Publish the page with those files beside it.
"""
import json
import os

from gen_airliners import ROOT, glb_to_gltf_json

SRC = os.path.join(ROOT, "tools", "mockups", "military_base_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "military-base.html")
FILES = os.path.join(ROOT, "docs", "mockups", "military-base")
MODELS = os.path.join(ROOT, "assets", "models")
KINDS = ["fighter", "military_cargo", "military_helicopter", "civil_helicopter"]
INLINE = {
    "__KIT__": os.path.join(ROOT, "docs", "mockups", "small_buildings_kit.js"),
    "__ENGINE__": os.path.join(ROOT, "docs", "mockups", "building_engine.js"),
    "__SITE_BUILDINGS__": os.path.join(ROOT, "docs", "mockups", "site_buildings_kit.js"),
    "__SITE_KIT__": os.path.join(ROOT, "tools", "mockups", "site_kit.js"),
    "__AIRFIELD_KIT__": os.path.join(ROOT, "tools", "mockups", "airfield_kit.js"),
    "__LOADER__": os.path.join(ROOT, "tools", "mockups", "gltf_json_loader.js"),
}


def planes():
    out = []
    for kind in KINDS:
        with open(os.path.join(MODELS, "aircraft", f"{kind}.json")) as f:
            man = json.load(f)
        url = glb_to_gltf_json(os.path.join(MODELS, "aircraft", man["file"]), os.path.join(FILES, f"{kind}.gltf.json"))
        out.append({"id": kind, "manifest": man, "url": url})
    return out


def yardstick():
    with open(os.path.join(MODELS, "mech_blockouts.json")) as f:
        meta = {m["id"]: m for m in json.load(f)["mechs"]}["foreman"]
    keep = {k: meta[k] for k in ("id", "name", "height_m", "width_m", "length_m")}
    keep["url"] = glb_to_gltf_json(os.path.join(MODELS, meta["file"]), os.path.join(FILES, "foreman.gltf.json"))
    return keep


def main():
    with open(SRC) as f:
        page = f.read()
    os.makedirs(FILES, exist_ok=True)
    for marker in list(INLINE) + ["__PLANES__", "__YARDSTICK__"]:
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__PLANES__", json.dumps(planes(), separators=(",", ":")))
    page = page.replace("__YARDSTICK__", json.dumps(yardstick(), separators=(",", ":")))
    for marker, path in INLINE.items():
        with open(path) as f:
            page = page.replace(marker, f.read())
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
