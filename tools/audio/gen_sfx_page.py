#!/usr/bin/env python3
"""Writes the Mech Sounds page (docs/mockups/sounds.html) from output/sfx/sfx.json.

Every sound gen_sfx.py makes, grouped as the game uses them, with its description, length, a
waveform strip from its peaks and a play button (loops play on repeat). The page is published with
output/sfx/*.mp3 beside it as files. Same look as the Crab Breeze Music page.

Usage: python3 tools/audio/gen_sfx_page.py   (after tools/audio/render_sfx.sh)
"""
import json
import os

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

HEAD = """<title>Crab Breeze Sounds</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Big+Shoulders+Display:wght@600;800&family=Barlow:wght@400;500;600&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* Layout: the works sheet for the game's sounds. A masthead plate, then one run sheet per group, each row a cue with its waveform strip. */
:root {
  --paper: #edf0ee; --plate: #f8faf9; --ink: #1a2026; --steel: #56626c; --rule: #c6cdd0;
  --amber: #b97700; --amber-ink: #1a2026; --wave: #2f6f86;
  --f-display: "Big Shoulders Display", "Arial Narrow", Impact, sans-serif;
  --f-body: "Barlow", "Segoe UI", Roboto, sans-serif;
  --f-mono: "IBM Plex Mono", ui-monospace, Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) {
  --paper: #111518; --plate: #181d21; --ink: #e2e6e7; --steel: #929ea7; --rule: #2b3339;
  --amber: #f0a62a; --amber-ink: #111518; --wave: #5fb0cc; color-scheme: dark } }
:root[data-theme="dark"] {
  --paper: #111518; --plate: #181d21; --ink: #e2e6e7; --steel: #929ea7; --rule: #2b3339;
  --amber: #f0a62a; --amber-ink: #111518; --wave: #5fb0cc; color-scheme: dark }
* { box-sizing: border-box }
body { background: var(--paper); color: var(--ink); font: 400 16px/1.55 var(--f-body);
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); padding-block: 28px 64px }
.wrap { max-width: 980px; margin: 0 auto; display: grid; gap: 44px }
h1, h2 { font-family: var(--f-display); font-weight: 800; line-height: 1; margin: 0; text-wrap: balance; text-transform: uppercase; letter-spacing: .01em }
h1 { font-size: clamp(56px, 11vw, 112px) }
h2 { font-size: clamp(30px, 5vw, 42px) }
p { margin: 0; max-width: 66ch }
.eyebrow { font: 500 12px/1.2 var(--f-mono); letter-spacing: .12em; text-transform: uppercase; color: var(--steel) }
.mast { display: grid; gap: 18px; border-top: 6px solid var(--ink); padding-top: 18px }
.mast .stripe { height: 10px; background: repeating-linear-gradient(-45deg, var(--amber) 0 12px, transparent 12px 24px); max-width: 280px }
.facts { display: flex; flex-wrap: wrap; gap: 10px 28px; font: 500 13px/1.3 var(--f-mono); color: var(--steel) }
.facts b { color: var(--ink); font-weight: 500 }
.status { border-left: 3px solid var(--amber); padding: 2px 0 2px 14px; color: var(--steel) }
section { display: grid; gap: 14px }
.head { display: flex; flex-wrap: wrap; align-items: baseline; justify-content: space-between; gap: 8px 20px; border-bottom: 2px solid var(--ink); padding-bottom: 8px }
.rows { display: grid; border-top: 1px solid var(--rule) }
.row { display: grid; grid-template-columns: auto minmax(0, 1fr) 200px; gap: 6px 16px; align-items: center; padding: 12px 0; border-bottom: 1px solid var(--rule) }
.row .txt { display: grid; gap: 2px; min-width: 0 }
.row .name { font: 500 14px/1.2 var(--f-mono) }
.row .name em { font-style: normal; color: var(--amber); margin-left: 8px; font-size: 11px; letter-spacing: .1em; text-transform: uppercase }
.row .what { color: var(--steel); font-size: 14.5px }
.row canvas { width: 200px; height: 40px; display: block; cursor: pointer }
button.play { appearance: none; border: 1.5px solid var(--ink); background: transparent; color: var(--ink); font: 600 13px/1 var(--f-mono);
  letter-spacing: .08em; text-transform: uppercase; padding: 9px 10px; min-width: 76px; border-radius: 2px; cursor: pointer }
button.play[aria-pressed="true"] { background: var(--amber); border-color: var(--amber); color: var(--amber-ink) }
button.play:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px }
footer { color: var(--steel); font-size: 14px; display: grid; gap: 6px; border-top: 1px solid var(--rule); padding-top: 14px }
@media (max-width: 620px) { .row { grid-template-columns: auto minmax(0, 1fr) } .row canvas { grid-column: 1 / 3; width: 100% } }
</style>
"""

