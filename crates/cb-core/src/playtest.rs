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
    /// Walk on foot out toward the site of the air drop on its way, stopping this many metres short, to watch it come
    /// down in the open (a hangar's walls hide the sky).
    WalkTowardDrop(f32),
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
    /// Walk on foot to the nearest standing, unfrightened person within this many metres, until they are in greeting
    /// reach and faced (`civilians::People::in_reach`).
    WalkToPerson(f32),
    /// Turn the mouse wheel this many notches (positive: down), one every 0.3 s, as a mouse's own wheel messages.
    Scroll(i32),
    /// Turn the wheel until Use's selection is this interaction (`Greet`, `Terminal`, `GetIn`, `GetOut`, `Board`),
    /// then press Use. It must come into reach within 10 s.
    UseOn(String),
    /// Wait, at most this many seconds, until the pilot is `OnFoot`, `Driving` or `Seated`.
    ExpectMode(String, f32),
    /// Walk on foot to the driver's door of the nearest car within this many metres, and face the car.
    WalkToCar(f32),
    /// Drive the car for this many seconds at this throttle (0..1), steering along its lane's line ahead, then brake
    /// to a stop.
    DriveCar(f32, f32),
    /// Walk in at the street door of the nearest standing building within this many metres and climb this many
    /// storeys of its stair, by its own route (`world::interior::Entrance::route`, `Stair::climb`).
    WalkIntoBuilding(f32, u32),
    /// Walk in at the front door of the nearest standing house within this many metres (not one `FireAtNearest` shot)
    /// and into its front room, ending there (the suburb's interiors, for performance).
    WalkIntoHouse(f32),
    /// Write what is about the pilot within this many metres (people by what they do, cars, lit junctions and their
    /// lights, aircraft, boats, harbours, the fog), a note for the report. Takes no time.
    Survey(f32),
    /// Turn the view toward the nearest of a kind (`person`, `car`, `light`, `aircraft`, `boat`, `harbour`, `door`,
    /// `bench`, `enemy`, `fortress`) and hold it there for this many seconds.
    LookAtNearest(String, f32),
    /// Skate the mech to the nearest of a kind (`harbour`, `runway`, `city`), as `SkateTo`, stopping this many metres
    /// short of it.
    SkateToNearest(String, f32),
    /// Fly the mech on its jets to the nearest of a kind (as `SkateToNearest`) at this height, as `JetTo`.
    JetToNearest(String, f32),
    /// Turn the wheel until the selected weapon station's name holds this text.
    SelectWeapon(String),
    /// Hold the primary trigger this many seconds, aimed at the nearest enemy in sight, else at the ground ahead.
    FireFor(f32),
    /// Hold the scope (the middle mouse button) this many seconds, turning the wheel this many notches while it is
    /// open (one every second).
    Scope(f32, i32),
    /// Hold the missile key this many seconds, then let go: a multi-lock pod fires its salvo.
    MultiLock(f32),
    /// Press target-next this many times, a second apart, writing what each selects.
    CycleTargets(u32),
    /// Hold free look, look at the left MFD and press fire on it, then let go: the page before, in focus, after.
    MfdFocus,
    /// Aim at the ground this many metres ahead and fire the missile key this many times, 2.5 s apart.
    FireAtGround(f32, u32),
    /// Bring down the nearest buildings (`building`) or houses (`house`) within 800 m of the mech, one after another:
    /// aim at each, hold the primary trigger and fire the missile key every 2.5 s for this many seconds, then the next,
    /// this many in all (the city-destruction performance runs).
    FireAtNearest(String, f32, u32),
    /// Press M, click START on this free-roam mission (`Fortress`, `Defence`), press M again.
    StartMission(String),
    /// Fly the mech up onto the Stratostacean's deck and stand there, at most this many seconds.
    LandOnFortress(f32),
    /// Bring the Stratostacean down, at most this many seconds (crab-airship section 10): stand off the thruster
    /// `Fortress::next_to_bring_down` names until its nozzle is at most `BRING_DOWN_PITCH_DEG` up, hold the selected
    /// gun on the nozzle; at a lit hull thruster, select it and fire missiles at the lock. Done when it crashes.
    BringDownFortress(f32),
    /// Fight every hostile left in the world (its fighters, its deck guard, any vehicle), nearest first, as `FightJob`
    /// fights, until none has been left for 3 s, at most this many seconds (owner, 2026-10-30: "take out all hostiles
    /// too").
    ClearHostiles(f32),
    /// Toggle the skates on (true) or off.
    Skates(bool),
    /// Walk on foot toward the mech's stair until within this many metres of it (not boarding).
    WalkToMech(f32),
}

