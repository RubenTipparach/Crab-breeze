#!/usr/bin/env python3
"""The melee lab: one page for designing the mechs' melee moves and gun recoil (openspec change `close-combat`;
owner, 2026-10-29: "I will need your help desigining melee animaitons in the mech artifact", "shooting guns should
cause mech arm to recoil", then "Just make a new one", "It might be too big" of the mech design page). One detailed
mech at a time, loaded when picked (about 3 MB), and a second copy of it as the sparring partner; the moves are
tools/mockups/mech_melee.js, posed by tools/mockups/mech_pose.js, edited on the page and saved to its database.

    python3 tools/mockups/gen_melee_lab.py      # writes output/melee_site (index.html and the mechs beside it)
"""
import json
import os
import shutil

import gen_mech_designs as designs

ROOT = designs.ROOT
SITE = os.path.join(ROOT, "output", "melee_site")

PAGE = r"""<title>Mech Melee Lab</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
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
.wrap { max-width: 1200px; margin: 0 auto; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 2px 0 6px; }
.lede { margin: 0 0 12px; color: var(--muted); max-width: 76ch; }
.bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 6ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button:hover { border-color: var(--muted); }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button:focus-visible, canvas:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
button.go { border-color: var(--accent); }
.scrub { display: flex; gap: 10px; align-items: center; flex: 1 1 220px; min-width: 0; }
.scrub input { flex: 1; min-width: 0; accent-color: var(--accent); }
.scrub output { font: 500 13px var(--mono); color: var(--muted); font-variant-numeric: tabular-nums; min-width: 6ch; }
.stage { position: relative; background: var(--stage); border: 1px solid var(--edge); border-radius: 4px; overflow: hidden; touch-action: none; }
canvas { display: block; width: 100%; aspect-ratio: 16 / 9; }
@media (max-width: 600px) { canvas { aspect-ratio: 1 / 1; } }
.load { position: absolute; inset: 0; display: grid; place-items: center; color: #c9d2dc; font: 500 14px var(--mono); pointer-events: none; }
.mode { position: absolute; left: 10px; top: 10px; font: 700 13px var(--display); letter-spacing: 0.08em; text-transform: uppercase;
  color: #e6ebf1; background: rgba(14, 10, 18, 0.85); border: 1px solid #3a2f40; padding: 5px 9px; pointer-events: none; }
.hint { font-size: 13px; color: var(--muted); margin: 6px 0 0; }
select { font: 500 13px var(--mono); background: var(--panel); color: var(--ink); border: 1px solid var(--edge); border-radius: 3px; padding: 6px; min-height: 34px; }
[hidden] { display: none !important; }
__LAB_CSS__
</style>

<div class="wrap">
  <div class="eyebrow">Crab Breeze · close combat</div>
  <h1>Mech Melee Lab</h1>
  <p class="lede">Shape the mechs' punches, kicks and gun recoil. Pick a move and press Play: your mech swings, and the
  copy facing it takes the hit when the blow lands. Edit keys pauses on a key so you can pose it bone by bone; Save
  keeps the move for everyone, and Claude reads it back into the game's design.</p>
  <div class="bar" role="group" aria-label="Mech">
    <span class="lbl">Mech</span>
__MECH_BUTTONS__
  </div>
  <div class="bar" role="group" aria-label="Gait">
    <span class="lbl">Gait</span>
    <button data-gait="idle" aria-pressed="true">Stand</button>
    <button data-gait="walk">Walk</button>
    <span class="lbl" style="margin-left:8px">View</span>
    <button data-view="side" aria-pressed="true">Side</button>
    <button data-view="three">Three quarter</button>
    <button data-view="front">Front</button>
    <button data-view="eye">Pilot's eye</button>
  </div>
__LAB_HTML__
  <div class="stage">
    <canvas id="view" tabindex="0" aria-label="3D view of the mech and its sparring partner"></canvas>
    <div class="mode" id="mode">Side</div>
    <div class="load" id="load">Loading the mech</div>
  </div>
  <p class="hint">Drag to orbit, scroll or pinch to zoom. Turns are radians, slides metres. On a bone, rx below zero swings it forward and up; ry above zero turns it toward the mech's left; a shoulder's rz lifts the arm out to the side (below zero on the right, above zero on the left).</p>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const T = THREE, TAU = Math.PI * 2;
const ORDER = __ORDER__;
__MECH_POSE__
__MECH_MELEE__
__GLTF_JSON__
const parseX = v => (typeof v === "string" ? JSON.parse(v) : v);

const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = T.PCFSoftShadowMap;
const scene = new T.Scene();
scene.background = new T.Color(0x2a1f30);
scene.fog = new T.Fog(0x2a1f30, 80, 220);
const camera = new T.PerspectiveCamera(42, 1.6, 0.05, 500);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
scene.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.55));
const sun = new T.DirectionalLight(0xffc89a, 2.0);
sun.position.set(-22, 30, 26); sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -36, right: 36, top: 36, bottom: -36, near: 1, far: 120 });
sun.shadow.bias = -0.0005; scene.add(sun);
const rim = new T.DirectionalLight(0x7fa8ff, 0.9); rim.position.set(18, 12, -22); scene.add(rim);
const ground = new T.Mesh(new T.PlaneGeometry(500, 500), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const grid = new T.GridHelper(160, 40, 0x4a4552, 0x3a3640); grid.position.y = 0.01; scene.add(grid);

// ---------- the two mechs: ours at the origin facing +z, the partner a copy of it facing back ----------
const loader = gltfJsonLoader(T);
const cache = {};
let current = null, me = null, foe = null, gait = "idle", view = "side";
const cur = () => me;
function makeMech(g, id, seed) {
  const root = g.scene.clone(true), nodes = {};
  root.traverse(o => { if (o.name) nodes[o.name] = o; if (o.isMesh) { o.castShadow = o.receiveShadow = true;
    if (o.material && o.material.transparent) { o.material.depthWrite = false; o.castShadow = false; o.renderOrder = 2; } } });
  // the chest's open-and-close rig is not driven here: everything boarding-only stays at its rest
  const x = (nodes.root && nodes.root.userData) || {};
  const b = { id, root, nodes, seed, ph: 0, t: seed, scale: x.scale || 0.13, legSegs: parseX(x.legSegs), footH: x.footH,
    walk: parseX(x.walk), idleP: parseX(x.idle), hitT: 9, hitSide: 1, wWalk: 0, rest: {}, yaw0: 0 };
  for (const n in nodes) if (MAPPED.has(n) || MELEE_BONES.includes(n) || n === "hand_L" || n === "hand_R") {
    const o = nodes[n]; b.rest[n] = { e: new T.Euler().setFromQuaternion(o.quaternion, "XYZ"), p: o.position.clone() }; }
  scene.add(root);
  return b;
}
function setMech(id) {
  current = id;
  document.querySelectorAll("[data-mech]").forEach(e => e.setAttribute("aria-pressed", String(e.dataset.mech === id)));
  const ld = document.getElementById("load"); ld.hidden = false; ld.textContent = "Loading the mech";
  (cache[id] = cache[id] || loader.loadAsync(`mech_${id}.gltf.json`)).then(g => {
    if (current !== id) return;
    for (const m of [me, foe]) if (m) scene.remove(m.root);
    me = makeMech(g, id, 3); foe = makeMech(g, id, 11);
    layout(); setView(view); ld.hidden = true; window.LAB_READY = true;
  }).catch(e => { ld.textContent = "Could not load: " + e; });
}
function layout() {
  if (!foe) return;
  foe.root.visible = mel.spar;
  foe.yaw0 = Math.PI; foe.root.rotation.y = Math.PI; foe.root.position.set(0, 0, SPAR_M);
}
function eye() { const p = new T.Vector3(); const e = me && (me.nodes.EYE_pilot || me.nodes.cockpit); if (e) e.getWorldPosition(p); return p; }
function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(e => e.setAttribute("aria-pressed", String(e.dataset.view === v)));
  document.getElementById("mode").textContent = { side: "Side", three: "Three quarter", front: "Front", eye: "Pilot's eye" }[v];
  if (v === "eye") {
    camera.fov = 78; camera.near = 0.02; camera.updateProjectionMatrix();
    const e = eye(); camera.position.copy(e); controls.target.copy(e).add(new T.Vector3(0, -0.15, 1));
    Object.assign(controls, { minDistance: 0.01, maxDistance: 1, enablePan: false, enableZoom: false, rotateSpeed: -0.35 });
  } else {
    camera.fov = 42; camera.near = 0.05; camera.updateProjectionMatrix();
    Object.assign(controls, { minDistance: 4, maxDistance: 160, enablePan: true, enableZoom: true, rotateSpeed: 1 });
    controls.target.set(0, 12, SPAR_M / 2);
    if (v === "side") camera.position.set(52, 15, SPAR_M / 2);
    else if (v === "front") camera.position.set(-6, 14, -38);
    else camera.position.set(38, 22, -22);
  }
  controls.update();
}
document.querySelectorAll("[data-view]").forEach(e => e.addEventListener("click", () => setView(e.dataset.view)));
document.querySelectorAll("[data-gait]").forEach(e => e.addEventListener("click", () => { gait = e.dataset.gait;
  document.querySelectorAll("[data-gait]").forEach(x => x.setAttribute("aria-pressed", String(x === e))); }));
document.querySelectorAll("[data-mech]").forEach(e => e.addEventListener("click", () => setMech(e.dataset.mech)));

function animate(b, dt, isMe) {
  b.t += dt; b.hitT += dt;
  b.wWalk += ((isMe && gait === "walk" ? 1 : 0) - b.wWalk) * Math.min(1, dt * 4);
  b.ph += dt * TAU * ((b.walk && b.walk.hz) || 0.5) * b.wWalk;
  const P = new Pose();
  P.w = 1 - b.wWalk; idleGeneric(P, b.t, b);
  if (b.walk) { P.w = b.wWalk; walkGeneric(P, b.t, b); }
  if (b.hitT < 0.9) { P.w = 1; hitGeneric(P, b.hitT, b); }
  if (isMe && mel.id && MELEE_MOVES[mel.id]) { P.w = 1; meleePose(P, b, MELEE_MOVES[mel.id], mel.t); }
  applyPose(b, P);
  b.root.rotation.y = b.yaw0;
  b.root.updateMatrixWorld(true);
}

__LAB_JS__

let last = 0;
function loop(ms) {
  const dt = Math.min(0.05, (ms - last) / 1000 || 0.016); last = ms;
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.round(w * renderer.getPixelRatio())) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
  if (me) {
    stepMelee(dt);
    animate(me, dt, true);
    if (foe.root.visible) animate(foe, dt, false);
    if (view === "eye") { const e = eye(), dl = e.clone().sub(camera.position); camera.position.add(dl); controls.target.add(dl); }
  }
  controls.update();
  renderer.render(scene, camera);
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);
setMech(ORDER[0]);
window.LAB = { setMech, setView, melee(id, t) { pickMove(id); if (t !== undefined) { mel.playing = false; mel.t = t; drawMelee(); } } };
</script>
"""


