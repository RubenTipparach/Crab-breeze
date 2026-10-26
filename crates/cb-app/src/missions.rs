//! Free roam's missions in the game (openspec change `free-roam-missions`): free roam's mail, one message per
//! mission, read on the menu's MAIL page (M), and the one switch every mission plugin listens to. Starting a
//! mission turns off the one running first (owner, 2026-10-25: "if activating a new mission it should turn off
//! existing mission"); each mission plugin begins and ends itself on `MissionSwitch`, so a new mission is one
//! plugin and one message in `data/missions/mail.ron`. The rule is `cb_core::missions`.

use crate::menu::Play;
use crate::sim::Data;
use bevy::prelude::*;
use cb_core::missions::{FreeRoam, MissionKind, Switch};

/// Free roam's mail and its running mission (a free-roam run, or one begun as the defence).
#[derive(Resource)]
pub struct FreeRoamRun {
    /// The mail and the running mission.
    pub free: FreeRoam,
    /// Switches asked for (by the mail page) and not yet sent.
    pending: Vec<Switch>,
}

impl FreeRoamRun {
    /// Starts `kind`, turning off the mission running (the switch goes out this frame).
    pub fn start(&mut self, kind: MissionKind) {
        let s = self.free.start(kind);
        self.pending.push(s);
    }

    /// Turns off the mission running.
    pub fn stop(&mut self) {
        let s = self.free.stop();
        if s.stopped.is_some() {
            self.pending.push(s);
        }
    }
}

/// Turn off `stop`, then begin `start`. Every mission plugin reads it and acts for its own kind only; a
/// mission begun before startup (`--mission defence`) is already running and is not switched.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionSwitch {
    /// The mission to turn off.
    pub stop: Option<MissionKind>,
    /// The mission to begin.
    pub start: Option<MissionKind>,
}

/// A mission to start as soon as free roam begins (`--mission fortress`).
#[derive(Resource, Clone, Copy)]
pub struct StartWith(pub MissionKind);

/// Free roam's mail and the switch.
pub struct MissionsPlugin;

/// Where `MissionSwitch` is written; mission plugins read it after this set.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MissionsSet;

impl Plugin for MissionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MissionSwitch>().add_systems(
            Update,
            (begin, send).chain().in_set(MissionsSet).before(crate::sim::PresentSet::Input),
        );
    }
}

/// Free roam's mail arrives when a free-roam or defence run begins (a career has its own mail).
fn begin(mut commands: Commands, play: Option<Res<Play>>, data: Res<Data>, start: Option<Res<StartWith>>, have: Option<Res<FreeRoamRun>>) {
    let Some(play) = play else { return };
    if have.is_some() || !play.is_added() || *play == Play::Career {
        return;
    }
    let mut run = FreeRoamRun {
        free: FreeRoam::new(&data.0.missions),
        pending: Vec::new(),
    };
    if *play == Play::Defence {
        // already running, begun by the title menu or the command line
        run.free.active = Some(MissionKind::Defence);
    }
    if let Some(s) = start {
        run.start(s.0);
        commands.remove_resource::<StartWith>();
    }
    commands.insert_resource(run);
}

/// Sends the switches the mail page asked for.
fn send(run: Option<ResMut<FreeRoamRun>>, mut out: MessageWriter<MissionSwitch>) {
    let Some(mut run) = run else { return };
    if run.pending.is_empty() {
        return;
    }
    for s in std::mem::take(&mut run.pending) {
        println!("MISSION switch: stop {:?}, start {:?}", s.stopped, s.started);
        out.write(MissionSwitch {
            stop: s.stopped,
            start: s.started,
        });
    }
}

/// `FREE ROAM: press M for mail` when no mission runs in free roam.
pub fn idle_line(run: Option<&FreeRoamRun>) -> Option<&'static str> {
    run.filter(|r| r.free.active.is_none()).map(|_| "FREE ROAM: press M for mail")
}
