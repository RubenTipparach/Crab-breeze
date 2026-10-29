//! A mech's stance and its knockdown (openspec change `close-combat`, section 2b): blows wear the stance down,
//! and the blow that breaks it drives the mech back and over, to lie a while and get up where it slid to. The
//! rules and the pose follow the arms lab's `tools/mockups/mech_stance.js`, which the owner shaped on the page
//! (2026-10-29: "when hit enough, the stance of enemy mech should break", "make the fall animation more dynamic",
//! "move the mech back a bit, as if they have some horizontal velocity"). Pure.

use crate::mech::MeleeConfig;
use glam::{EulerRot, Quat, Vec2, Vec3};

/// Where a knocked-down mech is in its fall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnockPhase {
    /// Stumbling a step back, then going over (`knockdown_fall_s`).
    Fall,
    /// Lying on the ground (`knockdown_down_s`).
    Down,
    /// Getting up through a kneel (`knockdown_getup_s`).
    GetUp,
}

/// A knockdown in progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knockdown {
    /// The phase.
    pub phase: KnockPhase,
    /// Seconds into the phase.
    pub t_s: f32,
    /// How fast it slides, m/s, along `dir`.
    pub speed_m_s: f32,
    /// The way it slides (x, z), a unit vector: the way the blow drove it.
    pub dir: Vec2,
    /// The side it was struck from, +1 its right, -1 its left: it rolls that way as it falls.
    pub side: f32,
}

/// What a blow did to a stance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StanceHit {
    /// It staggers and stays up.
    Stagger,
    /// Its stance broke: it falls now.
    Broken,
    /// It is already down: the blow adds nothing (no stun lock).
    Ignored,
}

/// A mech's stance: the damage blows have done to it, and the knockdown while it is down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stance {
    /// Stance damage, in blows: it falls at `melee.stance_break_at`.
    pub damage: f32,
    /// Seconds since the last blow.
    pub since_hit_s: f32,
    /// The knockdown, while it is down or getting up.
    pub knock: Option<Knockdown>,
    /// It hit the ground this step (for the dust and the thud).
    pub landed: bool,
}

impl Default for Stance {
    fn default() -> Self {
        Self {
            damage: 0.0,
            since_hit_s: f32::MAX,
            knock: None,
            landed: false,
        }
    }
}

impl Stance {
    /// Knocked down: falling, lying or getting up. It does not walk, turn, fire or strike.
    pub fn down(&self) -> bool {
        self.knock.is_some()
    }

    /// A blow of `amount` (blows), driving the mech along `push` (x, z; any length) from `side` (+1 its right):
    /// staggers it, or breaks its stance and starts its fall. A blow on a mech that is down adds nothing.
    pub fn hit(&mut self, amount: f32, push: Vec2, side: f32, cfg: &MeleeConfig) -> StanceHit {
        if self.down() {
            return StanceHit::Ignored;
        }
        if !amount.is_finite() || amount <= 0.0 {
            return StanceHit::Stagger;
        }
        self.damage += amount;
        self.since_hit_s = 0.0;
        if self.damage + 1e-4 < cfg.stance_break_at {
            return StanceHit::Stagger;
        }
        self.damage = cfg.stance_break_at;
        self.knock = Some(Knockdown {
            phase: KnockPhase::Fall,
            t_s: 0.0,
            speed_m_s: cfg.knockdown_speed_m_s,
            dir: push.try_normalize().unwrap_or(Vec2::Y),
            side: if side < 0.0 { -1.0 } else { 1.0 },
        });
        StanceHit::Broken
    }

