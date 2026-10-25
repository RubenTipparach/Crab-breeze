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

/// One kind of job in data.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KindSpec {
    /// The kind.
    pub kind: JobKind,
    /// How often it is offered, against the others.
    pub weight: f32,
    /// Enemy mechs, fewest and most.
    pub mechs: (u32, u32),
    /// Credits paid per enemy mech.
    pub pay_per_mech: i64,
    /// Credits paid on top, whatever the size.
    pub pay_base: i64,
    /// Seconds from accepting to win it in.
    pub time_limit_s: f32,
    /// How far from the city's centre the site lies, as a share of the city's half size (0 in its
    /// heart, over 1 outside it).
    pub site_out: (f32, f32),
    /// Who sends it.
    pub senders: Vec<String>,
    /// Subject lines; `{city}`, `{mechs}` and `{pay}` are filled in.
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
        positive("inbox_kept", self.inbox_kept as f32)?;
        if self.kinds.is_empty() {
            return Err("kinds: none".into());
        }
        for (i, k) in self.kinds.iter().enumerate() {
            let f = |n: &str| format!("kinds[{i}].{n}");
            positive(&f("weight"), k.weight)?;
            positive(&f("time_limit_s"), k.time_limit_s)?;
            if k.mechs.0 == 0 || k.mechs.1 < k.mechs.0 {
                return Err(format!("{}: at least one, most not below least", f("mechs")));
            }
            if k.pay_per_mech < 0 || k.pay_base < 0 {
                return Err(format!("{}: pay must not be negative", f("pay")));
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

    /// Accepts an offer, when nothing else is under way. True when it was accepted.
    pub fn accept(&mut self, id: u32) -> bool {
        if self.current().is_some() {
            return false;
        }
        match self.job_mut(id) {
            Some(j) if j.state == JobState::Offered => {
                j.state = JobState::Accepted;
                j.clock_s = j.time_limit_s;
                true
            }
            _ => false,
        }
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
        sites: &[Site],
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
        if open < cfg.open_offers && self.next_offer_in_s <= 0.0 && !sites.is_empty() {
            let id = self.next_job;
            self.next_job += 1;
            let mut rng = SimRng::new(seed).fork(&format!("job/{id}"));
            let (job, from, subject, body) = offer(cfg, sites, id, &mut rng);
            self.jobs.push(job);
            self.say(&from, &subject, &body, Some(id), cfg.inbox_kept);
            self.next_offer_in_s = rng.range_f32(cfg.offer_gap_s.0, cfg.offer_gap_s.1);
            events.push(Event::Offered(id));
        }
        events
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

/// Draws one offer: its kind by weight, its city and site, its size and pay, and its message.
fn offer(cfg: &JobsConfig, sites: &[Site], id: u32, rng: &mut SimRng) -> (Job, String, String, String) {
    let total: f32 = cfg.kinds.iter().map(|k| k.weight).sum();
    let mut pick = rng.range_f32(0.0, total);
    let spec = cfg
        .kinds
        .iter()
        .find(|k| {
            pick -= k.weight;
            pick <= 0.0
        })
        .unwrap_or(&cfg.kinds[0]);
    let s = sites[(rng.next_u64() % sites.len() as u64) as usize];
    let a = rng.range_f32(0.0, std::f32::consts::TAU);
    let out = rng.range_f32(spec.site_out.0, spec.site_out.1) * s.half_size_m;
    let site = s.centre + Vec2::new(a.cos(), a.sin()) * out;
    let mechs = spec.mechs.0 + (rng.next_u64() % u64::from(spec.mechs.1 - spec.mechs.0 + 1)) as u32;
    let pay = spec.pay_base + spec.pay_per_mech * i64::from(mechs);
    let pick_text = |list: &[String], rng: &mut SimRng| list[(rng.next_u64() % list.len() as u64) as usize].clone();
    let fill = |t: String| {
        t.replace("{city}", &format!("City {}", s.city + 1))
            .replace("{mechs}", &mechs.to_string())
            .replace("{pay}", &format!("{pay}"))
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
        time_limit_s: spec.time_limit_s,
        clock_s: cfg.offer_life_s,
        state: JobState::Offered,
    };
    (job, from, subject, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

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

    /// Runs `s` seconds of an idle pilot far from everything.
    fn run(b: &mut Board, s: f32) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..(s / DT) as usize {
            all.extend(b.step(&cfg(), &sites(), 42, Vec2::splat(1.0e6), None, false, DT));
        }
        all
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
        assert!(b.accept(id));
        let site = b.job(id).unwrap().site;
        let e = b.step(&cfg(), &sites(), 42, site, None, false, DT);
        assert!(e.contains(&Event::Arrived(id)));
        let e = b.step(&cfg(), &sites(), 42, site, Some(2), false, DT);
        assert!(!e.iter().any(|e| matches!(e, Event::Won(..))));
        let e = b.step(&cfg(), &sites(), 42, site, Some(0), false, DT);
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
        b.step(&cfg(), &sites(), 42, site, None, false, DT);
        let e = b.step(&cfg(), &sites(), 42, site, Some(3), true, DT);
        assert!(e.contains(&Event::Failed(id)));
        assert_eq!(b.job(id).unwrap().state, JobState::Failed);
    }

    #[test]
    fn only_one_job_at_a_time() {
        let mut b = Board::default();
        run(&mut b, 600.0);
        let ids: Vec<u32> = b.jobs.iter().filter(|j| j.state == JobState::Offered).map(|j| j.id).collect();
        assert!(ids.len() >= 2);
        assert!(b.accept(ids[0]));
        assert!(!b.accept(ids[1]), "one at a time");
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
