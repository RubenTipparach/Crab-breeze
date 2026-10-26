//! Which clip a person plays and where in it they are (openspec change `people-vat`, design section 5):
//! the rule from what the simulation says they are doing to a clip of the baked library, the walk's
//! cadence from the body's speed so the feet do not slide, and a short crossfade at every change of
//! clip. Pure: the game hands the result to the renderer as rows and a blend; nothing here knows of a GPU.

use super::{Activity, Person};
use crate::people_vat::Manifest;

/// A clip of the people's library, in the order the table names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Clip {
    /// Standing, breathing.
    Idle,
    /// Walking.
    Walk,
    /// Running.
    Run,
    /// Sitting.
    Sit,
    /// Waving a greeting.
    Wave,
    /// Talking, standing.
    Talk,
    /// Falling and lying still (does not loop: holds its last frame).
    Fall,
}

impl Clip {
    /// Every clip.
    pub const ALL: [Clip; 7] = [Clip::Idle, Clip::Walk, Clip::Run, Clip::Sit, Clip::Wave, Clip::Talk, Clip::Fall];

    /// Its name in the bake manifest.
    pub fn name(self) -> &'static str {
        match self {
            Clip::Idle => "idle",
            Clip::Walk => "walk",
            Clip::Run => "run",
            Clip::Sit => "sit",
            Clip::Wave => "wave",
            Clip::Talk => "talk",
            Clip::Fall => "fall",
        }
    }

    /// True for a clip whose cadence follows the body's speed.
    pub fn moves(self) -> bool {
        matches!(self, Clip::Walk | Clip::Run)
    }
}

/// Below this a person is standing still, m/s.
pub const STILL_M_S: f32 = 0.2;

/// The clip for what a person is doing: walking or running by their pace, still when they are not
/// moving, sitting on a bench, the wave for the first `wave_s` of a greeting and talking after it, and
/// the fall once struck down. `greet_hold_s` is how long a greeting lasts (`PeopleConfig`).
pub fn choose(p: &Person, greet_hold_s: f32, wave_s: f32) -> Clip {
    let moving = p.pace_m_s > STILL_M_S;
    match p.activity {
        Activity::Down { .. } => Clip::Fall,
        Activity::Sitting { .. } => Clip::Sit,
        Activity::Greeting { left_s, .. } => {
            if greet_hold_s - left_s < wave_s {
                Clip::Wave
            } else {
                Clip::Talk
            }
        }
        Activity::Fleeing { .. } if moving => Clip::Run,
        Activity::Walking if moving => Clip::Walk,
        _ => Clip::Idle,
    }
}

/// One clip's timing as the bake made it: frames, rate, loop, and the speed a moving clip walks at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipTiming {
    /// Its first row in the texture.
    pub row: u32,
    /// Frames.
    pub frames: u32,
    /// Frames a second.
    pub fps: f32,
    /// True when it loops.
    pub looped: bool,
    /// The ground speed it walks at played at its own rate, m/s (0 for a clip that does not move).
    pub speed_m_s: f32,
}

impl ClipTiming {
    /// Its length, seconds (a loop's whole period; a held clip's first to last frame).
    pub fn length_s(&self) -> f32 {
        if self.looped {
            self.frames as f32 / self.fps
        } else {
            (self.frames.max(1) - 1) as f32 / self.fps
        }
    }
}

/// Every clip's timing for one body, read from the bake manifest (never typed in).
#[derive(Clone, Debug, PartialEq)]
pub struct ClipTable(pub [ClipTiming; 7]);

