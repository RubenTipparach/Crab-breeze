//! The mech shop (openspec change `mech-shop`, design section 2): every roster mech is for sale from the
//! start (survey J14) at its price in `data/career/catalogue.ron`, bought with job pay through the career's
//! ledger and delivered to a free hangar bay: the current base's when the pilot is at one, else the nearest
//! base with a free bay. A purchase the money or the bays cannot cover changes nothing. Pure.

use crate::career::Career;
use crate::data::Validate;
use glam::Vec2;
use serde::Deserialize;

/// One mech for sale.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// Its design's data file, relative to `data/` (`mechs/sapper.ron`).
    pub design: String,
    /// Its price, credits.
    pub price: i64,
    /// The highest armour grade a refit may give it (mech-custom).
    #[serde(default)]
    pub armour_max: u8,
}

/// The catalogue, `data/career/catalogue.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    /// The prices allowed, least and most, credits (survey J4: 300 000 to 1 200 000).
    pub price_range: (i64, i64),
    /// Every mech for sale, cheapest first.
    pub listings: Vec<Listing>,
}

impl Catalogue {
    /// The listing for a design.
    pub fn listing(&self, design: &str) -> Option<&Listing> {
        self.listings.iter().find(|l| l.design == design)
    }
}

impl Validate for Catalogue {
    fn validate(&self) -> Result<(), String> {
        let (lo, hi) = self.price_range;
        if lo <= 0 || hi < lo {
            return Err("price_range: least above zero, most not below least".into());
        }
        if self.listings.is_empty() {
            return Err("listings: none".into());
        }
        for (i, l) in self.listings.iter().enumerate() {
            if !(lo..=hi).contains(&l.price) {
                return Err(format!("listings[{i}].price: {} is outside {lo}-{hi}", l.price));
            }
            if self.listings[..i].iter().any(|o| o.design == l.design) {
                return Err(format!("listings[{i}].design: {} is listed twice", l.design));
            }
        }
        Ok(())
    }
}

/// Where the bays are: how many each base has, and each base's centre (x, z), by base id.
#[derive(Clone, Copy, Debug)]
pub struct Bays<'a> {
    /// Hangar bays at every base.
    pub per_base: u32,
    /// Each base's centre, by id.
    pub centres: &'a [Vec2],
}

/// Why a purchase was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The money is short by this many credits.
    Short(i64),
    /// No base has a free bay.
    NoFreeBay,
    /// Not in the catalogue.
    NotListed,
}

impl Refusal {
    /// What the shop's page says.
    pub fn text(self) -> String {
        match self {
            Refusal::Short(n) => format!("SHORT {n}"),
            Refusal::NoFreeBay => "NO FREE BAY".into(),
            Refusal::NotListed => "NOT FOR SALE".into(),
        }
    }
}

/// Where a bought mech was delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    /// The new mech's key in the career.
    pub key: u32,
    /// The base it waits at.
    pub base: u32,
    /// Its bay there.
    pub bay: u32,
    /// Delivered to a base the pilot is not at (a mail says where).
    pub away: bool,
}

/// The bay a purchase goes to: the lowest free bay of `at` (the base the pilot is at), else of the nearest
/// base to `from` with one (ties by base id).
pub fn delivery_bay(career: &Career, bays: &Bays, at: Option<u32>, from: Vec2) -> Option<(u32, u32)> {
    if let Some(base) = at
        && let Some(bay) = career.free_bay(base, bays.per_base)
    {
        return Some((base, bay));
    }
    let mut order: Vec<u32> = (0..bays.centres.len() as u32).collect();
    order.sort_by(|a, b| {
        let da = bays.centres[*a as usize].distance(from);
        let db = bays.centres[*b as usize].distance(from);
        da.total_cmp(&db).then(a.cmp(b))
    });
    order
        .into_iter()
        .find_map(|base| career.free_bay(base, bays.per_base).map(|bay| (base, bay)))
}

