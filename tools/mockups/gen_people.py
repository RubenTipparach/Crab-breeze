"""Build the people review page: the two VAT bodies and every clip, orbitable at true scale beside the pilot and the
Foreman (openspec change `people-vat`).

    python3 tools/mockups/gen_people.py [out.html]

Writes output/people_vat/people.html by default. Reads assets/models/people/<id>.glb and people_vat.json (written by
tools/blender/build_people.py), the palette in data/people/people.ron, and the Foreman (assets/models/mech_foreman.glb
with its measured bounds in assets/models/mechs.json), and inlines them, so the page needs nothing but three.js r147.
Each body is coloured as the game colours it: every vertex's region (`_REGION`) picks a colour from the palette and
its shade (`_SHADE`) darkens or lightens it. The glb plays the clips with three's skinning, the source the bake was
sampled from; the game plays the bake (the page says so).
"""
import base64
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "output", "people_vat", "people.html")

PAGE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>People VAT Bodies</title>
<style>
:root { --bg: #eceef1; --panel: #f8f9fa; --edge: #c6ccd4; --ink: #14181d; --muted: #59626e; --accent: #b4561a; --stage: #20242a;
  --body: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif; --mono: ui-monospace, Menlo, monospace; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --bg: #0e1115; --panel: #161a20; --edge: #2b323c; --ink: #e5e9ee; --muted: #94a0ad; --accent: #f08a3c; color-scheme: dark; } }
:root[data-theme="dark"] { --bg: #0e1115; --panel: #161a20; --edge: #2b323c; --ink: #e5e9ee; --muted: #94a0ad; --accent: #f08a3c; color-scheme: dark; }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.45 var(--body); }
header { padding: 16px; max-width: 1200px; margin: 0 auto; }
h1 { margin: 0 0 4px; font-size: 22px; }
header p { margin: 0; color: var(--muted); }
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
  <h1>People: VAT bodies and clips</h1>
  <p>The two MPFB bodies (CC0 MakeHuman clothes, the 53-bone rig) with Quaternius' Universal Animation Library clips (CC0) and the authored wave, at true size beside the pilot (1.8 m) and the Foreman (24 m), on a 1 m grid. Drag to orbit, scroll or pinch to zoom; arrow keys and +/- also work. The page plays the skinned glb the bake was sampled from; the game plays the bake (positions per vertex per frame on the GPU).</p>
</header>
<main>
  <div id="stage"><canvas id="view" tabindex="0" aria-label="3D view of the people"></canvas><div id="labels"></div></div>
  <div class="side">
    <div class="card"><h2>Clip</h2><div class="row" id="clips"></div><div class="row" style="margin-top:6px"><button id="crowd">Crowd: every clip</button></div></div>
    <div class="card"><h2>Level of detail</h2><div class="row" id="lods"></div><p class="note">Near within 60 m of the eye; crowd beyond.</p></div>
    <div class="card"><h2>Show</h2><div class="row" id="toggles"></div><div class="row" style="margin-top:6px"><button id="outfit">New outfits and hair</button><button id="reset">Reset view</button><button id="heads">Heads close up</button><button id="hem">Hem close up</button><button id="sleeve">Sleeve close up</button></div></div>
    <div class="card"><h2>Measured</h2><table id="sizes"><tr><th>Body</th><th>Level</th><th>Tris</th><th>Verts</th></tr></table>
      <table id="cliptab" style="margin-top:8px"><tr><th>Clip</th><th>Frames</th><th>s</th><th>Walks m/s</th></tr></table></div>
  </div>
</main>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/utils/SkeletonUtils.js"></script>
<script>
const MANIFEST = __MANIFEST__, PALETTE = __PALETTE__, CONTRAST = __CONTRAST__, MECH = __MECH__, GLB = __GLB__;
const T = THREE;
const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.shadowMap.enabled = true;
const scene = new T.Scene(); scene.background = new T.Color(0x9fb4c8);
const camera = new T.PerspectiveCamera(40, 1.6, 0.05, 2000);
const controls = new T.OrbitControls(camera, canvas); controls.enableDamping = true;
scene.add(new T.HemisphereLight(0xdfe8f5, 0x3a3a38, 0.55));
const sun = new T.DirectionalLight(0xfff0dc, 1.5); sun.position.set(-8, 14, 10); sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -20, right: 20, top: 20, bottom: -20 }); scene.add(sun);
const ground = new T.Mesh(new T.PlaneGeometry(200, 200), new T.MeshStandardMaterial({ color: 0x5f625d, roughness: 1 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const grid = new T.GridHelper(60, 60, 0x55606b, 0x55606b); grid.position.y = 0.005; grid.material.opacity = 0.35; grid.material.transparent = true; scene.add(grid);
const rail = (y, c) => { const r = new T.Mesh(new T.BoxGeometry(40, 0.02, 0.02), new T.MeshBasicMaterial({ color: c })); r.position.set(0, y, -1.5); scene.add(r); return r; };
const pilotRail = rail(1.8, 0xf0b13c);

// the pilot, 1.8 m (the stand-in figure every Crab Breeze review page uses)
const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a });
[-0.1, 0.1].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.08, 0.06, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torso = new T.Mesh(new T.CylinderGeometry(0.2, 0.15, 0.62, 10), suit); torso.position.y = 1.18; pilot.add(torso);
const head = new T.Mesh(new T.SphereGeometry(0.11, 16, 12), skin); head.position.y = 1.69; pilot.add(head);
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; }); pilot.position.set(-1.6, 0, 0); scene.add(pilot);

