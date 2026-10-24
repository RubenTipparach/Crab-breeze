//! The mech in the game (`mech-rig` design sections 1, 5 and 6; `mech-boarding`): spawns the
//! stand-in from data or a glb named in data, binds the contract's bones and sockets by name (the
//! same code for both), steps its locomotion and boarding clock in the fixed step, and poses its
//! bones and hatch nodes from the interpolated state.

use crate::blast::Explosion;
use crate::fx::{Burst, Debris, FxKind, Ripple};
use crate::input::StepActions;
use crate::pilot::{PilotCamera, PilotEvents, PilotSim};
use crate::sim::{Data, PresentSet, SimSet};
use crate::world::{Live, Plan};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::boarding::{Hatch, HatchTarget};
use cb_core::input::DebugCommand;
use cb_core::mech::rig::{BONES, SOCKETS};
use cb_core::mech::{Damage, Hit, LocomotionMode};
use cb_core::mech::{LegRest, MechControls, MechState, RigRest, pose};
use cb_core::pilot::{MechAccess, PilotEvent, PilotMode};
use cb_core::world::{Ground, Scene};
use std::collections::HashMap;

/// The mech's root: on the ground, turned to its heading.
#[derive(Component)]
pub struct MechRoot {
    /// The child turned 180 degrees about y that holds the model in glTF space.
    model: Entity,
}

/// Simulation state at the last two fixed steps, for interpolation.
#[derive(Component, Clone)]
pub struct MechSim {
    /// The step before.
    pub prev: MechState,
    /// The latest step.
    pub curr: MechState,
    /// Boarding clock at the step before.
    pub hatch_prev: Hatch,
    /// Boarding clock at the latest step.
    pub hatch: Hatch,
    /// Damage by part.
    pub damage: Damage,
    /// Parts lost this step whose bones the drawing must cut off.
    pub cuts: Vec<usize>,
}

/// A hatch node found by a timeline motion's pattern.
#[derive(Clone, Copy)]
struct HatchNode {
    entity: Entity,
    rest: Transform,
    stage: usize,
    motion: usize,
    index: usize,
}

/// One bound bone: its entity and rest pose.
#[derive(Clone, Copy)]
struct BoneBind {
    entity: Entity,
    rest_local: Transform,
    rest_model_rot: Quat,
    parent_rest_model_rot: Quat,
}

/// A bound rig: every contract bone and socket found by name.
#[derive(Component)]
pub struct MechRig {
    bones: Vec<BoneBind>,
    sockets: HashMap<&'static str, Entity>,
    rest: RigRest,
    hatch_nodes: Vec<HatchNode>,
    /// Every named node under the model, for cutting parts off by bone name.
    named: HashMap<String, Entity>,
    /// Contract bones no longer driven (shot off), by `BONES` index.
    cut: Vec<bool>,
}

/// Smoke and fire rising from where a part was shot off.
#[derive(Component)]
struct Burning {
    until_s: f32,
}

/// The model lacks contract bones; it is shown but not driven.
#[derive(Component)]
struct RigRefused;

/// The mech's boarding sockets this fixed step, for the pilot.
#[derive(Resource, Default)]
pub struct MechAccessNow(pub Option<MechAccess>);

/// Spawns, binds, steps and poses the mech.
pub struct MechPlugin;

impl Plugin for MechPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MechAccessNow>()
            .add_systems(Startup, spawn_mech)
            .add_systems(FixedUpdate, (publish_access.in_set(SimSet::Input), step_mech.in_set(SimSet::Mech)))
            .add_systems(Update, (bind_rig, cut_parts, drive_mech, burn).chain().in_set(PresentSet::Pose));
    }
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::from_array(a)
}

