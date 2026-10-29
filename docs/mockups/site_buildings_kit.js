// Large buildings, structures and machines for the airport and the seaport (openspec/changes/airport-layout,
// seaport-layout), built from the small-buildings kit's own parts (docs/mockups/small_buildings_kit.js) so they reach
// the suburban houses' standard (owner, 2026-10-29: "Airport is not up to standard of like suburban houses"): real walls
// with framed, glazed openings, roll-up and hangar doors, floors and roofs in panels, columns, stairs, furniture, signs,
// every piece breakable by the kit's support rule. Each function draws into a kit Builder in metres, the building's
// front facing +z, standing on y = 0; a page places it with BuildingEngine.add (docs/mockups/building_engine.js).
//
// A big building is built the way one is: bays of at most BAY metres between columns, floors and roofs in TILE metre
// panels, so a shot takes out a bay, not a whole facade. Angled members (a jet bridge's tunnel, an arch, a crane's boom)
// are oriented plates: each a piece the engine drops as a tilted chunk of its material.
(function (root) {
  'use strict';
  const K = root.SmallBuildings || (typeof require !== 'undefined' ? require('./small_buildings_kit.js') : null);
  const { wall, tiles, place, FURN } = K;
  const BAY = 6, TILE = 5;

  // ---------------------------------------------------------------- vectors and oriented plates
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const mul = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const len = (a) => Math.hypot(a[0], a[1], a[2]);
  const norm = (a) => { const l = len(a) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };
  // An oriented box: centre c, unit axes [a0, a1, a2], sizes [s0, s1, s2]; a piece of its own (falls as one tilted chunk).
  function plate(B, mat, c, axes, size) {
    B.group(() => {
      const h = size.map((s) => s / 2);
      const P = (i, j, k) => add(add(add(c, mul(axes[0], i * h[0])), mul(axes[1], j * h[1])), mul(axes[2], k * h[2]));
      const face = (n, pts) => B.poly(mat, pts, n);
      face(axes[0], [P(1, -1, -1), P(1, 1, -1), P(1, 1, 1), P(1, -1, 1)]);
      face(mul(axes[0], -1), [P(-1, -1, -1), P(-1, -1, 1), P(-1, 1, 1), P(-1, 1, -1)]);
      face(axes[1], [P(-1, 1, -1), P(-1, 1, 1), P(1, 1, 1), P(1, 1, -1)]);
      face(mul(axes[1], -1), [P(-1, -1, -1), P(1, -1, -1), P(1, -1, 1), P(-1, -1, 1)]);
      face(axes[2], [P(-1, -1, 1), P(1, -1, 1), P(1, 1, 1), P(-1, 1, 1)]);
      face(mul(axes[2], -1), [P(-1, -1, -1), P(-1, 1, -1), P(1, 1, -1), P(1, -1, -1)]);
      B.meta[B.cur] = { mat, tint: B.tint.slice(), panel: { c, size, axes } };
    });
  }
  // A member from p to q, w wide and h deep (a beam, a tunnel, a boom, a stay), as one oriented plate.
  function member(B, mat, p, q, w, h) {
    const d = norm(sub(q, p));
    let s = cross(d, [0, 1, 0]); if (len(s) < 1e-6) s = [1, 0, 0]; s = norm(s);
    const n = norm(cross(s, d));
    plate(B, mat, mul(add(p, q), 0.5), [d, n, s], [len(sub(q, p)), h, w]);
  }
  // A round tube from p to q, radius r, as one piece (it falls as itself, not as a box of its material).
  function tube(B, mat, p, q, r, seg = 8) {
    const d = norm(sub(q, p)); let u = cross(d, [0, 1, 0]); if (len(u) < 1e-6) u = [1, 0, 0]; u = norm(u); const w = cross(d, u);
    const ring = (c) => Array.from({ length: seg }, (_, i) => { const a = 2 * Math.PI * i / seg; return add(c, add(mul(u, Math.cos(a) * r), mul(w, Math.sin(a) * r))); });
    B.group(() => {
      const A = ring(p), Q = ring(q);
      for (let i = 0; i < seg; i++) { const j = (i + 1) % seg, m = norm(add(sub(A[i], p), sub(A[j], p))); B.poly(mat, [A[i], A[j], Q[j], Q[i]], m); }
      B.poly(mat, A.slice().reverse(), mul(d, -1)); B.poly(mat, Q, d);
    });
  }
  // A tube from p to q cut into lengths of at most `piece` metres, each its own piece, overlapping 5 cm so they touch.
  function tubes(B, mat, p, q, r, piece = 3, seg = 8) {
    const L = len(sub(q, p)), n = Math.max(1, Math.ceil(L / piece - 1e-6)), u = norm(sub(q, p));
    for (let i = 0; i < n; i++) tube(B, mat, add(p, mul(u, Math.max(0, L * i / n - 0.05))), add(p, mul(u, Math.min(L, L * (i + 1) / n + 0.05))), r, seg);
  }
  // A vertical cylinder from y0 to y1 cut into lifts of at most `lift` metres, each a piece.
  function column(B, mat, x, z, r, y0, y1, seg = 12, lift = 6) {
    const n = Math.max(1, Math.ceil((y1 - y0) / lift - 1e-6));
    for (let i = 0; i < n; i++) B.cyl(mat, 'y', [x, 0, z], r, y0 + (y1 - y0) * i / n, y0 + (y1 - y0) * (i + 1) / n, seg);
  }
  // A vertical box post cut into lifts of at most `lift` metres (a crane's leg: a hit takes out a length of it).
  function post(B, mat, x0, z0, x1, z1, y0, y1, lift = 4) {
    const n = Math.max(1, Math.ceil((y1 - y0) / lift - 1e-6));
    for (let i = 0; i < n; i++) B.box(mat, x0, y0 + (y1 - y0) * i / n, z0, x1, y0 + (y1 - y0) * (i + 1) / n, z1);
  }
  // A long box cut into lengths of at most `piece` metres along its longest side (a girder, a parapet, a crown wall).
  function run(B, mat, x0, y0, z0, x1, y1, z1, piece = BAY) {
    const lx = x1 - x0, lz = z1 - z0;
    const n = Math.max(1, Math.ceil(Math.max(lx, lz) / piece - 1e-6));
    for (let i = 0; i < n; i++) {
      if (lx >= lz) B.box(mat, x0 + lx * i / n, y0, z0, x0 + lx * (i + 1) / n, y1, z1);
      else B.box(mat, x0, y0, z0 + lz * i / n, x1, y1, z0 + lz * (i + 1) / n);
    }
  }

  // ---------------------------------------------------------------- facades
  // Opening makers for one bay [a, b] of one storey [y0, y1]: what a real facade of each kind has in a structural bay.
  const BAYS = {
    blank: () => [],
    windows: (a, b, y0, y1) => (b - a > 2.2 ? [{ kind: 'win', a: a + 0.9, b: b - 0.9, y0: y0 + 1.0, y1: Math.min(y1 - 0.6, y0 + 2.4), frameTint: 0xd8dcde }] : []),
    ribbon: (a, b, y0, y1) => [{ kind: 'ribbon', a: a + 0.25, b: b - 0.25, y0: y0 + 1.0, y1: y1 - 0.8, frameTint: 0x8d9196, sill: false, lights: Math.max(2, Math.round((b - a) / 1.5)) }],
    glazed: (a, b, y0, y1) => [{ kind: 'pic', a: a + 0.2, b: b - 0.2, y0: y0 + 0.3, y1: y1 - 0.5, frameTint: 0x5a5f66, frameMat: 'metal', sill: false, lights: Math.max(2, Math.round((b - a) / 1.5)) }],
    entrance: (a, b, y0, y1) => [{ kind: 'shop', a: a + 0.2, b: b - 0.2, y0, y1: Math.min(y1 - 0.4, y0 + 3.4), door: 1, frameTint: 0x8d9196 }],
    roll: (a, b, y0, y1, o) => [{ kind: 'roll', a: a + 0.3, b: b - 0.3, y0, y1: Math.min(y1 - 0.5, y0 + (o.doorH || 4.5)), doorTint: o.doorTint || 0xe9e6de }],
    door: (a, b, y0, y1) => [{ kind: 'door', a: (a + b) / 2 - 0.5, b: (a + b) / 2 + 0.5, y0, y1: y0 + 2.2, doorMat: 'metal', doorTint: 0x8d9196, stoop: false }],
    open: (a, b, y0, y1) => [{ kind: 'gap', a, b, y0, y1 }],
  };
  // One face: along `axis` at `c`, from s0 to s1, `out` +1 when outside is on its + side, for each storey of `storeys`
  // ([y0, y1] pairs), bays of at most BAY metres each filled by `spec(bay, storey)` (a BAYS key or a function).
  function face(B, o, axis, c, s0, s1, out, spec) {
    const n = Math.max(1, Math.round((s1 - s0) / (o.bay || BAY)));
    o.storeys.forEach(([y0, y1], st) => {
      for (let i = 0; i < n; i++) {
        const a = s0 + (s1 - s0) * i / n, b = s0 + (s1 - s0) * (i + 1) / n;
        const kind = typeof spec === 'function' ? spec(i, st, n) : spec;
        const ops = (BAYS[kind] || BAYS.blank)(a, b, y0, y1, o);
        B.color(o.tint);
        wall(B, o.wall, axis, c, a, b, o.t, y0, y1, out, ops, { inner: o.inner === false ? null : 'plaster' });
      }
    });
  }
  // A hall: walls on four faces in bays, a slab, upper floors and a flat roof in panels, interior columns on a grid.
  // o: { x0, x1, z0, z1, storeys: [[y0, y1]...], wall, tint, t, front, back, left, right (bay specs), col (grid, metres),
  //      roof: { parapet, tint, wall }, floorMat, furnish(B) }
  function hall(B, o) {
    o = Object.assign({ t: 0.3, wall: 'block', tint: 0xe2ddd2, col: 12, bay: BAY }, o);
    const { x0, x1, z0, z1, t } = o, top = o.storeys[o.storeys.length - 1][1];
    B.color(0xbab4aa); tiles(B, 'concrete', x0, -0.3, z0, x1, o.storeys[0][0], z1, null, TILE);
    face(B, o, 'x', z1 - t / 2, x0, x1, 1, o.front || 'windows');
    face(B, o, 'x', z0 + t / 2, x0, x1, -1, o.back || 'blank');
    face(B, o, 'z', x0 + t / 2, z0 + t, z1 - t, -1, o.left || 'blank');
    face(B, o, 'z', x1 - t / 2, z0 + t, z1 - t, 1, o.right || 'blank');
    // interior columns, one lift a storey, under every floor and the roof
    const nx = Math.max(1, Math.round((x1 - x0) / o.col)), nz = Math.max(1, Math.round((z1 - z0) / o.col));
    o.storeys.forEach(([y0, y1]) => {
      for (let i = 1; i < nx; i++) for (let k = 1; k < nz; k++) {
        const x = x0 + (x1 - x0) * i / nx, z = z0 + (z1 - z0) * k / nz;
        B.color(0xd8d4cc); B.box('concrete', x - 0.3, y0, z - 0.3, x + 0.3, y1 - 0.3, z + 0.3);
      }
    });
    // floors between storeys and the roof, in panels resting on the walls and the columns
    o.storeys.forEach(([, y1], st) => {
      const last = st === o.storeys.length - 1;
      B.color(last ? 0x6d6a66 : 0xbab4aa);
      tiles(B, last ? 'roofing' : 'concrete', x0 + t, y1 - 0.3, z0 + t, x1 - t, y1 + (last ? 0.006 : 0), z1 - t, null, TILE);
      if (!last) { B.color(0xffffff); tiles(B, o.floorMat || 'floorTile', x0 + t, y1, z0 + t, x1 - t, y1 + 0.012, z1 - t, null, TILE); }
    });
    B.color(0xffffff); tiles(B, o.floorMat || 'floorTile', x0 + t, o.storeys[0][0], z0 + t, x1 - t, o.storeys[0][0] + 0.012, z1 - t, null, TILE);
    // the parapet and its coping, in lengths
    const par = o.roof && o.roof.parapet != null ? o.roof.parapet : 0.8;
    if (par > 0) {
      B.color(o.tint);
      run(B, o.wall, x0, top, z1 - t, x1, top + par, z1); run(B, o.wall, x0, top, z0, x1, top + par, z0 + t);
      run(B, o.wall, x0, top, z0 + t, x0 + t, top + par, z1 - t); run(B, o.wall, x1 - t, top, z0 + t, x1, top + par, z1 - t);
      B.color(0xd8d2c6);
      run(B, 'concrete', x0 - 0.05, top + par, z1 - t - 0.05, x1 + 0.05, top + par + 0.08, z1 + 0.05);
      run(B, 'concrete', x0 - 0.05, top + par, z0 - 0.05, x1 + 0.05, top + par + 0.08, z0 + t + 0.05);
    }
    if (o.furnish) o.furnish(B);
    return top + par;
  }
  // A sign band and its sign on the front at height y.
  function signBand(B, M, text, x, y, z, w, bg = '#1d4f8f', fg = '#ffffff') {
    B.color(parseInt(bg.slice(1), 16)); B.box('paint', x - w / 2 - 0.3, y - 0.6, z, x + w / 2 + 0.3, y + 0.6, z + 0.06);
    if (M) B.sign(M.sign(text, bg, fg), x, y, z + 0.065, w, 1.0, '+z');
  }
  // Rows of airport seats, a counter, a desk: dressing for a hall's floor at height y.
  function seats(B, x0, x1, z, y, n = 1) {
    for (let r = 0; r < n; r++) for (let x = x0; x + 0.6 <= x1; x += 0.62) place(B, x + 0.3, y, z + r * 1.8, 2, FURN.chair(0x2f5f8f));
  }

  // ---------------------------------------------------------------- fitting out (owner, 2026-10-29: "ferry terminals and
  // warehouses are empty, need furniture"). Each item is one piece, placed with the kit's place() like the houses' rooms.
  const hash = (a, b) => { let n = Math.imul((a * 73856093) ^ (b * 19349663), 0x27d4eb2d); n ^= n >>> 15; return ((n >>> 0) % 1000) / 1000; };
  const GOODS = [0xb08a5a, 0x9a7448, 0x2f5f8f, 0xc23b30, 0x3f6e3a, 0xd8d2c0, 0x6b4a2e];
  // One bay of pallet racking, 2.7 m along x, 1.1 m deep, three levels, with loaded pallets.
  const rackBay = (k) => {
    const it = [['metal', -1.35, 0, -0.55, -1.27, 5.6, -0.47, 0x2f5f9a], ['metal', 1.27, 0, -0.55, 1.35, 5.6, -0.47, 0x2f5f9a],
      ['metal', -1.35, 0, 0.47, -1.27, 5.6, 0.55, 0x2f5f9a], ['metal', 1.27, 0, 0.47, 1.35, 5.6, 0.55, 0x2f5f9a]];
    for (const [lv, y] of [[0, 0], [1, 1.9], [2, 3.8]]) {
      if (lv) it.push(['metal', -1.27, y - 0.12, -0.55, 1.27, y, 0.55, 0xe07a2a]);
      for (const sx of [-0.65, 0.65]) {
        if (hash(k * 7 + lv, sx > 0 ? 3 : 1) < 0.18) continue;
        const h = 0.7 + hash(k, lv * 5 + (sx > 0 ? 2 : 0)) * 0.8, c = GOODS[Math.floor(hash(k + lv, sx * 10) * GOODS.length)];
        it.push(['wood', sx - 0.55, y, -0.45, sx + 0.55, y + 0.14, 0.45, 0x9a7a52], ['board', sx - 0.52, y + 0.14, -0.42, sx + 0.52, y + 0.14 + h, 0.42, c]);
      }
    }
    return it;
  };
  // A forklift of the 1980s: body, counterweight, mast and forks, the overhead guard.
  const forklift = () => [['paint', -0.55, 0.25, -1.1, 0.55, 1.2, 0.9, 0xf2c02a], ['paint', -0.55, 0.25, -1.4, 0.55, 1.5, -1.1, 0x3a3a3a],
    ['metal', -0.5, 0.2, 0.9, -0.4, 2.6, 1.0, 0x3a3a3a], ['metal', 0.4, 0.2, 0.9, 0.5, 2.6, 1.0, 0x3a3a3a], ['metal', -0.35, 0.1, 1.0, -0.2, 0.15, 2.1, 0x3a3a3a], ['metal', 0.2, 0.1, 1.0, 0.35, 0.15, 2.1, 0x3a3a3a],
    ['metal', -0.55, 1.2, -0.7, -0.5, 2.3, -0.65, 0x3a3a3a], ['metal', 0.5, 1.2, -0.7, 0.55, 2.3, -0.65, 0x3a3a3a], ['metal', -0.55, 1.2, 0.55, -0.5, 2.3, 0.6, 0x3a3a3a], ['metal', 0.5, 1.2, 0.55, 0.55, 2.3, 0.6, 0x3a3a3a], ['metal', -0.58, 2.3, -0.72, 0.58, 2.36, 0.62, 0x3a3a3a],
    ['rubber', -0.6, 0, 0.3, -0.45, 0.5, 0.8, 0x1c1c1c], ['rubber', 0.45, 0, 0.3, 0.6, 0.5, 0.8, 0x1c1c1c], ['rubber', -0.6, 0, -1.2, -0.45, 0.45, -0.75, 0x1c1c1c], ['rubber', 0.45, 0, -1.2, 0.6, 0.45, -0.75, 0x1c1c1c]];
  // A loaded pallet on the floor, k picks its load.
  const pallet = (k) => { const h = 0.6 + hash(k, 9) * 1.0, c = GOODS[Math.floor(hash(k, 4) * GOODS.length)]; return [['wood', -0.6, 0, -0.5, 0.6, 0.14, 0.5, 0x9a7a52], ['board', -0.57, 0.14, -0.47, 0.57, 0.14 + h, 0.47, c]]; };
  // A fish market's stand: a table of crates on ice.
  const fishStand = () => { const it = [['metal', -1.5, 0, -0.5, 1.5, 0.85, 0.5, 0xd8dcde], ['stainless', -1.5, 0.85, -0.5, 1.5, 0.9, 0.5, 0xe8eef0]];
    for (const x of [-1.05, 0, 1.05]) it.push(['paint', x - 0.45, 0.9, -0.35, x + 0.45, 1.15, 0.35, 0x2f6f9f], ['paint', x - 0.4, 1.12, -0.3, x + 0.4, 1.16, 0.3, 0xd8e6ea]); return it; };
  // A stack of fish boxes, 3 to 6 high.
  const crates = (k) => { const it = [], h = 3 + Math.floor(hash(k, 3) * 4), c = [0x2f6f9f, 0xc23b30, 0xe8e8e2][Math.floor(hash(k, 6) * 3)];
    for (let i = 0; i < h; i++) it.push(['paint', -0.6, i * 0.32, -0.4, 0.6, i * 0.32 + 0.3, 0.4, c]); return it; };
  // A stack of steel plate on bearers, for a shipyard's shop.
  const plates = (k) => [['wood', -2.5, 0, -1.1, -2.2, 0.2, 1.1, 0x6b4a2e], ['wood', 2.2, 0, -1.1, 2.5, 0.2, 1.1, 0x6b4a2e], ['metal', -3, 0.2, -1.25, 3, 0.2 + 0.3 + hash(k, 2) * 0.5, 1.25, 0x7a6a5a]];
  // A bank of waiting-hall seats, back to back, 6 m long.
  const seatBank = (c) => { const it = [['metal', -3, 0, -0.1, 3, 0.4, 0.1, 0x8d9196]];
    for (let x = -2.7; x < 2.8; x += 0.6) for (const sz of [-1, 1]) it.push(['fabric', x - 0.25, 0.4, sz < 0 ? -0.6 : 0.1, x + 0.25, 0.48, sz < 0 ? -0.1 : 0.6, c], ['fabric', x - 0.25, 0.48, sz < 0 ? -0.18 : 0.1, x + 0.25, 1.05, sz < 0 ? -0.1 : 0.18, c]);
    return it; };
  // A shed's floor fitted out by its use, over the hall's inside x0..x1, z0..z1 (doors to +z) at floor height y.
  function fitOut(B, use, x0, x1, z0, z1, y) {
    if (use === 'fish') {
      // stands in aisles, crate stacks and an ice machine along the back, the auction ring, a forklift by the doors
      for (let z = z0 + 5; z < z1 - 8; z += 3.6) for (let x = x0 + 5; x < x1 - 5; x += 3.6) place(B, x, y, z, 0, fishStand());
      let k = 0;
      for (let x = x0 + 1.5; x < x1 - 1.5; x += 1.3) place(B, x, y, z0 + 1.2, 0, crates(k++));
      for (let z = z0 + 4; z < z1 - 4; z += 1.3) { place(B, x0 + 1.2, y, z, 1, crates(k++)); place(B, x1 - 1.2, y, z, 1, crates(k++)); }
      place(B, x1 - 4, y, z0 + 3.2, 0, [['metal', -1.4, 0, -0.8, 1.4, 2.2, 0.8, 0xd8dcde], ['metal', -1.2, 2.2, -0.6, 1.2, 2.6, 0.6, 0x8d9196]]);
      for (let x = x0 + 8; x < x1 - 8; x += 14) place(B, x, y, z1 - 4, 0, [['wood', -2.5, 0, -0.6, 2.5, 0.4, 0.6, 0x6b4a2e], ['wood', -2.5, 0.4, 0.4, 2.5, 1.0, 0.6, 0x6b4a2e], ['metal', -0.3, 0, -1.6, 0.3, 1.1, -1.0, 0xd8dcde], ['metal', -0.4, 1.1, -1.7, 0.4, 1.15, -0.9, 0x8d9196]]);
      place(B, (x0 + x1) / 2, y, z1 - 7, 2, forklift());
      return;
    }
    if (use === 'shop') {
      for (let z = z0 + 3; z < z1 - 4; z += 6) { place(B, x0 + 1.2, y, z, 1, FURN.workbench()); place(B, x1 - 1.2, y, z, 3, FURN.workbench()); }
      let k = 0; for (let x = x0 + 8; x < x1 - 8; x += 9) for (let z = z0 + 6; z < z1 - 8; z += 7) place(B, x, y, z, 0, plates(k++));
      return;
    }
    if (use === 'boats') {   // a boathouse: a launch on its cradle each bay, a workbench along the back
      for (let x = x0 + 8; x < x1 - 6; x += 14) place(B, x, y, (z0 + z1) / 2, 0, [['wood', -1.5, 0, -4, -1.2, 0.8, 4, 0x6b4a2e], ['wood', 1.2, 0, -4, 1.5, 0.8, 4, 0x6b4a2e], ['paint', -1.6, 0.8, -5, 1.6, 2.6, 5, 0xd8d2c0], ['paint', -1.2, 2.6, -1.5, 1.2, 4.2, 2, 0xe8e8e2], ['glassDark', -1.1, 3.4, 2, 1.1, 4.0, 2.02, 0x2a3a44]]);
      for (let x = x0 + 3; x < x1 - 3; x += 6) place(B, x, y, z0 + 1, 0, FURN.workbench());
      return;
    }
    // warehouse: rows of racking with aisles, a loading area by the doors with pallets and forklifts, a small office
    let k = 0;
    for (let z = z0 + 1.0; z < z1 - 12; z += 4.6) for (let x = x0 + 2; x + 2.7 <= x1 - 2; x += 2.7) place(B, x + 1.35, y, z + 0.6, 0, rackBay(k++));
    for (let x = x0 + 4; x < x1 - 4; x += 3.2) for (const dz of [-6, -8]) if (hash(k++, 5) < 0.45) place(B, x, y, z1 + dz, 0, pallet(k));
    for (let x = x0 + 12; x < x1 - 6; x += 30) place(B, x, y, z1 - 4, hash(x | 0, 1) < 0.5 ? 0 : 2, forklift());
    place(B, x0 + 2.5, y, z1 - 3, 1, FURN.desk());
  }
  // A terminal's two floors fitted out (owner, 2026-10-29: "terminals are empty in airport, more furniitures!").
  // Ground floor (landside to +z): check-in islands with their baggage belts and queue posts, a baggage hall with
  // carousels and trolleys at one end, seating. Upper floor: gate lounges of seat banks, a food court of booths and
  // counters, shops with shelves and coolers, departure boards.
  function terminalFloors(B, x0, x1, z0, z1, g, u) {
    const w = x1 - x0, d = z1 - z0, arr = x0 + Math.min(40, w * 0.25);
    // check-in islands: a row of desks back to back with a belt between, queue posts in front
    for (let x = arr + 8; x < x1 - 10; x += 16) {
      place(B, x, g, z1 - 10, 0, [['trim', -6, 0, -1.2, 6, 1.0, -0.6, 0x2f5f8f], ['trim', -6, 0, 0.6, 6, 1.0, 1.2, 0x2f5f8f], ['rubber', -6, 0, -0.6, 6, 0.6, 0.6, 0x2a2a2a],
        ['counter', -6.05, 1.0, -1.25, 6.05, 1.05, -0.55, 0xd8cdb0], ['counter', -6.05, 1.0, 0.55, 6.05, 1.05, 1.25, 0xd8cdb0], ['metal', -6, 1.05, -0.05, 6, 3.2, 0.05, 0x2f5f8f]]);
      for (const sz of [-1, 1]) for (let px = -5; px <= 5; px += 2.5) place(B, x + px, g, z1 - 10 + sz * 4, 0, [['metal', -0.06, 0, -0.06, 0.06, 1.0, 0.06, 0x8d9196], ['metal', -0.2, 0, -0.2, 0.2, 0.05, 0.2, 0x8d9196], ['fabric', 0.06, 0.85, -0.02, 2.44, 0.9, 0.02, 0xc23b30]]);
    }
    // the baggage hall: carousels, trolleys along the wall
    for (let z = z0 + 8; z < z1 - 8; z += 14) place(B, (x0 + arr) / 2, g, z, 0, [['metal', -9, 0, -2.2, 9, 0.5, 2.2, 0x8d9196], ['rubber', -9.1, 0.5, -2.3, 9.1, 0.6, 2.3, 0x2a2a2a], ['metal', -8, 0.6, -1.0, 8, 0.9, 1.0, 0xb8bcc0],
      ['paint', -6, 0.6, -2.2, -5.2, 1.1, -1.6, 0x2f5f8f], ['paint', -2, 0.6, 1.6, -1.1, 1.0, 2.2, 0xc23b30], ['paint', 3, 0.6, -2.2, 3.9, 1.2, -1.5, 0x3a3a3a]]);
    for (let z = z0 + 3; z < z1 - 3; z += 1.2) place(B, x0 + 1.5, g, z, 0, [['metal', -0.4, 0, -0.3, 0.4, 0.6, 0.3, 0xb8bcc0], ['metal', -0.4, 0.6, 0.25, 0.4, 1.0, 0.3, 0xb8bcc0]]);
    waitingHall(B, arr, x1, z0 + 2, z1 - 16, g);
    // upstairs: gate lounges along the airside, a food court and shops along the landside
    for (let z = z0 + 3; z < z0 + d * 0.5; z += 3.4) for (let x = x0 + 5; x + 3 < x1 - 5; x += 8) place(B, x + 3, u, z, 0, seatBank(z < z0 + 6 ? 0x2f5f8f : 0x3a5a7a));
    for (let x = x0 + 4; x < x1 - 12; x += 30) for (let k = 0; k < 4; k++) place(B, x + 3 + (k % 2) * 4, u, z1 - 13 + Math.floor(k / 2) * 3, 0, FURN.booth(0x8a2c2a));
    for (let x = x0 + 18; x < x1 - 8; x += 30) {
      place(B, x, u, z1 - 4, 2, FURN.counter(0xc23b30)); place(B, x + 4, u, z1 - 4, 2, FURN.cooler());
      for (let k = 0; k < 2; k++) place(B, x + 2, u, z1 - 9 - k * 1.6, 0, FURN.aisle(() => hash(x | 0, k + 3)));
    }
    for (let x = x0 + 10; x < x1 - 10; x += 40) place(B, x, u, (z0 + z1) / 2, 0, [['metal', -0.1, 0, -0.1, 0.1, 2.4, 0.1, 0x3a3a3a], ['metal', -1.5, 2.4, -0.15, 1.5, 3.4, 0.15, 0x1c1c1c], ['screen', -1.4, 2.5, 0.15, 1.4, 3.3, 0.16, 0xf2c02a]]);
  }
  // An office building's floors fitted out by its use (owner, 2026-10-29: "jobs and customs are empty", "port control
  // pilot station too"). Ground floor (door to +z): 'jobs' the jobs terminal (a counter, notice boards, benches),
  // 'customs' an inspection hall (a counter, steel tables, benches), 'pilot' a crew room (radio desk, lockers, a sofa and
  // a kitchen), 'office' desks; every upper floor desks, a meeting table and shelves.
  const board = () => [['paint', -1.2, 0.9, -0.05, 1.2, 2.2, 0.05, 0x8a6a46], ['paint', -1.1, 1.0, 0.05, -0.3, 1.5, 0.06, 0xf2efe8], ['paint', -0.2, 1.1, 0.05, 0.5, 1.9, 0.06, 0xf2efe8], ['paint', 0.6, 1.0, 0.05, 1.1, 1.6, 0.06, 0xe8d070]];
  const bench = (c) => [['wood', -1.2, 0.42, -0.25, 1.2, 0.48, 0.25, c], ['metal', -1.1, 0, -0.2, -1.0, 0.42, 0.2, 0x3a3a3a], ['metal', 1.0, 0, -0.2, 1.1, 0.42, 0.2, 0x3a3a3a]];
  const locker = () => [['metal', -0.3, 0, -0.25, 0.3, 1.9, 0.25, 0x5a6a7a], ['metal', -0.28, 0.9, 0.25, -0.22, 1.1, 0.27, 0xb8bcc0]];
  const steelTable = () => [['stainless', -1, 0.85, -0.5, 1, 0.9, 0.5, 0xe0e4e6], ['metal', -0.95, 0, -0.45, -0.9, 0.85, -0.4, 0x8d9196], ['metal', 0.9, 0, -0.45, 0.95, 0.85, -0.4, 0x8d9196], ['metal', -0.95, 0, 0.4, -0.9, 0.85, 0.45, 0x8d9196], ['metal', 0.9, 0, 0.4, 0.95, 0.85, 0.45, 0x8d9196], ['paint', -0.6, 0.9, -0.3, 0.1, 1.3, 0.2, 0x8a5a36]];
  function officeFit(B, use, x0, x1, z0, z1, floors) {
    const w = x1 - x0, g = floors[0];
    const desks = (y, za, zb) => { for (let x = x0 + 1.6; x < x1 - 1.4; x += 3) for (let z = za; z < zb; z += 2.6) place(B, x, y, z, 0, FURN.desk()); };
    for (let x = x0 + 1.2; x < x1 - 1; x += 2) place(B, x, g, z0 + 0.25, 0, FURN.shelf());   // shelves along the back
    if (use === 'jobs' || use === 'customs') {
      for (let x = x0 + 2; x < x1 - 2; x += 3.1) place(B, x, g, z1 - 4, 0, FURN.counter(use === 'jobs' ? 0x2f5f8f : 0x3a5a3a));
      for (let x = x0 + 2; x < x1 - 2; x += 3) place(B, x, g, z1 - 1.6, 0, bench(0x8a6a46));
      if (use === 'jobs') { for (const [x, r] of [[x0 + 0.12, 1], [x1 - 0.12, 3]]) for (let z = z0 + 2; z < z1 - 5; z += 3) place(B, x, g, z, r, board()); }
      else for (let x = x0 + 2.5; x < x1 - 2; x += 3.5) place(B, x, g, z1 - 6.8, 0, steelTable());
      desks(g, z0 + 2.5, z1 - 7.5);
    } else if (use === 'pilot') {
      place(B, x0 + 2, g, z0 + 1.5, 0, [['trim', -1.5, 0, -0.4, 1.5, 0.95, 0.4, 0x5a5f66], ['metal', -1.3, 0.95, -0.4, 1.3, 1.5, -0.1, 0x2a2a2a], ['glassDark', -1.2, 1.0, -0.1, 1.2, 1.45, -0.09, 0x3aa65a], ['fabric', -0.3, 0, 0.6, 0.3, 0.5, 1.1, 0x2f5f8f]]);
      for (let z = z0 + 1; z < z1 - 1.5; z += 0.7) place(B, x1 - 0.4, g, z, 3, locker());
      place(B, (x0 + x1) / 2, g, (z0 + z1) / 2, 0, FURN.sofa(0x6a4a3a)); place(B, (x0 + x1) / 2, g, (z0 + z1) / 2 + 1.6, 0, FURN.table());
      place(B, x0 + 2.2, g, z1 - 1.2, 2, FURN.kitchen(0xd8d2c0)); place(B, x0 + 5.5, g, z1 - 2, 0, FURN.tv());
    } else desks(g, z0 + 2.5, z1 - 2);
    for (const y of floors.slice(1)) {
      desks(y, z0 + 2.5, z1 - 5);
      for (let x = x0 + 3; x < x1 - 3; x += Math.max(6, w / 2)) place(B, x, y, z1 - 2.5, 0, FURN.table());
      for (let x = x0 + 1.2; x < x1 - 1; x += 2) place(B, x, y, z0 + 0.25, 0, FURN.shelf());
    }
  }
  // A ferry or bus waiting hall: banks of seats, ticket counters, a cafe's booths.
  function waitingHall(B, x0, x1, z0, z1, y) {
    for (let z = z0 + 4; z < z1 - 4; z += 3.4) for (let x = x0 + 5; x + 3 < x1 - 5; x += 8) place(B, x + 3, y, z, 0, seatBank(0x2f5f8f));
  }

  // ---------------------------------------------------------------- the airport
  // A terminal: two storeys, a glazed airside face with gates, an entrance landside under a canopy, check-in counters,
  // seats at the gates. w along x, d deep, airside to -z (the apron), landside to +z.
  function terminal(B, M, o) {
    const w = o.w, d = o.d, x0 = -w / 2, x1 = w / 2, z0 = -d / 2, z1 = d / 2;
    const top = hall(B, { x0, x1, z0, z1, storeys: [[0.15, 7.15], [7.15, 14.15]], wall: 'ribbed', tint: 0xd9d6cc, col: 12,
      front: (i, st) => (st === 0 ? (i % 4 === 1 ? 'entrance' : 'glazed') : 'ribbon'), back: 'glazed', left: (i, st) => (st === 0 && i === 1 ? 'door' : 'ribbon'), right: (i, st) => (st === 0 && i === 1 ? 'door' : 'ribbon'),
      furnish(B) {
        terminalFloors(B, x0, x1, z0, z1, 0.165, 7.165);
      } });
    // the landside canopy on its columns, and the sign
    B.color(0x8d9196); for (let x = x0 + 3; x <= x1 - 3; x += 12) column(B, 'metal', x, z1 + 8, 0.25, 0, 6.5, 10);
    B.color(0xc8ccce); tiles(B, 'metalRoof', x0, 6.5, z1, x1, 6.8, z1 + 9, null, TILE);
    signBand(B, M, o.name.toUpperCase(), 0, top - 2.2, z1 + 0.02, Math.min(30, w * 0.5));
    return top;
  }
  // A pier: a glazed concourse on a single storey raised over the apron on columns, a ramp at its root.
  function concourse(B, M, o) {
    const w = o.w, d = o.d, x0 = -w / 2, x1 = w / 2, z0 = -d / 2, z1 = d / 2;
    return hall(B, { x0, x1, z0, z1, storeys: [[0.15, 5.0], [5.0, 10.0]], wall: 'ribbed', tint: 0xd9d6cc, col: 10,
      front: (i, st) => (st === 1 ? 'glazed' : i % 3 === 0 ? 'roll' : 'blank'), back: (i, st) => (st === 1 ? 'glazed' : i % 3 === 0 ? 'roll' : 'blank'), left: 'glazed', right: 'glazed', doorTint: 0x8d9196, doorH: 3.6,
      furnish(B) {
        for (let z = z0 + 4; z < z1 - 6; z += 3.4) { place(B, x0 + 6, 5.012, z, 1, seatBank(0x2f5f8f)); place(B, x1 - 6, 5.012, z, 1, seatBank(0x2f5f8f)); }
        for (let z = z0 + 10; z < z1 - 10; z += 40) { place(B, 0, 5.012, z, 1, FURN.counter(0x2f5f8f)); place(B, 0, 5.012, z + 6, 0, FURN.cooler()); place(B, 0, 5.012, z + 12, 1, FURN.booth(0x8a2c2a)); }
        for (let z = z0 + 6; z < z1 - 6; z += 12) place(B, 0, 0.165, z, 0, [['paint', -1.2, 0, -3, 1.2, 1.6, 3, 0xf2c02a], ['glassDark', -1.1, 1.6, 1.5, 1.1, 2.4, 2.9, 0x2a3a44], ['paint', -1.2, 2.4, 1.4, 1.2, 2.5, 3, 0xf2c02a]]);
      } });
  }
  // A jet bridge from a rotunda at `face` ([x, y, z], the terminal's face at the bridge's floor) to an aircraft's door.
  function jetBridge(B, face, door) {
    const R = [face[0], 5.6, face[2]];
    B.color(0x2a2d31); column(B, 'metal', face[0], face[2], 0.9, 0, 4.2, 12, 5);
    B.color(0xa9aeb3); B.cyl('metal', 'y', [face[0], 0, face[2]], 2.6, 4.2, 7.4, 16);
    const cab = [door[0], door[1] + 0.2, door[2]], dir = sub(cab, R), L = len(dir), u = norm(dir);
    const pt = (s) => add(R, mul(u, s));
    const cuts = [2.4, L * 0.42, L * 0.72, L - 2.2];
    [[3.4, 3.2], [3.2, 3.0], [3.0, 2.8]].forEach(([w, h], i) => { B.color(0xa9aeb3); member(B, 'ribbed', pt(cuts[i]), pt(cuts[i + 1] + 0.6), w, h); });
    B.color(0x9aa4ad); member(B, 'metal', pt(L - 2.6), pt(L), 4.0, 3.4);
    const foot = pt(L * 0.66);
    B.color(0x2a2d31); B.box('metal', foot[0] - 0.4, 0, foot[2] - 0.4, foot[0] + 0.4, foot[1] - 1.5, foot[2] + 0.4);
    B.box('rubber', foot[0] - 1.6, 0, foot[2] - 0.6, foot[0] + 1.6, 1.1, foot[2] + 0.6);
  }
  // An arched maintenance hangar: block side and back walls, steel arch in plates (each rests on the one below), the
  // doors slid back, a mezzanine of offices inside. w wide (x), d deep (z), the door to +z.
  function archHangar(B, M, o) {
    const w = o.w, d = o.d, x0 = -w / 2, x1 = w / 2, z0 = -d / 2, z1 = d / 2, wallH = o.wallH, rise = o.rise;
    B.color(0xbab4aa); tiles(B, 'concrete', x0, -0.3, z0, x1, 0.12, z1, null, TILE);
    const o2 = { storeys: [[0.12, wallH]], wall: 'block', tint: 0xcfcac0, t: 0.4 };
    face(B, o2, 'z', x0 + 0.2, z0, z1, -1, (i) => (i % 3 === 1 ? 'ribbon' : 'blank'));
    face(B, o2, 'z', x1 - 0.2, z0, z1, 1, (i) => (i % 3 === 1 ? 'ribbon' : 'blank'));
    face(B, o2, 'x', z0 + 0.2, x0 + 0.4, x1 - 0.4, -1, (i, st, n) => (i === Math.floor(n / 2) ? 'door' : 'blank'));
    // the arch: 18 plates around, 8 along, each resting on the one below; the end cap above the back wall in strips
    const seg = 18, along = 8, a = w / 2, cx = 0;
    const at = (ang) => [cx + Math.cos(ang) * a, wallH + Math.sin(ang) * rise];
    for (let i = 0; i < seg; i++) {
      const p = at(Math.PI * i / seg), q = at(Math.PI * (i + 1) / seg);
      const tng = norm([q[0] - p[0], q[1] - p[1], 0]), nrm = norm(cross([0, 0, 1], tng));
      for (let j = 0; j < along; j++) {
        const za = z0 + d * j / along, zb = z0 + d * (j + 1) / along;
        B.color(0x9aa2a8);
        plate(B, 'corrugatedArc', [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2, (za + zb) / 2], [tng, nrm, [0, 0, 1]], [len([q[0] - p[0], q[1] - p[1], 0]) + 0.05, 0.25, zb - za]);
      }
    }
    for (let i = 0; i < 12; i++) { // the back end cap, vertical strips under the arch
      const xa = x0 + w * i / 12, xb = x0 + w * (i + 1) / 12, xm = (xa + xb) / 2;
      const yTop = wallH + rise * Math.sqrt(Math.max(0, 1 - (xm / a) * (xm / a))) - 0.3;
      if (yTop > wallH + 0.3) { B.color(0xcfcac0); run(B, 'ribbed', xa, wallH, z0, xb, yTop, z0 + 0.3, 6); }
    }
    // the doors, slid back to either side of the open front
    B.color(0x9aa4ad);
    for (const s of [-1, 1]) for (let k = 0; k < 3; k++) { const xa = s < 0 ? x0 - 0.5 + k * 6 : x1 - 18 + k * 6; B.box('ribbed', xa, 0, z1 + 0.8 + k * 0.4, xa + 6, wallH + rise * 0.3, z1 + 1.1 + k * 0.4); }
    // offices along the back wall, on a mezzanine
    B.color(0xd8d4cc); for (let x = x0 + 4; x <= x1 - 4; x += 8) B.box('concrete', x - 0.25, 0.12, z0 + 7.8, x + 0.25, 4.0, z0 + 8.3);
    B.color(0xbab4aa); tiles(B, 'concrete', x0 + 0.4, 4.0, z0 + 0.4, x1 - 0.4, 4.3, z0 + 8.3, null, TILE);
    B.color(0xe8e2d0); for (let x = x0 + 2; x < x1 - 4; x += 6) place(B, x, 4.3, z0 + 3, 0, FURN.desk());
    B.color(0xf2c02a); run(B, 'metal', x0 + 0.4, 4.3, z0 + 8.2, x1 - 0.4, 5.3, z0 + 8.3, 6);
    for (let z = z0 + 12; z <= z1 - 8; z += 8) { place(B, x0 + 1.2, 0.12, z, 1, FURN.workbench()); place(B, x1 - 1.2, 0.12, z, 3, FURN.workbench()); }   // along the walls, clear of the aircraft
  }
  // A control tower: a base building, a concrete shaft in lifts, the glazed cab with its catwalk and roof, the mast.
  function tower(B, M, o) {
    const h = o.h;
    hall(B, { x0: -25, x1: 25, z0: -12, z1: 18, storeys: [[0.15, 5.15], [5.15, 10.15]], wall: 'block', tint: 0xe0dccf, col: 10, front: (i, st) => (st === 0 && i === 4 ? 'entrance' : 'windows'), back: 'windows', left: 'windows', right: 'windows',
      furnish(B) {   // ground: the reception and the briefing room; upstairs: offices, the equipment room, the crew room
        place(B, 0, 0.165, 13, 2, FURN.counter(0x2f5f8f));
        for (let x = -20; x <= -6; x += 3.5) for (let z = -8; z <= 4; z += 2.4) place(B, x, 0.165, z, 2, FURN.chair(0x2f5f8f));
        place(B, -13, 0.165, -10.5, 0, FURN.tv());
        for (let x = 10; x <= 20; x += 3.5) for (let z = -8; z <= 10; z += 3) place(B, x, 0.165, z, 0, FURN.desk());
        for (let x = -22; x <= -8; x += 3.2) for (let z = -9; z <= 12; z += 3) place(B, x, 5.162, z, 0, FURN.desk());
        for (let x = 8; x <= 22; x += 1.0) for (const z of [-9, -6]) place(B, x, 5.162, z, 0, [['metal', -0.45, 0, -0.5, 0.45, 2.1, 0.5, 0x3a3d42], ['glassDark', -0.4, 1.6, 0.5, 0.4, 1.9, 0.505, 0x2aff4a]]);
        for (let x = 10; x <= 20; x += 5) place(B, x, 5.162, 10, 0, FURN.booth(0x3a5a7a));
        place(B, 22, 5.162, 3, 3, FURN.kitchen(0xd8d2c0)); place(B, 16, 5.162, 3, 0, FURN.sofa(0x6a4a3a));
      } });
    B.color(0xd8d4cc);
    // a landing and a stair flight inside the shaft each 5 m, the lift car
    for (let y = 10.15; y < h - 2; y += 5) { B.color(0xbab4aa); B.box('concrete', -4.5, y + 4.7, 1.5, 4.5, y + 5, 4.5); place(B, 0, y, -1, 1, [['concrete', -3, 0, -1, 3, 0.3, 1, 0xbab4aa], ['concrete', -1.5, 0.3, -1, 3, 2.4, 1, 0xbab4aa], ['concrete', 0, 2.4, -1, 3, 4.7, 1, 0xbab4aa]]); }
    for (let y = 10.15; y < h - 0.01; y += 5) for (const [a, b] of [[-6, -4.5], [4.5, 6]]) { B.box('concrete', a, y, -6, b, Math.min(h, y + 5), 6); B.box('concrete', -4.5, y, a, 4.5, Math.min(h, y + 5), b); }
    // the cab: a slab, glazed faces all round, the roof
    B.color(0xbab4aa); tiles(B, 'concrete', -10, h - 0.3, -10, 10, h + 0.2, 10, null, 5);
    const cab = { storeys: [[h + 0.2, h + 6.2]], wall: 'metal', tint: 0x5a5f66, t: 0.2, bay: 4 };
    face(B, cab, 'x', 9.9, -10, 10, 1, 'glazed'); face(B, cab, 'x', -9.9, -10, 10, -1, 'glazed');
    face(B, cab, 'z', -9.9, -9.8, 9.8, -1, 'glazed'); face(B, cab, 'z', 9.9, -9.8, 9.8, 1, 'glazed');
    B.color(0x6d6a66); tiles(B, 'roofing', -10.6, h + 6.2, -10.6, 10.6, h + 6.6, 10.6, null, 5);
    // the cab: consoles all round the glass, each with its screens and strips, a chair; the supervisor's desk; radar scopes
    const console_ = [['trim', -1.4, 0, -0.45, 1.4, 0.95, 0.45, 0x5a5f66], ['counter', -1.45, 0.95, -0.5, 1.45, 1.0, 0.5, 0xd8cdb0], ['metal', -1.2, 1.0, -0.45, -0.2, 1.6, -0.25, 0x2a2a2a],
      ['glassDark', -1.1, 1.08, -0.25, -0.3, 1.52, -0.24, 0x3aa65a], ['metal', 0.2, 1.0, -0.45, 1.2, 1.6, -0.25, 0x2a2a2a], ['glassDark', 0.3, 1.08, -0.25, 1.1, 1.52, -0.24, 0x3a8ac8],
      ['paint', -0.9, 1.0, 0.05, 0.9, 1.05, 0.35, 0xf2efe8], ['fabric', -0.3, 0, 0.7, 0.3, 0.5, 1.2, 0x2f5f8f], ['fabric', -0.3, 0.5, 1.12, 0.3, 1.1, 1.2, 0x2f5f8f]];
    for (const [rot, fx, fz] of [[0, 0, 1], [2, 0, -1], [1, 1, 0], [3, -1, 0]]) for (const s of [-5.5, -2.5, 2.5, 5.5]) {
      // rot turns the console's back (-z) to the glass on that side
      const x = fx ? fx * 8.6 : s, z = fz ? fz * 8.6 : s;
      if (Math.abs(x) < 6.5 && Math.abs(z) < 6.5) continue;
      place(B, x, h + 0.2, z, (rot + 2) % 4, console_);
    }
    place(B, 0, h + 0.2, 0, 0, [['wood', -1.5, 0.72, -0.8, 1.5, 0.78, 0.8, 0x5a3a22], ['trim', -1.4, 0, -0.7, 1.4, 0.72, 0.7, 0x5a5f66], ['metal', -0.9, 0.78, -0.6, 0.9, 1.3, -0.4, 0x2a2a2a], ['glassDark', -0.8, 0.85, -0.4, 0.8, 1.25, -0.39, 0x3aa65a]]);
    B.color(0xe8e8e2); B.box('trim', -0.25, h + 6.6, -0.25, 0.25, h + 14.6, 0.25);
    B.color(0xffffff); B.box('lampGreen', -0.4, h + 14.6, 0.0, 0.4, h + 15.4, 1.2); B.box('lamp', -0.4, h + 14.6, -1.2, 0.4, h + 15.4, 0.0);
  }
  // The mech base's own tower: a hollow block shaft, a glazed cab on top (today: a plain box cab, no glass).
  function baseTower(B, M, o) {
    const h = o.h || 30;
    B.color(0xbab4aa); tiles(B, 'concrete', -6, -0.3, -6, 6, 0.15, 6, null, TILE);
    const sh = { storeys: [], wall: 'block', tint: 0xcfcac0, t: 0.4, bay: 6 };
    for (let y = 0.15; y < h - 0.01; y += 5) sh.storeys.push([y, Math.min(h, y + 5)]);
    face(B, sh, 'x', 5.8, -6, 6, 1, (i, st) => (st === 0 && i === 1 ? 'door' : st % 2 ? 'windows' : 'blank'));
    face(B, sh, 'x', -5.8, -6, 6, -1, (i, st) => (st % 2 ? 'windows' : 'blank'));
    face(B, sh, 'z', -5.8, -5.6, 5.6, -1, 'blank'); face(B, sh, 'z', 5.8, -5.6, 5.6, 1, 'blank');
    // a floor every storey with a stair flight in its hole, and a room on each: the entrance and its desk, then offices,
    // the radio and equipment rooms, the crew's rest room (owner, 2026-10-29: "port control pilot station too")
    const ROOMS = [() => [FURN.desk(), FURN.shelf()], () => [FURN.desk(), FURN.desk()], () => [FURN.shelf(), FURN.desk()], () => [FURN.sofa(0x6a4a3a), FURN.table()]];
    for (let y = 0.15, f = 0; y < h - 0.01; y += 5, f++) {
      if (y > 0.2) { B.color(0xbab4aa); tiles(B, 'concrete', -5.6, y - 0.3, -5.6, 2.6, y, 5.6, null, 4); tiles(B, 'concrete', 2.6, y - 0.3, 0, 5.6, y, 5.6, null, 4); }
      const top = Math.min(h, y + 5) - 0.3;
      place(B, 4.1, y, -2.8, 0, Array.from({ length: 10 }, (_, k) => ['concrete', -1.4, 0, -2.75 + k * 0.55, 1.4, (k + 1) * (top - y) / 10, -2.2 + k * 0.55, 0xbab4aa]), 'stair');
      const R = ROOMS[f % ROOMS.length]();
      place(B, -3, y + 0.012, 2.5, 2, R[0]); place(B, -3, y + 0.012, -3, 0, R[1]);
      if (f % 4 === 2) for (let x = -5; x <= -1; x += 1) place(B, x, y + 0.012, 5.1, 2, [['metal', -0.45, 0, -0.4, 0.45, 2.1, 0.4, 0x3a3d42], ['glassDark', -0.4, 1.6, 0.4, 0.4, 1.9, 0.405, 0x2aff4a]]);
    }
    B.color(0xbab4aa); tiles(B, 'concrete', -8, h, -8, 8, h + 0.4, 8, null, 4);
    const cab = { storeys: [[h + 0.4, h + 5.4]], wall: o.today ? 'block' : 'metal', tint: o.today ? 0xcfcac0 : 0x5a5f66, t: 0.2, bay: 4 };
    const g = o.today ? 'windows' : 'glazed';
    face(B, cab, 'x', 6.9, -7, 7, 1, g); face(B, cab, 'x', -6.9, -7, 7, -1, g); face(B, cab, 'z', -6.9, -6.8, 6.8, -1, g); face(B, cab, 'z', 6.9, -6.8, 6.8, 1, g);
    B.color(0x6d6a66); tiles(B, 'roofing', -8.5, h + 5.4, -8.5, 8.5, h + 5.8, 8.5, null, 4);
    // the cab: consoles round the glass, each with screens and a chair, a radar scope in the middle
    const con = [['trim', -1.2, 0, -0.4, 1.2, 0.95, 0.4, 0x5a5f66], ['counter', -1.25, 0.95, -0.45, 1.25, 1.0, 0.45, 0xd8cdb0], ['metal', -1.0, 1.0, -0.4, 1.0, 1.55, -0.2, 0x2a2a2a],
      ['glassDark', -0.9, 1.06, -0.2, 0.9, 1.5, -0.19, 0x3a8ac8], ['fabric', -0.3, 0, 0.6, 0.3, 0.5, 1.1, 0x2f5f8f], ['fabric', -0.3, 0.5, 1.02, 0.3, 1.1, 1.1, 0x2f5f8f]];
    for (const [r, x, z] of [[2, 0, 5.8], [2, -3.5, 5.8], [2, 3.5, 5.8], [0, 0, -5.8], [0, -3.5, -5.8], [1, -5.8, 0], [3, 5.8, 0]]) place(B, x, h + 0.4, z, r, con);
    place(B, 0, h + 0.4, 0, 0, [['trim', -0.8, 0, -0.8, 0.8, 0.9, 0.8, 0x5a5f66], ['glassDark', -0.6, 0.9, -0.6, 0.6, 0.95, 0.6, 0x2aff4a]]);
  }
  // A de-icing spray boom: a mast and its arm reaching over the pad (the arm to -x).
  function sprayBoom(B) {
    B.color(0xf2c02a); B.box('concrete', -2, -0.2, -2, 2, 0.4, 2);
    column(B, 'metal', 0, 0, 1.4, 0.4, 12, 12, 6);
    tubes(B, 'metal', [1, 12.6, 0], [-20, 14, 0], 0.6, 3, 8);   // the arm in 3 m lengths
    for (let x = -18; x <= -4; x += 3.5) { const y = 12.6 + (1 - x) / 21 * 1.4; B.color(0x2a2d31); B.box('metal', x - 0.15, y - 1.0, -0.15, x + 0.15, y - 0.2, 0.15); }   // the nozzles, hung from the arm
  }
  // The surveillance radar's mast (its turning antenna is the page's own spinner).
  function radarMast(B) {
    B.color(0xbab4aa); B.box('concrete', -4, -0.3, -4, 4, 0.5, 4);
    B.color(0x8a8f92); for (const [x, z] of [[-1.6, -1.6], [1.6, -1.6], [1.6, 1.6], [-1.6, 1.6]]) run(B, 'metal', x - 0.15, 0.5, z - 0.15, x + 0.15, 22, z + 0.15, 6);
    for (let y = 4; y < 22; y += 5) { run(B, 'metal', -1.75, y, -1.75, 1.75, y + 0.2, -1.45); run(B, 'metal', -1.75, y, 1.45, 1.75, y + 0.2, 1.75); }
    B.color(0x6b7178); B.box('metal', -2, 22, -2, 2, 23, 2);
  }
  // A fire station: an engine hall with four red doors, the crew's two storeys beside it, the hose tower.
  function fireStation(B, M) {
    hall(B, { x0: -40, x1: 10, z0: -20, z1: 20, storeys: [[0.15, 9.15]], wall: 'brick', tint: 0xffffff, front: (i) => (i < 8 ? 'roll' : 'blank'), bay: 6, doorTint: 0xb8160f, doorH: 6.5, left: 'windows', back: 'windows' });
    hall(B, { x0: 10, x1: 30, z0: -20, z1: 20, storeys: [[0.15, 4.15], [4.15, 8.15]], wall: 'brick', tint: 0xffffff, front: (i, st) => (st === 0 && i === 1 ? 'entrance' : 'windows'), right: 'windows', back: 'windows' });
    B.color(0xffffff); for (let y = 0.15; y < 20; y += 5) { run(B, 'brick', 30, y, -20, 36, y + 5, -19.6); run(B, 'brick', 30, y, -14.4, 36, y + 5, -14); B.box('brick', 35.6, y, -19.6, 36, y + 5, -14.4); }
    signBand(B, M, 'FIRE STATION', -15, 10.2, 20.02, 22, '#b8160f');
  }
  // A cargo shed: a long clad hall, a dock door in every other bay, an office at one end.
  function cargoShed(B, M, o) {
    const w = o.w, d = o.d, h = o.h || 13;
    hall(B, { x0: -w / 2, x1: w / 2, z0: -d / 2, z1: d / 2, storeys: [[0.15, h + 0.15]], wall: 'ribbed', tint: o.tint || 0x9aa4ad, col: 18, front: (i) => (i % 2 ? 'roll' : 'blank'), back: (i) => (i % 3 === 1 ? 'roll' : 'blank'), left: (i) => (i === 1 ? 'entrance' : 'ribbon'), right: 'blank', doorH: o.doorH || 6, doorTint: o.doorTint || 0x5a6a7a, inner: false,
      furnish: (B) => fitOut(B, o.use || 'store', -w / 2 + 0.3, w / 2 - 0.3, -d / 2 + 0.3, d / 2 - 0.3, 0.162) });
    if (o.name) signBand(B, M, o.name, -w / 2 + 16, h - 2, d / 2 + 0.02, 24, o.sign || '#2c5d9c');
  }
  // A row of T-hangars: a low steel shed, a door in each bay.
  function tHangars(B, M, o) {
    hall(B, { x0: -o.w / 2, x1: o.w / 2, z0: -12, z1: 12, storeys: [[0.12, 5.12]], wall: 'ribbed', tint: 0xb8bcbe, col: 12, bay: 12, front: 'roll', back: 'blank', doorH: 4, doorTint: 0xd8d8d0, roof: { parapet: 0.3 }, inner: false });
  }
  // The multi-storey car park: four open decks on a 10 m column grid, edge parapets, a ramp, cars.
  function carPark(B, M, o, rnd) {
    const w = o.w, d = o.d, x0 = -w / 2, x1 = w / 2, z0 = -d / 2, z1 = d / 2, lv = 3.2;
    B.color(0xbab4aa); tiles(B, 'concrete', x0, -0.3, z0, x1, 0.1, z1, null, TILE);
    for (let k = 1; k <= 3; k++) {
      const y = 0.1 + k * lv;
      for (let x = x0; x <= x1 + 1e-6; x += 10) for (let z = z0; z <= z1 + 1e-6; z += 10) { B.color(0xd8d4cc); B.box('concrete', Math.max(x0, x - 0.3), y - lv, Math.max(z0, z - 0.3), Math.min(x1, x + 0.3), y - 0.3, Math.min(z1, z + 0.3)); }
      B.color(0xc8c2b6); tiles(B, 'concrete', x0, y - 0.3, z0, x1, y, z1, null, TILE);
      B.color(0xd8d4cc); run(B, 'concrete', x0, y, z0, x1, y + 1.0, z0 + 0.25); run(B, 'concrete', x0, y, z1 - 0.25, x1, y + 1.0, z1);
      run(B, 'concrete', x0, y, z0 + 0.25, x0 + 0.25, y + 1.0, z1 - 0.25); run(B, 'concrete', x1 - 0.25, y, z0 + 0.25, x1, y + 1.0, z1 - 0.25);
    }
    const CARS = [0x8a2c2a, 0xd8d2c0, 0x2c4a7a, 0x6a6a5a, 0xa87a3a, 0x3a5a3a, 0x1a1a1a];
    for (let k = 0; k <= 3; k++) for (let i = 0; i < 18; i++) if (rnd() < 0.55) carAt(B, x0 + 6 + (i % 9) * (w - 12) / 8, k === 0 ? 0.1 : 0.1 + k * lv, i < 9 ? z0 + 6 : z1 - 6, i < 9 ? 0 : 2, CARS[Math.floor(rnd() * CARS.length)]);
  }
  // A parked car standing on a deck at y: the kit's sedan, its wheels as boxes so the whole car is one piece at any height.
  function carAt(B, x, y, z, rot, tint) {
    const L = 2.35;
    place(B, x, y, z, rot, [['paint', -0.88, 0.32, -L, 0.88, 0.92, L, tint], ['glassDark', -0.8, 0.92, -1.15, 0.8, 1.38, 0.95, 0x2a3a44], ['paint', -0.78, 1.38, -1.05, 0.78, 1.42, 0.85, tint],
      ['metal', -0.9, 0.3, L, 0.9, 0.45, L + 0.08, 0xb8bcc0], ['metal', -0.9, 0.3, -L - 0.08, 0.9, 0.45, -L, 0xb8bcc0], ['lamp', -0.75, 0.62, L, -0.45, 0.75, L + 0.01, 0xfff4d0], ['lamp', 0.45, 0.62, L, 0.75, 0.75, L + 0.01, 0xfff4d0],
      ['rubber', -0.98, 0, L - 1.0, -0.76, 0.66, L - 0.3, 0x1c1c1c], ['rubber', 0.76, 0, L - 1.0, 0.98, 0.66, L - 0.3, 0x1c1c1c], ['rubber', -0.98, 0, -L + 0.3, -0.76, 0.66, -L + 1.0, 0x1c1c1c], ['rubber', 0.76, 0, -L + 0.3, 0.98, 0.66, -L + 1.0, 0x1c1c1c]], 'car');
  }
  // A fuel or oil tank: staves around (each grounded), the roof in segments resting on them, a bund wall handled by the page.
  function tank(B, o) {
    const r = o.r, h = o.h, seg = o.seg || 16;
    B.color(0xbab4aa); B.cyl('concrete', 'y', [0, 0, 0], r + 0.6, -0.3, 0.3, seg);
    for (let i = 0; i < seg; i++) {
      const a = 2 * Math.PI * i / seg, b = 2 * Math.PI * (i + 1) / seg, m = (a + b) / 2;
      const c = [Math.cos(m) * r, 0.3 + h / 2, Math.sin(m) * r], tng = [-Math.sin(m), 0, Math.cos(m)], out = [Math.cos(m), 0, Math.sin(m)];
      const wch = 2 * r * Math.sin(Math.PI / seg) + 0.05;
      const lifts = Math.max(1, Math.ceil(h / 6));
      for (let k = 0; k < lifts; k++) { B.color(o.tint || 0xdcdcd2); plate(B, 'metal', [c[0], 0.3 + h * (k + 0.5) / lifts, c[2]], [tng, [0, 1, 0], out], [wch, h / lifts, 0.3]); }
      const top = 0.3 + h, rise = r * 0.12;
      // the roof: one closed wedge a stave, rim to crown, so the segments meet with no gaps
      const ra = [Math.cos(a) * (r + 0.2), top, Math.sin(a) * (r + 0.2)], rb = [Math.cos(b) * (r + 0.2), top, Math.sin(b) * (r + 0.2)], apex = [0, top + rise, 0];
      const lift = (q, d) => [q[0], q[1] + d, q[2]];
      B.color(o.roofTint || 0xc8c8be);
      B.group(() => { const n = norm(cross(sub(rb, ra), sub(apex, ra))); const nu = n[1] < 0 ? mul(n, -1) : n;
        B.poly('metal', [ra, rb, apex], nu); B.poly('metal', [lift(ra, -0.2), lift(rb, -0.2), lift(apex, -0.2)], mul(nu, -1));
        B.poly('metal', [ra, rb, lift(rb, -0.2), lift(ra, -0.2)], [Math.cos(m), 0, Math.sin(m)]); });
    }
  }
  // A mech hangar of the game's mech base (cb_core::world::bases): three block walls and a slab roof, open to the front.
  function mechHangar(B, M, o) {
    const w = o.w, d = o.d, h = o.h, t = 1.5, x0 = -w / 2, x1 = w / 2, z0 = -d / 2, z1 = d / 2;
    B.color(0xbab4aa); tiles(B, 'concrete', x0, -0.3, z0, x1, 0.12, z1, null, TILE);
    const oo = { storeys: [[0.12, h]], wall: 'block', tint: 0xcfcac0, t };
    face(B, oo, 'z', x0 + t / 2, z0, z1, -1, 'blank'); face(B, oo, 'z', x1 - t / 2, z0, z1, 1, 'blank');
    face(B, oo, 'x', z0 + t / 2, x0 + t, x1 - t, -1, (i) => (i === 3 ? 'door' : 'blank'));
    for (let x = x0 + 11; x < x1 - 1; x += 11) { B.color(0x8d9196); B.box('metal', x - 0.5, h - 2.5, z0 + t, x + 0.5, h, z1); }
    B.color(0x80858a); tiles(B, 'metalRoof', x0, h, z0, x1, h + 0.6, z1, null, TILE);
    B.color(0xf2c02a); B.box('paint', x0 + t, 0.12, z1 - 1, x1 - t, 0.13, z1 - 0.5);
  }
  // A small airfield building: one storey, windows round, a door at the front.
  function hut(B, M, o) {
    const h = o.h || 4.15;   // over 6 m it is two storeys
    hall(B, { x0: -o.w / 2, x1: o.w / 2, z0: -o.d / 2, z1: o.d / 2, storeys: h > 6 ? [[0.15, (h + 0.15) / 2], [(h + 0.15) / 2, h]] : [[0.15, h]], wall: o.wall || 'block', tint: o.tint || 0xe0dccf, col: 8,
      front: (i, st, n) => (i === Math.floor(n / 2) ? 'door' : 'windows'), back: 'windows', left: 'windows', right: 'windows',
      furnish: (B) => officeFit(B, o.use || 'office', -o.w / 2 + 0.3, o.w / 2 - 0.3, -o.d / 2 + 0.3, o.d / 2 - 0.3, h > 6 ? [0.162, (h + 0.15) / 2 + 0.012] : [0.162]) });
    if (o.name) signBand(B, M, o.name, 0, (o.h || 4.15) - 0.6, o.d / 2 + 0.02, Math.min(o.w - 2, 10));
  }

  // ---------------------------------------------------------------- the seaport
  // Painted steel (the gantries, the straddles, the goliath) is 'paint': the 'metal' surface reads as bare steel.
  // A harbour's luffing crane (the harbours change's model, assets/models/boats/crane.glb, at its own sizes): a 12 m portal
  // on four legs, the slewing house, the tower and a 35 m lattice jib out over -z, every member short round tubes, so a
  // shot takes out a length of steel and what hangs beyond it falls. A page builds it with a machine's longer reach
  // (info.maxSide): the jib hangs from the tower by its own steel.
  function luffingCrane(B, o = {}) {
    const portal = o.portal || 12, H = o.height || 40, jib = o.jib || 35, p = portal / 2 - 0.6;
    B.color(0xeab81f);
    for (const sx of [-1, 1]) for (const sz of [-1, 1]) column(B, 'paint', sx * p, sz * p, 0.6, 0, 10, 10, 2.5);
    for (const sx of [-1, 1]) for (const sz of [-1, 1]) { B.color(0x2a2d31); B.box('metal', sx * p - 0.9, 0, sz * p - 0.9, sx * p + 0.9, 0.5, sz * p + 0.9); }
    B.color(0xeab81f);
    run(B, 'paint', -portal / 2, 10, -portal / 2, portal / 2, 11.2, -portal / 2 + 1.5, 3); run(B, 'paint', -portal / 2, 10, portal / 2 - 1.5, portal / 2, 11.2, portal / 2, 3);
    run(B, 'paint', -portal / 2, 10, -portal / 2 + 1.5, -portal / 2 + 1.5, 11.2, portal / 2 - 1.5, 3); run(B, 'paint', portal / 2 - 1.5, 10, -portal / 2 + 1.5, portal / 2, 11.2, portal / 2 - 1.5, 3);
    run(B, 'paint', -portal / 2 + 1.5, 10, -1, portal / 2 - 1.5, 11.2, 1, 3); run(B, 'paint', -1, 10, -portal / 2 + 1.5, 1, 11.2, -1, 3); run(B, 'paint', -1, 10, 1, 1, 11.2, portal / 2 - 1.5, 3);   // the cross girders under the ring
    for (const sx of [-1, 1]) for (const sz of [-1, 1]) tubes(B, 'paint', [sx * p, 1, sz * p], [0, 10, sz * p * 0.2], 0.15, 3, 6);
    // the slewing house on its ring, the cab's window to the jib
    B.color(0x2a2d31); B.cyl('metal', 'y', [0, 0, 0], 3.2, 11.2, 11.6, 16);
    B.color(0xeab81f); run(B, 'paint', -3, 11.6, -3, 3, 16.8, 5, 3);
    B.color(0xffffff); B.box('glassDark', -2, 14, -3.05, 2, 15.2, -3.0);
    B.color(0x1c1c1c); B.box('paint', -1.5, 16.8, 2.5, 1.5, 19.3, 5.5);   // the counterweight
    B.color(0xeab81f); column(B, 'paint', 0, 0, 1.2, 16.8, H, 10, 2.5);
    // the jib: two chords and a top chord, braced every 2.5 m
    const a = 28 * Math.PI / 180, root = [0, H - 3, -0.9], tip = [0, H - 3 + Math.sin(a) * jib, -0.9 - Math.cos(a) * jib];
    const at = (t, x, y) => [x * (1 - t * 0.7), root[1] + (tip[1] - root[1]) * t + y, root[2] + (tip[2] - root[2]) * t];
    const n = Math.round(jib / 2.5);
    for (const sx of [-1, 1]) tubes(B, 'paint', at(0, sx * 0.9, 0), at(1, sx * 0.9, 0), 0.22, 2.5, 6);
    tubes(B, 'paint', at(0, 0, 1.4), at(1, 0, 0.5), 0.18, 2.5, 6);
    for (let i = 0; i < n; i++) { const t0 = i / n, t1 = (i + 1) / n;
      B.color(0xeab81f); tube(B, 'paint', at(t0, -0.9, 0), at(t0, 0.9, 0), 0.08, 5);
      tube(B, 'paint', at(t0, (i % 2 ? -0.9 : 0.9), 0), at(t1, 0, 1.4 - 0.9 * t1), 0.08, 5); }
    // the tie from the tower head to the jib tip, the hook
    B.color(0x8d9196); tubes(B, 'metal', [0, H - 0.2, 0], [tip[0], tip[1] + 0.4, tip[2]], 0.12, 3, 5);
    B.color(0x2a2d31); B.box('metal', -0.5, tip[1] - 0.4, tip[2] - 0.3, 0.5, tip[1], tip[2] + 0.3);   // the sheave head between the chords
    tube(B, 'metal', [0, tip[1], tip[2]], [0, tip[1] - 10, tip[2]], 0.05, 4);   // the falls, one piece hung from the sheave head
    B.box('metal', -0.4, tip[1] - 10.8, tip[2] - 0.4, 0.4, tip[1] - 9.7, tip[2] + 0.4);
  }
  // A ship-to-shore container gantry of the 1980s: legs on rails `gauge` apart (z), its girder 42 m up running out over
  // the water as the boom, the machinery house, the A-frame and its stays, the trolley and cab. Rigid: a page builds it
  // as a machine (BuildingEngine.building({ rigid: true })).
  function gantry(B, o) {
    const g = o.gauge || 30, out = o.out || 55, back = o.back || 25, H = 40;
    B.color(0xf06a12);
    // every member in lengths of 4 or 5 m, so a hit takes out a length of steel and what hangs beyond it falls
    for (const z of [0, g]) for (const s of [-9, 9]) { post(B, 'paint', s - 1, z - 1, s + 1, z + 1, 1.5, H, 4); B.box('paint', s - 2, 0, z - 2.5, s + 2, 1.5, z + 2.5); }
    for (const z of [0, g]) run(B, 'paint', -10, H, z - 1.5, 10, H + 3, z + 1.5, 4);
    for (const s of [-9, 9]) run(B, 'paint', s - 1, 10, 1, s + 1, 12, g - 1, 4);
    for (const s of [-4, 4]) run(B, 'paint', s - 1.1, H + 3, -back, s + 1.1, H + 6, g + out, 5);
    B.color(0xe8e8e2); B.box('ribbed', -6, H + 6, -back + 1, 6, H + 13, -back + 17);
    B.color(0xf06a12);
    for (const s of [-4, 4]) { tubes(B, 'paint', [s, H + 5.8, 0], [0, H + 30, g / 2], 0.6, 3, 8); tubes(B, 'paint', [s, H + 5.8, g], [0, H + 30, g / 2], 0.6, 3, 8); }
    B.color(0x6b7178); for (const s of [-4, 4]) tubes(B, 'metal', [0, H + 30, g / 2], [s, H + 5.8, g + out - 5], 0.2, 3, 6);
    B.color(0xe8e8e2); B.box('ribbed', -3, H - 2, g + 18, 3, H + 4.5, g + 24); B.color(0xffffff); B.box('glass', -2.6, H - 1.5, g + 24, 2.6, H + 1.5, g + 24.05);
  }
  // A straddle carrier: four legs and a frame over one row of boxes, its cab on top. Rigid.
  function straddle(B) {
    B.color(0xf2c02a);
    for (const sx of [-1.9, 1.9]) for (const sz of [-4.5, 4.5]) { post(B, 'paint', sx - 0.3, sz - 0.3, sx + 0.3, sz + 0.3, 0, 13, 3); B.color(0x1c1c1c); B.cyl('rubber', 'x', [sx, 0.6, sz], 0.6, sx - 0.35, sx + 0.35, 10); B.color(0xf2c02a); }
    run(B, 'paint', -2.2, 13, -5, -1.4, 14.2, 5, 3); run(B, 'paint', 1.4, 13, -5, 2.2, 14.2, 5, 3);
    B.box('paint', -1.4, 13, -5, 1.4, 14.2, -4); B.box('paint', -1.4, 13, 4, 1.4, 14.2, 5);
    B.color(0xe8e8e2); B.box('paint', -1.2, 14.2, -4.5, 1.2, 16.4, -2); B.color(0xffffff); B.box('glass', -1.1, 15.0, -2.0, 1.1, 16.2, -1.95);
  }
  // A stack block: cols x rows stacks of 40 ft boxes, 1-4 high, liners' colours; each box a piece. A page builds the
  // block as one building; each box rests on the one under it.
  const LINERS = [0x2f5f8f, 0xa8322a, 0x3f6e3a, 0x8a8a84, 0xc0682a, 0x6b4a2e, 0x2a6a72, 0xb59a3a];
  function stackBlock(B, o, rnd) {
    for (let i = 0; i < o.cols; i++) for (let j = 0; j < o.rows; j++) {
      const h = 1 + Math.floor(rnd() * 4), x = i * 12.6, z = j * 2.9;
      for (let k = 0; k < h; k++) { B.color(LINERS[Math.floor(rnd() * LINERS.length)]); B.box('corrugated', x, k * 2.6, z, x + 12.2, (k + 1) * 2.6 - 0.02, z + 2.6); }
    }
  }
  // A goliath crane over a dry dock: legs either side of `span`, the girder and the crab riding on top of it. Rigid.
  function goliath(B, o) {
    const s = o.span / 2, H = o.h || 52;
    B.color(0xf2c02a);
    for (const x of [-s, s]) { for (const z of [-8, 8]) post(B, 'paint', x - 1, z - 1, x + 1, z + 1, 1.5, H, 4); run(B, 'paint', x - 1.5, H, -10, x + 1.5, H + 3, 10, 4); run(B, 'paint', x - 2, 0, -10, x + 2, 1.5, 10, 4); }
    run(B, 'paint', -s - 4, H + 3, -4, s + 4, H + 8, 4, 5);
    B.color(0xe8e8e2); B.box('ribbed', -6, H + 8, -4, 6, H + 13, 4);
  }
  // A light tower on a breakwater head: white tower in lifts, a coloured band, the lantern and its gallery.
  function lightTower(B, o) {
    B.color(0xe8e8e2); column(B, 'stucco', 0, 0, 2.5, 0, 12, 14, 4);
    B.color(o.red ? 0xb8160f : 0x1e7a3a); B.cyl('paint', 'y', [0, 0, 0], 2.62, 3.0, 6.0, 14);
    B.color(0xd8d2c6); B.cyl('concrete', 'y', [0, 0, 0], 3.4, 12, 12.4, 14);
    B.color(0xffffff); B.cyl(o.red ? 'lampRed' : 'lampGreen', 'y', [0, 0, 0], 1.4, 12.4, 14.4, 12);
    B.color(0x2a2d31); B.cyl('metal', 'y', [0, 0, 0], 1.7, 14.4, 15.0, 12);
  }
  // A lateral buoy: a float, its cage and topmark, the light. Red is a can, green a cone (IALA A).
  function buoy(B, o) {
    const c = o.red ? 0xb8160f : 0x1e7a3a;
    B.color(c); B.cyl('paint', 'y', [0, 0, 0], 1.5, -1.0, 1.2, 12);
    B.color(c); for (const [x, z] of [[-0.8, -0.8], [0.8, -0.8], [0.8, 0.8], [-0.8, 0.8]]) B.box('metal', x - 0.08, 1.2, z - 0.08, x + 0.08, 3.4, z + 0.08);
    B.color(c); B.box('metal', -0.9, 3.3, -0.9, 0.9, 3.4, 0.9); if (o.red) B.box('paint', -0.5, 3.4, -0.5, 0.5, 4.4, 0.5); else B.cyl('paint', 'y', [0, 0, 0], 0.55, 3.4, 4.2, 8);
    B.color(0xffffff); const top = o.red ? 4.4 : 4.2; B.box(o.red ? 'lampRed' : 'lampGreen', -0.2, top, -0.2, 0.2, top + 0.4, 0.2);
  }
  // A deck on piles (a pier, a jetty's trestle or head): pile caps on piles every `pitch` metres, the deck in panels on
  // top, a kerb along both edges. x0..x1 by z0..z1, deck top at `top`, piles down to `bed`.
  function pileDeck(B, o) {
    const { x0, x1, z0, z1, top, bed } = o, pitch = o.pitch || 10;
    const nx = Math.max(1, Math.round((x1 - x0) / pitch)), nz = Math.max(1, Math.round((z1 - z0) / pitch));
    for (let i = 0; i <= nx; i++) for (let k = 0; k <= nz; k++) {
      const x = Math.min(x1 - 0.6, Math.max(x0 + 0.6, x0 + (x1 - x0) * i / nx)), z = Math.min(z1 - 0.6, Math.max(z0 + 0.6, z0 + (z1 - z0) * k / nz));
      B.color(0x5a5850); column(B, 'concrete', x, z, 0.5, bed, top - 1.2, 10, 8);
      B.color(0xbab4aa); B.box('concrete', x - 0.9, top - 1.2, z - 0.9, x + 0.9, top - 0.6, z + 0.9);
    }
    B.color(0xbab4aa); tiles(B, 'concrete', x0, top - 0.6, z0, x1, top, z1, null, o.tile || TILE);
    B.color(0xd8d2c6);
    if (o.kerbs !== false) { if (x1 - x0 > z1 - z0) { run(B, 'concrete', x0, top, z0, x1, top + 0.3, z0 + 0.3); run(B, 'concrete', x0, top, z1 - 0.3, x1, top + 0.3, z1); } else { run(B, 'concrete', x0, top, z0, x0 + 0.3, top + 0.3, z1); run(B, 'concrete', x1 - 0.3, top, z0, x1, top + 0.3, z1); } }
  }
  // Fenders and bollards along a quay's sea face from p to q (axis-aligned), `out` the sea side unit [x, z], at deck y.
  function quayEdge(B, p, q, out, y) {
    const ax = Math.abs(q[0] - p[0]) > Math.abs(q[1] - p[1]) ? 0 : 1, L = Math.abs(q[ax] - p[ax]), dir = Math.sign(q[ax] - p[ax]);
    for (let s = 6, k = 0; s < L - 3; s += 12, k++) {
      const c = [p[0], p[1]]; c[ax] += dir * s;
      const lo = [c[0], c[1]], hi = [c[0], c[1]];
      lo[ax] -= 1.2; hi[ax] += 1.2;
      const o = 1 - ax; // the across-face coordinate: the fender stands from the face out 0.8 m
      lo[o] = Math.min(c[o], c[o] + out[o] * 0.8); hi[o] = Math.max(c[o], c[o] + out[o] * 0.8);
      B.color(0x1c1c1c); B.box('rubber', lo[0], -1.5, lo[1], hi[0], y - 0.3, hi[1]);
      if (k % 2 === 0) { B.color(0x2d2f33); B.cyl('metal', 'y', [c[0] - out[0] * 1.2, 0, c[1] - out[1] * 1.2], 0.35, y, y + 0.8, 10); }
    }
  }
  // A floating pontoon with finger berths, its deck just over the water at y = 0.45.
  function pontoon(B, o) {
    B.color(0xa7a49b);
    run(B, 'concrete', -1.5, -0.6, 0, 1.5, 0.45, o.len, 6);
    for (let z = 10; z < o.len; z += 14) for (const s of [-1, 1]) run(B, 'concrete', s < 0 ? -9 : 1.5, -0.6, z, s < 0 ? -1.5 : 9, 0.45, z + 1.4, 4);
  }
  // A tug of the 1980s: a block-out stand-in (no model yet), 28 x 9 m, bow to +z; a page builds it as a rigid machine.
  function tug(B) {
    B.color(0x2a2d31); run(B, 'metal', -4.5, -3.5, -14, 4.5, 2.2, 14, 7);
    B.color(0x7a0c08); run(B, 'paint', -4.6, 2.2, -14, 4.6, 2.6, 14, 7);
    B.color(0x1c1c1c); for (let z = -12; z <= 12; z += 4) for (const s of [-1, 1]) B.box('rubber', s * 4.5 - 0.3, 0.6, z - 0.8, s * 4.5 + 0.3, 2.2, z + 0.8);
    B.color(0xe8e8e2); B.box('trim', -3.5, 2.6, -2, 3.5, 7.5, 6); B.color(0xffffff); B.box('glassDark', -3.55, 6, -1.9, 3.55, 7, 5.9);
    B.color(0xc04a08); B.cyl('paint', 'y', [0, 0, -4], 0.9, 2.6, 9.6, 12);
  }

  root.SiteBuildings = { BAY, TILE, BAYS, plate, member, column, post, run, face, hall, signBand, seats,
    terminal, concourse, jetBridge, fitOut, waitingHall, archHangar, tower, baseTower, sprayBoom, radarMast, fireStation, cargoShed, tHangars, carPark, tank, mechHangar, hut,
    carAt, tube, tubes, luffingCrane, gantry, straddle, stackBlock, goliath, lightTower, buoy, pileDeck, quayEdge, pontoon, tug, LINERS };
  if (typeof module !== 'undefined' && module.exports) module.exports = root.SiteBuildings;
})(typeof window !== 'undefined' ? window : globalThis);
