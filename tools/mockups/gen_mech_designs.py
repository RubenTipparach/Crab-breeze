"""Build docs/mockups/mech_designs.html: the mech design candidates in an orbit viewer at true scale.

    python3 tools/mockups/gen_mech_designs.py

Reads assets/models/mech_*.glb and assets/models/mechs.json (written by tools/blender/build_mechs.py)
and inlines them, so the page needs nothing but three.js from the CDN. Every size on the page is
measured off the glb (the manifest), never typed. Mining Mike's rule for model review: orbit it, at
true relative scale, with a known object (the pilot, 1.8 m) and a rail at the mech's height.
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "mech_designs.html")

PAGE = r"""<title>Crab Breeze Mechs</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: a hangar review sheet. The stage is the page; the spec cards sit beside it like a clipboard. */
:root {
  --bg: #e9ecee; --panel: #f7f8f9; --edge: #c5ccd2; --ink: #11161b; --muted: #56616c;
  --accent: #1f5fbf; --accent-soft: #dbe6f7; --warn: #a86a00; --stage: #10161d;
  --display: "Saira Condensed", "Arial Narrow", sans-serif;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0c1014; --panel: #141a20; --edge: #2a333c; --ink: #e6ebf0; --muted: #93a0ac;
    --accent: #6ea8ff; --accent-soft: #1b2a40; --warn: #f0b13c; --stage: #10161d; color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #0c1014; --panel: #141a20; --edge: #2a333c; --ink: #e6ebf0; --muted: #93a0ac;
  --accent: #6ea8ff; --accent-soft: #1b2a40; --warn: #f0b13c; --stage: #10161d; color-scheme: dark;
}
* { box-sizing: border-box; }
body { background: var(--bg); color: var(--ink); font: 15px/1.5 var(--body); margin: 0;
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); padding-block: 18px 32px; }
header { display: grid; gap: 4px; margin-bottom: 14px; max-width: 1400px; margin-inline: auto; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; text-wrap: balance; }
.lede { margin: 0; color: var(--muted); max-width: 72ch; }
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
  color: #e6ebf0; background: rgba(12, 16, 20, 0.85); border: 1px solid #2a333c; padding: 2px 6px; white-space: nowrap; }
.mode { position: absolute; left: 10px; top: 10px; font: 700 13px var(--display); letter-spacing: 0.08em; text-transform: uppercase;
  color: #e6ebf0; background: rgba(12, 16, 20, 0.85); border: 1px solid #2a333c; padding: 5px 9px; pointer-events: none; }
.hint { font-size: 13px; color: var(--muted); margin: 6px 0 0; }
aside { display: grid; gap: 12px; min-width: 0; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 12px 14px; display: grid; gap: 6px; }
.card.sel { border-color: var(--accent); }
.card h2 { font: 700 24px/1 var(--display); letter-spacing: 0.05em; text-transform: uppercase; margin: 0; display: flex; gap: 8px; align-items: baseline; }
.card h2 small { font: 500 11px var(--mono); color: var(--warn); letter-spacing: 0.08em; }
.kv { display: grid; grid-template-columns: 9ch minmax(0, 1fr); gap: 2px 10px; font-size: 13px; }
.kv dt { color: var(--muted); font-family: var(--mono); font-size: 12px; padding-top: 1px; }
.kv dd { margin: 0; font-variant-numeric: tabular-nums; }
.note { font-size: 13px; color: var(--muted); margin: 0; }
.foot { max-width: 1400px; margin: 16px auto 0; font-size: 13px; color: var(--muted); display: grid; gap: 4px; }
.foot code { font: 12px var(--mono); }
@media (prefers-reduced-motion: reduce) { * { scroll-behavior: auto; } }
</style>

<header>
  <div class="eyebrow">Crab Breeze · mech design review · first pass</div>
  <h1>Pick a chassis</h1>
  <p class="lede">Three candidates, each built in Blender on the shared rig and shown at true scale beside a 1.8 m pilot and a four-storey block. Climb into the cockpit to see what you'd fight from. These are untextured design models (colours only); Material Maker textures come once a design is picked.</p>
</header>

<div class="wrap">
  <section>
    <div class="bar" role="group" aria-label="Mech">
      <span class="lbl">Mech</span>
      <button id="m-breeze" data-mech="breeze">Breeze</button>
      <button id="m-bulwark" data-mech="bulwark">Bulwark</button>
      <button id="m-hermit" data-mech="hermit">Hermit</button>
      <button id="m-all" data-mech="all">Line up</button>
    </div>
    <div class="bar" role="group" aria-label="View">
      <span class="lbl">View</span>
      <button id="v-orbit" data-view="orbit">Orbit</button>
      <button id="v-cockpit" data-view="cockpit">Cockpit</button>
      <button id="v-front" data-view="front">Front</button>
      <button id="v-side" data-view="side">Side</button>
      <span class="lbl" style="margin-left:8px">Show</span>
      <button id="t-city" data-toggle="city" aria-pressed="true">Block</button>
      <button id="t-pilot" data-toggle="pilot" aria-pressed="true">Pilot</button>
      <button id="t-rig" data-toggle="rig" aria-pressed="false">Rig test</button>
    </div>
    <div class="stage">
      <canvas id="view" tabindex="0" aria-label="3D view of the selected mech"></canvas>
      <div class="mode" id="mode">Orbit</div>
      <div id="tags"></div>
    </div>
    <p class="hint">Drag to orbit, scroll or pinch to zoom, right drag to pan. In the cockpit, drag to look around. Rig test drives the contract's bones (torso_twist, thigh, shin, hock) the way the engine will.</p>
  </section>
  <aside id="cards"></aside>
</div>

<div class="foot">
  <div>Grid: 4 m cells. The yellow rail is 12 m (survey M1). Sizes and triangle counts are measured off each glb by the generator, not typed.</div>
  <div>Source: <code>tools/blender/build_mechs.py</code> on <code>tools/blender/mech_kit.py</code>; rig: <code>docs/design/mech-rig.md</code>; page: <code>tools/mockups/gen_mech_designs.py</code>.</div>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__;
const GLB = __GLB__;
const ORDER = ["breeze", "bulwark", "hermit"];

const canvas = document.getElementById("view");
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = THREE.sRGBEncoding;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 0.8;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x1a222c);
scene.fog = new THREE.Fog(0x1a222c, 70, 180);
const camera = new THREE.PerspectiveCamera(42, 1.6, 0.05, 400);
const controls = new THREE.OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.target.set(0, 6, 0);
camera.position.set(17, 9, 19);

