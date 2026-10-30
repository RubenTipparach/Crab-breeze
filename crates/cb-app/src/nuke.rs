//! A nuclear blast (crab-airship section 11; owner, 2026-10-30: "goes nuclear in a big mushroom cloud and leave a giant
//! Crater", "I want big fireball expansion and turn into a mushroom cloud", "Followed by a big ass Crater"). One
//! implementation, set off by a `Nuke` message: the Stratostacean's crash sends one, and so does the nuke test scene
//! (`capture_nuke`). It digs the crater through the city, brings down what stands within the flatten radius, lights
//! the flash, and raises the mushroom cloud of smoke-sheet particles (owner, 2026-10-30: "that nuke should be made of
//! particles"), shaped, timed and coloured after Mining Mike's (godot-sandbox `scenes/nuke_mushroom.tscn` and
//! `resources/shaders/nuke_cloud.gdshader` at a5c3ec5a; owner: "I just sent you the geometry stuff to give you a guide
//! on how to set up the explosion + color + shape"): puffs filling his cap, the ring under it, the stem and the dust
//! skirt as his `explode` animation poses them, each coloured by his heat palette (white-hot through yellow, orange
//! and red to lit smoke, undersides hottest), thinning out as he dissolves it; his timeline stretched so the fireball
//! swells in a second and the smoke rises over the cloud's life. Its numbers are `crash.nuke` in
//! `data/missions/fortress.ron`.

use crate::fx::Fx;
use crate::sim::{Data, SimSet};
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::prelude::*;
use cb_core::fortress::NukeSpec;

/// Set off a nuclear blast on the ground at `at`.
#[derive(Message, Clone, Copy, Debug)]
pub struct Nuke {
    /// The ground point it goes off at.
    pub at: Vec3,
}

/// A mushroom cloud building: where it went off and the seconds since.
#[derive(Component)]
pub struct NukeCloud {
    at: Vec3,
    t_s: f32,
}

/// What Mining Mike's scene is authored at: its head stands `AUTHORED_HEAD_M` up at the end, so a cloud whose cap
/// climbs to `cap_height_m` is scaled by `cap_height_m / AUTHORED_HEAD_M`.
const AUTHORED_HEAD_M: f32 = 275.0;

/// The pose and drive of the cloud at Mining Mike's timeline time `u` (0..5 s, his `explode` animation's keys).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// The head's height and scale.
    pub head_y: f32,
    pub head_scale: f32,
    /// The stem's scale (x and z, y).
    pub stem: Vec2,
    /// The skirt's scale (x and z, y).
    pub skirt: Vec2,
    /// The cloud's dissolve, 0..1.
    pub dissolve: f32,
}

/// Linear between keys `(time, value)`, eased out over the first span as Godot's 0.5 transition is, held past the ends.
fn keyed(keys: &[(f32, f32)], u: f32) -> f32 {
    if u <= keys[0].0 {
        return keys[0].1;
    }
    for (i, w) in keys.windows(2).enumerate() {
        let ((t0, a), (t1, b)) = (w[0], w[1]);
        if u <= t1 {
            let x = ((u - t0) / (t1 - t0).max(1e-6)).clamp(0.0, 1.0);
            let x = if i == 0 { 1.0 - (1.0 - x) * (1.0 - x) } else { x };
            return a + (b - a) * x;
        }
    }
    keys[keys.len() - 1].1
}

/// The pose at timeline time `u`, from Mining Mike's `explode` keys.
pub fn pose(u: f32) -> Pose {
    Pose {
        head_y: keyed(&[(0.0, 30.0), (0.6, 95.0), (2.8, 250.0), (5.0, 275.0)], u),
        head_scale: keyed(&[(0.0, 0.35), (0.4, 0.6), (2.8, 1.0), (5.0, 1.08)], u),
        stem: Vec2::new(
            keyed(&[(0.0, 0.3), (0.6, 0.5), (2.8, 1.0), (5.0, 0.85)], u),
            keyed(&[(0.0, 0.05), (0.6, 0.32), (2.8, 1.0), (5.0, 1.1)], u),
        ),
        skirt: Vec2::new(
            keyed(&[(0.0, 0.2), (1.0, 1.0), (5.0, 1.7)], u),
            keyed(&[(0.0, 0.25), (1.0, 0.4), (5.0, 0.3)], u),
        ),
        dissolve: keyed(&[(0.0, 0.0), (3.2, 0.0), (5.0, 1.0)], u),
    }
}