fn spawn_mech(
    mut commands: Commands,
    data: Res<Data>,
    world: Res<Plan>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cfg = &data.0.mech;
    let start = world.start;
    let ground = world.terrain.height_at(start.mech_xz.x, start.mech_xz.y);
    let state = MechState::standing(
        Vec3::new(start.mech_xz.x, ground, start.mech_xz.y),
        start.mech_heading_rad,
        &cfg.gait,
    );
    let model = commands
        .spawn((
            Name::new("mech model (glTF space)"),
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            Visibility::default(),
        ))
        .id();
    commands
        .spawn((
            Name::new(format!("mech {}", cfg.name)),
            MechRoot { model },
            MechSim {
                prev: state,
                curr: state,
                hatch_prev: Hatch::default(),
                hatch: Hatch::default(),
                damage: Damage::new(&cfg.parts),
                cuts: Vec::new(),
            },
            Transform::from_translation(state.pos).with_rotation(Quat::from_rotation_y(state.heading_rad)),
            Visibility::default(),
        ))
        .add_child(model);

    if let Some(path) = &cfg.model {
        info!("mech model: {path}");
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
        commands.spawn((WorldAssetRoot(scene), ChildOf(model)));
        return;
    }

    // The stand-in: one entity per contract node, then boxes on the bones.
    let mut nodes: HashMap<&str, Entity> = HashMap::new();
    for n in &cfg.nodes {
        let e = commands
            .spawn((
                Name::new(n.name.clone()),
                Transform::from_translation(v3(n.offset_m)),
                Visibility::default(),
            ))
            .id();
        nodes.insert(n.name.as_str(), e);
    }
    for n in &cfg.nodes {
        let parent = if n.parent.is_empty() { model } else { nodes[n.parent.as_str()] };
        commands.entity(nodes[n.name.as_str()]).insert(ChildOf(parent));
    }
    let tones = [
        (0.40, 0.43, 0.36, 0.5),
        (0.18, 0.19, 0.20, 0.6),
        (0.30, 0.30, 0.32, 0.4),
        (0.12, 0.12, 0.13, 0.5),
        (0.50, 0.60, 0.65, 0.1),
        (0.85, 0.45, 0.10, 0.5),
    ]
    .map(|(r, g, b, rough)| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: rough,
            metallic: 0.3,
            ..default()
        })
    });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for b in &cfg.boxes {
        let transform = match b.along_m.map(v3) {
            Some(along) => Transform::from_translation(along * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, along.normalize_or(Vec3::Y)))
                .with_scale(Vec3::new(b.size_m[0], along.length(), b.size_m[2])),
            None => Transform::from_translation(v3(b.centre_m)).with_scale(v3(b.size_m)),
        };
        let tone = tones[(b.tone as usize).min(tones.len() - 1)].clone();
        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(tone),
            transform,
            ChildOf(nodes[b.bone.as_str()]),
        ));
    }
}

/// Composes local transforms from `e` up to (not including) `model`.
fn model_space(e: Entity, model: Entity, transforms: &Query<&Transform>, parents: &Query<&ChildOf>) -> Transform {
    let mut acc = transforms.get(e).copied().unwrap_or_default();
    let mut at = e;
    while let Ok(p) = parents.get(at) {
        at = p.parent();
        if at == model {
            break;
        }
        acc = transforms.get(at).copied().unwrap_or_default().mul_transform(acc);
    }
    acc
}

