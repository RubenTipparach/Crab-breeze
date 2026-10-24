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

/// Holds each frame to its slot at `frame_cap_hz` (sim.ron): sleeps to just short of the slot, then
/// spins the last millisecond, since a Windows sleep can overshoot by that much.
pub struct FrameCapPlugin;

#[derive(Resource)]
struct FrameCap {
    period: std::time::Duration,
    next: std::time::Instant,
}

impl Plugin for FrameCapPlugin {
    fn build(&self, app: &mut App) {
        let hz = app.world().resource::<Data>().0.sim.frame_cap_hz;
        if hz <= 0.0 {
            return;
        }
        app.insert_resource(FrameCap {
            period: std::time::Duration::from_secs_f64(1.0 / f64::from(hz)),
            next: std::time::Instant::now(),
        })
        .add_systems(Last, hold_frame);
    }
}

fn hold_frame(mut cap: ResMut<FrameCap>) {
    let margin = std::time::Duration::from_millis(1);
    let now = std::time::Instant::now();
    if cap.next > now + margin {
        std::thread::sleep(cap.next - now - margin);
    }
    while std::time::Instant::now() < cap.next {
        std::hint::spin_loop();
    }
    // A frame that ran long does not bank time for the next ones.
    cap.next = cap.next.max(std::time::Instant::now() - cap.period) + cap.period;
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let hz = app.world().resource::<Data>().0.sim.fixed_step_hz;
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(hz)))
            .configure_sets(FixedUpdate, (SimSet::Input, SimSet::Pilot, SimSet::Mech).chain())
            .configure_sets(Update, (PresentSet::Input, PresentSet::Pose, PresentSet::Camera).chain());
    }
}
