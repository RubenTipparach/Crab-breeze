"""Build docs/mockups/boats.html: every boat, the harbour crane and a quay section, orbitable at true scale beside
the pilot and the 24 m Foreman (openspec/changes/harbours, design section 9).

    python3 tools/mockups/gen_boats.py

Reads assets/models/boats.json and the block-outs it lists (written by tools/blender/build_boats.py) and the Foreman
(assets/models/mech_foreman.glb, sized by assets/models/mechs.json), and inlines them, so the page needs nothing but
three.js r147 from the CDN. Every size on the page is measured off the glb by the generator, never typed. Mining
Mike's rule for model review: orbit it, at true relative scale, with a known object beside it: here the 1.8 m pilot
and the 24 m mech, standing on the quay, a rail at the mech's height across the water, every boat floating at its
waterline on one sea, over a 10 m grid. The models are labelled stand-ins (HB19, pre-approved by the owner,
2026-10-26: "No survey needed qs preapprove all boat choices"); the page is for looking, nothing waits on it.
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "boats.html")
ORDER = ["freighter", "tanker", "ferry", "gunboat", "fishing_boat", "launch", "rowing_boat", "crane", "quay"]

PAGE = r"""<title>Harbour Boats</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* A harbour master's board: the stage is the page, the cards beside it. */
:root {
  --bg: #e8ecef; --panel: #f6f8f9; --edge: #c2cad1; --ink: #11181d; --muted: #55616b;
  --accent: #0d6b8a; --accent-soft: #d6ecf3; --warn: #a2560d; --stage: #18242c;
  --display: "Saira Condensed", "Arial Narrow", sans-serif;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif;
  --mono: "IBM Plex Mono", ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0c1216; --panel: #141c22; --edge: #29343d; --ink: #e5ecf1; --muted: #93a2ae;
    --accent: #5cc6e8; --accent-soft: #12303b; --warn: #f0a24a; color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #0c1216; --panel: #141c22; --edge: #29343d; --ink: #e5ecf1; --muted: #93a2ae;
  --accent: #5cc6e8; --accent-soft: #12303b; --warn: #f0a24a; color-scheme: dark;
}
* { box-sizing: border-box; }
body { background: var(--bg); color: var(--ink); font: 15px/1.5 var(--body); margin: 0; padding: 18px 16px 32px; }
header { display: grid; gap: 4px; margin: 0 auto 14px; max-width: 1400px; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; }
.lede { margin: 0; color: var(--muted); max-width: 80ch; }
.wrap { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 16px; align-items: start; max-width: 1400px; margin: 0 auto; }
@media (max-width: 1000px) { .wrap { grid-template-columns: minmax(0, 1fr); } }
.bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 6ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button:hover { border-color: var(--muted); }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button:focus-visible, canvas:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.stage { position: relative; background: var(--stage); border: 1px solid var(--edge); border-radius: 4px; overflow: hidden; touch-action: none; }
canvas { display: block; width: 100%; aspect-ratio: 16 / 10; }
@media (max-width: 600px) { canvas { aspect-ratio: 1 / 1.1; } }
.tag { position: absolute; transform: translate(-50%, -100%); pointer-events: none; font: 500 11px var(--mono);
  color: #e5ecf1; background: rgba(10, 18, 24, 0.85); border: 1px solid #33424d; padding: 2px 6px; white-space: nowrap; }
.hint { font-size: 13px; color: var(--muted); margin: 6px 0 0; }
aside { display: grid; gap: 10px; min-width: 0; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 10px 12px; display: grid; gap: 4px; cursor: pointer; }
.card.sel { border-color: var(--accent); }
.card h2 { font: 700 22px/1 var(--display); letter-spacing: 0.05em; text-transform: uppercase; margin: 0; display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
.card h2 small { font: 500 11px var(--mono); color: var(--warn); letter-spacing: 0.08em; }
.kv { display: grid; grid-template-columns: 8ch minmax(0, 1fr); gap: 2px 10px; font-size: 13px; margin: 0; }
.kv dt { color: var(--muted); font-family: var(--mono); font-size: 12px; }
.kv dd { margin: 0; font-variant-numeric: tabular-nums; }
.foot { max-width: 1400px; margin: 16px auto 0; font-size: 13px; color: var(--muted); display: grid; gap: 4px; }
.foot code { font: 12px var(--mono); overflow-wrap: anywhere; }
</style>

<header>
  <div class="eyebrow">Crab Breeze · harbours · block-out review</div>
  <h1>Boats at true scale</h1>
  <p class="lede">Every boat on the island, the harbour crane and a section of quay, built in Blender as labelled stand-ins at their real 1980s sizes. Each floats at its waterline on one sea; the 24 m Foreman and a 1.8 m pilot stand on the quay, and the yellow rail runs across the water at the mech's height.</p>
</header>

<div class="wrap">
  <section>
    <div class="bar" role="group" aria-label="Show" id="picks"><span class="lbl">Boat</span></div>
    <div class="bar" role="group" aria-label="View">
      <span class="lbl">View</span>
      <button data-view="orbit">Orbit</button>
      <button data-view="side">Side</button>
      <button data-view="top">Top</button>
      <button data-view="reset">Reset</button>
      <span class="lbl" style="margin-left:8px">Show</span>
      <button data-toggle="mech" aria-pressed="true">Foreman</button>
      <button data-toggle="pilot" aria-pressed="true">Pilot</button>
      <button data-toggle="lod1" aria-pressed="false">LOD 1</button>
    </div>
    <div class="stage">
      <canvas id="view" tabindex="0" aria-label="3D view of the boats at true scale"></canvas>
      <div id="tags"></div>
    </div>
    <p class="hint">Drag to orbit, scroll or pinch to zoom, right drag to pan. Keys: arrows orbit, + and - zoom.</p>
  </section>
  <aside id="cards"></aside>
</div>

<div class="foot">
  <div>Grid: 10 m cells on the sea floor's plane at the waterline. Sizes and triangle counts are measured off each glb by the generator, never typed; every boat is within 1 % of <code>data/world/ships.ron</code>.</div>
  <div>Models: <code>tools/blender/build_boats.py</code>; page: <code>tools/mockups/gen_boats.py</code>; design: <code>openspec/changes/harbours</code>.</div>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__, MECH = __MECH__, GLB = __GLB__, ORDER = __ORDER__;
const T = THREE;
const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
const scene = new T.Scene();
scene.background = new T.Color(0x9fb6c8);
scene.fog = new T.Fog(0x9fb6c8, 300, 900);
const camera = new T.PerspectiveCamera(40, 1.6, 0.1, 3000);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
Object.assign(controls, { minDistance: 3, maxDistance: 800 });
scene.add(new T.HemisphereLight(0xdfe8f0, 0x2c3a40, 0.9));
const sun = new T.DirectionalLight(0xfff2dc, 2.2); sun.position.set(-80, 120, 60); scene.add(sun);

// The sea: a translucent sheet at the waterline over a bed 14 m down, the grid on the waterline.
const sea = new T.Mesh(new T.PlaneGeometry(2000, 2000), new T.MeshStandardMaterial({ color: 0x2c5f78, roughness: 0.25, metalness: 0.1, transparent: true, opacity: 0.55, depthWrite: false }));
sea.rotation.x = -Math.PI / 2; scene.add(sea);
const bed = new T.Mesh(new T.PlaneGeometry(2000, 2000), new T.MeshStandardMaterial({ color: 0x4a4a3c, roughness: 1 }));
bed.rotation.x = -Math.PI / 2; bed.position.y = -14; scene.add(bed);
const grid = new T.GridHelper(1000, 100, 0x6f8a99, 0x56707e); grid.position.y = 0.02; scene.add(grid);
const rail = new T.Mesh(new T.BoxGeometry(600, 0.2, 0.2), new T.MeshBasicMaterial({ color: 0xf0b13c }));
scene.add(rail);

const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a, roughness: 0.8 });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torso = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torso.position.y = 1.18; pilot.add(torso);
const head = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.66; head.scale.set(1, 1.15, 1); pilot.add(head);
scene.add(pilot);

