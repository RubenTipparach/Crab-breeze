"""Build docs/mockups/mech_designs.html: the mech designs, at true scale, and their chests opening to board.

    python3 tools/mockups/gen_mech_designs.py
    python3 -m http.server 8766 -d output/mech_site      # to look at it or shoot it locally
    (output/mech_site/files.json is the map to publish its files with)

The owner, 2026-10-24: "Show me mech designs with the animation of the cockpit opening up", "Id like a couple
variations of cockpit designs please", "Breeze me h sucks", "Follow design philosophy of boss mechs from mining
mike. The cockpit is part of the mech, inside of it with the armor opening up at the chest".

Loads assets/models/mech_<id>.glb (tools/blender/build_mechs.py) and each one's cockpit interior,
assets/models/cockpit_<variant>/ (tools/blender/build_cockpit.py), by relative path: they are published as the
page's own files (files() below). The manifests are inlined; every size on the page is measured off the models.
The boarding timeline is the mech's own: its root extras carry the chest's tracks, rams, vents and ladder
(tools/blender/mech_boarding.py), the interior's manifest carries its inner tracks. Mining Mike's rule for model
review holds: orbit it, at true relative scale, beside a 1.8 m pilot and a rail at 12 m.

The animation is Mining Mike's (godot-sandbox tank_bosses_kit.js at 3336b88) through tools/mockups/mech_pose.js;
the steam is tools/mockups/steam.js and the screen wall tools/mockups/screen_wall.js, inlined.
"""
import base64
import json
import os
import shutil
import struct

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "assets", "models")
OUT = os.path.join(ROOT, "docs", "mockups", "mech_designs.html")
SITE = os.path.join(ROOT, "output", "mech_site")
HUD = os.path.join(ROOT, "assets", "textures", "cockpit", "displays", "hud.png")
ORDER = ["foreman", "ironhorse", "harrow", "roustabout", "driller", "mule", "picket", "sapper"]
# the block-outs the owner picked to build (2026-10-24: "1,2,6,8,11,12 start building them") keep their numbers
NUMBERS = dict(harrow=1, roustabout=2, driller=6, mule=8, picket=11, sapper=12)

