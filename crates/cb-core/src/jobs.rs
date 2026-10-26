//! Jobs and mail (openspec change `mission-jobs`): offers arrive as messages from clients, the pilot
//! accepts one, flies to its site (in or near a city), and the fight starts on arrival; destroying
//! its enemies wins it, its clock running out or the pilot's mech going down fails it. Pure and
//! deterministic: every offer is drawn from the career's seed and the offer's id, so the same career
//! gets the same mail. The game spawns the enemies, reports how many are left, and pays through the
//! career's ledger.

use crate::data::{Validate, positive};
use crate::rng::SimRng;
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// The riskiest job tier.
pub const MAX_TIER: u8 = 4;

fn tier_one() -> u8 {
    1
}

/// The kinds of job (owner, survey J12: defend a city, intercept a raid, destroy a crab nest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobKind {
    /// Raiders strike a city: stop them in its streets.
    Defend,
    /// A raid gathers on a city's edge: catch it before it moves in.
    Intercept,
    /// Crabs have dug in near a city: destroy them.
    Destroy,
}

/// One risk tier of a kind of job (openspec change `career-progression`): its size and pay, roughly doubling a tier.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TierSpec {
    /// The tier, 1 (a patrol or a small raid) to 4 (an assault on a nest).
    pub tier: u8,
    /// Enemy mechs, fewest and most.
    pub mechs: (u32, u32),
    /// Credits paid per enemy mech.
    pub pay_per_mech: i64,
    /// Credits paid on top, whatever the size.
    pub pay_base: i64,
}

/// One kind of job in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KindSpec {
    /// The kind.
    pub kind: JobKind,
    /// How often it is offered, against the others.
    pub weight: f32,
    /// Its risk tiers, lowest first.
    pub tiers: Vec<TierSpec>,
    /// Seconds from accepting to win it in.
    pub time_limit_s: f32,
    /// How far from the city's centre the site lies, as a share of the city's half size (0 in its
    /// heart, over 1 outside it).
    pub site_out: (f32, f32),
    /// Who sends it.
    pub senders: Vec<String>,
    /// Subject lines; `{city}`, `{mechs}`, `{pay}` and `{tier}` are filled in.
    pub subjects: Vec<String>,
    /// Message bodies; the same slots.
    pub bodies: Vec<String>,
}

/// Jobs in data, `data/jobs/jobs.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JobsConfig {
    /// Offers open at once.
    pub open_offers: usize,
    /// Seconds between one offer closing and the next arriving, least and most.
    pub offer_gap_s: (f32, f32),
    /// An offer left unanswered this long expires.
    pub offer_life_s: f32,
    /// The fight starts when the pilot comes within this of the site.
    pub arrive_m: f32,
    /// A job's enemy mechs come in this far from its site, least and most.
    pub spawn_ring_m: (f32, f32),
    /// And never nearer the pilot than this, so nothing is firing as the fight begins (owner, 2026-10-25:
    /// "the enemies shuodl be farther away so they dont start shooting as soon as I board").
    pub clear_of_pilot_m: f32,
    /// The newest messages kept.
    pub inbox_kept: usize,
    /// The kinds, by weight.
    pub kinds: Vec<KindSpec>,
}

