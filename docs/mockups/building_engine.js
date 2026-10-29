// The building engine for Crab Breeze's 3D mockups: everything a page needs to draw buildings made with the
// small-buildings kit (docs/mockups/small_buildings_kit.js) and to break them. One implementation, used by the street
// (docs/mockups/small_buildings.html), the airport and the seaport (inlined by tools/mockups/gen_airport.py and
// gen_seaport.py). It holds: the surfaces' generated textures and materials, the merged meshes (one draw a material,
// each piece a run of triangles), each building's pieces and the kit's support rule (SmallBuildings.supportLinks and
// standing), the shot (pieces blown out, big ones broken into chunks), the creak and fall of what is left hanging,
// a building's collapse when it loses half its load-bearing walls, chunks and whole pieces falling and settling into
// rubble piles at the angle of repose, dust, fire and flashes, and firing where the player points.
//
// Generalised from the street page (owner, 2026-10-29: "Airport is not up to standard of like suburban houses", "Make
// sure everything in seaport follow destructible rule, Including boats"): a building stands at any height and any
// quarter turn; the ground under the rubble is the page's own (a quay, the seabed); and any three.js model (a ship, a
// crane) can be cut into pieces the same rule breaks (fromObject). Metres; y up.
(function (root) {
  'use strict';

  function BuildingEngine(opts) {
    const THREE = opts.THREE, K = opts.K, scene = opts.scene;
    const clip = opts.clip || null;
    const ground = opts.ground || (() => 0);      // the ground under (x, z) before any rubble: where pieces come to rest
    const clipping = clip ? [clip] : [];

    // ---------------------------------------------------------------- textures and materials
    const TS = 512;
    function noiseFn(seed) {
      const h = (x, y) => { let n = (x * 374761393 + y * 668265263 + seed * 1442695041) | 0; n = Math.imul(n ^ (n >>> 13), 1274126177); return ((n ^ (n >>> 16)) >>> 0) / 4294967296; };
      return (x, y, s) => { // tileable value noise, period TS/s cells
        const P = Math.max(1, Math.round(TS / s)); const fx = x / s, fy = y / s, ix = Math.floor(fx), iy = Math.floor(fy), u = fx - ix, v = fy - iy;
        const g = (a, b) => h(((a % P) + P) % P, ((b % P) + P) % P);
        const su = u * u * (3 - 2 * u), sv = v * v * (3 - 2 * v);
        return (g(ix, iy) * (1 - su) + g(ix + 1, iy) * su) * (1 - sv) + (g(ix, iy + 1) * (1 - su) + g(ix + 1, iy + 1) * su) * sv;
      };
    }
    // Each generator fills colour (rgb 0..1) and height (0..1) per pixel through put(x, y, r, g, b, h).
    function makeTex(gen, seed) {
      const col = new Uint8ClampedArray(TS * TS * 4), hgt = new Float32Array(TS * TS);
      const N = noiseFn(seed);
      const put = (x, y, r, g, b, h) => { const i = y * TS + x; col[i * 4] = r * 255; col[i * 4 + 1] = g * 255; col[i * 4 + 2] = b * 255; col[i * 4 + 3] = 255; hgt[i] = h; };
      for (let y = 0; y < TS; y++) for (let x = 0; x < TS; x++) gen(x, y, N, put);
      const toCanvas = (data) => { const c = document.createElement('canvas'); c.width = c.height = TS; c.getContext('2d').putImageData(new ImageData(data, TS, TS), 0, 0); return c; };
      return { col: toCanvas(col), hgt, toCanvas };
    }
    function normalFrom(hgt, strength, toCanvas) {
      const d = new Uint8ClampedArray(TS * TS * 4);
      const H = (x, y) => hgt[((y + TS) % TS) * TS + ((x + TS) % TS)];
      for (let y = 0; y < TS; y++) for (let x = 0; x < TS; x++) {
        const dx = (H(x + 1, y) - H(x - 1, y)) * strength, dy = (H(x, y + 1) - H(x, y - 1)) * strength;
        const l = Math.hypot(dx, dy, 1); const i = (y * TS + x) * 4;
        d[i] = (-dx / l * 0.5 + 0.5) * 255; d[i + 1] = (dy / l * 0.5 + 0.5) * 255; d[i + 2] = (1 / l * 0.5 + 0.5) * 255; d[i + 3] = 255;
      }
      return toCanvas(d);
    }
    const cl = (v) => Math.max(0, Math.min(1, v));
    // Generators. Tile sizes (metres) in the kit's SURFACES.
    const GEN = {
      brick: (x, y, N, put) => { // 12 courses of 75 mm, 4 bricks of 225 mm across a 0.9 m tile
        const ch = TS / 12, bw = TS / 4, row = Math.floor(y / ch), off = (row % 2) * bw / 2, bx = (x + off) % TS, col = Math.floor(bx / bw);
        const mx = bx - col * bw, my = y - row * ch, mortar = mx < 5 || my < 5;
        const v = N(col * 7 + row * 13, row * 3, 1) * 0.25 + N(x, y, 3) * 0.15 + N(x, y, 40) * 0.1;
        if (mortar) put(x, y, 0.72 + v * 0.1, 0.70 + v * 0.1, 0.66 + v * 0.1, 0.25 + N(x, y, 2) * 0.1);
        else { const k = 0.85 + v; put(x, y, cl(0.62 * k), cl(0.30 * k), cl(0.22 * k), 0.85 + N(x, y, 2) * 0.15); }
      },
      block: (x, y, N, put) => { const bh = TS / 4, bw = TS / 2, row = Math.floor(y / bh), bx = (x + (row % 2) * bw / 2) % TS; const m = (bx % bw) < 5 || (y % bh) < 5; const v = N(x, y, 2) * 0.12 + N(x, y, 30) * 0.06; put(x, y, 0.86 + v, 0.85 + v, 0.82 + v, m ? 0.3 : 0.8 + N(x, y, 3) * 0.2); },
      siding: (x, y, N, put) => { const bh = TS / 5, f = (y % bh) / bh; const v = 0.9 + N(x, y, 60) * 0.06 + N(x, y, 4) * 0.03; const h = f * 0.9 + (f < 0.06 ? -0.6 : 0); put(x, y, v, v, v * 0.98, cl(h)); },
      board: (x, y, N, put) => { const bw = TS / 3.3, f = (x % bw) / bw, batt = f < 0.12; const g = N(x * 0.2, y * 3, 4) * 0.15; const v = 0.82 + g; put(x, y, v, v * 0.95, v * 0.9, batt ? 1 : 0.5 + g); },
      stucco: (x, y, N, put) => { const v = 0.88 + N(x, y, 3) * 0.08 + N(x, y, 20) * 0.04; put(x, y, v, v, v * 0.97, N(x, y, 3) * 0.7 + N(x, y, 10) * 0.3); },
      adobe: (x, y, N, put) => { const v = 0.86 + N(x, y, 50) * 0.1 + N(x, y, 6) * 0.05; put(x, y, v, v * 0.97, v * 0.94, N(x, y, 50) * 0.5 + N(x, y, 6) * 0.3); },
      shingle: (x, y, N, put) => { const rh = TS / 7, row = Math.floor(y / rh), tw = TS / 3, tx = (x + (row % 2) * tw / 2) % TS, f = (y % rh) / rh; const slot = (tx % tw) < 6; const v = 0.55 + N(Math.floor(tx / tw) + row * 5, row, 1) * 0.12 + N(x, y, 2) * 0.12; put(x, y, v, v * 0.98, v * 0.96, slot ? 0.1 : f * 0.6 + N(x, y, 2) * 0.3); },
      tile: (x, y, N, put) => { const w = TS / 4, f = (x % w) / w, rh = TS / 3, g = (y % rh) / rh; const h = Math.sin(f * Math.PI) * 0.8 + g * 0.2; const v = 0.85 + N(Math.floor(x / w), Math.floor(y / rh), 1) * 0.12; put(x, y, v, v * 0.95, v * 0.9, h); },
      metalRoof: (x, y, N, put) => { const w = TS / 2.5, f = (x % w) / w, seam = f < 0.04; const v = 0.86 + N(x, y, 80) * 0.06; put(x, y, v, v, v, seam ? 1 : 0.4 + Math.sin(f * Math.PI * 6) * 0.03); },
      corrugated: (x, y, N, put) => { const v = 0.85 + N(x, y, 60) * 0.08; put(x, y, v, v, v, 0.5 + 0.5 * Math.sin(y / TS * Math.PI * 2 * 13)); },
      ribbed: (x, y, N, put) => { const v = 0.92 + N(x, y, 60) * 0.04; const f = (x % (TS / 6)) / (TS / 6); put(x, y, v, v, v, f < 0.15 ? 1 : 0.5); },
      skirt: (x, y, N, put) => { const f = (x % (TS / 8)) / (TS / 8); const v = 0.85 + N(x, y, 50) * 0.05; put(x, y, v, v, v * 0.97, f < 0.5 ? 0.8 : 0.3); },
      concrete: (x, y, N, put) => { const v = 0.78 + N(x, y, 4) * 0.08 + N(x, y, 40) * 0.08 + (N(x, y, 1.5) > 0.93 ? -0.15 : 0); put(x, y, v, v * 0.99, v * 0.96, N(x, y, 3) * 0.6 + N(x, y, 20) * 0.4); },
      roofing: (x, y, N, put) => { const v = 0.45 + N(x, y, 1.5) * 0.25; put(x, y, v, v, v, N(x, y, 1.5)); },
      asphalt: (x, y, N, put) => { const v = 0.28 + N(x, y, 1.5) * 0.12 + N(x, y, 60) * 0.06; put(x, y, v, v, v * 1.02, N(x, y, 1.5)); },
      smooth: (x, y, N, put) => { const v = 0.94 + N(x, y, 40) * 0.04; put(x, y, v, v, v, N(x, y, 8) * 0.2); },
      shutter: (x, y, N, put) => { const f = (y % (TS / 16)) / (TS / 16); const v = 0.92 + N(x, y, 30) * 0.05; put(x, y, v, v, v, f); },
      door: (x, y, N, put) => { const fx = (x % (TS / 2)) / (TS / 2), fy = (y % (TS / 2)) / (TS / 2); const p = fx > 0.15 && fx < 0.85 && fy > 0.1 && fy < 0.9; const v = 0.9 + N(x, y, 30) * 0.06; put(x, y, v, v, v, p ? 0.4 : 0.6); },
      brushed: (x, y, N, put) => { const v = 0.82 + N(x * 0.02, y * 4, 3) * 0.12; put(x, y, v, v, v * 1.01, N(x * 0.02, y * 4, 3) * 0.3); },
      laminate: (x, y, N, put) => { const v = 0.9 + N(x, y, 3) * 0.05; put(x, y, v, v, v, 0.5); },
      wood: (x, y, N, put) => { const g = N(x * 0.15, y * 2, 3) * 0.3 + Math.sin((x + N(x, y, 50) * 40) * 0.25) * 0.05; const v = 0.75 + g; put(x, y, v, v * 0.92, v * 0.85, g); },
      planks: (x, y, N, put) => { const w = TS / 7, k = Math.floor(x / w), f = (x % w) / w, gap = f < 0.04; const g = N(k * 13, y * 0.1, 3) * 0.2 + N(x * 0.2, y * 3, 4) * 0.1; const v = 0.72 + g; put(x, y, v, v * 0.9, v * 0.8, gap ? 0 : 0.6 + g * 0.5); },
      pine: (x, y, N, put) => { const w = TS / 6, k = Math.floor(x / w), f = (x % w) / w, gap = f < 0.03; const knot = N(x, y, 12) > 0.9 ? -0.25 : 0; const g = N(k * 7, y * 0.2, 3) * 0.12 + N(x * 0.3, y * 2, 5) * 0.08; const v = 0.86 + g + knot; put(x, y, v, v * 0.88, v * 0.7, gap ? 0 : 0.6); },
      floorTile: (x, y, N, put) => { const c = TS / 3, ix = Math.floor(x / c), iy = Math.floor(y / c), grout = (x % c) < 3 || (y % c) < 3; const chk = (ix + iy) % 2; const v = grout ? 0.6 : (chk ? 0.92 : 0.18) + N(x, y, 10) * 0.04; put(x, y, v, v, v * 0.98, grout ? 0.2 : 0.8); },
      wallTile: (x, y, N, put) => { const c = TS / 6.6, grout = (x % c) < 3 || (y % c) < 3; const v = grout ? 0.7 : 0.95; put(x, y, v, v, v, grout ? 0.2 : 0.8); },
      carpet: (x, y, N, put) => { const v = 0.75 + N(x, y, 1.5) * 0.15; put(x, y, v, v * 0.92, v * 0.85, N(x, y, 1.5)); },
      fabric: (x, y, N, put) => { const w = ((x % 6) < 3) !== ((y % 6) < 3); const v = 0.86 + (w ? 0.05 : 0) + N(x, y, 20) * 0.05; put(x, y, v, v, v, w ? 0.6 : 0.4); },
      grass: (x, y, N, put) => { const v = N(x, y, 2) * 0.25 + N(x, y, 30) * 0.2 + N(x, y, 120) * 0.15; put(x, y, cl(0.22 + v * 0.3), cl(0.36 + v * 0.35), cl(0.12 + v * 0.12), N(x, y, 2)); },
      screen: (x, y, N, put) => { const g = (x % 4) < 1 || (y % 4) < 1; put(x, y, 0.2, 0.2, 0.2, g ? 1 : 0); },
    };
    const MATS = K.SURFACES;     // the surface table lives in the kit (one table for the mockups, the exporter and the game)
    const materials = {};
    function texMat(key) {
      const [g, tile, rough, metal, ns] = MATS[key];
      const t = makeTex(GEN[g], key.length * 31 + g.length);
      const map = new THREE.CanvasTexture(t.col); map.encoding = THREE.sRGBEncoding;
      const nrm = new THREE.CanvasTexture(normalFrom(t.hgt, ns, t.toCanvas));
      for (const tx of [map, nrm]) { tx.wrapS = tx.wrapT = THREE.RepeatWrapping; tx.repeat.set(1 / tile, 1 / tile); tx.anisotropy = 8; }
      return new THREE.MeshStandardMaterial({ map, normalMap: nrm, roughness: rough, metalness: metal, vertexColors: true, clippingPlanes: clipping });
    }
    const glows = []; // [material, day, dusk, night]
    function special(key) {
      const base = { vertexColors: true, clippingPlanes: clipping };
      if (key === 'glass') return new THREE.MeshPhysicalMaterial({ ...base, color: 0x9ab4bc, roughness: 0.04, metalness: 0, transparent: true, opacity: 0.32, depthWrite: false, envMapIntensity: 1.6 });
      if (key === 'glassShop') return new THREE.MeshPhysicalMaterial({ ...base, color: 0xc8dade, roughness: 0.03, metalness: 0, transparent: true, opacity: 0.18, depthWrite: false, envMapIntensity: 1.4 });
      if (key === 'glassDark') return new THREE.MeshStandardMaterial({ ...base, color: 0x2a3640, roughness: 0.1, metalness: 0.2, envMapIntensity: 1.2 });
      if (key === 'screen') return new THREE.MeshStandardMaterial({ ...base, color: 0x222222, transparent: true, opacity: 0.45, depthWrite: false });
      if (key === 'water') return new THREE.MeshPhysicalMaterial({ ...base, color: 0x3aa6c8, roughness: 0.05, transparent: true, opacity: 0.8, depthWrite: false });
      if (key === 'lamp') { const m = new THREE.MeshStandardMaterial({ ...base, color: 0xffffff, emissive: 0xfff2d8, emissiveIntensity: 0.5 }); glows.push([m, 0.5, 2.2, 3.0]); return m; }
      if (key === 'lampRed' || key === 'lampGreen') { const m = new THREE.MeshStandardMaterial({ ...base, color: 0xffffff, emissive: key === 'lampRed' ? 0xff2a1a : 0x2aff4a, emissiveIntensity: 1.5 }); glows.push([m, 1.5, 2.5, 3.0]); return m; }
      return null;
    }
    // Signs and awnings, made on demand from their text.
    const SIGNS = {};
    const M = {
      sign(text, bg, fg, neon) {
        const key = `sign:${text}|${bg}|${fg}|${neon ? 1 : 0}`;
        if (!SIGNS[key]) {
          const c = document.createElement('canvas'); c.width = 512; c.height = 128;
          const g = c.getContext('2d'); g.fillStyle = bg; g.fillRect(0, 0, 512, 128);
          g.font = '700 84px "Saira Condensed", "Arial Narrow", sans-serif'; g.textAlign = 'center'; g.textBaseline = 'middle';
          if (neon) { g.shadowColor = fg; g.shadowBlur = 18; }
          g.fillStyle = fg; g.fillText(text, 256, 68, 480);
          if (neon) { g.shadowBlur = 0; g.lineWidth = 3; g.strokeStyle = '#ffffff'; g.globalAlpha = 0.5; g.strokeText(text, 256, 68, 480); g.globalAlpha = 1; }
          const map = new THREE.CanvasTexture(c); map.encoding = THREE.sRGBEncoding; map.anisotropy = 8;
          const m = new THREE.MeshStandardMaterial({ map, roughness: 0.5, vertexColors: true, clippingPlanes: clipping, emissive: 0xffffff, emissiveMap: map, emissiveIntensity: neon ? 0.35 : 0.0 });
          if (neon) glows.push([m, 0.35, 1.8, 2.4]); else glows.push([m, 0, 0.25, 0.45]);
          SIGNS[key] = m;
        }
        return key;
      },
      awning(color) {
        const key = `awning:${color}`;
        if (!SIGNS[key]) {
          const c = document.createElement('canvas'); c.width = c.height = 128; const g = c.getContext('2d');
          for (let i = 0; i < 8; i++) { g.fillStyle = i % 2 ? '#f4f1ea' : color; g.fillRect(i * 16, 0, 16, 128); }
          const map = new THREE.CanvasTexture(c); map.encoding = THREE.sRGBEncoding; map.wrapS = map.wrapT = THREE.RepeatWrapping; map.repeat.set(1 / 2.4, 1 / 2.4);
          SIGNS[key] = new THREE.MeshStandardMaterial({ map, roughness: 0.85, vertexColors: true, clippingPlanes: clipping, side: THREE.DoubleSide });
        }
        return key;
      },
    };
    function material(key) {
      if (materials[key]) return materials[key];
      let m = SIGNS[key] || special(key);
      if (!m) m = MATS[key] ? texMat(key) : texMat(key === 'paint' ? 'paint' : 'trim');
      materials[key] = m;
      return m;
    }

    // ---------------------------------------------------------------- buildings, pieces and the merged meshes
    // A building: { name, base (the height its ground floor stands on), lo, hi, pieces, walls, wallArea, collapsed,
    // collapse (null: the kit's walls rule, half the load-bearing wall area lost; or { test(P), frac }),
    // anchorBuilt (true: what the rule would not hold as drawn is held as built, see buildGraph),
    // sink (true: a ship; when it collapses it goes down whole, see sinkHulk) }.
    const BLD = [];
    // Every piece: { bld (-1 cannot break), alive, ranges, lo, hi, meta }.
    const PIECES = [];
    const merged = {};
    function building(info = {}) {
      BLD.push({ name: info.name || 'building', base: info.base || 0, collapse: info.collapse || null, info, pieces: [], walls: [], wallArea: 0, collapsed: false, lo: [1e9, 1e9, 1e9], hi: [-1e9, -1e9, -1e9] });
      return BLD.length - 1;
    }
    // Add a kit Builder's geometry at (x, y, z) turned by `turn` quarter turns about +y (1: local +x to world -z), as
    // building `bld` (-1: scenery that cannot break).
    function add(B, o = {}) {
      const ox = o.x || 0, oy = o.y || 0, oz = o.z || 0, turn = ((o.turn || 0) % 4 + 4) % 4, bld = o.bld == null ? -1 : o.bld;
      const cs = [1, 0, -1, 0][turn], sn = [0, 1, 0, -1][turn];
      const P3 = (x, y, z) => [ox + cs * x + sn * z, oy + y, oz - sn * x + cs * z];
      const D3 = (x, y, z) => [cs * x + sn * z, y, -sn * x + cs * z];
      const base = PIECES.length;
      for (let i = 0; i < B.pid; i++) {
        const mt = B.meta[i];
        let meta = null;
        if (mt && mt.box) {
          const [x0, y0, z0, x1, y1, z1] = mt.box, a = P3(x0, y0, z0), b = P3(x1, y1, z1);
          meta = { mat: mt.mat, tint: mt.tint, box: [Math.min(a[0], b[0]), a[1], Math.min(a[2], b[2]), Math.max(a[0], b[0]), b[1], Math.max(a[2], b[2])] };
        } else if (mt && mt.panel) { // a roof panel: an oriented box
          const Pn = mt.panel;
          meta = { mat: mt.mat, tint: mt.tint, panel: { c: P3(...Pn.c), size: Pn.size, axes: Pn.axes.map((v) => D3(...v)) } };
        }
        PIECES.push({ bld, alive: true, ranges: [], lo: [1e9, 1e9, 1e9], hi: [-1e9, -1e9, -1e9], meta, tag: B.tags[i] || null });
      }
      for (const [key, a] of Object.entries(B.parts)) {
        const m = merged[key] || (merged[key] = { p: [], n: [], u: [], c: [], t: [] });
        for (let i = 0; i < a.p.length; i += 3) { m.p.push(...P3(a.p[i], a.p[i + 1], a.p[i + 2])); m.n.push(...D3(a.n[i], a.n[i + 1], a.n[i + 2])); }
        for (const v of a.u) m.u.push(v);
        for (const v of a.c) m.c.push(v);
        for (const t of a.t) m.t.push(base + t);
      }
    }
    function boundsOf(B) {
      const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
      for (const a of Object.values(B.parts)) for (let i = 0; i < a.p.length; i += 3) for (let k = 0; k < 3; k++) { lo[k] = Math.min(lo[k], a.p[i + k]); hi[k] = Math.max(hi[k], a.p[i + k]); }
      return { lo, hi };
    }
    // A three.js model (a ship, a crane from its glb) as a kit Builder: its triangles in its own frame (the object's
    // matrix is applied, so place it with add()), cut into pieces of about `cell` metres by where each triangle lies,
    // each triangle in the 'paint' surface tinted with its material's colour. The pieces then stand and fall by the
    // same rule as a building's.
    // Sutherland-Hodgman: the part of a polygon on the keep side of the plane x[axis] = at (keep +1: >=, -1: <=).
    function clipPoly(poly, axis, at, keep) {
      const out = [], inside = (p) => (p[axis] - at) * keep >= -1e-9;
      for (let i = 0; i < poly.length; i++) {
        const a = poly[i], b = poly[(i + 1) % poly.length], ia = inside(a), ib = inside(b);
        if (ia) out.push(a);
        if (ia !== ib) { const t = (at - a[axis]) / (b[axis] - a[axis]); out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]); }
      }
      return out;
    }
    // o.interior (a ship): the model is a hollow shell, so the cut gets an inside (owner, 2026-10-29: "boats still mising
    // interiors/backface"): every face is doubled inward in dark steel, and the hull (its o.interior.hull materials) is
    // filled with decks every 2.8 m and bulkheads every 12 m along its length, found by vertical rays through the hull on
    // a 1.5 m grid. They are cut into the same cells, so a shot shows compartments, not an empty can.
    function fromObject(obj, cell = 6, o = {}) {
      const B = new K.Builder();
      obj.updateMatrixWorld(true);
      const inv = new THREE.Matrix4().copy(obj.matrixWorld).invert();
      const cells = new Map(), v = [new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3()];
      // A polygon clipped to every cell it crosses, so a cell's piece is exactly that cell of the model (no long slivers).
      const put = (key, col, poly0, n) => {
        const lo = [0, 1, 2].map((a) => Math.floor(Math.min(...poly0.map((q) => q[a])) / cell));
        const hi = [0, 1, 2].map((a) => Math.floor((Math.max(...poly0.map((q) => q[a])) - 1e-6) / cell));
        for (let i = lo[0]; i <= hi[0]; i++) for (let j = lo[1]; j <= hi[1]; j++) for (let kk = lo[2]; kk <= hi[2]; kk++) {
          let poly = poly0;
          const cmin = [i * cell, j * cell, kk * cell];
          for (let a = 0; a < 3 && poly.length >= 3; a++) { poly = clipPoly(poly, a, cmin[a], 1); if (poly.length >= 3) poly = clipPoly(poly, a, cmin[a] + cell, -1); }
          if (poly.length < 3) continue;
          const ck = i + ',' + j + ',' + kk;
          if (!cells.has(ck)) cells.set(ck, []);
          cells.get(ck).push({ key, col, pts: poly, n });
        }
      };
      const I_ = o.interior, hullTris = [], houseTris = [];
      let meshNo = 0;
      const inner = I_ ? (I_.inner || '#4a4844') : null, ins = I_ ? (I_.surface || 'metal') : null;
      obj.traverse((m) => {
        if (!m.isMesh || !m.visible) return;
        const g = m.geometry, P = g.attributes.position, I = g.index, mm = new THREE.Matrix4().multiplyMatrices(inv, m.matrixWorld);
        const col = (m.material && m.material.color) ? '#' + m.material.color.getHexString() : '#888888';
        const key = o.surface ? o.surface(m) : 'paint';
        const mname = (m.material && m.material.name) || '';
        const isHull = I_ && I_.hull && I_.hull.test(mname), isHouse = I_ && I_.house && I_.house.test(mname);
        // Each mesh of the model stands 6 mm further out along its faces' normals than the one before, so two parts whose
        // faces share a plane (a house flush with the hull's side, a deck on its sheer) never z-fight.
        const nudge = 0.006 * (meshNo++ % 5);
        const n = I ? I.count : P.count;
        for (let t = 0; t + 2 < n; t += 3) {
          for (let k = 0; k < 3; k++) v[k].fromBufferAttribute(P, I ? I.getX(t + k) : t + k).applyMatrix4(mm);
          let tri = v.map((q) => [q.x, q.y, q.z]);
          const c = new THREE.Vector3().subVectors(v[1], v[0]).cross(new THREE.Vector3().subVectors(v[2], v[0]));
          const nrm = [c.x, c.y, c.z];
          if (nudge) { const l = Math.hypot(...nrm) || 1; tri = tri.map((q) => [q[0] + nrm[0] / l * nudge, q[1] + nrm[1] / l * nudge, q[2] + nrm[2] / l * nudge]); }
          if (isHouse) houseTris.push(tri);
          put(key, col, tri, nrm);
          if (I_ && key !== 'glassDark') {   // the inside of the plate, 3 cm in
            const l = Math.hypot(...nrm) || 1, d = nrm.map((x) => -x / l * 0.03);
            put(ins, inner, tri.map((q) => [q[0] + d[0], q[1] + d[1], q[2] + d[2]]), nrm.map((x) => -x));
          }
          if (isHull) hullTris.push(tri);
        }
      });
      if (I_ && hullTris.length) {
        const lo = [1e9, 1e9, 1e9], hi = [-1e9, -1e9, -1e9];
        for (const t of hullTris) for (const q of t) for (let a = 0; a < 3; a++) { lo[a] = Math.min(lo[a], q[a]); hi[a] = Math.max(hi[a], q[a]); }
        const alongX = hi[0] - lo[0] > hi[2] - lo[2], step = 1.5, deckCol = I_.deck || '#6e5e4c', wallCol = I_.wall || '#7a7670';
        // the hull's span (bottom, top) under a point, from the hull faces a vertical line crosses
        const span = (x, z) => {
          let b = 1e9, t = -1e9;
          for (const [A, Bq, C] of hullTris) {
            const d = (Bq[0] - A[0]) * (C[2] - A[2]) - (C[0] - A[0]) * (Bq[2] - A[2]);
            if (Math.abs(d) < 1e-9) continue;
            const u = ((x - A[0]) * (C[2] - A[2]) - (C[0] - A[0]) * (z - A[2])) / d, w = ((Bq[0] - A[0]) * (z - A[2]) - (x - A[0]) * (Bq[2] - A[2])) / d;
            if (u < -1e-6 || w < -1e-6 || u + w > 1 + 1e-6) continue;
            const y = A[1] + u * (Bq[1] - A[1]) + w * (C[1] - A[1]);
            b = Math.min(b, y); t = Math.max(t, y);
          }
          return t - b > 1 ? [b, t] : null;
        };
        const spanIn = (tris) => (x, z) => { const old = hullTris.slice(); hullTris.length = 0; hullTris.push(...tris); const r = span(x, z); hullTris.length = 0; hullTris.push(...old); return r; };
        const houseSpan = spanIn(houseTris);
        const hash = (a, b, c) => { let h = Math.imul(Math.round(a * 7) * 73856093 ^ Math.round(b * 7) * 19349663 ^ Math.round(c * 7) * 83492791, 0x27d4eb2d); h ^= h >>> 15; return ((h >>> 0) % 1000) / 1000; };
        // a box of the fit-out, its six faces cut into the cells like everything else
        const box = (x0, y0, z0, x1, y1, z1, col, key = 'paint') => {
          const c = [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], [x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]];
          for (const [f, n] of [[[4, 5, 6, 7], [0, 1, 0]], [[0, 1, 2, 3], [0, -1, 0]], [[1, 2, 6, 5], [1, 0, 0]], [[0, 3, 7, 4], [-1, 0, 0]], [[3, 2, 6, 7], [0, 0, 1]], [[0, 1, 5, 4], [0, 0, -1]]]) put(key, col, f.map((i) => c[i]), n);
        };
        // the stern is the end the house stands at; the engine room is the hull's last fifth there
        let hc = 0, hn = 0; for (const t of houseTris) for (const q of t) { hc += alongX ? q[0] : q[2]; hn++; }
        const a0lo = alongX ? lo[0] : lo[2], a0hi = alongX ? hi[0] : hi[2], L = a0hi - a0lo;
        const sternAtHi = hn ? hc / hn > (a0lo + a0hi) / 2 : true;
        const fromStern = (a) => (sternAtHi ? a0hi - a : a - a0lo) / L;
        const S = new Map(), key = (x, z) => x.toFixed(2) + ',' + z.toFixed(2);
        for (let x = lo[0]; x < hi[0] + step; x += step) for (let z = lo[2]; z < hi[2] + step; z += step) S.set(key(x, z), span(x, z));
        // a deck tile at y over the column at (x, z) only where all four of its corners are inside the hull at that height,
        // pulled 15 cm in from the plating, so no deck pokes through the bilge or the sheer
        const inside = (x, z, y) => { for (const [dx, dz] of [[0, 0], [step, 0], [step, step], [0, step]]) { const q = S.get(key(x + dx, z + dz)); if (!q || y < q[0] + 0.3 || y > q[1] - 0.3) return false; } return true; };
        const cargoCols = [0x2f5f8f, 0xa8322a, 0x3f6e3a, 0x8a8a84, 0xc0682a, 0x6b4a2e];
        for (let x = lo[0]; x < hi[0]; x += step) for (let z = lo[2]; z < hi[2]; z += step) {
          const sp = span(x + step / 2, z + step / 2);
          if (!sp) continue;
          const e = 0.02, a = alongX ? x + step / 2 : z + step / 2, f = fromStern(a);
          let level = 0;
          for (let y = sp[0] + 1.2; y < sp[1] - 1.0; y += 2.8, level++) {
            if (!inside(x, z, y)) continue;
            const q = [[x + e, y, z + e], [x + step - e, y, z + e], [x + step - e, y, z + step - e], [x + e, y, z + step - e]];
            put(ins, deckCol, q, [0, 1, 0]); put(ins, inner, q.map((p) => [p[0], p[1] - 0.05, p[2]]), [0, -1, 0]);
            const r = hash(x, y, z), top = Math.min(y + 2.6, sp[1] - 0.4);
            if (f < 0.2) {   // the engine room: the engines, pumps and their pipes
              if (level === 0 && r < 0.55) box(x + 0.15, y, z + 0.15, x + step - 0.15, Math.min(top, y + 1.6 + r * 1.2), z + step - 0.15, r < 0.3 ? 0x3a3d42 : 0x5a6a7a, 'metal');
              else if (r < 0.15) box(x + 0.5, y, z + 0.5, x + step - 0.5, top, z + step - 0.5, 0x8d9196, 'metal');
            } else if (I_.cargo !== 'oil' && r < 0.7) {   // the holds: crates and boxes of cargo
              box(x + 0.1, y, z + 0.1, x + step - 0.1, Math.min(top, y + 0.9 + r * 1.6), z + step - 0.1, cargoCols[Math.floor(r * 97) % cargoCols.length], 'board');
            }
          }
          // bulkheads across the hull every 12 m (every 6 m in a tanker's tanks), inset from the plating
          const at0 = a - step / 2, every = I_.cargo === 'oil' ? 6 : 12, fs = at0 - a0lo;
          if (Math.floor(fs / every) !== Math.floor((fs + step) / every)) {
            const at = a0lo + Math.floor((fs + step) / every) * every, q0 = sp[0] + 0.3, q1 = sp[1] - 0.3;
            const ok = inside(x, z, q0) || inside(x, z, (q0 + q1) / 2);
            if (ok) {
              const q = alongX ? [[at, q0, z + e], [at, q0, z + step - e], [at, q1, z + step - e], [at, q1, z + e]] : [[x + e, q0, at], [x + step - e, q0, at], [x + step - e, q1, at], [x + e, q1, at]];
              const nn = alongX ? [1, 0, 0] : [0, 0, 1];
              put(ins, wallCol, q, nn); put(ins, wallCol, q.map((p) => alongX ? [p[0] - 0.05, p[1], p[2]] : [p[0], p[1], p[2] - 0.05]), nn.map((c) => -c));
            }
          }
        }
        // the house: a floor every 2.8 m, cabins (bunks, lockers, tables) below, the bridge's consoles on top
        if (houseTris.length) {
          let hb = 1e9, ht = -1e9; for (const t of houseTris) for (const q of t) { hb = Math.min(hb, q[1]); ht = Math.max(ht, q[1]); }
          const levels = []; for (let y = hb + 0.05; y < ht - 2.2; y += 2.8) levels.push(y);
          for (let x = lo[0]; x < hi[0]; x += step) for (let z = lo[2]; z < hi[2]; z += step) {
            const hs = houseSpan(x + step / 2, z + step / 2);
            if (!hs) continue;
            const e = 0.25;
            levels.forEach((y, li) => {
              if (y < hs[0] - 0.1 || y > hs[1] - 2.2) return;
              const hs2 = [houseSpan(x + e, z + e), houseSpan(x + step - e, z + step - e)];
              if (!hs2[0] || !hs2[1]) return;   // a column on the house's edge: its wall, no floor through it
              const q = [[x + 0.02, y, z + 0.02], [x + step - 0.02, y, z + 0.02], [x + step - 0.02, y, z + step - 0.02], [x + 0.02, y, z + step - 0.02]];
              if (li) { put(ins, '#8a7a66', q, [0, 1, 0]); put(ins, inner, q.map((p) => [p[0], p[1] - 0.05, p[2]]), [0, -1, 0]); }
              const r = hash(x, y + 1, z), bridge = li === levels.length - 1 || y + 2.8 > hs[1] - 2.2;
              if (bridge) {   // consoles with their screens, a chart table
                if (r < 0.45) { box(x + 0.2, y, z + 0.4, x + step - 0.2, y + 1.0, z + step - 0.4, 0x5a5f66, 'trim'); box(x + 0.35, y + 1.0, z + 0.55, x + step - 0.35, y + 1.45, z + step - 0.55, 0x2a3a44, 'glassDark'); }
              } else if (r < 0.3) { box(x + 0.15, y, z + 0.3, x + step - 0.15, y + 0.45, z + step - 0.3, 0x6a4a30, 'wood'); box(x + 0.2, y + 0.45, z + 0.35, x + step - 0.2, y + 0.6, z + step - 0.35, 0xe8e2d0, 'fabric'); }
              else if (r < 0.45) { box(x + 0.4, y, z + 0.4, x + step - 0.4, y + 0.75, z + step - 0.4, 0x8a6a46, 'wood'); }
              else if (r < 0.55) { box(x + 0.5, y, z + 0.2, x + step - 0.5, y + 1.9, z + 0.75, 0x5a6a7a, 'metal'); }
            });
          }
        }
      }
      // each cell's tint is the model's own colour factor, as its material draws it
      for (const tris of cells.values()) B.group(() => { for (const t of tris) { B.color(new THREE.Color(t.col).getHex()); B.poly(t.key, t.pts, t.n); } });
      return B;
    }

    const TRANSPARENT = new Set(['glass', 'glassShop', 'screen', 'water']);
    const MESH = {}, ORIG = {};
    let group = new THREE.Group(); scene.add(group);
    // Make the meshes from everything added, the buildings' support graphs and the rubble ground. Call once a scene is laid out.
    function finish() {
      for (const [key, a] of Object.entries(merged)) {
        for (let t = 0; t < a.t.length;) {      // each piece's runs of triangles in this bucket, and its bounds
          const pid = a.t[t]; let n = 1;
          while (t + n < a.t.length && a.t[t + n] === pid) n++;
          const P = PIECES[pid]; P.ranges.push({ key, start: t, count: n });
          for (let i = t * 9; i < (t + n) * 9; i += 3) for (let k = 0; k < 3; k++) { P.lo[k] = Math.min(P.lo[k], a.p[i + k]); P.hi[k] = Math.max(P.hi[k], a.p[i + k]); }
          t += n;
        }
        const g = new THREE.BufferGeometry();
        g.setAttribute('position', new THREE.Float32BufferAttribute(a.p, 3));
        g.setAttribute('normal', new THREE.Float32BufferAttribute(a.n, 3));
        g.setAttribute('uv', new THREE.Float32BufferAttribute(a.u, 2));
        g.setAttribute('color', new THREE.Float32BufferAttribute(a.c, 3));
        g.computeBoundingSphere();
        const m = new THREE.Mesh(g, material(key));
        const tr = TRANSPARENT.has(key);
        m.castShadow = !tr && key !== 'lamp'; m.receiveShadow = !tr;
        if (tr) m.renderOrder = 2;
        m.userData.key = key;
        MESH[key] = m; ORIG[key] = Float32Array.from(a.p);
        group.add(m);
      }
      buildGraph();
      resetGrid();
    }
    // Everything gone: for a page that lays out another scene (the airport's Today against Proposed).
    function clear() {
      scene.remove(group); group.traverse((o) => { if (o.geometry && o.geometry !== unitBox) o.geometry.dispose(); });
      group = new THREE.Group(); scene.add(group);
      for (const k of Object.keys(merged)) delete merged[k];
      for (const k of Object.keys(MESH)) { delete MESH[k]; delete ORIG[k]; }
      for (const k of Object.keys(INST)) delete INST[k];
      for (const k of Object.keys(RUBBLE)) delete RUBBLE[k];
      PIECES.length = 0; BLD.length = 0; FALLING.length = 0; HULKS.length = 0; SCORCH.length = 0; rubbleDirty.clear(); dirtyKeys.clear();
      GRID.clear(); stats.rubbleTris = 0;
      if (fxReady) addFx();
    }
    const WALLMATS = new Set(K.WALLMATS);
    // Support: the kit's links, built once a building. The load-bearing walls (for its collapse) are its wall-material
    // pieces standing on its ground floor and over 1.5 m tall, or what its own `collapse.test` picks.
    function buildGraph() {
      PIECES.forEach((P, i) => { P.up = []; P.side = []; if (P.bld >= 0 && P.ranges.length) { const b = BLD[P.bld]; b.pieces.push(i); for (let k = 0; k < 3; k++) { b.lo[k] = Math.min(b.lo[k], P.lo[k]); b.hi[k] = Math.max(b.hi[k], P.hi[k]); } } });
      for (const b of BLD) {
        const L = b.pieces;
        const rel = L.map((pid) => { const P = PIECES[pid]; return { lo: [P.lo[0], P.lo[1] - b.base, P.lo[2]], hi: [P.hi[0], P.hi[1] - b.base, P.hi[2]] }; });
        const links = K.supportLinks(rel);
        // A model cut into cells (a ship, a crane) or a structure on mixed footings (a quay's fenders and bollards) is held
        // as built: a piece the rule would not hold as drawn counts as on the ground (its links stay its real shape), so
        // it stays up until something it rests on is shot away.
        b.anchored = 0;
        if (b.info.anchorBuilt) {
          const st = K.standing(rel, links, rel.map(() => true), b.info.maxSide);
          rel.forEach((q, i) => { if (!st.has(i)) { b.anchored++; rel[i] = { lo: [q.lo[0], 0, q.lo[2]], hi: q.hi }; } });
        }
        b.rel = rel;
        L.forEach((pid, i) => {
          const A = PIECES[pid], r = rel[i];
          A.up = links.up[i].map((j) => L[j]); A.side = links.side[i].map((j) => L[j]);
          const bearing = b.collapse ? b.collapse.test(A, r) : (A.meta && WALLMATS.has(A.meta.mat) && r.lo[1] < 1.2 && r.hi[1] - r.lo[1] > 1.5);
          if (bearing) {
            const area = Math.max(A.hi[0] - A.lo[0], A.hi[2] - A.lo[2]) * (A.hi[1] - A.lo[1]);
            b.walls.push(pid); b.wallArea += area; A.area = area;
          }
        });
      }
    }
    // Hide a piece's triangles in the merged meshes (collapse them to a point).
    const dirtyKeys = new Set();
    function hide(pid) {
      for (const r of PIECES[pid].ranges) { MESH[r.key].geometry.attributes.position.array.fill(0, r.start * 9, (r.start + r.count) * 9); dirtyKeys.add(r.key); }
    }
    function flushHidden() {
      for (const k of dirtyKeys) MESH[k].geometry.attributes.position.needsUpdate = true;
      dirtyKeys.clear();
    }

    // ---------------------------------------------------------------- the rubble ground
    // A sparse height grid over the page's own ground: pieces at rest stamp their tops into it, so piles grow.
    const CELL = opts.cell || 0.5;
    const GRID = new Map();
    const cellOf = (x, z) => [Math.floor(x / CELL), Math.floor(z / CELL)];
    function gridAt(x, z) { const [i, j] = cellOf(x, z); const h = GRID.get(i * 1e6 + j); return h != null ? h : ground((i + 0.5) * CELL, (j + 0.5) * CELL); }
    function gridMax(x0, z0, x1, z1) {
      const [i0, j0] = cellOf(x0, z0), [i1, j1] = cellOf(x1, z1);
      let m = -1e9;
      const si = Math.max(1, Math.ceil((i1 - i0) / 6)), sj = Math.max(1, Math.ceil((j1 - j0) / 6));
      for (let j = j0; j <= j1; j += sj) for (let i = i0; i <= i1; i += si) { const h = GRID.get(i * 1e6 + j); m = Math.max(m, h != null ? h : ground((i + 0.5) * CELL, (j + 0.5) * CELL)); }
      return m;
    }
    function gridStamp(x0, z0, x1, z1, top) {
      const [i0, j0] = cellOf(x0, z0), [i1, j1] = cellOf(x1, z1);
      if ((i1 - i0) * (j1 - j0) > 4e5) return;
      for (let j = j0; j <= j1; j++) for (let i = i0; i <= i1; i++) { const k = i * 1e6 + j, h = GRID.get(k); if (h == null ? top > ground((i + 0.5) * CELL, (j + 0.5) * CELL) : top > h) GRID.set(k, top); }
    }
    function resetGrid() {
      GRID.clear();
      for (const P of PIECES) {
        if (P.ranges.length === 0) continue;
        const b = P.bld >= 0 ? BLD[P.bld] : null, base = b ? b.base : ground((P.lo[0] + P.hi[0]) / 2, (P.lo[2] + P.hi[2]) / 2);
        const foundation = P.meta && P.meta.mat === 'concrete' && P.lo[1] < base - 0.1;
        const paving = P.bld < 0 && P.hi[1] < base + 0.3;
        if (foundation || paving) gridStamp(P.lo[0], P.lo[2], P.hi[0], P.hi[2], P.hi[1]);
      }
    }

    // ---------------------------------------------------------------- chunks and falling pieces
    const THIN = 0.06;              // pieces thinner than this (plaster faces, glass, signs) turn to dust
    const CHUNK = 1.5;              // chunk size from a blast, metres
    const MAX_FALLING = 1500;       // falling chunks and pieces at once (chunks are instanced, so cheap)
    const REPOSE = Math.tan(35 * Math.PI / 180);
    const FALLING = [];             // { kind: 'inst'|'mesh', ... }
    const stats = { rubbleTris: 0 };
    const INST = {}; const INST_CAP = 4000;
    let unitBox = null;
    function instFor(key) {
      if (INST[key]) return INST[key];
      if (!unitBox) {
        const B = new K.Builder(); B.box('u', -0.5, -0.5, -0.5, 0.5, 0.5, 0.5);
        const a = B.parts.u;
        unitBox = new THREE.BufferGeometry();
        unitBox.setAttribute('position', new THREE.Float32BufferAttribute(a.p, 3));
        unitBox.setAttribute('normal', new THREE.Float32BufferAttribute(a.n, 3));
        unitBox.setAttribute('uv', new THREE.Float32BufferAttribute(a.u, 2));
      }
      const m = new THREE.InstancedMesh(unitBox, material(key).userData.instMat || instMaterial(key), INST_CAP);
      m.count = 0; m.frustumCulled = false; m.castShadow = true; m.receiveShadow = true;
      m.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      group.add(m);
      return (INST[key] = { mesh: m, used: 0, dirty: false });
    }
    function instMaterial(key) { // the merged material without vertex colours (instances carry the tint)
      const base = material(key);
      const m = base.clone(); m.vertexColors = false; m.clippingPlanes = clipping;
      base.userData.instMat = m;
      return m;
    }
    const _mat = new THREE.Matrix4(), _scl = new THREE.Vector3(), _col = new THREE.Color();
    function addChunk(key, tint, lo, hi, v, delay, q) {
      if (FALLING.length >= MAX_FALLING) return false;
      const I = instFor(key);
      if (I.used >= INST_CAP) return false;
      const slot = I.used++; I.mesh.count = I.used;
      const pos = q ? new THREE.Vector3(...lo) : new THREE.Vector3((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2);
      const size = q ? hi.slice() : [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
      _col.setRGB(tint[0], tint[1], tint[2]); I.mesh.setColorAt(slot, _col); I.mesh.instanceColor.needsUpdate = true;
      const f = { kind: 'inst', I, slot, pos, q: q ? q.clone() : new THREE.Quaternion(), size, h: size.map((s) => s / 2), v, w: spin(size), delay, still: 0 };
      writeInst(f); FALLING.push(f);
      return true;
    }
    function writeInst(f) { _scl.set(f.size[0], f.size[1], f.size[2]); _mat.compose(f.pos, f.q, _scl); f.I.mesh.setMatrixAt(f.slot, _mat); f.I.dirty = true; }
    function spin(size) { const s = 2.2 / Math.max(0.6, Math.max(...size)); return new THREE.Vector3((Math.random() - 0.5) * s, (Math.random() - 0.5) * s, (Math.random() - 0.5) * s); }
    // A non-box piece (a window with its glass, a sofa, a car, a roof plane, part of a ship) falls whole as its own mesh.
    function pieceMesh(pid, v, delay) {
      if (FALLING.length >= MAX_FALLING) return;
      const P = PIECES[pid];
      const center = new THREE.Vector3((P.lo[0] + P.hi[0]) / 2, (P.lo[1] + P.hi[1]) / 2, (P.lo[2] + P.hi[2]) / 2);
      const grp = new THREE.Group(); grp.position.copy(center);
      for (const r of P.ranges) {
        const g = MESH[r.key].geometry, O = ORIG[r.key];
        const n = r.count * 9, p = new Float32Array(n);
        for (let i = 0; i < n; i += 3) { const s = r.start * 9 + i; p[i] = O[s] - center.x; p[i + 1] = O[s + 1] - center.y; p[i + 2] = O[s + 2] - center.z; }
        const ng = new THREE.BufferGeometry();
        ng.setAttribute('position', new THREE.BufferAttribute(p, 3));
        ng.setAttribute('normal', new THREE.BufferAttribute(g.attributes.normal.array.slice(r.start * 9, r.start * 9 + n), 3));
        ng.setAttribute('uv', new THREE.BufferAttribute(g.attributes.uv.array.slice(r.start * 6, r.start * 6 + r.count * 6), 2));
        ng.setAttribute('color', new THREE.BufferAttribute(g.attributes.color.array.slice(r.start * 9, r.start * 9 + n), 3));
        const mm = new THREE.Mesh(ng, material(r.key)); mm.userData.key = r.key; grp.add(mm);
      }
      grp.userData.debris = true;
      group.add(grp);
      const size = [P.hi[0] - P.lo[0], P.hi[1] - P.lo[1], P.hi[2] - P.lo[2]];
      FALLING.push({ kind: 'mesh', obj: grp, pos: grp.position, q: grp.quaternion, size, h: size.map((s) => s / 2), v, w: spin(size), delay, still: 0 });
      stats.rubbleTris += P.ranges.reduce((s, r) => s + r.count, 0);
    }
    // A holed ship (info.sink) goes down whole, at once and slowly as befits its size (owner, 2026-10-29: "ships empty on the inside, I expect them
    // to sink right away, when they touch sea floor they should just sink in to the ground slowly and despawn"): what is
    // left of it becomes one hulk that drops through the water, settles a little bow or stern first, then sinks into the
    // seabed at a crawl and is gone.
    const HULKS = [];
    // A big ship goes down slowly: its sinking speed falls with its length (a 120 m freighter at 0.25 m/s, a launch at 2).
    const HULK = { speed_m: 30, minV: 0.25, maxV: 2, accel: 0.15, bury: 0.4, tilt: 0.12 };
    function sinkHulk(B) {
      const c = new THREE.Vector3((B.lo[0] + B.hi[0]) / 2, B.base, (B.lo[2] + B.hi[2]) / 2);
      const grp = new THREE.Group(); grp.position.copy(c); grp.userData.debris = true;
      const byKey = {};
      let n = 0, minY = 1e9, maxY = -1e9;
      for (const pid of B.pieces) {
        const P = PIECES[pid];
        if (!P.alive) continue;
        P.alive = false; hide(pid); n++;
        for (const r of P.ranges) (byKey[r.key] = byKey[r.key] || []).push(r);
      }
      for (const [key, rs] of Object.entries(byKey)) {
        const g = MESH[key].geometry, O = ORIG[key], pos = [], nrm = [], uv = [], col = [];
        for (const r of rs) {
          for (let i = r.start * 9; i < (r.start + r.count) * 9; i += 3) { pos.push(O[i] - c.x, O[i + 1] - c.y, O[i + 2] - c.z); minY = Math.min(minY, O[i + 1] - c.y); maxY = Math.max(maxY, O[i + 1] - c.y); }
          nrm.push(...g.attributes.normal.array.slice(r.start * 9, (r.start + r.count) * 9));
          uv.push(...g.attributes.uv.array.slice(r.start * 6, (r.start + r.count) * 6));
          col.push(...g.attributes.color.array.slice(r.start * 9, (r.start + r.count) * 9));
        }
        const ng = new THREE.BufferGeometry();
        ng.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3)); ng.setAttribute('normal', new THREE.Float32BufferAttribute(nrm, 3));
        ng.setAttribute('uv', new THREE.Float32BufferAttribute(uv, 2)); ng.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
        const mm = new THREE.Mesh(ng, material(key)); mm.castShadow = mm.receiveShadow = true; grp.add(mm);
      }
      group.add(grp);
      const len = Math.max(B.hi[0] - B.lo[0], B.hi[2] - B.lo[2]), alongX = B.hi[0] - B.lo[0] > B.hi[2] - B.lo[2];
      const t = (Math.random() < 0.5 ? -1 : 1) * HULK.tilt * Math.min(1, 40 / Math.max(10, len));
      const vMax = Math.min(HULK.maxV, Math.max(HULK.minV, HULK.speed_m / len));
      HULKS.push({ grp, v: 0, vMax, minY, maxY, phase: 'water', tiltTo: alongX ? [0, 0, t] : [t, 0, 0], roll: (Math.random() - 0.5) * 0.1 });
      dustAt(c.x, 0.5, c.z, 120, Math.max(4, len * 0.3), [0.85, 0.88, 0.9]);
      return n;
    }
    // A shot on a sinking hulk tears out its plating there: every triangle within the blast is dropped from it.
    const _inv = new THREE.Matrix4(), _lp = new THREE.Vector3();
    function blastHulks(p, r) {
      for (const H of HULKS) {
        H.grp.updateMatrixWorld(true); _inv.copy(H.grp.matrixWorld).invert(); _lp.copy(p).applyMatrix4(_inv);
        for (const m of H.grp.children) {
          const a = m.geometry.attributes.position, A = a.array; let any = false;
          for (let i = 0; i < A.length; i += 9) {
            const cx = (A[i] + A[i + 3] + A[i + 6]) / 3 - _lp.x, cy = (A[i + 1] + A[i + 4] + A[i + 7]) / 3 - _lp.y, cz = (A[i + 2] + A[i + 5] + A[i + 8]) / 3 - _lp.z;
            if (cx * cx + cy * cy + cz * cz < r * r && (A[i] || A[i + 1] || A[i + 2])) { A.fill(0, i, i + 9); any = true; }
          }
          if (any) a.needsUpdate = true;
        }
      }
    }
    function stepHulks(dt) {
      for (let i = HULKS.length - 1; i >= 0; i--) {
        const H = HULKS[i], p = H.grp.position, floor = ground(p.x, p.z);
        if (H.phase === 'water') {
          H.v = Math.min(H.vMax, H.v + HULK.accel * dt);
          p.y -= H.v * dt;
          const k = Math.min(1, dt * 0.15);
          H.grp.rotation.x += (H.tiltTo[0] - H.grp.rotation.x) * k; H.grp.rotation.z += (H.tiltTo[2] + H.roll - H.grp.rotation.z) * k;
          if (Math.random() < dt * 6 && p.y + H.maxY > -2) dustAt(p.x + (Math.random() - 0.5) * 10, 0.3, p.z + (Math.random() - 0.5) * 10, 3, 2, [0.85, 0.88, 0.9]);
          if (p.y + H.minY <= floor) { H.phase = 'bury'; dustAt(p.x, floor + 0.5, p.z, 60, 6, [0.62, 0.56, 0.44]); }
        } else {
          p.y -= H.vMax * HULK.bury * dt;
          if (p.y + H.maxY < floor - 0.5) { group.remove(H.grp); H.grp.traverse((o) => { if (o.geometry) o.geometry.dispose(); }); HULKS.splice(i, 1); }
        }
      }
    }
    // What a destroyed piece becomes: dust, chunks of its material, or itself falling whole.
    function breakOff(pid, vel, delay, from, chunk = CHUNK) {
      const P = PIECES[pid];
      const size = [P.hi[0] - P.lo[0], P.hi[1] - P.lo[1], P.hi[2] - P.lo[2]];
      if (Math.min(...size) < THIN) { if (Math.random() < 0.15) dustAt(P.lo[0], P.lo[1], P.lo[2], 1, 0.5); return; }
      if (!P.meta) { pieceMesh(pid, vel, delay); return; }
      if (P.meta.panel) { // a roof panel falls as one tilted chunk of its roofing
        const A = P.meta.panel.axes;
        const q = new THREE.Quaternion().setFromRotationMatrix(new THREE.Matrix4().makeBasis(new THREE.Vector3(...A[0]), new THREE.Vector3(...A[1]), new THREE.Vector3(...A[2])));
        const v = from ? vel : vel.clone().add(new THREE.Vector3((Math.random() - 0.5) * 1.2, 0, (Math.random() - 0.5) * 1.2));
        addChunk(P.meta.mat, P.meta.tint, P.meta.panel.c, P.meta.panel.size, v, delay + (from ? 0 : Math.random() * 0.15), q);
        stats.rubbleTris += 12;
        return;
      }
      const [x0, y0, z0, x1, y1, z1] = P.meta.box;
      const nx = Math.min(4, Math.ceil((x1 - x0) / chunk)), ny = Math.min(4, Math.ceil((y1 - y0) / chunk)), nz = Math.min(4, Math.ceil((z1 - z0) / chunk));
      for (let i = 0; i < nx; i++) for (let j = 0; j < ny; j++) for (let k = 0; k < nz; k++) {
        const a = [x0 + (x1 - x0) * i / nx, y0 + (y1 - y0) * j / ny, z0 + (z1 - z0) * k / nz];
        const b = [x0 + (x1 - x0) * (i + 1) / nx, y0 + (y1 - y0) * (j + 1) / ny, z0 + (z1 - z0) * (k + 1) / nz];
        const c = new THREE.Vector3((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2);
        const jit = () => (Math.random() - 0.5);
        const away = from ? c.clone().sub(from).normalize().multiplyScalar(vel.length() * 0.5) : new THREE.Vector3(jit() * 1.5, 0, jit() * 1.5);
        const v = vel.clone().multiplyScalar(0.6).add(away).add(new THREE.Vector3(jit() * 3, jit() * 2, jit() * 3));
        addChunk(P.meta.mat, P.meta.tint, a, b, v, delay + (from ? 0 : (a[1] - y0) * 0.03 + Math.random() * 0.05));
        stats.rubbleTris += 12;
      }
    }

    // ---------------------------------------------------------------- dust, fire, flashes, scorch
    function softSprite() {
      const c = document.createElement('canvas'); c.width = c.height = 64; const g = c.getContext('2d');
      const r = g.createRadialGradient(32, 32, 2, 32, 32, 31); r.addColorStop(0, 'rgba(255,255,255,1)'); r.addColorStop(0.5, 'rgba(255,255,255,0.5)'); r.addColorStop(1, 'rgba(255,255,255,0)');
      g.fillStyle = r; g.fillRect(0, 0, 64, 64);
      return new THREE.CanvasTexture(c);
    }
    let SPRITE = null, DUST = null, FIRE = null, fxReady = false;
    function pool(n, size, blending) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(n * 3), 3));
      geo.setAttribute('color', new THREE.BufferAttribute(new Float32Array(n * 4), 4));
      const mat = new THREE.PointsMaterial({ size, map: SPRITE, vertexColors: true, transparent: true, depthWrite: false, blending, sizeAttenuation: true });
      const pts = new THREE.Points(geo, mat); pts.frustumCulled = false; pts.renderOrder = 3; group.add(pts);
      return { pts, n, next: 0, live: 0, life: new Float32Array(n), max: new Float32Array(n), vel: new Float32Array(n * 3), rgb: new Float32Array(n * 3), grav: 0, drag: 0.97 };
    }
    function emit(P, x, y, z, vx, vy, vz, life, rgb) {
      const i = P.next; P.next = (P.next + 1) % P.n;
      const pos = P.pts.geometry.attributes.position.array;
      pos[i * 3] = x; pos[i * 3 + 1] = y; pos[i * 3 + 2] = z;
      P.vel[i * 3] = vx; P.vel[i * 3 + 1] = vy; P.vel[i * 3 + 2] = vz;
      P.life[i] = life; P.max[i] = life; P.rgb.set(rgb, i * 3); P.live = Math.max(P.live, life);
    }
    function stepPool(P, dt, alpha) {
      if (P.live <= 0) return; // nothing alive: no work, no upload
      P.live -= dt;
      const pos = P.pts.geometry.attributes.position.array, col = P.pts.geometry.attributes.color.array;
      for (let i = 0; i < P.n; i++) {
        if (P.life[i] <= 0) { col[i * 4 + 3] = 0; continue; }
        P.life[i] -= dt;
        P.vel[i * 3 + 1] += P.grav * dt;
        for (let k = 0; k < 3; k++) { P.vel[i * 3 + k] *= P.drag; pos[i * 3 + k] += P.vel[i * 3 + k] * dt; }
        const fl = Math.max(0, ground(pos[i * 3], pos[i * 3 + 2])) + 0.05;
        if (pos[i * 3 + 1] < fl) { pos[i * 3 + 1] = fl; P.vel[i * 3 + 1] *= -0.3; }
        const f = Math.max(0, P.life[i] / P.max[i]);
        col[i * 4] = P.rgb[i * 3]; col[i * 4 + 1] = P.rgb[i * 3 + 1]; col[i * 4 + 2] = P.rgb[i * 3 + 2];
        col[i * 4 + 3] = alpha(f);
      }
      P.pts.geometry.attributes.position.needsUpdate = true; P.pts.geometry.attributes.color.needsUpdate = true;
    }
    function dustAt(x, y, z, n, spread, tone = [0.62, 0.58, 0.52]) {
      if (!DUST) return;
      for (let i = 0; i < n; i++) {
        const a = Math.random() * Math.PI * 2, s = Math.random() * spread;
        const v = 0.85 + Math.random() * 0.3;
        emit(DUST, x + (Math.random() - 0.5) * spread * 0.5, y + Math.random() * spread * 0.3, z + (Math.random() - 0.5) * spread * 0.5, Math.cos(a) * s, 0.6 + Math.random() * spread * 0.4, Math.sin(a) * s, 3 + Math.random() * 4, [tone[0] * v, tone[1] * v, tone[2] * v]);
      }
    }
    const FLASH = [];
    let flashNext = 0;
    function flash(p, r) {
      const f = FLASH[flashNext]; flashNext = (flashNext + 1) % FLASH.length;
      f.light.position.copy(p); f.ball.position.copy(p); f.t = 0; f.r = r; f.ball.visible = true;
    }
    const SCORCH = [];
    function scorch(p, r) {
      const m = new THREE.Mesh(new THREE.CircleGeometry(r * 0.8, 20), new THREE.MeshBasicMaterial({ color: 0x15120e, transparent: true, opacity: 0.55, depthWrite: false }));
      m.rotation.x = -Math.PI / 2; m.position.set(p.x, gridAt(p.x, p.z) + 0.02 + (SCORCH.length * 0.001) % 0.05, p.z);
      m.renderOrder = 1; group.add(m); SCORCH.push(m);
    }
    function addFx() {
      SPRITE = SPRITE || softSprite();
      DUST = pool(2500, 3.2, THREE.NormalBlending); DUST.grav = 0.25; DUST.drag = 0.985;
      FIRE = pool(600, 0.45, THREE.AdditiveBlending); FIRE.grav = -9.81; FIRE.drag = 0.99;
      FLASH.length = 0;
      for (let i = 0; i < 3; i++) {
        const light = new THREE.PointLight(0xffb060, 0, 40, 2);
        const ball = new THREE.Mesh(new THREE.SphereGeometry(1, 16, 10), new THREE.MeshBasicMaterial({ color: 0xffa040, transparent: true, opacity: 0, depthWrite: false, blending: THREE.AdditiveBlending }));
        ball.visible = false; group.add(light, ball);
        FLASH.push({ light, ball, t: 1, r: 1 });
      }
      fxReady = true;
    }

    // ---------------------------------------------------------------- the shot
    const WEAPONS = opts.weapons || { cannon: { r: 1.3, label: 'Autocannon', throwV: 9 }, shell: { r: 3.2, label: '120 mm shell', throwV: 14 }, missile: { r: 5.5, label: 'Missile', throwV: 18, flies: true }, demo: { r: 9, label: 'Demolition', throwV: 22 } };
    function blast(p, wkey) {
      if (!fxReady) addFx();
      const W = typeof wkey === 'string' ? WEAPONS[wkey] : wkey, r = W.r;
      flash(p, r);
      for (let i = 0; i < 20 * r; i++) { const a = Math.random() * 6.28, e = Math.random() * 1.2, s = 6 + Math.random() * 14; emit(FIRE, p.x, p.y, p.z, Math.cos(a) * Math.cos(e) * s, Math.sin(e) * s, Math.sin(a) * Math.cos(e) * s, 0.25 + Math.random() * 0.5, [1.0, 0.55 + Math.random() * 0.3, 0.2]); }
      dustAt(p.x, p.y, p.z, Math.round(25 * r), r * 0.9);
      if (p.y < gridAt(p.x, p.z) + 1.0) scorch(p, r);
      const hitB = new Set();
      let hit = 0;
      for (let pid = 0; pid < PIECES.length; pid++) {
        const P = PIECES[pid];
        if (!P.alive || P.bld < 0) continue;
        const dx = Math.max(P.lo[0] - p.x, 0, p.x - P.hi[0]), dy = Math.max(P.lo[1] - p.y, 0, p.y - P.hi[1]), dz = Math.max(P.lo[2] - p.z, 0, p.z - P.hi[2]);
        const d = Math.hypot(dx, dy, dz);
        if (d > r) continue;
        P.alive = false; hide(pid); hitB.add(P.bld); hit++;
        const c = new THREE.Vector3((P.lo[0] + P.hi[0]) / 2, (P.lo[1] + P.hi[1]) / 2, (P.lo[2] + P.hi[2]) / 2);
        const dir = c.clone().sub(p); if (dir.lengthSq() < 1e-4) dir.set(0, 1, 0); dir.normalize();
        const v = dir.multiplyScalar(W.throwV * (1 - d / r) + 1.5).add(new THREE.Vector3(0, 2 + Math.random() * 3, 0));
        breakOff(pid, v, 0, p);
      }
      let fell = 0;
      if (HULKS.length) blastHulks(p, r);
      for (const b of hitB) fell += settleBuilding(b);
      flushHidden();
      return { hit, fell };
    }
    // What still stands: from the ground up through 'up' links freely, and at most three 'side' steps. A building that
    // loses half its load-bearing walls (or its own `collapse.frac`) comes down whole.
    function settleBuilding(b) {
      const B = BLD[b];
      if (B.collapsed) return 0;
      const left = B.walls.reduce((s, pid) => s + (PIECES[pid].alive ? PIECES[pid].area : 0), 0);
      const frac = B.collapse ? B.collapse.frac : 0.5;
      if (B.walls.length && left < B.wallArea * frac) {
        B.collapsed = true;
        if (B.info.sink) return sinkHulk(B);
        let n = 0;
        for (const pid of B.pieces) {
          const P = PIECES[pid];
          if (!P.alive) continue;
          if (!B.collapse && P.lo[1] - B.base < 0.6 && P.hi[1] - B.base < 1.2) continue; // the slab and what sits on it stay
          P.alive = false; hide(pid); n++;
          breakOff(pid, new THREE.Vector3((Math.random() - 0.5) * 1.5, -0.5, (Math.random() - 0.5) * 1.5), 0.5 + (P.lo[1] - B.base) * 0.04 + Math.random() * 0.15, null, 2.2);
        }
        dustAt((B.lo[0] + B.hi[0]) / 2, B.base + 1, (B.lo[2] + B.hi[2]) / 2, 240, Math.max(6, (B.hi[0] - B.lo[0]) * 0.35));
        return n;
      }
      const L = B.pieces, idx = new Map(L.map((pid, i) => [pid, i]));
      const local = { up: L.map((pid) => PIECES[pid].up.map((q) => idx.get(q))), side: L.map((pid) => PIECES[pid].side.map((q) => idx.get(q))) };
      const st = K.standing(B.rel, local, L.map((pid) => PIECES[pid].alive), B.info.maxSide);
      const best = new Set([...st.keys()].map((i) => L[i]));
      let n = 0;
      for (const pid of B.pieces) {
        const P = PIECES[pid];
        if (!P.alive || best.has(pid)) continue;
        P.alive = false; hide(pid); n++;
        breakOff(pid, new THREE.Vector3((Math.random() - 0.5) * 0.8, -0.2, (Math.random() - 0.5) * 0.8), 0.25 + Math.random() * 0.6, null);
        dustAt((P.lo[0] + P.hi[0]) / 2, P.lo[1], (P.lo[2] + P.hi[2]) / 2, 2, 0.8);
      }
      return n;
    }
    // Pieces that the kit's rule would not hold as built (they hang, or float): reported by a page so it can fix them.
    function unheld() {
      const out = [];
      for (const B of BLD) {
        const L = B.pieces, idx = new Map(L.map((pid, i) => [pid, i]));
        const local = { up: L.map((pid) => PIECES[pid].up.map((q) => idx.get(q))), side: L.map((pid) => PIECES[pid].side.map((q) => idx.get(q))) };
        const st = K.standing(B.rel, local, L.map(() => true), B.info.maxSide);
        const n = L.length - st.size;
        if (n) out.push({ name: B.name, unheld: n, of: L.length });
      }
      return out;
    }

    // ---------------------------------------------------------------- physics of what falls
    const _q = new THREE.Quaternion(), _m = new THREE.Matrix4();
    function stepDebris(dt) {
      for (let i = FALLING.length - 1; i >= 0; i--) {
        const d = FALLING[i];
        if (d.delay > 0) {
          d.delay -= dt;
          d.pos.x += (Math.random() - 0.5) * 0.02; d.pos.z += (Math.random() - 0.5) * 0.02; // the creak
          if (Math.random() < dt * 4) dustAt(d.pos.x, d.pos.y - d.h[1], d.pos.z, 1, 0.4);
          if (d.kind === 'inst') writeInst(d);
          continue;
        }
        d.age = (d.age || 0) + dt;
        d.v.y -= 9.81 * dt;
        d.pos.addScaledVector(d.v, dt);
        const wl = d.w.length();
        if (wl > 1e-4) { _q.setFromAxisAngle(d.w.clone().divideScalar(wl), wl * dt); d.q.premultiply(_q); }
        _m.makeRotationFromQuaternion(d.q); const e = _m.elements;
        const hy = Math.abs(e[1]) * d.h[0] + Math.abs(e[5]) * d.h[1] + Math.abs(e[9]) * d.h[2];
        const hx = Math.abs(e[0]) * d.h[0] + Math.abs(e[4]) * d.h[1] + Math.abs(e[8]) * d.h[2];
        const hz = Math.abs(e[2]) * d.h[0] + Math.abs(e[6]) * d.h[1] + Math.abs(e[10]) * d.h[2];
        const floor = gridMax(d.pos.x - hx * 0.6, d.pos.z - hz * 0.6, d.pos.x + hx * 0.6, d.pos.z + hz * 0.6);
        let rest = false;
        if (d.pos.y - hy < floor) {
          d.pos.y = floor + hy;
          if (d.v.y < -2.5) { d.v.y *= -0.2; d.v.x *= 0.55; d.v.z *= 0.55; d.w.multiplyScalar(0.5); if (Math.random() < 0.4) dustAt(d.pos.x, floor, d.pos.z, 2, 0.6); }
          else { d.v.y = 0; d.v.x *= 0.82; d.v.z *= 0.82; d.w.multiplyScalar(0.7); }
          // Angle of repose: slide toward the lowest neighbour 1 m away if the drop is too steep.
          let low = floor, lx = 0, lz = 0;
          for (let a = 0; a < 8; a++) { const ax = Math.cos(a * Math.PI / 4), az = Math.sin(a * Math.PI / 4); const g = gridAt(d.pos.x + ax * (hx + 0.6), d.pos.z + az * (hz + 0.6)); if (g < low) { low = g; lx = ax; lz = az; } }
          if (floor - low > REPOSE * (Math.max(hx, hz) + 0.6)) { d.v.x += lx * 9 * dt + lx * 0.4; d.v.z += lz * 9 * dt + lz * 0.4; d.still = 0; }
          else if (d.v.lengthSq() < 0.15 && d.w.lengthSq() < 0.2) d.still += dt; else d.still = 0;
          if (d.still > 0.2 || d.age > 6) rest = true;
        } else if (d.pos.y < (opts.waterY ?? -1e9)) { d.v.multiplyScalar(Math.pow(0.15, dt)); } // in the water: drag, a slow sink
        if (d.kind === 'inst') writeInst(d);
        if (rest) {
          // A pile rises by the piece's thickness, not its tilted bounding box (a roof plane lies flat-ish).
          const thick = d.kind === 'inst' ? Math.min(...d.size) : 0.5;
          gridStamp(d.pos.x - hx * 0.85, d.pos.z - hz * 0.85, d.pos.x + hx * 0.85, d.pos.z + hz * 0.85, floor + Math.min(hy * 2, Math.max(thick, 0.3)) * 0.85);
          if (d.kind === 'mesh') bake(d.obj);
          FALLING.splice(i, 1);
        } else if (d.pos.y < -60) FALLING.splice(i, 1);
      }
      for (const I of Object.values(INST)) if (I.dirty) { I.mesh.instanceMatrix.needsUpdate = true; I.dirty = false; }
    }
    // A whole piece at rest is baked into one static rubble mesh a material (rebuilt for that material only).
    const RUBBLE = {}, rubbleDirty = new Set(); let rubbleTimer = 0;
    function bake(o) {
      o.updateMatrixWorld(true);
      const nm = new THREE.Matrix3(), v = new THREE.Vector3();
      for (const m of o.children) {
        const key = m.userData.key;
        const R = RUBBLE[key] || (RUBBLE[key] = { p: [], n: [], u: [], c: [], mesh: null });
        const g = m.geometry, P = g.attributes.position, N = g.attributes.normal;
        nm.getNormalMatrix(m.matrixWorld);
        for (let i = 0; i < P.count; i++) { v.fromBufferAttribute(P, i).applyMatrix4(m.matrixWorld); R.p.push(v.x, v.y, v.z); v.fromBufferAttribute(N, i).applyMatrix3(nm).normalize(); R.n.push(v.x, v.y, v.z); }
        for (const x of g.attributes.uv.array) R.u.push(x);
        for (const x of g.attributes.color.array) R.c.push(x);
        g.dispose(); rubbleDirty.add(key);
      }
      group.remove(o);
    }
    function rebuildRubble() {
      for (const key of rubbleDirty) {
        const R = RUBBLE[key];
        if (R.mesh) { group.remove(R.mesh); R.mesh.geometry.dispose(); R.mesh = null; }
        if (!R.p.length) continue;
        const g = new THREE.BufferGeometry();
        g.setAttribute('position', new THREE.Float32BufferAttribute(R.p, 3));
        g.setAttribute('normal', new THREE.Float32BufferAttribute(R.n, 3));
        g.setAttribute('uv', new THREE.Float32BufferAttribute(R.u, 2));
        g.setAttribute('color', new THREE.Float32BufferAttribute(R.c, 3));
        g.computeBoundingSphere();
        R.mesh = new THREE.Mesh(g, material(key)); R.mesh.castShadow = true; R.mesh.receiveShadow = true; R.mesh.userData.debris = true;
        group.add(R.mesh);
      }
      rubbleDirty.clear();
    }
    const TRACERS = [], MISSILES = [];
    function stepFx(dt) {
      rubbleTimer += dt;
      if (rubbleDirty.size && rubbleTimer > 0.5) { rubbleTimer = 0; rebuildRubble(); }
      for (const f of FLASH) {
        if (f.t >= 1) continue;
        f.t += dt / 0.45;
        const k = Math.min(1, f.t);
        f.light.intensity = 120 * Math.pow(1 - k, 2) * f.r;
        f.ball.scale.setScalar(f.r * (0.3 + 0.9 * Math.sqrt(k)));
        f.ball.material.opacity = 0.9 * (1 - k);
        if (k >= 1) { f.ball.visible = false; f.light.intensity = 0; }
      }
      if (DUST) stepPool(DUST, dt, (f) => 0.55 * Math.min(1, f * 1.6) * Math.min(1, (1 - f) * 8 + 0.2));
      if (FIRE) stepPool(FIRE, dt, (f) => f);
      for (const t of TRACERS.slice()) { t.life -= dt; t.line.material.opacity = Math.max(0, t.life / 0.08); if (t.life <= 0) { group.remove(t.line); TRACERS.splice(TRACERS.indexOf(t), 1); } }
      for (const m of MISSILES.slice()) {
        m.t += dt;
        const k = Math.min(1, m.t / m.dur);
        m.obj.position.lerpVectors(m.a, m.b, k);
        emit(DUST, m.obj.position.x, m.obj.position.y, m.obj.position.z, (Math.random() - 0.5) * 0.4, 0.3, (Math.random() - 0.5) * 0.4, 1.8, [0.85, 0.85, 0.85]);
        if (k >= 1) { group.remove(m.obj); MISSILES.splice(MISSILES.indexOf(m), 1); blast(m.b, m.w); }
      }
    }
    function step(dt) { stepDebris(dt); stepHulks(dt); stepFx(dt); }

    // ---------------------------------------------------------------- firing where the player points
    const ray = new THREE.Raycaster();
    // ndc: the pointer in normalised device coordinates. extra: more objects the shot can land on (the ground, models).
    function fireAt(ndc, camera, wkey, extra = []) {
      if (!fxReady) addFx();
      ray.setFromCamera(ndc, camera);
      const targets = Object.values(MESH).filter((m) => !TRANSPARENT.has(m.userData.key) || m.userData.key === 'glass' || m.userData.key === 'glassShop').concat(HULKS.map((H) => H.grp), extra);   // a sinking ship can still be shot
      const hits = ray.intersectObjects(targets, true);
      let p = hits.length ? hits[0].point : null;
      if (!p) { const t = -ray.ray.origin.y / ray.ray.direction.y; if (t > 0 && t < 4000) p = ray.ray.origin.clone().addScaledVector(ray.ray.direction, t); }
      if (!p) return null;
      const muzzle = camera.position.clone().add(new THREE.Vector3(0, -0.6, 0));
      const W = WEAPONS[wkey];
      if (W.flies) {
        const obj = new THREE.Mesh(new THREE.CylinderGeometry(0.12, 0.12, 1.4, 8), new THREE.MeshStandardMaterial({ color: 0xd8d8d0, emissive: 0xff8030, emissiveIntensity: 0.6 }));
        obj.position.copy(muzzle); obj.lookAt(p); obj.rotateX(Math.PI / 2); group.add(obj);
        MISSILES.push({ obj, a: muzzle, b: p.clone(), t: 0, dur: Math.max(0.15, muzzle.distanceTo(p) / 220), w: wkey });
        return p;
      }
      const g = new THREE.BufferGeometry().setFromPoints([muzzle, p]);
      const line = new THREE.Line(g, new THREE.LineBasicMaterial({ color: 0xffe0a0, transparent: true, opacity: 1 }));
      group.add(line); TRACERS.push({ line, life: 0.08 });
      blast(p.clone(), wkey);
      return p;
    }
    // A click that does not drag fires the weapon weapon() names where the pointer is.
    function bindFire(canvas, camera, weapon, extra = () => []) {
      let downAt = null;
      canvas.addEventListener('pointerdown', (e) => { downAt = [e.clientX, e.clientY]; });
      canvas.addEventListener('pointerup', (e) => {
        if (!downAt) return;
        const moved = Math.hypot(e.clientX - downAt[0], e.clientY - downAt[1]);
        downAt = null;
        if (moved >= 6 || e.button !== 0) return;
        const rect = canvas.getBoundingClientRect();
        fireAt(new THREE.Vector2(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1), camera, weapon(), extra());
      });
    }
    // Everything back as built.
    function reset() {
      for (const [k, m] of Object.entries(MESH)) { m.geometry.attributes.position.array.set(ORIG[k]); m.geometry.attributes.position.needsUpdate = true; }
      for (const P of PIECES) P.alive = true;
      for (const b of BLD) b.collapsed = false;
      for (const f of FALLING) if (f.kind === 'mesh') group.remove(f.obj);
      for (const R of Object.values(RUBBLE)) { if (R.mesh) group.remove(R.mesh); R.p = []; R.n = []; R.u = []; R.c = []; R.mesh = null; }
      FALLING.length = 0; HULKS.length = 0;
      group.children.filter((o) => o.userData.debris).forEach((o) => group.remove(o));
      for (const I of Object.values(INST)) { I.used = 0; I.mesh.count = 0; }
      stats.rubbleTris = 0;
      for (const s of SCORCH) group.remove(s);
      SCORCH.length = 0;
      resetGrid();
    }
    return {
      GEN, makeTex, normalFrom, material, materials, glows, M, TRANSPARENT, MESH, PIECES, BLD, WEAPONS, FALLING, HULKS, stats,
      building, add, boundsOf, fromObject, finish, clear, reset, blast, fireAt, bindFire, step, unheld, gridAt, dustAt,
    };
  }
  root.BuildingEngine = BuildingEngine;
})(typeof window !== 'undefined' ? window : this);
