//! The mech shop (openspec change `mech-shop`, design sections 2 and 6): every roster mech is for sale from the
//! start (survey J14) at its price in `data/career/catalogue.ron`, bought with job pay through the career's
//! ledger and air dropped near the pilot (owner, 2026-10-26: "when a player orders a mech it should be air dropped
//! close to wherever the player is on a good are on the ground (not stuck on a building)"), at the site
//! `air_drop::drop_site` found. Only with no site does it go to a free hangar bay: the current base's when the
//! pilot is at one, else the nearest base with a free bay. A purchase the money cannot cover, or with neither a
//! site nor a bay, changes nothing. Pure.

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

/// Where a bought mech went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum To {
    /// Air dropped at this site (x, ground height, z); its drop is on the career's clock.
    Drop(glam::Vec3),
    /// Parked in a bay, the fallback when there was no site.
    Bay {
        /// The base it waits at.
        base: u32,
        /// Its bay there.
        bay: u32,
        /// At a base the pilot is not at (a mail says where).
        away: bool,
    },
}

/// Where a bought mech was delivered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Delivery {
    /// The new mech's key in the career.
    pub key: u32,
    /// Where it went.
    pub to: To,
}

/// An order: where the pilot is, and the drop's site if `air_drop::drop_site` found one.
#[derive(Clone, Copy, Debug)]
pub struct Order<'a> {
    /// The design (`mechs/sapper.ron`).
    pub design: &'a str,
    /// The base the pilot is at, if any.
    pub at: Option<u32>,
    /// Where the pilot is (x, z).
    pub from: Vec2,
    /// The drop's site, or `None` for a bay.
    pub site: Option<glam::Vec3>,
    /// Seeds the drop's aircraft track (the career's seed, forked by the game).
    pub rng: &'a crate::rng::SimRng,
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

/// The base nearest `from` (ties by id): a dropped mech's home in the books.
fn nearest_base(bays: &Bays, from: Vec2) -> u32 {
    (0..bays.centres.len() as u32)
        .min_by(|a, b| {
            let da = bays.centres[*a as usize].distance(from);
            let db = bays.centres[*b as usize].distance(from);
            da.total_cmp(&db).then(a.cmp(b))
        })
        .unwrap_or(0)
}

