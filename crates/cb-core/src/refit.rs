//! Refits (openspec change `mech-custom`): the weapons the career buys and keeps, fitted to a mech's stations in a
//! hangar bay; armour grades bought for a mech, each adding structure and mass (and the mass slows it); and liveries,
//! free. The parts and their prices are `data/career/parts.ron`. The game builds the mech it flies from its design's
//! data plus this fit (`fitted`), so the guns it fires and the mass it walks with are the ones bought here. Pure.

use crate::career::{Career, OwnedMech, Short};
use crate::data::{Validate, positive};
use crate::mech::MechConfig;
use crate::weapons::WeaponSpec;
use serde::Deserialize;

/// What a weapon station takes: an arm's gun, or the missile pod.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum StationKind {
    /// A gun carried on an arm (fire groups 0 and 1).
    Arm,
    /// A missile launcher on the torso (fire group 2).
    Missile,
}

impl StationKind {
    /// The kind of station a design's stock weapon stands on: the missile group is the pod, any other an arm.
    pub fn of(stock: &WeaponSpec) -> Self {
        if stock.group == 2 { Self::Missile } else { Self::Arm }
    }
}

/// A weapon for sale; fitted, it takes its station's socket, arm and fire group.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartWeapon {
    /// Stable id, saved in the career.
    pub id: String,
    /// The station it fits.
    pub kind: StationKind,
    /// Price, credits.
    pub price: i64,
    /// Display name.
    pub name: String,
    /// Muzzle speed, m/s.
    pub speed_m_s: f32,
    /// Least time between shots, s.
    pub interval_s: f32,
    /// Armour it beats, mm.
    pub penetration_mm: f32,
    /// Structure points it takes.
    pub damage: f32,
    /// Explosive energy, MJ.
    pub blast_mj: f32,
    /// What it throws (`weapon-kinds`): a shell when not given.
    #[serde(default)]
    pub shot: crate::weapons::ShotKind,
    /// The old spelling of `shot: Rocket`.
    #[serde(default)]
    pub rocket: bool,
    /// Share of the weapon heat bank a shot (a beam's second) adds.
    #[serde(default)]
    pub heat: f32,
    /// A missile's seeker turn, degrees a second.
    #[serde(default)]
    pub seeker_turn_deg_s: f32,
    /// A multi-lock pod's seeker (`multi-lock`).
    #[serde(default)]
    pub seeker: Option<crate::targeting::Seeker>,
    /// Rounds a sortie; `None` for no limit.
    #[serde(default)]
    pub rounds: Option<u32>,
}

impl PartWeapon {
    /// This weapon on a station whose stock weapon is `stock` (its socket, arm and group).
    pub fn on(&self, stock: &WeaponSpec) -> WeaponSpec {
        WeaponSpec {
            name: self.name.clone(),
            socket: stock.socket.clone(),
            arm: stock.arm,
            group: stock.group,
            speed_m_s: self.speed_m_s,
            interval_s: self.interval_s,
            penetration_mm: self.penetration_mm,
            damage: self.damage,
            blast_mj: self.blast_mj,
            kind: self.shot,
            rocket: self.rocket,
            heat: self.heat,
            seeker_turn_deg_s: self.seeker_turn_deg_s,
            seeker: self.seeker,
            burst: None,
            lob: None,
            signature: false,
            rounds: self.rounds,
        }
    }
}

/// An armour grade a mech can be raised to.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmourGrade {
    /// The grade, 1 upward, one at a time.
    pub grade: u8,
    /// Price of raising a mech to it from the grade below, credits.
    pub price: i64,
    /// Every part's structure rises by this share of its design's (0.15: 15 % more), at this grade.
    pub structure_share: f32,
    /// Mass added at this grade, kg.
    pub mass_kg: f32,
}

/// A livery: a tint over the mech's paint, free (MC2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Livery {
    /// Display name.
    pub name: String,
    /// Linear RGB multiplied over the paint (white is the factory livery).
    pub tint: [f32; 3],
}

