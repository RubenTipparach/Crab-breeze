//! Civil air traffic's tests (openspec change `civil-air-traffic`, tasks 2.x): each reads as the
//! behaviour it pins, on flat ground with bases and cities placed by hand, against the shipped data.

use super::shape::{Spin, blockout, capsules, wreck};
use super::*;
use crate::data::load_ron;
use crate::world::City;

const DT: f32 = 1.0 / 60.0;

fn cfg() -> AirTrafficConfig {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/air_traffic.ron");
    load_ron(&p).unwrap()
}

fn flat(_: f32, _: f32) -> f32 {
    20.0
}

fn base(id: u32, x: f32, z: f32) -> AirBase {
    AirBase {
        id,
        centre: Vec2::new(x, z),
        half_m: Vec2::new(350.0, 250.0),
        ground_m: 20.0,
        highway: 0,
        runway: crate::world::bases::Runway::beside(
            Vec2::new(x, z),
            Vec2::new(350.0, 250.0),
            crate::world::bases::RunwaySide::Front,
            1800.0,
            0.0,
            &crate::world::BasesConfig::default(),
        ),
    }
}

fn city(id: u32, x: f32, z: f32) -> City {
    City {
        id,
        centre: Vec2::new(x, z),
        half_size_m: 600.0,
        ground_m: 20.0,
        streets: Vec::new(),
        blocks: Vec::new(),
    }
}

/// Flies `a` for up to `seconds`, returning every step's outcome.
fn fly(a: &mut Aircraft, c: &AirTrafficConfig, bases: &[AirBase], zones: &[Zone], seconds: f32) -> Vec<FlightStep> {
    let spec = c.kinds[&a.kind].clone();
    let ground = flat;
    let air = Air {
        ground: &ground,
        zones,
        bases,
        cfg: c,
    };
    let mut out = Vec::new();
    for _ in 0..(seconds / DT) as usize {
        let s = a.step(&spec, &air, DT);
        out.push(s);
        if s.done || s.crashed {
            break;
        }
    }
    out
}

#[test]
fn the_shipped_kinds_are_the_four_the_owner_asked_for_at_true_size() {
    let c = cfg();
    let roles: Vec<Role> = c.kinds.values().map(|k| k.role).collect();
    for r in [Role::Airliner, Role::Freighter, Role::LightPlane, Role::Helicopter] {
        assert!(roles.contains(&r), "{r:?}");
    }
    // The owner's "about 40 m" airliner.
    assert!((c.kinds["airliner"].length_m - 40.0).abs() < 5.0);
}

#[test]
fn every_kind_sounds_its_own_recorded_engine() {
    // Survey W49: a turbofan, a propeller and a rotor, cut from CC0 and public domain recordings.
    use super::Engine;
    let c = cfg();
    let sound = |k: &str| c.kinds[k].engine.map(|e| e.sound);
    assert_eq!(sound("airliner"), Some(Engine::Turbofan));
    assert_eq!(sound("freighter"), Some(Engine::Turbofan));
    assert_eq!(sound("light_plane"), Some(Engine::Propeller));
    assert_eq!(sound("helicopter"), Some(Engine::Rotor));
    let mut bad = c.clone();
    bad.kinds.get_mut("helicopter").unwrap().engine.as_mut().unwrap().pitch = 0.0;
    assert!(crate::data::Validate::validate(&bad).unwrap_err().contains("engine.pitch"));
}

#[test]
fn a_route_aircraft_that_cannot_hold_the_glide_path_from_cruise_is_refused() {
    let mut c = cfg();
    c.kinds.get_mut("airliner").unwrap().descent_m_s = 1.0;
    let e = crate::data::Validate::validate(&c).unwrap_err();
    assert!(e.contains("descent_m_s"), "{e}");
}

