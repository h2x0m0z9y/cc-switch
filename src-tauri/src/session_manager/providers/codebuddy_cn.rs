//! CodeBuddy CN sessions.
//!
//! The desktop app (VS Code fork) stores **session metadata** in
//! `<app data dir>/codebuddy-sessions.vscdb` (`ItemTable`, keys `session:<id>`,
//! JSON values with title / cwd / timestamps). Conversation bodies are
//! cloud-side, so the reader shows a note pointing back to the app.
//!
//! A CLI build may instead write CodeBuddy-Code-style JSONL under
//! `<data dir>/projects/`; that layout is used as a fallback and parsed by the
//! shared `workbuddy` parser.
use std::path::{Path, PathBuf};

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::session_manager::model::SessionBlock;
use crate::session_manager::{SessionMessage, SessionMeta};

use super::workbuddy;

const PROVIDER_ID: &str = "codebuddy-cn";

pub(crate) fn session_roots() -> Vec<PathBuf> {
    vec![crate::codebuddy_cn_config::data_dir().join("projects")]
}

fn projects_dir() -> PathBuf {
    crate::codebuddy_cn_config::data_dir().join("projects")
}

/// Parse `sqlite:<db_path>:<session_id>` (db path may contain `:` on Windows,
/// session ids are colon-free hex, so split at the last colon).
pub(crate) fn parse_sqlite_source(source: &str) -> Option<(PathBuf, String)> {
    let rest = source.strip_prefix("sqlite:")?;
    let sep = rest.rfind(':')?;
    let db_path = &rest[..sep];
    let session_id = &rest[sep + 1..];
    (!db_path.is_empty() && !session_id.is_empty())
        .then(|| (PathBuf::from(db_path), session_id.to_string()))
}

pub fn scan_sessions() -> Vec<SessionMeta> {
    let sqlite = scan_sessions_sqlite();
    if !sqlite.is_empty() {
        return sqlite;
    }
    workbuddy::scan(&projects_dir(), PROVIDER_ID, None)
}

fn scan_sessions_sqlite() -> Vec<SessionMeta> {
    let db = crate::codebuddy_cn_config::sessions_db();
    if !db.is_file() {
        return Vec::new();
    }
    let Ok(conn) = Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let db_display = db.display().to_string();
    let Ok(mut stmt) = conn.prepare("SELECT value FROM ItemTable WHERE key LIKE 'session:%'")
    else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, SqlValue>(0)) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for value in rows.flatten() {
        let bytes = match value {
            SqlValue::Text(text) => text.into_bytes(),
            SqlValue::Blob(bytes) => bytes,
            _ => continue,
        };
        if let Some(meta) = parse_meta(&bytes, &db_display) {
            sessions.push(meta);
        }
    }
    sessions
}

fn parse_meta(bytes: &[u8], db_display: &str) -> Option<SessionMeta> {
    let record: Value = serde_json::from_slice(bytes).ok()?;
    let session_id = record
        .get("conversationId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())?
        .to_string();
    let title = ["customTitle", "title"]
        .into_iter()
        .filter_map(|key| record.get(key).and_then(Value::as_str))
        .map(str::trim)
        .find(|text| !text.is_empty())
        .map(str::to_string);
    let project_dir = record
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|cwd| !cwd.is_empty())
        .map(str::to_string);
    let created_at = record.get("createdAt").and_then(Value::as_i64);
    let last_active_at = record.get("updatedAt").and_then(Value::as_i64);
    Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        session_id: session_id.clone(),
        title: title.clone(),
        summary: title,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(format!("sqlite:{db_display}:{session_id}")),
        resume_command: None,
    })
}

/// CLI/JSONL fallback (CodeBuddy Code engine format).
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    workbuddy::load_messages(path)
}

