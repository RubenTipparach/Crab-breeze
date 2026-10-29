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
const { LOD_M, SMALL_M3, exportKit, r3 } = require('./kit_export.cjs');

const args = process.argv.slice(2);
const opt = (name, def) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : def; };
const SEED = Number(opt('--seed', 7));
const OUT = opt('--out', 'assets/models/small_buildings');
// Triangle budgets per LOD (design section 4). LOD0 includes the interior.
const BUDGET = { LOD0: 16000, LOD1: 7000, LOD2: 120 };
// Per-type overrides, each with its reason.
const BUDGET_FOR = { motel: { LOD1: 8500, why: '28 rooms over 37 m: about three times the facade of the next largest type' } };
const budgetOf = (id, lod) => (BUDGET_FOR[id] && BUDGET_FOR[id][lod]) || BUDGET[lod];

// ------------------------------------------------------------------ export
fs.mkdirSync(OUT, { recursive: true });
const signs = (txt, bg, fg, neon) => `sign:${txt}|${bg}|${fg}|${neon ? 1 : 0}`;
const M = { sign: signs, awning: (c) => `awning:${c}` };
const manifest = { generator: 'tools/buildings/export_small_buildings.cjs', kit: 'docs/mockups/small_buildings_kit.js', seed: SEED, units: 'metres; front faces +z, centred on x = 0, standing on y = 0', lod_within_m: LOD_M, lod1_drops: { interior: true, pieces_under_m3: SMALL_M3 }, budget_triangles: BUDGET, types: [] };
let failed = false;
// Every type, and each suburban house's variants (design section 7.1).
const BUILDS = K.TYPES.flatMap((T) => [0, ...(T.variants || []).map((_, i) => i + 1)].map((v) => ({ T, v, id: K.variantId(T.id, v) })));
for (const { T, v, id } of BUILDS) {
  const B = K.build(T.id, SEED, M, v);
  const x = exportKit(B, id, OUT, { meta: { seed: SEED, variant: v } });
  const over = Object.entries(x.counted).filter(([l, n]) => n > budgetOf(T.id, l));
  if (over.length) { failed = true; console.error(`OVER BUDGET ${id}: ${over.map(([l, n]) => `${l} ${n} > ${budgetOf(T.id, l)}`).join(', ')}`); }
  manifest.types.push({
    id, name: v ? T.variants[v - 1] : T.name, kind: T.kind, ...(v ? { variant_of: T.id, variant: v } : {}), biomes: T.biome.split(',').map((b) => b.trim()), blurb: T.blurb,
    glb: `${id}.glb`, pieces: `${id}.pieces.json`, access: B.access && (B.access.drive ? { drive: B.access.drive.map(r3) } : { lot: r3(B.access.lot), stalls: B.access.stalls !== false }),
    bounds_m: { lo: x.lo, hi: x.hi }, triangles: x.triangles, budget_triangles: { LOD0: budgetOf(T.id, 'LOD0'), LOD1: budgetOf(T.id, 'LOD1'), LOD2: budgetOf(T.id, 'LOD2'), ...(BUDGET_FOR[T.id] ? { why: BUDGET_FOR[T.id].why } : {}) }, piece_count: x.live,
    surfaces: x.prims.LOD0, signs: x.prims.LOD0.filter((k) => k.startsWith('sign:')).map((k) => k.slice(5).split('|')[0]),
  });
  console.log(`${id.padEnd(11)} LOD0 ${String(x.triangles.LOD0).padStart(6)}  LOD1 ${String(x.triangles.LOD1).padStart(5)}  LOD2 ${String(x.triangles.LOD2).padStart(4)}  pieces ${x.live}  ${(fs.statSync(x.file).size / 1024).toFixed(0)} KB`);
}
fs.writeFileSync(path.join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 1) + '\n');
if (failed) process.exit(1);