const srgb = c => new T.Color().setRGB(c[0], c[1], c[2]).convertSRGBToLinear();
const LISTS = ["skin", "top", "bottom", "shoes", "hair"];
const lum = c => c.map(v => v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)).reduce((a, v, i) => a + v * [0.2126, 0.7152, 0.0722][i], 0);
// an outfit as the game rolls one: a colour from each list, the bottom stepped on until it differs from the top by the
// minimum contrast, and a hair style (none only for a body that may be bald)
function outfit(seed, body) { let k = 0; const r = () => { const x = Math.sin(seed * 12.9898 + (k++) * 78.233) * 43758.5453; return x - Math.floor(x); };
  const pick = name => PALETTE[name][Math.floor(r() * PALETTE[name].length)];
  const top = pick("top"); let bi = Math.floor(r() * PALETTE.bottom.length), bottom = PALETTE.bottom[bi];
  for (let i = 0; i < PALETTE.bottom.length && Math.abs(lum(bottom) - lum(top)) < CONTRAST; i++) bottom = PALETTE.bottom[(bi + i + 1) % PALETTE.bottom.length];
  const n = body.hairs.length + (body.bald ? 1 : 0), h = Math.floor(r() * n), hair = body.bald ? h : h + 1;
  return { colours: [pick("skin"), top, bottom, pick("shoes"), pick("hair"), PALETTE.eyes], hair }; }
