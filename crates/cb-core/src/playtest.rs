//! Playtest scripts (openspec change `playtest-pipeline`): a scripted pilot's beats, read from
//! `data/playtests/<name>.ron` and checked on load. The pilot that plays them lives in the app and writes only the
//! input a player's keyboard and mouse would (CLAUDE.md section 9); this is the script and nothing else.

use crate::data::{Validate, positive};
use serde::Deserialize;

/// One beat of a script.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum Beat {
    /// Do nothing for this many seconds.
    Wait(f32),
    /// Look slowly left and right for this many seconds.
    LookAround(f32),
    /// Walk on foot to the terminal of this bay at the career's current base, and face it.
    WalkToTerminal(u32),
    /// Press Use at the terminal: its screen opens.
    UseTerminal,
    /// Click a tab of the terminal's screen: `Jobs`, `Shop`, `Refit` or `Hangar`.
    Tab(String),
    /// Click Accept on the open offer nearest the pilot whose tier is at most this.
    AcceptJob(u8),
    /// Press M for the mail and click Accept on the open offer nearest or farthest from the mech whose tier is at most
    /// this, then press M again to close it (a free-roam performance run's trips). Waits for an offer while none is open.
    AcceptByMail(Pick, u8),
    /// Press Use to step back from the terminal's screen.
    CloseTerminal,
    /// Walk to the mech, press Use, and climb into the seat.
    Board,
    /// Drive the mech to the accepted job's site, until its fight starts.
    GoToJob,
    /// Skate the mech flat out to (x, z) metres, as `GoToJob` drives, until within 40 m: a fixed route across the island
    /// for a streaming performance run.
    SkateTo(f32, f32),
    /// Fly the mech on its jets to (x, z) metres, holding about this height over the ground (metres), until within
    /// 250 m (at jet speed the mech turns too wide to settle closer): the same route from the air.
    JetTo(f32, f32, f32),
    /// Fight the job's enemies until it is won, at most this many seconds.
    FightJob(f32),
    /// Drive the mech into this bay of the career's current base and stop there.
    EnterBay(u32),
    /// Hold Use in the seat until the pilot has climbed out.
    Disembark,
    /// Credits paid into the career through its ledger as a disclosed playtest grant (a scripted shortcut for
    /// earnings a single take cannot make; the report says so).
    Grant(i64),
    /// Click BUY on the shop's listing for this design (`mechs/sapper.ron`).
    BuyMech(String),
    /// Check the jobs board offers a job of at least this tier.
    ExpectOffer(u8),
    /// Wait, at most this many seconds, until every air drop on its way has landed (mech-shop section 6).
    AwaitDrop(f32),
    /// Walk on foot toward the newest landed air drop's mech until it is the one that kneels for the pilot.
    WalkToDrop,
    /// On the REFIT tab, click BUY for this weapon id (`data/career/parts.ron`).
    BuyPart(String),
    /// On the REFIT tab, click FIT for this spare weapon on this station (0 first).
    Fit(u32, String),
    /// Press Escape and click Quit: the game saves and exits. Writes what it saved for `ExpectRestored`.
    Quit,
    /// After Continue: check the career and the pilot's place came back, within this many metres.
    ExpectRestored(f32),
}

/// Which open offer `AcceptByMail` takes, by its site's distance from the mech.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum Pick {
    /// The closest.
    Nearest,
    /// The farthest.
    Farthest,
}

/// One launch of the game in a script: how it is started and the beats it plays.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The `--play` mode it starts in (`career-new`, `career-continue`, `free`, `free-continue`).
    pub play: String,
    /// The beats, in order.
    pub beats: Vec<Beat>,
}

/// A playtest script, `data/playtests/<name>.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// Its name, as in the file name.
    pub name: String,
    /// A beat that makes no progress for this long, seconds of game time, is stuck (design section 1: 10 s).
    pub stuck_s: f32,
    /// The runs, launched one after the other (`--run <i>`).
    pub runs: Vec<Run>,
}

