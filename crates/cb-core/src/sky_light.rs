//! The sky's light and the glass that reflects it (openspec change `glass-reflections`). Data only,
//! loaded from `data/world/sky_light.ron`; `cb-app` loads the cached sky cube map, gives it to the
//! pilot's eye as its environment light (reflections and sky fill), and makes the buildings' glass
//! from the `glass` section.

use crate::data::{Validate, finite3, within};
use serde::Deserialize;

/// `data/world/sky_light.ron`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SkyLightConfig {
    /// The sky's cube map under `assets/`: six square power-of-two faces stacked top to bottom
    /// (+X, -X, +Y, -Y, +Z, -Z), linear Radiance HDR, written by `tools/textures/gen_sky_cube.py`.
    /// Empty turns the sky light off (the flat ambient alone, as before).
    pub cube: String,
    /// What one unit of the cube map's radiance is, cd/m2.
    pub sky_cd_m2: f32,
    /// The sky's turn about the vertical, degrees.
    pub yaw_deg: f32,
    /// The flat ambient fill left under the sky light (what the sky cannot reach: it is not
    /// occluded, so this is kept low), cd/m2. With the sky light off the world uses
    /// `ambient_without_sky_cd_m2` instead.
    pub ambient_cd_m2: f32,
    /// The flat ambient when there is no sky cube, cd/m2.
    pub ambient_without_sky_cd_m2: f32,
    /// The flat ambient's colour, sRGB 0..1.
    pub ambient_colour: [f32; 3],
    /// The buildings' window glass.
    pub glass: GlassLook,
}

/// Window glass: see-through, with full-strength reflections (premultiplied alpha: the light it
/// reflects is added in full, and only what is behind it is dimmed by `alpha`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct GlassLook {
    /// The pane's colour, linear 0..1: its body colour (scaled by `1 - coating`, added over what is
    /// behind) and the colour of what its coating reflects.
    pub tint: [f32; 3],
    /// How much of what is behind the pane it hides, 0 clear to 1 opaque.
    pub alpha: f32,
    /// Perceptual roughness, 0 mirror to 1 matte.
    pub roughness: f32,
    /// Specular reflectance at normal incidence, Bevy's 0..1 scale (0.5 is 4 %, plain glass;
    /// higher is a coated pane). Plain glass tops out at 16 % here; the coating goes past it.
    pub reflectance: f32,
    /// The reflective coating, 0 plain glass to 1 a mirror: Bevy's `metallic`, so the head-on
    /// reflection becomes `coating x tint` (plus what is left of `reflectance`) and the body
    /// colour fades by `1 - coating`.
    pub coating: f32,
}

impl Default for SkyLightConfig {
    fn default() -> Self {
        Self {
            cube: "textures/sky/sky_cube.hdr".into(),
            sky_cd_m2: 2000.0,
            yaw_deg: 0.0,
            ambient_cd_m2: 250.0,
            ambient_without_sky_cd_m2: 900.0,
            ambient_colour: [1.0, 0.96, 0.9],
            glass: GlassLook::default(),
        }
    }
}

impl Default for GlassLook {
    fn default() -> Self {
        Self {
            tint: [0.24, 0.32, 0.38],
            alpha: 0.92,
            roughness: 0.02,
            reflectance: 1.0,
            coating: 0.9,
        }
    }
}

impl Validate for SkyLightConfig {
    fn validate(&self) -> Result<(), String> {
        within("sky_cd_m2", self.sky_cd_m2, 0.0, 1_000_000.0)?;
        within("yaw_deg", self.yaw_deg, -360.0, 360.0)?;
        within("ambient_cd_m2", self.ambient_cd_m2, 0.0, 100_000.0)?;
        within("ambient_without_sky_cd_m2", self.ambient_without_sky_cd_m2, 0.0, 100_000.0)?;
        finite3("glass.tint", self.glass.tint)?;
        finite3("ambient_colour", self.ambient_colour)?;
        if self.glass.tint.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err(format!("`glass.tint` must be within 0..1, got {:?}", self.glass.tint));
        }
        within("glass.alpha", self.glass.alpha, 0.0, 1.0)?;
        within("glass.roughness", self.glass.roughness, 0.0, 1.0)?;
        within("glass.reflectance", self.glass.reflectance, 0.0, 1.0)?;
        within("glass.coating", self.glass.coating, 0.0, 1.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse_ron;
    use std::path::Path;

    #[test]
    fn the_shipped_sky_light_file_is_the_code_default_written_out() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/sky_light.ron");
        let shipped: SkyLightConfig = crate::data::load_ron(&path).unwrap();
        assert_eq!(shipped, SkyLightConfig::default());
    }

    #[test]
    fn the_shipped_sky_cube_exists_with_six_square_power_of_two_faces() {
        let cfg = SkyLightConfig::default();
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(&cfg.cube);
        let head = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let text = String::from_utf8_lossy(&head[..head.len().min(256)]);
        let dims = text.lines().find(|l| l.starts_with("-Y ")).expect("a Radiance size line");
        let n: Vec<u32> = dims.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        let (h, w) = (n[0], n[1]);
        assert!(w.is_power_of_two(), "face side {w} is not a power of two");
        assert_eq!(h, 6 * w, "{} is not six square faces stacked", path.display());
    }

    #[test]
    fn glass_more_opaque_than_one_is_refused() {
        let e = parse_ron::<SkyLightConfig>("(glass: (alpha: 1.5))", Path::new("sky_light.ron")).unwrap_err();
        assert!(e.to_string().contains("glass.alpha"), "{e}");
    }

    #[test]
    fn a_missing_field_inherits_the_default() {
        let c = parse_ron::<SkyLightConfig>("(sky_cd_m2: 10.0)", Path::new("sky_light.ron")).unwrap();
        assert_eq!(c.sky_cd_m2, 10.0);
        assert_eq!(c.glass, GlassLook::default());
    }
}
