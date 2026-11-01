// The airfield kit for Crab Breeze's 3D site mockups (the airport, the military base): runways with their markings,
// taxiways, fillets, links and holding points, hangars, the tower, windsocks, helipads, and the runway lights' one rule
// with the glow dots that draw them. One implementation inlined by gen_airport.py and gen_military_base.py, after
// site_kit.js; the page defines T, PI, M (its materials), S and E (the site buildings and the engine), put(), and
// TAXI_W first. Metres; x east, y up, z south.
// Paving from p to q ([x, z]), w wide, flush with the runway.
const pave = (g, p, q, w, m, top = TOP.taxi) => beam(g, [p[0], top - 0.31, p[1]], [q[0], top - 0.31, q[1]], w, 0.62, m, false);
// Every flat layer has its own height, at least 3 cm from the next, so no two surfaces share a plane (CLAUDE.md
// section 10: no z-fighting). Grass -0.3; strips -0.15 and -0.05; aprons 0.05; taxiways 0.12; the secondary runway
// 0.18 and the main 0.24 (it crosses over); paint on aprons and taxiways 0.2, on a runway 3 and 6 cm over it.
const TOP = { stripMain: -0.15, stripCross: -0.05, apron: 0.05, taxi: 0.12, cross: 0.18, main: 0.24 };
const PAINT = 0.2, PAINT_LOW = 0.17;

