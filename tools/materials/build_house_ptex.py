#!/usr/bin/env python3
"""Author the Material Maker (.ptex) graphs for the houses and small businesses: one full PBR set (albedo, normal, ORM)
per surface generator of the small-buildings kit (`SURFACES` in `docs/mockups/small_buildings_kit.js`), so every wall,
roof, floor and fitting the kit draws has a real texture instead of a flat stand-in colour.

A graph is the one implementation of how a material looks (CLAUDE.md section 10). Each graph here is Material Maker's
own noise nodes (fbm2, voronoi) feeding one inline shader node whose GLSL draws the surface at true size (one texture
tile covers the generator's `tile_m` metres, the game's UVs being metres), then Material Maker's normal map and
occlusion nodes and its material node. The look follows the owner-approved Main Street mockup's canvas generators
(`GEN` in `docs/mockups/small_buildings.html`), made real: true-size bricks, laps, courses and seams, height to a normal
map, roughness variation and occlusion in the joints.

Conventions every graph keeps:
- **Up is +v.** The kit's UVs are (metres along the surface, metres up it) and the glb keeps them as they are, so in the
  game "up the wall" or "up the roof slope" is down the image (Material Maker's uv.y grows down the image). Laps,
  shingle courses and louvres are drawn with their thick butt edge at the low-v side of each course for that reason.
- **Paint shows true.** The game multiplies the albedo by each building's vertex colour, so painted surfaces have a light,
  near-neutral albedo and natural ones (brick, asphalt, wood...) carry their own colour at the mockup's levels.
- **Every map tiles.** Every pattern repeats a whole number of times per tile and every noise is sampled at whole-number
  scales, so the 2 x 2 tiling on the contact sheet shows no seam.
- **Normals at true depth.** `depth_m` is the height range in metres; the normal map strength is
  `EXAGGERATION * 16 * depth_m / tile_m` (Material Maker's normal node tilts by 128 * strength * the height step per
  pixel at its 2048 px buffer), so a 10 mm mortar joint reads as 10 mm, a little exaggerated.

Each .ptex opens in Material Maker for hand tweaking; once one is edited there, remove it from GRAPHS below or it will be
overwritten. The `Graph` helper, `shader` and `fbm` are imported from `build_ptex.py` (one implementation of the helper).

Run:  python3 tools/materials/build_house_ptex.py
Then: tools/materials/export_house_materials.sh
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_ptex import Graph, fbm, shader  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.join(HERE, "ptex")

# The normal maps are drawn at true depth and then lifted by this much, so relief reads at a distance.
EXAGGERATION = 1.5
# Material Maker's occlusion node is gentle at these radii; this deepens it so joints, laps and gaps read in the ORM red.
AO_GAIN = 2.2

# Helpers every house shader shares: a hash of a cell id (Dave Hoskins' hash without sine), so per-brick and per-board
# variation repeats with the tile, and a clamp.
PRELUDE = """
float hs_$name(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float sat_$name(float x) { return clamp(x, 0.0, 1.0); }
"""


def vor(g, scale, randomness=1.0, intensity=1.0):
    """Material Maker's voronoi node, whole-number scale so it tiles. Ports: 0 nodes, 1 borders, 2 random colour."""
    return g.node("voronoi", scale_x=scale, scale_y=scale, stretch_x=1, stretch_y=1, intensity=intensity,
                  randomness=randomness)


def finish(g, label, inputs, body, tile_m, depth_m, ao_radius=16, ao_strength=1.2):
    """Wire the shader node (height, albedo, roughness, metal) to the normal, occlusion and material nodes.

    body must define height_$name, albedo_$name (vec3, sRGB), rough_$name and metal_$name, each taking the uv."""
    s = shader(g, label, inputs, PRELUDE + body,
               [("f", "sat_$name(height_$name($uv))"), ("rgba", "vec4(clamp(albedo_$name($uv), vec3(0.0), vec3(1.0)), 1.0)"),
                ("f", "sat_$name(rough_$name($uv))"), ("f", "sat_$name(metal_$name($uv))")])
    normal = g.normal_map((s, 0), strength=round(EXAGGERATION * 16.0 * depth_m / tile_m, 4))
    ao = g.occlusion((s, 0), strength=round(AO_GAIN * ao_strength, 3), radius=ao_radius)
    g.material(albedo=(s, 1), roughness=(s, 2), metallic=(s, 3), normal=normal, ao=ao, depth=(s, 0))
    return g


# ----------------------------------------------------------------------------
# masonry
# ----------------------------------------------------------------------------

def brick():
    """Face brick in running bond: 215 x 65 mm bricks with 10 mm joints, 4 across and 12 courses in the 0.9 m tile.
    Each brick its own shade of red (a few dark flashed and a few salmon), chipped arrises, pits and iron spots; a
    recessed sandy mortar."""
    g = Graph("House Brick")
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=5, offset=0.3)),
           ("chip", "f", fbm(g, 64, iterations=4, persistence=0.6, offset=0.6)),
           ("fine", "f", fbm(g, 256, iterations=3, persistence=0.5, offset=0.1))]
    body = """
vec4 cell_$name(vec2 uv) {
	float row = floor(uv.y * 12.0);
	float x = uv.x * 4.0 + 0.5 * mod(row, 2.0);
	return vec4(fract(x) * 0.225, fract(uv.y * 12.0) * 0.075, mod(floor(x), 4.0), mod(row, 12.0));
}
// Metres in from the brick's edge (negative in the joint), the arris broken by noise.
float edge_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float d = min(min(c.x, 0.225 - c.x), min(c.y, 0.075 - c.y)) - 0.005;
	return d - 0.0028 * ($chip(uv) - 0.45);
}
float pit_$name(vec2 uv) { return smoothstep(0.72, 0.8, $fine(uv + vec2(0.5, 0.25))); }
float height_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float r = hs_$name(c.zw + 0.37);
	float d = edge_$name(uv);
	float face = 0.78 + 0.08 * (r - 0.5) + 0.06 * ($mid(uv) - 0.5) + 0.05 * ($fine(uv) - 0.5) - 0.12 * pit_$name(uv);
	float mortar = 0.18 + 0.1 * $fine(uv * 2.0);
	return mix(mortar, face, smoothstep(-0.0006, 0.0028, d));
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float r = hs_$name(c.zw + 0.37);
	vec3 b = mix(vec3(0.52, 0.23, 0.165), vec3(0.69, 0.345, 0.245), r);
	if (hs_$name(c.zw + 5.1) > 0.86) { b = mix(b, vec3(0.33, 0.16, 0.125), 0.75); }
	if (hs_$name(c.zw + 9.3) < 0.12) { b = mix(b, vec3(0.76, 0.48, 0.34), 0.6); }
	b *= 0.86 + 0.26 * $mid(uv) + 0.08 * ($fine(uv) - 0.5);
	b *= 1.0 - 0.45 * smoothstep(0.74, 0.82, $fine(uv * 2.0 + vec2(0.31, 0.77)));
	b = mix(b, b * 0.55, 0.6 * pit_$name(uv));
	vec3 mortar = vec3(0.70, 0.68, 0.635) * (0.84 + 0.22 * $fine(uv * 2.0));
	vec3 col = mix(mortar, b, smoothstep(-0.0004, 0.0016, edge_$name(uv)));
	return col * (0.9 + 0.14 * $lo(uv));
}
float rough_$name(vec2 uv) { return mix(0.94, 0.84 + 0.08 * $mid(uv), smoothstep(-0.0004, 0.0016, edge_$name(uv))); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house brick", ins, body, tile_m=0.9, depth_m=0.012, ao_radius=10, ao_strength=1.3)


def block():
    """Concrete masonry units in running bond: 390 x 190 mm blocks with 10 mm tooled joints, 2 across and 4 courses in
    the 0.8 m tile. Open porous faces, aggregate specks, each block its own grey."""
    g = Graph("House Block")
    v = vor(g, 96, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=5, offset=0.3)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.2)), ("pore", "f", (v, 0)), ("porec", "rgb", (v, 2)),
           ("chip", "f", fbm(g, 64, iterations=4, offset=0.7))]
    body = """
