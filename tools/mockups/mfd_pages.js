// The live MFD pages for the cockpit page (inlined by tools/mockups/gen_cockpit_page.py): status (the mech's
// health by part), nav (heading, the legs' direction against the torso's, speed), weapons (ammunition, heat, lock)
// and radar (enemies). Drawn on canvases, kept live by tick(dt), shown on every monitor whose material is
// SCREEN_mfd_<page> (the block-outs) or another SCREEN_ (the built interiors). The owner, 2026-10-24: "no displays
// for me to view mech health, walking direciton, amunition, enemies", "you had MFDs and a bunch of hud elements
// before".
//
//   const MFD = makeMfdPages(THREE);   mesh.material = new THREE.MeshBasicMaterial({ map: MFD.page("radar") });
//   MFD.tick(dt) every frame

function makeMfdPages(T) {
  const D = Math.PI / 180;
  const MFD_PX = 256, state = { t: 0, yaw: 0, legs: 0.3, speed: 34, hull: { head: 0.9, torso: 0.72, armL: 1, armR: 0.45, legL: 0.85, legR: 0.95 },
    ammo: [["L CANNON", 212, 300], ["R BREAKER", 0, 0], ["MISSILES", 12, 16]], heat: 0.46, enemies: [[0.45, 0.6], [-0.3, 0.75], [0.7, -0.2]] };
  const pages = {};
  function page(kind) {
    if (pages[kind]) return pages[kind].tex;
    const c = document.createElement("canvas"); c.width = c.height = MFD_PX;
    const tex = new T.CanvasTexture(c); tex.encoding = T.sRGBEncoding;
    pages[kind] = { c, q: c.getContext("2d"), tex };
    draw(kind);
    return tex;
  }
  const G = "#7dffa0", A = "#ffb347", R = "#ff5a4a", DIM = "#2f6b44";
  function frameBox(q, title) {
    q.fillStyle = "#04120a"; q.fillRect(0, 0, MFD_PX, MFD_PX);
    q.strokeStyle = DIM; q.lineWidth = 2; q.strokeRect(6, 6, MFD_PX - 12, MFD_PX - 12);
    q.fillStyle = G; q.font = "bold 15px monospace"; q.fillText(title, 14, 25);
    q.fillStyle = DIM; for (let i = 0; i < 5; i++) q.fillRect(14 + i * 48, MFD_PX - 16, 36, 6);
  }
  const hcol = v => v > 0.75 ? G : v > 0.4 ? A : R;
  function draw(kind) {
    const { q } = pages[kind], s = state;
    if (kind === "status") {
      frameBox(q, "STATUS  HULL");
      const H = s.hull, part = (x, y, w, h, v) => { q.fillStyle = hcol(v); q.globalAlpha = 0.85; q.fillRect(x, y, w, h); q.globalAlpha = 1; q.strokeStyle = "#04120a"; q.strokeRect(x, y, w, h); };
      part(108, 42, 40, 26, H.head); part(92, 72, 72, 66, H.torso); part(56, 76, 32, 70, H.armL); part(168, 76, 32, 70, H.armR);
      part(94, 142, 30, 76, H.legL); part(132, 142, 30, 76, H.legR);
      const tot = Math.round(100 * Object.values(H).reduce((a, b) => a + b, 0) / 6);
      q.fillStyle = hcol(tot / 100); q.font = "bold 30px monospace"; q.fillText(tot + "%", 172, 216);
      q.fillStyle = G; q.font = "12px monospace"; q.fillText("R ARM 45%", 14, 216); q.fillText("REACTOR OK", 14, 232);
    } else if (kind === "nav") {
      frameBox(q, "NAV  HDG " + String(Math.round(((s.yaw / D) % 360 + 360) % 360)).padStart(3, "0"));
      const cx = 128, cy = 132, r = 82;
      q.strokeStyle = G; q.lineWidth = 2; q.beginPath(); q.arc(cx, cy, r, 0, 2 * Math.PI); q.stroke();
      q.font = "bold 14px monospace"; q.fillStyle = G;
      ["N", "E", "S", "W"].forEach((l, i) => { const a = i * Math.PI / 2 - s.yaw; q.fillText(l, cx + Math.sin(a) * (r - 14) - 5, cy - Math.cos(a) * (r - 14) + 5); });
      for (let i = 0; i < 36; i++) { const a = i * 10 * D - s.yaw, l = i % 3 ? 5 : 10; q.beginPath(); q.moveTo(cx + Math.sin(a) * r, cy - Math.cos(a) * r); q.lineTo(cx + Math.sin(a) * (r - l), cy - Math.cos(a) * (r - l)); q.stroke(); }
      const arrow = (a, col, len, w) => { q.save(); q.translate(cx, cy); q.rotate(a); q.fillStyle = col; q.beginPath(); q.moveTo(0, -len); q.lineTo(w, -len + 18); q.lineTo(-w, -len + 18); q.fill(); q.fillRect(-2, -len + 16, 4, len - 16); q.restore(); };
      arrow(s.legs, A, 60, 10); arrow(0, G, 70, 8);
      q.fillStyle = A; q.font = "12px monospace"; q.fillText("LEGS", 14, 216); q.fillStyle = G; q.fillText("TORSO", 14, 232);
      q.font = "bold 20px monospace"; q.fillText(s.speed + " KM/H", 140, 216); q.font = "12px monospace"; q.fillText("WALK", 196, 232);
    } else if (kind === "weapons") {
      frameBox(q, "WEAPONS");
      s.ammo.forEach(([n, a, m], i) => {
        const y = 52 + i * 52; q.fillStyle = m ? G : A; q.font = "bold 15px monospace"; q.fillText(n, 14, y);
        q.font = "bold 18px monospace"; q.fillText(m ? a + "/" + m : "MELEE", 160, y);
        if (m) { q.strokeStyle = DIM; q.strokeRect(14, y + 8, 228, 10); q.fillStyle = a / m > 0.3 ? G : R; q.fillRect(15, y + 9, 226 * a / m, 8); }
      });
      q.fillStyle = G; q.font = "12px monospace"; q.fillText("HEAT", 14, 212);
      q.strokeStyle = DIM; q.strokeRect(60, 202, 182, 12); q.fillStyle = s.heat > 0.7 ? R : s.heat > 0.5 ? A : G; q.fillRect(61, 203, 180 * s.heat, 10);
      q.fillStyle = (s.t % 1) < 0.6 ? R : "#04120a"; q.font = "bold 13px monospace"; q.fillText("LOCK", 196, 232);
    } else if (kind === "radar") {
      frameBox(q, "RADAR  2 KM");
      const cx = 128, cy = 138, r = 92;
      q.strokeStyle = DIM; q.lineWidth = 1.5; [1, 0.66, 0.33].forEach(f => { q.beginPath(); q.arc(cx, cy, r * f, 0, 2 * Math.PI); q.stroke(); });
      q.beginPath(); q.moveTo(cx - r, cy); q.lineTo(cx + r, cy); q.moveTo(cx, cy - r); q.lineTo(cx, cy + r); q.stroke();
      const sw = s.t * 2.2; q.fillStyle = "rgba(125,255,160,0.18)"; q.beginPath(); q.moveTo(cx, cy); q.arc(cx, cy, r, sw - 0.6 - Math.PI / 2, sw - Math.PI / 2); q.fill();
      s.enemies.forEach(([x, z]) => { q.fillStyle = R; q.fillRect(cx + x * r - 5, cy - z * r - 5, 10, 10); });
      q.fillStyle = G; q.beginPath(); q.moveTo(cx, cy - 9); q.lineTo(cx + 6, cy + 6); q.lineTo(cx - 6, cy + 6); q.fill();
      q.fillStyle = R; q.font = "bold 13px monospace"; q.fillText(s.enemies.length + " HOSTILE", 14, 232);
    }
    pages[kind].tex.needsUpdate = true;
  }
  let acc = 0;
  function tick(dt) {
    state.t += dt; acc += dt;
    state.yaw += dt * 0.15; state.legs = 0.35 * Math.sin(state.t * 0.4); state.speed = Math.round(34 + 6 * Math.sin(state.t * 0.3));
    state.heat = 0.46 + 0.2 * Math.sin(state.t * 0.5);
    state.enemies = state.enemies.map(([x, z], i) => [x + Math.sin(state.t * 0.3 + i) * 0.002, z + Math.cos(state.t * 0.25 + i) * 0.002]);
    if (acc > 0.1) { acc = 0; for (const k in pages) draw(k); }
  }

  return { page, tick, kinds: ["status", "nav", "weapons", "radar"] };
}
