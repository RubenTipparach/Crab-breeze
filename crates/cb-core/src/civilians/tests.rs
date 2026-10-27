//! Tests for civilians, on a city laid out by the world's own layout code and a hand-placed air base,
//! with the shipped people data.

use super::*;
use crate::data::load_ron;
use crate::world::layout::lay_out_city;
use crate::world::{FlatGround, Rect};
use std::path::PathBuf;

fn data_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn shipped() -> PeopleData {
    PeopleData {
        cfg: load_ron(&data_path("data/people/people.ron")).expect("people.ron loads"),
        lines: load_ron(&data_path("data/people/lines.ron")).expect("lines.ron loads"),
    }
}

fn world_cfg() -> WorldConfig {
    load_ron(&data_path("data/world/world.ron")).expect("world.ron loads")
}

/// A 1.2 km city at the origin with its second block a park, and one base 2 km east.
struct Fixture {
    cfg: WorldConfig,
    cities: Vec<City>,
    bases: Vec<AirBase>,
    ground: FlatGround,
}

impl Fixture {
    fn new() -> Self {
        let cfg = world_cfg();
        let mut city = lay_out_city(&mut SimRng::new(7).fork("city"), 0, Vec2::ZERO, 600.0, 0.0, &cfg);
        for b in &mut city.blocks {
            b.park = false;
        }
        city.blocks[1].park = true;
        let base = AirBase {
            id: 0,
            centre: Vec2::new(2000.0, 0.0),
            half_m: cfg.bases.half_m,
            ground_m: 0.0,
            highway: 0,
            runway: crate::world::bases::Runway::beside(
                Vec2::new(2000.0, 0.0),
                cfg.bases.half_m,
                crate::world::bases::RunwaySide::Front,
                1800.0,
                0.0,
                &crate::world::BasesConfig::default(),
            ),
        };
        Self {
            cfg,
            cities: vec![city],
            bases: vec![base],
            ground: FlatGround::default(),
        }
    }

    fn world<'a>(&'a self, doors: &'a dyn Doors) -> World<'a> {
        World {
            cities: &self.cities,
            bases: &self.bases,
            cfg: &self.cfg,
            ground: &self.ground,
            doors,
        }
    }
}

const DT: f32 = 1.0 / 60.0;

/// Brings people in round `pilot` until nothing more comes.
fn fill(people: &mut People, w: &World, data: &PeopleData, pilot: Vec2) {
    people.keep_around(w, data, pilot);
    for _ in 0..200 {
        if people.keep_around(w, data, pilot) == 0 && !people.pending {
            break;
        }
    }
}

/// A stable summary of everyone, to centimetres.
fn fingerprint(p: &People) -> Vec<(u32, i64, i64, u16)> {
    p.list
        .iter()
        .map(|q| (q.id, (q.pos.x * 100.0).round() as i64, (q.pos.y * 100.0).round() as i64, q.role))
        .collect()
}

#[test]
fn the_shipped_people_data_loads() {
    let d = shipped();
    assert!(d.validate().is_ok());
}

#[test]
fn the_same_seed_brings_the_same_people_and_walks_them_the_same_way() {
    let f = Fixture::new();
    let data = shipped();
    let run = |seed: u64| {
        let w = f.world(&NoDoors);
        let mut people = People::new(seed);
        for i in 0..1200 {
            let pilot = Vec2::new(-300.0 + i as f32 * 0.5, 0.0);
            people.keep_around(&w, &data, pilot);
            people.step(&w, &data, DT);
        }
        fingerprint(&people)
    };
    let a = run(42);
    assert!(!a.is_empty());
    assert_eq!(a, run(42), "same seed, same people");
    assert_ne!(a, run(43), "another seed, other people");
}

#[test]
fn people_are_brought_in_round_the_pilot_and_let_go_far_from_them() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(1);
    let pilot = Vec2::ZERO;
    fill(&mut people, &w, &data, pilot);
    assert!(people.list.len() > 100, "a city centre is busy: {}", people.list.len());
    for p in &people.list {
        assert!(p.pos.distance(pilot) <= data.cfg.keep_radius_m);
    }
    // every block within the spawn radius has been peopled
    for (i, b) in f.cities[0].blocks.iter().enumerate() {
        if b.rect.distance(pilot) <= data.cfg.spawn_radius_m {
            assert!(people.active.contains(&Home::Block(0, i as u32)), "block {i}");
        }
    }
    // walk away: everyone is let go
    let gone = Vec2::new(0.0, 5000.0);
    people.keep_around(&w, &data, gone);
    assert!(people.list.is_empty());
    assert!(people.active.is_empty());
    // and back: the blocks are peopled again (a second visit, so other people)
    fill(&mut people, &w, &data, pilot);
    assert!(people.list.len() > 100);
}

