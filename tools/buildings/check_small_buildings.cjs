// Checks the exported kits against each other (CLAUDE.md section 6.5: validate the real artifact):
// every glb parses, has LOD0, LOD1 and LOD2, its triangle counts match the manifest and are within
// budget, every index is in range, and the piece table's runs cover LOD0's triangles exactly once
// with support links that name real pieces. With three installed it also loads each glb through
// three.js's GLTFLoader as an independent reader.
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
  for (const [l, n] of Object.entries(tris)) {
    if (n !== T.triangles[l]) fail(T.id, `${l} has ${n} triangles, manifest says ${T.triangles[l]}`);
    if (n > T.budget_triangles[l]) fail(T.id, `${l} ${n} over budget ${T.budget_triangles[l]}`);
  }
  // The piece table against LOD0.
  const table = JSON.parse(fs.readFileSync(path.join(DIR, T.pieces), 'utf8'));
  const lod0 = g.json.meshes[g.json.nodes.find((n) => n.name === 'LOD0').mesh];
  const prims = lod0.primitives.map((p) => p.extras.surface);
  if (JSON.stringify(prims) !== JSON.stringify(table.primitives)) fail(T.id, 'piece table primitives differ from LOD0');
  const covered = lod0.primitives.map((p) => new Uint8Array(g.json.accessors[p.indices].count / 3));
  const ids = new Set(table.pieces.map((p) => p.id));
  for (const p of table.pieces) {
    for (const [pi, start, count] of p.runs) {
      if (!covered[pi] || start + count > covered[pi].length) { fail(T.id, `piece ${p.id} run out of range`); continue; }
      for (let t = start; t < start + count; t++) covered[pi][t]++;
    }
    for (const q of [...p.up, ...p.side]) if (!ids.has(q)) fail(T.id, `piece ${p.id} links to missing ${q}`);
  }
  covered.forEach((c, pi) => { for (const v of c) if (v !== 1) { fail(T.id, `primitive ${prims[pi]}: a triangle is in ${v} pieces`); break; } });
  console.log(`${T.id.padEnd(9)} ok  LOD0 ${tris.LOD0}  LOD1 ${tris.LOD1}  LOD2 ${tris.LOD2}  pieces ${table.pieces.length}`);
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
