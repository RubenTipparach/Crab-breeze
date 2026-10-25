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
  const S = c.width, V = c.height, g = c.getContext("2d"), src = g.getImageData(0, 0, S, V).data;
  const out = document.createElement("canvas"); out.width = S; out.height = V;
  const o = out.getContext("2d"), im = o.createImageData(S, V), d = im.data;
  const h = (x, y) => src[(((y + V) % V) * S + ((x + S) % S)) * 4] / 255;
  for (let y = 0; y < V; y++) for (let x = 0; x < S; x++) {
    const dx = (h(x + 1, y - 1) + 2 * h(x + 1, y) + h(x + 1, y + 1)) - (h(x - 1, y - 1) + 2 * h(x - 1, y) + h(x - 1, y + 1));
    const dy = (h(x - 1, y + 1) + 2 * h(x, y + 1) + h(x + 1, y + 1)) - (h(x - 1, y - 1) + 2 * h(x, y - 1) + h(x + 1, y - 1));
    let nx = -dx * strength, ny = dy * strength, nz = 1; const l = Math.hypot(nx, ny, nz); nx /= l; ny /= l; nz /= l;
    const i = (y * S + x) * 4; d[i] = (nx * 0.5 + 0.5) * 255; d[i + 1] = (ny * 0.5 + 0.5) * 255; d[i + 2] = (nz * 0.5 + 0.5) * 255; d[i + 3] = 255;
  }
  o.putImageData(im, 0, 0); return out;
}
// Crab Breeze: an occlusion-roughness-metal map (glTF packing: R occlusion, G roughness, B metal) from the
// same two canvases, so the plating is full PBR. Occlusion is the bump's cavity against its blurred self
// (seams, rivet rings, dents read dark); roughness is the paint's, rougher where it is grimy and darker
// than its base, smoother where it is worn through; metal is the bare steel showing through the paint.
// metal0 is the plate's own metalness (paint 0, knurled steel high).
function ormOf(colC, bmpC, o) {
  const S = colC.width, V = colC.height;
  const blur = document.createElement("canvas"); blur.width = S; blur.height = V;
  const q = blur.getContext("2d"); q.filter = "blur(5px)";
  for (const ox of [-S, 0, S]) for (const oy of [-V, 0, V]) q.drawImage(bmpC, ox, oy);
  const hb = q.getImageData(0, 0, S, V).data, h = bmpC.getContext("2d").getImageData(0, 0, S, V).data;
  const c = colC.getContext("2d").getImageData(0, 0, S, V).data;
  const base = parseInt(o.base.slice(1), 16), bl = (((base >> 16) & 255) * 0.3 + ((base >> 8) & 255) * 0.59 + (base & 255) * 0.11) / 255;
  const out = document.createElement("canvas"); out.width = S; out.height = V;
  const g = out.getContext("2d"), im = g.createImageData(S, V), d = im.data;
  for (let i = 0; i < d.length; i += 4) {
    const cav = Math.max(0, hb[i] - h[i]) / 255, lum = (c[i] * 0.3 + c[i + 1] * 0.59 + c[i + 2] * 0.11) / 255;
    const ao = Math.min(1, Math.max(0.25, 1 - cav * 3.2));
    const dirt = Math.min(1, Math.max(0, (bl - lum) * 2.2)), worn = Math.min(1, Math.max(0, (lum - bl) * (o.light ? 0 : 4)));
    const rough = Math.min(0.95, Math.max(0.18, o.rough + dirt * 0.3 - worn * 0.35));
    const metal = Math.min(1, Math.max(0, o.metal0 + worn * 0.8));
    d[i] = ao * 255; d[i + 1] = rough * 255; d[i + 2] = metal * 255; d[i + 3] = 255;
  }
  g.putImageData(im, 0, 0); return out;
}

