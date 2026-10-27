// Checks the exported kits against each other (CLAUDE.md section 6.5: validate the real artifact):
// every glb parses, has LOD0, LOD1 and LOD2, its triangle counts match the manifest and are within
// budget, every index is in range, and the piece table's runs cover LOD0's triangles exactly once
// with support links that name real pieces, and no two faces z-fight (CLAUDE.md section 10: same-facing
// faces within 1.5 mm of one plane may not overlap by more than 1 cm2). With three installed it also
// loads each glb through three.js's GLTFLoader as an independent reader.
//
//   node tools/buildings/check_small_buildings.cjs [assets/models/small_buildings] [path/to/node_modules/three]
'use strict';
const fs = require('fs');
const path = require('path');
const DIR = process.argv[2] || 'assets/models/small_buildings';
const THREE_DIR = process.argv[3];
const manifest = JSON.parse(fs.readFileSync(path.join(DIR, 'manifest.json'), 'utf8'));
let errors = 0;
const fail = (id, msg) => { errors++; console.error(`FAIL ${id}: ${msg}`); };

function readGlb(file) {
  const b = fs.readFileSync(file);
  if (b.readUInt32LE(0) !== 0x46546c67 || b.readUInt32LE(4) !== 2) throw new Error('not a glTF 2 binary');
  const jl = b.readUInt32LE(12);
  const json = JSON.parse(b.slice(20, 20 + jl).toString('utf8'));
  const bl = b.readUInt32LE(20 + jl);
  const bin = b.slice(28 + jl, 28 + jl + bl);
  const read = (ai) => {
    const a = json.accessors[ai], v = json.bufferViews[a.bufferView];
    const comps = { SCALAR: 1, VEC2: 2, VEC3: 3 }[a.type];
    const Ctor = { 5126: Float32Array, 5125: Uint32Array, 5123: Uint16Array }[a.componentType];
    return new Ctor(bin.buffer.slice(bin.byteOffset + v.byteOffset, bin.byteOffset + v.byteOffset + a.count * comps * Ctor.BYTES_PER_ELEMENT));
  };
  return { json, read, bytes: b };
}

