//! The houses stills (a measurement instrument, CLAUDE.md section 4; `small-buildings` task 4.7): with
//! `CB_STILLS_HOUSES=1 --stills <dir>`, the pilot is stood in a suburb looking along a row of houses, in its street
//! (openspec change `lot-access`), at a front door
//! as it swings open for them, inside the living room with its modelled furniture, on a town's main street, in a
//! business's car park, beside a
//! house on sand or dry ground (its grass parched), and before a house an autocannon burst has just hit; each is shot, then the run
//! exits. The pilot is placed and turned directly, as the other stills do.

use crate::blast::Explosion;
use crate::capture::{shoot, stand_looking};
use crate::pilot::PilotSim;
use crate::state::GameApp;
use crate::world::Plan;
use bevy::prelude::*;
use cb_core::world::Biome;
use cb_core::world::Ground;
use cb_core::world::small_buildings::{Lot, Zone};
use std::path::PathBuf;

/// Runs the houses stills.
pub struct HousesStillsPlugin {
    /// Where the PNGs go.
    pub dir: PathBuf,
}

#[derive(Resource)]
struct Script {
    dir: PathBuf,
    beat: usize,
    beat_started_s: f32,
    done: Vec<&'static str>,
}

impl Script {
    fn once(&mut self, step: &'static str) -> bool {
        if self.done.contains(&step) {
            return false;
        }
        self.done.push(step);
        true
    }
}

impl Plugin for HousesStillsPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).ok();
        app.insert_resource(Script {
            dir: self.dir.clone(),
            beat: 0,
            beat_started_s: 0.0,
            done: Vec::new(),
        })
        .add_game_systems(Update, run);
    }
}

/// The first lot in `zone` whose kit is `id` (or any kit when `id` is empty), with its index.
fn find<'a>(plan: &'a Plan, zone: Zone, id: &str) -> Option<(u32, &'a Lot)> {
    plan.small
        .lots
        .iter()
        .enumerate()
        .find(|(i, l)| l.zone == zone && (id.is_empty() || plan.small.kit(*i as u32).id == id))
        .map(|(i, l)| (i as u32, l))
}

