"""Build docs/mockups/mech_designs.html: the mech design candidates, animated, at true scale.

    python3 tools/mockups/gen_mech_designs.py

Reads assets/models/mech_*.glb and assets/models/mechs.json (written by tools/blender/build_mechs.py)
and inlines them, so the page needs nothing but three.js r147 from the CDN. Every size on the page is
measured off the glb (the manifest), never typed. Mining Mike's rule for model review: orbit it, at
true relative scale, with a known object (the pilot, 1.8 m) and a rail at the mech's height.

The animation is Mining Mike's (godot-sandbox tank_bosses_kit.js at 3336b88): Pose, idleGeneric,
walkGeneric, turnStep, skatePose, hitGeneric and bendLegs, run on the glb's own nodes through a name
map from the kit's joints to the rig contract (docs/design/mech-rig.md). It is the reference for how
cb-app should drive the same bones.
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "mech_designs.html")
ORDER = ["breeze", "bulwark", "ravager"]

PAGE = r"""<title>Crab Breeze Mechs</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: a hangar review sheet. The stage is the page; the spec cards sit beside it like a clipboard. */
:root {
  --bg: #e9ebef; --panel: #f7f8fa; --edge: #c4cad3; --ink: #12161c; --muted: #56606d;
  --accent: #1f5fbf; --accent-soft: #dce6f6; --warn: #a86a00; --stage: #1c1420;
  --display: "Saira Condensed", "Arial Narrow", sans-serif;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0d1015; --panel: #151a21; --edge: #2a323d; --ink: #e6ebf1; --muted: #94a0ad;
    --accent: #6ea8ff; --accent-soft: #1b2a40; --warn: #f0b13c; color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #0d1015; --panel: #151a21; --edge: #2a323d; --ink: #e6ebf1; --muted: #94a0ad;
  --accent: #6ea8ff; --accent-soft: #1b2a40; --warn: #f0b13c; color-scheme: dark;
}
* { box-sizing: border-box; }
body { background: var(--bg); color: var(--ink); font: 15px/1.5 var(--body); margin: 0;
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); padding-block: 18px 32px; }
header { display: grid; gap: 4px; margin: 0 auto 14px; max-width: 1400px; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; text-wrap: balance; }
.lede { margin: 0; color: var(--muted); max-width: 76ch; }
.wrap { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 16px; align-items: start; max-width: 1400px; margin: 0 auto; }
@media (max-width: 1000px) { .wrap { grid-template-columns: minmax(0, 1fr); } }
.bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 6ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button:hover { border-color: var(--muted); }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button:focus-visible, canvas:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.stage { position: relative; background: var(--stage); border: 1px solid var(--edge); border-radius: 4px; overflow: hidden; touch-action: none; }
canvas { display: block; width: 100%; max-width: 100%; aspect-ratio: 16 / 10; }
@media (max-width: 600px) { canvas { aspect-ratio: 1 / 1.1; } }
.tag { position: absolute; transform: translate(-50%, -100%); pointer-events: none; font: 500 11px var(--mono);
  color: #e6ebf1; background: rgba(14, 10, 18, 0.85); border: 1px solid #3a2f40; padding: 2px 6px; white-space: nowrap; }
.mode { position: absolute; left: 10px; top: 10px; font: 700 13px var(--display); letter-spacing: 0.08em; text-transform: uppercase;
  color: #e6ebf1; background: rgba(14, 10, 18, 0.85); border: 1px solid #3a2f40; padding: 5px 9px; pointer-events: none; }
.hint { font-size: 13px; color: var(--muted); margin: 6px 0 0; }
aside { display: grid; gap: 12px; min-width: 0; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 12px 14px; display: grid; gap: 6px; cursor: pointer; }
.card.sel { border-color: var(--accent); }
.card h2 { font: 700 24px/1 var(--display); letter-spacing: 0.05em; text-transform: uppercase; margin: 0; display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
.card h2 small { font: 500 11px var(--mono); color: var(--warn); letter-spacing: 0.08em; }
.kv { display: grid; grid-template-columns: 9ch minmax(0, 1fr); gap: 2px 10px; font-size: 13px; margin: 0; }
.kv dt { color: var(--muted); font-family: var(--mono); font-size: 12px; padding-top: 1px; }
.kv dd { margin: 0; font-variant-numeric: tabular-nums; }
.note { font-size: 13px; color: var(--muted); margin: 0; }
.foot { max-width: 1400px; margin: 16px auto 0; font-size: 13px; color: var(--muted); display: grid; gap: 4px; }
.foot code { font: 12px var(--mono); overflow-wrap: anywhere; }
</style>

<header>
  <div class="eyebrow">Crab Breeze · mech design review · second pass, Mining Mike's boss kit</div>
  <h1>Pick a chassis</h1>
  <p class="lede">Built in Blender with a port of Mining Mike's boss mech kit: its plating textures, bevelled plates, livery bands, glowing vents, hoses, pistons and its walk. Bulwark and Ravager are Mining Mike's Uber mechs, refitted with a cockpit you can sit in; Breeze is ours in the same style. All at true scale beside a 1.8 m pilot and a four-storey block.</p>
</header>

<div class="wrap">
  <section>
    <div class="bar" role="group" aria-label="Mech">
      <span class="lbl">Mech</span>
      <button id="m-breeze" data-mech="breeze">Breeze</button>
      <button id="m-bulwark" data-mech="bulwark">Bulwark</button>
      <button id="m-ravager" data-mech="ravager">Ravager</button>
      <button id="m-all" data-mech="all">Line up</button>
    </div>
    <div class="bar" role="group" aria-label="Move">
      <span class="lbl">Move</span>
      <button id="a-idle" data-act="idle">Idle</button>
      <button id="a-walk" data-act="walk">Walk</button>
      <button id="a-turn" data-act="turn">Turn</button>
      <button id="a-skate" data-act="skate">Skate</button>
      <button id="a-hit" data-act="hit">Take a hit</button>
    </div>
    <div class="bar" role="group" aria-label="View">
      <span class="lbl">View</span>
      <button id="v-orbit" data-view="orbit">Orbit</button>
      <button id="v-cockpit" data-view="cockpit">Cockpit</button>
      <button id="v-front" data-view="front">Front</button>
      <button id="v-side" data-view="side">Side</button>
      <button id="v-back" data-view="back">Back</button>
      <span class="lbl" style="margin-left:8px">Show</span>
      <button id="t-city" data-toggle="city" aria-pressed="true">Block</button>
      <button id="t-pilot" data-toggle="pilot" aria-pressed="true">Pilot</button>
    </div>
    <div class="stage">
      <canvas id="view" tabindex="0" aria-label="3D view of the selected mech"></canvas>
      <div class="mode" id="mode">Orbit</div>
      <div id="tags"></div>
    </div>
    <p class="hint">Drag to orbit, scroll or pinch to zoom, right drag to pan. In the cockpit, drag to look around; the view rides the pilot's eye through every move.</p>
  </section>
  <aside id="cards"></aside>
</div>

<div class="foot">
  <div>Grid: 4 m cells. The yellow rail is 12 m (survey M1). Sizes and triangle counts are measured off each glb by the generator, not typed.</div>
  <div>Models: <code>tools/blender/build_mechs.py</code> on <code>tools/blender/mech_kit.py</code>; plating: <code>tools/textures/gen_mech_plating.mjs</code>; rig: <code>docs/design/mech-rig.md</code>. Kit, textures and animation after Mining Mike's <code>tank_bosses_kit.js</code>.</div>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__;
const GLB = __GLB__;
const ORDER = __ORDER__;
const T = THREE, TAU = Math.PI * 2;
__MECH_POSE__

const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.0;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = T.PCFSoftShadowMap;

// dusk, as Mining Mike's Uber Mechs page: violet sky, a low warm sun, cool rim
const scene = new T.Scene();
scene.background = new T.Color(0x2a1f30);
scene.fog = new T.Fog(0x2a1f30, 60, 190);
const camera = new T.PerspectiveCamera(42, 1.6, 0.05, 500);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
scene.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.5));
const sun = new T.DirectionalLight(0xffc89a, 2.0);
sun.position.set(-22, 26, 18); sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -32, right: 32, top: 32, bottom: -32, near: 1, far: 100 });
sun.shadow.bias = -0.0005;
scene.add(sun);
const rim = new T.DirectionalLight(0x7fa8ff, 0.9); rim.position.set(18, 12, -22); scene.add(rim);