/// Real seconds after the blast to Mining Mike's timeline: his first 0.6 s (the fireball swelling) in the first second,
/// the rest stretched over the cloud's life, so it rises slowly (owner, 2026-10-30: "a big fireball expands rapidly,
/// then cools into a smoke ball that then rises slowly").
pub fn timeline(t_s: f32, cloud_s: f32) -> f32 {
    if t_s < 1.0 {
        0.6 * t_s
    } else {
        0.6 + 4.4 * ((t_s - 1.0) / (cloud_s - 1.0).max(1.0)).min(1.0)
    }
}

/// The cloud's heat at `t_s` real seconds: white-hot at once, through yellow and orange, red by 3 s, smoke by 14 s.
pub fn heat(t_s: f32) -> f32 {
    keyed(&[(0.0, 0.0), (0.5, 0.15), (3.0, 0.45), (8.0, 0.75), (14.0, 1.0)], t_s)
}

/// Nuclear blasts.
pub struct NukePlugin;

impl Plugin for NukePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Nuke>()
            .add_game_systems(FixedUpdate, detonate.in_set(SimSet::World))
            .add_game_systems(Update, animate_clouds);
    }
}

/// The blast: the crater, everything within the flatten radius down, the flash; its cloud starts building.
fn detonate(
    mut nukes: MessageReader<Nuke>,
    data: Res<Data>,
    plan: Res<Plan>,
    mut live: ResMut<Live>,
    mut commands: Commands,
    mut fx: MessageWriter<Fx>,
) {
    let Some(n) = data.0.fortress.cfg.crash.nuke else {
        nukes.clear();
        return;
    };
    let live = &mut *live;
    for e in nukes.read() {
        let at = e.at;
        let xz = Vec2::new(at.x, at.z);
        let touched = live.dug.dig_crater(xz, n.crater_radius_m, n.crater_depth_m, n.rim_m);
        live.dirty_tiles.extend(touched);
        let mut flat = 0;
        for b in plan.collision.candidates(xz, n.flatten_radius_m) {
            let (broke, chunks) = live.structures.break_sphere(b, at, n.flatten_radius_m);
            if broke > 0 {
                live.dirty_buildings.push(b);
                live.new_chunks.extend(chunks);
                flat += 1;
            }
        }
        for lot in plan.small.candidates(xz, n.flatten_radius_m) {
            let h = live.structures.houses.break_sphere(&plan.small, lot, at, n.flatten_radius_m);
            if !h.pieces.is_empty() {
                live.dirty_houses.push(lot);
                live.new_chunks.extend(h.chunks);
                flat += 1;
            }
        }
        live.forest.grow_around(&plan, xz, n.flatten_radius_m);
        let felled = live.forest.blast(xz, n.flatten_radius_m, 1.0e9);
        live.moved_trees.extend(felled);
        fx.write(Fx::new("nuke_flash", at + Vec3::Y * 40.0).scaled(n.fireball_radius_m / 8.0));
        commands.spawn((Name::new("nuke cloud"), NukeCloud { at, t_s: 0.0 }));
        println!(
            "NUKE at ({:.0}, {:.0}): a crater {:.0} m across and {:.0} m deep, {flat} buildings and houses flattened",
            at.x,
            at.z,
            n.crater_radius_m * 2.0,
            n.crater_depth_m
        );
    }
}

/// The cloud puffs once every this long, seconds.
const TICK_S: f32 = 0.2;

/// Each puff lives this long, seconds (owner, 2026-10-30: "the life time of the puffs are too short!": at 2.5 s the cloud
/// was all fresh puffs and flickered)...
const PUFF_LIFE_S: f32 = 7.0;

/// ...starting at this share of its size: smoke grows 90 % of its starting size a second (`particles::motion`), so over
/// a 7 s life a puff averages about its size.
const PUFF_START: f32 = 0.3;

