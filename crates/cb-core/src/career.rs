//! The pilot's career (openspec change `career`): money, the mechs owned and the damage each carries,
//! the base the pilot stands at, the accepted job and the jobs done, and the ledger every payment and
//! charge goes through. Pure; the game writes it to `saves/<name>.ron` and reads it back.

use crate::data::{Validate, positive};
use crate::mech::Damage;
use serde::{Deserialize, Serialize};

/// The save format's version; an older one is upgraded or refused.
pub const VERSION: u32 = 1;

/// How a career starts, in `data/career/career.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CareerConfig {
    /// Credits a new career starts with (owner, 2026-10-25, survey J4: "start with 1000 credits").
    pub start_money: i64,
    /// The mech a new career owns, relative to `data/`.
    pub start_mech: String,
    /// The newest ledger entries kept in a save.
    pub ledger_kept: usize,
}

impl Default for CareerConfig {
    fn default() -> Self {
        Self {
            start_money: 1_000,
            start_mech: "mechs/foreman.ron".into(),
            ledger_kept: 200,
        }
    }
}

impl Validate for CareerConfig {
    fn validate(&self) -> Result<(), String> {
        if self.start_money < 0 {
            return Err("start_money: must not be negative".into());
        }
        if self.start_mech.is_empty() {
            return Err("start_mech: must name a mech file".into());
        }
        positive("ledger_kept", self.ledger_kept as f32)
    }
}

/// A mech the pilot owns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnedMech {
    /// Stable key, in purchase order.
    pub key: u32,
    /// Its design's data file, relative to `data/` (`mechs/foreman.ron`).
    pub design: String,
    /// The base it is parked at (air-bases), and its bay there.
    pub base: u32,
    /// Its bay at that base.
    pub bay: u32,
    /// Its damage, part by part, exactly as the mech carries it (`mech-damage`).
    pub structure: Vec<f32>,
    /// Parts lost.
    pub lost: Vec<bool>,
}

impl OwnedMech {
    /// The damage this mech carries.
    pub fn damage(&self) -> Damage {
        Damage {
            structure: self.structure.clone(),
            lost: self.lost.clone(),
        }
    }

    /// Takes on a mech's damage (when it lands, or when it is written back for a save).
    pub fn set_damage(&mut self, d: &Damage) {
        self.structure = d.structure.clone();
        self.lost = d.lost.clone();
    }
}

/// A line of the ledger: what moved and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Career time, seconds.
    pub at_s: f64,
    /// Credits: positive paid in, negative charged.
    pub amount: i64,
    /// What for.
    pub what: String,
}

/// A charge the money does not cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Short {
    /// How much more is needed.
    pub missing: i64,
}

/// A finished job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    /// The job's id (mission-jobs).
    pub job: u32,
    /// Won or not.
    pub won: bool,
    /// Credits paid for it.
    pub paid: i64,
}

/// The pilot's career.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Career {
    /// The save format's version.
    pub version: u32,
    /// The save's name.
    pub name: String,
    /// Credits; never below zero.
    pub money: i64,
    /// Mechs owned, in purchase order.
    pub mechs: Vec<OwnedMech>,
    /// The key of the mech piloted or last boarded.
    pub piloting: Option<u32>,
    /// The base the pilot is at or last left.
    pub base: u32,
    /// Career time, seconds of play.
    pub time_s: f64,
    /// Seeds the job generator and the mail (mission-jobs).
    pub seed: u64,
    /// Finished jobs, newest last.
    pub done: Vec<JobRecord>,
    /// Every credit and charge, newest last (the newest `ledger_kept`).
    pub ledger: Vec<Entry>,
    /// The jobs offered, taken and done, and the inbox (mission-jobs).
    pub board: crate::jobs::Board,
    next_key: u32,
}

impl Career {
    /// A new career: the starting money, and the starting mech whole in bay 0 of base 0.
    pub fn new(name: &str, seed: u64, cfg: &CareerConfig, start_mech_structure: &[f32]) -> Self {
        let mut c = Self {
            version: VERSION,
            name: name.to_string(),
            money: cfg.start_money,
            mechs: Vec::new(),
            piloting: None,
            base: 0,
            time_s: 0.0,
            seed,
            done: Vec::new(),
            ledger: Vec::new(),
            board: crate::jobs::Board::default(),
            next_key: 0,
        };
        let key = c.add_mech(&cfg.start_mech, 0, 0, start_mech_structure);
        c.piloting = Some(key);
        c
    }

    /// Adds a mech at full structure; returns its key.
    pub fn add_mech(&mut self, design: &str, base: u32, bay: u32, structure: &[f32]) -> u32 {
        let key = self.next_key;
        self.next_key += 1;
        self.mechs.push(OwnedMech {
            key,
            design: design.to_string(),
            base,
            bay,
            structure: structure.to_vec(),
            lost: vec![false; structure.len()],
        });
        key
    }

