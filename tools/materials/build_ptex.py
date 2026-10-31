#!/usr/bin/env python3
"""Author the Material Maker (.ptex) graphs for Crab Breeze's effect textures (openspec changes `blast-scorch` and
`fx-sprites`): the blast scorch decal, churned soil, and the particle sprite sheets.

A graph is the one implementation of how a material looks (CLAUDE.md section 10). Material Maker renders it
(`tools/materials/export_materials.sh`, through the `material-maker-headless` skill) and
`tools/materials/postprocess.py` turns the render into the game's textures. `data/materials/materials.ron` lists them.
Each graph here is Material Maker's own noise nodes (fbm2, voronoi) feeding one inline shader node, whose GLSL shapes the
blast or the sprite, then Material Maker's normal and occlusion nodes and its material node. Every .ptex opens in
Material Maker for hand tweaking; once one is edited there, remove it from GRAPHS below or it will be overwritten.

Provenance: the `Graph` helper and the small wrappers (rgb, hexc, grad, curve, the op and blend constants) are Star Crew
Bridge Simulator's `tools/materials/build_ptex.py` at revision 8b0bbf1 (itself fps-game-demo f6cd25c), unchanged; Crab
Breeze adds `Graph.shader` and the graphs.

Run:  python3 tools/materials/build_ptex.py
Then: tools/materials/export_materials.sh
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.join(HERE, "ptex")

# Material inputs on the "material" node (Material Maker's static PBR output).
ALBEDO, METALLIC, ROUGHNESS, EMISSION, NORMAL, AO, DEPTH, OPACITY = range(8)


def rgb(r, g, b, a=1.0):
    return {"r": r, "g": g, "b": b, "a": a, "type": "Color"}


def hexc(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255.0 for i in (0, 2, 4))


def grad(*stops, interpolation=1):
    """stops: (pos, '#rrggbb') or (pos, (r, g, b)) or (pos, grey_float)."""
    pts = []
    for pos, c in stops:
        if isinstance(c, str):
            c = hexc(c)
        elif isinstance(c, (int, float)):
            c = (c, c, c)
        pts.append({"pos": pos, "r": c[0], "g": c[1], "b": c[2], "a": 1})
    return {"interpolation": interpolation, "points": pts, "type": "Gradient"}


def curve(*pts):
    """Tonality curve points: (x, y) pairs, linear tangents."""
    out = []
    for i, (x, y) in enumerate(pts):
        out.append({"x": x, "y": y, "ls": 0.0, "rs": 0.0})
    # compute simple slopes
    for i in range(len(out)):
        if i > 0:
            dx = out[i]["x"] - out[i - 1]["x"]
            out[i]["ls"] = (out[i]["y"] - out[i - 1]["y"]) / dx if dx else 0
        if i < len(out) - 1:
            dx = out[i + 1]["x"] - out[i]["x"]
            out[i]["rs"] = (out[i + 1]["y"] - out[i]["y"]) / dx if dx else 0
    return {"points": out, "type": "Curve"}


class Graph:
    def __init__(self, label):
        self.label = label
        self.nodes = []
        self.conns = []
        self.counter = {}
        self.col = 0

    def node(self, type_, *inputs, name=None, **params):
        """Create a node. inputs: sequence of (node_name, port) or node_name (port 0) or None."""
        idx = self.counter.get(type_, 0)
        self.counter[type_] = idx + 1
        name = name or f"{type_}_{idx}"
        self.col += 1
        n = {
            "name": name,
            "type": type_,
            "node_position": {"x": 220 * (self.col % 8), "y": 160 * (self.col // 8)},
            "parameters": params,
        }
        self.nodes.append(n)
        for port, src in enumerate(inputs):
            if src is None:
                continue
            self.link(src, name, port)
        return name

    def link(self, src, dst, dst_port):
        if isinstance(src, tuple):
            s, sp = src
        else:
            s, sp = src, 0
        self.conns.append({"from": s, "from_port": sp, "to": dst, "to_port": dst_port})

    def material(self, albedo=None, metallic=None, roughness=None, emission=None,
                 normal=None, ao=None, depth=None, metallic_v=1.0, roughness_v=1.0,
                 emission_energy=1.0, normal_v=1.0, ao_v=1.0, transparent=False, opacity=None):
        name = self.node(
            "material", name="Material",
            albedo_color=rgb(1, 1, 1), metallic=metallic_v, roughness=roughness_v,
            emission_energy=emission_energy, normal=normal_v, ao=ao_v,
            depth_scale=0.0, flags_transparent=transparent, sss=0.0, size=11)
        self.nodes[-1]["export_paths"] = {}
        for port, src in ((ALBEDO, albedo), (METALLIC, metallic), (ROUGHNESS, roughness),
                          (EMISSION, emission), (NORMAL, normal), (AO, ao), (DEPTH, depth),
                          (OPACITY, opacity)):
            if src is not None:
                self.link(src, name, port)
        return name

    # --- convenience wrappers -------------------------------------------------
    def colorize(self, src, gradient):
        return self.node("colorize", src, gradient=gradient)

    def blend(self, top, bottom, mask=None, mode=0, amount=1.0):
        # Material Maker blend: s1 (top) over s2 (bottom) using mask * amount.
        return self.node("blend", top, bottom, mask, blend_type=mode, amount=amount)

    def math(self, a, b=None, op=0, in1=0.0, in2=0.0, clamp=False):
        return self.node("math", a, b, op=op, default_in1=in1, default_in2=in2, clamp=clamp)

    def normal_map(self, height, strength=1.0):
        return self.node("normal_map", height, param0=11, param1=strength, param2=0, param4=1)

    def occlusion(self, height, strength=1.5, radius=20):
        return self.node("occlusion2", height, param0=11, param1=radius, param2=strength, param3=1)

    def tone(self, src, *pts):
        return self.node("tonality", src, curve=curve(*pts))

    def grey(self, v):
        return self.node("uniform_greyscale", color=v)

    def color(self, c):
        if isinstance(c, str):
            c = hexc(c)
        return self.node("uniform", color=rgb(*c))

    def to_json(self):
        return {
            "name": "Material", "label": self.label, "type": "graph",
            "node_position": {"x": 0, "y": 0},
            "parameters": {}, "connections": self.conns, "nodes": self.nodes,
        }


# Math op indices (see Material Maker math node)
ADD, SUB, MUL, DIV = 0, 1, 2, 3
POW, ABS = 6, 7
MIN, MAX = 13, 14
LT = 15
SMOOTH = 20

# Blend modes
NORMAL_B, DISSOLVE, MULTIPLY, SCREEN, OVERLAY, HARD_LIGHT, SOFT_LIGHT = range(7)
BURN, DODGE, LIGHTEN, DARKEN, DIFFERENCE, ADDITIVE = range(7, 13)



def shader(g, label, inputs, instance, outputs):
    """An inline shader node (Material Maker's `shader` type, as a custom node saved in a graph).

    inputs: (name, type, source) with type "f", "rgb" or "rgba" and source a node name or (node, port); the GLSL calls
    `$name(uv)`. instance: Godot shading language with `$name` suffixes on its functions. outputs: (type, expression)
    in `$uv`. Returns the node's name (port i is outputs[i])."""
    # Every input is declared a function ("function": true), so the shader's own functions can call it anywhere (at
    # offsets, for the lighting and the rays); otherwise Material Maker hoists `$input(...)` into the output's code.
    name = f"shader_{len(g.nodes)}"
    g.col += 1
    g.nodes.append({
        "name": name, "type": "shader", "node_position": {"x": 220 * (g.col % 8), "y": 160 * (g.col // 8)},
        "parameters": {}, "seed_int": 0,
        "shader_model": {
            "code": "", "global": "", "instance": instance, "name": label, "shortdesc": label, "longdesc": label,
            "parameters": [],
            "inputs": [{"name": n, "type": t, "default": "0.0" if t == "f" else "vec4(0.0)" if t == "rgba" else "vec3(0.0)",
                        "function": True, "label": n, "shortdesc": n, "longdesc": n} for n, t, _ in inputs],
            "outputs": [{"type": t, t: e, "shortdesc": f"out{i}", "longdesc": f"out{i}"} for i, (t, e) in enumerate(outputs)],
        },
    })
    for port, (_, _, src) in enumerate(inputs):
        g.link(src, name, port)
    return name


def fbm(g, scale, iterations=5, persistence=0.55, offset=0.0, noise=2, sy=None):
    return g.node("fbm2", noise=noise, scale_x=scale, scale_y=sy or scale, folds=0, iterations=iterations,
                  persistence=persistence, offset=offset)


HASH = "float h_$name(float x) { return fract(sin(x * 12.9898 + 0.731) * 43758.5453); }\n"


# ----------------------------------------------------------------------------
# blast-scorch
# ----------------------------------------------------------------------------

def blast_scorch():
    """One whole blast mark on soil, not tiling (openspec change `blast-scorch` design 2): a charred, cracked centre,
    patchy ash, a churned ring and lip of dark soil, radial streaks of soot and thrown dirt, and a ragged edge in the
    alpha."""
    g = Graph("Blast Scorch")
    warp = fbm(g, 3, iterations=5, persistence=0.6)
    detail = fbm(g, 24, iterations=5, persistence=0.55, offset=0.4)
    cracks = g.node("voronoi", scale_x=26, scale_y=26, stretch_x=1, stretch_y=1, intensity=1, randomness=0.9)
    inst = HASH + """
float r_$name(vec2 uv) {
	vec2 p = uv - vec2(0.5);
	return length(p) * 2.0 * (1.0 + 0.45 * ($warp(uv) - 0.5));
}
float rays_$name(vec2 uv, float r) {
	vec2 p = uv - vec2(0.5);
	float a = atan(p.y, p.x) / 6.2831853 + 0.5;
	float rays = 0.0;
	for (int k = 0; k < 3; k++) {
		float fk = float(k);
		float s = a * mix(53.0, 131.0, fk * 0.5) + fk * 0.37;
		float id = floor(s);
		float f = fract(s) - 0.5;
		float h = h_$name(id + fk * 91.0);
		float reach = 0.5 + 0.55 * h;
		float width = (0.05 + 0.14 * h_$name(id * 3.1 + 7.0)) * (1.0 - 0.7 * clamp(r / reach, 0.0, 1.0));
		float ray = exp(-pow(f / max(width, 0.001), 2.0)) * smoothstep(reach, reach * 0.55, r) * step(0.35, h);
		rays = max(rays, ray * mix(1.0, 0.55, fk * 0.5));
	}
	// Broken along their length, as thrown soot and dirt lands in clumps.
	float clumps = smoothstep(0.3, 0.7, $detail(uv * 0.5 + vec2(0.25)));
	return rays * mix(0.45, 1.0, clumps) * smoothstep(0.22, 0.4, r);
}
float crack_$name(vec2 uv) { return 1.0 - smoothstep(0.0, 0.035, $cracks(uv)); }
float height_$name(vec2 uv) {
	float r = r_$name(uv);
	float d = $detail(uv);
	float bowl = -0.5 * (1.0 - smoothstep(0.0, 0.52, r));
	float lip = 0.32 * exp(-pow((r - 0.55) / 0.1, 2.0));
	float clods = (d - 0.5) * 0.3 * smoothstep(0.3, 0.48, r) * smoothstep(0.9, 0.6, r);
	float crust = -0.05 * crack_$name(uv) * smoothstep(0.36, 0.15, r);
	return 0.55 + bowl + lip + clods + crust + 0.06 * rays_$name(uv, r);
}
vec4 albedo_$name(vec2 uv) {
	vec2 p = uv - vec2(0.5);
	float r = r_$name(uv);
	float d = $detail(uv);
	float w = $warp(uv);
	vec3 soil = mix(vec3(0.15, 0.115, 0.08), vec3(0.25, 0.20, 0.14), d);
	vec3 churned = mix(vec3(0.06, 0.047, 0.035), vec3(0.15, 0.11, 0.075), d);
	vec3 charred = mix(vec3(0.025, 0.022, 0.019), vec3(0.085, 0.072, 0.06), d);
	vec3 ash = mix(vec3(0.27, 0.26, 0.245), vec3(0.45, 0.435, 0.41), d);
	vec3 c = mix(soil, churned, smoothstep(0.9, 0.55, r));
	c = mix(c, charred, smoothstep(0.52, 0.3, r));
	float ashm = smoothstep(0.56, 0.7, w + 0.45 * (d - 0.5)) * smoothstep(0.08, 0.22, r) * smoothstep(0.5, 0.34, r);
	c = mix(c, ash, ashm);
	c = mix(c, ash * 0.7, crack_$name(uv) * smoothstep(0.34, 0.12, r) * 0.55);
	float soot = smoothstep(1.0, 0.45, r) * mix(0.35, 0.75, w);
	c = mix(c, vec3(0.035, 0.03, 0.026), soot * 0.55);
	float rays = rays_$name(uv, r);
	c = mix(c, vec3(0.025, 0.021, 0.018), rays * 0.85);
	float body = smoothstep(0.78, 0.52, r + 0.18 * (d - 0.5));
	float a = max(body, max(rays * 0.85, soot * 0.5) * smoothstep(1.05, 0.7, r));
	a *= smoothstep(0.5, 0.45, max(abs(p.x), abs(p.y)));
	return vec4(c, clamp(a, 0.0, 1.0));
}
float rough_$name(vec2 uv) {
	float r = r_$name(uv);
	float ro = mix(0.9, 0.82, smoothstep(0.9, 0.55, r));
	return mix(ro, 0.97, smoothstep(0.5, 0.3, r));
}
"""
    s = shader(g, "blast scorch", [("warp", "f", warp), ("detail", "f", detail), ("cracks", "f", (cracks, 1))], inst,
               [("f", "height_$name($uv)"), ("rgba", "albedo_$name($uv)"), ("f", "rough_$name($uv)"),
                ("f", "albedo_$name($uv).a")])
    normal = g.normal_map((s, 0), strength=1.6)
    ao = g.occlusion((s, 0), strength=1.4, radius=16)
    g.material(albedo=(s, 1), roughness=(s, 2), normal=normal, ao=ao, depth=(s, 0), metallic_v=0.0, transparent=True,
               opacity=(s, 3))
    return g


def churned_soil():
    """Broken dark earth and clods, tiling every 2 m: the crater's bowl and lip."""
    g = Graph("Churned Soil")
    clods = g.node("voronoi", scale_x=7, scale_y=7, stretch_x=1, stretch_y=1, intensity=1, randomness=1.0)
    detail = fbm(g, 32, iterations=5, persistence=0.55, offset=0.2)
    warp = fbm(g, 4, iterations=4, persistence=0.6, offset=0.7)
    inst = """
float dome_$name(vec2 uv) { return (1.0 - smoothstep(0.0, 0.7, $clods(uv))) * smoothstep(0.25, 0.65, $warp(uv) + 0.3 * $detail(uv)); }
float height_$name(vec2 uv) { return 0.55 * dome_$name(uv) + 0.3 * $detail(uv) + 0.15 * $warp(uv); }
vec4 albedo_$name(vec2 uv) {
	float dm = dome_$name(uv);
	float d = $detail(uv);
	vec3 c = mix(vec3(0.075, 0.058, 0.042), vec3(0.24, 0.185, 0.13), 0.45 * dm + 0.55 * d);
	c *= mix(0.8, 1.1, $warp(uv));
	return vec4(c, 1.0);
}
"""
    s = shader(g, "churned soil", [("clods", "f", (clods, 0)), ("detail", "f", detail), ("warp", "f", warp)], inst,
               [("f", "height_$name($uv)"), ("rgba", "albedo_$name($uv)"), ("f", "0.92 - 0.08 * dome_$name($uv)")])
    normal = g.normal_map((s, 0), strength=1.4)
    ao = g.occlusion((s, 0), strength=1.3, radius=10)
    g.material(albedo=(s, 1), roughness=(s, 2), normal=normal, ao=ao, depth=(s, 0), metallic_v=0.0)
    return g


# ----------------------------------------------------------------------------
# fx-sprites: 4 x 4 sheets, each cell a sprite; RGB the colour, A the coverage
# ----------------------------------------------------------------------------

# The light on a lit sprite: from above and a little to the left (image y runs down).
LIT = """
vec3 lit_$name(vec2 uv, float e) {
	float hx = height_$name(uv + vec2(e, 0.0)) - height_$name(uv - vec2(e, 0.0));
	float hy = height_$name(uv + vec2(0.0, e)) - height_$name(uv - vec2(0.0, e));
	vec3 n = normalize(vec3(-hx, -hy, 2.0 * e * 6.0));
	return n;
}
"""
CELL = """
vec2 cell_$name(vec2 uv) { return floor(uv * 4.0); }
vec2 local_$name(vec2 uv) { return fract(uv * 4.0); }
float edge_$name(vec2 uv) { vec2 c = local_$name(uv) - vec2(0.5); return smoothstep(0.5, 0.43, max(abs(c.x), abs(c.y))); }
"""


PUFF_HEIGHT = """
float height_$name(vec2 uv) {
	vec2 cl = cell_$name(uv);
	vec2 c = local_$name(uv);
	float id = cl.x + cl.y * 4.0;
	float sum = 0.0;
	for (int i = 0; i < 9; i++) {
		float fi = float(i);
		float a = h_$name(id * 17.0 + fi * 3.7) * 6.2831853;
		float rr = 0.17 * h_$name(id * 5.3 + fi * 1.9) * min(fi, 1.0);
		vec2 ctr = vec2(0.5) + vec2(cos(a), sin(a)) * rr;
		float rad = mix(0.16, 0.27, h_$name(id * 9.1 + fi * 4.3)) * (1.0 - 0.04 * fi);
		float d = length(c - ctr);
		// A smooth union of the spheres, so they flow into each other with no crease.
		sum += exp(10.0 * sqrt(max(rad * rad - d * d, 0.0)) / 0.27);
	}
	float h = max(log(sum / 9.0) / 10.0, 0.0);
	// Billows: smaller rolls on the big ones, so the shading folds.
	return h + 0.22 * smoothstep(0.0, 0.25, h) * ($billow(uv) - 0.5);
}
"""


def smoke_puffs():
    """Sixteen billowing smoke puffs, each a cluster of soft spheres eaten at the edge by noise, lit from above."""
    g = Graph("Smoke Puffs")
    erode = fbm(g, 16, iterations=5, persistence=0.6)
    grain = fbm(g, 64, iterations=4, persistence=0.5, offset=0.3)
    inst = HASH + CELL + PUFF_HEIGHT + LIT + """
vec4 puff_$name(vec2 uv) {
	float h = height_$name(uv);
	vec3 n = lit_$name(uv, 0.0015);
	float light = clamp(dot(n, normalize(vec3(-0.35, -0.8, 0.5))), 0.0, 1.0);
	float shade = 0.42 + 0.58 * light;
	shade *= mix(0.94, 1.04, $grain(uv));
	float a = smoothstep(0.0, 0.45, h - 0.3 * ($erode(uv) - 0.5) - 0.05) * edge_$name(uv);
	return vec4(vec3(shade) * vec3(1.0, 0.985, 0.96), a);
}
"""
    billow = fbm(g, 10, iterations=3, persistence=0.45, offset=0.15)
    s = shader(g, "smoke puffs", [("erode", "f", erode), ("grain", "f", grain), ("billow", "f", billow)], inst, [("rgba", "puff_$name($uv)"), ("f", "puff_$name($uv).a")])
    g.material(albedo=(s, 0), metallic_v=0.0, transparent=True, opacity=(s, 1))
    return g


def fireball_puffs():
    """Sixteen fireball billows, the smoke puffs' own silhouettes (`PUFF_HEIGHT`, cell for cell) burning from within:
    white-yellow where a billow is thick and faces out, orange to deep red in its folds and at its rim, streaked with
    soot. Glows; not lit. A puff cross-fades from this sheet to the smoke sheet as it cools, with no change of shape
    (owner, 2026-10-30: "I dont buy the flames as falmes, they look way too much like smoke")."""
    g = Graph("Fireball Puffs")
    erode = fbm(g, 16, iterations=5, persistence=0.6)
    billow = fbm(g, 10, iterations=3, persistence=0.45, offset=0.15)
    turb = fbm(g, 24, iterations=5, persistence=0.6, offset=0.7)
    soot = fbm(g, 12, iterations=4, persistence=0.5, offset=0.9)
    inst = HASH + CELL + PUFF_HEIGHT + LIT + """
vec4 fire_$name(vec2 uv) {
	float h = height_$name(uv);
	vec3 n = lit_$name(uv, 0.0015);
	// a fold or the rim faces away from the eye: cooler there
	float fold = 1.0 - clamp(n.z, 0.0, 1.0);
	float heat = clamp(0.2 + 1.1 * smoothstep(0.03, 0.55, h) - 0.7 * fold + 0.8 * ($turb(uv) - 0.5), 0.0, 1.0);
	vec3 c = mix(vec3(0.30, 0.06, 0.02), vec3(1.0, 0.42, 0.08), smoothstep(0.0, 0.55, heat));
	c = mix(c, vec3(1.0, 0.93, 0.72), smoothstep(0.55, 1.0, heat));
	c *= mix(1.0, 0.3, smoothstep(0.58, 0.78, $soot(uv)) * (1.0 - heat));
	float a = smoothstep(0.0, 0.45, h - 0.3 * ($erode(uv) - 0.5) - 0.05) * edge_$name(uv);
	return vec4(c, a);
}
"""
    s = shader(g, "fireball puffs", [("erode", "f", erode), ("billow", "f", billow), ("turb", "f", turb), ("soot", "f", soot)], inst, [("rgba", "fire_$name($uv)"), ("f", "fire_$name($uv).a")])
    g.material(albedo=(s, 0), metallic_v=0.0, transparent=True, opacity=(s, 1))
    return g


def smoke_lit():
    """Sixteen smoke puffs for lighting in the game: the smoke puffs' own silhouettes (`PUFF_HEIGHT`, cell for cell),
    an unlit colour with fine grain and a little darkening in the folds, and a normal map from the same height, so the
    game lights each puff from the sun's real direction (owner, 2026-10-30: "Maybe make new smoke puffs with normals
    that you can light?")."""
    g = Graph("Smoke Lit")
    erode = fbm(g, 16, iterations=5, persistence=0.6)
    grain = fbm(g, 64, iterations=4, persistence=0.5, offset=0.3)
    billow = fbm(g, 10, iterations=3, persistence=0.45, offset=0.15)
    inst = HASH + CELL + PUFF_HEIGHT + LIT + """
vec4 lit_puff_$name(vec2 uv) {
	float h = height_$name(uv);
	vec3 n = lit_$name(uv, 0.0015);
	// folds and crevices a little darker, whatever the light: the occlusion, not the sun
	float fold = 1.0 - clamp(n.z, 0.0, 1.0);
	float shade = mix(0.94, 1.04, $grain(uv)) * (1.0 - 0.3 * fold);
	float a = smoothstep(0.0, 0.45, h - 0.3 * ($erode(uv) - 0.5) - 0.05) * edge_$name(uv);
	return vec4(vec3(shade), a);
}
"""
    s = shader(g, "smoke lit", [("erode", "f", erode), ("grain", "f", grain), ("billow", "f", billow)], inst,
               [("f", "height_$name($uv)"), ("rgba", "lit_puff_$name($uv)"), ("f", "lit_puff_$name($uv).a")])
    normal = g.normal_map((s, 0), strength=2.0)
    g.material(albedo=(s, 1), normal=normal, metallic_v=0.0, transparent=True, opacity=(s, 2))
    return g

def fire_flipbook():
    """Sixteen frames of a flame tongue licking up and breaking into wisps, played over a fire particle's life."""
    g = Graph("Fire Flipbook")
    n1 = fbm(g, 4, iterations=5, persistence=0.55)
    n2 = fbm(g, 8, iterations=5, persistence=0.6, offset=0.5)
    inst = CELL + """
float flame_$name(vec2 uv) {
	vec2 cl = cell_$name(uv);
	vec2 c = local_$name(uv);
	float t = (cl.x + cl.y * 4.0) / 15.0;
	float y = 1.0 - c.y;
	float grow = mix(0.55, 1.0, smoothstep(0.0, 0.35, t));
	float a = $n1(vec2(c.x * 0.25 + 0.13, fract(c.y * 0.25 + t * 0.9)));
	float b = $n2(vec2(c.x * 0.25 + 0.61, fract(c.y * 0.25 + t * 1.7)));
	float sway = (a - 0.5) * 0.4 * y;
	float taper = clamp(1.0 - y / (0.95 * grow), 0.0, 1.0);
	float half_w = max(0.36 * grow * sqrt(taper) * (0.7 + 0.6 * b), 0.0001);
	float body = smoothstep(half_w, half_w * 0.15, abs(c.x - 0.5 - sway)) * step(0.0001, taper);
	float breakup = smoothstep(0.2 + 0.6 * t, 0.42 + 0.6 * t, b + (1.0 - y) * 0.55);
	return body * breakup * smoothstep(0.0, 0.08, y) * edge_$name(uv);
}
vec4 fire_$name(vec2 uv) {
	float i = flame_$name(uv);
	vec3 c = mix(vec3(1.0, 0.62, 0.38), vec3(1.0, 0.97, 0.9), smoothstep(0.45, 0.95, i));
	return vec4(c, smoothstep(0.0, 0.6, i));
}
"""
    s = shader(g, "fire flipbook", [("n1", "f", n1), ("n2", "f", n2)], inst, [("rgba", "fire_$name($uv)"), ("f", "fire_$name($uv).a")])
    g.material(albedo=(s, 0), metallic_v=0.0, transparent=True, opacity=(s, 1))
    return g


def dust_wisps():
    """Sixteen thin, torn dust clouds, stretched along the ground."""
    g = Graph("Dust Wisps")
    n = fbm(g, 16, iterations=6, persistence=0.6, sy=6)
    n2 = fbm(g, 40, iterations=4, persistence=0.5, offset=0.6)
    inst = HASH + CELL + """
vec4 dust_$name(vec2 uv) {
	vec2 cl = cell_$name(uv);
	vec2 c = local_$name(uv) - vec2(0.5);
	float id = cl.x + cl.y * 4.0;
	float ang = (h_$name(id) - 0.5) * 0.6;
	vec2 q = vec2(c.x * cos(ang) - c.y * sin(ang), c.x * sin(ang) + c.y * cos(ang));
	float mask = 1.0 - smoothstep(0.15, 0.46, length(q * vec2(0.85, 1.6)));
	float v = $n(uv);
	float a = smoothstep(0.42, 0.72, v * 0.8 + mask * 0.55) * mask;
	a *= mix(0.7, 1.0, $n2(uv)) * edge_$name(uv);
	return vec4(vec3(0.86, 0.81, 0.73) * mix(0.82, 1.04, v), clamp(a, 0.0, 1.0));
}
"""
    s = shader(g, "dust wisps", [("n", "f", n), ("n2", "f", n2)], inst, [("rgba", "dust_$name($uv)"), ("f", "dust_$name($uv).a")])
    g.material(albedo=(s, 0), metallic_v=0.0, transparent=True, opacity=(s, 1))
    return g


def dirt_clods():
    """Sixteen chunks of soil and stone, hard edged, lit from above."""
    g = Graph("Dirt Clods")
    grain = fbm(g, 48, iterations=4, persistence=0.55, offset=0.1)
    inst = HASH + CELL + """
float lump_$name(vec2 c, float id, vec2 ctr, float rad) {
	vec2 p = c - ctr;
	float th = atan(p.y, p.x);
	float rr = rad * (0.86 + 0.09 * sin(2.0 * th + h_$name(id) * 6.28) + 0.07 * sin(5.0 * th + h_$name(id + 3.0) * 6.28)
		+ 0.05 * sin(11.0 * th + h_$name(id + 4.0) * 6.28));
	rr *= mix(1.0, 0.8, step(0.82, fract(th * 1.59 + h_$name(id + 9.0))));
	float d = length(p) / rr;
	return sqrt(max(1.0 - d * d, 0.0));
}
float height_$name(vec2 uv) {
	vec2 cl = cell_$name(uv);
	vec2 c = local_$name(uv);
	float id = cl.x + cl.y * 4.0;
	float h = lump_$name(c, id * 7.0, vec2(0.5), mix(0.22, 0.32, h_$name(id * 2.0)));
	h = max(h, 0.7 * lump_$name(c, id * 7.0 + 1.0, vec2(0.5) + 0.22 * vec2(h_$name(id + 5.0) - 0.5, h_$name(id + 6.0) - 0.5), 0.12));
	return h + 0.04 * $grain(uv) * step(0.001, h);
}
""" + LIT + """
vec4 clod_$name(vec2 uv) {
	vec2 cl = cell_$name(uv);
	float id = cl.x + cl.y * 4.0;
	float h = height_$name(uv);
	vec3 n = lit_$name(uv, 0.001);
	float light = clamp(dot(n, normalize(vec3(-0.35, -0.8, 0.5))), 0.0, 1.0);
	vec3 base = mix(vec3(0.24, 0.18, 0.12), vec3(0.36, 0.34, 0.31), step(0.7, h_$name(id * 11.0)));
	base *= mix(0.75, 1.15, $grain(uv));
	return vec4(base * (0.35 + 0.75 * light), smoothstep(0.0, 0.08, h) * edge_$name(uv));
}
"""
    s = shader(g, "dirt clods", [("grain", "f", grain)], inst, [("rgba", "clod_$name($uv)"), ("f", "clod_$name($uv).a")])
    g.material(albedo=(s, 0), metallic_v=0.0, transparent=True, opacity=(s, 1))
    return g


def hit_marks():
    """Sixteen hit marks for mech armour (openspec change `mech-damage-fx` section 1), a 4 x 4 sheet: row 0 autocannon
    punctures with torn bright rims, row 1 cannon dents with a burnt halo, row 2 rocket scorch splats, row 3 gouges to
    bare metal. Albedo and alpha, height (for the normal), roughness and metal (bare metal shines)."""
    g = Graph("Hit Marks")
    n1 = fbm(g, 24, iterations=5, persistence=0.55)
    n2 = fbm(g, 64, iterations=4, persistence=0.5, offset=0.4)
    inst = HASH + CELL + """
// Per cell: kind (row), a seed, and the local point centred on the cell.
float kind_$name(vec2 uv) { return cell_$name(uv).y; }
float seed_$name(vec2 uv) { vec2 c = cell_$name(uv); return c.x + c.y * 4.0; }
vec2 p_$name(vec2 uv) { return local_$name(uv) - vec2(0.5); }
// The mark's own shape: 0 outside, how far in (0 at the edge .. 1 at the centre) and the bare metal share.
float ragged_$name(vec2 uv, vec2 p) {
	return 1.0 + 0.35 * ($n1(uv) - 0.5) + 0.2 * sin(atan(p.y, p.x) * 7.0 + seed_$name(uv) * 3.0) * 0.5;
}
float height_$name(vec2 uv) {
	vec2 p = p_$name(uv);
	float k = kind_$name(uv);
	float sd = seed_$name(uv);
	float r = length(p) * ragged_$name(uv, p);
	float h = 0.5;
	if (k < 0.5) {
		// Puncture: a hole, a raised torn rim.
		h += -0.5 * smoothstep(0.1, 0.05, r) + 0.18 * exp(-pow((r - 0.11) / 0.03, 2.0));
	} else if (k < 1.5) {
		// Dent: a dished bowl.
		h += -0.35 * (1.0 - smoothstep(0.0, 0.3, r)) + 0.08 * exp(-pow((r - 0.31) / 0.04, 2.0));
	} else if (k < 2.5) {
		// Scorch: blistered paint, barely any shape.
		h += 0.04 * ($n2(uv) - 0.5) * smoothstep(0.42, 0.2, r);
	} else {
		// Gouge: a long groove across the cell, turned by the seed.
		float a = (h_$name(sd) - 0.5) * 1.2;
		vec2 q = vec2(p.x * cos(a) - p.y * sin(a), p.x * sin(a) + p.y * cos(a));
		float len = smoothstep(0.42, 0.3, abs(q.x));
		h += -0.3 * smoothstep(0.045, 0.0, abs(q.y + 0.03 * sin(q.x * 20.0 + sd))) * len;
	}
	return h;
}
vec4 albedo_$name(vec2 uv) {
	vec2 p = p_$name(uv);
	float k = kind_$name(uv);
	float sd = seed_$name(uv);
	float r = length(p) * ragged_$name(uv, p);
	float d = $n2(uv);
	vec3 soot = vec3(0.03, 0.028, 0.026);
	vec3 metal = mix(vec3(0.55, 0.56, 0.58), vec3(0.75, 0.76, 0.78), d);
	vec3 c = soot;
	float a = 0.0;
	if (k < 0.5) {
		float hole = smoothstep(0.1, 0.05, r);
		float rim = exp(-pow((r - 0.11) / 0.035, 2.0));
		c = mix(soot, metal, rim);
		c = mix(c, vec3(0.005), hole);
		a = max(max(hole, rim), 0.8 * smoothstep(0.28, 0.1, r) * (0.6 + 0.4 * d));
	} else if (k < 1.5) {
		float bowl = smoothstep(0.32, 0.22, r);
		float halo = smoothstep(0.45, 0.25, r);
		c = mix(vec3(0.06, 0.05, 0.045), metal * 0.7, bowl * smoothstep(0.4, 0.75, d));
		a = max(halo * 0.9, bowl);
	} else if (k < 2.5) {
		float burn = smoothstep(0.45, 0.12, r + 0.15 * (d - 0.5));
		c = mix(vec3(0.06, 0.045, 0.03), soot, smoothstep(0.3, 0.1, r));
		a = burn * (0.75 + 0.25 * d);
	} else {
		float a2 = (h_$name(sd) - 0.5) * 1.2;
		vec2 q = vec2(p.x * cos(a2) - p.y * sin(a2), p.x * sin(a2) + p.y * cos(a2));
		float len = smoothstep(0.42, 0.3, abs(q.x));
		float groove = smoothstep(0.05, 0.0, abs(q.y + 0.03 * sin(q.x * 20.0 + sd))) * len;
		float scuff = smoothstep(0.12, 0.0, abs(q.y)) * len * 0.5;
		c = mix(soot * 3.0, metal, groove);
		a = max(groove, scuff);
	}
	return vec4(c, clamp(a, 0.0, 1.0) * edge_$name(uv));
}
float metal_$name(vec2 uv) {
	vec2 p = p_$name(uv);
	float k = kind_$name(uv);
	float r = length(p) * ragged_$name(uv, p);
	if (k < 0.5) { return exp(-pow((r - 0.11) / 0.035, 2.0)); }
	if (k > 2.5) { return albedo_$name(uv).g > 0.4 ? 1.0 : 0.0; }
	return 0.0;
}
"""
    s = shader(g, "hit marks", [("n1", "f", n1), ("n2", "f", n2)], inst,
               [("f", "height_$name($uv)"), ("rgba", "albedo_$name($uv)"), ("f", "mix(0.85, 0.35, metal_$name($uv))"),
                ("f", "albedo_$name($uv).a"), ("f", "metal_$name($uv)")])
    normal = g.normal_map((s, 0), strength=2.0)
    ao = g.occlusion((s, 0), strength=1.2, radius=6)
    g.material(albedo=(s, 1), roughness=(s, 2), metallic=(s, 4), normal=normal, ao=ao, depth=(s, 0), transparent=True,
               opacity=(s, 3))
    return g


GRAPHS = {
    "hit_marks": hit_marks,
    "blast_scorch": blast_scorch,
    "churned_soil": churned_soil,
    "smoke_puffs": smoke_puffs,
    "fire_flipbook": fire_flipbook,
    "fireball_puffs": fireball_puffs,
    "smoke_lit": smoke_lit,
    "dust_wisps": dust_wisps,
    "dirt_clods": dirt_clods,
}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, fn in GRAPHS.items():
        path = os.path.join(OUT_DIR, name + ".ptex")
        with open(path, "w") as f:
            json.dump(fn().to_json(), f, indent=1, sort_keys=True)
        print("wrote", os.path.relpath(path))


if __name__ == "__main__":
    main()
