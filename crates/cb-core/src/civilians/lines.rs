//! Who people are and what they say when greeted, `data/people/lines.ron` (openspec change
//! `civilians`, design section 1): roles by place (a passer-by on a pavement, a mechanic on a base's
//! apron), and each role's lines. A greeting picks one by the person and how many times they have been
//! greeted, so the same person answers the same way in every run. A line marked `job` tells of an open
//! job and is only picked when there is one to tell of.

use crate::data::{Validate, positive};
use crate::rng::{SimRng, label_hash};
use serde::Deserialize;

/// Where a person lives, which decides who they can be and what they say.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// A city pavement.
    Pavement,
    /// A city park.
    Park,
    /// An air base's apron.
    Base,
}

/// Someone a person can be.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Its key, which lines name.
    pub id: String,
    /// What the greeting shows them as ("Mechanic").
    pub name: String,
    /// Where they are found.
    pub place: Place,
    /// How often, against the place's other roles.
    pub weight: f32,
}

/// Something a person says.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Line {
    /// The role that says it.
    pub role: String,
    /// The words; `{job}` is filled with the job told of.
    pub text: String,
    /// It tells of a job, and is only said while one is open.
    #[serde(default)]
    pub job: bool,
}

/// The roles and lines.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lines {
    /// Every role.
    pub roles: Vec<Role>,
    /// Every line.
    pub lines: Vec<Line>,
}

impl Validate for Lines {
    fn validate(&self) -> Result<(), String> {
        for (i, r) in self.roles.iter().enumerate() {
            positive(&format!("roles[{i}].weight"), r.weight)?;
            if self.roles[..i].iter().any(|o| o.id == r.id) {
                return Err(format!("roles[{i}]: `{}` is named twice", r.id));
            }
            if !self.lines.iter().any(|l| l.role == r.id && !l.job) {
                return Err(format!("roles[{i}]: `{}` has no line that is not a job's", r.id));
            }
        }
        for place in [Place::Pavement, Place::Park, Place::Base] {
            if !self.roles.iter().any(|r| r.place == place) {
                return Err(format!("roles: nobody lives on a {place:?}"));
            }
        }
        for (i, l) in self.lines.iter().enumerate() {
            if !self.roles.iter().any(|r| r.id == l.role) {
                return Err(format!("lines[{i}]: no role `{}`", l.role));
            }
            if l.text.trim().is_empty() {
                return Err(format!("lines[{i}]: the text is empty"));
            }
            if l.job != l.text.contains("{job}") {
                return Err(format!("lines[{i}]: a job line, and only a job line, names `{{job}}`"));
            }
        }
        Ok(())
    }
}

/// A greeting's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// Who answers ("Mechanic").
    pub who: String,
    /// What they say, slots filled.
    pub text: String,
    /// The line tells of a job.
    pub job: bool,
}

impl Lines {
    /// A role for someone living on `place`, drawn by weight; `None` when the place has no roles.
    pub fn draw_role(&self, place: Place, rng: &mut SimRng) -> Option<u16> {
        let total: f32 = self.roles.iter().filter(|r| r.place == place).map(|r| r.weight).sum();
        if total <= 0.0 {
            return None;
        }
        let mut pick = rng.next_f32() * total;
        let mut last = None;
        for (i, r) in self.roles.iter().enumerate().filter(|(_, r)| r.place == place) {
            last = Some(i as u16);
            if pick < r.weight {
                return last;
            }
            pick -= r.weight;
        }
        last
    }

    /// What person `person_id` of role `role` says on their `greeted`-th greeting: one of the role's
    /// lines, picked by the person and the count. Job lines are in the draw only when `job` names an
    /// open job (their `{job}` filled with it).
    pub fn answer(&self, role: u16, person_id: u32, greeted: u32, job: Option<&str>) -> Option<Answer> {
        let r = self.roles.get(role as usize)?;
        let pool: Vec<&Line> = self.lines.iter().filter(|l| l.role == r.id && (!l.job || job.is_some())).collect();
        if pool.is_empty() {
            return None;
        }
        let mut rng = SimRng::new(label_hash(&format!("greet/{person_id}/{greeted}")));
        let line = pool[(rng.next_u64() % pool.len() as u64) as usize];
        let text = match job {
            Some(j) => line.text.replace("{job}", j),
            None => line.text.clone(),
        };
        Some(Answer {
            who: r.name.clone(),
            text,
            job: line.job,
        })
    }
}