fn bind_rig(
    mut commands: Commands,
    roots: Query<(Entity, &MechRoot), (Without<MechRig>, Without<RigRefused>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    parents: Query<&ChildOf>,
    data: Res<Data>,
) {
    for (root, mech) in &roots {
        let mut found: HashMap<&'static str, Entity> = HashMap::new();
        let mut named: Vec<(String, Entity)> = Vec::new();
        for e in children.iter_descendants(mech.model) {
            let Ok(name) = names.get(e) else { continue };
            named.push((name.as_str().to_string(), e));
            if let Some(spec) = BONES.iter().chain(SOCKETS.iter()).find(|s| s.name == name.as_str()) {
                found.entry(spec.name).or_insert(e);
            }
        }
        // Name order, so a pattern's stagger runs the same way every time.
        named.sort_by(|a, b| a.0.cmp(&b.0));
        if !found.contains_key("root") {
            continue; // The model has not spawned yet.
        }
        let missing: Vec<&str> = BONES
            .iter()
            .chain(SOCKETS.iter())
            .map(|s| s.name)
            .filter(|n| !found.contains_key(n))
            .collect();
        if !missing.is_empty() {
            let what = data.0.mech.model.clone().unwrap_or_else(|| "the stand-in".into());
            error!("mech rig: {what} is missing contract nodes {missing:?} (docs/design/mech-rig.md); not driving it");
            commands.entity(root).insert(RigRefused);
            continue;
        }
        let bones: Vec<BoneBind> = BONES
            .iter()
            .map(|spec| {
                let e = found[spec.name];
                let parent_rot = parents
                    .get(e)
                    .ok()
                    .filter(|p| p.parent() != mech.model)
                    .map(|p| model_space(p.parent(), mech.model, &transforms, &parents).rotation)
                    .unwrap_or(Quat::IDENTITY);
                BoneBind {
                    entity: e,
                    rest_local: transforms.get(e).copied().unwrap_or_default(),
                    rest_model_rot: model_space(e, mech.model, &transforms, &parents).rotation,
                    parent_rest_model_rot: parent_rot,
                }
            })
            .collect();
        let at = |n: &str| {
            model_space(
                found[BONES.iter().find(|s| s.name == n).map(|s| s.name).unwrap_or("root")],
                mech.model,
                &transforms,
                &parents,
            )
            .translation
        };
        let leg = |s: &str| LegRest {
            hip: at(&format!("thigh_{s}")),
            knee: at(&format!("shin_{s}")),
            hock: at(&format!("hock_{s}")),
            ankle: at(&format!("foot_{s}")),
        };
        let sockets = SOCKETS.iter().map(|s| (s.name, found[s.name])).collect();
        let mut hatch_nodes = Vec::new();
        for (si, stage) in data.0.boarding.stages.iter().enumerate() {
            for (mi, m) in stage.motions.iter().enumerate() {
                let matches = |n: &str| match m.node.strip_suffix('*') {
                    Some(prefix) => n.starts_with(prefix),
                    None => n == m.node,
                };
                let hits: Vec<Entity> = named.iter().filter(|(n, _)| matches(n)).map(|(_, e)| *e).collect();
                if hits.is_empty() {
                    warn!(
                        "boarding: stage `{}` moves `{}`, which the model does not have yet",
                        stage.name, m.node
                    );
                }
                for (index, e) in hits.into_iter().enumerate() {
                    hatch_nodes.push(HatchNode {
                        entity: e,
                        rest: transforms.get(e).copied().unwrap_or_default(),
                        stage: si,
                        motion: mi,
                        index,
                    });
                }
            }
        }
        let rest = RigRest {
            pelvis: at("pelvis"),
            legs: [leg("L"), leg("R")],
        };
        info!(
            "mech rig bound: {} bones, {} sockets, {} hatch nodes",
            bones.len(),
            SOCKETS.len(),
            hatch_nodes.len()
        );
        let named = named.into_iter().collect();
        commands.entity(root).insert(MechRig {
            cut: vec![false; bones.len()],
            bones,
            sockets,
            rest,
            hatch_nodes,
            named,
        });
    }
}

/// Lerps between two fixed steps for drawing.
fn lerp_state(a: &MechState, b: &MechState, t: f32) -> MechState {
    use std::f32::consts::{PI, TAU};
    let ang = |x: f32, y: f32| x + ((y - x + PI).rem_euclid(TAU) - PI) * t;
    MechState {
        pos: a.pos.lerp(b.pos, t),
        heading_rad: ang(a.heading_rad, b.heading_rad),
        twist_rad: a.twist_rad + (b.twist_rad - a.twist_rad) * t,
        pitch_rad: a.pitch_rad + (b.pitch_rad - a.pitch_rad) * t,
        gait_cycles: a.gait_cycles + (b.gait_cycles - a.gait_cycles) * f64::from(t),
        gait_swing: a.gait_swing + (b.gait_swing - a.gait_swing) * t,
        stride_m: a.stride_m + (b.stride_m - a.stride_m) * t,
        ..*b
    }
}

fn drive_mech(
    fixed: Res<Time<Fixed>>,
    data: Res<Data>,
    mut roots: Query<(Entity, &MechSim, Option<&MechRig>), With<MechRoot>>,
    mut transforms: Query<&mut Transform>,
) {
    let alpha = fixed.overstep_fraction();
    for (root, sim, rig) in &mut roots {
        let s = lerp_state(&sim.prev, &sim.curr, alpha);
        let tl = &data.0.boarding;
        let t = sim.hatch_prev.t_s + (sim.hatch.t_s - sim.hatch_prev.t_s) * alpha;
        if let Ok(mut t) = transforms.get_mut(root) {
            t.translation = s.pos;
            t.rotation = Quat::from_rotation_y(s.heading_rad);
        }
        let Some(rig) = rig else { continue };
        // No legs left: it sits (mech-damage design section 2).
        let sit_amount = if s.legs_lost == [true, true] { 1.0 } else { tl.sit_amount(t) };
        let p = pose(&s, &data.0.mech.gait, &rig.rest, &tl.sit, sit_amount);
        for n in &rig.hatch_nodes {
            let stage = &tl.stages[n.stage];
            let m = &stage.motions[n.motion];
            let k = tl.motion_amount(stage, m, n.index, t);
            if let Ok(mut tr) = transforms.get_mut(n.entity) {
                let axis = Vec3::from_array(m.axis).normalize_or(Vec3::X);
                tr.rotation = n.rest.rotation * Quat::from_axis_angle(axis, m.angle_deg.to_radians() * k);
                tr.translation = n.rest.translation + n.rest.rotation * Vec3::from_array(m.translate_m) * k;
            }
        }
        let mut d = [Quat::IDENTITY; BONES.len()];
        let rotations = p.rotations();
        for (i, spec) in BONES.iter().enumerate() {
            let parent = spec.parent.and_then(|pn| BONES.iter().position(|b| b.name == pn));
            let inherited = parent.map(|pi| d[pi]).unwrap_or(Quat::IDENTITY);
            let own = rotations.iter().find(|(n, _)| *n == spec.name).map(|(_, q)| *q);
            d[i] = own.unwrap_or(inherited);
            let bind = &rig.bones[i];
            if rig.cut[i] {
                continue;
            }
            let Ok(mut t) = transforms.get_mut(bind.entity) else { continue };
            let d_parent = inherited;
            t.rotation = (d_parent * bind.parent_rest_model_rot).inverse() * d[i] * bind.rest_model_rot;
            t.translation = bind.rest_local.translation;
            if spec.name == "pelvis" {
                t.translation += (d_parent * bind.parent_rest_model_rot).inverse() * p.pelvis_offset;
            }
        }
    }
}

fn publish_access(
    roots: Query<(&MechSim, &MechRig), With<MechRoot>>,
    globals: Query<&GlobalTransform>,
    data: Res<Data>,
    mut out: ResMut<MechAccessNow>,
) {
    out.0 = roots.iter().next().and_then(|(sim, rig)| {
        let at = |n: &str| globals.get(rig.sockets[n]).ok().map(|g| g.translation());
        Some(MechAccess {
            ladder_base: at("LADDER_base")?,
            hatch_entry: at("HATCH_entry")?,
            eye_pilot: at("EYE_pilot")?,
            heading_rad: sim.curr.heading_rad,
            stopped: sim.curr.is_stopped(&data.0.mech.locomotion),
            hatch_open: sim.hatch.is_open(&data.0.boarding),
        })
    });
}

/// Steps the mech: boarding clock, test commands, locomotion on the live ground, trees in its way,
/// and the effects of its feet and jets.
pub fn step_mech(
    mut roots: Query<(&mut MechSim, Option<&MechRig>), With<MechRoot>>,
    actions: Res<StepActions>,
    pilot: Res<PilotSim>,
    mut events: ResMut<PilotEvents>,
    access: Res<MechAccessNow>,
    data: Res<Data>,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    time: Res<Time<Fixed>>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    globals: Query<&GlobalTransform>,
    mut explosions: MessageWriter<Explosion>,
    mut bursts: MessageWriter<Burst>,
    mut ripples: MessageWriter<Ripple>,
) {
    let cfg = &data.0.mech;
    let tl = &data.0.boarding;
    let seated = pilot.curr.mode == PilotMode::Seated;
    let live = &mut *live;
    for (mut sim, rig) in &mut roots {
        sim.prev = sim.curr;
        sim.hatch_prev = sim.hatch;
        // The boarding sequence follows the pilot (mech-boarding design section 4).
        for e in &events.0 {
            match e {
                PilotEvent::HatchWanted => sim.hatch.target = HatchTarget::Open,
                PilotEvent::Seated => sim.hatch.target = HatchTarget::SealedSitting,
                _ => {}
            }
        }
        let clear = access
            .0
            .as_ref()
            .is_some_and(|a| pilot.curr.mode == PilotMode::OnFoot && pilot.curr.distance_to_stair(a) > tl.clear_m);
        if sim.hatch.target == HatchTarget::Open && clear && pilot.curr.waiting.is_none() {
            sim.hatch.target = HatchTarget::SealedSitting;
        }
        if seated && sim.hatch.is_sealed_sitting(tl) && (actions.throttle_axis > 0.0 || actions.lift_axis > 0.0) {
            sim.hatch.target = HatchTarget::Standing;
        }
        sim.hatch.step(tl, time.delta_secs());

        // Test commands until weapons exist.
        match actions.debug {
            Some(DebugCommand::Hit(kind)) => {
                if let Some(part) = cfg.parts.iter().position(|p| p.kind == kind) {
                    let sim = &mut *sim;
                    if let Some(lost) = sim.damage.resolve(
                        &cfg.parts,
                        Hit {
                            part,
                            penetration_mm: 500.0,
                            damage: 1000.0,
                        },
                    ) {
                        sim.cuts.push(lost);
                        sim.curr.legs_lost = sim.damage.legs(&cfg.parts).map(|on| !on);
                        info!("damage: {:?} lost", cfg.parts[lost].kind);
                    }
                }
            }
            Some(DebugCommand::Shell) => {
                if let Some(at) = camera.iter().next().and_then(|c| aim_point(c, &plan, &live.dug)) {
                    explosions.write(Explosion { at, energy_mj: 6.0 });
                }
            }
            None => {}
        }

        let controls = if seated && sim.hatch.can_walk() {
            MechControls::from_actions(&actions)
        } else {
            MechControls::default()
        };
        let ground = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
        };
        let was_airborne = sim.curr.airborne();
        sim.curr.step(&controls, cfg, &ground, time.delta_secs());

        // Trees in the way snap or stop it.
        if !sim.curr.airborne() || sim.curr.pos.y - ground.height_at(sim.curr.pos.x, sim.curr.pos.z) < 15.0 {
            let fwd = cb_core::mech::forward(sim.curr.heading_rad) * sim.curr.speed_m_s;
            let bump = live.forest.bump(
                Vec2::new(sim.curr.pos.x, sim.curr.pos.z),
                cfg.locomotion.body_radius_m,
                Vec2::new(fwd.x, fwd.z),
                cfg.locomotion.mass_kg,
            );
            if bump.blocked {
                sim.curr.pos.x = bump.pos.x;
                sim.curr.pos.z = bump.pos.y;
                sim.curr.speed_m_s *= 0.5;
            }
            sim.curr.speed_m_s *= 1.0 - 0.15 * (bump.broke.len() as f32).min(1.0);
            live.moved_trees.extend(bump.broke);
        }

        // Footfalls: spray and ripples in water, dust on dry ground; a heavy landing throws more.
        let Some(rig) = rig else { continue };
        let landed = was_airborne && !sim.curr.airborne();
        for (side, offset) in [(0usize, 0.0f64), (1, 0.5)] {
            let before = (sim.prev.gait_cycles - offset).floor();
            let after = (sim.curr.gait_cycles - offset).floor();
            if (before == after || sim.curr.gait_swing < 0.3 || sim.curr.legs_lost[side]) && !landed {
                continue;
            }
            let ankle = rig.rest.legs[side].ankle;
            let turn = Quat::from_rotation_y(sim.curr.heading_rad + std::f32::consts::PI);
            let foot = sim.curr.pos + turn * Vec3::new(ankle.x, 0.0, ankle.z);
            let heavy = if landed { 3 } else { 1 };
            match ground.water_at(foot.x, foot.z) {
                Some(w) => {
                    let at = Vec3::new(foot.x, w.surface_m, foot.z);
                    bursts.write(Burst {
                        kind: FxKind::Spray,
                        at,
                        dir: Vec3::Y,
                        count: 70 * heavy,
                        speed_m_s: 7.0 + w.depth_m.min(4.0),
                        spread: 0.55,
                        size_m: 1.1,
                        life_s: 1.6,
                    });
                    ripples.write(Ripple {
                        at,
                        radius_m: 9.0 + 4.0 * heavy as f32,
                    });
                }
                None => {
                    bursts.write(Burst {
                        kind: FxKind::Dust,
                        at: foot,
                        dir: Vec3::Y,
                        count: 10 * heavy,
                        speed_m_s: 2.0 * heavy as f32,
                        spread: 1.0,
                        size_m: 1.4,
                        life_s: 1.8,
                    });
                }
            }
        }
        // Jet flame while lifting.
        if sim.curr.mode == LocomotionMode::Jet && controls.lift_axis > 0.0 && !sim.curr.jet_overheated {
            for jet in ["JET_L", "JET_R"] {
                if let Some(g) = rig.sockets.get(jet).and_then(|e| globals.get(*e).ok()) {
                    bursts.write(Burst {
                        kind: FxKind::Flame,
                        at: g.translation(),
                        dir: Vec3::NEG_Y,
                        count: 6,
                        speed_m_s: 30.0,
                        spread: 0.15,
                        size_m: 0.9,
                        life_s: 0.25,
                    });
                    bursts.write(Burst {
                        kind: FxKind::Smoke,
                        at: g.translation() - Vec3::Y * 3.0,
                        dir: Vec3::NEG_Y,
                        count: 1,
                        speed_m_s: 8.0,
                        spread: 0.4,
                        size_m: 2.5,
                        life_s: 2.0,
                    });
                }
            }
        }
    }
    events.0.clear();
}

