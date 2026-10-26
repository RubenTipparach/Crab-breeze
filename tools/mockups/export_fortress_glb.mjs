// Export the Hermit, the crab air fortress, from its mockup to the game's model (openspec/changes/crab-airship,
// design section 9): the block-out the owner approved ("looks good, build it in game") is the mockup, so the game
// draws the same geometry, built from the same tools/mockups/fortress_layout.json (CLAUDE.md 6.1; section 13 lists
// the exception to "meshes come from Blender" until the Blender model, task 3.1).
//
//   python3 tools/mockups/gen_crab_fortress.py
//   THREE_DIR=<dir>/package NODE_PATH=$(npm root -g) node tools/mockups/export_fortress_glb.mjs
//
// Writes assets/models/fortress_hermit.glb and its manifest fortress_hermit.json (triangles against the budget,
// materials, the named nodes the game drives). The page's armour maps are its 512 px copies; the export uses the
// full 1024 px sheets from assets/textures/mech and bakes each material's livery mask into its colour map (the page
// mixes the accent in a shader, which glTF has no slot for). Normal and ORM maps go in as they are.
// THREE_DIR: an unpacked `npm pack three@0.147.0`, where the CDN is not reachable (a cloud session's proxy).
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "../..");
const PAGE = path.join(ROOT, "docs/mockups/crab_fortress.html");
const OUT = path.join(ROOT, "assets/models/fortress_hermit.glb");
const MANIFEST = path.join(ROOT, "assets/models/fortress_hermit.json");
// what the game's rules need of the shape, measured off the same scene (as gen_mech_data.py measures a mech's glb)
const SHAPE = path.join(ROOT, "data/missions/fortress_shape.ron");
const LAYOUT = JSON.parse(fs.readFileSync(path.join(ROOT, "tools/mockups/fortress_layout.json"), "utf8"));
const BUDGET_TRIS = 60000;
const BUDGET_LOD1_TRIS = 40000;
const SHEETS = ["fortress_shell", "fortress_belly", "fortress_side", "nacelle", "panel"];
const MAPS = ["", "_n", "_orm", "_m"];
const CDN = "https://cdn.jsdelivr.net/npm/three@0.147.0/";

const tex = {};
for (const s of SHEETS) for (const k of MAPS) {
  tex[s + k] = "data:image/png;base64," + fs.readFileSync(path.join(ROOT, "assets/textures/mech", s + k + ".png")).toString("base64");
}

const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
page.on("pageerror", e => console.log("pageerror:", e.message));
if (process.env.THREE_DIR) {
  await page.route(/cdn\.jsdelivr\.net\/npm\/three@0\.147\.0\//, r => {
    const rel = new URL(r.request().url()).pathname.replace(/^\/npm\/three@0\.147\.0\//, "");
    r.fulfill({ path: path.join(process.env.THREE_DIR, rel), contentType: "application/javascript" });
  });
  await page.route(/fonts\.(googleapis|gstatic)\.com/, r => r.abort());
}
await page.goto("file://" + PAGE);
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
await page.addScriptTag({ url: CDN + "examples/js/exporters/GLTFExporter.js" });

