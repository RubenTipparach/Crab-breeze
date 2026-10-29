// Crab Breeze: Mining Mike's mech pose layer (godot-sandbox docs/mockups/tank_bosses_kit.js at
// 3336b88: Pose, bendLegs, idleGeneric, walkGeneric, turnStep, skatePose, hitGeneric) on the rig
// contract's bone names (docs/design/mech-rig.md), plus Crab Breeze's sitPose for boarding. One
// copy, inlined by every mockup that poses a mech (tools/mockups/gen_mech_designs.py,
// tools/mockups/gen_cockpit_page.py), and the reference for how cb-app drives the same bones.
const clamp = (x, a, b) => (x < a ? a : x > b ? b : x), sat = x => clamp(x, 0, 1);
const hash = i => { const s = Math.sin(i * 127.1 + 311.7) * 43758.5453; return s - Math.floor(s); };
const lerp = (a, b, u) => a + (b - a) * u, ease = u => { u = sat(u); return u * u * (3 - 2 * u); };
const scan = (t, period, amp, seed) => { const i = Math.floor(t / period), f = t / period - i; return lerp((hash(i + seed) - 0.5) * 2 * amp, (hash(i + 1 + seed) - 0.5) * 2 * amp, ease(f * 2.6)); };

// ---------- Mining Mike's pose layer, on the contract's bone names ----------
// "head" is the sensor head on its neck, a child of the torso: the cockpit is part of the chest now and moves only with
// it (owner, 2026-10-24: "head moves when mech leans ... that subtle torso sway ... breaks the immersion that the head is
// connected"; Mining Mike's kit had the cockpit for a head)
const MAP = { body: "pelvis", waist: "torso_twist", chest: "torso", head: "head", shL: "shoulder_L", shR: "shoulder_R", elbL: "forearm_L", elbR: "forearm_R",
  hipL: "thigh_L", hipR: "thigh_R", kneeL: "shin_L", kneeR: "shin_R", tarL: "hock_L", tarR: "hock_R", ankL: "foot_L", ankR: "foot_R" };
const MAPPED = new Set(Object.values(MAP));
const SEG = ["hip", "knee", "tar"];
class Pose { constructor() { this.m = new Map(); this.w = 1; }
  add(n, rx, ry, rz, px, py, pz) { if (!this.w) return; n = MAP[n] || n; let a = this.m.get(n); if (!a) { a = [0, 0, 0, 0, 0, 0]; this.m.set(n, a); }
    const w = this.w; a[0] += (rx || 0) * w; a[1] += (ry || 0) * w; a[2] += (rz || 0) * w; a[3] += (px || 0) * w; a[4] += (py || 0) * w; a[5] += (pz || 0) * w; } }