/// Where the pilot's view meets the ground, within 1.5 km.
fn aim_point(camera: &GlobalTransform, plan: &cb_core::world::WorldPlan, dug: &cb_core::world::Deformation) -> Option<Vec2> {
    let ground = Scene { plan, dug, trees: None };
    let (o, d) = (camera.translation(), camera.forward().as_vec3());
    let mut t = 5.0;
    while t < 1500.0 {
        let p = o + d * t;
        if p.y <= ground.height_at(p.x, p.z) || ground.water_at(p.x, p.z).is_some_and(|w| p.y <= w.surface_m) {
            return Some(Vec2::new(p.x, p.z));
        }
        t += 2.0;
    }
    None
}

/// Cuts lost parts off the rig: the bone and everything under it become a falling wreck; the stump
/// burns.
fn cut_parts(
    mut commands: Commands,
    mut roots: Query<(&mut MechSim, &mut MechRig)>,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    data: Res<Data>,
    time: Res<Time>,
    mut bursts: MessageWriter<Burst>,
) {
    for (mut sim, mut rig) in &mut roots {
        for part in std::mem::take(&mut sim.cuts) {
            let spec = &data.0.mech.parts[part];
            let Some(&e) = rig.named.get(&spec.cut_bone) else { continue };
            let Ok(g) = globals.get(e) else { continue };
            // Stop driving the cut bone and every contract bone under it.
            let mut under = vec![spec.cut_bone.as_str()];
            for (i, b) in BONES.iter().enumerate() {
                if b.name == spec.cut_bone || b.parent.is_some_and(|p| under.contains(&p)) {
                    under.push(b.name);
                    rig.cut[i] = true;
                }
            }
            let at = g.translation();
            let out = (at - globals.get(rig.bones[0].entity).map(|r| r.translation()).unwrap_or(at))
                .with_y(0.0)
                .normalize_or(Vec3::X);
            commands.entity(e).remove::<ChildOf>().insert((
                g.compute_transform(),
                Debris {
                    vel: out * 6.0 + Vec3::Y * 5.0,
                    spin: Vec3::new(1.5, 0.7, -1.1),
                },
            ));
            if let Ok(stump) = parents.get(e).map(|p| p.parent()).or(Ok::<Entity, ()>(e)) {
                commands.entity(stump).insert(Burning {
                    until_s: time.elapsed_secs() + 25.0,
                });
            }
            for (kind, count, speed, size, life) in [
                (FxKind::Spark, 90, 18.0, 0.25, 1.2),
                (FxKind::Fire, 30, 6.0, 1.6, 0.8),
                (FxKind::Smoke, 30, 3.0, 3.0, 6.0),
            ] {
                bursts.write(Burst {
                    kind,
                    at,
                    dir: Vec3::Y,
                    count,
                    speed_m_s: speed,
                    spread: 0.8,
                    size_m: size,
                    life_s: life,
                });
            }
        }
    }
}