const out = await page.evaluate(async tex => {
  const T = THREE, M0 = window.MOCKUP;
  M0.pause(); M0.reset();
  const { F, STATIC, parts } = M0.exportable();
  // the surfaces a mech stands on (the page tags them `walk`): each box's top face in the fortress's frame, as its
  // footprint and the plane over it (the pylon walkways slope)
  F.updateMatrixWorld(true);
  const toF = new T.Matrix4().copy(F.matrixWorld).invert(), walks = [];
  F.traverse(o => {
    if (!o.isMesh || !o.userData.walk) return;
    o.geometry.computeBoundingBox(); const bb = o.geometry.boundingBox, P = { width: bb.max.x - bb.min.x, height: bb.max.y - bb.min.y, depth: bb.max.z - bb.min.z }, rel = new T.Matrix4().multiplyMatrices(toF, o.matrixWorld);
    const c = [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([a, b]) => new T.Vector3(a * P.width / 2, P.height / 2, b * P.depth / 2).applyMatrix4(rel));
    const n = new T.Vector3().subVectors(c[1], c[0]).cross(new T.Vector3().subVectors(c[3], c[0])).normalize();
    const mid = c.reduce((m, v) => m.add(v), new T.Vector3()).multiplyScalar(0.25);
    walks.push({ name: o.userData.walk, min: [Math.min(...c.map(v => v.x)), Math.min(...c.map(v => v.z))], max: [Math.max(...c.map(v => v.x)), Math.max(...c.map(v => v.z))],
      centre: [mid.x, mid.z], top: mid.y, slope: [-n.x / n.y, -n.z / n.y] });
  });
  const partList = parts.filter(p => STATIC.has(p.obj) || [...STATIC].some(c => { let f = false; c.traverse(o => { f = f || o === p.obj; }); return f; })).map(p => {
    // a thruster's side: +1 port (+X), -1 starboard, 0 for parts on the centre or with no side
    const x = p.obj.getWorldPosition(new T.Vector3()).applyMatrix4(toF).x;
    return { id: p.id, kind: p.kind, side: ["lift", "hull"].includes(p.kind) ? Math.sign(x) : 0 };
  });
  const load = url => new Promise((ok, err) => { const im = new Image(); im.onload = () => ok(im); im.onerror = err; im.src = url; });
  const imgs = {};
  for (const [k, v] of Object.entries(tex)) imgs[k] = await load(v);
  const pixels = im => { const c = document.createElement("canvas"); c.width = im.width; c.height = im.height; const g = c.getContext("2d"); g.drawImage(im, 0, 0); return g.getImageData(0, 0, im.width, im.height); };
  const toLin = new Float32Array(256).map((_, i) => { const c = i / 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; });
  const LUT = 4096, toSrgb = new Uint8ClampedArray(LUT + 1).map((_, i) => { const c = i / LUT; return Math.round(255 * (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055)); });
  // A sheet's normal and ORM maps: unflipped, named `src:<sheet>_n`, so the file can point at the PNGs already in
  // assets/textures/mech instead of carrying copies (the meshes' V is flipped to glTF's convention to match). A mesh
  // that mixes them with the page's own canvas maps keeps three's convention and embedded, flipped copies.
  const plain = new Map();
  const texOf = (key, srgb, flip) => {
    const id = key + (flip ? ":flip" : "");
    if (!plain.has(id)) { const t = new T.Texture(imgs[key]); t.wrapS = t.wrapT = T.RepeatWrapping; if (srgb) t.encoding = T.sRGBEncoding;
      t.flipY = flip; if (!flip) t.name = "src:" + key; t.needsUpdate = true; plain.set(id, t); }
    return plain.get(id);
  };
  // a plated material's colour map with its tint and livery baked in: map * mix(colour, accent, mask), in linear light
  const bakedMats = new Map();
  const bake = (m, flip) => {
    const id = m.uuid + (flip ? ":flip" : "");
    if (bakedMats.has(id)) return bakedMats.get(id);
    const sheet = m.userData.sheet, base = pixels(imgs[sheet]), mask = pixels(imgs[sheet + "_m"]);
    const c = m.color, a = new T.Color(m.userData.accent).convertSRGBToLinear();
    const out = new ImageData(base.width, base.height), d = out.data, b = base.data, k = mask.data;
    for (let i = 0; i < d.length; i += 4) {
      const w = k[i] / 255, tint = [c.r + (a.r - c.r) * w, c.g + (a.g - c.g) * w, c.b + (a.b - c.b) * w];
      for (let ch = 0; ch < 3; ch++) d[i + ch] = toSrgb[Math.min(LUT, Math.round(toLin[b[i + ch]] * tint[ch] * LUT))];
      d[i + 3] = 255;
    }
    const cv = document.createElement("canvas"); cv.width = out.width; cv.height = out.height; cv.getContext("2d").putImageData(out, 0, 0);
    const map = new T.CanvasTexture(cv); map.encoding = T.sRGBEncoding; map.wrapS = map.wrapT = T.RepeatWrapping; map.flipY = flip; map.userData.mimeType = "image/jpeg";
    const orm = texOf(sheet + "_orm", false, flip);
    const nm = new T.MeshStandardMaterial({ name: `${sheet}_${m.color.getHexString()}_${m.userData.accent.slice(1)}`, map, normalMap: texOf(sheet + "_n", false, flip),
      roughnessMap: orm, metalnessMap: orm, aoMap: orm, roughness: 1, metalness: 1, side: m.side });
    bakedMats.set(id, nm); return nm;
  };
  const plated = m => !!(m && m.userData && m.userData.plated);
  const swap = (m, flip) => (plated(m) ? bake(m, flip) : m);
  // glTF's V runs down the image, three's (with flipY) up it
  const flipV = g => ["uv", "uv2"].forEach(k => { const a = g.attributes[k]; if (!a) return; for (let i = 0; i < a.count; i++) a.setY(i, 1 - a.getY(i)); a.needsUpdate = true; });
  // the fortress as built: what rides it (the deck guard, the pilot), the range shells and every effect stay out
  const root = new T.Group(); root.name = "FORTRESS_hermit";
  // cloning copies userData through JSON, and the page's part records point back at their objects: set it aside
  const kept = new Map();
  F.traverse(o => { kept.set(o, o.userData); o.userData = {}; });
  try { F.children.forEach(c => { if (STATIC.has(c) && c.visible) root.add(c.clone(true)); }); }
  finally { kept.forEach((u, o) => { o.userData = u; }); }
  const drop = [], inst = [];
  root.traverse(o => {
    if (o === root) return;
    o.userData = {};   // the page's bookkeeping (part records, saved materials) is not the model's
    if (!o.visible || o.isPoints || o.isLine || (o.isMesh && [].concat(o.material).some(m => m.isMeshBasicMaterial || m.isShaderMaterial || m.transparent))) { drop.push(o); return; }
    if (o.isInstancedMesh) { inst.push(o); return; }
    if (o.isMesh) {
      const all = [].concat(o.material), own = all.every(plated);
      if (own) { o.geometry = o.geometry.clone(); flipV(o.geometry); }
      o.material = Array.isArray(o.material) ? o.material.map(m => swap(m, !own)) : swap(o.material, !own);
    }
  });
  drop.forEach(o => o.parent && o.parent.remove(o));
  // instanced spikes become one plain mesh each (glTF instancing is an extension Bevy does not read), named
  // DETAIL_: the game draws them only up close (design section 9, its levels of detail)
  let detailN = 0;
  inst.forEach(o => {
    const geos = [], m4 = new T.Matrix4();
    for (let i = 0; i < o.count; i++) { o.getMatrixAt(i, m4); const g = (o.geometry.index ? o.geometry.toNonIndexed() : o.geometry.clone()); g.applyMatrix4(m4); geos.push(g); }
    const mesh = new T.Mesh(T.BufferGeometryUtils.mergeBufferGeometries(geos), swap(o.material, true));
    mesh.name = `DETAIL_spikes_${detailN++}`; mesh.position.copy(o.position); mesh.quaternion.copy(o.quaternion); mesh.scale.copy(o.scale);
    o.parent.add(mesh); o.parent.remove(o);
  });
  root.updateMatrixWorld(true);
  // nothing of the page's bookkeeping goes into the file (glTF extras): only a baked map's JPEG hint stays
  root.traverse(o => { if (!o.isMesh) return; o.geometry.userData = {};
    [].concat(o.material).forEach(m => { m.userData = {}; ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap", "emissiveMap"].forEach(k => {
      if (m[k]) m[k].userData = m[k].isCanvasTexture && k === "map" && m.normalMap ? { mimeType: "image/jpeg" } : {}; }); }); });
  let tris = 0, detailTris = 0; const names = [], mats = new Set(), box = new T.Box3().setFromObject(root);
  root.traverse(o => {
    if (o.isMesh) { const g = o.geometry, n = (g.index ? g.index.count : g.attributes.position.count) / 3; tris += n;
      if (o.name.startsWith("DETAIL_")) detailTris += n; [].concat(o.material).forEach(m => mats.add(m)); }
    if (/^(PART|YAW|PITCH|MUZZLE|LEG|DOOR|DETAIL)_/.test(o.name)) names.push(o.name);
  });
  const glb = await new Promise((ok, err) => new T.GLTFExporter().parse(root, ok, err, { binary: true, onlyVisible: true, maxTextureSize: 4096 }));
  let bin = ""; const bytes = new Uint8Array(glb);
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return { glb: btoa(bin), tris: Math.round(tris), detailTris: Math.round(detailTris), materials: mats.size, names: names.sort(), min: box.min.toArray(), max: box.max.toArray(), walks, parts: partList };
}, tex);
await browser.close();