vec4 cell_$name(vec2 uv) {
	float row = floor(uv.y * 4.0);
	float x = uv.x * 2.0 + 0.5 * mod(row, 2.0);
	return vec4(fract(x) * 0.4, fract(uv.y * 4.0) * 0.2, mod(floor(x), 2.0), mod(row, 4.0));
}
float edge_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	return min(min(c.x, 0.4 - c.x), min(c.y, 0.2 - c.y)) - 0.005 - 0.0015 * ($chip(uv) - 0.5);
}
float pores_$name(vec2 uv) { return smoothstep(0.16, 0.06, $pore(uv)) * step(0.55, $porec(uv).r); }
float height_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float face = 0.8 + 0.05 * (hs_$name(c.zw) - 0.5) + 0.06 * ($fine(uv) - 0.5) + 0.04 * ($mid(uv) - 0.5) - 0.22 * pores_$name(uv);
	// A tooled concave joint: deepest in the middle.
	float d = edge_$name(uv);
	float joint = 0.32 - 0.12 * smoothstep(-0.005, 0.0, -abs(d + 0.005) + 0.005) + 0.06 * $fine(uv * 2.0);
	return mix(joint, face, smoothstep(-0.0005, 0.0025, d));
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	vec3 b = vec3(0.80, 0.79, 0.765) * (0.96 + 0.06 * hs_$name(c.zw + 2.0));
	b *= 0.92 + 0.12 * $mid(uv);
	b *= 1.0 - 0.18 * smoothstep(0.62, 0.78, $fine(uv + vec2(0.13, 0.41)));
	b *= 1.0 + 0.1 * smoothstep(0.66, 0.8, $fine(uv * 2.0 + vec2(0.5, 0.0)));
	b *= 1.0 - 0.4 * pores_$name(uv);
	vec3 mortar = vec3(0.74, 0.73, 0.70) * (0.88 + 0.16 * $fine(uv * 2.0));
	return mix(mortar, b, smoothstep(-0.0004, 0.0015, edge_$name(uv))) * (0.92 + 0.1 * $lo(uv));
}
float rough_$name(vec2 uv) { return 0.86 + 0.08 * $fine(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house block", ins, body, tile_m=0.8, depth_m=0.01, ao_radius=12, ao_strength=1.2)


def stucco():
    """Sand-float stucco with a knockdown texture, over the 2 m tile: flattened spatter, fine sand, faint run-down
    streaks. Near white so the paint shows true."""
    g = Graph("House Stucco")
    v = vor(g, 48, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=5)),
           ("n64", "f", fbm(g, 64, iterations=4, offset=0.3)), ("sand", "f", fbm(g, 256, iterations=3, offset=0.5)),
           ("blob", "f", (v, 0)), ("blobc", "rgb", (v, 2)),
           ("streak", "f", g.node("fbm2", noise=2, scale_x=24, scale_y=2, folds=0, iterations=4, persistence=0.5, offset=0.2))]
    body = """
float knock_$name(vec2 uv) {
	float k = smoothstep(0.42 + 0.2 * $n64(uv), 0.3 + 0.2 * $n64(uv), $blob(uv)) * step(0.4, $blobc(uv).g);
	return k;
}
float height_$name(vec2 uv) {
	return 0.3 + 0.25 * $n64(uv) + 0.2 * $sand(uv) + 0.08 * $mid(uv) + 0.3 * knock_$name(uv);
}
vec3 albedo_$name(vec2 uv) {
	float v = 0.86 + 0.05 * ($mid(uv) - 0.5) + 0.04 * ($sand(uv) - 0.5) + 0.02 * knock_$name(uv);
	v *= 1.0 - 0.07 * smoothstep(0.5, 0.75, $streak(uv)) * $lo(uv);
	return vec3(v, v * 0.99, v * 0.965);
}
float rough_$name(vec2 uv) { return 0.9 + 0.06 * $sand(uv) - 0.04 * knock_$name(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house stucco", ins, body, tile_m=2.0, depth_m=0.004, ao_radius=8, ao_strength=1.0)


def adobe():
    """Mud plaster over adobe bricks, over the 2.5 m tile: soft lumps, trowel sweeps, the 0.5 x 0.125 m brick courses
    showing faintly where the plaster is thin, straw flecks and pits. Near neutral so the earth tint shows true."""
    g = Graph("House Adobe")
    ins = [("lo", "f", fbm(g, 5, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 20, iterations=5, offset=0.4)),
           ("trowel", "f", g.node("fbm2", noise=2, scale_x=10, scale_y=30, folds=0, iterations=4, persistence=0.55, offset=0.1)),
           ("fine", "f", fbm(g, 320, iterations=3, offset=0.2)),
           ("straw", "f", g.node("fbm2", noise=2, scale_x=640, scale_y=160, folds=0, iterations=2, persistence=0.5, offset=0.6))]
    body = """
