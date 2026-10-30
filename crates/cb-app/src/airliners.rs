//! The modelled airliners in the game (openspec change `airliner-models`): once an airliner's scene is in, its levels
//! of detail get their visibility ranges and its damage zones, moving parts, lamps and cabin fit-out are found by name
//! (`bind`); every frame its gear, flaps, control surfaces, propellers and fans move from its flight and its beacons
//! and strobes flash in their patterns (`pose_airliners`); a zone shot away leaves the aircraft with everything that
//! hangs from it and falls as a piece of the real model that lands and lies on the ground, its seats and fit-out
//! despawned, with debris chunks thrown from the break (`break_airliners`, `fall_parts`); a crash strews every zone so.
//! What moves and how far is each model's manifest (`cb_core::air_traffic::model`), the one description the
//! simulation and the drawing share.

use crate::air_traffic::{AirTraffic, CIVIL_ID_BASE, CivilDraw};
use crate::fx::Fx;
use crate::sim::Data;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::air_traffic::model::LoadedModel;
use cb_core::air_traffic::{Aircraft, Phase};
use std::collections::HashMap;
use std::sync::Arc;

/// A zone of a civil airliner came away (`CivilHits::hit`): the aircraft's civil id, the zones (it and every zone
/// that hangs from it) and where the shot struck.
#[derive(Message, Clone, Debug)]
pub struct AirlinerBroke {
    /// The aircraft's civil id (`CIVIL_ID_BASE` and up).
    pub aircraft: u32,
    /// The zones broken away, in the model's zone order.
    pub zones: Vec<usize>,
    /// Where the shot struck.
    pub at: Vec3,
}

/// A modelled airliner hit the ground: its zones are strewn as its wreck.
#[derive(Message, Clone, Copy, Debug)]
pub struct AirlinerCrashed {
    /// The aircraft's id in the traffic (not the civil id).
    pub aircraft: u32,
    /// Its heading when it hit.
    pub heading_rad: f32,
}

/// An airliner's parts, found by name in its scene.
#[derive(Component)]
pub struct AirlinerParts {
    model: Arc<LoadedModel>,
    /// Each damage zone's node, in the model's zone order (none: missing from the scene).
    zones: Vec<Option<Entity>>,
    /// Each moving part's pivot, its manifest entry's index and its rotation as built.
    rig: Vec<(Entity, usize, Quat)>,
    /// Each lamp's lit meshes and its manifest entry's index.
    lamps: Vec<(Vec<Entity>, usize)>,
    /// The cabin fit-out (seats, floor, bins, galleys) in each zone.
    interior: Vec<Vec<Entity>>,
    /// The zones that have come away.
    gone: Vec<bool>,
}

/// An airliner's moving parts as they stand: the gear's place in its sequence (0 down, 1 up) and the props' and fans'
/// turn.
#[derive(Component, Default)]
pub struct AirlinerPose {
    gear_t: f32,
    turn_rad: f32,
    /// Parked at a stand: gear down, engines stopped.
    parked: bool,
}

/// A piece of an airliner falling free: its velocity, its spin, and the point of it (in its own frame) it lands on
/// with the radius round that point. Lies still once it lands and stops; leaves the world after `LIE_S`.
#[derive(Component)]
pub struct FallingPart {
    vel: Vec3,
    spin: Vec3,
    centre: Vec3,
    radius_m: f32,
    at_rest: bool,
    lain_s: f32,
}

/// How long a piece lies before it leaves the world, seconds.
const LIE_S: f32 = 240.0;

/// The level of detail a node's name says: `<part>.L<n>` (and its glTF primitives, `<part>.L<n>.<i>`).
pub fn lod_of(name: &str) -> Option<u8> {
    let i = name.rfind(".L")?;
    let rest = &name[i + 2..];
    let d = rest.chars().next()?.to_digit(10)?;
    (d <= 2 && rest[1..].chars().next().is_none_or(|c| c == '.')).then_some(d as u8)
}