PAGE = r"""<title>Crab Breeze Mechs</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Condensed:wght@500;700&family=IBM+Plex+Sans:wght@400;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: a hangar review sheet. The stage is the page; the spec cards sit beside it like a clipboard. */
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
header { display: grid; grid-template-columns: minmax(0, 1fr); gap: 4px; margin: 0 auto 14px; max-width: 1400px; }
.eyebrow { font: 500 12px var(--mono); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
h1 { font: 700 34px/1 var(--display); letter-spacing: 0.03em; text-transform: uppercase; margin: 0; text-wrap: balance; }
.lede { margin: 0; color: var(--muted); max-width: 76ch; }
.wrap { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 16px; align-items: start; max-width: 1400px; margin: 0 auto; }
@media (max-width: 1000px) { .wrap { grid-template-columns: minmax(0, 1fr); } }
.bar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 8px; }
.bar .lbl { font: 500 12px var(--mono); color: var(--muted); min-width: 6ch; }
button { font: 700 14px/1 var(--display); letter-spacing: 0.06em; text-transform: uppercase; color: var(--ink);
  background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 9px 12px; cursor: pointer; min-height: 38px; }
button:hover { border-color: var(--muted); }
button[aria-pressed="true"] { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
button:focus-visible, canvas:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.cue { position: absolute; right: 12px; top: 10px; font: 700 15px var(--display); letter-spacing: 0.08em; text-transform: uppercase; color: #ffd28a; background: rgba(0,0,0,0.45); padding: 4px 10px; border-radius: 3px; pointer-events: none; }
.load { position: absolute; inset: 0; display: grid; place-items: center; color: #c9d2dc; font: 500 14px var(--mono); pointer-events: none; }
.scrub { display: flex; gap: 10px; align-items: center; flex: 1 1 220px; min-width: 0; }
.scrub input { flex: 1; min-width: 0; accent-color: var(--accent); }
.scrub output { font: 500 13px var(--mono); color: var(--muted); font-variant-numeric: tabular-nums; min-width: 6ch; }
button.go { border-color: var(--accent); }
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
.card h2 small { font: 500 11px var(--mono); color: var(--warn); letter-spacing: 0.08em; }
.kv { display: grid; grid-template-columns: 9ch minmax(0, 1fr); gap: 2px 10px; font-size: 13px; margin: 0; }
.kv dt { color: var(--muted); font-family: var(--mono); font-size: 12px; padding-top: 1px; }
.kv dd { margin: 0; font-variant-numeric: tabular-nums; }
.note { font-size: 13px; color: var(--muted); margin: 0; }
.foot { max-width: 1400px; margin: 16px auto 0; font-size: 13px; color: var(--muted); display: grid; gap: 4px; }
.foot code { font: 12px var(--mono); overflow-wrap: anywhere; }
.bo { margin: 8px 0 26px; min-width: 0 }
.boh { font: 700 24px "Saira Condensed", sans-serif; text-transform: uppercase; margin: 14px 0 6px }
.bop { max-width: 72ch; margin: 0 0 10px }
.borow { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px }
.bostats { overflow-x: auto; margin-top: 8px }
/* the melee lab: moves, playback and the keyframe editor */
.mel-ed { background: var(--panel); border: 1px solid var(--edge); border-radius: 4px; padding: 10px 12px; margin-bottom: 8px; display: grid; gap: 8px; }
.mel-ed .row { display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: center; }
.mel-ed label { font: 500 12px var(--mono); color: var(--muted); display: flex; gap: 6px; align-items: center; }
.mel-ed select, .mel-ed input[type=number] { font: 500 13px var(--mono); background: var(--bg); color: var(--ink); border: 1px solid var(--edge); border-radius: 3px; padding: 6px; min-height: 34px; }
.mel-ed input[type=number] { width: 7ch; }
.mel-ed .sl { display: grid; grid-template-columns: 3ch 1fr 7ch; gap: 8px; align-items: center; flex: 1 1 240px; min-width: 0; }
.mel-ed .sl input[type=range] { min-width: 0; accent-color: var(--accent); }
.mel-ed .sl output { font: 500 12px var(--mono); font-variant-numeric: tabular-nums; text-align: right; }
.mel-ed .keys button { min-height: 32px; padding: 6px 9px; font-size: 13px; }
.mel-ed .keys button.hit { border-style: dashed; }
.mel-ed .status { font: 500 12px var(--mono); color: var(--muted); }
.mel-ed .grid6 { display: flex; flex-wrap: wrap; gap: 6px 18px; }
.flash { position: absolute; inset: 0; pointer-events: none; }
[hidden] { display: none !important }
.bostats table { border-collapse: collapse; font-size: 14px; min-width: 520px }
.bostats th, .bostats td { text-align: left; padding: 4px 10px; border-bottom: 1px solid rgba(128,128,128,.3) }
</style>

<header>
  <div class="eyebrow">Crab Breeze · mech design review · fourth pass, the new cockpits in the chest</div>
  <h1>Pick a chassis and its cockpit</h1>
  <p class="lede">Original chassis from the 1980s industrial world, each with its cockpit inside the chest. The chest and the cockpit now open as one body: the armour in front of each opening part of the cockpit is cut round it and moves on that part's own stages, so nothing meets. Foreman is an excavator carrying the Panorama: the front under the screens drops into a step, then the front over them swings up and out on two rams with the dash. Ironhorse is a locomotive carrying the seated Excavator cab: the dogs swing off and the smokebox front drops forward as a ramp with the cab's front on it. Stevedore keeps its standing harness for now. Press Kneel and open, or Board it to climb in through the pilot's eyes. All at true scale beside a 1.8 m pilot and a four-storey block.</p>
  <section class="bo">
    <h2 class="boh">Redesign at 24 m, low-poly first</h2>
    <p class="bop">The owner's next round (2026-10-24): twice the size, legs heavier than the torso for balance, every
    pilot seated, blocked out low-poly in Blender with CSG before any detail. The Stevedore keeps its struts; the Foreman
    and the Ironhorse keep their characters. Ten are new, each after a 1980s working machine and each carrying a different
    one of the ten cockpit block-outs at true size: the Harrow (after the Mad Cat, the Canopy rising out of its nose), the
    Roustabout (after the Scopedog, the Titan in its chest), the Grader (a dozer, Panorama), the Logger (a harvester, Low
    bank), the Lineman (a bucket truck, Cinema), the Driller (a drill rig, the All-view sphere), the Bowser (a fuel tanker,
    Octagon), the Mule (a haul truck, Twin tier), the Pumper (a fire engine, Tile wall) and the Ploughman (a tractor,
    Excavator cab). Some carry bigger arms: punching fists on the Mule and the Roustabout, heavier guns on the Harrow, the
    Bowser and the Roustabout. Two are humanoids with a Gundam's build but none of its badges: the Picket (after the
    RX-79[G], Panorama, machine gun and shield) and the Sapper (heavy, bolt-on armour, Titan, shoulder cannon, missile
    pod). The new ones are numbered 1 to 12, as on their contact sheets. Legs against torso is
    measured off the meshes, and no arm touches a leg (a mesh intersection check in the generator).</p>
    <p class="hint">In the viewer below: Scene, Block-outs. Beside them a 1.8 m pilot, a 4.4 m car, a 14 m four-storey block and a rail at 24 m.</p>
  </section>
  <section class="bo">
    <h2 class="boh">Combat mechs, built from parts</h2>
    <p class="bop">The owner (2026-10-24): "I need more combat mech mockups ... I want combat mechs with guns like in armored
    core". So these are built the way Armored Core builds them, from interchangeable parts: legs (reverse joint, biped, tracks
    or four legs), a core with a cockpit block-out in it at true size, a head, two arms each holding a weapon (rifle, gatling,
    bazooka, sniper rifle, shotgun, pile driver, shield, grenade launcher), two back units (missile box, vertical missiles,
    shoulder cannon, railgun, radar, rocket pod) and boosters (a back pack, swept wing vanes for the flyers, side jets), a
    skate wheel in each foot. Every mech is one line of parts in the generator, so a new one is a new combination. They are
    numbered on from the twelve above, 13 to 22.</p>
    <p class="bop">The owner's verdict: "13,15,16,17, 19, and 21 all good. Don't build those just yet. Just note that I like
    them." The Sabre, Bulldog, Tarantula, Hatchet, Anvil and Brigand are marked <b>liked</b> below; none is built in detail
    yet.</p>
    <p class="hint">In the viewer below: Scene, Combat.</p>
  </section>
  <section class="bo">
    <h2 class="boh">Crab mechs</h2>
    <p class="bop">The owner (2026-10-24): "make crab mechs more alien like ... they definitely work with two hind legs, and
    meaty claws dangling in the front ... a variation on leg count would be cool, 1,2,3 pairs, 3 pairs would probably have to
    be shorter and thinner", with the CET-OPS crab, a pincer crab sheet, the King Crab and two hunched brutes as references.
    So a crab is a hunched carapace, low at the face and humped at the back, on hind legs only (one, two or three pairs,
    arching over the shell to spurred knees), its weight carried forward on two heavy claws that hang to the ground, a gun
    riding each forearm or sitting in the claw's palm. Alien in the shapes: spines down the hump, mandibles and fangs, a
    toothed maw on the two King Crab types, clustered eyes. Tyrant and Bruiser stand on one pair, Shrike and Pincher on two,
    Scuttler and Mite on three short thin pairs. Numbered on from the combat mechs, 23 to 28.</p>
    <p class="hint">In the viewer below: Scene, Crabs. Press Walk, then tap a leg or a claw to shoot it off and watch how the crab copes; Repair puts it back.</p>
  </section>
  <h2 class="boh">The detailed round: the first three at 12 m, the six picked from the block-outs at 24 m</h2>
</header>

<div class="wrap">
  <section>
    <!-- one canvas for every scene (owner, 2026-10-24: "Only one canvas with different scenes please, my phone can only
         render so much"): a scene is built the first time it is opened, and only the one on screen is drawn -->
    <div class="bar" role="group" aria-label="Scene">
      <span class="lbl">Scene</span>
      <button data-scene="detailed" aria-pressed="true">Detailed mechs</button>
      <button data-scene="BLOCKOUTS">Block-outs 1 to 12</button>
      <button data-scene="COMBAT">Combat 13 to 22</button>
      <button data-scene="CRABS">Crabs 23 to 28</button>
    </div>
    <div class="borow" id="bo-pick" hidden></div>
    <div class="borow" id="cb-pick" hidden></div>
    <div class="borow" id="cr-pick" hidden></div>
    <div id="detailed-bars">
    <div class="bar" role="group" aria-label="Mech">
      <span class="lbl">Mech</span>
__MECH_BUTTONS__
      <button id="m-all" data-mech="all">Line up</button>
    </div>
    <div class="bar" role="group" aria-label="Move">
      <span class="lbl">Move</span>
      <button id="a-idle" data-act="idle">Idle</button>
      <button id="a-walk" data-act="walk">Walk</button>
      <button id="a-turn" data-act="turn">Turn</button>
      <button id="a-skate" data-act="skate">Skate</button>
      <button id="a-hit" data-act="hit">Take a hit</button>
    </div>
    <!-- the melee lab (owner, 2026-10-29: "I will need your help desigining melee animaitons in the mech artifact",
         "shooting guns should cause mech arm to recoil"): moves from tools/mockups/mech_melee.js, edited here and saved
         to this page's database, where Claude reads them back into the close-combat change -->
    <div class="bar" role="group" aria-label="Melee" id="mel-bar">
      <span class="lbl">Melee</span>
    </div>
    <div class="bar" role="group" aria-label="Melee playback">
      <span class="lbl">Play</span>
      <button id="mel-play" class="go">Play</button>
      <button id="mel-loop" aria-pressed="false">Loop</button>
      <button id="mel-spar" aria-pressed="true">Sparring partner</button>
      <button id="mel-edit" aria-pressed="false">Edit keys</button>
      <label class="lbl" style="display:flex;gap:6px;align-items:center">Speed <select id="mel-speed" style="font:500 13px var(--mono);min-height:34px"><option value="0.1">0.1x</option><option value="0.25">0.25x</option><option value="0.5">0.5x</option><option value="1" selected>1x</option></select></label>
      <div class="scrub"><input type="range" id="mel-scrub" min="0" max="1000" value="0" aria-label="Move timeline"><output id="mel-t">0.00 s</output></div>
    </div>
    <div class="mel-ed" id="mel-ed" hidden>
      <div class="row keys" id="mel-keys" aria-label="Keys"></div>
      <div class="row">
        <label>Key time <input type="number" id="mel-kt" step="0.01" min="0"></label>
        <label>Hit lands at <input type="number" id="mel-hit" step="0.01" min="0"></label>
        <button id="mel-add">Add key here</button>
        <button id="mel-del">Delete key</button>
        <label>Bone <select id="mel-bone"></select></label>
      </div>
      <div class="grid6" id="mel-sliders"></div>
      <div class="row">
        <button id="mel-save" class="go">Save move</button>
        <button id="mel-reset">Reset to default</button>
        <button id="mel-copy">Copy JSON</button>
        <span class="status" id="mel-status">Pick a key, then a bone, and drag. Turns in radians, slides in metres.</span>
      </div>
    </div>
    <div class="bar" role="group" aria-label="Board">
      <span class="lbl">Board</span>
      <button id="b-open" class="go">Kneel and open</button>
      <button id="b-close" class="go">Close and stand</button>
      <button id="b-board" class="go">Board it</button>
      <button id="b-leave" class="go">Climb out</button>
      <div class="scrub"><input type="range" id="scrub" min="0" max="1000" value="0" aria-label="Boarding timeline"><output id="tlabel">0.0 s</output></div>
    </div>
    <div class="bar" role="group" aria-label="View">
      <span class="lbl">View</span>
      <button id="v-orbit" data-view="orbit">Orbit</button>
      <button id="v-hatch" data-view="hatch">Chest close up</button>
      <button id="v-cockpit" data-view="cockpit">Cockpit</button>
      <button id="v-front" data-view="front">Front</button>
      <button id="v-side" data-view="side">Side</button>
      <button id="v-back" data-view="back">Back</button>
      <span class="lbl" style="margin-left:8px">Show</span>
      <button id="t-city" data-toggle="city" aria-pressed="true">Block</button>
      <button id="t-pilot" data-toggle="pilot" aria-pressed="true">Pilot</button>
    </div>
    </div>
    <div class="stage">
      <canvas id="view" tabindex="0" aria-label="3D view of the selected mech"></canvas>
      <div class="mode" id="mode">Orbit</div>
      <div class="cue" id="cue" hidden></div>
      <div class="load" id="load">Loading the mechs</div>
      <div id="tags"></div>
    </div>
    <p class="hint">Melee: pick a move and press Play; the sparring partner is struck when the blow lands. Edit keys pauses the move on a key so you can pose it; Save keeps it for everyone. Drag to orbit, scroll or pinch to zoom, right drag to pan. In the cockpit, drag to look around; the view rides the pilot's eye through every move.</p>
    <div class="bostats" id="bo-stats" hidden></div>
    <div class="bostats" id="cb-stats" hidden></div>
    <div class="bostats" id="cr-stats" hidden></div>
  </section>
  <aside id="cards"></aside>
</div>

<div class="foot">
  <div>Grid: 4 m cells. The yellow rail is 12 m (survey M1). Sizes and triangle counts are measured off each glb by the generator, not typed.</div>
  <div>Models: <code>tools/blender/build_mechs.py</code> on <code>tools/blender/mech_kit.py</code>; plating: <code>tools/textures/gen_mech_plating.mjs</code>; rig: <code>docs/design/mech-rig.md</code>. Kit, textures and animation after Mining Mike's <code>tank_bosses_kit.js</code>.</div>
</div>

<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/build/three.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/loaders/GLTFLoader.js"></script>
<script src="https://cdn.jsdelivr.net/npm/three@0.147.0/examples/js/controls/OrbitControls.js"></script>
<script>
const META = __META__;
const INTERIOR = __INTERIOR__;
const ORDER = __ORDER__;
const HUD_SRC = "data:image/png;base64,__HUD__";
const T = THREE, TAU = Math.PI * 2;
__MECH_POSE__
__MECH_MELEE__

const canvas = document.getElementById("view");
const renderer = new T.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
renderer.outputEncoding = T.sRGBEncoding;
renderer.toneMapping = T.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.0;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = T.PCFSoftShadowMap;

// dusk, as Mining Mike's Uber Mechs page: violet sky, a low warm sun, cool rim
const scene = new T.Scene();
scene.background = new T.Color(0x2a1f30);
scene.fog = new T.Fog(0x2a1f30, 60, 190);
const camera = new T.PerspectiveCamera(42, 1.6, 0.05, 500);
const controls = new T.OrbitControls(camera, canvas);
controls.enableDamping = true;
scene.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.5));
const sun = new T.DirectionalLight(0xffc89a, 2.0);
sun.position.set(-22, 26, 18); sun.castShadow = true; sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -32, right: 32, top: 32, bottom: -32, near: 1, far: 100 });
sun.shadow.bias = -0.0005;
scene.add(sun);
const rim = new T.DirectionalLight(0x7fa8ff, 0.9); rim.position.set(18, 12, -22); scene.add(rim);

const ground = new T.Mesh(new T.PlaneGeometry(500, 500), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
const grid = new T.GridHelper(160, 40, 0x4a4552, 0x3a3640); grid.position.y = 0.01; scene.add(grid);
const rail = new T.Mesh(new T.BoxGeometry(80, 0.08, 0.08), new T.MeshBasicMaterial({ color: 0xf0b13c }));
rail.position.set(0, 12, -16); scene.add(rail);

const city = new T.Group();
function block(x, z, w, d, storeys, col) {
  const h = storeys * 3.5;
  const m = new T.Mesh(new T.BoxGeometry(w, h, d), new T.MeshStandardMaterial({ color: col, roughness: 0.85 }));
  m.position.set(x, h / 2, z); m.castShadow = m.receiveShadow = true; city.add(m);
  const win = new T.MeshStandardMaterial({ color: 0x1a2230, emissive: 0xffc878, emissiveIntensity: 0.5, roughness: 0.3 });
  for (let s = 0; s < storeys; s++) for (let i = 0; i < Math.floor(w / 2.6); i++) {
    const g = new T.Mesh(new T.BoxGeometry(1.4, 1.6, 0.1), win);
    g.position.set(x - w / 2 + 1.4 + i * 2.6, s * 3.5 + 2.0, z + d / 2 + 0.05); city.add(g);
  }
}
block(-22, -8, 14, 10, 4, 0x6a625a); block(-36, -10, 10, 12, 9, 0x545961); block(22, -12, 12, 10, 2, 0x6e6352);
const paint = new T.MeshStandardMaterial({ color: 0x7a1f1f, roughness: 0.35, metalness: 0.3 });
const carBody = new T.Mesh(new T.BoxGeometry(1.8, 0.75, 4.4), paint); carBody.position.set(-14, 0.55, 7);
const carCab = new T.Mesh(new T.BoxGeometry(1.6, 0.6, 2.2), new T.MeshStandardMaterial({ color: 0x1d2630, roughness: 0.2 })); carCab.position.set(-14, 1.2, 6.8);
[carBody, carCab].forEach(m => { m.castShadow = true; city.add(m); });
scene.add(city);

const pilot = new T.Group();
const suit = new T.MeshStandardMaterial({ color: 0x2e5ea8, roughness: 0.7 }), skin = new T.MeshStandardMaterial({ color: 0xd8b49a, roughness: 0.8 });
[-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), suit); l.position.set(x, 0.43, 0); pilot.add(l); });
const torsoM = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), suit); torsoM.position.y = 1.18; pilot.add(torsoM);
const head = new T.Mesh(new T.SphereGeometry(0.12, 16, 12), skin); head.position.y = 1.66; head.scale.set(1, 1.15, 1); pilot.add(head);
[-1, 1].forEach(s => { const a = new T.Mesh(new T.CylinderGeometry(0.06, 0.045, 0.62, 8), suit); a.position.set(0.3 * s, 1.15, 0); a.rotation.z = 0.12 * s; pilot.add(a); });
pilot.traverse(o => { if (o.isMesh) o.castShadow = true; });
scene.add(pilot);

// the halo sprite, Mining Mike's radial glow
const glowTex = (() => { const c = document.createElement("canvas"); c.width = c.height = 128; const q = c.getContext("2d");
  const g = q.createRadialGradient(64, 64, 0, 64, 64, 64); g.addColorStop(0, "rgba(255,255,255,1)"); g.addColorStop(0.18, "rgba(255,255,255,0.75)");
  g.addColorStop(0.45, "rgba(255,255,255,0.18)"); g.addColorStop(1, "rgba(255,255,255,0)"); q.fillStyle = g; q.fillRect(0, 0, 128, 128); return new T.CanvasTexture(c); })();

// ---------- shared pieces: steam and the screen wall ----------
__STEAM__
__SCREEN_WALL__
const steam = makeSteam(T, scene, 3000);
const wall = makeScreenWall(T, scene, HUD_SRC);
// The interiors' lamps are physical (candela, KHR_lights_punctual); this page is not, so they are scaled to
// sit under the dusk sun, and the exposure opens up inside the bay as an eye adapts.
const CD_TO_PAGE = 1 / 260, EXPOSURE_OUT = 1.0, EXPOSURE_IN = 1.7;

// ---------- load every mech and its interior ----------
__GLTF_JSON__
const loader = gltfJsonLoader(T);
const mechs = {};
const parseX = v => (typeof v === "string" ? JSON.parse(v) : v);
const loadOne = (id, idx) => loader.loadAsync(`mech_${id}.gltf.json`).then(g => {
  const root = g.scene, nodes = {}, halos = [], cores = [];
  root.traverse(o => {
    if (o.name) nodes[o.name] = o;
    if (o.isMesh) { o.castShadow = true; o.receiveShadow = true;
      if (o.material && o.material.transparent) { o.material.depthWrite = false; o.castShadow = false; o.renderOrder = 2; o.material.side = T.DoubleSide; } }
    if (o.name.startsWith("CORE_")) cores.push(o);
  });
  root.traverse(o => { const u = o.userData || {}; if (u.halo_size_m) {
    const mat = new T.SpriteMaterial({ map: glowTex, color: new T.Color(u.halo_color), blending: T.AdditiveBlending, depthWrite: false, transparent: true, opacity: u.halo_opacity, toneMapped: false });
    const sp = new T.Sprite(mat); sp.scale.setScalar(u.halo_size_m); o.add(sp); halos.push(sp); } });
  const x = (nodes.root && nodes.root.userData) || {};
  const b = { id, root, nodes, cores, halos, seed: idx * 7 + 3, ph: 0, t: hash(idx) * 10, scale: x.scale || 0.13,
    legSegs: parseX(x.legSegs), footH: x.footH, walk: parseX(x.walk), idleP: parseX(x.idle), hitT: 9, hitSide: 1, wWalk: 0, wTurn: 0, wSkate: 0, rest: {},
    board: parseX(x.boarding) || { tracks: [] }, cockpit: parseX(x.cockpit) || {}, vents: [], restT: {}, inner: [], interior: null };
  for (const n in nodes) if (MAPPED.has(n)) { const o = nodes[n]; b.rest[n] = { e: new T.Euler().setFromQuaternion(o.quaternion, "XYZ"), p: o.position.clone() }; }
  for (const n in nodes) { const o = nodes[n]; b.restT[n] = { q: o.quaternion.clone(), p: o.position.clone() }; if (n.startsWith("VENT_")) b.vents.push(o); }
  b.timeline = timeline(b);
  mechs[id] = b;
  scene.add(root);
  const v = b.cockpit.variant, I = v && INTERIOR[v];
  if (!I || !nodes.cockpit) return;
  return loader.loadAsync(I.file).then(gi => {
    const pod = gi.scene;
    pod.traverse(o => {
      if (o.isLight) { o.intensity *= CD_TO_PAGE; if (o.isSpotLight || o.isPointLight) o.castShadow = false; return; }
      if (!o.isMesh) return;
      o.castShadow = o.receiveShadow = true;
      for (const m of [].concat(o.material)) {
        if (m.name === "SCREEN_allview") { o.material = wall.material; o.castShadow = false; }
        else if (m.name.startsWith("SCREEN_")) m.toneMapped = false;
      }
    });
    nodes.cockpit.add(pod);
    b.interior = pod;
    const pn = {}; pod.traverse(o => { if (o.name) pn[o.name] = o; });
    b.pnodes = pn;
    // a chest that opens on the interior's own parts (mech_boarding mode "inner") carries each part on its last
    // joint, so the two open as one body: put the chest at rest, hang the part on the joint, skip its own tracks
    const rides = b.cockpit.rides || {}, ridden = Object.keys(rides).filter(n => pn[n] && nodes[rides[n]]);
    if (ridden.length) {
      for (const tr of b.board.tracks) { const o = nodes[tr.node], r = b.restT[tr.node]; if (o && r) { o.position.copy(r.p); o.quaternion.copy(r.q); } }
      root.updateMatrixWorld(true);
      for (const n of ridden) nodes[rides[n]].attach(pn[n]);
    }
    for (const tr of innerTracks(I.man)) if (pn[tr.node] && !ridden.some(n => tr.node.startsWith(n))) { b.inner.push(tr); b.restT["~" + tr.node] = { q: pn[tr.node].quaternion.clone(), p: pn[tr.node].position.clone() }; }
    b.timeline = timeline(b);
  }).catch(e => console.warn("interior", v, e));
});

// The interior's manifest names its moving nodes and how each opens; accept the shapes the generator writes.
function innerTracks(man) {
  const out = [], push = (node, m) => { if (!m) return; const t = Object.assign({ t0: 0, d: 1 }, m, { node });
    if (t.slide && !t.pos) { t.pos = t.slide; } if (t.axis && t.axis.length > 1 && t.axis.startsWith("pos_")) { t.pos = t.axis.slice(4); delete t.axis; }
    if (t.value !== undefined && t.v === undefined) t.v = t.value; if (t.start !== undefined) t.t0 = t.start; if (t.duration !== undefined) t.d = t.duration;
    out.push(t); };
  if (Array.isArray(man.tracks)) man.tracks.forEach(t => push(t.node, t));
  if (Array.isArray(man.open)) man.open.forEach(t => push(t.node, t));
  for (const [n, d] of Object.entries(man.motions || {})) push(n, d);
  if (!man.motions) for (const [n, d] of Object.entries(man.nodes || {})) push(n, d.open || d.motion);
  return out;
}

// ---------- the boarding timeline: sit, the chest opens, the interior opens, the ladder drops ----------
const SIT = [0, 3.0], CHEST_AT = 3.2;
function timeline(b) {
  const chestEnd = Math.max(0, ...b.board.tracks.map(t => t.t0 + t.d));
  const innerAt = CHEST_AT + chestEnd * 0.8;
  const innerEnd = Math.max(0, ...b.inner.map(t => t.t0 + t.d));
  const lad = b.board.ladder || { segs: 0 };
  const ladderAt = CHEST_AT + chestEnd + 0.1;
  const end = Math.max(innerAt + innerEnd, ladderAt + lad.segs * 0.12 + 0.4) + 0.3;
  return { chestEnd, innerAt, ladderAt, end, sealed: CHEST_AT };
}
const AX = { x: new T.Vector3(1, 0, 0), y: new T.Vector3(0, 1, 0), z: new T.Vector3(0, 0, 1) };
function play(o, r, tr, k) {
  if (tr.pos) { o.position.copy(r.p); o.position[tr.pos] += tr.v * k; }
  else o.quaternion.copy(r.q).multiply(new T.Quaternion().setFromAxisAngle(AX[tr.axis], tr.v * k));
}
const tmpV = new T.Vector3(), tmpW = new T.Vector3();
function setBoard(b, Tb) {
  const L = b.timeline;
  b.sitK = ease((Tb - SIT[0]) / (SIT[1] - SIT[0]));
  for (const tr of b.board.tracks) { const o = b.nodes[tr.node]; if (o) play(o, b.restT[tr.node], tr, ease((Tb - CHEST_AT - tr.t0) / tr.d)); }
  for (const tr of b.inner) { const o = b.pnodes[tr.node]; if (o) play(o, b.restT["~" + tr.node], tr, ease((Tb - L.innerAt - tr.t0) / tr.d)); }
}
function aimRams(b) {
  for (const r of b.board.rams || []) {
    const base = b.nodes[r.base], rod = b.nodes[r.rod], tip = b.nodes[r.tip];
    if (!base || !rod || !tip) continue;
    tip.getWorldPosition(tmpW);
    base.lookAt(tmpW);
    base.updateMatrixWorld(true);
    const D = base.getWorldPosition(tmpV).distanceTo(tmpW);
    rod.position.set(0, 0, Math.max(0, D - r.rod_len));
  }
}
function dropLadder(b, Tb) {
  const lad = b.board.ladder; if (!lad) return;
  const top = b.nodes[lad.node]; if (!top) return;
  top.updateMatrixWorld(true);
  const y0 = top.getWorldPosition(tmpV).y;
  for (let i = 0; i < lad.segs; i++) {
    const o = b.nodes[`LADDER_${i + 1}`]; if (!o) continue;
    const r = b.restT[o.name], k = ease((Tb - b.timeline.ladderAt - i * 0.12) / 0.3);
    o.position.copy(r.p); o.position.y -= k * (i + 1) * lad.seg_m;
    o.visible = k > 0.02 && y0 - (i + 1) * lad.seg_m * k > -0.4;
  }
}

let current = "foreman", view = "orbit", act = "idle";
let Tb = 0, target = 0, rate = 1, script = null, walk = null, powerOn = true;
const tagsEl = document.getElementById("tags"), cueEl = document.getElementById("cue");
const scrub = document.getElementById("scrub"), tlabel = document.getElementById("tlabel");
function layout() {
  const spread = current === "all";
  ORDER.forEach((id, i) => { const m = mechs[id]; if (!m) return; m.root.visible = spread || id === current; m.root.position.set(spread ? (i - 1) * 17 + Math.max(0, i - 2) * 4 : 0, 0, 0); });
  pilot.position.set(spread ? -9 : 5.5, 0, 7); pilot.rotation.y = -0.6;
  for (const id of ORDER) if (mechs[id]) mechs[id].yaw0 = 0;
  const sp = sparring();
  if (sp) { sp.root.visible = true; sp.yaw0 = Math.PI; sp.root.rotation.y = Math.PI; sp.root.position.set(0, 0, SPAR_M); }
}
const cur = () => mechs[current === "all" ? ORDER[0] : current];
function nodeWorld(b, name) { const p = new T.Vector3(); if (b && b.nodes[name]) b.nodes[name].getWorldPosition(p); return p; }
function eyeOf(id) { return nodeWorld(mechs[id], "EYE_pilot"); }
function setView(v) {
  view = v;
  document.querySelectorAll("[data-view]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
  document.getElementById("mode").textContent = { orbit: "Orbit", hatch: "Chest close up", hatch3q: "Three quarter", cockpit: "Cockpit, pilot's eye", eyes: "The pilot's eyes", front: "Front", side: "Side", back: "Back" }[v];
  if ((v === "cockpit" || v === "hatch" || v === "hatch3q") && current === "all") setMech(ORDER[0], true);
  if (v === "cockpit" || v === "eyes") {
    const e = eyeOf(current);
    camera.fov = 78; camera.near = 0.02; camera.updateProjectionMatrix();
    camera.position.copy(e); controls.target.copy(e).add(fwdOf(cur()).multiplyScalar(1).add(new T.Vector3(0, -0.1, 0)));
    Object.assign(controls, { minDistance: 0.01, maxDistance: 1.0, enablePan: false, enableZoom: false, rotateSpeed: -0.35, enabled: v === "cockpit" });
  } else {
    camera.fov = 42; camera.near = 0.05; camera.updateProjectionMatrix();
    Object.assign(controls, { minDistance: 2, maxDistance: 140, enablePan: true, enableZoom: true, rotateSpeed: 1, enabled: true });
    const d = current === "all" ? 70 : 40;            // the mechs stand 22 to 26 m: frame all of one
    controls.target.set(0, 11, 0);
    if (v === "hatch" || v === "hatch3q") { const h = nodeWorld(cur(), "HATCH_entry"), f = fwdOf(cur()), sd = new T.Vector3(f.z, 0, -f.x);
      const far = v === "hatch3q";
      // the three-quarter view takes in the whole chest and its armour swinging clear, at 24 m scale
      controls.target.copy(far ? new T.Vector3(h.x, h.y + 0.5, h.z) : h.clone().add(new T.Vector3(0, 0.8, 0)));
      camera.position.copy(h).addScaledVector(f, far ? 17 : 7.0).addScaledVector(sd, far ? 10 : 2.2).add(new T.Vector3(0, far ? 3.0 : 1.4, 0)); }
    else if (v === "front") camera.position.set(0, 12, d); else if (v === "side") camera.position.set(d, 12, 0);
    else if (v === "back") camera.position.set(-d * 0.5, 15, -d * 0.85); else camera.position.set(d * 0.7, 15, d * 0.8);
  }
  controls.update();
}
function fwdOf(b) { const q = new T.Quaternion(); (b.nodes.cockpit || b.root).getWorldQuaternion(q); return new T.Vector3(0, 0, 1).applyQuaternion(q).setY(0).normalize(); }
// ---------- scenes: the detailed mechs, or one of the block-out sets, on the one canvas ----------
let sceneId = "detailed";
const VIEWERS = {};
function setScene(id) {
  if (id === "detailed") loadDetailed();
  const ld = document.getElementById("load"); if (ld) ld.hidden = id !== "detailed";
  if (id === sceneId) return;
  sceneId = id;
  document.querySelectorAll("[data-scene]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.scene === id)));
  document.getElementById("detailed-bars").hidden = id !== "detailed";
  document.getElementById("cards").hidden = id !== "detailed";
  for (const k in VIEWERS) VIEWERS[k].activate(k === id);
  tagsEl.innerHTML = ""; cueEl.hidden = true;
  if (id === "detailed") setView(view); else { controls.enabled = false; document.getElementById("mode").textContent = VIEWERS[id].label; }
}
document.querySelectorAll("[data-scene]").forEach(b => b.addEventListener("click", () => setScene(b.dataset.scene)));
function setMech(id, keep) {
  if (sceneId !== "detailed") setScene("detailed");
  current = id; script = null; walk = null; Tb = target = 0;
  document.querySelectorAll("[data-mech]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.mech === id)));
  document.querySelectorAll(".card").forEach(c => c.classList.toggle("sel", c.dataset.id === id));
  layout(); if (!keep) setView(view === "eyes" ? "orbit" : (view === "cockpit" || view === "hatch") && id === "all" ? "orbit" : view);
}
function setAct(a) {
  if (a === "hit") { for (const id in mechs) { mechs[id].hitT = 0; mechs[id].hitSide = Math.random() < 0.5 ? 1 : -1; } return; }
  act = a; document.querySelectorAll("[data-act]").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.act === a)));
}
document.querySelectorAll("[data-mech]").forEach(b => b.addEventListener("click", () => setMech(b.dataset.mech)));
document.querySelectorAll("[data-view]").forEach(b => b.addEventListener("click", () => setView(b.dataset.view)));
document.querySelectorAll("[data-act]").forEach(b => b.addEventListener("click", () => setAct(b.dataset.act)));
document.querySelectorAll("[data-toggle]").forEach(b => b.addEventListener("click", () => {
  const on = b.getAttribute("aria-pressed") !== "true"; b.setAttribute("aria-pressed", String(on));
  if (b.dataset.toggle === "city") city.visible = on; if (b.dataset.toggle === "pilot") pilot.visible = on;
}));
scrub.addEventListener("input", () => { script = null; const b = cur(); if (b) Tb = target = scrub.value / 1000 * b.timeline.end; });

// ---------- scripted sequences: open, close, board, climb out ----------
function run(steps) { script = steps.map(s => Object.assign({ started: false, t: 0 }, s)); }
function stepScript(dt) {
  if (!script || !script.length) return;
  const s = script[0];
  if (!s.started) { s.started = true; if (s.fn) s.fn(); if (s.kind === "do") { script.shift(); return; } }
  s.t += dt;
  const end = cur().timeline.end;
  if (s.kind === "timeline") { const to = s.to === "end" ? end : s.to; target = to; rate = s.rate || 1; if (Math.abs(Tb - to) < 1e-3) script.shift(); }
  else if (s.kind === "wait") { if (s.t >= s.d) script.shift(); }
  else if (s.kind === "walk") { if (!walk || walk.done) script.shift(); }
}
const wantIdle = () => { setAct("idle"); };
document.getElementById("b-open").onclick = () => { if (current === "all") setMech(ORDER[0]); wantIdle(); run([{ kind: "timeline", to: "end" }]); };
document.getElementById("b-close").onclick = () => run([{ kind: "timeline", to: 0 }]);
document.getElementById("b-board").onclick = () => { if (current === "all") setMech(ORDER[0]); wantIdle(); run([
  { kind: "do", fn: () => { powerOn = false; setView("eyes"); walk = { inbound: true, wait: true, done: false }; } },
  { kind: "timeline", to: "end" },
  { kind: "walk" },
  { kind: "wait", d: 0.5 },
  { kind: "timeline", to: CHEST_AT + 0.6, rate: 0.9 },
  { kind: "do", fn: () => { sealPuff = 1.0; } },
  { kind: "wait", d: 0.8 },
  { kind: "do", fn: () => { powerOn = true; } },
  { kind: "wait", d: 2.6 },
  { kind: "timeline", to: 0, rate: 0.8 },
]); };
document.getElementById("b-leave").onclick = () => { if (current === "all") setMech(ORDER[0]); run([
  { kind: "do", fn: () => { setView("eyes"); walk = { hold: true, done: false }; } },
  { kind: "timeline", to: CHEST_AT + 0.6 },
  { kind: "do", fn: () => { powerOn = false; } },
  { kind: "wait", d: 0.6 },
  { kind: "timeline", to: "end" },
  { kind: "do", fn: () => { walk = { inbound: false, done: false }; } },
  { kind: "walk" },
]); };
let sealPuff = 0;

// The pilot's walk: from the ground to the ladder's foot (or, on a chest with no ladder, the foot of the sill),
// up, onto the sill, in, turn and settle at the eye. Waypoints in world space from the mech as it stands open; yaw is about the bay's forward.
function walkPath(b) {
  const f = fwdOf(b), side = new T.Vector3(f.z, 0, -f.x), lad = b.board.ladder;
  const sill = nodeWorld(b, "HATCH_entry"), eye = eyeOf(current);
  const ladTop = nodeWorld(b, lad ? lad.node : "HATCH_entry");
  const foot = ladTop.clone().setY(0).addScaledVector(f, 0.55);
  const eyeH = 1.7, yawF = Math.atan2(-f.x, -f.z);   // looking back at the mech
  const P = (v, yaw, pitch, d) => ({ p: v, yaw, pitch, d });
  return [
    P(foot.clone().addScaledVector(f, 5).add(new T.Vector3(0, eyeH, 0)), yawF, 0.35, 0),
    P(foot.clone().add(new T.Vector3(0, eyeH, 0)), yawF, 0.7, 3.0),
    P(foot.clone().add(new T.Vector3(0, Math.max(eyeH, sill.y - 0.3), 0)), yawF, 0.2, Math.max(1.5, sill.y * 0.55)),
    P(sill.clone().addScaledVector(f, 0.35).add(new T.Vector3(0, 1.2, 0)), yawF, -0.25, 1.2),
    P(sill.clone().addScaledVector(f, -0.6).add(new T.Vector3(0, 1.1, 0)), yawF + 0.3, -0.3, 1.2),
    P(eye.clone().addScaledVector(side, 0.15), yawF + 1.6, -0.15, 1.1),
    P(eye.clone(), yawF + Math.PI, -0.08, 1.4),
  ];
}
const eyeLook = { yaw: 0, pitch: 0 };
function stepWalk(dt) {
  const b = cur(); if (!walk || !b) return;
  if (walk.hold) { placeEye(eyeOf(current), Math.atan2(-fwdOf(b).x, -fwdOf(b).z) + Math.PI, -0.08); return; }
  if (!walk.pts) {
    if (walk.wait && Tb < b.timeline.end - 0.05) { const p = walkPath(b)[0]; placeEye(p.p, p.yaw, p.pitch); return; }
    let pts = walkPath(b);
    if (!walk.inbound) pts = pts.slice().reverse().map((p, i, a) => Object.assign({}, p, { d: i ? a[i - 1].d : 0, yaw: p.yaw + (i > a.length - 4 ? Math.PI : 0) }));
    walk.pts = pts; walk.i = 0; walk.u = 0;
  }
  const a = walk.pts[walk.i], c = walk.pts[walk.i + 1];
  if (!c) { walk.done = true; if (walk.inbound) walk = { hold: true, done: true }; return; }
  walk.u += dt / Math.max(0.2, c.d);
  const k = ease(walk.u);
  const p = a.p.clone().lerp(c.p, k);
  p.y += 0.025 * Math.sin(walk.u * Math.PI * 6);
  placeEye(p, a.yaw + (c.yaw - a.yaw) * k, a.pitch + (c.pitch - a.pitch) * k);
  if (walk.u >= 1) { walk.i++; walk.u = 0; }
}
function placeEye(p, yaw, pitch) {
  camera.position.copy(p);
  camera.quaternion.setFromEuler(new T.Euler(pitch, yaw, 0, "YXZ"));
  camera.fov = 78; camera.updateProjectionMatrix();
}

const cards = document.getElementById("cards");
ORDER.forEach(id => {
  const m = META[id], b = m.bounds_m, el = document.createElement("article");
  el.className = "card"; el.dataset.id = id;
  el.innerHTML = `<h2>${m.name}${id === "foreman" ? " <small>RECOMMENDED</small>" : ""}</h2>
    <dl class="kv"><dt>Height</dt><dd>${b.y[1].toFixed(1)} m</dd><dt>Width</dt><dd>${(b.x[1] - b.x[0]).toFixed(1)} m</dd>
    <dt>Length</dt><dd>${(b.z[1] - b.z[0]).toFixed(1)} m</dd><dt>Legs</dt><dd>${m.legs}</dd><dt>Cockpit</dt><dd>${m.cockpit}</dd>
    <dt>Opens</dt><dd>${m.opening || ""}</dd>
    <dt>Weapons</dt><dd>${m.weapons}</dd><dt>Triangles</dt><dd>${m.triangles.toLocaleString("en-US")} mech${m.interior_tris ? " + " + m.interior_tris.toLocaleString("en-US") + " cockpit" : ""}</dd></dl>
    <p class="note">${m.note}</p>`;
  el.addEventListener("click", () => setMech(id));
  cards.appendChild(el);
});

const CUES = [[0.0, "Kneeling"], [CHEST_AT, "Latches release, pressure equalising"], [CHEST_AT + 0.8, "Chest armour opens"], [99, ""]];
function animate(b, dt, Tm) {
  b.t += dt; b.hitT += dt;
  const busy = Tm > 0.01 && b.id === current;
  b.wWalk += ((act === "walk" && !busy ? 1 : 0) - b.wWalk) * Math.min(1, dt * 4);
  b.wTurn += ((act === "turn" && !busy ? 1 : 0) - b.wTurn) * Math.min(1, dt * 6);
  b.wSkate += ((act === "skate" && !busy ? 1 : 0) - b.wSkate) * Math.min(1, dt * 3);
  b.ph += dt * TAU * b.walk.hz * (b.wWalk + 0.9 * b.wTurn);
  if (b.id === current) setBoard(b, Tm); else setBoard(b, 0);
  const P = new Pose();
  P.w = (1 - b.wWalk) * (1 - b.sitK); idleGeneric(P, b.t, b);
  P.w = b.wWalk; walkGeneric(P, b.t, b);
  P.w = b.wTurn * (1 - b.wWalk); turnStep(P, b);
  P.w = b.wSkate; skatePose(P, b);
  P.w = 1; sitPose(P, b, b.sitK);
  if (b.hitT < 0.9) { P.w = 1; hitGeneric(P, b.hitT, b); }
  if (b.hold) { P.w = 1; for (const n in b.hold) { const v = b.hold[n]; P.add(n, v[0], v[1], v[2], v[3], v[4], v[5]); } }
  if (b === cur() && mel.id && MELEE_MOVES[mel.id]) { P.w = 1; meleePose(P, b, MELEE_MOVES[mel.id], mel.t); }
  applyPose(b, P);
  const yaw0 = b.yaw0 || 0;
  if (b.wTurn > 0.05) b.root.rotation.y += dt * 0.6 * b.wTurn; else b.root.rotation.y = yaw0 + (b.root.rotation.y - yaw0) * Math.max(0, 1 - dt * 2);
  for (const c of b.cores) c.rotation.y += dt * 1.4;
  b.root.updateMatrixWorld(true);
  aimRams(b);
  dropLadder(b, b.id === current ? Tm : 0);
}
const q4 = new T.Quaternion();
function vent(b, Tm, prevT, dt) {
  const moving = Math.abs(Tm - prevT) > 1e-5, L = b.timeline, open = Tm >= L.end - 0.05;
  for (const v of b.vents) {
    const u = v.userData, st = u.stage;
    let live = false, k = 1;
    if (st === "seam") live = moving && Tm > CHEST_AT - 0.1 && Tm < CHEST_AT + 1.2;
    else if (st === "open") live = moving && Tm > CHEST_AT + 0.3 && Tm < CHEST_AT + L.chestEnd;
    else if (st === "sill") { live = Tm > CHEST_AT + L.chestEnd * 0.6; k = 0.7; }
    if (sealPuff > 0 && st === "seam") live = Math.random() < 0.6;
    if (!live) continue;
    const n = (u.kind === "mist" ? 9 : 30) * (u.rate || 1) * dt * k, c = Math.floor(n) + (Math.random() < n % 1 ? 1 : 0);
    if (!c) continue;
    v.parent.getWorldQuaternion(q4);
    const dir = new T.Vector3(...parseX(u.dir)).applyQuaternion(q4), pw = v.getWorldPosition(new T.Vector3());
    for (let i = 0; i < c; i++) steam.emit(pw, dir, u.kind === "mist", 1, u.kind === "mist" ? 1.3 : 1.2);
  }
  if (moving && b.sitK > 0.02 && b.sitK < 0.98)      // the legs' rams vent as they work
    for (const nm of ["shin_L", "shin_R", "hock_L", "hock_R"]) if (b.nodes[nm] && Math.random() < 0.5)
      steam.emit(b.nodes[nm].getWorldPosition(new T.Vector3()), new T.Vector3(nm.endsWith("L") ? 1 : -1, 0.4, 0.3).normalize(), false, 1.2, 1.5);
}

function placeTags() {
  tagsEl.innerHTML = ""; if (view === "cockpit" || view === "eyes") return;
  const r = canvas.getBoundingClientRect();
  const add = (p, text) => { const v = p.clone().project(camera); if (v.z > 1 || Math.abs(v.x) > 1.05 || Math.abs(v.y) > 1.05) return;
    const d = document.createElement("div"); d.className = "tag"; d.textContent = text;
    d.style.left = ((v.x + 1) / 2 * r.width) + "px"; d.style.top = ((1 - v.y) / 2 * r.height) + "px"; tagsEl.appendChild(d); };
  if (pilot.visible) add(new T.Vector3(pilot.position.x, 2.1, pilot.position.z), "pilot 1.8 m");
  if (city.visible) { add(new T.Vector3(-22, 14.6, -3), "4 storeys, 14 m"); add(new T.Vector3(-14, 2.2, 7), "car 4.4 m"); }
  add(new T.Vector3(34, 12.4, -16), "12 m");
}
function resize() {
  const w = canvas.clientWidth, h = canvas.clientHeight, pr = renderer.getPixelRatio();
  if (canvas.width !== Math.round(w * pr) || canvas.height !== Math.round(h * pr)) { renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); }
}
let last = 0, frame = 0;
function loop(ms) {
  const dt = Math.min(0.05, (ms - last) / 1000 || 0.016); last = ms;
  resize();
  if (sceneId !== "detailed") { VIEWERS[sceneId].render(); requestAnimationFrame(loop); return; }
  const b = cur();
  if (b) {
    stepScript(dt);
    const prevT = Tb;
    if (Tb < target) Tb = Math.min(target, Tb + dt * rate); else if (Tb > target) Tb = Math.max(target, Tb - dt * rate);
    scrub.value = Math.round(Tb / b.timeline.end * 1000); tlabel.textContent = Tb.toFixed(1) + " s";
    stepMelee(dt);
    for (const id in mechs) if (mechs[id].root.visible) animate(mechs[id], dt, Tb);
    vent(b, Tb, prevT, dt);
    if (sealPuff > 0) sealPuff -= dt;
    let cue = ""; for (const [ct, c] of CUES) if (Tb >= ct - 1e-3) cue = c || cue;
    if (Tb > b.timeline.innerAt) cue = "Cockpit opens";
    if (b.board.ladder && Tb > b.timeline.ladderAt) cue = "Ladder drops";
    cueEl.hidden = !(Tb > 0.01 && Tb < b.timeline.end - 0.01); cueEl.textContent = cue;
  }
  steam.step(dt, camera, renderer);
  if (view === "eyes") stepWalk(dt);
  else if (view === "cockpit") { const e = eyeOf(current), dl = e.clone().sub(camera.position); camera.position.add(dl); controls.target.add(dl); controls.update(); }
  else controls.update();
  if (b && b.interior) {
    const e = eyeOf(current);
    wall.update(dt, e, b.nodes.cockpit, powerOn, true);
    const inside = (view === "cockpit" || view === "eyes") && camera.position.distanceTo(e) < 1.6;
    if (inside) wall.capture(renderer, e, [b.root, steam.cloud]);
    renderer.toneMappingExposure += ((inside ? EXPOSURE_IN : EXPOSURE_OUT) - renderer.toneMappingExposure) * Math.min(1, dt * 3);
  }
  renderer.render(scene, camera);
  if ((frame++ & 3) === 0) placeTags();
  requestAnimationFrame(loop);
}
// the detailed mechs (most of the page's weight) load when their scene first opens, so a link to #crabs, #combat or
// #blockouts opens straight into that scene without them (owner, 2026-10-24: "my phone can only render so much")
let loaded = null;
function loadDetailed() {
  if (loaded) return loaded;
  loaded = Promise.all(ORDER.map(loadOne)).catch(e => { document.getElementById("load").textContent = "Could not load: " + e; throw e; });
  loaded.then(() => { document.getElementById("load").remove(); setMech("foreman"); setAct("idle"); setView("orbit"); window.MOCKUP_READY = true; });
  return loaded;
}
// ---------- the melee lab ----------
// One move plays at a time on the selected mech (mel), on its own clock, layered over the gait (so Walk and Fire R is
// the gun kicking as the mech walks); a gun's recoil is a move like the rest, fired again each loop with a muzzle flash. The sparring partner is the next mech in the line, facing ours SPAR_M ahead,
// struck (hitGeneric) when a move's hit_s passes. Edits change MELEE_MOVES in place and are saved to the db.
const SPAR_M = 15;
const mel = { id: null, t: 0, playing: false, loop: false, spar: true, edit: false, speed: 1, key: 0, bone: "shoulder_R", wait: 0 };
function sparring() {
  if (!mel.spar || current === "all" || sceneId !== "detailed") return null;
  const i = ORDER.indexOf(current), id = ORDER[(i + 1) % ORDER.length];
  return mechs[id] || null;
}
function strike(move) {
  const sp = sparring(); if (!sp) return;
  sp.hitT = 0; sp.hitSide = move.side || (Math.random() < 0.5 ? 1 : -1);
}
function fire(b, m) {
  const arms = m.arm === "both" ? ["L", "R"] : [m.arm];
  for (const a of arms) { const mz = b.nodes["MUZZLE_" + a]; if (mz) flashAt(mz); }
}
const flashes = [];
function flashAt(node) {
  const s = new T.Sprite(new T.SpriteMaterial({ color: 0xffd28a, transparent: true, opacity: 1, depthWrite: false, blending: T.AdditiveBlending }));
  s.scale.set(3, 3, 3); node.add(s); flashes.push({ s, t: 0 });
}
function stepMelee(dt) {
  const b = cur(); if (!b) return;
  for (const f of flashes) { f.t += dt; f.s.material.opacity = Math.max(0, 1 - f.t / 0.08); f.s.scale.setScalar(3 + f.t * 30); }
  while (flashes.length && flashes[0].t > 0.08) { const f = flashes.shift(); f.s.parent.remove(f.s); f.s.material.dispose(); }
  const m = mel.id && MELEE_MOVES[mel.id];
  if (!m || !mel.playing) return;
  if (mel.wait > 0) { mel.wait -= dt; if (mel.wait <= 0) { mel.t = 0; if (m.fire) fire(b, m); } return; }
  const t0 = mel.t, end = meleeEnd(m);
  mel.t += dt * mel.speed;
  if (!m.fire && t0 < m.hit_s && mel.t >= m.hit_s) strike(m);
  if (mel.t >= end) {
    if (mel.loop) { mel.t = end; mel.wait = m.fire ? 0.12 : 0.6; }
    else { mel.t = end; mel.playing = false; playBtn.textContent = "Play"; }
  }
  drawMelee();
}
const playBtn = document.getElementById("mel-play"), melScrub = document.getElementById("mel-scrub"), melT = document.getElementById("mel-t");
function pickMove(id) {
  mel.id = id; mel.t = 0; mel.key = Math.min(mel.key, MELEE_MOVES[id].keys.length - 1);
  const m = MELEE_MOVES[id];
  if (!m.keys.some(k => k.b && k.b[mel.bone])) { const k = m.keys.find(k => k.b && Object.keys(k.b).length); if (k) mel.bone = Object.keys(k.b)[0]; }
  document.querySelectorAll("[data-mel]").forEach(e => e.setAttribute("aria-pressed", String(e.dataset.mel === id)));
  playMove();
}
function playMove() {
  const m = mel.id && MELEE_MOVES[mel.id]; if (!m) return;
  if (mel.edit) setEdit(false);
  mel.t = 0; mel.wait = 0; mel.playing = true; playBtn.textContent = "Again";
  layout(); if (m.fire && cur()) fire(cur(), m);
}
function drawMelee() {
  const m = mel.id && MELEE_MOVES[mel.id], end = m ? meleeEnd(m) : 1;
  melScrub.value = Math.round(Math.min(1, mel.t / Math.max(1e-6, end)) * 1000); melT.textContent = mel.t.toFixed(2) + " s";
}
const MOVE_LABEL = { jab_L: "Jab L", jab_R: "Jab R", hook_L: "Hook L", hook_R: "Hook R", uppercut_L: "Uppercut L", uppercut_R: "Uppercut R",
  smash: "Smash", charge: "Charge", kick_L: "Kick L", kick_R: "Kick R", recoil_L: "Fire L", recoil_R: "Fire R" };
{
  const bar = document.getElementById("mel-bar");
  for (const id of MELEE_ORDER) { const e = document.createElement("button"); e.dataset.mel = id; e.textContent = MOVE_LABEL[id] || id; e.title = MELEE_MOVES[id].name; e.addEventListener("click", () => pickMove(id)); bar.appendChild(e); }
}
playBtn.addEventListener("click", () => { if (!mel.id) pickMove("jab_R"); else playMove(); });
document.getElementById("mel-loop").addEventListener("click", e => { mel.loop = !mel.loop; e.currentTarget.setAttribute("aria-pressed", String(mel.loop)); if (mel.loop && mel.id && !mel.playing) playMove(); });
document.getElementById("mel-spar").addEventListener("click", e => { mel.spar = !mel.spar; e.currentTarget.setAttribute("aria-pressed", String(mel.spar)); layout(); });
document.getElementById("mel-speed").addEventListener("change", e => { mel.speed = +e.target.value; });
melScrub.addEventListener("input", () => { const m = mel.id && MELEE_MOVES[mel.id]; if (!m) return; mel.playing = false; playBtn.textContent = "Play"; mel.t = melScrub.value / 1000 * meleeEnd(m); melT.textContent = mel.t.toFixed(2) + " s"; });
// ----- the keyframe editor -----
const ed = document.getElementById("mel-ed"), keysEl = document.getElementById("mel-keys"), boneSel = document.getElementById("mel-bone"),
  slidersEl = document.getElementById("mel-sliders"), ktIn = document.getElementById("mel-kt"), hitIn = document.getElementById("mel-hit"),
  stEl = document.getElementById("mel-status");
for (const n of [...MELEE_BONES, "crouch"]) { const o = document.createElement("option"); o.value = n; o.textContent = n === "crouch" ? "crouch (legs fold)" : n; boneSel.appendChild(o); }
const MEL_AX = [["rx", -3.2, 3.2, 0.01], ["ry", -3.2, 3.2, 0.01], ["rz", -3.2, 3.2, 0.01], ["px", -6, 6, 0.05], ["py", -6, 6, 0.05], ["pz", -6, 6, 0.05]];
const sl = MEL_AX.map(([n, lo, hi, st], j) => {
  const w = document.createElement("div"); w.className = "sl";
  w.innerHTML = `<span class="lbl">${n}</span><input type="range" min="${lo}" max="${hi}" step="${st}" aria-label="${n}"><output>0</output>`;
  slidersEl.appendChild(w);
  const r = w.querySelector("input"), out = w.querySelector("output");
  r.addEventListener("input", () => { setVal(j, +r.value); out.textContent = (+r.value).toFixed(2); });
  return { w, r, out };
});
function curKey() { const m = MELEE_MOVES[mel.id]; return m && m.keys[mel.key]; }
function setVal(j, v) {
  const k = curKey(); if (!k) return;
  if (mel.bone === "crouch") { k.crouch = v; }
  else { k.b = k.b || {}; const a = (k.b[mel.bone] || [0, 0, 0, 0, 0, 0]).slice(); while (a.length < 6) a.push(0); a[j] = v; k.b[mel.bone] = a; }
  dirty(true);
}
let isDirty = false;
function dirty(d) { isDirty = d; stEl.textContent = d ? "Unsaved changes." : stEl.textContent; }
function drawEditor() {
  const m = MELEE_MOVES[mel.id]; if (!m) return;
  keysEl.innerHTML = "";
  m.keys.forEach((k, i) => { const e = document.createElement("button"); e.textContent = `${i + 1}: ${k.t.toFixed(2)} s`;
    e.setAttribute("aria-pressed", String(i === mel.key)); if (Math.abs(k.t - m.hit_s) < 1e-3) e.classList.add("hit");
    e.addEventListener("click", () => { mel.key = i; drawEditor(); }); keysEl.appendChild(e); });
  const k = m.keys[mel.key]; mel.t = k.t; mel.playing = false; drawMelee();
  ktIn.value = k.t.toFixed(2); hitIn.value = (m.hit_s || 0).toFixed(2); boneSel.value = mel.bone;
  const crouch = mel.bone === "crouch", v = crouch ? [k.crouch || 0] : ((k.b && k.b[mel.bone]) || []);
  sl.forEach((s, j) => { s.w.hidden = crouch && j > 0; s.w.querySelector(".lbl").textContent = crouch ? "k" : MEL_AX[j][0];
    const x = v[j] || 0; s.r.min = crouch ? 0 : MEL_AX[j][1]; s.r.max = crouch ? 1.25 : MEL_AX[j][2]; s.r.value = x; s.out.textContent = x.toFixed(2); });
  // the end keys are rest: a move starts and ends at the gait pose
  const ends = mel.key === 0 || mel.key === m.keys.length - 1;
  sl.forEach(s => s.r.disabled = ends);
  if (ends) stEl.textContent = "The first and last keys are rest. Pick a key between them to pose it.";
}
function setEdit(on) {
  if (on && !mel.id) pickMove("jab_R");
  mel.edit = on; ed.hidden = !on; document.getElementById("mel-edit").setAttribute("aria-pressed", String(on));
  if (on) { mel.playing = false; playBtn.textContent = "Play"; if (mel.key === 0) mel.key = Math.min(1, MELEE_MOVES[mel.id].keys.length - 1); drawEditor(); }
}
document.getElementById("mel-edit").addEventListener("click", () => setEdit(!mel.edit));
boneSel.addEventListener("change", () => { mel.bone = boneSel.value; drawEditor(); });
ktIn.addEventListener("change", () => {
  const m = MELEE_MOVES[mel.id], i = mel.key; if (!m) return;
  const lo = i > 0 ? m.keys[i - 1].t + 0.01 : 0, hi = i < m.keys.length - 1 ? m.keys[i + 1].t - 0.01 : 10;
  m.keys[i].t = Math.min(hi, Math.max(lo, +ktIn.value || 0)); dirty(true); drawEditor();
});
hitIn.addEventListener("change", () => { const m = MELEE_MOVES[mel.id]; if (!m) return; m.hit_s = Math.max(0, +hitIn.value || 0); dirty(true); drawEditor(); });
document.getElementById("mel-add").addEventListener("click", () => {
  const m = MELEE_MOVES[mel.id]; if (!m) return;
  const t = mel.t, end = meleeEnd(m);
  if (t <= 0.005 || t >= end - 0.005 || m.keys.some(k => Math.abs(k.t - t) < 0.01)) { stEl.textContent = "Scrub to a time between two keys first."; return; }
  const smp = meleeSample(m, t), k = { t: +t.toFixed(3), b: {} };
  for (const [n, v] of smp) { if (n === "crouch") k.crouch = +v[0].toFixed(3); else k.b[n] = v.map(x => +x.toFixed(3)); }
  m.keys.push(k); m.keys.sort((a, c) => a.t - c.t); mel.key = m.keys.indexOf(k); dirty(true); drawEditor();
});
document.getElementById("mel-del").addEventListener("click", () => {
  const m = MELEE_MOVES[mel.id]; if (!m || mel.key === 0 || mel.key === m.keys.length - 1) return;
  m.keys.splice(mel.key, 1); mel.key = Math.max(1, mel.key - 1); dirty(true); drawEditor();
});
document.getElementById("mel-reset").addEventListener("click", () => {
  if (!mel.id) return; MELEE_MOVES[mel.id] = JSON.parse(JSON.stringify(MELEE_DEFAULTS[mel.id])); mel.key = 1; dirty(true);
  stEl.textContent = "Back to the default. Save to keep it that way."; drawEditor();
});
document.getElementById("mel-copy").addEventListener("click", async () => {
  if (!mel.id) return; const txt = JSON.stringify({ [mel.id]: MELEE_MOVES[mel.id] });
  try { await navigator.clipboard.writeText(txt); stEl.textContent = "Copied."; } catch (e) { stEl.textContent = txt; }
});
// ----- saved moves: one document per move in the collection "melee", {move, updated_ms, by} -----
let melDb = null;
document.getElementById("mel-save").addEventListener("click", async () => {
  if (!mel.id) return;
  if (!melDb) { stEl.textContent = "Saving needs the page opened on claude.ai, signed in. Copy JSON works anywhere."; return; }
  try {
    const u = await claude.use("user"), by = u ? await u.id() : "";
    await melDb.collection("melee").doc(mel.id).set({ move: JSON.parse(JSON.stringify(MELEE_MOVES[mel.id])), updated_ms: Date.now(), by });
    isDirty = false; stEl.textContent = "Saved " + MELEE_MOVES[mel.id].name + ".";
  } catch (e) { stEl.textContent = "Could not save: " + (e && (e.code || e.message) || e); }
});
(async () => {
  try {
    if (!window.claude || !claude.use) return;
    melDb = await claude.use("db"); if (!melDb) return;
    melDb.collection("melee").onSnapshot(snap => {
      for (const d of snap.docs) { const v = d.data(); if (!v || !v.move || !MELEE_DEFAULTS[d.id]) continue;
        if (mel.edit && mel.id === d.id && isDirty) continue;          // never clobber the move being edited
        const mv = v.move; if (!Array.isArray(mv.keys) || mv.keys.length < 2) continue;
        MELEE_MOVES[d.id] = JSON.parse(JSON.stringify(mv)); }
      if (mel.edit) drawEditor();
    }, () => {});
  } catch (e) { melDb = null; }
})();
requestAnimationFrame(loop);
window.MOCKUP = {
  setMech, setView, setAct,
  // a fixed pose over idle on the current mech, {bone: [rx, ry, rz, px, py, pz]} (null clears): for checking axes
  hold(map) { cur().hold = map; },
  // the melee lab: play a move (or pin it at time t, paused), turn the partner on or off
  melee(id, t) { pickMove(id); if (t !== undefined) { mel.playing = false; mel.t = t; drawMelee(); } },
  spar(on) { mel.spar = on; layout(); },
  setT(v) { script = null; Tb = target = v; }, open() { document.getElementById("b-open").click(); }, board() { document.getElementById("b-board").click(); },
  end: () => cur().timeline.end,
  look(yaw, pitch, fov) { setView("cockpit"); const e = eyeOf(current), f = fwdOf(cur()), y0 = Math.atan2(f.x, f.z);
    const dir = new T.Vector3(Math.sin(y0 + yaw) * Math.cos(pitch), Math.sin(pitch), Math.cos(y0 + yaw) * Math.cos(pitch));
    camera.position.copy(e); controls.target.copy(e).add(dir); camera.fov = fov || 78; camera.updateProjectionMatrix(); controls.update(); },
  orbitTo(tx, ty, tz, cx, cy, cz) { setView("orbit"); controls.target.set(tx, ty, tz); camera.position.set(cx, cy, cz); controls.update(); },
  // what the camera sees at a point of the view (x, y from -1 to 1): the hit's node path and its height off the
  // cockpit floor, for checking from the pilot's seat that nothing stands in the doorway
  pick(x, y) { const rc = new T.Raycaster(); rc.setFromCamera(new T.Vector2(x, y), camera);
    const shown = o => { for (; o; o = o.parent) if (!o.visible) return false; return true; };
    const h = rc.intersectObject(scene, true).find(i => !i.object.isSprite && !i.object.isPoints && !i.object.isLine && shown(i.object)); if (!h) return null;
    const path = []; for (let o = h.object; o && o !== scene; o = o.parent) if (o.name) path.push(o.name);
    const c = cur().nodes.cockpit.worldToLocal(h.point.clone()); return { path: path.slice(0, 6), y: +c.y.toFixed(2), z: +c.z.toFixed(2), d: +h.distance.toFixed(2) }; },
  state: () => ({ Tb, end: cur() && cur().timeline.end, interior: !!(cur() && cur().interior), walk: walk && { i: walk.i, done: walk.done } }),
};

// ---------- the 24 m low-poly block-outs: their own viewers, side by side at true scale ----------
// the combat mechs the owner picked out (2026-10-24: "13,15,16,17, 19, and 21 all good ... Just note that I like them")
const LIKED = new Set(["sabre", "bulldog", "tarantula", "hatchet", "anvil", "brigand"]);
function blockoutViewer(BO, label, pickId, statsId, first, gap, api, frame) {
  frame = frame || { d: 1, ty: 12 };     // one mech's framing: distance scale and target height (the crabs are wide and low)
  // built on the shared renderer and canvas: its own scene, camera and controls, and nothing loaded until it is opened
  let sc = null, cam = null, oc = null, loading = null;
  const limbs = [], sparks = [], pose = {}; let last = performance.now(), walking = false;
  const groups = {}, ids = BO.mechs.map(m => m.id);
  const pick = document.getElementById(pickId), stats = document.getElementById(statsId);
  let cur = "all";
  function build() {
    sc = new T.Scene(); sc.background = new T.Color(0x2a1f30); sc.fog = new T.Fog(0x2a1f30, 90 * Math.max(1, gap / 24), 260 * Math.max(1, gap / 24));
    cam = new T.PerspectiveCamera(40, 2, 0.1, 800);
    oc = new T.OrbitControls(cam, canvas); oc.enableDamping = true; oc.enabled = false;
    sc.add(new T.HemisphereLight(0xb9a6d8, 0x2a2018, 0.7));
    const sunB = new T.DirectionalLight(0xffc89a, 2.2); sunB.position.set(-30, 40, 30); sunB.castShadow = true;
    sunB.shadow.mapSize.set(2048, 2048); Object.assign(sunB.shadow.camera, { left: -45, right: 45, top: 45, bottom: -45, near: 1, far: 150 }); sc.add(sunB);
    const gr = new T.Mesh(new T.PlaneGeometry(600, 600), new T.MeshStandardMaterial({ color: 0x1c1a1f, roughness: 0.95 }));
    gr.rotation.x = -Math.PI / 2; gr.receiveShadow = true; sc.add(gr);
    const gh = new T.GridHelper(200, 50, 0x4a4552, 0x3a3640); gh.position.y = 0.01; sc.add(gh);
    const railB = new T.Mesh(new T.BoxGeometry(110, 0.12, 0.12), new T.MeshBasicMaterial({ color: 0xf0b13c })); railB.position.set(0, 24, -16); sc.add(railB);
    const bld = new T.Mesh(new T.BoxGeometry(14, 14, 10), new T.MeshStandardMaterial({ color: 0x6a625a, roughness: 0.85 })); bld.position.set(-34, 7, -8); bld.castShadow = true; sc.add(bld);
    const carB = new T.Mesh(new T.BoxGeometry(1.8, 1.3, 4.4), new T.MeshStandardMaterial({ color: 0x7a1f1f })); carB.position.set(-14, 0.65, 10); sc.add(carB);
    const pil = new T.Group(); const sm = new T.MeshStandardMaterial({ color: 0x2e5ea8 });
    [-0.13, 0.13].forEach(x => { const l = new T.Mesh(new T.CylinderGeometry(0.1, 0.07, 0.86, 8), sm); l.position.set(x, 0.43, 0); pil.add(l); });
    const tb = new T.Mesh(new T.CylinderGeometry(0.24, 0.17, 0.62, 10), sm); tb.position.y = 1.18; pil.add(tb);
    const hb = new T.Mesh(new T.SphereGeometry(0.12, 12, 10), new T.MeshStandardMaterial({ color: 0xd8b49a })); hb.position.y = 1.66; pil.add(hb);
    pil.position.set(-8, 0, 9); sc.add(pil);
    document.getElementById("mode").textContent = "Loading " + label;
    loading = Promise.all(BO.mechs.map(m => loader.loadAsync(m.file).then(g => {
      g.scene.traverse(o => { if (o.isMesh) { o.castShadow = o.receiveShadow = true; } });
      groups[m.id] = g.scene; sc.add(g.scene);
      // the limbs a crab can lose: one node each, at its root joint (build_mech_blockouts.py `limb`)
      g.scene.traverse(o => {
        const mm = /^(ARM|LEG)_(\d?)([LR])/.exec(o.name);
        if (mm) limbs.push({ node: o, mech: m.id, leg: mm[1] === "LEG", pair: +(mm[2] || 0), side: mm[3] === "L" ? 1 : -1,
                             parent: o.parent, pos: o.position.clone(), rot: o.quaternion.clone(), v: null, w: null });
      });
      pose[m.id] = { roll: 0, pitch: 0, dip: 0, t: 0 };
    }))).then(() => { show(cur); if (sceneId === api) document.getElementById("mode").textContent = label; })
      .catch(e => { stats.textContent = "Could not load: " + e; });
    show(cur);
  }
  function show(id) {
    cur = id;
    pick.querySelectorAll("button").forEach(b => b.setAttribute("aria-pressed", String(b.dataset.bo === id)));
    if (!sc) return;
    ids.forEach((k, i) => { const g = groups[k]; if (!g) return; g.visible = id === "all" || id === k; const row = Math.floor(i / 5), col = i % 5; g.position.x = id === "all" ? (col - 2) * gap : 0; g.position.z = id === "all" ? -row * (gap + 6) : 0; });
    if (id === "all") { cam.position.set(40 * gap / 16, 34, 70 * gap / 16); oc.target.set(0, 10, -20); } else { cam.position.set(22 * frame.d, 18 * frame.d, 30 * frame.d); oc.target.set(0, frame.ty, 0); }
    oc.update();
  }
  // the new mechs carry the numbers on their contact sheets (owner, 2026-10-24: "can you add numbers to those mechs")
  const ORIGINAL = new Set(["foreman", "stevedore", "ironhorse"]);
  let num = first - 1;
  [...BO.mechs.map(m => [m.id, ORIGINAL.has(m.id) ? m.name : `${++num} ${m.name}${LIKED.has(m.id) ? " (liked)" : ""}`]), ["all", "Line up"]].forEach(([id, name]) => {
    const b = document.createElement("button"); b.textContent = name; b.dataset.bo = id; b.onclick = () => show(id); pick.appendChild(b);
  });
  // shoot off a leg or an arm (owner, 2026-10-25: "Can we shoot off each leg or arm"): only where the models have limbs
  if (BO.mechs.some(m => m.limbs)) {
    const walk = document.createElement("button"); walk.textContent = "Walk"; walk.setAttribute("aria-pressed", "false");
    walk.onclick = () => { walking = !walking; walk.setAttribute("aria-pressed", String(walking)); }; pick.appendChild(walk);
    const shoot = document.createElement("button"); shoot.textContent = "Shoot off a limb"; shoot.onclick = () => shootLimb(); pick.appendChild(shoot);
    // tap a leg or a claw to shoot that one off (a tap, not an orbit drag)
    let down = null;
    canvas.addEventListener("pointerdown", e => { if (sceneId === api) down = [e.clientX, e.clientY]; });
    canvas.addEventListener("pointerup", e => {
      if (sceneId !== api || !down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 6) return;
      const r = canvas.getBoundingClientRect(), ray = new T.Raycaster();
      ray.setFromCamera(new T.Vector2((e.clientX - r.left) / r.width * 2 - 1, -(e.clientY - r.top) / r.height * 2 + 1), cam);
      for (const hit of ray.intersectObjects(sc.children, true)) {
        let o = hit.object, l = null;
        while (o && !(l = limbs.find(x => x.node === o && !x.v))) o = o.parent;
        if (l) { shootLimb(l); break; }
        if (hit.object.isMesh && hit.object.parent && Object.values(groups).some(g => isUnder(hit.object, g))) break;
      }
    });
    const fix = document.createElement("button"); fix.textContent = "Repair"; fix.onclick = repair; pick.appendChild(fix);
  }
  function isUnder(o, g) { while (o) { if (o === g) return true; o = o.parent; } return false; }
  function shootLimb(l) {
    if (!l) {
      const on = limbs.filter(x => !x.v && (cur === "all" || x.mech === cur) && groups[x.mech].visible);
      if (!on.length) return;
      l = on[Math.floor(Math.random() * on.length)];
    }
    const g = groups[l.mech];
    const root = new T.Vector3(); l.node.getWorldPosition(root);
    const c = new T.Vector3(); g.getWorldPosition(c);
    const out = root.clone().sub(c).setY(0).normalize();
    sc.attach(l.node);
    l.v = out.multiplyScalar(7).add(new T.Vector3(0, 9, 0));
    l.w = new T.Vector3((Math.random() - 0.5) * 4, (Math.random() - 0.5) * 3, (Math.random() - 0.5) * 4);
    for (let i = 0; i < 40; i++) {
      const sp = new T.Mesh(new T.BoxGeometry(0.25, 0.25, 0.25), new T.MeshBasicMaterial({ color: i % 3 ? 0xffa030 : 0xfff0c0 }));
      sp.position.copy(root);
      sp.userData = { v: new T.Vector3(Math.random() - 0.5, Math.random() * 0.8 + 0.2, Math.random() - 0.5).multiplyScalar(18), t: 0.6 + Math.random() * 0.6 };
      sc.add(sp); sparks.push(sp);
    }
  }
  function repair() {
    limbs.forEach(l => { if (!l.v) return; l.parent.add(l.node); l.node.position.copy(l.pos); l.node.quaternion.copy(l.rot); l.v = l.w = null; });
  }
  // What the crab does with what it has left (openspec crab-enemies, "losing limbs"): the body drops and rolls toward a
  // side short of legs, a lost claw leans it onto the other, and it walks slower by the share of hind legs left; with no
  // hind leg on a side it cannot walk and sits down on its belly and claw.
  function stance(id) {
    const mine = limbs.filter(x => x.mech === id), legs = mine.filter(x => x.leg), pairs = Math.max(1, legs.length / 2);
    const lost = s => legs.filter(x => x.side === s && x.v).length, left = s => legs.filter(x => x.side === s && !x.v).length;
    const armLost = s => mine.some(x => !x.leg && x.side === s && x.v);
    const stuck = left(1) === 0 || left(-1) === 0;
    let roll = -(lost(1) - lost(-1)) / pairs * 0.22 - (armLost(1) ? 0.07 : 0) + (armLost(-1) ? 0.07 : 0);
    if (stuck) roll = left(1) === 0 && left(-1) === 0 ? 0 : (left(1) === 0 ? -0.32 : 0.32);
    const lostLegs = lost(1) + lost(-1), frontLost = legs.some(x => x.pair === 0 && x.v);
    return { roll, pitch: (frontLost ? 0.06 : 0) + (armLost(1) || armLost(-1) ? 0.05 : 0),
             dip: stuck ? 2.4 : lostLegs / (2 * pairs) * 1.6, speed: stuck ? 0 : (legs.length - lostLegs) / Math.max(1, legs.length) };
  }
  const Z = new T.Vector3(0, 0, 1), Y = new T.Vector3(0, 1, 0), X = new T.Vector3(1, 0, 0), q = new T.Quaternion();
  function stepWalk(dt) {
    let note = "";
    for (const id in pose) {
      const g = groups[id], p = pose[id]; if (!g || !g.visible) continue;
      const st = stance(id), k = Math.min(1, dt * 3);
      p.roll += (st.roll - p.roll) * k; p.pitch += (st.pitch - p.pitch) * k; p.dip += (st.dip - p.dip) * k;
      if (walking) p.t += dt * 1.6 * st.speed;
      const legs = limbs.filter(x => x.mech === id && x.leg).length, pairs = Math.max(1, legs / 2);
      g.rotation.set(p.pitch, 0, p.roll);
      g.position.y = -p.dip + (walking && st.speed ? Math.abs(Math.sin(p.t * Math.PI)) * 0.25 : 0);
      for (const l of limbs) {
        if (l.mech !== id || l.v) continue;
        l.node.quaternion.copy(l.rot);
        if (!walking || !st.speed) continue;
        // alternating groups: one pair's left with the next pair's right; the claws walk as the front pair
        const ph = p.t * Math.PI + ((l.leg ? l.pair + 1 : 0) + (l.side > 0 ? 0 : 1)) * Math.PI;
        const lift = Math.max(0, Math.sin(ph)), swing = Math.cos(ph);
        if (l.leg) {
          l.node.quaternion.multiply(q.setFromAxisAngle(Z, l.side * lift * 0.22));
          l.node.quaternion.multiply(q.setFromAxisAngle(Y, -l.side * swing * 0.16 / Math.sqrt(pairs)));
        } else {
          l.node.quaternion.multiply(q.setFromAxisAngle(X, -lift * 0.18));
        }
      }
      if (id === cur || cur === "all") note = st.speed === 0 ? "cannot walk: a side has no legs" : walking ? `walking at ${Math.round(st.speed * 100)}%` : "";
    }
    if (sceneId === api) document.getElementById("mode").textContent = label + (note ? ": " + note : "");
  }
  function stepDebris() {
    const now = performance.now(), dt = Math.min(0.05, (now - last) / 1000); last = now;
    stepWalk(dt);
    const box = new T.Box3();
    limbs.forEach(l => {
      if (!l.v || l.v.lengthSq() + l.w.lengthSq() < 1e-4) return;
      l.v.y -= 9.8 * dt;
      l.node.position.addScaledVector(l.v, dt);
      l.node.rotateX(l.w.x * dt); l.node.rotateY(l.w.y * dt); l.node.rotateZ(l.w.z * dt);
      box.setFromObject(l.node);
      if (box.min.y < 0) {                       // on the ground: stop sinking, bounce a little, settle
        l.node.position.y -= box.min.y;
        l.v.y = Math.abs(l.v.y) * 0.25; l.v.x *= 0.5; l.v.z *= 0.5; l.w.multiplyScalar(0.5);
        if (l.v.lengthSq() < 1) { l.v.set(0, 0, 0); l.w.set(0, 0, 0); }
      }
    });
    for (let i = sparks.length - 1; i >= 0; i--) {
      const sp = sparks[i], d = sp.userData;
      d.v.y -= 9.8 * dt; sp.position.addScaledVector(d.v, dt); d.t -= dt;
      if (d.t <= 0 || sp.position.y < 0) { sc.remove(sp); sp.geometry.dispose(); sparks.splice(i, 1); }
    }
  }
  stats.innerHTML = "<table><tr><th>Mech</th><th>After</th><th>Height</th><th>Width</th><th>Legs : torso</th><th>Triangles</th></tr>" +
    BO.mechs.map(m => `<tr><td>${m.name}</td><td>${m.what}</td><td>${m.height_m.toFixed(1)} m</td><td>${m.width_m.toFixed(1)} m</td><td>${m.legs_over_torso.toFixed(2)} by volume</td><td>${m.tris.toLocaleString()}</td></tr>`).join("") + "</table>";
  const v = {
    label,
    activate(on) {
      pick.hidden = stats.hidden = !on;
      if (on && !sc) build();
      if (oc) oc.enabled = on;
    },
    render() {
      const w = canvas.clientWidth, h = canvas.clientHeight;
      if (Math.abs(cam.aspect - w / h) > 1e-3) { cam.aspect = w / h; cam.updateProjectionMatrix(); }
      stepDebris();
      oc.update(); renderer.toneMappingExposure = 1.0; renderer.render(sc, cam);
    },
    show, ready: () => loading, shootLimb, repair,
  };
  VIEWERS[api] = v;
  window[api] = { show, shootLimb, repair, open: () => { setScene(api); return loading; } };
}
blockoutViewer(__MECH_BLOCKOUTS__, "Block-outs", "bo-pick", "bo-stats", 1, 16, "BLOCKOUTS");
// the combat mechs, numbered on from the working ones (owner, 2026-10-24: "I want combat mechs with guns like in armored core")
blockoutViewer(__MECH_COMBAT__, "Combat mechs", "cb-pick", "cb-stats", 13, 24, "COMBAT");
// the crab mechs, numbered on from the combat mechs (owner, 2026-10-24: "more practical big legged crabs with guns for arms")
blockoutViewer(__MECH_CRABS__, "Crab mechs", "cr-pick", "cr-stats", 23, 44, "CRABS", { d: 1.45, ty: 7 });
{ const start = { crabs: "CRABS", combat: "COMBAT", blockouts: "BLOCKOUTS" }[location.hash.slice(1)];
  if (start) setScene(start); else loadDetailed(); }
</script>
"""


