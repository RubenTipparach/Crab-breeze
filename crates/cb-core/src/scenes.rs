//! The test scenes' stored spots (`data/scenes/`; owner, 2026-10-30: "why arent you storing coordinates of good
//! spots?"): places in a world found good once, kept by world seed so a scene goes straight there.

use crate::data::{Validate, finite};
use serde::Deserialize;
use std::collections::BTreeMap;

/// One spot for the nuke test scene: where the blast goes off and where the pilot watches from, x and z in metres.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NukeSpot {
    /// The blast's ground point.
    pub blast: (f32, f32),
    /// Where the pilot stands.
    pub watcher: (f32, f32),
}

/// `data/scenes/nuke.ron`: the nuke test scene's spot for each world seed.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NukeSpots {
    /// By world seed.
    pub spots: BTreeMap<u64, NukeSpot>,
}

impl Validate for NukeSpots {
    fn validate(&self) -> Result<(), String> {
        for (seed, s) in &self.spots {
            for (k, v) in [
                ("blast.x", s.blast.0),
                ("blast.z", s.blast.1),
                ("watcher.x", s.watcher.0),
                ("watcher.z", s.watcher.1),
            ] {
                finite(&format!("spots.{seed}.{k}"), v)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_nuke_spot_for_seed_42_loads() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/scenes/nuke.ron");
        let s: NukeSpots = crate::data::load_ron(&p).unwrap();
        let spot = s.spots.get(&42).expect("seed 42 has a spot");
        let (b, w) = (spot.blast, spot.watcher);
        let d = ((b.0 - w.0).powi(2) + (b.1 - w.1).powi(2)).sqrt();
        assert!((d - 2500.0).abs() < 5.0, "the watcher 2.5 km off: {d}");
    }
}
