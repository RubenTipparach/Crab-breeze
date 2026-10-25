//! The mech's boarding sequence (openspec change `mech-boarding`): it sits down, its hatch opens in
//! timed stages, it closes the same way in reverse, and it stands up again. One clock does all of
//! it: the timeline time `t_s`, from 0 (standing, sealed) through the end of the sit to
//! `ready_s` (sat down, hatch fully open). Opening runs the clock forward, closing runs it back,
//! so closing is always the opening in reverse (design section 3). Pure: the timeline data, a
//! target and `dt_s` in; the clock out. `cb-app` turns the clock into node motion and vents.

use crate::data::{Validate, finite, finite3, positive, within};
use serde::Deserialize;

/// How the mech sits down (design section 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SitPose {
    /// Time to sit down (and to stand up).
    pub duration_s: f32,
    /// Height of the `pelvis` bone above `root` when sat.
    pub pelvis_height_m: f32,
    /// Angle of the hock segment when sat, from straight down, swinging back (90 is flat behind).
    pub hock_angle_deg: f32,
    /// Torso bow forward when sat.
    pub torso_bow_deg: f32,
}

impl Default for SitPose {
    fn default() -> Self {
        Self {
            duration_s: 3.5,
            pelvis_height_m: 1.4,
            hock_angle_deg: 90.0,
            torso_bow_deg: 20.0,
        }
    }
}

/// One node's motion in a stage: a turn about its own origin (the hinge) and a slide, both in the
/// node's own axes, eased from rest to full over the stage.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Motion {
    /// Node name; a trailing `*` matches every node with that prefix (`LATCH_*`).
    pub node: String,
    /// Hinge axis in the node's own frame.
    pub axis: [f32; 3],
    /// Turn when fully open.
    pub angle_deg: f32,
    /// Slide when fully open, in the node's own frame.
    pub translate_m: [f32; 3],
    /// For a `*` pattern, each matched node (in name order) starts this much after the last.
    pub stagger_s: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            node: String::new(),
            axis: [1.0, 0.0, 0.0],
            angle_deg: 0.0,
            translate_m: [0.0; 3],
            stagger_s: 0.0,
        }
    }
}

/// Which particle a vent releases.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
pub enum VentKind {
    /// Hot, fast, rising steam.
    #[default]
    Steam,
    /// Cold, slow mist that falls and spreads.
    Mist,
}

/// A burst from `VENT_*` empties, along each empty's +z.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Vent {
    /// Empty name; a trailing `*` matches every empty with that prefix.
    pub node: String,
    /// Steam or mist.
    pub kind: VentKind,
    /// Start, from the stage's start.
    pub delay_s: f32,
    /// How long it emits.
    pub duration_s: f32,
    /// Particles a second from each matched empty.
    pub rate_per_s: f32,
    /// Launch speed.
    pub speed_m_s: f32,
}

impl Default for Vent {
    fn default() -> Self {
        Self {
            node: String::new(),
            kind: VentKind::Steam,
            delay_s: 0.0,
            duration_s: 0.5,
            rate_per_s: 60.0,
            speed_m_s: 3.0,
        }
    }
}

/// One stage of the hatch sequence.
#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Stage {
    /// What it is, for logs and the mockup.
    pub name: String,
    /// Start on the timeline.
    pub start_s: f32,
    /// How long its motions take.
    pub duration_s: f32,
    /// What moves.
    pub motions: Vec<Motion>,
    /// What vents.
    pub vents: Vec<Vent>,
}

/// A mech's boarding timeline, `data/mechs/<mech>/boarding.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct BoardingTimeline {
    /// Sitting down.
    pub sit: SitPose,
    /// Hatch stages, in start order.
    pub stages: Vec<Stage>,
    /// Timeline time when the hatch is fully open.
    pub ready_s: f32,
    /// How far from the stair's foot a pilot must be before the empty mech closes up.
    pub clear_m: f32,
    /// How far out in front of the hatch's sill the boarding crane reaches, so the rope from its tip
    /// hangs clear of the knelt mech's hips and legs (owner, 2026-10-24: "extend a crane out then
    /// lower the rope"); measured off each glb by `tools/mechs/gen_mech_data.py`.
    pub crane_reach_m: f32,
    /// The all-view screen's power-up time.
    pub screen_power_up_s: f32,
    /// The all-view screen's power-down time.
    pub screen_power_down_s: f32,
}

impl Default for BoardingTimeline {
    fn default() -> Self {
        Self {
            sit: SitPose::default(),
            stages: Vec::new(),
            ready_s: 3.5,
            clear_m: 3.0,
            crane_reach_m: 3.0,
            screen_power_up_s: 2.5,
            screen_power_down_s: 0.6,
        }
    }
}

