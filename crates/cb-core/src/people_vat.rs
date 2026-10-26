//! The people's vertex animation textures (openspec change `people-vat`, design section 3): the bake
//! manifest `assets/models/people/people_vat.json` and the `.vat` files `tools/blender/build_people.py`
//! writes, read and checked here with no engine, so a test can hold the bake to its manifest and its
//! budgets and the game reads exactly what was checked.
//!
//! A `.vat` file is, little-endian: the magic `CBVAT001`; vertices, indices and rows (`u32` each) and a
//! reserved `u32`; the bounds' least and most corner (`f32` x 6); the triangles (`u32` each); per vertex
//! its colour region and shade (`u8` each) and two zero bytes; then per row (one frame of one clip) per
//! vertex four `u16`: the position quantised over the bounds and the octahedral normal, 8 bits a
//! component. Positions are in person space: +Y up, the body facing -Z, feet at the origin.

use serde::Deserialize;
use std::collections::BTreeMap;

/// The magic a `.vat` file starts with.
pub const MAGIC: &[u8; 8] = b"CBVAT001";
/// Header bytes: magic, four `u32`, six `f32`.
pub const HEADER_BYTES: usize = 8 + 16 + 24;
/// Bytes per vertex per row: four `u16`.
pub const TEXEL_BYTES: usize = 8;

/// The bake manifest.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// What wrote it.
    #[serde(rename = "_comment")]
    pub comment: String,
    /// The rate the clips were sampled at, frames a second (each clip's own `fps` is exact).
    pub bake_fps: f32,
    /// The colour regions, in the order a vertex's region numbers them.
    pub regions: Vec<String>,
    /// Every clip, in row order; every level of every body holds them all at the same rows.
    pub clips: Vec<ClipInfo>,
    /// The bodies.
    pub bodies: Vec<BodyInfo>,
}

/// One baked clip.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipInfo {
    /// Its name (`idle`, `walk`, `run`, `sit`, `wave`, `talk`, `fall`).
    pub name: String,
    /// Its first row.
    pub row: u32,
    /// How many rows (frames) it has.
    pub frames: u32,
    /// Frames a second.
    pub fps: f32,
    /// True when it loops; a clip that does not holds its last frame.
    #[serde(rename = "loop")]
    pub looped: bool,
    /// For a moving clip, the ground speed it walks at when played at its own rate, by body, m/s.
    #[serde(default)]
    pub speed_m_s_by_body: BTreeMap<String, f32>,
}

impl ClipInfo {
    /// Its length, seconds.
    pub fn length_s(&self) -> f32 {
        if self.looped {
            self.frames as f32 / self.fps
        } else {
            (self.frames.max(1) - 1) as f32 / self.fps
        }
    }
}

/// One body.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyInfo {
    /// Its id (`man`, `woman`).
    pub id: String,
    /// `male` or `female`.
    pub sex: String,
    /// The baked body's standing height, metres.
    pub height_m: f32,
    /// The skinned glb with every clip (`assets/models/people/`).
    pub glb: String,
    /// The glb's SHA-256.
    pub glb_sha256: String,
    /// The pelvis's height above the feet sitting, metres.
    pub sitting_pelvis_m: f32,
    /// The pelvis's height above the feet standing, metres.
    pub standing_pelvis_m: f32,
    /// Its levels of detail, nearest first.
    pub levels: Vec<LevelInfo>,
}

/// One level of detail of one body.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LevelInfo {
    /// `near` or `crowd`.
    pub name: String,
    /// Its `.vat` file.
    pub file: String,
    /// Vertices.
    pub vertices: u32,
    /// Triangles.
    pub triangles: u32,
    /// Rows (every clip's frames).
    pub rows: u32,
    /// The bounds' least corner, metres.
    pub bounds_min: [f32; 3],
    /// The bounds' most corner, metres.
    pub bounds_max: [f32; 3],
    /// The file's size.
    pub bytes: u64,
    /// The file's SHA-256.
    pub sha256: String,
    /// The most triangles it may have.
    pub budget_triangles: u32,
}

