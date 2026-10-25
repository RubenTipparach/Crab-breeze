//! The battle's lasting marks and damage effects (openspec changes `blast-scorch` and `mech-damage-fx`; owner,
//! 2026-10-25: "rocket explosions on dirt need work, need proper textures made from material maker", "we need decals on
//! mechs too when they get hit", "so we can see damage reflected on them", "when parts fly off or critical damage, the
//! mech needs to be on fire and smoking", "when they fall they also should kick up dust").
//!
//! - A crater dug in soil gets a forward decal of the Material Maker `blast_scorch` maps (one shared material, so every
//!   scorch batches), held for its life and capped.
//! - A hit on a mech leaves a decal from the `hit_marks` sheet on the node that draws the struck part (a skin's limb, or
//!   the rig's bone), so it moves with the part and leaves with it when it is thrown. Capped per mech and in all.
//! - A part under a quarter of its structure smokes; a mech under a quarter in all burns from the torso.
//! - A body marked `Falling` (a thrown limb, a cut bone) throws `fall_dust` where it first comes down.
//!
//! Presentation only: the simulation decides nothing here. The numbers are `data/fx/battle_fx.ron`.

use crate::fx::{Fx, FxStream};
use crate::mech::{MechId, MechRig, MechRoot, MechSim, MechSpec};
use crate::sim::{Data, PresentSet};
use crate::skins::SkinLimbs;
use crate::world::{Live, Plan};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::{Affine2, Affine3A};
use bevy::pbr::ExtendedMaterial;
use bevy::pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt};
use bevy::prelude::*;
use cb_core::battle_fx::DamageFxConfig;
use cb_core::mech::damage::PartKind;
use cb_core::world::{Ground, Surface};
use std::collections::VecDeque;

/// A shot that struck a mech part this step (written by `combat::fly_shots`).
#[derive(Message, Clone, Copy, Debug)]
pub struct MechHit {
    /// The mech's id.
    pub mech: u32,
    /// The part's index in its mech's part list.
    pub part: usize,
    /// Where it struck.
    pub at: Vec3,
    /// The shot's direction of travel.
    pub dir: Vec3,
    /// The weapon's penetration, millimetres.
    pub penetration_mm: f32,
    /// The weapon's blast, megajoules.
    pub blast_mj: f32,
}

/// A free body that throws dust where it first comes down: a thrown limb or a cut bone.
#[derive(Component)]
pub struct Falling {
    /// Its mass, tonnes.
    pub mass_t: f32,
}

/// The dust scale of a body of `mass_t` tonnes landing: the cube root of its mass over the config's, within 0.3 and 3.
pub fn dust_scale(cfg: &DamageFxConfig, mass_t: f32) -> f32 {
    (mass_t.max(0.0) / cfg.dust_per_cbrt_t).cbrt().clamp(0.3, 3.0)
}

type DecalMaterial = ForwardDecalMaterial<StandardMaterial>;

/// The decal materials: the scorch, and one hit mark per cell of the 4 by 4 sheet (row = `HitMark`).
#[derive(Resource)]
struct DecalAssets {
    scorch: Option<Handle<DecalMaterial>>,
    hits: Vec<Handle<DecalMaterial>>,
}

/// The decals standing, oldest first, with when they go.
#[derive(Resource, Default)]
struct Standing {
    scorches: VecDeque<(Entity, f32)>,
    hits: VecDeque<Entity>,
    blasts_seen: usize,
}

/// A mech's hit marks, oldest first, and which of its parts already smoke.
#[derive(Component, Default)]
struct Battered {
    marks: VecDeque<Entity>,
    smoking: Vec<bool>,
    hull: bool,
}

/// The battle's marks and damage effects.
pub struct BattleFxPlugin;

impl Plugin for BattleFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MechHit>()
            .init_resource::<Standing>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (scorch_craters, mark_hits, smoke_damaged, land_dust).in_set(PresentSet::Pose),
            );
    }
}