// No z-fighting: same-facing triangles of `nodes` (each moved by its translation) that lie within
// ZF_PLANE_M of one plane and overlap by more than ZF_AREA_M2. Faces nobody sees are skipped: those
// wholly below the ground, and down-facing ones at the ground. So are overlaps that draw the same pixels
// either way: the same surface, colour and texture coordinates across the overlap; and overlaps pressed
// against another solid's face (a wall's foot on its foundation), which nobody sees.
const ZF_PLANE_M = 0.0015, ZF_AREA_M2 = 1e-4;
function zfight(T, g, nodes) {
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const area = (P) => { let s = 0; for (let i = 0; i < P.length; i++) { const p = P[i], q = P[(i + 1) % P.length]; s += p[0] * q[1] - q[0] * p[1]; } return s / 2; };
  const clip = (poly, a, b) => {
    const out = [], side = (p) => (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    for (let i = 0; i < poly.length; i++) {
      const p = poly[i], q = poly[(i + 1) % poly.length], sp = side(p), sq = side(q);
      if (sp >= 0) out.push(p);
      if ((sp >= 0) !== (sq >= 0)) { const t = sp / (sp - sq); out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]); }
    }
    return out;
  };
  const tris = [];
  for (const name of nodes) {
    const n = g.json.nodes.find((x) => x.name === name);
    if (!n) continue;
    const at = n.translation || [0, 0, 0];
    for (const p of g.json.meshes[n.mesh].primitives) {
      const pos = g.read(p.attributes.POSITION), idx = g.read(p.indices), uv = g.read(p.attributes.TEXCOORD_0);
      const col = g.read(p.attributes.COLOR_0), cn = g.json.accessors[p.attributes.COLOR_0].type === 'VEC4' ? 4 : 3;
      for (let i = 0; i < idx.length; i += 3) {
        const v = [0, 1, 2].map((k) => [0, 1, 2].map((c) => pos[idx[i + k] * 3 + c] + at[c]));
        const nn = cross(sub(v[1], v[0]), sub(v[2], v[0])), l = Math.hypot(...nn);
        if (l < 1e-9) continue;
        const N = nn.map((c) => c / l), ys = v.map((q) => q[1]);
        if (Math.max(...ys) < 0 || (N[1] < -0.99 && Math.max(...ys) <= 0.01)) continue;
        const ix = [idx[i], idx[i + 1], idx[i + 2]];
        const look = ix.map((k) => [uv[k * 2], uv[k * 2 + 1], col[k * cn], col[k * cn + 1], col[k * cn + 2]]);
        tris.push({ v, N, d: dot(N, v[0]), surf: `${name}:${p.extras && p.extras.surface}`, look });
      }
    }
  }
  const groups = new Map();
  for (const t of tris) { const k = t.N.map((c) => Math.round(c * 200)).join(','); if (!groups.has(k)) groups.set(k, []); groups.get(k).push(t); }
  const found = new Map();
  for (const G of groups.values()) {
    G.sort((a, b) => a.d - b.d);
    for (let i = 0; i < G.length; i++) for (let j = i + 1; j < G.length && G[j].d - G[i].d <= ZF_PLANE_M; j++) {
      const A = G[i], B = G[j], ax = A.N.map(Math.abs), drop = ax.indexOf(Math.max(...ax)), keep = [0, 1, 2].filter((k) => k !== drop);
      const flat = (t) => { const P = t.v.map((q) => [q[keep[0]], q[keep[1]]]); return area(P) < 0 ? [P[0], P[2], P[1]] : P; };
      let poly = flat(A); const b = flat(B);
      for (let k = 0; k < 3 && poly.length; k++) poly = clip(poly, b[k], b[(k + 1) % 3]);
      const ov = poly.length >= 3 ? Math.abs(area(poly)) : 0;
      if (ov <= ZF_AREA_M2) continue;
      // What a triangle draws at a point of the plane: its texture coordinates and colour, interpolated.
      const lookAt = (t, q) => {
        const P = t.v.map((r) => [r[keep[0]], r[keep[1]]]), a = area(P);
        const w = [area([q, P[1], P[2]]) / a, area([P[0], q, P[2]]) / a, area([P[0], P[1], q]) / a];
        return [0, 1, 2, 3, 4].map((c) => w[0] * t.look[0][c] + w[1] * t.look[1][c] + w[2] * t.look[2][c]);
      };
      const mid = [poly.reduce((s2, q) => s2 + q[0], 0) / poly.length, poly.reduce((s2, q) => s2 + q[1], 0) / poly.length];
      const same = A.surf.split(':')[1] === B.surf.split(':')[1] && [mid, ...poly.map((q) => [(q[0] + mid[0]) / 2, (q[1] + mid[1]) / 2])].every((q) => {
        const x = lookAt(A, q), y = lookAt(B, q);
        return Math.abs(x[0] - y[0]) < 1e-3 && Math.abs(x[1] - y[1]) < 1e-3 && [2, 3, 4].every((c) => Math.abs(x[c] - y[c]) < 0.004);
      });
      if (same) continue;
      const opp = groups.get(A.N.map((c) => Math.round(-c * 200)).join(',')) || [];
      const inside = (q, t) => { const P = flat(t); return [0, 1, 2].every((k) => (P[(k + 1) % 3][0] - P[k][0]) * (q[1] - P[k][1]) - (P[(k + 1) % 3][1] - P[k][1]) * (q[0] - P[k][0]) >= -1e-6); };
      const pressed = [mid, ...poly.map((q) => [(q[0] + mid[0]) / 2, (q[1] + mid[1]) / 2])].every((q) =>
        opp.some((C) => Math.abs(C.d + A.d) <= ZF_PLANE_M && inside(q, C)));
      if (pressed) continue;
      const key = [A.surf, B.surf].sort().join(' and ');
      const e = found.get(key) || { n: 0, ov: 0, at: A.v[0], N: A.N };
      e.n++; e.ov += ov; found.set(key, e);
    }
  }
  for (const [k, e] of found) fail(T.id, `z-fighting: ${k}, ${e.n} overlaps, ${(e.ov * 1e4).toFixed(0)} cm2, near ${e.at.map((c) => c.toFixed(2)).join(', ')} facing ${e.N.map((c) => c.toFixed(1)).join(', ')}`);
}