// Crab Breeze: modern plating (owner, 2026-10-24: "rivets are kinda old school, I want more modern looking armor", with
// references of Armored Core VI frames, the Yokohama Gundam and weathered kit builds). Large smooth panels in a neutral
// light paint that each mech's palette tints, their seams engraved (a groove with a lit lip), clipped corners and
// inset panel lines, flush access hatches with pull slots and vent slots (no rivets anywhere), stencils (CAUTION,
// NO STEP, lift points, unit numbers), small hazard ticks, chipped edges and light grime. Seeded, so the same every run.
function modern(seed, o) {
  const S = 1024, col = mkCanvas(S), bmp = mkCanvas(S);
  const g = col.getContext("2d"), b = bmp.getContext("2d"), r = rng(seed);
  g.fillStyle = o.base; g.fillRect(0, 0, S, S);
  b.fillStyle = "#808080"; b.fillRect(0, 0, S, S);
  const blot = (ctx, x, y, rad, rgba) => {
    for (const ox of [-S, 0, S]) for (const oy of [-S, 0, S]) {
      const X = x + ox, Y = y + oy; if (X + rad < 0 || X - rad > S || Y + rad < 0 || Y - rad > S) continue;
      const gr = ctx.createRadialGradient(X, Y, 0, X, Y, rad); gr.addColorStop(0, rgba); gr.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = gr; ctx.fillRect(X - rad, Y - rad, 2 * rad, 2 * rad);
    }
  };
  for (let i = 0; i < 50; i++) blot(g, r() * S, r() * S, 90 + r() * 200, r() < 0.5 ? `rgba(0,0,0,${0.02 + r() * 0.03})` : `rgba(255,255,255,${0.02 + r() * 0.03})`);
  // the seam: a dark engraved groove, a lit lip on its far side, a dip in the bump
  const seam = (pts, wd) => {
    for (const [ctx, st, lw] of [[b, "#2a2a2a", wd + 1.5], [g, "rgba(16,17,19,0.8)", wd], [g, "rgba(255,255,255,0.28)", 1]]) {
      ctx.strokeStyle = st; ctx.lineWidth = lw; ctx.lineJoin = "miter"; ctx.beginPath();
      const off = st.startsWith("rgba(255") ? wd * 0.9 : 0;
      pts.forEach(([x, y], i) => i ? ctx.lineTo(x + off, y + off) : ctx.moveTo(x + off, y + off)); ctx.stroke();
    }
  };
  const clipped = (x, y, w, h, c) => [[x + c, y], [x + w - c, y], [x + w, y + c], [x + w, y + h - c], [x + w - c, y + h], [x + c, y + h], [x, y + h - c], [x, y + c], [x + c, y]];
  const rects = [];
  (function split(x, y, w, h, d) {
    if (d > 3 || (w < 260 && h < 260) || (d > 1 && r() < 0.25)) { rects.push([x, y, w, h]); return; }
    if (w > h ? r() < 0.75 : r() < 0.25) { const k = Math.round(w * (0.3 + r() * 0.4) / 16) * 16; split(x, y, k, h, d + 1); split(x + k, y, w - k, h, d + 1); }
    else { const k = Math.round(h * (0.3 + r() * 0.4) / 16) * 16; split(x, y, w, k, d + 1); split(x, y + k, w, h - k, d + 1); }
  })(0, 0, S, S, 0);
  // working stencils, mixed so none repeats across a plate (owner, 2026-10-24: "instead of \"danger\" plastered everywhere
  // it should say \"no step\", company logo, \"warning flamable\" etc")
  const words = o.words || ["NO STEP", "WARNING FLAMMABLE", "HYD FLUID 3000 PSI", "LIFT POINT", "TOW HERE", "GROUND POINT",
    "FUEL", "NO PUSH", "RESCUE", "EJECT", "LUBE WEEKLY", "07", "24", "A-12", "LOT 4417"];
  let wi = Math.floor(r() * words.length), logos = 0;
  // the maker's mark: Halverd Heavy Industries, Crab Breeze's own invented firm, a split chevron in a ring and its name
  const logo = (lx, ly) => {
    g.strokeStyle = "rgba(20,20,22,0.7)"; g.lineWidth = 3; g.beginPath(); g.arc(lx + 22, ly + 22, 20, 0, TAU); g.stroke();
    g.fillStyle = "rgba(20,20,22,0.7)";
    for (const d of [-1, 1]) { g.beginPath(); g.moveTo(lx + 22, ly + 8); g.lineTo(lx + 22 + d * 13, ly + 30); g.lineTo(lx + 22 + d * 8, ly + 30); g.lineTo(lx + 22 + d * 1.5, ly + 18); g.fill(); }
    g.font = "bold 18px Arial, sans-serif"; g.fillText("HALVERD", lx + 50, ly + 20);
    g.font = "bold 10px Arial, sans-serif"; g.fillText("HEAVY INDUSTRIES", lx + 50, ly + 36);
  };
  for (const [x, y, w, h] of rects) {
    g.fillStyle = r() < 0.5 ? `rgba(0,0,0,${0.02 + r() * 0.05})` : `rgba(255,255,255,${0.02 + r() * 0.04})`; g.fillRect(x, y, w, h);
    seam([[x, y + h], [x, y], [x + w, y]], 2.5);
    if (r() < 0.55 && w > 160 && h > 160) {                          // an inset plate with clipped corners
      const m = 18 + r() * 22, c = 14 + r() * 26;
      seam(clipped(x + m, y + m, w - 2 * m, h - 2 * m, c), 1.6);
      if (r() < 0.5) { b.fillStyle = "rgba(160,160,160,0.35)"; b.fillRect(x + m + 4, y + m + 4, w - 2 * m - 8, h - 2 * m - 8); }   // raised a touch
    }
    if (r() < 0.3 && w > 140 && h > 110) {                          // a flush access hatch with two pull slots
      const hw = 70 + r() * 60, hh = 46 + r() * 40, hx = x + 24 + r() * (w - hw - 48), hy = y + 24 + r() * (h - hh - 48);
      seam(clipped(hx, hy, hw, hh, 8), 1.4);
      for (const sx of [hx + 12, hx + hw - 28]) { g.fillStyle = "rgba(10,10,12,0.85)"; g.fillRect(sx, hy + hh / 2 - 3, 16, 6); b.fillStyle = "#202020"; b.fillRect(sx, hy + hh / 2 - 3, 16, 6); }
    }
    if (r() < 0.2 && w > 150 && h > 90) {                           // vent slots
      const n = 5 + Math.floor(r() * 5), vx = x + 26 + r() * (w - 140), vy = y + 26 + r() * (h - 80);
      for (let k = 0; k < n; k++) { g.fillStyle = "rgba(12,12,14,0.9)"; g.fillRect(vx + k * 12, vy, 6, 42); b.fillStyle = "#1c1c1c"; b.fillRect(vx + k * 12, vy, 6, 42); }
    }
    if (o.stencils && logos < 2 && r() < 0.18 && w > 220 && h > 90) { logo(x + 20 + r() * (w - 220), y + 20 + r() * (h - 80)); logos++; }
    else if (o.stencils && r() < 0.28 && w > 120) {                 // a stencil, the next word in turn (off: decals carry them)
      const t = words[wi++ % words.length], big = /^\d+$/.test(t);
      g.font = big ? "bold 64px Arial, sans-serif" : `bold ${14 + Math.floor(r() * 6)}px Arial, sans-serif`;
      g.fillStyle = big ? "rgba(20,20,22,0.55)" : "rgba(20,20,22,0.6)";
      g.fillText(t, x + 16 + r() * Math.max(1, w * 0.3), y + h - 18 - r() * Math.max(1, h * 0.4));
      if (!big && r() < 0.5) { g.fillRect(x + 16, y + h - 14, Math.min(w - 32, 90), 2); }
    }
    if (r() < 0.12 && w > 120 && h > 60) {                          // hazard ticks
      const tx = x + 16, ty = y + 18;
      for (let k = 0; k < 7; k++) { g.fillStyle = k % 2 ? "rgba(18,16,14,0.8)" : "rgba(230,170,40,0.85)"; g.beginPath(); g.moveTo(tx + k * 10, ty + 14); g.lineTo(tx + k * 10 + 10, ty + 14); g.lineTo(tx + k * 10 + 16, ty); g.lineTo(tx + k * 10 + 6, ty); g.fill(); }
    }
    for (let k = 0; k < 10; k++) {                                  // chips along the seams: paint gone to steel
      const along = r(), onTop = r() < 0.5, cx = onTop ? x + along * w : x + 2 + r() * 6, cy = onTop ? y + 2 + r() * 6 : y + along * h, s = 1.5 + r() * 5;
      g.fillStyle = o.chip || "rgba(200,200,205,0.55)"; g.beginPath(); g.moveTo(cx, cy); g.lineTo(cx + s, cy + s * 0.3); g.lineTo(cx + s * 0.5, cy + s); g.lineTo(cx - s * 0.3, cy + s * 0.6); g.fill();
      b.fillStyle = "#6a6a6a"; b.fillRect(cx, cy, s * 0.7, s * 0.7);
    }
  }
  for (let i = 0; i < 140; i++) {                                    // fine scratches
    const x = r() * S, y = r() * S, a = r() * TAU, l = 6 + r() * 30;
    g.strokeStyle = "rgba(230,230,235,0.25)"; g.lineWidth = 0.6 + r() * 0.6; g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * l, y + Math.sin(a) * l); g.stroke();
  }
  for (let i = 0; i < 70; i++) {                                     // grime run down from the seams
    const x = r() * S, y = r() * S, l = 40 + r() * 160, w = 2 + r() * 6;
    const gr = g.createLinearGradient(0, y, 0, y + l); gr.addColorStop(0, `rgba(30,26,22,${0.12 + r() * 0.12})`); gr.addColorStop(1, "rgba(30,26,22,0)");
    g.fillStyle = gr; g.fillRect(x, y, w, l);
  }
  for (let i = 0; i < 18; i++) blot(g, r() * S, r() * S, 30 + r() * 80, `rgba(40,34,28,${0.08 + r() * 0.1})`);
  for (const [ctx, amp] of [[g, 6], [b, 4]]) {
    const im = ctx.getImageData(0, 0, S, S), d = im.data;
    for (let i = 0; i < d.length; i += 4) { const n = (r() - 0.5) * amp; d[i] += n; d[i + 1] += n; d[i + 2] += n; }
    ctx.putImageData(im, 0, 0);
  }
  return { map: col, bump: bmp };
}