/// Binds a zoned airliner's scene once it is in: its meshes' visibility ranges by the level each hangs under, its
/// zones, rig, lamps and fit-out. False while the scene is not spawned yet.
#[allow(clippy::too_many_arguments)]
pub fn bind(
    commands: &mut Commands,
    root: Entity,
    model: &Arc<LoadedModel>,
    lod_m: [f32; 2],
    parked: bool,
    children: &Query<&Children>,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
    is_mesh: &Query<(), With<Mesh3d>>,
    transforms: &Query<&Transform>,
) -> bool {
    use bevy::camera::visibility::VisibilityRange;
    let mut by_name: HashMap<String, Entity> = HashMap::new();
    for e in children.iter_descendants(root) {
        if let Ok(n) = names.get(e) {
            by_name.insert(n.as_str().to_string(), e);
        }
    }
    if !by_name.contains_key("ZONE_centre") {
        return false;
    }
    let [a, b] = lod_m;
    let (ma, mb) = (a * 0.1, b * 0.1);
    let range = |lod: u8| match lod {
        0 => VisibilityRange {
            start_margin: 0.0..0.0,
            end_margin: (a - ma)..(a + ma),
            use_aabb: false,
        },
        1 => VisibilityRange {
            start_margin: (a - ma)..(a + ma),
            end_margin: (b - mb)..(b + mb),
            use_aabb: false,
        },
        _ => VisibilityRange {
            start_margin: (b - mb)..(b + mb),
            end_margin: f32::MAX..f32::MAX,
            use_aabb: false,
        },
    };
    // each mesh takes the level of the nearest named ancestor that has one
    for e in children.iter_descendants(root).filter(|e| is_mesh.contains(*e)) {
        let lod = std::iter::once(e)
            .chain(parents.iter_ancestors(e))
            .find_map(|q| names.get(q).ok().and_then(|n| lod_of(n.as_str())));
        if let Some(l) = lod {
            commands.entity(e).insert(range(l));
        }
    }
    let man = &model.manifest;
    let zones: Vec<Option<Entity>> = man.zones.iter().map(|z| by_name.get(&z.node).copied()).collect();
    let zone_of = |e: Entity| -> Option<usize> {
        parents.iter_ancestors(e).find_map(|q| {
            names
                .get(q)
                .ok()
                .and_then(|n| n.as_str().strip_prefix("ZONE_"))
                .and_then(|z| man.zones.iter().position(|y| y.name == z))
        })
    };
    let rig = man
        .rig
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let e = *by_name.get(&r.node)?;
            Some((e, i, transforms.get(e).map_or(Quat::IDENTITY, |t| t.rotation)))
        })
        .collect();
    let lamps = man
        .lamps
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let what = l.node.strip_prefix("LAMP_").unwrap_or(&l.node);
            let meshes = ["L0", "L1"]
                .iter()
                .filter_map(|lv| by_name.get(&format!("light_{what}.{lv}")).copied())
                .collect();
            (meshes, i)
        })
        .collect();
    let mut interior = vec![Vec::new(); man.zones.len()];
    for (n, &e) in &by_name {
        if n.starts_with("int_")
            && lod_of(n).is_some()
            && let Some(z) = zone_of(e)
        {
            interior[z].push(e);
        }
    }
    commands.entity(root).insert((
        AirlinerParts {
            model: model.clone(),
            zones,
            rig,
            lamps,
            interior,
            gone: vec![false; man.zones.len()],
        },
        AirlinerPose {
            gear_t: 0.0,
            turn_rad: 0.0,
            parked,
        },
    ));
    true
}

/// The gear down near the ground (below the kind's `gear_below_m`, or on its wheels), up otherwise.
fn gear_up(a: &Aircraft, ground_m: f32, gear_below_m: f32) -> bool {
    !(a.on_ground() || a.pos.y - ground_m < gear_below_m)
}

/// The flaps' setting by where it is in its flight: full on final and on the ground after landing, a take-off notch
/// rolling and climbing out, clean otherwise.
fn flap_step(a: &Aircraft) -> usize {
    match a.phase {
        Phase::Final | Phase::Rollout { .. } => 2,
        Phase::Roll { .. } | Phase::Climb | Phase::Intercept => 1,
        _ => 0,
    }
}