for (const T of manifest.types) {
  const g = readGlb(path.join(DIR, T.glb));
  const names = g.json.nodes.map((n) => n.name);
  for (const l of ['LOD0', 'LOD1', 'LOD2']) if (!names.includes(l)) fail(T.id, `no ${l} node`);
  const tris = {};
  g.json.nodes.forEach((n) => {
    const mesh = g.json.meshes[n.mesh];
    tris[n.name] = 0;
    for (const p of mesh.primitives) {
      const idx = g.read(p.indices), count = g.json.accessors[p.attributes.POSITION].count;
      for (const k of ['NORMAL', 'TEXCOORD_0', 'COLOR_0']) if (g.json.accessors[p.attributes[k]].count !== count) fail(T.id, `${n.name} ${k} count`);
      for (const i of idx) if (i >= count) { fail(T.id, `${n.name} index out of range`); break; }
      const pos = g.read(p.attributes.POSITION);
      for (const v of pos) if (!Number.isFinite(v)) { fail(T.id, `${n.name} non-finite position`); break; }
      tris[n.name] += idx.length / 3;
    }
  });
  // Leaves are summed; LOD0's budget counts what is drawn within 150 m (the shell, its leaves, its block furniture).
  const leafTris = Object.entries(tris).filter(([k]) => k.startsWith('LEAF')).reduce((s, [, n]) => s + n, 0);
  const got = { LOD0: tris.LOD0, LOD1: tris.LOD1, LOD2: tris.LOD2, FURN: tris.FURN || 0, leaves: leafTris };
  for (const [l, n] of Object.entries(got)) if (n !== T.triangles[l]) fail(T.id, `${l} has ${n} triangles, manifest says ${T.triangles[l]}`);
  const drawn = { LOD0: got.LOD0 + got.FURN + leafTris, LOD1: got.LOD1, LOD2: got.LOD2 };
  for (const [l, n] of Object.entries(drawn)) if (n > T.budget_triangles[l]) fail(T.id, `${l} ${n} over budget ${T.budget_triangles[l]}`);
  // The piece table against LOD0.
  const table = JSON.parse(fs.readFileSync(path.join(DIR, T.pieces), 'utf8'));
  // Every node a piece's runs can point into: LOD0, FURN and each LEAF. Runs must cover them exactly once.
  const meshOf = (name) => { const n = g.json.nodes.find((x) => x.name === name); return n ? g.json.meshes[n.mesh] : null; };
  const prims = meshOf('LOD0').primitives.map((p) => p.extras.surface);
  if (JSON.stringify(prims) !== JSON.stringify(table.primitives)) fail(T.id, 'piece table primitives differ from LOD0');
  const furnMesh = meshOf('FURN');
  if (JSON.stringify(furnMesh ? furnMesh.primitives.map((p) => p.extras.surface) : []) !== JSON.stringify(table.furn_primitives)) fail(T.id, 'furn_primitives differ from FURN');
  const covered = {};
  for (const n of g.json.nodes) if (n.name === 'LOD0' || n.name === 'FURN' || n.name.startsWith('LEAF')) covered[n.name] = g.json.meshes[n.mesh].primitives.map((p) => new Uint8Array(g.json.accessors[p.indices].count / 3));
  const ids = new Set(table.pieces.map((p) => p.id));
  for (const p of table.pieces) {
    const node = p.node || 'LOD0';
    if (p.leaf != null) { for (const c of covered[node] || []) c.fill(1); if (!covered[node]) fail(T.id, `piece ${p.id}: no ${node}`); }
    for (const [pi, start, count] of p.runs) {
      const c = (covered[node] || [])[pi];
      if (!c || start + count > c.length) { fail(T.id, `piece ${p.id} run out of range in ${node}`); continue; }
      for (let t = start; t < start + count; t++) c[t]++;
    }
    for (const q of [...p.up, ...p.side]) if (!ids.has(q)) fail(T.id, `piece ${p.id} links to missing ${q}`);
  }
  for (const [node, list] of Object.entries(covered)) list.forEach((c, pi) => { for (const v of c) if (v !== 1) { fail(T.id, `${node} primitive ${pi}: a triangle is in ${v} pieces`); break; } });
  for (const L of table.leaves) if (!ids.has(L.piece) || !(L.width_m > 0.3) || !(L.height_m > 1.5)) fail(T.id, `leaf of piece ${L.piece} is not a door`);
  for (const F of table.furniture) if (!ids.has(F.piece) || !F.item) fail(T.id, `furniture ${F.item} of piece ${F.piece}`);
  zfight(T, g, ['LOD0', 'FURN', ...names.filter((n) => n.startsWith('LEAF'))]);
  zfight(T, g, ['LOD1']);
  console.log(`${T.id.padEnd(11)} ok  LOD0 ${tris.LOD0}  LOD1 ${tris.LOD1}  LOD2 ${tris.LOD2}  pieces ${table.pieces.length}`);
}

// An independent reader: three.js's GLTFLoader.
if (THREE_DIR) {
  const THREE = require(path.join(THREE_DIR, 'build/three.cjs'));
  global.THREE = THREE;
  global.self = global; global.window = global;
  // eslint-disable-next-line no-new-func
  new Function('THREE', fs.readFileSync(path.join(THREE_DIR, 'examples/js/loaders/GLTFLoader.js'), 'utf8'))(THREE);
  const loader = new THREE.GLTFLoader();
  let pending = manifest.types.length;
  for (const T of manifest.types) {
    const b = fs.readFileSync(path.join(DIR, T.glb));
    loader.parse(b.buffer.slice(b.byteOffset, b.byteOffset + b.length), '', (gltf) => {
      let tris = 0;
      gltf.scene.getObjectByName('LOD0').traverse((o) => { if (o.isMesh) tris += o.geometry.index.count / 3; });
      if (tris !== T.triangles.LOD0) fail(T.id, `three.js reads ${tris} LOD0 triangles`);
      if (--pending === 0) { console.log(errors ? `${errors} errors` : 'three.js GLTFLoader reads every kit'); process.exit(errors ? 1 : 0); }
    }, (e) => { fail(T.id, `three.js cannot load: ${e.message || e}`); if (--pending === 0) process.exit(1); });
  }
} else {
  console.log(errors ? `${errors} errors` : 'all kits check');
  process.exit(errors ? 1 : 0);
}
