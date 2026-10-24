//! A mech glb read as data (openspec change `mech-roster` section 1): its node tree from the glTF JSON chunk,
//! checked against the rig contract the binder uses (`cb_core::mech::rig`) and measured at rest, so a shipped mech
//! file and the model it names cannot disagree. Lives in the tools: the game itself never parses a glb by hand.

use cb_core::mech::MechConfig;
use cb_core::mech::rig::{BONES, SOCKETS};
use glam::{Mat4, Quat, Vec3};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Sizes in a mech file must match the glb within this.
pub const SIZE_TOLERANCE_M: f32 = 0.05;

/// A model's node tree at rest, in model space (the scene's frame: +y up, +z forward).
pub struct Tree {
    names: Vec<String>,
    parent: Vec<Option<usize>>,
    world: Vec<Mat4>,
    json: Value,
}

impl Tree {
    /// Reads a `.glb` (its JSON chunk) or a `.gltf`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: Value = if bytes.starts_with(b"glTF") {
            let len = bytes
                .get(12..16)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
                .ok_or_else(|| format!("{}: truncated glb header", path.display()))?;
            let chunk = bytes
                .get(20..20 + len)
                .ok_or_else(|| format!("{}: truncated JSON chunk", path.display()))?;
            serde_json::from_slice(chunk).map_err(|e| format!("{}: {e}", path.display()))?
        } else {
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?
        };
        Self::from_json(json)
    }

    /// Builds the tree from glTF JSON.
    pub fn from_json(json: Value) -> Result<Self, String> {
        let nodes = json["nodes"].as_array().cloned().unwrap_or_default();
        let n = nodes.len();
        let names: Vec<String> = nodes.iter().map(|nd| nd["name"].as_str().unwrap_or("").to_string()).collect();
        let mut parent = vec![None; n];
        for (i, nd) in nodes.iter().enumerate() {
            for c in nd["children"].as_array().into_iter().flatten() {
                let c = c.as_u64().ok_or("a child index that is not a number")? as usize;
                if c >= n {
                    return Err(format!("node `{}` has a child {c} out of range", names[i]));
                }
                parent[c] = Some(i);
            }
        }
        let local: Vec<Mat4> = nodes.iter().map(local_matrix).collect();
        let mut world = vec![Mat4::IDENTITY; n];
        // parents before children: walk from each scene root
        let roots: Vec<usize> = json["scenes"][0]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_u64().map(|v| v as usize))
            .collect();
        let mut stack: Vec<(usize, Mat4)> = roots.into_iter().map(|r| (r, Mat4::IDENTITY)).collect();
        while let Some((i, up)) = stack.pop() {
            if i >= n {
                continue;
            }
            world[i] = up * local[i];
            for c in nodes[i]["children"].as_array().into_iter().flatten().filter_map(|v| v.as_u64()) {
                stack.push((c as usize, world[i]));
            }
        }
        Ok(Self {
            names,
            parent,
            world,
            json,
        })
    }

    fn index(&self) -> HashMap<&str, usize> {
        let mut m = HashMap::new();
        for (i, n) in self.names.iter().enumerate() {
            m.entry(n.as_str()).or_insert(i);
        }
        m
    }

    /// Every way the tree breaks the rig contract: a missing bone or socket, or one hung from the wrong parent.
    pub fn contract_problems(&self) -> Vec<String> {
        let idx = self.index();
        let mut out = Vec::new();
        for spec in BONES.iter().chain(SOCKETS.iter()) {
            let Some(&i) = idx.get(spec.name) else {
                out.push(format!("no `{}`", spec.name));
                continue;
            };
            let parent = self.parent[i].map(|p| self.names[p].as_str());
            if let Some(want) = spec.parent
                && parent != Some(want)
            {
                out.push(format!(
                    "`{}` hangs from `{}`, not `{want}`",
                    spec.name,
                    parent.unwrap_or("nothing")
                ));
            }
        }
        out
    }

    /// A node's position in model space at rest.
    pub fn at(&self, name: &str) -> Option<Vec3> {
        self.index().get(name).map(|&i| self.world[i].w_axis.truncate())
    }

    /// The highest point of any mesh at rest (its bounding boxes' corners in model space).
    pub fn top_m(&self) -> f32 {
        let mut top = f32::MIN;
        let nodes = self.json["nodes"].as_array().cloned().unwrap_or_default();
        for (i, nd) in nodes.iter().enumerate() {
            let Some(mesh) = nd["mesh"].as_u64() else { continue };
            for p in self.json["meshes"][mesh as usize]["primitives"].as_array().into_iter().flatten() {
                let Some(acc) = p["attributes"]["POSITION"].as_u64() else {
                    continue;
                };
                let a = &self.json["accessors"][acc as usize];
                let (Some(lo), Some(hi)) = (vec3(&a["min"]), vec3(&a["max"])) else {
                    continue;
                };
                for x in [lo.x, hi.x] {
                    for y in [lo.y, hi.y] {
                        for z in [lo.z, hi.z] {
                            top = top.max(self.world[i].transform_point3(Vec3::new(x, y, z)).y);
                        }
                    }
                }
            }
        }
        top
    }
}

