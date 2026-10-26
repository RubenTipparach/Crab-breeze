//! The pilot's career (openspec change `career`): money, the mechs owned and the damage each carries,
//! the base the pilot stands at, the accepted job and the jobs done, and the ledger every payment and
//! charge goes through; where each mech stands in the field, the air drops on their way (mech-shop section 6), and
//! calling a parked mech out at a base (owner, survey J16). Pure; the game writes it to `saves/<name>.ron` and reads
//! it back.

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
    /// Standing gained with the offering base for a job won (career-progression).
    pub standing_per_job: i64,
    /// Standing lost with it for a job failed or abandoned.
    pub standing_lost: i64,
    /// The most standing a base keeps for the pilot.
    pub standing_max: i64,
    /// The share a base's pay rises per point of standing there (0.05: 5 % a point).
    pub pay_bonus_per_standing: f32,
}

impl Default for CareerConfig {
    fn default() -> Self {
        Self {
            start_money: 1_000,
            start_mech: "mechs/foreman.ron".into(),
            ledger_kept: 200,
            standing_per_job: 1,
            standing_lost: 1,
            standing_max: 10,
            pay_bonus_per_standing: 0.05,
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
        if self.standing_per_job < 0 || self.standing_lost < 0 || self.standing_max < 0 {
            return Err("standing: gains, losses and the most must not be negative".into());
        }
        crate::data::within("pay_bonus_per_standing", self.pay_bonus_per_standing, 0.0, 1.0)?;
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
    /// Each weapon station's bought weapon (`mech-custom`, a `parts.ron` id), or `None` for the design's stock one.
    #[serde(default)]
    pub fit: Vec<Option<String>>,
    /// Its armour grade (0: as built).
    #[serde(default)]
    pub armour: u8,
    /// Its livery, an index into `parts.ron`'s liveries (0: the factory one).
    #[serde(default)]
    pub livery: u32,
    /// Where it stands in the world, when it stands in the world rather than waiting unseen in a bay: dropped near
    /// the pilot, left where they climbed out, or called out at a base. `base` and `bay` are only where it was
    /// bought or last parked while this is set.
    #[serde(default)]
    pub field: Option<Field>,
}

/// Where a mech stands in the world.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Its feet, world metres.
    pub pos: [f32; 3],
    /// Its heading, radians (0 faces -z).
    pub heading_rad: f32,
}

/// Why an owned mech cannot be called out at a base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallRefusal {
    /// No owned mech has that key.
    NotOwned,
    /// It is the mech the pilot is using.
    InUse,
    /// It is not parked at this base (and is not the Foreman, which always comes).
    NotHere,
}