function paint(mesh, o) {
  const g = mesh.geometry = mesh.geometry.clone(), reg = g.attributes._region, sh = g.attributes._shade, hv = g.attributes._hair, n = g.attributes.position.count;
  const col = new Float32Array(n * 3), cs = o.colours.map(srgb);
  for (let i = 0; i < n; i++) { const c = cs[Math.round(reg.getX(i))] || cs[0], s = sh.getX(i); col[3 * i] = c.r * s; col[3 * i + 1] = c.g * s; col[3 * i + 2] = c.b * s; }
  g.setAttribute("color", new T.BufferAttribute(col, 3));
  // only the person's own hair style (hair style 0 is the body)
  const idx = g.index.array, keep = [];
  for (let t = 0; t < idx.length; t += 3) { const h = Math.round(hv.getX(idx[t])); if (h === 0 || h === o.hair) keep.push(idx[t], idx[t + 1], idx[t + 2]); }
  g.setIndex(keep);
  mesh.material = new T.MeshStandardMaterial({ vertexColors: true, roughness: 0.8, flatShading: false });
}
const loader = new T.GLTFLoader();
const buf = b64 => { const s = atob(b64), u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer; };
const parse = id => new Promise(res => loader.parse(buf(GLB[id]), "", res));
const people = []; let lod = "near", clip = "walk", crowdMode = false, seed = 1;
const labels = document.getElementById("labels");
function makePerson(src, body, x, z, colours) {
  const root = T.SkeletonUtils.clone(src.scene);
  root.traverse(o => { if (o.isSkinnedMesh) { o.castShadow = o.receiveShadow = true; o.frustumCulled = false; paint(o, colours); } });
  root.position.set(x, 0, z); // the bodies face +Z in glTF, toward the default camera
  scene.add(root);
  const mixer = new T.AnimationMixer(root);
  const p = { root, mixer, body, src, colours, action: null }; people.push(p); return p;
}
function setLod(p) { p.root.traverse(o => { if (o.isSkinnedMesh) o.visible = o.name.endsWith(lod) || (o.parent && o.parent.name.endsWith(lod)); }); }
function play(p, name, offset) {
  const c = p.src.animations.find(a => a.name === name); if (!c) return;
  if (p.action) p.action.stop();
  p.action = p.mixer.clipAction(c); p.action.setLoop(name === "fall" ? T.LoopOnce : T.LoopRepeat); p.action.clampWhenFinished = true;
  p.action.reset().play(); if (offset) p.action.time = offset * c.duration;
}
let srcs = {};
function rebuild() {
  for (const p of people) scene.remove(p.root); people.length = 0;
  labels.innerHTML = "";
  const clips = MANIFEST.clips.map(c => c.name);
  if (crowdMode) {
    let i = 0;
    for (const name of clips) for (const b of MANIFEST.bodies) {
      const x = -6 + (i % 7) * 2.0, z = -Math.floor(i / 7) * 2.2;
      const p = makePerson(srcs[b.id], b.id, x, z, outfit(seed + i, b)); setLod(p); play(p, name, (i * 0.37) % 1); i++;
      if (b.id === MANIFEST.bodies[0].id) addLabel(p, name);
    }
  } else {
    MANIFEST.bodies.forEach((b, i) => { const p = makePerson(srcs[b.id], b.id, i * 1.4, 0, outfit(seed + i, b)); setLod(p); play(p, clip); addLabel(p, b.id + " " + clip); });
  }
}
function addLabel(p, text) { const s = document.createElement("span"); s.textContent = text; labels.appendChild(s); p.label = s; }
function button(parent, text, on, fn) { const b = document.createElement("button"); b.textContent = text; b.setAttribute("aria-pressed", on); b.onclick = () => fn(b); parent.appendChild(b); return b; }
function radio(parent, names, cur, fn) { names.forEach(n => button(parent, n, n === cur, () => { parent.querySelectorAll("button").forEach(x => x.setAttribute("aria-pressed", x.textContent === n)); fn(n); })); }
function resetView() { controls.target.set(0.7, 1.0, 0); camera.position.set(2.5, 1.9, 5.5); controls.update(); }
Promise.all([...MANIFEST.bodies.map(b => parse(b.id).then(g => { srcs[b.id] = g; })), parse("mech").then(g => { srcs.mech = g; })]).then(() => {
  const mech = srcs.mech.scene; mech.traverse(o => { if (o.isMesh) { o.castShadow = true; } }); mech.position.set(-18, 0, -14); scene.add(mech);
  radio(document.getElementById("clips"), MANIFEST.clips.map(c => c.name), clip, n => { clip = n; crowdMode = false; rebuild(); });
  radio(document.getElementById("lods"), ["near", "crowd"], lod, n => { lod = n; people.forEach(setLod); });
  const tog = document.getElementById("toggles");
  button(tog, "Pilot (1.8 m)", true, b => { pilot.visible = !pilot.visible; pilotRail.visible = pilot.visible; b.setAttribute("aria-pressed", pilot.visible); });
  button(tog, "Foreman (24 m)", true, b => { mech.visible = !mech.visible; b.setAttribute("aria-pressed", mech.visible); });
  document.getElementById("crowd").onclick = () => { crowdMode = true; rebuild(); };
  document.getElementById("outfit").onclick = () => { seed += 17; rebuild(); };
  document.getElementById("reset").onclick = resetView;
  document.getElementById("hem").onclick = () => { controls.target.set(0.7, 1.0, 0); camera.position.set(0.7, 1.05, 1.9); controls.update(); };
  document.getElementById("sleeve").onclick = () => { controls.target.set(1.65, 1.15, 0); camera.position.set(2.6, 1.25, 0.9); controls.update(); };
  document.getElementById("heads").onclick = () => { controls.target.set(0.7, 1.55, 0); camera.position.set(0.7, 1.6, 2.4); controls.update(); };
  const t = document.getElementById("sizes");
  for (const b of MANIFEST.bodies) for (const l of b.levels) t.insertAdjacentHTML("beforeend", `<tr><td>${b.id} ${b.height_m.toFixed(3)} m</td><td>${l.name}</td><td>${l.triangles}</td><td>${l.vertices}</td></tr>`);
  const ct = document.getElementById("cliptab");
  for (const c of MANIFEST.clips) { const sp = c.speed_m_s_by_body ? Object.values(c.speed_m_s_by_body).map(v => v.toFixed(2)).join(" / ") : "";
    ct.insertAdjacentHTML("beforeend", `<tr><td>${c.name}${c.loop ? "" : " (held)"}</td><td>${c.frames}</td><td>${(c.loop ? c.frames / c.fps : (c.frames - 1) / c.fps).toFixed(2)}</td><td>${sp}</td></tr>`); }
  rebuild(); resetView(); window.MOCKUP_READY = true;
});
canvas.addEventListener("keydown", e => {
  const k = { ArrowLeft: [0.06, 0], ArrowRight: [-0.06, 0], ArrowUp: [0, 0.06], ArrowDown: [0, -0.06] }[e.key];
  if (k) { const off = camera.position.clone().sub(controls.target); const sph = new T.Spherical().setFromVector3(off);
    sph.theta += k[0]; sph.phi = Math.min(Math.PI / 2 - 0.02, Math.max(0.05, sph.phi - k[1])); camera.position.copy(controls.target).add(new T.Vector3().setFromSpherical(sph)); e.preventDefault(); }
  if (e.key === "+" || e.key === "=" || e.key === "-") { const off = camera.position.clone().sub(controls.target).multiplyScalar(e.key === "-" ? 1.15 : 0.87); camera.position.copy(controls.target).add(off); }
});
function resize() { const r = canvas.parentElement.getBoundingClientRect(); renderer.setSize(r.width, r.height, false); camera.aspect = r.width / r.height; camera.updateProjectionMatrix(); }
window.addEventListener("resize", resize); resize();
const clock = new T.Clock(), v = new T.Vector3();
(function loop() { requestAnimationFrame(loop); const dt = Math.min(clock.getDelta(), 0.1); people.forEach(p => p.mixer.update(dt));
  controls.update(); renderer.render(scene, camera);
  const r = canvas.getBoundingClientRect();
  for (const p of people) { if (!p.label) continue; v.set(0, 2.05, 0); p.root.localToWorld(v); v.project(camera);
    const on = v.z < 1 && Math.abs(v.x) < 1.1 && Math.abs(v.y) < 1.1;
    p.label.style.display = on ? "block" : "none"; p.label.style.left = (v.x + 1) / 2 * r.width + "px"; p.label.style.top = (1 - v.y) / 2 * r.height + "px"; } })();
