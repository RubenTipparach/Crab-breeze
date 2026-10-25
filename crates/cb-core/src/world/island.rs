//! The island of `island-biomes` (owner, 2026-10-25: "I wanted this island"), ported from the
//! island mockup's generator (`docs/mockups/island_gen.js`, the reference implementation) so the
//! game's seed 42 is the island the mockup shows: a range through the middle on a curved spine,
//! running north-north-east, with passes for the highways; the desert plateau in its rain shadow to
//! the east, with mesas, dunes and a canyon system; beaches and a reef on the wide shelves, sea
//! cliffs on the north-east; an archipelago off the south-west.
//!
//! Its noise, randomness and rules are the mockup's line for line, in f64 like JavaScript, so the
//! same seed draws the same land; the game's rivers, lakes, cities and highways are then laid on
//! it by the game's own rules (`hydrology`, `layout`, `roads`). Pure functions of position: nothing
//! is stored but the spine, the canyons and the islets, worked out once from the seed.

use glam::Vec2;

/// The mockup's seeded random stream (mulberry32).
struct Mulberry(u32);

impl Mulberry {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// 0..1.
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// The mockup's lattice hash, 0..1.
fn hash(ix: i64, iz: i64, s: u32) -> f64 {
    let mut h = (ix as i32 as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((iz as i32 as u32).wrapping_mul(668_265_263))
        .wrapping_add(s.wrapping_mul(1_442_695_041));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    f64::from(h) / 4_294_967_296.0
}

/// Gradient noise, about -1..1.
fn perlin(x: f64, z: f64, s: u32) -> f64 {
    let (ix, iz) = (x.floor(), z.floor());
    let (fx, fz) = (x - ix, z - iz);
    let (ix, iz) = (ix as i64, iz as i64);
    let g = |cx: i64, cz: i64, dx: f64, dz: f64| {
        // The mockup's literal, not `TAU`: the same digits draw the same island.
        #[allow(clippy::approx_constant)]
        let a = hash(cx, cz, s) * 6.283_185_307;
        a.cos() * dx + a.sin() * dz
    };
    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uz = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);
    let a = g(ix, iz, fx, fz);
    let b = g(ix + 1, iz, fx - 1.0, fz);
    let c = g(ix, iz + 1, fx, fz - 1.0);
    let d = g(ix + 1, iz + 1, fx - 1.0, fz - 1.0);
    let top = a + (b - a) * ux;
    1.41 * (top + ((c + (d - c) * ux) - top) * uz)
}

/// Fractal gradient noise, about -1..1.
fn fbm(x: f64, z: f64, octaves: u32, s: u32) -> f64 {
    let (mut sum, mut amp, mut norm, mut x, mut z) = (0.0, 0.5, 0.0, x, z);
    for i in 0..octaves {
        sum += amp * perlin(x, z, s.wrapping_add(i * 17));
        norm += amp;
        amp *= 0.5;
        x *= 2.03;
        z *= 2.03;
    }
    sum / norm
}

/// Ridged multifractal (Musgrave), 0..1: detail gathers on the crests, valleys stay smooth.
fn ridged(x: f64, z: f64, octaves: u32, s: u32) -> f64 {
    let (mut sum, mut amp, mut norm, mut w, mut x, mut z) = (0.0, 0.5, 0.0, 1.0, x, z);
    for i in 0..octaves {
        let mut n = 1.0 - perlin(x, z, s.wrapping_add(i * 31)).abs();
        n *= n;
        n *= w;
        w = (n * 2.0).clamp(0.0, 1.0);
        sum += n * amp;
        norm += amp;
        amp *= 0.5;
        x *= 2.0;
        z *= 2.0;
    }
    sum / norm
}

/// Hermite smoothstep from `a` to `b` (either way round).
pub(crate) fn smooth(a: f64, b: f64, v: f64) -> f64 {
    let t = ((v - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Soft terraces: flat treads with risers of `riser` share of each step.
fn terrace(h: f64, step: f64, riser: f64) -> f64 {
    let t = h / step;
    let f = t.floor();
    (f + smooth(0.5 - riser / 2.0, 0.5 + riser / 2.0, t - f)) * step
}

/// Bearing of a point from the centre, degrees, 0 north (-z), 90 east (+x).
fn bearing(x: f64, z: f64) -> f64 {
    let b = x.atan2(-z).to_degrees();
    if b < 0.0 { b + 360.0 } else { b }
}

/// Weight 0..1 of a bearing inside `[from, to]`, soft by `soft` degrees.
fn sector(b: f64, from: f64, to: f64, soft: f64) -> f64 {
    let inside = |v: f64| smooth(from - soft, from + soft, v) * (1.0 - smooth(to - soft, to + soft, v));
    inside(b).max(inside(b - 360.0)).max(inside(b + 360.0))
}

/// Where a point lies against the range's spine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpineFix {
    /// Distance to the spine.
    pub d: f64,
    /// Position along it, 0 (south-south-west end) to 1 (north-north-east end).
    pub t: f64,
    /// Signed distance, east positive.
    pub s: f64,
    /// Signed distance from the nearest segment's line, continuous past the ends.
    pub perp: f64,
}

/// One cut of the canyon system: a segment with its depth and floor half-width.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cut {
    a: [f64; 2],
    b: [f64; 2],
    depth: f64,
    floor_half_m: f64,
}

/// An islet of the archipelago.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Islet {
    x: f64,
    z: f64,
    r: f64,
    h: f64,
    cliff: bool,
}

/// What a point's height came from, for the biomes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fields {
    /// Distance inland from the coast (negative at sea).
    pub coast_m: f32,
    /// How much of the north-east cliff coast this is, 0..1.
    pub cliff_coast: f32,
    /// Against the spine.
    pub spine: SpineFix,
    /// How much of the desert plateau, 0..1.
    pub plateau: f32,
    /// Depth of the canyon cut here.
    pub canyon_m: f32,
    /// Height of a mesa here.
    pub mesa_m: f32,
    /// Height of a dune here.
    pub dune_m: f32,
    /// Distance from the nearest islet's centre in its radii (9 far from every islet).
    pub islet_q: f32,
}