    /// The mech with this key.
    pub fn mech(&self, key: u32) -> Option<&OwnedMech> {
        self.mechs.iter().find(|m| m.key == key)
    }

    /// The mech with this key, to change.
    pub fn mech_mut(&mut self, key: u32) -> Option<&mut OwnedMech> {
        self.mechs.iter_mut().find(|m| m.key == key)
    }

    /// Pays `amount` credits in, for `what`. Non-positive or overflowing amounts change nothing.
    pub fn pay(&mut self, amount: i64, what: &str, kept: usize) -> bool {
        let Some(total) = (amount > 0).then(|| self.money.checked_add(amount)).flatten() else {
            return false;
        };
        self.money = total;
        self.record(amount, what, kept);
        true
    }

    /// Charges `amount` credits for `what`. A charge larger than the money changes nothing and says
    /// how much is missing; a non-positive one changes nothing.
    pub fn charge(&mut self, amount: i64, what: &str, kept: usize) -> Result<(), Short> {
        if amount <= 0 {
            return Ok(());
        }
        if amount > self.money {
            return Err(Short {
                missing: amount - self.money,
            });
        }
        self.money -= amount;
        self.record(-amount, what, kept);
        Ok(())
    }

    fn record(&mut self, amount: i64, what: &str, kept: usize) {
        self.ledger.push(Entry {
            at_s: self.time_s,
            amount,
            what: what.to_string(),
        });
        if self.ledger.len() > kept {
            let drop = self.ledger.len() - kept;
            self.ledger.drain(..drop);
        }
    }

    /// The career as the text of its save file.
    pub fn to_ron(&self) -> String {
        let body = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default();
        format!("// A Crab Breeze career (openspec change career), written by the game.\n{body}\n")
    }

    /// A career read from its save file's text; a newer version or a bad file is refused with why.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let c: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        if c.version > VERSION {
            return Err(format!("save version {} is newer than this game's {VERSION}", c.version));
        }
        if c.money < 0 {
            return Err("money: negative".into());
        }
        if c.mechs
            .iter()
            .any(|m| m.structure.len() != m.lost.len() || m.structure.iter().any(|s| !s.is_finite()))
        {
            return Err("mechs: damage does not match its parts".into());
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn career() -> Career {
        Career::new("test", 7, &CareerConfig::default(), &[80.0, 80.0, 140.0, 140.0, 40.0, 200.0])
    }

    #[test]
    fn a_new_career_starts_with_its_money_and_one_whole_mech() {
        let c = career();
        assert_eq!(c.money, 1_000);
        assert_eq!(c.mechs.len(), 1);
        assert_eq!(c.piloting, Some(c.mechs[0].key));
        assert!(c.mechs[0].lost.iter().all(|l| !l));
    }

    #[test]
    fn a_charge_you_cannot_afford_changes_nothing() {
        let mut c = career();
        assert_eq!(c.charge(1_500, "repair", 200), Err(Short { missing: 500 }));
        assert_eq!(c.money, 1_000);
        assert!(c.ledger.is_empty());
    }

    #[test]
    fn pay_and_charge_keep_exact_totals_and_a_ledger() {
        let mut c = career();
        assert!(c.pay(45_000, "job 1", 200));
        assert_eq!(c.charge(12_345, "repair", 200), Ok(()));
        assert_eq!(c.money, 1_000 + 45_000 - 12_345);
        assert_eq!(c.ledger.iter().map(|e| e.amount).sum::<i64>(), 45_000 - 12_345);
        assert!(!c.pay(0, "nothing", 200));
        assert!(!c.pay(i64::MAX, "too much", 200), "an overflow changes nothing");
    }

    #[test]
    fn the_ledger_keeps_only_its_newest_lines() {
        let mut c = career();
        for i in 0..10 {
            c.pay(1, &format!("{i}"), 4);
        }
        assert_eq!(c.ledger.len(), 4);
        assert_eq!(c.ledger[0].what, "6");
    }

    #[test]
    fn a_career_survives_a_save_and_load_with_its_damage() {
        let mut c = career();
        let key = c.mechs[0].key;
        let mut d = c.mechs[0].damage();
        d.lost[0] = true;
        d.structure[0] = 0.0;
        c.mech_mut(key).unwrap().set_damage(&d);
        c.pay(2_000, "job", 200);
        let back = Career::from_ron(&c.to_ron()).unwrap();
        assert_eq!(back, c);
        assert!(back.mech(key).unwrap().lost[0], "the left arm is still lost");
    }

    #[test]
    fn a_save_from_a_newer_game_is_refused() {
        let mut c = career();
        c.version = VERSION + 1;
        assert!(Career::from_ron(&c.to_ron()).is_err());
    }

    #[test]
    fn the_shipped_career_data_loads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/career.ron");
        let cfg: CareerConfig = crate::data::load_ron(&root).unwrap();
        assert_eq!(cfg.start_money, 1_000);
    }
}