impl ClipTable {
    /// The timings of `body` in `manifest`; an error names a clip missing or a moving clip with no speed.
    pub fn from_manifest(manifest: &Manifest, body: &str) -> Result<Self, String> {
        let mut out = [ClipTiming {
            row: 0,
            frames: 1,
            fps: 1.0,
            looped: true,
            speed_m_s: 0.0,
        }; 7];
        for (slot, clip) in out.iter_mut().zip(Clip::ALL) {
            let c = manifest.clip(clip.name()).ok_or_else(|| format!("no clip {}", clip.name()))?;
            let speed = if clip.moves() {
                *c.speed_m_s_by_body
                    .get(body)
                    .ok_or_else(|| format!("clip {}: no speed for {body}", c.name))?
            } else {
                0.0
            };
            *slot = ClipTiming {
                row: c.row,
                frames: c.frames,
                fps: c.fps,
                looped: c.looped,
                speed_m_s: speed,
            };
        }
        Ok(Self(out))
    }

    /// A clip's timing.
    pub fn get(&self, c: Clip) -> &ClipTiming {
        &self.0[c as usize]
    }

    /// How fast to play `clip` for a body moving at `pace_m_s`: a moving clip at the pace over the
    /// speed it was baked walking at, within `cadence` (least, most), so the feet keep to the ground;
    /// anything else at its own rate.
    pub fn rate(&self, clip: Clip, pace_m_s: f32, cadence: [f32; 2]) -> f32 {
        let t = self.get(clip);
        if clip.moves() && t.speed_m_s > 0.0 && pace_m_s.is_finite() {
            (pace_m_s / t.speed_m_s).clamp(cadence[0], cadence[1])
        } else {
            1.0
        }
    }
}

/// A clip being played: which, how far in (seconds of the clip, at its own rate) and how fast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Play {
    /// The clip.
    pub clip: Clip,
    /// Seconds into it.
    pub t_s: f32,
    /// Playback rate (1 its own).
    pub rate: f32,
}

impl Play {
    /// The frame to show, a fraction between two frames: wrapped within a loop, held at the last frame
    /// of a clip that does not loop.
    pub fn frame(&self, table: &ClipTable) -> f32 {
        let t = table.get(self.clip);
        let f = self.t_s * t.fps;
        if t.looped {
            f.rem_euclid(t.frames as f32)
        } else {
            f.clamp(0.0, (t.frames.max(1) - 1) as f32)
        }
    }

    fn advance(&mut self, dt_s: f32, table: &ClipTable) {
        let t = table.get(self.clip);
        self.t_s += dt_s * self.rate;
        let len = t.length_s();
        if t.looped {
            self.t_s = self.t_s.rem_euclid(len.max(1e-6));
        } else {
            self.t_s = self.t_s.min(len);
        }
    }
}

/// A person's animation: the clip playing, and the one it is fading from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anim {
    /// The clip playing.
    pub now: Play,
    /// The clip it is fading from, while the crossfade lasts.
    pub prev: Option<Play>,
    /// Seconds since the change.
    pub since_s: f32,
}

impl Anim {
    /// A person first drawn: `clip` from a point in it set by `id`, so a crowd is not in step.
    pub fn start(clip: Clip, id: u32, table: &ClipTable) -> Self {
        let t = table.get(clip);
        let offset = if t.looped {
            // a fixed fraction of the loop by id (Weyl sequence), never the global RNG
            (id as f32 * 0.618_034).fract() * t.length_s()
        } else {
            0.0
        };
        Self {
            now: Play {
                clip,
                t_s: offset,
                rate: 1.0,
            },
            prev: None,
            since_s: f32::INFINITY,
        }
    }

    /// One frame: change to `want` if it is not playing (fading from the old clip over `crossfade_s`;
    /// between walking and running the feet keep their place in the stride), set the rate, and advance.
    pub fn step(&mut self, want: Clip, rate: f32, dt_s: f32, crossfade_s: f32, table: &ClipTable) {
        if want != self.now.clip {
            let from = table.get(self.now.clip);
            let to = table.get(want);
            let t_s = if self.now.clip.moves() && want.moves() {
                self.now.t_s / from.length_s().max(1e-6) * to.length_s()
            } else {
                0.0
            };
            self.prev = Some(self.now);
            self.now = Play { clip: want, t_s, rate };
            self.since_s = 0.0;
        } else {
            self.since_s += dt_s;
        }
        self.now.rate = rate;
        self.now.advance(dt_s, table);
        if let Some(p) = self.prev.as_mut() {
            p.advance(dt_s, table);
        }
        if self.since_s >= crossfade_s {
            self.prev = None;
        }
    }

