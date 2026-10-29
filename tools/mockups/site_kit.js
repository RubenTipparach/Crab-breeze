// The site kit for Crab Breeze's 3D site mockups (the airport, the seaport): the shapes they are built from and the
// breakable buildings, one implementation inlined into each page by its generator (gen_airport.py, gen_seaport.py).
// The page loads SmallBuildings (the small-buildings kit) first and defines T (three.js), PI, `built` (the group rebuilt
// on Repair) and statEl (where a shot's result is written) before using any of it. Metres; x east, y up, z south.
function box(g, lo, hi, m, shadow = true) {
  if (BLD) return addBox(g, lo, hi, m, shadow);
  const mesh = new T.Mesh(new T.BoxGeometry(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]), m);
  mesh.position.set((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2);
  mesh.castShadow = shadow; mesh.receiveShadow = true; g.add(mesh); return mesh;
}
const rect = (x0, z0, x1, z1) => ({ min: [Math.min(x0, x1), Math.min(z0, z1)], max: [Math.max(x0, x1), Math.max(z0, z1)] });
const centre = r => [(r.min[0] + r.max[0]) / 2, (r.min[1] + r.max[1]) / 2];
const slab = (g, r, y0, y1, m) => box(g, [r.min[0], y0, r.min[1]], [r.max[0], y1, r.max[1]], m, false);
// A box between two 3D points, w wide and h high, its length along the line (a beam, a jet bridge tunnel).
function beam(g, P, Q, w, h, m, shadow = true) {
  const p = new T.Vector3(...P), q = new T.Vector3(...Q), len = p.distanceTo(q);
  const mesh = new T.Mesh(new T.BoxGeometry(w, h, len), m);
  mesh.position.copy(p).add(q).multiplyScalar(0.5); mesh.lookAt(q);
  mesh.castShadow = shadow; mesh.receiveShadow = true; g.add(mesh); if (BLD) addPiece(mesh); return mesh;
}
const stroke = (g, p, q, w, y, m) => strokeAt(g, p, q, w, y, m);
function strokeAt(g, p, q, w, y, m) {
  const dx = q[0] - p[0], dz = q[1] - p[1], len = Math.hypot(dx, dz) || 1;
  const mesh = new T.Mesh(new T.PlaneGeometry(len + w, w), m);
  mesh.rotation.x = -PI / 2; mesh.rotation.z = -Math.atan2(dz, dx);
  mesh.position.set((p[0] + q[0]) / 2, y, (p[1] + q[1]) / 2); mesh.receiveShadow = true; g.add(mesh);
}
function ring(g, c, r, w, y, m, from = 0, to = 2 * PI) {
  const mesh = new T.Mesh(new T.RingGeometry(r - w / 2, r + w / 2, 48, 1, from, to - from), m);
  mesh.rotation.x = -PI / 2; mesh.position.set(c[0], y, c[1]); g.add(mesh);
}
function outlineRect(g, r, w, y, m) {
  const P = [[r.min[0], r.min[1]], [r.max[0], r.min[1]], [r.max[0], r.max[1]], [r.min[0], r.max[1]]];
  for (let k = 0; k < 4; k++) stroke(g, P[k], P[(k + 1) % 4], w, y, m);
}
const cyl = (g, c, r, y0, h, m, seg = 24) => { const t = new T.Mesh(new T.CylinderGeometry(r, r, h, seg), m); t.position.set(c[0], y0 + h / 2, c[1]); t.castShadow = t.receiveShadow = true; g.add(t); if (BLD) addPiece(t); return t; };

