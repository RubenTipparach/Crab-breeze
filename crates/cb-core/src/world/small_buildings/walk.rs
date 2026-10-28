//! Walking into a house or small business (openspec change `small-buildings`, design section 7.2;
//! owner, 2026-10-26: "make sure I can walk into the interiors"). A building's colliders are the city's
//! (`interior::Walkable`): its pieces' boxes, in its own frame, with a `door::StreetDoor` for each of
//! its leaves, so its doors swing open as the pilot reaches them and shut behind them. A body meets
//! them in the building's frame: moved into it, pushed out, moved back. Built only for a pilot on foot
//! (`interior::Interiors::houses`).

use super::kit::{Kit, Node};
use super::lots::Lot;
use crate::world::door::{Body, StreetDoor};
use crate::world::dressing::Anchor;
use crate::world::interior::{InteriorConfig, Solid, Walkable};
use glam::{Vec2, Vec3};

/// A storey's height, for bucketing the colliders.
const STOREY_M: f32 = 2.75;

/// Surfaces that never stop a body (ceiling lamps).
const PASSABLE: [&str; 1] = ["lamp"];

/// The colliders of `kit` (lot id `lot`), its standing pieces only (`alive`: `None` for all), in its own
/// frame: every piece's boxes but the leaves' and the lamps', and a door for each standing leaf.
pub fn walkable(kit: &Kit, lot: u32, alive: Option<&[bool]>, cfg: &InteriorConfig) -> Walkable {
    let stands = |i: usize| alive.is_none_or(|a| a.get(i).copied().unwrap_or(false));
    let mut solids = Vec::new();
    for (i, p) in kit.pieces.iter().enumerate() {
        if !stands(i) || matches!(p.node, Node::Leaf(_)) || p.surfaces.iter().all(|s| PASSABLE.contains(&s.as_str())) {
            continue;
        }
        for (min, max) in p.solid_boxes() {
            if (max - min).min_element() > 1e-4 {
                solids.push(Solid {
                    min,
                    max,
                    hold: Anchor::Whole,
                    framed_only: false,
                });
            }
        }
    }
    let doors = kit
        .leaves
        .iter()
        .filter(|l| stands(l.piece as usize))
        .map(|l| {
            StreetDoor::hung(
                Vec2::new(l.hinge.x, l.hinge.z),
                l.shut,
                l.width_m,
                l.inward,
                l.hinge.y,
                l.hinge.y + l.height_m,
                Anchor::Whole,
                cfg.door_open_deg,
            )
        })
        .collect();
    let storeys = ((kit.hi.y - kit.lo.y) / STOREY_M).ceil().max(1.0) as usize;
    Walkable::from_solids(lot, solids, doors, kit.lo.y, STOREY_M, storeys)
}

/// A body at world `p` (x, z) with feet at `feet_y`, moved out of `w` on `lot`.
pub fn push_out(w: &Walkable, lot: &Lot, p: Vec2, radius_m: f32, feet_y: f32, cfg: &InteriorConfig) -> Vec2 {
    let local = lot.to_local(p);
    let moved = w.push_out(local, radius_m, feet_y - lot.ground_m, cfg, None);
    if moved == local { p } else { lot.to_world(moved) }
}

/// The highest top of `w` on `lot` under world `p` within a step of feet at `feet_y`, world height.
pub fn support(w: &Walkable, lot: &Lot, p: Vec2, feet_y: f32, cfg: &InteriorConfig) -> Option<f32> {
    w.support(lot.to_local(p), feet_y - lot.ground_m, cfg, None)
        .map(|y| y + lot.ground_m)
}

/// The pilot's body in `lot`'s frame, for its doors.
pub fn body_in(lot: &Lot, b: &Body) -> Body {
    Body {
        at: lot.to_local(b.at),
        feet_y: b.feet_y - lot.ground_m,
        radius_m: b.radius_m,
    }
}