impl Validate for BoardingTimeline {
    fn validate(&self) -> Result<(), String> {
        positive("sit.duration_s", self.sit.duration_s)?;
        finite("sit.pelvis_height_m", self.sit.pelvis_height_m)?;
        within("sit.hock_angle_deg", self.sit.hock_angle_deg, -180.0, 180.0)?;
        within("sit.torso_bow_deg", self.sit.torso_bow_deg, -90.0, 90.0)?;
        positive("ready_s", self.ready_s)?;
        if self.ready_s < self.sit.duration_s {
            return Err("`ready_s` must not come before the sit ends".into());
        }
        positive("clear_m", self.clear_m)?;
        positive("crane_reach_m", self.crane_reach_m)?;
        positive("screen_power_up_s", self.screen_power_up_s)?;
        positive("screen_power_down_s", self.screen_power_down_s)?;
        let mut last = 0.0;
        for s in &self.stages {
            let f = |k: &str| format!("stages[{}].{k}", s.name);
            finite(&f("start_s"), s.start_s)?;
            positive(&f("duration_s"), s.duration_s)?;
            if s.start_s < last {
                return Err(format!("stage `{}` starts before the stage above it", s.name));
            }
            last = s.start_s;
            for m in &s.motions {
                if m.node.is_empty() {
                    return Err(format!("{}: a motion needs a node", f("motions")));
                }
                finite3(&f("motions.axis"), m.axis)?;
                if m.angle_deg != 0.0 && glam::Vec3::from_array(m.axis).length() < 1e-3 {
                    return Err(format!("{}: `{}` turns about a zero axis", f("motions"), m.node));
                }
                finite(&f("motions.angle_deg"), m.angle_deg)?;
                finite3(&f("motions.translate_m"), m.translate_m)?;
                finite(&f("motions.stagger_s"), m.stagger_s)?;
            }
            for v in &s.vents {
                finite(&f("vents.delay_s"), v.delay_s)?;
                positive(&f("vents.duration_s"), v.duration_s)?;
                finite(&f("vents.rate_per_s"), v.rate_per_s)?;
                finite(&f("vents.speed_m_s"), v.speed_m_s)?;
            }
        }
        Ok(())
    }
}

fn ease(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl BoardingTimeline {
    /// Timeline time of the sealed, sat-down mech: the sit is over and no hatch stage has begun.
    pub fn sealed_sitting_s(&self) -> f32 {
        self.stages
            .first()
            .map_or(self.sit.duration_s, |s| s.start_s.max(self.sit.duration_s))
    }

    /// How far sat down, 0 standing to 1 sat, eased.
    pub fn sit_amount(&self, t_s: f32) -> f32 {
        ease(t_s / self.sit.duration_s)
    }

    /// How far a stage's motion is open, 0 to 1, eased, for the `index`-th node its pattern matched.
    pub fn motion_amount(&self, stage: &Stage, motion: &Motion, index: usize, t_s: f32) -> f32 {
        let start = stage.start_s + motion.stagger_s * index as f32;
        ease((t_s - start) / stage.duration_s)
    }

    /// True while a vent emits at timeline time `t_s`. Vents fire both ways: opening and closing.
    pub fn vent_active(&self, stage: &Stage, vent: &Vent, t_s: f32) -> bool {
        let start = stage.start_s + vent.delay_s;
        (start..start + vent.duration_s).contains(&t_s)
    }
}

/// Where the sequence is headed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HatchTarget {
    /// Standing, sealed: the mech can walk.
    #[default]
    Standing,
    /// Sat down, sealed: a pilot inside, or an empty mech waiting.
    SealedSitting,
    /// Sat down, hatch open, stair down.
    Open,
}

/// The sequence's clock.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Hatch {
    /// Timeline time.
    pub t_s: f32,
    /// Where it is going.
    pub target: HatchTarget,
}

impl Hatch {
    /// The timeline time a target sits at.
    pub fn target_s(&self, tl: &BoardingTimeline) -> f32 {
        match self.target {
            HatchTarget::Standing => 0.0,
            HatchTarget::SealedSitting => tl.sealed_sitting_s(),
            HatchTarget::Open => tl.ready_s,
        }
    }