/// Buys `design`: finds its bay, charges its price through the ledger (`kept` ledger lines), and parks it
/// whole (`structure`, its parts' full structure) in that bay. Refused before anything changes when it is not
/// listed, no bay is free, or the money is short.
pub fn buy(
    career: &mut Career,
    catalogue: &Catalogue,
    bays: &Bays,
    design: &str,
    at: Option<u32>,
    from: Vec2,
    structure: &[f32],
    kept: usize,
) -> Result<Delivery, Refusal> {
    let listing = catalogue.listing(design).ok_or(Refusal::NotListed)?;
    let (base, bay) = delivery_bay(career, bays, at, from).ok_or(Refusal::NoFreeBay)?;
    let name = design.trim_start_matches("mechs/").trim_end_matches(".ron");
    career
        .charge(listing.price, &format!("bought {name}"), kept)
        .map_err(|s| Refusal::Short(s.missing))?;
    let key = career.add_mech(design, base, bay, structure);
    Ok(Delivery {
        key,
        base,
        bay,
        away: at != Some(base),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::CareerConfig;

    const WHOLE: [f32; 6] = [80.0, 80.0, 140.0, 140.0, 40.0, 200.0];
    const CENTRES: [Vec2; 3] = [Vec2::new(0.0, 0.0), Vec2::new(5000.0, 0.0), Vec2::new(-9000.0, 0.0)];

    fn bays() -> Bays<'static> {
        Bays {
            per_base: 3,
            centres: &CENTRES,
        }
    }

    fn catalogue() -> Catalogue {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/career/catalogue.ron");
        crate::data::load_ron(&p).unwrap()
    }

    fn career(money: i64) -> Career {
        let cfg = CareerConfig {
            start_money: money,
            ..CareerConfig::default()
        };
        Career::new("test", 1, &cfg, &WHOLE)
    }

    #[test]
    fn a_purchase_short_of_money_changes_nothing() {
        let mut c = career(1_000_000);
        let before = c.clone();
        let harrow = catalogue().listing("mechs/harrow.ron").unwrap().price;
        let r = buy(&mut c, &catalogue(), &bays(), "mechs/harrow.ron", Some(0), Vec2::ZERO, &WHOLE, 200);
        assert_eq!(r, Err(Refusal::Short(harrow - 1_000_000)));
        assert_eq!(Refusal::Short(200_000).text(), "SHORT 200000");
        assert_eq!(c, before, "money, bays and ledger unchanged");
    }

    #[test]
    fn with_no_free_bay_anywhere_nothing_is_charged() {
        let mut c = career(100_000_000);
        for base in 0..3 {
            for bay in 0..3 {
                if (base, bay) != (0, 0) {
                    c.add_mech("mechs/mule.ron", base, bay, &WHOLE);
                }
            }
        }
        let before = c.clone();
        let r = buy(&mut c, &catalogue(), &bays(), "mechs/mule.ron", Some(0), Vec2::ZERO, &WHOLE, 200);
        assert_eq!(r, Err(Refusal::NoFreeBay));
        assert_eq!(c, before);
    }

    #[test]
    fn at_a_base_the_mech_goes_to_its_lowest_free_bay_and_the_ledger_records_the_price() {
        let mut c = career(1_000_000);
        let price = catalogue().listing("mechs/picket.ron").unwrap().price;
        let d = buy(&mut c, &catalogue(), &bays(), "mechs/picket.ron", Some(0), Vec2::ZERO, &WHOLE, 200).unwrap();
        assert_eq!((d.base, d.bay, d.away), (0, 1, false), "bay 0 holds the starting mech");
        assert_eq!(c.money, 1_000_000 - price);
        assert_eq!(c.ledger.last().map(|e| e.amount), Some(-price));
        assert_eq!(c.mech(d.key).map(|m| m.design.as_str()), Some("mechs/picket.ron"));
    }

    #[test]
    fn in_the_field_it_goes_to_the_nearest_base_with_a_free_bay() {
        let mut c = career(10_000_000);
        for bay in 0..3 {
            c.add_mech("mechs/mule.ron", 1, bay, &WHOLE);
        }
        // Nearest is base 1 (full), then base 0.
        let d = buy(
            &mut c,
            &catalogue(),
            &bays(),
            "mechs/mule.ron",
            None,
            Vec2::new(4800.0, 0.0),
            &WHOLE,
            200,
        )
        .unwrap();
        assert_eq!((d.base, d.bay, d.away), (0, 1, true));
    }

    #[test]
    fn job_pay_buys_a_mech() {
        let mut c = career(1_000);
        let price = catalogue().listing("mechs/driller.ron").unwrap().price;
        assert!(c.pay(price, "jobs", 200));
        assert!(buy(&mut c, &catalogue(), &bays(), "mechs/driller.ron", Some(0), Vec2::ZERO, &WHOLE, 200).is_ok());
        assert_eq!(c.money, 1_000);
    }

    #[test]
    fn the_shipped_catalogue_lists_every_roster_mech_within_its_prices() {
        let cat = catalogue();
        assert_eq!(cat.listings.len(), 8);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        for l in &cat.listings {
            assert!(root.join(&l.design).exists(), "{} is in data/", l.design);
        }
    }
}
