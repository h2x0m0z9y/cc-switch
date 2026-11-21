//! Qoder IDE data locations (`Qoder IDE.app`, bundle `com.qoder.ide`).
//!
//! Qoder IDE is a VS Code-based desktop app. Its local chat store is
//! `<app data dir>/SharedClientCache/cache/db/local.db`:
//! - `chat_session` — plaintext title / project / timestamps (listable),
//! - `chat_message` — `role` is plaintext, but message `content` is encrypted
//!   at rest; tool-call `tool_result` is plaintext JSON.
//!
//! A CLI build may also write CodeBuddy-Code-style JSONL under
//! `<data dir>/projects/`; that is used as a fallback.
use crate::config::get_home_dir;
use std::path::PathBuf;

/// `QODER_CONFIG_DIR`, then `~/.qoder` (CLI/JSONL fallback).
pub(crate) fn data_dir() -> PathBuf {
    std::env::var_os("QODER_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| get_home_dir().join(".qoder"))
}

/// Desktop app data directory (VS Code fork layout).
pub(crate) fn app_support_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("QODER_APP_SUPPORT_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "macos")]
    {
        get_home_dir().join("Library/Application Support/Qoder")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join("AppData/Roaming"))
            .join("Qoder")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join(".config"))
            .join("Qoder")
    }
}

/// SQLite chat store.
pub(crate) fn local_db() -> PathBuf {
    app_support_dir().join("SharedClientCache/cache/db/local.db")
}