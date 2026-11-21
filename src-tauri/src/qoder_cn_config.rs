//! Qoder CN IDE data locations (`Qoder CN IDE.app`, bundle `com.aliyun.lingma.ide`).
//!
//! Same chat store layout as Qoder IDE.
use crate::config::get_home_dir;
use std::path::PathBuf;

/// `QODER_CN_CONFIG_DIR`, then `~/.qoder-cn` (CLI/JSONL fallback).
pub(crate) fn data_dir() -> PathBuf {
    std::env::var_os("QODER_CN_CONFIG_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| get_home_dir().join(".qoder-cn"))
}

/// Desktop app data directory.
pub(crate) fn app_support_dir() -> PathBuf {
    if let Some(dir) =
        std::env::var_os("QODER_CN_APP_SUPPORT_DIR").filter(|value| !value.is_empty())
    {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "macos")]
    {
        // Qoder CN IDE bundles as `com.aliyun.lingma.ide` and uses `QoderCN`
        // (no space) as its Electron userData directory.
        get_home_dir().join("Library/Application Support/QoderCN")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join("AppData/Roaming"))
            .join("Qoder CN")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| get_home_dir().join(".config"))
            .join("Qoder CN")
    }
}

/// SQLite chat store.
pub(crate) fn local_db() -> PathBuf {
    app_support_dir().join("SharedClientCache/cache/db/local.db")
}