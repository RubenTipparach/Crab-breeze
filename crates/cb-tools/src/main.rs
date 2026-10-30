//! Crab Breeze command-line tools. `cb-tools validate <data dir>` loads every data file through the
//! same loader the game uses (CLAUDE.md section 11), so a file that would stop the game stops here.
//! `cities` and `map` are instruments: a seed's cities, and its island drawn from above.

mod audio_check;
mod glb;
mod map;
mod ships;
mod traffic_bench;

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
            // the modelled civil aircraft against their data and their manifests (civil-air-traffic, survey D8)
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_aircraft(&d.air_traffic, &root.join("..").join("assets"));
                if problems.is_empty() {
                    let n = d.air_traffic.kinds.values().filter(|k| k.model.is_some()).count();
                    println!("ok   {n} modelled civil aircraft and their levels of detail");
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
            // the boats' block-outs against their data (harbours)
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_boats(&d.ships, &root.join("..").join("assets"));
                if problems.is_empty() {
                    println!("ok   {} boat kinds, the crane and the quay", d.ships.kinds.len());
                }
                for p in problems {
                    eprintln!("FAIL {p}");
                    ok = false;
                }
            }
            // the Stratostacean's model against its shape data (crab-airship)
            if let Ok(d) = GameData::load(&root, &mechs[0]) {
                let problems = glb::check_fortress(&d.fortress, &root.join("..").join("assets"));
                if problems.is_empty() {
                    println!("ok   the {} ({} parts)", d.fortress.cfg.name, d.fortress.shape.parts.len());
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
        // A measurement instrument (openspec changes `airport-layout`, `seaport-layout`): where the sites stand on a
        // seed, and when no airport fits, how many candidates each rule turned down.
        Some("sites") => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
            let data = match GameData::load(&PathBuf::from("data"), "mechs/placeholder.ron") {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("FAIL {e}");
                    return ExitCode::FAILURE;
                }
            };
            let plan = cb_core::world::generate(seed, &data.world);
            for s in &plan.sites {
                let f = s.footprint();
                println!(
                    "{:?}: origin ({:.0}, {:.0}), turn {}, ground {:.1} m, city {}, footprint {:.0} x {:.0} m, {} lots",
                    s.template.kind,
                    s.origin.x,
                    s.origin.y,
                    s.turn,
                    s.ground_m,
                    s.city,
                    f.size().x,
                    f.size().y,
                    s.template.placements.len()
                );
            }
            // The air bases, their sites and runways (for the island-sites-layout mockup).
            for b in &plan.bases {
                println!(
                    "AirBase {}: centre ({:.0}, {:.0}), half ({:.0}, {:.0}), runway ({:.0}, {:.0}) to ({:.0}, {:.0}), {:.0} m wide",
                    b.id,
                    b.centre.x,
                    b.centre.y,
                    b.half_m.x,
                    b.half_m.y,
                    b.runway.a.x,
                    b.runway.a.y,
                    b.runway.b.x,
                    b.runway.b.y,
                    b.runway.width_m
                );
            }
            // How far each stands from the free roam start, and which way (0 degrees north, +z; 90 east, +x).
            let start = plan.start.mech_xz;
            println!("start: ({:.0}, {:.0})", start.x, start.y);
            for s in &plan.sites {
                let d = s.footprint().centre() - start;
                let bearing = d.x.atan2(d.y).to_degrees().rem_euclid(360.0);
                println!(
                    "{:?}: {:.1} km from the start, bearing {:.0} degrees",
                    s.template.kind,
                    d.length() / 1000.0,
                    bearing
                );
            }
            for (i, c) in plan.cities.iter().enumerate() {
                let d = c.centre - start;
                println!(
                    "city {i}: centre ({:.0}, {:.0}), {:.1} km from the start",
                    c.centre.x,
                    c.centre.y,
                    d.length() / 1000.0
                );
            }
            let [c, b, w, st, wa, r] = plan.airport_rejections;
            println!("airport candidates turned down: cities {c}, bases {b}, off dry land {w}, steep {st}, water {wa}, highways {r}");
            ExitCode::SUCCESS
        }
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
            let lamps = cb_core::world::lamps::place(&plan, &data.day_night.lamps);
            let count = |k| lamps.iter().filter(|l| l.kind == k).count();
            use cb_core::world::lamps::LampKind;
            println!(
                "street lamps: {} ({} on streets, {} on arterials, {} on highways)",
                lamps.len(),
                count(LampKind::Street),
                count(LampKind::Arterial),
                count(LampKind::Highway)
            );
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
        // A measurement instrument (openspec change `car-traffic` design sections 5 and 6): `traffic_bench`.
        Some("traffic-bench") => traffic_bench::run(&args),
        // A documentation instrument (openspec change `civil-air-traffic`): every civil aircraft kind's
        // size and block-out boxes as JSON, from the one table the game draws
        // (`cb_core::air_traffic::shape`), and the model a modelled kind is drawn from instead (survey D8),
        // for the mockup page `tools/mockups/gen_civil_aircraft.py`.
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
                        // the modelled kinds (survey D8): the glb the game draws, its levels and the air drop's sling
                        "model": k.model.as_ref().map(|m| serde_json::json!({
                            "scene": m.scene,
                            "lod_m": m.lod_m,
                            "gear_below_m": m.gear_below_m,
                            "sling": m.sling,
                            "sling_hook_m": m.sling_hook_m,
                        })),
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
        // A measurement instrument (openspec change `harbours`, design section 11): a seed's harbours, lanes and
        // timetable, how long they take to lay out, and what the boats cost a fixed step round each harbour.
        Some("ships") => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
            match ships::report(seed) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("FAIL {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "usage: cb-tools validate [data dir] | cb-tools cities [seed] | cb-tools map [seed] [out.ppm] [pixels] [centre x] [centre z] [span m] | cb-tools traffic-bench [seed] [steps] | cb-tools aircraft [data dir] | cb-tools ships [seed]"
            );
            ExitCode::FAILURE
        }
    }
}