impl Validate for JobsConfig {
    fn validate(&self) -> Result<(), String> {
        positive("open_offers", self.open_offers as f32)?;
        positive("offer_gap_s.0", self.offer_gap_s.0)?;
        if self.offer_gap_s.1 < self.offer_gap_s.0 {
            return Err("offer_gap_s: the most is less than the least".into());
        }
        positive("offer_life_s", self.offer_life_s)?;
        positive("arrive_m", self.arrive_m)?;
        positive("spawn_ring_m.0", self.spawn_ring_m.0)?;
        if self.spawn_ring_m.1 < self.spawn_ring_m.0 {
            return Err("spawn_ring_m: the most is less than the least".into());
        }
        positive("clear_of_pilot_m", self.clear_of_pilot_m)?;
        positive("inbox_kept", self.inbox_kept as f32)?;
        if self.kinds.is_empty() {
            return Err("kinds: none".into());
        }
        if !self.kinds.iter().any(|k| k.tiers.iter().any(|t| t.tier == 1)) {
            return Err("kinds: no tier 1 job, so a new career is offered nothing".into());
        }
        for (i, k) in self.kinds.iter().enumerate() {
            let f = |n: &str| format!("kinds[{i}].{n}");
            positive(&f("weight"), k.weight)?;
            positive(&f("time_limit_s"), k.time_limit_s)?;
            if k.tiers.is_empty() {
                return Err(format!("{}: none", f("tiers")));
            }
            for (ti, t) in k.tiers.iter().enumerate() {
                let g = |n: &str| format!("kinds[{i}].tiers[{ti}].{n}");
                if !(1..=MAX_TIER).contains(&t.tier) || k.tiers[..ti].iter().any(|o| o.tier >= t.tier) {
                    return Err(format!("{}: 1 to {MAX_TIER}, rising", g("tier")));
                }
                if t.mechs.0 == 0 || t.mechs.1 < t.mechs.0 {
                    return Err(format!("{}: at least one, most not below least", g("mechs")));
                }
                if t.pay_per_mech < 0 || t.pay_base < 0 {
                    return Err(format!("{}: pay must not be negative", g("pay")));
                }
            }
            if k.senders.is_empty() || k.subjects.is_empty() || k.bodies.is_empty() {
                return Err(format!("{}: senders, subjects and bodies each need one", f("text")));
            }
        }
        Ok(())
    }
}

/// Where a job stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// In the inbox, waiting.
    Offered,
    /// Taken: the pilot is on the way.
    Accepted,
    /// The pilot has arrived: the fight is on.
    Active,
    /// Won and paid.
    Won,
    /// Lost: the clock ran out or the mech went down.
    Failed,
    /// Turned down.
    Declined,
    /// Left too long.
    Expired,
    /// Given up for another (owner, 2026-10-25: "if activating a new mission it should turn off existing
    /// mission"; `free-roam-missions`): unpaid.
    Abandoned,
}

/// A job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Stable id, in offer order.
    pub id: u32,
    /// Its kind.
    pub kind: JobKind,
    /// The city it is in or near (the world plan's index).
    pub city: u32,
    /// Where the fight is (x, z).
    pub site: Vec2,
    /// Enemy mechs.
    pub mechs: u32,
    /// Credits paid on a win.
    pub pay: i64,
    /// Its risk tier, 1 to 4 (career-progression); jobs offered before tiers existed are tier 1.
    #[serde(default = "tier_one")]
    pub tier: u8,
    /// The base that offers it: the nearest to its site (its standing raises the pay).
    #[serde(default)]
    pub base: u32,
    /// Seconds to win it in, from accepting.
    pub time_limit_s: f32,
    /// Seconds left on its clock (accepted or active), or until it expires (offered).
    pub clock_s: f32,
    /// Where it stands.
    pub state: JobState,
}

/// A message in the inbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Stable id.
    pub id: u32,
    /// Sender.
    pub from: String,
    /// Subject line.
    pub subject: String,
    /// Body.
    pub body: String,
    /// The job it offers or reports on.
    pub job: Option<u32>,
    /// Read.
    pub read: bool,
}

/// A city a job can be sited in or near.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    /// The city's index in the world plan.
    pub city: u32,
    /// Its centre (x, z).
    pub centre: Vec2,
    /// Half its side.
    pub half_size_m: f32,
}