#[test]
fn bringing_people_in_is_spread_over_steps_and_capped() {
    let f = Fixture::new();
    let mut data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(1);
    let first = people.keep_around(&w, &data, Vec2::ZERO);
    // one home may overshoot the per-step number, never by more than a home's people
    assert!(first < data.cfg.spawn_per_step + 60, "{first}");
    assert!(people.pending, "the rest come next step");
    data.cfg.max_people = 50;
    let mut capped = People::new(1);
    fill(&mut capped, &w, &data, Vec2::ZERO);
    assert_eq!(capped.list.len(), 50);
}

#[test]
fn the_heart_of_a_city_is_busier_than_its_edge() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let city = &f.cities[0];
    let per_m = |b: usize| {
        let r = city.blocks[b].rect.inset(f.cfg.sidewalk_m * 0.5).unwrap();
        let n = home_count(&w, &data.cfg, Home::Block(0, b as u32), &r, Place::Pavement);
        n / (2.0 * (r.size().x + r.size().y))
    };
    let by_distance = |b: &usize| city.blocks[*b].rect.centre().length();
    let mut blocks: Vec<usize> = (0..city.blocks.len()).filter(|b| !city.blocks[*b].park).collect();
    blocks.sort_by(|a, b| by_distance(a).total_cmp(&by_distance(b)));
    let (heart, edge) = (blocks[0], *blocks.last().unwrap());
    assert!(per_m(heart) > 3.0 * per_m(edge), "{} vs {}", per_m(heart), per_m(edge));
    // and so it is with the people actually brought in
    let mut people = People::new(5);
    let pilot = Vec2::ZERO;
    let mut cfg = data.clone();
    cfg.cfg.spawn_radius_m = 2000.0;
    cfg.cfg.keep_radius_m = 2000.0;
    fill(&mut people, &w, &cfg, pilot);
    let inner = people.count_within(Vec2::ZERO, 200.0) as f32 / (200.0f32 * 200.0);
    let ring =
        (people.count_within(Vec2::ZERO, 600.0) - people.count_within(Vec2::ZERO, 450.0)) as f32 / (600.0f32 * 600.0 - 450.0 * 450.0);
    assert!(inner > 2.0 * ring, "{inner} per m2 in the heart, {ring} at the edge");
}

#[test]
fn walkers_keep_to_the_pavements_and_the_crossings() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(3);
    fill(&mut people, &w, &data, Vec2::ZERO);
    let lots: Vec<Rect> = f.cities[0]
        .blocks
        .iter()
        .filter(|b| !b.park)
        .filter_map(|b| b.rect.inset(f.cfg.sidewalk_m))
        .collect();
    let mut crossed = false;
    for _ in 0..(60 * 120) {
        people.step(&w, &data, DT);
        for p in people.list.iter().filter(|p| p.place == Place::Pavement) {
            for lot in &lots {
                assert!(lot.distance(p.pos) > 0.0, "person {} at {} walked into a building lot", p.id, p.pos);
            }
            crossed |= matches!(p.home, Home::Block(_, b) if b != p.block);
        }
    }
    assert!(crossed, "someone crossed the street to another block");
}

#[test]
fn people_wait_at_crossings_stand_about_and_sit_on_park_benches() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(9);
    let park = f.cities[0].blocks[1].rect.centre();
    fill(&mut people, &w, &data, park);
    let (mut waiting, mut idle, mut sitting) = (false, false, false);
    let path = f.cities[0].blocks[1].rect.inset(data.cfg.park_path_inset_m).unwrap();
    let benches = routes::park_benches(&path);
    for _ in 0..(60 * 180) {
        people.step(&w, &data, DT);
        for p in &people.list {
            match p.activity {
                Activity::Waiting { .. } => waiting = true,
                Activity::Idle { .. } => idle = true,
                Activity::Sitting { .. } => {
                    sitting = true;
                    assert!(benches.iter().any(|(seat, _)| seat.distance(p.pos) < 0.01), "sits on a bench");
                    assert_eq!(p.place, Place::Park);
                }
                _ => {}
            }
        }
    }
    assert!(waiting && idle && sitting, "waiting {waiting}, idle {idle}, sitting {sitting}");
}