    /// One step of `dt_s`: standing, the damage mends after `stance_recover_delay_s`; down, the fall runs on. Returns
    /// how far it slides this step (x, z), metres.
    pub fn step(&mut self, cfg: &MeleeConfig, dt_s: f32) -> Vec2 {
        self.landed = false;
        self.since_hit_s = (self.since_hit_s + dt_s).min(f32::MAX / 2.0);
        let Some(mut k) = self.knock else {
            if self.since_hit_s > cfg.stance_recover_delay_s {
                self.damage = (self.damage - cfg.stance_recover_per_s * dt_s).max(0.0);
            }
            return Vec2::ZERO;
        };
        // The slide: drag while it falls, the ground's friction once it is down.
        k.speed_m_s = match k.phase {
            KnockPhase::Fall => k.speed_m_s * (-cfg.knockdown_drag_per_s * dt_s).exp(),
            _ => (k.speed_m_s - cfg.knockdown_skid_m_s2 * dt_s).max(0.0),
        };
        let slide = k.dir * k.speed_m_s * dt_s;
        k.t_s += dt_s;
        if k.phase == KnockPhase::Fall && k.t_s >= cfg.knockdown_fall_s {
            k.phase = KnockPhase::Down;
            k.t_s -= cfg.knockdown_fall_s;
            self.landed = true;
        }
        if k.phase == KnockPhase::Down && k.t_s >= cfg.knockdown_down_s {
            k.phase = KnockPhase::GetUp;
            k.t_s -= cfg.knockdown_down_s;
            k.speed_m_s = 0.0;
        }
        if k.phase == KnockPhase::GetUp && k.t_s >= cfg.knockdown_getup_s {
            *self = Self::default();
            return slide;
        }
        self.knock = Some(k);
        slide
    }

    /// The knockdown pose now (the rest pose when standing).
    pub fn pose(&self, cfg: &MeleeConfig) -> KnockPose {
        self.knock.map_or_else(KnockPose::default, |k| knock_pose(&k, cfg))
    }
}

/// How a knocked-down mech is posed: the whole body carried about its heels, its legs folded by the kneel, and turns
/// over the pose beneath for the torso, head, arms and one thigh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KnockPose {
    /// Tipped back about the heel line, radians (0 standing; negative goes over backwards).
    pub tilt_rad: f32,
    /// Rolled about its forward axis, radians, toward the side it was struck from.
    pub roll_rad: f32,
    /// How far the legs fold, as the boarding sit's amount (0 standing, 1 knelt).
    pub sit: f32,
    /// Turns in each bone's own rest axes, over the pose beneath.
    pub turns: Vec<(&'static str, Quat)>,
}

