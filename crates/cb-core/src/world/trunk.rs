//! The highway through a city and its exits (openspec change `city-districts`, design section 5). Where two of a city's
//! highways come in from roughly opposite sides, they are joined by a trunk: a viaduct along the middle of a row of
//! blocks south (or east) of the downtown, from just outside the city's square to just outside the other side, its
//! row's blocks car parks between its piers (`BlockUse::Under`). At an arterial that crosses the row, an exit: four
//! one-way ramps, off and on each way, between the viaduct and the frontage street beside the row, landing at the
//! arterial's junction with it (a diamond with frontage roads, the shape the grid allows). Pure and deterministic.

use super::layout::{BlockUse, City, Downtown, Highway, Rect};
use crate::data::{Validate, positive, within};
use glam::{Vec2, Vec3};
use serde::Deserialize;

/// The trunks' and their exits' tuning: `trunk:` in `data/world/world.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TrunkConfig {
    /// Trunks are built (off: every highway stops at its city's edge, as before).
    pub enabled: bool,
    /// The viaduct's grade over the city's ground, metres.
    pub deck_m: f32,
    /// The trunk's row is the one nearest this share of the city's half size off its centre (south or east).
    pub row_offset_share: f32,
    /// The trunk runs on this far past each side of the square before the highways take over, metres.
    pub outside_m: f32,
    /// The two highways' ends stay level at the trunk's height for this long, metres.
    pub end_level_m: f32,
    /// Each highway runs on straight along the trunk's line for this long past its end before it may turn, so it
    /// meets the trunk in line (no corner in the road, no wedge of road left off the decks), metres.
    pub lead_in_m: f32,
    /// Two highways make a trunk when they leave the city at least this far apart, degrees.
    pub min_apart_deg: f32,
    /// A ramp's paved width, metres.
    pub ramp_width_m: f32,
    /// Its steepest grade (rise over run).
    pub ramp_grade: f32,
    /// Between the viaduct's edge and a ramp's, metres.
    pub ramp_gap_m: f32,
    /// A ramp swings out from the viaduct's middle until its edge meets the viaduct's edge over this length, metres.
    pub ramp_merge_m: f32,
    /// Then it runs on level beside the viaduct, its edge against the viaduct's, for this long before it climbs or
    /// descends, so the two decks meet at one height and cars cross from one to the other, metres.
    pub ramp_gore_m: f32,
    /// A ramp's level end, from where it reaches the street to the junction's corner, metres.
    pub ramp_landing_m: f32,
    /// The blocks either side of an arterial are drawn this long, least and most, so a ramp fits, metres.
    pub arterial_block_m: [f32; 2],
}

impl Default for TrunkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deck_m: 8.5,
            row_offset_share: 0.35,
            outside_m: 40.0,
            end_level_m: 16.0,
            lead_in_m: 120.0,
            min_apart_deg: 110.0,
            ramp_width_m: 9.0,
            ramp_grade: 0.08,
            ramp_gap_m: 0.5,
            ramp_merge_m: 40.0,
            ramp_gore_m: 30.0,
            ramp_landing_m: 25.0,
            arterial_block_m: [104.0, 112.0],
        }
    }
}

impl Validate for TrunkConfig {
    fn validate(&self) -> Result<(), String> {
        // room for 4.5 m of headroom under a 1.2 m slab
        within("trunk.deck_m", self.deck_m, 5.9, 20.0)?;
        within("trunk.row_offset_share", self.row_offset_share, 0.0, 1.0)?;
        positive("trunk.outside_m", self.outside_m)?;
        positive("trunk.end_level_m", self.end_level_m)?;
        positive("trunk.ramp_gore_m", self.ramp_gore_m)?;
        positive("trunk.lead_in_m", self.lead_in_m)?;
        within("trunk.min_apart_deg", self.min_apart_deg, 0.0, 180.0)?;
        positive("trunk.ramp_width_m", self.ramp_width_m)?;
        within("trunk.ramp_grade", self.ramp_grade, 0.01, 0.12)?;
        within("trunk.ramp_gap_m", self.ramp_gap_m, 0.0, 5.0)?;
        positive("trunk.ramp_merge_m", self.ramp_merge_m)?;
        positive("trunk.ramp_landing_m", self.ramp_landing_m)?;
        within("trunk.arterial_block_m[0]", self.arterial_block_m[0], 40.0, 300.0)?;
        within(
            "trunk.arterial_block_m[1]",
            self.arterial_block_m[1],
            self.arterial_block_m[0],
            300.0,
        )
    }
}