/// Desktop app sessions keep only metadata locally; the transcript is cloud-side.
/// Show what is available and point the user back to CodeBuddy CN.
pub fn load_messages_sqlite(source: &str) -> Result<Vec<SessionMessage>, String> {
    let (db, session_id) =
        parse_sqlite_source(source).ok_or("Invalid CodeBuddy CN session source")?;
    let mut note = String::from(
        "CodeBuddy CN 的完整对话保存在应用内（本地仅同步标题、目录与时间，正文在云端）。\n\
         \n请在 CodeBuddy CN 中打开此会话查看完整内容。\n",
    );
    note.push_str(&format!("\n会话 ID：{session_id}"));

    let mut ts = None;
    if let Ok(conn) = Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        let key = format!("session:{session_id}");
        if let Ok(value) = conn.query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get::<_, SqlValue>(0),
        ) {
            let bytes = match value {
                SqlValue::Text(text) => text.into_bytes(),
                SqlValue::Blob(bytes) => bytes,
                _ => Vec::new(),
            };
            if let Ok(record) = serde_json::from_slice::<Value>(&bytes) {
                if let Some(title) = ["customTitle", "title"]
                    .into_iter()
                    .filter_map(|k| record.get(k).and_then(Value::as_str))
                    .map(str::trim)
                    .find(|text| !text.is_empty())
                {
                    note.push_str(&format!("\n标题：{title}"));
                }
                if let Some(cwd) = record
                    .get("cwd")
                    .and_then(Value::as_str)
                    .filter(|cwd| !cwd.is_empty())
                {
                    note.push_str(&format!("\n目录：{cwd}"));
                }
                ts = record.get("updatedAt").and_then(Value::as_i64);
            }
        }
    }

    Ok(vec![SessionMessage::from_blocks(
        "assistant",
        ts,
        vec![SessionBlock::text(note)],
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_db(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
            [],
        )
        .unwrap();
        let values = [
            (
                "session:abc123",
                serde_json::json!({
                    "conversationId": "abc123",
                    "cwd": "/Users/me/proj",
                    "title": "first title",
                    "customTitle": "V2 开发",
                    "createdAt": 1_700_000_000_000i64,
                    "updatedAt": 1_700_000_100_000i64,
                })
                .to_string(),
            ),
            (
                "session:def456",
                serde_json::json!({
                    "conversationId": "def456",
                    "cwd": "/Users/me/proj",
                    "title": "plain title",
                    "createdAt": 1_700_000_200_000i64,
                    "updatedAt": 1_700_000_300_000i64,
                })
                .to_string(),
            ),
        ];
        for (key, value) in values {
            conn.execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn codebuddy_cn_reads_sqlite_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("codebuddy-sessions.vscdb");
        write_db(&db);

        let db_display = db.display().to_string();
        let conn = Connection::open(&db).unwrap();
        let mut stmt = conn
            .prepare("SELECT value FROM ItemTable WHERE key LIKE 'session:%'")
            .unwrap();
        let rows = stmt
            .query_map([], |row| row.get::<_, SqlValue>(0))
            .unwrap();
        let mut sessions: Vec<SessionMeta> = rows
            .flatten()
            .filter_map(|value| match value {
                SqlValue::Text(text) => parse_meta(text.as_bytes(), &db_display),
                SqlValue::Blob(bytes) => parse_meta(&bytes, &db_display),
                _ => None,
            })
            .collect();
        sessions.sort_by_key(|s| s.session_id.clone());
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].provider_id, "codebuddy-cn");
        assert_eq!(sessions[0].session_id, "abc123");
        assert_eq!(sessions[0].title.as_deref(), Some("V2 开发"));
        assert_eq!(sessions[0].project_dir.as_deref(), Some("/Users/me/proj"));
        assert_eq!(sessions[0].created_at, Some(1_700_000_000_000));
        assert_eq!(
            sessions[0].source_path.as_deref(),
            Some(format!("sqlite:{db_display}:abc123").as_str())
        );
        assert_eq!(sessions[1].title.as_deref(), Some("plain title"));
    }

    #[test]
    fn codebuddy_cn_parses_sqlite_source_with_windows_drive() {
        let (db, id) = parse_sqlite_source("sqlite:C:\\Users\\me\\cb\\sessions.vscdb:abc123").unwrap();
        assert_eq!(db, PathBuf::from("C:\\Users\\me\\cb\\sessions.vscdb"));
        assert_eq!(id, "abc123");
        assert!(parse_sqlite_source("abc").is_none());
    }

    #[test]
    fn codebuddy_cn_sqlite_reader_returns_metadata_note() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("codebuddy-sessions.vscdb");
        write_db(&db);
        let source = format!("sqlite:{}:abc123", db.display());
        let messages = load_messages_sqlite(&source).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("CodeBuddy CN"));
        assert!(messages[0].content.contains("V2 开发"));
        assert!(messages[0].content.contains("abc123"));
    }

    #[test]
    fn codebuddy_cn_jsonl_fallback_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path().join("projects/work-proj");
        fs::create_dir_all(&projects).unwrap();
        let path = projects.join("s1.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"id":"u1","timestamp":1700000000000,"type":"message","role":"user","content":[{"type":"input_text","text":"<user_query>hi</user_query>"}],"sessionId":"s1","cwd":"/p"}"#,
                "\n",
                r#"{"id":"a1","timestamp":1700000001000,"type":"message","role":"assistant","content":[{"type":"output_text","text":"yo"}],"sessionId":"s1","cwd":"/p"}"#,
                "\n",
            ),
        )
        .unwrap();
        let sessions = workbuddy::scan(&dir.path().join("projects"), PROVIDER_ID, None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider_id, "codebuddy-cn");
        let messages = load_messages(&path).unwrap();
        assert_eq!(messages.len(), 2);
    }
}