/// A leaf's hinge in the world, and its free edge's direction at `angle_deg` from shut (x, z).
pub fn leaf_in_world(d: &StreetDoor, lot: &Lot, angle_deg: f32) -> (Vec3, Vec2) {
    let h = d.leaves[0].hinge;
    let at = lot.point_to_world(Vec3::new(h.x, d.y0, h.y));
    let dir = d.leaf_dir(0, angle_deg);
    let (s, c) = lot.heading_rad.sin_cos();
    (at, Vec2::new(dir.x * c + dir.y * s, -dir.x * s + dir.y * c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::door::DoorPhase;
    use crate::world::layout::Rect;
    use crate::world::small_buildings::kit::shipped;
    use crate::world::small_buildings::lots::{Zone, heading_facing};

    fn lot_of(kit: u16, heading: f32) -> Lot {
        Lot {
            kit,
            centre: Vec2::new(40.0, -20.0),
            ground_m: 5.0,
            heading_rad: heading,
            rect: Rect::centred(Vec2::new(40.0, -20.0), Vec2::splat(20.0)),
            zone: Zone::Suburb,
        }
    }

    /// Walks a pilot from `from` towards `to` (world x, z) for `secs`, stepping the doors and the
    /// colliders as the game does; returns where they end and their feet's height.
    fn walk(kit: &Kit, lot: &Lot, from: Vec2, to: Vec2, secs: f32) -> (Vec2, f32) {
        let cfg = InteriorConfig::default();
        let mut w = walkable(kit, 0, None, &cfg);
        let (r, dt, speed) = (0.3, 1.0 / 60.0, 1.4);
        let mut p = from;
        let mut feet = lot.ground_m;
        for _ in 0..(secs / dt) as usize {
            let body = Body {
                at: p,
                feet_y: feet,
                radius_m: r,
            };
            for d in &mut w.doors {
                d.step(Some(body_in(lot, &body)), &cfg, dt);
            }
            let dir = (to - p).normalize_or_zero();
            let step = dir * speed * dt;
            let next = if (to - p).length() < step.length() { to } else { p + step };
            p = push_out(&w, lot, next, r, feet, &cfg);
            feet = support(&w, lot, p, feet, &cfg).unwrap_or(lot.ground_m).max(lot.ground_m);
        }
        (p, feet)
    }

    #[test]
    fn a_pilot_walks_through_the_front_door_into_the_living_room() {
        let kits = shipped();
        for (id, heading) in [
            ("ranch", 0.0),
            ("ranch", heading_facing(Vec2::new(0.6, -0.8))),
            ("bungalow_c", 2.0),
            ("corner", 0.4),
        ] {
            let k = kits.index_of(id).unwrap();
            let kit = &kits.kits[k];
            let lot = lot_of(k as u16, heading);
            let leaf = &kit.leaves[0];
            let mid = Vec2::new(leaf.hinge.x, leaf.hinge.z) + leaf.shut * (leaf.width_m * 0.5);
            let outside = lot.to_world(mid - leaf.inward * 3.0);
            let inside = lot.to_world(mid + leaf.inward * 2.5);
            let (end, feet) = walk(kit, &lot, outside, inside, 12.0);
            assert!(
                end.distance(inside) < 0.3,
                "{id}: stopped at {} short of {}",
                end.distance(inside),
                inside
            );
            assert!(feet > lot.ground_m + 0.1, "{id}: standing on the floor inside, feet {feet}");
        }
    }

    #[test]
    fn a_shut_door_is_a_wall_and_walls_hold() {
        let kits = shipped();
        let k = kits.index_of("ranch").unwrap();
        let kit = &kits.kits[k];
        let lot = lot_of(k as u16, 0.3);
        let cfg = InteriorConfig::default();
        let w = walkable(kit, 0, None, &cfg);
        assert!(w.doors.iter().all(|d| d.phase == DoorPhase::Closed));
        let leaf = &kit.leaves[0];
        let mid = Vec2::new(leaf.hinge.x, leaf.hinge.z) + leaf.shut * (leaf.width_m * 0.5);
        // Standing in the shut doorway: pushed out of the leaf.
        let p = lot.to_world(mid);
        let out = push_out(&w, &lot, p, 0.3, lot.ground_m + 0.45, &cfg);
        assert!(out.distance(p) > 0.2, "the shut leaf pushes");
        // Walking into a blank stretch of the front wall: stopped outside it.
        let wall_from = lot.to_world(Vec2::new(kit.lo.x + 1.0, 8.0));
        let wall_to = lot.to_world(Vec2::new(kit.lo.x + 1.0, 0.0));
        let (end, _) = walk(kit, &lot, wall_from, wall_to, 8.0);
        assert!(lot.to_local(end).y > 4.0, "held outside the wall, at {}", lot.to_local(end));
    }

    #[test]
    fn the_colonials_stairs_reach_the_upper_floor() {
        let kits = shipped();
        let k = kits.index_of("colonial").unwrap();
        let kit = &kits.kits[k];
        let cfg = InteriorConfig::default();
        let w = walkable(kit, 0, None, &cfg);
        let lot = lot_of(k as u16, 0.0);
        // Climb from the foot of the stair, tread by tread: the support rises to the upper floor.
        let lowest = kit
            .pieces
            .iter()
            .filter(|p| p.surfaces == ["wood"] && p.lo.y > 0.4 && p.lo.y < 0.5 && p.hi.y - p.lo.y < 0.3)
            .map(|p| (p.lo + p.hi) * 0.5)
            .next();
        let Some(start) = lowest else { panic!("the colonial has a stair") };
        let mut feet = 0.45;
        let mut z = start.z;
        for _ in 0..40 {
            let s = w.support(Vec2::new(start.x, z), feet, &cfg, None).unwrap_or(feet);
            feet = feet.max(s);
            z += 0.12;
        }
        assert!(feet > 3.0, "reached {feet} m, the upper floor is at 3.2 m");
        let _ = lot;
    }
}