// ---------------------------------------------------------------- runways
function runway(R) {
  const len = Math.hypot(R.b[0] - R.a[0], R.b[1] - R.a[1]), d = [(R.b[0] - R.a[0]) / len, (R.b[1] - R.a[1]) / len];
  const right = [-d[1], d[0]];
  const at = (u, v) => [R.a[0] + d[0] * u + right[0] * v, R.a[1] + d[1] * u + right[1] * v];
  const bearing = (Math.atan2(d[0], -d[1]) * 180 / PI + 360) % 360;
  const read = deg => { const n = Math.round(deg / 10) % 36; return n === 0 ? 36 : n; };
  return { ...R, len, d, right, at, w2: R.w / 2, des: [read(bearing), read(bearing + 180)] };
}
const SEG = [[0, 9, 5, 9], [5, 4.5, 5, 9], [5, 0, 5, 4.5], [0, 0, 5, 0], [0, 0, 0, 4.5], [0, 4.5, 0, 9], [0, 4.5, 5, 4.5]];
const ON = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];
const BLAST = 60, AIM = 400;
// `under`, if given, is a runway this one passes beneath: this one's markings stop at its edges, as at a real crossing
// (the higher runway's markings run through), so no paint lies in the other's surface.
function drawRunway(g, rw, full, top, under) {
  pave(g, rw.a, rw.b, rw.w, M.asphalt, top);
  let hole = null;
  if (under) {   // the stretch of this runway's centre line, in metres from rw.a, whose width overlaps `under`
    const n = under.right, off = q => (q[0] - under.a[0]) * n[0] + (q[1] - under.a[1]) * n[1];
    const o0 = off(rw.a), dO = rw.d[0] * n[0] + rw.d[1] * n[1], reach = under.w2 + 2 + Math.abs(rw.right[0] * n[0] + rw.right[1] * n[1]) * rw.w2;
    if (Math.abs(dO) > 1e-6) { const u1 = (-reach - o0) / dO, u2 = (reach - o0) / dO; hole = [Math.min(u1, u2), Math.max(u1, u2)]; }
  }
  const stroke = (g2, P, Q, w2, y2, m) => {
    if (!hole) return strokeAt(g2, P, Q, w2, y2, m);
    const u = q => (q[0] - rw.a[0]) * rw.d[0] + (q[1] - rw.a[1]) * rw.d[1], uP = u(P), uQ = u(Q);
    const lerp = t => [P[0] + (Q[0] - P[0]) * t, P[1] + (Q[1] - P[1]) * t];
    const lo = Math.min(uP, uQ), hi = Math.max(uP, uQ);
    if (hi <= hole[0] || lo >= hole[1]) return strokeAt(g2, P, Q, w2, y2, m);
    const t = v => (v - uP) / ((uQ - uP) || 1e-9);
    if (lo < hole[0]) strokeAt(g2, lerp(t(lo)), lerp(t(hole[0])), w2, y2, m);
    if (hi > hole[1]) strokeAt(g2, lerp(t(hole[1])), lerp(t(hi)), w2, y2, m);
  };
  const y = top + 0.06, yc = top + 0.03, w = rw.w2;
  stroke(g, rw.at(0, w - 1), rw.at(rw.len, w - 1), 0.45, y, M.white);
  stroke(g, rw.at(0, -w + 1), rw.at(rw.len, -w + 1), 0.45, y, M.white);
  const bars = rw.w >= 40 ? 8 : 6;
  for (const end of [0, 1]) {
    const from = end === 0 ? rw.a : rw.b, d = end === 0 ? rw.d : [-rw.d[0], -rw.d[1]], right = [-d[1], d[0]];
    const at = (u, v) => [from[0] + d[0] * u + right[0] * v, from[1] + d[1] * u + right[1] * v];
    if (!full) { for (let k = 0; k < 8; k++) { const o = (k - 3.5) * 4; stroke(g, at(8, o), at(38, o), 0.9, y, M.white); } continue; }
    for (let k = 0; k < 3; k++) { const tip = 15 + k * 15; stroke(g, at(tip, 0), at(tip - 12, w - 2), 0.9, yc, M.yellow); stroke(g, at(tip, 0), at(tip - 12, -w + 2), 0.9, yc, M.yellow); }
    for (let k = 0; k < bars; k++) { const o = (k - (bars - 1) / 2) * 4.6; stroke(g, at(BLAST + 3, o), at(BLAST + 33, o), 0.9, y, M.white); }
    const n = rw.des[end], digits = [Math.floor(n / 10), n % 10];
    digits.forEach((dg, i) => { const left = i === 0 ? -6.5 : 1.5; SEG.forEach((s, k) => { if (ON[dg] & (1 << k)) stroke(g, at(BLAST + 45 + s[1], left + s[0]), at(BLAST + 45 + s[3], left + s[2]), 0.6, y, M.white); }); });
    for (const [dist, nb] of [[150, 3], [300, 3], [600, 2], [750, 1]]) {
      if (BLAST + dist + 22.5 > rw.len / 2) continue;
      for (let k = 0; k < nb; k++) for (const s of [-1, 1]) { const o = s * (4.5 + k * 3) * rw.w / 45; stroke(g, at(BLAST + dist, o), at(BLAST + dist + 22.5, o), 0.9, y, M.white); }
    }
    if (BLAST + AIM + 50 < rw.len / 2) for (const s of [-1, 1]) stroke(g, at(BLAST + AIM, s * 10.5 * rw.w / 45), at(BLAST + AIM + 50, s * 10.5 * rw.w / 45), 4.5 * rw.w / 45, y, M.white);
    // PAPI on the approaching pilot's left, approach lights out past the end.
    for (let k = 0; k < 4; k++) {
      const p = at(BLAST + AIM, -w - 15 - k * 9);
      box(g, [p[0] - 0.8, 0, p[1] - 0.8], [p[0] + 0.8, 1, p[1] + 0.8], M.dark);
      box(g, [p[0] - 0.5, 1, p[1] - 0.5], [p[0] + 0.5, 1.3, p[1] + 0.5], k < 2 ? M.lamp : M.lampRed, false);
    }
    for (let i = 1; i <= 10; i++) {
      const half = i % 3 === 0 ? 9 : 6, p = at(-i * 30, 0);
      box(g, [p[0] - 0.15, -0.3, p[1] - 0.15], [p[0] + 0.15, 1.4, p[1] + 0.15], M.fence);
      const l = at(-i * 30, -half), r = at(-i * 30, half);
      beam(g, [l[0], 1.3, l[1]], [r[0], 1.3, r[1]], 0.2, 0.2, M.fence);
      for (let k = 0; k < 5; k++) { const q = at(-i * 30, -half + k * half / 2); box(g, [q[0] - 0.3, 1.4, q[1] - 0.3], [q[0] + 0.3, 1.8, q[1] + 0.3], M.lamp, false); }
    }
  }
  if (full) for (let u = BLAST + 70; u + 30 <= rw.len - BLAST - 70; u += 50) stroke(g, rw.at(u, 0), rw.at(u + 30, 0), 0.45, y, M.white);
}
// A taxi way: paving and its yellow centre line.
function taxi(g, p, q, w = TAXI_W) { pave(g, p, q, w, M.taxi); stroke(g, p, q, 0.3, PAINT, M.yellow); }
// Where a taxiway meets a paved edge (a runway's or the parallel taxiway's), it widens into it on curved fillets, as a
// real junction does. p is where the centre line meets the edge, dir the taxiway's direction away from the edge, e the
// edge's direction; R the fillet radius. Laid just under the taxiway's top so it never shares its plane.
function fillet(g, p, dir, e, w = TAXI_W, R = 25) {
  const n = [-dir[1], dir[0]], cross2 = (a, b) => a[0] * b[1] - a[1] * b[0], pts = [];
  for (const s of [1, -1]) {
    const S0 = [p[0] + n[0] * s * w / 2, p[1] + n[1] * s * w / 2];
    const den = cross2(dir, e); if (Math.abs(den) < 1e-6) return;
    const t = cross2([p[0] - S0[0], p[1] - S0[1]], e) / den, C = [S0[0] + dir[0] * t, S0[1] + dir[1] * t];
    const out = (e[0] * n[0] + e[1] * n[1]) * s > 0 ? 1 : -1;
    const A = [C[0] + dir[0] * R, C[1] + dir[1] * R], B = [C[0] + e[0] * out * R, C[1] + e[1] * out * R], side = [];
    for (let k = 0; k <= 10; k++) { const u = k / 10, a = (1 - u) * (1 - u), b = 2 * u * (1 - u), c = u * u;
      side.push([a * B[0] + b * C[0] + c * A[0], a * B[1] + b * C[1] + c * A[1]]); }
    pts.push(...(s === 1 ? side : side.reverse()));
  }
  const shape = new T.Shape(pts.map(q => new T.Vector2(q[0], -q[1])));
  const mesh = new T.Mesh(new T.ExtrudeGeometry(shape, { depth: 0.5, bevelEnabled: false }), M.taxi);
  mesh.rotation.x = -PI / 2; mesh.position.y = TOP.taxi - 0.03 - 0.5; mesh.receiveShadow = true; g.add(mesh);
}
// A taxiway from a paved edge at p to another at q: its ends tucked 30 m under what it meets and filleted into it.
function link(g, p, q, eP, eQ, w = TAXI_W, R = 25) {
  const len = Math.hypot(q[0] - p[0], q[1] - p[1]), d = [(q[0] - p[0]) / len, (q[1] - p[1]) / len];
  pave(g, [p[0] - d[0] * 30, p[1] - d[1] * 30], q, w, M.taxi); stroke(g, p, q, 0.3, PAINT, M.yellow);
  fillet(g, p, d, eP, w, R);
  if (eQ) fillet(g, q, [-d[0], -d[1]], eQ, w, R);
}
// A holding position across a taxiway at p, the line across `across` (unit), w wide: two solid, two dashed.
function hold(g, p, along, w = TAXI_W) {
  const across = [-along[1], along[0]], a = [p[0] - across[0] * w / 2, p[1] - across[1] * w / 2], b = [p[0] + across[0] * w / 2, p[1] + across[1] * w / 2];
  for (const k of [0, 1]) { const o = 1 + k * 0.9; stroke(g, [a[0] + along[0] * o, a[1] + along[1] * o], [b[0] + along[0] * o, b[1] + along[1] * o], 0.15, PAINT, M.yellow); }
  for (const k of [0, 1]) { const o = -(0.6 + k * 0.9);
    for (let i = 0; i < 8; i += 2) { const s = [a[0] + (b[0] - a[0]) * i / 8, a[1] + (b[1] - a[1]) * i / 8], e = [a[0] + (b[0] - a[0]) * (i + 1) / 8, a[1] + (b[1] - a[1]) * (i + 1) / 8];
      stroke(g, [s[0] + along[0] * o, s[1] + along[1] * o], [e[0] + along[0] * o, e[1] + along[1] * o], 0.15, PAINT, M.yellow); } }
}

