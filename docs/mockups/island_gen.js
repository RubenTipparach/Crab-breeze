// The island of openspec/changes/island-biomes, as pure functions of position, for the island
// mockup (docs/mockups/island.html). Metres throughout; x east, z south (north is -z), y up.
// The game will port these rules to cb-core::world::terrain and world::biome; this file is the
// mockup's stand-in, tuned by eye, and its numbers follow the design's section 1.
(function (root) {
  'use strict';

  const SEED = 42;
  const WORLD_M = 40960;
  const HALF_M = WORLD_M / 2;
  const CLOUD_BASE_M = 600;
  const CLOUD_TOP_M = 1100;

  // ---------------------------------------------------------------- randomness and noise
  function mulberry(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) | 0;
      let t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function hash(ix, iz, s) {
    let h = (Math.imul(ix, 374761393) + Math.imul(iz, 668265263) + Math.imul(s, 1442695041)) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  }
  // Gradient noise, about -1..1.
  function perlin(x, z, s) {
    const ix = Math.floor(x), iz = Math.floor(z);
    const fx = x - ix, fz = z - iz;
    const g = (cx, cz, dx, dz) => {
      const a = hash(cx, cz, s) * 6.283185307;
      return Math.cos(a) * dx + Math.sin(a) * dz;
    };
    const ux = fx * fx * fx * (fx * (fx * 6 - 15) + 10);
    const uz = fz * fz * fz * (fz * (fz * 6 - 15) + 10);
    const a = g(ix, iz, fx, fz), b = g(ix + 1, iz, fx - 1, fz);
    const c = g(ix, iz + 1, fx, fz - 1), d = g(ix + 1, iz + 1, fx - 1, fz - 1);
    return 1.41 * ((a + (b - a) * ux) + ((c + (d - c) * ux) - (a + (b - a) * ux)) * uz);
  }
  function fbm(x, z, oct, s) {
    let sum = 0, amp = 0.5, norm = 0;
    for (let i = 0; i < oct; i++) {
      sum += amp * perlin(x, z, s + i * 17);
      norm += amp; amp *= 0.5; x *= 2.03; z *= 2.03;
    }
    return sum / norm;
  }
  // Ridged multifractal (Musgrave): each octave weighted by the one before, so detail gathers on
  // the crests and the valleys stay smooth, the eroded look. 0..1.
  function ridged(x, z, oct, s) {
    let sum = 0, amp = 0.5, norm = 0, w = 1;
    for (let i = 0; i < oct; i++) {
      let n = 1 - Math.abs(perlin(x, z, s + i * 31));
      n *= n; n *= w;
      w = Math.min(1, Math.max(0, n * 2));
      sum += n * amp; norm += amp; amp *= 0.5; x *= 2.0; z *= 2.0;
    }
    return sum / norm;
  }
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const smooth = (a, b, v) => { const t = clamp((v - a) / (b - a), 0, 1); return t * t * (3 - 2 * t); };
  const mix = (a, b, t) => a + (b - a) * t;
  // Soft terrace: flat treads with risers of `riser` share of each step.
  function terrace(h, step, riser) {
    const t = h / step, f = Math.floor(t), fr = t - f;
    return (f + smooth(0.5 - riser / 2, 0.5 + riser / 2, fr)) * step;
  }

  // ---------------------------------------------------------------- the range's spine
  const NORTH = [Math.sin(20 * Math.PI / 180), -Math.cos(20 * Math.PI / 180)];
  const EAST = [-NORTH[1], NORTH[0]];
  const SPINE = [];
  {
    const p0 = [-8000 * NORTH[0] - 300 * EAST[0], -8000 * NORTH[1] - 300 * EAST[1]];
    const p1 = [-1300 * EAST[0], -1300 * EAST[1]];
    const p2 = [8000 * NORTH[0] + 500 * EAST[0], 8000 * NORTH[1] + 500 * EAST[1]];
    for (let i = 0; i <= 64; i++) {
      const t = i / 64, u = 1 - t;
      SPINE.push([u * u * p0[0] + 2 * u * t * p1[0] + t * t * p2[0], u * u * p0[1] + 2 * u * t * p1[1] + t * t * p2[1]]);
    }
  }
  // Distance to the spine, the position along it 0..1 and the signed side (east positive).
  function spineAt(x, z) {
    let best = 1e18, bt = 0, side = 0;
    for (let i = 0; i < 64; i++) {
      const a = SPINE[i], b = SPINE[i + 1];
      const dx = b[0] - a[0], dz = b[1] - a[1];
      const l2 = dx * dx + dz * dz;
      const u = clamp(((x - a[0]) * dx + (z - a[1]) * dz) / l2, 0, 1);
      const px = a[0] + dx * u, pz = a[1] + dz * u;
      const d2 = (x - px) * (x - px) + (z - pz) * (z - pz);
      if (d2 < best) { best = d2; bt = (i + u) / 64; side = dx * (z - a[1]) - dz * (x - a[0]); }
    }
    const d = Math.sqrt(best);
    // dx*(z-az) - dz*(x-ax) is positive to the east of a northbound line (z south).
    return { d, t: bt, s: side > 0 ? d : -d };
  }
  function spinePoint(t) {
    const f = clamp(t, 0, 1) * 64, i = Math.min(63, Math.floor(f)), u = f - i;
    return [mix(SPINE[i][0], SPINE[i + 1][0], u), mix(SPINE[i][1], SPINE[i + 1][1], u)];
  }
  const PASSES = [0.33, 0.68]; // where the highways cross, along the spine

  // ---------------------------------------------------------------- coast
  function bearing(x, z) { // degrees, 0 north, 90 east
    let b = Math.atan2(x, -z) * 180 / Math.PI;
    return b < 0 ? b + 360 : b;
  }
  function sector(b, from, to, soft) { // weight 0..1 for a bearing inside [from, to]
    const inside = (v) => smooth(from - soft, from + soft, v) * (1 - smooth(to - soft, to + soft, v));
    return Math.max(inside(b), inside(b - 360), inside(b + 360));
  }
  function coastDist(x, z) {
    const r = Math.hypot(x, z) + 1e-6;
    const cx = x / r, cz = z / r;
    const R = 12300 + 1700 * fbm(cx * 1.6 + 3.1, cz * 1.6 - 1.7, 4, SEED + 1) + 700 * fbm(x / 2600, z / 2600, 3, SEED + 2);
    return R - r;
  }

  // ---------------------------------------------------------------- canyons, islets, rivers (seeded once)
  const rng = mulberry(SEED * 7919);
  const CANYONS = [];
  {
    const roots = [0.28, 0.47, 0.64];
    const grow = (x, z, head, steps, depth, fh) => {
      for (let i = 0; i < steps; i++) {
        head += (rng() - 0.5) * 0.55 + 0.12 * Math.sin(-head) ; // drift back toward east
        const nx = x + Math.cos(head) * 230, nz = z + Math.sin(head) * 230;
        if (coastDist(nx, nz) < 1700) break;
        const k = 1 - i / (steps * 1.4);
        CANYONS.push({ ax: x, az: z, bx: nx, bz: nz, depth: depth * (0.55 + 0.45 * k), fh });
        if (i > 3 && rng() < 0.13) grow(nx, nz, head + (rng() < 0.5 ? -1 : 1) * (0.7 + rng() * 0.5), 8 + Math.floor(rng() * 12), depth * 0.7, fh * 0.6);
        x = nx; z = nz;
      }
    };
    for (const t of roots) {
      const p = spinePoint(t);
      const ang = Math.atan2(EAST[1], EAST[0]);
      grow(p[0] + EAST[0] * 4300, p[1] + EAST[1] * 4300, ang + (rng() - 0.5) * 0.4, 42, 160 + rng() * 90, 40 + rng() * 60);
    }
  }
  const ISLETS = [];
  {
    let tries = 0;
    while (ISLETS.length < 32 && tries++ < 6000) {
      const b = (200 + rng() * 55) * Math.PI / 180;
      const r = 13500 + rng() * 6500;
      const x = Math.sin(b) * r, z = -Math.cos(b) * r;
      if (Math.max(Math.abs(x), Math.abs(z)) > HALF_M - 1300) continue;
      const off = -coastDist(x, z);
      if (off < 2500 || off > 8000) continue;
      if (ISLETS.some((o) => Math.hypot(o.x - x, o.z - z) < 1150)) continue;
      const rad = 120 + Math.pow(rng(), 1.8) * 900;
      ISLETS.push({ x, z, r: rad, h: 14 + rng() * 66 * Math.min(1, rad / 500 + 0.3), cliff: rng() < 0.4 });
    }
  }

  // Segments in a grid of cells, for fast nearest queries.
  function cellIndex(segs, cell, reach) {
    const map = new Map();
    segs.forEach((s, i) => {
      const x0 = Math.floor((Math.min(s.ax, s.bx) - reach) / cell), x1 = Math.floor((Math.max(s.ax, s.bx) + reach) / cell);
      const z0 = Math.floor((Math.min(s.az, s.bz) - reach) / cell), z1 = Math.floor((Math.max(s.az, s.bz) + reach) / cell);
      for (let cx = x0; cx <= x1; cx++) for (let cz = z0; cz <= z1; cz++) {
        const k = cx * 100003 + cz;
        if (!map.has(k)) map.set(k, []);
        map.get(k).push(i);
      }
    });
    return (x, z) => map.get(Math.floor(x / cell) * 100003 + Math.floor(z / cell)) || [];
  }
  function segDist(s, x, z) {
    const dx = s.bx - s.ax, dz = s.bz - s.az;
    const u = clamp(((x - s.ax) * dx + (z - s.az) * dz) / (dx * dx + dz * dz), 0, 1);
    return { d: Math.hypot(x - s.ax - dx * u, z - s.az - dz * u), u };
  }
  const canyonCells = cellIndex(CANYONS, 600, 420);

  // ---------------------------------------------------------------- the height, before rivers
  function baseHeight(x, z, out) {
    const cd = coastDist(x, z);
    const b = bearing(x, z);
    const cliffW = sector(b, 18, 88, 10);
    const sp = spineAt(x, z);

    // Base island and its shelf; the north-east coast drops in sea cliffs instead of beaches.
    const hills = 38 * fbm(x / 3800, z / 3800, 4, SEED + 3) + 14 * fbm(x / 900, z / 900, 3, SEED + 4);
    let land = 20 + 30 * smooth(0, 2500, cd) + hills;
    const cliffTop = 95 + 55 * fbm(x / 3000, z / 3000, 3, SEED + 5);
    land = mix(land, Math.max(land, cliffTop), cliffW);
    const sea = -8 - Math.min(230, Math.max(0, -cd) * mix(0.05, 0.22, cliffW));
    const wl = mix(smooth(-220, 520, cd), smooth(-30, 6, cd), cliffW);
    let h = mix(sea, land, wl);

    // The range: a warped ridged multifractal under an envelope round the spine, with cliff bands.
    const taper = smooth(0.0, 0.16, sp.t) * (1 - smooth(0.84, 1.0, sp.t));
    const env = Math.exp(-Math.pow(sp.d / 4200, 2)) * taper;
    const foot = Math.exp(-Math.pow(sp.d / 7000, 2)) * smooth(0, 0.1, sp.t) * (1 - smooth(0.9, 1, sp.t));
    let range = 0;
    if (env > 0.002 || foot > 0.01) {
      const wx = x + 1200 * fbm(x / 4000, z / 4000, 3, SEED + 6);
      const wz = z + 1200 * fbm(x / 4000 + 9.2, z / 4000 - 4.1, 3, SEED + 7);
      const rm = ridged(wx / 5000, wz / 5000, 6, SEED + 8);
      range = env * (900 * (0.35 + 0.65 * rm) + 1300 * rm * rm) + foot * 260 * rm;
      const banded = terrace(range, 210, 0.25);
      range = mix(range, banded, 0.4 * smooth(250, 600, range));
      // Passes: pressed down to 500-700 m across a 1.5 km band.
      for (const pt of PASSES) {
        if (Math.abs(sp.t - pt) < 0.12) {
          const along = Math.abs(sp.t - pt) * 16000;
          const m = 1 - smooth(450, 1100, along);
          range = mix(range, Math.min(range, 560 + 0.12 * Math.max(0, range - 560)), m);
        }
      }
    }
    h += range * smooth(-200, 1500, cd);

    // Rain shadow: the plateau east of the range, edged by an escarpment.
    const plat = smooth(3300, 3650, sp.s) * smooth(700, 2600, cd) * smooth(0.02, 0.15, sp.t + 0.1) * (1 - smooth(0.92, 1.05, sp.t));
    let canyon = 0, mesa = 0, dune = 0;
    if (plat > 0) {
      let ph = 280 + 100 * fbm(x / 6000, z / 6000, 3, SEED + 9);
      // Mesas and buttes: terraced blobs on the plateau.
      const blob = fbm(x / 1700, z / 1700, 4, SEED + 10);
      mesa = smooth(0.34, 0.4, blob) * (70 + 110 * smooth(0.4, 0.75, blob));
      ph += terrace(mesa, 22, 0.3);
      // Dunes: a sand sea on part of the plateau, ridges across the wind.
      const sand = smooth(0.05, 0.25, fbm(x / 5200, z / 5200, 2, SEED + 11)) * (1 - smooth(0.2, 0.34, blob));
      if (sand > 0) {
        const ux = x * 0.94 + z * 0.34, uz = -x * 0.34 + z * 0.94;
        dune = sand * (6 + 16 * Math.pow(1 - Math.abs(perlin(ux / 320, uz / 900, SEED + 12)), 3));
        ph += dune;
      }
      // Canyons: a terraced cut, ledged walls of strata.
      let best = 1e9, seg = null;
      for (const i of canyonCells(x, z)) {
        const r = segDist(CANYONS[i], x, z);
        if (r.d < best) { best = r.d; seg = CANYONS[i]; }
      }
      if (seg) {
        const wall = 230;
        const cut = seg.depth * (1 - smooth(seg.fh, seg.fh + wall, best));
        canyon = terrace(cut, 18, 0.35);
        ph -= canyon;
      }
      h = Math.max(h, mix(h, h + ph, plat));
      if (h > 0 && canyon > 0) h = Math.max(h - canyon * (1 - plat), 4);
    }

    // The archipelago: small islands off the south-west, some cliffed.
    let isletQ = 9;
    if (cd < -1500) {
      for (const o of ISLETS) {
        const dd = Math.hypot(x - o.x, z - o.z);
        if (dd > o.r * 1.6) continue;
        const q = dd / (o.r * (1 + 0.32 * fbm(x / 380, z / 380, 3, SEED + 13)));
        isletQ = Math.min(isletQ, q);
        const top = o.h * (o.cliff ? smooth(1.0, 0.82, q) : smooth(1.05, 0.25, q)) + 6 * fbm(x / 160, z / 160, 2, SEED + 14) * smooth(1, 0.6, q);
        const hi = mix(sea, top, smooth(1.2, 0.9, q));
        h = Math.max(h, hi);
      }
    }
    if (out) { out.cd = cd; out.cliffW = cliffW; out.sp = sp; out.plat = plat; out.canyon = canyon; out.mesa = mesa; out.dune = dune; out.isletQ = isletQ; }
    return h;
  }

  // ---------------------------------------------------------------- rivers (traced downhill), falls, swamps
  const RIVERS = [];
  const FALLS = [];
  const SWAMPS = [];
  {
    const sources = [
      [0.18, -1500], [0.3, -1900], [0.42, -1300], [0.55, -2100], [0.7, -1600], [0.82, -1400],
      [0.36, 1500], [0.5, 1300], [0.76, 1700],
    ];
    for (const [t, s] of sources) {
      const p = spinePoint(t);
      let x = p[0] + EAST[0] * s, z = p[1] + EAST[1] * s;
      let dx = 0, dz = 0;
      const pts = [];
      let bed = baseHeight(x, z);
      for (let i = 0; i < 900; i++) {
        const h = baseHeight(x, z);
        bed = Math.min(bed, h);
        pts.push({ x, z, ground: h, bed });
        if (h < 0.3) break;
        const e = 30;
        const gx = (baseHeight(x + e, z) - baseHeight(x - e, z)) / (2 * e);
        const gz = (baseHeight(x, z + e) - baseHeight(x, z - e)) / (2 * e);
        let nx = -gx, nz = -gz;
        const gl = Math.hypot(nx, nz);
        if (gl < 1e-4) { nx = dx; nz = dz; } else { nx /= gl; nz /= gl; }
        if (i === 0) { dx = nx; dz = nz; }
        // Through a pit the river keeps its heading and leans out toward the sea, filling it.
        const rr = Math.hypot(x, z) || 1;
        dx = dx * 0.62 + nx * 0.38 + 0.12 * x / rr; dz = dz * 0.62 + nz * 0.38 + 0.12 * z / rr;
        const dl = Math.hypot(dx, dz) || 1; dx /= dl; dz /= dl;
        x += dx * 60; z += dz * 60;
      }
      if (pts.length < 20) continue;
      const river = { pts, west: s < 0 };
      RIVERS.push(river);
      // Falls: a drop of more than 25 m within one 60 m step; runs merge into one fall.
      for (let i = 1; i < pts.length; i++) {
        if (pts[i - 1].bed - pts[i].bed > 25) {
          let j = i;
          while (j + 1 < pts.length && pts[j].bed - pts[j + 1].bed > 12) j++;
          FALLS.push({ river: RIVERS.length - 1, i0: i - 1, i1: j, top: pts[i - 1].bed, foot: pts[j].bed, x: pts[j].x, z: pts[j].z, tx: pts[i - 1].x, tz: pts[i - 1].z });
          i = j;
        }
      }
      const mb = bearing(pts[pts.length - 1].x, pts[pts.length - 1].z);
      if (river.west && mb > 200 && mb < 320) {
        const m = pts[pts.length - 1];
        if (!SWAMPS.some((o) => Math.hypot(o.x - m.x, o.z - m.z) < 3500)) SWAMPS.push({ x: m.x, z: m.z, r: 2600 });
      }
    }
  }
  const RIVER_SEGS = [];
  RIVERS.forEach((rv, ri) => {
    for (let i = 0; i + 1 < rv.pts.length; i++) {
      const a = rv.pts[i], b = rv.pts[i + 1];
      const prog = i / rv.pts.length;
      RIVER_SEGS.push({ ax: a.x, az: a.z, bx: b.x, bz: b.z, bedA: a.bed, bedB: b.bed, hw: 24 + 26 * prog, ri, west: rv.west });
    }
  });
  const riverCells = cellIndex(RIVER_SEGS, 400, 260);

  // ---------------------------------------------------------------- the final height and fields
  function sample(x, z) {
    const f = {};
    let h = baseHeight(x, z, f);
    // Swamp flats at the western river mouths: flattened to 1-2 m with pools below the sea.
    let swamp = 0;
    for (const o of SWAMPS) {
      const m = (1 - smooth(o.r * 0.5, o.r, Math.hypot(x - o.x, z - o.z))) * smooth(34, 6, h) * smooth(-150, 200, f.cd);
      if (m > swamp) swamp = m;
    }
    if (swamp > 0) h = mix(h, 0.7 + 1.5 * fbm(x / 260, z / 260, 3, SEED + 15), swamp);
    // Rivers: a channel 4 m below the bed, banks at about 20 degrees; a plunge pool under each fall.
    let rdist = 1e9, murk = 0, water = -1e9;
    for (const i of riverCells(x, z)) {
      const s = RIVER_SEGS[i];
      const r = segDist(s, x, z);
      const bed = mix(s.bedA, s.bedB, r.u);
      const ch = bed - 4 + Math.max(0, r.d - s.hw) * 0.38;
      if (ch < h) h = ch;
      if (r.d < rdist) { rdist = r.d; murk = s.west ? 0.55 : 0.15; if (r.d < s.hw + 10) water = bed - 0.5; }
    }
    for (const fl of FALLS) {
      const d = Math.hypot(x - fl.x, z - fl.z);
      if (d < 140) h = Math.min(h, fl.foot - 9 * (1 - smooth(0, 140, d)));
    }
    if (swamp > 0.3) murk = Math.max(murk, swamp);
    return { h, cd: f.cd, cliffW: f.cliffW, s: f.sp.s, t: f.sp.t, spineD: f.sp.d, plat: f.plat, canyon: f.canyon, mesa: f.mesa, dune: f.dune, isletQ: f.isletQ, swamp, rdist, murk, water };
  }

  // ---------------------------------------------------------------- biomes
  const BIOMES = ['alpine', 'montane forest', 'jungle', 'swamp', 'temperate', 'savanna', 'desert', 'canyon', 'beach', 'cliff'];
  // Weights over the ten biomes from height, slope (degrees), side of the range, moisture, coast
  // and rivers. Borders wander with noise over 200-800 m.
  function biomeWeights(x, z, f, slopeDeg) {
    const h = f.h;
    const j = 220 * fbm(x / 1500, z / 1500, 3, SEED + 20);
    const moist = fbm(x / 3000, z / 3000, 3, SEED + 21);
    const b = bearing(x, z);
    const w = new Float32Array(10);
    const steep = smooth(40, 50, slopeDeg);
    const coastLow = smooth(160, 20, f.cd + j * 0.3) * smooth(14, 3, h) * (1 - f.cliffW);
    const isle = f.isletQ < 1.2 ? 1 : 0;
    w[0] = smooth(1450, 1700, h + j) + smooth(1000, 1250, h) * steep;
    w[1] = smooth(380, 520, h + j) * (1 - smooth(1500, 1700, h + j));
    const west = smooth(-600, -2600, f.s + j * 4);
    const south = smooth(-5000, 500, z + j * 5); // z grows southward
    const low = smooth(470, 330, h + j);
    w[2] = low * west * south * smooth(-0.45, 0.0, moist + 0.2) + isle * 0.6;
    w[3] = f.swamp * 2;
    const north = sector(b, 280, 70, 25);
    w[4] = low * (0.25 + north + (1 - south) * 0.8) * (1 - smooth(2000, 3400, f.s));
    w[5] = smooth(1200, 2600, f.s + j * 4) * (1 - smooth(3300, 3800, f.s + j * 2)) * smooth(500, 300, h);
    w[6] = smooth(3200, 3700, f.s + j * 2) * smooth(500, 1400, f.cd) * (1 - smooth(25, 70, f.canyon));
    w[7] = smooth(25, 70, f.canyon) * 1.5;
    w[8] = coastLow * 2.2;
    w[9] = steep * 1.2 * smooth(800, 1200, 2400 - h);
    let sum = 0;
    for (let i = 0; i < 10; i++) { w[i] = Math.max(0, w[i]); sum += w[i]; }
    if (sum < 1e-4) { w[4] = 1; sum = 1; }
    for (let i = 0; i < 10; i++) w[i] /= sum;
    return w;
  }

  const api = {
    SEED, WORLD_M, HALF_M, CLOUD_BASE_M, CLOUD_TOP_M, BIOMES,
    SPINE, PASSES, CANYONS, ISLETS, RIVERS, FALLS, SWAMPS, EAST, NORTH,
    mulberry, perlin, fbm, ridged, smooth, mix, clamp, bearing, coastDist, spineAt, spinePoint,
    baseHeight, sample, biomeWeights,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.IslandGen = api;
})(this);
