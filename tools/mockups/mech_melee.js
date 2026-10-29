// Crab Breeze: melee moves and gun recoil as keyframed poses on the rig contract's bones (openspec change
// `close-combat`; owner, 2026-10-29: "I will need your help desigining melee animaitons in the mech artifact", "shooting
// guns should cause mech arm to recoil"). One copy, inlined by the mech design page (tools/mockups/gen_mech_designs.py),
// and the reference for how cb-app plays the same moves.
//
// A move is a list of keys: each key a time in seconds and, per bone, an offset over the pose beneath it as
// [rx, ry, rz] in radians (the bone's own axes) and optionally [px, py, pz] in metres. Between keys the offsets ease
// (smoothstep); before the first key and after the last the move adds nothing, so a move starts and ends at rest.
// `hit_s` is when the blow lands (the target is struck then); `arm` is the arm that strikes or fires ("L", "R" or "both").
// A key may also carry `crouch`, how far the legs fold (bendLegs in mech_pose.js: 0 standing, 1.25 sat), eased the same way.
//
// The axes, read off the rigs (every contract bone's x axis lies along the mech's x, its left; +y up, +z forward):
//   rx < 0 swings a bone's far end forward and up (shoulder: the arm reaches out ahead; forearm: the gun lifts),
//   rx > 0 swings it back and down; torso rx > 0 hunches forward.
//   ry > 0 turns forward toward the mech's left (torso_twist: the right shoulder comes forward; shoulder_R: a hook
//   swings across the chest). Mirrored for the left side.
//   rz on a shoulder lifts the arm out to the side: rz < 0 on the right, rz > 0 on the left.
// The upper arm's mesh hangs on the shoulder bone, and upper_arm_* carries none: an arm is posed by its shoulder and its
// forearm, never upper_arm_* (which would swing the forearm and gun away from the upper arm).
const MELEE_BONES = ["pelvis", "torso_twist", "torso", "shoulder_L", "upper_arm_L", "forearm_L", "hand_L",
  "shoulder_R", "upper_arm_R", "forearm_R", "hand_R", "thigh_L", "shin_L", "hock_L", "thigh_R", "shin_R", "hock_R"];

const MELEE_MOVES = {};

// A right-handed move's left-handed twin: bone names swapped, and the turns and slides across the body (ry, rz, px)
// negated. One move per motion is written; its mirror is made, never typed.
function meleeMirror(move) {
  const sw = n => n.endsWith("_R") ? n.slice(0, -2) + "_L" : n.endsWith("_L") ? n.slice(0, -2) + "_R" : n;
  const keys = move.keys.map(k => {
    const b = {};
    for (const n in k.b || {}) { const v = k.b[n]; b[sw(n)] = [v[0] || 0, -(v[1] || 0), -(v[2] || 0), -(v[3] || 0), v[4] || 0, v[5] || 0]; }
    return Object.assign({}, k, { b });
  });
  return Object.assign({}, move, { keys, arm: move.arm === "R" ? "L" : move.arm === "L" ? "R" : move.arm,
    name: move.name.replace(/right/g, "left"), side: -(move.side || 0) });
}