// Crab Breeze: panelling for the crab air fortress laid out by the shape it covers (owner, 2026-10-25: "its all random
// paneling, I dont like that, paneling must look organized and intentional", "see how the panel aligns with the flow of
// the geometry?", "like it fits like big symmetric jigsaw", "each element has paneling that compliments its shape",
// "avoid 4 junctions in a paneling ... NEVER HAVE A CROSS in the seams", "you want t junctions, and about 20-30%
// diagonal lines", with a saucer's ring plating, a bird of prey's feathered wings and a painted kit panel for
// reference). Each sheet is drawn as a height field first, so the normal map carries the big shapes: plates (convex
// polygons) with deep bevels over recessed channels. Seams meet only in T-junctions: rows of plates are laid as
// bricks, each row half a plate out from the next, and a diagonal seam ends on a channel at three or five eighths of
// its plate, clear of every seam on the other side. Three layouts:
//   carapace: the shell's rings (v, from its middle out to its rim) cut into sectors (u, once round), each pair of rings
//             8, 16, 32 then 64 sectors, half a sector out from each other and from the ring before; mirrored about the
//             centreline (u = 0.5 the bow), a mirrored pair taking the same detail and mirror-image diagonals; a quarter
//             of the plates split by a diagonal; outer corners clipped like feathers; a livery mask for a band near the
//             rim and a stripe down the centreline; two ring channels lit as conduits.
//   drum:     a cylinder's wrap, two bands of one plate each, the second half a plate round and split by a diagonal.
//   slab:     a flat face's module, the same two-row brick: a whole symmetric plate over a pair split by a diagonal.
// The colour is a neutral paint the palette tints; each sheet also gives an emission and a livery mask.
function panelSheet(W, H) {
  const mk = () => { const c = document.createElement("canvas"); c.width = W; c.height = H; return c; };
  const K = { W, H, col: mk(), bmp: mk(), emi: mk(), liv: mk() };
  K.g = K.col.getContext("2d"); K.b = K.bmp.getContext("2d"); K.e = K.emi.getContext("2d"); K.l = K.liv.getContext("2d");
  K.g.fillStyle = "#2a2b2f"; K.g.fillRect(0, 0, W, H);
  K.b.fillStyle = "rgb(40,40,40)"; K.b.fillRect(0, 0, W, H);
  K.e.fillStyle = "#000"; K.e.fillRect(0, 0, W, H); K.l.fillStyle = "#000"; K.l.fillRect(0, 0, W, H);
  const grey = v => `rgb(${v},${v},${v})`;
  // a rectangle with each corner clipped by its own amount, as a polygon
  K.shape = (x, y, w, h, c) => [[x + c[0], y], [x + w - c[1], y], [x + w, y + c[1]], [x + w, y + h - c[2]], [x + w - c[2], y + h], [x + c[3], y + h], [x, y + h - c[3]], [x, y + c[0]]];
  K.poly = (ctx, pts) => { ctx.beginPath(); pts.forEach(([x, y], i) => i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)); ctx.closePath(); };
  const clean = pts => pts.filter((p, i) => { const q = pts[(i + 1) % pts.length]; return Math.hypot(p[0] - q[0], p[1] - q[1]) > 0.5; });
  // a convex polygon moved in by d: each edge pushed in along its normal, neighbours intersected
  K.offset = (pts, d) => {
    pts = clean(pts); const n = pts.length; if (n < 3 || d <= 0) return pts;
    const cx = pts.reduce((a, p) => a + p[0], 0) / n, cy = pts.reduce((a, p) => a + p[1], 0) / n;
    const lines = pts.map((p, i) => { const q = pts[(i + 1) % n], ex = q[0] - p[0], ey = q[1] - p[1], l = Math.hypot(ex, ey);
      let nx = -ey / l, ny = ex / l; if ((cx - p[0]) * nx + (cy - p[1]) * ny < 0) { nx = -nx; ny = -ny; }
      return { x: p[0] + nx * d, y: p[1] + ny * d, ex, ey }; });
    return lines.map((L1, i) => { const L0 = lines[(i + n - 1) % n], den = L0.ex * L1.ey - L0.ey * L1.ex;
      if (Math.abs(den) < 1e-9) return [L1.x, L1.y];
      const t = ((L1.x - L0.x) * L1.ey - (L1.y - L0.y) * L1.ex) / den; return [L0.x + L0.ex * t, L0.y + L0.ey * t]; });
  };
  // the part of a convex polygon at least gap/2 to one side of the line a-b (side +1 or -1)
  K.cut = (pts, a, b, side, gap) => {
    const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy), sd = p => side * ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / l - gap / 2, out = [];
    pts.forEach((p, i) => { const q = pts[(i + 1) % pts.length], sp = sd(p), sq = sd(q);
      if (sp >= 0) out.push(p);
      if ((sp >= 0) !== (sq >= 0)) { const t = sp / (sp - sq); out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]); } });
    return out;
  };
  // a polygon shrunk toward its centre (an inner panel: unlike a large offset it cannot turn itself inside out)
  K.shrink = (pts, f) => { const n = pts.length, cx = pts.reduce((a, p) => a + p[0], 0) / n, cy = pts.reduce((a, p) => a + p[1], 0) / n;
    return pts.map(([x, y]) => [cx + (x - cx) * f, cy + (y - cy) * f]); };
  K.size = pts => { const xs = pts.map(p => p[0]), ys = pts.map(p => p[1]); return Math.min(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)); };
  // a bevel: the polygon moved in step by step, its height ramping from h0 to h1 over d pixels
  K.bevel = (pts, d, h0, h1, steps = 12) => {
    for (let k = 0; k <= steps; k++) { const t = k / steps, v = Math.round(h0 + (h1 - h0) * Math.sin(t * Math.PI / 2));
      K.b.fillStyle = grey(v); K.poly(K.b, K.offset(pts, d * t)); K.b.fill(); }
  };
  // a plate: paint, a bevel up from the channel, a worn rim
  K.plate = (pts, base, d, tone) => {
    K.g.fillStyle = base; K.poly(K.g, pts); K.g.fill();
    if (tone) { K.g.fillStyle = tone; K.poly(K.g, pts); K.g.fill(); }
    K.bevel(pts, d, 80, 192);
    K.g.strokeStyle = "rgba(235,235,240,0.35)"; K.g.lineWidth = 2; K.poly(K.g, K.offset(pts, d * 0.3)); K.g.stroke();
  };
  // an inner panel following its plate's outline, raised or recessed
  K.inset = (pts, d, up) => {
    K.bevel(pts, d, 192, up ? 224 : 156, 6);
    K.g.strokeStyle = "rgba(14,14,16,0.55)"; K.g.lineWidth = 2; K.poly(K.g, pts); K.g.stroke();
    K.g.fillStyle = up ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.07)"; K.poly(K.g, K.offset(pts, d)); K.g.fill();
  };
  K.vents = (cx, cy, n, sw, sh, gap) => { const x0 = cx - (n * sw + (n - 1) * gap) / 2;
    for (let k = 0; k < n; k++) { K.b.fillStyle = grey(110); K.b.fillRect(x0 + k * (sw + gap), cy - sh / 2, sw, sh); K.g.fillStyle = "#18191c"; K.g.fillRect(x0 + k * (sw + gap), cy - sh / 2, sw, sh); } };
  K.hatch = (cx, cy, w, h) => { const pts = K.shape(cx - w / 2, cy - h / 2, w, h, [8, 8, 8, 8]);
    K.b.strokeStyle = grey(120); K.b.lineWidth = 3; K.poly(K.b, pts); K.b.stroke();
    K.g.strokeStyle = "rgba(16,16,18,0.7)"; K.g.lineWidth = 2.5; K.poly(K.g, pts); K.g.stroke();
    K.g.fillStyle = "rgba(16,16,18,0.8)"; K.g.fillRect(cx - 8, cy - 2.5, 16, 5); };
  K.bolt = (x, y, r = 6) => { const gr = K.b.createRadialGradient(x, y, 0, x, y, r); gr.addColorStop(0, grey(240)); gr.addColorStop(1, grey(192));
    K.b.fillStyle = gr; K.b.beginPath(); K.b.arc(x, y, r, 0, TAU); K.b.fill(); K.g.fillStyle = "rgba(60,60,64,0.8)"; K.g.beginPath(); K.g.arc(x, y, r - 1, 0, TAU); K.g.fill(); };
  K.grime = (x, y, w, h, r) => { for (let k = 0; k < 4; k++) { const gx = x + r() * w, l = h * (0.2 + r() * 0.4), gr = K.g.createLinearGradient(0, y, 0, y + l);
    gr.addColorStop(0, `rgba(30,26,22,${0.08 + r() * 0.1})`); gr.addColorStop(1, "rgba(30,26,22,0)"); K.g.fillStyle = gr; K.g.fillRect(gx, y, 2 + r() * 5, l); } };
  K.noise = r => { for (const [ctx, amp] of [[K.g, 6], [K.b, 3]]) { const im = ctx.getImageData(0, 0, W, H), d = im.data;
    for (let i = 0; i < d.length; i += 4) { const n = (r() - 0.5) * amp; d[i] += n; d[i + 1] += n; d[i + 2] += n; } ctx.putImageData(im, 0, 0); } };
  // one cell of plating: whole, or split by a diagonal from `a` of the way along its top to `b` along its bottom;
  // each piece bevelled, with an inner panel following it when it is big enough
  K.cell = (x, y, w, h, c, o) => {
    const outline = K.shape(x, y, w, h, c), pieces = [];
    if (o.diag) { const A = [x + w * o.diag[0], y], B = [x + w * o.diag[1], y + h]; pieces.push(K.cut(outline, A, B, 1, o.gap), K.cut(outline, A, B, -1, o.gap)); }
    else pieces.push(outline);
    pieces.forEach(pts => { if (pts.length < 3) return; K.plate(pts, o.base, o.bevel, o.tone);
      const m = K.size(pts); if (o.inner && m > 4 * o.bevel) K.inset(K.shrink(pts, 0.62), Math.max(3, o.bevel * 0.45), o.up); });
    return pieces;
  };
  return K;
}
function carapacePanels(seed, o) {
  const W = 4096, H = 1024, K = panelSheet(W, H);
  const RINGS = [0, 0.18, 0.33, 0.47, 0.6, 0.72, 0.83, 0.92, 1.0];
  const SECTORS = [[8, 0.5], [8, 0], [16, 0.5], [16, 0], [32, 0.5], [32, 0], [64, 0.5], [64, 0]];   // half a sector out, ring to ring
  const LIVERY_RING = 6, GLOW_RINGS = [2, 6];
  for (let i = 0; i < SECTORS.length; i++) {
    const [N, off] = SECTORS[i], y0 = RINGS[i] * H, y1 = RINGS[i + 1] * H, cw = W / N;
    for (let k = 0; k < N; k++) {
      const uc = (k + off + 0.5) / N, left = uc < 0.5, mirror = Math.round(Math.abs(uc - 0.5) * N * 2);
      const r = rng(seed * 7919 + i * 131 + mirror);                    // the same choices for a mirrored pair
      const gu = 5, gv = 6, x = (k + off) * cw + gu, y = y0 + gv, w = cw - 2 * gu, h = (y1 - y0) - 2 * gv, m = Math.min(w, h);
      const c = [0.05 * m, 0.05 * m, 0.22 * m, 0.22 * m];               // tapered toward the rim, like feathers
      const tone = r() < 0.5 ? "rgba(0,0,0,0.04)" : "rgba(255,255,255,0.035)", split = r() < 0.25, flip = r() < 0.5, inner = r() < 0.7, up = r() < 0.4, det = r();
      // a diagonal from 3/8 to 5/8 (or back), mirrored on the mirrored side; never on the plate on the centreline
      const onCentre = Math.abs(uc - 0.5) < 0.5 / N || uc < 0.5 / N || uc > 1 - 0.5 / N;
      let diag = split && !onCentre ? (flip ? [0.375, 0.625] : [0.625, 0.375]) : null;
      if (diag && left) diag = [1 - diag[0], 1 - diag[1]];
      for (const ox of [0, -W, W]) {                                    // a plate across the seam round the back, drawn on both sides
        const X = x + ox; if (X > W || X + w < 0) continue;
        const pieces = K.cell(X, y, w, h, c, { base: o.base, bevel: 9, tone, diag, gap: 10, inner, up });
        if (!diag && det < 0.25 && w > 90 && h > 60) K.vents(X + w / 2, y + h * 0.5, Math.max(3, Math.min(7, Math.floor(w / 26))), 8, h * 0.3, 8);
        else if (!diag && det < 0.5 && w > 80 && h > 60) K.hatch(X + w / 2, y + h / 2, Math.min(w * 0.4, 120), Math.min(h * 0.4, 70));
        for (const [bx, by] of [[X + 14, y + 14], [X + w - 14, y + 14]]) K.bolt(bx, by, 5);
        // livery: a band round the shell near the rim, and a stripe down the centreline fore and aft
        if (i === LIVERY_RING || (onCentre && i >= 2)) { K.l.fillStyle = "#fff"; pieces.forEach(pts => { K.poly(K.l, pts); K.l.fill(); }); }
      }
      K.grime(x, y, w, h, r);
    }
  }
  // conduits: two whole ring channels lit
  for (const gi of GLOW_RINGS) { const y = RINGS[gi] * H;
    K.e.fillStyle = "#fff"; K.e.fillRect(0, y - 2, W, 4); K.g.fillStyle = "#d8f0d8"; K.g.fillRect(0, y - 1.5, W, 3); K.b.fillStyle = "rgb(70,70,70)"; K.b.fillRect(0, y - 2, W, 4); }
  K.noise(rng(seed));
  return K;
}
// the two-row brick both modules use: a whole plate over a pair of half plates split at the middle (the next tile's
// plate edges fall at the middle of this one's), the lower one split by a diagonal; T-junctions only
function brickModule(K, o, r, top, bottom) {
  const S = K.W, g = 8, half = S;   // each row is S by S; the sheet is S wide and 2S tall
  K.cell(g, g, S - 2 * g, half - 2 * g, o.c, { base: o.base, bevel: o.bevel, gap: 2 * g, inner: true, up: true });
  top(0);
  // the shifted row: one plate from the middle across the tile edge to the middle, drawn on both sides
  for (const ox of [-S / 2, S / 2]) K.cell(ox + g, half + g, S - 2 * g, half - 2 * g, o.c, { base: o.base, bevel: o.bevel, gap: 2 * g, diag: o.diag, inner: true, up: false });
  bottom(half);
  K.grime(g, g, S - 2 * g, 2 * S - 2 * g, r); K.noise(r);
}
function drumPanels(seed, o) {
  const S = 1024, K = panelSheet(S, 2 * S), r = rng(seed);
  brickModule(K, { base: o.base, bevel: 22, c: [24, 24, 24, 24], diag: [0.375, 0.625] }, r,
    y => { for (const yy of [0.2, 0.5, 0.8]) for (const x of [0.09, 0.91]) K.bolt(x * S, y + yy * S, 9); K.vents(S / 2, y + S * 0.84, 7, 22, 70, 18);
      K.e.fillStyle = "#fff"; K.e.fillRect(S * 0.3, y + S - 6, S * 0.4, 4); },
    y => { for (const x of [0.06, 0.94]) K.bolt(x * S, y + S * 0.5, 9); });
  return K;
}
function slabPanels(seed, o) {
  const S = 1024, K = panelSheet(S, 2 * S), r = rng(seed);
  brickModule(K, { base: o.base, bevel: 24, c: [70, 70, 70, 70], diag: [0.375, 0.625] }, r,
    y => { for (const x of [0.12, 0.88]) K.hatch(x * S, y + S * 0.78, 100, 140);           // a mirrored pair of hatches
      K.vents(S / 2, y + S * 0.84, 9, 16, 80, 14);
      for (const [x, yy] of [[0.1, 0.1], [0.9, 0.1], [0.1, 0.9], [0.9, 0.9]]) K.bolt(x * S, y + yy * S, 10); },
    y => { for (const x of [0.06, 0.94]) K.bolt(x * S, y + S * 0.5, 10); });
  return K;
}