#[test]
fn a_crossing_reaches_the_block_opposite_in_the_same_row() {
    let f = Fixture::new();
    let city = &f.cities[0];
    // the block at the least corner of the grid, crossing +x from its corner 2 (the greatest corner)
    let b = (0..city.blocks.len())
        .filter(|i| !city.blocks[*i].park)
        .min_by(|a, b| {
            city.blocks[*a]
                .rect
                .min
                .length_squared()
                .total_cmp(&city.blocks[*b].rect.min.length_squared())
        })
        .unwrap() as u32;
    let here = city.blocks[b as usize].rect;
    let (to_b, to, k) = routes::crossing(city, &f.cfg, b, 4, Vec2::X).expect("a block across");
    let there = city.blocks[to_b as usize].rect;
    assert!((there.min.y - here.min.y).abs() < 0.5, "same row");
    assert!(there.min.x > here.max.x, "across, ahead");
    let loop_there = there.inset(f.cfg.sidewalk_m * 0.5).unwrap();
    assert!(routes::waypoint(&loop_there, k).distance(to) < 0.01);
    let from = routes::waypoint(&here.inset(f.cfg.sidewalk_m * 0.5).unwrap(), 4);
    assert!((to.y - from.y).abs() < 0.01, "straight across");
}

#[test]
fn a_shell_thirty_metres_off_sends_a_group_running_away_from_it() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(11);
    fill(&mut people, &w, &data, Vec2::ZERO);
    let group: Vec<u32> = people.list.iter().filter(|p| p.pos.length() < 60.0).map(|p| p.id).collect();
    assert!(group.len() >= 3, "a group near the centre");
    let blast = Vec3::new(30.0, 0.0, 0.0);
    let dead = people.danger(
        &w,
        &data,
        &Danger::Blast {
            at: blast,
            energy_mj: 4.0,
            by: None,
        },
    );
    let before: Vec<(u32, f32)> = people
        .list
        .iter()
        .filter(|p| group.contains(&p.id) && p.standing())
        .map(|p| (p.id, p.pos.distance(Vec2::new(blast.x, blast.z))))
        .collect();
    let fright = data.cfg.blast_fright_m_per_cbrt_mj * 4.0f32.cbrt();
    for p in people.list.iter().filter(|p| p.standing()) {
        let d = p.pos.distance(Vec2::new(blast.x, blast.z));
        if d < fright - 1.0 {
            assert!(matches!(p.activity, Activity::Fleeing { .. }), "person {} at {d:.0} m runs", p.id);
        }
    }
    for _ in 0..(60 * 4) {
        people.step(&w, &data, DT);
    }
    let mut away = 0;
    for (id, d0) in &before {
        if let Some(p) = people.list.iter().find(|p| p.id == *id) {
            let d1 = p.pos.distance(Vec2::new(blast.x, blast.z));
            if *d0 < fright - 1.0 {
                assert!(d1 > *d0, "person {id} ran from {d0:.1} m to {d1:.1} m");
                away += 1;
            }
        }
    }
    assert!(away >= 3, "{away} ran");
    for c in &dead {
        assert!(c.at.distance(blast) <= data.cfg.blast_lethal_m_per_cbrt_mj * 4.0f32.cbrt() + 2.0);
    }
}

/// One person standing at `at` on open ground, for the danger tests.
fn one_person(f: &Fixture, data: &PeopleData, at: Vec2) -> People {
    let w = f.world(&NoDoors);
    let mut people = People::new(1);
    fill(&mut people, &w, data, Vec2::new(2000.0, 0.0));
    people.list.truncate(1);
    let p = &mut people.list[0];
    p.pos = at;
    p.feet_y = 0.0;
    p.activity = Activity::Idle { left_s: 100.0 };
    people
}