impl KnockPose {
    /// The turn and the move that carry the whole mech: the body turned about the heel line, `heel_back_m` behind
    /// its feet, with `forward` its facing. Returns the rotation to put before its heading and the offset to add
    /// to its position.
    pub fn carry(&self, forward: Vec3, heel_back_m: f32) -> (Quat, Vec3) {
        if self.tilt_rad == 0.0 && self.roll_rad == 0.0 {
            return (Quat::IDENTITY, Vec3::ZERO);
        }
        let f = Vec3::new(forward.x, 0.0, forward.z).try_normalize().unwrap_or(Vec3::NEG_Z);
        // A negative tilt carries the top of the body back (toward -f): about f x up by its size.
        let tip = Quat::from_axis_angle(f.cross(Vec3::Y).normalize(), -self.tilt_rad);
        let roll = Quat::from_axis_angle(f, self.roll_rad);
        let r = roll * tip;
        let heel = -f * heel_back_m;
        (r, heel - r * heel)
    }
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

fn turn(rx: f32, ry: f32, rz: f32) -> Quat {
    Quat::from_euler(EulerRot::XYZ, rx, ry, rz)
}

/// The arms lab's `stancePose`, in the game's terms (the bones' own axes: x along the mech's left, so a negative x
/// turn swings a bone's far end forward and up). The lab's leg kick is left out: the game solves each leg bone on
/// its own, so a turn on one thigh would part it from its shin.
fn knock_pose(k: &Knockdown, cfg: &MeleeConfig) -> KnockPose {
    let s = k.side;
    let t = k.t_s;
    let (lie, roll) = (cfg.knockdown_lie_rad, cfg.knockdown_roll_rad);
    match k.phase {
        KnockPhase::Fall => {
            let us = (t / cfg.knockdown_stumble_s).min(1.0);
            let ut = ((t - cfg.knockdown_stumble_s) / (cfg.knockdown_fall_s - cfg.knockdown_stumble_s)).clamp(0.0, 1.0);
            // The stumble: knees buckle, the torso whips back and recoils, the head snaps, the arms fling late.
            let whip = ((t / 0.28).min(1.0) * std::f32::consts::PI).sin() * (1.0 - 0.4 * ut);
            let fl = smooth((t - 0.06) / 0.3);
            KnockPose {
                tilt_rad: -0.12 * smooth(us) - lie * ut * ut,
                roll_rad: roll * s * ut * ut,
                sit: 0.45 * smooth(us) + 0.2 * ut,
                turns: vec![
                    ("torso_twist", turn(-0.15 * whip, 0.35 * s * smooth(us), 0.08 * s * whip)),
                    ("torso", turn(-0.5 * whip - 0.15 * ut, 0.0, 0.0)),
                    ("head", turn(-0.6 * whip, 0.2 * s * whip, 0.0)),
                    ("shoulder_L", turn(-1.2 * fl - 0.9 * ut, 0.0, 0.55 * fl)),
                    ("shoulder_R", turn(-1.2 * fl - 0.9 * ut, 0.0, -0.55 * fl)),
                    ("forearm_L", turn(-0.5 * fl, 0.0, 0.0)),
                    ("forearm_R", turn(-0.5 * fl, 0.0, 0.0)),
                ],
            }
        }
        KnockPhase::Down => {
            // Two bounces as it lands, the arms slamming down after the body, the chest jolting; then it settles.
            let bounce = 0.11 * (-t * 5.0).exp() * (t * 13.0).sin().abs();
            let slam = smooth(t / 0.25);
            KnockPose {
                tilt_rad: -lie + bounce,
                roll_rad: roll * s * (1.0 - 0.3 * smooth(t / 0.5)),
                sit: 0.65 - 0.25 * smooth(t / 0.6),
                turns: vec![
                    ("torso", turn(-0.15 + 0.25 * (-t * 8.0).exp() * (t * 22.0).sin(), 0.0, 0.0)),
                    ("shoulder_L", turn(-2.1 + slam, 0.0, 0.55 - 0.2 * slam)),
                    ("shoulder_R", turn(-2.1 + slam, 0.0, -0.55 + 0.2 * slam)),
                    ("forearm_L", turn(-0.5 + 0.3 * slam, 0.0, 0.0)),
                    ("forearm_R", turn(-0.5 + 0.3 * slam, 0.0, 0.0)),
                ],
            }
        }
        KnockPhase::GetUp => {
            // Rolls level, rises onto its knees through the boarding kneel, then stands where it lies.
            let u = t / cfg.knockdown_getup_s;
            let a = smooth(u / 0.55);
            let sit = if u < 0.55 { 0.4 + 0.6 * a } else { 1.0 - smooth((u - 0.55) / 0.45) };
            let arms = 1.0 - a;
            KnockPose {
                tilt_rad: -lie * (1.0 - a),
                roll_rad: roll * s * 0.7 * (1.0 - smooth(u / 0.3)),
                sit,
                turns: vec![
                    ("torso", turn(-0.15 * arms, 0.0, 0.0)),
                    ("shoulder_L", turn(-1.1 * arms, 0.0, 0.35 * arms)),
                    ("shoulder_R", turn(-1.1 * arms, 0.0, -0.35 * arms)),
                ],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn cfg(break_at: f32) -> MeleeConfig {
        MeleeConfig {
            stance_break_at: break_at,
            ..MeleeConfig::default()
        }
    }

    fn run(st: &mut Stance, c: &MeleeConfig, s: f32) -> Vec2 {
        let mut slid = Vec2::ZERO;
        for _ in 0..(s / DT).round() as usize {
            slid += st.step(c, DT);
        }
        slid
    }

    #[test]
    fn a_mid_mech_staggers_twice_and_falls_on_the_third_blow() {
        let c = cfg(3.0);
        let mut st = Stance::default();
        assert_eq!(st.hit(1.0, Vec2::Y, 1.0, &c), StanceHit::Stagger);
        run(&mut st, &c, 0.5);
        assert_eq!(st.hit(1.0, Vec2::Y, 1.0, &c), StanceHit::Stagger);
        run(&mut st, &c, 0.5);
        assert_eq!(st.hit(1.0, Vec2::Y, 1.0, &c), StanceHit::Broken);
        assert!(st.down());
    }

    #[test]
    fn a_light_mech_falls_on_the_second_blow_and_a_heavy_one_on_the_fourth() {
        for (break_at, blows) in [(2.0, 2), (4.0, 4)] {
            let c = cfg(break_at);
            let mut st = Stance::default();
            for i in 1..=blows {
                let r = st.hit(1.0, Vec2::Y, 1.0, &c);
                assert_eq!(r == StanceHit::Broken, i == blows, "{break_at} blow {i}: {r:?}");
            }
        }
    }

    #[test]
    fn two_heavy_blows_break_a_mid_stance() {
        let c = cfg(3.0);
        let mut st = Stance::default();
        st.hit(1.5, Vec2::Y, 1.0, &c);
        assert_eq!(st.hit(1.5, Vec2::Y, 1.0, &c), StanceHit::Broken);
    }

    #[test]
    fn a_blow_on_a_mech_that_is_down_adds_nothing_and_it_stands_with_its_stance_whole() {
        let c = cfg(2.0);
        let mut st = Stance::default();
        st.hit(2.0, Vec2::Y, 1.0, &c);
        run(&mut st, &c, 1.2);
        assert_eq!(st.hit(1.0, Vec2::Y, 1.0, &c), StanceHit::Ignored);
        let whole = c.knockdown_fall_s + c.knockdown_down_s + c.knockdown_getup_s;
        run(&mut st, &c, whole);
        assert!(!st.down());
        assert_eq!(st.damage, 0.0);
    }

    #[test]
    fn it_lies_down_lands_once_and_is_up_after_the_whole_knockdown() {
        let c = cfg(1.0);
        let mut st = Stance::default();
        st.hit(1.0, Vec2::Y, -1.0, &c);
        let mut landings = 0;
        let mut down_at = None;
        let whole = c.knockdown_fall_s + c.knockdown_down_s + c.knockdown_getup_s;
        for i in 0..((whole + 0.2) / DT) as usize {
            st.step(&c, DT);
            landings += usize::from(st.landed);
            if down_at.is_none() && st.knock.is_some_and(|k| k.phase == KnockPhase::Down) {
                down_at = Some(i as f32 * DT);
            }
        }
        assert_eq!(landings, 1);
        assert!((down_at.unwrap() - c.knockdown_fall_s).abs() < 2.0 * DT);
        assert!(!st.down());
    }

    #[test]
    fn the_blow_drives_it_back_a_few_metres_along_the_blow() {
        let c = cfg(1.0);
        let mut st = Stance::default();
        st.hit(1.0, Vec2::new(3.0, 4.0), 1.0, &c);
        let slid = run(&mut st, &c, 4.0);
        assert!((3.0..6.0).contains(&slid.length()), "{slid}");
        assert!(slid.normalize().dot(Vec2::new(0.6, 0.8)) > 0.999);
    }

    #[test]
    fn blows_spread_out_mend_and_never_break_it() {
        let c = cfg(3.0);
        let mut st = Stance::default();
        for _ in 0..6 {
            assert_ne!(st.hit(1.0, Vec2::Y, 1.0, &c), StanceHit::Broken);
            run(&mut st, &c, 6.0);
        }
    }

    #[test]
    fn falling_it_goes_over_backwards_and_rolls_toward_the_blow_and_stands_level_after() {
        let c = cfg(1.0);
        let mut st = Stance::default();
        st.hit(1.0, Vec2::Y, -1.0, &c);
        run(&mut st, &c, c.knockdown_fall_s + 0.5);
        let p = st.pose(&c);
        assert!(p.tilt_rad < -1.2, "{p:?}");
        assert!(p.roll_rad < 0.0);
        run(&mut st, &c, 4.0);
        assert_eq!(st.pose(&c), KnockPose::default());
    }

    #[test]
    fn the_carry_keeps_the_heel_line_where_it_stands_and_tips_the_top_back() {
        let pose = KnockPose {
            tilt_rad: -1.4,
            ..KnockPose::default()
        };
        let f = Vec3::NEG_Z;
        let (r, off) = pose.carry(f, 2.5);
        let heel = -f * 2.5;
        assert!((r * heel + off - heel).length() < 1e-4);
        let top = r * Vec3::Y * 20.0 + off;
        assert!(top.dot(f) < -15.0, "{top}");
        assert!(top.y < 6.0);
    }
}