/// `data/career/parts.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Parts {
    /// Weapons for sale.
    pub weapons: Vec<PartWeapon>,
    /// Armour grades, lowest first.
    pub armour: Vec<ArmourGrade>,
    /// Liveries; the first is the factory one.
    pub liveries: Vec<Livery>,
    /// Share of a part's price paid back when it is sold (MC1: half).
    pub sell_share: f32,
}

impl Parts {
    /// A weapon by id.
    pub fn weapon(&self, id: &str) -> Option<&PartWeapon> {
        self.weapons.iter().find(|w| w.id == id)
    }

    /// An armour grade.
    pub fn grade(&self, grade: u8) -> Option<&ArmourGrade> {
        self.armour.iter().find(|a| a.grade == grade)
    }
}

impl Validate for Parts {
    fn validate(&self) -> Result<(), String> {
        for (i, w) in self.weapons.iter().enumerate() {
            let f = |n: &str| format!("weapons[{i}].{n}");
            if w.id.is_empty() || self.weapons[..i].iter().any(|o| o.id == w.id) {
                return Err(format!("{}: empty or twice", f("id")));
            }
            if w.price <= 0 {
                return Err(format!("{}: must be above zero", f("price")));
            }
            positive(&f("speed_m_s"), w.speed_m_s)?;
            positive(&f("interval_s"), w.interval_s)?;
            positive(&f("penetration_mm"), w.penetration_mm)?;
            positive(&f("damage"), w.damage)?;
            crate::data::finite(&f("blast_mj"), w.blast_mj)?;
        }
        for (i, a) in self.armour.iter().enumerate() {
            if a.grade as usize != i + 1 {
                return Err(format!("armour[{i}].grade: grades run 1, 2, 3 in order"));
            }
            if a.price <= 0 {
                return Err(format!("armour[{i}].price: must be above zero"));
            }
            positive(&format!("armour[{i}].structure_share"), a.structure_share)?;
            positive(&format!("armour[{i}].mass_kg"), a.mass_kg)?;
        }
        if self.liveries.is_empty() {
            return Err("liveries: none".into());
        }
        for (i, l) in self.liveries.iter().enumerate() {
            if l.tint.iter().any(|c| !(0.0..=1.0).contains(c)) {
                return Err(format!("liveries[{i}].tint: each channel 0 to 1"));
            }
        }
        crate::data::within("sell_share", self.sell_share, 0.0, 1.0)
    }
}

/// Why a refit was refused; nothing changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefitError {
    /// The money is short.
    Short(i64),
    /// No such part, mech, station, grade or livery.
    Unknown,
    /// The station does not take that kind of weapon.
    WrongKind,
    /// The career does not have one of that part spare.
    NotOwned,
    /// The mech is at its highest armour grade.
    ArmourMax,
}

impl From<Short> for RefitError {
    fn from(s: Short) -> Self {
        RefitError::Short(s.missing)
    }
}

impl RefitError {
    /// What the refit page says.
    pub fn text(&self) -> String {
        match self {
            RefitError::Short(n) => format!("SHORT {n}"),
            RefitError::Unknown => "NOT AVAILABLE".into(),
            RefitError::WrongKind => "WRONG STATION".into(),
            RefitError::NotOwned => "NONE SPARE".into(),
            RefitError::ArmourMax => "ARMOUR AT ITS LIMIT".into(),
        }
    }
}

/// Buys a weapon into the career's spares.
pub fn buy_part(career: &mut Career, parts: &Parts, id: &str, kept: usize) -> Result<(), RefitError> {
    let w = parts.weapon(id).ok_or(RefitError::Unknown)?;
    career.charge(w.price, &format!("bought {}", w.name), kept)?;
    career.parts.push(id.to_string());
    Ok(())
}

/// Sells a spare weapon back at `sell_share` of its price.
pub fn sell_part(career: &mut Career, parts: &Parts, id: &str, kept: usize) -> Result<(), RefitError> {
    let w = parts.weapon(id).ok_or(RefitError::Unknown)?;
    let at = career.parts.iter().position(|p| p == id).ok_or(RefitError::NotOwned)?;
    career.parts.remove(at);
    let back = (w.price as f32 * parts.sell_share).round() as i64;
    career.pay(back, &format!("sold {}", w.name), kept);
    Ok(())
}

