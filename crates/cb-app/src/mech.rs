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
use crate::state::GameApp;
use crate::world::{Live, Plan};
use avian3d::prelude::{AngularVelocity, Collider, LinearVelocity, RigidBody};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use cb_core::ai::AiPilot;
use cb_core::boarding::{Hatch, HatchTarget};
use cb_core::input::DebugCommand;
use cb_core::mech::rig::{BONES, SOCKETS};
use cb_core::mech::{Damage, Hit, LocomotionMode, MechConfig, OnWater, PartKind};
use cb_core::mech::{LegRest, MechControls, MechState, RigRest, foot_strikes, pose, stance_lead_m};
use cb_core::pilot::{MechAccess, PilotEvent, PilotMode};
use cb_core::rng::SimRng;
use cb_core::weapons::{Guns, Projectile};
use cb_core::world::{Ground, Scene};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// The mech's root: on the ground, turned to its heading.
#[derive(Component)]
pub struct MechRoot {
    /// The child turned 180 degrees about y that holds the model in glTF space.
    pub(crate) model: Entity,
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
    /// The weapon station the pilot's primary trigger fires (weapon-select).
    pub selected: usize,
    /// Close combat on or off (close-combat).
    pub close: cb_core::mech::CloseCombat,
    /// The arms' strikes at the step before, for drawing between steps.
    pub strikes_prev: cb_core::mech::Strikes,
    /// The arms' strikes at the latest step.
    pub strikes: cb_core::mech::Strikes,
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

/// A mech the career owns (its key, `cb_core::career::OwnedMech::key`): one the pilot may walk up to and board
/// (mech-shop section 6.3). The piloted mech carries it too, in a career.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Owned(pub u32);

/// The stable id of an owned mech that is not the piloted one (0 is always the piloted mech's): clear of every
/// encounter's and job's ids.
pub fn owned_id(key: u32) -> MechId {
    MechId(0x4000_0000 + key)
}

/// A mech driven by an AI pilot.
#[derive(Component)]
pub struct Enemy {
    /// Its pilot.
    pub ai: AiPilot,
    /// The line it flies with `CB_ENEMY_STUNT`, from the player out toward where it first stood.
    pub stunt_axis: Option<Vec2>,
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
    /// The stride's part of the pelvis bone's translation this frame (`MechPose::walk_offset` in the
    /// pelvis's local space), which the pilot's view leaves out.
    walk_local: Vec3,
}

impl MechRig {
    /// The entity of the node named `name` under the model, if any.
    pub(crate) fn named(&self, name: &str) -> Option<Entity> {
        self.named.get(name).copied()
    }

    /// The torso bone's entity.
    pub(crate) fn torso(&self) -> Entity {
        self.bones[3].entity
    }

    /// The entity of contract bone `i` (by `BONES` index).
    pub fn bone_entity(&self, i: usize) -> Entity {
        self.bones[i].entity
    }

    /// The optional sensor head.
    pub fn head(&self) -> Option<Entity> {
        self.named.get("head").copied()
    }

    /// The entity of a contract socket (`MUZZLE_L`, `JET_L`, `WHEEL_R`, `HATCH_entry`, `EYE_pilot`...),
    /// for something that rides it (a sound).
    pub fn socket(&self, name: &str) -> Option<Entity> {
        self.sockets.get(name).copied()
    }
}

/// Smoke and fire rising from where a part was shot off.
#[derive(Component)]
pub(crate) struct Burning {
    pub(crate) until_s: f32,
}

/// The model lacks contract bones; it is shown but not driven.
#[derive(Component)]
struct RigRefused;

/// The cockpit interior spawned beside the mech, waiting for the rig to be bound before it is hung in the chest
/// (openspec change `mech-roster` section 2).
#[derive(Component)]
struct PendingInterior(Entity);

/// The cockpit interior a mech carries: its meshes, drawn in the sealed cabin, and among them the screen wall,
/// through which the world shows.
#[derive(Component)]
pub struct Interior {
    meshes: HashSet<Entity>,
    windows: HashSet<Entity>,
}

/// The screen wall's window panes as triangles in the interior's own space (its `GlobalTransform` carries them to
/// the world): the lens pass lays the rain's drops on them, so drops sit on the glass and not on the pilot's view
/// (`water_render`).
#[derive(Component, Clone, Debug, Default)]
pub struct CabinGlass(pub Vec<[Vec3; 3]>);

/// A layer no camera draws: the hull and the screen wall go there while the pilot sits sealed in an interior.
const UNSEEN_LAYER: usize = 30;

/// Is this an interior node of the screen wall, a window on the world (material `SCREEN_allview`)? The interior
/// generator names the wall's mesh nodes `<part>_allview`; its `SCREEN_<name>_screen` nodes are the console's
/// monitors, which stay in the cabin.
fn is_window(name: &str) -> bool {
    name.ends_with("_allview")
}

/// The mech's boarding sockets this fixed step, for the pilot.
#[derive(Resource, Default)]
pub struct MechAccessNow(pub Option<MechAccess>);

/// Spawns, binds, steps and poses the mech.
pub struct MechPlugin;

impl Plugin for MechPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MechAccessNow>()
            .add_world_spawn(spawn_mech)
            .add_game_systems(Update, swap_mech.run_if(resource_exists::<SwapMech>).before(PresentSet::Pose))
            .add_game_systems(Update, spawn_encounters.run_if(resource_added::<crate::menu::Encounters>))
            .add_game_systems(FixedUpdate, (publish_access.in_set(SimSet::Input), step_mech.in_set(SimSet::Mech)))
            .add_game_systems(
                Update,
                (bind_rig, bind_interior, cut_parts, drive_mech, fall_wrecks, burn, jet_flames)
                    .chain()
                    .in_set(PresentSet::Pose),
            )
            .add_game_systems(Update, cockpit_layer.after(PresentSet::Pose));
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
}

/// Free roam's pause menu picked another mech (openspec change `menu-in-game`, section 3): its data is already in
/// `Data` (`menu::press`), and `swap_mech` puts it where the piloted mech stands, in this program.
#[derive(Resource)]
pub struct SwapMech;