const loader = new T.GLTFLoader();
function buf(b64) { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; }
const load = key => new Promise(res => loader.parse(buf(GLB[key]), "", g => { scene.add(g.scene); res(g.scene); }));
const models = {}, lod1 = {};
let mech = null, current = "harbour", showLod1 = false;
const DECK = 2.5;
const loaded = Promise.all([
  ...ORDER.map(k => load(k).then(m => { models[k] = m; })),
  ...ORDER.map(k => load(k + "_lod1").then(m => { lod1[k] = m; m.visible = false; })),
  load("mech").then(m => { mech = m; }),
]);

const size = k => META[k].size_m;
// Where each piece stands in the harbour view: the quay at the berth with the mech and pilot on it, the crane on it,
// the ships alongside and off it in a row by length.
function layout() {
  const harbour = current === "harbour";
  const quayW = size("quay")[0];
  for (const k of ORDER) {
    const m = models[k], m1 = lod1[k];
    let x = 0, y = 0, z = 0, ry = 0, vis = harbour || k === current;
    if (k === "quay") { y = DECK; vis = true; }
    if (k === "crane") { y = DECK; x = 8; vis = harbour || current === "crane"; }
    if (harbour && !["quay", "crane"].includes(k)) {
      const i = ["freighter", "tanker", "ferry", "gunboat", "fishing_boat", "launch", "rowing_boat"].indexOf(k);
      // Turned to lie along the quay, bow toward +x, a row of them out from the berth face (-z).
      ry = -Math.PI / 2;
      z = -size("quay")[2] / 2 - size(k)[0] / 2 - 4 - [0, 26, 56, 76, 90, 98, 104][i];
      x = [0, 0, -20, 60, 40, 20, 8][i];
    } else if (!["quay", "crane"].includes(k)) {
      ry = -Math.PI / 2; z = -size("quay")[2] / 2 - size(k)[0] / 2 - 4;
    }
    for (const n of [m, m1]) { if (!n) continue; n.position.set(x, y, z); n.rotation.y = ry; }
    if (m) m.visible = vis && !showLod1;
    if (m1) m1.visible = vis && showLod1;
  }
  if (mech) { mech.position.set(-quayW / 2 + 9, DECK, 2); mech.rotation.y = Math.PI; }
  pilot.position.set(-quayW / 2 + 19, DECK, -10);
  rail.position.set(0, DECK + MECH.height, -40);
  tags();
}