/// A city's trunk, chosen before it is laid: the two highways it joins (by index, and which end of each), its line and
/// its row.
#[derive(Clone, Debug, PartialEq)]
pub struct TrunkPlan {
    /// The city.
    pub city: u32,
    /// The two highways and which of their ends is at this city: `(highway, end)`; the first comes in at `ends[0]`.
    pub joins: [(usize, usize); 2],
    /// The trunk's ends, outside the square: `ends[0]` toward the first highway.
    pub ends: [Vec2; 2],
    /// The row of blocks it runs over (the blocks whose middles lie inside it).
    pub row: Rect,
    /// Its grade.
    pub top_m: f32,
}

/// One one-way ramp of an exit.
#[derive(Clone, Debug, PartialEq)]
pub struct Ramp {
    /// The city.
    pub city: u32,
    /// The highway it leaves or joins (the trunk's index in `WorldPlan::highways`).
    pub highway: u32,
    /// True: it leaves the highway for the street; false: it joins the highway from the street.
    pub off: bool,
    /// Where along the highway (0..1) it leaves or joins it.
    pub tap_t: f32,
    /// The street junction it lands at (x, z): an arterial's crossing with the frontage street.
    pub junction: Vec2,
    /// Its centre line in the direction of travel, (x, road top, z) at each point, from the highway to the junction or
    /// the other way.
    pub line: Vec<Vec3>,
    /// The part of `line` that stands clear of the viaduct (the deck that is drawn as the ramp's own).
    pub deck: Vec<Vec3>,
    /// Paved width, metres.
    pub width_m: f32,
}