/// The piloted mech swapped in place: the old one's tree is taken down and the picked one spawned through the same
/// path the world spawn uses (`spawn_mech_entity`), where the old one stood and facing its way, standing as a fresh
/// start stands it, ready to board. A pilot not on foot is put on foot beside it, where a fresh start stands them.
#[allow(clippy::too_many_arguments)]
fn swap_mech(
    mut commands: Commands,
    data: Res<Data>,
    world: Res<Plan>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pilot: ResMut<PilotSim>,
    piloted: Query<(Entity, &MechSim), With<Piloted>>,
) {
    commands.remove_resource::<SwapMech>();
    let (at, heading) = match piloted.iter().next() {
        Some((old, sim)) => {
            commands.entity(old).despawn();
            (sim.curr.pos, sim.curr.heading_rad)
        }
        None => {
            let s = world.start;
            let y = world.terrain.height_at(s.mech_xz.x, s.mech_xz.y);
            (Vec3::new(s.mech_xz.x, y, s.mech_xz.y), s.mech_heading_rad)
        }
    };
    let player = spawn_mech_entity(
        &mut commands,
        Arc::new(data.0.mech.clone()),
        at,
        heading,
        None,
        &tex.metal,
        &assets,
        &mut meshes,
        &mut materials,
    );
    commands.entity(player).insert((Piloted, MechId(0)));
    if pilot.curr.mode != PilotMode::OnFoot {
        let s = world.start;
        let turn = Quat::from_rotation_y(heading - s.mech_heading_rad);
        let off = turn * Vec3::new(s.pilot_xz.x - s.mech_xz.x, 0.0, s.pilot_xz.y - s.mech_xz.y);
        let xz = Vec2::new(at.x + off.x, at.z + off.z);
        let feet = Vec3::new(xz.x, world.terrain.height_at(xz.x, xz.y), xz.y);
        let to_mech = Vec2::new(at.x, at.z) - xz;
        let state = cb_core::pilot::PilotState::standing(feet, (-to_mech.x).atan2(-to_mech.y));
        pilot.prev = state.clone();
        pilot.curr = state;
    }
    info!("mech: swapped in place for the {}", data.0.mech.name);
}

/// Free roam's encounter mechs near the start, spawned only when a scripted take asks for them
/// (`menu::Encounters`): free roam as a player plays it starts with none. The defence mission brings its own
/// enemies, wave by wave (city-defence).
fn spawn_encounters(
    mut commands: Commands,
    play: Res<crate::menu::Play>,
    data: Res<Data>,
    world: Res<Plan>,
    tex: Res<crate::textures::Textures>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *play != crate::menu::Play::FreeRoam {
        return;
    }
    let start = world.start;
    for (i, e) in data.0.encounters.enemies.iter().enumerate() {
        let xz = start.mech_xz + Vec2::from_array(e.offset_m);
        let loadout = Loadout {
            mech: &e.mech,
            weapons: e.weapons.as_deref(),
            armour_scale: e.armour_scale,
            tint: e.tint,
            skill: &e.skill,
            skin: e.skin.as_ref().and_then(|s| data.0.skins.skins.get(s)),
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
    /// The block-out it wears over the stand-in (openspec change `blockout-enemies`); `None` shows the stand-in.
    pub skin: Option<&'a cb_core::skins::SkinSpec>,
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
    if l.skin.is_some() {
        cfg.boxes.clear(); // the skin is drawn instead; the skeleton stays for the sim, sockets and hits
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
    if let Some(skin) = l.skin {
        crate::skins::wear(commands, a.assets, ent, skin);
    }
    let ai = AiPilot::new(l.skill.clone(), rng);
    commands
        .entity(ent)
        .insert((Enemy { ai, stunt_axis: None }, id, EnemyGoal::default()));
    Some(ent)
}

/// Spawns an owned mech standing (or, for a drop, held in the air) at `pos`, empty and sealed, ready to be walked
/// up to and boarded: the one way the career's other mechs enter the world (a drop, a mech left where the pilot
/// climbed out, one called out at a base, a save continued).
pub fn spawn_owned(commands: &mut Commands, cfg: Arc<MechConfig>, pos: Vec3, heading_rad: f32, key: u32, a: &mut SpawnAssets) -> Entity {
    let ent = spawn_mech_entity(commands, cfg, pos, heading_rad, None, &a.tex.metal, a.assets, a.meshes, a.materials);
    commands.entity(ent).insert((Owned(key), owned_id(key)));
    ent
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
                selected: cb_core::weapons::first_station(&cfg.weapons),
                close: Default::default(),
                strikes_prev: Default::default(),
                strikes: Default::default(),
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
        if let Some(i) = &cfg.interior {
            // Beside the model, hidden, until the rig is bound and the interior can be hung on its cockpit bone.
            let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(i.model.clone()));
            let interior = commands
                .spawn((
                    Name::new("cockpit interior"),
                    Transform::default(),
                    Visibility::Hidden,
                    WorldAssetRoot(scene),
                    ChildOf(root),
                ))
                .id();
            commands.entity(root).insert(PendingInterior(interior));
        }
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
            walk_local: Vec3::ZERO,
            bones,
            sockets,
            rest,
            hatch_nodes,
            named,
        });
    }
}