/// Where the range is pressed down for the highways, along the spine.
const PASSES: [f64; 2] = [0.33, 0.68];

/// The island, from a seed.
#[derive(Clone, Debug, PartialEq)]
pub struct Island {
    seed: u32,
    spine: Vec<[f64; 2]>,
    east: [f64; 2],
    cuts: Vec<Cut>,
    /// The cuts by 600 m cell (reaching 420 m out), for the nearest-cut query.
    cut_cells: std::collections::HashMap<(i64, i64), Vec<u32>>,
    islets: Vec<Islet>,
}

impl Default for Island {
    fn default() -> Self {
        Self::new(42)
    }
}

/// Half the mockup's world: islets stay 1.3 km inside it.
const HALF_M: f64 = 20480.0;
const CUT_CELL_M: f64 = 600.0;

impl Island {
    /// The island for `seed` (the mockup's `SEED`; the game's seed 42 is the mockup's island).
    pub fn new(seed: u32) -> Self {
        let deg = 20f64.to_radians();
        let north = [deg.sin(), -deg.cos()];
        let east = [-north[1], north[0]];
        let p0 = [-8000.0 * north[0] - 300.0 * east[0], -8000.0 * north[1] - 300.0 * east[1]];
        let p1 = [-1300.0 * east[0], -1300.0 * east[1]];
        let p2 = [8000.0 * north[0] + 500.0 * east[0], 8000.0 * north[1] + 500.0 * east[1]];
        let spine = (0..=64)
            .map(|i| {
                let t = f64::from(i) / 64.0;
                let u = 1.0 - t;
                [
                    u * u * p0[0] + 2.0 * u * t * p1[0] + t * t * p2[0],
                    u * u * p0[1] + 2.0 * u * t * p1[1] + t * t * p2[1],
                ]
            })
            .collect();
        let mut island = Self {
            seed,
            spine,
            east,
            cuts: Vec::new(),
            cut_cells: std::collections::HashMap::new(),
            islets: Vec::new(),
        };
        let mut rng = Mulberry::new(seed.wrapping_mul(7919));
        // The canyon system: three roots east of the spine, growing toward the east coast and
        // branching.
        for t in [0.28, 0.47, 0.64] {
            let p = island.spine_point(t);
            let ang = east[1].atan2(east[0]);
            let head = ang + (rng.next() - 0.5) * 0.4;
            let depth = 160.0 + rng.next() * 90.0;
            let fh = 40.0 + rng.next() * 60.0;
            island.grow(&mut rng, p[0] + east[0] * 4300.0, p[1] + east[1] * 4300.0, head, 42, depth, fh);
        }
        // The archipelago off the south-west.
        let mut tries = 0;
        while island.islets.len() < 32 && tries < 6000 {
            tries += 1;
            let b = (200.0 + rng.next() * 55.0).to_radians();
            let r = 13500.0 + rng.next() * 6500.0;
            let (x, z) = (b.sin() * r, -b.cos() * r);
            if x.abs().max(z.abs()) > HALF_M - 1300.0 {
                continue;
            }
            let off = -island.coast_dist(x, z);
            if !(2500.0..=8000.0).contains(&off) {
                continue;
            }
            if island.islets.iter().any(|o| (o.x - x).hypot(o.z - z) < 1150.0) {
                continue;
            }
            let rad = 120.0 + rng.next().powf(1.8) * 900.0;
            let h = 14.0 + rng.next() * 66.0 * (rad / 500.0 + 0.3).min(1.0);
            let cliff = rng.next() < 0.4;
            island.islets.push(Islet { x, z, r: rad, h, cliff });
        }
        for (i, c) in island.cuts.iter().enumerate() {
            let reach = 420.0;
            let cell = |v: f64| (v / CUT_CELL_M).floor() as i64;
            for cx in cell(c.a[0].min(c.b[0]) - reach)..=cell(c.a[0].max(c.b[0]) + reach) {
                for cz in cell(c.a[1].min(c.b[1]) - reach)..=cell(c.a[1].max(c.b[1]) + reach) {
                    island.cut_cells.entry((cx, cz)).or_default().push(i as u32);
                }
            }
        }
        island
    }