/// A `.vat` file read.
#[derive(Clone, Debug, PartialEq)]
pub struct VatLevel {
    /// Vertices.
    pub vertices: u32,
    /// Rows.
    pub rows: u32,
    /// The bounds' least corner.
    pub bounds_min: [f32; 3],
    /// The bounds' most corner.
    pub bounds_max: [f32; 3],
    /// The triangles, three vertex indices each.
    pub indices: Vec<u32>,
    /// Each vertex's colour region.
    pub region: Vec<u8>,
    /// Each vertex's shade, 0-255 for 0-2.
    pub shade: Vec<u8>,
    /// The texels, row by row, vertex by vertex, four `u16` each, as bytes (little-endian).
    pub texels: Vec<u8>,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(b, at))
}

impl VatLevel {
    /// Reads a `.vat` file, refusing one whose sizes do not add up or whose triangles name a vertex it
    /// does not have.
    pub fn parse(b: &[u8]) -> Result<Self, String> {
        if b.len() < HEADER_BYTES || &b[..8] != MAGIC {
            return Err("not a .vat file (magic)".into());
        }
        let (n, ni, rows) = (u32_at(b, 8) as usize, u32_at(b, 12) as usize, u32_at(b, 16) as usize);
        let lo = [f32_at(b, 24), f32_at(b, 28), f32_at(b, 32)];
        let hi = [f32_at(b, 36), f32_at(b, 40), f32_at(b, 44)];
        let want = HEADER_BYTES + ni * 4 + n * 4 + rows * n * TEXEL_BYTES;
        if b.len() != want {
            return Err(format!("size {} where the header says {want}", b.len()));
        }
        if ni % 3 != 0 {
            return Err(format!("{ni} indices is not whole triangles"));
        }
        if lo.iter().zip(&hi).any(|(l, h)| !l.is_finite() || !h.is_finite() || l >= h) {
            return Err(format!("bounds {lo:?} to {hi:?}"));
        }
        let mut at = HEADER_BYTES;
        let indices: Vec<u32> = (0..ni).map(|i| u32_at(b, at + i * 4)).collect();
        if let Some(bad) = indices.iter().find(|&&i| i as usize >= n) {
            return Err(format!("a triangle names vertex {bad} of {n}"));
        }
        at += ni * 4;
        let region = (0..n).map(|i| b[at + i * 4]).collect();
        let shade = (0..n).map(|i| b[at + i * 4 + 1]).collect();
        at += n * 4;
        Ok(Self {
            vertices: n as u32,
            rows: rows as u32,
            bounds_min: lo,
            bounds_max: hi,
            indices,
            region,
            shade,
            texels: b[at..].to_vec(),
        })
    }

    /// Triangles.
    pub fn triangles(&self) -> u32 {
        self.indices.len() as u32 / 3
    }

    /// Vertex `v`'s position in row `row`, metres, person space.
    pub fn position(&self, row: u32, v: u32) -> [f32; 3] {
        let at = ((row * self.vertices + v) as usize) * TEXEL_BYTES;
        let q = |k: usize| u16::from_le_bytes([self.texels[at + 2 * k], self.texels[at + 2 * k + 1]]) as f32 / 65535.0;
        std::array::from_fn(|k| self.bounds_min[k] + q(k) * (self.bounds_max[k] - self.bounds_min[k]))
    }
}