/// Hangs a bound mech's cockpit interior in its chest, as the mockup page does (`gen_mech_designs.py`): the interior
/// at the `cockpit` bone's origin, each riding part moved onto its chest joint keeping where it is, so the two open
/// as one body; the interior's lamps light the cabin on the cockpit layer (`cockpit-lighting`). Waits until the
/// interior's scene has spawned.
#[allow(clippy::too_many_arguments)]
fn bind_interior(
    mut commands: Commands,
    roots: Query<(Entity, &MechRig, &MechSpec, &PendingInterior)>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    parents: Query<&ChildOf>,
    meshes: Query<&Mesh3d>,
    lights: Query<(), Or<(With<PointLight>, With<SpotLight>, With<DirectionalLight>)>>,
    mut points: Query<&mut PointLight>,
    mut spots: Query<&mut SpotLight>,
    piloted: Query<(), Or<(With<Piloted>, With<Owned>)>>,
    data: Res<Data>,
    mut assets: ResMut<Assets<Mesh>>,
    mut cut_done: Local<HashSet<AssetId<Mesh>>>,
) {
    for (root, rig, spec, pending) in &roots {
        let interior = pending.0;
        let mut named: HashMap<String, Entity> = HashMap::new();
        let (mut parts, mut windows, mut lamps) = (HashSet::new(), HashSet::new(), Vec::new());
        for e in children.iter_descendants(interior) {
            if let Ok(n) = names.get(e) {
                named.entry(n.as_str().to_string()).or_insert(e);
            }
            if lights.contains(e) {
                lamps.push(e);
            }
            if meshes.contains(e) {
                parts.insert(e);
                let parent = parents.get(e).ok().and_then(|p| names.get(p.parent()).ok());
                if names.get(e).ok().into_iter().chain(parent).any(|n| is_window(n.as_str())) {
                    windows.insert(e);
                }
            }
        }
        if parts.is_empty() {
            continue; // The interior has not spawned yet.
        }
        let Some(ci) = BONES.iter().position(|b| b.name == "cockpit") else {
            continue;
        };
        // The screen wall's openings cut into the cabin behind it, as the eye sees them (`screen_cut`).
        let eye = named
            .get("EYE_pilot")
            .map(|&e| model_space(e, interior, &transforms, &parents).translation);
        let in_space = |e: Entity| model_space(e, interior, &transforms, &parents).to_matrix();
        let mut glass: Vec<[Vec3; 3]> = Vec::new();
        for &w in &windows {
            let Some(mesh) = meshes.get(w).ok().and_then(|m| assets.get(&m.0)) else {
                continue;
            };
            glass.extend(crate::screen_cut::triangles(mesh, in_space(w)));
        }
        commands.entity(interior).insert(CabinGlass(glass.clone()));
        let mut cut = 0;
        if let Some(eye) = eye {
            for &e in parts.difference(&windows) {
                let Ok(handle) = meshes.get(e) else { continue };
                if !cut_done.insert(handle.0.id()) {
                    continue;
                }
                let Some(new) = assets
                    .get(&handle.0)
                    .and_then(|m| crate::screen_cut::cut(m, in_space(e), eye, &glass))
                else {
                    continue;
                };
                let name = parents
                    .get(e)
                    .ok()
                    .and_then(|p| names.get(p.parent()).ok())
                    .map(|n| n.as_str().to_string());
                let before = assets.get(&handle.0).and_then(|m| m.indices()).map(|i| i.len() / 3).unwrap_or(0);
                debug!(
                    "screen wall cut: {name:?} {before} -> {} triangles",
                    new.indices().map(|i| i.len() / 3).unwrap_or(0)
                );
                if let Some(mut m) = assets.get_mut(&handle.0) {
                    *m = new;
                    cut += 1;
                }
            }
        } else {
            warn!("interior: no EYE_pilot, so the screen wall is not cut into the cabin");
        }
        let cockpit = rig.bone_entity(ci);
        commands
            .entity(interior)
            .insert((ChildOf(cockpit), Transform::IDENTITY, Visibility::Inherited));
        let rides = spec.interior.as_ref().map(|i| i.rides.as_slice()).unwrap_or_default();
        for r in rides {
            let (Some(&part), Some(&joint)) = (named.get(&r.part), rig.named.get(&r.joint)) else {
                warn!("interior: ride {} on {} names a node the models do not have", r.part, r.joint);
                continue;
            };
            // Both measured from the cockpit bone, the interior at its origin and the chest shut (a mech is
            // spawned standing and sealed).
            let at = model_space(part, interior, &transforms, &parents).to_matrix();
            let on = model_space(joint, cockpit, &transforms, &parents).to_matrix();
            commands
                .entity(part)
                .insert((ChildOf(joint), Transform::from_matrix(on.inverse() * at)));
        }
        // The interior's own lamps light its cabin (owner, 2026-10-24: "Need proper dashboard
        // lighting"): on the cockpit layer only, so they light the cabin and never the world, and
        // nothing outside reaches the cabin (lights are clustered on the CPU, which keeps each to its
        // layer). No shadows: they are cheap fills. Only the pilot's mech and the mechs they own (any of which they
        // may board) keep them; an enemy's cabin is never seen from inside.
        let keep = piloted.contains(root);
        for l in lamps {
            if !keep {
                commands.entity(l).despawn();
                continue;
            }
            let scale = data.0.cockpit_light.interior_lamp_scale;
            if let Ok(mut p) = points.get_mut(l) {
                p.shadow_maps_enabled = false;
                p.intensity *= scale;
            }
            if let Ok(mut sp) = spots.get_mut(l) {
                sp.shadow_maps_enabled = false;
                sp.intensity *= scale;
            }
            commands
                .entity(l)
                .insert(bevy::camera::visibility::RenderLayers::layer(crate::pilot::COCKPIT_LAYER));
        }
        info!(
            "cockpit interior bound: {} meshes, {} of them the screen wall ({} triangles, cut into {} meshes), {} rides",
            parts.len(),
            windows.len(),
            glass.len(),
            cut,
            rides.len()
        );
        commands
            .entity(root)
            .remove::<PendingInterior>()
            .insert(Interior { meshes: parts, windows });
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
    mut roots: Query<(Entity, &MechSim, &MechSpec, Option<&mut MechRig>, Has<Piloted>), With<MechRoot>>,
    mut transforms: Query<&mut Transform>,
    wrecks: Query<(), With<Wreck>>,
) {
    let alpha = fixed.overstep_fraction();
    for (root, sim, spec, rig, piloted) in &mut roots {
        let s = lerp_state(&sim.prev, &sim.curr, alpha);
        let tl = &data.0.boarding;
        let t = sim.hatch_prev.t_s + (sim.hatch.t_s - sim.hatch_prev.t_s) * alpha;
        if let Ok(mut t) = transforms.get_mut(root)
            && !wrecks.contains(root)
        {
            t.translation = s.pos;
            t.rotation = Quat::from_rotation_y(s.heading_rad);
        }
        let Some(mut rig) = rig else { continue };
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
        let mut rotations = p.rotations().to_vec();
        // A striking arm swings over the torso (close-combat).
        let m = &spec.melee;
        let swing = [0, 1].map(|a| {
            let (from, to) = (sim.strikes_prev.swing(m, a), sim.strikes.swing(m, a));
            from + (to - from) * alpha
        });
        rotations.extend(cb_core::mech::arm_rotations(p.torso, swing));
        let mut walk_local = Vec3::ZERO;
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
                let to_local = (d_parent * bind.parent_rest_model_rot).inverse();
                // The piloted mech's body rides level: the stride's bob and sway would swing the
                // cockpit round the pilot (owner, 2026-10-25: "theeres still massive sway", "int he
                // cockpit"). Other mechs keep their stride.
                if piloted {
                    t.translation += to_local * (p.pelvis_offset - p.walk_offset);
                } else {
                    t.translation += to_local * p.pelvis_offset;
                    walk_local = to_local * p.walk_offset;
                }
            }
        }
        rig.walk_local = walk_local;
    }
}

/// The boarding rope's foot: on the ground below the tip of the boarding crane, `reach_m` out in
/// front of the hatch's sill (away from the mech's root), at the mech's own ground height. The
/// climbing pilot's eye rides this line up the rope (`rope`).
pub fn rope_foot(mech_feet: Vec3, hatch: Vec3, reach_m: f32) -> Vec3 {
    let out = Vec2::new(hatch.x - mech_feet.x, hatch.z - mech_feet.z).normalize_or(Vec2::Y);
    Vec3::new(hatch.x + out.x * reach_m, mech_feet.y, hatch.z + out.y * reach_m)
}

