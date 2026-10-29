// Exports the houses and small businesses (openspec/changes/small-buildings) as game kits:
// assets/models/small_buildings/<id>.glb (LOD0, LOD1, LOD2 nodes), <id>.pieces.json (the
// destruction table for LOD0) and manifest.json. The geometry comes from the one kit,
// docs/mockups/small_buildings_kit.js, the same code the Main Street mockup draws, so the game,
// the mockup and these files cannot disagree. Byte-reproducible: same kit, same files.
//
//   node tools/buildings/export_small_buildings.cjs [--seed 7] [--out assets/models/small_buildings]
//
// Refuses (exit 1) a type over its triangle budget for any LOD.
'use strict';
const fs = require('fs');
const path = require('path');
const K = require('../../docs/mockups/small_buildings_kit.js');

const args = process.argv.slice(2);
const opt = (name, def) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : def; };
const SEED = Number(opt('--seed', 7));
const OUT = opt('--out', 'assets/models/small_buildings');
// Triangle budgets per LOD (design section 4). LOD0 includes the interior.
const BUDGET = { LOD0: 16000, LOD1: 7000, LOD2: 120 };
// Per-type overrides, each with its reason.
const BUDGET_FOR = { motel: { LOD1: 8500, why: '28 rooms over 37 m: about three times the facade of the next largest type' } };
const budgetOf = (id, lod) => (BUDGET_FOR[id] && BUDGET_FOR[id][lod]) || BUDGET[lod];
// Switch distances in metres, measured from the nearest body (design section 4).
const LOD_M = { LOD0: 150, LOD1: 600, LOD2: 2000, FURN: 100 };
// LOD1 drops pieces whose bounds hold less than this many cubic metres.
const SMALL_M3 = 0.004;

// ------------------------------------------------------------------ materials
const hex3 = (h) => [((h >> 16) & 255) / 255, ((h >> 8) & 255) / 255, (h & 255) / 255];
const srgbToLinear = (c) => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
const SPECIAL = {
  glass: { base: [0.6, 0.7, 0.74, 0.32], roughness: 0.04, metallic: 0, blend: true },
  glassShop: { base: [0.78, 0.86, 0.87, 0.18], roughness: 0.03, metallic: 0, blend: true },
  glassFar: { base: [0.16, 0.2, 0.23, 1], roughness: 0.08, metallic: 0.3 },        // LOD1 glass: no room behind it
  glassDark: { base: [0.16, 0.21, 0.25, 1], roughness: 0.1, metallic: 0.2 },
  screen: { base: [0.13, 0.13, 0.13, 0.45], roughness: 0.8, metallic: 0, blend: true },
  water: { base: [0.23, 0.65, 0.78, 0.8], roughness: 0.05, metallic: 0, blend: true },
  lamp: { base: [1, 1, 1, 1], roughness: 0.5, metallic: 0, emissive: [1, 0.95, 0.85] },
};
function materialDef(key) {
  const def = { name: key, pbrMetallicRoughness: {}, extras: {} };
  const S = K.SURFACES[key], X = SPECIAL[key];
  if (S) {
    const c = hex3(S[5]).map(srgbToLinear);
    def.pbrMetallicRoughness = { baseColorFactor: [...c, 1], roughnessFactor: S[2], metallicFactor: S[3] };
    def.extras = { surface: key, generator: S[0], tile_m: S[1], standin: true };
  } else if (X) {
    def.pbrMetallicRoughness = { baseColorFactor: X.base, roughnessFactor: X.roughness, metallicFactor: X.metallic };
    if (X.blend) def.alphaMode = 'BLEND';
    if (X.emissive) def.emissiveFactor = X.emissive;
    def.extras = { surface: key };
  } else if (key.startsWith('sign:')) { // text the game draws onto the face (0..1 UVs)
    const [text, bg, fg, neon] = key.slice(5).split('|');
    def.pbrMetallicRoughness = { baseColorFactor: [1, 1, 1, 1], roughnessFactor: 0.5, metallicFactor: 0 };
    def.extras = { surface: 'sign', text, bg, fg, neon: neon === '1' };
  } else if (key.startsWith('awning:')) {
    def.pbrMetallicRoughness = { baseColorFactor: [1, 1, 1, 1], roughnessFactor: 0.85, metallicFactor: 0 };
    def.doubleSided = true;
    def.extras = { surface: 'awning', stripe: key.slice(7), tile_m: 2.4 };
  } else {
    def.pbrMetallicRoughness = { baseColorFactor: [1, 1, 1, 1], roughnessFactor: 0.6, metallicFactor: 0 };
    def.extras = { surface: key };
  }
  return def;
}