function bendLegs(P, b, kk) {
  if (!b.legSegs || !kk) return;
  const segs = b.legSegs, mult = [0.5, 1, 0.8];
  for (const S of ["L", "R"]) { let sum = 0; segs.forEach((sg, i) => { const d = kk * Math.sign(sg[1]) * mult[i] * 0.9; P.add(SEG[i] + S, d); sum += d; }); P.add("ank" + S, -sum); }
  let a = 0, h = b.footH, a2 = 0, h2 = b.footH;
  segs.forEach((sg, i) => { a += sg[1]; h += sg[0] * Math.cos(a); a2 += sg[1] + kk * Math.sign(sg[1]) * mult[i] * 0.9; h2 += sg[0] * Math.cos(a2); });
  P.add("body", 0, 0, 0, 0, h2 - h, 0);
}
function idleGeneric(P, t, b) {
  const o = b.idleP || {}, br = Math.sin(t * (o.breathHz || 1.5)), s = b.scale;
  P.add("chest", 0.016 * br, 0, 0, 0, 0.28 * br * s, 0);
  const ws = Math.sin(t * 0.42);
  P.add("body", 0, 0.03 * Math.sin(t * 0.31), 0.012 * ws, 0.5 * ws * s, 0.12 * Math.sin(t * 0.84) * s, 0);
  P.add("waist", 0, 0.05 * Math.sin(t * 0.37 + 1), 0);
  P.add("head", 0.05 * Math.sin(t * 0.7) + 0.6 * scan(t + 9, 2.3, 0.08, 3), scan(t, o.scanPeriod || 1.9, o.scan || 0.6, b.seed) * 0.35, 0);
  P.add("shL", 0.03 * br, 0, 0.02 * br); P.add("shR", 0.03 * br, 0, -0.02 * br);
  P.add("elbL", -0.04 * br); P.add("elbR", -0.04 * br);
  bendLegs(P, b, 0.06 + 0.04 * Math.sin(t * 0.84));
}
function walkGeneric(P, t, b) {
  const o = b.walk, ph = b.ph, K = o.liftK || [0.45, 1, 0.8];
  for (const s of [1, -1]) {
    const S = s > 0 ? "L" : "R", p = ph + (s > 0 ? 0 : Math.PI), sw = Math.sin(p), lift = Math.max(0, Math.cos(p));
    let sum = 0;
    b.legSegs.forEach((sg, i) => { const d = (i === 0 ? -o.A * sw : 0) + lift * o.lift * Math.sign(sg[1]) * K[i]; P.add(SEG[i] + S, d); sum += d; });
    P.add("ank" + S, -sum * 0.92 + 0.14 * sw);
    P.add("sh" + S, o.arm * Math.sin(p), 0, 0); P.add("elb" + S, -o.arm * 0.6 * (1 + Math.sin(p)) * 0.5);
  }
  const sw = Math.sin(ph);
  P.add("body", 0, o.yaw * sw, o.roll * sw, 0, -o.bob * Math.abs(sw), 0);
  P.add("waist", o.lean, -o.yaw * 0.8 * sw, -o.roll * 0.6 * sw);
  P.add("head", -o.lean * 0.6, -o.yaw * 0.4 * sw, 0);
  bendLegs(P, b, 0.12);
}
function turnStep(P, b) {
  const o = b.walk, ph = b.ph, dir = 1, K = o.liftK || [0.45, 1, 0.8];
  for (const s of [1, -1]) {
    const S = s > 0 ? "L" : "R", p = ph + (s > 0 ? 0 : Math.PI), lift = Math.max(0, Math.cos(p)), sw = Math.sin(p);
    let sum = 0;
    b.legSegs.forEach((sg, i) => { const d = (i === 0 ? -o.A * 0.25 * sw : 0) + lift * o.lift * 0.85 * Math.sign(sg[1]) * K[i]; P.add(SEG[i] + S, d, i === 0 ? dir * 0.12 * lift : 0, i === 0 ? s * 0.1 * lift : 0); sum += d; });
    P.add("ank" + S, -sum * 0.92);
  }
  P.add("body", 0, -dir * 0.1, 0, 0, -o.bob * 0.5 * Math.abs(Math.sin(ph)), 0);
  P.add("waist", 0, dir * 0.06, 0);
  bendLegs(P, b, 0.1);
}
function skatePose(P, b) {
  bendLegs(P, b, 0.6);
  P.add("hipL", 0, 0, 0.17); P.add("hipR", 0, 0, -0.17);
  P.add("ankL", 0, 0, -0.17); P.add("ankR", 0, 0, 0.17);
  P.add("waist", 0.16, 0, 0); P.add("chest", 0.1, 0, 0); P.add("head", -0.12, 0, 0);
  P.add("shL", -0.35, 0, 0.12); P.add("shR", -0.35, 0, -0.12);
}
function hitGeneric(P, t, b) {
  const j = sat(t / 0.05) * Math.exp(-Math.max(0, t - 0.05) * 5) * (1 + 0.35 * Math.sin(t * 30) * Math.exp(-t * 6));
  P.add("chest", -0.26 * j, 0, 0); P.add("waist", 0, 0.2 * j * b.hitSide, 0.05 * j * b.hitSide);
  P.add("head", -0.25 * j, 0.15 * j, 0); P.add("body", 0, 0, 0, 0, 0, -2.6 * j * b.scale);
  P.add("shL", -0.25 * j, 0, 0.18 * j); P.add("shR", -0.25 * j, 0, -0.18 * j);
  bendLegs(P, b, 0.3 * j);
}

// Crab Breeze: sitting down to be boarded (owner, 2026-10-24: "have the mech sit down on the
// ground so that pilots can enter/exit"). k runs 0 (standing) to 1 (sat): the legs fold to a deep
// crouch with the pelvis carried down by bendLegs, the torso bows forward, the arms come in.
// Where a mech's arms would sink into its folding thighs, the kneel alone moves them clear (owner, 2026-10-29: "ONLY
// MODDIFYING THE KNEELING POSE"): radians added on top of the kneel's own arm pose and scaled by the kneel, so standing,
// walking and the models are untouched. out swings the shoulders outward, fwd forward, elb bends the elbows (negative
// bends further). Measured clear of the legs at 75% and 100% kneel: Foreman 0.73 m, Harrow 0.23, Roustabout 0.37, Mule 0.30.
const KNEEL_ARMS = {
  foreman: { elb: -0.3 },
  harrow: { elb: -0.6 },
  roustabout: { out: 0.6, fwd: -0.25 },
  mule: { out: 0.6, fwd: -0.5 },
};
function sitPose(P, b, k) {
  if (!k) return;
  const e = k * k * (3 - 2 * k);
  bendLegs(P, b, 1.25 * e);
  P.add("waist", 0.28 * e, 0, 0); P.add("chest", 0.12 * e, 0, 0);
  P.add("shL", 0.5 * e, 0, 0.1 * e); P.add("shR", 0.5 * e, 0, -0.1 * e);
  P.add("elbL", -0.6 * e); P.add("elbR", -0.6 * e);
  const h = KNEEL_ARMS[b.id]; if (!h) return;
  P.add("shL", (h.fwd || 0) * e, 0, (h.out || 0) * e); P.add("shR", (h.fwd || 0) * e, 0, -(h.out || 0) * e);
  P.add("elbL", (h.elb || 0) * e); P.add("elbR", (h.elb || 0) * e);
}

function applyPose(b, P) {
  for (const n in b.rest) {
    const o = b.nodes[n], r = b.rest[n], a = P.m.get(n);
    if (a) { o.rotation.set(r.e.x + a[0], r.e.y + a[1], r.e.z + a[2]); o.position.set(r.p.x + a[3], r.p.y + a[4], r.p.z + a[5]); }
    else { o.rotation.copy(r.e); o.position.copy(r.p); }
  }
}
