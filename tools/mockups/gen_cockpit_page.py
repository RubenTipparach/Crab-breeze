"""The cockpit review page: the three cockpit interiors on their own, seen from the pilot's eye or orbited.

The owner, 2026-10-24: "Where is the new published cockpit", "wheres the new cockpit?". The mech page shows each
interior inside its chassis; this page is the interiors alone, the way a pilot meets them: sit at the eye and
look round the screen wall (the world outside on the screens), open and close the parts that let the pilot in,
or orbit the pod beside a 1.8 m pilot. Every number on it is read off the interiors' manifests
(assets/models/cockpit_<variant>.json), not typed.

    python3 tools/mockups/gen_cockpit_page.py
    python3 -m http.server 8768 -d output/cockpit_site

writes docs/mockups/cockpits.html and output/cockpit_site (the page, the glTF JSON and its textures, and
files.json, the publish map). The models load through tools/mockups/gltf_json_loader.js and the screens are
tools/mockups/screen_wall.js, both shared with the mech page; the steam at the seals is tools/mockups/steam.js.
"""
import base64
import json
import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_mech_designs as MD  # noqa: E402

ROOT = MD.ROOT
OUT = os.path.join(ROOT, "docs", "mockups", "cockpits.html")
SITE = os.path.join(ROOT, "output", "cockpit_site")
VARIANTS = [("wall", "foreman", "Wall", "Seated in a cab of flat screens: a wide front screen, chamfered corner screens, "
             "side screens, three monitors hanging from the ceiling. The front and the corner screens swing out and "
             "the sill folds down as the step."),
            ("harness", "stevedore", "Harness", "Set aside: every cockpit is seated now (owner, 2026-10-24). Standing in a tall octagon of screens, strapped to a backplate on a "
             "spine, forearms on two rests that end in the grips, a ring of monitors overhead. The front facet lifts, "
             "the front diagonals swing and the rests fold up to let the pilot in."),
            ("jockey", "ironhorse", "Jockey", "Set aside: every cockpit is seated now (owner, 2026-10-24). Riding it like a motorbike: leaning onto a chest pad, knees in pads, "
             "hands on handlebar grips, looking down a low wide bank of screens with a strip overhead. The screen "
             "bank lifts like a visor and the yoke swings up.")]