// ------------------------------------------------------------------ the three LODs
// A bucket set {mat: {p, n, u, c, t}} filtered to some pieces, materials optionally renamed.
function filterBuckets(parts, keep, rename = {}) {
  const out = {};
  for (const [mat, a] of Object.entries(parts)) {
    const m2 = rename[mat] || mat;
    for (let t = 0; t < a.t.length; t++) {
      if (!keep(a.t[t])) continue;
      const o = out[m2] || (out[m2] = { p: [], n: [], u: [], c: [], t: [] });
      for (let i = t * 9; i < t * 9 + 9; i++) { o.p.push(a.p[i]); o.n.push(a.n[i]); o.c.push(a.c[i]); }
      for (let i = t * 6; i < t * 6 + 6; i++) o.u.push(a.u[i]);
      o.t.push(a.t[t]);
    }
  }
  return out;
}
// LOD2: the volumes the kit recorded (a box to the eaves and its roof), plus one box over any
// load-bearing walls outside them, so every type has a far shape.
function farShapes(B, pieces) {
  const V = B.volumes.slice();
  const inside = (p) => V.some((v) => { const cx = (p.lo[0] + p.hi[0]) / 2, cz = (p.lo[2] + p.hi[2]) / 2; return cx >= v.x0 - 0.3 && cx <= v.x1 + 0.3 && cz >= v.z0 - 0.3 && cz <= v.z1 + 0.3; });
  const walls = pieces.filter((p) => p.runs.length && p.mats.some((m) => K.WALLMATS.includes(m)) && !K.isInterior(p) && !inside(p));
  if (walls.length) {
    const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
    const count = {};
    for (const p of walls) { for (let k = 0; k < 3; k++) { lo[k] = Math.min(lo[k], p.lo[k]); hi[k] = Math.max(hi[k], p.hi[k]); } for (const m of p.mats) if (K.WALLMATS.includes(m)) count[m] = (count[m] || 0) + 1; }
    const wall = Object.keys(count).sort((a, b) => count[b] - count[a] || (a < b ? -1 : 1))[0];
    const tintOf = walls.find((p) => p.meta && p.meta.mat === wall);
    V.push({ x0: lo[0], x1: hi[0], z0: lo[2], z1: hi[2], top: hi[1], ridgeY: hi[1], roof: 'flat', wall, wallTint: tintOf ? tintOf.meta.tint : [1, 1, 1] });
  }
  const F = new K.Builder();
  for (const v of V) {
    F.color(v.wallTint);
    F.box(v.wall, v.x0, 0, v.z0, v.x1, v.top, v.z1, v.roof === 'flat' ? 'y' : 'yY');
    const r = v.ridgeY;
    F.color(v.roofTint != null ? v.roofTint : 0x6d6a66);
    if (v.roof === 'flat') continue;
    const mx = (v.x0 + v.x1) / 2, mz = (v.z0 + v.z1) / 2;
    if (v.roof === 'hip') {
      const along = v.x1 - v.x0 >= v.z1 - v.z0, h = along ? (v.z1 - v.z0) / 2 : (v.x1 - v.x0) / 2;
      const a = along ? [v.x0 + h, r, mz] : [mx, r, v.z0 + h], b = along ? [v.x1 - h, r, mz] : [mx, r, v.z1 - h];
      const c00 = [v.x0, v.top, v.z0], c10 = [v.x1, v.top, v.z0], c11 = [v.x1, v.top, v.z1], c01 = [v.x0, v.top, v.z1];
      if (along) { F.poly(v.mat, [c01, c11, b, a], [0, 1, 1]); F.poly(v.mat, [c10, c00, a, b], [0, 1, -1]); F.poly(v.mat, [c00, c01, a], [-1, 1, 0]); F.poly(v.mat, [c11, c10, b], [1, 1, 0]); }
      // Ridge along z: a is the ridge's z0 end, b its z1 end. Each side runs round its corners in order and each end
      // meets its own end of the ridge (it took them crossed: each side a bow-tie, half of it facing in, and each end
      // reaching to the far end of the ridge; tools/build/check_winding.py, 2026-10-28).
      else { F.poly(v.mat, [c11, c10, a, b], [1, 1, 0]); F.poly(v.mat, [c00, c01, b, a], [-1, 1, 0]); F.poly(v.mat, [c01, c11, b], [0, 1, 1]); F.poly(v.mat, [c10, c00, a], [0, 1, -1]); }
    } else { // gable, ridge along x or z
      const alongZ = v.ridge === 'z';
      const r0 = alongZ ? [mx, r, v.z0] : [v.x0, r, mz], r1 = alongZ ? [mx, r, v.z1] : [v.x1, r, mz];
      const c00 = [v.x0, v.top, v.z0], c10 = [v.x1, v.top, v.z0], c11 = [v.x1, v.top, v.z1], c01 = [v.x0, v.top, v.z1];
      if (alongZ) { F.poly(v.mat, [c00, c01, r1, r0], [-1, 1, 0]); F.poly(v.mat, [c11, c10, r0, r1], [1, 1, 0]); }
      else { F.poly(v.mat, [c01, c11, r1, r0], [0, 1, 1]); F.poly(v.mat, [c10, c00, r0, r1], [0, 1, -1]); }
      F.color(v.wallTint);
      if (alongZ) { F.poly(v.wall, [c00, c10, r0], [0, 0, -1]); F.poly(v.wall, [c11, c01, r1], [0, 0, 1]); }
      else { F.poly(v.wall, [c00, c01, r0], [-1, 0, 0]); F.poly(v.wall, [c11, c10, r1], [1, 0, 0]); }
    }
  }
  return F.parts;
}