const ground = new T.Mesh(new T.PlaneGeometry(500, 500), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const grid = new T.GridHelper(160, 40, 0x4a4552, 0x3a3640); grid.position.y = 0.01; scene.add(grid);
const rail = new T.Mesh(new T.BoxGeometry(80, 0.08, 0.08), new T.MeshBasicMaterial({ color: 0xf0b13c }));
rail.position.set(0, 12, -16); scene.add(rail);

const city = new T.Group();
function block(x, z, w, d, storeys, col) {
  const h = storeys * 3.5;
  const m = new T.Mesh(new T.BoxGeometry(w, h, d), new T.MeshStandardMaterial({ color: col, roughness: 0.85 }));
  m.position.set(x, h / 2, z); m.castShadow = m.receiveShadow = true; city.add(m);
  const win = new T.MeshStandardMaterial({ color: 0x1a2230, emissive: 0xffc878, emissiveIntensity: 0.5, roughness: 0.3 });
  for (let s = 0; s < storeys; s++) for (let i = 0; i < Math.floor(w / 2.6); i++) {
    const g = new T.Mesh(new T.BoxGeometry(1.4, 1.6, 0.1), win);
    g.position.set(x - w / 2 + 1.4 + i * 2.6, s * 3.5 + 2.0, z + d / 2 + 0.05); city.add(g);
  }
}
block(-22, -8, 14, 10, 4, 0x6a625a); block(-36, -10, 10, 12, 9, 0x545961); block(22, -12, 12, 10, 2, 0x6e6352);
const paint = new T.MeshStandardMaterial({ color: 0x7a1f1f, roughness: 0.35, metalness: 0.3 });
const carBody = new T.Mesh(new T.BoxGeometry(1.8, 0.75, 4.4), paint); carBody.position.set(-14, 0.55, 7);
const carCab = new T.Mesh(new T.BoxGeometry(1.6, 0.6, 2.2), new T.MeshStandardMaterial({ color: 0x1d2630, roughness: 0.2 })); carCab.position.set(-14, 1.2, 6.8);
[carBody, carCab].forEach(m => { m.castShadow = true; city.add(m); });
scene.add(city);

const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a, roughness: 0.8 });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torsoM = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torsoM.position.y = 1.18; pilot.add(torsoM);
const head = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.66; head.scale.set(1, 1.15, 1); pilot.add(head);
[-1, 1].forEach(s => { const a = new T.Mesh(new T.CylinderGeometry(0.06, 0.045, 0.62, 8), suit); a.position.set(0.3 * s, 1.15, 0); a.rotation.z = 0.12 * s; pilot.add(a); });
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; });
scene.add(pilot);