    fn s(&self, k: u32) -> u32 {
        self.seed.wrapping_add(k)
    }

    #[allow(clippy::too_many_arguments)]
    fn grow(&mut self, rng: &mut Mulberry, mut x: f64, mut z: f64, mut head: f64, steps: u32, depth: f64, fh: f64) {
        for i in 0..steps {
            head += (rng.next() - 0.5) * 0.55 + 0.12 * (-head).sin();
            let (nx, nz) = (x + head.cos() * 230.0, z + head.sin() * 230.0);
            if self.coast_dist(nx, nz) < 1700.0 {
                break;
            }
            let k = 1.0 - f64::from(i) / (f64::from(steps) * 1.4);
            self.cuts.push(Cut {
                a: [x, z],
                b: [nx, nz],
                depth: depth * (0.55 + 0.45 * k),
                floor_half_m: fh,
            });
            if i > 3 && rng.next() < 0.13 {
                let turn = if rng.next() < 0.5 { -1.0 } else { 1.0 } * (0.7 + rng.next() * 0.5);
                let branch_steps = 8 + (rng.next() * 12.0).floor() as u32;
                self.grow(rng, nx, nz, head + turn, branch_steps, depth * 0.7, fh * 0.6);
            }
            x = nx;
            z = nz;
        }
    }

    /// The point `t` (0..1) along the spine.
    pub fn spine_point(&self, t: f64) -> [f64; 2] {
        let f = t.clamp(0.0, 1.0) * 64.0;
        let i = (f.floor() as usize).min(63);
        let u = f - i as f64;
        let (a, b) = (self.spine[i], self.spine[i + 1]);
        [mix(a[0], b[0], u), mix(a[1], b[1], u)]
    }

    /// The direction across the spine toward the east (x, z).
    pub fn east(&self) -> Vec2 {
        Vec2::new(self.east[0] as f32, self.east[1] as f32)
    }