def interiors(meta):
    """Each chassis's cockpit interior: the glTF published beside the page and its manifest."""
    out = {}
    for key in ORDER:
        v = meta[key].get("cockpit_variant")
        man = os.path.join(MODELS, f"cockpit_{v}.json")
        if v and os.path.exists(man) and os.path.exists(os.path.join(MODELS, f"cockpit_{v}", f"cockpit_{v}.gltf")):
            out[v] = dict(file=f"cockpit_{v}/cockpit_{v}.gltf.json", man=json.load(open(man)))
    return out


def as_json_gltf(src, dst):
    """A glb or a .gltf (with its .bin) as one glTF JSON file with the buffer embedded as a data URI. The
    artifact host serves .json but not .glb, .gltf or .bin; GLTFLoader reads glTF JSON whatever the name.
    Images stay where they were: inside the buffer for a glb, beside the file for a .gltf."""
    if src.endswith(".glb"):
        data = open(src, "rb").read()
        jlen = struct.unpack_from("<I", data, 12)[0]
        js = json.loads(data[20:20 + jlen])
        rest = data[20 + jlen:]
        blen = struct.unpack_from("<I", rest, 0)[0]
        js["buffers"][0]["uri"] = "data:application/octet-stream;base64," + base64.b64encode(rest[8:8 + blen]).decode("ascii")
    else:
        js = json.load(open(src))
        for b in js["buffers"]:
            raw = open(os.path.join(os.path.dirname(src), b["uri"]), "rb").read()
            b["uri"] = "data:application/octet-stream;base64," + base64.b64encode(raw).decode("ascii")
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    with open(dst, "w") as f:
        json.dump(js, f, separators=(",", ":"))


