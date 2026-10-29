#!/usr/bin/env python3
"""Mechs page patch only (openspec change mech-arm-clearance): adds the shoulder axle housings (build_mechs.py contract_arm, CB_SHOULDER_AXLES=1) to
the restored Mechs page model files, so the page shows that change and nothing else. Radius = kit radius x kit scale,
both printed by the build; the axle runs on the torso from 0.55 of the shoulder's offset out to the joint.

    python3 tools/mockups/patch_mech_axles.py SRC DST    # folders of mech_<id>.gltf.json
"""
import base64, json, math, struct, sys
AX = {"foreman": (3.2, 0.2921308542076134), "ironhorse": (3.0, 0.25070363896694764), "harrow": (1.9, 0.3946277200021328),
      "roustabout": (2.1, 0.41267453128988685), "driller": (2.3, 0.2944757763980427), "mule": (2.4, 0.38258441401732124),
      "picket": (2.0, 0.31986348976606754), "sapper": (2.24, 0.36140855823092316)}
src, dst = sys.argv[1], sys.argv[2]
for mech, (rk, s) in AX.items():
    g = json.load(open(f"{src}/mech_{mech}.gltf.json"))
    buf = bytearray(base64.b64decode(g["buffers"][0]["uri"].split(",", 1)[1]))
    nodes = g["nodes"]; torso = next(i for i, n in enumerate(nodes) if n.get("name") == "torso")
    frame = next(i for i, m in enumerate(g["materials"]) if "_frame" in m["name"])
    r = rk * s
    for S in ("L", "R"):
        t = next(n for n in nodes if n.get("name") == f"shoulder_{S}")["translation"]
        x1, x0, y, z = t[0], t[0] * 0.55, t[1], t[2]
        seg, P, N, U, I = 18, [], [], [], []
        for k in range(seg + 1):
            a = 2 * math.pi * k / seg; cy, cz = math.cos(a), math.sin(a)
            for xx in (x0, x1):
                P += [xx, y + r * cy, z + r * cz]; N += [0, cy, cz]; U += [a * r, xx]
        for k in range(seg):
            a, b, c, d = 2 * k, 2 * k + 1, 2 * k + 2, 2 * k + 3
            I += [a, c, b, b, c, d]
        cap = len(P) // 3; sgn = 1 if x1 > 0 else -1           # the outer end's cap (the inner end is inside the torso)
        P += [x1, y, z]; N += [sgn, 0, 0]; U += [0, 0]
        for k in range(seg + 1):
            a = 2 * math.pi * k / seg; P += [x1, y + r * math.cos(a), z + r * math.sin(a)]; N += [sgn, 0, 0]; U += [r * math.cos(a), r * math.sin(a)]
        for k in range(seg):
            I += ([cap, cap + 1 + k, cap + 2 + k] if sgn > 0 else [cap, cap + 2 + k, cap + 1 + k])
        # winding: outward normals must agree with counter-clockwise faces; flip the side if this side's faces point in
        acc = []
        for data, fmt, typ, comp, extra in ((P, "f", "VEC3", 5126, True), (N, "f", "VEC3", 5126, False), (U, "f", "VEC2", 5126, False), (I, "I", "SCALAR", 5125, False)):
            while len(buf) % 4: buf.append(0)
            off = len(buf); buf += struct.pack(f"<{len(data)}{fmt}", *data)
            g["bufferViews"].append({"buffer": 0, "byteOffset": off, "byteLength": len(buf) - off, **({"target": 34963} if fmt == "I" else {"target": 34962})})
            a = {"bufferView": len(g["bufferViews"]) - 1, "componentType": comp, "count": len(data) // (3 if typ == "VEC3" else 2 if typ == "VEC2" else 1), "type": typ}
            if extra: a["min"] = [min(data[i::3]) for i in range(3)]; a["max"] = [max(data[i::3]) for i in range(3)]
            g["accessors"].append(a); acc.append(len(g["accessors"]) - 1)
        g["meshes"].append({"name": f"axle_{S}", "primitives": [{"attributes": {"POSITION": acc[0], "NORMAL": acc[1], "TEXCOORD_0": acc[2]}, "indices": acc[3], "material": frame}]})
        nodes.append({"name": f"shoulder_axle_{S}", "mesh": len(g["meshes"]) - 1})
        nodes[torso].setdefault("children", []).append(len(nodes) - 1)
    g["buffers"][0] = {"byteLength": len(buf), "uri": "data:application/octet-stream;base64," + base64.b64encode(bytes(buf)).decode()}
    json.dump(g, open(f"{dst}/mech_{mech}.gltf.json", "w"), separators=(",", ":"))
    print(mech, "r", round(r, 2), "m")
