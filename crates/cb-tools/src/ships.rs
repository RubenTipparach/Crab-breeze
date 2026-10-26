//! `cb-tools ships [seed]`: a measurement instrument for the boats (openspec change `harbours`, design section 11).
//! Lays out a seed's harbours, sea lanes and timetable from the shipped data, says what it built and how long that
//! took, then steps the boats for two minutes of fixed steps with the pilot standing at each harbour in turn and
//! reports the cost of a step (the median, p95 and worst, in microseconds) and how many boats were real.

use cb_core::GameData;
use cb_core::ships::{Sea, Traffic};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// Prints the report for `seed`.
pub fn report(seed: u64) -> Result<(), String> {
    let data = GameData::load(&PathBuf::from("data"), "mechs/placeholder.ron").map_err(|e| e.to_string())?;
    let started = Instant::now();
    let plan = cb_core::world::generate(seed, &data.world);
    println!("seed {seed}: world generated in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
    let road_m = data.world.highway_width_m * 0.5 + data.world.highway_flat_margin_m;
    let started = Instant::now();
    let mut t = Traffic::from_plan(&plan, &data.ships, road_m, data.world.world_size_m, seed);
    println!(
        "harbours, lanes and timetable laid out in {:.0} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    for h in &t.harbours {
        println!(
            "harbour {}: city {}, berth ({:.0}, {:.0}), pier {:.0} m, deck {:.1} m, {} cranes, {} warehouses",
            h.id,
            h.city,
            h.berth.x,
            h.berth.y,
            h.berth.distance(h.shore),
            h.deck_y,
            h.cranes.len(),
            h.warehouses.len()
        );
    }
    for (i, r) in t.routes.iter().enumerate() {
        let what = if i < t.lanes { "lane" } else { "route" };
        println!(
            "{what} {i}: {:?} to {:?}, {:.1} km, {} points",
            r.ends[0],
            r.ends[1],
            r.length_m() / 1000.0,
            r.points.len()
        );
    }
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for s in &t.slots {
        *kinds
            .entry(format!("{}{}", s.kind, if s.lake { " (lake)" } else { "" }))
            .or_default() += 1;
    }
    println!("timetable: {} boats: {kinds:?}", t.slots.len());
    match t.islet {
        Some((c, r)) => println!("ferry to the islet at ({:.0}, {:.0}), {r:.0} m in radius", c.x, c.y),
        None => println!("no ferry: no islet with a landing, or no harbour"),
    }
    let sea = Sea {
        ground: &plan,
        sea_level_m: plan.waters.sea_level_m,
    };
    let dt = 1.0 / data.sim.fixed_step_hz;
    let spots: Vec<(String, glam::Vec2)> = t
        .harbours
        .iter()
        .map(|h| (format!("harbour {}", h.id), h.berth))
        .chain(plan.waters.lakes.list.first().map(|l| ("lake 0".to_string(), l.centre)))
        .collect();
    for (name, at) in spots {
        // Settle into the spot, then time two minutes of steps.
        for _ in 0..60 {
            t.step(&data.ships, &sea, at, dt);
        }
        let mut us: Vec<f64> = Vec::new();
        let mut most = 0;
        for _ in 0..(120.0 / dt) as usize {
            let s = Instant::now();
            t.step(&data.ships, &sea, at, dt);
            us.push(s.elapsed().as_secs_f64() * 1e6);
            most = most.max(t.boats.len());
        }
        us.sort_by(f64::total_cmp);
        let q = |p: f64| us[((us.len() - 1) as f64 * p) as usize];
        println!(
            "{name}: {most} boats real; a step {:.1} us median, {:.1} us p95, {:.1} us p99, {:.1} us worst",
            q(0.5),
            q(0.95),
            q(0.99),
            us.last().copied().unwrap_or(0.0)
        );
    }
    Ok(())
}