/// How far off a lit hull thruster the scripted pilot stands to fire missiles up into its port, metres along the
/// ground: nearly under it, so a missile (launched at most 25 deg up, turning 70 deg/s) climbs into the port within
/// 30 deg of vertical, the only way through (a thruster 530 m up is 13 deg off vertical from here). From 500 m out
/// they arrived about 45 deg off and did nothing (fortress-down run 3).
pub const BRING_DOWN_MISSILE_STANDOFF_M: f32 = 120.0;

/// How far off a lift thruster's nozzle the scripted pilot hovers on its jets, metres along the ground (owner,
/// 2026-10-30: "the mech should get up close to the crab carrier", "tap jets so it recharges as you fly up").
pub const BRING_DOWN_HOVER_STANDOFF_M: f32 = 150.0;

/// How far the nozzle is above the hovering pilot's eye, degrees: well under the Foreman's +25 deg aim limit.
pub const BRING_DOWN_HOVER_PITCH_DEG: f32 = 12.0;

/// The height to hover at to shoot up into a nozzle at `nozzle_y_m` from `standoff_m` off it: the nozzle
/// `BRING_DOWN_HOVER_PITCH_DEG` above the eye, the eye `eye_m` above the mech's feet.
pub fn bring_down_hover_y(nozzle_y_m: f32, standoff_m: f32, eye_m: f32) -> f32 {
    nozzle_y_m - standoff_m * BRING_DOWN_HOVER_PITCH_DEG.to_radians().tan() - eye_m
}

/// Whether a scripted pilot holding a height on its jets burns them now. It taps them (owner, 2026-10-30: "you can
/// tap jets so it recharges as you fly up"): on, it burns until it is rising (over 12 m/s, or at all once over the
/// height) or the jets are hot (0.8); off, it coasts at least 2 s, long enough for the jets to start cooling (1.5 s
/// after the last burn), unless it is 60 m under the height or falling at over 35 m/s. `below_m` is the height
/// less the mech's, `vy` its vertical speed, `off_s` how long the jets have been off.
pub fn tap_jets(on: bool, below_m: f32, vy: f32, heat: f32, off_s: f32) -> bool {
    if on {
        !(vy > 12.0 || (below_m < 0.0 && vy > 0.0) || heat > 0.8)
    } else {
        below_m > 60.0 || vy < -35.0 || (off_s > 2.0 && below_m > 0.0 && vy <= 0.0)
    }
}

/// What `UseOn` can ask for: `cb_core::interact::Interaction`'s names.
pub const USE_ON: [&str; 5] = ["Greet", "Terminal", "GetIn", "GetOut", "Board"];
/// What `LookAtNearest` can look at.
pub const LOOK_KINDS: [&str; 10] = [
    "person", "car", "light", "aircraft", "boat", "harbour", "door", "bench", "enemy", "fortress",
];
/// Where `SkateToNearest` and `JetToNearest` can go.
pub const GO_KINDS: [&str; 5] = ["harbour", "runway", "city", "house", "farthest-city"];

/// Heat at or over which a scripted pilot on a long jet leg comes down to skate (the jets lock out at 1, survey M3, M5).
pub const JET_REST_AT: f32 = 0.85;
/// Heat at or under which it takes to the jets again.
pub const JET_RELIGHT_AT: f32 = 0.25;

/// Whether a pilot flying a jet leg should be resting the jets now (down on the skates while they recharge), from the
/// jets' `heat` (0 full, 1 spent), whether they are `locked` out, and whether it was resting the step before. The jets
/// carry a mech about 20 s at a time, a short hop, never a 5 km leg: a pilot that held the lift on until the lock-out
/// dropped and climbed over the same ridge and never got anywhere (the streaming route's jet leg, stuck in mid-air
/// near (-8 597, 1 092) on both machines).
pub fn rest_jets(heat: f32, locked: bool, resting: bool) -> bool {
    if locked || heat >= JET_REST_AT {
        true
    } else if resting {
        heat > JET_RELIGHT_AT
    } else {
        false
    }
}