// Crab Breeze: the decal sheet (owner, 2026-10-24, with water-slide decal sheets for reference: "instead of \"danger\"
// plastered everywhere it should say \"no step\", company logo, \"warning flamable\" etc"). Crab Breeze's own markings on
// an 8 by 8 grid of 128 px cells, transparent between them; DECALS names each one's cells (col, row, cols, rows), and
// gen_mech_plating.mjs writes that table beside the image for the Blender kit. Dark and white variants, so a light
// scheme carries dark markings and a dark one white.
const DECALS = {};
function decalSheet() {
  const C = 128, S = 1024, cv = mkCanvas(S), g = cv.getContext("2d");
  g.clearRect(0, 0, S, S);
  const DK = "#16171a", WH = "#f2f0ea", YL = "#f2b21c", RD = "#c8281e";
  const cell = (name, c, r, w, h, draw) => { DECALS[name] = [c, r, w, h]; g.save(); g.translate(c * C, r * C); draw(w * C, h * C); g.restore(); };
  const text = (t, x, y, px, col, align = "center", weight = "bold") => { g.font = `${weight} ${px}px Arial, sans-serif`; g.textAlign = align; g.textBaseline = "middle"; g.fillStyle = col; g.fillText(t, x, y); };
  const box = (x, y, w, h, col, lw = 4) => { g.strokeStyle = col; g.lineWidth = lw; g.strokeRect(x, y, w, h); };
  for (const [nm, col, cx] of [["nostep_d", DK, 0], ["nostep_w", WH, 2]])
    cell(nm, cx, 0, 2, 1, (w, h) => { box(10, 22, w - 20, h - 44, col); text("NO STEP", w / 2, h / 2, 40, col); g.fillStyle = col; g.fillRect(16, 30, 8, h - 60); g.fillRect(w - 24, 30, 8, h - 60); });
  cell("flammable", 4, 0, 2, 1, (w, h) => { g.fillStyle = RD; g.fillRect(8, 18, w - 16, h - 36); text("WARNING", w / 2, h / 2 - 16, 28, WH); text("FLAMMABLE", w / 2, h / 2 + 18, 30, WH); });
  cell("caution_hyd", 6, 0, 2, 1, (w, h) => { g.fillStyle = YL; g.fillRect(8, 18, w - 16, h - 36); text("CAUTION", w / 2, h / 2 - 16, 28, DK); text("HYD FLUID 3000 PSI", w / 2, h / 2 + 16, 18, DK); });
  for (const [nm, col, cx] of [["logo_d", DK, 0], ["logo_w", WH, 2]])
    cell(nm, cx, 1, 2, 1, (w, h) => {
      g.strokeStyle = col; g.lineWidth = 6; g.beginPath(); g.arc(60, h / 2, 40, 0, TAU); g.stroke(); g.fillStyle = col;
      for (const d of [-1, 1]) { g.beginPath(); g.moveTo(60, h / 2 - 28); g.lineTo(60 + d * 26, h / 2 + 18); g.lineTo(60 + d * 16, h / 2 + 18); g.lineTo(60 + d * 3, h / 2 - 6); g.fill(); }
      text("HALVERD", 110, h / 2 - 12, 32, col, "left"); text("HEAVY INDUSTRIES", 111, h / 2 + 20, 14, col, "left");
    });
  cell("lift", 4, 1, 1, 1, (w, h) => { g.strokeStyle = DK; g.lineWidth = 6; g.beginPath(); g.arc(w / 2, 46, 26, 0, TAU); g.stroke(); g.fillStyle = DK; g.beginPath(); g.moveTo(w / 2 - 16, 80); g.lineTo(w / 2 + 16, 80); g.lineTo(w / 2, 100); g.fill(); text("LIFT", w / 2, 115, 20, DK); });
  cell("tow", 5, 1, 1, 1, (w, h) => { g.strokeStyle = YL; g.lineWidth = 10; g.beginPath(); g.arc(w / 2, 52, 30, 0, TAU); g.stroke(); text("TOW", w / 2, 110, 24, YL); });
  cell("ground", 6, 1, 1, 1, (w, h) => { g.fillStyle = DK; for (let i = 0; i < 3; i++) g.fillRect(w / 2 - 34 + i * 12, 60 + i * 14, 68 - i * 24, 6); g.fillRect(w / 2 - 3, 24, 6, 36); text("GND", w / 2, 116, 20, DK); });
  cell("eject", 7, 1, 1, 1, (w, h) => { g.fillStyle = RD; g.beginPath(); g.moveTo(w / 2, 12); g.lineTo(w - 12, h - 30); g.lineTo(12, h - 30); g.fill(); text("EJECT", w / 2, h - 14, 22, RD); text("!", w / 2, 64, 52, WH); });
  for (const [nm, col, cx] of [["chevron_d", DK, 0], ["chevron_w", WH, 1]])
    cell(nm, cx, 2, 1, 1, (w, h) => { g.fillStyle = col; for (let i = 0; i < 3; i++) { const y = 20 + i * 30; g.beginPath(); g.moveTo(14, y); g.lineTo(w / 2, y + 26); g.lineTo(w - 14, y); g.lineTo(w - 14, y + 14); g.lineTo(w / 2, y + 40); g.lineTo(14, y + 14); g.fill(); } });
  for (const [nm, col, cx] of [["emblem_d", DK, 2], ["emblem_w", WH, 4]])
    cell(nm, cx, 2, 2, 2, (w, h) => {       // Breakwater squadron: a shield, a crab's claw closing, the name on a scroll
      g.strokeStyle = col; g.lineWidth = 8; g.beginPath(); g.moveTo(w / 2, 14); g.lineTo(w - 30, 40); g.lineTo(w - 40, 150); g.lineTo(w / 2, 210); g.lineTo(40, 150); g.lineTo(30, 40); g.closePath(); g.stroke();
      g.fillStyle = col; const cx = w / 2, cy = 120;           // a crab's claw: the palm, the fixed finger and the closing dactyl
      g.beginPath(); g.ellipse(cx - 18, cy + 30, 34, 26, -0.5, 0, TAU); g.fill();
      g.beginPath(); g.moveTo(cx - 30, cy + 10); g.quadraticCurveTo(cx - 20, cy - 60, cx + 40, cy - 66); g.quadraticCurveTo(cx + 10, cy - 40, cx + 2, cy + 6); g.fill();
      g.beginPath(); g.moveTo(cx + 4, cy + 30); g.quadraticCurveTo(cx + 50, cy + 10, cx + 58, cy - 40); g.quadraticCurveTo(cx + 36, cy - 4, cx - 4, cy + 4); g.fill();
      text("BREAKWATER", w / 2, 236, 24, col);
    });
  cell("rescue", 6, 2, 2, 1, (w, h) => { g.fillStyle = YL; g.fillRect(8, 26, w - 16, h - 52); text("RESCUE", w / 2 - 20, h / 2, 34, DK); g.fillStyle = DK; g.beginPath(); g.moveTo(w - 70, h / 2 - 16); g.lineTo(w - 30, h / 2); g.lineTo(w - 70, h / 2 + 16); g.fill(); });
  cell("fuel", 0, 3, 1, 1, (w, h) => { box(10, 30, w - 20, h - 60, RD, 6); text("FUEL", w / 2, h / 2, 30, RD); });
  cell("marks", 1, 3, 1, 1, (w, h) => {                      // red marker boxes and a ring-and-tail
    g.fillStyle = RD; g.fillRect(10, 14, 46, 16); g.fillRect(64, 14, 24, 16); g.fillRect(10, 40, 24, 8);
    g.strokeStyle = RD; g.lineWidth = 5; g.beginPath(); g.arc(24, 84, 12, 0, TAU); g.stroke(); g.fillRect(36, 82, 70, 5);
    g.fillRect(10, 108, 90, 6);
  });
  cell("keepclear", 6, 3, 2, 1, (w, h) => { g.strokeStyle = RD; g.lineWidth = 4; g.strokeRect(10, 28, w - 20, h - 56); text("KEEP CLEAR", w / 2, h / 2 - 8, 30, RD); text("JET BLAST", w / 2, h / 2 + 20, 16, RD); });
  for (const [nm, col, cx] of [["data_d", DK, 0], ["data_w", WH, 2]])
    cell(nm, cx, 4, 2, 1, (w, h) => { box(8, 12, w - 16, h - 24, col, 3); text("HALVERD H.I.  MODEL HV-24", 18, 34, 13, col, "left"); text("SERIAL 0417-B  MASS 380 T", 18, 58, 13, col, "left"); text("INSPECTED 10-86  SVC 500 H", 18, 82, 13, col, "left"); g.fillStyle = col; g.fillRect(18, 98, w - 36, 3); });
  cell("hazard", 4, 4, 2, 1, (w, h) => { g.save(); g.beginPath(); g.rect(6, 34, w - 12, h - 68); g.clip(); g.fillStyle = DK; g.fillRect(6, 34, w - 12, h - 68); g.fillStyle = YL; for (let x = -60; x < w; x += 36) { g.beginPath(); g.moveTo(x, 34); g.lineTo(x + 18, 34); g.lineTo(x + 78, h - 34); g.lineTo(x + 60, h - 34); g.fill(); } g.restore(); });
  cell("arrow_w", 6, 4, 1, 1, (w, h) => { g.fillStyle = WH; g.fillRect(20, h / 2 - 8, 60, 16); g.beginPath(); g.moveTo(76, h / 2 - 26); g.lineTo(112, h / 2); g.lineTo(76, h / 2 + 26); g.fill(); });
  cell("cross_d", 7, 4, 1, 1, (w, h) => { g.strokeStyle = DK; g.lineWidth = 5; g.beginPath(); g.arc(w / 2, h / 2, 40, 0, TAU); g.stroke(); g.fillStyle = DK; g.fillRect(w / 2 - 2, 10, 4, h - 20); g.fillRect(10, h / 2 - 2, w - 20, 4); });
  // line decals after a kit builder's panel lines (owner's references, 2026-10-24): brackets and dashes that follow a
  // plate's edge, white and dark; small red marker boxes and ring-and-tail markers
  for (const [nm, col, cx] of [["lines_w", WH, 4], ["lines_d", DK, 6]])
    cell(nm, cx, 7, 2, 1, (w, h) => {
      g.fillStyle = col;
      g.fillRect(10, 14, 90, 6); g.fillRect(10, 14, 6, 40);                       // an L bracket
      g.fillRect(w - 100, h - 20, 90, 6); g.fillRect(w - 16, h - 54, 6, 40);      // its partner, the far corner
      for (let i = 0; i < 4; i++) g.fillRect(120 + i * 22, 14, 14, 6);           // a dash run
      g.beginPath(); g.moveTo(30, h - 30); g.lineTo(80, h - 30); g.lineTo(96, h - 46); g.lineTo(150, h - 46); g.lineTo(150, h - 40); g.lineTo(99, h - 40); g.lineTo(83, h - 24); g.lineTo(30, h - 24); g.fill();
      g.fillRect(w / 2 - 20, h / 2 - 3, 40, 6);
    });
  // the digits, white in rows 5 and 6, dark in 6 and 7, stencil weight
  for (let d = 0; d < 10; d++) {
    const iw = d, id = d + 10;
    cell(`dw${d}`, iw % 8, 5 + Math.floor(iw / 8), 1, 1, (w, h) => text(String(d), w / 2, h / 2 + 4, 116, WH));
    cell(`dd${d}`, id % 8, 5 + Math.floor(id / 8), 1, 1, (w, h) => text(String(d), w / 2, h / 2 + 4, 116, DK));
  }
  return cv;
}