#[test]
fn an_airliner_takes_off_cruises_and_lands_on_the_far_bases_strip() {
    let c = cfg();
    let bases = [base(0, 0.0, 0.0), base(1, 16_000.0, 9_000.0)];
    let spec = &c.kinds["airliner"];
    let from = departure_strip(&bases[0], &c.routes, bases[1].centre);
    let mut a = Aircraft::departing(1, "airliner", spec, from, &bases[1], &c.routes);
    let mut top = 0.0f32;
    let mut lifted_at = None;
    let mut landed_at = None;
    let ground = flat;
    let air = Air {
        ground: &ground,
        zones: &[],
        bases: &bases,
        cfg: &c,
    };
    for i in 0..(1800.0 / DT) as usize {
        let s = a.step(spec, &air, DT);
        top = top.max(a.pos.y - 20.0);
        if lifted_at.is_none() && a.phase == Phase::Climb {
            lifted_at = Some(a.xz());
        }
        if s.landed {
            landed_at = Some((a.xz(), a.heading_rad));
        }
        if s.done {
            break;
        }
        assert!(i < (1799.0 / DT) as usize, "never finished: {:?} at {:?}", a.phase, a.pos);
    }
    // It lifted off before the end of its strip.
    let lift = lifted_at.expect("never lifted off");
    assert!((lift - from.start).dot(from.dir()) <= from.len(), "{lift:?}");
    // It climbed to near its cruise height on a 18 km route.
    assert!(top > spec.cruise_agl_m * 0.6, "top {top}");
    // It touched down on base 1's strip, lined up, and stopped on it.
    let (at, heading) = landed_at.expect("never landed");
    let Flight::Route { strip, .. } = a.flight else { panic!() };
    assert_eq!(strip.base, 1);
    let lateral = (at - strip.start).perp_dot(strip.dir()).abs();
    assert!(lateral < 40.0, "touched down {lateral:.0} m off the centre line");
    assert!(crate::units::wrap(heading - strip.heading()).abs() < 0.35, "heading {heading}");
    let along = (a.xz() - strip.start).dot(strip.dir());
    assert!(
        along >= 0.0 && along <= strip.len() + 1.0,
        "stopped {along:.0} m along a {:.0} m strip",
        strip.len()
    );
    assert!(matches!(a.phase, Phase::Parked { .. }));
}

#[test]
fn an_airliner_too_high_over_the_threshold_goes_round_again() {
    let c = cfg();
    let bases = [base(0, 0.0, 0.0), base(1, 20_000.0, 0.0)];
    let spec = &c.kinds["airliner"];
    let strip = landing_strip(&bases[1], &c.routes, Vec2::ZERO);
    let start = strip.start - strip.dir() * 1_000.0;
    let mut a = Aircraft::new(
        1,
        "airliner",
        spec,
        Vec3::new(start.x, 20.0 + 400.0, start.y),
        strip.heading(),
        spec.approach_m_s,
        Phase::Final,
        Flight::Route {
            to: 1,
            strip,
            via: Vec::new(),
            next_via: 0,
        },
    );
    let steps = fly(&mut a, &c, &bases, &[], 30.0);
    assert!(steps.iter().all(|s| !s.landed), "it landed 300 m up");
    assert_eq!(a.phase, Phase::Enroute, "it went round");
}

#[test]
fn a_route_bends_round_a_fight_and_never_enters_it() {
    let c = cfg();
    let bases = [base(0, 0.0, 0.0), base(1, 24_000.0, 0.0)];
    let spec = &c.kinds["airliner"];
    let strip = landing_strip(&bases[1], &c.routes, Vec2::new(2_000.0, 0.0));
    // the fight sits square across the straight way to the runway's line
    let fight = Zone {
        centre: Vec2::new(12_000.0, strip.start.y),
        radius_m: c.avoid.keep_clear_m,
    };
    let mut a = Aircraft::new(
        1,
        "airliner",
        spec,
        Vec3::new(2_000.0, 1_100.0, strip.start.y),
        crate::units::heading_of(Vec3::X),
        spec.cruise_m_s,
        Phase::Enroute,
        Flight::Route {
            to: 1,
            strip,
            via: Vec::new(),
            next_via: 0,
        },
    );
    let ground = flat;
    let zones = [fight];
    let air = Air {
        ground: &ground,
        zones: &zones,
        bases: &bases,
        cfg: &c,
    };
    let mut nearest = f32::INFINITY;
    let mut landed = false;
    for _ in 0..(1200.0 / DT) as usize {
        let s = a.step(spec, &air, DT);
        nearest = nearest.min(a.xz().distance(fight.centre));
        landed |= s.landed;
        if s.done {
            break;
        }
    }
    assert!(nearest >= fight.radius_m, "came within {nearest:.0} m of the fight");
    assert!(landed, "and still got there");
}