#[test]
fn a_blast_kills_within_its_lethal_reach_and_frightens_beyond_it() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let lethal = data.cfg.blast_lethal_m_per_cbrt_mj * 8.0f32.cbrt();
    let mut near = one_person(&f, &data, Vec2::new(2000.0 + lethal - 0.5, 0.0));
    let blast = Danger::Blast {
        at: Vec3::new(2000.0, 0.0, 0.0),
        energy_mj: 8.0,
        by: Some(0),
    };
    let dead = near.danger(&w, &data, &blast);
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].cause, Cause::Blast);
    assert_eq!(dead[0].by, Some(0));
    assert!(!near.list[0].standing());
    let mut far = one_person(&f, &data, Vec2::new(2000.0 + lethal + 1.0, 0.0));
    assert!(far.danger(&w, &data, &blast).is_empty());
    assert!(matches!(far.list[0].activity, Activity::Fleeing { .. }));
}

#[test]
fn a_round_through_a_body_kills_and_one_overhead_only_frightens() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let at = Vec2::new(2000.0, 10.0);
    let shot = |y: f32| Danger::Shot {
        from: Vec3::new(at.x - 5.0, y, at.y),
        to: Vec3::new(at.x + 5.0, y, at.y),
        by: Some(0),
    };
    let mut hit = one_person(&f, &data, at);
    let dead = hit.danger(&w, &data, &shot(1.2));
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].cause, Cause::Shot);
    let mut over = one_person(&f, &data, at);
    assert!(over.danger(&w, &data, &shot(12.0)).is_empty());
    assert!(matches!(over.list[0].activity, Activity::Fleeing { .. }));
}

#[test]
fn a_mech_foot_crushes_beneath_it_and_frightens_round_it() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let at = Vec2::new(2000.0, 10.0);
    let foot = |dx: f32| Danger::Footfall {
        at: Vec3::new(at.x + dx, 0.0, at.y),
        by: Some(0),
    };
    let mut under = one_person(&f, &data, at);
    let dead = under.danger(&w, &data, &foot(2.0));
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].cause, Cause::Crushed);
    let mut round = one_person(&f, &data, at);
    assert!(round.danger(&w, &data, &foot(30.0)).is_empty());
    assert!(matches!(round.list[0].activity, Activity::Fleeing { .. }));
    let mut clear = one_person(&f, &data, at);
    assert!(clear.danger(&w, &data, &foot(200.0)).is_empty());
    assert!(matches!(clear.list[0].activity, Activity::Idle { .. }));
}

#[test]
fn the_struck_lie_still_then_are_cleared_away() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let at = Vec2::new(2000.0, 10.0);
    let mut p = one_person(&f, &data, at);
    p.danger(
        &w,
        &data,
        &Danger::Footfall {
            at: Vec3::new(at.x, 0.0, at.y),
            by: None,
        },
    );
    let steps = (data.cfg.down_s / DT) as usize;
    for _ in 0..steps - 2 {
        p.step(&w, &data, DT);
        assert_eq!(p.list[0].pos, at, "lies still");
    }
    for _ in 0..4 {
        p.step(&w, &data, DT);
    }
    assert!(p.list.is_empty());
}

#[test]
fn on_a_pavement_they_flee_along_the_street_not_into_the_buildings() {
    let r = Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(100.0, 80.0),
    };
    // on the side along x (z = 0), danger up and to the left: run along +x, never off the line
    let d = routes::flee_dir(Vec2::new(40.0, 0.0), Vec2::new(20.0, 30.0), Some(&r));
    assert_eq!(d, Vec2::X);
    // on the side along z (x = 100), danger below: run +z
    let d = routes::flee_dir(Vec2::new(100.0, 30.0), Vec2::new(90.0, 0.0), Some(&r));
    assert_eq!(d, Vec2::Y);
    // in the open, straight away
    let d = routes::flee_dir(Vec2::new(3.0, 4.0), Vec2::ZERO, None);
    assert!((d - Vec2::new(0.6, 0.8)).length() < 1e-5);
}

/// A street door at a fixed point.
struct OneDoor(Vec2);

impl Doors for OneDoor {
    fn refuge(&self, p: Vec2, danger: Vec2, reach_m: f32) -> Option<Vec2> {
        (self.0.distance(p) <= reach_m && self.0.distance(danger) > p.distance(danger)).then_some(self.0)
    }
}

#[test]
fn with_street_doors_they_run_inside_and_are_gone() {
    let f = Fixture::new();
    let data = shipped();
    let at = Vec2::new(2000.0, 10.0);
    let door = OneDoor(at + Vec2::new(0.0, 12.0));
    let w = f.world(&door);
    let mut p = one_person(&f, &data, at);
    p.danger(
        &w,
        &data,
        &Danger::Footfall {
            at: Vec3::new(at.x, 0.0, at.y - 40.0),
            by: None,
        },
    );
    assert!(matches!(p.list[0].activity, Activity::Fleeing { door: Some(_), .. }));
    for _ in 0..(60 * 6) {
        p.step(&w, &data, DT);
    }
    assert!(p.list.is_empty(), "inside");
}

