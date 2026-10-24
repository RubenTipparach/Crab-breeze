//! The mechs in the game (`mech-rig` sections 1, 5 and 6; `mech-boarding`; `enemy-mechs`;
//! `weapons`): spawns the player's mech and the encounter's enemies from data (the stand-in or a
//! glb), binds the contract's bones and sockets by name (the same code for all), steps each one's
//! locomotion, boarding clock and guns in the fixed step (the player's from the seat's actions,
//! enemies' from their AI pilot, through the same controls), and poses bones and hatch nodes.

use crate::blast::Explosion;
use crate::combat::Shots;
use crate::fx::Fx;
use crate::ground_marks::{Contact, GroundContact};
use crate::input::StepActions;
use crate::pilot::{PilotCamera, PilotEvents, PilotSim};
use crate::sim::{Data, PresentSet, SimSet};
use crate::world::{Live, Plan};
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, RigidBody};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::ai::AiPilot;
use cb_core::boarding::{Hatch, HatchTarget};
use cb_core::input::DebugCommand;
use cb_core::mech::rig::{BONES, SOCKETS};
use cb_core::mech::{Damage, Hit, LocomotionMode, MechConfig, PartKind};
use cb_core::mech::{LegRest, MechControls, MechState, RigRest, foot_strikes, pose, stance_lead_m};
use cb_core::pilot::{MechAccess, PilotEvent, PilotMode};
use cb_core::rng::SimRng;
use cb_core::weapons::{Guns, Projectile};
use cb_core::world::{Ground, Scene};
use std::collections::HashMap;
use std::sync::Arc;

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
    /// Gun reloads.
    pub guns: Guns,
    /// The torso is gone: the mech is a wreck.
    pub dead: bool,
    /// It took a hit since its last step (its AI evades).
    pub took_hit: bool,
}

/// A mech's data.
#[derive(Component, Clone, Deref)]
pub struct MechSpec(pub Arc<MechConfig>);

/// A mech's stable id (projectiles name their owner by it): 0 is the player's.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct MechId(pub u32);

/// The mech the pilot boards and drives.
#[derive(Component)]
pub struct Piloted;

/// A mech driven by an AI pilot.
#[derive(Component)]
pub struct Enemy {
    /// Its pilot.
    pub ai: AiPilot,
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

impl MechRig {
    /// The entity of contract bone `i` (by `BONES` index).
    pub fn bone_entity(&self, i: usize) -> Entity {
        self.bones[i].entity
    }

    /// The optional sensor head.
    pub fn head(&self) -> Option<Entity> {
        self.named.get("head").copied()
    }
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
            .add_systems(
                Update,
                (bind_rig, cut_parts, drive_mech, fall_wrecks, burn, jet_flames)
                    .chain()
                    .in_set(PresentSet::Pose),
            )
            .add_systems(Update, cockpit_layer.after(PresentSet::Pose));
    }
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::from_array(a)
}

#[allow(clippy::too_many_arguments)]
fn spawn_mech(
    mut commands: Commands,
    data: Res<Data>,
    world: Res<Plan>,
    defending: Option<Res<crate::defence::Defending>>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let start = world.start;
    let at = |xz: Vec2| Vec3::new(xz.x, world.terrain.height_at(xz.x, xz.y), xz.y);
    let player = spawn_mech_entity(
        &mut commands,
        Arc::new(data.0.mech.clone()),
        at(start.mech_xz),
        start.mech_heading_rad,
        None,
        &tex.metal,
        &assets,
        &mut meshes,
        &mut materials,
    );
    commands.entity(player).insert((Piloted, MechId(0)));
    // The defence mission brings its own enemies, wave by wave (city-defence).
    if defending.is_some() {
        return;
    }
    for (i, e) in data.0.encounters.enemies.iter().enumerate() {
        let xz = start.mech_xz + Vec2::from_array(e.offset_m);
        let loadout = Loadout {
            mech: &e.mech,
            weapons: e.weapons.as_deref(),
            armour_scale: e.armour_scale,
            tint: e.tint,
            skill: &e.skill,
        };
        let rng = SimRng::new(world.seed).fork(&format!("enemy/{i}"));
        let mut sa = SpawnAssets {
            tex: &tex,
            assets: &assets,
            meshes: &mut meshes,
            materials: &mut materials,
        };
        spawn_enemy(
            &mut commands,
            &world,
            &loadout,
            xz,
            e.heading_deg.to_radians(),
            rng,
            MechId(i as u32 + 1),
            &mut sa,
        );
    }
}