    /// How much of the clip playing is shown, 0-1 (1 once the crossfade is over).
    pub fn weight(&self, crossfade_s: f32) -> f32 {
        if self.prev.is_none() || crossfade_s <= 0.0 {
            1.0
        } else {
            (self.since_s / crossfade_s).clamp(0.0, 1.0)
        }
    }
}

/// What a person looks like, from their `look`: which body, and one colour of each palette list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outfit {
    /// The body (an index into the bake manifest's bodies).
    pub body: usize,
    /// Indices into the palette's skin, hair, top, bottom and shoes lists.
    pub colours: [u32; 5],
}

impl Outfit {
    /// The outfit for `look` given how many bodies there are and how long each palette list is (in the order
    /// of `Palette::MAX`). Every pick is a different slice of a mixed `look`, so neighbours differ.
    pub fn of(look: u32, bodies: usize, lists: [usize; 5]) -> Self {
        // a 32-bit mix (Murmur3's finaliser) so consecutive looks spread over every list
        let mut h = look;
        h ^= h >> 16;
        h = h.wrapping_mul(0x85eb_ca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2_ae35);
        h ^= h >> 16;
        let pick = |shift: u32, n: usize| ((h >> shift) & 0xff) % (n.max(1) as u32);
        Self {
            body: (h % (bodies.max(1) as u32)) as usize,
            colours: [
                pick(4, lists[0]),
                pick(10, lists[1]),
                pick(16, lists[2]),
                pick(22, lists[3]),
                pick(26, lists[4]),
            ],
        }
    }