PAGE = r"""<title>Mech Cockpits</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;500&family=IBM+Plex+Mono:wght@400&display=swap">
<style>
:root { --bg: #e9e7ea; --panel: #f5f4f6; --ink: #1d1a21; --dim: #5f5a66; --line: #c9c4cf; --acc: #c46a12; --btn: #ffffff;
        --on: #1d5fbf; --on-bg: #e3ecfa; --good: #2d7a3a; --bad: #b23a2a; --stage: #1a1620; color-scheme: light }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --bg: #16131a; --panel: #201c25; --ink: #ece8f0;
  --dim: #a59fae; --line: #3a3442; --acc: #f0a04a; --btn: #26212c; --on: #8fb6ff; --on-bg: #24324a; --good: #7fd08a; --bad: #ff8a78; --stage: #0e0c12; color-scheme: dark } }
:root[data-theme="dark"] { --bg: #16131a; --panel: #201c25; --ink: #ece8f0; --dim: #a59fae; --line: #3a3442; --acc: #f0a04a;
  --btn: #26212c; --on: #8fb6ff; --on-bg: #24324a; --good: #7fd08a; --bad: #ff8a78; --stage: #0e0c12; color-scheme: dark }
body { background: var(--bg); color: var(--ink); font: 15px/1.55 "IBM Plex Sans", system-ui, sans-serif }
.wrap { max-width: 1280px; margin: 0 auto; padding-block: 18px 40px; padding-left: max(16px, env(safe-area-inset-left, 0px));
        padding-right: max(16px, env(safe-area-inset-right, 0px)) }
.eyebrow { font: 12px "IBM Plex Mono", monospace; letter-spacing: .12em; text-transform: uppercase; color: var(--dim) }
h1 { font: 700 34px/1.1 "Saira Condensed", sans-serif; letter-spacing: .02em; text-transform: uppercase; margin: 4px 0 8px; text-wrap: balance }
p.lede { max-width: 68ch; color: var(--dim); margin: 0 0 14px }
.grid { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 16px; align-items: start }
@media (max-width: 900px) { .grid { grid-template-columns: minmax(0, 1fr) } }
.row { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px }
.row .k { font: 12px "IBM Plex Mono", monospace; color: var(--dim); width: 64px }
.big { font: 700 22px "Saira Condensed", sans-serif } .good { color: var(--good) } .bad { color: var(--bad) }
.tablewrap { overflow-x: auto; margin-top: 18px }
table { border-collapse: collapse; font-size: 14px; min-width: 560px }
th, td { text-align: left; padding: 5px 10px; border-bottom: 1px solid var(--line) }
th { font: 12px "IBM Plex Mono", monospace; color: var(--dim); text-transform: uppercase; letter-spacing: .06em }
#scrub { flex: 1 1 220px; min-width: 0; accent-color: var(--on); min-height: 36px }
.tnum { font: 13px "IBM Plex Mono", monospace; color: var(--dim); min-width: 92px; font-variant-numeric: tabular-nums }
button { font: 600 15px "Saira Condensed", sans-serif; letter-spacing: .05em; text-transform: uppercase; padding: 7px 12px;
         background: var(--btn); color: var(--ink); border: 1px solid var(--line); border-radius: 4px; cursor: pointer; min-height: 36px }
button.on { background: var(--on-bg); color: var(--on); border-color: var(--on) }
.stage { position: relative; background: var(--stage); border-radius: 6px; overflow: hidden; aspect-ratio: 16 / 10; max-width: 100% }
canvas { width: 100%; height: 100%; display: block; touch-action: none }
.tag { position: absolute; left: 10px; top: 10px; font: 12px "IBM Plex Mono", monospace; color: #f2eef6; background: rgba(10, 8, 14, .7);
       padding: 4px 8px; border-radius: 3px; letter-spacing: .08em; text-transform: uppercase }
.msg { position: absolute; inset: auto 10px 10px 10px; font: 13px "IBM Plex Mono", monospace; color: #ffd9a8 }
.hint { font-size: 13px; color: var(--dim) }
.card { background: var(--panel); border: 1px solid var(--line); border-radius: 6px; padding: 14px 16px }
.card h2 { font: 700 26px "Saira Condensed", sans-serif; text-transform: uppercase; margin: 0 }
.card .sub { font: 12px "IBM Plex Mono", monospace; color: var(--acc); text-transform: uppercase; letter-spacing: .08em }
.card p { margin: 8px 0; font-size: 14px }
dl { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 3px 12px; margin: 8px 0 0; font-size: 14px }
dt { color: var(--dim) } dd { margin: 0 }
code { font: 12px "IBM Plex Mono", monospace }
</style>
<div class="wrap">
  <div class="eyebrow">Crab Breeze · cockpit review · every cockpit, built and proposed</div>
  <h1>Every cockpit, from the pilot's seat</h1>
  <p class="lede">The three built interiors, fully modelled and baked, and ten new layouts of square screens blocked out to
  choose from before the next one is built. All are sealed and see out through flat screens, with live MFDs for the
  mech's health, heading and walking direction, ammunition and heat, and radar. Sit at the pilot's eye and drag to look
  round, open the built ones, or step outside and orbit beside a 1.8 m pilot. "Clear ahead" is measured off each
  layout: the angle between the nearest frame members either side of straight ahead.</p>
  <div class="grid">
    <section>
      <div class="row"><span class="k">Built</span><span id="pick" class="row" style="margin:0"></span></div>
      <div class="row"><span class="k">Concepts</span><span id="pickc" class="row" style="margin:0"></span></div>
      <div class="row"><span class="k">Entry</span><button data-door="front" class="on">Front</button><button data-door="back">Back</button></div>
      <div class="row"><span class="k">View</span><button data-view="eye" class="on">Pilot's eye</button><button data-view="orbit">Outside</button></div>
      <div class="row"><span class="k">Doors</span><button id="open">Open</button><button id="close" class="on">Closed</button>
        <span class="k" style="margin-left:10px">Screens</span><button id="power" class="on">On</button><button id="hud" class="on">HUD</button></div>
      <div class="row"><span class="k">Opening</span><input id="scrub" type="range" min="0" max="1000" value="0" aria-label="Opening, scrub through it">
        <span id="scrubT" class="tnum">0.0 s</span></div>
      <div class="stage"><canvas id="view"></canvas><div class="tag" id="tag">pilot's eye</div><div class="msg" id="msg">loading the interiors…</div></div>
      <p class="hint">Drag to look round from the seat; outside, drag to orbit, scroll or pinch to zoom.</p>
    </section>
    <aside class="card" id="card"></aside>
  </div>
  <div class="tablewrap"><table id="table"></table></div>
</div>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const COCKPITS = __COCKPITS__;
const HUD_SRC = "data:image/png;base64,__HUD__";
const T = THREE;
__GLTF_JSON__
__SCREEN_WALL__
__MFD_PAGES__
__STEAM__
const CONCEPTS = __BLOCKOUTS__;
const loader = gltfJsonLoader(T);
const canvas = document.getElementById("view"), msg = document.getElementById("msg"), tag = document.getElementById("tag");
const renderer = new T.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
const scene = new T.Scene();
scene.background = new T.Color(0x2a1f30);
scene.fog = new T.Fog(0x2a1f30, 60, 220);
const camera = new T.PerspectiveCamera(85, 1.6, 0.02, 600);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true; controls.enabled = false;
scene.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.5));
const sun = new T.DirectionalLight(0xffc89a, 2.0); sun.position.set(-22, 26, 18); scene.add(sun);
// the world on the screens: dusk, a street of blocks, as the mech page's yard; the pod rides 8 m up, in a chest
const POD_Y = 8;
const ground = new T.Mesh(new T.PlaneGeometry(600, 600), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2; scene.add(ground);
const grid = new T.GridHelper(200, 50, 0x4a4552, 0x3a3640); grid.position.y = 0.01; scene.add(grid);
const world = new T.Group(); scene.add(world);
function block(x, z, w, d, storeys, col) {
  const h = storeys * 3.5, m = new T.Mesh(new T.BoxGeometry(w, h, d), new T.MeshStandardMaterial({ color: col, roughness: 0.85 }));
  m.position.set(x, h / 2, z); world.add(m);
  const win = new T.MeshStandardMaterial({ color: 0x1a2230, emissive: 0xffc878, emissiveIntensity: 0.6 });
  for (let s = 0; s < storeys; s++) for (let i = 0; i < Math.floor(w / 2.6); i++) {
    const g = new T.Mesh(new T.BoxGeometry(1.4, 1.6, 0.1), win); g.position.set(x - w / 2 + 1.4 + i * 2.6, s * 3.5 + 2, z - d / 2 - 0.06); world.add(g);
  }
}
[[-18, 40, 14, 10, 4, 0x6a625a], [6, 52, 16, 12, 9, 0x545961], [28, 38, 12, 10, 2, 0x6e6352], [-40, 60, 18, 12, 12, 0x5d5a66],
 [44, 70, 20, 14, 6, 0x645a50], [-8, 90, 24, 14, 14, 0x4f5560]].forEach(b => block(...b));
const rail = new T.Mesh(new T.BoxGeometry(120, 0.12, 0.12), new T.MeshBasicMaterial({ color: 0xf0b13c })); rail.position.set(0, 12, 30); scene.add(rail);
// the platform the pod stands on outside, and the pilot beside it for scale
const deck = new T.Mesh(new T.BoxGeometry(3.4, 0.2, 3.4), new T.MeshStandardMaterial({ color: 0x3a3540, roughness: 0.7 }));
deck.position.set(0, POD_Y - 0.25, 0); scene.add(deck);
const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const tm = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); tm.position.y = 1.18; pilot.add(tm);
const hd = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); hd.position.y = 1.66; hd.scale.set(1, 1.15, 1); pilot.add(hd);
pilot.position.set(2.3, POD_Y - 0.15, -1.0); scene.add(pilot);

const wall = makeScreenWall(T, scene, HUD_SRC);
const MFD = makeMfdPages(T);
const CD_TO_PAGE = 1 / 260;
const pods = {};
const steam = makeSteam(T, scene, 2400);
const RAM_BODY = new T.MeshStandardMaterial({ color: 0x2b2c31, roughness: 0.5, metalness: 0.6 });
const RAM_ROD = new T.MeshStandardMaterial({ color: 0xd8dade, roughness: 0.2, metalness: 0.9 });
const UP = new T.Vector3(0, 1, 0);
// every track at time t: a slide along an axis or a direction, or a turn about an axis, eased in and out
function pose(P, t) {
  for (const tr of P.tr) {
    const k = Math.max(0, Math.min(1, (t - tr.t0) / tr.d)), s = k * k * (3 - 2 * k);
    if (tr.dir) tr.o.position.copy(tr.p).addScaledVector(tr.dir, tr.v * s);
    else if (tr.pos) { tr.o.position.copy(tr.p); tr.o.position[tr.pos] += tr.v * s; }
    else tr.o.quaternion.copy(tr.q).multiply(new T.Quaternion().setFromAxisAngle(AX[tr.axis], tr.v * s));
  }
}

let cur = null, door = "front", view = "eye", openK = 0, openTo = 0, power = 1, hudOn = 1, yaw = 0, pitch = -0.22;
const AX = { x: new T.Vector3(1, 0, 0), y: new T.Vector3(0, 1, 0), z: new T.Vector3(0, 0, 1) };

function tracks(man) {
  return Object.entries(man.motions || {}).map(([node, m]) => ({ node, axis: m.axis, pos: m.slide, v: m.value, t0: m.t0 || 0, d: m.d || 1 }));
}
// each block-out twice: entered by the front, and the same cabin entered by the back
const REARS = CONCEPTS.layouts.map(L => Object.assign({}, L, L.rear, { id: L.id + "_rear", door: "back" }));
Promise.all([...COCKPITS, ...CONCEPTS.layouts, ...REARS].map(c => loader.loadAsync(c.file).then(g => {
  const pod = g.scene;
  let nmfd = 1;
  pod.traverse(o => {
    if (o.isLight) { o.intensity *= CD_TO_PAGE; return; }
    if (!o.isMesh) return;
    for (const m of [].concat(o.material)) {
      if (m.name === "SCREEN_allview") o.material = wall.material;
      else if (m.name.startsWith("SCREEN_")) {        // the monitors show the live MFD pages
        const kind = m.name.startsWith("SCREEN_mfd_") ? m.name.slice(11) : MFD.kinds[nmfd++ % 4];
        o.material = new T.MeshBasicMaterial({ map: MFD.page(kind), toneMapped: false });
      }
    }
  });
  pod.position.y = POD_Y; pod.visible = false; scene.add(pod);
  const nodes = {}; pod.traverse(o => { if (o.name) nodes[o.name] = o; });
  const tr = (c.man ? tracks(c.man) : c.tracks.map(t => Object.assign({}, t))).filter(t => nodes[t.node]);
  tr.forEach(t => { t.o = nodes[t.node]; t.q = t.o.quaternion.clone(); t.p = t.o.position.clone(); if (t.dir) t.dir = new T.Vector3(...t.dir); });
  // a ram is a barrel on the body and a rod on the part, laid each frame between their two nodes; both long enough
  // to meet at full stretch and short enough not to poke past the other end when shut
  const rams = (c.rams || []).filter(r => nodes[r.base] && nodes[r.tip]).map(r => ({ b: nodes[r.base], t: nodes[r.tip] }));
  const vents = (c.vents || []).filter(v => nodes[v.node]).map(v => ({ o: nodes[v.node], dir: new T.Vector3(...v.dir).normalize(), t0: v.t0, d: v.d }));
  const id = c.variant || c.id;
  const P = pods[id] = { c, L: c.man ? null : c, pod, tr, rams, vents, end: Math.max(1, ...tr.map(t => t.t0 + t.d)), eye: c.man ? c.man.eye_m : c.eye_m, eyeNode: c.man ? null : nodes.EYE_pilot };
  if (rams.length) {
    const len = () => rams.map(r => r.b.getWorldPosition(new T.Vector3()).distanceTo(r.t.getWorldPosition(new T.Vector3())));
    pod.updateMatrixWorld(true); const d0 = len(); pose(P, P.end); pod.updateMatrixWorld(true); const d1 = len(); pose(P, 0);
    rams.forEach((r, i) => { const l = Math.min(d0[i] * 0.98, Math.max(d1[i] * 0.55, d0[i] * 0.6));
      r.barrel = new T.Mesh(new T.CylinderGeometry(0.035, 0.035, l, 10).translate(0, l / 2, 0), RAM_BODY);
      r.rod = new T.Mesh(new T.CylinderGeometry(0.018, 0.018, l, 8).translate(0, l / 2, 0), RAM_ROD);
      scene.add(r.barrel, r.rod); r.barrel.visible = r.rod.visible = false; });
  }
}))).then(() => { msg.textContent = ""; pick(COCKPITS[0].variant); }).catch(e => { msg.textContent = "Could not load: " + e; console.error(e); });

const pickEl = document.getElementById("pick"), pickC = document.getElementById("pickc");
COCKPITS.forEach(c => { const b = document.createElement("button"); b.textContent = c.title + " (" + c.mech + ")"; b.dataset.v = c.variant; b.onclick = () => pick(c.variant); pickEl.appendChild(b); });
CONCEPTS.layouts.forEach((L, i) => { const b = document.createElement("button"); b.textContent = (i + 1) + " " + L.name; b.dataset.v = L.id; b.onclick = () => pick(L.id); pickC.appendChild(b); });
const clearCell = (n, cls) => `<span class="${cls || (n >= 80 ? "good" : "bad")}">${n >= 360 ? "all round" : n + " degrees"}</span>`;
document.getElementById("table").innerHTML = "<tr><th>Cockpit</th><th>Kind</th><th>Pose</th><th>Clear ahead</th><th>Back door, clear ahead</th><th>Screens span</th></tr>" +
  COCKPITS.map(c => `<tr><td>${c.title}</td><td>built, in the ${c.mech}</td><td>${c.pose}</td><td>${clearCell(CONCEPTS.built[c.variant].clear_deg)}</td><td>n/a</td><td>${CONCEPTS.built[c.variant].span_deg} degrees</td></tr>`).join("") +
  CONCEPTS.layouts.map((L, i) => `<tr><td>${i + 1} ${L.name}</td><td>block-out</td><td>${L.pose}</td><td>${clearCell(L.clear_deg)}</td><td>${clearCell(L.rear.clear_deg)}</td><td>${L.span_deg >= 360 ? "360 degrees" : L.span_deg + " degrees"}</td></tr>`).join("");
function pick(v) {
  v = v.replace(/_rear$/, "");
  const P = pods[!pods[v].c.man && door === "back" ? v + "_rear" : v];
  if (!P) return;
  v = P.c.variant || P.c.id;
  for (const k in pods) { pods[k].pod.visible = k === v; pods[k].rams.forEach(r => { r.barrel.visible = r.rod.visible = k === v; }); }
  cur = P; openK = 0; openTo = 0; setOpen(false);
  [pickEl, pickC].forEach(el => el.querySelectorAll("button").forEach(b => b.classList.toggle("on", b.dataset.v === v.replace(/_rear$/, ""))));
  if (P.L) {
    const L = P.L;
    document.getElementById("card").innerHTML = `<div class="sub">block-out · seated · ${L.door === "back" ? "back door" : "front door"}</div><h2>${L.name}</h2><p>${L.text}</p><dl>
      <dt>Clear ahead</dt><dd class="big">${clearCell(L.clear_deg)}</dd>
      <dt>Screens span</dt><dd>${L.span_deg >= 360 ? "360 degrees, up and down too" : L.span_deg + " degrees round"}</dd>
      <dt>Eye</dt><dd>${L.eye_m[1].toFixed(2)} m over the floor, seated</dd>
      <dt>Opening</dt><dd>${L.opening}; ${L.open_s.toFixed(1)} s</dd>
      <dt>Moving parts</dt><dd>${L.parts.map(n => n.replace(/_/g, " ").toLowerCase()).join(", ")}, ${L.tracks.filter(t => t.dir).length} clamps, ${L.rams.length} rams, ${L.vents.length} vents</dd>
      <dt>MFDs</dt><dd>status, nav, weapons, radar: ${L.mfd.map(w => ({ top: "on the viewscreen, along the top of the front screen, as a HUD", dash: "on the dash", arms: "on the armrests, riding with the seat" })[w]).join(" and ")}</dd>
      <dt>Triangles</dt><dd>${L.tris.toLocaleString()}</dd></dl>
      <p class="hint">Low-poly and flat-coloured, built the CSG way in Blender (<code>tools/blender/build_cockpit_blockouts.py</code>): an airtight shell, screens cut into its inner wall, the opening parts cut off it by booleans with their seals and clamps in orange. The chosen layout is then modelled and baked in detail.</p>`;
  } else {
    const c = P.c, m = c.man, e = m.envelope_m, bc = { clear: CONCEPTS.built[c.variant].clear_deg };
    document.getElementById("card").innerHTML = `<div class="sub">built · in the ${c.mech}</div><h2>${c.title}</h2><p>${c.text}</p><dl>
    <dt>Clear ahead</dt><dd class="big">${clearCell(bc.clear)}</dd>
    <dt>Eye</dt><dd>${m.eye_m[1].toFixed(2)} m over the floor</dd>
    <dt>Envelope</dt><dd>${(2 * (Array.isArray(e.x) ? e.x[1] : e.x)).toFixed(2)} wide, ${(e.y[1] - e.y[0]).toFixed(2)} tall, ${(e.z[1] - e.z[0]).toFixed(2)} deep (m)</dd>
    <dt>Doorway</dt><dd>${m.doorway_m ? m.doorway_m.width.toFixed(2) + " wide, " + m.doorway_m.height.toFixed(2) + " tall (m)" : "n/a"}</dd>
    <dt>Opens in</dt><dd>${(m.inner_opening_s || P.end).toFixed(1)} s, after the chest armour</dd>
    <dt>Moving parts</dt><dd>${P.tr.map(t => t.node.replace(/_/g, " ").toLowerCase()).join(", ")}</dd>
    <dt>Lamps</dt><dd>${(m.lights || []).length}, lit live</dd>
    <dt>Triangles</dt><dd>${(m.tris || 0).toLocaleString()} of ${(m.tri_budget || 0).toLocaleString()}</dd>
    <dt>Maps</dt><dd>base colour, normal, ORM, emission</dd></dl>
    <p class="hint">Built by <code>tools/blender/build_cockpit.py --variant ${c.variant}</code>; its monitors show the live MFD pages here.</p>`;
  }
  yaw = 0; pitch = -0.22; setView(view);
}
function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(b => b.classList.toggle("on", b.dataset.view === v));
  controls.enabled = v === "orbit";
  tag.textContent = v === "eye" ? "pilot's eye" : "outside";
  camera.fov = v === "eye" ? 85 : 40; camera.updateProjectionMatrix();
  if (v === "orbit") { camera.position.set(4.2, POD_Y + 2.8, door === "back" ? -5.2 : 5.2); controls.target.set(0, POD_Y + 1.0, 0.1); controls.update(); }
}
document.querySelectorAll("[data-view]").forEach(b => b.onclick = () => setView(b.dataset.view));
document.querySelectorAll("[data-door]").forEach(b => b.onclick = () => {
  door = b.dataset.door;
  document.querySelectorAll("[data-door]").forEach(x => x.classList.toggle("on", x === b));
  if (cur) pick(cur.c.variant || cur.c.id);
  setView(view);
});
// the opening slider: drag it to scrub through the opening and hold any moment of it; Open and Closed play to
// either end from wherever it is
const scrub = document.getElementById("scrub"), scrubT = document.getElementById("scrubT");
let scrubbing = false;
scrub.addEventListener("input", () => { scrubbing = true; openK = openTo = scrub.value / 1000;
  document.getElementById("open").classList.toggle("on", openTo === 1); document.getElementById("close").classList.toggle("on", openTo === 0); });
scrub.addEventListener("change", () => { scrubbing = false; });
const setOpen = o => { openTo = o ? 1 : 0; document.getElementById("open").classList.toggle("on", o); document.getElementById("close").classList.toggle("on", !o); };
document.getElementById("open").onclick = () => setOpen(true);
document.getElementById("close").onclick = () => setOpen(false);
document.getElementById("power").onclick = e => { power = power ? 0 : 1; e.target.classList.toggle("on", !!power); };
document.getElementById("hud").onclick = e => { hudOn = hudOn ? 0 : 1; e.target.classList.toggle("on", !!hudOn); };
let drag = null;
canvas.addEventListener("pointerdown", e => { if (view === "eye") { drag = [e.clientX, e.clientY]; canvas.setPointerCapture(e.pointerId); } });
canvas.addEventListener("pointermove", e => { if (!drag) return; yaw -= (e.clientX - drag[0]) * 0.005; pitch = Math.max(-1.2, Math.min(1.1, pitch - (e.clientY - drag[1]) * 0.005)); drag = [e.clientX, e.clientY]; });
canvas.addEventListener("pointerup", () => { drag = null; });
window.MOCKUP = { camAt: (x, y, z) => { camera.position.set(x, POD_Y + y, z); controls.update(); }, pick, setView, setOpen, setDoor: d => document.querySelector(`[data-door="${d}"]`).click(), look: (y, p) => { yaw = y; pitch = p; }, ready: () => !!cur };

const clock = new T.Clock(), eyeW = new T.Vector3();
function frame() {
  requestAnimationFrame(frame);
  const dt = Math.min(clock.getDelta(), 0.05);
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.floor(w * renderer.getPixelRatio())) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
  MFD.tick(dt);
  if (cur) {
    openK += Math.sign(openTo - openK) * Math.min(Math.abs(openTo - openK), dt / cur.end);
    const t = openK * cur.end;
    if (!scrubbing) scrub.value = Math.round(openK * 1000);
    scrubT.textContent = `${t.toFixed(1)} of ${cur.end.toFixed(1)} s`;
    const tPrev = cur.t === undefined ? t : cur.t; cur.t = t;
    pose(cur, t);
    cur.pod.updateMatrixWorld(true);
    for (const r of cur.rams) {
      const a = r.b.getWorldPosition(new T.Vector3()), b = r.t.getWorldPosition(new T.Vector3()), d = b.clone().sub(a).normalize();
      r.barrel.position.copy(a); r.barrel.quaternion.setFromUnitVectors(UP, d);
      r.rod.position.copy(b); r.rod.quaternion.setFromUnitVectors(UP, d.clone().negate());
    }
    // steam where a seal breaks, while the opening plays forward through it
    if (t > tPrev) for (const v of cur.vents) if (t >= v.t0 && t <= v.t0 + v.d) {
      const pw = v.o.getWorldPosition(new T.Vector3());
      for (let i = 0; i < 3; i++) steam.emit(pw, v.dir, i === 0, 1, 0.5);
    }
    const e = cur.eye;
    if (cur.eyeNode) cur.eyeNode.getWorldPosition(eyeW); else eyeW.set(e[0], e[1] + POD_Y, e[2]);
    if (view === "eye") {
      camera.position.copy(eyeW);
      camera.quaternion.setFromEuler(new T.Euler(pitch, yaw + Math.PI, 0, "YXZ"));     // the pilot faces +z, a camera -z
      if (cur.eyeNode) camera.quaternion.premultiply(cur.eyeNode.getWorldQuaternion(new T.Quaternion()));   // a seat that swivels turns the pilot
      renderer.toneMappingExposure = 1.7;
    } else { controls.update(); renderer.toneMappingExposure = 1.0; }
    steam.step(dt, camera, renderer);
    wall.update(dt, eyeW, cur.pod, power, hudOn);
    wall.capture(renderer, eyeW, [cur.pod, pilot, deck]);
  }
  renderer.render(scene, camera);
}
frame();
</script>
"""


