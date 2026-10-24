//! The one loader for tuning and content in `data/` (CLAUDE.md section 6.4).
//!
//! Data types derive `Deserialize` with `#[serde(deny_unknown_fields, default)]`: a missing field
//! takes the code default, a present zero is zero, an unknown key is an error. After parsing,
//! `Validate` rejects non-finite and out-of-range numbers by field name. Every error names the file.

use serde::de::DeserializeOwned;
use std::fmt;
use std::path::{Path, PathBuf};

/// A data file that failed to load, with its path and the reason (which names the field).
#[derive(Debug)]
pub struct DataError {
    /// The file that failed.
    pub path: PathBuf,
    /// What was wrong, naming the field.
    pub reason: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.reason)
    }
}

impl std::error::Error for DataError {}

/// A data type that can check its own values after parsing.
pub trait Validate {
    /// Ok, or the first problem found, naming the field.
    fn validate(&self) -> Result<(), String>;
}

/// Reads, parses and validates a RON file.
pub fn load_ron<T: DeserializeOwned + Validate>(path: &Path) -> Result<T, DataError> {
    let text = std::fs::read_to_string(path).map_err(|e| DataError {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    parse_ron(&text, path)
}

/// Parses and validates RON text; `origin` names it in errors.
pub fn parse_ron<T: DeserializeOwned + Validate>(text: &str, origin: &Path) -> Result<T, DataError> {
    let err = |reason: String| DataError {
        path: origin.to_path_buf(),
        reason,
    };
    let value: T = ron::from_str(text).map_err(|e| err(e.to_string()))?;
    value.validate().map_err(err)?;
    Ok(value)
}

/// Rejects NaN and infinity.
pub fn finite(field: &str, v: f32) -> Result<(), String> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(format!("`{field}` must be a finite number, got {v}"))
    }
}

/// Requires a finite value strictly above zero.
pub fn positive(field: &str, v: f32) -> Result<(), String> {
    finite(field, v)?;
    if v > 0.0 {
        Ok(())
    } else {
        Err(format!("`{field}` must be above zero, got {v}"))
    }
}

/// Requires a finite value in `[lo, hi]`.
pub fn within(field: &str, v: f32, lo: f32, hi: f32) -> Result<(), String> {
    finite(field, v)?;
    if (lo..=hi).contains(&v) {
        Ok(())
    } else {
        Err(format!("`{field}` must be within {lo} and {hi}, got {v}"))
    }
}

/// Requires a finite range with `lo <= hi`, both above zero.
pub fn positive_range(field: &str, r: [f32; 2]) -> Result<(), String> {
    positive(field, r[0])?;
    positive(field, r[1])?;
    if r[0] <= r[1] {
        Ok(())
    } else {
        Err(format!("`{field}` must be [low, high] with low <= high, got {r:?}"))
    }
}

/// Requires every component of a vector to be finite.
pub fn finite3(field: &str, v: [f32; 3]) -> Result<(), String> {
    for x in v {
        finite(field, x)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields, default)]
    struct Sample {
        city_count: u32,
        stride_m: f32,
    }

    impl Default for Sample {
        fn default() -> Self {
            Self {
                city_count: 5,
                stride_m: 9.0,
            }
        }
    }

    impl Validate for Sample {
        fn validate(&self) -> Result<(), String> {
            finite("stride_m", self.stride_m)
        }
    }

    #[test]
    fn an_unknown_key_is_an_error_naming_the_file() {
        let e = parse_ron::<Sample>("(city_cuont: 3)", Path::new("data/world/world.ron"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("data/world/world.ron"), "{e}");
        assert!(e.contains("city_cuont"), "{e}");
    }

    #[test]
    fn a_missing_field_takes_the_default() {
        let s: Sample = parse_ron("(city_count: 0)", Path::new("x.ron")).unwrap();
        assert_eq!(s.city_count, 0, "a present zero is zero");
        assert_eq!(s.stride_m, 9.0, "a missing field takes the code default");
    }

    #[test]
    fn a_non_finite_number_is_rejected() {
        let e = parse_ron::<Sample>("(stride_m: inf)", Path::new("x.ron")).unwrap_err().to_string();
        assert!(e.contains("stride_m"), "{e}");
    }
}