    /// The colours packed for the renderer: skin 3 bits, hair 3, top 4, bottom 4, shoes 3.
    pub fn packed(&self) -> u32 {
        let [skin, hair, top, bottom, shoes] = self.colours;
        skin | hair << 3 | top << 6 | bottom << 10 | shoes << 14
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civilians::tests::a_person;

    fn table() -> ClipTable {
        let t = |row, frames, looped, speed_m_s| ClipTiming {
            row,
            frames,
            fps: 24.0,
            looped,
            speed_m_s,
        };
        ClipTable([
            t(0, 60, true, 0.0),
            t(60, 32, true, 1.4),
            t(92, 16, true, 5.0),
            t(108, 40, true, 0.0),
            t(148, 48, true, 0.0),
            t(196, 70, true, 0.0),
            t(266, 58, false, 0.0),
        ])
    }

    #[test]
    fn what_a_person_does_picks_the_clip() {
        let mut p = a_person();
        p.pace_m_s = 1.4;
        p.activity = Activity::Walking;
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Walk);
        p.pace_m_s = 0.0;
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Idle);
        p.activity = Activity::Waiting { left_s: 3.0 };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Idle);
        p.activity = Activity::Sitting { left_s: 30.0 };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Sit);
        p.pace_m_s = 4.5;
        p.activity = Activity::Fleeing {
            left_s: 9.0,
            dir: glam::Vec2::X,
            door: None,
        };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Run);
        p.activity = Activity::Down { left_s: 20.0 };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Fall);
    }

    #[test]
    fn a_greeting_waves_first_then_talks() {
        let mut p = a_person();
        p.activity = Activity::Greeting {
            left_s: 5.5,
            face: glam::Vec2::ZERO,
        };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Wave);
        p.activity = Activity::Greeting {
            left_s: 3.0,
            face: glam::Vec2::ZERO,
        };
        assert_eq!(choose(&p, 6.0, 2.0), Clip::Talk);
    }

    #[test]
    fn the_walk_keeps_time_with_the_pace_so_feet_do_not_slide() {
        let tb = table();
        assert!((tb.rate(Clip::Walk, 1.4, [0.5, 2.0]) - 1.0).abs() < 1e-6);
        assert!((tb.rate(Clip::Walk, 1.75, [0.5, 2.0]) - 1.25).abs() < 1e-6);
        assert_eq!(tb.rate(Clip::Walk, 10.0, [0.5, 2.0]), 2.0);
        assert_eq!(tb.rate(Clip::Idle, 1.4, [0.5, 2.0]), 1.0);
        // a stride's ground covered equals the body's: one loop at the pace's rate covers pace x time
        let rate = tb.rate(Clip::Walk, 1.6, [0.5, 2.0]);
        let loop_s = tb.get(Clip::Walk).length_s() / rate;
        assert!((loop_s * 1.6 - tb.get(Clip::Walk).length_s() * 1.4).abs() < 1e-4);
    }

    #[test]
    fn a_loop_wraps_and_the_fall_holds_its_last_frame() {
        let tb = table();
        let mut a = Anim::start(Clip::Walk, 0, &tb);
        for _ in 0..200 {
            a.step(Clip::Walk, 1.0, 1.0 / 60.0, 0.25, &tb);
            assert!((0.0..32.0).contains(&a.now.frame(&tb)));
        }
        let mut f = Anim::start(Clip::Fall, 3, &tb);
        for _ in 0..600 {
            f.step(Clip::Fall, 1.0, 1.0 / 60.0, 0.25, &tb);
        }
        assert_eq!(f.now.frame(&tb), 57.0);
    }

    #[test]
    fn a_change_of_clip_fades_over_the_crossfade_then_drops_the_old_clip() {
        let tb = table();
        let mut a = Anim::start(Clip::Idle, 0, &tb);
        a.step(Clip::Sit, 1.0, 0.1, 0.25, &tb);
        assert_eq!(a.prev.map(|p| p.clip), Some(Clip::Idle));
        assert!(a.weight(0.25) < 0.01);
        a.step(Clip::Sit, 1.0, 0.1, 0.25, &tb);
        assert!((a.weight(0.25) - 0.4).abs() < 1e-5);
        a.step(Clip::Sit, 1.0, 0.2, 0.25, &tb);
        assert_eq!(a.prev, None);
        assert_eq!(a.weight(0.25), 1.0);
    }

    #[test]
    fn walking_into_a_run_keeps_the_feet_where_they_are_in_the_stride() {
        let tb = table();
        let mut a = Anim::start(Clip::Walk, 0, &tb);
        a.now.t_s = tb.get(Clip::Walk).length_s() * 0.3;
        a.step(Clip::Run, 1.0, 0.0, 0.25, &tb);
        let into = a.now.t_s / tb.get(Clip::Run).length_s();
        assert!((into - 0.3).abs() < 1e-5, "{into}");
    }

    #[test]
    fn looks_spread_over_both_bodies_and_every_colour_and_pack_within_their_bits() {
        let lists = crate::civilians::Palette::MAX;
        let mut bodies = [0; 2];
        let mut tops = std::collections::BTreeSet::new();
        for look in 0..1000 {
            let o = Outfit::of(look, 2, lists);
            bodies[o.body] += 1;
            tops.insert(o.colours[2]);
            for (c, n) in o.colours.iter().zip(lists) {
                assert!((*c as usize) < n);
            }
            let p = o.packed();
            assert_eq!((p >> 6) & 15, o.colours[2]);
            assert_eq!((p >> 14) & 7, o.colours[4]);
        }
        assert!(bodies[0] > 400 && bodies[1] > 400, "{bodies:?}");
        assert_eq!(tops.len(), 16);
    }

    #[test]
    fn people_start_at_different_points_of_a_loop() {
        let tb = table();
        let a = Anim::start(Clip::Walk, 1, &tb).now.t_s;
        let b = Anim::start(Clip::Walk, 2, &tb).now.t_s;
        assert!((a - b).abs() > 0.05);
        assert_eq!(Anim::start(Clip::Walk, 7, &tb), Anim::start(Clip::Walk, 7, &tb));
    }
}
