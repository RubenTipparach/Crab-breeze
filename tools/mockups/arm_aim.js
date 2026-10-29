// Crab Breeze: the arms aim their guns at the point the pilot aims at (openspec change `arm-aim`; owner, 2026-10-29:
// "Shot is ok to deviate from muzzle but we want muzzle to aim as close as possible to shooting point"). One copy,
// inlined by the melee lab (tools/mockups/gen_melee_lab.py), and the reference for cb-core's solver.
//
// Each arm has three aiming joints over its pose: the shoulder's pitch and yaw (rx, ry) and the elbow's pitch
// (forearm rx), the bones posed as everywhere else (tools/mockups/mech_pose.js; shoulder, never upper_arm, which
// carries no mesh). The barrel is the muzzle socket's +z axis (on every gun arm of the eight detailed mechs it lies
// along the barrel). An arm whose muzzle does not point within 60 degrees of the mech's forward at rest is a launcher
// (the Sapper's left forearm pod fires up), not aimed: its missiles steer themselves. The solve finds the joint offsets that turn
// that line onto the line from the muzzle to the aim point: damped least squares on the barrel's error, a few
// iterations from last frame's answer, each joint held inside its limits and pulled lightly toward rest so a
// reachable aim is met with the least bend. The arm then moves toward that answer no faster than its slew rate, so
// a fast aim change is followed, not snapped to. Whatever the joints cannot reach is left as error: the shot still
// flies from the muzzle to the aim point, and the error is how far it leaves the barrel's line.

// Per arm, by joint: [lo, hi] radians and slew rad/s. The right arm's yaw limits; the left arm's are mirrored.
const AIM_DEFAULTS = {
  shoulder_pitch: { lo: -1.6, hi: 0.7, slew: 1.6 },   // rx: below zero raises the arm forward and up
  shoulder_yaw: { lo: -0.45, hi: 0.6, slew: 1.6 },    // ry on the right arm: above zero swings it in across the chest
  elbow_pitch: { lo: -1.1, hi: 0.9, slew: 2.4 },      // forearm rx: below zero lifts the gun
  rest_pull: 0.02,                                     // how strongly an unneeded bend is pulled back to rest
  iterations: 4,
};

// The aim state for a mech: per arm, the joint offsets now [shoulder rx, shoulder ry, forearm rx].
function aimState() { return { L: [0, 0, 0], R: [0, 0, 0], err: { L: null, R: null }, want: { L: [0, 0, 0], R: [0, 0, 0] }, gun: {} }; }

const _v = () => new THREE.Vector3();
// The barrel of arm S on mech b now: {from: the muzzle, dir: its +z axis}, or null with no muzzle.
const _q = new THREE.Quaternion();
function aimBarrel(b, S) {
  const m = b.nodes["MUZZLE_" + S];
  if (!m) return null;
  m.getWorldQuaternion(_q);
  return { from: m.getWorldPosition(_v()), dir: new THREE.Vector3(0, 0, 1).applyQuaternion(_q) };
}
// Whether arm S is a gun to aim: a muzzle within 60 degrees of the mech's forward (its root's +z) as it stands now.
function aimIsGun(b, S) {
  const br = aimBarrel(b, S); if (!br) return false;
  b.root.getWorldQuaternion(_q);
  return br.dir.dot(new THREE.Vector3(0, 0, 1).applyQuaternion(_q)) > Math.cos(Math.PI / 3);
}

// Sets the arm's three joints to rest + base + q (base: the pose beneath, as applyPose left it) and updates the arm.
function aimSet(b, S, base, q) {
  const sh = b.nodes["shoulder_" + S], fa = b.nodes["forearm_" + S];
  sh.rotation.x = base[0] + q[0]; sh.rotation.y = base[1] + q[1]; fa.rotation.x = base[2] + q[2];
  sh.updateMatrixWorld(true);
}

