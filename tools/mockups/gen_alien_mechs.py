"""Build docs/mockups/alien_mechs.html: the alien crab mechs, animated, at true scale beside our Foreman.

    python3 tools/mockups/gen_alien_mechs.py

Reads assets/models/alien_*.glb and assets/models/aliens.json (written by
tools/blender/build_alien_mechs.py) and our Foreman (assets/models/mech_foreman.glb, from
tools/blender/build_mechs.py), and inlines them, so the page needs nothing but three.js r147 from the
CDN. Every size on the page is measured off the glb (the manifests), never typed. Mining Mike's rule
for model review: orbit it, at true relative scale, with a known object (the pilot, 1.8 m, and a
12 m human mech) beside it.

The animation is a reference for how cb-app can drive the alien rig (docs/design/alien-rig.md): the
legs step in two alternating groups (coxa swings about its own Y, femur lifts about its Z, the foot
counter-rotates so the sole stays level), the crab scuttles sideways, masts sway, turrets scan and
reactor cores spin. Fire runs every weapon the root extras name: cannons and guns recoil along their
own axis with a muzzle flash, missile racks ripple tube by tube, a sword winds up and slashes. Legs only ever lift from the rest pose, which stands on the
ground, so no foot goes under it.
"""
import base64
import json
import os

HUMAN = "foreman"   # the yardstick: our recommended chassis (build_mechs.py)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "alien_mechs.html")
ORDER = ["skitter", "snapper", "bigboy"]

