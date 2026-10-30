//! `cb-tools traffic-bench`: a measurement instrument (openspec change `car-traffic`, design sections 5 and 6). The
//! island's lane graph for a seed, then the traffic round the start in the capital (the densest downtown) at the
//! shipped densities, stepped as the game steps it (`cb-app::traffic::step_traffic`): the wheels on the drawn road
//! (`OnRoads` over the live `Scene`), the refill kept out of the view cone and, beyond `hidden_beyond_m`, behind
//! cover (`traffic::in_cone`, `traffic::sight_blocked`). Three eyes: standing still on foot, a mech striding and a
//! mech skating on a circle round the start, looking where it goes. CPU only and headless: the simulation's own cost
//! per fixed step, by part (`traffic::StepCost`), not a frame time.

use cb_core::GameData;
use cb_core::car::Obstacle;
use cb_core::traffic::{CarState, MechNear, StepCost, Traffic, TrafficView, in_cone, sight_blocked, view_half_rad};
use cb_core::world::lanes::LaneGraph;
use cb_core::world::{Deformation, OnRoads, Scene, Structures, WorldPlan};
use glam::{Vec2, Vec3};
use std::path::PathBuf;
use std::process::ExitCode;

/// One eye to bench from: a name, its height over the ground, metres, and its speed along the streets from the
/// start, m/s (0: standing still on foot, looking north). A moving eye is the player's mech, which frightens the
/// cars near it and shoves the ones in its path off their lanes, as in the game.
struct Eye {
    name: &'static str,
    height_m: f32,
    speed_m_s: f32,
}

const EYES: [Eye; 3] = [
    Eye {
        name: "standing (on foot)",
        height_m: 1.7,
        speed_m_s: 0.0,
    },
    Eye {
        name: "striding (mech, 8 m/s)",
        height_m: 10.0,
        speed_m_s: 8.0,
    },
    Eye {
        name: "skating (mech, 20 m/s)",
        height_m: 10.0,
        speed_m_s: 20.0,
    },
];

/// A route along the lanes from the road lane nearest `start`, taking each lane's first way on, at least `length_m`
/// long: the points in order.
fn street_route(graph: &LaneGraph, start: Vec2, length_m: f32) -> Vec<Vec2> {
    let first = graph
        .road_lanes()
        .min_by(|a, b| {
            let d = |l: &cb_core::world::lanes::Lane| l.line.points.iter().map(|p| p.distance(start)).fold(f32::INFINITY, f32::min);
            d(a).total_cmp(&d(b)).then(a.id.cmp(&b.id))
        })
        .map(|l| l.id);
    let mut out: Vec<Vec2> = Vec::new();
    let mut len = 0.0;
    let mut lane = first;
    while let Some(id) = lane {
        let l = &graph.lanes[id as usize];
        for &p in &l.line.points {
            if let Some(&q) = out.last() {
                len += p.distance(q);
            }
            out.push(p);
        }
        if len >= length_m || out.len() > 1_000_000 {
            break;
        }
        lane = l.out.first().copied();
    }
    out
}

/// The point `s` metres along `route`, and the way it runs there.
fn along(route: &[Vec2], s: f32) -> (Vec2, Vec2) {
    let mut left = s;
    for w in route.windows(2) {
        let seg = w[0].distance(w[1]);
        if seg > 0.0 && left <= seg {
            return (w[0].lerp(w[1], left / seg), (w[1] - w[0]) / seg);
        }
        left -= seg;
    }
    let n = route.len();
    (route[n - 1], (route[n - 1] - route[n.saturating_sub(2)]).normalize_or(Vec2::NEG_Y))
}

/// Runs the bench: `traffic-bench [seed] [steps]`.
pub fn run(args: &[String]) -> ExitCode {
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
    let steps = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3600usize);
    let data = match GameData::load(&PathBuf::from("data"), "mechs/placeholder.ron") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("FAIL {e}");
            return ExitCode::FAILURE;
        }
    };
    let plan = cb_core::world::generate(seed, &data.world);
    let t0 = std::time::Instant::now();
    let graph = LaneGraph::build(&plan.cities, &plan.highways, &data.traffic.cfg.lanes, data.world.arterial_width_m);
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let lit = graph.nodes.iter().filter(|n| n.signal).count();
    println!(
        "seed {seed}: {} lanes, {} junctions ({lit} lit), graph built in {build_ms:.0} ms",
        graph.lanes.len(),
        graph.nodes.len()
    );
    let dug = Deformation::new(data.world.world_size_m);
    let structures = Structures::with_config(&plan, data.world.structure.clone(), data.world.craters.radius_m_per_cbrt_mj);
    let scene = Scene {
        plan: &plan,
        dug: &dug,
        trees: None,
        structures: Some(&structures),
        platforms: &[],
        interiors: None,
    };
    let ground = OnRoads {
        ground: &scene,
        plan: &plan,
    };
    for eye in &EYES {
        bench(&plan, &graph, &data, &ground, eye, seed, steps);
    }
    ExitCode::SUCCESS
}