/// Loads a decal map (`assets/textures/fx/<name>/<map>.png`), clamped at its edges so the decal's projection never
/// wraps around.
fn decal_map(root: &std::path::Path, images: &mut Assets<Image>, name: &str, map: &str, srgb: bool) -> Option<Handle<Image>> {
    let mut img = crate::pbr_set::load_tiling(root, &format!("textures/fx/{name}/{map}.png"), srgb)?;
    if let ImageSampler::Descriptor(d) = &mut img.sampler {
        d.address_mode_u = ImageAddressMode::ClampToEdge;
        d.address_mode_v = ImageAddressMode::ClampToEdge;
    } else {
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    }
    Some(images.add(img))
}

/// A decal material of a Material Maker set, its UV moved to `cell` of a `grid` by `grid` sheet.
fn decal_material(
    set: &(Handle<Image>, Option<Handle<Image>>, Option<Handle<Image>>),
    cell: (u32, u32),
    grid: u32,
    depth_fade_m: f32,
) -> DecalMaterial {
    let k = 1.0 / grid as f32;
    ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(set.0.clone()),
            normal_map_texture: set.1.clone(),
            occlusion_texture: set.2.clone(),
            metallic_roughness_texture: set.2.clone(),
            perceptual_roughness: 1.0,
            metallic: 1.0,
            alpha_mode: AlphaMode::Blend,
            uv_transform: Affine2::from_scale_angle_translation(Vec2::splat(k), 0.0, Vec2::new(cell.0 as f32, cell.1 as f32) * k),
            ..default()
        },
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: depth_fade_m,
        },
    }
}

fn setup(mut commands: Commands, data: Res<Data>, mut images: ResMut<Assets<Image>>, mut materials: ResMut<Assets<DecalMaterial>>) {
    let cfg = &data.0.battle_fx;
    let root = crate::cli::repo_root().join("assets");
    let mut set = |name: &str| {
        let albedo = decal_map(&root, &mut images, name, "albedo", true)?;
        Some((
            albedo,
            decal_map(&root, &mut images, name, "normal", false),
            decal_map(&root, &mut images, name, "orm", false),
        ))
    };
    let scorch = set("blast_scorch").map(|s| materials.add(decal_material(&s, (0, 0), 1, cfg.scorch.depth_fade_m)));
    let hits = set("hit_marks").map_or_else(Vec::new, |s| {
        (0..16)
            .map(|i| materials.add(decal_material(&s, (i % 4, i / 4), 4, cfg.hit_marks.depth_fade_m)))
            .collect()
    });
    commands.insert_resource(DecalAssets { scorch, hits });
}

/// A scorch decal over every crater dug in soil since the last frame; the oldest go past the cap and at the end of
/// their life.
fn scorch_craters(
    mut commands: Commands,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    assets: Res<DecalAssets>,
    time: Res<Time>,
    mut standing: ResMut<Standing>,
) {
    let cfg = &data.0.battle_fx.scorch;
    let now = time.elapsed_secs();
    while standing.scorches.front().is_some_and(|(_, until)| *until < now) {
        if let Some((e, _)) = standing.scorches.pop_front() {
            commands.entity(e).despawn();
        }
    }
    let blasts = &live.dug.blasts;
    let fresh = blasts.len().saturating_sub(standing.blasts_seen);
    standing.blasts_seen = blasts.len();
    let Some(material) = &assets.scorch else { return };
    let craters = &data.0.world.craters;
    for b in blasts.iter().skip(blasts.len() - fresh) {
        if plan.0.surface_at(b.centre) != Surface::Dirt {
            continue;
        }
        let r = craters.radius_m_per_cbrt_mj * b.energy_mj.max(0.0).cbrt();
        let side = cfg.scorch_per_radius * r;
        let ground = plan.height_at(b.centre.x, b.centre.y) + craters.rim_ratio * r;
        let yaw = (b.centre.x * 0.37 + b.centre.y * 0.61).sin() * std::f32::consts::PI;
        let e = commands
            .spawn((
                Name::new("blast scorch"),
                ForwardDecal,
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(b.centre.x, ground, b.centre.y)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::new(side, 1.0, side)),
            ))
            .id();
        standing.scorches.push_back((e, now + cfg.life_s));
        while standing.scorches.len() > cfg.max_scorches {
            if let Some((old, _)) = standing.scorches.pop_front() {
                commands.entity(old).despawn();
            }
        }
    }
}