float course_$name(vec2 uv) {
	float row = floor(uv.y * 20.0);
	float x = uv.x * 5.0 + 0.5 * mod(row, 2.0);
	float d = min(min(fract(x) * 0.5, (1.0 - fract(x)) * 0.5), min(fract(uv.y * 20.0), 1.0 - fract(uv.y * 20.0)) * 0.125);
	d += 0.02 * ($mid(uv) - 0.5);
	// Shows only where the plaster is thin.
	return (1.0 - smoothstep(0.0, 0.018, d)) * smoothstep(0.45, 0.7, $lo(uv + vec2(0.4, 0.2)));
}
float height_$name(vec2 uv) {
	return 0.45 + 0.22 * ($lo(uv) - 0.5) + 0.18 * ($mid(uv) - 0.5) + 0.12 * ($trowel(uv) - 0.5) + 0.06 * $fine(uv)
		- 0.14 * course_$name(uv) - 0.1 * smoothstep(0.76, 0.86, $fine(uv + vec2(0.5)));
}
vec3 albedo_$name(vec2 uv) {
	vec3 c = vec3(0.86, 0.82, 0.76) * (0.9 + 0.14 * $mid(uv) + 0.06 * ($trowel(uv) - 0.5));
	c *= 1.0 - 0.06 * course_$name(uv);
	c = mix(c, vec3(0.93, 0.85, 0.62), 0.55 * smoothstep(0.7, 0.78, $straw(uv)));
	c *= 1.0 - 0.35 * smoothstep(0.76, 0.86, $fine(uv + vec2(0.5)));
	return c * (0.94 + 0.08 * $lo(uv));
}
float rough_$name(vec2 uv) { return 0.93 + 0.05 * $fine(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house adobe", ins, body, tile_m=2.5, depth_m=0.015, ao_radius=14, ao_strength=1.0)


def concrete():
    """Cast concrete over the 2 m tile: fine sand, exposed aggregate specks, bug holes, damp blotches and a faint
    hairline crack."""
    g = Graph("House Concrete")
    v = vor(g, 40, randomness=1.0)
    c = vor(g, 3, randomness=0.85)
    ins = [("lo", "f", fbm(g, 4, iterations=5, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=5, offset=0.3)),
           ("n64", "f", fbm(g, 64, iterations=4, offset=0.6)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.2)),
           ("hole", "f", (v, 0)), ("holec", "rgb", (v, 2)), ("crack", "f", (c, 1))]
    body = """
float bug_$name(vec2 uv) { return smoothstep(0.09, 0.04, $hole(uv)) * step(0.82, $holec(uv).b); }
float crack_$name(vec2 uv) {
	float w = 1.0 - smoothstep(0.0, 0.012, $crack(uv + 0.02 * vec2($mid(uv) - 0.5, $n64(uv) - 0.5)));
	return w * smoothstep(0.55, 0.7, $lo(uv + vec2(0.3, 0.6)));
}
float height_$name(vec2 uv) {
	return 0.55 + 0.2 * ($n64(uv) - 0.5) + 0.18 * ($fine(uv) - 0.5) - 0.35 * bug_$name(uv) - 0.2 * crack_$name(uv);
}
vec3 albedo_$name(vec2 uv) {
	float v = 0.78 + 0.07 * ($lo(uv) - 0.5) + 0.06 * ($mid(uv) - 0.5) + 0.05 * ($fine(uv) - 0.5);
	v *= 1.0 - 0.1 * smoothstep(0.58, 0.75, $lo(uv + vec2(0.5, 0.1)));
	v += 0.04 * smoothstep(0.74, 0.82, $fine(uv * 2.0 + vec2(0.2, 0.7)));
	v -= 0.1 * smoothstep(0.7, 0.82, $fine(uv * 2.0 + vec2(0.6, 0.3)));
	v *= 1.0 - 0.4 * bug_$name(uv) - 0.25 * crack_$name(uv);
	return vec3(v, v * 0.99, v * 0.955);
}
float rough_$name(vec2 uv) { return 0.88 + 0.07 * $fine(uv) - 0.04 * $lo(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house concrete", ins, body, tile_m=2.0, depth_m=0.004, ao_radius=8, ao_strength=1.1)


# ----------------------------------------------------------------------------
# wood cladding and painted wood
# ----------------------------------------------------------------------------

def siding():
    """Painted clapboard lap siding, 6 boards of 167 mm exposure in the 1 m tile: each board tapers from a 12 mm butt
    at its foot to nothing where the board above laps it, one butt joint per board, wood grain telegraphing through
    the paint. Near white so the paint shows true."""
    g = Graph("House Siding")
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=4, offset=0.2)),
           ("grain", "f", g.node("fbm2", noise=2, scale_x=4, scale_y=96, folds=0, iterations=4, persistence=0.55, offset=0.4)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.3))]
    body = """
float joint_$name(vec2 uv) {
	float row = mod(floor(uv.y * 6.0), 6.0);
	float dj = abs(fract(uv.x - hs_$name(vec2(row, 3.0)) + 0.5) - 0.5);
	return 1.0 - smoothstep(0.0006, 0.0014, dj);
}
float height_$name(vec2 uv) {
	float f = fract(uv.y * 6.0);
	float lap = 0.3 + 0.7 * (1.0 - f);
	float h = mix(0.3, lap, smoothstep(0.0, 0.018, f));
	h += 0.02 * ($grain(uv) - 0.5) + 0.006 * $fine(uv);
	return h - 0.25 * joint_$name(uv);
}
vec3 albedo_$name(vec2 uv) {
	float f = fract(uv.y * 6.0);
	float v = 0.85 + 0.035 * ($grain(uv) - 0.5) + 0.03 * ($mid(uv) - 0.5) + 0.02 * ($fine(uv) - 0.5);
	v *= 1.0 - 0.06 * smoothstep(0.75, 1.0, f) * $lo(uv);
	v *= 0.95 + 0.07 * $lo(uv);
	v = mix(v, 0.32, 0.8 * joint_$name(uv));
	return vec3(v, v * 0.995, v * 0.975);
}
float rough_$name(vec2 uv) { return 0.6 + 0.1 * $mid(uv) + 0.05 * $grain(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house siding", ins, body, tile_m=1.0, depth_m=0.012, ao_radius=20, ao_strength=1.2)


def board():
    """Board and batten: 3 rough-sawn vertical boards of 333 mm in the 1 m tile, each seam covered by a 64 mm batten
    standing 19 mm proud with eased edges. Light warm neutral so the stain or paint shows true."""
    g = Graph("House Board")
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)),
           ("grain", "f", g.node("fbm2", noise=2, scale_x=96, scale_y=4, folds=0, iterations=5, persistence=0.55, offset=0.3)),
           ("saw", "f", g.node("fbm2", noise=2, scale_x=24, scale_y=128, folds=0, iterations=3, persistence=0.5, offset=0.8)),
           ("streak", "f", g.node("fbm2", noise=2, scale_x=12, scale_y=1, folds=0, iterations=4, persistence=0.5, offset=0.1)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.6))]
    body = """
float dist_$name(vec2 uv) { float xs = fract(uv.x * 3.0) / 3.0; return min(xs, 1.0 / 3.0 - xs); }
float batten_$name(vec2 uv) { return smoothstep(0.034, 0.029, dist_$name(uv)); }
float height_$name(vec2 uv) {
	float b = batten_$name(uv);
	float board = 0.12 + 0.05 * $grain(uv) + 0.03 * $saw(uv);
	float top = 0.92 + 0.03 * $grain(uv + vec2(0.5, 0.0)) - 0.08 * smoothstep(0.024, 0.032, dist_$name(uv));
	return mix(board, top, b);
}
vec3 albedo_$name(vec2 uv) {
	float k = floor(uv.x * 3.0 + 0.5);
	vec3 c = vec3(0.83, 0.79, 0.735) * (0.95 + 0.07 * hs_$name(vec2(k, 1.0)));
	c *= 0.9 + 0.14 * $grain(uv) + 0.05 * ($saw(uv) - 0.5);
	c *= 1.0 - 0.08 * smoothstep(0.55, 0.8, $streak(uv));
	c *= 0.95 + 0.07 * $lo(uv);
	return mix(c, c * 1.03, batten_$name(uv));
}
float rough_$name(vec2 uv) { return 0.74 + 0.1 * $saw(uv) - 0.04 * batten_$name(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house board", ins, body, tile_m=1.0, depth_m=0.02, ao_radius=24, ao_strength=1.2)


def shutter():
    """Louvred shutter slats, 12 of 42 mm in the 0.5 m tile: each slat leans back from a rounded nose at its foot, with
    a dark gap under the nose. Painted, light, so the shutter colour shows true."""
    g = Graph("House Shutter")
    ins = [("lo", "f", fbm(g, 4, iterations=4)),
           ("grain", "f", g.node("fbm2", noise=2, scale_x=4, scale_y=128, folds=0, iterations=4, persistence=0.55, offset=0.2)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.4))]
    body = """
