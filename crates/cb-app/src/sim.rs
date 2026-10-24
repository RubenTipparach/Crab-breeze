//! The simulation's schedule: the fixed step rate from data, and the named phases every
//! simulation system runs in (CLAUDE.md section 6.2), configured once here.

use bevy::prelude::*;
use cb_core::GameData;

/// All loaded data, as a resource.
#[derive(Resource)]
pub struct Data(pub GameData);

/// Phases of a fixed step, in order.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Turn sampled input into this step's actions.
    Input,
    /// The world grows what the bodies in it will touch this step (trees near them).
    World,
    /// The pilot moves, boards or climbs out.
    Pilot,
    /// The mech moves.
    Mech,
}

/// Phases of a rendered frame, in order.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentSet {
    /// Sample devices.
    Input,
    /// Pose bodies from interpolated simulation state.
    Pose,
    /// Place the one camera.
    Camera,
}

/// Fixed rate and set order.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let hz = app.world().resource::<Data>().0.sim.fixed_step_hz;
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(hz)))
            .configure_sets(FixedUpdate, (SimSet::Input, SimSet::World, SimSet::Pilot, SimSet::Mech).chain())
            .configure_sets(Update, (PresentSet::Input, PresentSet::Pose, PresentSet::Camera).chain());
    }
}