/// Fits a spare weapon (`Some(id)`) to station `station` of the mech `key`, or puts the stock weapon back (`None`).
/// The weapon taken off goes back to the spares (a stock weapon stays with its mech). `stock` is the design's own
/// weapons.
pub fn fit(career: &mut Career, parts: &Parts, key: u32, station: usize, id: Option<&str>, stock: &[WeaponSpec]) -> Result<(), RefitError> {
    let st = stock.get(station).ok_or(RefitError::Unknown)?;
    if let Some(id) = id {
        let w = parts.weapon(id).ok_or(RefitError::Unknown)?;
        if w.kind != StationKind::of(st) {
            return Err(RefitError::WrongKind);
        }
        if !career.parts.iter().any(|p| p == id) {
            return Err(RefitError::NotOwned);
        }
    }
    career.mech(key).ok_or(RefitError::Unknown)?;
    if let Some(id) = id
        && let Some(at) = career.parts.iter().position(|p| p == id)
    {
        career.parts.remove(at);
    }
    let m = career.mech_mut(key).ok_or(RefitError::Unknown)?;
    if m.fit.len() < stock.len() {
        m.fit.resize(stock.len(), None);
    }
    let off = std::mem::replace(&mut m.fit[station], id.map(str::to_string));
    if let Some(off) = off {
        career.parts.push(off);
    }
    Ok(())
}

/// Raises the mech `key` one armour grade, up to `armour_max`, for the next grade's price.
pub fn raise_armour(career: &mut Career, parts: &Parts, key: u32, armour_max: u8, kept: usize) -> Result<u8, RefitError> {
    let now = career.mech(key).ok_or(RefitError::Unknown)?.armour;
    if now >= armour_max {
        return Err(RefitError::ArmourMax);
    }
    let next = parts.grade(now + 1).ok_or(RefitError::ArmourMax)?;
    career.charge(next.price, &format!("armour grade {}", next.grade), kept)?;
    if let Some(m) = career.mech_mut(key) {
        m.armour = next.grade;
    }
    Ok(next.grade)
}

/// Paints the mech `key` in livery `livery`, free.
pub fn paint(career: &mut Career, parts: &Parts, key: u32, livery: u32) -> Result<(), RefitError> {
    if livery as usize >= parts.liveries.len() {
        return Err(RefitError::Unknown);
    }
    career.mech_mut(key).ok_or(RefitError::Unknown)?.livery = livery;
    Ok(())
}