/// What an enemy is: its mech, guns, armour, colour and pilot (from an encounter or a mission).
pub struct Loadout<'a> {
    /// Its mech's data file, relative to `data/`.
    pub mech: &'a str,
    /// Gun override.
    pub weapons: Option<&'a [cb_core::weapons::WeaponSpec]>,
    /// Scales every part's armour.
    pub armour_scale: f32,
    /// Hull colour override.
    pub tint: Option<[f32; 3]>,
    /// Its pilot.
    pub skill: &'a cb_core::ai::Skill,
}

/// The assets a spawned mech draws with.
pub struct SpawnAssets<'a> {
    /// The metal texture.
    pub tex: &'a crate::textures::Textures,
    /// The asset server (glb models).
    pub assets: &'a AssetServer,
    /// Meshes.
    pub meshes: &'a mut Assets<Mesh>,
    /// Materials.
    pub materials: &'a mut Assets<StandardMaterial>,
}

/// Spawns an enemy mech flown by an AI pilot, standing on the ground at `xz`: the one way enemies
/// enter the world, for the encounter and the defence waves alike.
#[allow(clippy::too_many_arguments)]
pub fn spawn_enemy(
    commands: &mut Commands,
    world: &cb_core::world::WorldPlan,
    l: &Loadout,
    xz: Vec2,
    heading_rad: f32,
    rng: SimRng,
    id: MechId,
    a: &mut SpawnAssets,
) -> Option<Entity> {
    let root = cli::repo_root().join("data");
    let mut cfg: MechConfig = match cb_core::data::load_ron(&root.join(l.mech)) {
        Ok(c) => c,
        Err(err) => {
            error!("enemy {}: {err}", id.0);
            return None;
        }
    };
    if let Some(w) = l.weapons {
        cfg.weapons = w.to_vec();
    }
    for p in &mut cfg.parts {
        p.armour_mm *= l.armour_scale;
    }
    let at = Vec3::new(xz.x, world.terrain.height_at(xz.x, xz.y), xz.y);
    let tint = l.tint.map(|c| Color::srgb(c[0], c[1], c[2]));
    let ent = spawn_mech_entity(
        commands,
        Arc::new(cfg),
        at,
        heading_rad,
        tint,
        &a.tex.metal,
        a.assets,
        a.meshes,
        a.materials,
    );
    let ai = AiPilot::new(l.skill.clone(), rng);
    commands.entity(ent).insert((Enemy { ai }, id, EnemyGoal::default()));
    Some(ent)
}

/// What an enemy goes for this step, when something other than the player's mech (a mission's
/// target); `None` is the player. Written before the mechs step.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EnemyGoal {
    /// The point to fight round and aim at, instead of the player's mech.
    pub point: Option<Vec3>,
    /// Hold fire this step.
    pub hold_fire: bool,
}

use crate::cli;