/// Moves every airliner's parts from its flight and flashes its lamps.
#[allow(clippy::too_many_arguments)]
pub fn pose_airliners(
    time: Res<Time>,
    air: Option<Res<AirTraffic>>,
    data: Res<Data>,
    plan: Res<Plan>,
    mut planes: Query<(&AirlinerParts, &mut AirlinerPose, Option<&CivilDraw>)>,
    mut transforms: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    for (parts, mut pose, draw) in &mut planes {
        let a = draw.and_then(|d| air.as_ref()?.0.get(d.0));
        let man = &parts.model.manifest;
        let spec = a.and_then(|a| data.0.air_traffic.kinds.get(&a.kind));
        let (want_up, flaps, bank, pitch) = match (a, spec) {
            (Some(a), Some(spec)) => {
                let below = spec.model.as_ref().map_or(250.0, |m| m.gear_below_m);
                let ground = plan.terrain.height_at(a.pos.x, a.pos.z);
                (gear_up(a, ground, below), flap_step(a), a.bank_rad, a.pitch_rad)
            }
            _ => (false, 0, 0.0, 0.0),
        };
        let seq = man.gear_sequence_s.max(1.0);
        let target = if want_up { 1.0 } else { 0.0 };
        pose.gear_t += (target - pose.gear_t).clamp(-dt / seq, dt / seq);
        // standing at its stand between flights its engines are off, as a parked one's
        let idle = pose.parked || a.is_some_and(|a| matches!(a.phase, Phase::Stand { .. }));
        if !idle {
            pose.turn_rad = (pose.turn_rad + dt * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU * 1000.0);
        }
        for &(e, i, base) in &parts.rig {
            let r = &man.rig[i];
            let angle = match r.role.as_str() {
                "gear_leg" | "gear_door" => r.angle_deg.to_radians() * r.gear_share(pose.gear_t),
                // a prop or fan at its rpm: `turn_rad` counts one revolution a second
                "prop" | "fan" | "rotor" | "tail_rotor" if !idle => pose.turn_rad * r.rpm / 60.0,
                "aileron" => {
                    let side = if r.node.ends_with("_l") { 1.0 } else { -1.0 };
                    (bank * 1.2).clamp(-1.0, 1.0) * r.range_deg.to_radians() * side
                }
                "elevator" => (pitch * 2.0).clamp(-1.0, 1.0) * r.range_deg.to_radians(),
                "flap" => r.detents_deg.get(flaps).copied().unwrap_or(0.0).to_radians(),
                _ => 0.0,
            };
            if let Ok(mut t) = transforms.get_mut(e) {
                let axis = Vec3::from_array(r.axis).normalize_or(Vec3::X);
                t.rotation = base * Quat::from_axis_angle(axis, angle);
            }
        }
        // the lamps: steady, the beacons once a second, the strobes twice; landing lights with the gear down; each
        // aircraft's flashes offset by its id so a sky of them does not blink as one
        let offset = draw.map_or(0.0, |d| (d.0 % 17) as f32 * 0.137);
        for (meshes, i) in &parts.lamps {
            let l = &man.lamps[*i];
            let lit = match l.pattern.as_str() {
                "beacon" => !idle && man.light_patterns.beacon.lit(now + offset),
                "strobe" => !idle && man.light_patterns.strobe.lit(now + offset),
                "landing" => !idle && pose.gear_t < 0.5,
                _ => true,
            };
            let want = if lit { Visibility::Inherited } else { Visibility::Hidden };
            for e in meshes {
                if let Ok(mut v) = vis.get_mut(*e)
                    && *v != want
                {
                    *v = want;
                }
            }
        }
    }
}

/// Its zone's point and size, from the zone's capsules: the middle of them, and the largest radius.
fn zone_centre(model: &LoadedModel, z: usize) -> (Vec3, f32) {
    let caps = &model.zones[z].capsules;
    if caps.is_empty() {
        return (Vec3::ZERO, 1.0);
    }
    let c = caps.iter().map(|(a, b, _)| (*a + *b) * 0.5).sum::<Vec3>() / caps.len() as f32;
    let r = caps.iter().map(|c| c.2).fold(0.5f32, f32::max);
    (c, r)
}

/// Lets zone `z` of an airliner fall free as a piece of its own: it leaves the aircraft where it is, flying on with
/// `vel` and tumbling.
fn set_free(commands: &mut Commands, parts: &AirlinerParts, z: usize, global: &GlobalTransform, vel: Vec3, spin: Vec3) {
    let Some(e) = parts.zones[z] else { return };
    let (centre, radius_m) = zone_centre(&parts.model, z);
    commands.entity(e).remove::<ChildOf>().insert((
        global.compute_transform(),
        FallingPart {
            vel,
            spin,
            centre,
            radius_m,
            at_rest: false,
            lain_s: 0.0,
        },
    ));
}