#[test]
fn a_greeted_person_stops_turns_to_the_pilot_and_answers_for_their_place() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(4);
    fill(&mut people, &w, &data, Vec2::new(2000.0, 0.0));
    let target = people.list.iter().position(|p| p.place == Place::Base).unwrap();
    let p = people.list[target].pos;
    // the pilot 2 m south of them (+z), facing north (-z, yaw 0)
    let pilot = Vec3::new(p.x, 0.0, p.y + 2.0);
    for q in people.list.iter_mut().enumerate().filter(|(i, _)| *i != target).map(|(_, q)| q) {
        q.pos += Vec2::new(500.0, 0.0);
    }
    let (id, answer) = people.greet(&data, pilot, 0.0, None).expect("greeted");
    assert_eq!(id, people.list[target].id);
    assert!(!answer.job, "no job open, no job line");
    let role = &data.lines.roles[people.list[target].role as usize];
    assert_eq!(role.place, Place::Base);
    assert_eq!(answer.who, role.name);
    for _ in 0..(60 * 3) {
        people.step(&w, &data, DT);
    }
    let q = &people.list[target];
    assert_eq!(q.pos, p, "stopped");
    let face = Vec2::new(-q.heading_rad.sin(), -q.heading_rad.cos());
    assert!(face.dot(Vec2::Y) > 0.99, "faces the pilot");
    // facing away, or out of reach, greets nobody
    assert!(people.greet(&data, pilot, std::f32::consts::PI, None).is_none());
    assert!(people.greet(&data, pilot + Vec3::Z * 5.0, 0.0, None).is_none());
}

#[test]
fn a_greeting_tells_of_a_job_only_while_one_is_open() {
    let data = shipped();
    let mechanic = data.lines.roles.iter().position(|r| r.id == "mechanic").unwrap() as u16;
    let mut told = false;
    for n in 0..200 {
        let a = data.lines.answer(mechanic, 7, n, None).unwrap();
        assert!(!a.job && !a.text.contains("{job}"));
        let b = data.lines.answer(mechanic, 7, n, Some("Raiders in Port Ash")).unwrap();
        if b.job {
            assert!(b.text.contains("Raiders in Port Ash"));
            told = true;
        }
    }
    assert!(told, "some greetings tell of the job");
    // the same person and count always say the same
    assert_eq!(data.lines.answer(mechanic, 7, 3, None), data.lines.answer(mechanic, 7, 3, None));
}

#[test]
fn a_job_line_without_its_slot_is_refused() {
    let mut lines = shipped().lines;
    lines.lines[0].job = true;
    assert!(lines.validate().is_err());
}

#[test]
fn the_pilots_kills_count_against_the_nearest_base_and_others_do_not() {
    let f = Fixture::new();
    let mut bases = f.bases.clone();
    bases.push(AirBase {
        id: 1,
        centre: Vec2::new(-2000.0, 0.0),
        ..bases[0]
    });
    let mut toll = Toll::default();
    let c = |x: f32, by| Casualty {
        person: 0,
        at: Vec3::new(x, 0.0, 0.0),
        cause: Cause::Shot,
        by,
    };
    let loss = toll.record(&c(-100.0, Some(0)), 0, &bases).expect("the pilot's");
    assert_eq!(loss.base, Some(1));
    toll.record(&c(100.0, Some(0)), 0, &bases);
    toll.record(&c(150.0, Some(0)), 0, &bases);
    assert!(toll.record(&c(100.0, Some(7)), 0, &bases).is_none(), "an enemy's kill");
    assert!(toll.record(&c(100.0, None), 0, &bases).is_none());
    assert_eq!(toll.by_base.get(&0), Some(&2));
    assert_eq!(toll.by_base.get(&1), Some(&1));
    assert_eq!(toll.by_pilot(), 3);
    assert_eq!(toll.by_others, 2);
}