window.__out = (() => {
  const X = textures(), png = c => c.toDataURL("image/png");
  // plating() is seeded, so calling it again gives the same canvases; wear's bump is not kept in TEX.
  const W = plating(63, { base: "#ddd6d3", light: true });
  const L = plating(71, { base: "#c3c7cd", panels: true, light: true });
  const flat = (S, ao, rough, metal) => { const c = document.createElement("canvas"); c.width = c.height = S; const g = c.getContext("2d"); g.fillStyle = `rgb(${ao * 255},${rough * 255},${metal * 255})`; g.fillRect(0, 0, S, S); return c; };
  return {
    armor: png(X.armor), armor_n: png(normalOf(X.armorBump, 2.5)), armor_orm: png(ormOf(X.armor, X.armorBump, { base: "#4d4042", rough: 0.55, metal0: 0 })),
    armorB: png(X.armorB), armorB_n: png(normalOf(X.armorBBump, 2.5)), armorB_orm: png(ormOf(X.armorB, X.armorBBump, { base: "#453739", rough: 0.6, metal0: 0 })),
    frame: png(X.frame), frame_n: png(normalOf(X.frameBump, 2.0)), frame_orm: png(ormOf(X.frame, X.frameBump, { base: "#38363a", rough: 0.42, metal0: 0.85 })),
    // Crab Breeze's one addition: a light panelled plate for white liveries, the same plating() call with a pale base.
    armorL: png(L.map), armorL_n: png(normalOf(L.bump, 2.5)), armorL_orm: png(ormOf(L.map, L.bump, { base: "#c3c7cd", rough: 0.5, metal0: 0, light: true })),
    wear: png(X.wear), wear_n: png(normalOf(W.bump, 2.0)), wear_orm: png(ormOf(W.map, W.bump, { base: "#ddd6d3", rough: 0.55, metal0: 0, light: true })),
    hazard: png(X.hazard), hazard_orm: png(flat(128, 1, 0.6, 0)),
    // Crab Breeze: the modern set, neutral so a palette tints it (plate and plateB the armour, frameM the structure)
    ...(() => {
      const A = modern(101, { base: "#dcdcdc" }), B = modern(131, { base: "#d2d2d2" });
      const F = modern(151, { base: "#4a4b4f", chip: "rgba(150,150,155,0.5)", words: ["HYD", "03", "SERVICE", "LUBE 4", "B-07"] });
      return {
        plate: png(A.map), plate_n: png(normalOf(A.bump, 2.0)), plate_orm: png(ormOf(A.map, A.bump, { base: "#dcdcdc", rough: 0.42, metal0: 0 })),
        plateB: png(B.map), plateB_n: png(normalOf(B.bump, 2.0)), plateB_orm: png(ormOf(B.map, B.bump, { base: "#d2d2d2", rough: 0.5, metal0: 0 })),
        frameM: png(F.map), frameM_n: png(normalOf(F.bump, 2.0)), frameM_orm: png(ormOf(F.map, F.bump, { base: "#4a4b4f", rough: 0.4, metal0: 0.7 })),
        // the crab air fortress: panelling laid out by shape, each with normal, ORM, emission and a livery mask
        ...(() => { const out = {};
          for (const [name, K, base] of [["carapace", carapacePanels(301, { base: "#d2d2d2" }), "#d2d2d2"], ["drum", drumPanels(311, { base: "#d0d0d0" }), "#d0d0d0"], ["slab", slabPanels(321, { base: "#d0d0d0" }), "#d0d0d0"]]) {
            out[name] = png(K.col); out[name + "_n"] = png(normalOf(K.bmp, 3.0)); out[name + "_orm"] = png(ormOf(K.col, K.bmp, { base, rough: 0.44, metal0: 0 }));
            out[name + "_e"] = png(K.emi); out[name + "_m"] = png(K.liv); }
          return out; })(),
        decals: png(decalSheet()),
      };
    })(),
  };
})();
window.__decals = DECALS;
