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
import sys

import gen_mech_designs as designs

ROOT = designs.ROOT
SITE = os.path.join(ROOT, "output", "melee_site")
# Each mech's weight class for its stance: the one table, in the mech data generator.
sys.path.insert(0, os.path.join(ROOT, "tools", "mechs"))
from gen_mech_data import MECH_CLASS  # noqa: E402

PAGE = r"""<title>Mech Arms Lab</title>
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
.aim { background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 10px 12px; margin-bottom: 8px; display: grid; gap: 8px; }
.aim .sl { display: grid; grid-template-columns: 9ch 1fr 8ch; gap: 8px; align-items: center; flex: 1 1 260px; min-width: 0; }
.aim .sl span { font: 500 12px var(--mono); color: var(--muted); }
.aim .sl input { min-width: 0; accent-color: var(--accent); }
.aim .sl output { font: 500 12px var(--mono); font-variant-numeric: tabular-nums; text-align: right; }
.aim .row { display: flex; flex-wrap: wrap; gap: 6px 16px; align-items: center; }
.readout { font: 500 13px var(--mono); font-variant-numeric: tabular-nums; display: flex; flex-wrap: wrap; gap: 4px 18px; }
.readout b { font-weight: 500; color: var(--muted); }
.ok { color: #2e9a5a; } .far { color: var(--warn); }
.doc { max-width: 78ch; margin-top: 22px; }
.stance { position: absolute; right: 10px; top: 10px; width: 190px; background: rgba(14, 10, 18, 0.85); border: 1px solid #3a2f40; padding: 7px 9px; color: #e6ebf1; pointer-events: none; }
.st-h { font: 700 12px var(--display); letter-spacing: 0.08em; text-transform: uppercase; color: #b9b0c4; }
.st-bar { position: relative; height: 10px; background: #2a2330; margin: 5px 0 4px; border: 1px solid #4a3f52; }
.st-bar i { position: absolute; left: 0; top: 0; bottom: 0; width: 0; background: #f0b13c; }
.st-bar::after { content: ""; position: absolute; inset: 0; background: repeating-linear-gradient(90deg, transparent 0 calc(100% / var(--n, 3) - 2px), #1c1420 calc(100% / var(--n, 3) - 2px) calc(100% / var(--n, 3))); }
.st-s { font: 500 12px var(--mono); }
.st-s.broken { color: #ff7a5c; }
.doc h2 { font: 700 24px/1.1 var(--display); letter-spacing: 0.04em; text-transform: uppercase; margin: 22px 0 6px; }
.doc h3 { font: 700 17px var(--display); letter-spacing: 0.05em; text-transform: uppercase; margin: 16px 0 4px; }
.doc table { border-collapse: collapse; font-size: 14px; display: block; overflow-x: auto; }
.doc th, .doc td { text-align: left; padding: 4px 10px; border-bottom: 1px solid var(--edge); vertical-align: top; }
.doc code { font: 13px var(--mono); }
__LAB_CSS__
</style>

<div class="wrap">
  <div class="eyebrow">Crab Breeze · close combat</div>
  <h1>Mech Arms Lab</h1>
  <p class="lede">The mech's arms: aiming their guns at the point you aim at, and the melee moves and gun recoil.
  Move the target: the shoulders and elbows turn so each barrel points at it, as near as the joints allow, and the
  readout says how far each barrel is off. The write-up for the game is below the viewer. For melee, pick a move and
  press Play: the copy facing your mech takes the hit when the blow lands; Edit keys poses a move bone by bone, and
  Save keeps it. Three blows break the partner's stance and it falls.</p>
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
  <div class="aim" role="group" aria-label="Aim">
    <div class="row">
      <span style="font:500 12px var(--mono);color:var(--muted)">Aim</span>
      <button id="aim-on" aria-pressed="true">Arms aim</button>
      <button id="aim-torso" aria-pressed="true">Torso follows</button>
      <button id="aim-sweep" aria-pressed="false">Sweep target</button>
      <button id="aim-place" aria-pressed="false">Click ground to place</button>
      <button id="aim-close">Close target</button>
      <button id="aim-far">Far target</button>
    </div>
    <div class="row">
      <label class="sl"><span>Range</span><input type="range" id="aim-r" min="12" max="600" step="1" value="120"><output id="aim-ro">120 m</output></label>
      <label class="sl"><span>Bearing</span><input type="range" id="aim-b" min="-120" max="120" step="1" value="0"><output id="aim-bo">0 deg</output></label>
      <label class="sl"><span>Height</span><input type="range" id="aim-h" min="0" max="120" step="0.5" value="14"><output id="aim-ho">14 m</output></label>
    </div>
    <div class="readout" id="aim-read" aria-live="polite"></div>
  </div>
  <div class="bar" role="group" aria-label="Stance">
    <span class="lbl">Stance</span>
    <label style="font:500 12px var(--mono);color:var(--muted);display:flex;gap:6px;align-items:center">Blows to break
      <select id="st-break"><option value="2">2: light</option><option value="3" selected>3: mid</option><option value="4">4: heavy</option></select></label>
    <button id="st-reset">Stand it up</button>
    <span class="hint" style="margin:0">Picking a mech sets its class. Smash and charge count 1.5 blows; guns never count. Untouched for 3 s, the stance mends.</span>
  </div>
__LAB_HTML__
  <div class="stage">
    <canvas id="view" tabindex="0" aria-label="3D view of the mech and its sparring partner"></canvas>
    <div class="mode" id="mode">Side</div>
    <div class="load" id="load">Loading the mech</div>
    <div class="stance" id="stance" aria-live="polite"><div class="st-h">Partner stance</div><div class="st-bar"><i id="st-fill"></i></div><div class="st-s" id="st-s">Standing</div></div>
  </div>
  <p class="hint">Yellow is where each barrel points, dashed blue where its shot flies. Drag to orbit, scroll or pinch to zoom. Turns are radians, slides metres. On a bone, rx below zero swings it forward and up; ry above zero turns it toward the mech's left; a shoulder's rz lifts the arm out to the side (below zero on the right, above zero on the left).</p>
__DOC__
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
__ARM_AIM__
__MECH_STANCE__
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

// ---------- the aim target and the lines drawn from each muzzle ----------
const aimT = { on: true, torso: true, sweep: false, place: false, r: 120, b: 0, h: 14, pos: new T.Vector3() };
const marker = new T.Group();
marker.add(new T.Mesh(new T.SphereGeometry(0.9, 20, 14), new T.MeshBasicMaterial({ color: 0xff5a3c, toneMapped: false })));
marker.add(new T.Mesh(new T.TorusGeometry(2.2, 0.12, 8, 40), new T.MeshBasicMaterial({ color: 0xff5a3c, toneMapped: false })));
scene.add(marker);
const pole = new T.Mesh(new T.CylinderGeometry(0.08, 0.08, 1, 6), new T.MeshBasicMaterial({ color: 0xff5a3c, transparent: true, opacity: 0.5 }));
scene.add(pole);
function mkLine(col, dashed) {
  const g = new T.BufferGeometry().setFromPoints([new T.Vector3(), new T.Vector3(0, 0, 1)]);
  const m = dashed ? new T.LineDashedMaterial({ color: col, dashSize: 2, gapSize: 1.5, toneMapped: false }) : new T.LineBasicMaterial({ color: col, toneMapped: false });
  const l = new T.Line(g, m); l.frustumCulled = false; scene.add(l); return l;
}
// yellow: where the barrel points; dashed blue: where the shot flies (muzzle to target)
const lines = { L: { barrel: mkLine(0xffd23c, false), shot: mkLine(0x5ad1ff, true) }, R: { barrel: mkLine(0xffd23c, false), shot: mkLine(0x5ad1ff, true) } };
function setLine(l, a, c) { const p = l.geometry.attributes.position; p.setXYZ(0, a.x, a.y, a.z); p.setXYZ(1, c.x, c.y, c.z); p.needsUpdate = true; l.computeLineDistances(); l.visible = true; }
function placeTarget() {
  const br = aimT.b * Math.PI / 180;
  aimT.pos.set(Math.sin(br) * aimT.r, aimT.h, Math.cos(br) * aimT.r);
  marker.position.copy(aimT.pos); marker.visible = aimT.on;
  pole.position.set(aimT.pos.x, aimT.h / 2, aimT.pos.z); pole.scale.set(1, Math.max(0.01, aimT.h), 1); pole.visible = aimT.on;
}

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
    walk: parseX(x.walk), idleP: parseX(x.idle), hitT: 9, hitSide: 1, wWalk: 0, rest: {}, yaw0: 0,
    aim: aimState(), tw: 0, twRate: 0, pitch: 0, armW: { L: 1, R: 1 }, stance: stanceState() };
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
    me.root.updateMatrixWorld(true); me.eyeRest = (me.nodes.EYE_pilot || me.nodes.cockpit || me.root).getWorldPosition(new T.Vector3());
    classBreak(id); layout(); setView(view); ld.hidden = true; window.LAB_READY = true;
  }).catch(e => { ld.textContent = "Could not load: " + e; });
}
function layout() {
  if (!foe) return;
  foe.root.visible = mel.spar;
  foe.yaw0 = Math.PI; foe.root.rotation.y = Math.PI; if (!foe.home) foe.home = new T.Vector3(0, 0, SPAR_M); foe.root.position.copy(foe.home);
  me.home = new T.Vector3();
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
  stanceStep(b.stance, dt, stanceCfg);
  P.w = 1; const kd = stancePose(P, b, b.stance, stanceCfg);
  if (b.stance.landed) dustAt(b);
  // standing again where it slid to
  if (b.stance.phase === "up" && b.stance.back_m) { b.home.add(new T.Vector3(0, 0, -b.stance.back_m).applyAxisAngle(new T.Vector3(0, 1, 0), b.yaw0)); b.stance.back_m = 0; }
  const mv = isMe && mel.id && MELEE_MOVES[mel.id];
  if (mv) { P.w = 1; meleePose(P, b, mv, mel.t); }
  if (isMe) torsoFollow(b, P, dt);
  applyPose(b, P);
  // knocked down: rolled onto its side and tipped back about the heel line, slid back along its facing
  const rq = new T.Quaternion().setFromAxisAngle(new T.Vector3(0, 0, 1), kd.roll).multiply(new T.Quaternion().setFromAxisAngle(new T.Vector3(1, 0, 0), kd.tilt));
  b.root.quaternion.setFromEuler(new T.Euler(0, b.yaw0, 0)).multiply(rq);
  const heel = new T.Vector3(0, 0, stanceCfg.heel_m);
  b.root.position.copy(b.home || b.root.position).add(heel.clone().sub(heel.clone().applyQuaternion(rq)).add(new T.Vector3(0, 0, -kd.back_m)).applyAxisAngle(new T.Vector3(0, 1, 0), b.yaw0));
  b.root.updateMatrixWorld(true);
  if (!isMe) return;
  // an arm that is striking lets go of its aim and takes it back after (a gun's recoil keeps it)
  const striking = mv && !mv.fire && mel.t > 0 && mel.t < meleeEnd(mv);
  for (const S of ["L", "R"]) {
    const free = !(striking && (mv.arm === S || mv.arm === "both"));
    b.armW[S] += ((free ? 1 : 0) - b.armW[S]) * Math.min(1, dt * (free ? 3 : 10));
  }
  const err = aimStep(b, b.aim, aimT.on ? aimT.pos : null, dt, b.armW);
  drawAim(b, err);
}
// The torso turns to the aim as the game's does (cb-core locomotion, MechConfig defaults): twist toward the target's
// bearing at up to 70 deg/s with 600 deg/s^2 of ease, inside 100 deg; pitch toward its elevation from the eye at
// 40 deg/s, inside -20 to +25 deg.
const TORSO = { twist_rate: 70, twist_accel: 600, pitch_rate: 40, pitch_lo: -20, pitch_hi: 25, twist_max: 100 };
function torsoFollow(b, P, dt) {
  let wantT = 0, wantP = 0;
  if (aimT.on && aimT.torso) {
    const e = b.eyeRest || new T.Vector3(0, 18, 1);
    wantT = Math.atan2(aimT.pos.x, aimT.pos.z);
    wantT = Math.max(-TORSO.twist_max, Math.min(TORSO.twist_max, wantT * 180 / Math.PI)) * Math.PI / 180;
    wantP = Math.atan2(aimT.pos.y - e.y, Math.hypot(aimT.pos.x - e.x, aimT.pos.z - e.z));
    wantP = Math.max(TORSO.pitch_lo, Math.min(TORSO.pitch_hi, wantP * 180 / Math.PI)) * Math.PI / 180;
  }
  const vmax = TORSO.twist_rate * Math.PI / 180, amax = TORSO.twist_accel * Math.PI / 180, err = wantT - b.tw;
  const vwant = Math.sign(err) * Math.min(vmax, Math.sqrt(2 * amax * Math.abs(err)));
  b.twRate += Math.max(-amax * dt, Math.min(amax * dt, vwant - b.twRate));
  b.tw += b.twRate * dt;
  const pm = TORSO.pitch_rate * Math.PI / 180 * dt; b.pitch += Math.max(-pm, Math.min(pm, wantP - b.pitch));
  P.w = 1; P.add("torso_twist", 0, b.tw, 0); P.add("torso", -b.pitch, 0, 0);
}
const readEl = document.getElementById("aim-read");
let readN = 0;
function drawAim(b, err) {
  for (const S of ["L", "R"]) {
    const br = aimT.on && b.aim.gun[S] ? aimBarrel(b, S) : null;
    if (!br) { lines[S].barrel.visible = lines[S].shot.visible = false; continue; }
    setLine(lines[S].barrel, br.from, br.from.clone().addScaledVector(br.dir, br.from.distanceTo(aimT.pos)));
    setLine(lines[S].shot, br.from, aimT.pos);
  }
  if ((readN += 1) % 6) return;
  const f = S => { const e = err[S], n = S === "L" ? "Left" : "Right"; if (e == null) return `<span><b>${n} arm</b> ${b.nodes["MUZZLE_" + S] ? "a launcher, not aimed" : "no gun"}</span>`;
    const cls = e < 1 ? "ok" : e > 5 ? "far" : ""; return `<span><b>${n} gun off by</b> <span class="${cls}">${e.toFixed(1)} deg</span></span>`; };
  const q = b.aim, deg = x => (x * 180 / Math.PI).toFixed(0);
  readEl.innerHTML = aimT.on ? f("L") + f("R") +
    `<span><b>Torso</b> twist ${deg(b.tw)}, pitch ${deg(b.pitch)} deg</span>` +
    `<span><b>Right arm</b> shoulder ${deg(-q.R[0])} up, ${deg(q.R[1])} in, elbow ${deg(-q.R[2])} up</span>` +
    `<span><b>Left arm</b> shoulder ${deg(-q.L[0])} up, ${deg(-q.L[1])} in, elbow ${deg(-q.L[2])} up</span>` : "Aim is off: the arms hang at their pose.";
}

__LAB_JS__
// ---------- the partner's stance ----------
// dust where a fallen mech hits the ground: soft sprites that swell and fade over 2 s
const dustTex = (() => { const c = document.createElement("canvas"); c.width = c.height = 64; const q = c.getContext("2d");
  const g = q.createRadialGradient(32, 32, 0, 32, 32, 32); g.addColorStop(0, "rgba(200,185,165,0.9)"); g.addColorStop(1, "rgba(200,185,165,0)");
  q.fillStyle = g; q.fillRect(0, 0, 64, 64); return new T.CanvasTexture(c); })();
const dust = [];
function dustAt(b) {
  const at = (b.nodes.torso || b.root).getWorldPosition(new T.Vector3());
  for (let i = 0; i < 26; i++) {
    const m = new T.Sprite(new T.SpriteMaterial({ map: dustTex, transparent: true, depthWrite: false, opacity: 0.8 }));
    const a = Math.random() * Math.PI * 2, r = Math.random() * 9;
    m.position.set(at.x + Math.cos(a) * r, 0.8 + Math.random() * 2, at.z + Math.sin(a) * r);
    m.scale.setScalar(4 + Math.random() * 4); scene.add(m);
    dust.push({ m, t: 0, v: new T.Vector3(Math.cos(a) * (3 + Math.random() * 6), 1 + Math.random() * 3, Math.sin(a) * (3 + Math.random() * 6)) });
  }
}
function stepDust(dt) {
  for (const d of dust) { d.t += dt; d.m.position.addScaledVector(d.v, dt); d.v.multiplyScalar(Math.exp(-dt * 1.5)); d.m.scale.multiplyScalar(1 + dt * 0.8); d.m.material.opacity = 0.8 * Math.max(0, 1 - d.t / 2.2); }
  while (dust.length && dust[0].t > 2.2) { const d = dust.shift(); scene.remove(d.m); d.m.material.dispose(); }
}
const stanceCfg = Object.assign({}, STANCE_DEFAULTS);
// each mech's class (light 2 blows, mid 3, heavy 4), as in data/mechs/<id>.ron melee.stance_break_at
const MECH_CLASS = __MECH_CLASS__;
const stFill = document.getElementById("st-fill"), stS = document.getElementById("st-s");
let stMsg = "", stMsgT = 0;
function stanceFlash(r) { if (r === "broken") { stMsg = "Stance broken"; stMsgT = 1.0; } else if (r === "ignored") { stMsg = "Already down"; stMsgT = 0.8; } }
function drawStance(dt) {
  if (!foe) return;
  const st = foe.stance, k = st.phase === "up" ? st.v / stanceCfg.break_at : 1;
  stFill.style.width = (k * 100).toFixed(1) + "%";
  stFill.style.background = st.phase === "up" ? "#f0b13c" : "#ff5a3c";
  stMsgT -= dt;
  const text = stMsgT > 0 ? stMsg : { up: st.v > 0.01 ? `Staggered, ${st.v.toFixed(1)} of ${stanceCfg.break_at}` : "Standing", fall: "Falling", down: "Down", getup: "Getting up" }[st.phase];
  stS.textContent = text; stS.classList.toggle("broken", st.phase !== "up" || stMsgT > 0);
  document.getElementById("stance").hidden = !mel.spar;
}
document.getElementById("st-break").addEventListener("change", e => { stanceCfg.break_at = +e.target.value;
  document.querySelector(".st-bar").style.setProperty("--n", e.target.value); });
document.getElementById("st-reset").addEventListener("click", () => { if (foe) { foe.stance = stanceState(); foe.home = null; layout(); } });
function classBreak(id) { const c = MECH_CLASS[id] || "mid"; stanceCfg.break_at = STANCE_BREAK_BY_CLASS[c];
  const sel = document.getElementById("st-break"); sel.value = String(stanceCfg.break_at); document.querySelector(".st-bar").style.setProperty("--n", sel.value); }
// ---------- the aim widget ----------
const aimIn = { r: document.getElementById("aim-r"), b: document.getElementById("aim-b"), h: document.getElementById("aim-h") };
const aimOut = { r: document.getElementById("aim-ro"), b: document.getElementById("aim-bo"), h: document.getElementById("aim-ho") };
function syncAim() {
  for (const k of ["r", "b", "h"]) { aimIn[k].value = aimT[k]; aimOut[k].textContent = (+aimT[k]).toFixed(k === "h" ? 1 : 0) + (k === "b" ? " deg" : " m"); }
  placeTarget();
}
function setPressed(id, on) { document.getElementById(id).setAttribute("aria-pressed", String(on)); return on; }
for (const k of ["r", "b", "h"]) aimIn[k].addEventListener("input", () => { aimT[k] = +aimIn[k].value; if (k === "b") aimT.sweep = setPressed("aim-sweep", false); syncAim(); });
document.getElementById("aim-on").addEventListener("click", () => { aimT.on = setPressed("aim-on", !aimT.on); placeTarget(); });
document.getElementById("aim-torso").addEventListener("click", () => { aimT.torso = setPressed("aim-torso", !aimT.torso); });
document.getElementById("aim-sweep").addEventListener("click", () => { aimT.sweep = setPressed("aim-sweep", !aimT.sweep); });
document.getElementById("aim-place").addEventListener("click", () => { aimT.place = setPressed("aim-place", !aimT.place); });
document.getElementById("aim-close").addEventListener("click", () => { Object.assign(aimT, { r: 22, b: 8, h: 10 }); syncAim(); });
document.getElementById("aim-far").addEventListener("click", () => { Object.assign(aimT, { r: 400, b: -12, h: 30 }); syncAim(); });
// a click on the ground (not a drag) moves the target there, at its height
const rc = new T.Raycaster(), ndc = new T.Vector2();
let downAt = null;
canvas.addEventListener("pointerdown", ev => { downAt = [ev.clientX, ev.clientY]; });
canvas.addEventListener("pointerup", ev => {
  if (!aimT.place || ev.button !== 0 || !downAt || Math.hypot(ev.clientX - downAt[0], ev.clientY - downAt[1]) > 5) return;
  const r = canvas.getBoundingClientRect(); ndc.set((ev.clientX - r.left) / r.width * 2 - 1, -((ev.clientY - r.top) / r.height) * 2 + 1);
  rc.setFromCamera(ndc, camera); const h = rc.intersectObject(ground)[0]; if (!h) return;
  aimT.r = Math.round(Math.max(12, Math.min(600, Math.hypot(h.point.x, h.point.z))));
  aimT.b = Math.round(Math.max(-120, Math.min(120, Math.atan2(h.point.x, h.point.z) * 180 / Math.PI)));
  syncAim();
});
let sweepT = 0;
function stepAim(dt) { if (!aimT.sweep) return; sweepT += dt; aimT.b = Math.round(55 * Math.sin(sweepT * 0.5)); syncAim(); }
syncAim();

let last = 0;
function loop(ms) {
  const dt = Math.min(0.05, (ms - last) / 1000 || 0.016); last = ms;
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.round(w * renderer.getPixelRatio())) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
  if (me) {
    stepAim(dt);
    stepMelee(dt);
    animate(me, dt, true);
    if (foe.root.visible) animate(foe, dt, false);
    drawStance(dt); stepDust(dt);
    if (view === "eye") { const e = eye(), dl = e.clone().sub(camera.position); camera.position.add(dl); controls.target.add(dl); }
  }
  controls.update();
  renderer.render(scene, camera);
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);
setMech(ORDER[0]);
window.LAB = { setMech, setView, aim(r, b, h) { Object.assign(aimT, { r, b, h }); syncAim(); }, stance: () => foe && Object.assign({}, foe.stance), state: () => ({ err: me && me.aim.err, tw: me && me.tw, q: me && me.aim }), melee(id, t) { pickMove(id); if (t !== undefined) { mel.playing = false; mel.t = t; drawMelee(); } } };
</script>
"""


def main():
    meta = json.load(open(os.path.join(designs.MODELS, "mechs.json")))
    rd = lambda n: open(os.path.join(ROOT, "tools", "mockups", n)).read()
    html = (PAGE.replace("__DOC__", rd("arm_aim_doc.html") + rd("stance_doc.html")).replace("__LAB_CSS__", rd("melee_lab.css")).replace("__LAB_HTML__", rd("melee_lab.html"))
            .replace("__LAB_JS__", rd("melee_lab.js")).replace("__MECH_POSE__", rd("mech_pose.js"))
            .replace("__MECH_MELEE__", rd("mech_melee.js")).replace("__ARM_AIM__", rd("arm_aim.js")).replace("__MECH_STANCE__", rd("mech_stance.js")).replace("__GLTF_JSON__", rd("gltf_json_loader.js"))
            .replace("__ORDER__", json.dumps(designs.ORDER)).replace("__MECH_CLASS__", json.dumps(MECH_CLASS))
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