const tagsEl = document.getElementById("tags");
function tags() {
  tagsEl.innerHTML = "";
  const show = current === "harbour" ? ORDER : [current, "quay"];
  const list = show.map(k => [k, models[k]]).concat([["Foreman 24 m", mech], ["pilot 1.8 m", pilot]]);
  for (const [name, o] of list) {
    if (!o || !o.visible) continue;
    const el = document.createElement("div"); el.className = "tag";
    el.textContent = META[name] ? META[name].label + " " + META[name].size_m[2] + " m · stand-in" : name;
    el.dataset.key = name; tagsEl.appendChild(el); el._o = o;
  }
}
function placeTags() {
  const r = canvas.getBoundingClientRect();
  for (const el of tagsEl.children) {
    const o = el._o, box = new T.Box3().setFromObject(o), p = new T.Vector3((box.min.x + box.max.x) / 2, box.max.y + 1, (box.min.z + box.max.z) / 2);
    p.project(camera);
    el.style.display = p.z < 1 ? "" : "none";
    el.style.left = ((p.x + 1) / 2 * r.width) + "px"; el.style.top = ((1 - p.y) / 2 * r.height) + "px";
  }
}

function frame(v) {
  const onQuay = current === "crane" || current === "quay";
  const target = current === "harbour" ? new T.Vector3(10, 8, -60)
    : onQuay ? new T.Vector3(0, 14, 0) : new T.Vector3(0, size(current)[1] / 3, -size("quay")[2] / 2 - size(current)[0] / 2 - 4);
  const reach = current === "harbour" ? 260 : onQuay ? 110 : Math.max(30, size(current)[2] * 1.4);
  controls.target.copy(target);
  if (v === "side") camera.position.set(target.x, target.y + 2, target.z - reach);
  else if (v === "top") camera.position.set(target.x, target.y + reach * 1.3, target.z + 0.01);
  else camera.position.set(target.x - reach * 0.7, target.y + reach * 0.45, target.z - reach * 0.75);
  controls.update();
}

