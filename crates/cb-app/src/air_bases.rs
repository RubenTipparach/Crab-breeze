//! The air bases in the game (openspec change `air-bases`; owner, 2026-10-24: "We'll need some hanger bays or air base
//! for the player to land and get their mech fixed", 2026-10-25: "going to airbase should replenish ammo for free, just
//! need to enter airbase hangar, and I would get repairs and free ammo (free repairs fornow)"). Each base the world plan
//! placed (`cb_core::world::bases`) is drawn as a block-out: a concrete apron over its flattened site, its hangar bays
//! along the back edge (open toward the apron, tall enough for a 24 m mech to walk in), landing pads and a control
//! tower. A piloted mech that stops inside a bay is repaired and rearmed at once, free, once each time it walks in.

use crate::mech::{MechSim, MechSpec, Piloted};
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::{MeshBuilder, Plan};
use bevy::prelude::*;
use cb_core::mech::Damage;
use cb_core::world::bases::hangar_at;

/// Draws the air bases and repairs and rearms a mech that stops in a hangar bay.
pub struct AirBasesPlugin;

impl Plugin for AirBasesPlugin {
    fn build(&self, app: &mut App) {
        app.add_world_spawn(spawn_bases)
            .add_game_systems(FixedUpdate, repair_in_hangars.in_set(SimSet::Mech).after(crate::mech::step_mech));
    }
}

/// Wall and roof thickness of a hangar, metres.
const WALL_M: f32 = 1.5;
/// The apron's lift over the flattened ground, metres (clear of the terrain, no z-fighting).
const APRON_LIFT_M: f32 = 0.12;
/// A landing pad's side, metres.
const PAD_M: f32 = 40.0;

fn spawn_bases(
    mut commands: Commands,
    plan: Res<Plan>,
    data: Res<Data>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let cfg = &data.0.world.bases;
    if plan.0.bases.is_empty() {
        return;
    }
    let concrete = crate::buildings::concrete_material(&mut images, &mut materials);
    let paint = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..default()
    });
    // Darker than a pavement: the whole site is concrete and a light tone washed out in the sun.
    let apron_tone = [0.26, 0.26, 0.25, 1.0];
    let wall_tone = [0.72, 0.72, 0.7, 1.0];
    let roof_tone = [0.5, 0.52, 0.54, 1.0];
    let pad_tone = [0.95, 0.75, 0.15, 1.0];
    for b in &plan.0.bases {
        let mut slab = MeshBuilder::tiled(4.0, 4.0);
        let mut marks = MeshBuilder::default();
        let g = b.ground_m;
        let r = b.rect();
        // The apron over the whole site.
        slab.block(
            Vec3::new(r.min.x, g - 0.5, r.min.y),
            Vec3::new(r.max.x, g + APRON_LIFT_M, r.max.y),
            apron_tone,
        );
        // The hangar bays: side walls, back wall and roof; open at the front (low z).
        let h = cfg.hangar_m.y;
        for bay in b.hangars(cfg) {
            let (lo, hi) = (bay.min, bay.max);
            let top = g + APRON_LIFT_M + h;
            slab.block(Vec3::new(lo.x - WALL_M, g, lo.y), Vec3::new(lo.x, top, hi.y), wall_tone);
            slab.block(Vec3::new(hi.x, g, lo.y), Vec3::new(hi.x + WALL_M, top, hi.y), wall_tone);
            slab.block(
                Vec3::new(lo.x - WALL_M, g, hi.y),
                Vec3::new(hi.x + WALL_M, top, hi.y + WALL_M),
                wall_tone,
            );
            slab.block(
                Vec3::new(lo.x - WALL_M, top, lo.y),
                Vec3::new(hi.x + WALL_M, top + WALL_M, hi.y + WALL_M),
                roof_tone,
            );
            // A yellow stop line across the bay's mouth.
            let y = g + APRON_LIFT_M + 0.02;
            marks.quad(
                [
                    Vec3::new(lo.x, y, lo.y + 1.0),
                    Vec3::new(hi.x, y, lo.y + 1.0),
                    Vec3::new(hi.x, y, lo.y),
                    Vec3::new(lo.x, y, lo.y),
                ],
                pad_tone,
            );
        }
        // Three landing pads across the front of the apron, outlined.
        for i in 0..3 {
            let c = Vec2::new(b.centre.x + (i as f32 - 1.0) * (PAD_M + 40.0), r.min.y + PAD_M);
            let y = g + APRON_LIFT_M + 0.02;
            let half = PAD_M * 0.5;
            for (a, z) in [
                (Vec2::new(-half, -half), Vec2::new(half, -half + 1.0)),
                (Vec2::new(-half, half - 1.0), Vec2::new(half, half)),
                (Vec2::new(-half, -half), Vec2::new(-half + 1.0, half)),
                (Vec2::new(half - 1.0, -half), Vec2::new(half, half)),
            ] {
                let (p, q) = (c + a, c + z);
                marks.quad(
                    [
                        Vec3::new(p.x, y, q.y),
                        Vec3::new(q.x, y, q.y),
                        Vec3::new(q.x, y, p.y),
                        Vec3::new(p.x, y, p.y),
                    ],
                    pad_tone,
                );
            }
        }
        // The control tower by the front corner.
        let t = Vec2::new(r.max.x - 40.0, r.min.y + 40.0);
        slab.block(
            Vec3::new(t.x - 6.0, g, t.y - 6.0),
            Vec3::new(t.x + 6.0, g + 36.0, t.y + 6.0),
            wall_tone,
        );
        slab.block(
            Vec3::new(t.x - 9.0, g + 36.0, t.y - 9.0),
            Vec3::new(t.x + 9.0, g + 44.0, t.y + 9.0),
            roof_tone,
        );
        commands.spawn((
            Name::new(format!("air base {}", b.id)),
            Mesh3d(meshes.add(slab.build_with_tangents())),
            MeshMaterial3d(concrete.clone()),
            Transform::IDENTITY,
        ));
        commands.spawn((
            Name::new(format!("air base {} markings", b.id)),
            Mesh3d(meshes.add(marks.build())),
            MeshMaterial3d(paint.clone()),
            Transform::IDENTITY,
        ));
    }
    info!("air bases: {} drawn", plan.0.bases.len());
}

/// A piloted mech that stops inside a hangar bay is repaired (every part not lost back to full) and
/// rearmed (every weapon's rounds back to its full load), free, once each time it walks in.
fn repair_in_hangars(
    plan: Res<Plan>,
    data: Res<Data>,
    mut mechs: Query<(&mut MechSim, &MechSpec), With<Piloted>>,
    mut inside: Local<Option<(u32, usize)>>,
) {
    let cfg = &data.0.world.bases;
    for (mut sim, spec) in &mut mechs {
        let at = hangar_at(&plan.0.bases, cfg, Vec2::new(sim.curr.pos.x, sim.curr.pos.z));
        if at.is_none() {
            *inside = None;
            continue;
        }
        if *inside == at || !sim.curr.is_stopped(&spec.locomotion) || sim.dead {
            continue;
        }
        *inside = at;
        let full = Damage::new(&spec.parts);
        let sim = &mut *sim;
        for (i, s) in sim.damage.structure.iter_mut().enumerate() {
            if !sim.damage.lost.get(i).copied().unwrap_or(false) {
                *s = full.structure[i];
            }
        }
        sim.guns.rounds_left = spec.weapons.iter().map(|w| w.rounds).collect();
        if let Some((base, bay)) = at {
            info!("air base {base}, bay {bay}: repaired and rearmed");
        }
    }
}