fs.writeFileSync(OUT, externalise(Buffer.from(out.glb, "base64")));
// Images of textures named `src:<sheet>_<map>` point at assets/textures/mech/<sheet>_<map>.png; their copies leave
// the binary chunk, which is repacked.
function externalise(glb) {
  const jsonLen = glb.readUInt32LE(12), json = JSON.parse(glb.subarray(20, 20 + jsonLen).toString("utf8"));
  const binStart = 20 + jsonLen + 8, bin = glb.subarray(binStart, binStart + glb.readUInt32LE(20 + jsonLen));
  const external = new Set();
  for (const t of json.textures || []) {
    if (!t.name || !t.name.startsWith("src:")) continue;
    const key = t.name.slice(4), img = json.images[t.source];
    if (!fs.existsSync(path.join(ROOT, "assets/textures/mech", key + ".png"))) throw new Error(`no source map for ${key}`);
    if (img.bufferView !== undefined) external.add(img.bufferView);
    json.images[t.source] = { uri: `../textures/mech/${key}.png`, mimeType: "image/png", name: key };
    t.name = key;
  }
  const keep = json.bufferViews.map((_, i) => !external.has(i)), remap = [], parts = [];
  let offset = 0;
  json.bufferViews = json.bufferViews.filter((v, i) => {
    if (!keep[i]) { remap[i] = -1; return false; }
    remap[i] = remap.filter(x => x >= 0).length;
    const bytes = bin.subarray(v.byteOffset || 0, (v.byteOffset || 0) + v.byteLength), pad = (4 - (bytes.length % 4)) % 4;
    v.byteOffset = offset; parts.push(bytes, Buffer.alloc(pad)); offset += bytes.length + pad; return true;
  });
  for (const a of json.accessors || []) if (a.bufferView !== undefined) a.bufferView = remap[a.bufferView];
  for (const im of json.images || []) if (im.bufferView !== undefined) im.bufferView = remap[im.bufferView];
  json.buffers = [{ byteLength: offset }];
  let text = Buffer.from(JSON.stringify(json), "utf8");
  text = Buffer.concat([text, Buffer.alloc((4 - (text.length % 4)) % 4, 0x20)]);
  const body = Buffer.concat(parts), head = Buffer.alloc(12), jh = Buffer.alloc(8), bh = Buffer.alloc(8);
  head.write("glTF", 0); head.writeUInt32LE(2, 4); head.writeUInt32LE(12 + 8 + text.length + 8 + body.length, 8);
  jh.writeUInt32LE(text.length, 0); jh.write("JSON", 4); bh.writeUInt32LE(body.length, 0); bh.write("BIN\0", 4);
  return Buffer.concat([head, jh, text, bh, body]);
}

