//! A nuclear blast (crab-airship section 11; owner, 2026-10-30: "goes nuclear in a big mushroom cloud and leave a giant
//! Crater", "I want big fireball expansion and turn into a mushroom cloud", "Followed by a big ass Crater"). One
//! implementation, set off by a `Nuke` message: the Stratostacean's crash sends one, and so does the nuke test scene
//! (`capture_nuke`). It digs the crater through the city, brings down what stands within the flatten radius, lights
//! the flash, and raises the mushroom cloud, of smoke-sheet particles (owner, 2026-10-30: "that nuke should be made of
//! particles"), shaped, timed and coloured after Mining Mike's mesh nuke (godot-sandbox `scenes/nuke_mushroom.tscn` and
//! `resources/shaders/nuke_cloud.gdshader` at a5c3ec5a), which the test scene stands beside it (`nuke_mesh`). As the
//! owner put it ("First the flash. It needs to be a big lensflare", "All fire ball has to appear at once. Then the
//! fireball grows and cools into a big round puff. That puff rises leaving a big thick column under it"): the flash is
//! the lens flare turned on the blast (`sun_flare::FlareFlash`); the cloud is a fixed set of puffs the nuke places
//! itself every frame (`cloud`), so the ball appears whole at once, swells, cools and rises as one, and the column
//! fills in under it. Its numbers are `crash.nuke` in `data/missions/fortress.ron`.

use crate::fx::Fx;
use crate::sim::{Data, PresentSet, SimSet};
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
pub const AUTHORED_HEAD_M: f32 = 275.0;

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

/// Real seconds after the blast to Mining Mike's timeline: his `explode` animation over the explosion's length,
/// `cloud_s` (owner, 2026-10-30: "The explosion part is also way too slow", "Review timing of mesh explosion with this
/// And sync them up!!!").
pub fn timeline(t_s: f32, cloud_s: f32) -> f32 {
    (5.0 * t_s / cloud_s.max(0.1)).clamp(0.0, 5.0)
}

/// The cloud's heat at timeline time `u`: Mining Mike's keys, white-hot at once, smoke by 3.6 of his 5 s.
pub fn heat(u: f32) -> f32 {
    keyed(&[(0.0, 0.0), (0.4, 0.2), (2.0, 0.6), (3.6, 1.0)], u)
}

/// Nuclear blasts.
pub struct NukePlugin;

impl Plugin for NukePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Nuke>()
            .add_game_systems(FixedUpdate, detonate.in_set(SimSet::World))
            .init_resource::<crate::sun_flare::FlareFlash>()
            .add_game_systems(Update, animate_clouds.before(PresentSet::Pose));
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

/// The flash lasts this long, seconds...
const FLASH_S: f32 = 0.8;

/// ...its lens flare this many times the sun's at first, falling to nothing.
const FLASH_GLOW: f32 = 12.0;

/// A puff's fire gives way to smoke over this span of heat, the fireball sheet cross-fading into the smoke sheet's
/// same shapes (`soft_sprites::Look::Burning`).
const BURN_TO_SMOKE: (f32, f32) = (0.4, 0.72);

/// A puff's opacity: fire, then smoke, then dust. Ten times the puffs drawn fainter (owner, 2026-10-30: "increase smoke
/// puff density even more, 10x original", "but make them more transparent too").
const FIRE_ALPHA: f32 = 0.4;
const SMOKE_ALPHA: f32 = 0.32;
const DUST_ALPHA: f32 = 0.08;

/// How far one puff's heat strays from the cloud's.
const HEAT_NOISE: f32 = 0.18;

/// After the explosion everything keeps cooling, a unit of heat in this many seconds, so no puff (the ring's
/// undersides, the hottest crevices) is still burning as the smoke hangs.
const COOL_S: f32 = 4.0;

/// Each puff spans this much of its part's radius (the ball's, the ring's tube, the column's, the skirt's tube): a
/// sheet cell is mostly its soft edge, so a puff must be several times its part's radius across to close the gaps
/// between its neighbours (owner, 2026-10-30: "There's still lots of gaps 4x puff sizes").
const PUFF_SPAN: f32 = 3.6;
/// The column's and the dust skirt's puffs span less of theirs: at the ball's span a column puff was wider than the
/// cloud and the skirt's 2.4 km, a brown wall over everything (side-by-side scene 18; owner, 2026-10-30: "Too much
/// dust now lol").
const COLUMN_SPAN: f32 = 1.6;
const SKIRT_SPAN: f32 = 0.7;
/// Every puff bursts out of the blast point to its place in the cloud in 0.2-0.5 s, each at its own pace (owner,
/// 2026-10-30: "They originate from one location!", "grow to their max size withing .2-.5 seconds").
const BURST_S: (f32, f32) = (0.2, 0.5);
/// A puff's size as it leaves the blast point, as a share of its full size.
const BURST_SEED: f32 = 0.12;