impl CallRefusal {
    /// What the terminal says.
    pub fn text(self) -> String {
        match self {
            CallRefusal::NotOwned => "NOT YOURS".into(),
            CallRefusal::InUse => "ALREADY IN USE".into(),
            CallRefusal::NotHere => "NOT PARKED AT THIS BASE".into(),
        }
    }
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
    /// Where the player was when the career was last saved (save-game); none in a new career and in
    /// saves from before it.
    #[serde(default)]
    pub place: Option<crate::save::Place>,
    /// The pilot's standing with each base, by base id (career-progression); never below zero. It raises
    /// that base's pay and never closes a base to the pilot (CP2).
    #[serde(default)]
    pub standing: std::collections::BTreeMap<u32, i64>,
    /// Weapons bought and not fitted to any mech, by `parts.ron` id (mech-custom).
    #[serde(default)]
    pub parts: Vec<String>,
    /// Air drops on their way, or landed and not yet boarded (mech-shop section 6).
    #[serde(default)]
    pub drops: Vec<crate::air_drop::Drop>,
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
            place: None,
            standing: Default::default(),
            parts: Vec::new(),
            drops: Vec::new(),
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
            fit: Vec::new(),
            armour: 0,
            livery: 0,
            field: None,
        });
        key
    }

    /// Buys into the field: a mech at full structure dropped at `drop`'s site, the drop on the career's clock.
    pub fn add_dropped_mech(&mut self, design: &str, base: u32, structure: &[f32], site: glam::Vec3, rng: &crate::rng::SimRng) -> u32 {
        let key = self.add_mech(design, base, 0, structure);
        let drop = crate::air_drop::Drop::new(key, site, self.time_s, rng);
        if let Some(m) = self.mech_mut(key) {
            m.field = Some(Field {
                pos: site.to_array(),
                heading_rad: drop.track_rad,
            });
        }
        self.drops.push(drop);
        key
    }

    /// The drop of mech `key`, if one is on its way or waiting to be boarded.
    pub fn drop_of(&self, key: u32) -> Option<&crate::air_drop::Drop> {
        self.drops.iter().find(|d| d.key == key)
    }

    /// Mech `key` touched down at `field`: it stands there, and its drop waits for the pilot to board it.
    pub fn landed(&mut self, key: u32, field: Field) {
        if let Some(m) = self.mech_mut(key) {
            m.field = Some(field);
        }
        if let Some(d) = self.drops.iter_mut().find(|d| d.key == key) {
            d.landed = true;
            d.site = field.pos;
        }
    }

    /// The pilot boarded mech `key` (it is the one they now pilot): a drop of it is done.
    pub fn boarded(&mut self, key: u32) {
        if self.mech(key).is_none() {
            return;
        }
        self.piloting = Some(key);
        self.drops.retain(|d| d.key != key);
    }

    /// Calls owned mech `key` out at base `base` into its bay `bay`, standing at `at` (owner, survey J16: "you can
    /// swap mechs out at base, with foreman always availible"): a mech parked at this base (standing on its site,
    /// `on_site`, or waiting unseen in one of its bays) comes out; the career's starting mech (`start_mech`, the
    /// Foreman) comes from anywhere, and comes back whole (`whole`, its parts' full structure) when it was wrecked.
    /// The mech in use cannot be called. Returns whether it was made whole.
    #[allow(clippy::too_many_arguments)]
    pub fn call_out(
        &mut self,
        key: u32,
        base: u32,
        bay: u32,
        at: Field,
        on_site: &dyn Fn(glam::Vec2) -> bool,
        start_mech: &str,
        wrecked: bool,
        whole: &[f32],
    ) -> Result<bool, CallRefusal> {
        if self.piloting == Some(key) {
            return Err(CallRefusal::InUse);
        }
        let m = self.mech(key).ok_or(CallRefusal::NotOwned)?;
        let foreman = m.design == start_mech;
        let here = match m.field {
            Some(f) => on_site(glam::Vec2::new(f.pos[0], f.pos[2])),
            None => m.base == base,
        };
        if !here && !foreman {
            return Err(CallRefusal::NotHere);
        }
        let repair = foreman && wrecked;
        let m = self.mech_mut(key).ok_or(CallRefusal::NotOwned)?;
        m.field = Some(at);
        m.base = base;
        m.bay = bay;
        if repair {
            m.structure = whole.to_vec();
            m.lost = vec![false; whole.len()];
        }
        self.drops.retain(|d| d.key != key);
        Ok(repair)
    }

    /// The pilot's standing with `base`.
    pub fn standing_at(&self, base: u32) -> i64 {
        self.standing.get(&base).copied().unwrap_or(0)
    }

    /// A job offered by `base` ended: won adds standing there, failed or abandoned takes some away, within
    /// 0 and the most. Returns the standing after.
    pub fn job_ended(&mut self, base: u32, won: bool, cfg: &CareerConfig) -> i64 {
        let now = self.standing_at(base);
        let next = if won { now + cfg.standing_per_job } else { now - cfg.standing_lost }.clamp(0, cfg.standing_max);
        self.standing.insert(base, next);
        next
    }

    /// Each base's pay bonus from the standing there, by id, for `bases` bases.
    pub fn pay_bonuses(&self, bases: usize, cfg: &CareerConfig) -> Vec<f32> {
        (0..bases as u32)
            .map(|b| self.standing_at(b) as f32 * cfg.pay_bonus_per_standing)
            .collect()
    }

    /// The riskiest job tier the owned mechs open: the highest `tier_of` any owned design (at least 1).
    pub fn max_tier(&self, tier_of: impl Fn(&str) -> Option<u8>) -> u8 {
        self.mechs.iter().filter_map(|m| tier_of(&m.design)).max().unwrap_or(1).max(1)
    }

    /// The lowest bay of `base` (of `per_base`) no owned mech is parked in, if any. A mech standing in the field
    /// holds no bay.
    pub fn free_bay(&self, base: u32, per_base: u32) -> Option<u32> {
        (0..per_base).find(|bay| !self.mechs.iter().any(|m| m.field.is_none() && m.base == base && m.bay == *bay))
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
        if let Some(p) = &c.place {
            p.check()?;
        }
        Ok(c)
    }
}