/// Spawns one mech: its root, its model (the stand-in's boxes or a glb), standing at `pos`.
fn spawn_mech_entity(
    commands: &mut Commands,
    cfg: Arc<MechConfig>,
    pos: Vec3,
    heading_rad: f32,
    tint: Option<Color>,
    metal: &Handle<Image>,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    let state = MechState::standing(pos, heading_rad, &cfg.gait);
    let model = commands
        .spawn((
            Name::new("mech model (glTF space)"),
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            Visibility::default(),
        ))
        .id();
    let root = commands
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
                guns: Guns::default(),
                dead: false,
                took_hit: false,
            },
            MechSpec(cfg.clone()),
            Transform::from_translation(state.pos).with_rotation(Quat::from_rotation_y(state.heading_rad)),
            Visibility::default(),
        ))
        .add_child(model)
        .id();

    if let Some(path) = &cfg.model {
        info!("mech model: {path}");
        let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
        commands.spawn((WorldAssetRoot(scene), ChildOf(model)));
        return root;
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
    let hull = tint.unwrap_or(Color::srgb(0.40, 0.43, 0.36));
    let tones = [
        (hull, 0.5),
        (Color::srgb(0.18, 0.19, 0.20), 0.6),
        (Color::srgb(0.30, 0.30, 0.32), 0.4),
        (Color::srgb(0.12, 0.12, 0.13), 0.5),
        (Color::srgb(0.50, 0.60, 0.65), 0.1),
        (Color::srgb(0.85, 0.45, 0.10), 0.5),
    ]
    .map(|(c, rough)| {
        materials.add(StandardMaterial {
            base_color: c,
            base_color_texture: Some(metal.clone()),
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
    root
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
    roots: Query<(Entity, &MechRoot, &MechSpec), (Without<MechRig>, Without<RigRefused>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    parents: Query<&ChildOf>,
    data: Res<Data>,
) {
    for (root, mech, spec) in &roots {
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
            let what = spec.model.clone().unwrap_or_else(|| "the stand-in".into());
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
        let named: HashMap<String, Entity> = named.into_iter().collect();
        // The optional sensor head.
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
    mut roots: Query<(Entity, &MechSim, &MechSpec, Option<&MechRig>), With<MechRoot>>,
    mut transforms: Query<&mut Transform>,
    wrecks: Query<(), With<Wreck>>,
) {
    let alpha = fixed.overstep_fraction();
    for (root, sim, spec, rig) in &mut roots {
        let s = lerp_state(&sim.prev, &sim.curr, alpha);
        let tl = &data.0.boarding;
        let t = sim.hatch_prev.t_s + (sim.hatch.t_s - sim.hatch_prev.t_s) * alpha;
        if let Ok(mut t) = transforms.get_mut(root)
            && !wrecks.contains(root)
        {
            t.translation = s.pos;
            t.rotation = Quat::from_rotation_y(s.heading_rad);
        }
        let Some(rig) = rig else { continue };
        // No legs left: it sits (mech-damage design section 2).
        let sit_amount = if s.legs_lost == [true, true] { 1.0 } else { tl.sit_amount(t) };
        let p = pose(&s, &spec.gait, &rig.rest, &tl.sit, sit_amount);
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
    roots: Query<(&MechSim, &MechRig), With<Piloted>>,
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
            seat_yaw_rad: sim.curr.heading_rad + sim.curr.twist_rad,
            stopped: sim.curr.is_stopped(&data.0.mech.locomotion),
            hatch_open: sim.hatch.is_open(&data.0.boarding),
        })
    });
}

/// Steps every mech: boarding clock (the piloted one), test commands, the AI's or the seat's
/// controls, locomotion on the live ground, trees and walls in its way, guns, and the effects of its
/// feet and jets.
pub fn step_mech(
    mut roots: Query<
        (
            &mut MechSim,
            &MechSpec,
            &MechId,
            Option<&MechRig>,
            Option<(&mut Enemy, &EnemyGoal)>,
            Has<Piloted>,
        ),
        With<MechRoot>,
    >,
    actions: Res<StepActions>,
    pilot: Res<PilotSim>,
    mut events: ResMut<PilotEvents>,
    access: Res<MechAccessNow>,
    data: Res<Data>,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    mut shots: ResMut<Shots>,
    time: Res<Time<Fixed>>,
    camera: Query<&GlobalTransform, With<PilotCamera>>,
    globals: Query<&GlobalTransform>,
    mut explosions: MessageWriter<Explosion>,
    mut fx: MessageWriter<Fx>,
    targeting: Res<crate::targeting::TargetState>,
    mut contacts: MessageWriter<GroundContact>,
) {
    let _t = crate::perf_hud::slow("step_mech");
    let tl = &data.0.boarding;
    let locked = targeting.locked(&data.0.targeting);
    let dt = time.delta_secs();
    let seated = pilot.curr.mode == PilotMode::Seated;
    let live = &mut *live;
    // What the enemies see: the player's mech.
    // Every mech's body, for the pilot's look ray: the guns converge on what the pilot looks at.
    let bodies: Vec<(u32, Vec3)> = roots.iter().filter(|r| !r.0.dead).map(|r| (r.2.0, r.0.curr.pos)).collect();
    let player = roots.iter().find(|r| r.5).map(|r| {
        (
            r.0.curr.pos,
            cb_core::mech::forward(r.0.curr.heading_rad) * r.0.curr.speed_m_s + Vec3::Y * r.0.curr.vel_y_m_s,
            r.0.dead,
        )
    });
    let cam = camera.iter().next().copied();
    for (mut sim, spec, id, rig, enemy, piloted) in &mut roots {
        let cfg: &MechConfig = spec;
        sim.prev = sim.curr;
        sim.hatch_prev = sim.hatch;
        if sim.dead {
            continue;
        }
        let mut fire = [false, false, false];
        let mut aim_at: Option<Vec3> = None;
        if piloted {
            // The player's mech's hit flag lasts one step (its enemies' brains take theirs below);
            // left set, a brain flying it would evade, and hop, forever.
            sim.took_hit = false;
        }
        let controls = if piloted {
            // The boarding sequence follows the pilot (mech-boarding design section 4).
            for e in &events.0 {
                match e {
                    PilotEvent::HatchWanted => sim.hatch.target = HatchTarget::Open,
                    // Seated: the hatch closes and the mech stands up, ready to go (the owner's flow:
                    // approach, it kneels, step in, it stands).
                    PilotEvent::Seated => sim.hatch.target = HatchTarget::Standing,
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
            // It is the pilot's own mech: when they walk up to it, it turns to them and kneels,
            // ready to board (owner, 2026-10-24: "when I approach it, I expect it to crouch down").
            let near = access
                .0
                .as_ref()
                .is_some_and(|a| pilot.curr.mode == PilotMode::OnFoot && pilot.curr.distance_to_stair(a) < data.0.pilot.board_reach_m);
            if near && sim.curr.is_stopped(&cfg.locomotion) {
                sim.hatch.target = HatchTarget::Open;
            }
            if seated && sim.hatch.is_sealed_sitting(tl) && (actions.throttle_axis > 0.0 || actions.lift_axis > 0.0) {
                sim.hatch.target = HatchTarget::Standing;
            }
            sim.hatch.step(tl, dt);
            // Test commands until every weapon exists.
            match actions.debug {
                Some(DebugCommand::Hit(kind)) => {
                    if let Some(part) = cfg.parts.iter().position(|p| p.kind == kind) {
                        let hit = Hit {
                            part,
                            penetration_mm: 500.0,
                            damage: 1000.0,
                        };
                        apply_hit(&mut sim, cfg, hit);
                    }
                }
                Some(DebugCommand::Shell) => {
                    if let Some(at) = cam.and_then(|c| aim_point(&c, &plan, &live.dug)) {
                        let y = plan.terrain.height_at(at.x, at.y) + live.dug.delta_at(at);
                        explosions.write(Explosion {
                            at: Vec3::new(at.x, y, at.y),
                            energy_mj: 6.0,
                        });
                    }
                }
                None => {}
            }
            if !seated && sim.hatch.target == HatchTarget::Open && pilot.curr.mode == PilotMode::OnFoot {
                // Asked to board: turn to face the pilot while kneeling, so the hatch is toward them.
                let to = pilot.curr.feet - sim.curr.pos;
                let want = (-to.x).atan2(-to.z);
                let err = (want - sim.curr.heading_rad + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                MechControls {
                    turn_axis: (err * 3.0).clamp(-1.0, 1.0),
                    ..default()
                }
            } else if seated && sim.hatch.can_walk() {
                fire = [actions.fire[0], actions.fire[1], actions.fire_missile];
                // The torso turns to follow where the pilot looks (the view leads, the torso
                // follows); holding free look keeps the torso where it is.
                let mut c = MechControls::from_actions(&actions);
                // Boost thrust goes where the pilot looks.
                c.view_yaw_rad = pilot.curr.yaw_rad;
                if !actions.free_look {
                    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    let twist = wrap(pilot.curr.yaw_rad - sim.curr.heading_rad);
                    c.aim_delta_deg = Vec2::new(
                        -(twist - sim.curr.aim_twist_rad).to_degrees(),
                        (pilot.curr.pitch_rad - sim.curr.aim_pitch_rad).to_degrees(),
                    );
                }
                let others: Vec<Vec3> = bodies.iter().filter(|b| b.0 != id.0).map(|b| b.1).collect();
                aim_at = cam.map(|c| look_point(&c, &plan, live, &others));
                c
            } else {
                MechControls::default()
            }
        } else if !seated {
            // The enemies wait until the pilot is in the seat: no one fights an empty mech.
            MechControls::default()
        } else if let (Some((mut enemy, goal)), Some((player_at, player_vel, player_dead))) = (enemy, player) {
            // A mission may send it at something else (city-defence: a building to shell).
            let (target, target_vel, target_dead) = match goal.point {
                Some(p) => (p, Vec3::ZERO, false),
                None => (player_at, player_vel, player_dead),
            };
            let eye = sim.curr.pos + Vec3::Y * 9.6;
            let speed = cfg.weapons.first().map_or(850.0, |w| w.speed_m_s);
            let allies: Vec<Vec3> = bodies.iter().filter(|b| b.0 != id.0 && b.0 != 0).map(|b| b.1).collect();
            let hit = std::mem::take(&mut sim.took_hit);
            let health = torso_health(&sim, cfg);
            let clear = |a: Vec3, b: Vec3| line_clear(&plan, live, a, b);
            let cover = |me: Vec3, threat: Vec3| cover_point(&plan, live, me, threat);
            let view = cb_core::ai::View {
                eye,
                target,
                target_vel,
                muzzle_speed_m_s: speed,
                drop: !cfg.weapons.first().is_some_and(|w| w.rocket),
                health,
                hit,
                allies: &allies,
                clear: &clear,
                cover: &cover,
            };
            let orders = enemy.ai.orders(&sim.curr, &view, dt);
            if !target_dead && !goal.hold_fire {
                fire = [orders.fire[0], orders.fire[1], false];
            }
            aim_at = Some(orders.aim_point);
            orders.controls
        } else {
            MechControls::default()
        };

        let ground = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
            structures: Some(&live.structures),
        };
        let was_airborne = sim.curr.airborne();
        sim.curr.step(&controls, cfg, &ground, dt);

        // Trees in the way: the legs snap trunks (or are stopped by them), the hull and arms snap
        // crowns, and each snapped tree takes some momentum (vegetation design section 5).
        {
            let l = &cfg.locomotion;
            let trees = &data.0.world.trees;
            let bump = live.forest.bump(
                Vec2::new(sim.curr.pos.x, sim.curr.pos.z),
                &cb_core::world::Body {
                    feet_y: sim.curr.pos.y,
                    trunk_radius_m: l.body_radius_m,
                    brush_radius_m: l.brush_radius_m,
                    height_m: l.body_height_m,
                    mass_kg: l.mass_kg,
                },
                sim.curr.vel,
                &cb_core::world::Push {
                    crown_leverage: trees.crown_leverage,
                    graze_share: trees.graze_share,
                },
            );
            if bump.blocked {
                sim.curr.pos.x = bump.pos.x;
                sim.curr.pos.z = bump.pos.y;
                sim.curr.vel *= 0.5;
            }
            sim.curr.vel *= bump.kept;
            live.moved_trees.extend(bump.broke);
        }
        let Some(rig) = rig else { continue };
        let damage = sim.damage.clone();
        // Guns: fire from the muzzle toward the aim point; a lost arm's gun is gone.
        let fired = sim.guns.trigger(&cfg.weapons, fire, |k| damage.is_lost(&cfg.parts, k), dt);
        for wi in fired {
            let w = &cfg.weapons[wi];
            // A mech without a missile pod launches its missiles from the right arm.
            let socket: Option<&'static str> = ["MUZZLE_L", "MUZZLE_R", "MISSILE_POD"].into_iter().find(|s| *s == w.socket);
            let socket = match socket {
                Some("MISSILE_POD") if !rig.sockets.contains_key("MISSILE_POD") => Some("MUZZLE_R"),
                s => s,
            };
            let Some(muzzle) = socket
                .and_then(|s| rig.sockets.get(s))
                .and_then(|e| globals.get(*e).ok())
                .map(|g| g.translation())
            else {
                continue;
            };
            let target = aim_at.unwrap_or(muzzle + cb_core::mech::forward(sim.curr.heading_rad + sim.curr.twist_rad) * 500.0);
            let dir = (target - muzzle).normalize_or(Vec3::NEG_Z);
            let carry = cb_core::mech::forward(sim.curr.heading_rad) * sim.curr.speed_m_s;
            shots.0.push(Projectile {
                owner: id.0,
                weapon: wi,
                pos: muzzle,
                vel: dir * w.speed_m_s + carry,
                age_s: 0.0,
                rocket: w.rocket,
                // A missile homes on the pilot's lock (targeting); unlocked it flies straight.
                target: (piloted && w.seeker_turn_deg_s > 0.0).then_some(locked).flatten(),
            });
            let recipe = if w.rocket {
                "muzzle_rocket"
            } else if w.blast_mj >= 1.0 {
                "muzzle_cannon"
            } else {
                "muzzle_autocannon"
            };
            fx.write(Fx::new(recipe, muzzle).toward(dir));
        }

        // Footfalls: spray and ripples in water; on dry ground a contact for the ground marks, which
        // print the foot, throw the dirt or the grit and jolt the pilot (ground-marks). A heavy
        // landing throws more.
        let landed = was_airborne && !sim.curr.airborne();
        let strikes = foot_strikes(&sim.prev, &sim.curr, landed);
        for side in [0usize, 1] {
            if !strikes[side] {
                continue;
            }
            let ankle = rig.rest.legs[side].ankle;
            let turn = Quat::from_rotation_y(sim.curr.heading_rad + std::f32::consts::PI);
            let foot = sim.curr.pos + turn * Vec3::new(ankle.x, 0.0, ankle.z);
            let (density, scale) = if landed { (3.0, 1.3) } else { (1.0, 1.0) };
            match ground.water_at(foot.x, foot.z) {
                Some(w) => {
                    let at = Vec3::new(foot.x, w.surface_m, foot.z);
                    fx.write(Fx::new("foot_spray", at).dense(density).scaled(scale));
                }
                None => {
                    contacts.write(GroundContact {
                        mech: id.0,
                        piloted,
                        contact: Contact::Step {
                            side,
                            foot,
                            heading_rad: sim.curr.heading_rad,
                            lead_m: stance_lead_m(&sim.curr),
                            speed_m_s: sim.curr.vel.length(),
                            landed,
                        },
                    });
                }
            }
        }
        // Skating on the ground: each wheel's contact, for its trail and its dirt or sparks
        // (ground-marks); a rooster tail of spray on water.
        if sim.curr.mode == LocomotionMode::Skate && !sim.curr.airborne() {
            let on_water = sim.curr.on_water == cb_core::mech::OnWater::Skimming;
            for (i, wheel) in ["WHEEL_L", "WHEEL_R"].into_iter().enumerate() {
                if let Some(g) = rig.sockets.get(wheel).and_then(|e| globals.get(*e).ok()) {
                    if on_water {
                        if sim.curr.vel.length() > 5.0 {
                            let back = -Vec3::new(sim.curr.vel.x, 0.0, sim.curr.vel.y).normalize_or(Vec3::Z);
                            let fast = (sim.curr.vel.length() / 30.0).clamp(0.5, 4.0);
                            fx.write(Fx::new("skate_spray", g.translation()).toward(back + Vec3::Y * 0.8).dense(fast));
                        }
                        continue;
                    }
                    contacts.write(GroundContact {
                        mech: id.0,
                        piloted,
                        contact: Contact::Wheel {
                            wheel: i as u8,
                            at: g.translation(),
                            vel: sim.curr.vel,
                        },
                    });
                }
            }
        }
        // A mech striding into a wall at speed smashes through it (building-destruction).
        let pushed_back =
            sim.prev.speed_m_s.abs() > 2.5 && sim.curr.speed_m_s.abs() < sim.prev.speed_m_s.abs() * 0.6 && !sim.curr.airborne();
        if pushed_back {
            let front = sim.curr.pos + cb_core::mech::forward(sim.curr.heading_rad) * (cfg.locomotion.body_radius_m + 2.5) + Vec3::Y * 5.0;
            let p2 = Vec2::new(front.x, front.z);
            for b in plan.collision.candidates(p2, 3.0) {
                let (broke, chunks) = live.structures.break_sphere(b, front, 4.5);
                if broke > 0 {
                    live.dirty_buildings.push(b);
                    live.new_chunks.extend(chunks);
                    sim.curr.speed_m_s = sim.prev.speed_m_s * 0.7;
                    fx.write(Fx::new("wall_smash", front));
                }
            }
        }
    }
    events.0.clear();
}

/// A mech's torso structure left, 0..1.
pub fn torso_health(sim: &MechSim, cfg: &MechConfig) -> f32 {
    cfg.parts
        .iter()
        .position(|p| p.kind == PartKind::Torso)
        .map_or(1.0, |i| sim.damage.structure[i] / cfg.parts[i].structure.max(1.0))
}

/// True when the straight line from `a` to `b` is clear of standing building cells and the ground.
pub fn line_clear(plan: &cb_core::world::WorldPlan, live: &Live, a: Vec3, b: Vec3) -> bool {
    let len = a.distance(b);
    let steps = (len / 4.0).ceil().max(1.0) as usize;
    for s in 1..steps {
        let p = a.lerp(b, s as f32 / steps as f32);
        let p2 = Vec2::new(p.x, p.z);
        if p.y <= plan.terrain.height_at(p.x, p.z) + live.dug.delta_at(p2) {
            return false;
        }
        if plan
            .collision
            .candidates(p2, 0.5)
            .into_iter()
            .any(|bi| live.structures.buildings[bi as usize].solid_at(p))
        {
            return false;
        }
    }
    true
}

/// A spot behind the nearest standing building within 200 m, on its far side from `threat`.
pub fn cover_point(plan: &cb_core::world::WorldPlan, live: &Live, me: Vec3, threat: Vec3) -> Option<Vec3> {
    let here = Vec2::new(me.x, me.z);
    let from = Vec2::new(threat.x, threat.z);
    plan.collision
        .candidates(here, 200.0)
        .into_iter()
        .filter(|b| live.structures.buildings[*b as usize].standing > 0)
        .map(|b| &plan.buildings[b as usize])
        .min_by(|a, b| a.footprint.centre().distance(here).total_cmp(&b.footprint.centre().distance(here)))
        .map(|b| {
            let c = b.footprint.centre();
            let away = (c - from).normalize_or(Vec2::X);
            let p = c + away * (b.footprint.size().max_element() * 0.5 + 14.0);
            Vec3::new(p.x, me.y, p.y)
        })
}

/// Applies a hit to a mech's part: structure, a lost part cut off, legs lost, death by the torso.
pub fn apply_hit(sim: &mut MechSim, cfg: &MechConfig, hit: Hit) {
    sim.took_hit = true;
    if let Some(lost) = sim.damage.resolve(&cfg.parts, hit) {
        sim.cuts.push(lost);
        sim.curr.legs_lost = sim.damage.legs(&cfg.parts).map(|on| !on);
        info!(
            "damage: {:?} lost by the mech at ({:.0}, {:.0})",
            cfg.parts[lost].kind, sim.curr.pos.x, sim.curr.pos.z
        );
        if cfg.parts[lost].kind == PartKind::Torso {
            sim.dead = true;
            sim.curr.legs_lost = [true, true];
            sim.curr.throttle = 0.0;
            sim.curr.speed_m_s = 0.0;
        }
    }
}

/// Where the pilot is looking: the first mech body, ground, water or standing building cell along
/// the view, or 1.5 km out.
fn look_point(camera: &GlobalTransform, plan: &cb_core::world::WorldPlan, live: &Live, mechs: &[Vec3]) -> Vec3 {
    let ground = Scene {
        plan,
        dug: &live.dug,
        trees: None,
        structures: Some(&live.structures),
    };
    let (o, d) = (camera.translation(), camera.forward().as_vec3());
    let mut t = 20.0;
    while t < 1500.0 {
        let p = o + d * t;
        let p2 = Vec2::new(p.x, p.z);
        let wall = plan
            .collision
            .candidates(p2, 0.5)
            .into_iter()
            .any(|b| live.structures.buildings[b as usize].solid_at(p));
        // A mech: a column 3 m round from its feet to the top of its torso.
        let mech = mechs
            .iter()
            .any(|m| Vec2::new(m.x, m.z).distance(p2) < 3.0 && p.y > m.y && p.y < m.y + 12.5);
        if wall || mech || p.y <= ground.height_at(p.x, p.z) {
            return p;
        }
        t += 2.0;
    }
    o + d * 1500.0
}

/// Where the pilot's view meets the ground, within 1.5 km.
fn aim_point(camera: &GlobalTransform, plan: &cb_core::world::WorldPlan, dug: &cb_core::world::Deformation) -> Option<Vec2> {
    let ground = Scene {
        plan,
        dug,
        trees: None,
        structures: None,
    };
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

/// A destroyed mech coming down: it falls under gravity to the ground and topples from its feet
/// like a felled tower, until it lies on its side. Presentation only: the simulation's mech is dead
/// and still. Computed against the ground's height, so it works anywhere, not only where the
/// physics has ground colliders (within 400 m of the eye).
#[derive(Component)]
pub struct Wreck {
    vel: Vec3,
    axis: Vec3,
    angle_rad: f32,
    rate_rad_s: f32,
    heading_rad: f32,
}

/// Height of a mech's centre of mass over its feet, for the topple.
const WRECK_CENTRE_M: f32 = 10.0;
/// It comes to rest this far over: lying on its side.
const WRECK_LIE_RAD: f32 = 1.45;

fn fall_wrecks(time: Res<Time>, plan: Res<Plan>, mut wrecks: Query<(&mut Wreck, &mut Transform)>) {
    let dt = time.delta_secs().min(0.05);
    for (mut w, mut t) in &mut wrecks {
        let ground = plan.height_at(t.translation.x, t.translation.z);
        if t.translation.y > ground + 0.01 || w.vel.y > 0.0 {
            w.vel.y -= 9.81 * dt;
            t.translation += w.vel * dt;
            if t.translation.y <= ground {
                t.translation.y = ground;
                w.vel = Vec3::ZERO;
            }
        } else {
            t.translation.y = ground;
            // A pole falling about its foot: angular acceleration 3 g sin(angle) / (2 L).
            if w.angle_rad < WRECK_LIE_RAD {
                w.rate_rad_s += 1.5 * 9.81 * w.angle_rad.sin() / WRECK_CENTRE_M * dt;
                w.angle_rad = (w.angle_rad + w.rate_rad_s * dt).min(WRECK_LIE_RAD);
            }
        }
        t.rotation = Quat::from_axis_angle(w.axis, w.angle_rad) * Quat::from_rotation_y(w.heading_rad);
    }
}

/// Cuts lost parts off the rig: the bone and everything under it become a falling wreck; the stump
/// burns.
fn cut_parts(
    mut commands: Commands,
    mut roots: Query<(Entity, &mut MechSim, &mut MechRig, &MechSpec)>,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    time: Res<Time>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
) {
    let _t = crate::perf_hud::slow("cut_parts");
    for (root, mut sim, mut rig, mech) in &mut roots {
        for part in std::mem::take(&mut sim.cuts) {
            let spec = &mech.parts[part];
            if spec.kind == PartKind::Torso {
                // The torso is gone: the mech blows up where it stands and burns.
                let at = globals.get(root).map(|g| g.translation()).unwrap_or_default() + Vec3::Y * 9.0;
                explosions.write(Explosion { at, energy_mj: 25.0 });
                // The wreck falls to the ground and topples from its feet (owner, 2026-10-24: dead
                // mechs "stuck floating in the air"), away from the side it was facing, at random.
                let side = if root.index().index() % 2 == 0 { 1.0 } else { -1.0 };
                let heading = sim.curr.heading_rad;
                commands.entity(root).insert(Wreck {
                    vel: Vec3::new(sim.curr.vel.x, sim.curr.vel_y_m_s, sim.curr.vel.y),
                    axis: Quat::from_rotation_y(heading) * Vec3::new(0.0, 0.0, side),
                    angle_rad: 0.05,
                    rate_rad_s: 0.0,
                    heading_rad: heading,
                });
                commands.entity(rig.bones[3].entity).insert(Burning {
                    until_s: time.elapsed_secs() + 60.0,
                });
                continue;
            }
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
            // The wreck falls under physics (debris-physics): a box about the part's size.
            commands.entity(e).remove::<ChildOf>().insert((
                g.compute_transform(),
                RigidBody::Dynamic,
                Collider::cuboid(1.6, 3.0, 1.6),
                LinearVelocity(out * 6.0 + Vec3::Y * 5.0),
                AngularVelocity(Vec3::new(1.5, 0.7, -1.1)),
            ));
            if let Ok(stump) = parents.get(e).map(|p| p.parent()).or(Ok::<Entity, ()>(e)) {
                commands.entity(stump).insert(Burning {
                    until_s: time.elapsed_secs() + 25.0,
                });
            }
            fx.write(Fx::new("part_lost", at));
        }
    }
}

/// While the pilot sits in the mech with the hatch shut, every mesh of the mech is on the cockpit
/// layer, drawn after the world on its own depth so nothing in the world draws through it
/// (cockpit-overlay; owner: "only when it is closed and player is inside"). Otherwise it is drawn
/// with the world like anything else. Walks the mech's descendants each frame, so a model that
/// loads late is caught.
fn cockpit_layer(
    mut commands: Commands,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mech: Query<(Entity, &MechSim), With<Piloted>>,
    children: Query<&Children>,
    meshes: Query<Option<&bevy::camera::visibility::RenderLayers>, With<Mesh3d>>,
) {
    use bevy::camera::visibility::RenderLayers;
    let Some((root, sim)) = mech.iter().next() else { return };
    let inside = pilot.curr.mode == PilotMode::Seated && sim.hatch.is_closed(&data.0.boarding);
    let want = RenderLayers::layer(crate::pilot::COCKPIT_LAYER);
    for e in children.iter_descendants(root) {
        let Ok(layers) = meshes.get(e) else { continue };
        match (inside, layers) {
            (true, Some(l)) if *l == want => {}
            (true, _) => {
                commands.entity(e).insert(want.clone());
            }
            (false, Some(_)) => {
                commands.entity(e).remove::<RenderLayers>();
            }
            (false, None) => {}
        }
    }
}

/// The booster flame while boosting: a stream on each jet socket (`FxStream`), switched with the
/// boost.
fn jet_flames(mut commands: Commands, mechs: Query<(&MechSim, &MechRig)>, mut streams: Query<&mut crate::fx::FxStream>) {
    for (sim, rig) in &mechs {
        for jet in ["JET_L", "JET_R"] {
            let Some(&e) = rig.sockets.get(jet) else { continue };
            let on = sim.curr.boosting && !sim.dead;
            match streams.get_mut(e) {
                Ok(mut s) => {
                    if s.on != on {
                        s.on = on;
                    }
                }
                Err(_) => {
                    commands.entity(e).insert(crate::fx::FxStream {
                        recipe: "jet_flame",
                        dir: Vec3::NEG_Y,
                        on,
                    });
                }
            }
        }
    }
}

fn burn(mut commands: Commands, q: Query<(Entity, &GlobalTransform, &Burning)>, time: Res<Time>, mut tick: Local<f32>) {
    *tick += time.delta_secs();
    if *tick < 0.1 {
        return;
    }
    *tick = 0.0;
    // A fire is a stream riding what burns (`FxStream`), lit while it burns.
    for (e, _, b) in &q {
        let on = time.elapsed_secs() <= b.until_s;
        commands.entity(e).insert(crate::fx::FxStream {
            recipe: "burning",
            dir: Vec3::Y,
            on,
        });
        if !on {
            commands.entity(e).remove::<Burning>();
        }
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