/// The node that draws part `part` of a mech: a skin's limb or root, or the rig's bone.
fn node_for(spec: &MechSpec, part: usize, rig: Option<&MechRig>, skin: Option<&SkinLimbs>) -> Option<Entity> {
    let kind = spec.parts.get(part)?.kind;
    if let Some(skin) = skin {
        return Some(skin.node_for(kind));
    }
    let rig = rig?;
    match kind {
        PartKind::Torso => Some(rig.torso()),
        _ => rig.named(&spec.parts[part].cut_bone).or(Some(rig.torso())),
    }
}

/// A hit-mark decal on the node that draws each struck part.
#[allow(clippy::too_many_arguments)]
fn mark_hits(
    mut commands: Commands,
    mut hits: MessageReader<MechHit>,
    data: Res<Data>,
    assets: Res<DecalAssets>,
    globals: Query<&GlobalTransform>,
    mut mechs: Query<(
        Entity,
        &MechId,
        &MechSpec,
        Option<&MechRig>,
        Option<&SkinLimbs>,
        Option<&mut Battered>,
    )>,
    mut standing: ResMut<Standing>,
) {
    let cfg = &data.0.battle_fx.hit_marks;
    for hit in hits.read() {
        if assets.hits.len() < 16 {
            continue;
        }
        let Some((root, _, spec, rig, skin, battered)) = mechs.iter_mut().find(|m| m.1.0 == hit.mech) else {
            continue;
        };
        let Some(node) = node_for(spec, hit.part, rig, skin) else {
            continue;
        };
        let Ok(parent) = globals.get(node) else { continue };
        let seed = (hit.at.x * 7.13 + hit.at.y * 3.71 + hit.at.z * 13.37)
            .to_bits()
            .wrapping_mul(2_654_435_761);
        let kind = cfg.hit_kind(hit.penetration_mm, hit.blast_mj, seed >> 8);
        let column = (seed >> 4) % 4;
        let scale = skin.map_or(1.0, SkinLimbs::scale);
        let size = cfg.size_m[kind as usize] * scale;
        let facing = Quat::from_rotation_arc(Vec3::Y, -hit.dir.normalize_or(Vec3::Y));
        let turn = Quat::from_rotation_y((seed % 628) as f32 / 100.0);
        let world = Affine3A::from_scale_rotation_translation(Vec3::new(size, size, size), facing * turn, hit.at);
        let local = Transform::from_matrix(Mat4::from(parent.affine().inverse() * world));
        let cell = kind as usize * 4 + column as usize;
        let e = commands
            .spawn((
                Name::new(format!("hit mark {kind:?}")),
                ForwardDecal,
                MeshMaterial3d(assets.hits[cell].clone()),
                local,
                ChildOf(node),
            ))
            .id();
        let mut fresh = Battered::default();
        let own = match battered {
            Some(b) => &mut b.into_inner().marks,
            None => &mut fresh.marks,
        };
        own.push_back(e);
        while own.len() > cfg.per_mech {
            if let Some(old) = own.pop_front() {
                commands.entity(old).try_despawn();
            }
        }
        if !fresh.marks.is_empty() {
            commands.entity(root).insert(fresh);
        }
        standing.hits.push_back(e);
        while standing.hits.len() > cfg.max {
            if let Some(old) = standing.hits.pop_front() {
                commands.entity(old).try_despawn();
            }
        }
    }
}