#[test]
fn a_flight_whose_base_comes_under_attack_diverts_to_the_nearest_quiet_one() {
    let c = cfg();
    let bases = [base(0, 0.0, 0.0), base(1, 20_000.0, 0.0), base(2, 14_000.0, 9_000.0)];
    let spec = &c.kinds["freighter"];
    let from = departure_strip(&bases[0], &c.routes, bases[1].centre);
    let mut a = Aircraft::departing(1, "freighter", spec, from, &bases[1], &c.routes);
    let zones = [Zone {
        centre: bases[1].centre,
        radius_m: c.avoid.keep_clear_m,
    }];
    let steps = fly(&mut a, &c, &bases, &zones, 1800.0);
    assert!(steps.iter().any(|s| s.diverted));
    assert!(steps.iter().any(|s| s.landed));
    let Flight::Route { to, strip, .. } = a.flight else { panic!() };
    assert_ne!(to, 1, "it did not land in the fight");
    assert_eq!(strip.base, to);
}

#[test]
fn a_helicopter_circles_its_city_in_its_band_and_leaves_when_the_city_is_attacked() {
    let c = cfg();
    let spec = &c.kinds["helicopter"];
    let centre = Vec2::new(5_000.0, 0.0);
    let radius = 500.0;
    let mut a = Aircraft::new(
        1,
        "helicopter",
        spec,
        Vec3::new(5_000.0 + radius, 20.0 + spec.cruise_agl_m, 0.0),
        0.0,
        spec.cruise_m_s,
        Phase::Orbit,
        Flight::Orbit {
            city: 0,
            centre,
            radius_m: radius,
            dir: 1.0,
        },
    );
    fly(&mut a, &c, &[], &[], 60.0);
    let mut far = 0.0f32;
    for _ in 0..120 {
        fly(&mut a, &c, &[], &[], 1.0);
        far = far.max((a.xz().distance(centre) - radius).abs());
        assert!((a.pos.y - 20.0 - spec.cruise_agl_m).abs() < 15.0, "{:?}", a.pos);
    }
    assert!(far < radius * 0.25, "strayed {far:.0} m off its circle");
    let zones = [Zone {
        centre,
        radius_m: c.avoid.keep_clear_m,
    }];
    fly(&mut a, &c, &[], &zones, 200.0);
    assert_eq!(a.phase, Phase::Leaving);
    assert!(a.xz().distance(centre) > c.avoid.keep_clear_m, "{:?}", a.pos);
}

#[test]
fn steering_clear_takes_the_near_tangent_and_leaves_a_zone_it_is_in() {
    let z = [Zone {
        centre: Vec2::new(1000.0, 0.0),
        radius_m: 300.0,
    }];
    // Straight at it: bent off to a side by at least the tangent angle.
    let d = steer_clear_of(Vec2::ZERO, Vec2::X, &z);
    assert!(d.angle_to(Vec2::X).abs() >= (0.3f32).asin(), "{d:?}");
    // Passing wide of it: unchanged.
    assert_eq!(steer_clear_of(Vec2::new(0.0, 500.0), Vec2::X, &z), Vec2::X);
    // Inside it: straight out.
    let out = steer_clear_of(Vec2::new(1100.0, 0.0), -Vec2::X, &z);
    assert!(out.dot(Vec2::X) > 0.99, "{out:?}");
}

fn steer_clear_of(p: Vec2, want: Vec2, z: &[Zone]) -> Vec2 {
    flight::steer_clear(p, want, z)
}

#[test]
fn a_shot_that_does_not_beat_the_armour_does_nothing_and_one_that_does_brings_it_down() {
    let c = cfg();
    let spec = &c.kinds["light_plane"];
    let mut a = Aircraft::new(
        1,
        "light_plane",
        spec,
        Vec3::new(0.0, 400.0, 0.0),
        0.0,
        spec.cruise_m_s,
        Phase::Orbit,
        Flight::Orbit {
            city: 0,
            centre: Vec2::ZERO,
            radius_m: 800.0,
            dir: 1.0,
        },
    );
    assert!(!a.hit(spec.armour_mm - 0.5, 1000.0, spec.armour_mm));
    assert_eq!(a.hp, spec.hp);
    assert!(a.hit(spec.armour_mm, spec.hp, spec.armour_mm));
    assert_eq!(a.phase, Phase::Falling);
    assert!(!a.hit(100.0, 100.0, 0.0), "a downed aircraft is not downed twice");
}