def blockouts_for_page(manifest="mech_blockouts.json"):
    """A block-out manifest (the working mechs, or mech_combat.json) with each model's path as published beside the page."""
    bo = json.load(open(os.path.join(MODELS, manifest)))
    for m in bo["mechs"]:
        m["file"] = f"mech_blockouts/{m['id']}.gltf.json"
    return bo


def files(meta, inside):
    """Write the page's own files into SITE and return them: published path (relative to the page) to the
    file on disk. Models become glTF JSON (as_json_gltf); textures are copied as they are."""
    out = {}
    for k in ORDER:
        rel = f"mech_{k}.gltf.json"
        as_json_gltf(os.path.join(MODELS, meta[k]["file"]), os.path.join(SITE, rel))
        out[rel] = os.path.join(SITE, rel)
    for m in json.load(open(os.path.join(MODELS, "mech_blockouts.json")))["mechs"] + json.load(open(os.path.join(MODELS, "mech_combat.json")))["mechs"] + \
            json.load(open(os.path.join(MODELS, "mech_crabs.json")))["mechs"]:
        rel = f"mech_blockouts/{m['id']}.gltf.json"
        as_json_gltf(os.path.join(MODELS, m["file"]), os.path.join(SITE, rel))
        out[rel] = os.path.join(SITE, rel)
    for v in inside:
        d = os.path.join(MODELS, f"cockpit_{v}")
        rel = f"cockpit_{v}/cockpit_{v}.gltf.json"
        as_json_gltf(os.path.join(d, f"cockpit_{v}.gltf"), os.path.join(SITE, rel))
        out[rel] = os.path.join(SITE, rel)
        for f in sorted(os.listdir(os.path.join(d, "textures"))):
            r = f"cockpit_{v}/textures/{f}"
            os.makedirs(os.path.dirname(os.path.join(SITE, r)), exist_ok=True)
            shutil.copyfile(os.path.join(d, "textures", f), os.path.join(SITE, r))
            out[r] = os.path.join(SITE, r)
    return out


