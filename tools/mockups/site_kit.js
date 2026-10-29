// The site kit for Crab Breeze's 3D site mockups (the airport, the seaport): the shapes the ground is laid out from
// (paving, paint, kerbs, plain boxes for small dressing), one implementation inlined into each page by its generator
// (gen_airport.py, gen_seaport.py). The page defines T (three.js) and PI first. The buildings are not drawn here: they
// are the small-buildings kit's (docs/mockups/site_buildings_kit.js) and break by docs/mockups/building_engine.js.
// Metres; x east, y up, z south.
function box(g, lo, hi, m, shadow = true) {
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
  mesh.castShadow = shadow; mesh.receiveShadow = true; g.add(mesh); return mesh;
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
const cyl = (g, c, r, y0, h, m, seg = 24) => { const t = new T.Mesh(new T.CylinderGeometry(r, r, h, seg), m); t.position.set(c[0], y0 + h / 2, c[1]); t.castShadow = t.receiveShadow = true; g.add(t); return t; };