fn publish_access(
    roots: Query<(&MechSim, &MechRig, &MechSpec), With<Piloted>>,
    globals: Query<&GlobalTransform>,
    data: Res<Data>,
    mut out: ResMut<MechAccessNow>,
) {
    out.0 = roots.iter().next().and_then(|(sim, rig, spec)| {
        let at = |n: &str| globals.get(rig.sockets[n]).ok().map(|g| g.translation());
        let hatch = at("HATCH_entry")?;
        Some(MechAccess {
            // The boarding rope's foot, under the crane's tip (`rope`). The kneel stops as far as the
            // legs can go, so the hatch stays high and the pilot is winched up (owner, 2026-10-24:
            // "there should be a rope dangling down for me the player to climb up", "extend a crane
            // out then lower the rope").
            ladder_base: rope_foot(sim.curr.pos, hatch, data.0.boarding.crane_reach_m),
            hatch_entry: hatch,
            eye_pilot: at("EYE_pilot")?,
            heading_rad: sim.curr.heading_rad,
            seat_yaw_rad: sim.curr.heading_rad + sim.curr.twist_rad,
            // The piloted mech's own data: it may be any owned design (mech-shop section 6.3).
            stopped: sim.curr.is_stopped(&spec.locomotion),
            speed_m_s: sim.curr.vel.length(),
            destroyed: sim.dead,
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
    let bodies: Vec<(u32, Body)> = roots
        .iter()
        .filter(|r| !r.0.dead)
        .map(|r| {
            let l = &r.1.locomotion;
            (
                r.2.0,
                Body {
                    feet: r.0.curr.pos,
                    radius_m: l.body_radius_m,
                    height_m: l.body_height_m,
                },
            )
        })
        .collect();
    let player = roots.iter().find(|r| r.5).map(|r| {
        (
            r.0.curr.pos,
            cb_core::mech::forward(r.0.curr.heading_rad) * r.0.curr.speed_m_s + Vec3::Y * r.0.curr.vel_y_m_s,
            r.0.dead,
            r.1.aim_height_m(),
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
        let mut fire = [false; 4];
        // Close combat's strikes, [left arm, right arm] (close-combat).
        let mut strike = [false; 2];
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
                            by: None,
                        });
                    }
                }
                None => {}
            }
            // Asked to board: turn to face the pilot while kneeling, so the hatch is toward them. Once
            // the kneel is complete it holds still (owner, 2026-10-25: "when mech has completed
            // kneeling animation it should no longer turn to face the player").
            let kneeling = sim.hatch.t_s < tl.sit.duration_s;
            if !seated && kneeling && sim.hatch.target == HatchTarget::Open && pilot.curr.mode == PilotMode::OnFoot {
                let to = pilot.curr.feet - sim.curr.pos;
                let want = (-to.x).atan2(-to.z);
                let err = (want - sim.curr.heading_rad + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                MechControls {
                    turn_axis: (err * 3.0).clamp(-1.0, 1.0),
                    ..default()
                }
            } else if seated && sim.hatch.can_walk() {
                // The mouse wheel steps the selected station, past any lost arm's gun (weapon-select).
                if actions.weapon_cycle != 0 {
                    let lost = |i: usize| sim.damage.is_lost(&cfg.parts, cfg.weapons[i].arm);
                    sim.selected = cb_core::weapons::next_station(&cfg.weapons, sim.selected, actions.weapon_cycle, |i| !lost(i));
                }
                // Close combat comes on when the targeted hostile is within melee range and Tab
                // toggles it (close-combat); only a targeted hostile counts.
                let near = targeting.selected.map(|_| targeting.range_m);
                sim.close.step(near, actions.close_combat, &cfg.melee);
                // Free look is for pressing the cockpit's MFDs: the guns hold (cockpit-mfds). In
                // close combat the guns keep firing and the punch keys strike with the arms (owner,
                // 2026-10-25: "Guns keep firing in close combat, and punches move to other keys").
                if actions.free_look {
                    fire = [false; 4];
                } else {
                    fire = [actions.fire[0], actions.fire[1], actions.fire_missile, actions.fire_back];
                    if sim.close.on {
                        strike = actions.strike;
                    }
                }
                // The torso turns to follow where the pilot looks (the view leads, the torso
                // follows); holding free look keeps the torso where it is.
                let mut c = MechControls::from_actions(&actions);
                // Boost thrust goes where the pilot looks.
                c.view_yaw_rad = pilot.curr.yaw_rad;
                // Holding Use to climb out, or a tap of free look on the ground, brings the mech to
                // a full stop: the throttle goes to zero and the legs brake (owner, 2026-10-25).
                if pilot.curr.exit_hold_s > 0.0 || (actions.full_stop && !sim.curr.airborne()) {
                    c.throttle_zero = true;
                    c.throttle_axis = 0.0;
                }
                // On deep water a tap of Ctrl turns skimming off, as C does, and the mech goes under (owner,
                // 2026-10-25: "ctrl above water turns skiming off as well").
                if actions.full_stop && matches!(sim.curr.on_water, OnWater::Skimming | OnWater::Hovering) {
                    c.sink = true;
                }
                // The mech turns toward the view, faster the further off its nose the view is
                // (seated-look): the view comes round no faster than the mech can turn.
                let limit = data.0.pilot.seated_look_limit_deg.to_radians();
                // The stronger of the keys' turn and the look's (skating, the keys still turn).
                if let Some(t) = cb_core::pilot::look_turn(pilot.curr.seated_rel_rad, limit, actions.free_look)
                    && t.abs() > c.turn_axis.abs()
                {
                    c.turn_axis = t;
                }
                // Walking, WASD is direct: toward the view, back from it, a step to either side; the
                // legs turn to the way to walk and the mech stops with no key held (walk-controls).
                // Skating and the jets keep the lever and the leg turn (owner, 2026-10-25: "tank
                // controls on skate makes sense, but when in walking mode, wasd should be standard
                // walking").
                if actions.walk_keys && sim.curr.mode == LocomotionMode::Walk && !sim.curr.skates_on && !sim.curr.airborne() {
                    let w = cb_core::mech::walk::walk(
                        actions.move_xy,
                        pilot.curr.yaw_rad,
                        sim.curr.heading_rad,
                        cfg.locomotion.throttle_min,
                    );
                    c.throttle_axis = 0.0;
                    if !c.throttle_zero {
                        c.throttle_set = Some(w.throttle);
                    }
                    if actions.move_xy.length() > 0.1 {
                        c.turn_axis = w.turn_axis;
                    }
                }
                if !actions.free_look {
                    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    let twist = wrap(pilot.curr.yaw_rad - sim.curr.heading_rad);
                    c.aim_delta_deg = Vec2::new(
                        -(twist - sim.curr.aim_twist_rad).to_degrees(),
                        (pilot.curr.pitch_rad - sim.curr.aim_pitch_rad).to_degrees(),
                    );
                }
                let others: Vec<Body> = bodies.iter().filter(|b| b.0 != id.0).map(|b| b.1).collect();
                aim_at = cam.map(|c| look_point(&c, &plan, live, &others));
                c
            } else {
                MechControls::default()
            }
        } else if !seated && !std::env::var("CB_ENEMY_STUNT").is_ok_and(|v| v == "hover") {
            // The enemies wait until the pilot is in the seat: no one fights an empty mech. (The hover
            // test instrument flies them anyway, so an on-foot still can look at their jets.)
            MechControls::default()
        } else if let (Some((mut enemy, goal)), Some((player_at, player_vel, player_dead, player_aim_m))) = (enemy, player) {
            // A mission may send it at something else (city-defence: a building to shell).
            let (target, target_vel, target_dead, aim_height_m) = match goal.point {
                Some(p) => (p, Vec3::ZERO, false, cfg.aim_height_m()),
                None => (player_at, player_vel, player_dead, player_aim_m),
            };
            let eye = sim.curr.pos + Vec3::Y * cfg.body.eye_height_m;
            // The main gun leads the aim: a beam at the target itself, a shell with its drop (weapon-kinds).
            let main = cfg.weapons.first();
            let kind = main.map_or_else(Default::default, |w| w.shot());
            let speed = kind.lead_speed_m_s(main.map_or(850.0, |w| w.speed_m_s));
            let allies: Vec<Vec3> = bodies.iter().filter(|b| b.0 != id.0 && b.0 != 0).map(|b| b.1.feet).collect();
            let hit = std::mem::take(&mut sim.took_hit);
            let health = torso_health(&sim, cfg);
            let clear = |a: Vec3, b: Vec3| line_clear(&plan, live, a, b);
            let cover = |me: Vec3, threat: Vec3| cover_point(&plan, live, me, threat);
            let radius = cfg.locomotion.body_radius_m;
            let open = |a: Vec2, b: Vec2| path_open(&plan, a, b, radius);
            let view = cb_core::ai::View {
                eye,
                target,
                aim_height_m,
                target_vel,
                muzzle_speed_m_s: speed,
                drop: !kind.straight(),
                kind,
                heat: sim.guns.heat.heat,
                health,
                hit,
                allies: &allies,
                clear: &clear,
                cover: &cover,
                open: &open,
            };
            let orders = enemy.ai.orders(&sim.curr, &view, dt);
            if let Some(c) = stunt(
                &sim.curr,
                &plan,
                player_at,
                &mut enemy.stunt_axis,
                time.elapsed_secs() + id.0 as f32 * 7.0,
            ) {
                aim_at = Some(orders.aim_point);
                c
            } else {
                if !target_dead && !goal.hold_fire {
                    fire = [orders.fire[0], orders.fire[1], false, false];
                }
                aim_at = Some(orders.aim_point);
                orders.controls
            }
        } else {
            MechControls::default()
        };

        let ground = Scene {
            plan: &plan,
            dug: &live.dug,
            trees: None,
            structures: Some(&live.structures),
            platforms: &live.platforms,
            interiors: None,
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
        if !(piloted && seated) {
            sim.close = Default::default();
        }
        // Strikes: each arm on its own clock; one landing this step sweeps from its shoulder out to
        // its reach, toward the aim point (close-combat). A lost arm cannot strike.
        sim.strikes_prev = sim.strikes;
        let arm_lost = [PartKind::ArmL, PartKind::ArmR].map(|k| damage.is_lost(&cfg.parts, k));
        let lands = sim.strikes.step(&cfg.melee, strike, arm_lost, dt);
        for (arm, shoulder) in ["shoulder_L", "shoulder_R"].into_iter().enumerate() {
            if !lands[arm] {
                continue;
            }
            let Some(from) = BONES
                .iter()
                .position(|b| b.name == shoulder)
                .and_then(|i| globals.get(rig.bone_entity(i)).ok())
                .map(|g| g.translation())
            else {
                continue;
            };
            let ahead = cb_core::mech::forward(sim.curr.heading_rad + sim.curr.twist_rad);
            let dir = aim_at.map_or(ahead, |t| (t - from).normalize_or(ahead));
            shots.1.push(crate::combat::Blow {
                owner: id.0,
                from,
                to: from + dir * cfg.melee.reach_m,
                penetration_mm: cfg.melee.penetration_mm,
                damage: cfg.melee.damage,
            });
        }
        // Guns: fire from the muzzle toward the aim point; a lost arm's gun is gone.
        // The pilot's left mouse fires the selected station; the enemy AI fires by group (weapon-select).
        let pulls = if piloted {
            cb_core::weapons::station_pulls(&cfg.weapons, sim.selected, fire)
        } else {
            cb_core::weapons::group_pulls(&cfg.weapons, fire)
        };
        let fired = sim
            .guns
            .trigger(&cfg.weapons, &pulls, |k| damage.is_lost(&cfg.parts, k), &cfg.weapon_heat, dt);
        let mut rng = cb_core::weapons::shot_rng(id.0, sim.guns.pulls);
        for wi in fired {
            let w = &cfg.weapons[wi];
            // The weapon's own socket when the model has it (a back unit's barrel, a pod's mouth), else the
            // contract muzzle of its arm; a missile without a pod leaves from the right arm.
            let named = rig.socket(&w.socket).or_else(|| rig.named(&w.socket));
            let fallback = if w.arm == PartKind::ArmL { "MUZZLE_L" } else { "MUZZLE_R" };
            let Some(muzzle) = named
                .or_else(|| rig.socket(fallback))
                .and_then(|e| globals.get(e).ok())
                .map(|g| g.translation())
            else {
                continue;
            };
            let target = aim_at.unwrap_or(muzzle + cb_core::mech::forward(sim.curr.heading_rad + sim.curr.twist_rad) * 500.0);
            let dir = (target - muzzle).normalize_or(Vec3::NEG_Z);
            let kind = w.shot();
            if let cb_core::weapons::ShotKind::Beam { range_m } = kind {
                // A laser burns this step's share of its damage into the first thing on its line (weapon-kinds).
                shots.2.push(crate::combat::Beam {
                    owner: id.0,
                    from: muzzle,
                    dir,
                    range_m,
                    penetration_mm: w.penetration_mm,
                    damage: w.damage * dt,
                });
                continue;
            }
            let carry = cb_core::mech::forward(sim.curr.heading_rad) * sim.curr.speed_m_s;
            for vel in cb_core::weapons::launch(kind, dir, w.speed_m_s, &mut rng) {
                shots.0.push(Projectile {
                    owner: id.0,
                    weapon: wi,
                    pos: muzzle,
                    vel: vel + carry,
                    age_s: 0.0,
                    rocket: kind.straight(),
                    // A missile homes on the pilot's lock (targeting); unlocked it flies straight.
                    target: (piloted && w.seeker_turn_deg_s > 0.0).then_some(locked).flatten(),
                });
            }
            let recipe = if kind.straight() {
                "muzzle_rocket"
            } else if w.blast_mj >= 1.0
                || matches!(
                    kind,
                    cb_core::weapons::ShotKind::Pellets { .. } | cb_core::weapons::ShotKind::Slug { .. }
                )
            {
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
        // A mech that rams a wall at speed smashes through it; one that only walks into it is
        // stopped and breaks nothing (mech-ramming: `RamConfig::hole_radius_m`).
        let ram = &cfg.ram;
        let hole = if sim.curr.airborne() {
            None
        } else {
            ram.hole_radius_m(sim.prev.speed_m_s, sim.curr.speed_m_s)
        };
        if let Some(radius_m) = hole {
            let front = sim.curr.pos
                + cb_core::mech::forward(sim.curr.heading_rad) * (cfg.locomotion.body_radius_m + ram.reach_m)
                + Vec3::Y * ram.height_m;
            let p2 = Vec2::new(front.x, front.z);
            for b in plan.collision.candidates(p2, radius_m) {
                let (broke, chunks) = live.structures.break_sphere(b, front, radius_m);
                if broke > 0 {
                    live.dirty_buildings.push(b);
                    live.new_chunks.extend(chunks);
                    sim.curr.speed_m_s = sim.prev.speed_m_s * ram.kept_fraction;
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
/// True when a mech `radius_m` across its middle can walk the straight line from `a` to `b` (x, z)
/// without meeting a building: sampled every 3 m against the buildings' footprints, as the legs
/// collide (`push_out_at`). What the AI's legs look ahead with.
pub fn path_open(plan: &cb_core::world::WorldPlan, a: Vec2, b: Vec2, radius_m: f32) -> bool {
    let steps = (a.distance(b) / 3.0).ceil().max(1.0) as usize;
    (1..=steps).all(|s| {
        let p = a.lerp(b, s as f32 / steps as f32);
        let y = plan.terrain.height_at(p.x, p.y);
        plan.collision.push_out_at(p, radius_m, y).distance(p) < 1e-3
    })
}

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

/// A mech's body for the pilot's look ray: a column from its feet, its own radius and height.
#[derive(Clone, Copy)]
struct Body {
    feet: Vec3,
    radius_m: f32,
    height_m: f32,
}

/// Where the pilot is looking: the first mech body, ground, water or standing building cell along
/// the view, or 1.5 km out.
fn look_point(camera: &GlobalTransform, plan: &cb_core::world::WorldPlan, live: &Live, mechs: &[Body]) -> Vec3 {
    let ground = Scene {
        plan,
        dug: &live.dug,
        trees: None,
        structures: Some(&live.structures),
        platforms: &live.platforms,
        interiors: None,
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
        // A mech: a column its body's radius round, from its feet to its top.
        let mech = mechs
            .iter()
            .any(|m| Vec2::new(m.feet.x, m.feet.z).distance(p2) < m.radius_m && p.y > m.feet.y && p.y < m.feet.y + m.height_m);
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
        platforms: &[],
        interiors: None,
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
    /// Height of its centre of mass over its feet, for the topple.
    centre_m: f32,
    vel: Vec3,
    axis: Vec3,
    angle_rad: f32,
    rate_rad_s: f32,
    heading_rad: f32,
    /// Its mass, tonnes, for the dust it throws when it lands.
    mass_t: f32,
    /// It has thrown its dust.
    dusted: bool,
}

/// It comes to rest this far over: lying on its side.
const WRECK_LIE_RAD: f32 = 1.45;

fn fall_wrecks(
    time: Res<Time>,
    plan: Res<Plan>,
    data: Res<Data>,
    mut fx: MessageWriter<Fx>,
    mut wrecks: Query<(&mut Wreck, &mut Transform)>,
) {
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
                w.rate_rad_s += 1.5 * 9.81 * w.angle_rad.sin() / w.centre_m * dt;
                w.angle_rad = (w.angle_rad + w.rate_rad_s * dt).min(WRECK_LIE_RAD);
            } else if !w.dusted {
                // It hits the ground along its length: dust where its middle lands (openspec change `mech-damage-fx`).
                w.dusted = true;
                let along = (t.rotation * Vec3::Y).with_y(0.0) * w.centre_m;
                let scale = crate::battle_fx::dust_scale(&data.0.battle_fx.damage, w.mass_t);
                fx.write(Fx::new("fall_dust", t.translation + along).scaled(scale));
            }
        }
        t.rotation = Quat::from_axis_angle(w.axis, w.angle_rad) * Quat::from_rotation_y(w.heading_rad);
    }
}

/// Cuts lost parts off the rig: the bone and everything under it become a falling wreck; the stump
/// burns.
fn cut_parts(
    mut commands: Commands,
    mut roots: Query<(Entity, &mut MechSim, &mut MechRig, &MechSpec, Option<&mut crate::skins::SkinLimbs>)>,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    time: Res<Time>,
    data: Res<Data>,
    mut fx: MessageWriter<Fx>,
    mut explosions: MessageWriter<Explosion>,
) {
    let _t = crate::perf_hud::slow("cut_parts");
    let burn = &data.0.battle_fx.damage;
    for (root, mut sim, mut rig, mech, mut skin) in &mut roots {
        for part in std::mem::take(&mut sim.cuts) {
            let spec = &mech.parts[part];
            if spec.kind == PartKind::Torso {
                // The torso is gone: the mech blows up where it stands and burns.
                let at = globals.get(root).map(|g| g.translation()).unwrap_or_default() + Vec3::Y * mech.body.blast_height_m;
                explosions.write(Explosion {
                    at,
                    energy_mj: 25.0,
                    by: None,
                });
                // The wreck falls to the ground and topples from its feet (owner, 2026-10-24: dead
                // mechs "stuck floating in the air"), away from the side it was facing, at random.
                let side = if root.index().index() % 2 == 0 { 1.0 } else { -1.0 };
                let heading = sim.curr.heading_rad;
                commands.entity(root).insert(Wreck {
                    centre_m: mech.body.wreck_centre_m,
                    vel: Vec3::new(sim.curr.vel.x, sim.curr.vel_y_m_s, sim.curr.vel.y),
                    axis: Quat::from_rotation_y(heading) * Vec3::new(0.0, 0.0, side),
                    angle_rad: 0.05,
                    rate_rad_s: 0.0,
                    heading_rad: heading,
                    mass_t: mech.locomotion.mass_kg / 1000.0,
                    dusted: false,
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
            let centre = globals.get(rig.bones[0].entity).map(|r| r.translation()).unwrap_or(at);
            // A skinned enemy throws its skin's limb instead; the stand-in's bone is not drawn and stays put.
            let thrown = skin
                .as_deref_mut()
                .and_then(|s| crate::skins::throw_limb(&mut commands, s, spec.kind, &globals, centre, mech.body.cut_box_m));
            let now = time.elapsed_secs();
            let box_t = mech.body.cut_box_m.iter().product::<f32>() * 8.0 * 0.8;
            if let Some(t) = &thrown {
                // The thrown limb burns where it lands and throws dust; its stump burns where the limb hung
                // (openspec change `mech-damage-fx`: on a skin the stand-in's bone is inside the skin, unseen).
                let mass_t = box_t * t.scale.powi(3);
                commands.entity(t.limb).insert((
                    Burning {
                        until_s: now + burn.limb_burn_s,
                    },
                    crate::battle_fx::Falling { mass_t },
                ));
                commands.spawn((
                    Name::new("stump fire"),
                    Transform::from_translation(t.pivot),
                    Burning {
                        until_s: now + burn.stump_burn_s,
                    },
                    ChildOf(t.parent),
                ));
            }
            let thrown = thrown.is_some();
            let out = (at - centre).with_y(0.0).normalize_or(Vec3::X);
            // The wreck falls under physics (debris-physics): a box about the part's size.
            if !thrown && skin.is_none() {
                commands.entity(e).remove::<ChildOf>().insert((
                    g.compute_transform(),
                    RigidBody::Dynamic,
                    Collider::cuboid(mech.body.cut_box_m[0], mech.body.cut_box_m[1], mech.body.cut_box_m[2]),
                    LinearVelocity(out * 6.0 + Vec3::Y * 5.0),
                    AngularVelocity(Vec3::new(1.5, 0.7, -1.1)),
                    crate::battle_fx::Falling { mass_t: box_t },
                ));
            }
            if !thrown && let Ok(stump) = parents.get(e).map(|p| p.parent()).or(Ok::<Entity, ()>(e)) {
                commands.entity(stump).insert(Burning {
                    until_s: now + burn.stump_burn_s,
                });
            }
            fx.write(Fx::new("part_lost", at));
        }
    }
}

/// A test instrument: `CB_ENEMY_STUNT=1` flies every enemy back and forth along the line from the
/// player toward where it first stood (down the street it started in, so the street's view is
/// open along it), 120 m to 400 m out, 30 m up on the rockets with the skates pushing, holding
/// fire. When the rockets lock it comes down and skates on. Its jet smoke and skate trails can then
/// be checked from the seat (owner, 2026-10-25: "do other mechs and do my mech while I'm in the
/// pilot seat").
fn stunt(m: &MechState, plan: &Plan, player: Vec3, axis: &mut Option<Vec2>, t_s: f32) -> Option<MechControls> {
    if !crate::cli::switched_off("CB_ENEMY_STUNT") {
        return None;
    }
    let me = Vec2::new(m.pos.x, m.pos.z);
    let home = Vec2::new(player.x, player.z);
    let axis = *axis.get_or_insert_with(|| (me - home).normalize_or(Vec2::NEG_Y));
    // Toward the player for 12 s, then away for 12 s; `CB_ENEMY_STUNT=hover` holds it 120 m out, 40 m
    // up, so its jets and the column of smoke under it are in plain view.
    let hold = std::env::var("CB_ENEMY_STUNT").is_ok_and(|v| v == "hover");
    let goal = home + axis * if hold || t_s.rem_euclid(24.0) < 12.0 { 120.0 } else { 400.0 };
    let to = goal - me;
    let want = (-to.x).atan2(-to.y);
    let off = (want - m.heading_rad + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let agl = m.pos.y - plan.height_at(m.pos.x, m.pos.z);
    Some(MechControls {
        skate_toggle: !m.skates_on,
        turn_axis: (off * 2.0).clamp(-1.0, 1.0),
        view_yaw_rad: want,
        lift_axis: if agl < if hold { 40.0 } else { 30.0 } {
            1.0
        } else {
            (0.175 - m.vel_y_m_s * 0.025).clamp(0.0, 1.0)
        },
        move_xy: if m.airborne() && !(hold && to.length() < 20.0) {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::ZERO
        },
        throttle_set: Some(1.0),
        ..default()
    })
}

/// While the pilot sits in the mech with the hatch shut, every mesh of the mech is on the cockpit
/// layer, drawn after the world on its own depth so nothing in the world draws through it
/// (cockpit-overlay; owner: "only when it is closed and player is inside"). Otherwise it is drawn
/// with the world like anything else. Walks the mech's descendants each frame, so a model that
/// loads late is caught.
/// A mesh hung under a mech that sets its own render layers (an MFD's outline, `mfd-menus`): the cockpit pass leaves it
/// alone.
#[derive(Component)]
pub struct OwnLayers;

fn cockpit_layer(
    mut commands: Commands,
    pilot: Res<PilotSim>,
    data: Res<Data>,
    mech: Query<(Entity, &MechSim, Option<&Interior>), With<Piloted>>,
    children: Query<&Children>,
    meshes: Query<Option<&bevy::camera::visibility::RenderLayers>, (With<Mesh3d>, Without<OwnLayers>)>,
) {
    use bevy::camera::visibility::RenderLayers;
    let Some((root, sim, interior)) = mech.iter().next() else { return };
    let inside = pilot.curr.mode == PilotMode::Seated && sim.hatch.is_closed(&data.0.boarding);
    let cabin = RenderLayers::layer(crate::pilot::COCKPIT_LAYER);
    let unseen = RenderLayers::layer(UNSEEN_LAYER);
    for e in children.iter_descendants(root) {
        let Ok(layers) = meshes.get(e) else { continue };
        // A mech with an interior shows the cabin alone from the seat (mech-roster section 2): the screen wall
        // and the hull, which stand between the pilot and the world, are not drawn, so the world shows through.
        let want = match (inside, interior) {
            (false, _) => None,
            (true, None) => Some(&cabin),
            (true, Some(i)) if i.meshes.contains(&e) && !i.windows.contains(&e) => Some(&cabin),
            (true, Some(_)) => Some(&unseen),
        };
        match (want, layers) {
            (Some(w), Some(l)) if l == w => {}
            (Some(w), _) => {
                commands.entity(e).insert(w.clone());
            }
            (None, Some(_)) => {
                commands.entity(e).remove::<RenderLayers>();
            }
            (None, None) => {}
        }
    }
}

/// The booster flame while boosting: a stream on each jet socket (`FxStream`), switched with the
/// boost.
/// A jet socket's smoke trail: the child entity streaming `jet_trail`, kept apart
/// from the socket's own flame stream so each has its own rate.
#[derive(Component)]
struct JetTrail(Entity);

/// A jet's exhaust flame, under its nozzle socket (openspec change `rocket-plumes`).
#[derive(Component)]
struct JetPlume;

#[allow(clippy::too_many_arguments)]
fn jet_flames(
    mut commands: Commands,
    mechs: Query<(&MechSim, &MechRig)>,
    trails: Query<&JetTrail>,
    mut streams: Query<&mut crate::fx::FxStream>,
    mut plumes: Query<(&mut Transform, &mut Visibility), With<JetPlume>>,
    children: Query<&Children>,
    globals: Query<&GlobalTransform>,
    plume: Res<crate::plumes::PlumeAssets>,
    data: Res<Data>,
    plan: Res<Plan>,
    time: Res<Time>,
    mut fx: MessageWriter<Fx>,
    mut wash_s: Local<f32>,
) {
    let cfg = &data.0.fx;
    *wash_s += time.delta_secs();
    let wash = *wash_s >= 0.1;
    if wash {
        *wash_s = 0.0;
    }
    for (sim, rig) in &mechs {
        let on = sim.curr.boosting && !sim.dead;
        // The nozzles point against the rockets' push, swivelled by WASD (`MechState::thrust_tilt`).
        let tilt = sim.curr.thrust_tilt;
        let nozzle_world = -Vec3::new(tilt.x, (1.0 - tilt.length_squared()).max(0.0).sqrt(), tilt.y);
        for jet in ["JET_L", "JET_R"] {
            let Some(&e) = rig.sockets.get(jet) else { continue };
            // In the socket's own frame, where straight down is -y.
            let nozzle = globals.get(e).map_or(Vec3::NEG_Y, |g| {
                (g.to_scale_rotation_translation().1.inverse() * nozzle_world).normalize_or(Vec3::NEG_Y)
            });
            let trail = trails.get(e).ok().map(|t| t.0);
            match (streams.get(e).is_ok(), trail) {
                (true, Some(t)) => {
                    for s in [e, t] {
                        if let Ok(mut s) = streams.get_mut(s) {
                            if s.on != on {
                                s.on = on;
                            }
                            if s.dir != nozzle {
                                s.dir = nozzle;
                            }
                        }
                    }
                }
                _ => {
                    let child = commands
                        .spawn((
                            Name::new("jet trail"),
                            Transform::default(),
                            crate::fx::FxStream {
                                recipe: "jet_trail",
                                dir: Vec3::NEG_Y,
                                on,
                                density: 1.0,
                            },
                        ))
                        .id();
                    commands.entity(e).add_child(child).insert((
                        crate::fx::FxStream {
                            recipe: "jet_flame",
                            dir: Vec3::NEG_Y,
                            on,
                            density: 1.0,
                        },
                        JetTrail(child),
                    ));
                }
            }
            // The flame under the nozzle, flickering in length while the jets fire.
            let flicker = 0.85 + 0.3 * (time.elapsed_secs() * 37.0 + e.index().index() as f32).sin().abs();
            let shape = crate::plumes::placed(Vec3::ZERO, nozzle, cfg.jet_plume_m[0] * flicker, cfg.jet_plume_m[1]);
            let mine = children.get(e).ok().and_then(|c| c.iter().find(|c| plumes.contains(*c)));
            match mine.and_then(|p| plumes.get_mut(p).ok()) {
                Some((mut t, mut v)) => {
                    *t = shape;
                    *v = if on { Visibility::Inherited } else { Visibility::Hidden };
                }
                None => {
                    commands.spawn((
                        Name::new("jet plume"),
                        Mesh3d(plume.mesh.clone()),
                        MeshMaterial3d(plume.jet.clone()),
                        shape,
                        Visibility::Hidden,
                        bevy::light::NotShadowCaster,
                        JetPlume,
                        ChildOf(e),
                    ));
                }
            }
        }
        // Low over the ground the jets blow the dust out in a ring.
        let p = sim.curr.pos;
        let ground = plan.height_at(p.x, p.z);
        if on && wash && p.y - ground < cfg.jet_wash_m {
            fx.write(Fx::new("jet_wash", Vec3::new(p.x, ground, p.z)));
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
            density: 1.0,
        });
        if !on {
            commands.entity(e).remove::<Burning>();
        }
    }
}

/// The world transform of a rig socket this frame, composed from local transforms (so it already
/// includes this frame's pose, before Bevy propagates transforms), without the stride's bob and sway
/// of the pelvis: where the socket would be with the body carried level. The seated pilot's eye
/// (owner, 2026-10-25: "I would remove the footstep motion forom the cokcpit entirely").
pub fn socket_steady(
    rig: &MechRig,
    name: &str,
    transforms: &Query<&Transform, Without<crate::pilot::PilotCamera>>,
    parents: &Query<&ChildOf>,
) -> Option<Transform> {
    let pelvis = BONES.iter().position(|b| b.name == "pelvis").map(|i| rig.bones[i].entity);
    let local = |e: Entity| {
        let mut t = transforms.get(e).ok().copied().unwrap_or_default();
        if Some(e) == pelvis {
            t.translation -= rig.walk_local;
        }
        t
    };
    let e = *rig.sockets.get(name)?;
    transforms.get(e).ok()?;
    let mut acc = local(e);
    let mut at = e;
    while let Ok(p) = parents.get(at) {
        at = p.parent();
        acc = local(at).mul_transform(acc);
    }
    Some(acc)
}