// ------------------------------------------------------------------ glTF writing
const r4 = (v) => Math.round(v * 1e4) / 1e4;
// Welds identical vertices; the index order keeps the triangle order, so piece runs stay valid.
function weld(a) {
  const map = new Map(), P = [], N = [], U = [], C = [], I = [];
  for (let v = 0; v < a.p.length / 3; v++) {
    const key = [a.p[v * 3], a.p[v * 3 + 1], a.p[v * 3 + 2], a.n[v * 3], a.n[v * 3 + 1], a.n[v * 3 + 2], a.u[v * 2], a.u[v * 2 + 1], a.c[v * 3], a.c[v * 3 + 1], a.c[v * 3 + 2]].map(r4).join(',');
    let idx = map.get(key);
    if (idx === undefined) {
      idx = P.length / 3; map.set(key, idx);
      P.push(a.p[v * 3], a.p[v * 3 + 1], a.p[v * 3 + 2]); N.push(a.n[v * 3], a.n[v * 3 + 1], a.n[v * 3 + 2]);
      U.push(a.u[v * 2], a.u[v * 2 + 1]); C.push(a.c[v * 3], a.c[v * 3 + 1], a.c[v * 3 + 2]);
    }
    I.push(idx);
  }
  return { P, N, U, C, I };
}
function writeGlb(file, lods, meta) {
  const json = { asset: { version: '2.0', generator: 'Crab Breeze tools/buildings/export_small_buildings.cjs', extras: meta }, scene: 0, scenes: [{ name: meta.id, nodes: [] }], nodes: [], meshes: [], materials: [], accessors: [], bufferViews: [], buffers: [] };
  const chunks = []; let offset = 0;
  const matIndex = new Map();
  const mat = (key) => { if (!matIndex.has(key)) { matIndex.set(key, json.materials.length); json.materials.push(materialDef(key)); } return matIndex.get(key); };
  const view = (typed, target) => {
    const buf = Buffer.from(typed.buffer, typed.byteOffset, typed.byteLength);
    const pad = (4 - (buf.length % 4)) % 4;
    json.bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: buf.length, target });
    chunks.push(buf, Buffer.alloc(pad)); offset += buf.length + pad;
    return json.bufferViews.length - 1;
  };
  const acc = (typed, type, comp, count, target, minmax) => {
    const a = { bufferView: view(typed, target), componentType: comp, count, type };
    if (minmax) { a.min = minmax[0]; a.max = minmax[1]; }
    json.accessors.push(a); return json.accessors.length - 1;
  };
  const tris = {};
  const primsOf = {};
  for (const [name, parts, node] of lods) {
    const prims = [];
    tris[name] = 0;
    for (const key of Object.keys(parts).sort()) {
      const w = weld(parts[key]);
      if (!w.I.length) continue;
      const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
      for (let i = 0; i < w.P.length; i += 3) for (let k = 0; k < 3; k++) { lo[k] = Math.min(lo[k], w.P[i + k]); hi[k] = Math.max(hi[k], w.P[i + k]); }
      const n = w.P.length / 3;
      const idx = n > 65535 ? new Uint32Array(w.I) : new Uint16Array(w.I);
      prims.push({
        attributes: {
          POSITION: acc(new Float32Array(w.P), 'VEC3', 5126, n, 34962, [lo.map(Math.fround), hi.map(Math.fround)]),
          NORMAL: acc(new Float32Array(w.N), 'VEC3', 5126, n, 34962),
          TEXCOORD_0: acc(new Float32Array(w.U), 'VEC2', 5126, n, 34962),
          COLOR_0: acc(new Float32Array(w.C), 'VEC3', 5126, n, 34962),
        },
        indices: acc(idx, 'SCALAR', n > 65535 ? 5125 : 5123, w.I.length, 34963),
        material: mat(key),
        extras: { surface: key, triangles: w.I.length / 3 },
      });
      tris[name] += w.I.length / 3;
    }
    primsOf[name] = prims.map((p) => p.extras.surface);
    if (!prims.length) continue; // a type with no block furniture has no FURN node
    json.meshes.push({ name: `${meta.id}_${name}`, primitives: prims });
    json.nodes.push(Object.assign({ name, mesh: json.meshes.length - 1, extras: { lod: name, within_m: LOD_M[name] } }, node || {}));
    json.scenes[0].nodes.push(json.nodes.length - 1);
  }
  const bin = Buffer.concat(chunks);
  json.buffers.push({ byteLength: bin.length });
  let js = Buffer.from(JSON.stringify(json), 'utf8');
  js = Buffer.concat([js, Buffer.alloc((4 - (js.length % 4)) % 4, 0x20)]);
  const header = Buffer.alloc(12); header.writeUInt32LE(0x46546c67, 0); header.writeUInt32LE(2, 4); header.writeUInt32LE(12 + 8 + js.length + 8 + bin.length, 8);
  const jh = Buffer.alloc(8); jh.writeUInt32LE(js.length, 0); jh.writeUInt32LE(0x4e4f534a, 4);
  const bh = Buffer.alloc(8); bh.writeUInt32LE(bin.length, 0); bh.writeUInt32LE(0x004e4942, 4);
  fs.writeFileSync(file, Buffer.concat([header, jh, js, bh, bin]));
  return { tris, prims: primsOf };
}