/// A point of a lot's frame (x, z) on the ground, world.
fn on_ground(plan: &Plan, l: &Lot, x: f32, z: f32) -> Vec3 {
    let p = l.to_world(Vec2::new(x, z));
    Vec3::new(p.x, plan.height_at(p.x, p.y), p.y)
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut pilot: ResMut<PilotSim>,
    plan: Res<Plan>,
    time: Res<Time>,
    ready: Option<Res<crate::loading::GameReady>>,
    mut blasts: MessageWriter<Explosion>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let t = now - script.beat_started_s;
    let Some((ranch_id, ranch)) = find(&plan, Zone::Suburb, "ranch").or_else(|| find(&plan, Zone::Suburb, "")) else {
        println!("STILLS houses: no suburb lots");
        exit.write(AppExit::error());
        return;
    };
    let kit = plan.small.kit(ranch_id);
    let mut next = false;
    match script.beat {
        0 => {
            next = ready.is_some_and(|r| r.0) && t > 2.0;
            if next {
                let count = |z: Zone| plan.small.lots.iter().filter(|l| l.zone == z).count();
                println!(
                    "STILLS houses: {} lots (suburb {}, town {}, roadside {}); shooting lot {ranch_id} ({})",
                    plan.small.lots.len(),
                    count(Zone::Suburb),
                    count(Zone::Town),
                    count(Zone::Roadside),
                    kit.id
                );
            }
        }
        // A suburb: the house from across its drive, its neighbours along the row.
        1 => {
            if script.once("suburb") {
                let feet = on_ground(&plan, ranch, kit.hi.x - 1.0, kit.hi.z - 0.5);
                stand_looking(&mut pilot, feet, on_ground(&plan, ranch, 0.0, 0.0) + Vec3::Y * 3.0);
            }
            if t > 6.0 && script.once("suburb shot") {
                shoot(&mut commands, &script.dir, "houses-suburb.png");
            }
            next = t > 8.0;
        }
        // In the street in front of the row (lot-access): the drives running down to it, the houses along it.
        2 => {
            let off = kit.hi.z + plan.small.paving.streets.first().map_or(6.0, |s| s.width_m * 0.5 + 2.0);
            if script.once("street") {
                let feet = on_ground(&plan, ranch, kit.hi.x + 6.0, off);
                stand_looking(
                    &mut pilot,
                    feet,
                    on_ground(&plan, ranch, kit.lo.x - 25.0, kit.hi.z - 2.0) + Vec3::Y * 2.0,
                );
            }
            if t > 6.0 && script.once("street shot") {
                shoot(&mut commands, &script.dir, "houses-street.png");
            }
            next = t > 8.0;
        }
        // At the front door, which swings open as the pilot reaches it.
        3 => {
            let leaf = &kit.leaves[0];
            let mid = Vec2::new(leaf.hinge.x, leaf.hinge.z) + leaf.shut * (leaf.width_m * 0.5);
            if script.once("door") {
                let out = mid - leaf.inward * 1.2;
                let feet = on_ground(&plan, ranch, out.x, out.y) + Vec3::Y * leaf.hinge.y;
                let inside = mid + leaf.inward * 4.0;
                stand_looking(&mut pilot, feet, on_ground(&plan, ranch, inside.x, inside.y) + Vec3::Y * 1.6);
            }
            if t > 4.0 && script.once("door shot") {
                shoot(&mut commands, &script.dir, "houses-door.png");
            }
            next = t > 6.0;
        }
        // Inside, among the modelled furniture.
        4 => {
            let leaf = &kit.leaves[0];
            let mid = Vec2::new(leaf.hinge.x, leaf.hinge.z) + leaf.shut * (leaf.width_m * 0.5);
            if script.once("inside") {
                let at = mid + leaf.inward * 1.6;
                let feet = ranch.point_to_world(Vec3::new(at.x, leaf.hinge.y, at.y));
                let target = kit
                    .furniture
                    .iter()
                    .find(|f| f.item == "sofa")
                    .or(kit.furniture.first())
                    .map_or(Vec3::new(0.0, 1.0, 0.0), |f| f.at);
                stand_looking(&mut pilot, feet, ranch.point_to_world(target + Vec3::Y * 0.5));
            }
            if t > 6.0 && script.once("inside shot") {
                shoot(&mut commands, &script.dir, "houses-inside.png");
            }
            next = t > 8.0;
        }
        // A town's main street, from the highway.
        5 => {
            let Some((town_id, town)) = find(&plan, Zone::Town, "") else {
                script.beat += 1;
                script.beat_started_s = now;
                return;
            };
            let tk = plan.small.kit(town_id);
            if script.once("town") {
                let feet = on_ground(&plan, town, tk.hi.x + 20.0, tk.hi.z + 18.0);
                stand_looking(&mut pilot, feet, on_ground(&plan, town, 0.0, 0.0) + Vec3::Y * 3.0);
            }
            if t > 6.0 && script.once("town shot") {
                shoot(&mut commands, &script.dir, "houses-town.png");
            }
            next = t > 8.0;
        }
        // A business's car park (lot-access): its stalls, its curb cut onto the highway.
        6 => {
            let Some((biz_id, biz)) = plan.small.lots.iter().enumerate().find(|(i, l)| {
                l.zone == Zone::Town && plan.small.kit(*i as u32).kind == cb_core::world::small_buildings::kit::Kind::Business
            }) else {
                script.beat += 1;
                script.beat_started_s = now;
                return;
            };
            let bk = plan.small.kit(biz_id as u32);
            if script.once("car park") {
                println!("STILLS houses: car park of lot {biz_id} ({})", bk.id);
                let feet = on_ground(&plan, biz, bk.hi.x + 4.0, bk.hi.z + 15.0);
                stand_looking(
                    &mut pilot,
                    feet,
                    on_ground(&plan, biz, (bk.lo.x + bk.hi.x) * 0.5, bk.hi.z) + Vec3::Y * 1.0,
                );
            }
            if t > 6.0 && script.once("car park shot") {
                shoot(&mut commands, &script.dir, "houses-carpark.png");
            }
            next = t > 8.0;
        }
        // A house on sand or dry ground, where the grass is parched (grass `biome_dryness`).
        7 => {
            let dry = plan.small.lots.iter().enumerate().find(|(_, l)| {
                let at = l.to_world(Vec2::ZERO);
                let b = plan.biomes.dominant(at, plan.height_at(at.x, at.y), 0.0);
                matches!(b, Biome::Desert | Biome::Beach | Biome::Canyon | Biome::Savanna)
            });
            let Some((dry_id, lot)) = dry else {
                println!("STILLS houses: no lot on dry ground");
                script.beat += 1;
                script.beat_started_s = now;
                return;
            };
            let dk = plan.small.kit(dry_id as u32);
            if script.once("dry") {
                let at = lot.to_world(Vec2::ZERO);
                let b = plan.biomes.dominant(at, plan.height_at(at.x, at.y), 0.0);
                println!("STILLS houses: dry lot {dry_id} ({}) in {b:?}", dk.id);
                let feet = on_ground(&plan, lot, dk.hi.x + 12.0, dk.hi.z + 10.0);
                stand_looking(&mut pilot, feet, on_ground(&plan, lot, 0.0, 0.0) + Vec3::Y * 1.0);
            }
            if t > 6.0 && script.once("dry shot") {
                shoot(&mut commands, &script.dir, "houses-dry.png");
            }
            next = t > 8.0;
        }
        // An autocannon burst's worth (1 MJ) on the front wall, seen from the street.
        8 => {
            if script.once("hit") {
                let feet = on_ground(&plan, ranch, kit.hi.x + 4.0, kit.hi.z + 4.0);
                stand_looking(&mut pilot, feet, on_ground(&plan, ranch, 0.0, 0.0) + Vec3::Y * 2.0);
                blasts.write(Explosion {
                    at: ranch.point_to_world(Vec3::new(-2.0, 1.6, kit.hi.z.min(6.0) - 1.0)),
                    energy_mj: 1.0,
                    by: None,
                });
            }
            if t > 3.0 && script.once("hit shot") {
                shoot(&mut commands, &script.dir, "houses-hit.png");
            }
            if t > 9.0 && script.once("settled shot") {
                shoot(&mut commands, &script.dir, "houses-hit-settled.png");
            }
            next = t > 11.0;
        }
        _ => {
            println!("STILLS houses: done");
            exit.write(AppExit::Success);
        }
    }
    if next {
        script.beat += 1;
        script.beat_started_s = now;
    }
}