#[test]
fn a_downed_airliner_falls_with_its_momentum_and_crashes() {
    let c = cfg();
    let spec = &c.kinds["airliner"];
    let bases = [base(0, 0.0, 0.0), base(1, 30_000.0, 0.0)];
    let strip = landing_strip(&bases[1], &c.routes, Vec2::ZERO);
    let start = Vec3::new(0.0, 1_000.0, 0.0);
    let mut a = Aircraft::new(
        1,
        "airliner",
        spec,
        start,
        crate::units::heading_of(Vec3::X),
        spec.cruise_m_s,
        Phase::Enroute,
        Flight::Route {
            to: 1,
            strip,
            via: Vec::new(),
            next_via: 0,
        },
    );
    a.hit(100.0, 1e6, spec.armour_mm);
    let steps = fly(&mut a, &c, &bases, &[], 120.0);
    assert!(steps.last().is_some_and(|s| s.crashed), "never came down");
    assert_eq!(a.phase, Phase::Wrecked);
    assert!((a.pos.y - 20.0).abs() < 0.01);
    // About 14 s of fall from 980 m: it carries on several hundred metres the way it was going.
    assert!(a.pos.x > 500.0, "{:?}", a.pos);
}

#[test]
fn a_freighter_overflies_its_via_point_and_says_so_then_lands() {
    let c = cfg();
    let bases = [base(0, 0.0, 0.0), base(1, 20_000.0, 0.0)];
    let mut t = Traffic::new(7);
    let drop = Vec2::new(8_000.0, 6_000.0);
    let id = t.launch(&c, "freighter", &bases[0], &bases[1], vec![drop]).unwrap();
    let ground = flat;
    let island = Island {
        ground: &ground,
        bases: &bases,
        cities: &[],
        zones: &[],
    };
    // The pilot is at the drop point, looking straight up: the launch flies regardless.
    let w = Watcher {
        eye: Vec3::new(drop.x, 30.0, drop.y),
        look: Vec3::Y,
    };
    let mut c2 = c.clone();
    for k in c2.kinds.values_mut() {
        k.max_near = 0;
    }
    let mut passed = None;
    let mut landed = false;
    for _ in 0..(1500.0 / DT) as usize {
        for e in t.step(&c2, &w, &island, DT) {
            if e.id == id {
                match e.what {
                    Happened::Passed(0) => passed = Some(Vec2::new(e.at.x, e.at.z)),
                    Happened::Landed => landed = true,
                    _ => {}
                }
            }
        }
        if t.get(id).is_none() {
            break;
        }
    }
    let p = passed.expect("never passed its drop point");
    assert!(p.distance(drop) < 300.0, "passed it {:.0} m off", p.distance(drop));
    assert!(landed);
}

fn island_scene() -> (Vec<AirBase>, Vec<City>) {
    (
        vec![
            base(0, -12_000.0, -6_000.0),
            base(1, 10_000.0, -9_000.0),
            base(2, 2_000.0, 11_000.0),
            base(3, -14_000.0, 12_000.0),
            base(4, 15_000.0, 8_000.0),
        ],
        vec![city(0, 0.0, 0.0), city(1, 6_000.0, 4_000.0), city(2, -8_000.0, 3_000.0)],
    )
}

/// Runs traffic for `seconds` with the pilot at `w`; returns it and every aircraft as first seen.
fn run_traffic(seed: u64, w: Watcher, seconds: f32, zones: &[Zone]) -> (Traffic, Vec<Aircraft>) {
    let c = cfg();
    let (bases, cities) = island_scene();
    let ground = flat;
    let island = Island {
        ground: &ground,
        bases: &bases,
        cities: &cities,
        zones,
    };
    let mut t = Traffic::new(seed);
    let mut seen: Vec<Aircraft> = Vec::new();
    for _ in 0..(seconds / DT) as usize {
        t.step(&c, &w, &island, DT);
        for a in &t.list {
            if !seen.iter().any(|s| s.id == a.id) {
                seen.push(a.clone());
            }
        }
    }
    (t, seen)
}

