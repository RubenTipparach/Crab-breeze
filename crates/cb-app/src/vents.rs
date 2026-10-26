//! The boarding's steam and mist (openspec change `boarding-vents`): every vent in the piloted mech's boarding
//! timeline streams from its node (a `VENT_*` empty on the chest or in the cabin, or a leg bone) while
//! `BoardingTimeline::vent_emits` says so, through the effects' one continuous emitter (`fx::FxStream`, recipes
//! `vent_steam` and `vent_mist`). Presentation only: the boarding clock is the simulation's, the vents show it.

use crate::fx::FxStream;
use crate::mech::{Interior, MechRig, MechSim, MechSpec, Piloted};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use bevy::prelude::*;
use cb_core::boarding::VentKind;

/// A vent's stream: stage `stage`'s vent `vent` in the boarding timeline.
#[derive(Component)]
struct VentEmitter {
    stage: usize,
    vent: usize,
}

/// The mech's vents are bound (its streams spawned under their nodes).
#[derive(Component)]
struct VentsBound;

/// Plays the boarding vents.
pub struct VentsPlugin;

impl Plugin for VentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_game_systems(Update, (bind, switch).chain().after(PresentSet::Pose));
    }
}

/// The recipe a vent streams.
fn recipe(kind: VentKind) -> &'static str {
    match kind {
        VentKind::Steam => "vent_steam",
        VentKind::Mist => "vent_mist",
    }
}

/// Does a vent's node pattern name this node? A trailing `*` matches a prefix.
fn matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == pattern,
    }
}

/// Once the piloted mech's rig is bound and its interior hung in its chest (so every vent node is under its root),
/// puts a stream under each vent node, off.
fn bind(
    mut commands: Commands,
    data: Res<Data>,
    mechs: Query<(Entity, &MechSpec, Has<Interior>), (With<Piloted>, With<MechRig>, Without<VentsBound>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok((root, spec, has_interior)) = mechs.single() else {
        return;
    };
    if spec.interior.is_some() && !has_interior {
        return; // the cabin's vents are not under the root yet
    }
    let cfg = &data.0.fx;
    let mut named: Vec<(&str, Entity)> = children
        .iter_descendants(root)
        .filter_map(|e| names.get(e).ok().map(|n| (n.as_str(), e)))
        .collect();
    named.sort_by(|a, b| a.0.cmp(b.0)); // name order, as hatch motions match
    let (mut bound, mut missing) = (0, Vec::new());
    for (si, stage) in data.0.boarding.stages.iter().enumerate() {
        for (vi, vent) in stage.vents.iter().enumerate() {
            let rec = recipe(vent.kind);
            // The recipe streams `stream_per_s` bursts of its layers' counts a second for a vent at rate 1.
            let base = cfg.stream_per_s.get(rec).copied().unwrap_or(1.0)
                * cfg.recipes.get(rec).map_or(1.0, |layers| {
                    layers
                        .iter()
                        .map(|l| match l {
                            cb_core::fx::Layer::Burst { count, .. } => *count as f32,
                            _ => 0.0,
                        })
                        .sum::<f32>()
                        .max(1.0)
                });
            let dir = Vec3::from_array(vent.dir).normalize_or(Vec3::Z);
            let mut any = false;
            for &(_, node) in named.iter().filter(|(n, _)| matches(&vent.node, n)) {
                let stream = commands
                    .spawn((
                        Name::new(format!("vent {}", vent.node)),
                        Transform::IDENTITY,
                        // The stream's emitters are its children: without visibility here they inherit none and
                        // are never drawn (Bevy warning B0004).
                        Visibility::Inherited,
                        FxStream {
                            recipe: rec,
                            dir,
                            on: false,
                            density: vent.rate_per_s / base,
                        },
                        VentEmitter { stage: si, vent: vi },
                    ))
                    .id();
                commands.entity(node).add_child(stream);
                bound += 1;
                any = true;
            }
            if !any {
                missing.push(vent.node.clone());
            }
        }
    }
    if !missing.is_empty() {
        warn!("boarding vents: no node for {missing:?}");
    }
    info!("boarding vents bound: {bound} streams");
    commands.entity(root).insert(VentsBound);
}

/// Each frame, a vent streams while the boarding clock moves through its window, or rests there if it holds.
fn switch(
    data: Res<Data>,
    mech: Query<&MechSim, With<Piloted>>,
    mut vents: Query<(&VentEmitter, &mut FxStream)>,
    mut streaming: Local<usize>,
) {
    let Ok(sim) = mech.single() else {
        return;
    };
    let tl = &data.0.boarding;
    let now = vents
        .iter()
        .filter(|(v, _)| {
            let stage = &tl.stages[v.stage];
            tl.vent_emits(stage, &stage.vents[v.vent], sim.hatch_prev.t_s, sim.hatch.t_s)
        })
        .count();
    if now != *streaming {
        debug!("boarding vents: {now} streaming at {:.2} s", sim.hatch.t_s);
        *streaming = now;
    }
    for (v, mut stream) in &mut vents {
        let Some(stage) = tl.stages.get(v.stage) else {
            continue;
        };
        let Some(vent) = stage.vents.get(v.vent) else {
            continue;
        };
        let on = tl.vent_emits(stage, vent, sim.hatch_prev.t_s, sim.hatch.t_s);
        if stream.on != on {
            stream.on = on;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_with_a_star_matches_a_prefix_and_without_one_only_the_name() {
        assert!(matches("VENT_seam*", "VENT_seam_04"));
        assert!(!matches("VENT_seam*", "VENT_sill_00"));
        assert!(matches("shin_L", "shin_L"));
        assert!(!matches("shin_L", "shin_L_mesh"));
    }

    #[test]
    fn steam_and_mist_stream_their_own_recipes() {
        assert_eq!(recipe(VentKind::Steam), "vent_steam");
        assert_eq!(recipe(VentKind::Mist), "vent_mist");
    }
}
