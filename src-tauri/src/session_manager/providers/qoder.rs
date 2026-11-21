//! Qoder sessions.
//!
//! Prefers the desktop app's SQLite chat store (plaintext session metadata;
//! message body encrypted at rest). Falls back to CodeBuddy-Code-style JSONL
//! under `<data dir>/projects/` for a CLI build.
use std::path::{Path, PathBuf};

use crate::session_manager::{SessionMessage, SessionMeta};

use super::{qoder_format, workbuddy};

const PROVIDER_ID: &str = "qoder";

pub(crate) fn session_roots() -> Vec<PathBuf> {
    vec![crate::qoder_config::data_dir().join("projects")]
}

fn projects_dir() -> PathBuf {
    crate::qoder_config::data_dir().join("projects")
}

pub fn scan_sessions() -> Vec<SessionMeta> {
    let sqlite = qoder_format::scan(&crate::qoder_config::local_db(), PROVIDER_ID);
    if !sqlite.is_empty() {
        return sqlite;
    }
    workbuddy::scan(&projects_dir(), PROVIDER_ID, None)
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    workbuddy::load_messages(path)
}

pub fn load_messages_sqlite(source: &str) -> Result<Vec<SessionMessage>, String> {
    qoder_format::load_sqlite(source, "Qoder")
}