fn vec3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    Some(Vec3::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ))
}

fn local_matrix(nd: &Value) -> Mat4 {
    if let Some(m) = nd["matrix"].as_array() {
        let f: Vec<f32> = m.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
        if f.len() == 16 {
            return Mat4::from_cols_slice(&f);
        }
    }
    let t = vec3(&nd["translation"]).unwrap_or(Vec3::ZERO);
    let s = vec3(&nd["scale"]).unwrap_or(Vec3::ONE);
    let r = nd["rotation"]
        .as_array()
        .filter(|a| a.len() == 4)
        .map(|a| {
            let g = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
            Quat::from_xyzw(g(0), g(1), g(2), g(3))
        })
        .unwrap_or(Quat::IDENTITY);
    Mat4::from_scale_rotation_translation(s, r, t)
}

/// Checks a mech file's model (and its interior) under `assets`: the contract always; the eye, hip and height
/// too for a mech generated from its glb (one that carries an interior, `tools/mechs/gen_mech_data.py`).
pub fn check_mech(cfg: &MechConfig, assets: &Path) -> Vec<String> {
    let Some(model) = &cfg.model else { return Vec::new() };
    let path = assets.join(model);
    let tree = match Tree::read(&path) {
        Ok(t) => t,
        Err(e) => return vec![e],
    };
    let mut out: Vec<String> = tree.contract_problems().into_iter().map(|p| format!("{model}: {p}")).collect();
    if let Some(interior) = &cfg.interior {
        if !assets.join(&interior.model).exists() {
            out.push(format!("interior.model: `{}` is not under assets/", interior.model));
        }
        let near = |what: &str, file: f32, glb: Option<f32>| match glb {
            Some(g) if (g - file).abs() <= SIZE_TOLERANCE_M => None,
            Some(g) => Some(format!(
                "{what} is {file:.2} m in the file but {g:.2} m in {model}: regenerate the data"
            )),
            None => Some(format!("{what}: {model} has no node to measure it by")),
        };
        out.extend(near(
            "locomotion.hip_height_m",
            cfg.locomotion.hip_height_m,
            tree.at("pelvis").map(|p| p.y),
        ));
        out.extend(near("body.eye_height_m", cfg.body.eye_height_m, tree.at("EYE_pilot").map(|p| p.y)));
        out.extend(near("locomotion.body_height_m", cfg.locomotion.body_height_m, Some(tree.top_m())));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn contract_tree(skip: &str) -> Value {
        let names: Vec<(&str, Option<&str>)> = BONES
            .iter()
            .chain(SOCKETS.iter())
            .filter(|b| b.name != skip)
            .map(|b| (b.name, b.parent))
            .collect();
        let index: HashMap<&str, usize> = names.iter().enumerate().map(|(i, (n, _))| (*n, i)).collect();
        let nodes: Vec<Value> = names
            .iter()
            .map(|(n, _)| {
                let kids: Vec<usize> = names
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, p))| *p == Some(*n))
                    .map(|(i, _)| i)
                    .collect();
                json!({ "name": n, "children": kids })
            })
            .collect();
        json!({ "nodes": nodes, "scenes": [{ "nodes": [index["root"]] }] })
    }

    #[test]
    fn a_tree_with_every_contract_node_passes() {
        let t = Tree::from_json(contract_tree("")).unwrap();
        assert!(t.contract_problems().is_empty(), "{:?}", t.contract_problems());
    }

    #[test]
    fn a_tree_missing_a_socket_is_refused_by_name() {
        let t = Tree::from_json(contract_tree("LADDER_base")).unwrap();
        let p = t.contract_problems();
        assert!(p.iter().any(|p| p.contains("LADDER_base")), "{p:?}");
    }

    #[test]
    fn every_shipped_mech_glb_follows_the_contract_and_matches_its_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut checked = 0;
        for mech in cb_core::GameData::mech_files(&root.join("data")) {
            let cfg: MechConfig = cb_core::data::load_ron(&root.join("data").join(&mech)).unwrap();
            let problems = check_mech(&cfg, &root.join("assets"));
            assert!(problems.is_empty(), "{mech}: {problems:?}");
            checked += usize::from(cfg.interior.is_some());
        }
        assert!(checked >= 8, "the eight detailed mechs are shipped ({checked} found)");
    }
}
