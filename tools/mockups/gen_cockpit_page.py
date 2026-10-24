"""Build docs/mockups/cockpit.html: the Breeze cockpit seen from the pilot's eye.

    python3 tools/mockups/gen_cockpit_page.py

Reads assets/models/cockpit_breeze.glb and its manifest (written by tools/blender/build_cockpit.py)
and the panel contact sheet, and inlines them, so the page needs nothing but three.js r147 from
the CDN. The camera sits on the glb's EYE_pilot empty, as the game's will (the law: first person,
the cockpit is the mech); drag to turn the head, scroll to zoom. The baked surfaces are unlit, so
what the page shows is the bake itself. The world outside is a stand-in (sky, ground, a few blocks)
so the canopy has something to look at; the cockpit sits at the mech's eye height, 9.6 m.
"""
import base64
import io
import json
import os

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
GLB = os.path.join(ROOT, "assets", "models", "cockpit_breeze.glb")
MAN = os.path.join(ROOT, "assets", "models", "cockpit_breeze.json")
SHEET = os.path.join(ROOT, "docs", "screenshots", "cockpit", "panels-contact-sheet.png")
OUT = os.path.join(ROOT, "docs", "mockups", "cockpit.html")

PAGE = r"""<title>Breeze Cockpit</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: the pilot's view is the page; the spec sheet sits under it like a maintenance card. */
:root {
  --bg: #e9ebef; --panel: #f7f8fa; --edge: #c4cad3; --ink: #12161c; --muted: #56606d;
  --accent: #1f5fbf; --accent-soft: #dce6f6; --warn: #a86a00; --stage: #0b0d10;
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
header, main { max-width: 1400px; margin: 0 auto; }
header { display: grid; gap: 4px; margin-bottom: 14px; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; text-wrap: balance; }
h2 { font: 700 20px/1.1 var(--display); letter-spacing: 0.04em; text-transform: uppercase; margin: 0 0 8px; }
.lede { margin: 0; color: var(--muted); max-width: 80ch; }
.bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 6ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.stage { position: relative; width: 100%; aspect-ratio: 16 / 9; background: var(--stage); border-radius: 6px; overflow: hidden; touch-action: none; }
.stage canvas { display: block; width: 100%; height: 100%; cursor: grab; }
.stage .hint { position: absolute; left: 12px; bottom: 10px; font: 500 12px var(--mono); color: #c9d2dc; background: rgba(0,0,0,0.45); padding: 4px 8px; border-radius: 3px; pointer-events: none; }
.stage .load { position: absolute; inset: 0; display: grid; place-items: center; color: #c9d2dc; font: 500 14px var(--mono); }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; margin-top: 16px; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 6px; padding: 14px 16px; }
.card p { margin: 0 0 8px; }
.card ul { margin: 0; padding-left: 18px; }
.card li { margin: 2px 0; }
table { border-collapse: collapse; width: 100%; font: 13px var(--mono); }
td { padding: 3px 0; border-bottom: 1px solid var(--edge); }
td:last-child { text-align: right; font-variant-numeric: tabular-nums; }
.sheet { margin-top: 16px; }
.sheet img { width: 100%; height: auto; border-radius: 6px; border: 1px solid var(--edge); display: block; }
.note { color: var(--warn); font-size: 14px; }
</style>
<header>
  <div class="eyebrow">Crab Breeze / cockpit review / Breeze chassis</div>
  <h1>The Breeze cockpit, from the pilot's seat</h1>
  <p class="note">Work in progress, 2026-10-24: sealed, with an all-view screen wall in place of the glass (your direction). The boarding sequence (the mech sits down, hatches and flaps open, steam and mist) comes next on this page. Quick bake: 1024 px, 16 samples.</p>
  <p class="lede">This is the view you fight from. Drag to turn your head, scroll to zoom. The cockpit is sealed: the walls above the consoles are one wall of screens (Gundam's all-view monitor) showing what the mech's cameras see, with the HUD drawn on it. Every panel is modelled in Blender and baked to texture the Star Crew way (plates, screws, backlit buttons, guarded switches, dials, stencilled warnings, live circuit boards behind inspection windows), then the cabin's light and occlusion are baked over it: sky through the canopy, the glare shield and sill floodlights, the dome lamp, a red footwell lamp and the glow of the screens.</p>
</header>
<main>
  <div class="bar" role="group" aria-label="Look">
    <span class="lbl">Look</span>
    <button data-view="forward" aria-pressed="true">Forward</button>
    <button data-view="dash">Dash</button>
    <button data-view="left">Left console</button>
    <button data-view="right">Right console</button>
    <button data-view="knees">Knees</button>
    <button data-view="back">Over shoulder</button>
    <button data-view="orbit">Outside</button>
  </div>
  <div class="bar" role="group" aria-label="Show">
    <span class="lbl">Show</span>
    <button id="hud" aria-pressed="true">HUD</button>
    <button id="power" aria-pressed="true">Screens on</button>
    <button id="anim" aria-pressed="true">Controls moving</button>
  </div>
  <div class="stage" id="stage"><div class="load" id="load">Loading the cockpit</div><div class="hint">drag: look / scroll: zoom</div></div>
  <div class="cards">
    <div class="card">
      <h2>What you are looking at</h2>
      <ul>
        <li><b>Dash:</b> three MFDs (damage and heat, the tactical radar, weapons and jet fuel) in bezels with real push buttons; the caution and warning panel and master lights on the left; reactor and jet fuel standby gauges on the right; the mode row (walk, skate, jet).</li>
        <li><b>All-view screen:</b> the sealed shell is lined with screen tiles showing the outside; Screens on/off plays the tile by tile power-up. The HUD (pitch ladder, heading, speed, height, missile lock, Ace Combat style) is drawn on the screen ahead.</li>
        <li><b>Left console:</b> the throttle in its quadrant (idle, walk, run, skate, and the jet gate), reactor pumps, comms keypad.</li>
        <li><b>Right console:</b> the twist grip that turns the torso, the guarded master arm, weapon select, countermeasures.</li>
        <li><b>Knees:</b> standby gauges and the emergency panel in hazard stripes: reactor scram, jet cutoff, jettison arms, under red and yellow guards. The eject handle sits between your knees.</li>
        <li><b>Walls and bulkhead:</b> live circuit boards behind inspection windows with their 880 V warnings, the main breaker board, the reactor coolant lines, the canopy jettison handle.</li>
      </ul>
    </div>
    <div class="card">
      <h2>What it costs</h2>
      <table id="cost"></table>
      <p class="note" style="margin-top:8px">Measured off the glb and the bake. A cloud session has no GPU, so frame time is not measured here (CLAUDE.md 7).</p>
    </div>
    <div class="card">
      <h2>What is placeholder</h2>
      <ul>
        <li>The screens and HUD show pictures of what they will show; in the game each number is the simulation's own (CLAUDE.md 6.1), drawn live on the SCREEN_* and HUD_combiner nodes.</li>
        <li>The light was baked for the earlier glass canopy (sky through it); the next bake lights the sealed cabin from the screen wall and the cabin lamps.</li>
        <li>The world on the screens is a stand-in city captured from the eye.</li>
        <li>No pilot body or hands yet, and the world outside is a stand-in.</li>
      </ul>
    </div>
  </div>
  <div class="sheet">
    <h2>The panels, as modelled</h2>
    <p class="lede">Each panel rendered under a fixed key light at one scale. The cockpit uses its albedo, normal, occlusion and emission maps, not this picture.</p>
    <img alt="Contact sheet of every cockpit panel" src="data:image/jpeg;base64,__SHEET__">
  </div>
</main>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const GLB = "__GLB__";
const MAN = __MAN__;
const MECH_EYE_M = 9.6;
const stage = document.getElementById("stage");
const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
renderer.outputEncoding = THREE.sRGBEncoding;
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
stage.appendChild(renderer.domElement);
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(80, 16 / 9, 0.02, 4000);

// The world outside: a sky dome, the ground, a few city blocks. Stand-ins.
const sky = new THREE.Mesh(new THREE.SphereGeometry(3000, 32, 16), new THREE.ShaderMaterial({
  side: THREE.BackSide, depthWrite: false,
  vertexShader: "varying vec3 v; void main(){ v = normalize(position); gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }",
  fragmentShader: "varying vec3 v; void main(){ float h = v.y; vec3 hor = vec3(0.78,0.80,0.80); vec3 zen = vec3(0.30,0.46,0.74); vec3 c = mix(hor, zen, smoothstep(0.0,0.6,h)); if (h < 0.0) c = mix(hor, vec3(0.32,0.30,0.27), smoothstep(0.0,0.05,-h)); gl_FragColor = vec4(c,1.0); }"
}));
scene.add(sky);
scene.add(new THREE.HemisphereLight(0xc8d6ea, 0x4a4238, 0.9));
const sun = new THREE.DirectionalLight(0xfff1dc, 1.1); sun.position.set(300, 500, 200); scene.add(sun);
const ground = new THREE.Mesh(new THREE.PlaneGeometry(6000, 6000), new THREE.MeshLambertMaterial({ color: 0x5b5a52 }));
ground.rotation.x = -Math.PI / 2; scene.add(ground);
scene.fog = new THREE.Fog(0xb9c0c4, 300, 2600);
(function city() {
  let s = 7; const rnd = () => (s = (s * 16807) % 2147483647) / 2147483647;
  const mat = new THREE.MeshLambertMaterial({ color: 0x8a8f96 }), dark = new THREE.MeshLambertMaterial({ color: 0x6b7077 });
  for (let i = 0; i < 160; i++) {
    const a = rnd() * Math.PI * 2, r = 70 + rnd() * 900, w = 14 + rnd() * 30, d = 14 + rnd() * 30, h = 10 + rnd() * (r > 400 ? 120 : 45);
    const b = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), rnd() < 0.5 ? mat : dark);
    b.position.set(Math.sin(a) * r, h / 2, Math.cos(a) * r); b.rotation.y = Math.round(rnd() * 4) * Math.PI / 2 + 0.1; scene.add(b);
  }
  const road = new THREE.MeshLambertMaterial({ color: 0x3e3f42 });
  for (const [x, z, w, d] of [[0, 300, 22, 1200], [0, -300, 22, 1200], [200, 0, 1200, 22]]) {
    const r = new THREE.Mesh(new THREE.PlaneGeometry(w, d), road); r.rotation.x = -Math.PI / 2; r.position.set(x, 0.02, z); scene.add(r);
  }
})();

const rig = new THREE.Group(); rig.position.y = MECH_EYE_M - MAN.eye_m[1]; scene.add(rig);
let eye = new THREE.Vector3(...MAN.eye_m), nodes = {}, canopyNode = null, hudNode = null;
const look = { yaw: 0, pitch: -0.12, fov: 80 }, target = { yaw: 0, pitch: -0.12, fov: 80 };
let mode = "eye";
const orbit = new THREE.OrbitControls(camera, renderer.domElement); orbit.enabled = false; orbit.enableDamping = true;
const VIEWS = { forward: [0, -0.12, 80], dash: [0, -0.5, 52], left: [0.75, -0.85, 70], right: [-0.75, -0.85, 70], knees: [0, -1.05, 70], back: [2.5, -0.25, 80] };

function setView(v) {
  document.querySelectorAll("[data-view]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
  if (v === "orbit") {
    mode = "orbit"; orbit.enabled = true;
    const c = new THREE.Vector3(0, 0.8, 0.15).add(rig.position);
    camera.fov = 45; camera.updateProjectionMatrix(); camera.position.set(2.6, rig.position.y + 2.2, 3.4); orbit.target.copy(c); orbit.update();
    return;
  }
  mode = "eye"; orbit.enabled = false;
  const [y, p, f] = VIEWS[v]; target.yaw = y; target.pitch = p; target.fov = f;
}
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => setView(b.dataset.view)));
const toggles = { hud: true, power: true, anim: true };
for (const id of Object.keys(toggles)) document.getElementById(id).addEventListener("click", e => {
  toggles[id] = !toggles[id]; e.currentTarget.setAttribute("aria-pressed", String(toggles[id]));
});

// drag to look, wheel to zoom, keyboard arrows for the same
let drag = null;
renderer.domElement.addEventListener("pointerdown", e => { if (mode !== "eye") return; drag = [e.clientX, e.clientY]; renderer.domElement.setPointerCapture(e.pointerId); });
renderer.domElement.addEventListener("pointermove", e => {
  if (!drag) return; const k = 0.004 * target.fov / 80;
  target.yaw += (e.clientX - drag[0]) * k; target.pitch -= (e.clientY - drag[1]) * k; drag = [e.clientX, e.clientY]; clampLook();
});
renderer.domElement.addEventListener("pointerup", () => drag = null);
renderer.domElement.addEventListener("wheel", e => { if (mode !== "eye") return; e.preventDefault(); target.fov = Math.min(95, Math.max(30, target.fov + e.deltaY * 0.03)); }, { passive: false });
window.addEventListener("keydown", e => {
  if (mode !== "eye") return; const s = 0.08;
  if (e.key === "ArrowLeft") target.yaw += s; else if (e.key === "ArrowRight") target.yaw -= s;
  else if (e.key === "ArrowUp") target.pitch += s; else if (e.key === "ArrowDown") target.pitch -= s; else return;
  clampLook(); e.preventDefault();
});
function clampLook() { target.yaw = Math.max(-2.7, Math.min(2.7, target.yaw)); target.pitch = Math.max(-1.25, Math.min(1.2, target.pitch)); }

// The all-view screen (Gundam's panoramic monitor): the cockpit is sealed, and the inside of the
// shell is one wall of screens showing what the mech's cameras see. Here a cube camera at the eye
// renders the world once; the shader shows it by direction from the eye, with tile seams,
// scanlines, a phosphor grade, the HUD drawn on the screen ahead, and a tile by tile power-up.
const cubeRT = new THREE.WebGLCubeRenderTarget(1536, { generateMipmaps: true, minFilter: THREE.LinearMipmapLinearFilter });
const cubeCam = new THREE.CubeCamera(0.5, 5000, cubeRT);
scene.add(cubeCam);
const allview = new THREE.ShaderMaterial({
  side: THREE.DoubleSide,
  uniforms: { env: { value: cubeRT.texture }, eye: { value: new THREE.Vector3() }, power: { value: 1 }, hud: { value: null },
              hudOn: { value: 1 }, rigInv: { value: new THREE.Matrix4() }, time: { value: 0 } },
  vertexShader: "varying vec3 wp; void main(){ vec4 w = modelMatrix * vec4(position,1.0); wp = w.xyz; gl_Position = projectionMatrix * viewMatrix * w; }",
  fragmentShader: `
    uniform samplerCube env; uniform sampler2D hud; uniform vec3 eye; uniform float power, hudOn, time; uniform mat4 rigInv;
    varying vec3 wp;
    float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
    void main(){
      vec3 d = normalize(wp - eye);
      vec3 ld = normalize((rigInv * vec4(d, 0.0)).xyz);          // direction in the cockpit's frame
      float az = atan(ld.x, ld.z), el = asin(clamp(ld.y, -1.0, 1.0));
      vec2 tile = vec2(floor(az / 0.349), floor(el / 0.262));
      float on = smoothstep(0.0, 0.15, power * 1.3 - hash(tile) * 0.3);
      float flick = step(0.5, fract(sin(time * 40.0 + hash(tile) * 10.0) * 7.0)) * (1.0 - step(0.999, power));
      vec3 c = textureCube(env, d).rgb;
      c = pow(c, vec3(0.95)) * vec3(0.94, 1.0, 0.98) * 1.08;
      float sa = abs(fract(az / 0.349 + 0.5) - 0.5) * 0.349, se = abs(fract(el / 0.262 + 0.5) - 0.5) * 0.262;
      float seam = 1.0 - (1.0 - smoothstep(0.0, 0.003, sa)) * 0.85 - (1.0 - smoothstep(0.0, 0.003, se)) * 0.85;
      c *= clamp(seam, 0.0, 1.0);
      c *= 0.93 + 0.07 * sin(gl_FragCoord.y * 1.6);
      if (hudOn > 0.5 && abs(az) < 0.42 && el > -0.2 && el < 0.44) {
        vec2 uv = vec2(0.5 + az / 0.84, (el + 0.2) / 0.64);
        vec4 h = texture2D(hud, uv);
        c = mix(c, h.rgb * 1.2, h.a * 0.9);
      }
      vec3 off = vec3(0.012, 0.016, 0.02);
      c = mix(off, c, on * (1.0 - flick * 0.7));
      gl_FragColor = vec4(c, 1.0);
    }`
});
function captureWorld() {
  rig.visible = false; cubeCam.position.copy(eye).add(rig.position); cubeCam.update(renderer, scene); rig.visible = true;
}
let powerT = 1;

const bytes = Uint8Array.from(atob(GLB), c => c.charCodeAt(0));
new THREE.GLTFLoader().parse(bytes.buffer, "", gltf => {
  const root = gltf.scene; rig.add(root);
  root.traverse(o => {
    nodes[o.name] = o;
    if (o.isMesh) {
      for (const m of [].concat(o.material)) {
        if (m.map) { m.map.anisotropy = renderer.capabilities.getMaxAnisotropy(); m.map.needsUpdate = true; }
        if (o.name.startsWith("GLASS")) { m.transparent = true; m.depthWrite = false; m.envMapIntensity = 0.5; }
        if (o.name.startsWith("HUD")) { m.transparent = true; m.depthWrite = false; m.side = THREE.DoubleSide; m.blending = THREE.AdditiveBlending; }
      }
    }
  });
  const e = nodes["EYE_pilot"]; if (e) { e.updateWorldMatrix(true, false); eye = rig.worldToLocal(e.getWorldPosition(new THREE.Vector3())); }
  canopyNode = nodes["canopy"]; hudNode = nodes["HUD_combiner"];
  if (canopyNode) canopyNode.position.set(0, 0, 0);   // this export applied the hinge twice; the sealed pod build replaces the canopy
  const glass = nodes["GLASS_canopy"];
  if (glass) { glass.userData.glassMat = glass.material; glass.material = allview; }
  if (hudNode) { allview.uniforms.hud.value = hudNode.material.map; hudNode.visible = false; }
  rig.updateMatrixWorld(true); allview.uniforms.rigInv.value.copy(rig.matrixWorld).invert();
  captureWorld();
  for (const k of ["CTRL_stick", "CTRL_throttle", "CTRL_twist"]) if (nodes[k]) nodes[k].userData.rest = nodes[k].quaternion.clone();
  document.getElementById("load").remove();
  window.MOCKUP_READY = true;
}, err => { document.getElementById("load").textContent = "Could not load: " + err; });

const cost = document.getElementById("cost");
const rows = [["Triangles", MAN.tris.toLocaleString("en-GB") + " / " + MAN.tri_budget.toLocaleString("en-GB")]];
for (const [g, a] of Object.entries(MAN.atlases)) rows.push([`Baked atlas, ${g}`, `${a.px} x ${a.px}`]);
rows.push(["Texture memory (2 atlases, RGBA8, mips)", (Object.values(MAN.atlases).reduce((s, a) => s + a.px * a.px * 4 * 4 / 3, 0) / 1048576).toFixed(0) + " MiB"]);
rows.push(["Draw calls (materials)", String(MAN.draw_calls)]);
rows.push(["Panels modelled", String(MAN.panels.length)]);
rows.push(["Bake samples", String(MAN.bake.samples) + (MAN.bake.denoised ? ", denoised" : "")]);
rows.push(["glb size", (MAN.glb_bytes / 1048576).toFixed(1) + " MiB"]);
cost.innerHTML = rows.map(([a, b]) => `<tr><td>${a}</td><td>${b}</td></tr>`).join("");

const clock = new THREE.Clock();
function frame() {
  const dt = Math.min(0.05, clock.getDelta()), t = clock.elapsedTime;
  const w = stage.clientWidth, h = stage.clientHeight;
  if (renderer.domElement.width !== Math.round(w * renderer.getPixelRatio())) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
  if (mode === "eye") {
    const k = 1 - Math.exp(-dt * 8);
    look.yaw += (target.yaw - look.yaw) * k; look.pitch += (target.pitch - look.pitch) * k; look.fov += (target.fov - look.fov) * k;
    camera.fov = look.fov; camera.updateProjectionMatrix();
    camera.position.copy(eye).add(rig.position);
    camera.rotation.set(look.pitch, look.yaw + Math.PI, 0, "YXZ");   // the pilot faces +z; a three.js camera looks down -z
  } else orbit.update();
  if (hudNode) hudNode.visible = false;
  powerT = toggles.power ? Math.min(1, powerT + dt / 2.5) : Math.max(0, powerT - dt / 0.6);
  allview.uniforms.power.value = powerT; allview.uniforms.hudOn.value = toggles.hud && powerT > 0.95 ? 1 : 0;
  allview.uniforms.time.value = t; allview.uniforms.eye.value.copy(eye).add(rig.position);
  const glass = nodes["GLASS_canopy"];
  if (glass) glass.material = mode === "orbit" ? glass.userData.glassMat : allview;
  const sway = toggles.anim ? 1 : 0;
  const st = nodes["CTRL_stick"], th = nodes["CTRL_throttle"], tw = nodes["CTRL_twist"];
  if (st && st.userData.rest) st.quaternion.copy(st.userData.rest).multiply(new THREE.Quaternion().setFromEuler(new THREE.Euler(0.08 * Math.sin(t * 0.9) * sway, 0, 0.06 * Math.sin(t * 1.3) * sway)));
  if (th && th.userData.rest) th.quaternion.copy(th.userData.rest).multiply(new THREE.Quaternion().setFromEuler(new THREE.Euler(0.12 * Math.sin(t * 0.4) * sway, 0, 0)));
  if (tw && tw.userData.rest) tw.quaternion.copy(tw.userData.rest).multiply(new THREE.Quaternion().setFromEuler(new THREE.Euler(0, 0.3 * Math.sin(t * 0.6) * sway, 0)));
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}
frame();
window.MOCKUP = { nodes: () => nodes, allview, setView, set(id, v) { toggles[id] = v; document.getElementById(id).setAttribute("aria-pressed", String(v)); },
  look(y, p, f) { mode = "eye"; orbit.enabled = false; target.yaw = look.yaw = y; target.pitch = look.pitch = p; target.fov = look.fov = f; } };
</script>
"""


def main():
    man = json.load(open(MAN))
    glb = base64.b64encode(open(GLB, "rb").read()).decode("ascii")
    im = Image.open(SHEET).convert("RGB")
    im = im.resize((1600, int(im.size[1] * 1600 / im.size[0])), Image.LANCZOS)
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=82, optimize=True)
    sheet = base64.b64encode(buf.getvalue()).decode("ascii")
    html = PAGE.replace("__GLB__", glb).replace("__MAN__", json.dumps(man)).replace("__SHEET__", sheet)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(OUT, f"{len(html) / 1e6:.1f} MB")


main()