def main(only=None):
    hud = base64.b64encode(open(MD.HUD, "rb").read()).decode("ascii")
    shutil.rmtree(SITE, ignore_errors=True)
    os.makedirs(SITE)
    cockpits, files = [], {}
    for v, mech, title, text in VARIANTS:
        if only and v not in only:
            continue
        d = os.path.join(MD.MODELS, f"cockpit_{v}")
        if not os.path.exists(os.path.join(d, f"cockpit_{v}.gltf")):
            continue
        rel = f"cockpit_{v}/cockpit_{v}.gltf.json"
        MD.as_json_gltf(os.path.join(d, f"cockpit_{v}.gltf"), os.path.join(SITE, rel))
        files[rel] = os.path.join(SITE, rel)
        for f in sorted(os.listdir(os.path.join(d, "textures"))):
            r = f"cockpit_{v}/textures/{f}"
            os.makedirs(os.path.dirname(os.path.join(SITE, r)), exist_ok=True)
            shutil.copyfile(os.path.join(d, "textures", f), os.path.join(SITE, r))
            files[r] = os.path.join(SITE, r)
        man = json.load(open(os.path.join(MD.MODELS, f"cockpit_{v}.json")))
        cockpits.append(dict(variant=v, mech=mech.capitalize(), title=title, text=text, file=rel, man=man,
                             pose={"wall": "seated", "harness": "standing", "jockey": "leaning"}[v]))
    blockouts = json.load(open(os.path.join(MD.MODELS, "cockpit_blockouts.json")))
    for L in blockouts["layouts"]:
        for d in (L, L["rear"]):
            rel = "blockouts/" + os.path.basename(d["file"]).replace(".glb", ".gltf.json")
            MD.as_json_gltf(os.path.join(MD.MODELS, d["file"]), os.path.join(SITE, rel))
            files[rel] = os.path.join(SITE, rel)
            d["file"] = rel
    js = lambda name: open(os.path.join(ROOT, "tools", "mockups", name)).read()   # noqa: E731
    html = (PAGE.replace("__GLTF_JSON__", js("gltf_json_loader.js")).replace("__SCREEN_WALL__", js("screen_wall.js")).replace("__MFD_PAGES__", js("mfd_pages.js"))
            .replace("__STEAM__", js("steam.js"))
            .replace("__BLOCKOUTS__", json.dumps(blockouts))
            .replace("__COCKPITS__", json.dumps(cockpits)).replace("__HUD__", hud))
    with open(OUT, "w") as f:
        f.write(html)
    os.symlink(OUT, os.path.join(SITE, "index.html"))
    with open(os.path.join(SITE, "files.json"), "w") as f:
        json.dump({k: os.path.relpath(p, ROOT) for k, p in files.items()}, f, indent=1)
    print(f"wrote {OUT} ({len(html) / 1e6:.2f} MB), {len(files)} files beside it "
          f"({sum(os.path.getsize(p) for p in files.values()) / 1e6:.1f} MB): {[c['variant'] for c in cockpits]}")


if __name__ == "__main__":
    main(sys.argv[1:] or None)       # optionally only some variants, e.g. while the others rebake
