//! The frame budget rule (CLAUDE.md section 7; owner, 2026-10-26: "ensure that when you run perf, everything is always
//! above 60 fps"): after the loading screen's `GAME_READY`, no frame of a perf route may take longer than 16.7 ms. This
//! is the one place that rule is decided, so the game's `--perf-gate` mode, its log line and the tests agree. It lives
//! in the core because it is a rule over numbers, with no engine in it.

/// The longest a frame may take, ms: one frame at 60 fps.
pub const FRAME_BUDGET_MS: f32 = 1000.0 / 60.0;

/// One frame over the budget: which frame after `GAME_READY` (counting from 0) and how long it took, ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverBudget {
    /// The frame's index after `GAME_READY`.
    pub frame: u64,
    /// Its length, ms.
    pub ms: f32,
}

/// Every frame after `GAME_READY`, judged against the budget.
#[derive(Clone, Debug)]
pub struct FrameBudget {
    /// The budget, ms.
    pub budget_ms: f32,
    /// Frames seen after `GAME_READY`.
    pub frames: u64,
    /// The frames over the budget, in order (the first `KEEP_OVER` of them; `over` counts them all).
    pub listed: Vec<OverBudget>,
    /// How many frames were over the budget.
    pub over: u64,
    /// The longest frame, ms.
    pub worst_ms: f32,
}

/// The frames over budget kept for the report; past this only the count grows.
pub const KEEP_OVER: usize = 200;

impl Default for FrameBudget {
    fn default() -> Self {
        Self::new(FRAME_BUDGET_MS)
    }
}

impl FrameBudget {
    /// An empty tally against `budget_ms`.
    pub fn new(budget_ms: f32) -> Self {
        Self {
            budget_ms,
            frames: 0,
            listed: Vec::new(),
            over: 0,
            worst_ms: 0.0,
        }
    }

    /// Adds one frame of `ms` milliseconds. A frame that is not a finite, non-negative number is counted as over
    /// (a broken clock must not pass the gate).
    pub fn add(&mut self, ms: f32) {
        let frame = self.frames;
        self.frames += 1;
        let bad = !ms.is_finite() || ms < 0.0;
        if bad || ms > self.budget_ms {
            self.over += 1;
            if self.listed.len() < KEEP_OVER {
                self.listed.push(OverBudget {
                    frame,
                    ms: if bad { f32::INFINITY } else { ms },
                });
            }
        }
        if !bad {
            self.worst_ms = self.worst_ms.max(ms);
        } else {
            self.worst_ms = f32::INFINITY;
        }
    }

    /// True when at least one frame was seen and none was over the budget.
    pub fn passed(&self) -> bool {
        self.frames > 0 && self.over == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_whose_every_frame_is_within_16_7_ms_passes() {
        let mut b = FrameBudget::default();
        for _ in 0..1000 {
            b.add(8.0);
        }
        b.add(16.6);
        assert!(b.passed());
        assert_eq!(b.over, 0);
        assert!((b.worst_ms - 16.6).abs() < 1e-4);
    }

    #[test]
    fn one_frame_over_16_7_ms_fails_the_run_and_is_named() {
        let mut b = FrameBudget::default();
        for _ in 0..500 {
            b.add(7.0);
        }
        b.add(16.8);
        b.add(7.0);
        assert!(!b.passed());
        assert_eq!(b.over, 1);
        assert_eq!(b.listed, vec![OverBudget { frame: 500, ms: 16.8 }]);
    }

    #[test]
    fn a_run_with_no_frames_does_not_pass() {
        assert!(!FrameBudget::default().passed());
    }

    #[test]
    fn a_broken_clock_fails_the_run() {
        let mut b = FrameBudget::default();
        b.add(5.0);
        b.add(f32::NAN);
        assert!(!b.passed());
        assert_eq!(b.worst_ms, f32::INFINITY);
    }

    #[test]
    fn past_the_kept_list_only_the_count_grows() {
        let mut b = FrameBudget::default();
        for _ in 0..(KEEP_OVER + 50) {
            b.add(20.0);
        }
        assert_eq!(b.listed.len(), KEEP_OVER);
        assert_eq!(b.over, (KEEP_OVER + 50) as u64);
    }
}