// ------------------------------------------------------------------ export
fs.mkdirSync(OUT, { recursive: true });
const signs = (txt, bg, fg, neon) => `sign:${txt}|${bg}|${fg}|${neon ? 1 : 0}`;
const M = { sign: signs, awning: (c) => `awning:${c}` };
const manifest = { generator: 'tools/buildings/export_small_buildings.cjs', kit: 'docs/mockups/small_buildings_kit.js', seed: SEED, units: 'metres; front faces +z, centred on x = 0, standing on y = 0', lod_within_m: LOD_M, lod1_drops: { interior: true, pieces_under_m3: SMALL_M3 }, budget_triangles: BUDGET, types: [] };
let failed = false;
const r3 = (v) => Math.round(v * 1000) / 1000;
// Every type, and each suburban house's variants (design section 7.1).
const BUILDS = K.TYPES.flatMap((T) => [0, ...(T.variants || []).map((_, i) => i + 1)].map((v) => ({ T, v, id: K.variantId(T.id, v) })));
for (const { T, v, id } of BUILDS) {
  const B = K.build(T.id, SEED, M, v);
  const pieces = K.piecesOf(B);
  const live = pieces.filter((p) => p.runs.length);
  const links = K.supportLinks(live);
  const interior = new Set(live.filter((p) => K.isInterior(p)).map((p) => p.id));
  // Block furniture is its own node, drawn within 100 m (design section 7.3); each door's leaf is its own node, in
  // its hinge's frame, so the game can swing it (7.2). LOD0 is the rest.
  const furn = new Set(live.filter((p) => p.tag === 'furniture').map((p) => p.id));
  const leaves = live.filter((p) => p.tag === 'leaf' && p.meta && p.meta.leaf);
  const leafIds = new Set(leaves.map((p) => p.id));
  const lod0 = filterBuckets(B.parts, (pid) => !furn.has(pid) && !leafIds.has(pid));
  const furnParts = filterBuckets(B.parts, (pid) => furn.has(pid));
  const leafParts = leaves.map((p) => {
    const parts = filterBuckets(B.parts, (pid) => pid === p.id);
    const h = p.meta.leaf.hinge;
    for (const a of Object.values(parts)) for (let i = 0; i < a.p.length; i += 3) { a.p[i] -= h[0]; a.p[i + 1] -= h[1]; a.p[i + 2] -= h[2]; }
    return parts;
  });
  // LOD1 (150-600 m): no interior, and no piece smaller than 4 litres (balusters, knobs, slats).
  const small = new Set(live.filter((p) => (p.hi[0] - p.lo[0]) * (p.hi[1] - p.lo[1]) * (p.hi[2] - p.lo[2]) < SMALL_M3).map((p) => p.id));
  const lod1 = filterBuckets(B.parts, (id) => !interior.has(id) && !small.has(id), { glass: 'glassFar', glassShop: 'glassFar' });
  const lod2 = farShapes(B, pieces);
  const file = path.join(OUT, `${id}.glb`);
  const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
  for (const p of live) for (let k = 0; k < 3; k++) { lo[k] = Math.min(lo[k], p.lo[k]); hi[k] = Math.max(hi[k], p.hi[k]); }
  const res = writeGlb(file, [['LOD0', lod0], ['LOD1', lod1], ['LOD2', lod2], ['FURN', furnParts], ...leafParts.map((parts, i) => [`LEAF${i}`, parts, { translation: leaves[i].meta.leaf.hinge, extras: { leaf: i, within_m: LOD_M.LOD0 } }])], { id, seed: SEED, variant: v });
  // The destruction table: LOD0 pieces with their triangle runs in LOD0's primitives (FURN's for furniture, the
  // leaf's own node for a leaf).
  const primOf = new Map(res.prims.LOD0.map((k, i) => [k, i]));
  const furnPrimOf = new Map(res.prims.FURN.map((k, i) => [k, i]));
  // Runs of a node re-counted after filtering: a piece's first triangle in its node's primitive.
  const runsIn = (parts, keep) => {
    const out = new Map();
    for (const key of Object.keys(parts).sort()) {
      const t = parts[key].t;
      for (let i = 0; i < t.length;) { let k = 1; while (i + k < t.length && t[i + k] === t[i]) k++; if (keep(t[i])) { const r = out.get(t[i]) || []; r.push([key, i, k]); out.set(t[i], r); } i += k; }
    }
    return out;
  };
  const lod0Runs = runsIn(lod0, () => true), furnRuns = runsIn(furnParts, () => true);
  const leafIndex = new Map(leaves.map((p, i) => [p.id, i]));
  const wallMats = new Set(K.WALLMATS);
  const table = {
    id, lod: 'LOD0', rules: 'openspec/changes/small-buildings/design.md section 6; docs/design/small-buildings-kit.md',
    max_side_steps: K.MAX_SIDE, collapse_wall_share: 0.5,
    primitives: res.prims.LOD0, furn_primitives: res.prims.FURN,
    leaves: leaves.map((p) => { const L = p.meta.leaf; return { piece: p.id, hinge: L.hinge.map(r3), shut: L.shut, inward: L.inward, width_m: r3(L.width_m), height_m: r3(L.height_m) }; }),
    furniture: live.filter((p) => p.meta && p.meta.furn).map((p) => { const F = p.meta.furn; return { piece: p.id, item: F.item, at: F.at.map(r3), rot: F.rot, tint: F.tint.map(r3) }; }),
    pieces: live.map((p, i) => {
      const own = leafIndex.has(p.id) ? [] : furn.has(p.id) ? (furnRuns.get(p.id) || []).map(([k, s, n]) => [furnPrimOf.get(k), s, n]) : (lod0Runs.get(p.id) || []).map(([k, s, n]) => [primOf.get(k), s, n]);
      const e = { id: p.id, lo: p.lo.map(r3), hi: p.hi.map(r3), runs: own };
      if (furn.has(p.id)) e.node = 'FURN';
      if (leafIndex.has(p.id)) { e.node = `LEAF${leafIndex.get(p.id)}`; e.leaf = leafIndex.get(p.id); }
      if (p.tag) e.tag = p.tag;
      // The boxes it is made of, when it is only boxes (for colliders); otherwise its bounds stand in.
      const bx = B.boxes[p.id];
      if (bx && bx.length > 1 && !B.nonBox[p.id]) e.boxes = bx.map((b) => b.map(r3));
      if (interior.has(p.id)) e.interior = true;
      if (p.meta && p.meta.box) e.box = { mat: p.meta.mat, tint: p.meta.tint.map(r3) };
      if (p.meta && p.meta.panel) e.panel = { mat: p.meta.mat, tint: p.meta.tint.map(r3), c: p.meta.panel.c.map(r3), size: p.meta.panel.size.map(r3), axes: p.meta.panel.axes.map((a) => a.map(r3)) };
      if (p.meta && p.meta.box && wallMats.has(p.meta.mat) && p.lo[1] < 1.2 && p.hi[1] - p.lo[1] > 1.5) e.wall_area_m2 = r3(Math.max(p.hi[0] - p.lo[0], p.hi[2] - p.lo[2]) * (p.hi[1] - p.lo[1]));
      e.up = links.up[i].map((j) => live[j].id);
      e.side = links.side[i].map((j) => live[j].id);
      return e;
    }),
  };
  fs.writeFileSync(path.join(OUT, `${id}.pieces.json`), JSON.stringify(table));
  // LOD0's budget counts what is drawn within 150 m: the shell, its leaves and its block furniture.
  const drawn0 = res.tris.LOD0 + res.tris.FURN + Object.entries(res.tris).filter(([k]) => k.startsWith('LEAF')).reduce((s, [, n]) => s + n, 0);
  const counted = { LOD0: drawn0, LOD1: res.tris.LOD1, LOD2: res.tris.LOD2 };
  const over = Object.entries(counted).filter(([l, n]) => n > budgetOf(T.id, l));
  if (over.length) { failed = true; console.error(`OVER BUDGET ${id}: ${over.map(([l, n]) => `${l} ${n} > ${budgetOf(T.id, l)}`).join(', ')}`); }
  manifest.types.push({
    id, name: v ? T.variants[v - 1] : T.name, kind: T.kind, ...(v ? { variant_of: T.id, variant: v } : {}), biomes: T.biome.split(',').map((b) => b.trim()), blurb: T.blurb,
    glb: `${id}.glb`, pieces: `${id}.pieces.json`, access: B.access && (B.access.drive ? { drive: B.access.drive.map(r3) } : { lot: r3(B.access.lot), stalls: B.access.stalls !== false }),
    bounds_m: { lo: lo.map(r3), hi: hi.map(r3) }, triangles: { LOD0: res.tris.LOD0, LOD1: res.tris.LOD1, LOD2: res.tris.LOD2, FURN: res.tris.FURN, leaves: drawn0 - res.tris.LOD0 - res.tris.FURN }, budget_triangles: { LOD0: budgetOf(T.id, 'LOD0'), LOD1: budgetOf(T.id, 'LOD1'), LOD2: budgetOf(T.id, 'LOD2'), ...(BUDGET_FOR[T.id] ? { why: BUDGET_FOR[T.id].why } : {}) }, piece_count: live.length,
    surfaces: res.prims.LOD0, signs: res.prims.LOD0.filter((k) => k.startsWith('sign:')).map((k) => k.slice(5).split('|')[0]),
  });
  console.log(`${id.padEnd(11)} LOD0 ${String(res.tris.LOD0).padStart(6)}  LOD1 ${String(res.tris.LOD1).padStart(5)}  LOD2 ${String(res.tris.LOD2).padStart(4)}  pieces ${live.length}  ${(fs.statSync(file).size / 1024).toFixed(0)} KB`);
}
fs.writeFileSync(path.join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 1) + '\n');
if (failed) process.exit(1);