/// What the board may offer now (career-progression): the cities jobs are sited by, the bases that offer them, the
/// riskiest tier the career's mechs open, and each base's pay bonus from the pilot's standing there.
#[derive(Clone, Copy, Debug)]
pub struct Offers<'a> {
    /// The cities.
    pub sites: &'a [Site],
    /// Each base's centre (x, z), by id; none puts every job on base 0.
    pub bases: &'a [Vec2],
    /// The riskiest tier offered.
    pub max_tier: u8,
    /// Each base's pay bonus, a share (0.1 pays 10 % more), by id; a missing one is none.
    pub bonus: &'a [f32],
}

/// Pay rounded to the nearest 10 credits.
fn round10(c: f32) -> i64 {
    ((c / 10.0).round() as i64) * 10
}

/// What happened to the board this step.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A new offer arrived (the job's id).
    Offered(u32),
    /// The pilot reached the accepted job's site: the fight starts (the job's id).
    Arrived(u32),
    /// The active job was won (its id, its pay).
    Won(u32, i64),
    /// The active or accepted job failed (its id).
    Failed(u32),
    /// The job under way was given up for another (its id): its enemies leave.
    Abandoned(u32),
}

/// The jobs and the inbox.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Board {
    /// Every job, offered or done, in offer order.
    pub jobs: Vec<Job>,
    /// The inbox, newest last.
    pub inbox: Vec<Message>,
    next_job: u32,
    next_message: u32,
    next_offer_in_s: f32,
}

impl Board {
    /// The job accepted or under way, if any.
    pub fn current(&self) -> Option<&Job> {
        self.jobs.iter().find(|j| matches!(j.state, JobState::Accepted | JobState::Active))
    }

    /// A job by id.
    pub fn job(&self, id: u32) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    fn job_mut(&mut self, id: u32) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    /// Accepts an offer, abandoning the job under way if there is one (one job at a time). Returns the events:
    /// `Abandoned` for the job given up, if any; empty when `id` is not an open offer.
    pub fn accept(&mut self, id: u32) -> Vec<Event> {
        if !self.job(id).is_some_and(|j| j.state == JobState::Offered) {
            return Vec::new();
        }
        let mut events = Vec::new();
        if let Some(cur) = self.current().map(|j| j.id)
            && let Some(j) = self.job_mut(cur)
        {
            j.state = JobState::Abandoned;
            events.push(Event::Abandoned(cur));
        }
        if let Some(j) = self.job_mut(id) {
            j.state = JobState::Accepted;
            j.clock_s = j.time_limit_s;
        }
        events
    }

    /// Turns an offer down. True when it was declined.
    pub fn decline(&mut self, id: u32) -> bool {
        match self.job_mut(id) {
            Some(j) if j.state == JobState::Offered => {
                j.state = JobState::Declined;
                true
            }
            _ => false,
        }
    }

