// Exports the approved airport and seaport (openspec changes airport-layout and seaport-layout) as game kits: every
// building, crane, stack, jetty, pontoon, light and ship becomes a breakable kit through the one kit writer
// (kit_export.cjs, as the houses are), and the paving, paint, quays and breakwaters a static ground glb.
//
// The layout is the mockup page's own (docs/mockups/airport.html, seaport.html, generated from tools/mockups/*_src.html):
// this script runs the page's script in node with its DOM, renderer and controls stubbed, takes each building from the
// page's `put` (window.SITE_EXPORT) and the ground from the group the page builds, so the game and the approved page
// cannot disagree (CLAUDE.md section 6.1). Writes, for each site, assets/models/sites/<site>/:
//   <kit>.glb, <kit>.pieces.json   one per distinct building (identical ones share a kit)
//   ground.glb                     the site's paving, paint, quays and breakwaters, one primitive per surface
//   site.json                      every placement (kit, x, y, z, quarter turns, base, rules) and the ground's solids
//
//   node tools/buildings/export_sites.cjs [--only airport|seaport] [--out assets/models/sites]
//
// Needs three.js r147 (the pages' version): from THREE_JS if set, else fetched once into target/three-0.147.0.min.js.
'use strict';
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const crypto = require('crypto');
const { exportKit, writeGlb, r3 } = require('./kit_export.cjs');

const ROOT = path.join(__dirname, '..', '..');
const args = process.argv.slice(2);
const opt = (name, def) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : def; };
const OUT = opt('--out', path.join(ROOT, 'assets/models/sites'));
const ONLY = opt('--only', null);

// Triangle budgets per kit, LOD0 as drawn within 150 m with its furniture (small-buildings design section 4). A site's
// halls are many times a house's floor area, so their budgets scale with it; each override says why. Measured from the
// pages (airport-layout section 6, seaport-layout section 2): the largest kit, Terminal 2, carries a whole hall of gates.
const BUDGET = { LOD0: 120000, LOD1: 40000, LOD2: 400 };
const BUDGET_FOR = {
  terminal_2: { LOD0: 460000, LOD1: 90000, why: 'a 280 x 65 m hall over two floors, every check-in island, gate lounge, shop and the food court furnished (furniture drawn within 100 m)' },
  terminal_1: { LOD0: 400000, LOD1: 90000, why: 'a 320 x 40 m terminal over two floors, furnished as Terminal 2' },
  cargo_terminal: { LOD0: 180000, why: 'a 180 x 65 m shed racked full of pallets (furniture drawn within 100 m)' },
  transit_shed: { LOD0: 150000, why: 'a 240 m shed racked full of pallets, with forklifts (furniture drawn within 100 m)' },
  multi_storey_car_park: { LOD1: 60000, why: 'four decks of parked cars: every car is shell, so the far level keeps them' },
};
const budgetOf = (id, lod) => (BUDGET_FOR[id] && BUDGET_FOR[id][lod]) || BUDGET[lod];

// ---------------------------------------------------------------- three.js in node
function threeSource() {
  if (process.env.THREE_JS) return fs.readFileSync(process.env.THREE_JS, 'utf8');
  const cache = path.join(ROOT, 'target', 'three-0.147.0.min.js');
  if (!fs.existsSync(cache)) {
    const { execFileSync } = require('child_process');
    fs.mkdirSync(path.dirname(cache), { recursive: true });
    execFileSync('curl', ['-sSfL', '-o', cache, 'https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js']);
  }
  return fs.readFileSync(cache, 'utf8');
}

// A stand-in for everything a page touches that is not geometry: the DOM, the renderer, the controls, the loader.
// Any property is itself, any call returns itself, so `document.getElementById("x").style.display = ...` runs.
function stub() {
  const f = function () {};
  const p = new Proxy(f, {
    get: (t, k) => (k === Symbol.toPrimitive ? () => 0 : k === 'then' ? undefined : k === Symbol.iterator ? function* () {} : k === 'length' ? 0 : p),
    set: () => true, apply: () => p, construct: () => p, has: () => true,
  });
  return p;
}