#[test]
fn bad_numbers_change_nothing() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(1);
    assert_eq!(people.keep_around(&w, &data, Vec2::NAN), 0);
    fill(&mut people, &w, &data, Vec2::ZERO);
    let before = people.clone();
    people.step(&w, &data, f32::NAN);
    people.step(&w, &data, -1.0);
    assert_eq!(people, before);
    let none = people.danger(
        &w,
        &data,
        &Danger::Blast {
            at: Vec3::NAN,
            energy_mj: 4.0,
            by: None,
        },
    );
    assert!(none.is_empty());
    assert_eq!(people, before);
}

#[test]
fn segments_measure_their_least_distance() {
    let d = segment_distance(
        Vec3::new(-1.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, 1.0),
    );
    assert!((d - 1.0).abs() < 1e-6);
    let d = segment_distance(Vec3::ZERO, Vec3::X, Vec3::new(3.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0));
    assert!((d - 2.0).abs() < 1e-6);
}

/// Seed 42's people after a minute's walk past the city centre, as one number. A change that moves
/// anyone changes it; update it only for a change meant to move people.
#[test]
fn golden_seed_42() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(42);
    for i in 0..3600 {
        let pilot = Vec2::new(-300.0 + i as f32 * 0.2, 0.0);
        people.keep_around(&w, &data, pilot);
        people.step(&w, &data, DT);
        if i == 1800 {
            people.danger(
                &w,
                &data,
                &Danger::Blast {
                    at: Vec3::new(0.0, 0.0, 40.0),
                    energy_mj: 4.0,
                    by: Some(0),
                },
            );
        }
    }
    let text = format!("{:?}", fingerprint(&people));
    let got = crate::rng::label_hash(&text);
    eprintln!("golden seed 42: {} people, {got:#x}", people.list.len());
    assert_eq!(got, GOLDEN_42);
}

const GOLDEN_42: u64 = 0x9aad_d894_81f3_1a6e;

/// A measurement instrument, not a check: the CPU time of a fixed step with 2,000 people (the design's
/// budget), and of a blast and twenty rounds among them. Run it in release:
/// `cargo test --release -p cb-core civilians::tests::cost_of_a_step -- --ignored --nocapture`.
#[test]
#[ignore]
fn cost_of_a_step() {
    let f = Fixture::new();
    let mut data = shipped();
    data.cfg.spawn_radius_m = 2000.0;
    data.cfg.keep_radius_m = 2000.0;
    data.cfg.pavement_per_100m = [20.0, 10.0];
    let w = f.world(&NoDoors);
    let mut people = People::new(42);
    fill(&mut people, &w, &data, Vec2::ZERO);
    assert_eq!(people.list.len(), data.cfg.max_people);
    let steps = 3000;
    let mut ms: Vec<f64> = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = std::time::Instant::now();
        people.keep_around(&w, &data, Vec2::new(i as f32 * 0.01, 0.0));
        people.step(&w, &data, DT);
        if i % 60 == 0 {
            people.danger(
                &w,
                &data,
                &Danger::Blast {
                    at: Vec3::new(0.0, 0.0, 40.0),
                    energy_mj: 4.0,
                    by: Some(0),
                },
            );
            for s in 0..20 {
                let z = s as f32 * 10.0 - 100.0;
                people.danger(
                    &w,
                    &data,
                    &Danger::Shot {
                        from: Vec3::new(-200.0, 1.0, z),
                        to: Vec3::new(-180.0, 1.0, z),
                        by: Some(0),
                    },
                );
            }
        }
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    ms.sort_by(f64::total_cmp);
    let q = |p: f64| ms[((ms.len() - 1) as f64 * p) as usize];
    eprintln!(
        "{} people: step p50 {:.3} ms, p95 {:.3} ms, p99 {:.3} ms, max {:.3} ms",
        people.list.len(),
        q(0.5),
        q(0.95),
        q(0.99),
        q(1.0)
    );
}

#[test]
fn street_doors_give_the_nearest_door_in_reach_that_is_not_toward_the_danger() {
    let doors = StreetDoors::new([Vec2::new(10.0, 0.0), Vec2::new(-20.0, 0.0), Vec2::new(0.0, 200.0)]);
    assert_eq!(doors.len(), 3);
    let p = Vec2::ZERO;
    // danger far to the south: the nearest door, 10 m east
    assert_eq!(doors.refuge(p, Vec2::new(0.0, -100.0), 60.0), Some(Vec2::new(10.0, 0.0)));
    // danger just east of the east door: that door is toward it, so the west one
    assert_eq!(doors.refuge(p, Vec2::new(14.0, 0.0), 60.0), Some(Vec2::new(-20.0, 0.0)));
    // nothing within reach
    assert_eq!(doors.refuge(p, Vec2::new(0.0, -100.0), 5.0), None);
    assert_eq!(StreetDoors::default().refuge(p, Vec2::new(0.0, -100.0), 60.0), None);
}