    /// One step of `dt_s`: new offers arrive and old ones expire, the accepted job's clock runs and it
    /// starts when the pilot (`pilot_xz`) comes within reach, and the active job is won when
    /// `enemies_left` reaches zero or fails when its clock runs out or `pilot_down`.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        cfg: &JobsConfig,
        offers: &Offers,
        seed: u64,
        pilot_xz: Vec2,
        enemies_left: Option<u32>,
        pilot_down: bool,
        dt_s: f32,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        for j in &mut self.jobs {
            match j.state {
                JobState::Offered => {
                    j.clock_s -= dt_s;
                    if j.clock_s <= 0.0 {
                        j.state = JobState::Expired;
                    }
                }
                JobState::Accepted | JobState::Active => {
                    j.clock_s -= dt_s;
                    if j.clock_s <= 0.0 || pilot_down {
                        j.state = JobState::Failed;
                        events.push(Event::Failed(j.id));
                    } else if j.state == JobState::Accepted && j.site.distance(pilot_xz) <= cfg.arrive_m {
                        j.state = JobState::Active;
                        events.push(Event::Arrived(j.id));
                    } else if j.state == JobState::Active && enemies_left == Some(0) {
                        j.state = JobState::Won;
                        events.push(Event::Won(j.id, j.pay));
                    }
                }
                _ => {}
            }
        }
        let open = self.jobs.iter().filter(|j| j.state == JobState::Offered).count();
        self.next_offer_in_s -= dt_s;
        if open < cfg.open_offers && self.next_offer_in_s <= 0.0 {
            let id = self.next_job;
            let mut rng = SimRng::new(seed).fork(&format!("job/{id}"));
            if self.make_offer(cfg, offers, &mut rng, None).is_some() {
                self.next_offer_in_s = rng.range_f32(cfg.offer_gap_s.0, cfg.offer_gap_s.1);
                events.push(Event::Offered(id));
            }
        }
        events
    }

    /// A tier has just opened (the career bought a mech rated for it): says so by mail and offers a job of that
    /// tier at once, beyond the open limit. Returns the new job's id; `None` when no kind has that tier or there
    /// is nowhere to site it.
    pub fn open_tier(&mut self, cfg: &JobsConfig, offers: &Offers, seed: u64, tier: u8) -> Option<u32> {
        let mut rng = SimRng::new(seed).fork(&format!("job/{}", self.next_job));
        let id = self.make_offer(cfg, offers, &mut rng, Some(tier))?;
        self.say(
            "Island Command",
            &format!("Tier {tier} contracts open"),
            &format!("Your new mech is rated for tier {tier} work. Riskier contracts, better pay: the first is in your mail."),
            None,
            cfg.inbox_kept,
        );
        Some(id)
    }

    /// Draws one offer (at `forced` tier, else any the career opens) and mails it.
    fn make_offer(&mut self, cfg: &JobsConfig, offers: &Offers, rng: &mut SimRng, forced: Option<u8>) -> Option<u32> {
        if offers.sites.is_empty() {
            return None;
        }
        let id = self.next_job;
        let (job, from, subject, body) = offer(cfg, offers, id, rng, forced)?;
        self.next_job += 1;
        self.jobs.push(job);
        self.say(&from, &subject, &body, Some(id), cfg.inbox_kept);
        Some(id)
    }

    /// Adds a message to the inbox, keeping the newest `kept`.
    pub fn say(&mut self, from: &str, subject: &str, body: &str, job: Option<u32>, kept: usize) {
        self.inbox.push(Message {
            id: self.next_message,
            from: from.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            job,
            read: false,
        });
        self.next_message += 1;
        if self.inbox.len() > kept {
            let drop = self.inbox.len() - kept;
            self.inbox.drain(..drop);
        }
    }
}