    /// Where `(x, z)` lies against the spine.
    pub fn spine_at(&self, x: f64, z: f64) -> SpineFix {
        let (mut best, mut bt, mut side, mut perp) = (1e18, 0.0, 0.0, 0.0);
        for i in 0..64 {
            let (a, b) = (self.spine[i], self.spine[i + 1]);
            let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
            let l2 = dx * dx + dz * dz;
            let u = (((x - a[0]) * dx + (z - a[1]) * dz) / l2).clamp(0.0, 1.0);
            let (px, pz) = (a[0] + dx * u, a[1] + dz * u);
            let d2 = (x - px) * (x - px) + (z - pz) * (z - pz);
            if d2 < best {
                best = d2;
                bt = (i as f64 + u) / 64.0;
                side = dx * (z - a[1]) - dz * (x - a[0]);
                perp = side / l2.sqrt();
            }
        }
        let d = best.sqrt();
        SpineFix {
            d,
            t: bt,
            s: if side > 0.0 { d } else { -d },
            perp,
        }
    }

    /// Distance inland from the coast at `(x, z)`: lobes, headlands and bays, coves, and bays proper
    /// up to 2 km deep. Negative at sea.
    pub fn coast_dist(&self, x: f64, z: f64) -> f64 {
        let r = x.hypot(z) + 1e-6;
        let (cx, cz) = (x / r, z / r);
        let big = 12300.0
            + 1700.0 * fbm(cx * 1.6 + 3.1, cz * 1.6 - 1.7, 4, self.s(1))
            + 700.0 * fbm(x / 2600.0, z / 2600.0, 3, self.s(2))
            + 420.0 * fbm(x / 1100.0, z / 1100.0, 3, self.s(30))
            + 140.0 * fbm(x / 320.0, z / 320.0, 2, self.s(31));
        let bay = 3200.0 * fbm(cx * 9.0 + 1.3, cz * 9.0 - 4.2, 2, self.s(32)).max(0.0).powf(1.6);
        big - bay - r
    }

