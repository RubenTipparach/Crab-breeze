//! A modelled aircraft's manifest (openspec change `airliner-models`): the JSON its Blender generator writes beside
//! the glb (`tools/blender/build_airliners.py`), read once at load so the simulation and the drawing share one
//! description of the aircraft: where it rests on its gear, its damage zones (the part tree, health shares and hit
//! capsules measured off the meshes), its moving parts and its lamps. Only what the game reads is parsed; the rest of
//! the manifest (triangles, bounds, hashes) is for `cb-tools validate`.
//!
//! Frame: the model's, the shape frame's (`shape`): x to the right wing, y up, the nose toward -z, the origin on the
//! fuselage's centre line half way along it.

use glam::Vec3;
use serde::Deserialize;
use std::path::Path;

/// What a zone is to the airframe, and so what losing it does (design section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneKind {
    /// The main box (the wing's carry-through, the middle of the fuselage): the root of the tree.
    Centre,
    /// A length of fuselage (the nose with the flight deck, a cabin section).
    Fuselage,
    /// The tail: the cone, the fin and the tailplanes.
    Tail,
    /// A wing from the root to the split.
    WingInner,
    /// A wing from the split to the tip.
    WingOuter,
    /// An engine (and, on the propliner, the main gear in its nacelle).
    Engine,
}

impl ZoneKind {
    /// The kind a zone's name says: `centre`, `nose`, `cabin_*`, `tail`, `wing_inner_*`, `wing_outer_*`, `engine_*`.
    pub fn of(name: &str) -> Option<Self> {
        Some(match name {
            "centre" => Self::Centre,
            "nose" => Self::Fuselage,
            "tail" => Self::Tail,
            n if n.starts_with("cabin_") => Self::Fuselage,
            n if n.starts_with("wing_inner_") => Self::WingInner,
            n if n.starts_with("wing_outer_") => Self::WingOuter,
            n if n.starts_with("engine_") => Self::Engine,
            _ => return None,
        })
    }

    /// True when losing it brings the aircraft down: every zone but an engine (losing one engine leaves it flying on
    /// the others, slower).
    pub fn downs(self) -> bool {
        self != Self::Engine
    }
}

/// A hit capsule in the model's frame.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelCapsule {
    /// One end, metres.
    pub a: [f32; 3],
    /// The other end, metres.
    pub b: [f32; 3],
    /// Radius, metres.
    pub radius_m: f32,
}

/// One damage zone as the manifest writes it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ZoneEntry {
    /// Its name (`centre`, `wing_outer_l`, ...).
    pub name: String,
    /// Its node in the glb (`ZONE_<name>`).
    pub node: String,
    /// The zone it hangs from; none for the centre.
    pub parent: Option<String>,
    /// Its share of the kind's `hp`.
    pub share: f32,
    /// Its hit capsules, measured off its own meshes by the build.
    pub capsules: Vec<ModelCapsule>,
}

/// A moving part as the manifest writes it (design section 2): a pivot node turning about `axis`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RigEntry {
    /// The pivot node (`RIG_<what>`).
    pub node: String,
    /// What it is: `gear_leg`, `gear_door`, `prop`, `fan`, `aileron`, `elevator`, `rudder`, `flap`.
    pub role: String,
    /// The zone it hangs in.
    pub zone: String,
    /// Its axis in the model's frame.
    pub axis: [f32; 3],
    /// A gear part: the angle at the end of its travel, degrees.
    #[serde(default)]
    pub angle_deg: f32,
    /// A gear part: its share of `angle_deg` over the gear sequence's time, `[t, share]` from 0 (down) to 1 (up).
    #[serde(default)]
    pub keys: Vec<[f32; 2]>,
    /// A prop or fan: its speed at cruise, revolutions a minute.
    #[serde(default)]
    pub rpm: f32,
    /// A control surface: most it moves either way, degrees.
    #[serde(default)]
    pub range_deg: f32,
    /// A flap: its settings, degrees.
    #[serde(default)]
    pub detents_deg: Vec<f32>,
}

impl RigEntry {
    /// A gear part's share of its travel at gear time `t` (0 down and locked, 1 up and shut), eased between keys.
    pub fn gear_share(&self, t: f32) -> f32 {
        let k = &self.keys;
        let Some(first) = k.first() else { return t.clamp(0.0, 1.0) };
        if t <= first[0] {
            return first[1];
        }
        for w in k.windows(2) {
            let ([t0, v0], [t1, v1]) = (w[0], w[1]);
            if t <= t1 {
                let u = ((t - t0) / (t1 - t0).max(1e-6)).clamp(0.0, 1.0);
                return v0 + (v1 - v0) * (u * u * (3.0 - 2.0 * u));
            }
        }
        k[k.len() - 1][1]
    }
}