/// Draws one offer: its kind by weight among the kinds with a tier open (or the `forced` tier), its tier among
/// those open, its city and site, the base that offers it (the nearest), its size and its pay (with that base's
/// standing bonus), and its message.
fn offer(cfg: &JobsConfig, offers: &Offers, id: u32, rng: &mut SimRng, forced: Option<u8>) -> Option<(Job, String, String, String)> {
    let open = |t: &TierSpec| match forced {
        Some(f) => t.tier == f,
        None => t.tier <= offers.max_tier,
    };
    let kinds: Vec<&KindSpec> = cfg.kinds.iter().filter(|k| k.tiers.iter().any(open)).collect();
    if kinds.is_empty() {
        return None;
    }
    let total: f32 = kinds.iter().map(|k| k.weight).sum();
    let mut pick = rng.range_f32(0.0, total);
    let spec = *kinds
        .iter()
        .find(|k| {
            pick -= k.weight;
            pick <= 0.0
        })
        .unwrap_or(&kinds[0]);
    let tiers: Vec<&TierSpec> = spec.tiers.iter().filter(|t| open(t)).collect();
    let tier = tiers[(rng.next_u64() % tiers.len() as u64) as usize];
    let sites = offers.sites;
    let s = sites[(rng.next_u64() % sites.len() as u64) as usize];
    let a = rng.range_f32(0.0, std::f32::consts::TAU);
    let out = rng.range_f32(spec.site_out.0, spec.site_out.1) * s.half_size_m;
    let site = s.centre + Vec2::new(a.cos(), a.sin()) * out;
    let mechs = tier.mechs.0 + (rng.next_u64() % u64::from(tier.mechs.1 - tier.mechs.0 + 1)) as u32;
    let base = (0..offers.bases.len())
        .min_by(|a, b| offers.bases[*a].distance(site).total_cmp(&offers.bases[*b].distance(site)))
        .unwrap_or(0) as u32;
    let bonus = offers.bonus.get(base as usize).copied().unwrap_or(0.0).max(0.0);
    let pay = round10((tier.pay_base + tier.pay_per_mech * i64::from(mechs)) as f32 * (1.0 + bonus));
    let pick_text = |list: &[String], rng: &mut SimRng| list[(rng.next_u64() % list.len() as u64) as usize].clone();
    let fill = |t: String| {
        t.replace("{city}", &format!("City {}", s.city + 1))
            .replace("{mechs}", &mechs.to_string())
            .replace("{pay}", &format!("{pay}"))
            .replace("{tier}", &tier.tier.to_string())
    };
    let from = pick_text(&spec.senders, rng);
    let subject = fill(pick_text(&spec.subjects, rng));
    let body = fill(pick_text(&spec.bodies, rng));
    let job = Job {
        id,
        kind: spec.kind,
        city: s.city,
        site,
        mechs,
        pay,
        tier: tier.tier,
        base,
        time_limit_s: spec.time_limit_s,
        clock_s: cfg.offer_life_s,
        state: JobState::Offered,
    };
    Some((job, from, subject, body))
}

