//! CodeBuddy CN data locations.
//!
//! CodeBuddy CN ships as a VS Code-based desktop app. Its local session
//! **metadata** (title / cwd / timestamps) lives in a SQLite store:
//! `<app data dir>/codebuddy-sessions.vscdb`, table `ItemTable`, rows keyed
//! `session:<conversationId>`. The conversation bodies themselves are cloud-side
//! and are not available locally.
//!
//! A CLI build may also write JSONL under `<data dir>/projects/`; that layout is
//! still supported as a fallback.
use crate::config::get_home_dir;
use std::path::PathBuf;

/// CLI-style data directory: `CODEBUDDY_CN_CONFIG_DIR` or `~/.codebuddy-cn`.
pub(crate) fn data_dir() -> PathBuf {
    std::env::var_os("CODEBUDDY_CN_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| get_home_dir().join(".codebuddy-cn"))
}

/// Desktop app data directory (VS Code fork layout).
pub(crate) fn app_support_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CODEBUDDY_CN_APP_SUPPORT_DIR")
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "macos")]
    {
        get_home_dir().join("Library/Application Support/CodeBuddy CN")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join("AppData/Roaming"))
            .join("CodeBuddy CN")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join(".config"))
            .join("CodeBuddy CN")
    }
}

/// SQLite store holding session metadata.
pub(crate) fn sessions_db() -> PathBuf {
    app_support_dir().join("codebuddy-sessions.vscdb")
}