// ---------------------------------------------------------------- destructible buildings
// Every building breaks the small-buildings way (owner, 2026-10-29: "use our breakable framework, all buildings are
// destructible"). A building's boxes split into pieces of at most CHUNK metres, drawn as one instanced mesh per material;
// its other shapes (a tank, a tunnel, a hangar's arch segment) are a piece each. The small-buildings kit decides what
// stands (SmallBuildings.supportLinks and standing, the rule the game's buildings use): a piece stands while it reaches
// the ground through pieces under it, or through at most MAX_SIDE pieces beside it. Click a building to fire at it.
const K = window.SmallBuildings, CHUNK = [10, 6, 10], UNIT = new T.BoxGeometry(1, 1, 1), ZERO = new T.Matrix4().makeScale(0, 0, 0);
let BLD = null, BUILDINGS = [], targets = [], fallers = [], bits = [];
// groundY is the height its ground floor stands on (0 at the airport, a quay's deck at the seaport).
function building(name, fn, groundY = 0) {
  const prev = BLD; BLD = { name, pieces: [], groups: new Map(), g: null, groundY }; fn(); finishBuilding(BLD); BLD = prev;
}
function addBox(g, lo, hi, m, shadow) {
  BLD.g = BLD.g || g;
  const n = [0, 1, 2].map(a => Math.max(1, Math.ceil((hi[a] - lo[a]) / CHUNK[a] - 1e-6)));
  const key = m.uuid + (shadow ? "s" : "");
  if (!BLD.groups.has(key)) BLD.groups.set(key, { m, shadow, list: [] });
  const grp = BLD.groups.get(key), at = (a, i) => lo[a] + (hi[a] - lo[a]) * i / n[a];
  for (let i = 0; i < n[0]; i++) for (let j = 0; j < n[1]; j++) for (let k = 0; k < n[2]; k++) {
    const pl = [at(0, i), at(1, j), at(2, k)], ph = [at(0, i + 1), at(1, j + 1), at(2, k + 1)];
    grp.list.push(BLD.pieces.length); BLD.pieces.push({ lo: pl, hi: ph, grp, m, gy: BLD.groundY });
  }
  return null;
}
function addPiece(o) {
  BLD.g = BLD.g || o.parent;
  o.updateMatrixWorld(true);
  const b = new T.Box3().setFromObject(o);
  BLD.pieces.push({ lo: b.min.toArray(), hi: b.max.toArray(), obj: o, gy: BLD.groundY });
  o.userData.piece = [BLD, BLD.pieces.length - 1]; targets.push(o);
}
function finishBuilding(B) {
  if (!B.pieces.length) return;
  const m4 = new T.Matrix4();
  for (const grp of B.groups.values()) {
    const im = new T.InstancedMesh(UNIT, grp.m, grp.list.length);
    grp.list.forEach((pi, i) => { const P = B.pieces[pi]; P.im = im; P.idx = i;
      m4.makeScale(P.hi[0] - P.lo[0], P.hi[1] - P.lo[1], P.hi[2] - P.lo[2]).setPosition((P.lo[0] + P.hi[0]) / 2, (P.lo[1] + P.hi[1]) / 2, (P.lo[2] + P.hi[2]) / 2);
      im.setMatrixAt(i, m4); });
    im.castShadow = grp.shadow; im.receiveShadow = true; im.userData.bld = [B, grp.list];
    B.g.add(im); targets.push(im);
  }
  // the kit's rule reads "on the ground" as starting within 5 cm of 0; a door sill at 12 cm stands on the ground here
  // (heights are taken from the building's own ground, groundY)
  const gy = B.groundY;
  B.sup = B.pieces.map(P => ({ lo: [P.lo[0], P.lo[1] - gy <= 0.25 ? 0 : P.lo[1] - gy, P.lo[2]], hi: [P.hi[0], P.hi[1] - gy, P.hi[2]] }));
  B.links = K.supportLinks(B.sup); B.alive = B.pieces.map(() => true);
  // a piece the rule would not hold as built (a bridge's tunnel hung off its rotunda) is anchored and reported
  // (the links stay those of its real shape; only the grounding test, B.ground, counts it as on the ground)
  const st0 = K.standing(B.sup, B.links, B.alive); B.anchored = 0;
  B.ground = B.sup.map((q, i) => st0.has(i) ? q : (B.anchored++, { lo: [q.lo[0], 0, q.lo[2]], hi: q.hi }));
  if (B.anchored) console.log(`${B.name}: ${B.anchored} of ${B.pieces.length} pieces anchored`);
  B.lo = [0, 1, 2].map(a => Math.min(...B.pieces.map(P => P.lo[a]))); B.hi = [0, 1, 2].map(a => Math.max(...B.pieces.map(P => P.hi[a])));
  BUILDINGS.push(B);
}
function hidePiece(P) {
  if (P.obj) { P.obj.visible = false; return; }
  P.im.setMatrixAt(P.idx, ZERO); P.im.instanceMatrix.needsUpdate = true;
}
// A piece that has lost its support drops as one body, tips a little and lies where it lands.
function fall(P) {
  let o = P.obj;
  if (!o) { hidePiece(P); o = new T.Mesh(UNIT, P.m); o.scale.set(P.hi[0] - P.lo[0], P.hi[1] - P.lo[1], P.hi[2] - P.lo[2]);
    o.position.set((P.lo[0] + P.hi[0]) / 2, (P.lo[1] + P.hi[1]) / 2, (P.lo[2] + P.hi[2]) / 2); o.castShadow = o.receiveShadow = true; built.add(o); }
  const r = () => (Math.random() - 0.5) * 0.6;
  fallers.push({ o, v: 0, drop: Math.max(0, P.lo[1] - P.gy), done: 0, spin: [r(), r(), r()] });
}
function blast(p, radius) {
  let hit = 0, fell = 0;
  const flash = new T.Mesh(new T.SphereGeometry(1, 16, 12), new T.MeshBasicMaterial({ color: 0xffb040, transparent: true, opacity: 0.9 }));
  flash.position.copy(p); built.add(flash); bits.push({ o: flash, flash: radius, life: 0.5 });
  for (const B of BUILDINGS) {
    if (p.x < B.lo[0] - radius || p.x > B.hi[0] + radius || p.y < B.lo[1] - radius || p.y > B.hi[1] + radius || p.z < B.lo[2] - radius || p.z > B.hi[2] + radius) continue;
    let any = false;
    B.pieces.forEach((P, i) => {
      if (!B.alive[i]) return;
      const dx = Math.max(P.lo[0] - p.x, 0, p.x - P.hi[0]), dy = Math.max(P.lo[1] - p.y, 0, p.y - P.hi[1]), dz = Math.max(P.lo[2] - p.z, 0, p.z - P.hi[2]);
      if (dx * dx + dy * dy + dz * dz > radius * radius) return;
      B.alive[i] = false; hidePiece(P); any = true; hit++;
      for (let k = 0; k < 3 && bits.length < 400; k++) debris(P, p);
    });
    if (!any) continue;
    const st = K.standing(B.ground, B.links, B.alive);
    B.pieces.forEach((P, i) => { if (B.alive[i] && !st.has(i)) { B.alive[i] = false; fall(P); fell++; } });
  }
  statEl.textContent = `last shot: ${hit} pieces destroyed, ${fell} fell`;
  return { hit, fell };
}
function debris(P, p) {
  const s = 0.5 + Math.random() * 1.2, o = new T.Mesh(UNIT, P.m || P.obj.material);
  o.scale.set(s, s * 0.7, s); o.castShadow = true;
  o.position.set(P.lo[0] + Math.random() * (P.hi[0] - P.lo[0]), P.lo[1] + Math.random() * (P.hi[1] - P.lo[1]), P.lo[2] + Math.random() * (P.hi[2] - P.lo[2]));
  const v = o.position.clone().sub(p).normalize().multiplyScalar(8 + Math.random() * 14); v.y += 6 + Math.random() * 8;
  built.add(o); bits.push({ o, v, life: 6 });
}
function stepBreakage(dt) {
  for (let i = fallers.length - 1; i >= 0; i--) { const f = fallers[i];
    f.v += 9.81 * dt; const d = Math.min(f.v * dt, f.drop - f.done); f.done += d; f.o.position.y -= d;
    f.o.rotation.x += f.spin[0] * dt; f.o.rotation.z += f.spin[2] * dt;
    if (f.done >= f.drop - 1e-3) fallers.splice(i, 1); }
  for (let i = bits.length - 1; i >= 0; i--) { const b = bits[i]; b.life -= dt;
    if (b.flash) { b.o.scale.setScalar(b.flash * (1.2 - b.life)); b.o.material.opacity = Math.max(0, b.life * 1.8); }
    else if (b.o.position.y > 0.3) { b.v.y -= 9.81 * dt; b.o.position.addScaledVector(b.v, dt); b.o.rotation.x += dt * 3; }
    if (b.life <= 0) { built.remove(b.o); bits.splice(i, 1); } }
}
// A click that does not drag fires at the building under it, with the blast radius radius() gives.
function bindFire(canvas, camera, radius) {
  let downAt = null;
  canvas.addEventListener("pointerdown", e => { downAt = [e.clientX, e.clientY]; });
  canvas.addEventListener("pointerup", e => {
    if (!downAt || Math.hypot(e.clientX - downAt[0], e.clientY - downAt[1]) > 5) return;
    const r = canvas.getBoundingClientRect(), rc = new T.Raycaster();
    rc.setFromCamera(new T.Vector2((e.clientX - r.left) / r.width * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), camera);
    const h = rc.intersectObjects(targets, true).find(i => { for (let o = i.object; o; o = o.parent) if (!o.visible) return false; return true; });
    if (h) blast(h.point, radius());
  });
}