/// Where a job's enemy mech comes in: `ring_m` (0 to 1, from the least of `spawn_ring_m` to the most) out from the
/// site along `bearing_rad`, pushed straight out from the pilot to `clear_of_pilot_m` when that lands nearer. The
/// draws are the caller's (one forked stream per mech), so the same job brings its mechs to the same places.
pub fn enemy_spot(cfg: &JobsConfig, site: Vec2, pilot: Vec2, bearing_rad: f32, ring: f32) -> Vec2 {
    let (lo, hi) = cfg.spawn_ring_m;
    let at = site + Vec2::new(bearing_rad.cos(), bearing_rad.sin()) * (lo + (hi - lo) * ring.clamp(0.0, 1.0));
    let off = at - pilot;
    if off.length() >= cfg.clear_of_pilot_m {
        return at;
    }
    // Straight out from the pilot; on top of the pilot, out along the bearing.
    let away = off.try_normalize().unwrap_or(Vec2::new(bearing_rad.cos(), bearing_rad.sin()));
    pilot + away * cfg.clear_of_pilot_m
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn a_job_mech_comes_in_on_its_ring_round_the_site() {
        let c = cfg();
        let site = Vec2::new(5000.0, 0.0);
        let p = enemy_spot(&c, site, Vec2::splat(-1.0e5), 0.7, 0.5);
        let d = p.distance(site);
        assert!(
            (d - (c.spawn_ring_m.0 + c.spawn_ring_m.1) * 0.5).abs() < 0.01,
            "halfway out the ring: {d}"
        );
    }

    #[test]
    fn a_job_mech_never_comes_in_near_the_pilot() {
        let c = cfg();
        let site = Vec2::new(300.0, 0.0);
        for k in 0..64 {
            let a = k as f32 * 0.1;
            for pilot in [Vec2::ZERO, site, site + Vec2::new(c.spawn_ring_m.0, 0.0)] {
                let p = enemy_spot(&c, site, pilot, a, (k % 5) as f32 / 4.0);
                assert!(
                    p.distance(pilot) >= c.clear_of_pilot_m - 0.01,
                    "bearing {a}: {:.0} m from the pilot",
                    p.distance(pilot)
                );
                assert!(p.is_finite());
            }
        }
    }

    fn cfg() -> JobsConfig {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/jobs/jobs.ron");
        crate::data::load_ron(&p).unwrap()
    }

    fn sites() -> Vec<Site> {
        vec![
            Site {
                city: 0,
                centre: Vec2::new(0.0, 0.0),
                half_size_m: 500.0,
            },
            Site {
                city: 1,
                centre: Vec2::new(8000.0, 2000.0),
                half_size_m: 300.0,
            },
        ]
    }

    const BASES: [Vec2; 2] = [Vec2::new(0.0, 900.0), Vec2::new(8000.0, 3000.0)];

    /// Runs `s` seconds of an idle pilot far from everything, offered tiers up to `max_tier`.
    fn run_to(b: &mut Board, s: f32, max_tier: u8) -> Vec<Event> {
        let sites = sites();
        let o = Offers {
            sites: &sites,
            bases: &BASES,
            max_tier,
            bonus: &[],
        };
        let mut all = Vec::new();
        for _ in 0..(s / DT) as usize {
            all.extend(b.step(&cfg(), &o, 42, Vec2::splat(1.0e6), None, false, DT));
        }
        all
    }

    /// Runs `s` seconds of an idle pilot far from everything (the starting Foreman: tiers 1 and 2).
    fn run(b: &mut Board, s: f32) -> Vec<Event> {
        run_to(b, s, 2)
    }

    /// One step with the pilot at `at`.
    fn step_at(b: &mut Board, at: Vec2, left: Option<u32>, down: bool) -> Vec<Event> {
        let sites = sites();
        let o = Offers {
            sites: &sites,
            bases: &BASES,
            max_tier: 2,
            bonus: &[],
        };
        b.step(&cfg(), &o, 42, at, left, down, DT)
    }

    #[test]
    fn offers_arrive_by_mail_up_to_the_open_limit() {
        let mut b = Board::default();
        run(&mut b, 600.0);
        let open = b.jobs.iter().filter(|j| j.state == JobState::Offered).count();
        assert!(open <= cfg().open_offers && open > 0, "{open}");
        assert!(b.inbox.iter().any(|m| m.job.is_some()), "each offer is a message");
    }

    #[test]
    fn the_same_career_gets_the_same_mail() {
        let (mut a, mut b) = (Board::default(), Board::default());
        run(&mut a, 300.0);
        run(&mut b, 300.0);
        assert_eq!(a, b);
    }

    #[test]
    fn jobs_are_sited_in_or_near_a_city() {
        let mut b = Board::default();
        run(&mut b, 2000.0);
        for j in &b.jobs {
            let s = sites().into_iter().find(|s| s.city == j.city).unwrap();
            assert!(j.site.distance(s.centre) <= s.half_size_m * 3.0 + 1.0, "{j:?}");
        }
    }

    #[test]
    fn an_accepted_job_starts_on_arrival_and_pays_when_its_enemies_are_gone() {
        let mut b = Board::default();
        run(&mut b, 120.0);
        let id = b.jobs[0].id;
        assert!(b.accept(id).is_empty());
        assert_eq!(b.current().map(|j| j.id), Some(id));
        let site = b.job(id).unwrap().site;
        let e = step_at(&mut b, site, None, false);
        assert!(e.contains(&Event::Arrived(id)));
        let e = step_at(&mut b, site, Some(2), false);
        assert!(!e.iter().any(|e| matches!(e, Event::Won(..))));
        let e = step_at(&mut b, site, Some(0), false);
        let pay = b.job(id).unwrap().pay;
        assert!(e.contains(&Event::Won(id, pay)));
        assert!(pay > 0);
    }

    #[test]
    fn a_job_fails_when_the_mech_goes_down() {
        let mut b = Board::default();
        run(&mut b, 120.0);
        let id = b.jobs[0].id;
        b.accept(id);
        let site = b.job(id).unwrap().site;
        step_at(&mut b, site, None, false);
        let e = step_at(&mut b, site, Some(3), true);
        assert!(e.contains(&Event::Failed(id)));
        assert_eq!(b.job(id).unwrap().state, JobState::Failed);
    }

    #[test]
    fn accepting_a_job_abandons_the_one_under_way() {
        let mut b = Board::default();
        run(&mut b, 600.0);
        let ids: Vec<u32> = b.jobs.iter().filter(|j| j.state == JobState::Offered).map(|j| j.id).collect();
        assert!(ids.len() >= 2);
        assert!(b.accept(ids[0]).is_empty());
        assert_eq!(
            b.accept(ids[1]),
            vec![Event::Abandoned(ids[0])],
            "one at a time: the first is given up"
        );
        assert_eq!(b.job(ids[0]).unwrap().state, JobState::Abandoned);
        assert_eq!(b.current().map(|j| j.id), Some(ids[1]));
        assert!(b.accept(ids[0]).is_empty(), "an abandoned job is not an offer");
    }

    #[test]
    fn with_only_the_foreman_no_tier_3_or_4_job_is_offered() {
        let mut b = Board::default();
        run_to(&mut b, 3000.0, 2);
        assert!(b.jobs.len() > 5);
        assert!(
            b.jobs.iter().all(|j| j.tier <= 2),
            "{:?}",
            b.jobs.iter().map(|j| j.tier).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_tier_opening_mails_a_job_of_that_tier_at_once() {
        let mut b = Board::default();
        run(&mut b, 600.0);
        let sites = sites();
        let o = Offers {
            sites: &sites,
            bases: &BASES,
            max_tier: 3,
            bonus: &[],
        };
        let id = b.open_tier(&cfg(), &o, 42, 3).unwrap();
        assert_eq!(b.job(id).map(|j| (j.tier, j.state)), Some((3, JobState::Offered)));
        assert!(b.inbox.iter().any(|m| m.subject == "Tier 3 contracts open"));
        assert!(b.open_tier(&cfg(), &o, 42, 9).is_none(), "no kind has tier 9");
    }

    #[test]
    fn riskier_tiers_pay_more_and_standing_raises_the_pay() {
        let c = cfg();
        for k in &c.kinds {
            for w in k.tiers.windows(2) {
                let least = |t: &TierSpec| t.pay_base + t.pay_per_mech * i64::from(t.mechs.0);
                assert!(least(&w[1]) > least(&w[0]), "{:?} tier {} pays more", k.kind, w[1].tier);
            }
        }
        let sites = sites();
        let plain = Offers {
            sites: &sites,
            bases: &BASES,
            max_tier: 2,
            bonus: &[],
        };
        let liked = Offers {
            bonus: &[0.5, 0.5],
            ..plain
        };
        let (mut a, mut b) = (Board::default(), Board::default());
        let mut ra = SimRng::new(1).fork("x");
        let mut rb = SimRng::new(1).fork("x");
        a.make_offer(&c, &plain, &mut ra, None);
        b.make_offer(&c, &liked, &mut rb, None);
        let (pa, pb) = (a.jobs[0].pay, b.jobs[0].pay);
        assert_eq!(pb, round10(pa as f32 * 1.5));
        assert_eq!(pb % 10, 0, "rounded to 10 credits");
    }

    #[test]
    fn a_job_is_offered_by_the_base_nearest_its_site() {
        let mut b = Board::default();
        run(&mut b, 2000.0);
        for j in &b.jobs {
            let near = if BASES[0].distance(j.site) <= BASES[1].distance(j.site) {
                0
            } else {
                1
            };
            assert_eq!(j.base, near);
        }
    }

    #[test]
    fn the_board_survives_a_save() {
        let mut b = Board::default();
        run(&mut b, 300.0);
        let text = ron::to_string(&b).unwrap();
        let back: Board = ron::from_str(&text).unwrap();
        assert_eq!(back, b);
    }
}
