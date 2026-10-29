// Crab Breeze: a mech's stance, broken by blows until it falls (openspec change `close-combat`, section 2b; owner,
// 2026-10-29: "when hit enough, the stance of enemy mech should break, (usually 3 times) and they fall over"). One
// copy, inlined by the arms lab (tools/mockups/gen_melee_lab.py), and the reference for cb-core's Knockdown.
//
// Each blow adds its move's `stance` (1 for a jab, hook, uppercut or kick; 1.5 for the smash and the charge) to the
// struck mech's stance damage. Below `break_at` a blow staggers it (the hit pose); the blow that reaches `break_at`
// breaks its stance and it falls (owner, 2026-10-29: "make the fall animation more dynamic", "move the mech back a
// bit, as if they have some horizontal velocity"): the blow drives it back at knock_speed_m_s, slowing at
// knock_drag_per_s; it stumbles a step (stumble_s) with its torso whipped back and its arms flung, then goes over
// backwards about its heels with a twist toward the side it was struck from (fall_s in all), lands with two bounces,
// a skid and dust, lies (down_s) and gets up through a kneel where it lies (getup_s). While it is down or getting
// up, blows add nothing (no stun lock); standing again, its stance is whole.
// Untouched for recover_delay_s, the damage drains at recover_per_s, so three blows must come close together.

const STANCE_DEFAULTS = { break_at: 3, recover_delay_s: 3, recover_per_s: 0.5, stumble_s: 0.35, fall_s: 1.0, down_s: 1.6,
  getup_s: 1.3, lie_rad: 1.45, roll_rad: 0.22, heel_m: -2.5, knock_speed_m_s: 8, knock_drag_per_s: 1.4, skid_drag_m_s2: 9 };

// The stance damage that breaks each class (owner, 2026-10-29: "lets adjust 2,3,4 light mech, mid, heavy").
const STANCE_BREAK_BY_CLASS = { light: 2, mid: 3, heavy: 4 };

function stanceState() { return { v: 0, since: 99, phase: "up", t: 0, side: 1, back_m: 0, vel: 0, landed: false }; }

// A blow of `amount` from side `side` (+1 its right) on stance st: "stagger", "broken" (it falls now) or "ignored".
function stanceHit(st, amount, cfg, side) {
  cfg = cfg || STANCE_DEFAULTS;
  if (st.phase !== "up") return "ignored";
  st.v += amount; st.since = 0;
  if (st.v >= cfg.break_at - 1e-6) {
    st.v = cfg.break_at; st.phase = "fall"; st.t = 0; st.side = side || 1; st.vel = cfg.knock_speed_m_s; st.back_m = 0; st.landed = false;
    return "broken";
  }
  return "stagger";
}

// Steps the stance by dt seconds: the recovery standing; the slide back, the fall, lying and getting up after a break.
// Sets st.landed for the one step it hits the ground (for the dust and the thud).
function stanceStep(st, dt, cfg) {
  cfg = cfg || STANCE_DEFAULTS;
  st.since += dt; st.landed = false;
  if (st.phase === "up") { if (st.since > cfg.recover_delay_s) st.v = Math.max(0, st.v - cfg.recover_per_s * dt); return; }
  // the slide: air drag while it falls, ground friction once it lands
  st.vel = st.phase === "fall" ? st.vel * Math.exp(-cfg.knock_drag_per_s * dt) : Math.max(0, st.vel - cfg.skid_drag_m_s2 * dt);
  st.back_m += st.vel * dt;
  st.t += dt;
  if (st.phase === "fall" && st.t >= cfg.fall_s) { st.phase = "down"; st.t -= cfg.fall_s; st.landed = true; }
  if (st.phase === "down" && st.t >= cfg.down_s) { st.phase = "getup"; st.t -= cfg.down_s; st.vel = 0; }
  if (st.phase === "getup" && st.t >= cfg.getup_s) { st.phase = "up"; st.t = 0; st.v = 0; st.since = 99; }
}

