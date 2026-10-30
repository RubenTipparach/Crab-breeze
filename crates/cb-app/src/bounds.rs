//! The offshore limit in the game (openspec change `offshore-limit`): where the pilot is, every fixed step, read
//! against `cb_core::bounds`, wherever they are (on foot, driving, or in the mech). Within `warn_m` of the limit a
//! banner shows how far is left and the alarm sounds; past it for the grace the running mission ends (free roam's
//! mission too, owner: "Even in free roam") and `OUT OF BOUNDS` shows; then the pilot comes back on foot in the nearest
//! city beside a fresh mech, brought as the mech swap brings one, to walk to and board (the first-person law: boarding
//! is walked, never cut to). Look input is the player's throughout: only the pilot's place changes.

use crate::audio::Sound;
use crate::mech::{MechSim, Piloted, SwapMech};
use crate::pilot::PilotSim;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::bounds::{Verdict, Watch};
use cb_core::pilot::{PilotMode, PilotState};

/// The watch and what it said this step.
#[derive(Resource, Default)]
pub struct Offshore {
    /// The rule's state.
    pub watch: Watch,
    /// What it said this step.
    pub verdict: Option<Verdict>,
    /// Seconds until the pilot comes back, once failed.
    pub respawn_in_s: f32,
    /// Seconds until the alarm sounds again while warning.
    alarm_in_s: f32,
    /// Where a fresh mech lands near a pilot on foot (`data/career/air_drop.ron`), as the mech swap's.
    drop: Option<cb_core::air_drop::DropConfig>,
}

/// The banner over the view.
#[derive(Component)]
struct Banner;

/// Watches the offshore limit.
pub struct BoundsPlugin;

impl Plugin for BoundsPlugin {
    fn build(&self, app: &mut App) {
        let drop = cb_core::data::load_ron(&crate::cli::repo_root().join("data/career/air_drop.ron"))
            .map_err(|e| warn!("bounds: no drop sites, the mech comes back where it stood: {e}"))
            .ok();
        app.insert_resource(Offshore { drop, ..default() })
            .add_world_spawn(spawn_banner)
            .add_game_systems(FixedUpdate, watch.after(SimSet::Mech))
            .add_game_systems(Update, show_banner);
    }
}

/// Where the pilot is (x, z): the mech they sit in, the car they drive, or their feet.
fn pilot_xz(pilot: &PilotState, mech: Option<&MechSim>, car: Option<Vec3>) -> Vec2 {
    let p = match pilot.mode {
        PilotMode::Seated | PilotMode::Boarding(..) | PilotMode::Disembarking(..) => mech.map(|m| m.curr.pos),
        _ if pilot.mode.in_car() => car,
        _ => None,
    }
    .unwrap_or(pilot.feet);
    Vec2::new(p.x, p.z)
}

#[allow(clippy::too_many_arguments)]
fn watch(
    mut commands: Commands,
    mut off: ResMut<Offshore>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time<Fixed>>,
    mut pilot: ResMut<PilotSim>,
    mech: Query<&MechSim, With<Piloted>>,
    traffic: Option<ResMut<crate::traffic::TrafficSim>>,
    free: Option<ResMut<crate::missions::FreeRoamRun>>,
    run: Option<ResMut<crate::defence::DefenceRun>>,
    mut sounds: MessageWriter<Sound>,
) {
    let cfg = &data.0.world.bounds;
    let dt = time.delta_secs();
    let car = traffic.as_ref().and_then(|t| t.traffic.driven().map(|c| c.pos));
    let at = pilot_xz(&pilot.curr, mech.iter().next(), car);
    let left = cb_core::bounds::left_m(plan.terrain.shape(), data.0.world.world_size_m, at, cfg);
    let verdict = off.watch.step(left, dt, cfg);
    off.verdict = Some(verdict);
    match verdict {
        Verdict::Warn(_) => {
            off.alarm_in_s -= dt;
            if off.alarm_in_s <= 0.0 {
                off.alarm_in_s = 2.0;
                let seated = pilot.curr.mode == PilotMode::Seated;
                sounds.write(if seated {
                    Sound::speaker("alarm_warning")
                } else {
                    Sound::at("alarm_warning", Vec3::new(at.x, pilot.curr.feet.y + 1.6, at.y))
                });
            }
        }
        Verdict::Fail => {
            info!("BOUNDS out of bounds at ({:.0}, {:.0}): the mission fails", at.x, at.y);
            if let Some(mut f) = free {
                f.stop();
            }
            if let Some(mut r) = run {
                r.state.fail(cb_core::mission::Loss::OutOfBounds);
            }
            off.respawn_in_s = cfg.respawn_after_s;
            sounds.write(Sound::speaker("alarm_warning"));
        }
        Verdict::Failed => {
            off.respawn_in_s -= dt;
            if off.respawn_in_s > 0.0 {
                return;
            }
            // Back on foot in the nearest city's square, a fresh mech dropped beside them (the swap's drop search).
            let Some(city) = cb_core::bounds::respawn_city(&plan.cities, at) else {
                return;
            };
            if let Some(mut t) = traffic
                && let Some(id) = t.traffic.driven().map(|c| c.id)
            {
                t.traffic.leave(id);
            }
            let c = city.centre;
            let feet = Vec3::new(c.x, plan.terrain.height_at(c.x, c.y), c.y);
            let state = PilotState::standing(feet, pilot.curr.yaw_rad);
            pilot.prev = state.clone();
            pilot.curr = state;
            commands.insert_resource(SwapMech { drop: off.drop.clone() });
            off.watch.respawned();
            info!("BOUNDS respawned in city {} at ({:.0}, {:.0})", city.id, c.x, c.y);
        }
        Verdict::Clear => off.alarm_in_s = 0.0,
    }
}

fn spawn_banner(mut commands: Commands) {
    commands.spawn((
        Banner,
        Text::new(""),
        TextFont::from_font_size(30.0),
        TextColor(Color::srgb(1.0, 0.3, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(14.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextLayout::justify(Justify::Center),
        Visibility::Hidden,
    ));
}

fn show_banner(off: Res<Offshore>, mut banner: Query<(&mut Text, &mut Visibility), With<Banner>>) {
    let (line, show) = match off.verdict {
        Some(Verdict::Warn(left)) if left >= 0.0 => (format!("RETURN TO COAST  {:.1} KM", left / 1000.0), true),
        Some(Verdict::Warn(_)) => ("RETURN TO COAST  0.0 KM".to_string(), true),
        Some(Verdict::Fail | Verdict::Failed) => ("OUT OF BOUNDS\nMISSION FAILED".to_string(), true),
        _ => (String::new(), false),
    };
    for (mut t, mut v) in &mut banner {
        if t.0 != line {
            t.0 = line.clone();
        }
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
}
