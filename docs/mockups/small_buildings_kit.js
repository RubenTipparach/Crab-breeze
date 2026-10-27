// Houses and small businesses for openspec/changes/small-buildings: sixteen building types built
// from one table of parts (walls with real openings, frames, sills and glass, roofs, porches,
// interiors, signs, cars). Metres throughout; each building's front faces +z, centred on x = 0,
// standing on y = 0. Used by docs/mockups/small_buildings.html and testable headless in node.
// Geometry is gathered per material with world-metre UVs (a texture tiles at true scale) and a
// vertex colour tint, so one material serves every building and the page has few draws.
(function (root) {
  'use strict';

  function mulberry(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) | 0;
      let t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  // Interior finishes: muted 1980s wall paints and a warm grey ceiling, never pure white (owner, 2026-10-27: "Pure
  // white not good for interior. Hurts my eyes"). A house picks its wall paint; its partitions and lining share it.
  const INSIDE = [0xb9ad98, 0xa8b09a, 0x9fadb4, 0xbfa88e, 0xb3a6a0, 0xc2b48f];
  const CEILING = 0xb8b0a2;
  const hex = (h) => [((h >> 16) & 255) / 255, ((h >> 8) & 255) / 255, (h & 255) / 255];

  // ------------------------------------------------------------------ the geometry gatherer
  class Builder {
    constructor() { this.parts = {}; this.tint = [1, 1, 1]; this.count = 0; this.pid = 0; this.cur = -1; this.meta = []; this.tags = []; this.volumes = []; this.boxes = []; this.nonBox = []; this.inBox = false; this.inside = INSIDE[0]; this.ceiling = CEILING; }
    color(h) { this.tint = typeof h === 'number' ? hex(h) : h; return this; }
    arr(mat) { return this.parts[mat] || (this.parts[mat] = { p: [], n: [], u: [], c: [], t: [] }); }
    // Pieces: what breaks off together (a wall segment, a window, a roof plane, a sofa, a car).
    // Every top-level box, cylinder or polygon is its own piece unless drawn inside group().
    group(fn) {
      if (this.cur >= 0) return fn();
      this.cur = this.pid++;
      try { return fn(); } finally { this.cur = -1; }
    }
    // A piece of its own even inside a group (a door's leaf inside its opening), with a tag and meta.
    own(tag, meta, fn) {
      const save = this.cur;
      this.cur = this.pid++;
      this.tags[this.cur] = tag;
      if (meta) this.meta[this.cur] = meta;
      try { return fn(); } finally { this.cur = save; }
    }
    // A planar convex polygon, wound to face `hint`; UVs in metres on the plane (u level, v up the slope).
    poly(mat, pts, hint, uvFn) {
      const a = pts[0], b = pts[1], c = pts[2];
      let n = cross(sub(b, a), sub(c, a));
      if (len(n) < 1e-9) { // degenerate first corner: find another
        for (let i = 2; i < pts.length && len(n) < 1e-9; i++) n = cross(sub(pts[1], pts[0]), sub(pts[i], pts[0]));
      }
      n = norm(n);
      if (hint && dot(n, hint) < 0) { pts = pts.slice().reverse(); n = [-n[0], -n[1], -n[2]]; }
      let t = cross([0, 1, 0], n);
      if (len(t) < 1e-6) t = [1, 0, 0]; else t = norm(t);
      const bt = cross(n, t);
      const A = this.arr(mat);
      const piece = this.cur >= 0 ? this.cur : this.pid++;
      if (!this.inBox) this.nonBox[piece] = true;
      const uv = (p) => uvFn ? uvFn(p) : [dot(p, t), dot(p, bt)];
      for (let i = 1; i + 1 < pts.length; i++) {
        for (const p of [pts[0], pts[i], pts[i + 1]]) {
          A.p.push(p[0], p[1], p[2]); A.n.push(n[0], n[1], n[2]);
          const q = uv(p); A.u.push(q[0], q[1]); A.c.push(this.tint[0], this.tint[1], this.tint[2]);
        }
        A.t.push(piece);
        this.count++;
      }
    }
    // An axis-aligned box. skip: a string of faces to leave out (x X y Y z Z).
    box(mat, x0, y0, z0, x1, y1, z1, skip) {
      if (x0 > x1) [x0, x1] = [x1, x0];
      if (y0 > y1) [y0, y1] = [y1, y0];
      if (z0 > z1) [z0, z1] = [z1, z0];
      if (x1 - x0 < 1e-4 || y1 - y0 < 1e-4 || z1 - z0 < 1e-4) return;
      if (this.cur < 0) { // a lone box: remember it so it can be broken into chunks
        return this.group(() => { this.meta[this.cur] = { mat, box: [x0, y0, z0, x1, y1, z1], tint: this.tint.slice() }; this.box(mat, x0, y0, z0, x1, y1, z1, skip); });
      }
      // Every box a piece is made of, for the game's colliders (a door's frame is three boxes round a hole).
      (this.boxes[this.cur] || (this.boxes[this.cur] = [])).push([x0, y0, z0, x1, y1, z1]);
      const s = skip || '';
      const uvX = (p) => [p[2], p[1]], uvZ = (p) => [p[0], p[1]], uvY = (p) => [p[0], p[2]];
      this.inBox = true;
      if (!s.includes('x')) this.poly(mat, [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], [-1, 0, 0], uvX);
      if (!s.includes('X')) this.poly(mat, [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]], [1, 0, 0], uvX);
      if (!s.includes('y')) this.poly(mat, [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], [0, -1, 0], uvY);
      if (!s.includes('Y')) this.poly(mat, [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]], [0, 1, 0], uvY);
      if (!s.includes('z')) this.poly(mat, [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]], [0, 0, -1], uvZ);
      if (!s.includes('Z')) this.poly(mat, [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], [0, 0, 1], uvZ);
      this.inBox = false;
    }
    // A cylinder along an axis, centred at c, from h0 to h1 along that axis.
    cyl(mat, axis, c, r, h0, h1, seg, caps = true) {
      if (this.cur < 0) return this.group(() => this.cyl(mat, axis, c, r, h0, h1, seg, caps));
      const P = (ang, h) => {
        const a = Math.cos(ang) * r, b = Math.sin(ang) * r;
        return axis === 'y' ? [c[0] + a, h, c[2] + b] : axis === 'x' ? [h, c[1] + a, c[2] + b] : [c[0] + a, c[1] + b, h];
      };
      const AX = axis === 'y' ? [0, 1, 0] : axis === 'x' ? [1, 0, 0] : [0, 0, 1];
      for (let i = 0; i < seg; i++) {
        const a0 = (i / seg) * Math.PI * 2, a1 = ((i + 1) / seg) * Math.PI * 2, am = (a0 + a1) / 2;
        const mid = P(am, (h0 + h1) / 2), ctr = axis === 'y' ? [c[0], mid[1], c[2]] : axis === 'x' ? [mid[0], c[1], c[2]] : [c[0], c[1], mid[2]];
        this.poly(mat, [P(a0, h0), P(a1, h0), P(a1, h1), P(a0, h1)], sub(mid, ctr), (p) => [am * r, dot(p, AX)]);
      }
      if (caps) {
        const ring0 = [], ring1 = [];
        for (let i = 0; i < seg; i++) { const a = (i / seg) * Math.PI * 2; ring0.push(P(a, h0)); ring1.push(P(a, h1)); }
        this.poly(mat, ring0, AX.map((v) => -v));
        this.poly(mat, ring1, AX);
      }
    }
    // A face with 0..1 UVs (signs), facing +z, -z, +x or -x.
    sign(mat, cx, cy, cz, w, h, facing) {
      const hw = w / 2, hh = h / 2;
      let pts, hint;
      if (facing === '+z') { pts = [[cx - hw, cy - hh, cz], [cx + hw, cy - hh, cz], [cx + hw, cy + hh, cz], [cx - hw, cy + hh, cz]]; hint = [0, 0, 1]; }
      else if (facing === '-z') { pts = [[cx + hw, cy - hh, cz], [cx - hw, cy - hh, cz], [cx - hw, cy + hh, cz], [cx + hw, cy + hh, cz]]; hint = [0, 0, -1]; }
      else if (facing === '+x') { pts = [[cx, cy - hh, cz + hw], [cx, cy - hh, cz - hw], [cx, cy + hh, cz - hw], [cx, cy + hh, cz + hw]]; hint = [1, 0, 0]; }
      else { pts = [[cx, cy - hh, cz - hw], [cx, cy - hh, cz + hw], [cx, cy + hh, cz + hw], [cx, cy + hh, cz - hw]]; hint = [-1, 0, 0]; }
      const uvs = [[0, 0], [1, 0], [1, 1], [0, 1]];
      const map = new Map(pts.map((p, i) => [p.join(','), uvs[i]]));
      this.poly(mat, pts, hint, (p) => map.get(p.join(',')) || [0, 0]);
    }
  }
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const len = (a) => Math.hypot(a[0], a[1], a[2]);
  const norm = (a) => { const l = len(a) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };

  // ------------------------------------------------------------------ walls with real openings
  // A wall along `axis` ('x' runs along x at z = c; 'z' runs along z at x = c), from s0 to s1, of
  // thickness t, between y0 and y1. `out` is +1 when the exterior is on the + side. Openings
  // {a, b, y0, y1, kind} are holes; each is then filled by its kind (window, door, shop front...).
  function wall(B, mat, axis, c, s0, s1, t, y0, y1, out, openings, o = {}) {
    const bx = (sa, sb, ya, yb, da, db, m = mat) => {
      if (axis === 'x') B.box(m, sa, ya, c + da, sb, yb, c + db);
      else B.box(m, c + da, ya, sa, c + db, yb, sb);
    };
    const ops = (openings || []).slice().sort((p, q) => p.a - q.a);
    let s = s0;
    for (const op of ops) {
      if (op.a > s) bx(s, op.a, y0, y1, -t / 2, t / 2);
      if (op.y0 > y0) bx(op.a, op.b, y0, op.y0, -t / 2, t / 2);
      if (op.y1 < y1) bx(op.a, op.b, op.y1, y1, -t / 2, t / 2);
      s = Math.max(s, op.b);
    }
    if (s < s1) bx(s, s1, y0, y1, -t / 2, t / 2);
    const inner = o.inner;
    for (const op of ops) fill(B, bx, Object.assign({ axis, line: c }, op), t, out, o);
    // Interior face of plaster, a hair inside the structural wall (the wall box's own inner face is hidden).
    if (inner) {
      const d = -out * (t / 2 + 0.012);
      let s2 = s0 + (o.innerPad || 0);
      const e = s1 - (o.innerPad || 0);
      const save = B.tint;
      B.color(B.inside);
      const strip = (sa, sb, ya, yb) => { if (sb - sa > 1e-3 && yb - ya > 1e-3) bx(sa, sb, ya, yb, Math.min(d, d + out * 0.01), Math.max(d, d + out * 0.01), inner); };
      for (const op of ops) {
        if (op.a > s2) strip(s2, Math.min(op.a, e), y0, y1);
        if (op.y0 > y0) strip(Math.max(op.a, s2), Math.min(op.b, e), y0, op.y0);
        if (op.y1 < y1) strip(Math.max(op.a, s2), Math.min(op.b, e), op.y1, y1);
        s2 = Math.max(s2, op.b);
      }
      if (s2 < e) strip(s2, e, y0, y1);
      B.tint = save;
    }
  }
  // Where a leaf hangs, in the building's frame: its hinge on the jamb at s = s0 in the middle of the
  // wall's depth (offset m from the wall's line), shut towards s = s1, opening inwards.
  function leafOf(op, s0, s1, y0, y1, m, out) {
    const x = op.axis === 'x';
    return {
      hinge: x ? [s0, y0, op.line + m] : [op.line + m, y0, s0],
      shut: x ? [1, 0] : [0, 1],
      inward: x ? [0, -out] : [-out, 0],
      width_m: s1 - s0, height_m: y1 - y0,
    };
  }
  function fill(B, bx, op, t, out, o) { B.group(() => fillOpening(B, bx, op, t, out, o)); }
  function fillOpening(B, bx, op, t, out, o) {
    const { a, b, kind } = op;
    const y0 = op.y0, y1 = op.y1;
    const f = op.frame || 0.07;
    const ext = out * t / 2;                       // the exterior face
    const dIn = out * (t / 2 - 0.16), dOut = out * (t / 2 - 0.05);
    const lo = Math.min(dIn, dOut), hi = Math.max(dIn, dOut);
    const frameMat = op.frameMat || 'trim';
    const tintSave = B.tint;
    const glass = (sa, sb, ya, yb, gm = 'glass') => { const m = (lo + hi) / 2; bx(sa, sb, ya, yb, m - 0.006, m + 0.006, gm); };
    if (kind === 'gap') return;
    if (op.frameTint) B.color(op.frameTint);
    if (kind === 'win' || kind === 'pic' || kind === 'ribbon') {
      bx(a, a + f, y0, y1, lo, hi, frameMat); bx(b - f, b, y0, y1, lo, hi, frameMat);
      bx(a + f, b - f, y0, y0 + f, lo, hi, frameMat); bx(a + f, b - f, y1 - f, y1, lo, hi, frameMat);
      glass(a + f, b - f, y0 + f, y1 - f);
      const lights = op.lights || (kind === 'pic' ? 3 : kind === 'ribbon' ? 1 : 2);
      // Muntins and the rail run 1 cm into the frame, so their ends never share a plane with the glass's edges.
      for (let i = 1; i < lights; i++) { const x = a + (b - a) * i / lights; bx(x - 0.025, x + 0.025, y0 + f - 0.01, y1 - f + 0.01, lo + 0.02, hi - 0.02, frameMat); }
      if (kind === 'win' && op.rail !== false) { const y = (y0 + y1) / 2; bx(a + f - 0.01, b - f + 0.01, y - 0.025, y + 0.025, lo + 0.02, hi - 0.02, frameMat); }
      // Sill, proud of the face.
      if (op.sill !== false) {
        B.color(op.sillTint || 0xd9d4c8);
        // It stands out from the wall's face (not into the wall), so its top never shares a plane with the wall below.
        const s0 = Math.min(ext, ext + out * 0.06), s1 = Math.max(ext, ext + out * 0.06);
        bx(a - 0.06, b + 0.06, y0 - 0.07, y0, s0, s1, op.sillMat || 'concrete');
      }
      if (op.lintel) {
        B.color(op.lintelTint || 0xffffff);
        // Proud of the wall's face, so its underside never shares a plane with the wall's head over the opening.
        const l0 = Math.min(ext, ext + out * 0.025), l1 = Math.max(ext, ext + out * 0.025);
        bx(a - 0.1, b + 0.1, y1, y1 + 0.2, l0, l1, op.lintel);
      }
      if (op.shutters) {
        B.color(op.shutters);
        const s0 = Math.min(ext + out * 0.008, ext + out * 0.045), s1 = Math.max(ext + out * 0.008, ext + out * 0.045);
        const w = (b - a) * 0.5;
        bx(a - w - 0.04, a - 0.04, y0, y1, s0, s1, 'shutter'); bx(b + 0.04, b + w + 0.04, y0, y1, s0, s1, 'shutter');
      }
      if (op.ac) { // a window air conditioner, 1980s
        B.color(0xcfcac0);
        const c0 = Math.min(ext - out * 0.1, ext + out * 0.4), c1 = Math.max(ext - out * 0.1, ext + out * 0.4);
        const m = (a + b) / 2; bx(m - 0.33, m + 0.33, y0 + 0.02, y0 + 0.42, c0, c1, 'metal');
      }
    } else if (kind === 'door') {
      bx(a, a + f, y0, y1, lo, hi, frameMat); bx(b - f, b, y0, y1, lo, hi, frameMat); bx(a + f, b - f, y1 - f, y1, lo, hi, frameMat);
      const m = (lo + hi) / 2;
      // The leaf (slab, lite and knob) is a piece of its own, hung on the jamb at a + f, so the game
      // can swing it open (design section 7.2).
      B.own('leaf', { leaf: leafOf(op, a + f, b - f, y0, y1 - f, m, out) }, () => {
        B.color(op.doorTint || 0x6b3b22);
        bx(a + f, b - f, y0, y1 - f, m - 0.025, m + 0.025, op.doorMat || 'door');
        if (op.lite) glass(a + f + 0.18, b - f - 0.18, y1 - 0.75, y1 - f - 0.15);
        B.color(0xc8b26a);
        const k = out > 0 ? hi : lo;
        bx(b - f - 0.14, b - f - 0.08, y0 + 0.95, y0 + 1.05, k - 0.04 * out, k + 0.04 * out, 'metal');
      });
      if (op.stoop !== false) {
        B.color(0xbab4aa);
        const s0 = Math.min(ext, ext + out * 1.1), s1 = Math.max(ext, ext + out * 1.1);
        bx(a - 0.4, b + 0.4, 0, y0, s0, s1, 'concrete');
      }
    } else if (kind === 'shop') {
      const fm = 'metal';
      B.color(op.frameTint || 0xb8bcc0);
      bx(a, a + 0.06, y0, y1, lo, hi, fm); bx(b - 0.06, b, y0, y1, lo, hi, fm);
      const n = Math.max(2, Math.round((b - a) / 1.6));
      const tr = y1 - 0.65;
      // A glazed door in one bay: the kick plate and the lower glass stop at its frame.
      const dx = op.door != null ? a + (b - a) * (op.door + 0.5) / n : null;
      const lower = dx == null ? [[a, b]] : [[a, dx - 0.48], [dx + 0.48, b]];
      for (const [p0, p1] of lower) bx(p0, p1, y0, y0 + 0.45, lo, hi, op.kick || 'metal');
      bx(a, b, y1 - 0.06, y1, lo, hi, fm);
      bx(a, b, tr - 0.05, tr + 0.05, lo, hi, fm);
      for (let i = 1; i < n; i++) { const x = a + (b - a) * i / n; if (dx == null || Math.abs(x - dx) > 0.5) bx(x - 0.03, x + 0.03, y0 + 0.45, y1, lo, hi, fm); }
      for (const [p0, p1] of lower) glass(Math.max(p0, a + 0.06), Math.min(p1, b - 0.06), y0 + 0.45, tr - 0.05, 'glassShop');
      glass(a + 0.06, b - 0.06, tr + 0.05, y1 - 0.06, 'glassShop');
      if (dx != null) {
        const x = dx;
        B.color(0x9aa0a6); bx(x - 0.48, x - 0.42, y0, tr, lo - 0.01, hi + 0.01, fm); bx(x + 0.42, x + 0.48, y0, tr, lo - 0.01, hi + 0.01, fm);
        // The leaf: an aluminium frame round a pane, hung on its left stile.
        B.own('leaf', { leaf: leafOf(op, x - 0.42, x + 0.42, y0, tr - 0.05, (lo + hi) / 2, out) }, () => {
          B.color(0x9aa0a6);
          bx(x - 0.42, x + 0.42, y0, y0 + 0.2, lo + 0.02, hi - 0.02, fm);
          bx(x - 0.42, x + 0.42, tr - 0.12, tr - 0.05, lo + 0.02, hi - 0.02, fm);
          bx(x - 0.42, x - 0.36, y0 + 0.2, tr - 0.12, lo + 0.02, hi - 0.02, fm);
          bx(x + 0.36, x + 0.42, y0 + 0.2, tr - 0.12, lo + 0.02, hi - 0.02, fm);
          bx(x + 0.22, x + 0.3, y0 + 0.95, y0 + 1.05, lo - 0.03, hi + 0.03, fm);
          glass(x - 0.36, x + 0.36, y0 + 0.2, tr - 0.12, 'glassShop');
        });
      }
    } else if (kind === 'garage' || kind === 'roll') {
      bx(a, a + f, y0, y1, lo, hi, frameMat); bx(b - f, b, y0, y1, lo, hi, frameMat); bx(a, b, y1 - f, y1, lo, hi, frameMat);
      const m = (lo + hi) / 2;
      B.color(op.doorTint || 0xe9e6de);
      if (kind === 'garage') {
        const panels = 4, h = (y1 - f - y0) / panels;
        for (let i = 0; i < panels; i++) bx(a + f, b - f, y0 + i * h + 0.015, y0 + (i + 1) * h - 0.015, m - 0.025, m + 0.025, 'door');
        bx(a + f, b - f, y0, y1 - f, m - 0.035, m - 0.03, 'door'); // the panels' backing, no gaps to see through
      } else {
        bx(a + f, b - f, y0, y1 - f, m - 0.02, m + 0.02, 'corrugated');
        B.color(0x8d9196);
        const h0 = Math.min(ext, ext + out * 0.35), h1 = Math.max(ext, ext + out * 0.35);
        bx(a - 0.1, b + 0.1, y1, y1 + 0.5, h0, h1, 'metal');
      }
    }
    B.tint = tintSave;
  }
  // Openings spread evenly along s0..s1 from a row of specs [kind, width, height, sill, extra].
  function row(s0, s1, base, specs) {
    const total = specs.reduce((v, s) => v + s[1], 0);
    const gap = (s1 - s0 - total) / (specs.length + 1);
    let x = s0 + gap;
    return specs.map((s) => {
      const op = Object.assign({ kind: s[0], a: x, b: x + s[1], y0: base + (s[3] || 0), y1: base + (s[3] || 0) + s[2] }, s[4] || {});
      x += s[1] + gap;
      return op;
    });
  }

  // ------------------------------------------------------------------ roofs
  // Each roof is a top surface and an underside (seen under the eaves), with fascia boards.
  // A roof plane, cut into panels about 1.6 m across, each its own piece (so a shell holes a roof
  // and a lost wall peels one side off, panel by panel). pts: a quad or a triangle, a-b one edge,
  // d-c the opposite edge (c = d for a triangle).
  const PANEL_M = 1.6;
  function slope(B, mat, pts, under = 'soffit', thick = 0.14) {
    const [a, b, c] = pts, d = pts[3] || pts[2];
    const L = (p, q) => Math.hypot(p[0] - q[0], p[1] - q[1], p[2] - q[2]);
    const nu = Math.max(1, Math.round(Math.max(L(a, b), L(d, c)) / PANEL_M)), nv = Math.max(1, Math.round(Math.max(L(a, d), L(b, c)) / PANEL_M));
    const lerp = (p, q, t) => [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, p[2] + (q[2] - p[2]) * t];
    const at = (u, v) => lerp(lerp(a, b, u), lerp(d, c, u), v);
    for (let i = 0; i < nu; i++) for (let j = 0; j < nv; j++) {
      const p00 = at(i / nu, j / nv), p10 = at((i + 1) / nu, j / nv), p11 = at((i + 1) / nu, (j + 1) / nv), p01 = at(i / nu, (j + 1) / nv);
      const cell = [p00, p10, p11, p01].filter((p, k, arr) => k === 0 || L(p, arr[k - 1]) > 1e-6);
      if (cell.length < 3 || (cell.length === 4 && L(cell[3], cell[0]) < 1e-6)) continue;
      B.group(() => {
        slopeFaces(B, mat, cell, under, thick);
        // The panel as an oriented box, so it falls as an instanced chunk of its material.
        const ctr = [0, 1, 2].map((k) => cell.reduce((s, p) => s + p[k], 0) / cell.length);
        const tu = sub(p10, p00), tv = sub(p01, p00);
        let n = norm(cross(tu, tv)); if (n[1] < 0) n = n.map((x) => -x);
        const tuN = len(tu) > 1e-6 ? norm(tu) : norm(cross(n, tv));
        const tvN = norm(cross(n, tuN));
        B.meta[B.cur] = { mat, tint: B.tint.slice(), panel: { c: [ctr[0], ctr[1] - thick / 2, ctr[2]], size: [Math.max(len(tu), L(p11, p01), 0.2), thick, Math.max(len(tv), L(p11, p10), 0.2)], axes: [tuN, n, tvN] } };
      });
    }
  }
  function slopeFaces(B, mat, pts, under, thick) {
    const up = [0, 1, 0];
    B.poly(mat, pts, up);
    const low = pts.map((p) => [p[0], p[1] - thick, p[2]]);
    const st = B.tint; B.color(0xf2efe8);
    B.poly(under, low, [0, -1, 0]);
    B.tint = st;
  }
  // A big horizontal slab (flat roof, ceiling, upper floor) as tiles about 2.5 m across.
  function tiles(B, mat, x0, y0, z0, x1, y1, z1, skip, size = 2.5) {
    const nx = Math.max(1, Math.round((x1 - x0) / size)), nz = Math.max(1, Math.round((z1 - z0) / size));
    for (let i = 0; i < nx; i++) for (let k = 0; k < nz; k++) B.box(mat, x0 + (x1 - x0) * i / nx, y0, z0 + (z1 - z0) * k / nz, x0 + (x1 - x0) * (i + 1) / nx, y1, z0 + (z1 - z0) * (k + 1) / nz, skip);
  }
  // R: {x0, x1, z0, z1, y, pitch, over, mat, tint, wall (gable end material), wallTint, t}
  function hipRoof(B, R) {
    const X0 = R.x0 - R.over, X1 = R.x1 + R.over, Z0 = R.z0 - R.over, Z1 = R.z1 + R.over, y = R.y;
    B.color(R.tint || 0x4a4744);
    const W = X1 - X0, D = Z1 - Z0;
    if (W >= D) {
      const h = D / 2, top = y + h * R.pitch, mz = (Z0 + Z1) / 2;
      slope(B, R.mat, [[X0, y, Z1], [X1, y, Z1], [X1 - h, top, mz], [X0 + h, top, mz]]);
      slope(B, R.mat, [[X1, y, Z0], [X0, y, Z0], [X0 + h, top, mz], [X1 - h, top, mz]]);
      slope(B, R.mat, [[X0, y, Z0], [X0, y, Z1], [X0 + h, top, mz]]);
      slope(B, R.mat, [[X1, y, Z1], [X1, y, Z0], [X1 - h, top, mz]]);
    } else {
      const h = W / 2, top = y + h * R.pitch, mx = (X0 + X1) / 2;
      slope(B, R.mat, [[X1, y, Z1], [X1, y, Z0], [mx, top, Z0 + h], [mx, top, Z1 - h]]);
      slope(B, R.mat, [[X0, y, Z0], [X0, y, Z1], [mx, top, Z1 - h], [mx, top, Z0 + h]]);
      slope(B, R.mat, [[X0, y, Z1], [X1, y, Z1], [mx, top, Z1 - h]]);
      slope(B, R.mat, [[X1, y, Z0], [X0, y, Z0], [mx, top, Z0 + h]]);
    }
    fascia(B, X0, X1, Z0, Z1, y, R.fasciaTint);
  }
  function fascia(B, X0, X1, Z0, Z1, y, tint) {
    B.color(tint || 0xf4f1ea);
    // Foot and head 6 mm off the eave's line, so they never share a plane with a ceiling's underside or a wall's head.
    const d = 0.04, h = 0.214, e = y - 0.006;
    B.box('trim', X0 - d, y - h, Z1, X1 + d, e, Z1 + d);
    B.box('trim', X0 - d, y - h, Z0 - d, X1 + d, e, Z0);
    B.box('trim', X0 - d, y - h, Z0, X0, e, Z1);
    B.box('trim', X1, y - h, Z0, X1 + d, e, Z1);
  }
  // A gable roof with its ridge along x (or along z when R.ridge === 'z'), and its gable-end walls.
  function gableRoof(B, R) {
    const alongZ = R.ridge === 'z';
    const P = (u, y, v) => alongZ ? [v, y, u] : [u, y, v];
    const u0 = alongZ ? R.z0 : R.x0, u1 = alongZ ? R.z1 : R.x1, v0 = alongZ ? R.x0 : R.z0, v1 = alongZ ? R.x1 : R.z1;
    const ou = R.overEnd != null ? R.overEnd : R.over, ov = R.over;
    const U0 = u0 - ou, U1 = u1 + ou, V0 = v0 - ov, V1 = v1 + ov, y = R.y;
    const h = (V1 - V0) / 2, top = y + h * R.pitch, mv = (V0 + V1) / 2;
    B.color(R.tint || 0x4a4744);
    slope(B, R.mat, [P(U0, y, V1), P(U1, y, V1), P(U1, top, mv), P(U0, top, mv)]);
    slope(B, R.mat, [P(U1, y, V0), P(U0, y, V0), P(U0, top, mv), P(U1, top, mv)]);
    // Ridge cap.
    B.color(R.tint || 0x4a4744);
    const rc = 0.12;
    const r0 = P(U0, top - 0.02, mv - rc), r1 = P(U1, top + 0.06, mv + rc);
    B.box(R.mat, Math.min(r0[0], r1[0]), r0[1], Math.min(r0[2], r1[2]), Math.max(r0[0], r1[0]), r1[1], Math.max(r0[2], r1[2]));
    // Gable ends: a pentagon on each end wall, outside and inside.
    if (R.wall) {
      const t = R.t || 0.25;
      const yWall = y + ov * R.pitch - 0.14;
      for (const [u, outward] of [[u0, -1], [u1, 1]]) {
        for (const side of [0, 1]) {
          const uu = side === 0 ? u : u - outward * t;
          const ptsG = [P(uu, R.yWall, v0), P(uu, R.yWall, v1), P(uu, yWall, v1), P(uu, top - 0.14, mv), P(uu, yWall, v0)];
          const hint = alongZ ? [0, 0, outward * (side === 0 ? 1 : -1)] : [outward * (side === 0 ? 1 : -1), 0, 0];
          if (side === 0) B.color(R.wallTint || 0xffffff); else B.color(B.inside);
          B.poly(side === 0 ? R.wall : 'plaster', ptsG, hint);
        }
      }
    }
    // Barge boards along the rakes, fascia along the eaves.
    B.color(R.fasciaTint || 0xf4f1ea);
    if (alongZ) {
      B.box('trim', V0 - 0.04, y - 0.214, U1, V1 + 0.04, y - 0.006, U1 + 0.04);
      B.box('trim', V0 - 0.04, y - 0.214, U0 - 0.04, V1 + 0.04, y - 0.006, U0);
      B.box('trim', V1, y - 0.214, U0, V1 + 0.04, y - 0.006, U1);
      B.box('trim', V0 - 0.04, y - 0.214, U0, V0, y - 0.006, U1);
    } else {
      B.box('trim', U0, y - 0.214, V1, U1, y - 0.006, V1 + 0.04);
      B.box('trim', U0, y - 0.214, V0 - 0.04, U1, y - 0.006, V0);
    }
    return top;
  }
  // A flat roof with a parapet and coping.
  function flatRoof(B, x0, x1, z0, z1, y, o = {}) {
    const par = o.parapet != null ? o.parapet : 0.7, t = o.t || 0.25;
    // The roofing stands 6 mm over the walls' heads, so it never shares a plane with the top of a wall's inner finish.
    B.color(0x6d6a66); tiles(B, 'roofing', x0 + t, y - 0.3, z0 + t, x1 - t, y + 0.006, z1 - t);
    if (par > 0) {
      B.color(o.wallTint || 0xffffff);
      B.box(o.wall, x0, y, z1 - t, x1, y + par, z1);
      B.box(o.wall, x0, y, z0, x1, y + par, z0 + t);
      B.box(o.wall, x0, y, z0 + t, x0 + t, y + par, z1 - t);
      B.box(o.wall, x1 - t, y, z0 + t, x1, y + par, z1 - t);
      B.color(o.copeTint || 0xd8d2c6);
      B.box(o.cope || 'concrete', x0 - 0.05, y + par, z0 - 0.05, x1 + 0.05, y + par + 0.08, z1 + 0.05);
    }
    if (o.ac) { B.color(0xb9bcbe); B.box('metal', o.ac[0] - 0.9, y, o.ac[1] - 0.6, o.ac[0] + 0.9, y + 1.1, o.ac[1] + 0.6); B.color(0x55585a); B.box('metal', o.ac[0] - 0.5, y + 1.1, o.ac[1] - 0.4, o.ac[0] + 0.5, y + 1.2, o.ac[1] + 0.4); }
  }

  // ------------------------------------------------------------------ furniture and life, local to a spot
  // Items are boxes in a local frame facing +z, placed at (x, y, z) turned by rot quarter turns.
  // tag 'furniture' marks interior dressing (dropped at LOD 1); 'dressing' stays outside.
  // An item from FURN (it carries .item and .tint) is recorded with where it stands, so the game can draw a
  // modelled piece of furniture in its place (design section 7.3).
  function place(B, x, y, z, rot, items, tag = 'furniture') {
    B.group(() => {
      B.tags[B.cur] = tag;
      if (items.item) B.meta[B.cur] = Object.assign(B.meta[B.cur] || {}, { furn: { item: items.item, at: [x, y, z], rot, tint: items.tint != null ? hex(items.tint) : [1, 1, 1] } });
      placeItems(B, x, y, z, rot, items);
    });
  }
  function placeItems(B, x, y, z, rot, items) {
    const T = (lx, lz) => rot === 1 ? [-lz, lx] : rot === 2 ? [-lx, -lz] : rot === 3 ? [lz, -lx] : [lx, lz];
    for (const it of items) {
      const [mat, ax, ay, az, bx, by, bz, tint] = it;
      const p = T(ax, az), q = T(bx, bz);
      B.color(tint != null ? tint : 0xffffff);
      B.box(mat, x + Math.min(p[0], q[0]), y + ay, z + Math.min(p[1], q[1]), x + Math.max(p[0], q[0]), y + by, z + Math.max(p[1], q[1]));
    }
  }
  const FURN = {
    sofa: (c) => [['fabric', -1.0, 0, -0.45, 1.0, 0.42, 0.45, c], ['fabric', -1.0, 0.42, -0.45, 1.0, 0.85, -0.22, c], ['fabric', -1.01, 0, -0.46, -0.82, 0.62, 0.46, c], ['fabric', 0.82, 0, -0.46, 1.01, 0.62, 0.46, c]],
    bed: (c) => [['wood', -0.8, 0, -1.0, 0.8, 0.42, 1.0, 0x7a5232], ['fabric', -0.78, 0.42, -0.98, 0.78, 0.6, 0.98, c], ['fabric', -0.6, 0.6, -0.95, 0.6, 0.72, -0.6, 0xf2f0ea], ['wood', -0.8, 0, -1.06, 0.8, 1.05, -1.0, 0x6a4428]],
    table: () => [['wood', -0.6, 0.72, -0.45, 0.6, 0.76, 0.45, 0x8a5a36], ['wood', -0.55, 0, -0.4, -0.5, 0.72, -0.35, 0x6a4428], ['wood', 0.5, 0, -0.4, 0.55, 0.72, -0.35, 0x6a4428], ['wood', -0.55, 0, 0.35, -0.5, 0.72, 0.4, 0x6a4428], ['wood', 0.5, 0, 0.35, 0.55, 0.72, 0.4, 0x6a4428],
      ['wood', -0.2, 0, -0.85, 0.2, 0.45, -0.55, 0x7a5232], ['wood', -0.2, 0.45, -0.88, 0.2, 0.95, -0.84, 0x7a5232], ['wood', -0.2, 0, 0.55, 0.2, 0.45, 0.85, 0x7a5232], ['wood', -0.2, 0.45, 0.84, 0.2, 0.95, 0.88, 0x7a5232]],
    kitchen: (c) => [['trim', -1.8, 0, -0.3, 1.8, 0.88, 0.3, c], ['counter', -1.82, 0.88, -0.32, 1.82, 0.92, 0.32, 0xe8e2d0], ['trim', -1.8, 1.45, -0.3, 1.0, 2.15, 0.02, c], ['metal', 1.1, 0, -0.33, 1.81, 1.8, 0.34, 0xd9d6cc], ['metal', -0.6, 0.92, -0.25, 0.0, 0.94, 0.15, 0x9fa3a6]],
    tv: () => [['wood', -0.55, 0, -0.25, 0.55, 0.55, 0.25, 0x5a3a22], ['metal', -0.38, 0.55, -0.25, 0.38, 1.08, 0.2, 0x2a2a2a], ['glassDark', -0.3, 0.62, 0.2, 0.3, 1.0, 0.205, 0x3a4a52]],
    shelf: () => [['wood', -0.9, 0, -0.18, 0.9, 1.9, 0.18, 0x7a5232]],
    desk: () => [['wood', -0.75, 0.72, -0.4, 0.75, 0.76, 0.4, 0x6a4a30], ['wood', -0.75, 0, -0.4, -0.3, 0.72, 0.4, 0x5a3a22], ['fabric', -0.25, 0, 0.5, 0.25, 0.5, 0.95, 0x3a3a3a], ['metal', -0.2, 0.76, -0.3, 0.25, 1.15, 0.05, 0xd8d0c0]],
    aisle: (rnd) => { const it = [['metal', -2, 0, -0.45, 2, 0.12, 0.45, 0xd8d8d8], ['metal', -1.98, 0, -0.05, 1.98, 1.6, 0.05, 0xd8d8d8]]; for (const lv of [0.15, 0.6, 1.05]) { it.push(['metal', -2, lv - 0.03, -0.45, 2, lv, 0.45, 0xcfcfcf]); for (let x = -1.95; x < 1.9; x += 0.32) { const c = [0xc23b30, 0xe8b23a, 0x3b67c4, 0x2f8a48, 0xf2f0ea, 0xe07a2a][Math.floor(rnd() * 6)]; it.push(['product', x, lv, -0.42, x + 0.28, lv + 0.25 + rnd() * 0.12, -0.08, c]); it.push(['product', x, lv, 0.08, x + 0.28, lv + 0.25 + rnd() * 0.12, 0.42, c]); } } return it; },
    cooler: () => [['metal', -1.5, 0, -0.4, 1.5, 2.1, 0.4, 0xe8e8e8], ['glassShop', -1.42, 0.25, 0.4, 1.42, 1.95, 0.41, 0xffffff], ['lamp', -1.4, 1.98, 0.3, 1.4, 2.02, 0.38, 0xffffff]],
    counter: (c) => [['trim', -1.5, 0, -0.35, 1.5, 1.0, 0.35, c], ['counter', -1.55, 1.0, -0.4, 1.55, 1.05, 0.4, 0xd8cdb0], ['metal', 0.4, 1.05, -0.2, 0.85, 1.35, 0.15, 0x3a3a3a]],
    booth: (c) => [['fabric', -0.9, 0, -1.05, 0.9, 0.45, -0.55, c], ['fabric', -0.9, 0.45, -1.05, 0.9, 1.1, -0.9, c], ['fabric', -0.9, 0, 0.55, 0.9, 0.45, 1.05, c], ['fabric', -0.9, 0.45, 0.9, 0.9, 1.1, 1.05, c], ['counter', -0.85, 0.72, -0.4, 0.85, 0.76, 0.4, 0xe8e2d0], ['metal', -0.06, 0, -0.06, 0.06, 0.72, 0.06, 0xb8bcc0]],
    stool: (c) => [['metal', -0.04, 0, -0.04, 0.04, 0.7, 0.04, 0xb8bcc0], ['fabric', -0.2, 0.7, -0.2, 0.2, 0.78, 0.2, c]],
    washer: () => [['metal', -0.34, 0, -0.34, 0.34, 0.9, 0.34, 0xf0eee8], ['glassDark', -0.2, 0.3, 0.34, 0.2, 0.7, 0.345, 0x4a5a62]],
    chair: (c) => [['fabric', -0.3, 0, -0.3, 0.3, 0.5, 0.3, c], ['fabric', -0.3, 0.5, -0.32, 0.3, 1.2, -0.22, c], ['glassDark', -0.4, 0.9, 0.5, 0.4, 0.92, 0.55, 0xeeeeee]],
    workbench: () => [['wood', -1.2, 0.85, -0.35, 1.2, 0.9, 0.35, 0x8a6a46], ['metal', -1.15, 0, -0.3, -1.05, 0.85, 0.3, 0x55585a], ['metal', 1.05, 0, -0.3, 1.15, 0.85, 0.3, 0x55585a], ['metal', -1.2, 0.9, -0.35, 1.2, 1.9, -0.3, 0x8d4a3a], ['metal', -0.9, 1.2, -0.3, -0.4, 1.5, -0.1, 0xc23b30]],
    lift: () => [['metal', -1.6, 0, -0.2, -1.3, 3.2, 0.2, 0x2f5f9a], ['metal', 1.3, 0, -0.2, 1.6, 3.2, 0.2, 0x2f5f9a], ['metal', -1.3, 1.6, -1.2, -0.6, 1.7, 1.2, 0x55585a], ['metal', 0.6, 1.6, -1.2, 1.3, 1.7, 1.2, 0x55585a]],
    tires: () => { const it = []; for (let i = 0; i < 4; i++) it.push(['rubber', -0.35, i * 0.22, -0.35, 0.35, i * 0.22 + 0.2, 0.35, 0x222222]); return it; },
    pump: () => [['concrete', -0.6, 0, -1.4, 0.6, 0.2, 1.4, 0xbab4aa], ['metal', -0.3, 0.2, -0.25, 0.3, 1.75, 0.25, 0xe8e6e0], ['metal', -0.31, 1.15, -0.26, 0.31, 1.55, 0.26, 0xc23b30], ['glassDark', -0.2, 1.2, 0.26, 0.2, 1.45, 0.27, 0x334048]],
  };
  // Each FURN item names itself and its tint on the boxes it returns (the pump is outdoor dressing, not furniture).
  for (const [key, f] of Object.entries(FURN)) {
    if (key === 'pump' || typeof f !== 'function') continue;
    FURN[key] = (arg) => { const it = f(arg); it.item = key; if (typeof arg === 'number') it.tint = arg; return it; };
  }
  // An 80s box sedan or pickup.
  function car(B, x, z, rot, tint, kind = 'sedan') { B.group(() => { carParts(B, x, z, rot, tint, kind); B.tags[B.cur] = 'car'; }); }
  function carParts(B, x, z, rot, tint, kind) {
    const L = kind === 'pickup' ? 2.6 : 2.35;
    const items = [['paint', -0.88, 0.32, -L, 0.88, 0.92, L, tint]];
    if (kind === 'pickup') items.push(['paint', -0.84, 0.92, -0.2, 0.84, 1.55, 1.0, tint], ['glassDark', -0.8, 1.0, 1.0, 0.8, 1.48, 1.02, 0x2a3a44], ['metal', -0.88, 0.92, -L, 0.88, 1.15, -L + 0.05, tint]);
    else items.push(['glassDark', -0.8, 0.92, -1.15, 0.8, 1.38, 0.95, 0x2a3a44], ['paint', -0.78, 1.38, -1.05, 0.78, 1.42, 0.85, tint]);
    items.push(['metal', -0.9, 0.3, L, 0.9, 0.45, L + 0.08, 0xb8bcc0], ['metal', -0.9, 0.3, -L - 0.08, 0.9, 0.45, -L, 0xb8bcc0], ['lamp', -0.75, 0.62, L, -0.45, 0.75, L + 0.01, 0xfff4d0], ['lamp', 0.45, 0.62, L, 0.75, 0.75, L + 0.01, 0xfff4d0]);
    place(B, x, 0, z, rot, items);
    B.color(0x1c1c1c);
    const T = (lx, lz) => rot === 1 ? [-lz, lx] : rot === 2 ? [-lx, -lz] : rot === 3 ? [lz, -lx] : [lx, lz];
    for (const [wx, wz] of [[-0.82, L - 0.65], [0.82, L - 0.65], [-0.82, -L + 0.65], [0.82, -L + 0.65]]) {
      const p = T(wx, wz);
      const ax = (rot % 2 === 0) ? 'x' : 'z';
      const c = [x + p[0], 0.33, z + p[1]];
      const h0 = (ax === 'x' ? c[0] : c[2]) - 0.11, h1 = h0 + 0.22;
      B.cyl('rubber', ax, c, 0.33, h0, h1, 10);
    }
  }
  function pilotFigure(B, x, z) { B.group(() => pilotParts(B, x, z)); }
  function pilotParts(B, x, z) { // the 1.8 m pilot for scale
    B.color(0x3d4a3a); B.box('fabric', x - 0.2, 0, z - 0.12, x - 0.03, 0.85, z + 0.12); B.box('fabric', x + 0.03, 0, z - 0.12, x + 0.2, 0.85, z + 0.12);
    B.color(0xd08a2a); B.box('fabric', x - 0.25, 0.85, z - 0.15, x + 0.25, 1.5, z + 0.15);
    B.color(0xd9b08c); B.box('fabric', x - 0.1, 1.52, z - 0.1, x + 0.1, 1.8, z + 0.1);
  }

  // ------------------------------------------------------------------ the generic house
  // o: {w, d, x, z, floors, fh, base, wall, tint, t, front: [[specs storey 0], [storey 1]],
  //     sideWin, backWin, roof: {type, pitch, over, mat, tint, ridge}, plan: [room kinds by width share],
  //     floorMat, chimney, porch, trim}
  function house(B, rnd, o) {
    const x0 = (o.x || 0) - o.w / 2, x1 = (o.x || 0) + o.w / 2, z0 = (o.z || 0) - o.d / 2, z1 = (o.z || 0) + o.d / 2;
    const t = o.t || 0.25, fh = o.fh || 2.75, base = o.base != null ? o.base : 0.45, floors = o.floors || 1;
    B.inside = o.inside || INSIDE[Math.floor(rnd() * INSIDE.length)];
    const top = base + floors * fh;
    B.color(0xbab4aa); B.box('concrete', x0, -0.3, z0, x1, base, z1);
    const winDefaults = { shutters: o.shutters, lintel: o.lintel, lintelTint: o.lintelTint, frameTint: o.trim, ac: false };
    let doorMid = null; // the front door's middle: a porch's steps line up with it
    for (let s = 0; s < floors; s++) {
      const yb = base + s * fh, yt = yb + fh;
      const deco = (ops) => ops.map((op) => Object.assign({}, op.kind === 'win' || op.kind === 'pic' ? winDefaults : {}, op));
      const frontSpecs = (o.front && o.front[s]) || [['win', 1.2, 1.4, 0.9]];
      const front = deco(row(x0 + 0.6, x1 - 0.6, yb, frontSpecs));
      const frontDoor = front.find((op) => op.kind === 'door');
      if (s === 0 && frontDoor) doorMid = (frontDoor.a + frontDoor.b) / 2;
      const back = deco(row(x0 + 0.6, x1 - 0.6, yb, (o.back && o.back[s]) || [['win', 1.0, 1.2, 0.95], ['door', 0.9, 2.1, 0, { lite: true, stoop: s === 0 }], ['win', 1.0, 1.2, 0.95]].slice(0, s === 0 ? 3 : 2).map((v) => v[0] === 'door' && s > 0 ? ['win', 1.0, 1.2, 0.95] : v)));
      const sw = o.sideWin != null ? o.sideWin : Math.max(1, Math.floor(o.d / 4));
      const side = deco(row(z0 + t + 0.5, z1 - t - 0.5, yb, Array.from({ length: sw }, () => ['win', 1.0, 1.25, 0.95])));
      B.color(o.tint || 0xffffff);
      const inner = 'plaster';
      wall(B, o.wall, 'x', z1 - t / 2, x0, x1, t, yb, yt, 1, front, { inner, innerPad: t });
      B.color(o.tint || 0xffffff);
      wall(B, o.wall, 'x', z0 + t / 2, x0, x1, t, yb, yt, -1, back, { inner, innerPad: t });
      B.color(o.tint || 0xffffff);
      wall(B, o.wall, 'z', x0 + t / 2, z0 + t, z1 - t, t, yb, yt, -1, o.blankLeft ? [] : side, { inner });
      B.color(o.tint || 0xffffff);
      wall(B, o.wall, 'z', x1 - t / 2, z0 + t, z1 - t, t, yb, yt, 1, o.blankRight ? [] : side, { inner });
      // Floor and ceiling, with the stairwell of the storey below cut out of them (so the stair can be climbed).
      const hole = s > 0 ? stairHole(o, s - 1, x0, x1, z0, t) : null;
      for (const [ax0, az0, ax1, az1] of rectMinus([x0 + t, z0 + t, x1 - t, z1 - t], hole)) {
        B.color(B.ceiling); tiles(B, 'plaster', ax0, yb - 0.22, az0, ax1, yb - 0.03, az1, s === 0 ? '' : 'Y');
      }
      for (const [ax0, az0, ax1, az1] of rectMinus([x0 + t + 0.012, z0 + t + 0.012, x1 - t - 0.012, z1 - t - 0.012], hole)) {
        // The finish stands 12 mm proud of the foundation's top, so the two never share a plane (no z-fighting).
        B.color(0xffffff); tiles(B, o.floorMat || 'floorWood', ax0, yb - 0.03, az0, ax1, yb + 0.012, az1);
      }
      // Partitions with door gaps, and the rooms' furniture.
      const plan = (o.plan && o.plan[s]) || ['living', 'kitchen', 'bed'];
      const shares = plan.map((r) => (typeof r === 'object' ? r.w : 1));
      const total = shares.reduce((a, b) => a + b, 0);
      let x = x0 + t;
      const inW = x1 - x0 - 2 * t;
      plan.forEach((r, i) => {
        const kind = typeof r === 'object' ? r.k : r;
        const w = inW * shares[i] / total;
        const rx0 = x, rx1 = x + w;
        if (i > 0) {
          B.color(B.inside);
          const gz = (z0 + z1) / 2 + (i % 2 ? 0.8 : -0.8);
          // A partition stops 1.3 m short of a front or back wall with a door near it, so the door opens
          // onto a hall rather than into the partition's end (the pilot walks in, design section 7.2).
          const near = (ops) => ops.some((op) => op.kind === 'door' && op.a - 0.4 < rx0 && op.b + 0.4 > rx0);
          // Its ends run 6 mm into the walls' plaster, so they never share a plane with the floor finish's edges.
          const pz0 = z0 + t + 0.006 + (near(back) ? 1.3 : 0), pz1 = z1 - t - 0.006 - (near(front) ? 1.3 : 0);
          wall(B, 'plaster', 'z', rx0, pz0, pz1, 0.11, yb, yt - 0.012, 1, [{ kind: 'gap', a: Math.max(pz0 + 0.1, gz - 0.45), b: Math.min(pz1 - 0.1, gz + 0.45), y0: yb, y1: yb + 2.1 }]);
        }
        furnishRoom(B, rnd, kind, rx0 + 0.15, rx1 - 0.15, z0 + t + 0.1, z1 - t - 0.1, yb, s, o);
        B.color(0xfff6e0); B.box('lamp', (rx0 + rx1) / 2 - 0.3, yt - 0.06, (z0 + z1) / 2 - 0.3, (rx0 + rx1) / 2 + 0.3, yt - 0.012, (z0 + z1) / 2 + 0.3);
        x = rx1;
      });
    }
    // Ceiling of the top storey.
    B.color(B.ceiling); tiles(B, 'plaster', x0 + t, top - 0.012, z0 + t, x1 - t, top + 0.08, z1 - t, 'Y');
    // Roof.
    const R = Object.assign({ x0, x1, z0, z1, y: top, pitch: 0.4, over: 0.55, mat: 'shingle', wall: o.wall, wallTint: o.tint, t, yWall: top }, o.roof || {});
    let ridgeY = top;
    if (R.type === 'hip') { hipRoof(B, R); ridgeY = top + Math.min(o.w, o.d) / 2 * R.pitch; }
    else if (R.type === 'flat') flatRoof(B, x0, x1, z0, z1, top, { wall: o.wall, wallTint: o.tint, parapet: R.parapet, t, cope: R.cope, copeTint: R.copeTint, ac: R.ac });
    else if (R.type !== 'none') ridgeY = gableRoof(B, R);
    if (o.chimney) {
      const cx = o.chimney.x != null ? o.chimney.x : x1 - 1.2, cz = o.chimney.z != null ? o.chimney.z : (z0 + z1) / 2;
      B.color(0xffffff); B.box('brick', cx - 0.45, base, cz - 0.6, cx + 0.45, ridgeY + 0.9, cz + 0.6);
      B.color(0x9a948a); B.box('concrete', cx - 0.52, ridgeY + 0.9, cz - 0.67, cx + 0.52, ridgeY + 1.0, cz + 0.67);
    }
    if (o.porch) porch(B, x0, x1, z1, base, top, Object.assign({ mid: doorMid }, o.porch));
    if (o.gutters !== false && R.type !== 'flat' && R.type !== 'none') {
      B.color(0xece9e2);
      const g = R.over + 0.06;
      B.box('metal', x0 - g, top - 0.32, z1 + R.over + 0.04, x1 + g, top - 0.2, z1 + R.over + 0.14);
      B.box('metal', x1 + R.over - 0.1, 0, z1 + R.over + 0.04, x1 + R.over, top - 0.3, z1 + R.over + 0.14);
    }
    if (o.antenna) { B.color(0x9a9ea2); B.cyl('metal', 'y', [x0 + 1.5, 0, z0 + 2], 0.03, ridgeY - 0.5, ridgeY + 3.2, 6, false); B.box('metal', x0 + 0.8, ridgeY + 2.6, z0 + 1.97, x0 + 2.2, ridgeY + 2.64, z0 + 2.03); B.box('metal', x0 + 1.0, ridgeY + 3.0, z0 + 1.97, x0 + 2.0, ridgeY + 3.04, z0 + 2.03); }
    // The far-away volume (LOD 2): a box to the eaves with this roof on it.
    B.volumes.push({ x0, x1, z0, z1, top, ridgeY, roof: R.type || 'gable', ridge: R.ridge || 'x', pitch: R.pitch, mat: R.mat, roofTint: R.tint || 0x4a4744, wall: o.wall, wallTint: o.tint || 0xffffff });
    return { x0, x1, z0, z1, top, ridgeY };
  }
  // The stairwell over storey k's stair, if it has one: (x0, z0, x1, z1), the stair's run with a little room.
  function stairHole(o, k, x0, x1, z0, t) {
    const plan = (o.plan && o.plan[k]) || ['living', 'kitchen', 'bed'];
    const shares = plan.map((r) => (typeof r === 'object' ? r.w : 1));
    const total = shares.reduce((a, b) => a + b, 0);
    const inW = x1 - x0 - 2 * t;
    let x = x0 + t;
    for (let i = 0; i < plan.length; i++) {
      const w = inW * shares[i] / total;
      if ((typeof plan[i] === 'object' ? plan[i].k : plan[i]) === 'stair') {
        const sx = x + 0.15, sz = z0 + t + 0.1;
        return [sx + 0.05, sz + 0.25, sx + 1.15, sz + 0.62 + 13 * 0.27];
      }
      x += w;
    }
    return null;
  }
  // A rect (x0, z0, x1, z1) less a hole in it, as up to four rects.
  function rectMinus(r, h) {
    if (!h || h[0] >= r[2] || h[2] <= r[0] || h[1] >= r[3] || h[3] <= r[1]) return [r];
    const c = [Math.max(r[0], h[0]), Math.max(r[1], h[1]), Math.min(r[2], h[2]), Math.min(r[3], h[3])];
    return [[r[0], r[1], r[2], c[1]], [r[0], c[3], r[2], r[3]], [r[0], c[1], c[0], c[3]], [c[2], c[1], r[2], c[3]]]
      .filter((q) => q[2] - q[0] > 0.01 && q[3] - q[1] > 0.01);
  }
  function porch(B, x0, x1, z1, base, top, p) {
    const d = p.d || 2.4, px0 = p.x0 != null ? p.x0 : x0, px1 = p.x1 != null ? p.x1 : x1;
    const deckY = base - 0.05;
    B.color(p.deckTint || 0xa98a64); B.box('deck', px0, deckY - 0.18, z1, px1, deckY, z1 + d);
    B.color(0xbab4aa); for (const x of [px0 + 0.2, px1 - 0.2]) B.box('concrete', x - 0.15, 0, z1 + d - 0.35, x + 0.15, deckY - 0.18, z1 + d - 0.05);
    const eave = p.eave != null ? p.eave : top - 0.25;
    const n = Math.max(2, Math.round((px1 - px0) / 2.6));
    B.color(p.postTint || 0xf4f1ea);
    for (let i = 0; i <= n; i++) {
      const x = px0 + 0.15 + (px1 - px0 - 0.3) * i / n;
      if (p.piers) { B.color(0xffffff); B.box('brick', x - 0.25, deckY, z1 + d - 0.5, x + 0.25, deckY + 1.0, z1 + d); B.color(p.postTint || 0xf4f1ea); B.box('trim', x - 0.14, deckY + 1.0, z1 + d - 0.39, x + 0.14, eave - 0.1, z1 + d - 0.11); }
      else B.box(p.postMat || 'trim', x - 0.09, deckY, z1 + d - 0.34, x + 0.09, eave - 0.1, z1 + d - 0.16);
    }
    // Railing with balusters, open in the middle bay for the steps.
    if (p.rail !== false) {
      const mid = p.mid != null && p.mid > px0 + 1.2 && p.mid < px1 - 1.2 ? p.mid : (px0 + px1) / 2;
      B.color(p.postTint || 0xf4f1ea);
      for (const [a, b] of [[px0 + 0.2, mid - 0.7], [mid + 0.7, px1 - 0.2]]) {
        B.box('trim', a, deckY + 0.9, z1 + d - 0.28, b, deckY + 0.96, z1 + d - 0.22);
        B.box('trim', a, deckY + 0.1, z1 + d - 0.28, b, deckY + 0.15, z1 + d - 0.22);
        for (let x = a + 0.15; x < b - 0.05; x += 0.16) B.box('trim', x - 0.02, deckY + 0.15, z1 + d - 0.27, x + 0.02, deckY + 0.9, z1 + d - 0.23);
      }
      B.color(0xa98a64);
      const steps = Math.max(1, Math.round(deckY / 0.18));
      for (let i = 0; i < steps; i++) B.box('deck', mid - 0.7, 0, z1 + d + i * 0.28, mid + 0.7, deckY - i * deckY / steps, z1 + d + (i + 1) * 0.28);
    }
    // Shed roof.
    B.color(p.roofTint || 0x4a4744);
    slope(B, p.roofMat || 'shingle', [[px0 - 0.2, eave - d * 0.22, z1 + d + 0.35], [px1 + 0.2, eave - d * 0.22, z1 + d + 0.35], [px1 + 0.2, eave + 0.02, z1 + 0.02], [px0 - 0.2, eave + 0.02, z1 + 0.02]]);
    fascia(B, px0 - 0.2, px1 + 0.2, z1 + d + 0.31, z1 + d + 0.35, eave - d * 0.22, 0xf4f1ea);
  }
  function furnishRoom(B, rnd, kind, x0, x1, z0, z1, y, storey, o) {
    const cx = (x0 + x1) / 2, cz = (z0 + z1) / 2;
    const fab = o.fabric || [0x8a6a3a, 0x6a7a3a, 0x9a4a3a, 0x4a5a7a][Math.floor(rnd() * 4)];
    if (kind === 'living') { place(B, cx, y, z0 + 0.5, 0, FURN.sofa(fab)); place(B, cx, y, z1 - 0.4, 2, FURN.tv()); }
    else if (kind === 'kitchen') { if (x1 - x0 > 3.7) place(B, cx, y, z0 + 0.35, 0, FURN.kitchen(o.cabinet || 0xe8dcc0)); place(B, cx, y, cz + 0.6, 0, FURN.table()); }
    else if (kind === 'bed') place(B, cx, y, z0 + 1.15, 0, FURN.bed(fab));
    else if (kind === 'stair') { B.color(0x8a6a46); for (let i = 0; i < 14; i++) B.box('wood', x0 + 0.1, y, z0 + 0.3 + i * 0.27, x0 + 1.05, y + (i + 1) * (o.fh || 2.75) / 14, z0 + 0.57 + i * 0.27); }
    else if (kind === 'office') place(B, cx, y, cz, 2, FURN.desk());
  }

  // ------------------------------------------------------------------ the building types
  const PAINT = { white: 0xf1eee6, cream: 0xe9dfc4, sage: 0x9fae8f, blue: 0x8fa7b8, yellow: 0xe6cf8a, mint: 0xa8d5c2, coral: 0xe8a28c, pink: 0xeab8c4, grey: 0xa8aaa8, brown: 0x8a6a4a, green: 0x5f7a52 };
  const CARS = [0x8a2c2a, 0xd8d2c0, 0x2c4a7a, 0x6a6a5a, 0xa87a3a, 0x3a5a3a, 0x1a1a1a, 0xb8b2a2];
  const pick = (rnd, a) => a[Math.floor(rnd() * a.length)];

  const TYPES = [
    { id: 'ranch', name: 'Ranch house', kind: 'house', biome: 'temperate, savanna', size: [17, 10], blurb: 'Single storey, low hip roof, a picture window, brick front with siding ends and an attached garage. The commonest 1950s-80s house.',
      // Variants (owner, 2026-10-26: "we may need a few variations of the subrabn house"): b, painted siding under a
      // side gable with a carport; c, an L-plan with a gabled brick bedroom wing stepped forward.
      variants: ['Ranch house, siding and carport', 'Ranch house, L-plan'],
      build(B, rnd, M, v = 0) {
        if (v === 1) {
          const paint = pick(rnd, [PAINT.cream, PAINT.sage, PAINT.yellow, PAINT.blue]);
          house(B, rnd, { x: -2.4, w: 12.8, d: 9.0, wall: 'siding', tint: paint, trim: 0xf4f1ea, front: [[['win', 1.2, 1.3, 0.9], ['win', 1.2, 1.3, 0.9], ['door', 0.95, 2.1, 0, { lite: true, doorTint: pick(rnd, [0x7a2a24, 0x2c4a3a, 0x3a3a3a]) }], ['pic', 2.8, 1.5, 0.8]]], roof: { type: 'gable', ridge: 'x', pitch: 0.3, over: 0.6, overEnd: 0.5, tint: pick(rnd, [0x4a4744, 0x5a4a3c, 0x3c4048]) }, plan: [['bed', 'bed', 'kitchen', 'living']], chimney: { x: 2.6 }, shutters: pick(rnd, [0x2c3a2c, 0x5a2a24, 0x2a2f3a]) });
          // The carport: four posts and a flat steel roof beside the house.
          B.color(0xf4f1ea);
          for (const [x, z] of [[4.4, -3.4], [7.9, -3.4], [4.4, 4.3], [7.9, 4.3]]) B.box('trim', x - 0.07, 0, z - 0.07, x + 0.07, 2.55, z + 0.07);
          B.color(0xc8ccce); B.box('metalRoof', 4.02, 2.55, -3.8, 8.3, 2.7, 4.7);
          B.color(0xf4f1ea); B.box('trim', 4.02, 2.45, 4.7, 8.32, 2.72, 4.78);
          B.color(0xb8b2a6); B.box('concrete', 4.2, 0, -3.6, 8.1, 0.06, 12);
          car(B, 6.15, 0.6, 0, pick(rnd, CARS), pick(rnd, ['sedan', 'pickup']));
          return;
        }
        if (v === 2) {
          const paint = pick(rnd, [PAINT.white, PAINT.grey, PAINT.cream]);
          const roof = pick(rnd, [0x4a4744, 0x5a4a3c, 0x3c4048]);
          house(B, rnd, { x: -3.0, w: 11.0, d: 9.0, wall: 'siding', tint: paint, front: [[['win', 1.2, 1.3, 0.9], ['door', 0.95, 2.1, 0, { lite: true, doorTint: pick(rnd, [0x2c4a3a, 0x7a2a24]) }], ['win', 1.2, 1.3, 0.9], ['win', 1.2, 1.3, 0.9]]], roof: { type: 'hip', pitch: 0.33, over: 0.6, tint: roof }, plan: [['kitchen', 'living', 'bed']], blankRight: true, lintel: 'concrete', lintelTint: 0xd8d2c6, antenna: true });
          house(B, rnd, { x: 5.0, z: 1.75, w: 5.0, d: 9.5, wall: 'brick', front: [[['pic', 2.6, 1.5, 0.75]]], sideWin: 2, roof: { type: 'gable', ridge: 'z', pitch: 0.45, over: 0.5, tint: roof }, plan: [['bed']], blankLeft: true, lintel: 'concrete', lintelTint: 0xd8d2c6, chimney: { x: 6.79, z: -1.5 } });
          car(B, 1.2, 9.0, 1, pick(rnd, CARS), 'sedan');
          return;
        }
        house(B, rnd, { x: -2.2, w: 12.5, d: 9.5, wall: 'brick', front: [[['pic', 3.2, 1.6, 0.75], ['door', 0.95, 2.1, 0, { lite: true, doorTint: pick(rnd, [0x7a2a24, 0x2c4a3a, 0x3a3a3a]) }], ['win', 1.2, 1.3, 0.9], ['win', 1.2, 1.3, 0.9]]], roof: { type: 'hip', pitch: 0.33, over: 0.7, tint: pick(rnd, [0x4a4744, 0x5a4a3c, 0x3c4048]) }, plan: [['living', 'kitchen', 'bed', 'bed']], blankRight: true, chimney: { x: -5.6 }, lintel: 'concrete', lintelTint: 0xd8d2c6, antenna: true });
        house(B, rnd, { x: 6.025, w: 3.95, d: 7.5, z: -1.0, wall: 'siding', tint: PAINT.cream, base: 0.1, front: [[['garage', 2.9, 2.2, 0]]], sideWin: 1, back: [[['door', 0.9, 2.1, 0]]], roof: { type: 'gable', ridge: 'z', pitch: 0.33, over: 0.5, tint: 0x4a4744 }, plan: [[]], floorMat: 'concrete', blankLeft: true });
        B.color(0xb8b2a6); B.box('concrete', 4.2, 0, 2.75, 7.8, 0.06, 12);
        car(B, 6.0, 7.5, 0, pick(rnd, CARS), 'sedan');
      } },
    { id: 'colonial', name: 'Colonial', kind: 'house', biome: 'temperate', size: [12, 9], blurb: 'Two storeys, five bays of shuttered windows, a centre door under a columned portico, white clapboard and end chimneys.',
      // Variants: b, red brick, three bays and a one-storey sunroom wing; c, painted clapboard behind a full-width
      // porch, with an attached garage.
      variants: ['Colonial, brick with a sunroom', 'Colonial, porch and garage'],
      build(B, rnd, M, v = 0) {
        if (v === 1) {
          const shutters = pick(rnd, [0x2c3a2c, 0x2a2f3a, 0xf4f1ea]);
          const r = house(B, rnd, { x: -1.0, w: 9.0, d: 8.0, floors: 2, wall: 'brick', shutters, front: [[['win', 1.0, 1.55, 0.8], ['door', 1.0, 2.2, 0, { lite: true, doorTint: pick(rnd, [0x2a2a2a, 0x5a2a24, 0x2c4a3a]) }], ['win', 1.0, 1.55, 0.8]], [['win', 1.0, 1.45, 0.8], ['win', 0.8, 1.2, 0.95], ['win', 1.0, 1.45, 0.8]]], roof: { type: 'gable', pitch: 0.62, over: 0.35, overEnd: 0.25, tint: 0x3a3c40 }, plan: [['living', 'stair', 'kitchen'], ['bed', 'bed']], chimney: { x: -4.8 }, lintel: 'concrete', lintelTint: 0xe6e0d4 });
          house(B, rnd, { x: 5.2, z: -0.8, w: 3.4, d: 6.0, wall: 'siding', tint: PAINT.white, front: [[['win', 1.0, 1.6, 0.6], ['win', 1.0, 1.6, 0.6]]], sideWin: 2, roof: { type: 'hip', pitch: 0.3, over: 0.35, tint: 0x3a3c40 }, plan: [['living']], blankLeft: true });
          B.color(0xb8b2a6); B.box('concrete', -1.6, 0, r.z1 + 1.1, -0.4, 0.04, 12);
          return;
        }
        if (v === 2) {
          const paint = pick(rnd, [PAINT.yellow, PAINT.blue, PAINT.grey, PAINT.sage]);
          house(B, rnd, { x: -2.0, w: 10.0, d: 8.2, floors: 2, wall: 'siding', tint: paint, trim: 0xf4f1ea, front: [[['win', 1.0, 1.55, 0.8], ['win', 1.0, 1.55, 0.8], ['door', 1.0, 2.2, 0, { lite: true, doorTint: pick(rnd, [0x2a2a2a, 0x7a2a24]) }], ['win', 1.0, 1.55, 0.8]], [['win', 1.0, 1.45, 0.8], ['win', 1.0, 1.45, 0.8], ['win', 1.0, 1.45, 0.8], ['win', 1.0, 1.45, 0.8]]], roof: { type: 'gable', pitch: 0.58, over: 0.4, overEnd: 0.25, tint: 0x4a4744 }, plan: [['living', 'stair', 'kitchen'], ['bed', 'bed', 'bed']], porch: { d: 2.2, postTint: 0xf4f1ea }, chimney: { x: -6.4 } });
          house(B, rnd, { x: 5.2, z: -0.6, w: 4.4, d: 7.0, wall: 'siding', tint: paint, base: 0.1, front: [[['garage', 3.0, 2.2, 0]]], sideWin: 1, back: [[['door', 0.9, 2.1, 0]]], roof: { type: 'gable', ridge: 'z', pitch: 0.4, over: 0.45, tint: 0x4a4744 }, plan: [[]], floorMat: 'concrete', blankLeft: true });
          B.color(0xb8b2a6); B.box('concrete', 3.4, 0, 2.9, 7.0, 0.06, 12);
          car(B, 5.2, 7.6, 0, pick(rnd, CARS), 'sedan');
          return;
        }
        const r = house(B, rnd, { w: 11.5, d: 8.4, floors: 2, wall: 'siding', tint: PAINT.white, shutters: pick(rnd, [0x2c3a2c, 0x2a2f3a, 0x5a2a24]), front: [[['win', 1.0, 1.55, 0.8], ['win', 1.0, 1.55, 0.8], ['door', 1.0, 2.2, 0, { doorTint: 0x2a2a2a }], ['win', 1.0, 1.55, 0.8], ['win', 1.0, 1.55, 0.8]], [['win', 1.0, 1.45, 0.8], ['win', 1.0, 1.45, 0.8], ['win', 0.8, 1.2, 0.95], ['win', 1.0, 1.45, 0.8], ['win', 1.0, 1.45, 0.8]]], roof: { type: 'gable', pitch: 0.62, over: 0.4, overEnd: 0.25, tint: 0x3a3c40 }, plan: [['living', 'stair', 'kitchen'], ['bed', 'bed', 'bed']], chimney: { x: 6.2 } });
        // Portico: two columns and a pediment over the door.
        B.color(0xf4f1ea);
        for (const x of [-0.95, 0.95]) B.cyl('trim', 'y', [x, 0, r.z1 + 1.35], 0.13, 0.45, 3.0, 12);
        B.box('trim', -1.35, 3.0, r.z1 + 0.02, 1.35, 3.25, r.z1 + 1.6);
        B.poly('trim', [[-1.4, 3.25, r.z1 + 1.6], [1.4, 3.25, r.z1 + 1.6], [0, 3.95, r.z1 + 1.6]], [0, 0, 1]);
        B.color(0x3a3c40); slope(B, 'shingle', [[-1.45, 3.25, r.z1 + 1.65], [0, 3.97, r.z1 + 1.65], [0, 3.97, r.z1 + 0.02], [-1.45, 3.25, r.z1 + 0.02]]); slope(B, 'shingle', [[0, 3.97, r.z1 + 1.65], [1.45, 3.25, r.z1 + 1.65], [1.45, 3.25, r.z1 + 0.02], [0, 3.97, r.z1 + 0.02]]);
        B.color(0xbab4aa); B.box('concrete', -1.5, 0, r.z1, 1.5, 0.45, r.z1 + 1.9);
        B.color(0xb8b2a6); B.box('concrete', -0.6, 0, r.z1 + 1.9, 0.6, 0.04, 12);
      } },
    { id: 'bungalow', name: 'Craftsman bungalow', kind: 'house', biome: 'temperate, coast, montane forest', size: [9.5, 13], blurb: 'Gable to the street, a deep full-width porch on tapered posts and brick piers, painted siding and a big front window.',
      // Variants: b, a hip roof over a porch on plain posts; c, a side gable in brick, the porch across the front.
      variants: ['Bungalow, hip roof', 'Bungalow, brick side gable'],
      build(B, rnd, M, v = 0) {
        if (v === 1) {
          house(B, rnd, { w: 10.0, d: 10.5, wall: 'siding', tint: pick(rnd, [PAINT.mint, PAINT.coral, PAINT.cream, PAINT.blue]), trim: 0xf4f1ea, front: [[['win', 1.1, 1.5, 0.75], ['door', 1.0, 2.15, 0, { lite: true, doorTint: pick(rnd, [0x6b3b22, 0x2c4a3a]) }], ['win', 1.1, 1.5, 0.75]]], roof: { type: 'hip', pitch: 0.42, over: 0.7, tint: pick(rnd, [0x4a3a30, 0x3c4048]) }, plan: [['bed', 'living', 'kitchen']], sideWin: 3, porch: { d: 2.4, x0: -3.6, x1: 3.6, postTint: 0xf4f1ea }, chimney: { x: 3.6, z: -2 } });
          return;
        }
        if (v === 2) {
          house(B, rnd, { w: 11.0, d: 9.0, wall: 'brick', trim: 0xf4f1ea, front: [[['pic', 2.2, 1.5, 0.75], ['door', 1.0, 2.15, 0, { lite: true, doorTint: pick(rnd, [0x6b3b22, 0x7a2a24]) }], ['win', 1.1, 1.5, 0.75], ['win', 1.1, 1.5, 0.75]]], roof: { type: 'gable', ridge: 'x', pitch: 0.55, over: 0.6, overEnd: 0.4, tint: pick(rnd, [0x4a3a30, 0x5a4a3c]) }, plan: [['living', 'kitchen', 'bed']], sideWin: 2, porch: { d: 2.4, piers: true, postTint: pick(rnd, [0xf4f1ea, 0x6b4a2a]) }, chimney: { x: -6.0 }, lintel: 'concrete', lintelTint: 0xd8d2c6 });
          return;
        }
        house(B, rnd, { w: 9.2, d: 12, wall: 'siding', tint: pick(rnd, [PAINT.sage, PAINT.blue, PAINT.yellow]), trim: 0xf4f1ea, front: [[['win', 1.0, 1.5, 0.75], ['door', 1.0, 2.15, 0, { lite: true, doorTint: 0x6b3b22 }], ['pic', 2.2, 1.5, 0.75]]], roof: { type: 'gable', ridge: 'z', pitch: 0.5, over: 0.7, tint: 0x4a3a30 }, plan: [['bed', 'living', 'kitchen']], sideWin: 3, porch: { d: 2.6, piers: true, postTint: 0xf4f1ea }, chimney: { x: -5.0, z: -2 } });
      } },
    { id: 'split', name: 'Split-level', kind: 'house', biome: 'temperate, montane forest', size: [16, 9], blurb: 'A one-storey living wing beside a two-storey bedroom wing half a level up, the garage under it. Very 1970s.',
      // Variants: b, hip roofs and painted siding throughout; c, board and batten with gables to the street and a
      // chimney on the living wing.
      variants: ['Split-level, hip roofs', 'Split-level, board and batten'],
      build(B, rnd, M, v = 0) {
        if (v === 1 || v === 2) {
          const b = v === 2;
          const wall = b ? 'board' : 'siding';
          const tint = b ? pick(rnd, [0xb89a74, 0x8a6a4a, 0x9a8a6a]) : pick(rnd, [PAINT.cream, PAINT.blue, PAINT.grey, PAINT.green]);
          const roofTint = pick(rnd, [0x5a4a3c, 0x3c4048, 0x4a4744]);
          const roof = (ridge) => b ? { type: 'gable', ridge, pitch: 0.42, over: 0.6, tint: roofTint } : { type: 'hip', pitch: 0.36, over: 0.6, tint: roofTint };
          house(B, rnd, { x: -3.6, w: 7.6, d: 9, wall, tint, front: [[['win', 1.2, 1.3, 0.9], ['win', 1.2, 1.3, 0.9], ['door', 0.95, 2.1, 0, { lite: true, doorTint: pick(rnd, [0x7a5a2a, 0x2c4a3a]) }]]], roof: roof('z'), plan: [['living', 'kitchen']], blankRight: true, chimney: b ? { x: -6.6 } : null });
          house(B, rnd, { x: 3.9, w: 7.4, d: 8.4, base: 0.15, floors: 2, fh: 2.65, wall, tint, front: [[['garage', 2.8, 2.15, 0], ['win', 1.0, 1.1, 1.0]], [['win', 1.0, 1.3, 0.9], ['win', 1.0, 1.3, 0.9], ['win', 1.0, 1.3, 0.9]]], roof: roof('z'), plan: [['office', 'stair'], ['bed', 'bed']], blankLeft: true, floorMat: 'carpet', shutters: b ? null : pick(rnd, [0x2c3a2c, 0x5a2a24]) });
          B.color(0xb8b2a6); B.box('concrete', 2.3, 0, 4.2, 5.4, 0.06, 12);
          car(B, 3.85, 8.4, 0, pick(rnd, CARS), pick(rnd, ['sedan', 'pickup']));
          return;
        }
        house(B, rnd, { x: -3.6, w: 7.6, d: 9, wall: 'brick', front: [[['pic', 3.0, 1.5, 0.7], ['door', 0.95, 2.1, 0, { lite: true, doorTint: 0x7a5a2a }]]], roof: { type: 'gable', pitch: 0.36, over: 0.6, overEnd: 0.4, tint: 0x5a4a3c }, plan: [['living', 'kitchen']], blankRight: true, lintel: 'concrete', lintelTint: 0xd8d2c6 });
        house(B, rnd, { x: 3.9, w: 7.4, d: 8.4, base: 0.15, floors: 2, fh: 2.65, wall: 'siding', tint: pick(rnd, [PAINT.brown, PAINT.cream, PAINT.green]), front: [[['garage', 2.8, 2.15, 0], ['win', 1.0, 1.1, 1.0]], [['win', 1.6, 1.3, 0.9], ['win', 1.6, 1.3, 0.9]]], roof: { type: 'gable', ridge: 'z', pitch: 0.36, over: 0.6, tint: 0x5a4a3c }, plan: [['office', 'stair'], ['bed', 'bed']], blankLeft: true, floorMat: 'carpet' });
        B.color(0xb8b2a6); B.box('concrete', 2.3, 0, 4.2, 5.4, 0.06, 12);
        car(B, 3.85, 8.4, 0, pick(rnd, CARS), 'pickup');
      } },
    // The A-frame cabin is gone (owner, 2026-10-26: "the one house I dont like is the one with the single big triangle
    // roof, all others look good").
    { id: 'adobe', name: 'Pueblo Revival house', kind: 'house', biome: 'desert', size: [14, 10], blurb: 'Thick stucco walls with rounded corners, flat roof behind stepped parapets, vigas through the wall, a portal porch and a turquoise door.',
      build(B, rnd) {
        const tint = pick(rnd, [0xd6a679, 0xc99a6c, 0xddb78e]);
        house(B, rnd, { x: -1, w: 12, d: 9, base: 0.2, t: 0.5, wall: 'adobe', tint, trim: 0x6b4a2a, front: [[['win', 0.9, 1.1, 1.0, { frameMat: 'wood', frameTint: 0x6b4a2a, sill: false, rail: false }], ['door', 1.0, 2.15, 0, { doorTint: 0x2f8f8a, doorMat: 'door', stoop: false }], ['win', 0.9, 1.1, 1.0, { frameMat: 'wood', frameTint: 0x6b4a2a, sill: false, rail: false }], ['win', 0.9, 1.1, 1.0, { frameMat: 'wood', frameTint: 0x6b4a2a, sill: false, rail: false }]]], roof: { type: 'flat', parapet: 0.6, cope: 'adobe', copeTint: tint }, plan: [['living', 'kitchen', 'bed']], floorMat: 'floorTile', gutters: false, sideWin: 2 });
        house(B, rnd, { x: 6.5, z: -1, w: 3.0, d: 6, base: 0.2, t: 0.5, fh: 3.4, wall: 'adobe', tint, front: [[['win', 0.8, 0.9, 1.3, { frameMat: 'wood', frameTint: 0x6b4a2a, sill: false, rail: false }]]], roof: { type: 'flat', parapet: 0.9, cope: 'adobe', copeTint: tint }, plan: [['bed']], blankLeft: true, gutters: false, sideWin: 1 });
        // Vigas: round beams through the front wall under the parapet.
        B.color(0x7a5a3a);
        for (let x = -6.5; x <= 4.6; x += 1.1) B.cyl('wood', 'z', [x, 2.75, 0], 0.11, 4.2, 4.95, 8);
        // Portal: posts with corbels and a beam.
        for (let x = -5.5; x <= 2.6; x += 2.7) { B.box('wood', x - 0.12, 0.2, 6.1, x + 0.12, 2.6, 6.34); B.box('wood', x - 0.45, 2.45, 6.08, x + 0.45, 2.6, 6.36); }
        B.box('wood', -5.8, 2.6, 6.05, 2.9, 2.85, 6.4);
        B.color(0x6d6a66); B.box('roofing', -5.8, 2.85, 4.5, 2.9, 2.95, 6.4);
        B.color(0xb88a5c); B.box('floorTile', -5.8, 0, 4.5, 2.9, 0.2, 6.4);
        // Rounded parapet tops (half cylinders at the corners).
        B.color(tint);
        for (const [x, z] of [[-7, 4.5], [5, 4.5], [-7, -4.5], [8, 2]]) B.cyl('adobe', 'y', [x, 0, z], 0.35, 0.2, 3.5, 10);
        B.color(0x5a7a4a); for (let i = 0; i < 5; i++) { const x = -6 + rnd() * 12, z = 7 + rnd() * 3; B.cyl('fabric', 'y', [x, 0, z], 0.12 + rnd() * 0.1, 0, 0.8 + rnd() * 1.4, 7); }
      } },
    { id: 'stilt', name: 'Stilt house', kind: 'house', biome: 'jungle, swamp', size: [10, 10], blurb: 'Raised 2.4 m on timber piles over flood and swamp, plank walls, a steep corrugated hip roof and a veranda all round.',
      build(B, rnd) {
        const lift = 2.4;
        const r = house(B, rnd, { w: 7.5, d: 6.5, base: lift, t: 0.12, wall: 'board', tint: pick(rnd, [0xb08a5c, 0x8fa78a, 0xc4b08a]), front: [[['win', 1.1, 1.1, 0.9, { shutters: 0x4a6a4a, sill: false }], ['door', 0.9, 2.05, 0, { doorTint: 0x4a6a4a, stoop: false }], ['win', 1.1, 1.1, 0.9, { shutters: 0x4a6a4a, sill: false }]]], roof: { type: 'hip', pitch: 0.75, over: 1.9, mat: 'metalRoof', tint: pick(rnd, [0x8a4a3a, 0x7a8a8a, 0x6a6a5a]) }, plan: [['living', 'bed']], gutters: false, sideWin: 1 });
        // Piles and veranda deck.
        B.color(0x5a4630);
        for (let x = -5.0; x <= 5.01; x += 2.5) for (let z = -4.25; z <= 4.26; z += 2.83) B.box('wood', x - 0.13, -0.6, z - 0.13, x + 0.13, lift - 0.2, z + 0.13);
        B.color(0x8a6a46); B.box('deck', -5.3, lift - 0.25, -4.55, 5.3, lift - 0.03, 4.55);
        for (const [a, b, z] of [[-5.25, 5.25, 4.45], [-5.25, 5.25, -4.45]]) { B.box('wood', a, lift + 0.85, z - 0.04, b, lift + 0.92, z + 0.04); for (let x = a; x <= b; x += 0.3) B.box('wood', x - 0.02, lift, z - 0.02, x + 0.02, lift + 0.85, z + 0.02); }
        for (const x of [-5.2, 5.2]) { B.box('wood', x - 0.04, lift + 0.85, -4.45, x + 0.04, lift + 0.92, 4.45); for (let z = -4.4; z <= 4.4; z += 0.3) B.box('wood', x - 0.02, lift, z - 0.02, x + 0.02, lift + 0.85, z + 0.02); }
        for (const x of [-5.1, 5.1]) for (const z of [-4.35, 4.35]) B.box('wood', x - 0.08, lift, z - 0.08, x + 0.08, r.top + 0.1, z + 0.08);
        // Ladder stair down the front.
        for (let i = 0; i < 9; i++) B.box('wood', -0.6, lift - 0.27 - i * 0.27, 4.55 + i * 0.25, 0.6, lift - 0.22 - i * 0.27, 4.8 + i * 0.25);
        B.box('wood', -0.68, 0, 4.5, -0.6, lift, 7); B.box('wood', 0.6, 0, 4.5, 0.68, lift, 7);
        // A dugout boat under the house.
        B.color(0x6a4a2a); B.box('wood', 1.2, 0, -1.5, 2.0, 0.4, 3.0);
      } },
    { id: 'beach', name: 'Beach cottage', kind: 'house', biome: 'beach', size: [9, 11], blurb: 'Pastel siding on concrete piers, a metal roof, louvred shutters and a screened porch across the front. Florida and the Gulf, 1980s.',
      build(B, rnd) {
        const tint = pick(rnd, [PAINT.mint, PAINT.coral, PAINT.pink, PAINT.yellow, PAINT.blue]);
        house(B, rnd, { w: 8.5, d: 8, base: 1.3, t: 0.15, wall: 'siding', tint, shutters: 0xf4f1ea, front: [[['win', 1.1, 1.4, 0.8], ['door', 0.95, 2.1, 0, { lite: true, doorTint: 0xf4f1ea, stoop: false }], ['win', 1.1, 1.4, 0.8]]], roof: { type: 'hip', pitch: 0.42, over: 0.6, mat: 'metalRoof', tint: 0xb8bcbe }, plan: [['bed', 'living', 'kitchen']], sideWin: 2, porch: { d: 2.6, roofMat: 'metalRoof', roofTint: 0xb8bcbe, deckTint: 0xb89a74 } });
        // Concrete piers under the floor and lattice skirting.
        B.color(0xcac4b8); for (let x = -3.9; x <= 3.95; x += 2.6) for (const z of [-3.7, 0, 3.7]) B.box('concrete', x - 0.2, -0.3, z - 0.2, x + 0.2, 0.9, z + 0.2);
        B.color(0xf4f1ea); for (let x = -4.2; x <= 4.2; x += 0.3) B.box('trim', x - 0.02, 0, 4.006, x + 0.02, 0.88, 4.046);
        // Screen mesh on the porch (dark transparent).
        B.color(0xffffff); for (const x0 of [-4.1, 0.85]) B.box('screen', x0, 1.35, 6.4, x0 + 3.25, 3.4, 6.42);
        B.color(0x5a7a4a); B.cyl('wood', 'y', [-5.8, 0, 7], 0.15, 0, 6.5, 7); for (let i = 0; i < 6; i++) { const a = i / 6 * Math.PI * 2; B.box('fabric', -5.8 + Math.cos(a) * 1.1 - 0.2, 6.3, 7 + Math.sin(a) * 1.1 - 0.2, -5.8 + Math.cos(a) * 1.1 + 0.2, 6.45, 7 + Math.sin(a) * 1.1 + 0.2); }
      } },
    { id: 'trailer', name: 'Single-wide trailer', kind: 'house', biome: 'desert, savanna', size: [20, 6], blurb: 'A 1970s mobile home: ribbed metal siding with a two-tone stripe, small windows, a window air conditioner, skirting, metal steps and an awning.',
      build(B, rnd) {
        const L = 19.5, W = 4.3, base = 0.75, H = 2.45, t = 0.1;
        const x0 = -L / 2, x1 = L / 2, z0 = -W / 2, z1 = W / 2;
        const stripe = pick(rnd, [0x7a5a2a, 0x3a5a7a, 0x7a3a2a, 0x4a6a3a]);
        B.color(0xdcd6c6); B.box('skirt', x0 + 0.1, 0, z0 + 0.1, x1 - 0.1, base, z1 - 0.1);
        const front = row(x0 + 0.5, x1 - 0.5, base, [['win', 1.4, 1.0, 0.95, { ac: true }], ['door', 0.85, 1.95, 0, { lite: true, doorTint: 0xe8e2d0, stoop: false }], ['win', 1.0, 0.9, 1.0], ['win', 2.0, 1.0, 0.95], ['win', 1.0, 0.9, 1.0], ['win', 1.0, 0.6, 1.3]]);
        B.color(0xece6d8); wall(B, 'ribbed', 'x', z1 - t / 2, x0, x1, t, base, base + H, 1, front, { inner: 'woodInt', innerPad: t });
        B.color(0xece6d8); wall(B, 'ribbed', 'x', z0 + t / 2, x0, x1, t, base, base + H, -1, row(x0 + 0.5, x1 - 0.5, base, [['win', 1.0, 0.9, 1.0], ['win', 1.0, 0.9, 1.0], ['win', 1.0, 0.9, 1.0]]), { inner: 'woodInt', innerPad: t });
        B.color(0xece6d8); wall(B, 'ribbed', 'z', x0 + t / 2, z0 + t, z1 - t, t, base, base + H, -1, [{ kind: 'win', a: -0.7, b: 0.7, y0: base + 1.0, y1: base + 2.0 }], { inner: 'woodInt' });
        B.color(0xece6d8); wall(B, 'ribbed', 'z', x1 - t / 2, z0 + t, z1 - t, t, base, base + H, 1, [], { inner: 'woodInt' });
        B.color(stripe); B.box('paint', x0 - 0.01, base + 0.6, z1, x1 + 0.01, base + 0.82, z1 + 0.015); B.box('paint', x0 - 0.01, base + 0.6, z0 - 0.015, x1 + 0.01, base + 0.82, z0);
        B.color(0xd8d2c6); B.box('metalRoof', x0 - 0.05, base + H, z0 - 0.05, x1 + 0.05, base + H + 0.18, z1 + 0.05);
        B.color(0xffffff); B.box('carpet', x0 + t + 0.012, base - 0.03, z0 + t + 0.012, x1 - t - 0.012, base + 0.012, z1 - t - 0.012);
        B.color(0xf2efe8); B.box('woodInt', x0 + t, base + H - 0.03, z0 + t, x1 - t, base + H, z1 - t);
        place(B, x0 + 3, base, 0, 0, FURN.sofa(0x8a6a3a)); place(B, x0 + 8, base, z0 + 0.5, 0, FURN.kitchen(0xd8c8a0)); place(B, x1 - 2.5, base, 0, 1, FURN.bed(0x4a5a7a));
        // Hitch, steps, awning.
        B.color(0x55585a); B.box('metal', x1, 0.3, -0.1, x1 + 1.4, 0.45, 0.1);
        const dx = front[1].a; B.color(0x9a9ea2); for (let i = 0; i < 4; i++) B.box('metal', dx - 0.2, 0, z1 + 0.9 - i * 0.25, dx + 1.05, base - i * 0.19, z1 + 1.15 - i * 0.25);
        B.color(0xece6d8); B.box('metal', dx - 2.2, base + 2.35, z1, dx + 2.6, base + 2.42, z1 + 2.4);
        for (const x of [dx - 2.1, dx + 2.5]) B.box('metal', x - 0.04, 0, z1 + 2.3, x + 0.04, base + 2.35, z1 + 2.38);
        car(B, x0 + 4.5, z1 + 4.5, 1, pick(rnd, CARS), 'pickup');
      } },
    // ---------------------------------------------------------------- businesses
    { id: 'diner', name: 'Streamline diner', kind: 'business', biome: 'any roadside', size: [21, 8], blurb: 'Stainless steel with enamel stripes and rounded ends, a ribbon of windows over the booths, a counter with stools, and DINER in neon on the roof.',
      build(B, rnd, M) {
        const L = 17, W = 6.4, base = 0.8, H = 3.1, t = 0.2;
        const x0 = -L / 2, x1 = L / 2, z0 = -W / 2, z1 = W / 2, top = base + H;
        const stripe = pick(rnd, [0xc23b30, 0x2f6f8f, 0x2f8a48]);
        B.color(0xbab4aa); B.box('concrete', x0 - W / 2, -0.3, z0, x1 + W / 2, base, z1);
        const wins = []; for (let x = x0 + 0.3; x + 1.25 <= x1 - 0.3; x += 1.4) wins.push({ kind: 'ribbon', a: x, b: x + 1.25, y0: base + 1.0, y1: base + 2.25, frameMat: 'metal', frameTint: 0xd8dcde, sill: false, lights: 1 });
        const frontWins = wins.filter((w) => w.b < -1.1 || w.a > 1.1);
        frontWins.push({ kind: 'door', a: -0.5, b: 0.5, y0: base, y1: base + 2.2, doorMat: 'metal', doorTint: 0xd8dcde, lite: true, frameMat: 'metal', stoop: false });
        B.color(0xffffff); wall(B, 'stainless', 'x', z1 - t / 2, x0, x1, t, base, top, 1, frontWins, { inner: 'tileWall', innerPad: t });
        B.color(0xffffff); wall(B, 'stainless', 'x', z0 + t / 2, x0, x1, t, base, top, -1, [{ kind: 'win', a: -3, b: -1.8, y0: base + 1.3, y1: base + 2.2 }], { inner: 'tileWall', innerPad: t });
        // Rounded ends.
        for (const [cx, a0] of [[x0, Math.PI / 2], [x1, -Math.PI / 2]]) {
          const seg = 10;
          for (let i = 0; i < seg; i++) {
            const a = a0 + (i / seg) * Math.PI, b = a0 + ((i + 1) / seg) * Math.PI;
            const P = (ang, r, y) => [cx + Math.cos(ang) * r, y, Math.sin(ang) * r];
            B.color(0xffffff); B.poly('stainless', [P(a, W / 2, base), P(b, W / 2, base), P(b, W / 2, top), P(a, W / 2, top)], [Math.cos((a + b) / 2), 0, Math.sin((a + b) / 2)]);
            B.color(0xf2efe8); B.poly('tileWall', [P(a, W / 2 - t, base), P(b, W / 2 - t, base), P(b, W / 2 - t, top), P(a, W / 2 - t, top)], [-Math.cos((a + b) / 2), 0, -Math.sin((a + b) / 2)]);
            B.color(stripe); B.poly('paint', [P(a, W / 2 + 0.012, base + 0.55), P(b, W / 2 + 0.012, base + 0.55), P(b, W / 2 + 0.012, base + 0.8), P(a, W / 2 + 0.012, base + 0.8)], [Math.cos((a + b) / 2), 0, Math.sin((a + b) / 2)]);
            B.color(0x6d6a66); B.poly('roofing', [[cx, top + 0.25, 0], P(b, W / 2 + 0.25, top + 0.25), P(a, W / 2 + 0.25, top + 0.25)], [0, 1, 0]);
            B.color(stripe); B.poly('paint', [P(a, W / 2 + 0.25, top), P(b, W / 2 + 0.25, top), P(b, W / 2 + 0.25, top + 0.25), P(a, W / 2 + 0.25, top + 0.25)], [Math.cos((a + b) / 2), 0, Math.sin((a + b) / 2)]);
            B.color(0xffffff); B.poly('floorTile', [[cx, base + 0.012, 0], P(a, W / 2 - t, base + 0.012), P(b, W / 2 - t, base + 0.012)], [0, 1, 0]);
          }
        }
        // Stripes and roof band.
        B.color(stripe); B.box('paint', x0, base + 0.55, z1, x1, base + 0.8, z1 + 0.012); B.box('paint', x0, base + 0.55, z0 - 0.012, x1, base + 0.8, z0);
        B.color(0x6d6a66); B.box('roofing', x0, top, z0 - 0.25, x1, top + 0.25, z1 + 0.25, 'y');
        B.color(stripe); B.box('paint', x0 + 0.006, top, z1 + 0.25, x1 - 0.006, top + 0.25, z1 + 0.27); B.box('paint', x0 + 0.006, top, z0 - 0.27, x1 - 0.006, top + 0.25, z0 - 0.25);
        B.color(0xffffff); B.box('floorTile', x0, base - 0.03, z0 + t, x1, base + 0.012, z1 - t);
        B.color(0xf2efe8); B.box('tileWall', x0, top - 0.03, z0 + t, x1, top - 0.006, z1 - t);
        // Vestibule canopy and steps.
        B.color(0xd8dcde); B.box('stainless', -1.3, base + 2.4, z1, 1.3, base + 2.55, z1 + 1.4);
        B.color(0xbab4aa); for (let i = 0; i < 4; i++) B.box('concrete', -1.0, 0, z1 + 0.2 + i * 0.3, 1.0, base - i * 0.2, z1 + 0.5 + i * 0.3);
        // Inside: booths along the windows, a counter with stools, the kitchen behind.
        for (let x = x0 + 1.2; x < x1 - 1.5; x += 2.3) if (Math.abs(x) > 1.4) place(B, x, base, z1 - t - 1.1, 1, FURN.booth(0xc23b30));
        place(B, 0, base, z0 + 1.6, 0, [['counter', -6.5, 0, -0.35, 6.5, 1.05, 0.35, 0xd8cdb0], ['stainless', -6.49, 0, 0.3, 6.49, 0.95, 0.36, 0xffffff]]);
        for (let x = -6; x <= 6; x += 0.9) place(B, x, base, z0 + 2.45, 0, FURN.stool(0xc23b30));
        place(B, -2, base, z0 + t + 0.35, 0, FURN.kitchen(0xd8dcde)); place(B, 3, base, z0 + t + 0.35, 0, FURN.kitchen(0xd8dcde));
        for (let x = -6; x <= 6; x += 2) { B.color(0xfff6e0); B.box('lamp', x - 0.4, top - 0.07, -0.2, x + 0.4, top - 0.03, 0.2); }
        // Rooftop neon sign.
        B.color(0x2a2a2a); B.box('metal', -0.1, top + 0.25, -0.1, 0.1, top + 1.0, 0.1); B.box('metal', -2.9, top + 1.0, -0.12, 2.9, top + 2.4, 0.12);
        B.sign(M.sign('DINER', '#1c1c1c', '#ff5a4a', true), 0, top + 1.7, 0.13, 5.6, 1.3, '+z');
        B.sign(M.sign('DINER', '#1c1c1c', '#ff5a4a', true), 0, top + 1.7, -0.13, 5.6, 1.3, '-z');
        car(B, -6, z1 + 5.5, 0, pick(rnd, CARS)); car(B, 6.5, z1 + 5.5, 2, pick(rnd, CARS), 'pickup');
      } },
    { id: 'gas', name: 'Gas station', kind: 'business', biome: 'any roadside', size: [24, 20], blurb: 'A flat canopy on four columns over two pump islands, a block kiosk with a glass front, a tall price sign, and an oil-stained apron.',
      build(B, rnd, M) {
        const brand = pick(rnd, [['BREEZE', '#1d4f8f', '#ffffff', 0x1d4f8f], ['CRESCENT', '#2f8a48', '#ffffff', 0x2f8a48], ['SUNSTAR', '#c23b30', '#ffd34a', 0xc23b30]]);
        // Apron and the canopy.
        B.color(0xa9a59c); B.box('concrete', -12, 0, -4, 12, 0.05, 14);
        for (const [x, z] of [[-5.5, 3.5], [5.5, 3.5], [-5.5, 9.5], [5.5, 9.5]]) { B.color(0xe8e6e0); B.box('metal', x - 0.22, 0, z - 0.22, x + 0.22, 5.0, z + 0.22); }
        B.color(0xf2f0ea); B.box('metal', -8, 5.0, 1.8, 8, 5.25, 11.2);
        B.color(brand[3]); B.box('paint', -8.05, 5.25, 1.75, 8.05, 6.0, 11.25, 'y');
        B.sign(M.sign(brand[0], brand[1], brand[2]), 0, 5.62, 11.27, 6, 0.7, '+z');
        B.sign(M.sign(brand[0], brand[1], brand[2]), 0, 5.62, 1.73, 6, 0.7, '-z');
        for (let x = -6; x <= 6; x += 3) { B.color(0xfff6e0); B.box('lamp', x - 0.6, 4.96, 4, x + 0.6, 5.0, 4.6); B.box('lamp', x - 0.6, 4.96, 8.4, x + 0.6, 5.0, 9.0); }
        for (const x of [-3.2, 3.2]) { place(B, x, 0, 6.5, 0, FURN.pump(), 'dressing'); place(B, x, 0.2, 5.5, 0, [['metal', -0.3, 0, -0.25, 0.3, 1.55, 0.25, 0xe8e6e0], ['metal', -0.31, 1.0, -0.26, 0.31, 1.4, 0.26, brand[3]]], 'dressing'); }
        // Kiosk.
        house(B, rnd, { z: -8.5, w: 10, d: 7, base: 0.15, fh: 3.6, wall: 'block', tint: 0xf0ece2, front: [[['shop', 6.5, 3.0, 0, { door: 1 }], ['door', 0.9, 2.1, 0, { doorMat: 'metal', doorTint: 0x8d9196, stoop: false }]]], roof: { type: 'flat', parapet: 0.9, ac: [2.5, -9] }, plan: [[{ k: 'shop', w: 3 }, { k: 'office', w: 1 }]], floorMat: 'floorTile', sideWin: 0 });
        B.color(brand[3]); B.box('paint', -5.05, 3.75, -5.0, 5.05, 4.65, -4.98);
        B.sign(M.sign('FOOD MART', brand[1], brand[2]), 0, 4.2, -4.975, 4.5, 0.7, '+z');
        place(B, -1.5, 0.15, -9.5, 0, FURN.aisle(rnd)); place(B, -1.5, 0.15, -7.6, 0, FURN.aisle(rnd)); place(B, 2.5, 0.15, -6.3, 2, FURN.counter(0xc8c0b0)); place(B, -2.0, 0.15, -11.4, 0, FURN.cooler());
        // Price pylon.
        B.color(0xe8e6e0); B.box('metal', 10.2, 0, 11.5, 10.5, 7.5, 11.8); B.box('metal', 11.5, 0, 11.5, 11.8, 7.5, 11.8);
        B.color(0x1c1c1c); B.box('metal', 9.9, 4.2, 11.45, 12.1, 8.6, 11.85);
        B.sign(M.sign(brand[0], brand[1], brand[2]), 11.0, 7.9, 11.86, 2.1, 1.2, '+z');
        B.sign(M.sign('1.09 9/10', '#1c1c1c', '#ffd34a'), 11.0, 6.2, 11.86, 2.1, 0.8, '+z');
        B.sign(M.sign('1.19 9/10', '#1c1c1c', '#ffd34a'), 11.0, 5.0, 11.86, 2.1, 0.8, '+z');
        // Air and water post, ice chest, cars.
        B.color(0xc23b30); B.box('metal', -10.5, 0, -2, -10.1, 1.3, -1.6);
        B.color(0xf0f0f0); B.box('metal', 5.2, 0.15, -4.5, 6.6, 1.2, -3.8); B.sign(M.sign('ICE', '#2f6f8f', '#ffffff'), 5.9, 0.8, -3.79, 1.2, 0.5, '+z');
        car(B, -3.2, 9.0, 0, pick(rnd, CARS)); car(B, 3.2, 4.0, 2, pick(rnd, CARS), 'pickup');
      } },
    { id: 'corner', name: 'Corner store', kind: 'business', biome: 'towns', size: [10, 14], blurb: 'Two storeys of brick: a glass shop front under a striped awning with GROCERY on the sign band, an apartment above, a corbelled cornice.',
      build(B, rnd, M) {
        const W = 9.5, D = 13, fh0 = 4.0, fh1 = 3.0, t = 0.3;
        const x0 = -W / 2, x1 = W / 2, z0 = -D / 2, z1 = D / 2, base = 0.15;
        const brick = pick(rnd, [0xffffff, 0xd8c0b0, 0xe8d8c8]);
        B.color(0xbab4aa); B.box('concrete', x0, -0.3, z0, x1, base, z1);
        const y1 = base + fh0, y2 = y1 + fh1;
        B.color(brick); wall(B, 'brick', 'x', z1 - t / 2, x0, x1, t, base, y1, 1, [{ kind: 'shop', a: x0 + 0.6, b: x1 - 2.2, y0: base, y1: base + 3.0, door: 2 }, { kind: 'door', a: x1 - 1.6, b: x1 - 0.7, y0: base, y1: base + 2.2, doorTint: 0x2a3a2a, stoop: false }], { inner: 'plaster', innerPad: t });
        B.color(brick); wall(B, 'brick', 'x', z1 - t / 2, x0, x1, t, y1, y2, 1, row(x0 + 0.4, x1 - 0.4, y1, [['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6, frameTint: 0xf4f1ea }], ['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6, frameTint: 0xf4f1ea }], ['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6, frameTint: 0xf4f1ea }]]), { inner: 'plaster', innerPad: t });
        for (const [yb, h] of [[base, fh0], [y1, fh1]]) {
          const side = row(z0 + 0.6, z1 - 0.6, yb, yb === base ? [['shop', 3.0, 3.0, 0], ['win', 1.0, 1.6, 1.0], ['win', 1.0, 1.6, 1.0]] : [['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6 }], ['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6 }], ['win', 1.0, 1.7, 0.7, { lintel: 'concrete', lintelTint: 0xd8d2c6 }]]);
          B.color(brick); wall(B, 'brick', 'z', x1 - t / 2, z0 + t, z1 - t, t, yb, yb + h, 1, side, { inner: 'plaster' });
          B.color(brick); wall(B, 'brick', 'z', x0 + t / 2, z0 + t, z1 - t, t, yb, yb + h, -1, [], { inner: 'plaster' });
          B.color(brick); wall(B, 'brick', 'x', z0 + t / 2, x0, x1, t, yb, yb + h, -1, yb === base ? [{ kind: 'door', a: -0.5, b: 0.5, y0: base, y1: base + 2.1, doorMat: 'metal', doorTint: 0x6a6e72, stoop: false }] : row(x0 + 1, x1 - 1, yb, [['win', 1.0, 1.4, 0.9], ['win', 1.0, 1.4, 0.9]]), { inner: 'plaster', innerPad: t });
        }
        flatRoof(B, x0, x1, z0, z1, y2, { wall: 'brick', wallTint: brick, parapet: 1.0, t });
        // Cornice and sign band.
        B.color(0xd8d2c6); B.box('concrete', x0 - 0.15, y2 + 0.55, z1 - 0.1, x1 + 0.15, y2 + 0.8, z1 + 0.3); for (let x = x0; x < x1; x += 0.6) B.box('concrete', x, y2 + 0.35, z1, x + 0.25, y2 + 0.55, z1 + 0.2);
        B.color(0x2a2f2a); B.box('paint', x0 + 0.3, base + 3.2, z1, x1 - 0.3, base + 3.9, z1 + 0.06);
        B.sign(M.sign('GROCERY', '#2a2f2a', '#f2d06a'), 0, base + 3.55, z1 + 0.065, 6.5, 0.6, '+z');
        // Striped awning, a sloped canvas face.
        B.color(0xffffff); slope(B, M.awning(pick(rnd, ['#c23b30', '#2f6f8f', '#2f8a48'])), [[x0 + 0.5, base + 2.4, z1 + 1.4], [x1 - 2.1, base + 2.4, z1 + 1.4], [x1 - 2.1, base + 3.1, z1 + 0.05], [x0 + 0.5, base + 3.1, z1 + 0.05]], 'fabric', 0.03);
        // Floors and inside.
        B.color(0xffffff); B.box('floorTile', x0 + t, base - 0.03, z0 + t, x1 - t, base + 0.012, z1 - t);
        B.color(0xe6e0d4); B.box('plaster', x0 + t, y1 - 0.25, z0 + t, x1 - t, y1 - 0.03, z1 - t); B.color(0xffffff); B.box('floorWood', x0 + t, y1 - 0.03, z0 + t, x1 - t, y1 + 0.012, z1 - t);
        B.color(0xf2efe8); B.box('plaster', x0 + t, y2 - 0.012, z0 + t, x1 - t, y2 + 0.05, z1 - t, 'Y');
        place(B, -1.5, base, 0.5, 1, FURN.aisle(rnd)); place(B, 1.2, base, 0.5, 1, FURN.aisle(rnd)); place(B, x0 + 0.75, base, -2, 1, FURN.cooler()); place(B, 2.5, base, z1 - 1.6, 2, FURN.counter(0x6a4a30));
        for (const z of [-3, 1, 4]) { B.color(0xffffff); B.box('lamp', -2.5, y1 - 0.3, z - 0.15, 2.5, y1 - 0.25, z + 0.15); }
        place(B, -2, y1, -3, 0, FURN.sofa(0x6a7a3a)); place(B, 2, y1, -4.5, 0, FURN.bed(0x9a4a3a)); place(B, 0, y1, 2, 0, FURN.table());
        // Fire escape on the side.
        B.color(0x2a2a2a); B.box('metal', x1, y1 + 0.0, -2.5, x1 + 1.1, y1 + 0.06, 2.5);
        for (let z = -2.5; z <= 2.5; z += 0.25) B.box('metal', x1 + 1.05, y1, z - 0.015, x1 + 1.08, y1 + 1.0, z + 0.015);
        B.box('metal', x1 + 1.03, y1 + 1.0, -2.5, x1 + 1.1, y1 + 1.05, 2.5);
      } },
    { id: 'garage', name: 'Auto repair', kind: 'business', biome: 'towns, roadside', size: [16, 14], blurb: 'Painted concrete block with three roll-up doors, an office with a shop window, a sign band, a lift and tyre stacks inside.',
      build(B, rnd, M) {
        house(B, rnd, { w: 15, d: 11, base: 0.1, fh: 4.6, t: 0.25, wall: 'block', tint: pick(rnd, [0xe8e2d0, 0xd8dcd8, 0xe8d8b8]), front: [[['roll', 3.2, 3.6, 0], ['roll', 3.2, 3.6, 0], ['roll', 3.2, 3.6, 0], ['shop', 2.2, 2.6, 0.4], ['door', 0.9, 2.1, 0, { doorMat: 'metal', doorTint: 0x8d9196, stoop: false }]]], roof: { type: 'flat', parapet: 1.0 }, plan: [[{ k: 'bay', w: 3 }, { k: 'office', w: 1 }]], floorMat: 'concrete', sideWin: 2 });
        B.color(0x1d4f8f); B.box('paint', -7.55, 4.75, 5.5, 7.55, 5.6, 5.56);
        B.sign(M.sign('AUTO REPAIR', '#1d4f8f', '#ffffff'), -1.5, 5.18, 5.565, 8.5, 0.7, '+z');
        place(B, -4.8, 0.1, 0, 0, FURN.lift()); car(B, -4.8, 0, 0, pick(rnd, CARS));
        place(B, 0.1, 0.1, -4.6, 0, FURN.workbench()); place(B, 3.5, 0.1, -4.2, 0, FURN.tires()); place(B, 4.2, 0.1, -4.2, 0, FURN.tires());
        B.color(0xa9a59c); B.box('concrete', -7.5, 0, 5.5, 7.5, 0.05, 13);
        B.color(0x3a3a3a); for (let i = 0; i < 3; i++) B.box('concrete', -6 + rnd() * 9, 0.051, 7 + rnd() * 4, -5 + rnd() * 9, 0.052, 8 + rnd() * 4);
        car(B, 4.5, 9.5, 1, pick(rnd, CARS), 'pickup');
        for (let i = 0; i < 6; i++) { B.color(0x1c1c1c); B.cyl('rubber', 'y', [6.6 + (i % 2) * 0.75, 0, -6.2 - Math.floor(i / 2) * 0.75], 0.34, 0, 0.22 + (i % 3) * 0.22, 10); }
      } },
    { id: 'motel', name: 'Motor court motel', kind: 'business', biome: 'roadside, beach', size: [32, 20], blurb: 'Two storeys of rooms off an open walkway with a railing, a door, window and air conditioner per room, the office at one end, a pool, and a neon MOTEL pylon.',
      build(B, rnd, M) {
        const rooms = 7, rw = 3.8, L = rooms * rw, D = 8, fh = 2.9, base = 0.15, t = 0.25;
        const x0 = -L / 2 - 2, x1 = x0 + L, z0 = -5, z1 = z0 + D;
        const tint = pick(rnd, [0xf2ead8, 0xeadfd0, 0xe8e0c8]), door = pick(rnd, [0x2f6f8f, 0xc23b30, 0x2f8a48, 0xe07a2a]);
        B.color(0xbab4aa); B.box('concrete', x0, -0.3, z0, x1, base, z1 + 2.0);
        for (let s = 0; s < 2; s++) {
          const yb = base + s * fh, yt = yb + fh;
          const ops = [];
          for (let i = 0; i < rooms; i++) { const a = x0 + i * rw; ops.push({ kind: 'door', a: a + 0.5, b: a + 1.4, y0: yb, y1: yb + 2.1, doorTint: door, doorMat: 'door', stoop: false }); ops.push({ kind: 'win', a: a + 1.9, b: a + 3.4, y0: yb + 0.9, y1: yb + 2.2, ac: true, frameMat: 'metal', frameTint: 0xb8bcc0, sill: false, rail: false }); }
          B.color(tint); wall(B, 'block', 'x', z1 - t / 2, x0, x1, t, yb, yt, 1, ops, { inner: 'plaster', innerPad: t });
          B.color(tint); wall(B, 'block', 'x', z0 + t / 2, x0, x1, t, yb, yt, -1, Array.from({ length: rooms }, (_, i) => ({ kind: 'win', a: x0 + i * rw + 1.5, b: x0 + i * rw + 2.3, y0: yb + 1.5, y1: yb + 2.2, sill: false, rail: false })), { inner: 'plaster', innerPad: t });
          B.color(tint); wall(B, 'block', 'z', x0 + t / 2, z0 + t, z1 - t, t, yb, yt, -1, [], { inner: 'plaster' });
          B.color(tint); wall(B, 'block', 'z', x1 - t / 2, z0 + t, z1 - t, t, yb, yt, 1, [], { inner: 'plaster' });
          for (let i = 1; i < rooms; i++) { B.color(B.inside); B.box('plaster', x0 + i * rw - 0.06, yb, z0 + t, x0 + i * rw + 0.06, yt - 0.01, z1 - t); }
          for (let i = 0; i < rooms; i++) { const cx = x0 + i * rw + rw / 2; place(B, cx - 0.4, yb, z0 + t + 1.2, 0, FURN.bed(0x8a6a3a)); place(B, cx + 1.1, yb, z1 - 1.0, 2, FURN.tv()); B.color(0xfff6e0); B.box('lamp', cx - 0.3, yt - 0.06, -1.3, cx + 0.3, yt - 0.012, -0.7); }
          B.color(0xe6e0d4); B.box('plaster', x0 + t, yb - 0.22, z0 + t, x1 - t, yb - 0.03, z1 - t, s ? 'Y' : ''); B.color(0xffffff); B.box('carpet', x0 + t, yb - 0.03, z0 + t, x1 - t, yb + 0.012, z1 - t);
        }
        const top = base + 2 * fh;
        B.color(0xf2efe8); B.box('plaster', x0 + t, top - 0.012, z0 + t, x1 - t, top + 0.06, z1 - t, 'Y');
        // The upper walkway: a slab on posts with a railing, and a stair at the end.
        B.color(0xcac4b8); B.box('concrete', x0, base + fh - 0.2, z1, x1, base + fh, z1 + 1.8);
        for (let x = x0 + 0.2; x <= x1; x += rw) { B.color(0xe8e6e0); B.box('metal', x - 0.08, base, z1 + 1.6, x + 0.08, top + 0.2, z1 + 1.76); }
        B.color(door); B.box('metal', x0, base + fh + 0.95, z1 + 1.66, x1, base + fh + 1.02, z1 + 1.74);
        for (let x = x0 + 0.1; x < x1; x += 0.14) B.box('metal', x - 0.012, base + fh, z1 + 1.69, x + 0.012, base + fh + 0.95, z1 + 1.71);
        B.color(0xcac4b8); for (let i = 0; i < 16; i++) B.box('concrete', x1 + 0.1, base + i * fh / 16, z1 + 1.8 - i * 0.28 - 0.28, x1 + 1.3, base + (i + 1) * fh / 16, z1 + 1.8 - i * 0.28);
        // A wide shed roof over the walkway, the main roof hipped.
        hipRoof(B, { x0, x1, z0, z1: z1 + 1.8, y: top + 0.2, pitch: 0.28, over: 0.6, mat: 'tile', tint: 0xb85a3a });
        // Office at the end, glass front.
        house(B, rnd, { x: x1 + 4.2, z: -1.0, w: 5.6, d: 7, base, fh: 3.4, wall: 'block', tint, front: [[['shop', 3.6, 2.8, 0, { door: 0 }]]], roof: { type: 'flat', parapet: 0.6 }, plan: [['office']], floorMat: 'floorTile', sideWin: 1 });
        place(B, x1 + 4.2, base, -1.2, 2, FURN.counter(0x6a4a30));
        // Pool and the pylon.
        B.color(0xd8d2c6); B.box('concrete', x0 + 4, 0, z1 + 4.5, x0 + 16, 0.12, z1 + 11);
        B.color(0xffffff); B.box('water', x0 + 5, 0.02, z1 + 5.5, x0 + 15, 0.1, z1 + 10);
        B.color(0xf2f0ea); for (let i = 0; i < 4; i++) B.box('fabric', x0 + 5.5 + i * 2.4, 0.12, z1 + 10.3, x0 + 6.3 + i * 2.4, 0.45, z1 + 10.9);
        const px = x1 + 7, pz = z1 + 9;
        B.color(0xe8e6e0); B.box('metal', px - 0.2, 0, pz - 0.2, px + 0.2, 8.99, pz + 0.2);
        B.color(0x1c1c1c); B.box('metal', px - 2.4, 5.5, pz - 0.25, px + 2.4, 9.0, pz + 0.25);
        B.sign(M.sign('MOTEL', '#1c1c1c', '#4ae0ff', true), px, 7.6, pz + 0.26, 4.6, 1.8, '+z');
        B.sign(M.sign('VACANCY', '#1c1c1c', '#ff5aa8', true), px, 6.1, pz + 0.26, 4.0, 0.8, '+z');
        B.sign(M.sign('MOTEL', '#1c1c1c', '#4ae0ff', true), px, 7.6, pz - 0.26, 4.6, 1.8, '-z');
        for (let i = 0; i < 4; i++) car(B, x0 + 2 + i * 5.5, z1 + 3.6 + (i % 2 ? 0 : 0.2), 0, pick(rnd, CARS), i === 2 ? 'pickup' : 'sedan');
      } },
    { id: 'strip', name: 'Strip mall', kind: 'business', biome: 'towns', size: [36, 18], blurb: 'Four shops under one shingled mansard band and a walkway canopy on posts: a laundromat, a video rental, a pizzeria and a hair salon.',
      build(B, rnd, M) {
        const units = [['LAUNDROMAT', '#2f6f8f'], ['VIDEO', '#7a3fc0'], ['PIZZA', '#c23b30'], ['HAIR', '#d06a9a']];
        const uw = 8, L = units.length * uw, D = 12, H = 4.2, base = 0.15, t = 0.25;
        const x0 = -L / 2, x1 = L / 2, z0 = -D / 2, z1 = D / 2, top = base + H;
        B.color(0xbab4aa); B.box('concrete', x0, -0.3, z0, x1, base, z1 + 3);
        const ops = [];
        units.forEach((u, i) => { const a = x0 + i * uw; ops.push({ kind: 'shop', a: a + 0.6, b: a + uw - 0.6, y0: base, y1: base + 3.1, door: i % 2 === 0 ? 0 : 3 }); });
        const tint = pick(rnd, [0xe8dcc0, 0xd8c8a8, 0xe0d8c8]);
        B.color(tint); wall(B, 'stucco', 'x', z1 - t / 2, x0, x1, t, base, top, 1, ops, { inner: 'plaster', innerPad: t });
        B.color(tint); wall(B, 'block', 'x', z0 + t / 2, x0, x1, t, base, top, -1, units.map((_, i) => ({ kind: 'door', a: x0 + i * uw + 1, b: x0 + i * uw + 1.9, y0: base, y1: base + 2.1, doorMat: 'metal', doorTint: 0x8d9196, stoop: false })), { inner: 'plaster', innerPad: t });
        B.color(tint); wall(B, 'stucco', 'z', x0 + t / 2, z0 + t, z1 - t, t, base, top, -1, [], { inner: 'plaster' });
        B.color(tint); wall(B, 'stucco', 'z', x1 - t / 2, z0 + t, z1 - t, t, base, top, 1, [], { inner: 'plaster' });
        for (let i = 1; i < units.length; i++) { B.color(B.inside); B.box('plaster', x0 + i * uw - 0.08, base, z0 + t, x0 + i * uw + 0.08, top - 0.01, z1 - t); }
        flatRoof(B, x0, x1, z0, z1, top, { wall: 'stucco', wallTint: tint, parapet: 0.5, t, ac: [x0 + 6, 0] });
        B.color(0xffffff); B.box('floorTile', x0 + t, base - 0.03, z0 + t, x1 - t, base + 0.012, z1 - t);
        B.color(0xf2efe8); B.box('plaster', x0 + t, top - 0.012, z0 + t, x1 - t, top + 0.05, z1 - t, 'Y');
        // Walkway canopy with the mansard band, on posts.
        B.color(0xf2efe8); B.box('soffit', x0, top - 0.6, z1, x1, top - 0.45, z1 + 3);
        for (let x = x0 + 0.3; x <= x1; x += uw / 2) { B.color(0x6a4a30); B.box('wood', x - 0.12, base, z1 + 2.7, x + 0.12, top - 0.6, z1 + 2.94); }
        B.color(0x5a4a3c); slope(B, 'shingle', [[x0 - 0.2, top - 0.6, z1 + 3.3], [x1 + 0.2, top - 0.6, z1 + 3.3], [x1 + 0.2, top + 1.0, z1 + 2.5], [x0 - 0.2, top + 1.0, z1 + 2.5]]);
        B.color(0x5a4a3c); B.box('shingle', x0 - 0.2, top + 0.85, z1, x1 + 0.2, top + 1.0, z1 + 2.5);
        units.forEach((u, i) => {
          const cx = x0 + i * uw + uw / 2;
          B.color(0xf4f1ea); B.box('metal', cx - 2.6, top - 0.35, z1 + 3.0, cx + 2.6, top + 0.55, z1 + 3.32);
          B.sign(M.sign(u[0], '#f4f1ea', u[1], true), cx, top + 0.1, z1 + 3.33, 5.0, 0.8, '+z');
          for (const z of [-2, 2]) { B.color(0xffffff); B.box('lamp', cx - 2.5, top - 0.08, z - 0.15, cx + 2.5, top - 0.03, z + 0.15); }
          if (i === 0) { for (let x = -3.2; x <= 3.2; x += 0.75) { place(B, cx + x, base, z0 + t + 0.4, 0, FURN.washer()); place(B, cx + x, base, z0 + t + 0.4 + 0.0, 0, [['metal', -0.34, 0.9, -0.34, 0.34, 1.75, 0.34, 0xf0eee8]]); } for (let x = -2.5; x <= 2.5; x += 0.75) place(B, cx + x, base, 0.5, 0, FURN.washer()); }
          if (i === 1) { for (const z of [-3, -0.5, 2]) place(B, cx, base, z, 0, FURN.aisle(rnd)); place(B, cx + 2, base, z1 - 1.3, 2, FURN.counter(0x3a3a4a)); }
          if (i === 2) { place(B, cx, base, z0 + 1.8, 0, FURN.counter(0xc23b30)); for (const z of [0.5, 3]) place(B, cx - 1.6, base, z, 0, FURN.table()), place(B, cx + 1.6, base, z, 0, FURN.table()); B.color(0x2a2a2a); B.box('metal', cx - 1.5, base, z0 + t, cx + 1.5, base + 1.6, z0 + t + 0.9); }
          if (i === 3) { for (let x = -2.4; x <= 2.4; x += 1.6) place(B, cx + x, base, z0 + 2.2, 0, FURN.chair(0xd06a9a)); B.color(0xffffff); B.box('glassShop', cx - 3, base + 1.0, z0 + t + 0.02, cx + 3, base + 2.2, z0 + t + 0.04); }
        });
        B.color(0x2b2b2b); B.box('asphalt', x0 - 2, 0, z1 + 3, x1 + 2, 0.04, z1 + 18);
        B.color(0xf2f0ea); for (let x = x0; x <= x1; x += 2.8) B.box('paint', x - 0.06, 0.041, z1 + 4, x + 0.06, 0.045, z1 + 9);
        for (let i = 0; i < 5; i++) car(B, x0 + 1.4 + Math.floor(rnd() * 11) * 2.8, z1 + 6.5, rnd() < 0.5 ? 0 : 2, pick(rnd, CARS), rnd() < 0.3 ? 'pickup' : 'sedan');
      } },
    { id: 'drivein', name: 'Burger drive-in', kind: 'business', biome: 'roadside', size: [16, 16], blurb: 'A swooping roof rising to the street over a glass front, red and white panels, a drive-through window, and a big arrow sign.',
      build(B, rnd, M) {
        const W = 12, D = 9, base = 0.15, t = 0.25;
        const x0 = -W / 2, x1 = W / 2, z0 = -D / 2, z1 = D / 2;
        const Hf = 4.6, Hb = 3.2;
        B.color(0xbab4aa); B.box('concrete', x0, -0.3, z0, x1, base, z1);
        B.color(0xffffff); wall(B, 'tileWall', 'x', z1 - t / 2, x0, x1, t, base, base + Hf - 0.4, 1, [{ kind: 'shop', a: x0 + 0.5, b: x1 - 0.5, y0: base, y1: base + 3.8, door: 3 }], { inner: 'tileWall', innerPad: t });
        B.color(0xffffff); wall(B, 'tileWall', 'x', z0 + t / 2, x0, x1, t, base, base + Hb - 0.2, -1, [{ kind: 'door', a: -2, b: -1.1, y0: base, y1: base + 2.1, doorMat: 'metal', doorTint: 0x8d9196, stoop: false }], { inner: 'tileWall', innerPad: t });
        for (const [x, out] of [[x0 + t / 2, -1], [x1 - t / 2, 1]]) {
          B.color(0xc23b30); wall(B, 'tileRed', 'z', x, z0 + t, z1 - t, t, base, base + Hb - 0.2, out, out > 0 ? [{ kind: 'win', a: -1.0, b: 0.6, y0: base + 1.0, y1: base + 2.2, frameMat: 'metal', sill: false, rail: false }] : [], { inner: 'tileWall' });
          // The wedge above the side wall, up to the front height.
          const hint = [out, 0, 0];
          // 6 mm proud of the side wall, so it never shares a plane with the front wall's end behind it.
          const xw = x + out * (t / 2 + 0.006);
          B.color(0xc23b30); B.poly('tileRed', [[xw, base + Hb - 0.2, z0], [xw, base + Hb - 0.2, z1], [xw, base + Hf - 0.4, z1]], hint);
          B.color(0xf2efe8); B.poly('tileWall', [[x - out * t / 2, base + Hb - 0.2, z0 + t], [x - out * t / 2, base + Hb - 0.2, z1 - t], [x - out * t / 2, base + Hf - 0.4, z1 - t]], [-out, 0, 0]);
        }
        // The roof: a single sweep rising to the front, white with a red fascia.
        B.color(0xf2f0ea); slope(B, 'metalRoof', [[x0 - 0.8, base + Hb - 0.2, z0 - 0.6], [x1 + 0.8, base + Hb - 0.2, z0 - 0.6], [x1 + 0.8, base + Hf + 0.4, z1 + 2.2], [x0 - 0.8, base + Hf + 0.4, z1 + 2.2]], 'soffit', 0.2);
        B.color(0xc23b30); B.box('paint', x0 - 0.85, base + Hf - 0.05, z1 + 2.15, x1 + 0.85, base + Hf + 0.45, z1 + 2.25);
        B.sign(M.sign('BURGERS', '#c23b30', '#ffffff'), 0, base + Hf + 0.2, z1 + 2.26, 5.5, 0.48, '+z');
        B.color(0xffffff); B.box('floorTile', x0 + t, base - 0.03, z0 + t, x1 - t, base + 0.012, z1 - t);
        place(B, 0, base, z0 + 2.3, 0, FURN.counter(0xc23b30)); place(B, -2.5, base, z0 + t + 0.35, 0, FURN.kitchen(0xd8dcde)); place(B, 2.5, base, z0 + t + 0.35, 0, FURN.kitchen(0xd8dcde));
        for (const x of [-3.5, 0, 3.5]) place(B, x, base, z1 - 1.6, 0, FURN.booth(0xe8b23a));
        for (const x of [-3, 0, 3]) { B.color(0xfff6e0); B.box('lamp', x - 0.5, base + Hb, -0.2, x + 0.5, base + Hb + 0.05, 0.2); }
        // The arrow sign on a pole.
        B.color(0xe8e6e0); B.box('metal', 9.3, 0, 7.8, 9.6, 8.5, 8.1);
        B.color(0xe8b23a); B.box('metal', 7.4, 6.2, 7.75, 11.6, 8.4, 8.15);
        B.sign(M.sign('BURGERS', '#e8b23a', '#c23b30', true), 9.5, 7.3, 8.16, 4.0, 1.9, '+z');
        B.color(0xc23b30); B.poly('paint', [[11.6, 5.6, 8.2], [12.8, 7.3, 8.2], [11.6, 9.0, 8.2]], [0, 0, 1]); B.box('paint', 6.8, 5.8, 7.794, 11.6, 6.15, 8.106);
        // Drive-through lane and menu board.
        B.color(0x2b2b2b); B.box('asphalt', x1, 0, z0 - 2, x1 + 4, 0.04, z1 + 6);
        B.color(0x2a2a2a); B.box('metal', x1 + 3.3, 0, -4.5, x1 + 3.5, 2.2, -2.5); B.sign(M.sign('MENU', '#2a2a2a', '#ffd34a', true), x1 + 3.29, 1.6, -3.5, 1.9, 1.0, '-x');
        car(B, x1 + 2, 1.5, 0, pick(rnd, CARS));
      } },
    { id: 'quonset', name: 'Quonset workshop', kind: 'business', biome: 'savanna, coast, any', size: [11, 22], blurb: 'A corrugated half-cylinder of war surplus, a block end wall with a sliding door and windows, now FEED & SEED.',
      build(B, rnd, M) {
        const R = 5.2, L = 20, seg = 16, z0 = -L / 2, z1 = L / 2;
        B.color(0xbab4aa); B.box('concrete', -R - 0.3, -0.3, z0, R + 0.3, 0.12, z1);
        B.color(0xb8bcbe);
        for (let i = 0; i < seg; i++) {
          const a = Math.PI * i / seg, b = Math.PI * (i + 1) / seg;
          const P = (ang, r, z) => [Math.cos(ang) * r, 0.12 + Math.sin(ang) * r, z];
          B.color(0xb8bcbe); B.poly('corrugatedArc', [P(a, R, z0), P(b, R, z0), P(b, R, z1), P(a, R, z1)], [Math.cos((a + b) / 2), Math.sin((a + b) / 2), 0], (p) => [Math.atan2(p[1] - 0.12, p[0]) * R, p[2]]);
          B.color(0xc8b89a); B.poly('woodInt', [P(a, R - 0.12, z0 + 0.2), P(b, R - 0.12, z0 + 0.2), P(b, R - 0.12, z1 - 0.2), P(a, R - 0.12, z1 - 0.2)], [-Math.cos((a + b) / 2), -Math.sin((a + b) / 2), 0]);
        }
        // End walls: a block rectangle with openings below 3.2 m, the arc cap and side slivers.
        const yS = 3.2, hw = Math.sqrt(R * R - (yS - 0.12) * (yS - 0.12));
        for (const [z, out] of [[z1 - 0.12, 1], [z0 + 0.12, -1]]) {
          const ops = out > 0 ? [{ kind: 'roll', a: -1.8, b: 1.8, y0: 0.12, y1: 3.0 }, { kind: 'win', a: -3.6, b: -2.5, y0: 1.2, y1: 2.3, frameTint: 0xf4f1ea }, { kind: 'win', a: 2.5, b: 3.6, y0: 1.2, y1: 2.3, frameTint: 0xf4f1ea }] : [{ kind: 'door', a: -0.45, b: 0.45, y0: 0.12, y1: 2.2, doorMat: 'metal', doorTint: 0x8d9196, stoop: false }];
          B.color(0xe8e2d0); wall(B, 'block', 'x', z, -hw, hw, 0.24, 0.12, yS, out, ops, { inner: 'plaster' });
          const cap = [], capIn = [];
          for (let i = 0; i <= 12; i++) { const ang = Math.asin((yS - 0.12) / R) + (Math.PI - 2 * Math.asin((yS - 0.12) / R)) * i / 12; cap.push([Math.cos(ang) * R, 0.12 + Math.sin(ang) * R, z + out * 0.12]); capIn.push([Math.cos(ang) * (R - 0.12), 0.12 + Math.sin(ang) * (R - 0.12), z - out * 0.126]); }
          B.color(0xe8e2d0); B.poly('block', cap, [0, 0, out]); B.color(B.inside); B.poly('plaster', capIn, [0, 0, -out]);
          for (const sx of [-1, 1]) {
            const sl = [[sx * hw, 0.12, z + out * 0.12]];
            for (let i = 0; i <= 6; i++) { const y = 0.12 + (yS - 0.12) * i / 6; sl.push([sx * Math.sqrt(Math.max(0, R * R - (y - 0.12) * (y - 0.12))), y, z + out * 0.12]); }
            sl.push([sx * hw, yS, z + out * 0.12]);
            B.color(0xe8e2d0); B.poly('block', sl, [0, 0, out]);
          }
        }
        B.color(0x6a2a24); B.box('paint', -3.4, 3.6, z1 + 0.01, 3.4, 4.5, z1 + 0.06);
        B.sign(M.sign('FEED & SEED', '#6a2a24', '#f2e8c8'), 0, 4.05, z1 + 0.065, 6.4, 0.8, '+z');
        place(B, 2.8, 0.12, 2, 1, FURN.workbench()); place(B, -3.4, 0.12, -4, 1, FURN.shelf()); place(B, -3.4, 0.12, -1, 1, FURN.shelf());
        B.color(0xd8c8a0); for (let i = 0; i < 18; i++) B.box('fabric', -2 + (i % 6) * 0.65, 0.12 + Math.floor(i / 6) * 0.3, -7, -1.45 + (i % 6) * 0.65, 0.4 + Math.floor(i / 6) * 0.3, -6.2);
        for (let z = -7; z <= 7; z += 3.5) { B.color(0xfff6e0); B.box('lamp', -0.4, 0.12 + R - 0.4, z - 0.3, 0.4, 0.12 + R - 0.35, z + 0.3); }
        car(B, -3, z1 + 5, 0, pick(rnd, CARS), 'pickup');
      } },
  ];

  // ------------------------------------------------------------------ surfaces
  // Every surface the kit draws: [stand-in generator, tile size m, roughness, metallic, normal
  // strength, stand-in base colour]. UVs are in metres, so a texture repeats every tile size.
  // The game binds each key to its Material Maker PBR set; until those exist it uses the colour.
  const SURFACES = {
    brick: ['brick', 0.9, 0.85, 0, 6, 0xa05a44],
    block: ['block', 0.8, 0.8, 0, 5, 0xd8d4cc],
    siding: ['siding', 1.0, 0.65, 0, 7, 0xece8e0],
    board: ['board', 1.0, 0.75, 0, 5, 0xb89a74],
    stucco: ['stucco', 2.0, 0.92, 0, 4, 0xe0d8c8],
    adobe: ['adobe', 2.5, 0.95, 0, 3, 0xd6a679],
    shingle: ['shingle', 1.0, 0.9, 0, 6, 0x5a5652],
    tile: ['tile', 1.0, 0.7, 0, 6, 0xb85a3a],
    metalRoof: ['metalRoof', 1.0, 0.45, 0.6, 5, 0xb8bcbe],
    corrugated: ['corrugated', 1.0, 0.45, 0.6, 3, 0xa8acae],
    corrugatedArc: ['corrugated', 1.0, 0.5, 0.6, 3, 0xb8bcbe],
    ribbed: ['ribbed', 1.0, 0.45, 0.4, 4, 0xe8e4da],
    skirt: ['skirt', 1.0, 0.6, 0.2, 3, 0xdcd6c6],
    concrete: ['concrete', 2.0, 0.9, 0, 2, 0xbab4aa],
    roofing: ['roofing', 1.0, 0.95, 0, 2, 0x6d6a66],
    asphalt: ['asphalt', 2.0, 0.95, 0, 2, 0x3a3a3c],
    trim: ['smooth', 1.0, 0.5, 0, 1, 0xf4f1ea],
    paint: ['smooth', 1.0, 0.4, 0, 1, 0xffffff],
    soffit: ['smooth', 1.0, 0.8, 0, 1, 0xf2efe8],
    plaster: ['smooth', 2.0, 0.9, 0, 1, 0xf0ece4],
    shutter: ['shutter', 0.5, 0.55, 0, 4, 0x3a4a3a],
    door: ['door', 1.0, 0.45, 0, 3, 0x6b3b22],
    metal: ['smooth', 1.0, 0.35, 0.7, 1, 0x9aa0a6],
    stainless: ['brushed', 1.0, 0.25, 1.0, 1, 0xd8dcde],
    counter: ['laminate', 1.0, 0.3, 0, 1, 0xe8e2d0],
    wood: ['wood', 1.0, 0.6, 0, 3, 0x8a6a46],
    deck: ['planks', 1.0, 0.75, 0, 4, 0xa98a64],
    floorWood: ['planks', 1.0, 0.35, 0, 3, 0xb08860],
    woodInt: ['pine', 1.0, 0.55, 0, 2, 0xc8a878],
    floorTile: ['floorTile', 0.9, 0.2, 0, 2, 0x9a9a9a],
    tileWall: ['wallTile', 1.0, 0.15, 0, 2, 0xf2f2f0],
    tileRed: ['wallTile', 1.0, 0.15, 0, 2, 0xc23b30],
    carpet: ['carpet', 1.0, 0.95, 0, 2, 0x8a7a6a],
    fabric: ['fabric', 0.5, 0.9, 0, 1, 0x8a7a6a],
    product: ['smooth', 1.0, 0.5, 0, 0, 0xc8a040],
    rubber: ['smooth', 1.0, 0.85, 0, 0, 0x222222],
    paintCar: ['smooth', 1.0, 0.3, 0.3, 0, 0x8a2c2a],
  };

  // ------------------------------------------------------------------ shared rules (mockup, exporter, game)
  // Load-bearing wall materials, for the collapse rule.
  const WALLMATS = ['brick', 'block', 'siding', 'board', 'stucco', 'adobe', 'ribbed', 'stainless', 'tileWall', 'tileRed', 'corrugatedArc'];
  // A piece is interior (dropped at LOD 1) if it is furniture, or every material in it is an
  // interior finish, or it is a thin tiled lining.
  const INTERIOR_MATS = new Set(['plaster', 'floorWood', 'floorTile', 'carpet', 'woodInt', 'lamp', 'counter', 'product', 'glassDark']);
  function isInterior(piece) {
    if (piece.tag === 'furniture') return true;
    if (piece.tag === 'car' || piece.tag === 'dressing') return false;
    if (piece.mats.length && piece.mats.every((m) => INTERIOR_MATS.has(m))) return true;
    const thin = Math.min(piece.hi[0] - piece.lo[0], piece.hi[1] - piece.lo[1], piece.hi[2] - piece.lo[2]) < 0.06;
    return thin && piece.mats.every((m) => m === 'tileWall');
  }
  // Support links between pieces of one building, from their bounds:
  // up[i]: pieces that i holds from below (i's top meets their bottom, overlapping in plan);
  // side[i]: pieces touching i that share enough of its height to hold it beside.
  // A piece stands while it can be reached from a piece on the ground through up links, and at
  // most MAX_SIDE side links (design section 6).
  const MAX_SIDE = 3, TOUCH_M = 0.04;
  function supportLinks(pieces) {
    const n = pieces.length, up = pieces.map(() => []), side = pieces.map(() => []);
    for (let a = 0; a < n; a++) {
      const A = pieces[a];
      for (let c = a + 1; c < n; c++) {
        const C = pieces[c], e = TOUCH_M;
        if (A.lo[0] - e > C.hi[0] || C.lo[0] - e > A.hi[0] || A.lo[1] - e > C.hi[1] || C.lo[1] - e > A.hi[1] || A.lo[2] - e > C.hi[2] || C.lo[2] - e > A.hi[2]) continue;
        const rests = (L, H) => L.hi[1] >= H.lo[1] - 0.12 && L.hi[1] <= H.lo[1] + 0.3;
        if (rests(A, C) && A.lo[1] < C.lo[1]) up[a].push(c);
        else if (rests(C, A) && C.lo[1] < A.lo[1]) up[c].push(a);
        const shared = Math.min(A.hi[1], C.hi[1]) - Math.max(A.lo[1], C.lo[1]);
        if (shared > Math.min(0.3, 0.5 * Math.min(A.hi[1] - A.lo[1], C.hi[1] - C.lo[1]))) { side[a].push(c); side[c].push(a); }
      }
    }
    return { up, side };
  }
  // Which pieces still stand, given which are alive (an array of booleans).
  function standing(pieces, links, alive) {
    const best = new Map(), queue = [];
    pieces.forEach((P, i) => { if (alive[i] && P.lo[1] <= 0.05) { best.set(i, 0); queue.push(i); } });
    for (let qi = 0; qi < queue.length; qi++) {
      const q = queue[qi], lat = best.get(q);
      for (const nb of links.up[q]) if (alive[nb] && (!best.has(nb) || best.get(nb) > 0)) { best.set(nb, 0); queue.push(nb); }
      if (lat < MAX_SIDE) for (const nb of links.side[q]) if (alive[nb] && (!best.has(nb) || best.get(nb) > lat + 1)) { best.set(nb, lat + 1); queue.push(nb); }
    }
    return best;
  }
  // The pieces of a built type with their bounds, materials, tag and triangle runs per material.
  function piecesOf(B) {
    const P = Array.from({ length: B.pid }, (_, i) => ({ id: i, lo: [1e9, 1e9, 1e9], hi: [-1e9, -1e9, -1e9], mats: [], runs: [], tag: B.tags[i] || null, meta: B.meta[i] || null }));
    for (const [mat, a] of Object.entries(B.parts)) {
      for (let t = 0; t < a.t.length;) {
        const id = a.t[t]; let k = 1;
        while (t + k < a.t.length && a.t[t + k] === id) k++;
        const p = P[id];
        if (!p.mats.includes(mat)) p.mats.push(mat);
        p.runs.push({ mat, start: t, count: k });
        for (let i = t * 9; i < (t + k) * 9; i += 3) for (let q = 0; q < 3; q++) { p.lo[q] = Math.min(p.lo[q], a.p[i + q]); p.hi[q] = Math.max(p.hi[q], a.p[i + q]); }
        t += k;
      }
    }
    return P;
  }

  // Builds one type into material buckets. M supplies sign and awning materials by name.
  // `variant` 0 is the type as first designed; 1 and 2 are its `variants` (the suburban houses have them).
  function build(id, seed, M, variant = 0) {
    const T = TYPES.find((t) => t.id === id);
    if (!T) throw new Error('unknown type ' + id);
    if (variant && !(T.variants && T.variants[variant - 1])) throw new Error(`${id} has no variant ${variant}`);
    const B = new Builder();
    const rnd = mulberry(seed * 2654435761 + id.length * 97 + variant * 7919);
    const MM = M || { sign: (txt) => 'sign:' + txt, awning: (c) => 'awning:' + c };
    T.build(B, rnd, MM, variant);
    return B;
  }
  // The id of a type's variant as exported: `ranch`, `ranch_b`, `ranch_c`.
  const variantId = (id, v) => (v ? `${id}_${String.fromCharCode(97 + v)}` : id);

  const api = { variantId, SURFACES, WALLMATS, INTERIOR_MATS, MAX_SIDE, isInterior, supportLinks, standing, piecesOf, TYPES, build, Builder, house, wall, row, hipRoof, gableRoof, flatRoof, slope, tiles, place, FURN, car, pilotFigure, mulberry };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.SmallBuildings = api;
})(this);
