//! Harm to civilians costs career standing (survey W34, W37, W42): reads the four features' hooks, people
//! killed (`civilians::CivilianLoss`), cars harmed (`traffic::CivilianHarmed`), civil aircraft hit
//! (`air_traffic::CivilAirHit`) and boats harmed (`boats::BoatHarmed`, harbours HB16), turns each into a `cb_core::civil_standing::CivilHarm` and hands it to the one
//! rule, `Charges::charge`, against the career being played. Outside a career nothing is charged; the features'
//! own counters still count.

use crate::air_traffic::CivilAirHit;
use crate::career::CareerRun;
use crate::civilians::CivilianLoss;
use crate::state::GameApp;
use crate::traffic::CivilianHarmed;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::civil_standing::{Charges, CivilHarm, Victim};

/// The victims already charged in this run.
#[derive(Resource, Default)]
struct CivilCharges(Charges);

/// Wires civilian harm to career standing.
pub struct CivilStandingPlugin;

impl Plugin for CivilStandingPlugin {
    fn build(&self, app: &mut App) {
        // Registered here too, so the reader is valid when a feature is left out (`CB_NO_PEOPLE=1`).
        app.add_message::<CivilianLoss>()
            .add_message::<CivilianHarmed>()
            .add_message::<CivilAirHit>()
            .add_message::<crate::boats::BoatHarmed>()
            .init_resource::<CivilCharges>()
            .add_game_systems(Update, charge);
    }
}

fn charge(
    mut people: MessageReader<CivilianLoss>,
    mut cars: MessageReader<CivilianHarmed>,
    mut aircraft: MessageReader<CivilAirHit>,
    mut boats: MessageReader<crate::boats::BoatHarmed>,
    run: Option<ResMut<CareerRun>>,
    plan: Res<Plan>,
    mut charges: ResMut<CivilCharges>,
) {
    let flat = |v: Vec3| Vec2::new(v.x, v.z);
    // A `CivilianLoss` is already the pilot's (the toll filters out the enemy's kills).
    let harms: Vec<CivilHarm> = people
        .read()
        .map(|CivilianLoss(l)| CivilHarm {
            victim: Victim::Person(l.person),
            at: flat(l.at),
            by_pilot: true,
        })
        .chain(cars.read().map(|CivilianHarmed(h)| CivilHarm {
            victim: Victim::Car {
                id: h.car,
                occupied: h.occupied,
            },
            at: flat(h.at),
            by_pilot: h.by_player,
        }))
        .chain(aircraft.read().map(|h| CivilHarm {
            victim: Victim::Aircraft(h.aircraft),
            at: flat(h.at),
            by_pilot: h.by_pilot,
        }))
        // A `BoatHarmed` is already the pilot's harm to a crewed boat (`ships::harm::report`); its crew says so again.
        .chain(boats.read().map(|h| CivilHarm {
            victim: Victim::Boat {
                id: h.boat,
                crewed: h.crew > 0,
            },
            at: h.at,
            by_pilot: true,
        }))
        .collect();
    // Touched only with harm to charge: the terminal and mail pages redraw on any change to the career, and a
    // write every frame rebuilt their buttons under the pointer, so no click on them ever landed.
    if harms.is_empty() {
        return;
    }
    let Some(mut run) = run else { return };
    let run = &mut *run;
    for h in &harms {
        if let Some((base, now)) = charges.0.charge(&mut run.career, &run.data.career, &plan.bases, h) {
            info!(
                "career: harmed a civilian ({:?}); standing with base {} now {now}",
                h.victim,
                base + 1
            );
        }
    }
}
