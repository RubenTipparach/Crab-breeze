"""Build docs/mockups/cockpit.html: boarding the Breeze, and its sealed cockpit from the seat.

    python3 tools/mockups/gen_cockpit_page.py

Reads assets/models/cockpit_breeze.glb (tools/blender/build_cockpit.py), assets/models/mech_breeze.glb
(tools/blender/build_mechs.py), the cockpit manifest and the panel contact sheet, and inlines them,
so the page needs nothing but three.js r147 from the CDN. The pose layer is tools/mockups/mech_pose.js
(Mining Mike's, plus the sit pose), inlined: the same file the mech page uses.

The owner, 2026-10-24: "The cockpit should be enclosed fully. We can view outside via screens like in
gundam. For the mockups have the mech sit down on the ground so that pilots can enter/exit. The cockpit
should open up with intricate doors and flaps and puffs of steam and myst coming out. I want getting
in and out of a mech to be a beautiful cinematic event." The screen wall follows the references the
owner sent the same day (Armored Core, Titanfall, the Gundam pods): a big front screen, angled side
screens, heavy pillars, monitors hanging from the overhead, the HUD drawn on the screens.

The boarding timeline is the first draft of openspec/changes/mech-boarding (design section 3). The
game's camera is only ever the pilot's eyes (the law); the outside cameras on this page are review
tools, and "Board it" plays the whole thing from the pilot's eyes.
"""
import base64
import io
import json
import os

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
GLB = os.path.join(ROOT, "assets", "models", "cockpit_breeze.glb")
MAN = os.path.join(ROOT, "assets", "models", "cockpit_breeze.json")
MECH = os.path.join(ROOT, "assets", "models", "mech_breeze.glb")
POSE = os.path.join(ROOT, "tools", "mockups", "mech_pose.js")
HUD = os.path.join(ROOT, "assets", "textures", "cockpit", "displays", "hud.png")
SHEET = os.path.join(ROOT, "docs", "screenshots", "cockpit", "panels-contact-sheet.png")
OUT = os.path.join(ROOT, "docs", "mockups", "cockpit.html")