/// A small deterministic spread from a seed: three numbers in -1..1.
fn spread(seed: u32) -> Vec3 {
    let mut r = cb_core::rng::SimRng::new(u64::from(seed)).fork("airliner part");
    Vec3::new(r.range_f32(-1.0, 1.0), r.range_f32(-1.0, 1.0), r.range_f32(-1.0, 1.0))
}

/// A zone shot away: it and the zones hanging from it leave the aircraft as one falling piece, their cabin fit-out
/// gone with them (owner, 2026-10-30: "the seats should just despawn when that section of plane is destroyed", for
/// cheaper physics), and the break throws debris chunks (the debris rules': a body near you, dust far away).
#[allow(clippy::too_many_arguments)]
pub fn break_airliners(
    mut commands: Commands,
    mut broke: MessageReader<AirlinerBroke>,
    air: Option<Res<AirTraffic>>,
    mut planes: Query<(&mut AirlinerParts, &CivilDraw)>,
    globals: Query<&GlobalTransform>,
    mut live: ResMut<Live>,
    mut fx: MessageWriter<Fx>,
) {
    for b in broke.read() {
        let id = b.aircraft - CIVIL_ID_BASE;
        let Some((mut parts, _)) = planes.iter_mut().find(|(_, d)| d.0 == id) else {
            continue;
        };
        let vel = air.as_ref().and_then(|a| a.0.get(id)).map_or(Vec3::ZERO, |a| a.vel);
        // skin torn away where the shot struck, too
        for k in 0..2u32 {
            let j = spread(b.aircraft * 71 + k);
            live.new_chunks.push(cb_core::world::Chunk {
                building: b.aircraft,
                centre: b.at + j,
                size: Vec3::new(1.4 + j.x.abs(), 0.2, 1.1 + j.z.abs()),
            });
        }
        for &z in &b.zones {
            for e in std::mem::take(&mut parts.interior[z]) {
                commands.entity(e).despawn();
            }
            parts.gone[z] = true;
        }
        // the top of what came away: a broken zone whose parent did not break with it
        let tops: Vec<usize> = b
            .zones
            .iter()
            .copied()
            .filter(|&z| parts.model.zones[z].parent.is_none_or(|p| !b.zones.contains(&p)))
            .collect();
        for z in tops {
            let Some(g) = parts.zones[z].and_then(|e| globals.get(e).ok()) else {
                continue;
            };
            let (centre, radius) = zone_centre(&parts.model, z);
            let world_centre = g.transform_point(centre);
            let joint = parts.model.zones[z]
                .parent
                .map_or(world_centre, |p| g.transform_point((centre + zone_centre(&parts.model, p).0) * 0.5));
            let out = (world_centre - joint).normalize_or(Vec3::Y);
            let jitter = spread(b.aircraft * 31 + z as u32);
            set_free(&mut commands, &parts, z, g, vel + out * 6.0 + Vec3::Y * 3.0, jitter * 1.4);
            fx.write(Fx::new("explosion_air", joint));
            // torn skin and frame thrown from the break, about 2 m pieces, as the boats break
            for k in 0..4u32 {
                let j = spread(b.aircraft * 97 + z as u32 * 13 + k);
                live.new_chunks.push(cb_core::world::Chunk {
                    building: b.aircraft,
                    centre: joint + j * radius.min(2.5),
                    size: Vec3::new(1.6 + j.x.abs(), 0.25, 1.2 + j.z.abs()),
                });
            }
        }
    }
}

/// A modelled airliner hit the ground: every zone still on it comes apart and is strewn as its wreck, fit-out gone,
/// and the aircraft's drawing leaves the traffic's hands (it is a wreck now, not a flight).
pub fn crash_airliners(
    mut commands: Commands,
    mut crashed: MessageReader<AirlinerCrashed>,
    mut planes: Query<(Entity, &mut AirlinerParts, &CivilDraw)>,
    globals: Query<&GlobalTransform>,
) {
    for c in crashed.read() {
        let Some((root, mut parts, _)) = planes.iter_mut().find(|(_, _, d)| d.0 == c.aircraft) else {
            continue;
        };
        let fwd = cb_core::units::forward(c.heading_rad);
        for z in 0..parts.zones.len() {
            for e in std::mem::take(&mut parts.interior[z]) {
                commands.entity(e).despawn();
            }
            if parts.gone[z] {
                continue;
            }
            parts.gone[z] = true;
            let Some(g) = parts.zones[z].and_then(|e| globals.get(e).ok()) else {
                continue;
            };
            let j = spread(c.aircraft * 53 + z as u32);
            let vel = fwd * (8.0 + j.x * 4.0) + Vec3::new(j.y, 0.5 + j.z.abs(), j.z) * 4.0;
            set_free(&mut commands, &parts, z, g, vel, j * 0.8);
        }
        commands.entity(root).remove::<CivilDraw>().insert(Wrecked(0.0));
    }
}