/// Everything a career reads from `data/`: how it starts, the jobs, the shop's catalogue, and each listed
/// design's mech data, loaded and checked together so a catalogue naming a missing mech stops startup (and
/// `cb-tools validate`).
#[derive(Clone, Debug)]
pub struct CareerData {
    /// `career/career.ron`.
    pub career: CareerConfig,
    /// `jobs/jobs.ron`.
    pub jobs: crate::jobs::JobsConfig,
    /// `career/catalogue.ron`.
    pub catalogue: crate::shop::Catalogue,
    /// Each listed design's data, by its file (`mechs/sapper.ron`), in catalogue order.
    pub designs: Vec<(String, crate::mech::MechConfig)>,
    /// `career/parts.ron`: weapons, armour grades and liveries for refits (mech-custom).
    pub parts: crate::refit::Parts,
    /// `career/air_drop.ron`: where a bought mech lands near the pilot and how it comes down (mech-shop section 6).
    pub drop: crate::air_drop::DropConfig,
}

impl CareerData {
    /// Loads and cross-checks the career's data under `root` (the `data/` directory).
    pub fn load(root: &std::path::Path) -> Result<Self, crate::data::DataError> {
        use crate::data::load_ron;
        let career: CareerConfig = load_ron(&root.join("career/career.ron"))?;
        let jobs: crate::jobs::JobsConfig = load_ron(&root.join("jobs/jobs.ron"))?;
        let cat_path = root.join("career/catalogue.ron");
        let catalogue: crate::shop::Catalogue = load_ron(&cat_path)?;
        let parts: crate::refit::Parts = load_ron(&root.join("career/parts.ron"))?;
        let drop: crate::air_drop::DropConfig = load_ron(&root.join("career/air_drop.ron"))?;
        if let Some(l) = catalogue
            .listings
            .iter()
            .find(|l| parts.grade(l.armour_max).is_none() && l.armour_max > 0)
        {
            return Err(crate::data::DataError {
                path: cat_path,
                reason: format!(
                    "listings: {} allows armour grade {}, which parts.ron does not price",
                    l.design, l.armour_max
                ),
            });
        }
        let mut designs = Vec::new();
        for l in &catalogue.listings {
            designs.push((l.design.clone(), load_ron(&root.join(&l.design))?));
        }
        if catalogue.listing(&career.start_mech).is_none() {
            return Err(crate::data::DataError {
                path: cat_path,
                reason: format!("listings: the starting mech {} is not listed", career.start_mech),
            });
        }
        Ok(Self {
            career,
            jobs,
            catalogue,
            designs,
            parts,
            drop,
        })
    }

    /// A listed design's data.
    pub fn design(&self, design: &str) -> Option<&crate::mech::MechConfig> {
        self.designs.iter().find(|(d, _)| d == design).map(|(_, c)| c)
    }

