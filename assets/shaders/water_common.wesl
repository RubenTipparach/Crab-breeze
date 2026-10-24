// The water's shared math (openspec change `water-rendering`): the one copy of the noise, the
// light lost through water and the drops on the glass, imported by the surface material
// (`water_surface.wesl`) and the fullscreen passes (`water_post.wesl`). No bindings here.
//
// Provenance: Pale Blue Dot (`RubenTipparach/pale-blue-dot` at `db25fe2`,
// `assets/shaders/water.wgsl`), which carries Tenebris's `water.fs.glsl` and `composite.fs.glsl`
// (`RubenTipparach/tenebris` at `ef98651`) term for term. The drop field is "Heartfelt" by Martijn
// Steinrucken (BigWings), 2017, CC BY-NC-SA 3.0, as Tenebris carries it (design: open question
// WR3). Changes from the source: the drop functions take their density and size as arguments
// instead of reading a uniform, and one dead assignment in `rd_drop_layer` is dropped.

fn safe_normal(v: vec3<f32>) -> vec3<f32> {
    return v * inverseSqrt(max(dot(v, v), 1e-12));
}

// ---- Gradient noise and fbm, hash constants as Tenebris has them ----------
fn gn_fade(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn gn_hash(p: vec3<i32>) -> u32 {
    // Integer wrapping preserves the original lattice hash without signed overflow.
    let q = bitcast<vec3<u32>>(p) + vec3<u32>(1073741824u);
    var h = (q.x * 374761393u) ^ (q.y * 668265263u) ^ (q.z * 1274126177u);
    h = (h ^ (h >> 13u)) * 1103515245u;
    return h ^ (h >> 16u);
}

fn gn_grad(hash: u32, p: vec3<f32>) -> f32 {
    let h = hash & 15u;
    let u = select(p.y, p.x, h < 8u);
    let v = select(select(p.z, p.x, h == 12u || h == 14u), p.y, h < 4u);
    return select(u, -u, (h & 1u) != 0u) + select(v, -v, (h & 2u) != 0u);
}

fn gnoise3(p: vec3<f32>) -> f32 {
    let lattice = vec3<i32>(floor(p));
    let f = fract(p);
    let s = gn_fade(f);
    var values: array<f32, 8>;
    for (var i = 0u; i < 8u; i++) {
        let corner = vec3<i32>(i32(i & 1u), i32((i >> 1u) & 1u), i32((i >> 2u) & 1u));
        values[i] = gn_grad(gn_hash(lattice + corner), f - vec3<f32>(corner));
    }
    return mix(mix(mix(values[0], values[1], s.x), mix(values[2], values[3], s.x), s.y),
               mix(mix(values[4], values[5], s.x), mix(values[6], values[7], s.x), s.y), s.z);
}

fn fbm3(p: vec3<f32>, t: f32) -> f32 {
    var q = p * 0.9 + vec3<f32>(0.35, 0.18, -0.42) * t;
    var h = gnoise3(q) * 0.5;
    q = q * 2.1 + vec3<f32>(-0.22, 0.33, 0.17) * t;
    h += gnoise3(q) * 0.28;
    q = q * 2.3 + vec3<f32>(0.19, -0.27, 0.11) * t;
    return h + gnoise3(q) * 0.15;
}

// ---- Light through water ----------------------------------------------------
// The share of light left after `path_m` metres of water absorbing `absorption` per metre (Beer and
// Lambert, per channel). The one copy: the surface's refraction, the murk and the light falling
// onto drowned ground all use it.
fn water_transmittance(absorption: vec3<f32>, path_m: f32) -> vec3<f32> {
    return exp(-max(absorption, vec3<f32>(0.0)) * max(path_m, 0.0));
}

// The water body's own colour seen `depth_m` below the surface: the deep colour lit like an albedo
// by `light` (the light reaching the water, times exposure), dimmed by the water above. Pale Blue
// Dot's `murk`, without its night floor.
fn water_murk(deep: vec3<f32>, light: vec3<f32>, absorption: vec3<f32>, depth_m: f32) -> vec3<f32> {
    return deep * light * water_transmittance(absorption, depth_m);
}

// ---- Drops on the glass: "Heartfelt" by Martijn Steinrucken (BigWings), 2017, CC BY-NC-SA 3.0,
// as Tenebris and Pale Blue Dot carry it. `density` is Pale Blue Dot's `lens.x`, `size` its
// `lens.w`. The frame is the reference's: y up.
fn rd_n13(p: f32) -> vec3<f32> {
    var p3 = fract(vec3<f32>(p) * vec3<f32>(0.1031, 0.11369, 0.13787));
    p3 += dot(p3, p3.yzx + 19.19);
    return fract(vec3<f32>((p3.x + p3.y) * p3.z, (p3.x + p3.z) * p3.y, (p3.y + p3.z) * p3.x));
}

fn rd_n(t: f32) -> f32 {
    return fract(sin(t * 12345.564) * 7658.76);
}

fn rd_saw(b: f32, t: f32) -> f32 {
    return smoothstep(0.0, b, t) * smoothstep(1.0, b, t);
}

fn rd_drop_layer(uv_in: vec2<f32>, t: f32, density: f32, size: f32) -> vec2<f32> {
    var uv = uv_in;
    let whole = uv_in;
    uv.y += t * 0.75;
    let a = vec2<f32>(6.0, 1.0);
    let grid = a * 2.0 * max(size, 0.05);
    var id = floor(uv * grid);
    uv.y += rd_n(id.x);
    id = floor(uv * grid);
    let n = rd_n13(id.x * 35.2 + id.y * 2376.1);
    let keep = step(1.0 - clamp(density, 0.0, 1.0), fract(n.x * 3.17 + n.y * 6.91 + n.z));
    let st = fract(uv * grid) - vec2<f32>(0.5, 0.0);
    var x = n.x - 0.5;
    var y = whole.y * 20.0;
    let wiggle = sin(y + sin(y));
    x += wiggle * (0.5 - abs(x)) * (n.z - 0.5);
    x *= 0.7;
    let ti = fract(t + n.z);
    // Monotonic fall, faded at both ends so the wrap is never a flick upward.
    y = 0.92 - ti * 0.88;
    let fade = smoothstep(0.0, 0.12, ti) * smoothstep(1.0, 0.86, ti);
    let p = vec2<f32>(x, y);
    let d = length((st - p) * a.yx);
    let main_drop = smoothstep(0.4, 0.0, d);
    let r = sqrt(smoothstep(1.0, y, st.y));
    let cd = abs(st.x - x);
    var trail = smoothstep(0.23 * r, 0.15 * r * r, cd);
    let trail_front = smoothstep(-0.02, 0.02, st.y - y);
    trail *= trail_front * r * r;
    y = fract(whole.y * 10.0) + (st.y - 0.5);
    let dd = length(st - vec2<f32>(x, y));
    let droplets = smoothstep(0.3, 0.0, dd);
    let m = main_drop + droplets * r * trail_front;
    return vec2<f32>(m, trail) * (keep * fade);
}

fn rd_static_drops(uv_in: vec2<f32>, t: f32, density: f32, size: f32) -> f32 {
    var uv = uv_in * 40.0 * max(size, 0.05);
    let id = floor(uv);
    uv = fract(uv) - 0.5;
    let n = rd_n13(id.x * 107.45 + id.y * 3543.654);
    let keep = step(1.0 - clamp(density, 0.0, 1.0), fract(n.x * 3.17 + n.y * 6.91 + n.z));
    let p = (n.xy - 0.5) * 0.7;
    let d = length(uv - p);
    let fade = rd_saw(0.025, fract(t + n.z));
    return smoothstep(0.3, 0.0, d) * fract(n.z * 10.0) * fade * keep;
}

// The drops' coverage (x) and trails (y) at `uv`, three layers weighted `l0`, `l1`, `l2`.
fn rd_drops(uv: vec2<f32>, t: f32, l0: f32, l1: f32, l2: f32, density: f32, size: f32) -> vec2<f32> {
    let s = rd_static_drops(uv, t, density, size) * l0;
    let m1 = rd_drop_layer(uv, t, density, size) * l1;
    let m2 = rd_drop_layer(uv * 1.85, t, density, size) * l2;
    var c = s + m1.x + m2.x;
    // Density above one stacks same-size layers rather than shrinking drops.
    let extra = clamp(density - 1.0, 0.0, 2.0);
    if (extra > 0.001) {
        let m3 = rd_drop_layer(uv + vec2<f32>(0.50, 0.37), t, density, size);
        let m4 = rd_drop_layer(uv * 1.85 + vec2<f32>(0.27, 0.61), t, density, size);
        c += (m3.x * l1 + m4.x * l2) * min(extra, 1.0);
    }
    c = smoothstep(0.3, 1.0, c);
    return vec2<f32>(c, max(m1.y * l0, m2.y * l1));
}