</script>
</body>
</html>
"""


def palette():
    """The palette lists from data/people/people.ron (sRGB 0-1), as the game reads them."""
    text = open(os.path.join(ROOT, "data", "people", "people.ron"), encoding="utf-8").read()
    block = text[text.index("palette: ("):]
    out = {}
    for name in ("skin", "hair", "top", "bottom", "shoes"):
        m = re.search(name + r": \[(.*?)\]", block, re.S)
        out[name] = [[float(v) for v in t.split(",")] for t in re.findall(r"\(([0-9., ]+)\)", m.group(1))]
    out["eyes"] = [float(v) for v in re.search(r"eyes: \(([0-9., ]+)\)", block).group(1).split(",")]
    return out, float(re.search(r"min_top_bottom_contrast: ([0-9.]+)", text).group(1))


def main():
    people = os.path.join(MODELS, "people")
    manifest = json.load(open(os.path.join(people, "people_vat.json")))
    mechs = json.load(open(os.path.join(MODELS, "mechs.json")))
    glb = {}
    for b in manifest["bodies"]:
        with open(os.path.join(people, b["glb"]), "rb") as f:
            glb[b["id"]] = base64.b64encode(f.read()).decode("ascii")
    with open(os.path.join(MODELS, mechs["foreman"]["file"]), "rb") as f:
        glb["mech"] = base64.b64encode(f.read()).decode("ascii")
    pal, contrast = palette()
    html = (PAGE.replace("__MANIFEST__", json.dumps(manifest)).replace("__PALETTE__", json.dumps(pal))
            .replace("__CONTRAST__", json.dumps(contrast))
            .replace("__MECH__", json.dumps(mechs["foreman"])).replace("__GLB__", json.dumps(glb)))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB)")


main()