float height_$name(vec2 uv) {
	float f = fract(uv.y * 12.0);
	float slat = 0.15 + 0.8 * (1.0 - f) - 0.12 * pow(1.0 - f, 6.0);
	return mix(0.03, slat, smoothstep(0.0, 0.07, f)) + 0.012 * $grain(uv);
}
vec3 albedo_$name(vec2 uv) {
	float f = fract(uv.y * 12.0);
	float v = 0.86 + 0.03 * ($grain(uv) - 0.5) + 0.02 * ($fine(uv) - 0.5);
	v *= 0.95 + 0.07 * $lo(uv);
	v = mix(0.3, v, smoothstep(0.0, 0.05, f));
	return vec3(v, v * 0.995, v * 0.98);
}
float rough_$name(vec2 uv) { return 0.52 + 0.08 * $fine(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house shutter", ins, body, tile_m=0.5, depth_m=0.012, ao_radius=10, ao_strength=1.3)


def door():
    """A painted wooden door face: vertical boards of 125 mm with V-grooves, the grain and vertical brush strokes showing
    through the paint. Light so the door colour shows true."""
    g = Graph("House Door")
    ins = [("lo", "f", fbm(g, 4, iterations=4)),
           ("grain", "f", g.node("fbm2", noise=2, scale_x=96, scale_y=4, folds=0, iterations=4, persistence=0.55, offset=0.5)),
           ("brush", "f", g.node("fbm2", noise=2, scale_x=192, scale_y=6, folds=0, iterations=3, persistence=0.5, offset=0.1)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.7))]
    body = """
float groove_$name(vec2 uv) { float xs = fract(uv.x * 8.0) * 0.125; float d = min(xs, 0.125 - xs); return 1.0 - smoothstep(0.0, 0.0035, d); }
float height_$name(vec2 uv) {
	return 0.7 + 0.08 * $grain(uv) + 0.05 * $brush(uv) + 0.02 * $fine(uv) - 0.6 * groove_$name(uv);
}
vec3 albedo_$name(vec2 uv) {
	float v = 0.85 + 0.03 * ($grain(uv) - 0.5) + 0.025 * ($brush(uv) - 0.5);
	v *= 0.96 + 0.06 * $lo(uv);
	v = mix(v, v * 0.6, groove_$name(uv));
	return vec3(v, v * 0.995, v * 0.98);
}
float rough_$name(vec2 uv) { return 0.42 + 0.1 * $brush(uv) + 0.05 * $lo(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house door", ins, body, tile_m=1.0, depth_m=0.004, ao_radius=8, ao_strength=1.1)


def wood():
    """Bare or oiled timber (posts, rails, frames): flat-sawn grain running along the surface, earlywood and darker
    latewood rings, fibres and pores, the earlywood weathered a little lower. Warm and light as the mockup's, under
    the tint."""
    g = Graph("House Wood")
    ins = [("warp", "f", g.node("fbm2", noise=2, scale_x=1, scale_y=2, folds=0, iterations=3, persistence=0.5, offset=0.2)),
           ("fibre", "f", g.node("fbm2", noise=2, scale_x=8, scale_y=384, folds=0, iterations=3, persistence=0.5, offset=0.4)),
           ("pore", "f", g.node("fbm2", noise=2, scale_x=96, scale_y=768, folds=0, iterations=2, persistence=0.5, offset=0.7)),
           ("lo", "f", fbm(g, 4, iterations=4))]
    body = """
float late_$name(vec2 uv) {
	float ring = fract(uv.y * 22.0 + 2.0 * $warp(uv) + 0.25 * $warp(uv * 4.0 + vec2(0.5)));
	return smoothstep(0.62, 0.86, ring) * (1.0 - smoothstep(0.9, 1.0, ring));
}
float height_$name(vec2 uv) {
	return 0.45 + 0.3 * late_$name(uv) + 0.12 * $fibre(uv) - 0.08 * smoothstep(0.78, 0.9, $pore(uv));
}
vec3 albedo_$name(vec2 uv) {
	vec3 c = vec3(0.76, 0.70, 0.645);
	c *= 1.0 - 0.24 * late_$name(uv);
	c *= 0.93 + 0.1 * $fibre(uv) + 0.06 * ($lo(uv) - 0.5);
	c *= 1.0 - 0.12 * smoothstep(0.78, 0.9, $pore(uv));
	return c;
}
float rough_$name(vec2 uv) { return 0.58 + 0.08 * $fibre(uv) - 0.06 * late_$name(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house wood", ins, body, tile_m=1.0, depth_m=0.0015, ao_radius=6, ao_strength=0.8)


def planks():
    """Deck and floor planks: 7 boards of 143 mm with 5 mm gaps in the 1 m tile, running up the tile, each its own
    colour with one butt joint and nail heads over joists every 0.5 m."""
    g = Graph("House Planks")
    ins = [("warp", "f", g.node("fbm2", noise=2, scale_x=2, scale_y=1, folds=0, iterations=3, persistence=0.5, offset=0.3)),
           ("fibre", "f", g.node("fbm2", noise=2, scale_x=384, scale_y=8, folds=0, iterations=3, persistence=0.5, offset=0.6)),
           ("lo", "f", fbm(g, 4, iterations=4)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.1))]
    body = """
float k_$name(vec2 uv) { return mod(floor(uv.x * 7.0), 7.0); }
float gapd_$name(vec2 uv) { float xs = fract(uv.x * 7.0) / 7.0; return min(xs, 1.0 / 7.0 - xs); }
float buttd_$name(vec2 uv) { return abs(fract(uv.y - hs_$name(vec2(k_$name(uv), 7.0)) + 0.5) - 0.5); }
float nail_$name(vec2 uv) {
	float xs = fract(uv.x * 7.0) / 7.0;
	float yj = fract(uv.y * 2.0 + 0.25) * 0.5;
	float dy = min(yj, 0.5 - yj);
	float dx = min(abs(xs - 0.025), abs(xs - 0.118));
	return 1.0 - smoothstep(0.0016, 0.0024, length(vec2(dx, dy)));
}
float late_$name(vec2 uv) {
	float ring = fract(uv.x * 42.0 + 1.5 * $warp(uv) + 0.2 * $warp(uv * 4.0 + vec2(0.3)) + 5.0 * hs_$name(vec2(k_$name(uv), 1.0)));
	return smoothstep(0.6, 0.85, ring) * (1.0 - smoothstep(0.9, 1.0, ring));
}
float height_$name(vec2 uv) {
	float d = min(gapd_$name(uv), buttd_$name(uv));
	float top = 0.85 + 0.05 * late_$name(uv) + 0.05 * $fibre(uv) - 0.05 * nail_$name(uv);
	return mix(0.0, top, smoothstep(0.0024, 0.0045, d));
}
vec3 albedo_$name(vec2 uv) {
	float k = k_$name(uv);
	vec3 c = vec3(0.73, 0.655, 0.585) * (0.84 + 0.26 * hs_$name(vec2(k, 2.0)));
	c *= 1.0 - 0.2 * late_$name(uv);
	c *= 0.92 + 0.12 * $fibre(uv) + 0.06 * ($lo(uv) - 0.5);
	c = mix(c, vec3(0.2, 0.2, 0.21), 0.85 * nail_$name(uv));
	float d = min(gapd_$name(uv), buttd_$name(uv));
	return mix(c * 0.25, c, smoothstep(0.0022, 0.004, d));
}
float rough_$name(vec2 uv) { return 0.55 + 0.1 * $fibre(uv) + 0.06 * $lo(uv) - 0.25 * nail_$name(uv); }
float metal_$name(vec2 uv) { return 0.8 * nail_$name(uv); }
"""
    return finish(g, "house planks", ins, body, tile_m=1.0, depth_m=0.012, ao_radius=10, ao_strength=1.3)


def pine():
    """Knotty pine interior panelling: 6 tongue-and-groove boards of 167 mm in the 1 m tile, running up the tile, with
    V-joints, warm yellow grain, and a dark knot or two per board with the grain swirling round it."""
    g = Graph("House Pine")
    ins = [("warp", "f", g.node("fbm2", noise=2, scale_x=2, scale_y=1, folds=0, iterations=3, persistence=0.5, offset=0.7)),
           ("fibre", "f", g.node("fbm2", noise=2, scale_x=384, scale_y=8, folds=0, iterations=3, persistence=0.5, offset=0.2)),
           ("lo", "f", fbm(g, 4, iterations=4))]
    body = """
float k_$name(vec2 uv) { return mod(floor(uv.x * 6.0), 6.0); }
float vd_$name(vec2 uv) { float xs = fract(uv.x * 6.0) / 6.0; return min(xs, 1.0 / 6.0 - xs); }
// Distance to this board's knot, in metres (an ellipse, longer along the board).
float knot_$name(vec2 uv) {
	float k = k_$name(uv);
	vec2 c = vec2((k + 0.3 + 0.4 * hs_$name(vec2(k, 4.0))) / 6.0, hs_$name(vec2(k, 5.0)));
	vec2 d = uv - c;
	d.y = abs(fract(d.y + 0.5) - 0.5);
	float r = 0.008 + 0.012 * hs_$name(vec2(k, 6.0));
	return length(d * vec2(1.0, 0.55)) / r;
}
float late_$name(vec2 uv) {
	float kn = knot_$name(uv);
	float swirl = 0.6 * exp(-kn * 0.35);
	float ring = fract(uv.x * 36.0 + 1.5 * $warp(uv) + 0.2 * $warp(uv * 4.0 + vec2(0.6)) + swirl * 3.0 + 4.0 * hs_$name(vec2(k_$name(uv), 1.0)));
	return smoothstep(0.62, 0.86, ring) * (1.0 - smoothstep(0.9, 1.0, ring));
}
float height_$name(vec2 uv) {
	float v = smoothstep(0.0, 0.004, vd_$name(uv));
	return 0.25 + 0.6 * v + 0.04 * late_$name(uv) + 0.03 * $fibre(uv) - 0.05 * (1.0 - smoothstep(0.8, 1.0, knot_$name(uv)));
}
vec3 albedo_$name(vec2 uv) {
	vec3 c = vec3(0.87, 0.765, 0.61) * (0.94 + 0.1 * hs_$name(vec2(k_$name(uv), 2.0)));
	c *= 1.0 - 0.17 * late_$name(uv);
	c *= 0.94 + 0.08 * $fibre(uv) + 0.05 * ($lo(uv) - 0.5);
	float kn = knot_$name(uv);
	c = mix(c, vec3(0.42, 0.25, 0.12), 1.0 - smoothstep(0.7, 1.15, kn));
	c = mix(c, vec3(0.62, 0.45, 0.28), 0.5 * (1.0 - smoothstep(1.0, 1.9, kn)) * smoothstep(0.7, 1.0, kn));
	return mix(c * 0.55, c, smoothstep(0.0, 0.0035, vd_$name(uv)));
}
float rough_$name(vec2 uv) { return 0.52 + 0.08 * $fibre(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house pine", ins, body, tile_m=1.0, depth_m=0.004, ao_radius=8, ao_strength=1.0)


# ----------------------------------------------------------------------------
# roofs
# ----------------------------------------------------------------------------

def shingle():
    """Three-tab asphalt shingles: 8 courses of 125 mm exposure and tabs of 333 mm in the 1 m tile, every other course
    offset half a tab, 6 mm slots, each tab its own shade, mineral granules, the butts throwing a shadow line down
    the roof. Mid grey like the mockup's, under the roof tint."""
    g = Graph("House Shingle")
    v = vor(g, 384, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 24, iterations=4, offset=0.3)),
           ("gran", "f", (v, 1)), ("granc", "rgb", (v, 2)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.5))]
    body = """
vec4 cell_$name(vec2 uv) {
	float row = floor(uv.y * 8.0);
	float x = uv.x * 3.0 + 0.5 * mod(row, 2.0);
	return vec4(fract(x), fract(uv.y * 8.0), mod(floor(x), 3.0), mod(row, 8.0));
}
float slot_$name(vec2 uv) { vec4 c = cell_$name(uv); float d = min(c.x, 1.0 - c.x) / 3.0; return 1.0 - smoothstep(0.0025, 0.0035, d); }
float height_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float f = c.y;
	float tab = 0.3 + 0.62 * (1.0 - f);
	float h = mix(0.3, tab, smoothstep(0.0, 0.035, f));
	h += 0.08 * smoothstep(0.0, 0.4, $gran(uv)) + 0.03 * $fine(uv);
	return mix(h, 0.15 + 0.04 * $fine(uv), slot_$name(uv));
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float t = 0.94 + 0.13 * hs_$name(c.zw + 1.7);
	float gr = $granc(uv).r;
	float v = mix(0.4, 0.76, gr) * t;
	if ($granc(uv).g > 0.93) { v = 0.25; }
	v *= 0.94 + 0.1 * $mid(uv) + 0.06 * ($lo(uv) - 0.5);
	v = mix(v, 0.22, 0.85 * slot_$name(uv));
	v *= mix(0.8, 1.0, smoothstep(0.0, 0.05, c.y));
	return vec3(v, v * 0.985, v * 0.97);
}
float rough_$name(vec2 uv) { return 0.88 + 0.08 * $granc(uv).b; }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house shingle", ins, body, tile_m=1.0, depth_m=0.008, ao_radius=14, ao_strength=1.3)


def tile():
    """Clay barrel (Spanish) roof tiles: 4 columns of 250 mm, the caps and pans of an S-tile, 3 courses of 333 mm in the
    1 m tile, each tile lapping the one below with a rounded nose, each its own fired shade, dirt in the pans. Light
    warm neutral like the mockup's, under the terracotta tint."""
    g = Graph("House Tile")
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=5, offset=0.5)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.3))]
    body = """
vec4 cell_$name(vec2 uv) {
	return vec4(fract(uv.x * 4.0), fract(uv.y * 3.0), mod(floor(uv.x * 4.0), 4.0), mod(floor(uv.y * 3.0), 3.0));
}
float prof_$name(float fx) { float p = 0.5 - 0.5 * cos(6.2831853 * fx); return pow(p, 0.8); }
float height_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float body = 0.12 + 0.62 * prof_$name(c.x) + 0.2 * (1.0 - c.y);
	float below = 0.12 + 0.62 * prof_$name(c.x);
	float h = mix(below, body, smoothstep(0.0, 0.05, c.y));
	return h + 0.015 * $fine(uv) + 0.02 * ($mid(uv) - 0.5);
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float r = hs_$name(c.zw + 0.5);
	vec3 col = mix(vec3(0.94, 0.86, 0.79), vec3(0.86, 0.80, 0.76), r);
	col *= 0.9 + 0.12 * hs_$name(c.zw + 3.3);
	col *= 0.92 + 0.12 * $mid(uv) + 0.04 * ($fine(uv) - 0.5);
	col *= mix(0.82, 1.0, smoothstep(0.0, 0.25, c.y));
	col *= mix(0.86, 1.0, smoothstep(0.0, 0.25, prof_$name(c.x)));
	col *= 0.94 + 0.08 * $lo(uv);
	return col;
}
float rough_$name(vec2 uv) { return 0.68 + 0.1 * $mid(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house tile", ins, body, tile_m=1.0, depth_m=0.05, ao_radius=40, ao_strength=1.0)


def metal_roof():
    """Standing-seam painted steel roof: panels of 500 mm with 25 mm seams, two pencil ribs between, faint oil canning,
    chalking and streaks running down the slope. Light near-neutral so the roof colour shows true; metal and gloss
    near the kit's values."""
    g = Graph("House Metal Roof")
    ins = [("lo", "f", fbm(g, 3, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=4, offset=0.2)),
           ("streak", "f", g.node("fbm2", noise=2, scale_x=48, scale_y=2, folds=0, iterations=4, persistence=0.55, offset=0.4)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.8))]
    body = """
float ds_$name(vec2 uv) { float xs = fract(uv.x * 2.0) * 0.5; return min(xs, 0.5 - xs); }
float seam_$name(vec2 uv) { return smoothstep(0.0075, 0.004, ds_$name(uv)); }
float height_$name(vec2 uv) {
	float d = ds_$name(uv);
	float pan = 0.1 + 0.05 * ($lo(uv) - 0.5) + 0.06 * exp(-pow((d - 0.1667) / 0.004, 2.0)) + 0.06 * exp(-pow((d - 0.3333) / 0.004, 2.0));
	float top = 1.0 - 0.15 * smoothstep(0.0, 0.004, d);
	return mix(pan, top, seam_$name(uv)) + 0.004 * $fine(uv);
}
vec3 albedo_$name(vec2 uv) {
	float v = 0.82 + 0.03 * ($mid(uv) - 0.5);
	v *= 1.0 - 0.07 * smoothstep(0.5, 0.8, $streak(uv));
	v += 0.02 * smoothstep(0.4, 0.9, $lo(uv + vec2(0.3, 0.5)));
	v += 0.03 * seam_$name(uv);
	return vec3(v, v * 1.0, v * 1.005);
}
float rough_$name(vec2 uv) { return 0.4 + 0.08 * smoothstep(0.45, 0.8, $streak(uv)) + 0.04 * $mid(uv); }
float metal_$name(vec2 uv) { return 0.6 - 0.04 * smoothstep(0.4, 0.9, $lo(uv + vec2(0.3, 0.5))); }
"""
    return finish(g, "house metal roof", ins, body, tile_m=1.0, depth_m=0.025, ao_radius=16, ao_strength=1.0)