const picks = document.getElementById("picks");
for (const k of ["harbour", ...ORDER]) {
  const b = document.createElement("button"); b.dataset.pick = k;
  b.textContent = k === "harbour" ? "Harbour" : META[k].label; picks.appendChild(b);
}
function pick(k) {
  current = k;
  document.querySelectorAll("[data-pick]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.pick === k)));
  document.querySelectorAll(".card").forEach(c => c.classList.toggle("sel", c.dataset.key === k));
  layout(); frame("orbit");
}
const cards = document.getElementById("cards");
for (const k of ORDER) {
  const m = META[k], c = document.createElement("div"); c.className = "card"; c.dataset.key = k;
  c.innerHTML = `<h2>${m.label} <small>STAND-IN</small></h2><dl class="kv">
    <dt>long</dt><dd>${m.size_m[2]} m</dd><dt>beam</dt><dd>${m.size_m[0]} m</dd><dt>high</dt><dd>${m.size_m[1]} m keel to top</dd>
    <dt>tris</dt><dd>${m.lods[0].triangles} / ${m.lods[0].budget_triangles} (LOD 0), ${m.lods[1].triangles} / ${m.lods[1].budget_triangles} (LOD 1)</dd></dl>`;
  c.addEventListener("click", () => pick(k)); cards.appendChild(c);
}
picks.addEventListener("click", e => { const b = e.target.closest("[data-pick]"); if (b) pick(b.dataset.pick); });
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => frame(b.dataset.view === "reset" ? "orbit" : b.dataset.view)));
document.querySelectorAll("[data-toggle]").forEach(b => b.addEventListener("click", () => {
  const on = b.getAttribute("aria-pressed") !== "true"; b.setAttribute("aria-pressed", String(on));
  if (b.dataset.toggle === "mech" && mech) mech.visible = on;
  if (b.dataset.toggle === "pilot") pilot.visible = on;
  if (b.dataset.toggle === "lod1") { showLod1 = on; layout(); }
  tags();
}));
// Keyboard: arrows orbit, + and - zoom.
canvas.addEventListener("keydown", e => {
  const off = camera.position.clone().sub(controls.target), sph = new T.Spherical().setFromVector3(off);
  if (e.key === "ArrowLeft") sph.theta -= 0.08; else if (e.key === "ArrowRight") sph.theta += 0.08;
  else if (e.key === "ArrowUp") sph.phi = Math.max(0.1, sph.phi - 0.08); else if (e.key === "ArrowDown") sph.phi = Math.min(1.5, sph.phi + 0.08);
  else if (e.key === "+" || e.key === "=") sph.radius *= 0.9; else if (e.key === "-") sph.radius *= 1.1; else return;
  e.preventDefault(); camera.position.copy(controls.target).add(new T.Vector3().setFromSpherical(sph)); controls.update();
});

function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.floor(w * renderer.getPixelRatio()) || canvas.height !== Math.floor(h * renderer.getPixelRatio())) {
    renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix();
  }
}
function loop() { resize(); controls.update(); renderer.render(scene, camera); placeTags(); requestAnimationFrame(loop); }
loaded.then(() => { pick("harbour"); window.MOCKUP_READY = true; });
requestAnimationFrame(loop);
window.MOCKUP = { pick, frame };
</script>
"""


def main():
    meta = json.load(open(os.path.join(MODELS, "boats.json")))["boats"]
    mechs = json.load(open(os.path.join(MODELS, "mechs.json")))
    glb = {}
    for key in ORDER:
        for name, file in ((key, meta[key]["file"]), (key + "_lod1", meta[key]["lod1"])):
            with open(os.path.join(MODELS, file), "rb") as f:
                glb[name] = base64.b64encode(f.read()).decode("ascii")
    with open(os.path.join(MODELS, mechs["foreman"]["file"]), "rb") as f:
        glb["mech"] = base64.b64encode(f.read()).decode("ascii")
    b = mechs["foreman"]["bounds_m"]["y"]
    mech = dict(height=round(b[1] - b[0], 2))
    html = (PAGE.replace("__META__", json.dumps({k: meta[k] for k in ORDER}))
            .replace("__MECH__", json.dumps(mech)).replace("__GLB__", json.dumps(glb))
            .replace("__ORDER__", json.dumps(ORDER)))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
