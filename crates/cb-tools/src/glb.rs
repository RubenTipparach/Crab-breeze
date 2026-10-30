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

    /// The bounds (least and greatest corner) of every mesh under the node `under` at rest, model space; `None` if
    /// there is no such node or it holds no mesh.
    pub fn bounds_under(&self, under: &str) -> Option<(Vec3, Vec3)> {
        let top = *self.index().get(under)?;
        let nodes = self.json["nodes"].as_array().cloned().unwrap_or_default();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for (i, nd) in nodes.iter().enumerate() {
            let mut p = Some(i);
            while let Some(k) = p.filter(|k| *k != top) {
                p = self.parent[k];
            }
            if p != Some(top) {
                continue;
            }
            let Some(mesh) = nd["mesh"].as_u64() else { continue };
            for prim in self.json["meshes"][mesh as usize]["primitives"].as_array().into_iter().flatten() {
                let Some(acc) = prim["attributes"]["POSITION"].as_u64() else {
                    continue;
                };
                let a = &self.json["accessors"][acc as usize];
                let (Some(l), Some(h)) = (vec3(&a["min"]), vec3(&a["max"])) else {
                    continue;
                };
                for x in [l.x, h.x] {
                    for y in [l.y, h.y] {
                        for z in [l.z, h.z] {
                            let w = self.world[i].transform_point3(Vec3::new(x, y, z));
                            lo = lo.min(w);
                            hi = hi.max(w);
                        }
                    }
                }
            }
        }
        (lo.x <= hi.x).then_some((lo, hi))
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

/// The six MFD screens every cockpit carries, the Foreman's (openspec change `cockpit-parity`): three hung above the
/// front screen, three on the dash. `cb-app::mfd` binds them by these names.
pub const COCKPIT_SCREENS: [&str; 6] = [
    "HUD_mfd_0_screen",
    "HUD_mfd_1_screen",
    "HUD_mfd_2_screen",
    "SCREEN_left_screen",
    "SCREEN_center_screen",
    "SCREEN_right_screen",
];

/// The fewest lamps (`LIGHT_*`) a cockpit lights its cabin with.
pub const COCKPIT_MIN_LAMPS: usize = 4;

/// Every way an interior falls short of the Foreman's cockpit: a missing MFD screen, eye, seat, lamps, or a part the
/// mech's data asks to ride its chest.
pub fn cockpit_problems(tree: &Tree, interior: &cb_core::mech::config::InteriorConfig) -> Vec<String> {
    let idx = tree.index();
    let mut out: Vec<String> = COCKPIT_SCREENS
        .iter()
        .chain(["EYE_pilot", "SEAT_pilot"].iter())
        .filter(|n| !idx.contains_key(*n))
        .map(|n| format!("no `{n}`"))
        .collect();
    let lamps = tree.names.iter().filter(|n| n.starts_with("LIGHT_")).count();
    if lamps < COCKPIT_MIN_LAMPS {
        out.push(format!("{lamps} `LIGHT_*` lamps, fewer than {COCKPIT_MIN_LAMPS}"));
    }
    for r in &interior.rides {
        if !idx.contains_key(r.part.as_str()) {
            out.push(format!("no `{}` to ride `{}`", r.part, r.joint));
        }
    }
    out
}

/// Every vent and hatch motion in a boarding timeline whose node is in neither the mech's model nor its interior
/// (openspec change `boarding-vents`): a timeline left stale by a rebuild.
pub fn boarding_problems(tl: &cb_core::boarding::BoardingTimeline, trees: &[&Tree]) -> Vec<String> {
    let found = |pattern: &str| {
        trees.iter().any(|t| {
            t.names.iter().any(|n| match pattern.strip_suffix('*') {
                Some(prefix) => n.starts_with(prefix),
                None => n == pattern,
            })
        })
    };
    let mut out = Vec::new();
    for s in &tl.stages {
        let nodes = s.motions.iter().map(|m| &m.node).chain(s.vents.iter().map(|v| &v.node));
        for n in nodes {
            if !found(n) {
                out.push(format!("boarding stage `{}`: no node `{n}` in the mech or its interior", s.name));
            }
        }
    }
    out
}

/// Checks a mech file's model (and its interior) under `assets`: the contract always; the eye, hip and height
/// too for a mech generated from its glb (one that carries an interior, `tools/mechs/gen_mech_data.py`); the
/// interior's cockpit (its screens, eye, seat, lamps and riding parts), and for such a mech the nodes its boarding
/// moves and vents from, when given.
pub fn check_mech(cfg: &MechConfig, boarding: Option<&cb_core::boarding::BoardingTimeline>, assets: &Path) -> Vec<String> {
    let Some(model) = &cfg.model else { return Vec::new() };
    let path = assets.join(model);
    let tree = match Tree::read(&path) {
        Ok(t) => t,
        Err(e) => return vec![e],
    };
    let mut out: Vec<String> = tree.contract_problems().into_iter().map(|p| format!("{model}: {p}")).collect();
    // Every gun fires from a node the model has: a contract muzzle, or its own (a back unit's barrel, a pod's
    // mouth, `signature-weapons`). A missile without a pod leaves from the right arm, so `MISSILE_POD` may be absent.
    for w in &cfg.weapons {
        if w.socket != "MISSILE_POD" && !tree.names.contains(&w.socket) {
            out.push(format!("weapon `{}`: {model} has no node `{}` to fire from", w.name, w.socket));
        }
    }
    let mut cabin = None;
    if let Some(interior) = &cfg.interior {
        match Tree::read(&assets.join(&interior.model)) {
            Ok(t) => {
                out.extend(
                    cockpit_problems(&t, interior)
                        .into_iter()
                        .map(|p| format!("{}: {p}", interior.model)),
                );
                cabin = Some(t);
            }
            Err(e) => out.push(format!("interior.model: {e}")),
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
    // The boarding's nodes, for a mech whose timeline is generated from its glb and its interior. The retired
    // Breeze's hand-written timeline names hatch nodes that only its separate pod glb carried (`requests.md`, item 3),
    // so it is left out, as its sizes are.
    if let (Some(tl), Some(cabin)) = (boarding, cabin.as_ref()) {
        out.extend(boarding_problems(tl, &[&tree, cabin]));
    }
    out
}

/// Checks every enemy skin (openspec change `blockout-enemies`): its model is under `assets` and carries the
/// limb nodes (`LEG_*`, `ARM_*`) the game walks and throws, so a skin cannot ship without legs to shoot off.
pub fn check_skins(skins: &cb_core::skins::Skins, assets: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (name, spec) in &skins.skins {
        match Tree::read(&assets.join(&spec.model)) {
            Ok(t) => {
                if !t.names.iter().any(|n| n.starts_with("LEG_")) {
                    out.push(format!("skin `{name}`: {} has no LEG_ node", spec.model));
                }
            }
            Err(e) => out.push(format!("skin `{name}`: {e}")),
        }
    }
    out
}

/// Checks the Stratostacean's model against its data (openspec change `crab-airship` section 9): every part in
/// `fortress_shape.ron` has its `PART_<id>` node, every lift thruster its `LEG_<id>`, every turning gun its
/// `YAW_`, `PITCH_` and `MUZZLE_` nodes, the spikes are `DETAIL_` meshes (drawn only up close), and every image
/// the model points at outside itself is on disk.
pub fn check_fortress(d: &cb_core::fortress::FortressData, assets: &Path) -> Vec<String> {
    let path = assets.join(&d.shape.model);
    let t = match Tree::read(&path) {
        Ok(t) => t,
        Err(e) => return vec![format!("fortress: {e}")],
    };
    let mut out = Vec::new();
    let has = |n: &str| t.names.iter().any(|x| x == n);
    for (i, p) in d.shape.parts.iter().enumerate() {
        let mut want = vec![format!("PART_{}", p.id)];
        if p.kind == "lift" {
            want.push(format!("LEG_{}", p.id));
        }
        if d.kind(i).gun.as_ref().is_some_and(|g| g.turns) {
            want.extend(["YAW", "PITCH", "MUZZLE"].map(|k| format!("{k}_{}", p.id)));
        }
        for n in want.into_iter().filter(|n| !has(n)) {
            out.push(format!("fortress: {} has no node `{n}`", d.shape.model));
        }
    }
    // its detail level: the spikes the game draws only up close
    if !t.names.iter().any(|n| n.starts_with("DETAIL_")) {
        out.push(format!("fortress: {} has no DETAIL_ meshes (its spikes)", d.shape.model));
    }
    let dir = path.parent().unwrap_or(assets);
    for im in t.json["images"].as_array().into_iter().flatten() {
        if let Some(uri) = im["uri"].as_str().filter(|u| !u.starts_with("data:"))
            && !dir.join(uri).exists()
        {
            out.push(format!("fortress: {} points at {uri}, which is not there", d.shape.model));
        }
    }
    out
}

/// Checks every modelled civil aircraft (survey D8) against its data and its manifest: the glb is under `assets` with
/// its three levels (`LOD0`-`LOD2`), each with its landing gear (`GEAR_0`-`GEAR_2`); its full-detail level is its
/// kind's true length, span and height (gear on the ground at the kind's rest height) within half a metre; its sling
/// hook (`SLING_hook`) is where the data hangs a mech from; the sling's glb is there; and the manifest beside it
/// (`<name>.json`, written by the Blender build) keeps every level within its triangle budget.
pub fn check_aircraft(d: &cb_core::air_traffic::AirTrafficConfig, assets: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (name, k) in &d.kinds {
        let Some(m) = &k.model else { continue };
        let path = assets.join(&m.scene);
        let t = match Tree::read(&path) {
            Ok(t) => t,
            Err(e) => {
                out.push(format!("aircraft `{name}`: {e}"));
                continue;
            }
        };
        // An airliner built of damage zones (airliner-models): every zone, pivot and lamp its manifest names is a node,
        // and its size, standing on its gear, is its data's.
        if let Some(l) = m.loaded.as_ref().filter(|l| !l.zones.is_empty()) {
            let man = &l.manifest;
            let nodes = man
                .zones
                .iter()
                .map(|z| &z.node)
                .chain(man.rig.iter().map(|r| &r.node))
                .chain(man.lamps.iter().map(|l| &l.node));
            for n in nodes {
                if t.at(n).is_none() {
                    out.push(format!("aircraft `{name}`: {} has no node `{n}` its manifest names", m.scene));
                }
            }
            match t.bounds_under(name) {
                Some((lo, hi)) => {
                    let size = hi - lo;
                    let level = man.rest_pitch_deg == 0.0;
                    let mut checks = vec![("length", size.z, k.length_m), ("span", size.x, k.span_m)];
                    if level {
                        checks.push(("height", size.y, k.height_m));
                        checks.push(("ground under its gear", -lo.y, cb_core::air_traffic::rest_m(k)));
                    }
                    for (what, got, want) in checks {
                        if (got - want).abs() > 0.5 {
                            out.push(format!(
                                "aircraft `{name}`: its {what} is {got:.2} m in {} but {want:.2} m in its data; rebuild it",
                                m.scene
                            ));
                        }
                    }
                }
                None => out.push(format!("aircraft `{name}`: {} has no mesh under its root `{name}`", m.scene)),
            }
        } else {
            for n in ["LOD0", "LOD1", "LOD2", "GEAR_0", "GEAR_1", "GEAR_2"] {
                if t.at(n).is_none() {
                    out.push(format!("aircraft `{name}`: {} has no node `{n}`", m.scene));
                }
            }
            match t.bounds_under("LOD0") {
                Some((lo, hi)) => {
                    let size = hi - lo;
                    let rest = cb_core::air_traffic::rest_m(k);
                    for (what, got, want) in [
                        ("length", size.z, k.length_m),
                        ("span", size.x, k.span_m),
                        ("height", size.y, k.height_m),
                        ("ground under its gear", -lo.y, rest),
                    ] {
                        if (got - want).abs() > 0.5 {
                            out.push(format!(
                                "aircraft `{name}`: its {what} is {got:.2} m in {} but {want:.2} m in its data; rebuild it",
                                m.scene
                            ));
                        }
                    }
                }
                None => out.push(format!("aircraft `{name}`: {} has no mesh under LOD0", m.scene)),
            }
        }
        if let Some(sling) = &m.sling {
            match t.at("SLING_hook") {
                Some(at) if at.distance(Vec3::from_array(m.sling_hook_m)) <= 0.01 => {}
                Some(at) => out.push(format!(
                    "aircraft `{name}`: SLING_hook is at {at} in {} but {:?} in its data",
                    m.scene, m.sling_hook_m
                )),
                None => out.push(format!("aircraft `{name}`: {} has no node `SLING_hook`", m.scene)),
            }
            if !assets.join(sling).exists() {
                out.push(format!("aircraft `{name}`: its sling {sling} is not there"));
            }
        }
        let manifest = path.with_extension("json");
        let lods = std::fs::read_to_string(&manifest)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v["lods"].as_array().cloned());
        match lods {
            Some(lods) if lods.len() == 3 => {
                for l in lods {
                    let (tris, budget) = (
                        l["triangles"].as_u64().unwrap_or(u64::MAX),
                        l["budget_triangles"].as_u64().unwrap_or(0),
                    );
                    if tris > budget {
                        out.push(format!(
                            "aircraft `{name}`: LOD{} has {tris} triangles over its budget {budget}",
                            l["lod"]
                        ));
                    }
                }
            }
            _ => out.push(format!("aircraft `{name}`: {} has no three-level manifest", manifest.display())),
        }
    }
    out
}

/// The nodes every car model carries (car-traffic, car-driving): its sockets, the wheels that turn, and the
/// steering wheel and speedometer needle the dashboard turns from the simulation.
pub const CAR_NODES: [&str; 8] = [
    "EYE_driver",
    "DOOR_driver",
    "STEER_wheel",
    "NEEDLE_speed",
    "WHEEL_FL",
    "WHEEL_FR",
    "WHEEL_RL",
    "WHEEL_RR",
];

/// Checks every traffic car's model against its data (`data/cars/<kind>.ron`, raised by `car_height_scale`): the model
/// is under `assets`, it has every node in `CAR_NODES`, its `EYE_driver` and `DOOR_driver` sit where the data says, so
/// the driver's eye the game uses and the seat the model shows are one place, and `assets/models/cars.json` lists it
/// built at the data's size, so a model left behind by a change to the height scale is refused.
pub fn check_cars(d: &cb_core::traffic::TrafficData, assets: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let manifest_path = assets.join("models").join("cars.json");
    let manifest: Value = match std::fs::read(&manifest_path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => return vec![format!("cars: {}: {e}", manifest_path.display())],
    };
    for (name, spec) in d.cfg.kinds.iter().zip(&d.kinds) {
        let built: Vec<f32> = manifest[name.as_str()]["size_m"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_f64())
            .map(|v| v as f32)
            .collect();
        for (i, axis) in ["wide", "high", "long"].iter().enumerate() {
            let (want, got) = (spec.size_m[i], built.get(i).copied().unwrap_or(0.0));
            if (got - want).abs() > want * 0.01 {
                out.push(format!(
                    "car `{name}`: built {got} m {axis} (cars.json), {want} m in its data; rebuild with tools/blender/build_cars.py"
                ));
            }
        }
        let t = match Tree::read(&assets.join(&spec.model)) {
            Ok(t) => t,
            Err(e) => {
                out.push(format!("car `{name}`: {e}"));
                continue;
            }
        };
        for n in CAR_NODES.iter().filter(|n| t.at(n).is_none()) {
            out.push(format!("car `{name}`: {} has no node `{n}`", spec.model));
        }
        for (socket, want) in [("EYE_driver", spec.eye_driver_m), ("DOOR_driver", spec.door_driver_m)] {
            if let Some(at) = t.at(socket)
                && at.distance(Vec3::from_array(want)) > 0.01
            {
                out.push(format!(
                    "car `{name}`: {socket} is at {at} in {} but {want:?} in its data; rebuild with tools/blender/build_cars.py",
                    spec.model
                ));
            }
        }
    }
    out
}

/// Checks the boats' block-outs against their data (openspec change `harbours`, design section 9): every kind in
/// `ships.ron` names a model listed in `assets/models/boats.json` whose glb (and its LOD 1) is on disk and reads as
/// glTF, whose size measured off the export is within 5 % of the kind's `size_m`, and whose triangles are within each
/// LOD's budget; the harbour's crane and quay are listed too.
pub fn check_boats(ships: &cb_core::ships::ShipsConfig, assets: &Path) -> Vec<String> {
    let path = assets.join("models").join("boats.json");
    let manifest: Value = match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => return vec![format!("boats: {}: {e}", path.display())],
    };
    let rows = &manifest["boats"];
    let mut out = Vec::new();
    let check_row = |out: &mut Vec<String>, key: &str, row: &Value| {
        for f in ["file", "lod1"] {
            let Some(file) = row[f].as_str() else {
                out.push(format!("boats: {key} has no `{f}`"));
                continue;
            };
            if let Err(e) = Tree::read(&assets.join("models").join(file)) {
                out.push(format!("boats: {key}: {e}"));
            }
        }
        for lod in row["lods"].as_array().into_iter().flatten() {
            let (t, b) = (
                lod["triangles"].as_u64().unwrap_or(u64::MAX),
                lod["budget_triangles"].as_u64().unwrap_or(0),
            );
            if t > b {
                out.push(format!("boats: {key} LOD {} has {t} triangles, over its budget of {b}", lod["lod"]));
            }
        }
    };
    for (name, spec) in &ships.kinds {
        let file = spec.model.trim_start_matches("models/");
        let Some((key, row)) = rows
            .as_object()
            .and_then(|o| o.iter().find(|(_, r)| r["file"].as_str() == Some(file)))
        else {
            out.push(format!("boats: kind `{name}` names {}, which boats.json does not list", spec.model));
            continue;
        };
        check_row(&mut out, key, row);
        let size: Vec<f32> = row["size_m"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_f64())
            .map(|v| v as f32)
            .collect();
        for (i, axis) in ["across", "high", "long"].iter().enumerate() {
            let (want, got) = (spec.size_m[i], size.get(i).copied().unwrap_or(0.0));
            if (got - want).abs() > want * 0.05 {
                out.push(format!(
                    "boats: {name} is {got} m {axis} as built, {want} m in ships.ron (more than 5 % off)"
                ));
            }
        }
    }
    for piece in ["crane", "quay"] {
        match rows.get(piece) {
            Some(row) => check_row(&mut out, piece, row),
            None => out.push(format!("boats: boats.json has no `{piece}`")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_shipped_freighter_model_is_its_kinds_size_with_its_levels_and_its_hook() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let d: cb_core::air_traffic::AirTrafficConfig = cb_core::data::load_ron(&root.join("world/air_traffic.ron")).unwrap();
        assert!(d.kinds["freighter"].model.is_some(), "the freighter is modelled (survey D8)");
        assert_eq!(check_aircraft(&d, &root.join("../assets")), Vec::<String>::new());
    }

    #[test]
    fn every_shipped_car_model_has_its_nodes_where_its_data_says() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let d = cb_core::traffic::TrafficData::load(&root.join("data")).unwrap();
        let problems = check_cars(&d, &root.join("assets"));
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn the_shipped_fortress_model_has_every_node_its_data_names() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let d = cb_core::GameData::load(&root.join("data"), "mechs/placeholder.ron").unwrap();
        assert_eq!(check_fortress(&d.fortress, &root.join("assets")), Vec::<String>::new());
    }

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
            let d = cb_core::GameData::load(&root.join("data"), &mech).unwrap();
            let problems = check_mech(&d.mech, Some(&d.boarding), &root.join("assets"));
            assert!(problems.is_empty(), "{mech}: {problems:?}");
            checked += usize::from(d.mech.interior.is_some());
        }
        assert!(checked >= 8, "the eight detailed mechs are shipped ({checked} found)");
    }

    fn cockpit(names: &[&str]) -> Tree {
        let nodes: Vec<Value> = names.iter().map(|n| json!({ "name": n })).collect();
        let roots: Vec<usize> = (0..names.len()).collect();
        Tree::from_json(json!({ "nodes": nodes, "scenes": [{ "nodes": roots }] })).unwrap()
    }

    fn rides(part: &str) -> cb_core::mech::config::InteriorConfig {
        cb_core::mech::config::InteriorConfig {
            model: "models/cockpit_x/cockpit_x.gltf".into(),
            rides: vec![cb_core::mech::config::RideSpec {
                part: part.into(),
                joint: "CHEST_ramp_1".into(),
            }],
        }
    }

    const FULL: [&str; 13] = [
        "HUD_mfd_0_screen",
        "HUD_mfd_1_screen",
        "HUD_mfd_2_screen",
        "SCREEN_left_screen",
        "SCREEN_center_screen",
        "SCREEN_right_screen",
        "EYE_pilot",
        "SEAT_pilot",
        "LIGHT_screen",
        "LIGHT_overhead",
        "LIGHT_rear",
        "LIGHT_footwell",
        "INNER_ramp",
    ];

    #[test]
    fn a_cockpit_with_the_six_screens_eye_seat_and_lamps_passes() {
        assert!(cockpit_problems(&cockpit(&FULL), &rides("INNER_ramp")).is_empty());
    }

    #[test]
    fn a_cockpit_without_a_dash_is_refused_by_the_missing_screen() {
        let names: Vec<&str> = FULL.iter().copied().filter(|n| *n != "SCREEN_center_screen").collect();
        let p = cockpit_problems(&cockpit(&names), &rides("INNER_ramp"));
        assert_eq!(p, vec!["no `SCREEN_center_screen`".to_string()]);
    }

    #[test]
    fn a_cockpit_short_of_lamps_or_a_ridden_part_is_refused() {
        let names: Vec<&str> = FULL.iter().copied().filter(|n| *n != "LIGHT_rear").collect();
        let p = cockpit_problems(&cockpit(&names), &rides("INNER_canopy"));
        assert!(p.iter().any(|p| p.contains("lamps")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("INNER_canopy")), "{p:?}");
    }

    #[test]
    fn a_vent_on_a_node_the_models_lack_is_refused() {
        let mut tl = cb_core::boarding::BoardingTimeline::default();
        tl.stages.push(cb_core::boarding::Stage {
            name: "the seals break".into(),
            start_s: 3.1,
            duration_s: 1.3,
            vents: vec![
                cb_core::boarding::Vent {
                    node: "VENT_seam*".into(),
                    ..Default::default()
                },
                cb_core::boarding::Vent {
                    node: "VENT_gone".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let p = boarding_problems(&tl, &[&cockpit(&["VENT_seam_04"])]);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("VENT_gone"));
    }

    #[test]
    fn every_enemy_skin_names_a_shipped_model_with_legs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let d = cb_core::GameData::load(&root.join("data"), "mechs/placeholder.ron").unwrap();
        assert!(!d.skins.skins.is_empty());
        let problems = check_skins(&d.skins, &root.join("assets"));
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn every_boat_model_matches_its_data() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ships: cb_core::ships::ShipsConfig = cb_core::data::load_ron(&root.join("data/world/ships.ron")).unwrap();
        let problems = check_boats(&ships, &root.join("assets"));
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn a_boat_built_the_wrong_size_is_refused() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut ships: cb_core::ships::ShipsConfig = cb_core::data::load_ron(&root.join("data/world/ships.ron")).unwrap();
        ships.kinds.get_mut("freighter").unwrap().size_m[2] = 200.0;
        let problems = check_boats(&ships, &root.join("assets"));
        assert!(
            problems.iter().any(|p| p.contains("freighter") && p.contains("long")),
            "{problems:?}"
        );
    }
}