    fn nearest_cut(&self, x: f64, z: f64) -> Option<(f64, &Cut)> {
        let key = ((x / CUT_CELL_M).floor() as i64, (z / CUT_CELL_M).floor() as i64);
        let mut best: Option<(f64, &Cut)> = None;
        for &i in self.cut_cells.get(&key)? {
            let c = &self.cuts[i as usize];
            let (dx, dz) = (c.b[0] - c.a[0], c.b[1] - c.a[1]);
            let u = (((x - c.a[0]) * dx + (z - c.a[1]) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let d = (x - c.a[0] - dx * u).hypot(z - c.a[1] - dz * u);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, c));
            }
        }
        best
    }

    /// The ground's height at `p` (before rivers, lakes, cities and roads), and what it came from.
    pub fn height(&self, p: Vec2) -> (f32, Fields) {
        let (x, z) = (f64::from(p.x), f64::from(p.y));
        let cd = self.coast_dist(x, z);
        let b = bearing(x, z);
        let cliff_w = sector(b, 18.0, 88.0, 10.0);
        let sp = self.spine_at(x, z);

        // The base island and its shelf; the north-east coast drops in sea cliffs.
        let hills = 38.0 * fbm(x / 3800.0, z / 3800.0, 4, self.s(3)) + 14.0 * fbm(x / 900.0, z / 900.0, 3, self.s(4));
        let beach_w = 120.0 + 380.0 * smooth(-0.2, 0.4, fbm(x / 2500.0, z / 2500.0, 2, self.s(34)));
        let mut land = 1.2
            + 3.0 * smooth(0.0, beach_w, cd)
            + (16.0 + 30.0 * smooth(0.0, 2500.0, cd)) * smooth(beach_w * 0.6, beach_w + 900.0, cd)
            + hills * smooth(beach_w, beach_w + 1200.0, cd);
        land = land.max(0.9 * smooth(0.0, 40.0, cd));
        let cliff_top = 95.0 + 55.0 * fbm(x / 3000.0, z / 3000.0, 3, self.s(5));
        land = mix(land, land.max(cliff_top), cliff_w);
        let shelf_n = fbm(x / 7000.0, z / 7000.0, 2, self.s(33));
        let shelf = 250.0 + 1350.0 * smooth(-0.25, 0.35, shelf_n);
        let off = (-cd).max(0.0);
        let mut sea = -1.5 - off * mix(0.004, 0.03, smooth(0.35, -0.25, shelf_n)) - (off - shelf).max(0.0) * 0.07;
        // The reef: a rim a metre under the surface along wide shelves, broken by gaps.
        let reef = smooth(0.0, 0.35, shelf_n)
            * (-((off - shelf * 0.85) / 110.0).powi(2)).exp()
            * smooth(-0.1, 0.25, fbm(x / 900.0, z / 900.0, 2, self.s(35)));
        sea = mix(sea, -1.0, reef);
        sea = mix(sea.max(-230.0), -8.0 - (off * 0.22).min(230.0), cliff_w);
        let wl = mix(smooth(-40.0, 40.0, cd), smooth(-30.0, 6.0, cd), cliff_w);
        let mut h = mix(sea, land, wl);

        // The range: a warped ridged multifractal under an envelope round the spine, banded.
        let taper = smooth(0.0, 0.16, sp.t) * (1.0 - smooth(0.84, 1.0, sp.t));
        let env = (-(sp.d / 4200.0).powi(2)).exp() * taper;
        let foot = (-(sp.d / 7000.0).powi(2)).exp() * smooth(0.0, 0.1, sp.t) * (1.0 - smooth(0.9, 1.0, sp.t));
        let mut range = 0.0;
        if env > 0.002 || foot > 0.01 {
            let wx = x + 1200.0 * fbm(x / 4000.0, z / 4000.0, 3, self.s(6));
            let wz = z + 1200.0 * fbm(x / 4000.0 + 9.2, z / 4000.0 - 4.1, 3, self.s(7));
            let rm = ridged(wx / 5000.0, wz / 5000.0, 6, self.s(8));
            range = env * (900.0 * (0.35 + 0.65 * rm) + 1300.0 * rm * rm) + foot * 260.0 * rm;
            let banded = terrace(range, 210.0, 0.25);
            range = mix(range, banded, 0.4 * smooth(250.0, 600.0, range));
            // Passes: pressed down to 500-700 m across a 1.5 km band.
            for pt in PASSES {
                if (sp.t - pt).abs() < 0.12 {
                    let along = (sp.t - pt).abs() * 16000.0;
                    let m = 1.0 - smooth(450.0, 1100.0, along);
                    range = mix(range, range.min(560.0 + 0.12 * (range - 560.0).max(0.0)), m);
                }
            }
        }
        h += range * smooth(-200.0, 1500.0, cd);

        // The rain shadow: the plateau east of the range, edged by an escarpment.
        let plat =
            smooth(3300.0, 3650.0, sp.s) * smooth(700.0, 2600.0, cd) * smooth(0.02, 0.15, sp.t + 0.1) * (1.0 - smooth(0.92, 1.05, sp.t));
        let (mut canyon, mut mesa, mut dune) = (0.0, 0.0, 0.0);
        if plat > 0.0 {
            let mut ph = 280.0 + 100.0 * fbm(x / 6000.0, z / 6000.0, 3, self.s(9));
            // Mesas and buttes: terraced blobs on the plateau.
            let blob = fbm(x / 1700.0, z / 1700.0, 4, self.s(10));
            mesa = smooth(0.34, 0.4, blob) * (70.0 + 110.0 * smooth(0.4, 0.75, blob));
            ph += terrace(mesa, 22.0, 0.3);
            // Dunes: a sand sea on part of the plateau, ridges across the wind.
            let sand = smooth(0.05, 0.25, fbm(x / 5200.0, z / 5200.0, 2, self.s(11))) * (1.0 - smooth(0.2, 0.34, blob));
            if sand > 0.0 {
                let (ux, uz) = (x * 0.94 + z * 0.34, -x * 0.34 + z * 0.94);
                dune = sand * (6.0 + 16.0 * (1.0 - perlin(ux / 320.0, uz / 900.0, self.s(12)).abs()).powi(3));
                ph += dune;
            }
            // Canyons: a terraced cut with ledged walls.
            if let Some((best, cut)) = self.nearest_cut(x, z) {
                let wall = 230.0;
                let cut_m = cut.depth * (1.0 - smooth(cut.floor_half_m, cut.floor_half_m + wall, best));
                canyon = terrace(cut_m, 18.0, 0.35);
                ph -= canyon;
            }
            h = h.max(mix(h, h + ph, plat));
            if h > 0.0 && canyon > 0.0 {
                h = (h - canyon * (1.0 - plat)).max(4.0);
            }
        }

        // The archipelago: small islands off the south-west, some cliffed.
        let mut islet_q = 9.0;
        if cd < -1500.0 {
            for o in &self.islets {
                let dd = (x - o.x).hypot(z - o.z);
                if dd > o.r * 1.6 {
                    continue;
                }
                let q = dd / (o.r * (1.0 + 0.32 * fbm(x / 380.0, z / 380.0, 3, self.s(13))));
                islet_q = f64::min(islet_q, q);
                let top = o.h * if o.cliff { smooth(1.0, 0.82, q) } else { smooth(1.05, 0.25, q) }
                    + 6.0 * fbm(x / 160.0, z / 160.0, 2, self.s(14)) * smooth(1.0, 0.6, q);
                let hi = mix(sea, top, smooth(1.2, 0.9, q));
                h = h.max(hi);
            }
        }
        let fields = Fields {
            coast_m: cd as f32,
            cliff_coast: cliff_w as f32,
            spine: sp,
            plateau: plat as f32,
            canyon_m: canyon as f32,
            mesa_m: mesa as f32,
            dune_m: dune as f32,
            islet_q: islet_q as f32,
        };
        (h as f32, fields)
    }

    /// Noise of the mockup's for the biomes, about -1..1.
    pub(crate) fn fbm(&self, x: f64, z: f64, octaves: u32, k: u32) -> f64 {
        fbm(x, z, octaves, self.s(k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::approx_constant)] // measured values that happen to look like constants
    /// `baseHeight(x, z)` of `docs/mockups/island_gen.js` (seed 42), run in Node on 2026-10-25:
    /// x, z, height, coast distance, plateau, canyon. It drew 383 canyon cuts and 32 islets.
    const MOCKUP: [(f64, f64, f64, f64, f64, f64); 30] = [
        (-18000.0000, 14000.0000, -230.0000, -10710.472, 0.0000, 0.000),
        (-16468.3000, 12812.7000, -230.0000, -8861.884, 0.0000, 0.000),
        (-14936.6000, 11625.4000, 15.1134, -7446.046, 0.0000, 0.000),
        (-13404.9000, 10438.1000, -230.0000, -5063.670, 0.0000, 0.000),
        (-11873.2000, 9250.8000, 60.6409, -3027.817, 0.0000, 0.000),
        (-10341.5000, 8063.5000, -81.1243, -1000.382, 0.0000, 0.000),
        (-8809.8000, 6876.2000, 7.9556, 417.138, 0.0000, 0.000),
        (-7278.1000, 5688.9000, 90.9961, 2829.235, 0.0000, 0.000),
        (-5746.4000, 4501.6000, 886.8613, 4666.765, 0.0000, 0.000),
        (-4214.7000, 3314.3000, 844.2558, 6193.758, 0.0000, 0.000),
        (-2683.0000, 2127.0000, 723.2197, 8625.306, 0.0000, 0.000),
        (-1151.3000, 939.7000, 703.6353, 10479.634, 0.0000, 0.000),
        (380.4000, -247.6000, 1770.7227, 12006.207, 0.0000, 0.000),
        (1912.1000, -1434.9000, 869.4755, 10429.184, 0.0000, 0.000),
        (3443.8000, -2622.2000, 723.3833, 8038.305, 0.0000, 0.000),
        (4975.5000, -3809.5000, 762.0685, 6043.961, 0.0641, 0.000),
        (6507.2000, -4996.8000, 224.3611, 4235.651, 0.4718, 0.000),
        (8038.9000, -6184.1000, 178.2100, 2252.930, 0.3010, 0.000),
        (9570.6000, -7371.4000, 92.2156, 245.936, 0.0000, 0.000),
        (11102.3000, -8558.7000, -238.0000, -1783.142, 0.0000, 0.000),
        (12634.0000, -9746.0000, -238.0000, -3487.339, 0.0000, 0.000),
        (14165.7000, -10933.3000, -238.0000, -5382.999, 0.0000, 0.000),
        (15697.4000, -12120.6000, -238.0000, -7292.395, 0.0000, 0.000),
        (17229.1000, -13307.9000, -238.0000, -9187.736, 0.0000, 0.000),
        (2000.0000, -6000.0000, 466.2261, 5892.611, 0.0000, 0.000),
        (6500.0000, 1200.0000, 354.6580, 5477.049, 1.0000, 18.000),
        (-4000.0000, 3000.0000, 881.4509, 6680.047, 0.0000, 0.000),
        (9000.0000, -2000.0000, 472.5203, 3263.456, 1.0000, 0.000),
        (-16000.0000, 11000.0000, -230.0000, -7310.789, 0.0000, 0.000),
        (12000.0000, 9000.0000, -151.1229, -2408.996, 0.0000, 0.000),
    ];

    #[test]
    fn seed_42_is_the_mockups_island() {
        let island = Island::new(42);
        assert_eq!(island.cuts.len(), 383);
        assert_eq!(island.islets.len(), 32);
        for (x, z, h, cd, plat, canyon) in MOCKUP {
            let (gh, f) = island.height(Vec2::new(x as f32, z as f32));
            // f32 positions and results: within a few centimetres of the mockup's f64.
            assert!((f64::from(gh) - h).abs() < 0.05, "height at ({x}, {z}): {gh} against {h}");
            assert!((f64::from(f.coast_m) - cd).abs() < 0.05, "coast at ({x}, {z})");
            assert!((f64::from(f.plateau) - plat).abs() < 1e-3, "plateau at ({x}, {z})");
            assert!((f64::from(f.canyon_m) - canyon).abs() < 0.05, "canyon at ({x}, {z})");
        }
    }

    #[test]
    fn the_range_stands_in_the_middle_and_the_sea_round_the_edge() {
        let island = Island::new(42);
        let mut top = f32::MIN;
        for k in 0..400 {
            let t = k as f64 / 400.0;
            let p = island.spine_point(0.2 + 0.6 * t);
            top = top.max(island.height(Vec2::new(p[0] as f32, p[1] as f32)).0);
        }
        assert!(top > 1300.0, "peaks through the clouds, {top}");
        for p in [Vec2::new(19000.0, 0.0), Vec2::new(0.0, -19000.0), Vec2::new(-19000.0, 19000.0)] {
            assert!(island.height(p).0 < 0.0, "sea at {p}");
        }
    }

    #[test]
    fn the_desert_plateau_lies_east_of_the_range() {
        let island = Island::new(42);
        let p = island.spine_point(0.5);
        let e = island.east();
        let east = Vec2::new(p[0] as f32, p[1] as f32) + e * 6000.0;
        let (h, f) = island.height(east);
        assert!(f.plateau > 0.5, "plateau {}", f.plateau);
        assert!(h > 150.0, "{h}");
        assert!(!island.cuts.is_empty() && island.islets.len() > 10);
    }

    #[test]
    fn the_same_seed_is_the_same_island() {
        let (a, b) = (Island::new(42), Island::new(42));
        for k in 0..50 {
            let p = Vec2::new(-15000.0 + k as f32 * 613.0, 9000.0 - k as f32 * 377.0);
            assert_eq!(a.height(p), b.height(p));
        }
        assert_ne!(
            Island::new(42).height(Vec2::new(500.0, 500.0)).0,
            Island::new(7).height(Vec2::new(500.0, 500.0)).0
        );
    }
}
