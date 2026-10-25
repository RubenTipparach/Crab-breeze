//! The scripted demolition (`structural-destruction` task 2.4): the mid-rise nearest the mech, the
//! pilot 11 m off its -z face, F9 (a shell at the aim point, a key every player has) at the foot
//! of each column on that face in turn, then watching it creak, fall and settle, then a walk up to
//! the rubble. One implementation for the two instruments that play it: the building stills
//! (`capture`, which shoots each stage) and the `demolition` benchmark take (`--bench demolition`,
//! which times every frame of it). The pilot is placed and turned directly, as the stills always
//! did; everything that breaks, breaks through the game's own shell.

use crate::input::InputAccum;
use crate::pilot::PilotSim;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::input::Actions;
use cb_core::pilot::PilotState;

/// Eye height over the feet, for aiming the view.
const EYE_M: f32 = 1.7;
/// Seconds the whole building is looked at before the first shell.
pub const WHOLE_S: f32 = 2.0;
/// Least seconds between shells; each also waits for an aim held since the frame before.
pub const SHELL_GAP_S: f32 = 0.5;
/// Seconds of watching the fall after the last shell.
pub const FALL_S: f32 = 13.0;
/// Seconds at the rubble.
pub const CLOSE_S: f32 = 2.0;

/// The demolition's stages, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Looking at the whole building.
    Whole,
    /// Shelling the foot of each facing column.
    Shelling,
    /// Watching it creak, fall and settle.
    Fall,
    /// Up close at the rubble.
    Close,
}

/// One demolition in progress.
pub struct Demolition {
    /// The building being brought down.
    pub building: u32,
    feet: Vec3,
    aims: Vec<Vec3>,
    shells_fired: usize,
    aimed_at: Option<usize>,
}

impl Demolition {
    /// The 8 to 20 storey building nearest `near`, with the pilot's stand and the column aims; none
    /// when there is no such building.
    pub fn pick(plan: &Plan, live: &Live, near: Vec2) -> Option<Self> {
        let b = plan
            .0
            .buildings
            .iter()
            .filter(|b| (8..=20).contains(&b.storeys))
            .min_by(|p, q| p.footprint.centre().distance(near).total_cmp(&q.footprint.centre().distance(near)))?;
        let ground = plan.0.cities[b.city as usize].ground_m;
        let c = b.footprint.centre();
        let feet = Vec3::new(c.x, ground, b.footprint.min.y - 11.0);
        let feet = Vec3::new(feet.x, plan.0.terrain.height_at(feet.x, feet.z).max(ground), feet.z);
        // The foot of every column on the facing side, as a player taking out a side would aim.
        let cells = &live.structures.buildings[b.id as usize];
        let aims = (0..=cells.n[0])
            .map(|a| Vec3::new(b.footprint.min.x + cells.size.x * a as f32, ground, b.footprint.min.y + 1.0))
            .collect();
        println!("DEMOLITION_BUILDING id={} storeys={} at ({:.0}, {:.0})", b.id, b.storeys, c.x, c.y);
        Some(Self {
            building: b.id,
            feet,
            aims,
            shells_fired: 0,
            aimed_at: None,
        })
    }

    /// Plays `stage`, `in_stage_s` seconds into it (game time): places the pilot and turns the view,
    /// and while shelling presses F9 at each column in turn. True when the stage is over.
    pub fn play(&mut self, stage: Stage, in_stage_s: f32, plan: &Plan, pilot: &mut PilotSim, acc: &mut InputAccum) -> bool {
        let b = &plan.0.buildings[self.building as usize];
        let ground = plan.0.cities[b.city as usize].ground_m;
        let face = |up: f32| Vec3::new(b.footprint.centre().x, ground + up, b.footprint.min.y);
        match stage {
            Stage::Whole => {
                place(pilot, self.feet, face(b.height_m * 0.45));
                in_stage_s > WHOLE_S
            }
            Stage::Shelling => {
                acc.script = Some(Actions::default());
                let i = self.shells_fired;
                let Some(&aim) = self.aims.get(i) else {
                    return true;
                };
                place(pilot, self.feet, aim);
                if self.aimed_at == Some(i) && in_stage_s > i as f32 * SHELL_GAP_S + 0.2 {
                    // Pressed like the key: latched until the next simulation step takes it.
                    acc.debug = Some(cb_core::input::DebugCommand::Shell);
                    self.shells_fired += 1;
                }
                self.aimed_at = Some(i);
                false
            }
            Stage::Fall => {
                place(pilot, self.feet, face(b.height_m * 0.4));
                in_stage_s > FALL_S
            }
            Stage::Close => {
                let close = Vec3::new(self.feet.x, self.feet.y, b.footprint.min.y - 6.0);
                place(
                    pilot,
                    close,
                    Vec3::new(b.footprint.centre().x, ground + 2.0, b.footprint.min.y + 6.0),
                );
                in_stage_s > CLOSE_S
            }
        }
    }