fn burn(
    mut commands: Commands,
    q: Query<(Entity, &GlobalTransform, &Burning)>,
    time: Res<Time>,
    mut bursts: MessageWriter<Burst>,
    mut tick: Local<f32>,
) {
    *tick += time.delta_secs();
    if *tick < 0.1 {
        return;
    }
    *tick = 0.0;
    for (e, g, b) in &q {
        if time.elapsed_secs() > b.until_s {
            commands.entity(e).remove::<Burning>();
            continue;
        }
        bursts.write(Burst {
            kind: FxKind::Smoke,
            at: g.translation(),
            dir: Vec3::Y,
            count: 2,
            speed_m_s: 3.0,
            spread: 0.3,
            size_m: 1.8,
            life_s: 5.0,
        });
        bursts.write(Burst {
            kind: FxKind::Fire,
            at: g.translation(),
            dir: Vec3::Y,
            count: 1,
            speed_m_s: 2.0,
            spread: 0.4,
            size_m: 0.9,
            life_s: 0.5,
        });
    }
}

/// The world transform of a rig socket this frame, composed from local transforms (so it already
/// includes this frame's pose, before Bevy propagates transforms).
pub fn socket_now(
    rig: &MechRig,
    name: &str,
    transforms: &Query<&Transform, Without<crate::pilot::PilotCamera>>,
    parents: &Query<&ChildOf>,
) -> Option<Transform> {
    let e = *rig.sockets.get(name)?;
    let mut acc = transforms.get(e).ok().copied()?;
    let mut at = e;
    while let Ok(p) = parents.get(at) {
        at = p.parent();
        acc = transforms.get(at).ok().copied().unwrap_or_default().mul_transform(acc);
    }
    Some(acc)
}