scene.add(new THREE.HemisphereLight(0xbfd4ee, 0x2b2620, 0.45));
const sun = new THREE.DirectionalLight(0xffe2c0, 1.25);
sun.position.set(-18, 30, 16);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -30, right: 30, top: 30, bottom: -30, near: 1, far: 90 });
scene.add(sun);
const rim = new THREE.DirectionalLight(0x7fb4ff, 0.35);
rim.position.set(14, 10, -20);
scene.add(rim);

// ground: asphalt with a 4 m grid, the real scale ruler
const ground = new THREE.Mesh(new THREE.PlaneGeometry(400, 400), new THREE.MeshStandardMaterial({ color: 0x262b31, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2;
ground.receiveShadow = true;
scene.add(ground);
const grid = new THREE.GridHelper(160, 40, 0x58626e, 0x48515b);
grid.position.y = 0.01;
scene.add(grid);

// the 12 m rail
const rail = new THREE.Mesh(new THREE.BoxGeometry(70, 0.08, 0.08), new THREE.MeshBasicMaterial({ color: 0xf0b13c }));
rail.position.set(0, 12, -14);
scene.add(rail);

// a four-storey block (3.5 m a storey) with windows: the city a mech walks through
const city = new THREE.Group();
function block(x, z, w, d, storeys, col) {
  const h = storeys * 3.5;
  const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), new THREE.MeshStandardMaterial({ color: col, roughness: 0.85 }));
  m.position.set(x, h / 2, z); m.castShadow = m.receiveShadow = true; city.add(m);
  const win = new THREE.MeshStandardMaterial({ color: 0x223040, emissive: 0xffc878, emissiveIntensity: 0.25, roughness: 0.3 });
  for (let s = 0; s < storeys; s++) for (let i = 0; i < Math.floor(w / 2.6); i++) {
    const g = new THREE.Mesh(new THREE.BoxGeometry(1.4, 1.6, 0.1), win);
    g.position.set(x - w / 2 + 1.4 + i * 2.6, s * 3.5 + 2.0, z + d / 2 + 0.05); city.add(g);
  }
}
block(-18, -6, 14, 10, 4, 0x5e5852);
block(-31, -8, 10, 12, 9, 0x4b5057);
block(18, -10, 12, 10, 2, 0x6a604f);
const paint = new THREE.MeshStandardMaterial({ color: 0x7a1f1f, roughness: 0.35, metalness: 0.3 });
const carBody = new THREE.Mesh(new THREE.BoxGeometry(1.8, 0.75, 4.4), paint); carBody.position.set(-13, 0.55, 6);
const carCab = new THREE.Mesh(new THREE.BoxGeometry(1.6, 0.6, 2.2), new THREE.MeshStandardMaterial({ color: 0x1d2630, roughness: 0.2 })); carCab.position.set(-13, 1.2, 5.8);
[carBody, carCab].forEach(m => { m.castShadow = true; city.add(m); });
scene.add(city);

// the pilot, 1.8 m, tapered not boxy
const pilot = new THREE.Group();
const suit = new THREE.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 });
const skin = new THREE.MeshStandardMaterial({ color: 0xd8b49a, roughness: 0.8 });
[[-0.13, 0.42], [0.13, 0.42]].forEach(([x]) => { const l = new THREE.Mesh(new THREE.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torsoM = new THREE.Mesh(new THREE.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torsoM.position.y = 1.18; pilot.add(torsoM);
const head = new THREE.Mesh(new THREE.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.66; head.scale.set(1, 1.15, 1); pilot.add(head);
[-1, 1].forEach(s => { const a = new THREE.Mesh(new THREE.CylinderGeometry(0.06, 0.045, 0.62, 8), suit); a.position.set(0.3 * s, 1.15, 0); a.rotation.z = 0.12 * s; pilot.add(a); });
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; });
scene.add(pilot);

// load every mech once
const loader = new THREE.GLTFLoader();
const mechs = {};
function b64ToBuf(b64) { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; }
const loaded = Promise.all(ORDER.map(id => new Promise(res => loader.parse(b64ToBuf(GLB[id]), "", g => {
  const root = g.scene;
  root.traverse(o => {
    if (o.isMesh) { o.castShadow = true; o.receiveShadow = true;
      if (o.material && o.material.transparent) { o.material.depthWrite = false; o.castShadow = false; o.renderOrder = 2; } }
  });
  const bones = {};
  root.traverse(o => { if (o.name) bones[o.name] = o; });
  const rest = {};
  for (const k in bones) rest[k] = bones[k].quaternion.clone();
  mechs[id] = { root, bones, rest };
  scene.add(root);
  res();
}))));

let current = "breeze", view = "orbit", rigOn = false;
const tagsEl = document.getElementById("tags");

function layout() {
  const spread = current === "all";
  ORDER.forEach((id, i) => {
    const m = mechs[id]; if (!m) return;
    m.root.visible = spread || id === current;
    m.root.position.set(spread ? (i - 1) * 13 : 0, 0, 0);
  });
  pilot.position.set(spread ? -6.5 : 4.2, 0, 5);
  pilot.rotation.y = -0.6;
}

function eyeOf(id) {
  const e = mechs[id] && mechs[id].bones.EYE_pilot;
  const p = new THREE.Vector3();
  if (e) e.getWorldPosition(p);
  return p;
}

function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
  document.getElementById("mode").textContent = { orbit: "Orbit", cockpit: "Cockpit, pilot's eye", front: "Front", side: "Side" }[v];
  const id = current === "all" ? "breeze" : current;
  if (v === "cockpit") {
    if (current === "all") setMech("breeze", true);
    const e = eyeOf(current);
    camera.fov = 70; camera.near = 0.02; camera.updateProjectionMatrix();
    camera.position.copy(e);
    controls.target.copy(e).add(new THREE.Vector3(0, -0.15, 1));
    controls.minDistance = 0.01; controls.maxDistance = 1.0; controls.enablePan = false; controls.enableZoom = false;
    controls.rotateSpeed = -0.35;
  } else {
    camera.fov = 42; camera.near = 0.05; camera.updateProjectionMatrix();
    controls.minDistance = 4; controls.maxDistance = 120; controls.enablePan = true; controls.enableZoom = true; controls.rotateSpeed = 1;
    const wide = current === "all";
    const d = wide ? 40 : 24;
    controls.target.set(0, 6, 0);
    if (v === "front") camera.position.set(0, 7, d);
    else if (v === "side") camera.position.set(d, 7, 0);
    else camera.position.set(d * 0.7, 9, d * 0.8);
  }
  controls.update();
}

function setMech(id, keepView) {
  current = id;
  document.querySelectorAll("[data-mech]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.mech === id)));
  document.querySelectorAll(".card").forEach(c => c.classList.toggle("sel", c.dataset.id === id));
  layout();
  if (!keepView) setView(view === "cockpit" && id === "all" ? "orbit" : view);
}

document.querySelectorAll("[data-mech]").forEach(b => b.addEventListener("click", () => setMech(b.dataset.mech)));
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => setView(b.dataset.view)));
document.querySelectorAll("[data-toggle]").forEach(b => b.addEventListener("click", () => {
  const on = b.getAttribute("aria-pressed") !== "true";
  b.setAttribute("aria-pressed", String(on));
  if (b.dataset.toggle === "city") city.visible = on;
  if (b.dataset.toggle === "pilot") pilot.visible = on;
  if (b.dataset.toggle === "rig") { rigOn = on; if (!on) resetRig(); }
}));

// spec cards, every number from the manifest
const cards = document.getElementById("cards");
ORDER.forEach(id => {
  const m = META[id];
  const b = m.bounds_m;
  const el = document.createElement("article");
  el.className = "card"; el.dataset.id = id;
  el.innerHTML = `<h2>${m.name}${id === "breeze" ? " <small>RECOMMENDED</small>" : ""}</h2>
    <dl class="kv">
      <dt>Height</dt><dd>${b.y[1].toFixed(1)} m</dd>
      <dt>Width</dt><dd>${(b.x[1] - b.x[0]).toFixed(1)} m</dd>
      <dt>Length</dt><dd>${(b.z[1] - b.z[0]).toFixed(1)} m</dd>
      <dt>Legs</dt><dd>${m.legs}</dd>
      <dt>Cockpit</dt><dd>${m.cockpit}</dd>
      <dt>Weapons</dt><dd>${m.weapons}</dd>
      <dt>Triangles</dt><dd>${m.triangles.toLocaleString("en-US")} (LOD0)</dd>
    </dl>
    <p class="note">${m.note}</p>`;
  el.addEventListener("click", () => setMech(id));
  cards.appendChild(el);
});

// rig test: a procedural stride on the contract's bones, the same way cb-app drives them
const Q = new THREE.Quaternion(), X = new THREE.Vector3(1, 0, 0), Y = new THREE.Vector3(0, 1, 0);
function turn(m, name, axis, ang) {
  const b = m.bones[name]; if (!b) return;
  b.quaternion.copy(m.rest[name]).multiply(Q.setFromAxisAngle(axis, ang));
}
function resetRig() { for (const id in mechs) { const m = mechs[id]; for (const k in m.rest) m.bones[k].quaternion.copy(m.rest[k]); if (m.bones.pelvis) m.bones.pelvis.position.y = m.bones.pelvis.userData.y0 ?? m.bones.pelvis.position.y; } }
function animate(t) {
  for (const id in mechs) {
    const m = mechs[id];
    if (!rigOn || !m.root.visible) continue;
    const ph = t * 2.2;
    turn(m, "torso_twist", Y, Math.sin(t * 0.7) * 0.5);
    [["L", 0], ["R", Math.PI]].forEach(([S, o]) => {
      const s = Math.sin(ph + o);
      turn(m, `thigh_${S}`, X, s * 0.32);
      turn(m, `shin_${S}`, X, -Math.max(0, -Math.cos(ph + o)) * 0.45);
      turn(m, `hock_${S}`, X, Math.max(0, -Math.cos(ph + o)) * 0.35);
      turn(m, `upper_arm_${S}`, X, -s * 0.12);
    });
    const pv = m.bones.pelvis;
    if (pv) { pv.userData.y0 ??= pv.position.y; pv.position.y = pv.userData.y0 - Math.abs(Math.sin(ph)) * 0.12; }
  }
}

function placeTags() {
  tagsEl.innerHTML = "";
  if (view === "cockpit") return;
  const r = canvas.getBoundingClientRect();
  const add = (p, text) => {
    const v = p.clone().project(camera);
    if (v.z > 1) return;
    const d = document.createElement("div"); d.className = "tag"; d.textContent = text;
    d.style.left = ((v.x + 1) / 2 * r.width) + "px"; d.style.top = ((1 - v.y) / 2 * r.height) + "px";
    tagsEl.appendChild(d);
  };
  if (pilot.visible) add(new THREE.Vector3(pilot.position.x, 2.1, pilot.position.z), "pilot 1.8 m");
  if (city.visible) { add(new THREE.Vector3(-18, 14.6, -1), "4 storeys, 14 m"); add(new THREE.Vector3(-13, 2.2, 6), "car 4.4 m"); }
  add(new THREE.Vector3(30, 12.4, -14), "12 m");
}

function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.round(w * renderer.getPixelRatio()) || canvas.height !== Math.round(h * renderer.getPixelRatio())) {
    renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix();
  }
}
let frame = 0;
function loop(ms) {
  resize();
  animate(ms / 1000);
  if (view === "cockpit") {
    // keep the eye on EYE_pilot while the rig moves under it
    const e = eyeOf(current), dlt = e.clone().sub(camera.position);
    camera.position.add(dlt); controls.target.add(dlt);
  }
  controls.update();
  renderer.render(scene, camera);
  if ((frame++ & 3) === 0) placeTags();
  requestAnimationFrame(loop);
}
loaded.then(() => { setMech("breeze"); setView("orbit"); window.MOCKUP_READY = true; });
setMech("breeze", true);
requestAnimationFrame(loop);
window.MOCKUP = { setMech, setView };
</script>
"""


def main():
    meta = json.load(open(os.path.join(MODELS, "mechs.json")))
    glb = {}
    for key in meta:
        with open(os.path.join(MODELS, meta[key]["file"]), "rb") as f:
            glb[key] = base64.b64encode(f.read()).decode("ascii")
    html = PAGE.replace("__META__", json.dumps(meta)).replace("__GLB__", json.dumps(glb))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