/// Chooses each city's trunk, if it has one: two of its highways leaving it at least `min_apart_deg` apart (the
/// widest pair), the trunk along the street axis nearer their line, over the row of blocks nearest `row_offset_share`
/// of the half size off the centre (south for a trunk along x, east along z) that keeps clear of the `downtown`'s radius (whatever its
/// shape, so the highways it re-lays do not move with the shape; a downtown block the row takes is a car park under the
/// viaduct, `zone_rows`). `None` with fewer than two such highways.
pub fn choose(city: &City, highways: &[Highway], downtown: &Downtown, cfg: &TrunkConfig) -> Option<TrunkPlan> {
    if !cfg.enabled {
        return None;
    }
    let c = city.centre;
    let mine: Vec<(usize, usize, Vec2)> = highways
        .iter()
        .enumerate()
        .flat_map(|(k, h)| {
            (0..2)
                .filter(move |&e| h.cities[e] == city.id && h.cities[0] != h.cities[1])
                .map(move |e| (k, e, h.ends[e]))
        })
        .collect();
    let mut best: Option<(f32, usize, usize)> = None;
    for i in 0..mine.len() {
        for j in i + 1..mine.len() {
            let (a, b) = ((mine[i].2 - c).normalize_or(Vec2::X), (mine[j].2 - c).normalize_or(Vec2::NEG_X));
            let apart = a.dot(b).clamp(-1.0, 1.0).acos().to_degrees();
            if apart >= cfg.min_apart_deg && best.is_none_or(|(ba, _, _)| apart > ba) {
                best = Some((apart, i, j));
            }
        }
    }
    let (_, i, j) = best?;
    let (ha, hb) = (mine[i], mine[j]);
    let line = hb.2 - ha.2;
    let along_x = line.x.abs() >= line.y.abs();
    let (xs, zs) = super::lanes::street_axes(city);
    // the street lines across the trunk's axis bound its rows: lines at z for a trunk along x
    let lines = if along_x { &zs } else { &xs };
    let across = |p: Vec2| if along_x { p.y } else { p.x };
    let centre = across(c);
    let downtown_m = downtown.radius_m;
    let want = centre + city.half_size_m * cfg.row_offset_share;
    let row = lines
        .windows(2)
        .map(|w| (w[0].0 + w[0].1 * 0.5, w[1].0 - w[1].1 * 0.5))
        .filter(|&(lo, _)| lo - centre > downtown_m)
        .min_by(|a, b| ((a.0 + a.1) * 0.5 - want).abs().total_cmp(&((b.0 + b.1) * 0.5 - want).abs()))?;
    let mid = (row.0 + row.1) * 0.5;
    let r = city.rect();
    let (e0, e1, row_rect) = if along_x {
        (
            Vec2::new(r.min.x - cfg.outside_m, mid),
            Vec2::new(r.max.x + cfg.outside_m, mid),
            Rect {
                min: Vec2::new(r.min.x, row.0),
                max: Vec2::new(r.max.x, row.1),
            },
        )
    } else {
        (
            Vec2::new(mid, r.min.y - cfg.outside_m),
            Vec2::new(mid, r.max.y + cfg.outside_m),
            Rect {
                min: Vec2::new(row.0, r.min.y),
                max: Vec2::new(row.1, r.max.y),
            },
        )
    };
    // the first highway takes the nearer end
    let (first, second) = if ha.2.distance(e0) + hb.2.distance(e1) <= ha.2.distance(e1) + hb.2.distance(e0) {
        (ha, hb)
    } else {
        (hb, ha)
    };
    Some(TrunkPlan {
        city: city.id,
        joins: [(first.0, first.1), (second.0, second.1)],
        ends: [e0, e1],
        row: row_rect,
        top_m: city.ground_m + cfg.deck_m,
    })
}

/// The trunk itself, as a highway of its own (both ends the city): straight from one end to the other, a point every
/// 16 m, level at its grade.
pub fn trunk_highway(plan: &TrunkPlan, width_m: f32, world_size_m: f32) -> Highway {
    let len = plan.ends[0].distance(plan.ends[1]);
    let n = (len / 16.0).ceil().max(1.0) as usize;
    let path: Vec<Vec2> = (0..=n).map(|i| plan.ends[0].lerp(plan.ends[1], i as f32 / n as f32)).collect();
    let grade = vec![plan.top_m; path.len()];
    Highway::new([plan.city, plan.city], path, grade, width_m, 0.0, world_size_m)
}