#[test]
fn traffic_appears_only_where_the_pilot_cannot_see_it_appear() {
    let c = cfg();
    let w = Watcher {
        eye: Vec3::new(500.0, 40.0, 500.0),
        look: Vec3::NEG_Z,
    };
    let (_, seen) = run_traffic(42, w, 600.0, &[]);
    assert!(seen.len() >= 6, "only {} aircraft in ten minutes", seen.len());
    for a in &seen {
        assert!(hidden(&w, a.pos, &c.near), "{} appeared in sight at {:?}", a.kind, a.pos);
    }
    let kinds: std::collections::BTreeSet<&str> = seen.iter().map(|a| a.kind.as_str()).collect();
    assert_eq!(kinds.len(), 4, "every kind flies: {kinds:?}");
}

#[test]
fn traffic_lives_near_the_pilot_only_and_never_past_each_kinds_cap() {
    let c = cfg();
    let w = Watcher {
        eye: Vec3::new(0.0, 40.0, 0.0),
        look: Vec3::X,
    };
    let (t, _) = run_traffic(3, w, 300.0, &[]);
    for (name, k) in &c.kinds {
        let n = t.list.iter().filter(|a| &a.kind == name).count() as u32;
        assert!(n <= k.max_near, "{name}: {n}");
    }
    let cap: u32 = c.kinds.values().map(|k| k.max_near).sum();
    assert!(t.list.len() as u32 <= cap);
    // The pilot is gone 60 km out to sea: every aircraft is removed.
    let (bases, cities) = island_scene();
    let ground = flat;
    let island = Island {
        ground: &ground,
        bases: &bases,
        cities: &cities,
        zones: &[],
    };
    let mut t = t;
    let away = Watcher {
        eye: Vec3::new(60_000.0, 40.0, 0.0),
        look: Vec3::X,
    };
    t.step(&c, &away, &island, DT);
    assert!(t.list.is_empty(), "{} left", t.list.len());
}

#[test]
fn no_flight_starts_in_a_fight_or_at_a_base_in_one() {
    let c = cfg();
    let (bases, _) = island_scene();
    let zones: Vec<Zone> = [bases[1].centre, Vec2::new(0.0, 0.0)]
        .into_iter()
        .map(|centre| Zone {
            centre,
            radius_m: c.avoid.keep_clear_m,
        })
        .collect();
    let w = Watcher {
        eye: Vec3::new(500.0, 40.0, 500.0),
        look: Vec3::NEG_Z,
    };
    let (_, seen) = run_traffic(11, w, 400.0, &zones);
    for a in &seen {
        assert!(!in_zone(a.xz(), &zones, 0.0), "{} started in a fight", a.kind);
        if let Flight::Route { to, .. } = a.flight {
            assert_ne!(to, 1, "a flight was sent to the base in a fight");
        }
        if let Flight::Orbit { city, .. } = a.flight {
            assert_ne!(city, 0, "a flight was sent to circle the city in a fight");
        }
    }
}

#[test]
fn the_same_seed_flies_the_same_traffic_and_another_does_not() {
    let w = Watcher {
        eye: Vec3::new(500.0, 40.0, 500.0),
        look: Vec3::NEG_Z,
    };
    let (a, _) = run_traffic(42, w, 120.0, &[]);
    let (b, _) = run_traffic(42, w, 120.0, &[]);
    let (c, _) = run_traffic(43, w, 120.0, &[]);
    assert_eq!(a, b);
    assert_ne!(a.list, c.list);
}

#[test]
fn hidden_means_far_or_behind() {
    let c = cfg();
    let w = Watcher {
        eye: Vec3::ZERO,
        look: Vec3::NEG_Z,
    };
    assert!(!hidden(&w, Vec3::new(0.0, 0.0, -5_000.0), &c.near), "ahead at 5 km is seen");
    assert!(hidden(&w, Vec3::new(0.0, 0.0, 5_000.0), &c.near), "behind at 5 km is not");
    assert!(!hidden(&w, Vec3::new(0.0, 0.0, 1_000.0), &c.near), "behind but too near");
    assert!(hidden(&w, Vec3::new(0.0, 0.0, -11_000.0), &c.near), "ahead past 10 km");
}