/// A lamp as the manifest writes it (design section 3).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LampEntry {
    /// The lamp's empty (`LAMP_<what>`); its lit mesh is `light_<what>.L0` and `.L1`.
    pub node: String,
    /// The zone it is in.
    pub zone: String,
    /// `red`, `green` or `white`.
    pub colour: String,
    /// `steady`, `beacon`, `strobe` or `landing`.
    pub pattern: String,
}

/// The lamps' flashing patterns.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct LightPatterns {
    /// The beacons: once every `period_s` for `on_s`.
    #[serde(default)]
    pub beacon: Flash,
    /// The strobes: `flashes` of `on_s`, starting `gap_s` apart, every `period_s`.
    #[serde(default)]
    pub strobe: Flash,
}

/// A flashing pattern.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub struct Flash {
    /// Seconds between the starts of two flashes (or groups).
    #[serde(default)]
    pub period_s: f32,
    /// Each flash, seconds.
    #[serde(default)]
    pub on_s: f32,
    /// From the start of one flash of a group to the start of the next, seconds.
    #[serde(default)]
    pub gap_s: f32,
    /// Flashes in a group (0 or 1: one).
    #[serde(default)]
    pub flashes: u32,
}

impl Flash {
    /// True while the lamp is lit `t_s` into its pattern.
    pub fn lit(&self, t_s: f32) -> bool {
        if self.period_s.is_nan() || self.period_s <= 0.0 {
            return true;
        }
        let u = t_s.rem_euclid(self.period_s);
        (0..self.flashes.max(1)).any(|i| {
            let start = i as f32 * self.gap_s;
            u >= start && u < start + self.on_s
        })
    }
}

/// The part of a model's manifest the game reads.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ModelManifest {
    /// The origin's height over the ground standing on its gear, metres.
    #[serde(default)]
    pub rest_m: Option<f32>,
    /// Nose up standing on its gear (a taildragger), degrees.
    #[serde(default)]
    pub rest_pitch_deg: f32,
    /// The damage zones, the centre first.
    #[serde(default)]
    pub zones: Vec<ZoneEntry>,
    /// The moving parts.
    #[serde(default)]
    pub rig: Vec<RigEntry>,
    /// The lamps.
    #[serde(default)]
    pub lamps: Vec<LampEntry>,
    /// How the lamps flash.
    #[serde(default)]
    pub light_patterns: LightPatterns,
    /// The gear's sequence, seconds from down to up.
    #[serde(default)]
    pub gear_sequence_s: f32,
}

/// One damage zone as the simulation keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageZone {
    /// Its name.
    pub name: String,
    /// What it is to the airframe.
    pub kind: ZoneKind,
    /// The index of the zone it hangs from; none for the centre.
    pub parent: Option<usize>,
    /// Its share of the kind's `hp`.
    pub share: f32,
    /// Its hit capsules in the model's frame: (a, b, radius).
    pub capsules: Vec<(Vec3, Vec3, f32)>,
}

impl ModelManifest {
    /// Reads the manifest beside a model's glb (`scene` under `assets`, its `.json`).
    pub fn load(assets: &Path, scene: &str) -> Result<Self, String> {
        let path = assets.join(scene).with_extension("json");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let m: Self = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        m.zones().map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(r) = m.rest_m
            && !(r > 0.0 && r.is_finite())
        {
            return Err(format!("{}: rest_m must be above zero", path.display()));
        }
        Ok(m)
    }

    /// The damage zones with their parents as indices: every zone a known kind, the centre the one root, every parent
    /// a zone before it, every share above zero, every capsule finite.
    pub fn zones(&self) -> Result<Vec<DamageZone>, String> {
        let mut out: Vec<DamageZone> = Vec::with_capacity(self.zones.len());
        for z in &self.zones {
            let kind = ZoneKind::of(&z.name).ok_or_else(|| format!("zone `{}` is not a known kind", z.name))?;
            let parent = match &z.parent {
                None if kind == ZoneKind::Centre => None,
                None => return Err(format!("zone `{}` hangs from nothing", z.name)),
                Some(p) => Some(
                    out.iter()
                        .position(|q| &q.name == p)
                        .ok_or_else(|| format!("zone `{}` hangs from `{p}`, not a zone listed before it", z.name))?,
                ),
            };
            if !(z.share > 0.0 && z.share.is_finite()) {
                return Err(format!("zone `{}`: share must be above zero", z.name));
            }
            let mut capsules = Vec::with_capacity(z.capsules.len());
            for c in &z.capsules {
                let (a, b) = (Vec3::from_array(c.a), Vec3::from_array(c.b));
                if !(a.is_finite() && b.is_finite() && c.radius_m > 0.0 && c.radius_m.is_finite()) {
                    return Err(format!("zone `{}`: a capsule is not finite", z.name));
                }
                capsules.push((a, b, c.radius_m));
            }
            out.push(DamageZone {
                name: z.name.clone(),
                kind,
                parent,
                share: z.share,
                capsules,
            });
        }
        if !out.is_empty() && out[0].kind != ZoneKind::Centre {
            return Err("the first zone must be the centre".into());
        }
        Ok(out)
    }
}