BODY = """<div class="wrap">
  <header class="mast">
    <div class="stripe" aria-hidden="true"></div>
    <span class="eyebrow">Crab Breeze sound effects · works sheet 1 · 2026-10-25</span>
    <h1>Crab Breeze Sounds</h1>
    <p>The first __COUNT__ sounds for the mech, its weapons, what they hit and the cockpit, voiced electronic and futuristic. Every cue the sound and weapon plans name has one: footfalls, servos, the hatch and its steam, jets, every gun from the autocannon to the plasma cannon, lock tones, explosions and alarms.</p>
    <div class="facts"><span>Cues <b>__COUNT__</b></span><span>Made by <b>tools/audio/gen_sfx.py</b></span><span>Format <b>mono 44.1 kHz OGG</b></span><span>Licence <b>ours, no strings</b></span></div>
    <p class="status">Synthesised placeholders: built from oscillators, noise and filters because this session cannot reach the free sound libraries. Any recorded CC0 sound that beats one of these replaces it under the same cue name. Not in the game yet: playing them is the sound plan (openspec/changes/audio).</p>
  </header>
  <div id="groups" style="display:grid;gap:44px"></div>
  <footer>
    <span>Generator: tools/audio/gen_sfx.py · Encode: tools/audio/render_sfx.sh · Game copies: assets/audio/sfx/*.ogg · Plans: openspec/changes/audio, weapon-kinds, multi-lock</span>
    <span>Sources for recorded replacements and their licences: docs/analysis/audio-sources.md</span>
  </footer>
</div>
"""

SCRIPT = """<script>
const SOUNDS = __DATA__;
const GROUPS = { Mech: 'The mech', Weapons: 'Weapons', Impacts: 'Impacts', Cockpit: 'Cockpit' };
const fmt = (s) => s.toFixed(1) + ' s';
let current = null;
const stop = () => { if (current) { current.audio.pause(); current.audio.currentTime = 0; current.btn.setAttribute('aria-pressed', 'false'); current.btn.textContent = 'Play'; current = null; } };
const canvases = [];
const root = document.getElementById('groups');
for (const [g, title] of Object.entries(GROUPS)) {
  const list = SOUNDS.filter((s) => s.group === g);
  if (!list.length) continue;
  const sec = document.createElement('section');
  sec.innerHTML = `<div class="head"><h2>${title}</h2><span class="eyebrow">${list.length} cues</span></div><div class="rows"></div>`;
  root.appendChild(sec);
  const rows = sec.querySelector('.rows');
  for (const s of list) {
    const row = document.createElement('div');
    row.className = 'row';
    row.innerHTML = `<button class="play" type="button" id="play-${s.name}" aria-pressed="false" aria-label="Play ${s.name}">Play</button>
      <div class="txt"><span class="name">${s.name}${s.loop ? '<em>loop</em>' : ''} · ${fmt(s.seconds)}</span><span class="what">${s.what}</span></div>
      <canvas aria-hidden="true"></canvas>`;
    rows.appendChild(row);
    const audio = new Audio();
    audio.preload = 'none';
    audio.src = s.name + '.mp3';
    audio.loop = s.loop;
    const btn = row.querySelector('button');
    const me = { audio, btn };
    const play = () => { if (current === me) { stop(); return; } stop(); current = me; audio.currentTime = 0; audio.play().catch(() => {}); btn.setAttribute('aria-pressed', 'true'); btn.textContent = s.loop ? 'Stop' : 'Playing'; };
    btn.addEventListener('click', play);
    row.querySelector('canvas').addEventListener('click', play);
    audio.addEventListener('ended', () => { if (current === me) stop(); });
    canvases.push({ s, cv: row.querySelector('canvas') });
  }
}
function draw() {
  const cs = getComputedStyle(document.documentElement);
  const wave = cs.getPropertyValue('--wave').trim(), rule = cs.getPropertyValue('--rule').trim();
  for (const { s, cv } of canvases) {
    const w = cv.clientWidth, h = cv.clientHeight, dpr = window.devicePixelRatio || 1;
    cv.width = Math.round(w * dpr); cv.height = Math.round(h * dpr);
    const g = cv.getContext('2d'); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, w, h);
    g.fillStyle = rule; g.fillRect(0, h / 2, w, 1);
    g.fillStyle = wave;
    const bw = w / s.peaks.length;
    s.peaks.forEach((p, i) => { const ph = Math.max(1, p * (h - 4)); g.fillRect(i * bw, (h - ph) / 2, Math.max(1, bw - 0.6), ph); });
  }
}
draw();
addEventListener('resize', draw);
try { matchMedia('(prefers-color-scheme: dark)').addEventListener('change', draw); } catch (e) {}
new MutationObserver(draw).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
</script>
"""


def main() -> None:
    data = json.load(open(os.path.join(REPO, "output", "sfx", "sfx.json")))
    page = HEAD + BODY.replace("__COUNT__", str(len(data))) + SCRIPT.replace("__DATA__", json.dumps(data))
    out = os.path.join(REPO, "docs", "mockups", "sounds.html")
    with open(out, "w") as f:
        f.write(page)
    print(f"wrote {out}: {len(data)} sounds")


if __name__ == "__main__":
    main()