/// The interaction a `UseOn` beat names.
pub fn interaction(name: &str) -> Option<crate::interact::Interaction> {
    use crate::interact::Interaction as I;
    Some(match name {
        "Greet" => I::Greet,
        "Terminal" => I::Terminal,
        "GetIn" => I::GetIn,
        "GetOut" => I::GetOut,
        "Board" => I::Board,
        _ => return None,
    })
}

/// The wheel notches that move a selection from `at` to `want` in a wrapping list of `len`, the short way round
/// (positive: down the list). Zero when it is already there or the list is empty.
pub fn notches_to(at: usize, want: usize, len: usize) -> i32 {
    if len == 0 || at >= len || want >= len {
        return 0;
    }
    let down = (want + len - at) % len;
    let up = len - down;
    if down == 0 {
        0
    } else if down <= up {
        down as i32
    } else {
        -(up as i32)
    }
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
                    Beat::ExpectRestored(m) | Beat::WalkTowardDrop(m) => !(m.is_finite() && *m > 0.0),
                    Beat::Tab(t) => !matches!(t.as_str(), "Jobs" | "Shop" | "Refit" | "Hangar"),
                    Beat::AcceptJob(t) | Beat::AcceptByMail(_, t) | Beat::ExpectOffer(t) => !(1..=crate::jobs::MAX_TIER).contains(t),
                    Beat::Grant(c) => *c <= 0,
                    Beat::SkateTo(x, z) => !(x.is_finite() && z.is_finite()),
                    Beat::JetTo(x, z, h) => !(x.is_finite() && z.is_finite() && h.is_finite() && *h > 0.0),
                    Beat::BuyMech(d) => !d.starts_with("mechs/"),
                    Beat::BuyPart(id) | Beat::Fit(_, id) => id.is_empty(),
                    Beat::WalkToPerson(m) | Beat::WalkToCar(m) | Beat::Survey(m) | Beat::WalkToMech(m) => !(m.is_finite() && *m > 0.0),
                    Beat::Scroll(n) => *n == 0,
                    Beat::UseOn(i) => !USE_ON.contains(&i.as_str()),
                    Beat::ExpectMode(m, s) => !(matches!(m.as_str(), "OnFoot" | "Driving" | "Seated") && s.is_finite() && *s > 0.0),
                    Beat::DriveCar(s, t) => !(s.is_finite() && *s > 0.0 && (0.0..=1.0).contains(t)),
                    Beat::WalkIntoBuilding(m, _) => !(m.is_finite() && *m > 0.0),
                    Beat::WalkIntoHouse(m) => !(m.is_finite() && *m > 0.0),
                    Beat::LookAtNearest(k, s) => !(LOOK_KINDS.contains(&k.as_str()) && s.is_finite() && *s > 0.0),
                    Beat::SkateToNearest(k, m) => !(GO_KINDS.contains(&k.as_str()) && m.is_finite() && *m >= 0.0),
                    Beat::JetToNearest(k, h) => !(GO_KINDS.contains(&k.as_str()) && h.is_finite() && *h > 0.0),
                    Beat::SelectWeapon(w) => w.is_empty(),
                    Beat::FireFor(s)
                    | Beat::MultiLock(s)
                    | Beat::LandOnFortress(s)
                    | Beat::BringDownFortress(s)
                    | Beat::ClearHostiles(s) => !(s.is_finite() && *s > 0.0),
                    Beat::Scope(s, _) => !(s.is_finite() && *s > 0.0),
                    Beat::CycleTargets(n) => *n == 0,
                    Beat::FireAtGround(m, n) => !(m.is_finite() && *m > 0.0 && *n > 0),
                    Beat::FireAtNearest(k, s, n) => !(matches!(k.as_str(), "building" | "house") && s.is_finite() && *s > 0.0 && *n > 0),
                    Beat::StartMission(m) => !matches!(m.as_str(), "Fortress" | "Defence"),
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
    fn every_use_on_name_is_an_interaction_and_nothing_else_is() {
        for n in USE_ON {
            assert!(interaction(n).is_some(), "{n}");
        }
        assert!(interaction("Fly").is_none());
        let text = r#"(name: "x", stuck_s: 10.0, runs: [(play: "free", beats: [UseOn("Fly")])])"#;
        assert!(crate::data::parse_ron::<Script>(text, std::path::Path::new("x.ron")).is_err());
        let text = r#"(name: "x", stuck_s: 10.0, runs: [(play: "free", beats: [UseOn("GetIn"), Scroll(-1), LookAtNearest("car", 2.0)])])"#;
        assert!(crate::data::parse_ron::<Script>(text, std::path::Path::new("x.ron")).is_ok());
    }

    #[test]
    fn a_long_jet_leg_rests_the_jets_before_they_lock_and_lights_them_again_once_recharged() {
        assert!(!rest_jets(0.5, false, false), "fresh jets fly");
        assert!(rest_jets(0.9, false, false), "nearly spent: come down before the lock-out");
        assert!(rest_jets(0.3, true, false), "locked out: down whatever the heat");
        assert!(rest_jets(0.5, false, true), "resting holds until they have recharged");
        assert!(!rest_jets(0.2, false, true), "recharged: up again");
    }

    #[test]
    fn the_wheel_goes_the_short_way_round_to_a_selection() {
        assert_eq!(notches_to(0, 1, 2), 1);
        assert_eq!(notches_to(1, 0, 2), 1, "two options: one notch down wraps");
        assert_eq!(notches_to(0, 2, 3), -1, "up one is shorter than down two");
        assert_eq!(notches_to(2, 2, 3), 0);
        assert_eq!(notches_to(0, 0, 0), 0);
        assert_eq!(notches_to(5, 0, 3), 0, "out of the list: nothing");
    }

    #[test]
    fn every_verification_script_loads() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/playtests");
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "ron") {
                let s: Script = crate::data::load_ron(&p).unwrap_or_else(|e| panic!("{e}"));
                assert_eq!(Some(s.name.as_str()), p.file_stem().and_then(|f| f.to_str()));
            }
        }
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

    #[test]
    fn hovering_puts_the_nozzle_12_degrees_up_from_150_m_off() {
        let y = bring_down_hover_y(530.0, BRING_DOWN_HOVER_STANDOFF_M, 20.0);
        let up = (530.0 - (y + 20.0)).atan2(BRING_DOWN_HOVER_STANDOFF_M).to_degrees();
        assert!((up - BRING_DOWN_HOVER_PITCH_DEG).abs() < 1e-3 && up < 25.0, "{y} m, {up} deg up");
    }

    #[test]
    fn tapping_the_jets_holds_a_height_and_lets_them_cool() {
        // a mech with the Foreman's jets (56 m/s2 up, 20 s burn, 8 s recharge after 1.5 s) holding 500 m from 450 m
        let (mut y, mut vy, mut heat, mut on, mut off_s, mut idle_s) = (450.0f32, 0.0f32, 0.3f32, false, 10.0f32, 10.0f32);
        let dt = 1.0 / 60.0;
        let mut lowest_after = f32::MAX;
        for i in 0..(120 * 60) {
            let want = tap_jets(on, 500.0 - y, vy, heat, off_s);
            if want != on {
                on = want;
                off_s = 0.0;
            }
            if on {
                vy += (56.0 - 9.81) * dt;
                heat = (heat + dt / 20.0).min(1.0);
                idle_s = 0.0;
            } else {
                vy -= 9.81 * dt;
                off_s += dt;
                idle_s += dt;
                if idle_s >= 1.5 {
                    heat = (heat - dt / 8.0).max(0.0);
                }
            }
            vy = vy.clamp(-60.0, 60.0);
            y += vy * dt;
            if i > 30 * 60 {
                lowest_after = lowest_after.min(y);
            }
            assert!(heat < 1.0, "never locks out: {heat} at {i}");
        }
        assert!(lowest_after > 400.0, "holds about its height: lowest {lowest_after}");
        assert!(heat < 0.8, "and the jets stay cool enough: {heat}");
    }
}