// The default moves. Wind-up 0.35 s, hit 0.15 s, recovery 0.4 s for the jab (the close-combat change's timing, which
// cb-core's MeleeConfig carries); the heavier blows wind up longer. `side` is the way the struck mech is knocked (+1
// toward its right). Times in seconds; rotations in radians; slides in metres.
const MELEE_DEFAULTS = {
  jab_R: { name: "Jab, right", arm: "R", hit_s: 0.45, side: 0, keys: [
    { t: 0 },
    { t: 0.35, b: { shoulder_R: [0.45, -0.1, 0], forearm_R: [-0.45, 0, 0], torso_twist: [0, -0.22, 0], torso: [0.05, 0, 0] } },
    { t: 0.45, b: { shoulder_R: [-1.35, 0.12, 0], forearm_R: [1.15, 0, 0], torso_twist: [0, 0.3, 0], torso: [0.14, 0, 0], pelvis: [0, 0, 0, 0, 0, 1.2] }, crouch: 0.15 },
    { t: 0.55, b: { shoulder_R: [-1.3, 0.12, 0], forearm_R: [1.1, 0, 0], torso_twist: [0, 0.28, 0], torso: [0.12, 0, 0], pelvis: [0, 0, 0, 0, 0, 1.2] }, crouch: 0.15 },
    { t: 0.95 } ] },
  hook_R: { name: "Hook, right", arm: "R", hit_s: 0.55, side: 1, keys: [
    { t: 0 },
    { t: 0.42, b: { shoulder_R: [-0.35, -0.45, -0.75], forearm_R: [-0.7, 0, 0], torso_twist: [0, -0.45, 0] }, crouch: 0.2 },
    { t: 0.55, b: { shoulder_R: [-1.25, 0.85, -0.25], forearm_R: [0.3, 0, 0], torso_twist: [0, 0.5, 0], torso: [0.1, 0, 0.06], pelvis: [0, 0.08, 0, 0, 0, 0.8] }, crouch: 0.3 },
    { t: 0.68, b: { shoulder_R: [-1.1, 1.0, -0.15], forearm_R: [0.25, 0, 0], torso_twist: [0, 0.58, 0], torso: [0.1, 0, 0.06], pelvis: [0, 0.1, 0, 0, 0, 0.8] }, crouch: 0.25 },
    { t: 1.15 } ] },
  uppercut_R: { name: "Uppercut, right", arm: "R", hit_s: 0.55, side: 0, keys: [
    { t: 0 },
    { t: 0.42, b: { shoulder_R: [0.35, 0.1, 0], forearm_R: [-1.0, 0, 0], torso_twist: [0, -0.3, 0], torso: [0.25, 0, 0] }, crouch: 0.6 },
    { t: 0.55, b: { shoulder_R: [-1.75, 0.2, 0], forearm_R: [-0.55, 0, 0], torso_twist: [0, 0.3, 0], torso: [-0.15, 0, 0], pelvis: [0, 0, 0, 0, 0, 0.6] }, crouch: 0 },
    { t: 0.7, b: { shoulder_R: [-1.85, 0.22, 0], forearm_R: [-0.6, 0, 0], torso_twist: [0, 0.3, 0], torso: [-0.18, 0, 0], pelvis: [0, 0, 0, 0, 0, 0.6] } },
    { t: 1.2 } ] },
  smash: { name: "Overhead smash, both", arm: "both", hit_s: 0.85, side: 0, keys: [
    { t: 0 },
    { t: 0.65, b: { shoulder_L: [-2.5, 0, 0.15], shoulder_R: [-2.5, 0, -0.15], forearm_L: [-0.5, 0, 0], forearm_R: [-0.5, 0, 0], torso: [-0.22, 0, 0], pelvis: [0, 0, 0, 0, 0, -0.6] } },
    { t: 0.85, b: { shoulder_L: [-0.75, -0.15, 0], shoulder_R: [-0.75, 0.15, 0], forearm_L: [0.75, 0, 0], forearm_R: [0.75, 0, 0], torso: [0.42, 0, 0], pelvis: [0, 0, 0, 0, 0, 1.4] }, crouch: 0.55 },
    { t: 1.15, b: { shoulder_L: [-0.7, -0.15, 0], shoulder_R: [-0.7, 0.15, 0], forearm_L: [0.7, 0, 0], forearm_R: [0.7, 0, 0], torso: [0.38, 0, 0], pelvis: [0, 0, 0, 0, 0, 1.4] }, crouch: 0.5 },
    { t: 1.8 } ] },
  charge: { name: "Shoulder charge", arm: "both", hit_s: 0.7, side: -1, keys: [
    { t: 0 },
    { t: 0.4, b: { torso_twist: [0, -0.75, 0], torso: [0.3, 0, 0], shoulder_L: [-0.3, 0, 0.1], shoulder_R: [0.2, 0, -0.1], forearm_L: [-0.9, 0, 0], forearm_R: [-0.9, 0, 0], pelvis: [0, -0.2, 0, 0, 0, -0.8] }, crouch: 0.5 },
    { t: 0.7, b: { torso_twist: [0, -0.85, 0], torso: [0.38, 0, 0], shoulder_L: [-0.35, 0, 0.1], shoulder_R: [0.2, 0, -0.1], forearm_L: [-0.9, 0, 0], forearm_R: [-0.9, 0, 0], pelvis: [0, -0.25, 0, 0, 0, 4.5] }, crouch: 0.4 },
    { t: 0.85, b: { torso_twist: [0, -0.7, 0], torso: [0.2, 0, 0], shoulder_L: [-0.3, 0, 0.1], forearm_L: [-0.7, 0, 0], forearm_R: [-0.6, 0, 0], pelvis: [0, -0.2, 0, 0, 0, 4.2] }, crouch: 0.35 },
    { t: 1.5 } ] },
  kick_R: { name: "Stomp kick, right", arm: "R", hit_s: 0.7, side: 0, keys: [
    { t: 0 },
    { t: 0.5, b: { thigh_R: [-1.1, 0, 0], shin_R: [0.9, 0, 0], hock_R: [-0.4, 0, 0], torso: [-0.12, 0, 0], pelvis: [0, 0, -0.06, -0.5, 0.6, 0], shoulder_L: [-0.3, 0, 0.2], shoulder_R: [-0.3, 0, -0.2] } },
    { t: 0.7, b: { thigh_R: [-1.25, 0, 0], shin_R: [-0.2, 0, 0], hock_R: [0.3, 0, 0], torso: [-0.2, 0, 0], pelvis: [0, 0, -0.06, -0.5, 0.7, 0.6], shoulder_L: [-0.35, 0, 0.25], shoulder_R: [-0.35, 0, -0.25] } },
    { t: 0.85, b: { thigh_R: [-1.1, 0, 0], shin_R: [0.1, 0, 0], hock_R: [0.2, 0, 0], torso: [-0.15, 0, 0], pelvis: [0, 0, -0.05, -0.5, 0.6, 0.5], shoulder_L: [-0.3, 0, 0.2], shoulder_R: [-0.3, 0, -0.2] } },
    { t: 1.45 } ] },
  // A gun's kick: the forearm jerks up and the gun slides back along its own length (the hand's +y in the forearm's
  // frame points back along the barrel), the shoulder gives, the torso turns a hair; it settles in 0.3 s with one
  // small overshoot. Played on every shot, layered over whatever the arm is doing.
  recoil_R: { name: "Gun recoil, right", arm: "R", hit_s: 0, side: 0, fire: true, keys: [
    { t: 0 },
    { t: 0.035, b: { forearm_R: [-0.16, 0, 0], hand_R: [0, 0, 0, 0, 0.7, 0], shoulder_R: [0.08, 0, 0], torso_twist: [0, -0.035, 0] } },
    { t: 0.14, b: { forearm_R: [-0.06, 0, 0], hand_R: [0, 0, 0, 0, 0.15, 0], shoulder_R: [0.03, 0, 0], torso_twist: [0, -0.012, 0] } },
    { t: 0.22, b: { forearm_R: [0.015, 0, 0], hand_R: [0, 0, 0, 0, -0.04, 0] } },
    { t: 0.32 } ] },
};
for (const id of ["jab_R", "hook_R", "uppercut_R", "kick_R", "recoil_R"]) MELEE_DEFAULTS[id.slice(0, -2) + "_L"] = meleeMirror(MELEE_DEFAULTS[id]);
// The order the page lists them in.
const MELEE_ORDER = ["jab_L", "jab_R", "hook_L", "hook_R", "uppercut_L", "uppercut_R", "smash", "charge", "kick_L", "kick_R", "recoil_L", "recoil_R"];
for (const id in MELEE_DEFAULTS) MELEE_MOVES[id] = JSON.parse(JSON.stringify(MELEE_DEFAULTS[id]));