/// Buys `order.design`: charges its price through the ledger (`kept` ledger lines) and air drops it whole
/// (`structure`, its parts' full structure) at the order's site, or with no site parks it in a bay. Refused before
/// anything changes when it is not listed, there is neither a site nor a free bay, or the money is short.
pub fn buy(
    career: &mut Career,
    catalogue: &Catalogue,
    bays: &Bays,
    order: &Order,
    structure: &[f32],
    kept: usize,
) -> Result<Delivery, Refusal> {
    let design = order.design;
    let listing = catalogue.listing(design).ok_or(Refusal::NotListed)?;
    let bay = match order.site {
        Some(_) => None,
        None => Some(delivery_bay(career, bays, order.at, order.from).ok_or(Refusal::NoFreeBay)?),
    };
    let name = design.trim_start_matches("mechs/").trim_end_matches(".ron");
    career
        .charge(listing.price, &format!("bought {name}"), kept)
        .map_err(|s| Refusal::Short(s.missing))?;
    // Found before the charge: a site, or (with none) the bay.
    let Some(site) = order.site else {
        let (base, bay) = bay.unwrap_or_default();
        let key = career.add_bought_mech(design, base, bay, structure);
        let away = order.at != Some(base);
        return Ok(Delivery {
            key,
            to: To::Bay { base, bay, away },
        });
    };
    let home = order.at.unwrap_or_else(|| nearest_base(bays, order.from));
    let key = career.add_dropped_mech(design, home, structure, site, order.rng);
    Ok(Delivery { key, to: To::Drop(site) })
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

    fn order<'a>(design: &'a str, at: Option<u32>, from: Vec2, site: Option<glam::Vec3>) -> Order<'a> {
        static RNG: std::sync::OnceLock<crate::rng::SimRng> = std::sync::OnceLock::new();
        Order {
            design,
            at,
            from,
            site,
            rng: RNG.get_or_init(|| crate::rng::SimRng::new(9)),
        }
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
        let r = buy(
            &mut c,
            &catalogue(),
            &bays(),
            &order("mechs/harrow.ron", Some(0), Vec2::ZERO, None),
            &WHOLE,
            200,
        );
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
        let r = buy(
            &mut c,
            &catalogue(),
            &bays(),
            &order("mechs/mule.ron", Some(0), Vec2::ZERO, None),
            &WHOLE,
            200,
        );
        assert_eq!(r, Err(Refusal::NoFreeBay));
        assert_eq!(c, before);
    }

    #[test]
    fn at_a_base_the_mech_goes_to_its_lowest_free_bay_and_the_ledger_records_the_price() {
        let mut c = career(1_000_000);
        let price = catalogue().listing("mechs/picket.ron").unwrap().price;
        let d = buy(
            &mut c,
            &catalogue(),
            &bays(),
            &order("mechs/picket.ron", Some(0), Vec2::ZERO, None),
            &WHOLE,
            200,
        )
        .unwrap();
        assert_eq!(
            d.to,
            To::Bay {
                base: 0,
                bay: 1,
                away: false
            },
            "bay 0 holds the starting mech"
        );
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
        let o = order("mechs/mule.ron", None, Vec2::new(4800.0, 0.0), None);
        let d = buy(&mut c, &catalogue(), &bays(), &o, &WHOLE, 200).unwrap();
        assert_eq!(
            d.to,
            To::Bay {
                base: 0,
                bay: 1,
                away: true
            }
        );
    }

    #[test]
    fn with_a_site_it_is_air_dropped_even_when_every_bay_is_full() {
        let mut c = career(10_000_000);
        for base in 0..3 {
            for bay in 0..3 {
                if (base, bay) != (0, 0) {
                    c.add_mech("mechs/mule.ron", base, bay, &WHOLE);
                }
            }
        }
        let site = glam::Vec3::new(4870.0, 3.0, 40.0);
        let price = catalogue().listing("mechs/sapper.ron").unwrap().price;
        let d = buy(
            &mut c,
            &catalogue(),
            &bays(),
            &order("mechs/sapper.ron", None, Vec2::new(4800.0, 0.0), Some(site)),
            &WHOLE,
            200,
        )
        .unwrap();
        assert_eq!(d.to, To::Drop(site));
        assert_eq!(c.money, 10_000_000 - price);
        let m = c.mech(d.key).unwrap();
        assert_eq!(m.field.map(|f| f.pos), Some(site.to_array()), "it stands at the site");
        assert_eq!(m.base, 1, "booked to the nearest base");
        assert!(c.drop_of(d.key).is_some_and(|dr| dr.site == site.to_array() && !dr.landed));
    }

    #[test]
    fn a_drop_short_of_money_changes_nothing() {
        let mut c = career(1_000);
        let before = c.clone();
        let site = Some(glam::Vec3::new(50.0, 0.0, 0.0));
        let r = buy(
            &mut c,
            &catalogue(),
            &bays(),
            &order("mechs/mule.ron", Some(0), Vec2::ZERO, site),
            &WHOLE,
            200,
        );
        assert!(matches!(r, Err(Refusal::Short(_))));
        assert_eq!(c, before, "no mech, no drop, no charge");
    }

    #[test]
    fn job_pay_buys_a_mech() {
        let mut c = career(1_000);
        let price = catalogue().listing("mechs/driller.ron").unwrap().price;
        assert!(c.pay(price, "jobs", 200));
        assert!(
            buy(
                &mut c,
                &catalogue(),
                &bays(),
                &order("mechs/driller.ron", Some(0), Vec2::ZERO, None),
                &WHOLE,
                200
            )
            .is_ok()
        );
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