// the halo sprite, Mining Mike's radial glow
const glowTex = (() => { const c = document.createElement("canvas"); c.width = c.height = 128; const q = c.getContext("2d");
  const g = q.createRadialGradient(64, 64, 0, 64, 64, 64); g.addColorStop(0, "rgba(255,255,255,1)"); g.addColorStop(0.18, "rgba(255,255,255,0.75)");
  g.addColorStop(0.45, "rgba(255,255,255,0.18)"); g.addColorStop(1, "rgba(255,255,255,0)"); q.fillStyle = g; q.fillRect(0, 0, 128, 128); return new T.CanvasTexture(c); })();

// ---------- load every mech ----------
const loader = new T.GLTFLoader();
const mechs = {};
function b64ToBuf(b64) { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; }
const parseX = v => (typeof v === "string" ? JSON.parse(v) : v);
const loaded = Promise.all(ORDER.map((id, idx) => new Promise(res => loader.parse(b64ToBuf(GLB[id]), "", g => {
  const root = g.scene, nodes = {}, halos = [], cores = [];
  root.traverse(o => {
    if (o.name) nodes[o.name] = o;
    if (o.isMesh) { o.castShadow = true; o.receiveShadow = true;
      if (o.material && o.material.transparent) { o.material.depthWrite = false; o.castShadow = false; o.renderOrder = 2; o.material.side = T.DoubleSide; } }
    if (o.name.startsWith("CORE_")) cores.push(o);
  });
  root.traverse(o => { const u = o.userData || {}; if (u.halo_size_m) {
    const mat = new T.SpriteMaterial({ map: glowTex, color: new T.Color(u.halo_color), blending: T.AdditiveBlending, depthWrite: false, transparent: true, opacity: u.halo_opacity, toneMapped: false });
    const sp = new T.Sprite(mat); sp.scale.setScalar(u.halo_size_m); o.add(sp); halos.push(sp); } });
  const x = (nodes.root && nodes.root.userData) || {};
  const b = { id, root, nodes, cores, halos, seed: idx * 7 + 3, ph: 0, t: hash(idx) * 10, scale: x.scale || 0.13,
    legSegs: parseX(x.legSegs), footH: x.footH, walk: parseX(x.walk), idleP: parseX(x.idle), hitT: 9, hitSide: 1, wWalk: 0, wTurn: 0, wSkate: 0, rest: {} };
  for (const n in nodes) if (MAPPED.has(n)) { const o = nodes[n]; b.rest[n] = { e: new T.Euler().setFromQuaternion(o.quaternion, "XYZ"), p: o.position.clone() }; }
  mechs[id] = b;
  scene.add(root);
  res();
}))));

