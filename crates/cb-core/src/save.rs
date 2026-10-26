//! Where a saved game left the player (openspec change `save-game`; owner, 2026-10-25: "where is the persistence? if I
//! click continue, it should load the game state where I was at last"). A `Place` is the pilot and their mech: where
//! each is, where the pilot looks, whether they sit in the seat, the mech's damage and its missiles. A career carries
//! one with its books; free roam saves one with the world it was played in (`FreeRoamSave`). Pure: the app builds and
//! applies them.

use serde::{Deserialize, Serialize};

/// The free-roam save format's version.
pub const VERSION: u32 = 1;

/// Where the pilot and their mech are, and the mech's state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// The pilot's feet (on foot), world metres.
    pub pilot_feet: [f32; 3],
    /// Where the pilot looks: yaw from -z, left positive, radians.
    pub pilot_yaw_rad: f32,
    /// Where the pilot looks: pitch, up positive, radians.
    pub pilot_pitch_rad: f32,
    /// In the seat (else on foot).
    pub seated: bool,
    /// The mech's feet, world metres.
    pub mech_pos: [f32; 3],
    /// The mech's heading, radians.
    pub mech_heading_rad: f32,
    /// Each part's structure left.
    pub structure: Vec<f32>,
    /// Each part lost.
    pub lost: Vec<bool>,
    /// Each weapon's rounds left (`None`: unlimited).
    pub rounds_left: Vec<Option<u32>>,
}

impl Place {
    /// Every number is finite and the damage lists agree.
    pub fn check(&self) -> Result<(), String> {
        let numbers = self
            .pilot_feet
            .iter()
            .chain(&self.mech_pos)
            .chain([&self.pilot_yaw_rad, &self.pilot_pitch_rad, &self.mech_heading_rad])
            .chain(&self.structure);
        if numbers.into_iter().any(|v| !v.is_finite()) {
            return Err("place: a number is not finite".into());
        }
        if self.structure.len() != self.lost.len() {
            return Err("place: damage does not match its parts".into());
        }
        Ok(())
    }
}

/// A free-roam game: the world it was played in and where the player was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeRoamSave {
    /// The format's version.
    pub version: u32,
    /// The world seed.
    pub seed: u64,
    /// The piloted mech's data file, relative to `data/`.
    pub mech: String,
    /// Where the player was.
    pub place: Place,
}

impl FreeRoamSave {
    /// The save file's text.
    pub fn to_ron(&self) -> String {
        let body = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default();
        format!("// A Crab Breeze free-roam game (openspec change save-game), written by the game.\n{body}\n")
    }

    /// A save read from its file's text; a newer version or a bad file is refused with why.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let s: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        if s.version > VERSION {
            return Err(format!("save version {} is newer than this game's {VERSION}", s.version));
        }
        s.place.check()?;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place() -> Place {
        Place {
            pilot_feet: [120.5, 41.0, -830.25],
            pilot_yaw_rad: 1.25,
            pilot_pitch_rad: -0.1,
            seated: true,
            mech_pos: [118.0, 40.5, -826.0],
            mech_heading_rad: 0.75,
            structure: vec![100.0, 42.5, 0.0],
            lost: vec![false, false, true],
            rounds_left: vec![None, Some(17)],
        }
    }

    #[test]
    fn a_free_roam_save_comes_back_exactly() {
        let s = FreeRoamSave {
            version: VERSION,
            seed: 42,
            mech: "mechs/foreman.ron".into(),
            place: place(),
        };
        assert_eq!(FreeRoamSave::from_ron(&s.to_ron()), Ok(s));
    }

    #[test]
    fn a_broken_or_newer_save_is_refused() {
        assert!(FreeRoamSave::from_ron("not a save").is_err());
        let mut s = FreeRoamSave {
            version: VERSION + 1,
            seed: 42,
            mech: "mechs/foreman.ron".into(),
            place: place(),
        };
        assert!(FreeRoamSave::from_ron(&s.to_ron()).is_err(), "newer than the game");
        s.version = VERSION;
        s.place.lost.pop();
        assert!(FreeRoamSave::from_ron(&s.to_ron()).is_err(), "damage lists disagree");
    }

    #[test]
    fn a_place_with_a_broken_number_is_refused() {
        let mut p = place();
        p.mech_heading_rad = f32::NAN;
        assert!(p.check().is_err());
    }
}
