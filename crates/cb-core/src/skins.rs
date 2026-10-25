//! Enemy skins (openspec change `blockout-enemies`): the block-out models an enemy wears over the stand-in's skeleton,
//! by name. Data only: the game hangs the model and walks its limbs (`cb-app::skins`). Lives in the core so the data
//! check and the encounters can refuse a skin that does not exist.

use crate::data::{Validate, positive};
use serde::Deserialize;
use std::collections::BTreeMap;

/// One skin: a block-out model with one node per leg and arm at its root joint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SkinSpec {
    /// The glb under `assets/` (`models/mech_blockouts/<id>.glb`).
    pub model: String,
    /// Scales the model; 1 keeps its built size.
    #[serde(default = "one")]
    pub scale: f32,
}

fn one() -> f32 {
    1.0
}

/// `data/enemies/skins.ron`: every skin, by name.
#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Skins {
    /// Skins by name.
    pub skins: BTreeMap<String, SkinSpec>,
}

impl Skins {
    /// Ok, or the first name in `names` that is not a skin, with where it was named.
    pub fn check_names<'a>(&self, names: impl IntoIterator<Item = (&'a str, String)>) -> Result<(), String> {
        for (name, at) in names {
            if !self.skins.contains_key(name) {
                return Err(format!("{at}: no skin `{name}` in data/enemies/skins.ron"));
            }
        }
        Ok(())
    }
}

impl Validate for Skins {
    fn validate(&self) -> Result<(), String> {
        for (name, s) in &self.skins {
            if s.model.is_empty() {
                return Err(format!("skins.{name}.model is empty"));
            }
            positive(&format!("skins.{name}.scale"), s.scale)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_named_but_not_listed_is_refused_by_name() {
        let skins: Skins = crate::data::parse_ron(
            r#"(skins: { "tyrant": (model: "models/mech_blockouts/tyrant.glb") })"#,
            std::path::Path::new("skins.ron"),
        )
        .unwrap();
        assert!(skins.check_names([("tyrant", "a".to_string())]).is_ok());
        let e = skins.check_names([("tyrnat", "encounters.enemies[0]".to_string())]).unwrap_err();
        assert!(e.contains("tyrnat") && e.contains("enemies[0]"), "{e}");
    }

    #[test]
    fn a_skin_with_no_scale_keeps_its_size_and_a_zero_scale_is_refused() {
        let ok: Skins = crate::data::parse_ron(r#"(skins: { "a": (model: "m.glb") })"#, std::path::Path::new("s")).unwrap();
        assert_eq!(ok.skins["a"].scale, 1.0);
        assert!(crate::data::parse_ron::<Skins>(r#"(skins: { "a": (model: "m.glb", scale: 0.0) })"#, std::path::Path::new("s")).is_err());
    }
}