/// A crashed airliner's empty drawing, cleared away once its pieces have left it.
#[derive(Component)]
pub struct Wrecked(f32);

/// Falls every free piece under gravity, tumbling, until it lands on the ground by its lowest point, bounces a little,
/// rubs to a stop and lies (owner, 2026-10-30: "I expected the chunks to fall to the ground instead of fall through
/// it"); a piece that has lain `LIE_S` leaves the world.
pub fn fall_parts(
    mut commands: Commands,
    time: Res<Time>,
    plan: Res<Plan>,
    mut parts: Query<(Entity, &mut FallingPart, &mut Transform)>,
    mut wrecks: Query<(Entity, &mut Wrecked)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (e, mut p, mut t) in &mut parts {
        if p.at_rest {
            p.lain_s += dt;
            if p.lain_s > LIE_S {
                commands.entity(e).despawn();
            }
            continue;
        }
        p.vel.y -= 9.81 * dt;
        let pivot = t.transform_point(p.centre);
        if p.spin.length_squared() > 1e-8 {
            t.rotate_around(pivot, Quat::from_scaled_axis(p.spin * dt));
        }
        t.translation += p.vel * dt;
        let c = t.transform_point(p.centre);
        let ground = plan.terrain.height_at(c.x, c.z);
        let low = c.y - p.radius_m;
        if low < ground {
            t.translation.y += ground - low;
            if p.vel.y < 0.0 {
                p.vel.y *= -0.2;
            }
            p.vel.x *= 0.7;
            p.vel.z *= 0.7;
            p.spin *= 0.6;
            if p.vel.y.abs() < 0.8 && p.vel.x * p.vel.x + p.vel.z * p.vel.z < 0.5 {
                p.vel = Vec3::ZERO;
                p.spin = Vec3::ZERO;
                p.at_rest = true;
            }
        }
    }
    for (e, mut w) in &mut wrecks {
        w.0 += dt;
        if w.0 > 1.0 {
            commands.entity(e).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_names_its_level_and_its_primitives_inherit_it() {
        assert_eq!(lod_of("fuselage_nose.L0"), Some(0));
        assert_eq!(lod_of("wing_l_outer.L2"), Some(2));
        assert_eq!(lod_of("int_seats_centre.L0.3"), Some(0));
        assert_eq!(lod_of("ZONE_centre"), None);
        assert_eq!(lod_of("RIG_gear_nose"), None);
        assert_eq!(lod_of("tail.Lx"), None);
    }

    #[test]
    fn the_flaps_come_down_for_landing_and_the_gear_comes_up_away_from_the_ground() {
        let root = crate::cli::repo_root();
        let mut cfg: cb_core::air_traffic::AirTrafficConfig = cb_core::data::load_ron(&root.join("data/world/air_traffic.ron")).unwrap();
        cfg.load_models(&root.join("assets")).unwrap();
        let spec = &cfg.kinds["airliner"];
        let mut a = Aircraft::new(
            1,
            "airliner",
            spec,
            Vec3::new(0.0, 1000.0, 0.0),
            0.0,
            100.0,
            Phase::Enroute,
            cb_core::air_traffic::Flight::Orbit {
                city: 0,
                centre: Vec2::ZERO,
                radius_m: 1.0,
                dir: 1.0,
            },
        );
        assert!(gear_up(&a, 0.0, 250.0));
        assert_eq!(flap_step(&a), 0);
        a.phase = Phase::Final;
        a.pos.y = 120.0;
        assert!(!gear_up(&a, 0.0, 250.0), "gear down on final");
        assert_eq!(flap_step(&a), 2);
    }
}