// The barrel's error toward `target` for joint offsets q: the barrel direction minus the wanted one (3 numbers).
function aimResidual(b, S, base, q, target) {
  aimSet(b, S, base, q);
  const br = aimBarrel(b, S);
  const want = target.clone().sub(br.from).normalize();
  return [br.dir.x - want.x, br.dir.y - want.y, br.dir.z - want.z];
}

function aimLimits(S, cfg) {
  const y = cfg.shoulder_yaw, m = S === "L";
  return [[cfg.shoulder_pitch.lo, cfg.shoulder_pitch.hi], m ? [-y.hi, -y.lo] : [y.lo, y.hi], [cfg.elbow_pitch.lo, cfg.elbow_pitch.hi]];
}

// Solves one arm toward `target` (world point), starting from q0; returns the joint offsets it wants.
function aimSolve(b, S, base, q0, target, cfg) {
  const lim = aimLimits(S, cfg), q = q0.slice(), eps = 1e-3, lam = 0.02;
  for (let it = 0; it < cfg.iterations; it++) {
    const r = aimResidual(b, S, base, q, target);
    const J = [0, 1, 2].map(j => { const p = q.slice(); p[j] += eps; const rp = aimResidual(b, S, base, p, target); return rp.map((x, i) => (x - r[i]) / eps); });
    // (J^T J + lam I + rest W) dq = -(J^T r + rest W q): 3 x 3, solved directly
    const A = [[0, 0, 0], [0, 0, 0], [0, 0, 0]], g = [0, 0, 0];
    for (let a = 0; a < 3; a++) {
      for (let c = 0; c < 3; c++) for (let i = 0; i < 3; i++) A[a][c] += J[a][i] * J[c][i];
      for (let i = 0; i < 3; i++) g[a] -= J[a][i] * r[i];
      A[a][a] += lam + cfg.rest_pull; g[a] -= cfg.rest_pull * q[a];
    }
    const dq = solve3(A, g);
    for (let j = 0; j < 3; j++) q[j] = Math.min(lim[j][1], Math.max(lim[j][0], q[j] + dq[j]));
  }
  return q;
}
function solve3(A, g) {
  const det = m => m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
  const D = det(A); if (Math.abs(D) < 1e-12) return [0, 0, 0];
  return [0, 1, 2].map(k => det(A.map((row, i) => row.map((v, j) => (j === k ? g[i] : v)))) / D);
}

// One frame of arm aim on mech b, after applyPose has set the pose beneath: each arm with weight w[S] (0 leaves the
// arm alone, as while it strikes) solves toward `target`, slews toward the answer, and is left posed with it.
// Returns the per-arm angle in degrees between the barrel and the shot's line (muzzle to target).
function aimStep(b, st, target, dt, w, cfg) {
  cfg = cfg || AIM_DEFAULTS;
  const slew = [cfg.shoulder_pitch.slew, cfg.shoulder_yaw.slew, cfg.elbow_pitch.slew];
  for (const S of ["L", "R"]) {
    const sh = b.nodes["shoulder_" + S], fa = b.nodes["forearm_" + S];
    if (st.gun[S] === undefined) st.gun[S] = !!(sh && fa) && aimIsGun(b, S);
    if (!st.gun[S]) { st.err[S] = null; continue; }
    const base = [sh.rotation.x, sh.rotation.y, fa.rotation.x];
    const want = target && w[S] > 0 ? aimSolve(b, S, base, st[S], target, cfg) : [0, 0, 0];
    st.want[S] = want;
    for (let j = 0; j < 3; j++) { const d = want[j] - st[S][j], m = slew[j] * dt; st[S][j] += Math.max(-m, Math.min(m, d)); }
    const q = st[S].map(x => x * (w[S] ?? 1));
    aimSet(b, S, base, q);
    if (target) { const br = aimBarrel(b, S); st.err[S] = THREE.MathUtils.radToDeg(br.dir.angleTo(target.clone().sub(br.from))); }
    else st.err[S] = null;
  }
  return st.err;
}
