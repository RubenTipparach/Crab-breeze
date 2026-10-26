//! The fortress's defences aiming and firing (design sections 4 and 9): every live turret turns toward where its
//! round will meet the pilot's mech (`weapons::lead`, the cockpit's own lead), within its elevation, at the
//! turret rate; it fires its bursts only on its aim, in range and with a clear line past the hull. A missile
//! battery's cells do not turn: its missiles leave straight up and the seeker does the rest. A beam turret
//! charges where the pilot can see it, then burns what its beam touches. Losing the bridge doubles every aim
//! error. The game fires the rounds through the one `Shots` and draws the beams.

use super::{BeamPhase, Fortress, FortressData, FortressShape};
use crate::weapons::lead;
use glam::Vec3;

/// Where a defence is this step, in the world (from its model's nodes): the centre it turns about and its
/// muzzle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    /// The yaw ring's centre (or the part's, for one that does not turn).
    pub pivot: Vec3,
    /// Where its rounds leave.
    pub muzzle: Vec3,
}

/// What the defences shoot at: the pilot's mech.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Its torso.
    pub pos: Vec3,
    /// Its velocity.
    pub vel: Vec3,
}

/// A round fired this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fire {
    /// The part that fired it.
    pub part: usize,
    /// Its muzzle, world.
    pub from: Vec3,
    /// Its direction, world, unit length.
    pub dir: Vec3,
}

/// A beam turret this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// The part.
    pub part: usize,
    /// Its lens, world.
    pub from: Vec3,
    /// Where the beam ends (on the target when it is on its aim), world.
    pub to: Vec3,
    /// Charging or burning.
    pub phase: BeamPhase,
    /// Damage done to the target this step (0 unless it burns on its aim with a clear line).
    pub damage: f32,
}

fn seg_seg_distance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    // Closest points of two segments (Ericson, Real-Time Collision Detection, 5.1.9).
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t) = if a <= 1e-9 && e <= 1e-9 {
        (0.0, 0.0)
    } else if a <= 1e-9 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= 1e-9 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s = if denom > 1e-9 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (p1 + d1 * s).distance(p2 + d2 * t)
}

/// The hull stands between `a` and `b` (fortress frame). The line's first 12 m are its muzzle's own, and the
/// capsules are taken a little inside the hull they stand for, so a turret never hides behind its own mount.
pub fn hull_hides(shape: &FortressShape, a: Vec3, b: Vec3) -> bool {
    let len = a.distance(b);
    if len <= 12.0 {
        return false;
    }
    let a = a + (b - a) * (12.0 / len);
    shape.hull.iter().any(|c| seg_seg_distance(a, b, c.a, c.b) < c.radius_m * 0.85)
}

fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// The direction of yaw and elevation in the fortress's frame.
fn aim_dir(yaw: f32, elev: f32) -> Vec3 {
    Vec3::new(yaw.sin() * elev.cos(), elev.sin(), yaw.cos() * elev.cos())
}