    /// A design's full structure, part by part (a mech as it is delivered).
    pub fn whole(&self, design: &str) -> Option<Vec<f32>> {
        self.design(design).map(|c| c.parts.iter().map(|p| p.structure).collect())
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
    fn standing_rises_with_jobs_won_falls_with_jobs_lost_and_raises_the_pay() {
        let mut c = career();
        let cfg = CareerConfig::default();
        assert_eq!(c.job_ended(2, true, &cfg), 1);
        assert_eq!(c.job_ended(2, true, &cfg), 2);
        assert_eq!(c.job_ended(2, false, &cfg), 1);
        assert_eq!(c.job_ended(0, false, &cfg), 0, "never below zero");
        for _ in 0..50 {
            c.job_ended(1, true, &cfg);
        }
        assert_eq!(c.standing_at(1), cfg.standing_max);
        let bonus = c.pay_bonuses(3, &cfg);
        assert_eq!(bonus[0], 0.0);
        assert!((bonus[2] - cfg.pay_bonus_per_standing).abs() < 1e-6);
        let back = Career::from_ron(&c.to_ron()).unwrap();
        assert_eq!(back.standing, c.standing, "standing is saved");
    }

    #[test]
    fn the_owned_mechs_open_their_tiers() {
        let mut c = career();
        let tier = |d: &str| match d {
            "mechs/foreman.ron" => Some(2),
            "mechs/sapper.ron" => Some(3),
            _ => None,
        };
        assert_eq!(c.max_tier(tier), 2, "the Foreman opens 1 and 2");
        c.add_mech("mechs/sapper.ron", 0, 1, &[1.0]);
        assert_eq!(c.max_tier(tier), 3);
    }

    #[test]
    fn a_mech_takes_the_lowest_bay_no_other_mech_holds() {
        let mut c = career();
        assert_eq!(c.free_bay(0, 3), Some(1), "the starting mech holds bay 0");
        c.add_mech("mechs/mule.ron", 0, 1, &[1.0]);
        c.add_mech("mechs/mule.ron", 0, 2, &[1.0]);
        assert_eq!(c.free_bay(0, 3), None);
        assert_eq!(c.free_bay(1, 3), Some(0));
    }

    fn spot(x: f32) -> Field {
        Field {
            pos: [x, 0.0, 0.0],
            heading_rad: 0.0,
        }
    }

    #[test]
    fn a_dropped_mech_stands_in_the_field_holds_no_bay_and_its_drop_lasts_until_it_is_boarded() {
        let mut c = career();
        let rng = crate::rng::SimRng::new(1);
        let key = c.add_dropped_mech("mechs/sapper.ron", 0, &[1.0, 2.0], glam::Vec3::new(90.0, 4.0, 10.0), &rng);
        assert_eq!(c.mech(key).unwrap().field.map(|f| f.pos), Some([90.0, 4.0, 10.0]));
        assert_eq!(c.free_bay(0, 3), Some(1), "the dropped mech holds no bay");
        assert!(c.drop_of(key).is_some_and(|d| !d.landed));
        c.landed(key, spot(95.0));
        assert!(
            c.drop_of(key).is_some_and(|d| d.landed),
            "landed, the marker stays until it is boarded"
        );
        let back = Career::from_ron(&c.to_ron()).unwrap();
        assert_eq!(back, c, "a save keeps it where it landed, and its drop");
        c.boarded(key);
        assert_eq!(c.piloting, Some(key));
        assert!(c.drop_of(key).is_none());
    }

    #[test]
    fn a_save_from_before_the_air_drop_still_loads() {
        let c = career();
        let old = c.to_ron().replace("    drops: [],\n", "").replace("            field: None,\n", "");
        assert!(!old.contains("drops"), "the old save has no drops");
        assert_eq!(Career::from_ron(&old).unwrap(), c);
    }

    #[test]
    fn at_a_base_a_parked_mech_is_called_out_and_one_parked_elsewhere_is_not() {
        let mut c = career();
        let on_base_0 = |p: glam::Vec2| p.x < 1000.0;
        let foreman = c.mechs[0].key;
        let here = c.add_mech("mechs/mule.ron", 0, 1, &[1.0]);
        let away = c.add_mech("mechs/picket.ron", 2, 0, &[1.0]);
        let dropped_here = c.add_mech("mechs/sapper.ron", 3, 0, &[1.0]);
        c.mech_mut(dropped_here).unwrap().field = Some(spot(200.0));
        let call = |c: &mut Career, k: u32| c.call_out(k, 0, 2, spot(5.0), &on_base_0, "mechs/foreman.ron", false, &[9.0]);
        assert_eq!(call(&mut c, foreman), Err(CallRefusal::InUse), "the Foreman is in use");
        assert_eq!(call(&mut c, here), Ok(false));
        assert_eq!(c.mech(here).unwrap().field, Some(spot(5.0)), "it stands in the bay");
        assert_eq!((c.mech(here).unwrap().base, c.mech(here).unwrap().bay), (0, 2));
        assert_eq!(call(&mut c, away), Err(CallRefusal::NotHere));
        assert_eq!(
            call(&mut c, dropped_here),
            Ok(false),
            "standing on this base's site counts as parked here"
        );
        assert_eq!(call(&mut c, 99), Err(CallRefusal::NotOwned));
        assert_eq!(CallRefusal::NotHere.text(), "NOT PARKED AT THIS BASE");
    }

    #[test]
    fn the_foreman_is_always_available_from_anywhere_and_comes_back_whole_when_wrecked() {
        let mut c = career();
        let foreman = c.mechs[0].key;
        let sapper = c.add_mech("mechs/sapper.ron", 0, 1, &[1.0]);
        c.boarded(sapper);
        // The Foreman wrecked far from this base.
        {
            let m = c.mech_mut(foreman).unwrap();
            m.field = Some(spot(50_000.0));
            m.structure = vec![0.0; 6];
            m.lost = vec![true; 6];
        }
        let whole = [80.0, 80.0, 140.0, 140.0, 40.0, 200.0];
        let r = c.call_out(foreman, 1, 0, spot(7.0), &|p| p.x < 1000.0, "mechs/foreman.ron", true, &whole);
        assert_eq!(r, Ok(true), "it comes, made whole");
        let m = c.mech(foreman).unwrap();
        assert_eq!(m.structure, whole.to_vec());
        assert!(m.lost.iter().all(|l| !l));
        assert_eq!(m.field, Some(spot(7.0)));
    }

    #[test]
    fn the_career_data_loads_with_every_listed_mech() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d = CareerData::load(&root).unwrap();
        assert_eq!(d.designs.len(), d.catalogue.listings.len());
        assert!(d.whole("mechs/foreman.ron").is_some_and(|w| !w.is_empty()));
    }

    #[test]
    fn the_shipped_career_data_loads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/career.ron");
        let cfg: CareerConfig = crate::data::load_ron(&root).unwrap();
        assert_eq!(cfg.start_money, 1_000);
    }
}
