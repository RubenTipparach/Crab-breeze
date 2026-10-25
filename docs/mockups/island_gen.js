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
    let best = 1e18, bt = 0, side = 0, perp = 0;
    for (let i = 0; i < 64; i++) {
      const a = SPINE[i], b = SPINE[i + 1];
      const dx = b[0] - a[0], dz = b[1] - a[1];
      const l2 = dx * dx + dz * dz;
      const u = clamp(((x - a[0]) * dx + (z - a[1]) * dz) / l2, 0, 1);
      const px = a[0] + dx * u, pz = a[1] + dz * u;
      const d2 = (x - px) * (x - px) + (z - pz) * (z - pz);
      if (d2 < best) { best = d2; bt = (i + u) / 64; side = dx * (z - a[1]) - dz * (x - a[0]); perp = side / Math.sqrt(l2); }
    }
    const d = Math.sqrt(best);
    // dx*(z-az) - dz*(x-ax) is positive to the east of a northbound line (z south).
    // perp: the signed distance from the nearest segment's line. Past the spine's ends it stays
    // continuous, where s jumps from -d to +d across the line the spine would continue on.
    return { d, t: bt, s: side > 0 ? d : -d, perp };
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
  // Each fall's face stands halfway from its lip point to its foot point. The water leaves the lip
  // at the river's surface speed, 3 m/s, and falls freely: after falling y it is 3 sqrt(2y / g) m
  // out from the face, a parabola. It lands `throw` m out, where the plunge pool is.
  const FALL_LIP_SPEED_M_S = 3, G_M_S2 = 9.81;
  for (const f of FALLS) {
    const ex = f.x - f.tx, ez = f.z - f.tz, el = Math.hypot(ex, ez) || 1;
    f.dx = ex / el; f.dz = ez / el; f.run = el;
    f.fx = f.tx + ex * 0.5; f.fz = f.tz + ez * 0.5;
    f.throw = FALL_LIP_SPEED_M_S * Math.sqrt(2 * (f.top - f.foot) / G_M_S2);
    f.lx = f.fx + f.dx * f.throw; f.lz = f.fz + f.dz * f.throw;
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
  // Cities (placed below, once sample() exists): squares flattened to their ground level.
  const CITIES = [];
  const CITY_BLEND_M = 240; // the game blends over 12 + 120 m; the mockup's 80 m grid needs more
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
    for (const c of CITIES) {
      const e = Math.max(Math.abs(x - c.x), Math.abs(z - c.z));
      if (e < c.half + CITY_BLEND_M) h = mix(h, c.ground, 1 - smooth(c.half, c.half + CITY_BLEND_M, e));
    }
    // Rivers: a channel 4 m below the bed, banks at about 20 degrees; a plunge pool under each fall.
    let rdist = 1e9, murk = 0, water = -1e9;
    for (const i of riverCells(x, z)) {
      const s = RIVER_SEGS[i];
      const r = segDist(s, x, z);
      const bed = mix(s.bedA, s.bedB, r.u);
      const ch = bed - 4 + Math.max(0, r.d - s.hw) * 0.38;
      if (ch < h) h = ch;
      if (r.d < rdist) { rdist = r.d; murk = (s.west ? 0.55 : 0.15) * smooth(150, 20, bed); if (r.d < s.hw + 10) water = bed - 0.5; }
    }
    for (const fl of FALLS) {
      // A face: across the fall's 300 m corridor the ground steps from the lip's height to the
      // foot's at the face (on the 80 m grid that is one steep cell; the game's ground is 1 m).
      const u = ((x - fl.tx) * fl.dx + (z - fl.tz) * fl.dz) / fl.run;
      if (u > -0.05 && u < 1.05) {
        // Only between its own lip and foot: falls follow each other down a river.
        const across = Math.abs((x - fl.tx) * fl.dz - (z - fl.tz) * fl.dx);
        const k = (1 - smooth(90, 160, across)) * smooth(-0.05, 0.05, u) * (1 - smooth(0.95, 1.05, u));
        if (k > 0) h = mix(h, mix(fl.top, fl.foot, smooth(0.47, 0.53, u)), k);
      }
      // The plunge pool where the water lands, downstream of the face only.
      const d = Math.hypot(x - fl.lx, z - fl.lz);
      if (d < 140 && u > 0.52) h = Math.min(h, fl.foot - 9 * (1 - smooth(0, 140, d)));
    }
    if (swamp > 0.3) murk = Math.max(murk, swamp);
    return { h, cd: f.cd, cliffW: f.cliffW, s: f.sp.s, perp: f.sp.perp, t: f.sp.t, spineD: f.sp.d, plat: f.plat, canyon: f.canyon, mesa: f.mesa, dune: f.dune, isletQ: f.isletQ, swamp, rdist, murk, water };
  }

  // ---------------------------------------------------------------- cities and highways
  // The game's placement (cb-core world::layout::place_cities, data/world/world.ron): 24 squares of
  // half-size 280-480 m, centres 2.6 km apart with 600 m of open ground between, the capital the
  // largest. island-biomes section 4 adds: ground below 300 m, under 6 degrees across the square,
  // the capital on the northern plain within 6 km of the centre, an oasis city at a canyon mouth
  // and a river city in the jungle.
  {
    const crng = mulberry(SEED * 31 + 7);
    const TAN6 = Math.tan(6 * Math.PI / 180);
    const fits = (cx, cz, half, opts) => {
      if (CITIES.some((o) => Math.hypot(o.x - cx, o.z - cz) < 2600 ||
        Math.max(Math.abs(o.x - cx), Math.abs(o.z - cz)) - half - o.half < 600)) return null;
      let lo = 1e9, hi = -1e9, sum = 0, n = 0;
      for (let a = -1; a <= 1; a += 0.5) for (let b = -1; b <= 1; b += 0.5) {
        const f = sample(cx + a * half, cz + b * half);
        if (f.h < 3 || f.h > (opts.maxH || 300) || f.canyon > 5 || f.swamp > 0.3) return null;
        if (!opts.river && f.rdist < 80) return null;
        lo = Math.min(lo, f.h); hi = Math.max(hi, f.h); sum += f.h; n++;
      }
      if (hi - lo > TAN6 * 2 * half || coastDist(cx, cz) < half) return null;
      return sum / n;
    };
    const put = (x, z, half, kind) => {
      const g = fits(x, z, half, kind === 'oasis' ? { maxH: 420, river: true } : { river: kind === 'river' });
      if (g === null) return false;
      CITIES.push({ x, z, half, ground: Math.max(g, 4), kind, seed: CITIES.length * 97 + 13 });
      return true;
    };
    // The capital: the site nearest the centre on the northern half that fits. With the range down
    // the middle the nearest is about 7.5 km out, past the design's 6 km.
    const sites = [];
    for (let z = -12000; z <= -1000; z += 500) for (let x = -12000; x <= 12000; x += 500) sites.push([Math.hypot(x, z), x, z]);
    sites.sort((a, b) => a[0] - b[0]);
    for (const [, x, z] of sites) if (put(x, z, 480, 'capital')) break;
    // The oasis: past the seaward end of a canyon branch, as close to the mouth as fits (the ground
    // at the mouths themselves is too rough for 6 degrees).
    const ends = CANYONS.filter((c) => !CANYONS.some((o) => o.ax === c.bx && o.az === c.bz));
    oasis: for (let d = 700; d <= 3100; d += 300) {
      for (const c of ends) {
        const l = Math.hypot(c.bx - c.ax, c.bz - c.az) || 1;
        if (put(c.bx + (c.bx - c.ax) / l * d, c.bz + (c.bz - c.az) / l * d, 320, 'oasis')) break oasis;
      }
    }
    // The river city: on a western river, from the mouth upstream, where the ground is low and wet.
    outer: for (const rv of RIVERS.filter((r) => r.west)) {
      for (let i = rv.pts.length - 30; i > 10; i -= 6) {
        const p = rv.pts[i];
        if (p.ground > 8 && p.ground < 160 && put(p.x, p.z, 340, 'river')) break outer;
      }
    }
    for (let t = 0; t < 4000 && CITIES.length < 24; t++) {
      const half = 280 + crng() * 200;
      put((crng() - 0.5) * 2 * (HALF_M - 300 - half - 132), (crng() - 0.5) * 2 * (HALF_M - 300 - half - 132), half, 'town');
    }
  }
  // Highways: a minimum spanning tree over the cities (Prim, nearest first), each road crossing the
  // range by the nearer pass. A road is a polyline of [x, z] points.
  const HIGHWAYS = [];
  if (CITIES.length > 1) {
    const inTree = [0];
    const passPts = PASSES.map((t) => spinePoint(t));
    while (inTree.length < CITIES.length) {
      let best = null;
      for (const i of inTree) CITIES.forEach((c, j) => {
        if (inTree.includes(j)) return;
        const d = Math.hypot(c.x - CITIES[i].x, c.z - CITIES[i].z);
        if (!best || d < best.d) best = { i, j, d };
      });
      inTree.push(best.j);
      const a = CITIES[best.i], b = CITIES[best.j];
      const sa = spineAt(a.x, a.z), sb = spineAt(b.x, b.z);
      const route = [[a.x, a.z]];
      if (Math.sign(sa.s) !== Math.sign(sb.s) && Math.min(Math.abs(sa.s), Math.abs(sb.s)) > 800) {
        const p = passPts.slice().sort((u, v) => (Math.hypot(u[0] - a.x, u[1] - a.z) + Math.hypot(u[0] - b.x, u[1] - b.z)) -
          (Math.hypot(v[0] - a.x, v[1] - a.z) + Math.hypot(v[0] - b.x, v[1] - b.z)))[0];
        route.push([p[0] - EAST[0] * 1200, p[1] - EAST[1] * 1200 * Math.sign(sa.s) * -1]);
        route.push([p[0], p[1]]);
      }
      route.push([b.x, b.z]);
      HIGHWAYS.push(route);
    }
  }

  // ---------------------------------------------------------------- weather
  // The prevailing wind blows from the west (+x) at 6 m/s: the west is wet, the east in the rain
  // shadow. Weather is a set of systems, each a rule that lays out cloud puffs:
  // - fair-weather cumulus over the land, thick where the air is wet and thin over the desert;
  // - storm cells (cumulonimbus): towers from 700 m to 7-10 km, an anvil spread downwind at the
  //   top, dark bases and a rain shaft under each;
  // - a stratocumulus deck over the sea to the north-west, 700-1,000 m;
  // - orographic cloud banked against the range's windward (west) flank, 900-1,700 m;
  // - lenticular caps over the highest peaks;
  // - cirrus streaks at 8-9 km.
  // A puff is { x, y, z, rx, ry, kind, dark, seed, base, top }: an ellipsoid of radii rx (across)
  // and ry (up), lit from base (m) to top (m) of its system; dark 0 white to 1 storm grey.
  const WEATHER = { puffs: [], shafts: [], cirrus: [], storms: [] };
  {
    const wr = mulberry(SEED * 53 + 1);
    const P = WEATHER.puffs;
    const puff = (x, y, z, rx, ry, kind, dark, base, top) => P.push({ x, y, z, rx, ry, kind, dark, seed: wr() * 100, base, top });
    // Storm cells: one over the western jungle, one at sea off the archipelago, one decaying to the north-west.
    const west = spinePoint(0.62);
    const cells = [
      { x: west[0] - EAST[0] * 6500, z: west[1] - EAST[1] * 6500, r: 2600, top: 9500 },
      { x: -15500, z: 9500, r: 3200, top: 10500 },
      { x: -11000, z: -14000, r: 1900, top: 6500 },
    ];
    for (const c of cells) {
      WEATHER.storms.push({ x: c.x, z: c.z, r: c.r * 1.6 });
      // The tower: rings of puffs, widest low, narrowing, then the anvil.
      for (let y = 750; y < c.top * 0.86; y += 260) {
        const t = y / c.top;
        const ring = c.r * (0.95 - 0.35 * t) * (0.85 + 0.3 * wr());
        const n = Math.max(3, Math.round(ring / 260));
        for (let k = 0; k < n; k++) {
          const a = wr() * 6.283, d = Math.sqrt(wr()) * ring;
          const rr = 380 + wr() * 420 + 260 * t;
          puff(c.x + Math.cos(a) * d + t * 900, y + wr() * 120, c.z + Math.sin(a) * d, rr, rr * 0.85, 1, 0.75 - 0.6 * t, 700, c.top);
        }
      }
      for (let k = 0; k < 70; k++) {
        const a = wr() * 6.283, d = Math.sqrt(wr()) * c.r * 2.4;
        // The anvil leans downwind (east).
        puff(c.x + Math.cos(a) * d + 2600, c.top * (0.86 + 0.08 * wr()), c.z + Math.sin(a) * d * 0.7, 1300 + wr() * 900, 260 + wr() * 160, 2, 0.15, 700, c.top);
      }
      // Ragged scud under the base.
      for (let k = 0; k < 24; k++) {
        const a = wr() * 6.283, d = Math.sqrt(wr()) * c.r;
        puff(c.x + Math.cos(a) * d, 520 + wr() * 160, c.z + Math.sin(a) * d, 260 + wr() * 260, 90 + wr() * 60, 3, 0.85, 450, 900);
      }
      WEATHER.shafts.push({ x: c.x + 300, z: c.z, r: c.r * 0.75, top: 720 });
    }
    const stormAt = (x, z) => WEATHER.storms.some((s) => Math.hypot(x - s.x, z - s.z) < s.r);
    // Fair-weather cumulus: clusters on a 2 km grid, more over wet ground, few over the desert.
    for (let tz = -19000; tz < 19000; tz += 2000) for (let tx = -19000; tx < 19000; tx += 2000) {
      const cx0 = tx + 1000, cz0 = tz + 1000;
      const f = sample(cx0, cz0);
      const onLand = f.cd > 0;
      const m = onLand ? moisture(cx0, cz0, f).m : 0.45;
      let count = (onLand ? 0.4 + 2.6 * m : 1.1) + 0.9 * fbm(tx / 9000, tz / 9000, 2, 55);
      if (stormAt(cx0, cz0)) count = 0;
      for (let c = 0, n = Math.floor(count + wr()); c < n; c++) {
        const cx = tx + wr() * 2000, cz = tz + wr() * 2000;
        const big = wr() < 0.25 * (onLand ? m : 0.4);
        const puffs = big ? 14 + Math.floor(wr() * 10) : 5 + Math.floor(wr() * 7);
        const spread = big ? 600 : 260 + wr() * 260;
        const tops = big ? 2200 + wr() * 1400 : 1100;
        for (let p = 0; p < puffs; p++) {
          const r = 90 + wr() * 200 + (big ? 120 : 0);
          const y = CLOUD_BASE_M + r * 0.7 + wr() * Math.max(10, (tops - CLOUD_BASE_M) * (big ? 0.75 : 0.35));
          puff(cx + (wr() - 0.5) * spread * 2, y, cz + (wr() - 0.5) * spread * 1.4, r * 1.15, r, 0, big ? 0.3 : 0.1, CLOUD_BASE_M, tops);
        }
      }
    }
    // The stratocumulus deck over the sea to the north-west: wide flat puffs that merge into a sheet.
    for (let k = 0; k < 900; k++) {
      const b = (275 + wr() * 70) * Math.PI / 180, r = 14500 + wr() * 7000;
      const x = Math.sin(b) * r, z = -Math.cos(b) * r;
      if (coastDist(x, z) > -600 || Math.max(Math.abs(x), Math.abs(z)) > HALF_M + 2000) continue;
      if (fbm(x / 4000, z / 4000, 2, 61) < -0.25) continue; // breaks in the deck
      puff(x, 780 + wr() * 160, z, 520 + wr() * 420, 120 + wr() * 70, 3, 0.25 + 0.2 * wr(), 700, 1000);
    }
    // Orographic cloud on the windward flank of the range, hugging the ground.
    for (let k = 0; k < 520; k++) {
      const t = 0.14 + wr() * 0.72, p = spinePoint(t);
      const off = -(600 + wr() * 2600);
      const x = p[0] + EAST[0] * off, z = p[1] + EAST[1] * off;
      const g = baseHeight(x, z);
      if (g < 500) continue;
      // Clear of the ground beneath: a billboard that dips into a slope ends in a hard line there.
      const rx = 260 + wr() * 260, ry = 120 + wr() * 90;
      puff(x, Math.min(g + ry * 1.4 + 80 + wr() * 220, 1900), z, rx, ry, 4, 0.2, 800, 2000);
    }
    // Lenticular caps over the peaks above 1,900 m.
    for (let k = 0; k < 4000 && WEATHER.puffs.filter((q) => q.kind === 5).length < 5; k++) {
      const t = 0.15 + wr() * 0.7, p = spinePoint(t);
      const x = p[0] + (wr() - 0.5) * 3000, z = p[1] + (wr() - 0.5) * 3000;
      const g = baseHeight(x, z);
      if (g < 1900 || WEATHER.puffs.some((q) => q.kind === 5 && Math.hypot(q.x - x, q.z - z) < 2500)) continue;
      for (let l = 0; l < 3; l++) puff(x + 300, g + 280 + l * 110, z, 900 - l * 180, 70, 5, 0.05, g, g + 600);
    }
    // Cirrus: long thin streaks high up, along the wind.
    for (let k = 0; k < 90; k++) {
      WEATHER.cirrus.push({ x: (wr() - 0.5) * 50000, y: 8200 + wr() * 1200, z: (wr() - 0.5) * 50000 + 6000 * fbm(k, 0, 1, 70),
        len: 5000 + wr() * 9000, wid: 500 + wr() * 900, ang: (wr() - 0.5) * 0.5, seed: wr() * 100 });
    }
  }

  // ---------------------------------------------------------------- biomes
  const BIOMES = ['alpine', 'montane forest', 'jungle', 'swamp', 'temperate', 'savanna', 'desert', 'canyon', 'beach', 'cliff'];
  // Moisture, 0 (desert) to 1 (rain forest): the rain shadow east of the spine over a 7 km band,
  // wetter to the south, by rivers and in the swamps, with 3 km noise. Its inputs are warped by
  // 900 m of noise so the bands meander instead of following the spine's curve.
  function moisture(x, z, f) {
    const ws = f.perp + 900 * fbm(x / 2600, z / 2600, 3, SEED + 22);
    const south = smooth(-9000, 4000, z + 1500 * fbm(x / 5000, z / 5000, 2, SEED + 24));
    let m = 1 - smooth(-1800, 5200, ws);
    m = m * (0.72 + 0.36 * south) + 0.22 * (fbm(x / 3000, z / 3000, 3, SEED + 21)) + 0.18 * smooth(250, 40, f.rdist) + 0.25 * f.swamp;
    return { m: clamp(m, 0, 1), south };
  }
  // Weights over the ten biomes from height, slope (degrees), moisture, coast and rivers. Every
  // border is a smoothstep over a band of a field that itself wanders, so none is a straight line,
  // and the dry side runs down to its beaches instead of turning to grass at the coast.
  function biomeWeights(x, z, f, slopeDeg) {
    const h = f.h;
    const j = 220 * fbm(x / 1500, z / 1500, 3, SEED + 20);
    const { m, south } = moisture(x, z, f);
    const w = new Float32Array(10);
    const hT = h + j;
    const steep = smooth(40, 50, slopeDeg);
    const coastLow = smooth(160, 20, f.cd + j * 0.3) * smooth(14, 3, h) * (1 - f.cliffW);
    const isle = f.isletQ < 1.2 ? 1 : 0;
    const low = smooth(540, 360, hT);
    const mid = smooth(360, 540, hT) * (1 - smooth(1500, 1750, hT));
    w[0] = smooth(1450, 1750, hT) + smooth(1000, 1250, h) * steep;
    w[1] = mid * smooth(0.25, 0.45, m);
    w[2] = low * smooth(0.6, 0.8, m) * (0.3 + 0.7 * south) + isle * 0.6;
    w[3] = f.swamp * 2;
    w[4] = low * smooth(0.3, 0.48, m) * (1 - smooth(0.68, 0.86, m * (0.55 + 0.45 * south)));
    w[5] = (low + 0.6 * mid) * smooth(0.1, 0.24, m) * (1 - smooth(0.32, 0.46, m));
    w[6] = smooth(0.22, 0.08, m) * (1 - smooth(1100, 1400, hT)) * (1 - smooth(25, 70, f.canyon));
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
    baseHeight, sample, biomeWeights, moisture, FALL_LIP_SPEED_M_S, G_M_S2, CITIES, HIGHWAYS, WEATHER, CITY_BLEND_M,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.IslandGen = api;
})(this);
