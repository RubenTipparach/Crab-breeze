//! A smoke puff's life (openspec change `smoke-puffs`): thrown, slowed by the air, rising as warm
//! smoke does, carried by the wind, growing and fading. Pure and deterministic; `cb-app` draws each
//! puff as a lit blob the way the clouds are drawn. Lives in `cb-core` so it is tested without a
//! GPU and stays one implementation for every source (explosions, trails, thrusters, wrecks).

use glam::Vec3;

/// How fast a puff loses the speed it was thrown at, per second (it keeps `exp(-DRAG_PER_S * t)`).
pub const DRAG_PER_S: f32 = 1.2;

/// The shape a puff is born with, from its effect layer and where it was thrown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PuffSpec {
    /// Starting diameter, metres.
    pub size_m: f32,
    /// Diameter added per second.
    pub grow_m_s: f32,
    /// Upward drift, metres per second.
    pub rise_m_s: f32,
    /// Lifetime, seconds.
    pub life_s: f32,
    /// Share of the life spent fading out at the end, 0..1.
    pub fade: f32,
    /// Opacity at full strength, 0..1.
    pub opacity: f32,
}

/// One puff in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Its centre, world metres.
    pub pos: Vec3,
    /// Its thrown velocity, metres per second (slowed by the air).
    pub vel: Vec3,
    /// Seconds lived.
    pub age_s: f32,
    /// What it was born as.
    pub spec: PuffSpec,
}

impl Puff {
    /// A puff born at `pos`, thrown at `vel`.
    pub fn new(pos: Vec3, vel: Vec3, spec: PuffSpec) -> Self {
        Self {
            pos,
            vel,
            age_s: 0.0,
            spec,
        }
    }

    /// Advances it `dt_s` seconds in `wind` (metres per second). A non-finite step changes nothing.
    pub fn step(&mut self, dt_s: f32, wind: Vec3) {
        if !dt_s.is_finite() || dt_s <= 0.0 || !wind.is_finite() {
            return;
        }
        self.vel *= (-DRAG_PER_S * dt_s).exp();
        self.pos += (self.vel + wind + Vec3::Y * self.spec.rise_m_s) * dt_s;
        self.age_s += dt_s;
    }

    /// True once its life is over.
    pub fn spent(&self) -> bool {
        self.age_s >= self.spec.life_s
    }

    /// Its diameter now, metres.
    pub fn size_m(&self) -> f32 {
        (self.spec.size_m + self.spec.grow_m_s * self.age_s).max(0.01)
    }

    /// Its opacity now: a quick fade in over the first tenth of a second, full, then fading out
    /// over the last `fade` share of its life.
    pub fn opacity(&self) -> f32 {
        let t = (self.age_s / self.spec.life_s).clamp(0.0, 1.0);
        let fade_in = (self.age_s / 0.1).clamp(0.0, 1.0);
        let fade_out = if self.spec.fade <= 0.0 {
            if t >= 1.0 { 0.0 } else { 1.0 }
        } else {
            ((1.0 - t) / self.spec.fade).clamp(0.0, 1.0)
        };
        self.spec.opacity * fade_in * fade_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> PuffSpec {
        PuffSpec {
            size_m: 3.0,
            grow_m_s: 2.0,
            rise_m_s: 1.5,
            life_s: 6.0,
            fade: 0.5,
            opacity: 0.8,
        }
    }

    #[test]
    fn a_puff_grows_rises_and_fades_over_its_life() {
        let mut p = Puff::new(Vec3::ZERO, Vec3::X * 10.0, spec());
        let mut last_y = p.pos.y;
        for _ in 0..30 {
            p.step(0.1, Vec3::ZERO);
            assert!(p.pos.y > last_y, "warm smoke rises");
            last_y = p.pos.y;
        }
        assert!((p.size_m() - 9.0).abs() < 1e-3, "3 m grown by 2 m a second for 3 s");
        assert!((p.opacity() - 0.8).abs() < 1e-3, "full strength mid-life");
        assert!(p.vel.length() < 10.0 * 0.05, "the air slows the throw");
        for _ in 0..25 {
            p.step(0.1, Vec3::ZERO);
        }
        assert!(p.opacity() < 0.8 * 0.4, "fading in the last half of its life");
        for _ in 0..10 {
            p.step(0.1, Vec3::ZERO);
        }
        assert!(p.spent() && p.opacity() == 0.0, "gone at the end");
    }

    #[test]
    fn the_wind_carries_a_puff_and_a_bad_step_changes_nothing() {
        let mut p = Puff::new(Vec3::ZERO, Vec3::ZERO, spec());
        p.step(1.0, Vec3::new(4.0, 0.0, 0.0));
        assert!((p.pos.x - 4.0).abs() < 1e-4);
        let before = p;
        p.step(f32::NAN, Vec3::ZERO);
        p.step(0.1, Vec3::new(f32::INFINITY, 0.0, 0.0));
        assert_eq!(p, before);
    }

    #[test]
    fn the_same_puff_moves_the_same_way_twice() {
        let run = || {
            let mut p = Puff::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(3.0, 5.0, -2.0), spec());
            for _ in 0..40 {
                p.step(1.0 / 60.0, Vec3::new(2.0, 0.0, 1.0));
            }
            p
        };
        assert_eq!(run(), run());
    }
}