    /// The building's state now, as the line both instruments print: standing cells of all, and
    /// standing frame elements.
    pub fn standing(&self, live: &Live) -> String {
        let c = &live.structures.buildings[self.building as usize];
        format!(
            "cells={}/{} frame_elements={}",
            c.standing,
            c.intact.len(),
            c.frame.as_ref().map_or(0, |f| f.standing())
        )
    }
}

/// Stands the pilot at `feet` facing `target`.
pub fn place(pilot: &mut PilotSim, feet: Vec3, target: Vec3) {
    let d = target - (feet + Vec3::Y * EYE_M);
    let mut s = PilotState::standing(feet, (-d.x).atan2(-d.z));
    s.pitch_rad = d.y.atan2(Vec2::new(d.x, d.z).length());
    pilot.prev = s.clone();
    pilot.curr = s;
}

/// The `demolition` take's beats, for the benchmark's report.
pub const TAKE_BEATS: [&str; 6] = ["load", "whole", "shelling", "fall", "close", "end"];
/// Game seconds the take waits after `GAME_READY` before it starts: the benchmark throws away its
/// first 10 s of frames, and the demolition must fall inside what it keeps.
const SETTLE_S: f32 = 12.0;

/// The `demolition` take (`--showcase demolition`, `--bench demolition`).
#[derive(Resource, Default)]
pub struct DemolitionTake {
    demolition: Option<Demolition>,
    beat_started_s: f32,
}

/// Plays the demolition take beat by beat on the game's clock, and prints the recorder's lines and,
/// every half second from the first shell, how much of the building stands.
#[allow(clippy::too_many_arguments)]
pub fn play_take(
    mut take: ResMut<crate::showcase::Take>,
    mut state: ResMut<DemolitionTake>,
    mut acc: ResMut<InputAccum>,
    mut pilot: ResMut<PilotSim>,
    access: Res<crate::mech::MechAccessNow>,
    plan: Res<Plan>,
    live: Res<Live>,
    ready: Res<crate::loading::GameReady>,
    time: Res<Time<Virtual>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let in_beat = now - state.beat_started_s;
    if let Some(d) = state.demolition.as_ref()
        && take.beat >= 2
        && (now * 2.0) as u32 != ((now - time.delta_secs()) * 2.0) as u32
    {
        println!("DEMOLITION_STANDING beat={} {}", TAKE_BEATS[take.beat.min(5)], d.standing(&live));
    }
    let next = match take.beat {
        0 => {
            let Some(a) = access.0 else { return };
            if !ready.0 || in_beat < SETTLE_S {
                return;
            }
            let Some(d) = Demolition::pick(&plan, &live, Vec2::new(a.ladder_base.x, a.ladder_base.z)) else {
                println!("PLAYTEST_RESULT fail demolition: no 8 to 20 storey building in the world");
                exit.write(AppExit::error());
                return;
            };
            state.demolition = Some(d);
            println!("PLAYTEST_START demolition");
            true
        }
        b @ 1..=4 => {
            let stage = [Stage::Whole, Stage::Shelling, Stage::Fall, Stage::Close][b - 1];
            let d = state.demolition.as_mut().expect("picked in the load beat");
            d.play(stage, in_beat, &plan, &mut pilot, &mut acc)
        }
        _ => {
            if in_beat > 1.5 {
                let d = state.demolition.as_ref().expect("picked in the load beat");
                let c = &live.structures.buildings[d.building as usize];
                let verdict = if c.standing < c.intact.len() { "ok" } else { "fail" };
                println!(
                    "PLAYTEST_RESULT {verdict} demolition: building {} {}",
                    d.building,
                    d.standing(&live)
                );
                println!("PLAYTEST_END demolition");
                exit.write(AppExit::Success);
            }
            false
        }
    };
    if next {
        take.beat += 1;
        state.beat_started_s = now;
    }
}