/// The heat palette (Mining Mike's `nuke_cloud.gdshader`): white-hot, yellow, orange, red, then smoke lit from `lit`
/// (0 shade, 1 sunlit); the fire pushed `FIRE_GLOW` times into HDR, so it blooms. Linear RGB.
pub fn heat_colour(h: f32, lit: f32) -> Vec3 {
    const WHITE: Vec3 = Vec3::new(1.0, 0.97, 0.86);
    const YELLOW: Vec3 = Vec3::new(1.0, 0.80, 0.30);
    const ORANGE: Vec3 = Vec3::new(1.0, 0.42, 0.08);
    const RED: Vec3 = Vec3::new(0.62, 0.11, 0.05);
    const SMOKE_DARK: Vec3 = Vec3::new(0.20, 0.17, 0.16);
    const SMOKE_LIT: Vec3 = Vec3::new(0.62, 0.55, 0.50);
    const FIRE_GLOW: f32 = 3.0;
    let ss = |a: f32, b: f32, x: f32| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let h = h.clamp(0.0, 1.0);
    let mut fire = WHITE.lerp(YELLOW, ss(0.0, 0.15, h));
    fire = fire.lerp(ORANGE, ss(0.15, 0.35, h));
    fire = fire.lerp(RED, ss(0.35, 0.55, h));
    let glow = FIRE_GLOW + (1.0 - FIRE_GLOW) * ss(0.3, 0.55, h);
    let smoke = SMOKE_DARK.lerp(SMOKE_LIT, lit.clamp(0.0, 1.0));
    (fire * glow).lerp(smoke, ss(0.45, 0.78, h))
}

/// One puff of the cloud: where, how big (m), its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Where, world.
    pub at: Vec3,
    /// Its size, metres.
    pub size_m: f32,
    /// Its colour, linear HDR.
    pub colour: Vec3,
    /// Part of the flash: a short life (`FLASH_S`), not a puff's.
    pub flash: bool,
}

/// The flash lasts this long, seconds (owner, 2026-10-30: "Make sure theres a big flash at the beginning")...
const FLASH_S: f32 = 0.6;

/// ...this many times brighter than a lit white surface at first, falling to nothing.
const FLASH_GLOW: f32 = 40.0;

