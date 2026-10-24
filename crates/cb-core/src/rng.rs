//! The one seeded random stream for simulation and generation (`deterministic-simulation`).
//!
//! SplitMix64 (Steele, Lea and Flood, 2014; public domain): one multiply-xorshift per draw,
//! identical on every platform. Streams fork by label, so a new subsystem never shifts the draws
//! of an existing one.

/// A seeded, forkable random stream. Never seeded from the clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimRng {
    seed: u64,
    state: u64,
}

const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a over a label: a stable 64-bit hash that does not depend on the platform or the process.
pub fn label_hash(label: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in label.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

impl SimRng {
    /// A stream from a seed.
    pub fn new(seed: u64) -> Self {
        Self { seed, state: seed }
    }

    /// A child stream that depends only on this stream's seed and `label`. Does not advance this
    /// stream.
    pub fn fork(&self, label: &str) -> Self {
        Self::new(mix(self.seed ^ mix(label_hash(label))))
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix(self.state)
    }

    /// A uniform number in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits fill an f32 mantissa exactly.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A uniform number in `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_stream() {
        let mut a = SimRng::new(42);
        let mut b = SimRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn forking_does_not_advance_the_parent() {
        let mut forked = SimRng::new(7);
        let _child = forked.fork("cities");
        let mut plain = SimRng::new(7);
        assert_eq!(forked.next_u64(), plain.next_u64());
    }

    #[test]
    fn different_labels_give_different_streams() {
        let root = SimRng::new(7);
        assert_ne!(root.fork("a").next_u64(), root.fork("b").next_u64());
    }

    #[test]
    fn floats_stay_in_the_unit_interval() {
        let mut r = SimRng::new(1);
        for _ in 0..10_000 {
            let x = r.next_f32();
            assert!((0.0..1.0).contains(&x));
        }
    }
}