def roofing():
    """A flat built-up roof surfaced with gravel: 10 mm stones of grey, tan and white bedded in tar, mid grey on
    average like the mockup's."""
    g = Graph("House Roofing")
    v = vor(g, 96, randomness=1.0)
    w = vor(g, 160, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("st", "f", (v, 1)), ("stc", "rgb", (v, 2)), ("st2", "f", (w, 1)),
           ("st2c", "rgb", (w, 2)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.4))]
    body = """
float stone_$name(vec2 uv) { return smoothstep(0.02, 0.22, $st(uv)); }
float stone2_$name(vec2 uv) { return smoothstep(0.03, 0.25, $st2(uv)) * step(0.55, $st2c(uv).r); }
float height_$name(vec2 uv) {
	float a = sqrt(stone_$name(uv)) * 0.75;
	float b = sqrt(stone2_$name(uv)) * 0.9;
	return 0.1 + max(a, b) + 0.05 * $fine(uv);
}
vec3 stonecol_$name(vec3 r) {
	vec3 c = mix(vec3(0.46, 0.45, 0.43), vec3(0.74, 0.72, 0.68), r.r);
	c = mix(c, vec3(0.66, 0.58, 0.47), step(0.7, r.g) * 0.8);
	c = mix(c, vec3(0.3, 0.3, 0.3), step(0.85, r.b));
	return c;
}
vec3 albedo_$name(vec2 uv) {
	vec3 tar = vec3(0.13, 0.13, 0.135);
	vec3 c = mix(tar, stonecol_$name($stc(uv)) * (0.85 + 0.2 * $st(uv)), stone_$name(uv));
	c = mix(c, stonecol_$name($st2c(uv).gbr) * (0.85 + 0.2 * $st2(uv)), stone2_$name(uv));
	return c * (0.9 + 0.14 * $lo(uv));
}
float rough_$name(vec2 uv) { return 0.9 + 0.06 * $fine(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house roofing", ins, body, tile_m=1.0, depth_m=0.008, ao_radius=6, ao_strength=1.3)


# ----------------------------------------------------------------------------
# sheet metal
# ----------------------------------------------------------------------------

def corrugated():
    """Galvanised corrugated sheet: 13 corrugations of 77 mm pitch and 18 mm depth across the 1 m tile, running along
    it, with the zinc's spangle and dull stains. Light so a paint tint shows true; metal and gloss near the kit's."""
    g = Graph("House Corrugated")
    v = vor(g, 24, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("mid", "f", fbm(g, 16, iterations=4, offset=0.3)),
           ("sp", "rgb", (v, 2)), ("spb", "f", (v, 1)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.6))]
    body = """
