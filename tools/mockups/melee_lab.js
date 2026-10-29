// Crab Breeze: the melee lab's playback and keyframe editor, inlined by tools/mockups/gen_melee_lab.py (the page
// supplies cur(), layout(), the partner `foe` and the mechs' pose). Moves are tools/mockups/mech_melee.js.
// One move plays at a time on the selected mech (mel), on its own clock, layered over the gait (so Walk and Fire R is
// the gun kicking as the mech walks); a gun's recoil is a move like the rest, fired again each loop with a muzzle flash. The sparring partner is a copy of the mech, facing it SPAR_M ahead,
// struck (hitGeneric) when a move's hit_s passes. Edits change MELEE_MOVES in place and are saved to the db.
const SPAR_M = 15;
const mel = { id: null, t: 0, playing: false, loop: false, spar: true, edit: false, speed: 1, key: 0, bone: "shoulder_R", wait: 0 };
function sparring() { return mel.spar ? foe : null; }
function strike(move) {
  const sp = sparring(); if (!sp) return;
  // a blow staggers the partner until its stance breaks; then it falls (mech_stance.js)
  const side = move.side || (Math.random() < 0.5 ? 1 : -1);
  const r = stanceHit(sp.stance, move.stance || 1, stanceCfg, side);
  if (r === "stagger") { sp.hitT = 0; sp.hitSide = side; }
  stanceFlash(r);
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
