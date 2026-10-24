// Runs in a browser page for tools/textures/gen_mech_plating.mjs. Provenance is in that file's header:
// mkCanvas, toTex, plating and textures are Mining Mike's (godot-sandbox tank_bosses_kit.js at 3336b88).
const TAU = Math.PI * 2;
function rng(seed) { let a = seed | 0; return () => { a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
let TEX = null;
  function mkCanvas(S) { const c = document.createElement("canvas"); c.width = c.height = S; return c; }
  function toTex(c, srgb) { return c; }
  function plating(seed, o) {
    const S = 512, col = mkCanvas(S), bmp = mkCanvas(S);
    const g = col.getContext("2d"), b = bmp.getContext("2d");
    const r = rng(seed);
    g.fillStyle = o.base; g.fillRect(0, 0, S, S);
    b.fillStyle = "#808080"; b.fillRect(0, 0, S, S);
    const blot = (ctx, x, y, rad, rgba) => {
      for (const ox of [-S, 0, S]) for (const oy of [-S, 0, S]) {
        const X = x + ox, Y = y + oy; if (X + rad < 0 || X - rad > S || Y + rad < 0 || Y - rad > S) continue;
        const gr = ctx.createRadialGradient(X, Y, 0, X, Y, rad); gr.addColorStop(0, rgba); gr.addColorStop(1, "rgba(0,0,0,0)");
        ctx.fillStyle = gr; ctx.fillRect(X - rad, Y - rad, 2 * rad, 2 * rad);
      }
    };
    for (let i = 0; i < 240; i++) { const a = 0.04 + r() * 0.08; blot(g, r() * S, r() * S, 10 + r() * 46, r() < 0.55 ? `rgba(20,12,12,${a})` : `rgba(${o.light ? "255,250,245" : "135,110,108"},${a * 0.8})`); }
    if (o.knurl) {
      g.strokeStyle = "rgba(0,0,0,0.22)"; b.strokeStyle = "rgba(40,40,40,0.5)"; g.lineWidth = b.lineWidth = 1.2;
      for (let i = -S; i <= S; i += 12) for (const ctx of [g, b]) { ctx.beginPath(); ctx.moveTo(i, 0); ctx.lineTo(i + S, S); ctx.moveTo(i, S); ctx.lineTo(i + S, 0); ctx.stroke(); }
    }
    if (o.panels) {
      const rects = [];
      (function split(x, y, w, h, d) {
        if (d > 3 || (w < 130 && h < 130) || (d > 1 && r() < 0.22)) { rects.push([x, y, w, h]); return; }
        if (w > h ? r() < 0.8 : r() < 0.2) { const k = Math.round(w * (0.3 + r() * 0.4) / 8) * 8; split(x, y, k, h, d + 1); split(x + k, y, w - k, h, d + 1); }
        else { const k = Math.round(h * (0.3 + r() * 0.4) / 8) * 8; split(x, y, w, k, d + 1); split(x, y + k, w, h - k, d + 1); }
      })(0, 0, S, S, 0);
      const labels = ["RV-07", "NO STEP", "04", "HX-2", "LIFT", "R7", "88", "DANGER", "CELL 3", "VENT", "A-12"];
      for (const [x, y, w, h] of rects) {
        const tone = r(); g.fillStyle = tone < 0.5 ? `rgba(0,0,0,${0.03 + r() * 0.07})` : `rgba(255,235,230,${0.02 + r() * 0.04})`; g.fillRect(x, y, w, h);
        g.fillStyle = "rgba(10,6,7,0.9)"; g.fillRect(x, y, w, 2.5); g.fillRect(x, y, 2.5, h);
        g.fillStyle = "rgba(170,145,140,0.35)"; g.fillRect(x, y + 2.5, w, 1); g.fillRect(x + 2.5, y, 1, h);
        b.fillStyle = "#262626"; b.fillRect(x, y, w, 3); b.fillRect(x, y, 3, h);
        if (r() < 0.5) for (let px = x + 8; px < x + w - 5; px += 14) for (const ctx of [g, b]) {
          ctx.beginPath(); ctx.arc(px, y + 8, 2.3, 0, TAU); ctx.fillStyle = ctx === g ? "#857170" : "#d0d0d0"; ctx.fill();
          if (ctx === g) { ctx.fillStyle = "rgba(0,0,0,0.5)"; ctx.fillRect(px - 0.5, y + 9.5, 1.8, 1.4); }
        }
        if (r() < 0.22 && w > 80 && h > 70) {
          const ix = x + 12, iy = y + 14, iw = w - 24, ih = h - 28;
          g.strokeStyle = "rgba(8,5,6,0.85)"; g.lineWidth = 2; g.strokeRect(ix, iy, iw, ih);
          g.strokeStyle = "rgba(160,140,135,0.3)"; g.lineWidth = 1; g.strokeRect(ix + 1.5, iy + 1.5, iw, ih);
          b.strokeStyle = "#3a3a3a"; b.lineWidth = 2.5; b.strokeRect(ix, iy, iw, ih);
          for (const [cx, cy] of [[ix + 6, iy + 6], [ix + iw - 6, iy + 6], [ix + 6, iy + ih - 6], [ix + iw - 6, iy + ih - 6]]) {
            g.beginPath(); g.arc(cx, cy, 2.8, 0, TAU); g.fillStyle = "#6e5c5b"; g.fill(); g.fillStyle = "#1a1010"; g.fillRect(cx - 2.2, cy - 0.5, 4.4, 1);
            b.beginPath(); b.arc(cx, cy, 2.8, 0, TAU); b.fillStyle = "#c0c0c0"; b.fill();
          }
        }
        if (r() < 0.16 && w > 60) { g.font = "bold 12px monospace"; g.fillStyle = "rgba(225,205,195,0.32)"; g.fillText(labels[Math.floor(r() * labels.length)], x + 9, y + h - 9); }
        if (r() < 0.1 && w > 70 && h > 40) {
          for (let k = 0; k < 6; k++) { g.fillStyle = k % 2 ? "rgba(20,12,10,0.45)" : "rgba(200,150,40,0.28)"; g.beginPath(); const sx = x + 10 + k * 8; g.moveTo(sx, y + h - 6); g.lineTo(sx + 8, y + h - 6); g.lineTo(sx + 14, y + h - 14); g.lineTo(sx + 6, y + h - 14); g.fill(); }
        }
      }
    }
    for (let i = 0; i < 260; i++) {
      const x = r() * S, y = r() * S, a = r() * TAU, l = 4 + r() * 24;
      g.strokeStyle = o.light ? "rgba(70,62,60,0.35)" : "rgba(195,175,170,0.24)"; g.lineWidth = 0.6 + r() * 0.8;
      g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * l, y + Math.sin(a) * l); g.stroke();
      b.strokeStyle = "rgba(95,95,95,0.5)"; b.lineWidth = 0.8; b.beginPath(); b.moveTo(x, y); b.lineTo(x + Math.cos(a) * l, y + Math.sin(a) * l); b.stroke();
    }
    for (let i = 0; i < 80; i++) {
      const x = r() * S, y = r() * S, s = 1.5 + r() * 4;
      g.fillStyle = o.light ? "rgba(90,85,85,0.35)" : "rgba(150,145,145,0.42)"; g.beginPath(); g.moveTo(x, y); g.lineTo(x + s, y + s * 0.4); g.lineTo(x + s * 0.6, y + s); g.lineTo(x - s * 0.3, y + s * 0.7); g.fill();
      b.fillStyle = "#606060"; b.fillRect(x, y, s * 0.8, s * 0.8);
    }
    for (let i = 0; i < 100; i++) {
      const x = r() * S, y = r() * S, l = 20 + r() * 90, w = 1.5 + r() * 4;
      const gr = g.createLinearGradient(0, y, 0, y + l); gr.addColorStop(0, "rgba(16,9,7,0.3)"); gr.addColorStop(1, "rgba(16,9,7,0)");
      g.fillStyle = gr; g.fillRect(x, y, w, l);
    }
    if (o.battered) {
      for (let i = 0; i < 7; i++) { const x = r() * S, y = r() * S, rad = 26 + r() * 56; blot(g, x, y, rad, "rgba(18,10,6,0.55)"); blot(g, x, y, rad * 0.45, "rgba(4,2,1,0.45)"); blot(b, x, y, rad * 0.6, "rgba(70,70,70,0.4)"); }
      for (let i = 0; i < 30; i++) {
        const x = r() * S, y = r() * S, rad = 2.2 + r() * 3;
        g.beginPath(); g.arc(x, y, rad + 1.4, 0, TAU); g.fillStyle = "rgba(175,160,155,0.35)"; g.fill();
        g.beginPath(); g.arc(x, y, rad, 0, TAU); g.fillStyle = "rgba(6,3,3,0.85)"; g.fill();
        b.beginPath(); b.arc(x, y, rad + 1, 0, TAU); b.fillStyle = "#1e1e1e"; b.fill();
      }
    }
    for (const [ctx, amp] of [[g, 14], [b, 8]]) {
      const im = ctx.getImageData(0, 0, S, S), d = im.data;
      for (let i = 0; i < d.length; i += 4) { const n = (r() - 0.5) * amp; d[i] += n; d[i + 1] += n; d[i + 2] += n; }
      ctx.putImageData(im, 0, 0);
    }
    return { map: toTex(col, true), bump: toTex(bmp, false) };
  }
  function textures() {
    if (TEX) return TEX;
    const armor = plating(11, { base: "#4d4042", panels: true });
    const armorB = plating(29, { base: "#453739", panels: true, battered: true });
    const frame = plating(47, { base: "#38363a", knurl: true });
    const wear = plating(63, { base: "#ddd6d3", light: true });
    const hz = mkCanvas(128), h = hz.getContext("2d");
    h.fillStyle = "#151010"; h.fillRect(0, 0, 128, 128);
    h.fillStyle = "#d8a21e";
    for (let i = -128; i < 256; i += 32) { h.beginPath(); h.moveTo(i, 0); h.lineTo(i + 16, 0); h.lineTo(i + 16 + 128, 128); h.lineTo(i + 128, 128); h.fill(); }
    const hr = rng(5);
    for (let i = 0; i < 60; i++) { h.fillStyle = `rgba(20,12,10,${0.2 + hr() * 0.4})`; h.fillRect(hr() * 128, hr() * 128, 1 + hr() * 6, 1 + hr() * 3); }
    const gl = mkCanvas(128), q = gl.getContext("2d");
    const gr = q.createRadialGradient(64, 64, 0, 64, 64, 64);
    gr.addColorStop(0, "rgba(255,255,255,1)"); gr.addColorStop(0.18, "rgba(255,255,255,0.75)"); gr.addColorStop(0.45, "rgba(255,255,255,0.18)"); gr.addColorStop(1, "rgba(255,255,255,0)");
    q.fillStyle = gr; q.fillRect(0, 0, 128, 128);
    const pc = mkCanvas(64), p = pc.getContext("2d");
    const pg = p.createLinearGradient(0, 0, 0, 64); pg.addColorStop(0, "rgba(255,255,255,0)"); pg.addColorStop(0.7, "rgba(255,255,255,0.35)"); pg.addColorStop(1, "rgba(255,255,255,1)");
    p.fillStyle = pg; p.fillRect(0, 0, 64, 64);
    const glow = gl; const pillar = pc;
    TEX = { armor: armor.map, armorBump: armor.bump, armorB: armorB.map, armorBBump: armorB.bump, frame: frame.map, frameBump: frame.bump, wear: wear.map, hazard: toTex(hz, true), glow, pillar };
    return TEX;
  }


// Crab Breeze: a normal map from a bump canvas (Sobel), strength in the kit's bumpScale spirit.
function normalOf(c, strength) {
  const S = c.width, g = c.getContext("2d"), src = g.getImageData(0, 0, S, S).data;
  const out = document.createElement("canvas"); out.width = out.height = S;
  const o = out.getContext("2d"), im = o.createImageData(S, S), d = im.data;
  const h = (x, y) => src[(((y + S) % S) * S + ((x + S) % S)) * 4] / 255;
  for (let y = 0; y < S; y++) for (let x = 0; x < S; x++) {
    const dx = (h(x + 1, y - 1) + 2 * h(x + 1, y) + h(x + 1, y + 1)) - (h(x - 1, y - 1) + 2 * h(x - 1, y) + h(x - 1, y + 1));
    const dy = (h(x - 1, y + 1) + 2 * h(x, y + 1) + h(x + 1, y + 1)) - (h(x - 1, y - 1) + 2 * h(x, y - 1) + h(x + 1, y - 1));
    let nx = -dx * strength, ny = dy * strength, nz = 1; const l = Math.hypot(nx, ny, nz); nx /= l; ny /= l; nz /= l;
    const i = (y * S + x) * 4; d[i] = (nx * 0.5 + 0.5) * 255; d[i + 1] = (ny * 0.5 + 0.5) * 255; d[i + 2] = (nz * 0.5 + 0.5) * 255; d[i + 3] = 255;
  }
  o.putImageData(im, 0, 0); return out;
}
window.__out = (() => {
  const X = textures(), png = c => c.toDataURL("image/png");
  return {
    armor: png(X.armor), armor_n: png(normalOf(X.armorBump, 2.5)),
    armorB: png(X.armorB), armorB_n: png(normalOf(X.armorBBump, 2.5)),
    frame: png(X.frame), frame_n: png(normalOf(X.frameBump, 2.0)),
    // Crab Breeze's one addition: a light panelled plate for white liveries, the same plating() call with a pale base.
    ...(() => { const L = plating(71, { base: "#c3c7cd", panels: true, light: true }); return { armorL: png(L.map), armorL_n: png(normalOf(L.bump, 2.5)) }; })(),
    wear: png(X.wear), hazard: png(X.hazard),
  };
})();