def main():
    meta = json.load(open(os.path.join(designs.MODELS, "mechs.json")))
    rd = lambda n: open(os.path.join(ROOT, "tools", "mockups", n)).read()
    html = (PAGE.replace("__LAB_CSS__", rd("melee_lab.css")).replace("__LAB_HTML__", rd("melee_lab.html"))
            .replace("__LAB_JS__", rd("melee_lab.js")).replace("__MECH_POSE__", rd("mech_pose.js"))
            .replace("__MECH_MELEE__", rd("mech_melee.js")).replace("__GLTF_JSON__", rd("gltf_json_loader.js"))
            .replace("__ORDER__", json.dumps(designs.ORDER))
            .replace("__MECH_BUTTONS__", "\n".join(f'    <button data-mech="{k}">{meta[k]["name"]}</button>' for k in designs.ORDER)))
    shutil.rmtree(SITE, ignore_errors=True)
    os.makedirs(SITE)
    with open(os.path.join(SITE, "index.html"), "w") as f:
        f.write(html)
    for k in designs.ORDER:
        designs.as_json_gltf(os.path.join(designs.MODELS, meta[k]["file"]), os.path.join(SITE, f"mech_{k}.gltf.json"))
    size = sum(os.path.getsize(os.path.join(SITE, f)) for f in os.listdir(SITE))
    print(f"wrote {SITE}: index.html ({len(html) / 1e3:.0f} kB) and {len(designs.ORDER)} mechs ({size / 1e6:.1f} MB in all)")


if __name__ == "__main__":
    main()