// The knockdown pose: adds the body's own motion to pose P of mech b and returns how the whole mech is carried:
// {tilt: radians tipped back about its heels, roll: radians turned onto its side, back_m: metres slid back}.
function stancePose(P, b, st, cfg) {
  cfg = cfg || STANCE_DEFAULTS;
  const sm = u => { u = Math.max(0, Math.min(1, u)); return u * u * (3 - 2 * u); };
  const s = st.side, out = { tilt: 0, roll: 0, back_m: st.back_m };
  if (st.phase === "fall") {
    const t = st.t, us = Math.min(1, t / cfg.stumble_s), ut = Math.max(0, (t - cfg.stumble_s) / (cfg.fall_s - cfg.stumble_s));
    // the stumble: knees buckle, the torso whips back past and recoils, the head snaps, the arms fling up and out late
    const whip = Math.sin(Math.min(1, t / 0.28) * Math.PI) * (1 - 0.4 * ut);
    bendLegs(P, b, 0.55 * sm(us) + 0.25 * ut);
    P.add("waist", -0.15 * whip, 0.35 * s * sm(us), 0.08 * s * whip);
    P.add("chest", -0.5 * whip - 0.15 * ut, 0, 0);
    P.add("head", -0.6 * whip, 0.2 * s * whip, 0);
    const fl = sm((t - 0.06) / 0.3);
    P.add("shL", -1.2 * fl - 0.9 * ut, 0, 0.55 * fl); P.add("shR", -1.2 * fl - 0.9 * ut, 0, -0.55 * fl);
    P.add("elbL", -0.5 * fl); P.add("elbR", -0.5 * fl);
    // one leg kicks up as it goes over
    P.add(s > 0 ? "hipL" : "hipR", -0.6 * ut);
    out.tilt = -0.12 * sm(us) - cfg.lie_rad * ut * ut;            // a lean back as it stumbles, then over, faster and faster
    out.roll = cfg.roll_rad * s * ut * ut;
    return out;
  }
  if (st.phase === "down") {
    // the landing: two bounces, the arms slam down after the body, the chest jolts; then it settles
    const t = st.t, bounce = 0.11 * Math.exp(-t * 5) * Math.abs(Math.sin(t * 13));
    const slam = sm(t / 0.25);
    bendLegs(P, b, 0.8 - 0.3 * sm(t / 0.6));
    P.add("chest", -0.15 + 0.25 * Math.exp(-t * 8) * Math.sin(t * 22), 0, 0);
    P.add("shL", -2.1 + 1.0 * slam, 0, 0.55 - 0.2 * slam); P.add("shR", -2.1 + 1.0 * slam, 0, -0.55 + 0.2 * slam);
    P.add("elbL", -0.5 + 0.3 * slam); P.add("elbR", -0.5 + 0.3 * slam);
    P.add(s > 0 ? "hipL" : "hipR", -0.6 * (1 - slam));
    out.tilt = -cfg.lie_rad + bounce; out.roll = cfg.roll_rad * s * (1 - 0.3 * sm(t / 0.5));
    return out;
  }
  if (st.phase === "getup") {
    // rolls back level, pushes up onto its knees, then stands, where it lies
    const u = st.t / cfg.getup_s, a = sm(u / 0.55), k = u < 0.55 ? 0.5 + 0.5 * a : 1 - sm((u - 0.55) / 0.45);
    sitPose(P, b, k);
    const arms = 1 - a;
    P.add("chest", -0.15 * arms, 0, 0); P.add("shL", -1.1 * arms, 0, 0.35 * arms); P.add("shR", -1.1 * arms, 0, -0.35 * arms);
    out.tilt = -cfg.lie_rad * (1 - a); out.roll = cfg.roll_rad * s * 0.7 * (1 - sm(u / 0.3));
    return out;
  }
  return out;
}