def main():
    meta = json.load(open(os.path.join(MODELS, "mechs.json")))
    inside = interiors(meta)
    for key in ORDER:
        v = meta[key].get("cockpit_variant")
        if v in inside:
            meta[key]["interior_tris"] = inside[v]["man"].get("tris")
    pose = open(os.path.join(ROOT, "tools", "mockups", "mech_pose.js")).read()
    melee = open(os.path.join(ROOT, "tools", "mockups", "mech_melee.js")).read()
    steam = open(os.path.join(ROOT, "tools", "mockups", "steam.js")).read()
    wall = open(os.path.join(ROOT, "tools", "mockups", "screen_wall.js")).read()
    gltf_json = open(os.path.join(ROOT, "tools", "mockups", "gltf_json_loader.js")).read()
    hud = base64.b64encode(open(HUD, "rb").read()).decode("ascii")
    html = (PAGE.replace("__MECH_POSE__", pose).replace("__MECH_MELEE__", melee).replace("__STEAM__", steam).replace("__SCREEN_WALL__", wall).replace("__GLTF_JSON__", gltf_json)
            .replace("__MECH_BLOCKOUTS__", json.dumps(blockouts_for_page()))
            .replace("__MECH_BUTTONS__", "\n".join(f'      <button id="m-{k}" data-mech="{k}">{(str(NUMBERS[k]) + " ") if k in NUMBERS else ""}{meta[k]["name"]}</button>' for k in ORDER))
            .replace("__MECH_COMBAT__", json.dumps(blockouts_for_page("mech_combat.json")))
            .replace("__MECH_CRABS__", json.dumps(blockouts_for_page("mech_crabs.json")))
            .replace("__META__", json.dumps({k: meta[k] for k in ORDER}))
            .replace("__INTERIOR__", json.dumps(inside)).replace("__ORDER__", json.dumps(ORDER)).replace("__HUD__", hud))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write(html)
    shutil.rmtree(SITE, ignore_errors=True)
    os.makedirs(SITE)
    os.symlink(OUT, os.path.join(SITE, "index.html"))
    fs = files(meta, inside)
    with open(os.path.join(SITE, "files.json"), "w") as f:      # the publish map, paths from the repository root
        json.dump({k: os.path.relpath(v, ROOT) for k, v in fs.items()}, f, indent=1)
    print(f"wrote {OUT} ({len(html) / 1e6:.1f} MB), {len(fs)} files beside it "
          f"({sum(os.path.getsize(p) for p in fs.values()) / 1e6:.1f} MB), interiors: {sorted(inside) or 'none yet'}")


if __name__ == "__main__":
    main()
