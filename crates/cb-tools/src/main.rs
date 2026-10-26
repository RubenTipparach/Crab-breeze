//! Crab Breeze command-line tools. `cb-tools validate <data dir>` loads every data file through the
//! same loader the game uses (CLAUDE.md section 11), so a file that would stop the game stops here.
//! `cities` and `map` are instruments: a seed's cities, and its island drawn from above.

mod audio_check;
mod glb;
mod map;

use cb_core::GameData;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") => {
            let root = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("data"));
            let mechs = GameData::mech_files(&root);
            if mechs.is_empty() {
                eprintln!("{}: no mech files in mechs/", root.display());
                return ExitCode::FAILURE;
            }
            let mut ok = true;
            for mech in &mechs {
                match GameData::load(&root, mech) {
                    Ok(d) => {
                        // the model it names, read as data: the rig contract, the sizes it was generated from, its
                        // cockpit's screens, eye, seat and lamps, and every node its boarding moves or vents from
                        let problems = glb::check_mech(&d.mech, Some(&d.boarding), &root.join("..").join("assets"));
                        if problems.is_empty() {
                            println!("ok   {}/{mech} (with sim, world, pilot, bindings, model, cockpit)", root.display());
                        } else {
                            for p in problems {
                                eprintln!("FAIL {mech}: {p}");
                            }
                            ok = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("FAIL {e}");
                        ok = false;
                    }
                }
            }
            // the enemy skins, once: every one names a shipped model with limbs to walk and shoot off
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_skins(&d.skins, &root.join("..").join("assets"));
                if problems.is_empty() {
                    println!("ok   {} enemy skins", d.skins.skins.len());
                }
                for p in problems {
                    eprintln!("FAIL {p}");
                    ok = false;
                }
                // the sounds: every file there, every placed one mono, music tempos match the scores
                let repo = root.join("..");
                let problems = audio_check::check_audio(&d.audio, &repo.join("assets"), &repo.join("music"));
                if problems.is_empty() {
                    println!(
                        "ok   {} sound effects, {} ambience beds, {} music cues",
                        d.audio.cues.cues.len(),
                        d.audio.ambience.beds.len(),
                        d.audio.music.cues.len()
                    );
                }
                for p in problems {
                    eprintln!("FAIL {p}");
                    ok = false;
                }
            }
            // the traffic cars' models against their data (car-traffic, car-driving)
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_cars(&d.traffic, &root.join("..").join("assets"));
                if problems.is_empty() {
                    println!("ok   {} car kinds and their models", d.traffic.kinds.len());
                }
                for p in problems {
                    eprintln!("FAIL {p}");
                    ok = false;
                }
            }
            // the people: roles and lines (civilians), loaded and checked with the rest
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let jobs = d.people.lines.lines.iter().filter(|l| l.job).count();
                println!(
                    "ok   {} people roles, {} lines ({jobs} tell of jobs)",
                    d.people.lines.roles.len(),
                    d.people.lines.lines.len()
                );
            }
            // civil air traffic's kinds (civil-air-traffic): loaded and validated with the rest; said here
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                println!("ok   {} civil aircraft kinds", d.air_traffic.kinds.len());
            }
            // the career: how it starts, the jobs, the catalogue and every mech it lists (career, mech-shop)
            match cb_core::career::CareerData::load(&root) {
                Ok(c) => {
                    println!(
                        "ok   career: {} job kinds, {} mechs for sale",
                        c.jobs.kinds.len(),
                        c.catalogue.listings.len()
                    );
                    // the air drop's aircraft is a civil air traffic kind (mech-shop, civil-air-traffic)
                    if let Ok(d) = GameData::load(&root, &mechs[0])
                        && let Err(e) = c.drop.check_aircraft(&d.air_traffic)
                    {
                        eprintln!("FAIL career/air_drop.ron: {e}");
                        ok = false;
                    }
                }
                Err(e) => {
                    eprintln!("FAIL {e}");
                    ok = false;
                }
            }
            // the Hermit's model against its shape data (crab-airship)
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_fortress(&d.fortress, &root.join("..").join("assets"));
                if problems.is_empty() {
                    println!("ok   the Hermit ({} parts)", d.fortress.shape.parts.len());
                }
                for p in problems {
                    eprintln!("FAIL {p}");
                    ok = false;
                }
            }
            if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        // A measurement instrument (openspec change `city-defence`): each city's size, buildings
        // and destructible cells for a seed, and how far it is from the start.
        Some("cities") => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
            let data = match GameData::load(&PathBuf::from("data"), "mechs/placeholder.ron") {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("FAIL {e}");
                    return ExitCode::FAILURE;
                }
            };
            let plan = cb_core::world::generate(seed, &data.world);
            let cells = cb_core::world::Structures::new(&plan);
            let start = plan.start.mech_xz;
            println!("seed {seed}: {} cities, {} buildings", plan.cities.len(), plan.buildings.len());
            for c in &plan.cities {
                let ids: Vec<usize> = plan.buildings.iter().filter(|b| b.city == c.id).map(|b| b.id as usize).collect();
                let n: usize = ids.iter().map(|&i| cells.buildings[i].intact.len()).sum();
                let tallest = ids.iter().map(|&i| plan.buildings[i].storeys).max().unwrap_or(0);
                println!(
                    "city {}: centre ({:.0}, {:.0}), half size {:.0} m, {} buildings, {n} cells, tallest {tallest} storeys, {:.0} m from the start",
                    c.id,
                    c.centre.x,
                    c.centre.y,
                    c.half_size_m,
                    ids.len(),
                    c.centre.distance(start)
                );
            }
            ExitCode::SUCCESS
        }
        // A measurement instrument (openspec change `car-traffic` design section 5): the island's lane graph for a
        // seed and what it costs to build, then the traffic filled round the start in the capital (the densest
        // downtown) at the shipped densities and stepped for `steps` fixed steps with the eye standing still, timing
        // each step. CPU only and headless: the simulation's own cost per fixed step, not a frame time.
        Some("traffic-bench") => {
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
            let graph =
                cb_core::world::lanes::LaneGraph::build(&plan.cities, &plan.highways, &data.traffic.cfg.lanes, data.world.arterial_width_m);
            let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
            let s = plan.start.mech_xz;
            let eye = glam::Vec3::new(s.x, plan.terrain.height_at(s.x, s.y) + 1.7, s.y);
            let mut traffic = cb_core::traffic::Traffic::new(seed);
            let never = |_: glam::Vec3| false;
            traffic.fill(&graph, &data.traffic, &plan, eye, &never, &never, u32::MAX);
            let dt = 1.0 / data.sim.fixed_step_hz;
            let view = cb_core::traffic::TrafficView {
                graph: &graph,
                data: &data.traffic,
                ground: &plan,
                mechs: &[],
                obstacles: &[],
                driver: None,
            };
            let mut times: Vec<f64> = Vec::with_capacity(steps);
            let mut fills: Vec<f64> = Vec::with_capacity(steps);
            for _ in 0..steps {
                let t = std::time::Instant::now();
                traffic.step(&view, eye, dt);
                let f = std::time::Instant::now();
                traffic.fill(&graph, &data.traffic, &plan, eye, &never, &never, data.traffic.cfg.spawn_per_step);
                fills.push(f.elapsed().as_secs_f64() * 1000.0);
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            fills.sort_by(f64::total_cmp);
            let pct = |v: &[f64], p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
            let q = |p: f64| pct(&times, p);
            println!(
                "of which the refill: p50 {:.3} ms, p99 {:.3} ms",
                pct(&fills, 0.5),
                pct(&fills, 0.99)
            );
            let driving = traffic
                .cars
                .iter()
                .filter(|c| c.state == cb_core::traffic::CarState::Driving)
                .count();
            println!(
                "seed {seed}: {} lanes, {} junctions, graph built in {build_ms:.0} ms; {} cars round the start ({driving} driving, the rest parked)",
                graph.lanes.len(),
                graph.nodes.len(),
                traffic.cars.len()
            );
            println!(
                "traffic step and refill over {steps} steps: p50 {:.3} ms, p95 {:.3} ms, p99 {:.3} ms, max {:.3} ms",
                q(0.5),
                q(0.95),
                q(0.99),
                times[times.len() - 1]
            );
            ExitCode::SUCCESS
        }
        // A documentation instrument (openspec change `civil-air-traffic`): every civil aircraft kind's
        // size and block-out boxes as JSON, from the one table the game draws
        // (`cb_core::air_traffic::shape`), for the mockup page `tools/mockups/gen_civil_aircraft.py`.
        Some("aircraft") => {
            let root = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("data"));
            let data = match GameData::load(&root, "mechs/placeholder.ron") {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("FAIL {e}");
                    return ExitCode::FAILURE;
                }
            };
            let kinds: Vec<serde_json::Value> = data
                .air_traffic
                .kinds
                .iter()
                .map(|(id, k)| {
                    let blocks: Vec<serde_json::Value> = cb_core::air_traffic::shape::blockout(k)
                        .iter()
                        .map(|b| {
                            serde_json::json!({
                                "name": b.name,
                                "centre": b.centre.to_array(),
                                "size": b.size.to_array(),
                                "dark": b.dark,
                                "spin": b.spin.map(|s| format!("{s:?}")),
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "id": id,
                        "role": format!("{:?}", k.role),
                        "length_m": k.length_m,
                        "span_m": k.span_m,
                        "height_m": k.height_m,
                        "rest_m": cb_core::air_traffic::rest_m(k),
                        "tint": k.tint,
                        "blocks": blocks,
                    })
                })
                .collect();
            let out = serde_json::json!({ "source": "data/world/air_traffic.ron", "kinds": kinds });
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            ExitCode::SUCCESS
        }
        // A documentation instrument (openspec change `hydrology`): the generated island seen from
        // above, as a PPM picture: ground by height, snow, the sea and lakes, rivers, waterfalls
        // (red), cities and highways.
        Some("map") => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
            let out = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("map.ppm"));
            let px = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1024usize);
            let num = |i: usize, d: f32| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
            let view = (glam::Vec2::new(num(4, 0.0), num(5, 0.0)), num(6, 0.0));
            let data = match GameData::load(&PathBuf::from("data"), "mechs/placeholder.ron") {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("FAIL {e}");
                    return ExitCode::FAILURE;
                }
            };
            let plan = cb_core::world::generate(seed, &data.world);
            match std::fs::write(&out, map::draw(&plan, &data.world, px, view)) {
                Ok(()) => {
                    println!("wrote {}", out.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("FAIL {}: {e}", out.display());
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "usage: cb-tools validate [data dir] | cb-tools cities [seed] | cb-tools map [seed] [out.ppm] [pixels] [centre x] [centre z] [span m] | cb-tools traffic-bench [seed] [steps] | cb-tools aircraft [data dir]"
            );
            ExitCode::FAILURE
        }
    }
}