let current = "breeze", view = "orbit", act = "idle";
const tagsEl = document.getElementById("tags");
function layout() {
  const spread = current === "all";
  ORDER.forEach((id, i) => { const m = mechs[id]; if (!m) return; m.root.visible = spread || id === current; m.root.position.set(spread ? (i - 1) * 15 : 0, 0, 0); });
  pilot.position.set(spread ? -7.5 : 5, 0, 6); pilot.rotation.y = -0.6;
}
function eyeOf(id) { const m = mechs[id]; const p = new T.Vector3(); if (m && m.nodes.EYE_pilot) m.nodes.EYE_pilot.getWorldPosition(p); return p; }
function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
  document.getElementById("mode").textContent = { orbit: "Orbit", cockpit: "Cockpit, pilot's eye", front: "Front", side: "Side", back: "Back" }[v];
  if (v === "cockpit") {
    if (current === "all") setMech("breeze", true);
    const e = eyeOf(current);
    camera.fov = 72; camera.near = 0.02; camera.updateProjectionMatrix();
    camera.position.copy(e); controls.target.copy(e).add(new T.Vector3(0, -0.3, 1));
    Object.assign(controls, { minDistance: 0.01, maxDistance: 1.0, enablePan: false, enableZoom: false, rotateSpeed: -0.35 });
  } else {
    camera.fov = 42; camera.near = 0.05; camera.updateProjectionMatrix();
    Object.assign(controls, { minDistance: 4, maxDistance: 140, enablePan: true, enableZoom: true, rotateSpeed: 1 });
    const d = current === "all" ? 44 : 24;
    controls.target.set(0, 6.2, 0);
    if (v === "front") camera.position.set(0, 7, d); else if (v === "side") camera.position.set(d, 7, 0);
    else if (v === "back") camera.position.set(-d * 0.5, 9, -d * 0.85); else camera.position.set(d * 0.7, 9, d * 0.8);
  }
  controls.update();
}
function setMech(id, keep) {
  current = id;
  document.querySelectorAll("[data-mech]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.mech === id)));
  document.querySelectorAll(".card").forEach(c => c.classList.toggle("sel", c.dataset.id === id));
  layout(); if (!keep) setView(view === "cockpit" && id === "all" ? "orbit" : view);
}
function setAct(a) {
  if (a === "hit") { for (const id in mechs) { mechs[id].hitT = 0; mechs[id].hitSide = Math.random() < 0.5 ? 1 : -1; } return; }
  act = a; document.querySelectorAll("[data-act]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.act === a)));
}
document.querySelectorAll("[data-mech]").forEach(b => b.addEventListener("click", () => setMech(b.dataset.mech)));
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => setView(b.dataset.view)));
document.querySelectorAll("[data-act]").forEach(b => b.addEventListener("click", () => setAct(b.dataset.act)));
document.querySelectorAll("[data-toggle]").forEach(b => b.addEventListener("click", () => {
  const on = b.getAttribute("aria-pressed") !== "true"; b.setAttribute("aria-pressed", String(on));
  if (b.dataset.toggle === "city") city.visible = on; if (b.dataset.toggle === "pilot") pilot.visible = on;
}));

const cards = document.getElementById("cards");
ORDER.forEach(id => {
  const m = META[id], b = m.bounds_m, el = document.createElement("article");
  el.className = "card"; el.dataset.id = id;
  el.innerHTML = `<h2>${m.name}${id === "breeze" ? " <small>RECOMMENDED</small>" : ""}</h2>
    <dl class="kv"><dt>Height</dt><dd>${b.y[1].toFixed(1)} m</dd><dt>Width</dt><dd>${(b.x[1] - b.x[0]).toFixed(1)} m</dd>
    <dt>Length</dt><dd>${(b.z[1] - b.z[0]).toFixed(1)} m</dd><dt>Legs</dt><dd>${m.legs}</dd><dt>Cockpit</dt><dd>${m.cockpit}</dd>
    <dt>Weapons</dt><dd>${m.weapons}</dd><dt>Triangles</dt><dd>${m.triangles.toLocaleString("en-US")} (LOD0)</dd></dl>
    <p class="note">${m.note}</p>`;
  el.addEventListener("click", () => setMech(id));
  cards.appendChild(el);
});