/// The exits of trunk `highway` (its index), laid out by `plan` over `city`'s grid: at every arterial crossing the
/// row whose blocks either side give a ramp room (its part lower than `low_m` inside its block, where nothing passes
/// under it), four one-way ramps, the way traffic keeps right; an exit whose ramps would meet another's is left out.
/// Ramps are smoothed into curves; their tops run from the trunk's grade plus `road_lift_m` to the street's.
pub fn exits(
    plan: &TrunkPlan,
    highway: u32,
    trunk: &Highway,
    city: &City,
    road_lift_m: f32,
    low_m: f32,
    arterial_width_m: f32,
    cfg: &TrunkConfig,
) -> Vec<Ramp> {
    let (a, b) = (plan.ends[0], plan.ends[1]);
    let len = a.distance(b);
    if len <= 1.0 {
        return Vec::new();
    }
    let u_dir = (b - a) / len;
    let right = Vec2::new(-u_dir.y, u_dir.x);
    let along_x = u_dir.x.abs() >= u_dir.y.abs();
    let mid_v = right.dot(plan.row.centre());
    let at = |u: f32, v: f32| a + u_dir * u + right * (v - right.dot(a));
    let (xs, zs) = super::lanes::street_axes(city);
    // lines across the trunk (each crosses it at u), and the frontage lines beside its row (at v)
    let (crossing, beside) = if along_x { (&xs, &zs) } else { (&zs, &xs) };
    let u_of = |c: f32| {
        let p = if along_x { Vec2::new(c, a.y) } else { Vec2::new(a.x, c) };
        (p - a).dot(u_dir)
    };
    let v_of = |c: f32| {
        let p = if along_x { Vec2::new(a.x, c) } else { Vec2::new(c, a.y) };
        right.dot(p)
    };
    let row_lo = v_of(if along_x { plan.row.min.y } else { plan.row.min.x });
    let row_hi = v_of(if along_x { plan.row.max.y } else { plan.row.max.x });
    let (row_lo, row_hi) = (row_lo.min(row_hi), row_lo.max(row_hi));
    // the frontage streets: the beside-lines nearest each side of the row
    let front = |side: f32| {
        beside
            .iter()
            .map(|&(c, w)| (v_of(c), w))
            .filter(|&(v, _)| if side > 0.0 { v >= row_hi - 1.0 } else { v <= row_lo + 1.0 })
            .min_by(|p, q| (p.0 - mid_v).abs().total_cmp(&(q.0 - mid_v).abs()))
    };
    let (Some((v_pos, w_pos)), Some((v_neg, w_neg))) = (front(1.0), front(-1.0)) else {
        return Vec::new();
    };
    let deck_top = plan.top_m + road_lift_m;
    let street_top = city.ground_m + road_lift_m;
    let rise = deck_top - street_top;
    let desc = rise / cfg.ramp_grade;
    let low = (low_m - road_lift_m).max(0.0) / cfg.ramp_grade;
    let off_v = trunk.width_m * 0.5 + cfg.ramp_gap_m + cfg.ramp_width_m * 0.5;
    // beside the viaduct with its edge against the viaduct's, where the two decks meet level
    let flush_v = trunk.width_m * 0.5 + cfg.ramp_width_m * 0.5;
    let mut lines: Vec<(f32, f32)> = crossing.iter().map(|&(c, w)| (u_of(c), w)).collect();
    lines.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut used: [Vec<(f32, f32)>; 2] = [Vec::new(), Vec::new()];
    let mut out = Vec::new();
    for k in 1..lines.len().saturating_sub(1) {
        let (ua, wa) = lines[k];
        if wa < arterial_width_m - 1e-3 {
            continue;
        }
        let before = (lines[k - 1].0 + lines[k - 1].1 * 0.5, ua - wa * 0.5);
        let after = (ua + wa * 0.5, lines[k + 1].0 - lines[k + 1].1 * 0.5);
        let room = low + cfg.ramp_landing_m + 2.0;
        if before.1 - before.0 < room || after.1 - after.0 < room {
            continue;
        }
        // the four ramps' extents along the trunk, (side, from, to)
        let reach = desc + cfg.ramp_merge_m + cfg.ramp_gore_m + cfg.ramp_landing_m;
        let spans = [
            (0usize, before.1 - reach, before.1),
            (0, after.0, after.0 + reach),
            (1, after.0, after.0 + reach),
            (1, before.1 - reach, before.1),
        ];
        // every junction on the trunk keeps clear of its ends (the highway between two junctions needs room for its
        // lanes past both: `lanes::LaneGraph::lay_lanes`)
        let margin = trunk.width_m * 1.5 + 10.0;
        let fits = spans
            .iter()
            .all(|&(side, u0, u1)| u0 > margin && u1 < len - margin && used[side].iter().all(|&(p, q)| u1 < p || u0 > q));
        if !fits {
            continue;
        }
        for &(side, u0, u1) in &spans {
            used[side].push((u0, u1));
        }
        // +u traffic keeps to +v (right), -u to -v
        for (side_sign, v_front, w_front) in [(1.0f32, v_pos, w_pos), (-1.0, v_neg, w_neg)] {
            let v_ramp = mid_v + side_sign * off_v;
            // just inside the block's corner, before the frontage street
            let corner_v = v_front - side_sign * (w_front * 0.5 + 2.0);
            let junction = at(ua, v_front);
            // the ramp leaving +u traffic is in the block before the arterial; -u traffic's, the block after
            let (off_block, on_block) = if side_sign > 0.0 { (before, after) } else { (after, before) };
            let dir = side_sign; // travel along u for this side
            // off-ramp: from the trunk, down, to the junction
            let g_u = if dir > 0.0 {
                off_block.1 - cfg.ramp_landing_m
            } else {
                off_block.0 + cfg.ramp_landing_m
            };
            let top_u = g_u - dir * desc;
            let flush_u = top_u - dir * cfg.ramp_gore_m;
            let tap_u = flush_u - dir * cfg.ramp_merge_m;
            let corner_u = ua - dir * (wa * 0.5 + 2.0);
            let flush = mid_v + side_sign * flush_v;
            let off_pts = vec![
                (tap_u, mid_v),
                (flush_u, flush),
                (top_u, flush),
                (g_u, v_ramp),
                (corner_u, corner_v),
                (ua, v_front),
            ];
            // level at the deck until it is past the gore, then down at the ramp's grade (by distance along the trunk,
            // never steeper along the ramp itself), level at the street
            let off_y = move |u: f32| deck_top - rise * (dir * (u - top_u) / desc).clamp(0.0, 1.0);
            // on-ramp: from the junction, up, onto the trunk past the arterial
            let g2_u = if dir > 0.0 {
                on_block.0 + cfg.ramp_landing_m
            } else {
                on_block.1 - cfg.ramp_landing_m
            };
            let top2_u = g2_u + dir * desc;
            let flush2_u = top2_u + dir * cfg.ramp_gore_m;
            let tap2_u = flush2_u + dir * cfg.ramp_merge_m;
            let corner2_u = ua + dir * (wa * 0.5 + 2.0);
            let on_pts = vec![
                (ua, v_front),
                (corner2_u, corner_v),
                (g2_u, v_ramp),
                (top2_u, flush),
                (flush2_u, flush),
                (tap2_u, mid_v),
            ];
            let on_y = move |u: f32| street_top + rise * (dir * (u - g2_u) / desc).clamp(0.0, 1.0);
            let ys: [&dyn Fn(f32) -> f32; 2] = [&off_y, &on_y];
            for ((off, pts, tap_u), y_of) in [(true, off_pts, tap_u), (false, on_pts, tap2_u)].into_iter().zip(ys) {
                // laid out in plan from the viaduct's side (an on-ramp is laid as an off-ramp and turned round): an
                // eased swing out to the viaduct's edge, straight beside it, an eased swing away, then rounded into the
                // street; the heights follow the profile, so it stays level with the deck until it is beside it
                let seq: Vec<(f32, f32)> = if off { pts.clone() } else { pts.iter().rev().copied().collect() };
                let mut plan_line = ramp_plan(&seq.iter().map(|&(u, v)| at(u, v)).collect::<Vec<_>>());
                if !off {
                    plan_line.reverse();
                }
                let line: Vec<Vec3> = plan_line
                    .into_iter()
                    .map(|p| Vec3::new(p.x, y_of((p - a).dot(u_dir)), p.y))
                    .collect();
                // the deck drawn as the ramp's own: from where it stands clear of the viaduct
                let clear = trunk.width_m * 0.5 + cfg.ramp_width_m * 0.5 - 0.01;
                let deck: Vec<Vec3> = line
                    .iter()
                    .copied()
                    .filter(|p| (right.dot(Vec2::new(p.x, p.z)) - mid_v).abs() >= clear)
                    .collect();
                out.push(Ramp {
                    city: city.id,
                    highway,
                    off,
                    tap_t: (tap_u / len).clamp(0.0, 1.0),
                    junction,
                    line,
                    deck,
                    width_m: cfg.ramp_width_m,
                });
            }
        }
    }
    out
}

