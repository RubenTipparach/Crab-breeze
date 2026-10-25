//! Rock formations (openspec change `rock-formations`; owner, 2026-10-25: "add stacks and
//! arches?"): sea stacks and arches off the north-east's cliff coast and round the cliffed islets,
//! and hoodoos on the canyon floors, placed by the island mockup's rule (`buildRocks` in
//! `docs/mockups/island.html`, with its own random stream, so they stand where the mockup puts
//! them). Terrain cannot make an arch or a stack: each rock is a kit piece (a mesh the game draws,
//! `cb-app::rocks`) placed, scaled and turned here. Pure: the same island gives the same rocks.

use super::island::{Island, Mulberry};
use glam::{Vec2, Vec3};

/// What a rock is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RockKind {
    /// A sea stack: a pillar standing in the water off a cliff.
    Stack,
    /// A sea arch: a half ring of rock, its feet in the sea.
    Arch,
    /// A hoodoo: a thin spire capped by a wider stone, on a canyon floor.
    Hoodoo,
}

/// One rock: a kit piece at `base`, scaled and turned about the vertical.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rock {
    /// What it is.
    pub kind: RockKind,
    /// Its base (the kit piece's origin), world metres.
    pub base: Vec3,
    /// Its size: the kit piece (about a unit across) times this, x across, y up, z along.
    pub scale: Vec3,
    /// Its turn about the vertical, radians.
    pub yaw_rad: f32,
    /// Its brightness, about 0.85-1.15 (no two stacks quite alike).
    pub shade: f32,
}

/// A full turn as the mockup writes it (6.28, not `TAU`): the same digits place the same rocks.
#[allow(clippy::approx_constant)]
const TURN: f32 = 6.28;

/// The rocks of an island: `ground` is the ground's height (the game's, with its rivers), and
/// `keep_clear` refuses a place (water, roads, cities).
pub fn make_rocks(island: &Island, ground: &dyn Fn(Vec2) -> f32, keep_clear: &dyn Fn(Vec2) -> bool) -> Vec<Rock> {
    let mut r = Mulberry::new(4242);
    let mut f = || r.next() as f32;
    let mut out = Vec::new();
    let polar = |b_deg: f32, rad: f32| {
        let b = b_deg.to_radians();
        Vec2::new(b.sin() * rad, -b.cos() * rad)
    };
    // Sea stacks off the north-east cliffs, 60-520 m out from the coast.
    let mut stacks = 0;
    for _ in 0..4000 {
        if stacks >= 70 {
            break;
        }
        let (b, rad) = (20.0 + f() * 66.0, 9000.0 + f() * 7000.0);
        let p = polar(b, rad);
        let cd = island.coast_dist(f64::from(p.x), f64::from(p.y)) as f32;
        if !(-520.0..=-60.0).contains(&cd) {
            continue;
        }
        let (h0, rd) = (30.0 + f() * 90.0, 12.0 + f() * 22.0);
        let (sz, yaw, shade) = (rd * (0.7 + f() * 0.5), f() * TURN, 0.85 + f() * 0.3);
        out.push(Rock {
            kind: RockKind::Stack,
            base: Vec3::new(p.x, -20.0, p.y),
            scale: Vec3::new(rd, h0 + 20.0, sz),
            yaw_rad: yaw,
            shade,
        });
        stacks += 1;
    }
    // Three round each cliffed islet (not one that lands on a neighbouring islet).
    for (c, rad) in island.cliffed_islets() {
        for _ in 0..3 {
            let (a, d) = (f() * TURN, rad * (1.15 + f() * 0.4));
            let (sx, sy, sz, yaw) = (10.0 + f() * 12.0, 30.0 + f() * 50.0, 10.0 + f() * 12.0, f() * 6.0);
            let at = Vec2::new(c.x + a.cos() * d, c.y + a.sin() * d);
            if island.height(at).0 >= 0.0 {
                continue;
            }
            out.push(Rock {
                kind: RockKind::Stack,
                base: Vec3::new(at.x, -20.0, at.y),
                scale: Vec3::new(sx, sy, sz),
                yaw_rad: yaw,
                shade: 1.0,
            });
        }
    }
    // Arches just off the cliffs, 40-180 m out, spanning 60-200 m, turned along the coast.
    let mut arches = 0;
    for _ in 0..4000 {
        if arches >= 7 {
            break;
        }
        let (b, rad) = (25.0 + f() * 55.0, 9000.0 + f() * 7000.0);
        let p = polar(b, rad);
        let cd = island.coast_dist(f64::from(p.x), f64::from(p.y)) as f32;
        if !(-180.0..=-40.0).contains(&cd) {
            continue;
        }
        let span = 60.0 + f() * 140.0;
        let (sy, sz) = (span / 2.0 * (0.8 + f() * 0.5), 18.0 + f() * 14.0);
        out.push(Rock {
            kind: RockKind::Arch,
            base: Vec3::new(p.x, -6.0, p.y),
            scale: Vec3::new(span / 2.0, sy, sz),
            yaw_rad: -b.to_radians(),
            shade: 1.0,
        });
        arches += 1;
    }
    // Hoodoos on the canyon floors (40 m deep and more), clear of water and roads.
    let cuts: Vec<(Vec2, Vec2, f32)> = island.canyon_cuts().collect();
    let mut hoodoos = 0;
    for _ in 0..6000 {
        if hoodoos >= 500 || cuts.is_empty() {
            break;
        }
        let (a, b, fh) = cuts[((f() * cuts.len() as f32) as usize).min(cuts.len() - 1)];
        let u = f();
        let p = a + (b - a) * u + Vec2::new(f() - 0.5, f() - 0.5) * fh * 2.4;
        let (_, fields) = island.height(p);
        if fields.canyon_m < 40.0 || keep_clear(p) {
            continue;
        }
        let (h0, yaw, shade) = (15.0 + f() * 45.0, f() * TURN, 0.85 + f() * 0.3);
        out.push(Rock {
            kind: RockKind::Hoodoo,
            base: Vec3::new(p.x, ground(p) - 1.0, p.y),
            scale: Vec3::new(h0 * 0.32, h0, h0 * 0.32),
            yaw_rad: yaw,
            shade,
        });
        hoodoos += 1;
    }
    out
}