const manifest = {
  _: "The Hermit's block-out, exported from its mockup by tools/mockups/export_fortress_glb.mjs (openspec/changes/crab-airship section 9). Frame: metres, +Y up, the fortress faces +Z, +X its own left (port), origin at the hull's middle.",
  file: "fortress_hermit.glb",
  // LOD0 with its DETAIL_ meshes (the spikes), drawn up close; LOD1 without them, beyond the data's detail_range_m
  lods: [
    { lod: 0, triangles: out.tris, budget_triangles: BUDGET_TRIS },
    { lod: 1, triangles: out.tris - out.detailTris, budget_triangles: BUDGET_LOD1_TRIS },
  ],
  materials: out.materials,
  bounds_m: { min: out.min.map(v => +v.toFixed(2)), max: out.max.map(v => +v.toFixed(2)) },
  nodes: out.names,
};
fs.writeFileSync(MANIFEST, JSON.stringify(manifest, null, 2) + "\n");

// The hull as capsules that stop rounds and hide the pilot from the far side's guns: the shell's ellipsoid in
// lengthwise capsules across its width, the belly's likewise (fortress_layout.json's numbers)
const hull = [];
const SH = LAYOUT.shell, BE = LAYOUT.belly;
for (const x0 of [0, 45, -45, 85, -85, 115, -115]) {
  const f = Math.sqrt(1 - (x0 / SH.ax) ** 2), r = (SH.top_m + SH.under_m) / 2 * f, y = (SH.top_m - SH.under_m) / 2 * f, half = Math.max(0, SH.az * f - r);
  hull.push({ a: [x0, y, -half], b: [x0, y, half], r });
}
for (const x0 of [0, 40, -40]) {
  const f = Math.sqrt(1 - (x0 / BE.ax) ** 2), r = 28 * f, half = Math.max(0, BE.az * f - r);
  hull.push({ a: [x0, -30, BE.cz - half], b: [x0, -30, BE.cz + half], r });
}
const n = v => (Math.abs(v) < 5e-4 ? 0 : v).toFixed(3);
const v2 = v => `(${n(v[0])}, ${n(v[1])})`, v3 = v => `(${n(v[0])}, ${n(v[1])}, ${n(v[2])})`;
const ron = [
  "// The Hermit's shape for the game's rules, measured off its mockup by tools/mockups/export_fortress_glb.mjs:",
  "// do not edit by hand (openspec/changes/crab-airship section 9). Frame: metres, +Y up, the fortress faces +Z,",
  "// +X its own left (port). Parts are the glb's PART_<id> nodes; platforms are the tops a mech stands on (a",
  "// footprint and the plane over it); hull capsules stop rounds and lines of sight.",
  "(",
  `    model: "models/${manifest.file}",`,
  "    parts: [",
  ...out.parts.map(p => `        (id: "${p.id}", kind: "${p.kind}", side: ${p.side}),`),
  "    ],",
  "    platforms: [",
  ...out.walks.map(w => `        (name: "${w.name}", min: ${v2(w.min)}, max: ${v2(w.max)}, centre: ${v2(w.centre)}, top_m: ${n(w.top)}, slope: ${v2(w.slope)}),`),
  "    ],",
  "    hull: [",
  ...hull.map(h => `        (a: ${v3(h.a)}, b: ${v3(h.b)}, radius_m: ${n(h.r)}),`),
  "    ],",
  ")",
];
fs.writeFileSync(SHAPE, ron.join("\n") + "\n");
console.log(`wrote ${path.relative(ROOT, SHAPE)} (${out.parts.length} parts, ${out.walks.length} platforms, ${hull.length} hull capsules)`);
console.log(`wrote ${path.relative(ROOT, OUT)} (${(fs.statSync(OUT).size / 1e6).toFixed(2)} MB, LOD0 ${out.tris} triangles, LOD1 ${out.tris - out.detailTris}, ${out.materials} materials, ${out.names.length} named nodes)`);
if (out.tris > BUDGET_TRIS) { console.error(`LOD0 over budget: ${out.tris} triangles against ${BUDGET_TRIS}`); process.exit(1); }
if (out.tris - out.detailTris > BUDGET_LOD1_TRIS) { console.error(`LOD1 over budget: ${out.tris - out.detailTris} triangles against ${BUDGET_LOD1_TRIS}`); process.exit(1); }