#[test]
fn people_run_in_through_the_real_street_doors_of_the_island() {
    use crate::world::{Structures, generate};
    let cfg = world_cfg();
    let plan = generate(42, &cfg);
    let structures = Structures::new(&plan);
    // the seed the game lays its interiors out with (cb-app `buildings::building_rng`)
    let seed = SimRng::new(42).fork("dressing");
    let started = std::time::Instant::now();
    let doors = StreetDoors::of_world(&plan, &structures, &cfg.interiors, &seed);
    eprintln!(
        "street doors: {} for {} buildings in {:.0} ms",
        doors.len(),
        plan.buildings.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(doors.len(), plan.buildings.len(), "every building has its street door");
    // a building's door, from the same rule the game hangs the leaves by
    let b = plan.buildings.len() / 2;
    let c = &structures.buildings[b];
    let site = crate::world::interior::Site::of(&plan, b as u32, c.n, c.origin, c.size);
    let entrance = crate::world::interior::shell(&site, &cfg.interiors, &seed).entrance;
    let door = entrance.route[0];
    // someone on the street 8 m out from it, a mech foot coming down 40 m further out
    let out = (door - entrance.centre).normalize_or(Vec2::X);
    let at = door + out * 8.0;
    let danger = door + out * 48.0;
    let data = shipped();
    let refuge = doors.refuge(at, danger, data.cfg.door_reach_m).expect("a door in reach");
    assert!(refuge.distance(at) <= 8.0 + 1e-3, "the nearest door, at most as far as this one");
    let f = Fixture::new();
    let w = f.world(&doors);
    let mut p = one_person(&f, &data, at);
    p.danger(
        &w,
        &data,
        &Danger::Footfall {
            at: Vec3::new(danger.x, 0.0, danger.y),
            by: None,
        },
    );
    assert!(
        matches!(p.list[0].activity, Activity::Fleeing { door: Some(d), .. } if d == refuge),
        "runs for the street door"
    );
    for _ in 0..(60 * 6) {
        p.step(&w, &data, DT);
    }
    assert!(p.list.is_empty(), "gone inside");
}

/// People stand on the drawn surface by the same rule as the pilot (`City::surface_top`, `WorldPlan::walk_top_at`):
/// on a pavement the kerb's top, on a street the road's top, never sunk in either.
#[test]
fn people_stand_on_the_pavement_and_road_tops_the_pilot_stands_on() {
    let f = Fixture::new();
    let data = shipped();
    let w = f.world(&NoDoors);
    let mut people = People::new(3);
    let city = &f.cities[0];
    fill(&mut people, &w, &data, city.centre);
    for _ in 0..120 {
        people.step(&w, &data, DT);
    }
    let (mut on_pavement, mut on_road) = (0, 0);
    for p in people.list.iter().filter(|p| matches!(p.home, Home::Block(..))) {
        let Some(top) = city.surface_top(p.pos, f.cfg.road_lift_m, f.cfg.kerb_height_m) else {
            continue;
        };
        assert!(
            (p.feet_y - top).abs() < 1e-4,
            "a person at {} stands at {} not {top}",
            p.pos,
            p.feet_y
        );
        if (top - f.cfg.kerb_height_m).abs() < 1e-4 {
            on_pavement += 1;
        } else {
            on_road += 1;
        }
    }
    assert!(on_pavement > 10, "{on_pavement} on the pavements");
    let _ = on_road;
}

/// One person walking, for the rules that read a person but not the world (`anim`).
pub(crate) fn a_person() -> Person {
    Person {
        id: 1,
        home: Home::Base(0),
        place: Place::Pavement,
        role: 0,
        pos: Vec2::ZERO,
        feet_y: 0.0,
        heading_rad: 0.0,
        height_m: 1.75,
        walk_m_s: 1.4,
        run_m_s: 4.5,
        pace_m_s: 0.0,
        look: 0,
        activity: Activity::Walking,
        greeted: 0,
        block: 0,
        leg: Leg::Straight { dir: Vec2::X },
        rng: SimRng::new(1),
    }
}