impl Rock {
    /// The circles (centre x, z and radius) its foot stands on, for bodies walking into it: a
    /// stack's or a hoodoo's base, an arch's two feet.
    pub fn feet(&self) -> Vec<(Vec2, f32)> {
        let c = Vec2::new(self.base.x, self.base.z);
        match self.kind {
            RockKind::Stack => vec![(c, 0.9 * self.scale.x.min(self.scale.z))],
            RockKind::Hoodoo => vec![(c, 0.42 * self.scale.x)],
            RockKind::Arch => {
                // The kit's arch is a half ring of radius 1 and tube 0.3 in its x-y plane.
                let along = Vec2::new(self.yaw_rad.cos(), -self.yaw_rad.sin()) * self.scale.x;
                let r = 0.3 * self.scale.x.min(self.scale.z);
                vec![(c + along, r), (c - along, r)]
            }
        }
    }

    /// Its highest point.
    pub fn top_m(&self) -> f32 {
        self.base.y + self.scale.y * if self.kind == RockKind::Arch { 1.3 } else { 1.0 }
    }
}

/// A body of `radius_m` at `p` (x, z) with its feet at `feet_y`, moved out of every rock it stands
/// against (one it is above is passed over).
pub fn push_out(rocks: &[Rock], p: Vec2, radius_m: f32, feet_y: f32) -> Vec2 {
    let mut p = p;
    for r in rocks {
        if r.top_m() <= feet_y {
            continue;
        }
        for (c, rr) in r.feet() {
            let d = p - c;
            let reach = rr + radius_m;
            let len = d.length();
            if len < reach {
                p = c + if len > 1e-4 { d / len } else { Vec2::X } * reach;
            }
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rocks() -> Vec<Rock> {
        let island = Island::new(42);
        make_rocks(&island, &|p| island.height(p).0, &|_| false)
    }

    #[test]
    fn the_same_island_gives_the_same_rocks() {
        assert_eq!(rocks(), rocks());
    }

    #[test]
    fn stacks_and_arches_stand_in_the_sea_off_the_cliffs_and_hoodoos_on_canyon_floors() {
        let island = Island::new(42);
        let all = rocks();
        let count = |k: RockKind| all.iter().filter(|r| r.kind == k).count();
        assert!(count(RockKind::Stack) > 70, "{}", count(RockKind::Stack));
        assert!((1..=7).contains(&count(RockKind::Arch)), "{}", count(RockKind::Arch));
        assert!(count(RockKind::Hoodoo) > 100, "{}", count(RockKind::Hoodoo));
        for r in &all {
            let p = Vec2::new(r.base.x, r.base.z);
            let (h, f) = island.height(p);
            match r.kind {
                RockKind::Stack | RockKind::Arch => assert!(h < 0.0, "{:?} on land at {p}", r.kind),
                RockKind::Hoodoo => {
                    assert!(f.canyon_m >= 40.0, "hoodoo off a canyon floor at {p}");
                    assert!((r.base.y - (h - 1.0)).abs() < 1e-3, "sunk a metre into the floor");
                }
            }
            assert!(r.scale.min_element() > 0.0 && r.base.is_finite());
        }
    }

    #[test]
    fn a_body_walks_round_a_stack_and_passes_over_it() {
        let stack = Rock {
            kind: RockKind::Stack,
            base: Vec3::new(100.0, -20.0, 50.0),
            scale: Vec3::new(20.0, 80.0, 20.0),
            yaw_rad: 0.0,
            shade: 1.0,
        };
        let at = Vec2::new(105.0, 50.0);
        let out = push_out(&[stack], at, 6.0, -5.0);
        assert!(
            (out.distance(Vec2::new(100.0, 50.0)) - 24.0).abs() < 1e-3,
            "pushed to the stack's edge: {out}"
        );
        assert_eq!(push_out(&[stack], at, 6.0, 61.0), at, "above its 60 m top");
        assert_eq!(push_out(&[stack], Vec2::new(200.0, 50.0), 6.0, 0.0), Vec2::new(200.0, 50.0));
    }

    #[test]
    fn an_arch_stands_on_two_feet_with_its_span_open() {
        let arch = Rock {
            kind: RockKind::Arch,
            base: Vec3::new(0.0, -6.0, 0.0),
            scale: Vec3::new(60.0, 50.0, 20.0),
            yaw_rad: 0.0,
            shade: 1.0,
        };
        let feet = arch.feet();
        assert_eq!(feet.len(), 2);
        assert!((feet[0].0.distance(feet[1].0) - 120.0).abs() < 1e-3);
        // Under the middle of the span a body passes.
        assert_eq!(push_out(&[arch], Vec2::ZERO, 6.0, 0.0), Vec2::ZERO);
    }

    #[test]
    fn a_kept_clear_place_gets_no_hoodoo() {
        let island = Island::new(42);
        let none = make_rocks(&island, &|p| island.height(p).0, &|_| true);
        assert!(none.iter().all(|r| r.kind != RockKind::Hoodoo));
    }
}