/// A part under the critical share of its structure smokes from the node that draws it; a mech under it in all
/// burns from its torso. Each starts once and keeps going (a lost part's stump has its own fire).
fn smoke_damaged(
    mut commands: Commands,
    data: Res<Data>,
    mut mechs: Query<
        (
            Entity,
            &MechSim,
            &MechSpec,
            Option<&MechRig>,
            Option<&SkinLimbs>,
            Option<&mut Battered>,
        ),
        With<MechRoot>,
    >,
) {
    let crit = data.0.battle_fx.damage.critical_frac;
    for (root, sim, spec, rig, skin, battered) in &mut mechs {
        let n = spec.parts.len();
        let mut fresh = Battered::default();
        let had = battered.is_some();
        let b = match battered {
            Some(b) => b.into_inner(),
            None => &mut fresh,
        };
        let mut state = (std::mem::take(&mut b.smoking), b.hull);
        state.0.resize(n, false);
        let (mut changed, mut total, mut whole) = (false, 0.0, 0.0);
        for (i, p) in spec.parts.iter().enumerate() {
            let left = sim.damage.structure.get(i).copied().unwrap_or(p.structure);
            total += left;
            whole += p.structure;
            let lost = sim.damage.lost.get(i).copied().unwrap_or(false);
            if lost || state.0[i] || left >= crit * p.structure || p.kind == PartKind::Torso {
                continue;
            }
            if let Some(node) = node_for(spec, i, rig, skin) {
                commands.spawn((
                    Name::new("smoking part"),
                    Transform::default(),
                    FxStream {
                        recipe: "smoking",
                        dir: Vec3::Y,
                        on: true,
                        density: 1.0,
                    },
                    ChildOf(node),
                ));
                state.0[i] = true;
                changed = true;
            }
        }
        if !state.1 && !sim.dead && whole > 0.0 && total < crit * whole {
            // The torso: on a skin, above its hips; on the stand-in, its torso bone.
            let at = match skin {
                Some(s) => Some((s.scene(), Vec3::Y * (s.hip_height() + 3.0))),
                None => rig.map(|r| (r.torso(), Vec3::ZERO)),
            };
            if let Some((node, offset)) = at {
                commands.spawn((
                    Name::new("burning hull"),
                    Transform::from_translation(offset),
                    FxStream {
                        recipe: "burning_hull",
                        dir: Vec3::Y,
                        on: true,
                        density: 1.0,
                    },
                    ChildOf(node),
                ));
                state.1 = true;
                changed = true;
            }
        }
        b.smoking = state.0;
        b.hull = state.1;
        if changed && !had {
            commands.entity(root).insert(fresh);
        }
    }
}

/// Dust where a falling body first comes down.
fn land_dust(
    mut commands: Commands,
    data: Res<Data>,
    plan: Res<Plan>,
    bodies: Query<(Entity, &GlobalTransform, &Falling)>,
    mut fx: MessageWriter<Fx>,
) {
    let cfg = &data.0.battle_fx.damage;
    for (e, g, f) in &bodies {
        let p = g.translation();
        let ground = plan.height_at(p.x, p.z);
        if p.y - ground <= cfg.land_m {
            fx.write(Fx::new("fall_dust", Vec3::new(p.x, ground, p.z)).scaled(dust_scale(cfg, f.mass_t)));
            commands.entity(e).remove::<Falling>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heavier_body_throws_more_dust_within_bounds() {
        let cfg = DamageFxConfig::default();
        assert!(dust_scale(&cfg, 60.0) > dust_scale(&cfg, 3.0));
        assert!((dust_scale(&cfg, 10.0) - 1.0).abs() < 1e-5);
        assert_eq!(dust_scale(&cfg, 0.0), 0.3);
        assert_eq!(dust_scale(&cfg, 1.0e9), 3.0);
    }
}