// Runs a generated page's scripts; returns the buildings its `put` gave and the group of ground it built.
function runPage(file, exportFlags) {
  const html = fs.readFileSync(file, 'utf8');
  const scripts = [...html.matchAll(/<script(?:\s+src="([^"]*)")?>([\s\S]*?)<\/script>/g)];
  const S = stub();
  const sandbox = {
    console, Math, JSON, Date, Array, Object, Number, String, Map, Set, Symbol, Error, Promise, Proxy, Reflect, Uint8Array,
    Uint16Array, Uint32Array, Int8Array, Int16Array, Int32Array, Float32Array, Float64Array, Uint8ClampedArray, ArrayBuffer,
    DataView, WeakMap, WeakSet, parseInt, parseFloat, isFinite, isNaN, TextDecoder, TextEncoder, atob: (s) => Buffer.from(s, 'base64').toString('binary'),
    document: S, navigator: { userAgent: 'node' }, location: { hash: '', search: '' }, performance: { now: () => 0 },
    requestAnimationFrame: () => 0, cancelAnimationFrame: () => 0, setTimeout: () => 0, clearTimeout: () => 0,
    setInterval: () => 0, addEventListener: () => {}, devicePixelRatio: 1, innerWidth: 1280, innerHeight: 800,
    Image: function () { return S; }, OffscreenCanvas: function () { return S; }, ImageData: function () { return S; },
    fetch: () => new Promise(() => {}), createImageBitmap: undefined,
  };
  sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
  const taken = [];
  sandbox.SITE_EXPORT = Object.assign({ put: (name, B, at, info) => { taken.push({ name, B, at, info }); return taken.length - 1; } }, exportFlags);
  vm.createContext(sandbox);
  vm.runInContext(threeSource(), sandbox, { filename: 'three.min.js' });
  // The page's renderer, controls and loader are stubs; geometry is real three.js.
  sandbox.THREE.WebGLRenderer = function () { return S; };
  sandbox.THREE.OrbitControls = function () { return S; };
  sandbox.THREE.GLTFLoader = function () { return S; };
  const code = scripts.filter((m) => !m[1]).map((m) => m[2]).join('\n;\n') + '\n;globalThis.__BUILT__ = built; globalThis.__M__ = M;';
  vm.runInContext(code, sandbox, { filename: path.basename(file) });
  return { taken, built: sandbox.__BUILT__, M: sandbox.__M__, THREE: sandbox.THREE, pads: sandbox.SITE_EXPORT.pads || [], marks: sandbox.SITE_EXPORT.marks || {}, lights: sandbox.SITE_EXPORT.lights || { masts: [], dots: [], runways: [] } };
}

// ---------------------------------------------------------------- the ground: the page's group, by material
const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');
function groundOf(built, THREE, M) {
  const names = new Map(Object.entries(M).map(([k, m]) => [m, k]));
  const parts = {}; const solids = []; let unnamed = 0;
  const p = new THREE.Vector3(), n = new THREE.Vector3(), nm = new THREE.Matrix3();
  built.updateMatrixWorld(true);
  built.traverse((o) => {
    if (!o.isMesh) return;
    for (let a = o; a; a = a.parent) if (a.userData && a.userData.skip) return;
    const mats = Array.isArray(o.material) ? o.material : [o.material];
    let key = names.get(mats[0]);
    if (!key) { const c = mats[0].color; key = `colour_${c.getHexString()}`; unnamed++; }
    const m = mats[0];
    const def = { colour: [m.color.r, m.color.g, m.color.b], roughness: m.roughness, metallic: m.metalness || 0, emissive: m.emissive ? [m.emissive.r * (m.emissiveIntensity || 1), m.emissive.g * (m.emissiveIntensity || 1), m.emissive.b * (m.emissiveIntensity || 1)] : [0, 0, 0] };
    const out = parts[key] || (parts[key] = { def, p: [], n: [], u: [], c: [], t: [] });
    const g = o.geometry.index ? o.geometry.toNonIndexed() : o.geometry;
    const pos = g.attributes.position, nor = g.attributes.normal, uv = g.attributes.uv;
    const count = o.isInstancedMesh ? o.count : 1, im = new THREE.Matrix4(), world = new THREE.Matrix4();
    for (let i = 0; i < count; i++) {
      world.copy(o.matrixWorld);
      if (o.isInstancedMesh) { o.getMatrixAt(i, im); world.multiply(im); }
      nm.getNormalMatrix(world);
      for (let v = 0; v < pos.count; v++) {
        p.fromBufferAttribute(pos, v).applyMatrix4(world); out.p.push(p.x, p.y, p.z);
        if (nor) { n.fromBufferAttribute(nor, v).applyMatrix3(nm).normalize(); out.n.push(n.x, n.y, n.z); } else out.n.push(0, 1, 0);
        // UVs in metres on the ground plane, as the kit's are, so a surface's texture tiles at true size.
        out.u.push(p.x, p.z); out.c.push(1, 1, 1);
      }
      for (let t = 0; t < pos.count / 3; t++) out.t.push(0);
      // A box standing out of the ground (a quay, a plinth) is a solid the game walks on and collides with.
      if (o.geometry.type === 'BoxGeometry' && !o.isInstancedMesh) {
        const bb = new THREE.Box3().setFromObject(o);
        if (bb.max.y - bb.min.y > 0.6) solids.push({ material: key, lo: bb.min.toArray().map(r3), hi: bb.max.toArray().map(r3) });
      }
    }
  });
  return { parts, solids, unnamed };
}

// Lifts everything a Builder holds by dy: its triangles, its boxes and every point its pieces' meta records, so a kit
// stands on y = 0 at its ground (the page draws a building from `at.y`, its ground at `info.base`).
function shiftY(B, dy) {
  if (!dy) return;
  for (const a of Object.values(B.parts)) for (let i = 1; i < a.p.length; i += 3) a.p[i] += dy;
  for (const bs of B.boxes) if (bs) for (const b of bs) { b[1] += dy; b[4] += dy; }
  for (const m of B.meta) {
    if (!m) continue;
    if (m.box) { m.box[1] += dy; m.box[4] += dy; }
    if (m.panel) m.panel.c[1] += dy;
    if (m.leaf) m.leaf.hinge[1] += dy;
    if (m.furn) m.furn.at[1] += dy;
  }
  for (const v of B.volumes) { v.top += dy; if (v.ridgeY != null) v.ridgeY += dy; }
}

// ---------------------------------------------------------------- one site
function exportSite(site, page, flags) {
  const dir = path.join(OUT, site);
  fs.mkdirSync(dir, { recursive: true });
  for (const f of fs.readdirSync(dir)) fs.unlinkSync(path.join(dir, f));
  const t0 = Date.now();
  const { taken, built, M, THREE, pads, marks, lights } = runPage(page, flags);
  // Identical buildings share one kit: the same name and the same geometry.
  const kits = new Map(); const placements = []; const named = new Map(); let failed = false;
  const manifest = [];
  for (const { name, B, at, info } of taken) {
    shiftY(B, at.y - info.base);
    const geo = crypto.createHash('sha1');
    for (const k of Object.keys(B.parts).sort()) { const a = B.parts[k]; geo.update(k); geo.update(Buffer.from(new Float32Array(a.p).buffer)); geo.update(Buffer.from(new Float32Array(a.c).buffer)); }
    const hash = geo.digest('hex');
    let id = kits.get(hash);
    if (!id) {
      const base = slug(name); const k = (named.get(base) || 0) + 1; named.set(base, k);
      id = k === 1 ? base : `${base}_${k}`;
      kits.set(hash, id);
      const x = exportKit(B, id, dir, { maxSide: info.maxSide, meta: { site, name } });
      const over = Object.entries(x.counted).filter(([l, n]) => n > budgetOf(id, l));
      if (over.length) { failed = true; console.error(`OVER BUDGET ${site}/${id}: ${over.map(([l, n]) => `${l} ${n} > ${budgetOf(id, l)}`).join(', ')}`); }
      manifest.push({
        id, name, kind: info.sink ? 'ship' : info.maxSide > 3 ? 'machine' : 'building', glb: `${id}.glb`, pieces: `${id}.pieces.json`,
        bounds_m: { lo: x.lo, hi: x.hi }, triangles: x.triangles,
        budget_triangles: { LOD0: budgetOf(id, 'LOD0'), LOD1: budgetOf(id, 'LOD1'), LOD2: budgetOf(id, 'LOD2'), ...(BUDGET_FOR[id] ? { why: BUDGET_FOR[id].why } : {}) },
        piece_count: x.live, surfaces: x.prims.LOD0, signs: x.prims.LOD0.filter((k) => k.startsWith('sign:')).map((k) => k.slice(5).split('|')[0]),
        max_side_steps: info.maxSide != null ? info.maxSide : 3,
        // A ship's waterline over its keel (the page draws it from its waterline, its ground at the keel).
        ...(info.sink ? { waterline_m: r3(at.y - info.base) } : {}),
      });
      console.log(`${site}/${id.padEnd(24)} LOD0 ${String(x.triangles.LOD0).padStart(7)}  LOD1 ${String(x.triangles.LOD1).padStart(6)}  LOD2 ${String(x.triangles.LOD2).padStart(4)}  pieces ${x.live}`);
    }
    const e = { kit: id, x: r3(at.x), z: r3(at.z), base: r3(info.base), turn: at.turn & 3 };
    if (info.anchorBuilt) e.anchor_built = true;
    if (info.sink) e.ship = true;
    placements.push(e);
  }
  const { parts, solids, unnamed } = groundOf(built, THREE, M);
  const lods = [['GROUND', Object.fromEntries(Object.entries(parts).map(([k, v]) => [k, v]))]];
  const gres = writeGlb(path.join(dir, 'ground.glb'), lods, { id: `${site}_ground`, site });
  const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
  for (const P of placements) { const m = manifest.find((k) => k.id === P.kit); if (!m) continue;
    for (const c of [[m.bounds_m.lo[0], m.bounds_m.lo[2]], [m.bounds_m.hi[0], m.bounds_m.hi[2]]]) {
      const [cx, cz] = P.turn === 1 ? [c[1], -c[0]] : P.turn === 2 ? [-c[0], -c[1]] : P.turn === 3 ? [-c[1], c[0]] : c;
      lo[0] = Math.min(lo[0], P.x + cx); hi[0] = Math.max(hi[0], P.x + cx); lo[2] = Math.min(lo[2], P.z + cz); hi[2] = Math.max(hi[2], P.z + cz); } }
  const table = {
    site, generator: 'tools/buildings/export_sites.cjs', page: path.relative(ROOT, page),
    units: 'metres; the site frame as the page draws it (x east, z south, y up from still water or the airfield ground); a kit stands on y = 0 at (x, base, z), turned by quarter turns (1: its front, +z, to +x)',
    marks: Object.fromEntries(Object.entries(marks).map(([k, v]) => [k, v.map(r3)])),
    pads: pads.map((p) => ({ lo: [r3(p[0]), r3(p[1])], hi: [r3(p[2]), r3(p[3])], y: r3(p[4]) })),
    kits: manifest, placements, ground: { glb: 'ground.glb', surfaces: gres.prims.GROUND, solids, materials: Object.fromEntries(Object.entries(parts).map(([k, v]) => [k, { colour: v.def.colour.map(r3), roughness: r3(v.def.roughness), metallic: r3(v.def.metallic), emissive: v.def.emissive.map(r3) }])) },
    // The night's lights (openspec/changes/night-voxel-light section 2d), the page's own list (its `nightLights`):
    // masts and floodlights, glow dots, and the runways the game lights by its runway rule. `lot` is a placement's
    // index: the light goes out when that placement is damaged.
    lights: {
      masts: lights.masts.map((m) => Object.assign({ x: r3(m.x), z: r3(m.z), base: r3(m.base), h: r3(m.h), aim: m.aim.map(r3), power: r3(m.power), range: r3(m.range), cone: r3(m.cone), colour: m.col, pole: m.pole }, m.lot != null ? { lot: m.lot } : {})),
      dots: lights.dots.map((d) => Object.assign({ at: d.slice(0, 3).map(r3), colour: d[3] }, d[4] != null ? { lot: d[4] } : {})),
      runways: lights.runways.map((r) => ({ a: r.a.map(r3), b: r.b.map(r3), w: r3(r.w), y: r3(r.y) })),
    },
    bounds_m: { lo: [r3(Math.min(lo[0], ...pads.map((p) => p[0]))), 0, r3(Math.min(lo[2], ...pads.map((p) => p[1])))], hi: [r3(Math.max(hi[0], ...pads.map((p) => p[2]))), 0, r3(Math.max(hi[2], ...pads.map((p) => p[3])))] },
  };
  fs.writeFileSync(path.join(dir, 'site.json'), JSON.stringify(table, null, 1) + '\n');
  console.log(`${site}: ${placements.length} placements of ${manifest.length} kits, ${lights.masts.length} masts, ${lights.dots.length} dots, ${lights.runways.length} runways, ground ${gres.tris.GROUND} triangles in ${gres.prims.GROUND.length} surfaces (${unnamed} unnamed meshes), ${solids.length} solids, ${((Date.now() - t0) / 1000).toFixed(1)} s`);
  return !failed;
}

let ok = true;
if (!ONLY || ONLY === 'airport') ok = exportSite('airport', path.join(ROOT, 'docs/mockups/airport.html'), { civil: true }) && ok;
if (!ONLY || ONLY === 'seaport') ok = exportSite('seaport', path.join(ROOT, 'docs/mockups/seaport.html'), {}) && ok;
if (!ok) process.exit(1);