float height_$name(vec2 uv) { return 0.5 + 0.48 * cos(6.2831853 * uv.y * 13.0) + 0.004 * $fine(uv); }
vec3 albedo_$name(vec2 uv) {
	float v = 0.8 + 0.025 * ($sp(uv).r - 0.5) + 0.03 * ($mid(uv) - 0.5);
	v *= 1.0 - 0.06 * smoothstep(0.45, 0.9, $lo(uv + vec2(0.2, 0.7)));
	return vec3(v, v * 1.005, v * 1.01);
}
float rough_$name(vec2 uv) { return 0.42 + 0.08 * $sp(uv).g + 0.12 * smoothstep(0.55, 0.8, $lo(uv + vec2(0.2, 0.7))); }
float metal_$name(vec2 uv) { return 0.62 - 0.1 * smoothstep(0.55, 0.8, $lo(uv + vec2(0.2, 0.7))); }
"""
    return finish(g, "house corrugated", ins, body, tile_m=1.0, depth_m=0.018, ao_radius=12, ao_strength=0.9)


def ribbed():
    """Ribbed aluminium siding (trailers, mobile homes): 6 panels of 167 mm in the 1 m tile, each with a raised
    trapezoid rib, painted near white and chalked a little."""
    g = Graph("House Ribbed")
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("mid", "f", fbm(g, 16, iterations=4, offset=0.4)),
           ("streak", "f", g.node("fbm2", noise=2, scale_x=32, scale_y=2, folds=0, iterations=4, persistence=0.5, offset=0.6)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.2))]
    body = """
float ds_$name(vec2 uv) { float xs = fract(uv.x * 6.0) / 6.0; return min(xs, 1.0 / 6.0 - xs); }
float height_$name(vec2 uv) { return 0.15 + 0.8 * smoothstep(0.017, 0.009, ds_$name(uv)) + 0.004 * $fine(uv); }
vec3 albedo_$name(vec2 uv) {
	float v = 0.88 + 0.025 * ($mid(uv) - 0.5);
	v *= 1.0 - 0.06 * smoothstep(0.55, 0.8, $streak(uv));
	v *= 0.96 + 0.05 * $lo(uv);
	return vec3(v, v * 0.997, v * 0.985);
}
float rough_$name(vec2 uv) { return 0.42 + 0.08 * $mid(uv) + 0.06 * smoothstep(0.5, 0.8, $streak(uv)); }
float metal_$name(vec2 uv) { return 0.4; }
"""
    return finish(g, "house ribbed", ins, body, tile_m=1.0, depth_m=0.012, ao_radius=12, ao_strength=1.0)


def skirt():
    """Vinyl skirting under a mobile home: vertical panels of 125 mm with a V-groove between, embossed wood grain, light
    near neutral."""
    g = Graph("House Skirt")
    ins = [("lo", "f", fbm(g, 4, iterations=4)),
           ("grain", "f", g.node("fbm2", noise=2, scale_x=128, scale_y=6, folds=0, iterations=4, persistence=0.55, offset=0.3)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.9))]
    body = """
