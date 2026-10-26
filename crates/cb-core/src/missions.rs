//! Free roam's missions (openspec change `free-roam-missions`): one standing message for each mission in free
//! roam's mail, and one mission running at a time. Starting a mission turns off the one running first (owner,
//! 2026-10-25: "if activating a new mission it should turn off existing mission"), the same one included, so
//! starting the running mission restarts it. Pure: the game begins and ends the missions this says.

use crate::data::Validate;
use crate::jobs::Message;
use serde::Deserialize;

/// A mission free roam can start. One per mission plugin in the game.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MissionKind {
    /// The three-city defence (`city-defence`).
    Defence,
    /// The Hermit, the crab air fortress (`crab-airship`).
    Fortress,
}

impl MissionKind {
    /// Every mission, in a stable order.
    pub const ALL: [MissionKind; 2] = [MissionKind::Defence, MissionKind::Fortress];
}

/// A mission's message in free roam's mail.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissionMail {
    /// The mission it starts.
    pub mission: MissionKind,
    /// Who sends it.
    pub from: String,
    /// Its subject.
    pub subject: String,
    /// Its body.
    pub body: String,
}

/// `data/missions/mail.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissionsMail {
    /// One message per mission, newest last.
    pub mail: Vec<MissionMail>,
}

impl Validate for MissionsMail {
    fn validate(&self) -> Result<(), String> {
        for k in MissionKind::ALL {
            match self.mail.iter().filter(|m| m.mission == k).count() {
                1 => {}
                n => return Err(format!("mail: {n} messages for {k:?}, one wanted")),
            }
        }
        for (i, m) in self.mail.iter().enumerate() {
            if m.from.is_empty() || m.subject.is_empty() || m.body.is_empty() {
                return Err(format!("mail[{i}]: from, subject and body each need text"));
            }
        }
        Ok(())
    }
}

/// A change of mission: what to turn off, then what to begin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Switch {
    /// The mission turned off, if one ran.
    pub stopped: Option<MissionKind>,
    /// The mission begun, if any.
    pub started: Option<MissionKind>,
}

/// Free roam's mail and its running mission.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FreeRoam {
    /// The mission running, if any.
    pub active: Option<MissionKind>,
    /// The mail: one message per mission (its `job` is unused here).
    pub inbox: Vec<Message>,
}

impl FreeRoam {
    /// Free roam with every mission's message in its inbox.
    pub fn new(mail: &MissionsMail) -> Self {
        let inbox = mail
            .mail
            .iter()
            .enumerate()
            .map(|(i, m)| Message {
                id: i as u32,
                from: m.from.clone(),
                subject: m.subject.clone(),
                body: m.body.clone(),
                job: None,
                read: false,
            })
            .collect();
        Self { active: None, inbox }
    }

    /// Starts `kind`, turning off the mission running first (itself included: a restart).
    pub fn start(&mut self, kind: MissionKind) -> Switch {
        let stopped = self.active.replace(kind);
        Switch {
            stopped,
            started: Some(kind),
        }
    }

    /// Turns off the mission running, if any; none runs after.
    pub fn stop(&mut self) -> Switch {
        Switch {
            stopped: self.active.take(),
            started: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail() -> MissionsMail {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/missions/mail.ron");
        crate::data::load_ron(&p).unwrap()
    }

    #[test]
    fn every_mission_has_one_message() {
        let m = mail();
        let f = FreeRoam::new(&m);
        assert_eq!(f.inbox.len(), MissionKind::ALL.len());
        let mut twice = m.clone();
        twice.mail.push(m.mail[0].clone());
        assert!(twice.validate().is_err());
    }

    #[test]
    fn starting_a_mission_turns_off_the_one_running() {
        let mut f = FreeRoam::new(&mail());
        assert_eq!(
            f.start(MissionKind::Defence),
            Switch {
                stopped: None,
                started: Some(MissionKind::Defence)
            }
        );
        assert_eq!(
            f.start(MissionKind::Fortress),
            Switch {
                stopped: Some(MissionKind::Defence),
                started: Some(MissionKind::Fortress)
            }
        );
        assert_eq!(f.active, Some(MissionKind::Fortress));
    }

    #[test]
    fn starting_the_running_mission_restarts_it() {
        let mut f = FreeRoam::new(&mail());
        f.start(MissionKind::Fortress);
        let s = f.start(MissionKind::Fortress);
        assert_eq!(s.stopped, Some(MissionKind::Fortress));
        assert_eq!(s.started, Some(MissionKind::Fortress));
    }

    #[test]
    fn stop_leaves_no_mission_running() {
        let mut f = FreeRoam::new(&mail());
        f.start(MissionKind::Defence);
        assert_eq!(f.stop().stopped, Some(MissionKind::Defence));
        assert_eq!(f.active, None);
        assert_eq!(
            f.stop(),
            Switch {
                stopped: None,
                started: None
            }
        );
    }
}
