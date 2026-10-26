//! What Use does in the game (survey W40; owner, 2026-10-26: "You should be able to scroll wheel select multiple
//! options"). Each fixed step, before the pilot steps, every interaction in reach is offered to the one selector
//! (`cb_core::interact::Selector`): the person in front of the pilot, a terminal, a car door, a stopped car to get out
//! of, the stopped mech. The wheel moves the selection, and a press of Use goes to the selected interaction only
//! (`UseChoice::now`), which the greeting, the terminal, the car and the mech each check before acting. The prompt
//! (`pilot::show_prompt`) draws the selector's lines.

use crate::input::StepActions;
use crate::mech::MechAccessNow;
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use bevy::prelude::*;
use cb_core::interact::{Interaction, Selector};
use cb_core::pilot::PilotMode;

/// The interactions in reach, the one selected, and the one Use does this step.
#[derive(Resource, Default)]
pub struct UseChoice {
    /// What is in reach and what is selected.
    pub selector: Selector,
    /// The interaction Use does this fixed step, if Use was pressed with something in reach.
    pub now: Option<Interaction>,
}

/// Offers what is in reach to the selector and routes Use to the selected interaction.
pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UseChoice>().add_game_systems(
            FixedUpdate,
            choose
                .in_set(SimSet::Pilot)
                .after(crate::traffic::find_car)
                .before(crate::pilot::step_pilot),
        );
    }
}

/// This step's options: the greeting when someone is in reach on foot; the terminal the pilot looks at; the car whose
/// driver's door they stand at, or getting out of the stopped car they drive; stepping into the stopped mech.
#[allow(clippy::too_many_arguments)]
pub fn choose(
    pilot: Res<PilotSim>,
    data: Res<Data>,
    actions: Res<StepActions>,
    access: Res<MechAccessNow>,
    car: Res<crate::traffic::CarAccessNow>,
    terminal: Option<Res<crate::air_bases::TerminalNear>>,
    crowd: Option<Res<crate::civilians::Crowd>>,
    mut choice: ResMut<UseChoice>,
) {
    let p = &pilot.curr;
    let cfg = &data.0.pilot;
    let free = p.waiting.is_none();
    let car = car.0.as_ref();
    let mut in_reach = Vec::new();
    if p.mode == PilotMode::OnFoot
        && crowd
            .as_ref()
            .is_some_and(|c| c.people.in_reach(&data.0.people.cfg, p.feet, p.yaw_rad).is_some())
    {
        in_reach.push(Interaction::Greet);
    }
    if terminal.as_ref().is_some_and(|t| t.near.is_some()) {
        in_reach.push(Interaction::Terminal);
    }
    if free && car.is_some_and(|c| p.can_take(c, cfg)) {
        in_reach.push(Interaction::GetIn);
    }
    if p.mode == PilotMode::Driving && car.is_some_and(|c| c.speed_m_s <= cfg.car_stopped_m_s) {
        in_reach.push(Interaction::GetOut);
    }
    if free && access.0.as_ref().is_some_and(|a| p.can_board(a, cfg)) {
        in_reach.push(Interaction::Board);
    }
    let choice = &mut *choice;
    choice.selector.offer(&in_reach);
    // The wheel moves this list only out of the mech's seat: seated it steps the weapon, or the scope's
    // magnification while the scope is open (railgun-scope), so one turn of it never does two things.
    if p.mode != PilotMode::Seated {
        choice.selector.scroll(actions.0.option_cycle);
    }
    choice.now = choice.selector.pressed(actions.0.use_pressed);
}