/// The puffs of each part of the cloud, ten times the first cloud's.
const BALL: u32 = 900;
const RING: u32 = 240;
const COLUMN: u32 = 500;
const SKIRT: u32 = 240;

/// The fire's tint over the fireball sheet (which carries its own white cores and red folds): white-hot, then orange,
/// then a dull red, pushed into HDR while it burns so it blooms. Linear RGB.
fn fire_tint(h: f32) -> Vec3 {
    let h = (h / BURN_TO_SMOKE.1).clamp(0.0, 1.0);
    let c = Vec3::new(1.0, 0.98, 0.92).lerp(Vec3::new(1.0, 0.62, 0.32), (h * 2.0).min(1.0));
    let c = c.lerp(Vec3::new(0.8, 0.32, 0.16), (h * 2.0 - 1.0).max(0.0));
    c * (4.0 - 3.0 * h)
}

/// How long the whole cloud lasts: the explosion, then its smoke hanging.
pub fn lasts_s(n: &NukeSpec) -> f32 {
    n.cloud_s + n.smoke_linger_s
}

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

/// The cloud `t_s` after a blast at `at`: every puff placed for this frame, pure, the same puffs every frame (each its
/// own fixed place in its part, cell and spin), so the cloud moves as one. Mining Mike's parts as his animation poses
/// them (his units, scaled so the head reaches `cap_height_m`):
/// - the ball: his cap made round, a ball of `BALL` puffs 90 of his units in radius times the head's scale round the
///   head, swelling and climbing with the head;
/// - the ring rolling under it (a torus 77.5 round and 37.5 thick under the head);
/// - the column: a thick cone (his stem, 1.3 times as wide) from the ground up to the ball's foot, filling in as the
///   ball climbs off it, half dust low down, the ground sucked up into it;
/// - the dust skirt round the foot (a torus 90 round, 50 thick, on the ground), all dust.
///
/// Every puff bursts out of the blast point to its place in 0.2-0.5 s. Each takes the heat now plus his per-part
/// offset and its own noise, its undersides hotter: fire (the fireball sheet) that cross-fades into lit smoke as it
/// cools. After the explosion's `cloud_s` the smoke hangs `smoke_linger_s`, spreading slowly to `smoke_grow` times
/// its size, each puff fading at its own point over the last fifth.
pub fn cloud(n: &NukeSpec, at: Vec3, t_s: f32, to_sun: Vec3) -> Vec<crate::soft_sprites::Placed> {
    use crate::soft_sprites::{Look, Placed};
    let rand = |i: u32| ((i.wrapping_mul(2_654_435_761).rotate_left(13) ^ 0x9e37_79b9) >> 8 & 0xffff) as f32 / 65_535.0;
    let tau = std::f32::consts::TAU;
    let u = timeline(t_s, n.cloud_s);
    let p = pose(u);
    let s = n.cap_height_m / AUTHORED_HEAD_M;
    let heat_now = heat(u);
    // the hanging smoke: 0 at the explosion's end, 1 at the cloud's
    let hang = ((t_s - n.cloud_s) / n.smoke_linger_s.max(0.1)).clamp(0.0, 1.0);
    let grow = 1.0 + (n.smoke_grow - 1.0) * (1.0 - (1.0 - hang) * (1.0 - hang));
    let mut out = Vec::with_capacity((BALL + RING + COLUMN + SKIRT) as usize);
    let mut put = |i: u32, pos: Vec3, normal: Vec3, size: f32, heat_offset: f32, dust: bool| {
        let normal = normal.normalize_or(Vec3::Y);
        let under = (-normal.y).clamp(0.0, 1.0);
        // a little noise and a little hotter underneath, so the ball cools as one: at the puffs' size a puff left
        // hot behind the rest reads as a new blast (owner, 2026-10-30: "What are these two explosion puffs doing")
        let h = heat_now + heat_offset + (rand(i + 7) - 0.5) * HEAT_NOISE - under * 0.08 + (t_s - n.cloud_s).max(0.0) / COOL_S;
        let lit = normal.dot(to_sun) * 0.5 + 0.5;
        // each puff goes at its own point over the last fifth of the hang
        let keep = ((rand(i + 11) * 0.2 + 0.8 - hang) / 0.05).clamp(0.0, 1.0);
        // out of the blast point, fast then easing into place
        let k = 1.0 - (1.0 - (t_s / (BURST_S.0 + (BURST_S.1 - BURST_S.0) * rand(i + 9))).clamp(0.0, 1.0)).powi(3);
        let pos = at + (pos - at) * k * grow;
        let size = size * (BURST_SEED + (1.0 - BURST_SEED) * k) * grow;
        let ss = |a: f32, b: f32, x: f32| {
            let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let fire = 1.0 - ss(BURN_TO_SMOKE.0, BURN_TO_SMOKE.1, h);
        let (look, colour, alpha) = if dust {
            let dust = Vec3::new(0.30, 0.25, 0.20).lerp(Vec3::new(0.62, 0.52, 0.42), lit.clamp(0.0, 1.0));
            (Look::Dust, dust.lerp(heat_colour(h, lit), fire * 0.5), DUST_ALPHA)
        } else {
            let colour = heat_colour(h.max(BURN_TO_SMOKE.1), lit).lerp(fire_tint(h), fire);
            (Look::Burning(fire), colour, SMOKE_ALPHA + (FIRE_ALPHA - SMOKE_ALPHA) * fire)
        };
        out.push(Placed {
            at: pos,
            size_m: size * (0.8 + 0.4 * rand(i + 6)),
            colour: colour.extend(alpha * keep),
            look,
            cell: (rand(i + 5) * 16.0).floor().min(15.0),
            spin_rad: rand(i + 3) * tau + t_s * 0.05 * (rand(i + 4) - 0.5),
        });
    };
    // the ball, round, every puff out of the blast point at once
    let head = at + Vec3::Y * p.head_y * s;
    let centre = head + Vec3::Y * 10.0 * p.head_scale * s;
    let r = 90.0 * p.head_scale * s;
    for j in 0..BALL {
        let i = j * 13 + 1;
        let (a, z) = (rand(i) * tau, rand(i + 1) * 2.0 - 1.0);
        let dir = Vec3::new((1.0 - z * z).sqrt() * a.cos(), z, (1.0 - z * z).sqrt() * a.sin());
        let reach = rand(i + 2).cbrt();
        put(i, centre + dir * reach * r, dir, r * PUFF_SPAN, 0.0, false);
    }
    // the ring rolling under it
    for j in 0..RING {
        let i = j * 17 + 3;
        let (a, b) = (rand(i) * tau, rand(i + 1) * tau);
        let ring_c = head - Vec3::Y * 25.0 * p.head_scale * s;
        let round = Vec3::new(a.cos(), 0.0, a.sin());
        let tube = round * 37.5 * b.cos() + Vec3::Y * 37.5 * b.sin();
        let pos = ring_c + (round * 77.5 + tube) * p.head_scale * s;
        put(i, pos, tube, 37.5 * 2.0 * p.head_scale * s * PUFF_SPAN, 0.05, false);
    }
    // the column, from the ground up to the ball's foot, the ground sucked up into it low down
    let top = (centre.y - at.y - r * 0.7).max(0.0);
    for j in 0..COLUMN {
        let i = j * 19 + 5;
        let f = (j as f32 + rand(i)) / COLUMN as f32;
        let wide = (40.0 + (22.0 - 40.0) * f) * p.stem.x * 1.3 * s;
        let a = rand(i + 1) * tau;
        let off = Vec3::new(a.cos(), 0.0, a.sin()) * wide * 0.6 * rand(i + 2).sqrt();
        let dust = rand(i + 8) < 0.35 * (1.0 - f) * (1.0 - f);
        put(i, at + Vec3::Y * f * top + off, off, wide * 2.0 * COLUMN_SPAN, 0.12, dust);
    }
    // the dust skirt round the foot
    for j in 0..SKIRT {
        let i = j * 23 + 6;
        let (a, b) = (rand(i) * tau, rand(i + 1) * std::f32::consts::PI);
        let round = Vec3::new(a.cos(), 0.0, a.sin());
        let pos = at + (round * (90.0 + 50.0 * b.cos()) * p.skirt.x + Vec3::Y * (8.0 + 50.0 * b.sin() * p.skirt.y)) * s;
        put(i, pos, round + Vec3::Y * 0.3, 50.0 * 2.0 * p.skirt.x * s * SKIRT_SPAN, 0.35, true);
    }
    out
}

/// Plays each cloud: its puffs placed for this frame among the soft sprites (sorted and fogged with every other smoke),
/// the lens flare turned on it for the flash; gone at the end.
fn animate_clouds(
    mut commands: Commands,
    mut clouds: Query<(Entity, &mut NukeCloud)>,
    mut sprites: ResMut<crate::soft_sprites::SoftSprites>,
    mut flash: ResMut<crate::sun_flare::FlareFlash>,
    data: Res<Data>,
    time: Res<Time>,
) {
    flash.0 = None;
    let Some(n) = data.0.fortress.cfg.crash.nuke else { return };
    let to_sun = cb_core::world::sun_shade::to_sun();
    for (e, mut c) in &mut clouds {
        c.t_s += time.delta_secs();
        if c.t_s > lasts_s(&n) {
            commands.entity(e).despawn();
            continue;
        }
        if c.t_s < FLASH_S {
            let k = 1.0 - c.t_s / FLASH_S;
            flash.0 = Some((c.at + Vec3::Y * n.fireball_radius_m * 0.5, FLASH_GLOW * k * k));
        }
        for p in cloud(&n, c.at, c.t_s, to_sun) {
            sprites.place(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cloud_plays_mining_mikes_timeline_over_the_explosion() {
        assert_eq!(timeline(0.0, 6.0), 0.0);
        assert!(
            (timeline(3.0, 6.0) - 2.5).abs() < 1e-5,
            "half way through his 5 s at half the explosion"
        );
        assert_eq!(timeline(9.0, 6.0), 5.0, "held at his end");
        let swollen = pose(timeline(0.5, 6.0));
        assert!(swollen.head_scale > 0.55, "the fireball swells in half a second: {swollen:?}");
    }

    fn spec() -> NukeSpec {
        NukeSpec {
            crater_radius_m: 400.0,
            crater_depth_m: 100.0,
            rim_m: 25.0,
            flatten_radius_m: 700.0,
            fireball_radius_m: 250.0,
            cap_height_m: 1100.0,
            cloud_s: 6.0,
            smoke_linger_s: 60.0,
            smoke_grow: 1.5,
        }
    }

    #[test]
    fn the_fireball_is_whole_at_once_then_a_round_smoke_ball_rises_on_a_thick_column() {
        let n = spec();
        let first = cloud(&n, Vec3::ZERO, 0.0, Vec3::Y);
        assert_eq!(
            first.len() as u32,
            BALL + RING + COLUMN + SKIRT,
            "every puff there from the first frame"
        );
        let hot = first.iter().filter(|p| p.colour.x > 1.0).count();
        assert!(hot * 10 >= first.len() * 6, "white-hot at once: {hot} of {}", first.len());
        let late = cloud(&n, Vec3::ZERO, n.cloud_s * 0.8, Vec3::Y);
        // undersides and crevices stay hot longest, as in his shader: most of it, not all, is smoke by now
        let smoke = late.iter().filter(|p| p.colour.x < 1.0).count();
        assert!(smoke * 10 >= late.len() * 8, "cooled into smoke: {smoke} of {}", late.len());
        let top = late.iter().map(|p| p.at.y).fold(0.0f32, f32::max);
        let low = first.iter().map(|p| p.at.y).fold(0.0f32, f32::max);
        assert!(top > low * 2.0 && top > 700.0, "the ball has risen: {low} then {top}");
        let column = late.iter().filter(|p| p.at.y > 100.0 && p.at.y < top * 0.5).count();
        assert!(column >= 15, "a thick column under it: {column} puffs");
        let ball_w = late
            .iter()
            .filter(|p| p.at.y > top - 300.0)
            .map(|p| p.at.xz().length())
            .fold(0.0f32, f32::max);
        let col_w = late
            .iter()
            .filter(|p| p.at.y > 150.0 && p.at.y < 400.0)
            .map(|p| p.at.xz().length())
            .fold(0.0f32, f32::max);
        assert!(ball_w > col_w * 1.5, "a mushroom: the ball {ball_w} m out, the column {col_w} m");
    }

    #[test]
    fn the_smoke_hangs_a_minute_spreading_half_again_then_fades_away() {
        let n = spec();
        assert_eq!(lasts_s(&n), 66.0);
        let shown = |ps: &[crate::soft_sprites::Placed]| ps.iter().filter(|p| p.colour.w > 0.05).count();
        let size = |ps: &[crate::soft_sprites::Placed]| ps.iter().map(|p| p.size_m).sum::<f32>();
        let done = cloud(&n, Vec3::ZERO, n.cloud_s, Vec3::Y);
        let hanging = cloud(&n, Vec3::ZERO, n.cloud_s + 40.0, Vec3::Y);
        assert_eq!(shown(&hanging), hanging.len(), "all of it still there 40 s on");
        let spread = size(&hanging) / size(&done);
        assert!(spread > 1.3 && spread < 1.5, "spreading toward half again its size: {spread}");
        let gone = cloud(&n, Vec3::ZERO, lasts_s(&n) - 0.01, Vec3::Y);
        assert!(shown(&gone) < gone.len() / 20, "faded away at the end: {}", shown(&gone));
    }

    #[test]
    fn the_ball_cools_as_one_with_no_puff_left_burning_in_the_smoke() {
        use crate::soft_sprites::Look;
        let n = spec();
        for k in 1..60 {
            let t = n.cloud_s * k as f32 / 60.0;
            let mut fire: Vec<f32> = cloud(&n, Vec3::ZERO, t, Vec3::Y)
                .iter()
                .take(BALL as usize)
                .map(|p| if let Look::Burning(f) = p.look { f } else { 0.0 })
                .collect();
            fire.sort_by(f32::total_cmp);
            let (median, hottest) = (fire[fire.len() / 2], fire[fire.len() - 1]);
            assert!(
                hottest - median < 0.6,
                "at {t:.1} s a puff burns at {hottest} in a ball at {median}"
            );
        }
    }

    #[test]
    fn its_fire_cools_into_smoke_and_its_foot_is_dust() {
        use crate::soft_sprites::Look;
        let n = spec();
        let burning = |ps: &[crate::soft_sprites::Placed]| ps.iter().filter(|p| matches!(p.look, Look::Burning(k) if k > 0.5)).count();
        let first = cloud(&n, Vec3::ZERO, 0.3, Vec3::Y);
        let late = cloud(&n, Vec3::ZERO, n.cloud_s, Vec3::Y);
        assert!(burning(&first) * 2 > BALL as usize, "the fireball burns: {}", burning(&first));
        assert!(burning(&late) * 10 < late.len(), "smoke by the end: {}", burning(&late));
        let hanging = cloud(&n, Vec3::ZERO, n.cloud_s + 5.0, Vec3::Y);
        assert_eq!(burning(&hanging), 0, "nothing still burning as the smoke hangs");
        let dust = late.iter().filter(|p| p.look == Look::Dust).count();
        assert!(
            dust >= SKIRT as usize && dust < late.len() / 2,
            "the skirt and the column's foot are dust: {dust}"
        );
    }

    #[test]
    fn every_puff_bursts_out_of_the_blast_point_into_place_within_half_a_second() {
        let n = spec();
        let at = Vec3::new(100.0, 20.0, -50.0);
        let first = cloud(&n, at, 0.0, Vec3::Y);
        assert!(first.iter().all(|p| p.at.distance(at) < 1e-3), "all from the one point");
        let early = cloud(&n, at, 0.1, Vec3::Y);
        let placed = cloud(&n, at, BURST_S.1, Vec3::Y);
        let reach = |ps: &[crate::soft_sprites::Placed]| ps.iter().map(|p| p.at.distance(at)).sum::<f32>() / ps.len() as f32;
        assert!(reach(&early) > 0.0 && reach(&early) < reach(&placed), "flying outward");
        let size = |ps: &[crate::soft_sprites::Placed]| ps.iter().map(|p| p.size_m).sum::<f32>();
        assert!(size(&first) < size(&placed) * 0.2, "small as they leave it, full size in place");
    }

    #[test]
    fn it_burns_white_then_cools_into_smoke_then_dissolves_at_the_end() {
        assert_eq!(heat(0.0), 0.0, "white-hot at once");
        assert!(heat(1.0) < 0.5 && heat(3.6) >= 1.0, "smoke by 3.6 of his 5 s");
        assert_eq!(pose(3.0).dissolve, 0.0, "whole while it rises");
        assert!(pose(5.0).dissolve > 0.99, "gone at the end");
    }
}