fn bench<G: cb_core::world::Ground>(plan: &WorldPlan, graph: &LaneGraph, data: &GameData, ground: &G, eye: &Eye, seed: u64, steps: usize) {
    let d = &data.traffic;
    let dt = 1.0 / data.sim.fixed_step_hz;
    let half = view_half_rad(data.pilot.view_fov_deg);
    let start = plan.start.mech_xz;
    let route = street_route(graph, start, eye.speed_m_s * dt * steps as f32 + 100.0);
    // where the eye is and which way it looks at step `k`
    let pose = |k: usize| -> (Vec3, Vec2) {
        let (xz, fwd) = if eye.speed_m_s > 0.0 {
            along(&route, eye.speed_m_s * dt * k as f32)
        } else {
            (start, Vec2::new(0.0, -1.0))
        };
        let y = ground.height_at(xz.x, xz.y) + eye.height_m;
        (Vec3::new(xz.x, y, xz.y), fwd)
    };
    let mut traffic = Traffic::new(seed);
    let (e0, _) = pose(0);
    let never = |_: Vec3| false;
    let behind = |p: Vec3| sight_blocked(plan, e0, p);
    traffic.fill(graph, d, ground, e0, &never, &behind, &[], u32::MAX);
    let mut times: Vec<f64> = Vec::with_capacity(steps);
    let mut sum = StepCost::default();
    let mut cars = 0usize;
    let mut worst = (0usize, StepCost::default());
    for k in 0..steps {
        let (e, f) = pose(k);
        let fwd = Vec3::new(f.x, 0.0, f.y);
        let mech = [MechNear {
            at: Vec2::new(e.x, e.z),
            vel: f * eye.speed_m_s,
            radius_m: data.mech.locomotion.body_radius_m,
            piloted: true,
        }];
        let mechs: &[MechNear] = if eye.speed_m_s > 0.0 { &mech } else { &[] };
        let obstacles: Vec<Obstacle> = mechs
            .iter()
            .map(|m| Obstacle {
                id: u32::MAX,
                at: m.at,
                radius_m: m.radius_m,
                vel: m.vel,
                mass_kg: f32::INFINITY,
            })
            .collect();
        let view = TrafficView {
            graph,
            data: d,
            ground,
            mechs,
            obstacles: &obstacles,
            driver: None,
        };
        let t = std::time::Instant::now();
        traffic.step(&view, e, dt);
        let seen = |p: Vec3| in_cone(e, fwd, half, p);
        let behind = |p: Vec3| sight_blocked(plan, e, p);
        let bodies: Vec<(Vec2, f32)> = mechs.iter().map(|m| (m.at, m.radius_m)).collect();
        traffic.refill(graph, d, ground, e, &seen, &behind, &bodies);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if times.iter().all(|&m| m < ms) {
            worst = (k, traffic.cost);
        }
        times.push(ms);
        let c = traffic.cost;
        sum.mechs_us += c.mechs_us;
        sum.free_us += c.free_us;
        sum.lane_map_us += c.lane_map_us;
        sum.grant_us += c.grant_us;
        sum.follow_us += c.follow_us;
        sum.advance_us += c.advance_us;
        sum.place_us += c.place_us;
        sum.rest_us += c.rest_us;
        sum.fill_us += c.fill_us;
        sum.fill_tries += c.fill_tries;
        sum.fill_covered += c.fill_covered;
        cars += traffic.cars.len();
    }
    let mean_ms = times.iter().sum::<f64>() / steps as f64;
    times.sort_by(f64::total_cmp);
    let q = |p: f64| times[((times.len() - 1) as f64 * p).round() as usize];
    let n = steps as f64;
    let driving = traffic.cars.iter().filter(|c| c.state == CarState::Driving).count();
    let loose = traffic.cars.iter().filter(|c| c.body.is_some()).count();
    println!(
        "{}: {:.0} cars on average ({} at the end, {driving} driving, {loose} loose); step and refill mean {mean_ms:.3} ms, p50 {:.3}, p95 {:.3}, p99 {:.3}, max {:.3} ms",
        eye.name,
        cars as f64 / n,
        traffic.cars.len(),
        q(0.5),
        q(0.95),
        q(0.99),
        times[times.len() - 1]
    );
    println!(
        "  mean per step, ms: lane map {:.3}, junctions {:.3}, following {:.3}, advance {:.3}, place {:.3}, mechs {:.3}, free bodies {:.3}, rest {:.3}, refill {:.3} ({:.1} places tried, {:.1} sight lines walked)",
        sum.lane_map_us / n / 1000.0,
        sum.grant_us / n / 1000.0,
        sum.follow_us / n / 1000.0,
        sum.advance_us / n / 1000.0,
        sum.place_us / n / 1000.0,
        sum.mechs_us / n / 1000.0,
        sum.free_us / n / 1000.0,
        sum.rest_us / n / 1000.0,
        sum.fill_us / n / 1000.0,
        sum.fill_tries as f64 / n,
        sum.fill_covered as f64 / n,
    );
    let (k, w) = worst;
    println!(
        "  the slowest step ({k}), ms: lane map {:.3}, junctions {:.3}, following {:.3}, advance {:.3}, place {:.3}, free bodies {:.3}, refill {:.3} ({} places tried, {} sight lines walked)",
        w.lane_map_us / 1000.0,
        w.grant_us / 1000.0,
        w.follow_us / 1000.0,
        w.advance_us / 1000.0,
        w.place_us / 1000.0,
        w.free_us / 1000.0,
        w.fill_us / 1000.0,
        w.fill_tries,
        w.fill_covered,
    );
}