/// One tick's puffs of the cloud `t_s` after a blast at `at`, pure and placed by a fixed sequence (the tick's number):
/// Mining Mike's parts as his animation poses them (his units, scaled so the head reaches `cap_height_m`), filled with
/// puffs: the cap (an ellipsoid 90 across and 55 high over the head), the ring rolling under it (a torus 77.5 round,
/// 37.5 thick), the stem (a cone 40 wide at the foot, 22 at the top, 240 tall) and the dust skirt (a torus 90 round, 50
/// thick, on the ground). Each takes the heat now plus his per-part offset and a little noise, its undersides hotter;
/// fewer and fewer as he dissolves it.
pub fn puffs(n: &NukeSpec, at: Vec3, t_s: f32, to_sun: Vec3) -> Vec<Puff> {
    let k = (t_s / TICK_S).floor() as u32;
    let rand = |i: u32| ((i.wrapping_mul(2_654_435_761).rotate_left(13) ^ 0x9e37_79b9) >> 8 & 0xffff) as f32 / 65_535.0;
    let p = pose(timeline(t_s, n.cloud_s));
    let s = n.cap_height_m / AUTHORED_HEAD_M;
    let heat_now = heat(t_s);
    let left = 1.0 - p.dissolve;
    let mut out = Vec::new();
    let mut put = |i: u32, local: Vec3, centre: Vec3, size: f32, heat_offset: f32| {
        let normal = (local - centre).normalize_or(Vec3::Y);
        let under = (-normal.y).clamp(0.0, 1.0);
        let h = heat_now + heat_offset + (rand(i + 7) - 0.5) * 0.7 - under * 0.3;
        let lit = normal.dot(to_sun) * 0.5 + 0.5;
        out.push(Puff {
            at: at + local * s,
            size_m: size * s * (0.75 + 0.5 * rand(i + 9)),
            colour: heat_colour(h, lit),
            flash: false,
        });
    };
    let count = |full: f32| (full * left).round() as u32;
    let tau = std::f32::consts::TAU;
    // the flash: a blinding white burst far wider than the fireball, gone in `FLASH_S`
    let mut flashes = Vec::new();
    if t_s < FLASH_S {
        let k_flash = 1.0 - t_s / FLASH_S;
        for j in 0..12u32 {
            let i = k * 61 + j * 7 + 1;
            let a = rand(i) * tau;
            let off = Vec3::new(a.cos(), rand(i + 1) * 0.8, a.sin()) * 60.0 * rand(i + 2);
            flashes.push(Puff {
                at: at + (Vec3::Y * 40.0 + off) * s,
                size_m: 260.0 * s * (0.8 + 0.4 * rand(i + 3)),
                colour: Vec3::new(1.0, 0.97, 0.9) * FLASH_GLOW * k_flash * k_flash,
                flash: true,
            });
        }
    }
    // the cap: an ellipsoid over the head, its surface and just inside
    let head = Vec3::new(0.0, p.head_y, 0.0);
    let cap_c = head + Vec3::Y * 10.0 * p.head_scale;
    for j in 0..count(16.0) {
        let i = k * 97 + j * 11;
        let (a, z) = (rand(i) * tau, rand(i + 1) * 2.0 - 1.0);
        let r = (1.0 - z * z).sqrt() * (0.7 + 0.3 * rand(i + 2));
        let local = cap_c + Vec3::new(a.cos() * r * 90.0, z * 55.0, a.sin() * r * 90.0) * p.head_scale;
        put(i, local, cap_c, 55.0 * p.head_scale, 0.0);
    }
    // the ring rolling under the cap
    for j in 0..count(10.0) {
        let i = k * 89 + j * 13 + 3;
        let (a, b) = (rand(i) * tau, rand(i + 1) * tau);
        let ring = head + Vec3::new(0.0, -25.0, 0.0) * p.head_scale;
        let round = 77.5 + 37.5 * b.cos();
        let local = ring + Vec3::new(a.cos() * round, 37.5 * b.sin(), a.sin() * round) * p.head_scale;
        put(
            i,
            local,
            ring + Vec3::new(a.cos() * 77.5, 0.0, a.sin() * 77.5) * p.head_scale,
            38.0 * p.head_scale,
            0.05,
        );
    }
    // the stem: a cone up to the head
    for j in 0..count(8.0) {
        let i = k * 83 + j * 17 + 5;
        let f = rand(i);
        let a = rand(i + 1) * tau;
        let r = (40.0 + (22.0 - 40.0) * f) * rand(i + 2).sqrt();
        let local = Vec3::new(a.cos() * r * p.stem.x, 240.0 * f * p.stem.y, a.sin() * r * p.stem.x);
        put(i, local, Vec3::new(0.0, local.y, 0.0), 30.0 * p.stem.x, 0.12);
    }
    // the dust skirt round the foot
    for j in 0..count(6.0) {
        let i = k * 79 + j * 19 + 6;
        let (a, b) = (rand(i) * tau, rand(i + 1) * std::f32::consts::PI);
        let round = 90.0 + 50.0 * b.cos();
        let local = Vec3::new(
            a.cos() * round * p.skirt.x,
            8.0 + 50.0 * b.sin() * p.skirt.y,
            a.sin() * round * p.skirt.x,
        );
        put(
            i,
            local,
            Vec3::new(a.cos() * 90.0 * p.skirt.x, 8.0, a.sin() * 90.0 * p.skirt.x),
            45.0 * p.skirt.x,
            0.35,
        );
    }
    out.extend(flashes);
    out
}

