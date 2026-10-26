"""Build docs/mockups/cars.html: the seven traffic car block-outs at true scale beside the pilot and the Foreman.

    python3 tools/mockups/gen_cars.py

Reads assets/models/cars/<kind>.glb and assets/models/cars.json (written by tools/blender/build_cars.py) and our
Foreman (assets/models/mech_foreman.glb) and inlines them, so the page needs nothing but three.js r147. Every size
on the page is measured off the glb (the manifests), never typed (CLAUDE.md section 10, Mining Mike's rule: orbit
it, at true relative scale, with a known object beside it). Each car can be shown whole, crushed or burnt, and seen
from its own `EYE_driver`, where the block-out dashboard, steering wheel and speedometer needle sit.

The cars are block-out stand-ins awaiting the owner's approval; the page says so on every card.
"""
import base64
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "cars.html")
ORDER = ["sedan", "wagon", "pickup", "van", "taxi", "bus", "semi"]
HUMAN = "foreman"

PAGE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Traffic Car Block-outs</title>
<style>
:root {
  --bg: #eceef1; --panel: #f8f9fa; --edge: #c6ccd4; --ink: #14181d; --muted: #59626e; --accent: #b4561a;
  --stage: #20242a; --warn: #9a2a10;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif; --mono: ui-monospace, Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { --bg: #0e1115; --panel: #161a20; --edge: #2b323c; --ink: #e5e9ee; --muted: #94a0ad; --accent: #f08a3c; --warn: #ff8a6a; color-scheme: dark; }
}
:root[data-theme="dark"] { --bg: #0e1115; --panel: #161a20; --edge: #2b323c; --ink: #e5e9ee; --muted: #94a0ad; --accent: #f08a3c; --warn: #ff8a6a; color-scheme: dark; }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.45 var(--body); }
header { padding: 16px; max-width: 1200px; margin: 0 auto; }
h1 { margin: 0 0 4px; font-size: 22px; }
header p { margin: 0; color: var(--muted); }
.flag { display: inline-block; margin-top: 8px; padding: 2px 8px; border: 1px solid var(--warn); color: var(--warn); border-radius: 4px; font-size: 13px; }
main { display: grid; grid-template-columns: minmax(0, 1fr) 300px; gap: 16px; padding: 0 16px 16px; max-width: 1200px; margin: 0 auto; }
@media (max-width: 820px) { main { grid-template-columns: 1fr; } }
#stage { position: relative; background: var(--stage); border-radius: 8px; overflow: hidden; aspect-ratio: 16 / 10; }
#view { width: 100%; height: 100%; display: block; touch-action: none; }
#labels { position: absolute; inset: 0; pointer-events: none; }
#labels span { position: absolute; transform: translate(-50%, -100%); color: #fff; font: 12px var(--mono); background: rgba(0,0,0,.55); padding: 1px 5px; border-radius: 3px; white-space: nowrap; }
.side { display: flex; flex-direction: column; gap: 10px; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 8px; padding: 10px 12px; }
.card h2 { margin: 0 0 6px; font-size: 14px; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); }
.row { display: flex; flex-wrap: wrap; gap: 6px; }
button { font: inherit; font-size: 13px; padding: 4px 9px; border-radius: 5px; border: 1px solid var(--edge); background: var(--bg); color: var(--ink); cursor: pointer; }
button[aria-pressed="true"] { background: var(--accent); color: #fff; border-color: var(--accent); }
table { width: 100%; border-collapse: collapse; font: 12px var(--mono); }
td, th { text-align: left; padding: 2px 4px; border-bottom: 1px solid var(--edge); }
.note { color: var(--muted); font-size: 13px; }
</style>
</head>
<body>
<header>
  <h1>Traffic car block-outs</h1>
  <p>The seven 1980s civilian cars (survey J7) at true size beside the pilot (1.8 m) and the Foreman (24 m), on the 24 m street they drive. Drag to orbit, scroll or pinch to zoom; arrow keys and +/- also work.</p>
  <span class="flag">Block-out stand-ins: flat colours, no textures, no LODs. Awaiting approval before the detailed models.</span>
</header>
<main>
  <div id="stage"><canvas id="view" tabindex="0" aria-label="3D view of the cars"></canvas><div id="labels"></div></div>
  <div class="side">
    <div class="card"><h2>Show</h2><div class="row" id="toggles"></div><div class="row" style="margin-top:6px"><button id="reset">Reset view</button></div></div>
    <div class="card"><h2>State</h2><div class="row" id="states"></div><p class="note">Crushed and burnt are how the game shows a wreck until the crushed variants are modelled: crushed is the block-out pressed to 35 % of its height, burnt is its paint blackened.</p></div>
    <div class="card"><h2>Driver's eye</h2><div class="row" id="eyes"></div><p class="note">From each car's <code>EYE_driver</code>: the dashboard, the wheel (<code>STEER_wheel</code>) and the speedometer needle (<code>NEEDLE_speed</code>), which the game turns from the simulation.</p></div>
    <div class="card"><h2>Measured</h2><table id="sizes"><tr><th>Car</th><th>W x H x L m</th><th>Tris</th></tr></table></div>
  </div>
</main>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__, HUMAN = __HUMAN__, GLB = __GLB__, ORDER = __ORDER__;
const T = THREE;
const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.shadowMap.enabled = true;
const scene = new T.Scene();
scene.background = new T.Color(0x9fb4c8);
const camera = new T.PerspectiveCamera(45, 1.6, 0.05, 2000);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
scene.add(new T.HemisphereLight(0xdfe8f5, 0x3a3a38, 0.8));
const sun = new T.DirectionalLight(0xfff0dc, 1.6); sun.position.set(-30, 50, 20); sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -60, right: 60, top: 60, bottom: -60 }); scene.add(sun);
// the street: 24 m of asphalt with its lane marks 3.5 m either side of the centre line (data/world/traffic.ron)
const ground = new T.Mesh(new T.PlaneGeometry(400, 400), new T.MeshStandardMaterial({ color: 0x5d6b52, roughness: 1 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const road = new T.Mesh(new T.PlaneGeometry(200, 24), new T.MeshStandardMaterial({ color: 0x2c2e31, roughness: 0.95 }));
road.rotation.x = -Math.PI / 2; road.position.y = 0.01; road.receiveShadow = true; scene.add(road);
for (const z of [0]) { const m = new T.Mesh(new T.PlaneGeometry(200, 0.15), new T.MeshBasicMaterial({ color: 0xe8c040 })); m.rotation.x = -Math.PI / 2; m.position.set(0, 0.02, z); scene.add(m); }
const grid = new T.GridHelper(200, 50, 0x6f7a86, 0x6f7a86); grid.position.y = 0.03; grid.material.opacity = 0.25; grid.material.transparent = true; scene.add(grid);
// rails at the pilot's eye (1.7 m) and the mech's top
const rail = (y, c) => { const r = new T.Mesh(new T.BoxGeometry(120, 0.06, 0.06), new T.MeshBasicMaterial({ color: c })); r.position.set(0, y, -10); scene.add(r); return r; };
rail(1.7, 0xf0b13c); rail(HUMAN.bounds_m.y[1], 0x4cc3ff);

// the pilot, 1.8 m
const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torso = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torso.position.y = 1.18; pilot.add(torso);
const head = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.68; pilot.add(head);
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; });
pilot.position.set(-2.5, 0, 6); scene.add(pilot);

const loader = new T.GLTFLoader();
const buf = b64 => { const s = atob(b64), u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; };
const load = id => new Promise(res => loader.parse(buf(GLB[id]), "", g => { const nodes = {};
  g.scene.traverse(o => { if (o.name) nodes[o.name] = o; if (o.isMesh) { o.castShadow = o.receiveShadow = true; o.material = o.material.clone(); } });
  res({ root: g.scene, nodes }); }));

const exhibits = {};
let state = "whole";
const labels = document.getElementById("labels");
function placeAll() {
  // the cars in a line along the near lane (3.5 m right of the centre line), nose to tail with 3 m between
  let x = -40;
  for (const id of ORDER) {
    const e = exhibits[id]; if (!e) continue;
    const L = META[id].bounds_m.z[1] - META[id].bounds_m.z[0];
    e.root.rotation.y = Math.PI / 2; e.root.position.set(x + L / 2, 0, 3.5); x += L + 3;
  }
  if (exhibits.foreman) exhibits.foreman.root.position.set(x + 14, 0, -6);
}
function setState(s) {
  state = s;
  for (const id of ORDER) { const e = exhibits[id]; if (!e) continue;
    e.root.scale.set(1, s === "crushed" ? 0.35 : 1, 1);
    e.root.traverse(o => { if (o.isMesh) { if (!o.userData.c) o.userData.c = o.material.color.clone();
      o.material.color.copy(o.userData.c); if (s === "burnt") o.material.color.multiplyScalar(0.08); } }); }
  document.querySelectorAll("#states button").forEach(b => b.setAttribute("aria-pressed", b.dataset.s === s));
}
function resetView() { controls.target.set(-10, 2, 0); camera.position.set(-18, 14, 34); controls.update(); }
function driverView(id) {
  const e = exhibits[id]; if (!e) return; setState("whole");
  const eye = new T.Vector3(); e.nodes.EYE_driver.getWorldPosition(eye);
  const fwd = new T.Vector3(-1, 0, 0); // the cars face -z, turned to face -x here
  camera.position.copy(eye); controls.target.copy(eye.clone().add(fwd.multiplyScalar(0.6)).add(new T.Vector3(0, -0.12, 0))); controls.update();
}
const tbody = document.getElementById("sizes");
const tog = document.getElementById("toggles"), eyes = document.getElementById("eyes"), states = document.getElementById("states");
function button(parent, text, on, fn) { const b = document.createElement("button"); b.textContent = text; b.setAttribute("aria-pressed", on); b.onclick = () => fn(b); parent.appendChild(b); return b; }
for (const s of ["whole", "crushed", "burnt"]) { const b = button(states, s, s === "whole", () => setState(s)); b.dataset.s = s; }
document.getElementById("reset").onclick = resetView;
Promise.all([...ORDER.map(id => load(id).then(e => { exhibits[id] = e; })), load("foreman").then(e => { exhibits.foreman = e; })]).then(() => {
  for (const id of [...ORDER, "foreman"]) { const e = exhibits[id]; scene.add(e.root);
    button(tog, id === "foreman" ? "Foreman (24 m)" : META[id].name, true, b => { e.root.visible = !e.root.visible; b.setAttribute("aria-pressed", e.root.visible); }); }
  for (const id of ORDER) { button(eyes, META[id].name, false, () => driverView(id));
    const m = META[id], b = m.bounds_m, f = v => v.toFixed(2);
    tbody.insertAdjacentHTML("beforeend", `<tr><td>${m.name}</td><td>${f(b.x[1] - b.x[0])} x ${f(b.y[1] - b.y[0])} x ${f(b.z[1] - b.z[0])}</td><td>${m.triangles}</td></tr>`); }
  button(tog, "Pilot (1.8 m)", true, b => { pilot.visible = !pilot.visible; b.setAttribute("aria-pressed", pilot.visible); });
  placeAll(); resetView();
  for (const id of ORDER) { const s = document.createElement("span"); s.textContent = META[id].name + " (stand-in)"; labels.appendChild(s); exhibits[id].label = s; }
  window.MOCKUP_READY = true;
});
canvas.addEventListener("keydown", e => {
  const k = { ArrowLeft: [0.06, 0], ArrowRight: [-0.06, 0], ArrowUp: [0, 0.06], ArrowDown: [0, -0.06] }[e.key];
  if (k) { const off = camera.position.clone().sub(controls.target); const sph = new T.Spherical().setFromVector3(off);
    sph.theta += k[0]; sph.phi = Math.min(Math.PI / 2 - 0.02, Math.max(0.05, sph.phi - k[1])); camera.position.copy(controls.target).add(new T.Vector3().setFromSpherical(sph)); e.preventDefault(); }
  if (e.key === "+" || e.key === "=" || e.key === "-") { const off = camera.position.clone().sub(controls.target).multiplyScalar(e.key === "-" ? 1.15 : 0.87); camera.position.copy(controls.target).add(off); }
});
function resize() { const r = canvas.parentElement.getBoundingClientRect(); renderer.setSize(r.width, r.height, false); camera.aspect = r.width / r.height; camera.updateProjectionMatrix(); }
window.addEventListener("resize", resize); resize();
const v = new T.Vector3();
(function loop() { requestAnimationFrame(loop); controls.update(); renderer.render(scene, camera);
  const r = canvas.getBoundingClientRect();
  for (const id of ORDER) { const e = exhibits[id]; if (!e || !e.label) continue;
    v.set(0, (META[id].bounds_m.y[1] + 0.6) * e.root.scale.y, 0); e.root.localToWorld(v); v.project(camera);
    const on = e.root.visible && v.z < 1 && Math.abs(v.x) < 1.1 && Math.abs(v.y) < 1.1;
    e.label.style.display = on ? "block" : "none"; e.label.style.left = (v.x + 1) / 2 * r.width + "px"; e.label.style.top = (1 - v.y) / 2 * r.height + "px"; } })();
</script>
</body>
</html>
"""


def main():
    meta = json.load(open(os.path.join(MODELS, "cars.json")))
    human = json.load(open(os.path.join(MODELS, "mechs.json")))[HUMAN]
    glb = {}
    for key in ORDER:
        with open(os.path.join(MODELS, "cars", meta[key]["file"]), "rb") as f:
            glb[key] = base64.b64encode(f.read()).decode("ascii")
    with open(os.path.join(MODELS, human["file"]), "rb") as f:
        glb["foreman"] = base64.b64encode(f.read()).decode("ascii")
    html = (PAGE.replace("__META__", json.dumps({k: meta[k] for k in ORDER})).replace("__HUMAN__", json.dumps(human))
            .replace("__GLB__", json.dumps(glb)).replace("__ORDER__", json.dumps(ORDER)))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
