//! Crab Breeze command-line tools. `cb-tools validate <data dir>` loads every data file through the
//! same loader the game uses (CLAUDE.md section 11), so a file that would stop the game stops here.

mod audio_check;
mod glb;

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
                        // the model it names, read as data: the rig contract, and the sizes it was generated from
                        let problems = glb::check_mech(&d.mech, &root.join("..").join("assets"));
                        if problems.is_empty() {
                            println!("ok   {}/{mech} (with sim, world, pilot, bindings, model)", root.display());
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
        _ => {
            eprintln!("usage: cb-tools validate [data dir] | cb-tools cities [seed]");
            ExitCode::FAILURE
        }
    }
}