float ds_$name(vec2 uv) { float xs = fract(uv.x * 8.0) * 0.125; return min(xs, 0.125 - xs); }
float height_$name(vec2 uv) {
	float d = ds_$name(uv);
	return 0.2 + 0.65 * smoothstep(0.004, 0.012, d) + 0.04 * $grain(uv);
}
vec3 albedo_$name(vec2 uv) {
	float v = 0.86 + 0.03 * ($grain(uv) - 0.5) + 0.02 * ($fine(uv) - 0.5);
	v *= 0.93 + 0.09 * $lo(uv);
	return vec3(v, v * 0.99, v * 0.97);
}
float rough_$name(vec2 uv) { return 0.58 + 0.06 * $grain(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house skirt", ins, body, tile_m=1.0, depth_m=0.01, ao_radius=12, ao_strength=1.0)


def brushed():
    """Brushed stainless steel: fine straight brushing along the surface, faint smudges. Fully metal."""
    g = Graph("House Brushed")
    ins = [("s1", "f", g.node("fbm2", noise=2, scale_x=2, scale_y=512, folds=0, iterations=3, persistence=0.5, offset=0.2)),
           ("s2", "f", g.node("fbm2", noise=2, scale_x=8, scale_y=1024, folds=0, iterations=2, persistence=0.5, offset=0.5)),
           ("lo", "f", fbm(g, 6, iterations=4, offset=0.1))]
    body = """
float height_$name(vec2 uv) { return 0.5 + 0.3 * ($s1(uv) - 0.5) + 0.2 * ($s2(uv) - 0.5); }
vec3 albedo_$name(vec2 uv) {
	float v = 0.8 + 0.05 * ($s1(uv) - 0.5) + 0.04 * ($s2(uv) - 0.5);
	return vec3(v * 0.98, v * 0.99, v);
}
float rough_$name(vec2 uv) { return 0.26 + 0.08 * $s1(uv) + 0.1 * smoothstep(0.55, 0.8, $lo(uv)); }
float metal_$name(vec2 uv) { return 1.0; }
"""
    return finish(g, "house brushed", ins, body, tile_m=1.0, depth_m=0.0003, ao_radius=4, ao_strength=0.3)


# ----------------------------------------------------------------------------
# paved
# ----------------------------------------------------------------------------

def asphalt():
    """Asphalt paving over the 2 m tile: dark binder, 12 mm stones worn through to grey, fines, and a few fine cracks."""
    g = Graph("House Asphalt")
    v = vor(g, 160, randomness=1.0)
    c = vor(g, 4, randomness=0.9)
    ins = [("lo", "f", fbm(g, 4, iterations=5, persistence=0.6)), ("mid", "f", fbm(g, 24, iterations=4, offset=0.5)),
           ("st", "f", (v, 1)), ("stc", "rgb", (v, 2)), ("fine", "f", fbm(g, 384, iterations=3, offset=0.3)),
           ("crack", "f", (c, 1))]
    body = """
float stone_$name(vec2 uv) { return smoothstep(0.05, 0.22, $st(uv)) * step(0.45, $stc(uv).g); }
float crack_$name(vec2 uv) {
	float w = 1.0 - smoothstep(0.0, 0.01, $crack(uv + 0.015 * vec2($mid(uv) - 0.5, $mid(uv + vec2(0.5)) - 0.5)));
	return w * smoothstep(0.58, 0.7, $lo(uv + vec2(0.6, 0.2)));
}
float height_$name(vec2 uv) {
	return 0.42 + 0.3 * sqrt(stone_$name(uv)) + 0.15 * ($fine(uv) - 0.5) - 0.35 * crack_$name(uv);
}
vec3 albedo_$name(vec2 uv) {
	vec3 bind = vec3(0.24, 0.24, 0.245) * (0.85 + 0.3 * $fine(uv));
	vec3 st = mix(vec3(0.42, 0.41, 0.4), vec3(0.6, 0.58, 0.55), $stc(uv).r);
	vec3 c = mix(bind, st, 0.8 * stone_$name(uv));
	c *= 0.9 + 0.16 * $lo(uv) + 0.06 * ($mid(uv) - 0.5);
	return c * (1.0 - 0.55 * crack_$name(uv));
}
float rough_$name(vec2 uv) { return 0.93 - 0.08 * stone_$name(uv) + 0.04 * $fine(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house asphalt", ins, body, tile_m=2.0, depth_m=0.006, ao_radius=6, ao_strength=1.2)


# ----------------------------------------------------------------------------
# interiors
# ----------------------------------------------------------------------------

def smooth():
    """Smooth paint (trim, plain painted surfaces, soffits, plaster and every other smooth key): a faint roller stipple
    and orange peel, near white so the paint shows true."""
    g = Graph("House Smooth")
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("mid", "f", fbm(g, 24, iterations=4, offset=0.5)),
           ("peel", "f", fbm(g, 128, iterations=3, offset=0.2)), ("fine", "f", fbm(g, 512, iterations=2, offset=0.7))]
    body = """
float height_$name(vec2 uv) { return 0.5 + 0.3 * ($peel(uv) - 0.5) + 0.2 * ($fine(uv) - 0.5); }
vec3 albedo_$name(vec2 uv) {
	float v = 0.92 + 0.02 * ($mid(uv) - 0.5) + 0.012 * ($fine(uv) - 0.5);
	v *= 0.98 + 0.03 * $lo(uv);
	return vec3(v, v * 0.997, v * 0.99);
}
float rough_$name(vec2 uv) { return 0.48 + 0.06 * $peel(uv) + 0.04 * $lo(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house smooth", ins, body, tile_m=1.0, depth_m=0.0003, ao_radius=4, ao_strength=0.3)


def laminate():
    """A 1980s plastic laminate countertop: near-white with fine grey and beige flecks, glossy."""
    g = Graph("House Laminate")
    v = vor(g, 256, randomness=1.0)
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("fl", "f", (v, 0)), ("flc", "rgb", (v, 2)),
           ("fine", "f", fbm(g, 256, iterations=2, offset=0.4))]
    body = """
float fleck_$name(vec2 uv) { return smoothstep(0.32, 0.22, $fl(uv)) * step(0.7, $flc(uv).r); }
float height_$name(vec2 uv) { return 0.5 + 0.5 * ($fine(uv) - 0.5); }
vec3 albedo_$name(vec2 uv) {
	vec3 c = vec3(0.9, 0.885, 0.85) * (0.98 + 0.03 * $lo(uv));
	vec3 f = mix(vec3(0.55, 0.54, 0.52), vec3(0.74, 0.66, 0.55), $flc(uv).g);
	return mix(c, f, fleck_$name(uv));
}
float rough_$name(vec2 uv) { return 0.3 + 0.05 * $lo(uv); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house laminate", ins, body, tile_m=1.0, depth_m=0.0004, ao_radius=4, ao_strength=0.2)


def floor_tile():
    """Black and white checkerboard floor tiles of 450 mm with 3 mm grout, 2 x 2 in the 0.9 m tile: marbled streaks,
    eased edges, scuffs on the glaze."""
    g = Graph("House Floor Tile")
    ins = [("lo", "f", fbm(g, 4, iterations=4)),
           ("marb", "f", g.node("fbm2", noise=2, scale_x=6, scale_y=18, folds=0, iterations=5, persistence=0.6, offset=0.3)),
           ("scuff", "f", fbm(g, 32, iterations=4, offset=0.8)), ("fine", "f", fbm(g, 256, iterations=3, offset=0.1))]
    body = """
vec4 cell_$name(vec2 uv) { vec2 p = uv * 2.0; return vec4(fract(p) * 0.45, mod(floor(p), 2.0)); }
float edge_$name(vec2 uv) { vec4 c = cell_$name(uv); return min(min(c.x, 0.45 - c.x), min(c.y, 0.45 - c.y)) - 0.0015; }
float height_$name(vec2 uv) {
	float d = edge_$name(uv);
	return mix(0.2 + 0.05 * $fine(uv), 0.85 + 0.01 * $fine(uv), smoothstep(0.0, 0.0025, d));
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float white = 1.0 - mod(c.z + c.w, 2.0);
	float m = smoothstep(0.55, 0.75, $marb(uv + vec2(c.z * 0.37, c.w * 0.21)));
	vec3 w = vec3(0.9, 0.89, 0.86) * (1.0 - 0.07 * m) * (1.0 - 0.06 * smoothstep(0.6, 0.8, $scuff(uv)));
	vec3 b = vec3(0.12, 0.12, 0.13) * (1.0 + 0.6 * m);
	vec3 t = mix(b, w, white);
	vec3 grout = vec3(0.55, 0.54, 0.5) * (0.88 + 0.2 * $fine(uv));
	return mix(grout, t, smoothstep(0.0, 0.001, edge_$name(uv)));
}
float rough_$name(vec2 uv) {
	float t = 0.18 + 0.15 * smoothstep(0.55, 0.8, $scuff(uv)) + 0.04 * $lo(uv);
	return mix(0.88, t, smoothstep(0.0, 0.001, edge_$name(uv)));
}
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house floor tile", ins, body, tile_m=0.9, depth_m=0.003, ao_radius=6, ao_strength=1.0)


def wall_tile():
    """Glazed ceramic wall tiles of 111 mm (9 to the metre) with 2 mm grout and cushioned edges: near-white glaze so the
    tile colour shows true, glossy, the grout matt."""
    g = Graph("House Wall Tile")
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("mid", "f", fbm(g, 18, iterations=4, offset=0.4)),
           ("fine", "f", fbm(g, 256, iterations=3, offset=0.6))]
    body = """
vec4 cell_$name(vec2 uv) { vec2 p = uv * 9.0; return vec4(fract(p) / 9.0, mod(floor(p), 9.0)); }
float edge_$name(vec2 uv) { vec4 c = cell_$name(uv); float s = 1.0 / 9.0; return min(min(c.x, s - c.x), min(c.y, s - c.y)) - 0.001; }
float height_$name(vec2 uv) {
	float d = edge_$name(uv);
	return mix(0.15 + 0.05 * $fine(uv), 0.88 + 0.04 * ($mid(uv) - 0.5), smoothstep(0.0, 0.004, d));
}
vec3 albedo_$name(vec2 uv) {
	vec4 c = cell_$name(uv);
	float v = 0.93 * (0.975 + 0.04 * hs_$name(c.zw)) * (0.985 + 0.02 * $mid(uv));
	float grout = 0.66 * (0.9 + 0.15 * $fine(uv)) * (0.92 + 0.1 * $lo(uv));
	float t = mix(grout, v, smoothstep(0.0, 0.0008, edge_$name(uv)));
	return vec3(t, t, t * 0.99);
}
float rough_$name(vec2 uv) { return mix(0.85, 0.1 + 0.05 * $mid(uv), smoothstep(0.0, 0.0008, edge_$name(uv))); }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house wall tile", ins, body, tile_m=1.0, depth_m=0.003, ao_radius=5, ao_strength=1.0)


def carpet():
    """Cut-pile carpet: tufts, a faint shading where the pile lies different ways, light warm neutral so the carpet
    colour shows true."""
    g = Graph("House Carpet")
    ins = [("lo", "f", fbm(g, 4, iterations=4, persistence=0.6)), ("n64", "f", fbm(g, 64, iterations=3, offset=0.3)),
           ("tuft", "f", fbm(g, 256, iterations=2, offset=0.6)), ("fine", "f", fbm(g, 512, iterations=2, offset=0.1))]
    body = """
float height_$name(vec2 uv) { return 0.5 * $tuft(uv) + 0.35 * $fine(uv) + 0.15 * $n64(uv); }
vec3 albedo_$name(vec2 uv) {
	float v = 0.78 * (0.84 + 0.16 * $tuft(uv) + 0.08 * $fine(uv)) * (0.95 + 0.08 * $lo(uv));
	return vec3(v, v * 0.96, v * 0.92);
}
float rough_$name(vec2 uv) { return 0.95; }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house carpet", ins, body, tile_m=1.0, depth_m=0.004, ao_radius=4, ao_strength=1.0)


def fabric():
    """Upholstery cloth: a plain weave of 3.9 mm threads, 128 each way in the 0.5 m tile, slubbed and heathered, light
    near neutral so the cloth colour shows true."""
    g = Graph("House Fabric")
    ins = [("lo", "f", fbm(g, 4, iterations=4)), ("heather", "f", fbm(g, 64, iterations=3, offset=0.4)),
           ("slubx", "f", g.node("fbm2", noise=2, scale_x=32, scale_y=256, folds=0, iterations=2, persistence=0.5, offset=0.3)),
           ("sluby", "f", g.node("fbm2", noise=2, scale_x=256, scale_y=32, folds=0, iterations=2, persistence=0.5, offset=0.7))]
    body = """
float over_$name(vec2 uv) { vec2 c = floor(uv * 128.0); return mod(c.x + c.y, 2.0); }
float height_$name(vec2 uv) {
	vec2 f = fract(uv * 128.0);
	float warp = sin(3.14159 * f.x) * (0.55 + 0.45 * sin(3.14159 * f.y));
	float weft = sin(3.14159 * f.y) * (0.55 + 0.45 * sin(3.14159 * f.x));
	return 0.15 + 0.8 * mix(weft, warp, over_$name(uv));
}
vec3 albedo_$name(vec2 uv) {
	float o = over_$name(uv);
	float v = mix(0.8 + 0.08 * ($sluby(uv) - 0.5), 0.86 + 0.08 * ($slubx(uv) - 0.5), o);
	v *= 0.93 + 0.1 * $heather(uv) + 0.04 * ($lo(uv) - 0.5);
	vec2 f = fract(uv * 128.0);
	v *= 0.82 + 0.18 * sin(3.14159 * mix(f.y, f.x, o));
	return vec3(v, v * 0.98, v * 0.96);
}
float rough_$name(vec2 uv) { return 0.9; }
float metal_$name(vec2 uv) { return 0.0; }
"""
    return finish(g, "house fabric", ins, body, tile_m=0.5, depth_m=0.0008, ao_radius=3, ao_strength=0.8)


# Every generator of the kit's SURFACES table, in that table's order, to its graph (written as house_<generator>.ptex).
GRAPHS = {
    "brick": brick,
    "block": block,
    "siding": siding,
    "board": board,
    "stucco": stucco,
    "adobe": adobe,
    "shingle": shingle,
    "tile": tile,
    "metalRoof": metal_roof,
    "corrugated": corrugated,
    "ribbed": ribbed,
    "skirt": skirt,
    "concrete": concrete,
    "roofing": roofing,
    "asphalt": asphalt,
    "smooth": smooth,
    "shutter": shutter,
    "door": door,
    "brushed": brushed,
    "laminate": laminate,
    "wood": wood,
    "planks": planks,
    "pine": pine,
    "floorTile": floor_tile,
    "wallTile": wall_tile,
    "carpet": carpet,
    "fabric": fabric,
}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    only = set(sys.argv[1:])
    for name, fn in GRAPHS.items():
        if only and name not in only:
            continue
        path = os.path.join(OUT_DIR, f"house_{name}.ptex")
        with open(path, "w") as f:
            json.dump(fn().to_json(), f, indent=1, sort_keys=True)
        print("wrote", os.path.relpath(path))


if __name__ == "__main__":
    main()