PAGE = r"""<title>Alien Crab Mechs</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: the threat sheet of a hangar briefing. The stage is the page; the threat cards sit beside it. */
:root {
  --bg: #e9ebef; --panel: #f7f8fa; --edge: #c4cad3; --ink: #12161c; --muted: #56606d;
  --accent: #4f7a12; --accent-soft: #e3efd2; --warn: #7a3fc0; --stage: #1c1420;
  --display: "Saira Condensed", "Arial Narrow", sans-serif;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0d1015; --panel: #151a21; --edge: #2a323d; --ink: #e6ebf1; --muted: #94a0ad;
    --accent: #9cf04a; --accent-soft: #1f2c14; --warn: #c08cff; color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #0d1015; --panel: #151a21; --edge: #2a323d; --ink: #e6ebf1; --muted: #94a0ad;
  --accent: #9cf04a; --accent-soft: #1f2c14; --warn: #c08cff; color-scheme: dark;
}
* { box-sizing: border-box; }
body { background: var(--bg); color: var(--ink); font: 15px/1.5 var(--body); margin: 0;
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); padding-block: 18px 32px; }
header { display: grid; gap: 4px; margin: 0 auto 14px; max-width: 1400px; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; text-wrap: balance; }
.lede { margin: 0; color: var(--muted); max-width: 78ch; }
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
.card h2 small { font: 500 11px var(--mono); color: var(--warn); letter-spacing: 0.08em; text-transform: uppercase; }
.kv { display: grid; grid-template-columns: 9ch minmax(0, 1fr); gap: 2px 10px; font-size: 13px; margin: 0; }
.kv dt { color: var(--muted); font-family: var(--mono); font-size: 12px; padding-top: 1px; }
.kv dd { margin: 0; font-variant-numeric: tabular-nums; }
.note { font-size: 13px; color: var(--muted); margin: 0; }
.foot { max-width: 1400px; margin: 16px auto 0; font-size: 13px; color: var(--muted); display: grid; gap: 4px; }
.foot code { font: 12px var(--mono); overflow-wrap: anywhere; }
</style>

<header>
  <div class="eyebrow">Crab Breeze · enemy design review · the invaders</div>
  <h1>Alien crab mechs</h1>
  <p class="lede">Three alien war machines, each after one of the owner's crab reference sheets, built in Blender with Mining Mike's boss kit: plates, pistons, hoses, vents and livery bands. Like Mining Mike's bosses, where a crab would have claws each one carries its weapons: guns, a sword, cannons and missile racks. Acid green glows and violet cores and glyphs mark them as not ours. All at true scale beside Foreman, our 12 m mech, and a 1.8 m pilot.</p>
</header>

<div class="wrap">
  <section>
    <div class="bar" role="group" aria-label="Alien">
      <span class="lbl">Alien</span>
      <button data-mech="skitter">Skitter</button>
      <button data-mech="snapper">Snapper</button>
      <button data-mech="bigboy">Big Boy</button>
      <button data-mech="all">Line up</button>
    </div>
    <div class="bar" role="group" aria-label="Move">
      <span class="lbl">Move</span>
      <button data-act="idle">Idle</button>
      <button data-act="scuttle">Scuttle</button>
      <button data-act="fire">Fire weapons</button>
    </div>
    <div class="bar" role="group" aria-label="View">
      <span class="lbl">View</span>
      <button data-view="orbit">Orbit</button>
      <button data-view="front">Front</button>
      <button data-view="side">Side</button>
      <button data-view="top">Top</button>
      <span class="lbl" style="margin-left:8px">Show</span>
      <button data-toggle="breeze" aria-pressed="true">Foreman</button>
      <button data-toggle="pilot" aria-pressed="true">Pilot</button>
    </div>
    <div class="stage">
      <canvas id="view" tabindex="0" aria-label="3D view of the selected alien mech"></canvas>
      <div class="mode" id="mode">Orbit</div>
      <div id="tags"></div>
    </div>
    <p class="hint">Drag to orbit, scroll or pinch to zoom, right drag to pan.</p>
  </section>
  <aside id="cards"></aside>
</div>

<div class="foot">
  <div>Grid: 4 m cells. The yellow rail is 12 m, the height of a human mech. Sizes and triangle counts are measured off each glb by the generator, not typed.</div>
  <div>Models: <code>tools/blender/build_alien_mechs.py</code> on <code>tools/blender/mech_kit.py</code>; rig: <code>docs/design/alien-rig.md</code>; page: <code>tools/mockups/gen_alien_mechs.py</code>.</div>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__;
const BREEZE_META = __BREEZE_META__;
const GLB = __GLB__;
const ORDER = __ORDER__;
const LINEUP = ["bigboy", "snapper", "skitter"];
const T = THREE, TAU = Math.PI * 2;
const ease = x => (x <= 0 ? 0 : x >= 1 ? 1 : x * x * (3 - 2 * x));
// the frame each kind of weapon is held on for a still (MOCKUP.hold): a flash, a slash
const HOLD = { cannon: 0.03, smg: 0.0, rack: 0.62, blade: 0.7 };

const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = T.PCFSoftShadowMap;

// dusk, as the human mech page: violet sky, a low warm sun, a cool rim
const scene = new T.Scene();
scene.background = new T.Color(0x2a1f30);
scene.fog = new T.Fog(0x2a1f30, 120, 360);
const camera = new T.PerspectiveCamera(40, 1.6, 0.1, 900);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
Object.assign(controls, { minDistance: 4, maxDistance: 320 });
scene.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.55));
const sun = new T.DirectionalLight(0xffc89a, 2.0);
sun.position.set(-40, 50, 34); sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -70, right: 70, top: 70, bottom: -70, near: 1, far: 200 });
sun.shadow.bias = -0.0005;
scene.add(sun);
const rim = new T.DirectionalLight(0x9fb8ff, 0.9); rim.position.set(30, 20, -40); scene.add(rim);
// a dusk environment for reflections, so polished metal reads as metal and not as black
scene.environment = (() => {
  const es = new T.Scene(), geo = new T.SphereGeometry(50, 32, 16), col = [];
  const top = new T.Color(0x8a78a8), mid = new T.Color(0xd8a890), low = new T.Color(0x2a2420), c = new T.Color();
  for (let i = 0; i < geo.attributes.position.count; i++) { const y = geo.attributes.position.getY(i) / 50;
    if (y > 0.05) c.copy(mid).lerp(top, Math.min(1, (y - 0.05) * 1.6)); else c.copy(mid).lerp(low, Math.min(1, (0.05 - y) * 4)); col.push(c.r, c.g, c.b); }
  geo.setAttribute("color", new T.Float32BufferAttribute(col, 3));
  es.add(new T.Mesh(geo, new T.MeshBasicMaterial({ vertexColors: true, side: T.BackSide })));
  const panel = new T.Mesh(new T.PlaneGeometry(30, 12), new T.MeshBasicMaterial({ color: 0xfff0e0, side: T.DoubleSide }));
  panel.position.set(-30, 26, 24); panel.lookAt(0, 0, 0); es.add(panel);
  const pm = new T.PMREMGenerator(renderer); return pm.fromScene(es, 0.04).texture;
})();

const ground = new T.Mesh(new T.PlaneGeometry(900, 900), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const grid = new T.GridHelper(320, 80, 0x4a4552, 0x3a3640); grid.position.y = 0.01; scene.add(grid);
const rail = new T.Mesh(new T.BoxGeometry(200, 0.1, 0.1), new T.MeshBasicMaterial({ color: 0xf0b13c }));
rail.position.set(0, 12, -30); scene.add(rail);

const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a, roughness: 0.8 });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torsoM = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torsoM.position.y = 1.18; pilot.add(torsoM);
const head = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.66; head.scale.set(1, 1.15, 1); pilot.add(head);
[-1, 1].forEach(s => { const a = new T.Mesh(new T.CylinderGeometry(0.06, 0.045, 0.62, 8), suit); a.position.set(0.3 * s, 1.15, 0); a.rotation.z = 0.12 * s; pilot.add(a); });
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; });
scene.add(pilot);

const glowTex = (() => { const c = document.createElement("canvas"); c.width = c.height = 128; const q = c.getContext("2d");
  const g = q.createRadialGradient(64, 64, 0, 64, 64, 64); g.addColorStop(0, "rgba(255,255,255,1)"); g.addColorStop(0.18, "rgba(255,255,255,0.75)");
  g.addColorStop(0.45, "rgba(255,255,255,0.18)"); g.addColorStop(1, "rgba(255,255,255,0)"); q.fillStyle = g; q.fillRect(0, 0, 128, 128); return new T.CanvasTexture(c); })();

const loader = new T.GLTFLoader();
function b64ToBuf(b64) { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; }
const parseX = v => (typeof v === "string" ? JSON.parse(v) : v);
function load(id) { return new Promise(res => loader.parse(b64ToBuf(GLB[id]), "", g => {
  const root = g.scene, nodes = {}, cores = [];
  root.traverse(o => {
    if (o.name) nodes[o.name] = o;
    if (o.isMesh) { o.castShadow = true; o.receiveShadow = true;
      if (o.material && o.material.transparent) { o.material.depthWrite = false; o.castShadow = false; } }
    if (o.name.startsWith("CORE_")) cores.push(o);
  });
  root.traverse(o => { const u = o.userData || {}; if (u.halo_size_m) {
    const mat = new T.SpriteMaterial({ map: glowTex, color: new T.Color(u.halo_color), blending: T.AdditiveBlending, depthWrite: false, transparent: true, opacity: u.halo_opacity, toneMapped: false });
    const sp = new T.Sprite(mat); sp.scale.setScalar(u.halo_size_m); o.add(sp); } });
  const x = (nodes.root && nodes.root.userData) || {};
  const rest = {};
  for (const n in nodes) rest[n] = { q: nodes[n].quaternion.clone(), p: nodes[n].position.clone() };
  const weapons = parseX(x.weapons) || [];
  for (const W of weapons) {
    const r = rest[W.joint];
    W.axisP = new T.Vector3(...W.axis).applyQuaternion(r ? r.q : new T.Quaternion());
    W.flashes = W.muzzles.filter(m => nodes[m]).map(m => {
      const sp = new T.Sprite(new T.SpriteMaterial({ map: glowTex, color: 0xeaffa8, blending: T.AdditiveBlending, depthWrite: false,
        transparent: true, opacity: 0, toneMapped: false }));
      sp.position.z = W.flash_m * 0.35; sp.visible = false; nodes[m].add(sp); return sp; });
  }
  scene.add(root);
  res({ id, root, nodes, cores, rest, legs: parseX(x.legs) || [], gait: parseX(x.gait), idle: parseX(x.idle),
        arms: parseX(x.arms) || [], weapons, masts: parseX(x.masts) || [], t: 0, ph: 0, wS: 0, wF: 0, fireT: 0, baseX: 0 });
})); }

const aliens = {}; let breeze = null;
const loaded = Promise.all([...ORDER.map(id => load(id).then(a => { aliens[id] = a; })), load("breeze").then(b => { breeze = b; })]);

const H = id => META[id].bounds_m.y[1], W = id => META[id].bounds_m.x[1] - META[id].bounds_m.x[0];
let current = "snapper", view = "orbit", act = "idle", showBreeze = true, span = [-10, 10], hold = false;
const tagsEl = document.getElementById("tags");
function layout() {
  const all = current === "all";
  const bw = BREEZE_META.bounds_m.x[1] - BREEZE_META.bounds_m.x[0];
  if (all) {
    let x = -46;
    LINEUP.forEach(id => { const a = aliens[id]; if (!a) return; x += W(id) / 2; a.baseX = x; a.root.visible = true; a.root.position.set(x, 0, 0); x += W(id) / 2 + 6; });
    if (breeze) { breeze.root.position.set(x + bw / 2, 0, 4); }
    pilot.position.set(x - 1, 0, 9);
    span = [-46, x + bw + 2];
  } else {
    ORDER.forEach(id => { const a = aliens[id]; if (!a) return; a.root.visible = id === current; a.baseX = 0; a.root.position.set(0, 0, 0); });
    const off = W(current) / 2 + bw / 2 + Math.max(7, 14 - H(current));
    if (breeze) breeze.root.position.set(off, 0, 0);
    pilot.position.set(off - bw / 2 - 2, 0, Math.max(4, META[current].bounds_m.z[1] * 0.5));
    span = [-W(current) / 2, showBreeze ? off + bw / 2 : W(current) / 2];
  }
  if (breeze) breeze.root.visible = showBreeze;
  pilot.rotation.y = -0.6;
}
function frameFor() {
  const mid = (span[0] + span[1]) / 2, wide = span[1] - span[0];
  if (current === "all") return { tx: mid + wide * 0.08, ty: 7, d: wide * 0.92 };
  const h = H(current), b = META[current].bounds_m;
  return { tx: mid, ty: h * 0.42, d: Math.max(Math.max(h, b.z[1] - b.z[0]) * 1.7 + 6, wide * 1.15) };
}
function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
  document.getElementById("mode").textContent = { orbit: "Orbit", front: "Front", side: "Side", top: "Top" }[v];
  const f = frameFor();
  controls.target.set(f.tx, f.ty, 0);
  if (v === "front") camera.position.set(f.tx, f.ty + f.d * 0.12, f.d);
  else if (v === "side") camera.position.set(f.tx - f.d, f.ty + f.d * 0.12, 0);
  else if (v === "top") camera.position.set(f.tx, f.ty + f.d * 1.1, 0.01);
  else if (current === "all") camera.position.set(f.tx + f.d * 0.28, f.ty + f.d * 0.34, f.d * 0.95);
  else camera.position.set(f.tx + f.d * 0.62, f.ty + f.d * 0.38, f.d * 0.78);
  controls.update();
}
function setMech(id) {
  current = id;
  document.querySelectorAll("[data-mech]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.mech === id)));
  document.querySelectorAll(".card").forEach(c => c.classList.toggle("sel", c.dataset.id === id));
  layout(); setView(view);
}
function setAct(a) { act = a; document.querySelectorAll("[data-act]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.act === a))); }
document.querySelectorAll("[data-mech]").forEach(b => b.addEventListener("click", () => setMech(b.dataset.mech)));
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => setView(b.dataset.view)));
document.querySelectorAll("[data-act]").forEach(b => b.addEventListener("click", () => setAct(b.dataset.act)));
document.querySelectorAll("[data-toggle]").forEach(b => b.addEventListener("click", () => {
  const on = b.getAttribute("aria-pressed") !== "true"; b.setAttribute("aria-pressed", String(on));
  if (b.dataset.toggle === "breeze") { showBreeze = on; layout(); } if (b.dataset.toggle === "pilot") pilot.visible = on;
}));

const cards = document.getElementById("cards");
ORDER.forEach(id => {
  const m = META[id], b = m.bounds_m, el = document.createElement("article");
  el.className = "card"; el.dataset.id = id;
  el.innerHTML = `<h2>${m.name} <small>${m.role}</small></h2>
    <dl class="kv"><dt>Height</dt><dd>${b.y[1].toFixed(1)} m</dd><dt>Width</dt><dd>${(b.x[1] - b.x[0]).toFixed(1)} m</dd>
    <dt>Length</dt><dd>${(b.z[1] - b.z[0]).toFixed(1)} m</dd><dt>Legs</dt><dd>${m.legs}</dd>
    <dt>Weapons</dt><dd>${m.weapons}</dd><dt>After</dt><dd>${m.reference}</dd>
    <dt>Triangles</dt><dd>${m.triangles.toLocaleString("en-US")} (LOD0)</dd></dl>
    <p class="note">${m.note}</p>`;
  el.addEventListener("click", () => setMech(id));
  cards.appendChild(el);
});

// ---------- the scuttle: a reference for driving the alien rig ----------
const qd = new T.Quaternion(), ed = new T.Euler();
function turn(a, name, rx, ry, rz) {
  const n = a.nodes[name], r = a.rest[name]; if (!n) return;
  ed.set(rx, ry, rz, "XYZ"); qd.setFromEuler(ed); n.quaternion.copy(r.q).multiply(qd);
}
function animate(a, dt) {
  a.t += dt;
  a.wS += ((act === "scuttle" ? 1 : 0) - a.wS) * Math.min(1, dt * 3);
  a.wF += ((act === "fire" ? 1 : 0) - a.wF) * Math.min(1, dt * 5);
  if (!hold) a.fireT += dt;
  const g = a.gait, t = a.t;
  a.ph += dt * TAU * g.hz * a.wS;
  let lifted = 0;
  for (const L of a.legs) {
    const ph = a.ph + (L.group ? Math.PI : 0) + (L.side > 0 ? 0 : 0.3);
    const lift = g.lift * Math.max(0, Math.sin(ph)) * a.wS + 0.03 * (1 - a.wS) * Math.max(0, Math.sin(t * 0.9 + L.group * 2 + L.side));
    const swing = g.swing * Math.cos(ph) * a.wS;
    lifted += lift;
    turn(a, `leg_${L.id}_coxa`, 0, L.side * swing, 0);
    turn(a, `leg_${L.id}_femur`, 0, 0, lift);
    turn(a, `leg_${L.id}_tibia`, 0, 0, -0.45 * lift);
    turn(a, `leg_${L.id}_foot`, 0, 0, -0.55 * lift);
  }
  // the body rides up a little as legs lift, never down (that would push the planted feet into the ground)
  const body = a.nodes.body; if (body) { body.position.copy(a.rest.body.p); body.position.y += g.bob * a.wS * Math.abs(Math.sin(a.ph * 2)) * 0.5; }
  a.root.position.x = a.baseX + g.scuttle_m * Math.sin(a.ph / 4) * Math.min(1, a.wS * 1.2);
  // weapon arms: an idle sway; then every weapon at its own phase of its fire cycle
  const idle = a.idle;
  for (const S of a.arms) { const sd = S === "L" ? 1 : -1;
    turn(a, `arm_${S}`, 0, 0, 0.04 * Math.sin(t * 0.8 + sd)); turn(a, `wrist_${S}`, 0, 0, 0.03 * Math.sin(t * 1.1 + sd)); }
  a.weapons.forEach((W, wi) => fireWeapon(a, W, wi));
  for (const S of ["L", "R"]) turn(a, `mandible_${S}`, 0, (S === "L" ? 1 : -1) * 0.25 * Math.max(0, Math.sin(t * 5)), 0);
  for (const M of a.masts) turn(a, `mast_${M}`, 0.03 * Math.sin(t * 1.7 + M.length), 0, 0.04 * Math.sin(t * 1.3 + (M === "L" ? 0 : 1)));
  turn(a, "turret_1", 0, idle.scan * Math.sin(TAU * t / idle.scanPeriod), 0);
  for (const S of ["L", "R"]) turn(a, `pod_${S}`, -0.12 * (0.5 + 0.5 * Math.sin(t * 0.7)), 0, 0);
  for (const c of a.cores) c.rotation.y += dt * 1.4;
}

function fireWeapon(a, W, wi) {
  const P = W.period_s, tt = hold ? HOLD[W.kind] : (a.fireT + wi * 0.37 * P) % P, on = a.wF > 0.5 ? 1 : 0;
  const n = W.flashes.length, flash = new Array(n).fill(0);
  let recoil = 0, swing = 0;
  if (W.kind === "cannon") {
    if (tt < 0.14) flash.fill(1 - tt / 0.14);
    recoil = tt < 0.05 ? tt / 0.05 : Math.max(0, 1 - (tt - 0.05) / 0.7);
  } else if (W.kind === "smg") {
    if (tt < 1.2) { const shot = Math.floor(tt * 11), k = (tt * 11) % 1, f = k < 0.4 ? 1 - k / 0.4 : 0;
      for (let i = 0; i < n; i++) flash[i] = n === 1 || i === shot % n ? f : 0; recoil = 0.6 * f; }
  } else if (W.kind === "rack") {
    const slot = 1.1 / Math.max(1, n);
    for (let i = 0; i < n; i++) { const d = tt - i * slot; flash[i] = d >= 0 && d < 0.16 ? 1 - d / 0.16 : 0; }
    recoil = tt < 1.2 ? 0.5 * Math.max(0, ...flash) : 0;
  } else if (W.kind === "blade") {
    swing = tt < 0.6 ? 0.55 * ease(tt / 0.6) : tt < 0.8 ? 0.55 - 1.6 * ease((tt - 0.6) / 0.2) : -1.05 * (1 - ease((tt - 0.8) / 1.0));
  }
  const node = a.nodes[W.joint], rest = a.rest[W.joint];
  if (W.kind === "blade") turn(a, W.joint, 0, 0, swing * a.wF);
  else if (node && W.recoil_m > 0) node.position.copy(rest.p).addScaledVector(W.axisP, -W.recoil_m * recoil * a.wF);
  W.flashes.forEach((sp, i) => { const f = flash[i] * on; sp.material.opacity = f; sp.visible = f > 0.01; sp.scale.setScalar(W.flash_m * (0.6 + 0.6 * f)); });
}
function placeTags() {
  tagsEl.innerHTML = "";
  const r = canvas.getBoundingClientRect();
  const add = (p, text) => { const v = p.clone().project(camera); if (v.z > 1 || Math.abs(v.x) > 1.05 || Math.abs(v.y) > 1.05) return;
    const d = document.createElement("div"); d.className = "tag"; d.textContent = text;
    d.style.left = ((v.x + 1) / 2 * r.width) + "px"; d.style.top = ((1 - v.y) / 2 * r.height) + "px"; tagsEl.appendChild(d); };
  if (pilot.visible) add(new T.Vector3(pilot.position.x, 2.2, pilot.position.z), "pilot 1.8 m");
  if (breeze && breeze.root.visible) add(new T.Vector3(breeze.root.position.x, BREEZE_META.bounds_m.y[1] + 0.8, breeze.root.position.z), `Foreman ${BREEZE_META.bounds_m.y[1].toFixed(0)} m`);
  ORDER.forEach(id => { const a = aliens[id]; if (a && a.root.visible) add(new T.Vector3(a.root.position.x, H(id) + 0.8, 0), `${META[id].name} ${H(id).toFixed(0)} m`); });
}
function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight, pr = renderer.getPixelRatio();
  if (canvas.width !== Math.round(w * pr) || canvas.height !== Math.round(h * pr)) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
}
let last = 0, frame = 0;
function loop(ms) {
  const dt = Math.min(0.05, (ms - last) / 1000 || 0.016); last = ms;
  resize();
  for (const id in aliens) if (aliens[id].root.visible) animate(aliens[id], dt);
  if (breeze) for (const c of breeze.cores) c.rotation.y += dt * 1.4;
  controls.update(); renderer.render(scene, camera);
  if ((frame++ & 3) === 0) placeTags();
  requestAnimationFrame(loop);
}
loaded.then(() => { setMech("snapper"); setAct("idle"); setView("orbit"); window.MOCKUP_READY = true; });
requestAnimationFrame(loop);
window.MOCKUP = { setMech, setView, setAct, hold: on => { hold = !!on; } };
</script>
"""


def main():
    meta = json.load(open(os.path.join(MODELS, "aliens.json")))
    human = json.load(open(os.path.join(MODELS, "mechs.json")))
    glb = {}
    for key in ORDER:
        with open(os.path.join(MODELS, meta[key]["file"]), "rb") as f:
            glb[key] = base64.b64encode(f.read()).decode("ascii")
    with open(os.path.join(MODELS, human[HUMAN]["file"]), "rb") as f:
        glb["breeze"] = base64.b64encode(f.read()).decode("ascii")
    html = (PAGE.replace("__META__", json.dumps({k: meta[k] for k in ORDER}))
            .replace("__BREEZE_META__", json.dumps(human[HUMAN]))
            .replace("__GLB__", json.dumps(glb)).replace("__ORDER__", json.dumps(ORDER)))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
