//! User settings: validation and the JSON file in the app config directory.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

const MAX_NAME_CHARS: usize = 63;
const STRENGTH_RANGE: std::ops::RangeInclusive<f64> = 1.0..=10.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub input_name: String,
    pub output_name: String,
    pub nudge_strength_pct: f64,
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            input_name: "Deeboo Clock Sync In".into(),
            output_name: "Deeboo Clock Sync Out".into(),
            nudge_strength_pct: 4.0,
            launch_at_login: false,
        }
    }
}

fn clean_name(label: &str, raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(format!("{label} name cannot be empty"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "{label} name cannot be longer than {MAX_NAME_CHARS} characters"
        ));
    }
    Ok(name.to_owned())
}

/// Trims the names and rejects values the app cannot use.
///
/// # Errors
/// Returns a user-facing message for an empty or too long name, or a strength outside 1-10 %.
pub fn validate(s: Settings) -> Result<Settings, String> {
    let Settings {
        input_name,
        output_name,
        nudge_strength_pct,
        launch_at_login,
    } = s;
    if !STRENGTH_RANGE.contains(&nudge_strength_pct) {
        return Err("Nudge strength must be between 1 and 10 percent".into());
    }
    Ok(Settings {
        input_name: clean_name("Input", &input_name)?,
        output_name: clean_name("Output", &output_name)?,
        nudge_strength_pct,
        launch_at_login,
    })
}

/// A missing or corrupt file gives the defaults.
#[must_use]
pub fn load(path: &Path) -> Settings {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("cannot read {}: {e}", path.display());
            }
            return Settings::default();
        }
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        log::warn!("corrupt settings file {}: {e}", path.display());
        Settings::default()
    })
}

/// Writes a temp file next to `path`, then renames it over `path`.
///
/// # Errors
/// Returns the I/O error of the directory creation, write or rename.
pub fn save(path: &Path, s: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("deeboo-settings-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("settings.json")
    }

    fn with_pct(pct: f64) -> Settings {
        Settings {
            nudge_strength_pct: pct,
            ..Settings::default()
        }
    }

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.input_name, "Deeboo Clock Sync In");
        assert_eq!(s.output_name, "Deeboo Clock Sync Out");
        assert!((s.nudge_strength_pct - 4.0).abs() < f64::EPSILON);
        assert!(!s.launch_at_login);
    }

    #[test]
    fn validate_trims_and_rejects_empty_names() {
        let ok = validate(Settings {
            input_name: "  X  ".into(),
            ..Settings::default()
        })
        .unwrap();
        assert_eq!(ok.input_name, "X");
        let err = validate(Settings {
            output_name: "   ".into(),
            ..Settings::default()
        })
        .unwrap_err();
        assert_eq!(err, "Output name cannot be empty");
        assert!(
            validate(Settings {
                input_name: "x".repeat(64),
                ..Settings::default()
            })
            .is_err()
        );
    }

    #[test]
    fn validate_rejects_strength_out_of_range() {
        assert!(validate(with_pct(0.9)).is_err());
        assert!(validate(with_pct(10.1)).is_err());
        assert!(validate(with_pct(1.0)).is_ok());
        assert!(validate(with_pct(10.0)).is_ok());
    }

    #[test]
    fn load_missing_file_gives_defaults() {
        assert_eq!(load(&temp_path("missing")), Settings::default());
    }

    #[test]
    fn load_corrupt_file_gives_defaults() {
        let path = temp_path("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let path = temp_path("roundtrip");
        let s = Settings {
            input_name: "A".into(),
            output_name: "B".into(),
            nudge_strength_pct: 7.5,
            launch_at_login: true,
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
        assert!(!path.with_extension("json.tmp").exists());
    }
}