impl Fortress {
    /// One fixed step of every defence: aim, and fire at `target` if it can. `mounts` holds each part's mount by
    /// part index (`None` for parts the game has not placed).
    pub fn guns(&mut self, d: &FortressData, mounts: &[Option<Mount>], target: Option<Target>, dt_s: f32) -> (Vec<Fire>, Vec<Beam>) {
        let mut fires = Vec::new();
        let mut beams = Vec::new();
        if self.crashed {
            return (fires, beams);
        }
        let rot = self.rotation();
        let inv = rot.inverse();
        let turn = d.cfg.turret_turn_deg_s.to_radians() * dt_s;
        let cone = d.cfg.fire_cone_deg.to_radians();
        let error_scale = if self.bridge_alive(d) { 1.0 } else { 2.0 };
        for i in 0..d.shape.parts.len() {
            let Some(g) = d.kind(i).gun.as_ref() else { continue };
            let Some(m) = mounts.get(i).copied().flatten() else { continue };
            if !self.parts[i].alive {
                continue;
            }
            let mut st = self.parts[i].gun;
            st.cool_s -= dt_s;
            let Some(t) = target else {
                st.beam = BeamPhase::Idle;
                self.parts[i].gun = st;
                continue;
            };
            let dist = m.muzzle.distance(t.pos);
            let in_range = dist <= g.range_m;
            let aim_at = if g.beam.is_some() {
                t.pos
            } else {
                lead(m.muzzle, t.pos, t.vel, g.weapon.speed_m_s, !g.weapon.shot().straight())
            };
            let want = inv * (aim_at - m.pivot);
            let want_yaw = want.x.atan2(want.z);
            let want_elev = want.y.atan2(want.x.hypot(want.z));
            let (lo, hi) = (g.elev_deg[0].to_radians(), g.elev_deg[1].to_radians());
            if g.turns {
                st.yaw_rad = wrap(st.yaw_rad + wrap(want_yaw - st.yaw_rad).clamp(-turn, turn));
                st.elev_rad += (want_elev.clamp(lo, hi) - st.elev_rad).clamp(-turn, turn);
            } else {
                // a battery's cells point up; its missiles turn on their seekers
                st.yaw_rad = 0.0;
                st.elev_rad = hi;
            }
            let pointing = aim_dir(st.yaw_rad, st.elev_rad);
            let on_aim = !g.turns || pointing.angle_between(want.normalize_or(Vec3::Z)) <= cone;
            let clear = !hull_hides(&d.shape, inv * (m.muzzle - self.pos), inv * (t.pos - self.pos));
            let world_dir = rot * pointing;
            if let Some(b) = &g.beam {
                let mut damage = 0.0;
                match st.beam {
                    BeamPhase::Idle => {
                        if st.cool_s <= 0.0 && in_range && clear {
                            st.beam = BeamPhase::Charging;
                            st.cool_s = b.charge_s;
                        }
                    }
                    BeamPhase::Charging => {
                        if st.cool_s <= 0.0 {
                            st.beam = BeamPhase::On;
                            st.cool_s = b.on_s;
                        }
                    }
                    BeamPhase::On => {
                        if on_aim && clear && in_range {
                            damage = b.damage_per_s * dt_s;
                        }
                        if st.cool_s <= 0.0 {
                            st.beam = BeamPhase::Idle;
                            st.cool_s = g.pause_s;
                        }
                    }
                }
                if st.beam != BeamPhase::Idle {
                    let reach = if on_aim && clear { dist.min(g.range_m) } else { g.range_m };
                    beams.push(Beam {
                        part: i,
                        from: m.muzzle,
                        to: m.muzzle + world_dir * reach,
                        phase: st.beam,
                        damage,
                    });
                }
            } else if st.cool_s <= 0.0 && in_range && clear && on_aim && (g.magazine == 0 || st.fired < g.magazine) {
                let e = (g.aim_error_deg * error_scale).to_radians();
                let (ea, eb) = (self.rng().range_f32(-e, e), self.rng().range_f32(-e, e));
                let side = world_dir.cross(Vec3::Y).normalize_or(Vec3::X);
                let up = side.cross(world_dir).normalize_or(Vec3::Y);
                let dir = (world_dir + side * ea.tan() + up * eb.tan()).normalize_or(world_dir);
                fires.push(Fire {
                    part: i,
                    from: m.muzzle,
                    dir,
                });
                st.fired += 1;
                st.left = st.left.saturating_sub(1);
                st.cool_s = g.weapon.interval_s;
                if st.left == 0 {
                    st.left = g.burst;
                    st.cool_s = g.pause_s.max(g.weapon.interval_s);
                }
            }
            st.cool_s = st.cool_s.max(-1.0);
            self.parts[i].gun = st;
        }
        (fires, beams)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{DT, fortress, kill};
    use super::*;

    /// Mounts for every gun at its part's rough place: 40 m above the hull's middle, so lines over the top are
    /// clear.
    fn mounts(f: &Fortress, d: &FortressData) -> Vec<Option<Mount>> {
        (0..d.shape.parts.len())
            .map(|i| {
                d.kind(i).gun.as_ref().map(|_| {
                    let p = f.pos + Vec3::Y * 60.0;
                    Mount { pivot: p, muzzle: p }
                })
            })
            .collect()
    }

    #[test]
    fn a_turret_does_not_fire_until_it_is_on_its_aim() {
        let (d, mut f) = fortress();
        let m = mounts(&f, &d);
        // straight behind every turret (they start facing +Z), a little above, 800 m out
        let behind = f.pos + f.rotation() * Vec3::new(0.0, 160.0, -800.0);
        let t = Target {
            pos: behind,
            vel: Vec3::ZERO,
        };
        let flak = d.part("flak1").unwrap();
        let (fires, _) = f.guns(&d, &m, Some(t), DT);
        assert!(fires.iter().all(|x| x.part != flak), "it has 180 degrees to turn");
        let mut first = None;
        for k in 1..600 {
            let (fires, _) = f.guns(&d, &m, Some(t), DT);
            if fires.iter().any(|x| x.part == flak) {
                first = Some(k as f32 * DT);
                break;
            }
        }
        let first = first.expect("it fires once on its aim");
        let turn_s = 180.0 / d.cfg.turret_turn_deg_s;
        assert!(first > turn_s * 0.8, "fired at {first} s, before it could have turned ({turn_s} s)");
    }

    #[test]
    fn the_hull_hides_the_pilot_from_the_far_side() {
        let (d, f) = fortress();
        let _ = f;
        // from above the deck to a point straight under the hull: through it
        assert!(hull_hides(&d.shape, Vec3::new(0.0, 55.0, 0.0), Vec3::new(0.0, -400.0, 0.0)));
        // from above the deck to a point high above: clear
        assert!(!hull_hides(&d.shape, Vec3::new(0.0, 55.0, 0.0), Vec3::new(300.0, 500.0, 0.0)));
    }

    #[test]
    fn a_turret_out_of_range_holds_fire() {
        let (d, mut f) = fortress();
        let m = mounts(&f, &d);
        let far = f.pos + Vec3::new(0.0, 3000.0, 3000.0);
        for _ in 0..600 {
            let (fires, beams) = f.guns(&d, &m, Some(Target { pos: far, vel: Vec3::ZERO }), DT);
            assert!(fires.is_empty() && beams.is_empty());
        }
    }

    #[test]
    fn losing_the_bridge_doubles_the_spread() {
        let (d, mut f) = fortress();
        let m = mounts(&f, &d);
        let t = Target {
            pos: f.pos + Vec3::new(0.0, 500.0, 600.0),
            vel: Vec3::ZERO,
        };
        let spread = |f: &mut Fortress| {
            let mut worst: f32 = 0.0;
            // settle on the target first: a turret fires anywhere within its cone while it swings on
            for _ in 0..600 {
                f.guns(&d, &m, Some(t), DT);
            }
            for _ in 0..1800 {
                let (fires, _) = f.guns(&d, &m, Some(t), DT);
                for x in fires.into_iter().filter(|x| d.shape.parts[x.part].kind == "flak") {
                    let want =
                        (lead(x.from, t.pos, t.vel, d.kind(x.part).gun.as_ref().unwrap().weapon.speed_m_s, true) - x.from).normalize();
                    worst = worst.max(x.dir.angle_between(want).to_degrees());
                }
            }
            worst
        };
        let whole = spread(&mut f.clone());
        kill(&d, &mut f, &["bridge"]);
        let blind = spread(&mut f);
        assert!(blind > whole * 1.5, "{whole} -> {blind}");
    }
}