// The offsets at time t: a map of bone to [rx, ry, rz, px, py, pz], and "crouch" to [k].
function meleeSample(move, t) {
  const out = new Map(), keys = move.keys;
  if (!keys.length || t <= keys[0].t || t >= keys[keys.length - 1].t) return out;
  let i = 0;
  while (i < keys.length - 2 && t > keys[i + 1].t) i++;
  const a = keys[i], c = keys[i + 1], u = (t - a.t) / Math.max(1e-6, c.t - a.t), k = u * u * (3 - 2 * u);
  const names = new Set([...Object.keys(a.b || {}), ...Object.keys(c.b || {})]);
  for (const n of names) {
    const x = (a.b && a.b[n]) || [], y = (c.b && c.b[n]) || [], v = [];
    for (let j = 0; j < 6; j++) v.push((x[j] || 0) + ((y[j] || 0) - (x[j] || 0)) * k);
    out.set(n, v);
  }
  const cr = (a.crouch || 0) + ((c.crouch || 0) - (a.crouch || 0)) * k;
  if (cr) out.set("crouch", [cr]);
  return out;
}

// The move's length, seconds: its last key.
const meleeEnd = move => move.keys.length ? move.keys[move.keys.length - 1].t : 0;

// Adds move's offsets at time t to the pose P (tools/mockups/mech_pose.js) of mech b, weighted by P.w.
function meleePose(P, b, move, t) {
  for (const [n, v] of meleeSample(move, t)) {
    if (n === "crouch") bendLegs(P, b, v[0]);
    else P.add(n, v[0], v[1], v[2], v[3], v[4], v[5]);
  }
}