function archHangar(g, x0, x1, z0, z1, wallH, rise, labels, name) {
  const cx = (x0 + x1) / 2, cz = (z0 + z1) / 2;
  put(name || "Hangar", B => S.archHangar(B, E.M, { w: x1 - x0, d: z1 - z0, wallH, rise }), cx, cz, 2);   // its door to the apron (-z)
  stroke(g, [x0 + 2, z0 - 3], [x1 - 2, z0 - 3], 0.6, PAINT, M.yellow);
  if (name) labels.push([name, [cx, cz], wallH + rise + 4]);
}
function tower(g, c, h, labels) {
  put("Control tower", B => S.tower(B, E.M, { h }), c[0], c[1]);
  labels.push(["Control tower, beacon", c, h + 20]);
}
function windsock(g, p, d) {
  box(g, [p[0] - 0.12, 0, p[1] - 0.12], [p[0] + 0.12, 8, p[1] + 0.12], M.white);
  for (let k = 0; k < 5; k++) { const q = [p[0] + d[0] * (0.4 + k * 0.6), p[1] + d[1] * (0.4 + k * 0.6)], r = 0.5 - k * 0.06;
    box(g, [q[0] - r, 7.4 - r, q[1] - r], [q[0] + r, 7.4 + r, q[1] + r], k % 2 ? M.white : M.orange); }
}
function helipad(g, c, labels, name) {
  slab(g, rect(c[0] - 15, c[1] - 15, c[0] + 15, c[1] + 15), -0.5, TOP.apron + 0.05, M.pad);
  for (let k = 0; k < 12; k++) { const t = k / 12 * 4, side = Math.floor(t), f = t - side, P = [[-14, -14], [14, -14], [14, 14], [-14, 14]], a = P[side], b = P[(side + 1) % 4];
    const s = [c[0] + a[0] + (b[0] - a[0]) * f, c[1] + a[1] + (b[1] - a[1]) * f], e = [c[0] + a[0] + (b[0] - a[0]) * (f + 0.18), c[1] + a[1] + (b[1] - a[1]) * (f + 0.18)];
    stroke(g, s, e, 0.6, PAINT + 0.05, M.white); }
  ring(g, c, 10, 0.8, PAINT + 0.05, M.yellow);
  for (const [x0, z0, x1, z1] of [[-3, -4.5, -3, 4.5], [3, -4.5, 3, 4.5], [-3, 0, 3, 0]]) stroke(g, [c[0] + x0, c[1] + z0], [c[0] + x1, c[1] + z1], 1, PAINT + 0.08, M.white);
  if (name) labels.push([name, c, 4]);
}
// The glow dots: at least 3 px across however far, additive, lighting nothing (as the street lamps' far glow).
const dotMat = new T.ShaderMaterial({
  transparent: true, depthWrite: false, blending: T.AdditiveBlending, vertexColors: true, uniforms: { scale: { value: 500 } },
  // the page draws with a logarithmic depth buffer, so the dots write the same depth (three's logdepthbuf chunks)
  vertexShader: "#include <common>\n#include <logdepthbuf_pars_vertex>\nuniform float scale; varying vec3 vC; void main() { vC = color; vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_Position = projectionMatrix * mv; gl_PointSize = max(3.0, 0.9 * scale / -mv.z);\n#include <logdepthbuf_vertex>\n}",
  fragmentShader: "#include <logdepthbuf_pars_fragment>\nvarying vec3 vC; void main() {\n#include <logdepthbuf_fragment>\nfloat r = length(gl_PointCoord - 0.5) * 2.0; float a = smoothstep(1.0, 0.0, r); gl_FragColor = vec4(vC * (a * a * 2.2 + smoothstep(0.35, 0.0, r)), 1.0); }",
});
const DOT = { white: 0xfff6e8, green: 0x37e07a, red: 0xff3a2e, blue: 0x4a7dff, halide: 0xe6eeff, sodium: 0xffb35a };
// The always-lit tower cab: a warm light in the cab, day and night (outside the night group).
const cabLight = new T.PointLight(0xfff0d0, 2.2, 30, 1.2); scene.add(cabLight);
// The runway lights' one rule (section 2d), as the game's (`cb_core::world::lamps::runway_dots`): edge lights every
// 60 m both sides, 3 m outside the paving; the centre line every 30 m between the thresholds' touchdown ends; across
// each threshold, 60 m in, a green bar every 3 m, and 2 m past it a red one (the end, for the other way). Heights over
// the runway's surface. The page draws it; the game draws each exported runway (and the air bases') by its own copy.
function runwayDots(rw, dot) {
  const R = runway(rw), y = rw.y;
  for (let u = 0; u <= R.len + 0.1; u += 60) for (const v of [-(R.w2 + 3), R.w2 + 3]) { const q = R.at(u, v); dot(q[0], y + 0.35, q[1], "white"); }
  for (let u = BLAST + 70; u <= R.len - BLAST - 70; u += 30) { const q = R.at(u, 0); dot(q[0], y + 0.1, q[1], "white"); }
  for (const [u, c] of [[BLAST, "green"], [R.len - BLAST, "green"], [BLAST - 2, "red"], [R.len - BLAST + 2, "red"]])
    for (let v = -R.w2; v <= R.w2 + 0.1; v += 3) { const q = R.at(u, v); dot(q[0], y + 0.2, q[1], c); }
}