/// A ramp's line in plan, from the viaduct's side, through its six points (`seq`: the tap on the viaduct's middle, where
/// its edge meets the viaduct's, the end of the gore beside it, its foot, the block's corner and the junction): an eased
/// swing from the tap out to the viaduct's edge and from the gore's end away to its foot (no corner at either end of
/// either swing), straight between, then rounded into the street.
fn ramp_plan(seq: &[Vec2]) -> Vec<Vec2> {
    let step = 2.0f32;
    let ease = |from: Vec2, to: Vec2, out: &mut Vec<Vec2>| {
        let along = (to - from).dot((seq[2] - seq[1]).normalize_or(Vec2::X));
        let n = (along.abs() / step).ceil().max(1.0) as usize;
        let dir = (seq[2] - seq[1]).normalize_or(Vec2::X);
        let side = Vec2::new(-dir.y, dir.x);
        let (s0, s1) = (from.dot(side), to.dot(side));
        for k in 1..=n {
            let t = k as f32 / n as f32;
            let e = (1.0 - (std::f32::consts::PI * t).cos()) * 0.5;
            let p = from + dir * (along * t);
            out.push(p + side * ((s0 + (s1 - s0) * e) - p.dot(side)));
        }
    };
    let mut out = vec![seq[0]];
    ease(seq[0], seq[1], &mut out);
    out.push(seq[2]);
    ease(seq[2], seq[3], &mut out);
    // into the street: rounded, leaving its foot straight on along the viaduct
    let dir = (seq[3] - seq[2]).normalize_or(Vec2::X);
    let lead = seq[3] + dir * (seq[3].distance(seq[4]) * 0.4).min(12.0);
    let turn: Vec<Vec3> = [seq[3], lead, seq[4], seq[5]].iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect();
    out.extend(smooth(&turn, 3).into_iter().skip(1).map(|p| Vec2::new(p.x, p.z)));
    out
}