function animate(b, dt) {
  b.t += dt; b.hitT += dt;
  b.wWalk += ((act === "walk" ? 1 : 0) - b.wWalk) * Math.min(1, dt * 4);
  b.wTurn += ((act === "turn" ? 1 : 0) - b.wTurn) * Math.min(1, dt * 6);
  b.wSkate += ((act === "skate" ? 1 : 0) - b.wSkate) * Math.min(1, dt * 3);
  b.ph += dt * TAU * b.walk.hz * (b.wWalk + 0.9 * b.wTurn);
  const P = new Pose();
  P.w = 1 - b.wWalk; idleGeneric(P, b.t, b);
  P.w = b.wWalk; walkGeneric(P, b.t, b);
  P.w = b.wTurn * (1 - b.wWalk); turnStep(P, b);
  P.w = b.wSkate; skatePose(P, b);
  if (b.hitT < 0.9) { P.w = 1; hitGeneric(P, b.hitT, b); }
  applyPose(b, P);
  if (b.wTurn > 0.05) b.root.rotation.y += dt * 0.6 * b.wTurn; else b.root.rotation.y *= Math.max(0, 1 - dt * 2);
  for (const c of b.cores) c.rotation.y += dt * 1.4;
  if (b.nodes.ladder) b.nodes.ladder.visible = act === "idle" && b.wWalk < 0.05;
}

function placeTags() {
  tagsEl.innerHTML = ""; if (view === "cockpit") return;
  const r = canvas.getBoundingClientRect();
  const add = (p, text) => { const v = p.clone().project(camera); if (v.z > 1 || Math.abs(v.x) > 1.05 || Math.abs(v.y) > 1.05) return;
    const d = document.createElement("div"); d.className = "tag"; d.textContent = text;
    d.style.left = ((v.x + 1) / 2 * r.width) + "px"; d.style.top = ((1 - v.y) / 2 * r.height) + "px"; tagsEl.appendChild(d); };
  if (pilot.visible) add(new T.Vector3(pilot.position.x, 2.1, pilot.position.z), "pilot 1.8 m");
  if (city.visible) { add(new T.Vector3(-22, 14.6, -3), "4 storeys, 14 m"); add(new T.Vector3(-14, 2.2, 7), "car 4.4 m"); }
  add(new T.Vector3(34, 12.4, -16), "12 m");
}
function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight, pr = renderer.getPixelRatio();
  if (canvas.width !== Math.round(w * pr) || canvas.height !== Math.round(h * pr)) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
}
let last = 0, frame = 0;
function loop(ms) {
  const dt = Math.min(0.05, (ms - last) / 1000 || 0.016); last = ms;
  resize();
  for (const id in mechs) if (mechs[id].root.visible) animate(mechs[id], dt);
  if (view === "cockpit") { const e = eyeOf(current), dl = e.clone().sub(camera.position); camera.position.add(dl); controls.target.add(dl); }
  controls.update(); renderer.render(scene, camera);
  if ((frame++ & 3) === 0) placeTags();
  requestAnimationFrame(loop);
}
loaded.then(() => { setMech("breeze"); setAct("idle"); setView("orbit"); window.MOCKUP_READY = true; });
setMech("breeze", true); requestAnimationFrame(loop);
window.MOCKUP = { setMech, setView, setAct };
</script>
"""


def main():
    meta = json.load(open(os.path.join(MODELS, "mechs.json")))
    glb = {}
    for key in ORDER:
        with open(os.path.join(MODELS, meta[key]["file"]), "rb") as f:
            glb[key] = base64.b64encode(f.read()).decode("ascii")
    pose = open(os.path.join(ROOT, "tools", "mockups", "mech_pose.js")).read()
    html = (PAGE.replace("__MECH_POSE__", pose).replace("__META__", json.dumps({k: meta[k] for k in ORDER}))
            .replace("__GLB__", json.dumps(glb)).replace("__ORDER__", json.dumps(ORDER)))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
