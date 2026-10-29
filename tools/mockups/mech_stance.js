// Crab Breeze: a mech's stance, broken by blows until it falls (openspec change `close-combat`, section 2b; owner,
// 2026-10-29: "when hit enough, the stance of enemy mech should break, (usually 3 times) and they fall over"). One
// copy, inlined by the arms lab (tools/mockups/gen_melee_lab.py), and the reference for cb-core's Knockdown.
//
// Each blow adds its move's `stance` (1 for a jab, hook, uppercut or kick; 1.5 for the smash and the charge) to the
// struck mech's stance damage. Below `break_at` a blow staggers it (the hit pose); the blow that reaches `break_at`
// breaks its stance and it falls: tipping back about its heels (fall_s), lying (down_s), getting up through a kneel
// (getup_s). While it is down or getting up, blows add nothing (no stun lock); standing again, its stance is whole.
// Untouched for recover_delay_s, the damage drains at recover_per_s, so three blows must come close together.

const STANCE_DEFAULTS = { break_at: 3, recover_delay_s: 3, recover_per_s: 0.5, fall_s: 0.6, down_s: 1.4, getup_s: 1.2,
  lie_rad: 1.45, heel_m: -2.5 };

function stanceState() { return { v: 0, since: 99, phase: "up", t: 0 }; }

// A blow of `amount` on stance st: "stagger", "broken" (it falls now) or "ignored" (it is already down).
function stanceHit(st, amount, cfg) {
  cfg = cfg || STANCE_DEFAULTS;
  if (st.phase !== "up") return "ignored";
  st.v += amount; st.since = 0;
  if (st.v >= cfg.break_at - 1e-6) { st.v = cfg.break_at; st.phase = "fall"; st.t = 0; return "broken"; }
  return "stagger";
}

// Steps the stance by dt seconds: the recovery while standing, the fall, lying and getting up after a break.
function stanceStep(st, dt, cfg) {
  cfg = cfg || STANCE_DEFAULTS;
  st.since += dt;
  if (st.phase === "up") { if (st.since > cfg.recover_delay_s) st.v = Math.max(0, st.v - cfg.recover_per_s * dt); return; }
  st.t += dt;
  if (st.phase === "fall" && st.t >= cfg.fall_s) { st.phase = "down"; st.t -= cfg.fall_s; }
  if (st.phase === "down" && st.t >= cfg.down_s) { st.phase = "getup"; st.t -= cfg.down_s; }
  if (st.phase === "getup" && st.t >= cfg.getup_s) { st.phase = "up"; st.t = 0; st.v = 0; st.since = 99; }
}

// The knockdown pose: adds the legs and arms to pose P of mech b and returns how far the body is tipped back about
// its heels, radians (0 standing). The page or the app turns the whole mech by that about the heel line.
function stancePose(P, b, st, cfg) {
  cfg = cfg || STANCE_DEFAULTS;
  const sm = u => { u = Math.max(0, Math.min(1, u)); return u * u * (3 - 2 * u); };
  if (st.phase === "fall") {
    const u = st.t / cfg.fall_s;
    bendLegs(P, b, 0.5 * sm(u));
    P.add("chest", -0.3 * u, 0, 0); P.add("shL", -0.9 * u, 0, 0.3 * u); P.add("shR", -0.9 * u, 0, -0.3 * u);
    return -cfg.lie_rad * u * u;                                   // it goes over faster and faster
  }
  if (st.phase === "down") {
    bendLegs(P, b, 0.5);
    P.add("chest", -0.3, 0, 0); P.add("shL", -0.9, 0, 0.3); P.add("shR", -0.9, 0, -0.3);
    return -cfg.lie_rad + 0.07 * Math.exp(-st.t * 6) * Math.sin(st.t * 18);   // it bounces once as it lands
  }
  if (st.phase === "getup") {
    const u = st.t / cfg.getup_s, a = sm(u / 0.55), k = u < 0.55 ? 0.5 + 0.5 * a : 1 - sm((u - 0.55) / 0.45);
    sitPose(P, b, k);                                              // up through a kneel, as the boarding sit
    const arms = 1 - a;
    P.add("chest", -0.3 * arms, 0, 0); P.add("shL", -0.9 * arms, 0, 0.3 * arms); P.add("shR", -0.9 * arms, 0, -0.3 * arms);
    return -cfg.lie_rad * (1 - a);
  }
  return 0;
}