impl Manifest {
    /// Reads the manifest's JSON.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("people_vat.json: {e}"))
    }

    /// The clip named `name`.
    pub fn clip(&self, name: &str) -> Option<&ClipInfo> {
        self.clips.iter().find(|c| c.name == name)
    }

    /// The manifest's own rules: clips in row order with no gap, every level within its budget and as
    /// many rows as the clips hold, and every clip the game plays present.
    pub fn check(&self) -> Result<(), String> {
        let mut row = 0;
        for c in &self.clips {
            if c.row != row || c.frames == 0 || !(c.fps.is_finite() && c.fps > 0.0) {
                return Err(format!(
                    "clip {}: row {} frames {} fps {} (the next row is {row})",
                    c.name, c.row, c.frames, c.fps
                ));
            }
            row += c.frames;
        }
        for name in crate::civilians::anim::Clip::ALL.map(|c| c.name()) {
            if self.clip(name).is_none() {
                return Err(format!("no clip {name}"));
            }
        }
        if self.bodies.is_empty() {
            return Err("no bodies".into());
        }
        for b in &self.bodies {
            if b.levels.is_empty() {
                return Err(format!("{}: no levels", b.id));
            }
            for l in &b.levels {
                if l.triangles > l.budget_triangles {
                    return Err(format!(
                        "{} {}: {} triangles, over its budget of {}",
                        b.id, l.name, l.triangles, l.budget_triangles
                    ));
                }
                if l.rows != row {
                    return Err(format!("{} {}: {} rows, the clips hold {row}", b.id, l.name, l.rows));
                }
            }
            for c in self.clips.iter().filter(|c| !c.speed_m_s_by_body.is_empty()) {
                match c.speed_m_s_by_body.get(&b.id) {
                    Some(v) if v.is_finite() && *v > 0.1 => {}
                    other => return Err(format!("clip {}: {}'s speed is {other:?}", c.name, b.id)),
                }
            }
        }
        Ok(())
    }

    /// A level's file checked against its manifest entry: counts, rows and bounds.
    pub fn check_level(&self, level: &LevelInfo, vat: &VatLevel) -> Result<(), String> {
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-4);
        if vat.vertices != level.vertices || vat.triangles() != level.triangles || vat.rows != level.rows {
            return Err(format!(
                "{}: the file has {} vertices, {} triangles, {} rows; the manifest {}, {}, {}",
                level.file,
                vat.vertices,
                vat.triangles(),
                vat.rows,
                level.vertices,
                level.triangles,
                level.rows
            ));
        }
        if !close(vat.bounds_min, level.bounds_min) || !close(vat.bounds_max, level.bounds_max) {
            return Err(format!("{}: bounds differ from the manifest", level.file));
        }
        if let Some(r) = vat.region.iter().find(|&&r| r as usize >= self.regions.len()) {
            return Err(format!("{}: a vertex has region {r} of {}", level.file, self.regions.len()));
        }
        Ok(())
    }
}