/// Plays each cloud: this tick's puffs into the soft sprites (sorted and fogged with every other smoke), as smoke-sheet
/// puffs that live `PUFF_LIFE_S`, refreshed every tick; gone at the end.
fn animate_clouds(
    mut commands: Commands,
    mut clouds: Query<(Entity, &mut NukeCloud)>,
    mut sprites: ResMut<crate::soft_sprites::SoftSprites>,
    data: Res<Data>,
    time: Res<Time>,
) {
    let Some(n) = data.0.fortress.cfg.crash.nuke else { return };
    let to_sun = cb_core::world::sun_shade::to_sun();
    let spec = crate::soft_sprites::SoftSpec {
        kind: cb_core::fx::ParticleKind::Smoke,
        speed_m_s: 0.02,
        spread: 1.0,
        size_m: 1.0,
        life_s: PUFF_LIFE_S,
    };
    for (e, mut c) in &mut clouds {
        let was = c.t_s;
        c.t_s += time.delta_secs();
        if c.t_s > n.cloud_s {
            commands.entity(e).despawn();
            continue;
        }
        if (c.t_s / TICK_S).floor() == (was / TICK_S).floor() {
            continue;
        }
        for p in puffs(&n, c.at, c.t_s, to_sun) {
            if p.flash {
                let flash = crate::soft_sprites::SoftSpec { life_s: FLASH_S, ..spec };
                sprites.emit_tinted(flash, 1, p.at, Vec3::Y, p.size_m / 2.0, p.colour);
            } else {
                sprites.emit_tinted(spec, 1, p.at, Vec3::Y, p.size_m * PUFF_START, p.colour);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fireball_swells_within_a_second_then_rises_slowly() {
        let early = pose(timeline(1.0, 75.0));
        assert!(
            (early.head_scale - 0.6).abs() < 0.05 && early.head_y > 90.0,
            "swollen in a second: {early:?}"
        );
        let mid = pose(timeline(30.0, 75.0));
        let late = pose(timeline(74.0, 75.0));
        assert!(
            mid.head_y > early.head_y && late.head_y > mid.head_y,
            "rising: {} {} {}",
            early.head_y,
            mid.head_y,
            late.head_y
        );
        let rate = (late.head_y - mid.head_y) / 44.0 * 1100.0 / AUTHORED_HEAD_M;
        assert!(rate < 15.0, "slowly: {rate} m/s");
    }

    fn spec() -> NukeSpec {
        NukeSpec {
            crater_radius_m: 400.0,
            crater_depth_m: 100.0,
            rim_m: 25.0,
            flatten_radius_m: 700.0,
            fireball_radius_m: 250.0,
            cap_height_m: 1100.0,
            cloud_s: 75.0,
        }
    }

    #[test]
    fn the_puffs_make_a_mushroom_white_hot_then_smoke() {
        let n = spec();
        let early = puffs(&n, Vec3::ZERO, 0.5, Vec3::Y);
        let flash = puffs(&n, Vec3::ZERO, 0.0, Vec3::Y);
        assert!(flash.iter().any(|p| p.flash && p.colour.x > 30.0), "a blinding flash at the start");
        assert!(puffs(&n, Vec3::ZERO, 1.0, Vec3::Y).iter().all(|p| !p.flash), "gone within a second");
        let early: Vec<_> = early.into_iter().filter(|p| !p.flash).collect();
        let hot = early.iter().filter(|p| p.colour.x > 1.0).count();
        assert!(
            hot * 10 >= early.len() * 7,
            "most of it white-hot at first, blooming: {hot} of {}",
            early.len()
        );
        let late = puffs(&n, Vec3::ZERO, 30.0, Vec3::Y);
        assert!(late.iter().all(|p| p.colour.max_element() < 1.0), "smoke later");
        // a cap well over the ground, wider than the stem under it
        let top = late.iter().map(|p| p.at.y).fold(0.0f32, f32::max);
        assert!(top > 700.0, "the cap is up: {top}");
        let cap_w = late
            .iter()
            .filter(|p| p.at.y > top - 250.0)
            .map(|p| p.at.xz().length())
            .fold(0.0f32, f32::max);
        let stem_w = late
            .iter()
            .filter(|p| p.at.y > 150.0 && p.at.y < 400.0)
            .map(|p| p.at.xz().length())
            .fold(0.0f32, f32::max);
        assert!(cap_w > stem_w * 1.8, "a mushroom: the cap {cap_w} m out, the stem {stem_w} m");
        assert!(
            puffs(&n, Vec3::ZERO, 74.9, Vec3::Y).len() < late.len() / 4,
            "thinning out as it dissolves"
        );
    }

    #[test]
    fn it_burns_white_then_cools_into_smoke_then_dissolves_at_the_end() {
        assert_eq!(heat(0.0), 0.0, "white-hot at once");
        assert!(heat(3.0) < 0.5 && heat(14.0) >= 1.0, "smoke by 14 s");
        assert_eq!(pose(timeline(30.0, 75.0)).dissolve, 0.0, "whole while it rises");
        assert!(pose(timeline(75.0, 75.0)).dissolve > 0.99, "gone at the end");
    }
}