PAGE = r"""<title>Breeze Cockpit</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: the stage is the page; the controls sit above it like a director's slate, the spec cards under it. */
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
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 7ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button.go { border-color: var(--accent); }
button:focus-visible, input:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.scrub { display: flex; gap: 10px; align-items: center; flex: 1 1 260px; min-width: 0; }
.scrub input { flex: 1; min-width: 0; accent-color: var(--accent); }
.scrub output { font: 500 13px var(--mono); color: var(--muted); font-variant-numeric: tabular-nums; min-width: 8ch; }
.stage { position: relative; width: 100%; max-width: 100%; aspect-ratio: 16 / 9; background: var(--stage); border-radius: 6px; overflow: hidden; touch-action: none; }
.stage canvas { display: block; width: 100%; height: 100%; cursor: grab; }
.stage .hint { position: absolute; left: 12px; bottom: 10px; font: 500 12px var(--mono); color: #c9d2dc; background: rgba(0,0,0,0.45); padding: 4px 8px; border-radius: 3px; pointer-events: none; }
.stage .cue { position: absolute; right: 12px; top: 10px; font: 700 15px var(--display); letter-spacing: 0.08em; text-transform: uppercase; color: #ffd28a; background: rgba(0,0,0,0.45); padding: 4px 10px; border-radius: 3px; pointer-events: none; }
.stage .load { position: absolute; inset: 0; display: grid; place-items: center; color: #c9d2dc; font: 500 14px var(--mono); }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; margin-top: 16px; }
.card { background: var(--panel); border: 1px solid var(--edge); border-radius: 6px; padding: 14px 16px; min-width: 0; }
.card ul, .card ol { margin: 0; padding-left: 20px; }
.card li { margin: 2px 0; }
.tl { font-variant-numeric: tabular-nums; }
.tl b { font-family: var(--mono); font-weight: 500; color: var(--muted); }
table { border-collapse: collapse; width: 100%; font: 13px var(--mono); }
td { padding: 3px 0; border-bottom: 1px solid var(--edge); }
td:last-child { text-align: right; font-variant-numeric: tabular-nums; }
.sheet { margin-top: 16px; }
.sheet img { width: 100%; height: auto; border-radius: 6px; border: 1px solid var(--edge); display: block; }
.note { color: var(--warn); font-size: 14px; margin: 0; }
</style>
<header>
  <div class="eyebrow">Crab Breeze / cockpit review / Breeze chassis</div>
  <h1>Boarding the Breeze</h1>
  <p class="lede">The mech sits down, unlatches, and opens its chest in stages: the armoured visor lifts on its rams with the front screen inside it, the side screens swing out as doors, the flaps fold back, the lower plate drops into a step and the stair unfolds, steam bursting from every seam and cold mist rolling out over the sill. Board it to climb up, sit, and watch it seal and the screens come alive. Inside, the cockpit is sealed: you see out through a wall of screens, as in the references you sent.</p>
  <p class="note" id="wip"></p>
</header>
<main>
  <div class="bar" role="group" aria-label="Sequence">
    <span class="lbl">Mech</span>
    <button class="go" id="open">Sit and open</button>
    <button class="go" id="board">Board it</button>
    <button class="go" id="leave">Climb out</button>
    <button class="go" id="close">Close and stand</button>
    <div class="scrub"><input type="range" id="scrub" min="0" max="1000" value="0" aria-label="Timeline"><output id="tlabel">0.0 s</output></div>
  </div>
  <div class="bar" role="group" aria-label="Camera">
    <span class="lbl">Camera</span>
    <button data-cam="review" aria-pressed="true">Walk round</button>
    <button data-cam="hatch" aria-pressed="false">Hatch close up</button>
    <button data-cam="side" aria-pressed="false">Side</button>
    <button data-cam="seat" aria-pressed="false">In the seat</button>
  </div>
  <div class="bar" role="group" aria-label="Show">
    <span class="lbl">Show</span>
    <button id="hud" aria-pressed="true">HUD</button>
    <button id="power" aria-pressed="true">Screens on</button>
    <button id="steam" aria-pressed="true">Steam and mist</button>
  </div>
  <div class="stage" id="stage"><div class="load" id="load">Loading the mech</div><div class="cue" id="cue" hidden></div><div class="hint" id="hint">drag: orbit / scroll: zoom</div></div>
  <div class="cards">
    <div class="card">
      <h2>The sequence</h2>
      <ol class="tl" id="tl"></ol>
    </div>
    <div class="card">
      <h2>What you are looking at</h2>
      <ul>
        <li><b>The pod:</b> a sealed armoured cockpit at the front of the chest. Everything that moves is a named node the game animates: visor, rams, latches, side doors, flaps, lower plate, three stair sections, the front console, the two grips.</li>
        <li><b>The screen wall:</b> one big front screen (it lifts with the visor), an angled screen each side (they are the doors), two behind your shoulders, three monitors hanging from the overhead. They show what the mech's cameras see, with the HUD drawn on the front.</li>
        <li><b>Dash and consoles:</b> three MFDs, caution and warning panel, standby gauges, the emergency panel under red guards, the throttle grip in its quadrant with the jet gate, the flight grip on the right, master arm, cyan edge lights.</li>
        <li><b>Inside the walls:</b> live circuit boards behind inspection windows with their 880 V warnings, breakers, reactor coolant lines.</li>
      </ul>
    </div>
    <div class="card">
      <h2>What it costs</h2>
      <table id="cost"></table>
      <p class="note" style="margin-top:8px">Measured off the glb and the bake. A cloud session has no GPU, so frame time is not measured here (CLAUDE.md 7). The screens cost extra renders of the world in the game: measure on your PC first.</p>
    </div>
    <div class="card">
      <h2>Placeholder for now</h2>
      <ul>
        <li>The screens show pictures of what they will show; in the game each number is the simulation's own.</li>
        <li>No pilot body or hands, no sound, a stand-in city.</li>
        <li>The cabin's light is baked from its own lamps and the screens' glow; the outside armour is lit live.</li>
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
const POD_GLB = "__GLB__", MECH_GLB = "__MECH__";
const MAN = __MAN__;
const WIP = __WIP__;
__MECH_POSE__

// ---------- the boarding timeline (openspec/changes/mech-boarding, design section 3) ----------
// Each track: a node, the axis it turns about (or the axis it slides along), its open value, the
// start and the duration in seconds. Closing plays the same tracks backwards.
const OPEN_END = 9.0, SEALED_AT = 3.5;
const TRACKS = [
  { what: "sit", t0: 0.0, d: 3.5 },
  { what: "latch", t0: 3.6, d: 0.45 },
  { node: "HATCH_upper", axis: "x", v: -1.65, t0: 4.6, d: 1.6 },
  { node: "DOOR_L", axis: "y", v: 1.3, t0: 5.2, d: 1.4 },
  { node: "DOOR_R", axis: "y", v: -1.3, t0: 5.2, d: 1.4 },
  { node: "FLAP_L", axis: "y", v: 1.15, t0: 5.45, d: 1.2 },
  { node: "FLAP_R", axis: "y", v: -1.15, t0: 5.45, d: 1.2 },
  { node: "HATCH_lower", axis: "x", v: 1.8, t0: 5.9, d: 1.2 },
  { node: "STAIR_1", axis: "x", v: -0.9, t0: 6.9, d: 0.8 },
  { node: "STAIR_2", axis: "x", v: 3.05, t0: 7.3, d: 0.9 },
  { node: "STAIR_3", pos: "y", v: 0.42, t0: 8.0, d: 0.6 },
  { node: "CONSOLE_front", axis: "x", v: -1.0, t0: 7.4, d: 1.3 },
];
const VENT_WINDOWS = { latch: [3.65, 4.1], seam: [4.15, 4.9], visor: [4.6, 6.2], doors: [5.2, 6.5], flaps: [5.45, 6.5], open: [5.9, 9.0] };
const CUES = [[0.0, "Sitting down"], [3.6, "Latches release"], [4.15, "Pressure equalising"], [4.6, "Visor lifts"], [5.2, "Side screens swing out"],
  [5.45, "Flaps fold back"], [5.9, "Step drops"], [6.9, "Stair unfolds"], [7.4, "Console clears"], [8.6, "Open"]];
document.getElementById("tl").innerHTML = CUES.map(([t, c]) => `<li><b>${t.toFixed(1)} s</b> ${c}</li>`).join("") +
  "<li><b>board</b> the pilot climbs the stair, ducks in, turns and sits; the hatch closes in reverse, a last hiss of steam seals it, the screens power up screen by screen, and the mech stands</li>";
if (WIP) document.getElementById("wip").textContent = WIP;

const stage = document.getElementById("stage");
const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
renderer.outputEncoding = THREE.sRGBEncoding;
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.shadowMap.enabled = true;
stage.appendChild(renderer.domElement);
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(45, 16 / 9, 0.02, 5000);

// The world: a late afternoon sky, the ground, a pad, a few city blocks. Stand-ins.
scene.add(new THREE.Mesh(new THREE.SphereGeometry(3000, 32, 16), new THREE.ShaderMaterial({
  side: THREE.BackSide, depthWrite: false,
  vertexShader: "varying vec3 v; void main(){ v = normalize(position); gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }",
  fragmentShader: "varying vec3 v; void main(){ float h = v.y; vec3 hor = vec3(0.86,0.78,0.70); vec3 zen = vec3(0.30,0.42,0.66); vec3 c = mix(hor, zen, smoothstep(0.0,0.55,h)); if (h < 0.0) c = mix(hor, vec3(0.30,0.28,0.26), smoothstep(0.0,0.05,-h)); gl_FragColor = vec4(c,1.0); }"
})));
scene.add(new THREE.HemisphereLight(0xc6d2e6, 0x4a4238, 0.75));
const sun = new THREE.DirectionalLight(0xffe2bf, 1.5);
sun.position.set(-30, 26, 22); sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -14, right: 14, top: 14, bottom: -14, near: 1, far: 120 });
scene.add(sun);
const ground = new THREE.Mesh(new THREE.PlaneGeometry(6000, 6000), new THREE.MeshLambertMaterial({ color: 0x5d5b54 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const pad = new THREE.Mesh(new THREE.CircleGeometry(9, 48), new THREE.MeshLambertMaterial({ color: 0x47474a }));
pad.rotation.x = -Math.PI / 2; pad.position.y = 0.01; pad.receiveShadow = true; scene.add(pad);
scene.fog = new THREE.Fog(0xc4bfb8, 200, 2400);
(function city() {
  let s = 7; const rnd = () => (s = (s * 16807) % 2147483647) / 2147483647;
  const mat = new THREE.MeshLambertMaterial({ color: 0x8a8f96 }), dark = new THREE.MeshLambertMaterial({ color: 0x6b7077 });
  for (let i = 0; i < 170; i++) {
    const a = rnd() * Math.PI * 2, r = 45 + rnd() * 900, w = 14 + rnd() * 30, d = 14 + rnd() * 30, h = 10 + rnd() * (r > 400 ? 120 : 45);
    const b = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), rnd() < 0.5 ? mat : dark);
    b.position.set(Math.sin(a) * r, h / 2, Math.cos(a) * r); b.rotation.y = Math.round(rnd() * 4) * Math.PI / 2 + 0.1; scene.add(b);
  }
})();

// ---------- steam and mist: one point cloud of soft discs, aged on the CPU ----------
const N_P = 2400;
const pGeo = new THREE.BufferGeometry();
const pPos = new Float32Array(N_P * 3), pSize = new Float32Array(N_P), pAlpha = new Float32Array(N_P), pTint = new Float32Array(N_P);
pGeo.setAttribute("position", new THREE.BufferAttribute(pPos, 3));
pGeo.setAttribute("size", new THREE.BufferAttribute(pSize, 1));
pGeo.setAttribute("alpha", new THREE.BufferAttribute(pAlpha, 1));
pGeo.setAttribute("tint", new THREE.BufferAttribute(pTint, 1));
const parts = Array.from({ length: N_P }, () => ({ live: false, p: new THREE.Vector3(), v: new THREE.Vector3(), age: 0, life: 1, s0: 0.1, s1: 1, a: 0.5, mist: false }));
const pMat = new THREE.ShaderMaterial({
  transparent: true, depthWrite: false,
  uniforms: { scale: { value: 800 } },
  vertexShader: `attribute float size; attribute float alpha; attribute float tint; varying float va; varying float vt; uniform float scale;
    void main(){ vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_Position = projectionMatrix * mv; gl_PointSize = size * scale / max(0.1, -mv.z); va = alpha; vt = tint; }`,
  fragmentShader: `varying float va; varying float vt;
    void main(){ vec2 c = gl_PointCoord - 0.5; float r = length(c) * 2.0; if (r > 1.0) discard;
      float a = (1.0 - r * r) * (1.0 - r * r) * va; vec3 col = mix(vec3(0.93, 0.94, 0.95), vec3(0.80, 0.87, 0.93), vt); gl_FragColor = vec4(col, a); }`
});
const cloud = new THREE.Points(pGeo, pMat); cloud.frustumCulled = false; cloud.renderOrder = 5; scene.add(cloud);
let pNext = 0;
function emit(p, dir, mist, k) {
  const q = parts[pNext]; pNext = (pNext + 1) % N_P;
  q.live = true; q.age = 0; q.mist = mist; q.p.copy(p);
  const j = new THREE.Vector3(Math.random() - 0.5, Math.random() - 0.5, Math.random() - 0.5);
  if (mist) { q.v.copy(dir).multiplyScalar(0.35 + Math.random() * 0.4).addScaledVector(j, 0.25); q.life = 4 + Math.random() * 3; q.s0 = 0.25; q.s1 = 1.6 + Math.random(); q.a = 0.22; }
  else { q.v.copy(dir).multiplyScalar(1.6 + Math.random() * 2.2 * k).addScaledVector(j, 0.9); q.life = 1.2 + Math.random() * 1.3; q.s0 = 0.08; q.s1 = 0.9 + Math.random() * 0.8; q.a = 0.55; }
}
function stepParticles(dt) {
  for (let i = 0; i < N_P; i++) {
    const q = parts[i];
    if (q.live) {
      q.age += dt; if (q.age > q.life) q.live = false;
      const u = q.age / q.life;
      if (q.mist) { q.v.y -= 0.35 * dt; q.v.multiplyScalar(1 - 0.5 * dt); if (q.p.y < 0.15) { q.p.y = 0.15; q.v.y = 0; q.v.x *= 1 + dt; q.v.z *= 1 + dt * 0.5; } }
      else { q.v.multiplyScalar(1 - 2.2 * dt); q.v.y += 0.9 * dt; }
      q.p.addScaledVector(q.v, dt);
      pPos[i * 3] = q.p.x; pPos[i * 3 + 1] = q.p.y; pPos[i * 3 + 2] = q.p.z;
      pSize[i] = q.s0 + (q.s1 - q.s0) * Math.sqrt(u);
      pAlpha[i] = q.a * Math.min(1, u * 8) * (1 - u) * (1 - u);
      pTint[i] = q.mist ? 1 : 0;
    } else { pAlpha[i] = 0; pSize[i] = 0; }
  }
  for (const k of ["position", "size", "alpha", "tint"]) pGeo.attributes[k].needsUpdate = true;
}

// ---------- the screen wall: the world as the mech's cameras see it ----------
// A cube camera at the eye renders the world; each screen shows it by direction from the eye, so
// the screens read as windows. Scanlines, a phosphor grade, the HUD on the front screen, and a
// screen by screen power-up.
const cubeRT = new THREE.WebGLCubeRenderTarget(1024, { generateMipmaps: true, minFilter: THREE.LinearMipmapLinearFilter });
const cubeCam = new THREE.CubeCamera(0.3, 5000, cubeRT);
scene.add(cubeCam);
const allview = new THREE.ShaderMaterial({
  side: THREE.DoubleSide,
  uniforms: { env: { value: cubeRT.texture }, eye: { value: new THREE.Vector3() }, power: { value: 1 }, hud: { value: null },
              hudOn: { value: 1 }, podInv: { value: new THREE.Matrix4() }, time: { value: 0 }, podC: { value: new THREE.Vector3(0, 0.96, 0.08) } },
  vertexShader: "varying vec3 wp; void main(){ vec4 w = modelMatrix * vec4(position,1.0); wp = w.xyz; gl_Position = projectionMatrix * viewMatrix * w; }",
  fragmentShader: `
    uniform samplerCube env; uniform sampler2D hud; uniform vec3 eye, podC; uniform float power, hudOn, time; uniform mat4 podInv;
    varying vec3 wp;
    float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
    void main(){
      vec3 d = normalize(wp - eye);
      vec3 ld = normalize((podInv * vec4(d, 0.0)).xyz);
      vec3 lp = (podInv * vec4(wp, 1.0)).xyz - podC;
      float paz = atan(lp.x, lp.z), pel = atan(lp.y, length(lp.xz));
      vec2 tile = vec2(floor(paz / 0.9 + 0.5), floor(pel / 0.6));
      float on = smoothstep(0.0, 0.12, power * 1.35 - hash(tile) * 0.35);
      float flick = step(0.5, fract(sin(time * 40.0 + hash(tile) * 10.0) * 7.0)) * (1.0 - step(0.999, power));
      vec3 c = textureCube(env, d).rgb;
      c = pow(c, vec3(0.95)) * vec3(0.94, 1.0, 0.99) * 1.05;
      c *= 0.93 + 0.07 * sin(gl_FragCoord.y * 1.6);
      float az = atan(ld.x, ld.z), el = asin(clamp(ld.y, -1.0, 1.0));
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
(function hudTexture() {
  const img = new Image();
  img.onload = () => { const t = new THREE.Texture(img); t.encoding = THREE.sRGBEncoding; t.needsUpdate = true; allview.uniforms.hud.value = t; };
  img.src = "data:image/png;base64,__HUD__";
})();

// ---------- load the mech and the pod ----------
const b64 = s => Uint8Array.from(atob(s), c => c.charCodeAt(0)).buffer;
const loader = new THREE.GLTFLoader();
const N = {}, P = {};   // the mech's nodes, the pod's nodes
let mech = null, pod = null, B = null;
const rest = {}, latches = [], vents = [];
const parse = buf => new Promise((res, rej) => loader.parse(buf, "", res, rej));
Promise.all([parse(b64(MECH_GLB)), parse(b64(POD_GLB))]).then(([gm, gp]) => {
  mech = gm.scene; pod = gp.scene;
  mech.traverse(o => { if (o.name) N[o.name] = o; if (o.isMesh) o.castShadow = o.receiveShadow = true; });
  pod.traverse(o => {
    if (o.name) P[o.name] = o;
    if (!o.isMesh) return;
    o.castShadow = o.receiveShadow = true;
    for (const m of [].concat(o.material)) if (m.map) m.map.anisotropy = renderer.capabilities.getMaxAnisotropy();
    if ([].concat(o.material).some(m => m.name === "SCREEN_allview")) { o.material = allview; o.castShadow = false; }
  });
  scene.add(mech);
  const x = (N.root && N.root.userData) || {}, px = v => (typeof v === "string" ? JSON.parse(v) : v);
  B = { nodes: N, t: 0, seed: 3, scale: x.scale || 0.13, legSegs: px(x.legSegs), footH: x.footH, walk: px(x.walk), idleP: px(x.idle), rest: {} };
  for (const n in N) if (MAPPED.has(n)) { const o = N[n]; B.rest[n] = { e: new THREE.Euler().setFromQuaternion(o.quaternion, "XYZ"), p: o.position.clone() }; }
  // mount the pod so its eye is the mech's eye
  pod.position.copy(N.EYE_pilot.position).sub(new THREE.Vector3(...MAN.eye_m));
  N.cockpit.add(pod);
  for (const name in P) {
    const o = P[name];
    rest[name] = { q: o.quaternion.clone(), p: o.position.clone() };
    if (name.startsWith("LATCH_")) latches.push(o);
    if (name.startsWith("VENT_")) vents.push(o);
  }
  document.getElementById("load").remove();
  setCam("review");
  window.MOCKUP_READY = true;
}).catch(e => { document.getElementById("load").textContent = "Could not load: " + e; });

// ---------- driving the timeline ----------
const AX = { x: new THREE.Vector3(1, 0, 0), y: new THREE.Vector3(0, 1, 0), z: new THREE.Vector3(0, 0, 1) };
let T = 0, target = 0, rate = 1, sitK = 0, powerT = 1;
function setNodes(t) {
  for (const tr of TRACKS) {
    const k = ease((t - tr.t0) / tr.d);
    if (tr.what === "sit") { sitK = k; continue; }
    if (tr.what === "latch") {
      for (const o of latches) { const r = rest[o.name]; const d = r.p.clone().sub(new THREE.Vector3(0, 0.96, 0.08)).normalize(); o.position.copy(r.p).addScaledVector(d, -0.045 * k); }
      continue;
    }
    const o = P[tr.node]; if (!o) continue;
    const r = rest[tr.node];
    if (tr.pos) { o.position.copy(r.p); o.position[tr.pos] += tr.v * k; }
    else o.quaternion.copy(r.q).multiply(new THREE.Quaternion().setFromAxisAngle(AX[tr.axis], tr.v * k));
  }
  for (const S of ["L", "R"]) {          // the visor's rams follow it
    const base = P["STRUT_" + S], tip = P["STRUT_" + S + "_tip"], rod = P["STRUT_" + S + "_rod"];
    if (!base || !tip || !rod) continue;
    pod.updateMatrixWorld(true);
    const w = tip.getWorldPosition(new THREE.Vector3());
    base.lookAt(w);
    base.updateMatrixWorld(true);
    const D = base.getWorldPosition(new THREE.Vector3()).distanceTo(w);
    const L = rest["STRUT_" + S + "_rod"].p.z / 0.45;
    rod.position.set(0, 0, D - 0.55 * L);
  }
  const blink = t > 3.5 && t < OPEN_END && Math.floor(t * 4) % 2 ? 3 : 0.5;
  for (const o of latches) o.traverse(m => { if (m.isMesh) for (const mt of [].concat(m.material)) if (mt.name.includes("amber")) mt.emissiveIntensity = blink; });
}

// ---------- cameras ----------
const orbit = new THREE.OrbitControls(camera, renderer.domElement);
orbit.enableDamping = true; orbit.maxPolarAngle = Math.PI * 0.495;
let cam = "review";
const look = { yaw: 0, pitch: -0.08, fov: 80 };
const podPoint = (x, y, z) => pod.localToWorld(new THREE.Vector3(x, y, z));
function setCam(c) {
  cam = c;
  document.querySelectorAll("[data-cam]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.cam === c)));
  document.getElementById("hint").textContent = (c === "seat" || c === "eyes") ? "drag: look / scroll: zoom" : "drag: orbit / scroll: zoom";
  orbit.enabled = c !== "seat" && c !== "eyes";
  if (!pod) return;
  mech.updateMatrixWorld(true);
  if (c === "review") { camera.fov = 45; orbit.target.set(0, 4.5, 1); camera.position.set(13, 7, 16); }
  if (c === "hatch") { camera.fov = 40; orbit.target.copy(podPoint(0, 0.9, 0.8)); camera.position.copy(podPoint(-2.4, 1.7, 4.4)); }
  if (c === "side") { camera.fov = 40; orbit.target.set(0, 4.5, 0.5); camera.position.set(19, 5, 2); }
  if (c === "seat") { look.yaw = 0; look.pitch = -0.08; look.fov = 80; }
  camera.updateProjectionMatrix(); orbit.update();
}
document.querySelectorAll("[data-cam]").forEach(b => b.addEventListener("click", () => setCam(b.dataset.cam)));
let drag = null;
renderer.domElement.addEventListener("pointerdown", e => { if (orbit.enabled) return; drag = [e.clientX, e.clientY]; renderer.domElement.setPointerCapture(e.pointerId); });
renderer.domElement.addEventListener("pointermove", e => { if (!drag) return; const k = 0.004 * look.fov / 80;
  look.yaw += (e.clientX - drag[0]) * k; look.pitch -= (e.clientY - drag[1]) * k; drag = [e.clientX, e.clientY];
  look.yaw = Math.max(-2.7, Math.min(2.7, look.yaw)); look.pitch = Math.max(-1.25, Math.min(1.2, look.pitch)); });
renderer.domElement.addEventListener("pointerup", () => drag = null);
renderer.domElement.addEventListener("wheel", e => { if (orbit.enabled) return; e.preventDefault(); look.fov = Math.min(95, Math.max(30, look.fov + e.deltaY * 0.03)); }, { passive: false });

// ---------- the pilot's walk: up the stair, in, turn, sit (and the reverse) ----------
// Waypoints: where the eyes are and the yaw they look along in the pod's frame (0 is the pod's
// forward, pi is facing the pod), built from the stair as it lies when the walk starts.
function stairPoint(name, y) { const o = P[name]; o.updateMatrixWorld(true); return o.localToWorld(new THREE.Vector3(0, y, 0)); }
function walkPath() {
  mech.updateMatrixWorld(true);
  const foot = stairPoint("STAIR_3", 0.5);
  const eyeH = 1.7, up = v => v.clone().setY(v.y + eyeH);
  const s2 = stairPoint("STAIR_2", 0.25), s1 = stairPoint("STAIR_1", 0.25);
  const sill = podPoint(0, 0.62, 1.15), inside = podPoint(0, 0.62, 0.55);
  const F = Math.PI;
  return [
    { p: up(new THREE.Vector3(foot.x, 0, foot.z + 4.5)), yaw: F, pitch: 0.3, d: 0 },
    { p: up(new THREE.Vector3(foot.x, 0, foot.z + 1.0)), yaw: F, pitch: 0.45, d: 3.2 },
    { p: up(new THREE.Vector3(foot.x, Math.max(0, foot.y), foot.z)), yaw: F, pitch: 0.4, d: 1.2 },
    { p: up(s2), yaw: F, pitch: 0.35, d: 1.4 },
    { p: up(s1), yaw: F, pitch: 0.2, d: 1.4 },
    { p: sill.clone().add(new THREE.Vector3(0, 1.0, 0)), yaw: F, pitch: -0.15, d: 1.2 },
    { p: inside.clone().add(new THREE.Vector3(0, 0.7, 0)), yaw: F - 0.2, pitch: -0.35, d: 1.3 },
    { p: podPoint(0.05, 1.42, 0.05), yaw: F - 1.5, pitch: -0.2, d: 1.1 },
    { p: podPoint(...MAN.eye_m), yaw: 0, pitch: -0.08, d: 1.6 },
  ];
}
let walk = null;
function stepWalk(dt) {
  if (!walk) return;
  if (walk.hold) { placeEye(podPoint(...MAN.eye_m), look.yaw, look.pitch, look.fov); return; }
  if (!walk.pts) {
    if (walk.waitOpen && T < OPEN_END - 0.05) { const p = walkPath()[0]; placeEye(p.p, p.yaw, p.pitch, 80); return; }
    let pts = walkPath();
    if (!walk.inbound) pts = pts.slice().reverse().map((p, i, a) => Object.assign({}, p, { d: i ? a[i - 1].d : 0, yaw: p.yaw === 0 ? 0 : p.yaw - Math.PI }));
    walk.pts = pts; walk.i = 0; walk.u = 0;
  }
  const a = walk.pts[walk.i], b = walk.pts[walk.i + 1];
  if (!b) { walk.done = true; if (walk.inbound) { look.yaw = 0; look.pitch = -0.08; look.fov = 80; walk = { hold: true, done: true }; } return; }
  walk.u += dt / Math.max(0.2, b.d);
  const k = ease(walk.u);
  const p = a.p.clone().lerp(b.p, k);
  p.y += 0.025 * Math.sin(walk.u * Math.PI * 6) * (walk.i < 5 ? 1 : 0);
  placeEye(p, a.yaw + (b.yaw - a.yaw) * k, a.pitch + (b.pitch - a.pitch) * k, 80);
  if (walk.u >= 1) { walk.i++; walk.u = 0; }
}
function placeEye(p, yaw, pitch, fov) {
  const q = new THREE.Quaternion(); pod.getWorldQuaternion(q);
  const e = new THREE.Euler().setFromQuaternion(q, "YXZ");
  camera.position.copy(p);
  camera.quaternion.setFromEuler(new THREE.Euler(pitch + (yaw === 0 ? e.x : 0), e.y + yaw + Math.PI, 0, "YXZ"));
  camera.fov = fov; camera.updateProjectionMatrix();
}

// ---------- scripted sequences ----------
let script = null, sealPuff = 0;
function run(steps) { script = steps.map(s => Object.assign({ started: false, t: 0 }, s)); }
function stepScript(dt) {
  if (!script || !script.length) return;
  const s = script[0];
  if (!s.started) { s.started = true; if (s.fn) s.fn(); if (s.kind === "do") { script.shift(); return; } }
  s.t += dt;
  if (s.kind === "timeline") { target = s.to; rate = s.rate || 1; if (Math.abs(T - s.to) < 1e-3) script.shift(); }
  else if (s.kind === "wait") { if (s.t >= s.d) script.shift(); }
  else if (s.kind === "walk") { if (!walk || walk.done) script.shift(); }
}
const powerOn = v => { toggles.power = v; document.getElementById("power").setAttribute("aria-pressed", String(v)); };
document.getElementById("open").onclick = () => run([{ kind: "timeline", to: OPEN_END }]);
document.getElementById("close").onclick = () => run([{ kind: "timeline", to: 0 }]);
document.getElementById("board").onclick = () => run([
  { kind: "do", fn: () => { powerOn(false); cam = "eyes"; setCam("eyes"); walk = { inbound: true, waitOpen: true, done: false }; } },
  { kind: "timeline", to: OPEN_END },
  { kind: "walk" },
  { kind: "wait", d: 0.6 },
  { kind: "timeline", to: SEALED_AT, rate: 0.9 },
  { kind: "do", fn: () => { sealPuff = 1.0; } },
  { kind: "wait", d: 0.9 },
  { kind: "do", fn: () => powerOn(true) },
  { kind: "wait", d: 2.8 },
  { kind: "timeline", to: 0, rate: 0.8 },
]);
document.getElementById("leave").onclick = () => run([
  { kind: "do", fn: () => { setCam("eyes"); walk = { hold: true, done: false }; } },
  { kind: "timeline", to: SEALED_AT },
  { kind: "do", fn: () => powerOn(false) },
  { kind: "wait", d: 0.8 },
  { kind: "timeline", to: OPEN_END },
  { kind: "do", fn: () => { walk = { inbound: false, waitOpen: false, done: false }; } },
  { kind: "walk" },
]);

// ---------- the frame ----------
const toggles = { hud: true, power: true, steam: true };
for (const id of Object.keys(toggles)) document.getElementById(id).addEventListener("click", e => {
  toggles[id] = !toggles[id]; e.currentTarget.setAttribute("aria-pressed", String(toggles[id]));
});
const scrub = document.getElementById("scrub"), tlabel = document.getElementById("tlabel"), cueEl = document.getElementById("cue");
scrub.addEventListener("input", () => { script = null; T = target = scrub.value / 1000 * OPEN_END; });
const dirOf = v => { const d = v.userData.dir; return new THREE.Vector3(...(typeof d === "string" ? JSON.parse(d) : d)); };
const clock = new THREE.Clock();
let lastEye = new THREE.Vector3(1e9, 0, 0), capTimer = 0;
function frame() {
  const dt = Math.min(0.05, clock.getDelta()), t = clock.elapsedTime;
  const w = stage.clientWidth, h = stage.clientHeight;
  if (renderer.domElement.width !== Math.round(w * renderer.getPixelRatio())) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
  if (mech && pod) {
    stepScript(dt);
    const prevT = T;
    if (T < target) T = Math.min(target, T + dt * rate); else if (T > target) T = Math.max(target, T - dt * rate);
    scrub.value = Math.round(T / OPEN_END * 1000); tlabel.textContent = T.toFixed(1) + " s";
    setNodes(T);
    B.t += dt;
    const Pz = new Pose();
    Pz.w = 1 - sitK; idleGeneric(Pz, B.t, B);
    Pz.w = 1; sitPose(Pz, B, sitK);
    applyPose(B, Pz);
    mech.updateMatrixWorld(true);
    const moving = Math.abs(T - prevT) > 1e-5;
    if (toggles.steam) {
      const q = new THREE.Quaternion();
      for (const v of vents) {
        const u = v.userData, win = VENT_WINDOWS[u.stage]; if (!win) continue;
        const live = u.stage === "open" ? T > win[0] : (moving && T > win[0] && T < win[1]);
        if (!live) continue;
        const n = (u.kind === "mist" ? 10 : 34) * (u.rate || 1) * dt, k = Math.floor(n) + (Math.random() < n % 1 ? 1 : 0);
        if (!k) continue;
        v.parent.getWorldQuaternion(q);
        const dir = dirOf(v).applyQuaternion(q), pw = v.getWorldPosition(new THREE.Vector3());
        for (let i = 0; i < k; i++) emit(pw, dir, u.kind === "mist", 1);
      }
      if (sitK > 0.02 && sitK < 0.98 && moving)       // the legs' rams vent as they work
        for (const n of ["shin_L", "shin_R", "hock_L", "hock_R"]) if (N[n] && Math.random() < 0.6)
          emit(N[n].getWorldPosition(new THREE.Vector3()), new THREE.Vector3(n.endsWith("L") ? 1 : -1, 0.4, 0.3).normalize(), false, 1.2);
      if (sealPuff > 0) { sealPuff -= dt; for (const v of vents) if (v.userData.stage === "seam" && Math.random() < 0.5) {
        v.parent.getWorldQuaternion(q); emit(v.getWorldPosition(new THREE.Vector3()), dirOf(v).applyQuaternion(q), false, 0.6); } }
    }
    stepParticles(dt);
    let cue = ""; for (const [ct, c] of CUES) if (T >= ct - 1e-3) cue = c;
    cueEl.hidden = !(T > 0.01 && T < OPEN_END - 0.01); cueEl.textContent = cue;
    if (cam === "eyes") stepWalk(dt);
    else if (cam === "seat") placeEye(podPoint(...MAN.eye_m), look.yaw, look.pitch, look.fov);
    else orbit.update();
    // the screens: the world captured from the eye whenever the eye has moved
    const eye = podPoint(...MAN.eye_m);
    pod.updateMatrixWorld(true);
    allview.uniforms.eye.value.copy(eye);
    allview.uniforms.podInv.value.copy(pod.matrixWorld).invert();
    powerT = toggles.power ? Math.min(1, powerT + dt / 2.5) : Math.max(0, powerT - dt / 0.6);
    allview.uniforms.power.value = powerT; allview.uniforms.hudOn.value = toggles.hud && powerT > 0.95 ? 1 : 0;
    allview.uniforms.time.value = t;
    capTimer -= dt;
    if (eye.distanceTo(lastEye) > 0.01 && capTimer <= 0 && (cam === "seat" || cam === "eyes")) {
      pod.visible = false; cloud.visible = false; cubeCam.position.copy(eye); cubeCam.update(renderer, scene); pod.visible = true; cloud.visible = true;
      lastEye.copy(eye); capTimer = 0.1;
    }
  }
  pMat.uniforms.scale.value = renderer.domElement.height / (2 * Math.tan(THREE.MathUtils.degToRad(camera.fov) / 2));
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}
frame();

const rows = [["Triangles (pod)", MAN.tris.toLocaleString("en-GB") + " / " + MAN.tri_budget.toLocaleString("en-GB")]];
for (const [g, a] of Object.entries(MAN.atlases)) rows.push([`Baked atlas, ${g}`, `${a.px} x ${a.px}`]);
rows.push(["Baked texture memory (RGBA8, mips)", (Object.values(MAN.atlases).reduce((s, a) => s + a.px * a.px * 4 * 4 / 3, 0) / 1048576).toFixed(0) + " MiB"]);
rows.push(["Draw calls (mesh materials)", String(MAN.draw_calls)]);
rows.push(["Moving nodes", String(Object.keys(MAN.nodes).length)]);
rows.push(["Steam and mist vents", String(MAN.vents)]);
rows.push(["Panels modelled", String(MAN.panels.length)]);
rows.push(["Bake samples", String(MAN.bake.samples) + (MAN.bake.denoised ? ", denoised" : "")]);
rows.push(["glb size", (MAN.glb_bytes / 1048576).toFixed(1) + " MiB"]);
document.getElementById("cost").innerHTML = rows.map(([a, b]) => `<tr><td>${a}</td><td>${b}</td></tr>`).join("");

window.MOCKUP = {
  setCam, setT(v) { script = null; T = target = v; }, open() { document.getElementById("open").click(); }, board() { document.getElementById("board").click(); },
  set(id, v) { toggles[id] = v; document.getElementById(id).setAttribute("aria-pressed", String(v)); },
  look(y, p, f) { setCam("seat"); look.yaw = y; look.pitch = p; look.fov = f; },
  orbitTo(tx, ty, tz, cx, cy, cz) { setCam("review"); orbit.target.set(tx, ty, tz); camera.position.set(cx, cy, cz); orbit.update(); },
  hatchView(cx, cy, cz, tx, ty, tz) { setCam("hatch"); orbit.target.copy(podPoint(tx, ty, tz)); camera.position.copy(podPoint(cx, cy, cz)); orbit.update(); },
  state: () => ({ T, sitK, walk: walk && { i: walk.i, done: walk.done } }),
};
</script>
"""


def main():
    man = json.load(open(MAN))
    b64 = lambda p: base64.b64encode(open(p, "rb").read()).decode("ascii")
    im = Image.open(SHEET).convert("RGB")
    im = im.resize((1600, int(im.size[1] * 1600 / im.size[0])), Image.LANCZOS)
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=82, optimize=True)
    html = (PAGE.replace("__MECH_POSE__", open(POSE).read()).replace("__GLB__", b64(GLB)).replace("__MECH__", b64(MECH))
            .replace("__HUD__", b64(HUD)).replace("__MAN__", json.dumps(man))
            .replace("__SHEET__", base64.b64encode(buf.getvalue()).decode("ascii"))
            .replace("__WIP__", json.dumps(os.environ.get("COCKPIT_WIP", ""))))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    print(OUT, f"{len(html) / 1e6:.1f} MB")


main()