#[test]
fn every_blockout_is_its_kinds_true_size() {
    let c = cfg();
    for (name, k) in &c.kinds {
        let parts = blockout(k);
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        for p in &parts {
            lo = lo.min(p.centre - p.size * 0.5);
            hi = hi.max(p.centre + p.size * 0.5);
        }
        let size = hi - lo;
        assert!((size.x - k.span_m).abs() < 0.05 * k.span_m, "{name} span {}", size.x);
        assert!((size.z - k.length_m).abs() < 0.08 * k.length_m, "{name} length {}", size.z);
        // Standing on its wheels or skids: from the ground to the top of the fin or rotor.
        let tall = hi.y + rest_m(k);
        assert!((tall - k.height_m).abs() < 0.05 * k.height_m, "{name} height {tall}");
        if k.role == Role::Helicopter {
            assert!(parts.iter().any(|p| p.spin == Some(Spin::Up)), "{name} has a rotor");
        }
    }
}

#[test]
fn the_hit_capsules_cover_the_nose_and_the_wing_tips() {
    let c = cfg();
    let k = &c.kinds["airliner"];
    let a = Aircraft::new(
        1,
        "airliner",
        k,
        Vec3::new(10.0, 500.0, 0.0),
        0.7,
        k.cruise_m_s,
        Phase::Enroute,
        Flight::Orbit {
            city: 0,
            centre: Vec2::ZERO,
            radius_m: 1.0,
            dir: 1.0,
        },
    );
    let caps = capsules(&a, k);
    let q = shape::attitude(&a);
    let inside = |p: Vec3| {
        caps.iter().any(|c| {
            let ab = c.b - c.a;
            let t = ((p - c.a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            (c.a + ab * t).distance(p) <= c.radius_m + 0.01
        })
    };
    let nose = a.pos + q * Vec3::new(0.0, 0.0, -k.length_m * 0.5 + 0.5);
    assert!(inside(nose));
    let wing = blockout(k).into_iter().find(|b| b.name == "wing").unwrap();
    for s in [-1.0, 1.0] {
        assert!(inside(a.pos + q * (wing.centre + Vec3::X * s * (k.span_m * 0.5 - 1.0))));
    }
    assert!(!inside(a.pos + Vec3::Y * 30.0));
}

#[test]
fn a_wreck_is_the_same_pieces_every_time_and_lies_round_the_crash() {
    let c = cfg();
    let k = &c.kinds["airliner"];
    let at = Vec3::new(100.0, 20.0, -50.0);
    let a = wreck(k, 9, at, 1.0, c.fall.wreck_pieces);
    assert_eq!(a, wreck(k, 9, at, 1.0, c.fall.wreck_pieces));
    assert_ne!(a, wreck(k, 10, at, 1.0, c.fall.wreck_pieces));
    assert_eq!(a.len() as u32, c.fall.wreck_pieces);
    for (centre, size) in &a {
        assert!(centre.distance(at) < k.length_m * 1.5, "{centre:?}");
        assert!(size.min_element() > 0.0);
    }
}

#[test]
fn the_pilots_hits_are_tallied_against_the_nearest_base() {
    let (bases, _) = island_scene();
    let mut s = Standing::default();
    let near_two = bases[2].centre + Vec2::new(300.0, -200.0);
    let b = nearest_base(&bases, near_two).unwrap();
    assert_eq!(b, 2);
    s.record(b, false);
    s.record(b, true);
    s.record(nearest_base(&bases, bases[0].centre).unwrap(), false);
    assert_eq!(s.by_base[&2], Tally { hits: 2, downed: 1 });
    assert_eq!(s.by_base[&0], Tally { hits: 1, downed: 0 });
    assert_eq!(nearest_base(&[], Vec2::ZERO), None);
}

/// A measurement instrument (design section 8): CPU time of one traffic step on the shipped island
/// (its ground is noise read point by point, the dear part), every kind at its cap, the pilot at the
/// start. `cargo test -p cb-core --release -- --ignored measure_traffic_step --nocapture`.
#[test]
#[ignore]
fn measure_traffic_step() {
    use crate::world::Ground;
    let c = cfg();
    let plan = island_42();
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    let island = Island {
        ground: &ground,
        bases: &plan.bases,
        cities: &plan.cities,
        zones: &[],
    };
    let s = plan.start.mech_xz;
    let w = Watcher {
        eye: Vec3::new(s.x, plan.height_at(s.x, s.y) + 20.0, s.y),
        look: Vec3::NEG_Z,
    };
    let mut t = Traffic::new(42);
    for _ in 0..(600.0 / DT) as usize {
        t.step(&c, &w, &island, DT);
    }
    let n = 36_000;
    let mut flying = 0usize;
    let mut worst = 0.0f64;
    let start = std::time::Instant::now();
    for _ in 0..n {
        let one = std::time::Instant::now();
        t.step(&c, &w, &island, DT);
        worst = worst.max(one.elapsed().as_secs_f64());
        flying += t.list.len();
    }
    let us = start.elapsed().as_secs_f64() * 1e6 / n as f64;
    let probe = std::time::Instant::now();
    let mut sum = 0.0f32;
    for i in 0..100_000 {
        sum += plan.height_at(i as f32 * 0.37 - 15_000.0, (i % 977) as f32 * 31.0 - 15_000.0);
    }
    eprintln!(
        "one ground read: {:.2} us ({sum:.0})",
        probe.elapsed().as_secs_f64() * 1e6 / 100_000.0
    );
    eprintln!(
        "traffic step on seed 42: {us:.1} us a fixed step on average, worst {:.0} us, {:.1} aircraft on average",
        worst * 1e6,
        flying as f64 / n as f64
    );
}

/// The shipped seed-42 island, its five bases and their real ground.
fn island_42() -> &'static crate::world::WorldPlan {
    static PLAN: std::sync::OnceLock<crate::world::WorldPlan> = std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world/world.ron");
        let w: crate::world::WorldConfig = load_ron(&p).unwrap();
        crate::world::generate(42, &w)
    })
}