    /// Runs the clock toward its target at one second a second.
    pub fn step(&mut self, tl: &BoardingTimeline, dt_s: f32) {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return;
        }
        let goal = self.target_s(tl);
        self.t_s += (goal - self.t_s).clamp(-dt_s, dt_s);
    }

    /// True when standing and sealed: only then may the legs walk.
    pub fn can_walk(&self) -> bool {
        self.t_s == 0.0 && self.target == HatchTarget::Standing
    }

    /// True when fully open with the stair down.
    pub fn is_open(&self, tl: &BoardingTimeline) -> bool {
        self.target == HatchTarget::Open && self.t_s >= tl.ready_s
    }

    /// True when the hatch is shut: not heading open, and no further along the timeline than sat
    /// and sealed (standing, standing up, sitting down, or sat sealed). The cockpit pass draws only
    /// then (cockpit-overlay).
    pub fn is_closed(&self, tl: &BoardingTimeline) -> bool {
        self.target != HatchTarget::Open && self.t_s <= tl.sealed_sitting_s() + 1e-3
    }

    /// True when sat and sealed (no stage under way).
    pub fn is_sealed_sitting(&self, tl: &BoardingTimeline) -> bool {
        self.target == HatchTarget::SealedSitting && (self.t_s - tl.sealed_sitting_s()).abs() < 1e-6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_ron;
    use std::path::PathBuf;

    const DT: f32 = 1.0 / 60.0;

    fn shipped(mech: &str) -> BoardingTimeline {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/mechs/{mech}/boarding.ron"));
        load_ron(&p).unwrap_or_else(|e| panic!("{e}"))
    }

    fn run(h: &mut Hatch, tl: &BoardingTimeline, seconds: f32) {
        for _ in 0..(seconds / DT).ceil() as usize {
            h.step(tl, DT);
        }
    }

    #[test]
    fn the_shipped_timelines_load() {
        // every mech's, the eight detailed mechs' among them (mech-roster)
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mechs = crate::GameData::mech_files(&root);
        assert!(mechs.len() >= 10, "{mechs:?}");
        for m in mechs {
            shipped(m.trim_start_matches("mechs/").trim_end_matches(".ron"));
        }
    }

    #[test]
    fn opening_sits_then_opens_by_ready_time() {
        let tl = shipped("breeze");
        let mut h = Hatch {
            target: HatchTarget::Open,
            ..Default::default()
        };
        run(&mut h, &tl, tl.sit.duration_s);
        assert!((tl.sit_amount(h.t_s) - 1.0).abs() < 1e-3, "sat after the sit's duration");
        assert!(!h.is_open(&tl));
        run(&mut h, &tl, tl.ready_s);
        assert!(h.is_open(&tl));
        assert!(!h.can_walk());
    }

    #[test]
    fn the_hatch_is_closed_only_while_shut() {
        let tl = shipped("breeze");
        let mut h = Hatch::default();
        assert!(h.is_closed(&tl), "standing, sealed");
        h.target = HatchTarget::Open;
        h.step(&tl, DT);
        assert!(!h.is_closed(&tl), "opening");
        run(&mut h, &tl, tl.ready_s + 1.0);
        assert!(!h.is_closed(&tl), "open");
        h.target = HatchTarget::SealedSitting;
        run(&mut h, &tl, tl.ready_s + 1.0);
        assert!(h.is_closed(&tl), "sat and sealed again");
    }

    #[test]
    fn closing_is_the_opening_in_reverse() {
        let tl = shipped("breeze");
        let stage = tl.stages.iter().find(|s| s.name == "visor lifts").expect("visor stage");
        let motion = &stage.motions[0];
        let at = stage.start_s + stage.duration_s * 0.3;
        let mut opening = Hatch {
            target: HatchTarget::Open,
            ..Default::default()
        };
        run(&mut opening, &tl, at);
        let mut closing = Hatch {
            t_s: tl.ready_s,
            target: HatchTarget::SealedSitting,
        };
        run(&mut closing, &tl, tl.ready_s - at);
        let a = tl.motion_amount(stage, motion, 0, opening.t_s);
        let b = tl.motion_amount(stage, motion, 0, closing.t_s);
        // Both clocks land within one step of `at`; the pose is a function of the clock alone.
        assert!((opening.t_s - closing.t_s).abs() <= DT + 1e-4);
        assert!((a - b).abs() < 0.05, "the same pose at the same time either way: {a} vs {b}");
        run(&mut closing, &tl, tl.ready_s);
        assert!(closing.is_sealed_sitting(&tl), "closing stops sat and sealed, not standing");
        assert_eq!(tl.motion_amount(stage, motion, 0, closing.t_s), 0.0);
    }

    #[test]
    fn standing_up_takes_the_sit_in_reverse() {
        let tl = shipped("breeze");
        let mut h = Hatch {
            t_s: tl.sealed_sitting_s(),
            target: HatchTarget::Standing,
        };
        assert!(!h.can_walk());
        run(&mut h, &tl, tl.sealed_sitting_s() + 0.1);
        assert!(h.can_walk());
    }

    #[test]
    fn stages_open_in_order() {
        let tl = shipped("breeze");
        for w in tl.stages.windows(2) {
            assert!(w[0].start_s <= w[1].start_s, "{} before {}", w[0].name, w[1].name);
        }
        let last = tl.stages.last().unwrap();
        assert!(
            last.start_s + last.duration_s <= tl.ready_s + 1e-3,
            "every stage is open by ready_s"
        );
    }
}