/// The mech the game flies: its design's data with the career's fit on it. Each fitted station fires the bought
/// weapon; the armour grade raises every part's structure by its share and adds its mass, and the walk's top
/// speed and acceleration fall with the mass (the same push moving more mass).
pub fn fitted(design: &MechConfig, owned: &OwnedMech, parts: &Parts) -> MechConfig {
    let mut c = design.clone();
    for (i, slot) in owned.fit.iter().enumerate() {
        if let (Some(id), Some(stock)) = (slot, design.weapons.get(i))
            && let Some(w) = parts.weapon(id)
        {
            c.weapons[i] = w.on(stock);
        }
    }
    if let Some(g) = parts.grade(owned.armour) {
        for p in &mut c.parts {
            p.structure *= 1.0 + g.structure_share;
        }
        let m0 = design.locomotion.mass_kg.max(1.0);
        let m1 = m0 + g.mass_kg;
        c.locomotion.mass_kg = m1;
        c.locomotion.walk_speed_max_m_s *= m0 / m1;
        c.locomotion.accel_m_s2 *= m0 / m1;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::CareerConfig;

    fn parts() -> Parts {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/parts.ron");
        crate::data::load_ron(&p).unwrap()
    }

    fn design() -> MechConfig {
        MechConfig::default()
    }

    fn career(money: i64) -> Career {
        let whole: Vec<f32> = design().parts.iter().map(|p| p.structure).collect();
        let cfg = CareerConfig {
            start_money: money,
            ..CareerConfig::default()
        };
        Career::new("t", 1, &cfg, &whole)
    }

    #[test]
    fn a_bought_40_mm_cannon_fitted_to_the_right_arm_is_what_it_fires() {
        let mut c = career(1_000_000);
        let key = c.mechs[0].key;
        let stock = design().weapons;
        let right = stock.iter().position(|w| w.socket == "MUZZLE_R").unwrap();
        assert_eq!(
            fit(&mut c, &parts(), key, right, Some("cannon_40"), &stock),
            Err(RefitError::NotOwned)
        );
        buy_part(&mut c, &parts(), "cannon_40", 200).unwrap();
        fit(&mut c, &parts(), key, right, Some("cannon_40"), &stock).unwrap();
        assert!(c.parts.is_empty(), "the spare is on the mech now");
        let flown = fitted(&design(), c.mech(key).unwrap(), &parts());
        assert_eq!(flown.weapons[right].name, "40 mm cannon");
        assert_eq!(flown.weapons[right].socket, "MUZZLE_R", "it keeps the station's muzzle");
        assert_eq!(flown.weapons[right].group, stock[right].group);
    }

    #[test]
    fn a_station_takes_only_its_kind_and_a_weapon_taken_off_goes_back_to_the_spares() {
        let mut c = career(1_000_000);
        let key = c.mechs[0].key;
        let stock = design().weapons;
        let pod = stock.iter().position(|w| w.group == 2).unwrap();
        buy_part(&mut c, &parts(), "cannon_40", 200).unwrap();
        let before = c.clone();
        assert_eq!(
            fit(&mut c, &parts(), key, pod, Some("cannon_40"), &stock),
            Err(RefitError::WrongKind)
        );
        assert_eq!(c, before, "a refused fit changes nothing");
        fit(&mut c, &parts(), key, 0, Some("cannon_40"), &stock).unwrap();
        fit(&mut c, &parts(), key, 0, None, &stock).unwrap();
        assert_eq!(c.parts, vec!["cannon_40".to_string()], "back to stock: the cannon is spare again");
        assert_eq!(fitted(&design(), c.mech(key).unwrap(), &parts()).weapons, stock);
    }

    #[test]
    fn armour_adds_structure_and_mass_and_slows_the_mech_up_to_its_limit() {
        let mut c = career(10_000_000);
        let key = c.mechs[0].key;
        assert_eq!(raise_armour(&mut c, &parts(), key, 1, 200), Ok(1));
        assert_eq!(raise_armour(&mut c, &parts(), key, 1, 200), Err(RefitError::ArmourMax));
        let g = parts().grade(1).unwrap().clone();
        let d = design();
        let flown = fitted(&d, c.mech(key).unwrap(), &parts());
        assert!((flown.parts[0].structure - d.parts[0].structure * (1.0 + g.structure_share)).abs() < 1e-3);
        assert_eq!(flown.locomotion.mass_kg, d.locomotion.mass_kg + g.mass_kg);
        assert!(flown.locomotion.walk_speed_max_m_s < d.locomotion.walk_speed_max_m_s);
        assert_eq!(c.money, 10_000_000 - g.price);
    }

    #[test]
    fn a_refit_the_money_cannot_cover_changes_nothing() {
        let mut c = career(10);
        let before = c.clone();
        assert!(matches!(buy_part(&mut c, &parts(), "cannon_40", 200), Err(RefitError::Short(_))));
        let key = c.mechs[0].key;
        assert!(matches!(raise_armour(&mut c, &parts(), key, 3, 200), Err(RefitError::Short(_))));
        assert_eq!(c, before);
    }

    #[test]
    fn liveries_are_free_and_parts_sell_back_at_half() {
        let mut c = career(1_000_000);
        let key = c.mechs[0].key;
        paint(&mut c, &parts(), key, 2).unwrap();
        assert_eq!(c.mech(key).unwrap().livery, 2);
        assert_eq!(c.money, 1_000_000, "free");
        assert_eq!(paint(&mut c, &parts(), key, 999), Err(RefitError::Unknown));
        let price = parts().weapon("cannon_40").unwrap().price;
        buy_part(&mut c, &parts(), "cannon_40", 200).unwrap();
        sell_part(&mut c, &parts(), "cannon_40", 200).unwrap();
        assert_eq!(c.money, 1_000_000 - price + price / 2);
        let back = Career::from_ron(&c.to_ron()).unwrap();
        assert_eq!(back, c, "the fit, the livery and the spares are saved");
    }
}
