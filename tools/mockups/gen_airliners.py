"""Build docs/mockups/airliners.html: the three airliners (openspec change airliner-models) at true size, rigged, lit and
breakable, beside a 1.8 m pilot and the 24 m Foreman.

    python3 tools/mockups/gen_airliners.py

The page itself is tools/mockups/airliners_src.html. This script inlines into it:
- __PLANES__: each airliner's manifest (assets/models/aircraft/<kind>.json), as tools/blender/build_airliners.py wrote
  it: the rig, zones, lamps and capsules the game will read;
- __YARDSTICK__: the Foreman block-out's numbers (assets/models/mech_blockouts.json);
- __LOADER__: tools/mockups/gltf_json_loader.js.
The models themselves go beside the page as glTF JSON with the buffer embedded (docs/mockups/airliners/<kind>.gltf.json,
foreman.gltf.json), the files the artifact host serves: each airliner's glb unchanged inside, so the page draws exactly
the file the game will load. Publish the page with those files beside it.
The page loads three.js r147 from the CDN, as the other mockup pages here do.
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "tools", "mockups", "airliners_src.html")
OUT = os.path.join(ROOT, "docs", "mockups", "airliners.html")
MODELS = os.path.join(ROOT, "assets", "models")
KINDS = ["propliner", "airliner", "widebody"]


FILES = os.path.join(ROOT, "docs", "mockups", "airliners")


def glb_to_gltf_json(src, dst):
    """A glb's JSON chunk with its binary chunk as a data URI buffer (the loader turns it back into a glb in memory)."""
    import struct
    data = open(src, "rb").read()
    jl = struct.unpack_from("<I", data, 12)[0]
    js = json.loads(data[20:20 + jl])
    bl = struct.unpack_from("<I", data, 20 + jl)[0]
    binc = data[28 + jl:28 + jl + bl]
    js["buffers"][0]["uri"] = "data:application/octet-stream;base64," + base64.b64encode(binc).decode("ascii")
    with open(dst, "w") as f:
        json.dump(js, f, separators=(",", ":"))
    return os.path.relpath(dst, os.path.dirname(OUT))


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
    for marker in ("__PLANES__", "__YARDSTICK__", "__LOADER__"):
        if marker not in page:
            raise SystemExit(f"{SRC}: no {marker} marker")
    page = page.replace("__PLANES__", json.dumps(planes(), separators=(",", ":")))
    page = page.replace("__YARDSTICK__", json.dumps(yardstick(), separators=(",", ":")))
    page = page.replace("__LOADER__", open(os.path.join(ROOT, "tools", "mockups", "gltf_json_loader.js")).read())
    with open(OUT, "w") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({len(page) / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