impl Validate for Script {
    fn validate(&self) -> Result<(), String> {
        positive("stuck_s", self.stuck_s)?;
        if self.runs.is_empty() {
            return Err("runs: none".into());
        }
        for (i, r) in self.runs.iter().enumerate() {
            if !matches!(r.play.as_str(), "career-new" | "career-continue" | "free" | "free-continue") {
                return Err(format!("runs[{i}].play: `{}` is not a --play mode", r.play));
            }
            if r.beats.is_empty() {
                return Err(format!("runs[{i}].beats: none"));
            }
            for (j, b) in r.beats.iter().enumerate() {
                let bad = match b {
                    Beat::Wait(s) | Beat::LookAround(s) | Beat::FightJob(s) | Beat::AwaitDrop(s) => !(s.is_finite() && *s > 0.0),
                    Beat::ExpectRestored(m) => !(m.is_finite() && *m > 0.0),
                    Beat::Tab(t) => !matches!(t.as_str(), "Jobs" | "Shop" | "Refit" | "Hangar"),
                    Beat::AcceptJob(t) | Beat::AcceptByMail(_, t) | Beat::ExpectOffer(t) => !(1..=crate::jobs::MAX_TIER).contains(t),
                    Beat::Grant(c) => *c <= 0,
                    Beat::SkateTo(x, z) => !(x.is_finite() && z.is_finite()),
                    Beat::JetTo(x, z, h) => !(x.is_finite() && z.is_finite() && h.is_finite() && *h > 0.0),
                    Beat::BuyMech(d) => !d.starts_with("mechs/"),
                    Beat::BuyPart(id) | Beat::Fit(_, id) => id.is_empty(),
                    _ => false,
                };
                if bad {
                    return Err(format!("runs[{i}].beats[{j}]: {b:?} has a value out of range"));
                }
            }
        }
        Ok(())
    }
}

/// What a run saved at `Quit`, for the next run's `ExpectRestored` (written to `output/playtests/`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
pub struct Expect {
    /// Credits.
    pub money: i64,
    /// Owned mechs' designs, in order.
    pub mechs: Vec<String>,
    /// The piloted mech's fit.
    pub fit: Vec<Option<String>>,
    /// Jobs done.
    pub done: usize,
    /// Where the pilot's feet were (on foot) or the mech's (seated), x and z.
    pub at: [f32; 2],
    /// Seated in the mech.
    pub seated: bool,
}

impl Expect {
    /// Its file's text.
    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default()
    }

    /// Read back from its file's text.
    pub fn from_ron(text: &str) -> Option<Self> {
        ron::from_str(text).ok()
    }

    /// Where `now` differs from `self`, each as a line; empty when it came back (positions within `tol_m`).
    pub fn differences(&self, now: &Expect, tol_m: f32) -> Vec<String> {
        let mut out = Vec::new();
        if self.money != now.money {
            out.push(format!("money {} became {}", self.money, now.money));
        }
        if self.mechs != now.mechs {
            out.push(format!("mechs {:?} became {:?}", self.mechs, now.mechs));
        }
        if self.fit != now.fit {
            out.push(format!("fit {:?} became {:?}", self.fit, now.fit));
        }
        if self.done != now.done {
            out.push(format!("jobs done {} became {}", self.done, now.done));
        }
        let d = ((self.at[0] - now.at[0]).powi(2) + (self.at[1] - now.at[1]).powi(2)).sqrt();
        if d.is_nan() || d > tol_m {
            out.push(format!("place moved {d:.1} m"));
        }
        if self.seated != now.seated {
            out.push(format!("seated {} became {}", self.seated, now.seated));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_career_loop_script_loads() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/playtests/career-loop.ron");
        let s: Script = crate::data::load_ron(&p).unwrap();
        assert_eq!(s.name, "career-loop");
        assert_eq!(s.runs[0].play, "career-new");
        assert_eq!(s.runs.last().map(|r| r.play.as_str()), Some("career-continue"));
        // Beat 5: the bought mech is air dropped, walked up to and boarded (mech-shop section 6).
        let beats = &s.runs[0].beats;
        let bought = beats.iter().position(|b| matches!(b, Beat::BuyMech(_))).unwrap();
        let wait = beats.iter().position(|b| matches!(b, Beat::AwaitDrop(_))).unwrap();
        assert!(bought < wait);
        assert!(matches!(beats[wait + 1], Beat::WalkToDrop));
        assert!(matches!(beats[wait + 2], Beat::Board));
    }

    #[test]
    fn a_script_with_a_bad_beat_is_refused_naming_it() {
        let text = r#"(name: "x", stuck_s: 10.0, runs: [(play: "career-new", beats: [Wait(1.0), Tab("Mail")])])"#;
        let e = crate::data::parse_ron::<Script>(text, std::path::Path::new("x.ron")).unwrap_err();
        assert!(e.reason.contains("beats[1]"), "{e}");
        let text = r#"(name: "x", stuck_s: 10.0, runs: [(play: "career-new", beats: [Fly])])"#;
        assert!(
            crate::data::parse_ron::<Script>(text, std::path::Path::new("x.ron")).is_err(),
            "an unknown beat"
        );
    }

    #[test]
    fn a_restore_is_checked_field_by_field() {
        let a = Expect {
            money: 10,
            mechs: vec!["mechs/foreman.ron".into()],
            fit: vec![None],
            done: 1,
            at: [5.0, 5.0],
            seated: false,
        };
        let mut b = a.clone();
        b.at = [5.5, 5.0];
        assert!(a.differences(&b, 1.0).is_empty());
        b.money = 9;
        b.at = [50.0, 5.0];
        assert_eq!(a.differences(&b, 1.0).len(), 2);
    }
}