/// From a glb: each mesh's name and triangles, and each animation's name and keyframe count.
pub fn glb_summary(glb: &[u8]) -> Result<(BTreeMap<String, u32>, BTreeMap<String, u32>), String> {
    if glb.len() < 20 || &glb[..4] != b"glTF" || &glb[16..20] != b"JSON" {
        return Err("not a glb".into());
    }
    let n = u32_at(glb, 12) as usize;
    let j: serde_json::Value = serde_json::from_slice(&glb[20..20 + n]).map_err(|e| e.to_string())?;
    let count =
        |acc: &serde_json::Value| -> u32 { acc.as_u64().and_then(|a| j["accessors"][a as usize]["count"].as_u64()).unwrap_or(0) as u32 };
    let mut meshes = BTreeMap::new();
    for m in j["meshes"].as_array().into_iter().flatten() {
        let tris = m["primitives"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| count(&p["indices"]) / 3)
            .sum();
        meshes.insert(m["name"].as_str().unwrap_or_default().to_string(), tris);
    }
    let mut clips = BTreeMap::new();
    for a in j["animations"].as_array().into_iter().flatten() {
        clips.insert(
            a["name"].as_str().unwrap_or_default().to_string(),
            count(&a["samplers"][0]["input"]),
        );
    }
    Ok((meshes, clips))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn people_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/people")
    }

    /// The shipped bake against its manifest and its glbs, all read from the files (never a copied value):
    /// the manifest's own rules and budgets, each `.vat` file's counts, rows and bounds, each level's
    /// triangles against the glb's mesh of that name, and every clip in the glb with a key a frame (a loop
    /// also keys its closing frame).
    #[test]
    fn the_shipped_bake_matches_its_manifest_and_its_glbs() {
        let dir = people_dir();
        let m = Manifest::parse(&std::fs::read_to_string(dir.join("people_vat.json")).unwrap()).unwrap();
        m.check().unwrap();
        for b in &m.bodies {
            let (meshes, clips) = glb_summary(&std::fs::read(dir.join(&b.glb)).unwrap()).unwrap();
            for l in &b.levels {
                let bytes = std::fs::read(dir.join(&l.file)).unwrap();
                assert_eq!(bytes.len() as u64, l.bytes, "{}", l.file);
                let vat = VatLevel::parse(&bytes).unwrap();
                m.check_level(l, &vat).unwrap();
                assert_eq!(
                    meshes.get(&format!("{}_{}", b.id, l.name)),
                    Some(&l.triangles),
                    "{} {}",
                    b.id,
                    l.name
                );
            }
            for c in &m.clips {
                let keys = c.frames + u32::from(c.looped);
                assert_eq!(clips.get(&c.name), Some(&keys), "{} {}", b.id, c.name);
            }
            // true relative scale: the baked body stands within a few centimetres of 1.75 m (the pilot is 1.8)
            assert!((b.height_m - 1.75).abs() < 0.02, "{}: {} m", b.id, b.height_m);
        }
    }

    fn manifest() -> Manifest {
        let level = LevelInfo {
            name: "near".into(),
            file: "a_near.vat".into(),
            vertices: 3,
            triangles: 1,
            rows: 0,
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            bytes: 0,
            sha256: String::new(),
            budget_triangles: 1,
        };
        let mut row = 0;
        let clips = crate::civilians::anim::Clip::ALL
            .iter()
            .map(|c| {
                let info = ClipInfo {
                    name: c.name().into(),
                    row,
                    frames: 2,
                    fps: 24.0,
                    looped: true,
                    speed_m_s_by_body: BTreeMap::new(),
                };
                row += 2;
                info
            })
            .collect();
        Manifest {
            comment: String::new(),
            bake_fps: 24.0,
            regions: vec!["skin".into()],
            clips,
            bodies: vec![BodyInfo {
                id: "a".into(),
                sex: "male".into(),
                height_m: 1.75,
                glb: "a.glb".into(),
                glb_sha256: String::new(),
                sitting_pelvis_m: 0.5,
                standing_pelvis_m: 0.9,
                levels: vec![LevelInfo { rows: row, ..level }],
            }],
        }
    }

    #[test]
    fn a_manifest_within_its_budgets_passes_and_one_triangle_over_is_refused() {
        let mut m = manifest();
        assert_eq!(m.check(), Ok(()));
        m.bodies[0].levels[0].triangles = 2;
        let e = m.check().unwrap_err();
        assert!(e.contains("over its budget"), "{e}");
    }

    #[test]
    fn a_missing_clip_or_a_gap_in_the_rows_is_refused() {
        let mut m = manifest();
        m.clips.retain(|c| c.name != "wave");
        assert!(m.check().is_err());
        let mut m = manifest();
        m.clips[1].row += 1;
        assert!(m.check().is_err());
    }

    #[test]
    fn a_vat_file_round_trips_its_header_and_refuses_a_bad_size() {
        let mut b = MAGIC.to_vec();
        for v in [3u32, 3, 1, 0] {
            b.extend(v.to_le_bytes());
        }
        for v in [0.0f32, 0.0, 0.0, 2.0, 2.0, 2.0] {
            b.extend(v.to_le_bytes());
        }
        for i in [0u32, 1, 2] {
            b.extend(i.to_le_bytes());
        }
        b.extend([0, 128, 0, 0, 1, 128, 0, 0, 2, 128, 0, 0]);
        for v in 0..3u16 {
            for q in [0u16, 65535, 32768 * (v % 2), 0] {
                b.extend(q.to_le_bytes());
            }
        }
        let vat = VatLevel::parse(&b).unwrap();
        assert_eq!((vat.vertices, vat.triangles(), vat.rows), (3, 1, 1));
        assert_eq!(vat.region, vec![0, 1, 2]);
        assert_eq!(vat.position(0, 0), [0.0, 2.0, 0.0]);
        b.pop();
        assert!(VatLevel::parse(&b).is_err());
    }
}
