//! Per-user data directory resolution for the packaged desktop app.
//!
//! Only the `rsahp-desktop` wrapper uses this. The standalone `backend`/`frontend`
//! binaries keep their cwd-relative config/DB/logs behavior (dev unchanged).

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::path::{Path, PathBuf};

/// Characters we percent-encode in a filesystem path before embedding it in a
/// `sqlite://` URL. Beyond CONTROLS: a SPACE (common in Windows usernames like
/// `C:\Users\John Doe\...`) and the URL-structural chars that would otherwise be
/// misparsed. We deliberately do NOT encode `/`, `:`, `.`, `-`, `_`, `~` (needed
/// intact for drive letters and path separators).
const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'|')
    .add(b'^');

/// Resolved, absolute, per-user paths for the packaged app, under the OS-specific
/// **local** (non-roaming) data directory.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    #[must_use]
    pub fn database_url(&self) -> String {
        database_url_from_path(&self.db_path)
    }
}

/// Builds a URI-safe sea-orm/sqlx `SQLite` URL from an absolute filesystem path.
///
/// Backslashes → forward slashes; a leading `/` is guaranteed (empty-authority
/// absolute-path form); unsafe chars (esp. SPACE) are percent-encoded. Yields
/// `sqlite:///C:/Users/John%20Doe/.../rsahp.db?mode=rwc` on Windows and
/// `sqlite:///home/.../rsahp.db?mode=rwc` on Linux.
#[must_use]
pub fn database_url_from_path(db_path: &Path) -> String {
    let mut s = db_path.to_string_lossy().replace('\\', "/");
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let encoded = utf8_percent_encode(&s, PATH_ENCODE_SET).to_string();
    format!("sqlite://{encoded}?mode=rwc")
}

/// Resolves the per-user local data dir and derived paths. `None` only if the OS
/// cannot supply a home/data directory.
#[must_use]
pub fn resolve() -> Option<AppPaths> {
    // Empty qualifier/org → `<LocalAppData>\rsahp\...` / `~/.local/share/rsahp`, no org
    // segment. data_local_dir() is LOCAL (non-roaming) — a SQLite DB must never roam.
    let dirs = directories::ProjectDirs::from("", "", "rsahp")?;
    let data_dir = dirs.data_local_dir().to_path_buf();
    let config_path = data_dir.join("config.json");
    let db_path = data_dir.join("rsahp.db");
    let logs_dir = data_dir.join("logs");
    Some(AppPaths {
        data_dir,
        config_path,
        db_path,
        logs_dir,
    })
}

/// Default `config.json` seeded on first run.
///
/// Contains **every key both binaries read** across their separate `AppConfig` structs
/// (backend: `database_url`, `port`; frontend: `api_url`, `use_gpu`). `use_gpu: false` =
/// safe default (GPU init can crash on faulty drivers; capable machines opt in). Written
/// verbatim (neither struct owns all keys).
pub const DEFAULT_CONFIG_JSON: &str = r#"{
  "api_url": "http://127.0.0.1:4002/api/documents",
  "use_gpu": false,
  "database_url": "sqlite://rsahp.db?mode=rwc",
  "port": 4002
}
"#;

/// Ensures `data_dir` + `logs_dir` exist and seeds `config_path`.
///
/// Seeds with [`DEFAULT_CONFIG_JSON`] **only if it does not already exist** (idempotent —
/// never clobbers a user-edited config). NOTE: the wrapper creates `data_dir` separately
/// *before* acquiring the single-instance lock (the lock file lives in it); this call
/// runs *after* the lock so only the lock-winner seeds the config (no first-run race).
///
/// # Errors
///
/// Returns an error if creating `data_dir` or `logs_dir`, or writing `config_path`, fails
/// (e.g. permissions or a full disk).
pub fn ensure_dirs_and_seed(
    data_dir: &Path,
    logs_dir: &Path,
    config_path: &Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::create_dir_all(logs_dir)?;
    if !config_path.exists() {
        std::fs::write(config_path, DEFAULT_CONFIG_JSON)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_to_local_non_roaming_dir_containing_rsahp() {
        let paths = resolve().expect("data dir should resolve on a normal machine");
        let s = paths.data_dir.to_string_lossy().to_lowercase();
        assert!(s.contains("rsahp"), "data_dir should contain 'rsahp': {s}");

        #[cfg(windows)]
        {
            let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA set");
            assert!(
                paths.data_dir.starts_with(&local),
                "must be under %LocalAppData%: {s}"
            );
        }
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").expect("HOME set");
            let xdg =
                std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
            assert!(
                paths.data_dir.starts_with(&xdg),
                "must be under XDG data home: {s}"
            );
        }

        assert_eq!(paths.config_path, paths.data_dir.join("config.json"));
        assert_eq!(paths.db_path, paths.data_dir.join("rsahp.db"));
        assert_eq!(paths.logs_dir, paths.data_dir.join("logs"));
    }

    #[test]
    fn database_url_unix_absolute() {
        let unix = database_url_from_path(Path::new("/home/u/.local/share/rsahp/rsahp.db"));
        assert_eq!(
            unix,
            "sqlite:///home/u/.local/share/rsahp/rsahp.db?mode=rwc"
        );
    }

    #[test]
    fn database_url_windows_encodes_space_in_username() {
        // The critical case: a Windows username with a space must be percent-encoded.
        let win =
            database_url_from_path(Path::new(r"C:\Users\John Doe\AppData\Local\rsahp\rsahp.db"));
        assert_eq!(
            win,
            "sqlite:///C:/Users/John%20Doe/AppData/Local/rsahp/rsahp.db?mode=rwc"
        );
    }

    #[test]
    fn seed_writes_all_keys_and_is_idempotent() {
        let tmp = std::env::temp_dir().join(format!("rsahp_seed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = tmp.join("config.json");

        ensure_dirs_and_seed(&tmp, &tmp.join("logs"), &cfg).expect("seed ok");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        for key in ["api_url", "use_gpu", "database_url", "port"] {
            assert!(v.get(key).is_some(), "missing key: {key}");
        }
        assert!(tmp.join("logs").is_dir());

        std::fs::write(&cfg, r#"{"api_url":"http://custom"}"#).unwrap();
        ensure_dirs_and_seed(&tmp, &tmp.join("logs"), &cfg).expect("second seed ok");
        assert_eq!(
            std::fs::read_to_string(&cfg).unwrap(),
            r#"{"api_url":"http://custom"}"#
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