/// A kind's model as the game holds it once loaded: its manifest and its damage zones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadedModel {
    /// The manifest.
    pub manifest: ModelManifest,
    /// Its damage zones (empty: the aircraft takes hits as a whole).
    pub zones: Vec<DamageZone>,
}

impl LoadedModel {
    /// Reads a model's manifest and its zones.
    pub fn load(assets: &Path, scene: &str) -> Result<Self, String> {
        let manifest = ModelManifest::load(assets, scene)?;
        let zones = manifest.zones()?;
        Ok(Self { manifest, zones })
    }
}

/// Every zone that comes away when `zone` breaks: it and all that hang from it, in the zones' order.
pub fn falls_with(zones: &[DamageZone], zone: usize) -> Vec<usize> {
    (0..zones.len())
        .filter(|&i| {
            let mut z = Some(i);
            while let Some(j) = z {
                if j == zone {
                    return true;
                }
                z = zones[j].parent;
            }
            false
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped(kind: &str) -> ModelManifest {
        let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        ModelManifest::load(&assets, &format!("models/aircraft/{kind}.glb")).unwrap()
    }

    #[test]
    fn every_airliner_manifest_reads_with_a_whole_part_tree() {
        for kind in ["propliner", "airliner", "widebody"] {
            let m = shipped(kind);
            let z = m.zones().unwrap();
            assert_eq!(z[0].kind, ZoneKind::Centre, "{kind}");
            assert!(z.iter().skip(1).all(|q| q.parent.is_some()), "{kind}: one root");
            assert!(z.iter().all(|q| !q.capsules.is_empty()), "{kind}: a capsule for every zone");
            assert!(
                z.iter().filter(|q| q.kind == ZoneKind::Engine).count() >= 2,
                "{kind}: its engines are zones"
            );
            assert!(m.rest_m.unwrap() > 1.0, "{kind}");
            assert!(m.rig.iter().any(|r| r.role == "gear_leg"), "{kind}: its gear moves");
            assert!(m.lamps.iter().any(|l| l.pattern == "beacon"), "{kind}: a beacon");
        }
    }

    #[test]
    fn breaking_a_wing_root_takes_the_outer_wing_and_its_engines_with_it() {
        let z = shipped("airliner").zones().unwrap();
        let at = |n: &str| z.iter().position(|q| q.name == n).unwrap();
        let gone: Vec<&str> = falls_with(&z, at("wing_inner_l")).into_iter().map(|i| z[i].name.as_str()).collect();
        assert!(gone.contains(&"wing_inner_l") && gone.contains(&"wing_outer_l"));
        assert!(gone.iter().filter(|n| n.starts_with("engine_")).count() == 2, "{gone:?}");
        assert!(!gone.contains(&"wing_inner_r") && !gone.contains(&"centre"));
    }

    #[test]
    fn the_gear_folds_on_its_keys_and_the_strobe_flashes_twice() {
        let m = shipped("airliner");
        let leg = m.rig.iter().find(|r| r.role == "gear_leg").unwrap();
        let door = m.rig.iter().find(|r| r.role == "gear_door").unwrap();
        assert_eq!(leg.gear_share(0.0), 0.0);
        assert_eq!(leg.gear_share(1.0), 1.0);
        assert_eq!(door.gear_share(0.5), 1.0, "a door stays open while the leg swings");
        assert_eq!(door.gear_share(1.0), 0.0, "and shuts once it is up");
        let s = m.light_patterns.strobe;
        let lit: Vec<bool> = [0.01, 0.1, 0.16, 0.25, 0.6].iter().map(|&t| s.lit(t)).collect();
        assert_eq!(lit, vec![true, false, true, false, false]);
    }

    #[test]
    fn a_tree_with_an_unknown_parent_is_refused() {
        let mut m = shipped("propliner");
        m.zones[2].parent = Some("nowhere".into());
        assert!(m.zones().unwrap_err().contains("nowhere"));
    }
}