/// Chaikin's corner cutting on a line with heights, its two ends kept: corners become curves, grades are kept.
pub fn smooth(pts: &[Vec3], iterations: usize) -> Vec<Vec3> {
    let mut pts = pts.to_vec();
    for _ in 0..iterations {
        if pts.len() < 3 {
            break;
        }
        let mut out = vec![pts[0]];
        for w in pts.windows(2) {
            out.push(w[0] * 0.75 + w[1] * 0.25);
            out.push(w[0] * 0.25 + w[1] * 0.75);
        }
        out.push(pts[pts.len() - 1]);
        pts = out;
    }
    pts
}

/// Marks the blocks of each trunk's row as car parks under the viaduct (`BlockUse::Under`).
pub fn zone_rows(cities: &mut [City], plans: &[TrunkPlan]) {
    for p in plans {
        if let Some(city) = cities.iter_mut().find(|c| c.id == p.city) {
            for b in &mut city.blocks {
                if p.row.distance(b.rect.centre()) <= 0.0 {
                    b.kind = BlockUse::Under;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_keeps_the_ends_and_never_steepens_a_grade() {
        let raw = vec![
            Vec3::new(0.0, 9.0, 0.0),
            Vec3::new(30.0, 9.0, 19.0),
            Vec3::new(136.0, 0.15, 19.0),
            Vec3::new(160.0, 0.15, 50.0),
        ];
        let s = smooth(&raw, 3);
        assert_eq!(s[0], raw[0]);
        assert_eq!(*s.last().unwrap(), *raw.last().unwrap());
        let worst = s
            .windows(2)
            .map(|w| (w[1].y - w[0].y).abs() / Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length().max(1e-6))
            .fold(0.0f32, f32::max);
        assert!(worst <= 8.85 / 106.0 + 1e-3, "{worst}");
    }
}