#[test]
fn on_the_shipped_island_a_flight_from_every_base_lands_at_the_next_over_the_real_ground() {
    use crate::world::Ground;
    let plan = island_42();
    let c = cfg();
    assert_eq!(plan.bases.len(), 5, "the five bases (survey J5)");
    let ground = |x: f32, z: f32| plan.height_at(x, z);
    let island = Island {
        ground: &ground,
        bases: &plan.bases,
        cities: &plan.cities,
        zones: &[],
    };
    for (i, from) in plan.bases.iter().enumerate() {
        let to = &plan.bases[(i + 1) % plan.bases.len()];
        for kind in ["airliner", "freighter"] {
            let mut t = Traffic::new(1);
            let id = t.launch(&c, kind, from, to, Vec::new()).unwrap();
            // The pilot rides along at the aircraft, so it is never removed for being far away.
            let mut no_spawn = c.clone();
            for k in no_spawn.kinds.values_mut() {
                k.max_near = 0;
            }
            let mut landed = None;
            let mut lowest_agl = f32::INFINITY;
            // A coarser step than the game's keeps the ten flights quick; the rules are the same.
            let dt = 0.05;
            for _ in 0..(2400.0 / dt) as usize {
                let Some(a) = t.get(id) else { break };
                let w = Watcher { eye: a.pos, look: Vec3::X };
                if matches!(a.phase, Phase::Enroute | Phase::Orbit) {
                    lowest_agl = lowest_agl.min(a.pos.y - plan.height_at(a.pos.x, a.pos.z));
                }
                for e in t.step(&no_spawn, &w, &island, dt) {
                    assert_ne!(e.what, Happened::Crashed, "{kind} {i} flew into the ground");
                    if e.what == Happened::Landed {
                        landed = Some(e.at);
                    }
                }
            }
            let at = landed.unwrap_or_else(|| panic!("{kind} from base {} never landed at base {}", from.id, to.id));
            // On the base's runway (survey W45), not its apron.
            assert!(
                to.runway.rect().distance(Vec2::new(at.x, at.z)) < 1.0,
                "{kind} touched down off base {}'s runway: {at:?}",
                to.id
            );
            assert!(
                lowest_agl >= c.routes.min_clear_m * 0.5,
                "{kind} {i} came within {lowest_agl:.0} m of the ground en route"
            );
        }
    }
}
