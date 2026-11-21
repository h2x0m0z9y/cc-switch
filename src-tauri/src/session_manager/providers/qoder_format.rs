//! Shared Qoder / Qoder CN desktop store reader.
//!
//! `chat_session` holds plaintext metadata (title / project / timestamps), so
//! sessions can be listed. `chat_message.role` and `tool_result` are plaintext,
//! but message `content` is encrypted at rest, so the transcript itself cannot
//! be rendered locally — the reader shows what is available and points back to
//! the app.
use std::path::{Path, PathBuf};

use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::session_manager::model::SessionBlock;
use crate::session_manager::{SessionMessage, SessionMeta};

/// Parse `sqlite:<db_path>:<session_id>`; split at the last colon so Windows
/// drive paths keep working.
pub(crate) fn parse_sqlite_source(source: &str) -> Option<(PathBuf, String)> {
    let rest = source.strip_prefix("sqlite:")?;
    let sep = rest.rfind(':')?;
    let db_path = &rest[..sep];
    let session_id = &rest[sep + 1..];
    (!db_path.is_empty() && !session_id.is_empty())
        .then(|| (PathBuf::from(db_path), session_id.to_string()))
}

fn open_read_only(db: &Path) -> Option<Connection> {
    Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()
}

/// List top-level sessions (sub-agent sessions have a parent and are skipped).
pub(crate) fn scan(db: &Path, provider_id: &str) -> Vec<SessionMeta> {
    if !db.is_file() {
        return Vec::new();
    }
    let Some(conn) = open_read_only(db) else {
        return Vec::new();
    };
    let db_display = db.display().to_string();
    let Ok(mut stmt) = conn.prepare(
        "SELECT session_id, session_title, project_uri, project_id, gmt_create, gmt_modified \
         FROM chat_session WHERE COALESCE(parent_session_id, '') = ''",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<i64>>(5)?,
        ))
    }) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for (id, title, project_uri, project_id, created, modified) in rows.flatten() {
        let Some(session_id) = id.filter(|id| !id.is_empty()) else {
            continue;
        };
        let title = title
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        let project_dir = project_uri
            .filter(|dir| !dir.trim().is_empty())
            .or_else(|| project_id.filter(|dir| !dir.trim().is_empty()));
        sessions.push(SessionMeta {
            provider_id: provider_id.to_string(),
            session_id: session_id.clone(),
            summary: title.clone(),
            title,
            project_dir,
            created_at: created,
            last_active_at: modified,
            source_path: Some(format!("sqlite:{db_display}:{session_id}")),
            resume_command: None,
        });
    }
    sessions
}

/// Build a readable note for a session whose body is encrypted on disk.
pub(crate) fn load_sqlite(source: &str, app_name: &str) -> Result<Vec<SessionMessage>, String> {
    let (db, session_id) =
        parse_sqlite_source(source).ok_or("Invalid Qoder session source")?;
    let some_text = |value: Option<String>| {
        value
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    };

    let mut title = None;
    let mut project = None;
    let mut ts = None;
    let mut message_count = 0usize;
    let mut tool_names: Vec<String> = Vec::new();

    if let Some(conn) = open_read_only(&db) {
        if let Ok(row) = conn.query_row(
            "SELECT session_title, project_uri, project_id, gmt_modified \
             FROM chat_session WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        ) {
            title = some_text(row.0);
            project = some_text(row.1).or_else(|| some_text(row.2));
            ts = row.3;
        }
        if let Ok(mut stmt) = conn.prepare(
            "SELECT role, tool_result FROM chat_message WHERE session_id = ?1 ORDER BY gmt_create",
        ) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<SqlValue>>(1)?,
                ))
            }) {
                for (role, tool_result) in rows.flatten() {
                    message_count += 1;
                    if role.as_deref() == Some("tool") {
                        if let Some(name) = tool_result.and_then(|value| tool_name(&value)) {
                            if !tool_names.contains(&name) {
                                tool_names.push(name);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut note = format!(
        "{app_name} 的对话正文在本地是加密存储的（仅标题、项目与工具调用可读）。\n\
         \n请在 {app_name} 中打开此会话查看完整内容。"
    );
    if let Some(title) = title {
        note.push_str(&format!("\n\n标题：{title}"));
    }
    if let Some(project) = project {
        note.push_str(&format!("\n项目：{project}"));
    }
    note.push_str(&format!("\n本地记录：{message_count} 条消息"));
    if !tool_names.is_empty() {
        note.push_str(&format!("\n工具调用：{}", tool_names.join(", ")));
    }
    note.push_str(&format!("\n会话 ID：{session_id}"));

    Ok(vec![SessionMessage::from_blocks(
        "assistant",
        ts,
        vec![SessionBlock::text(note)],
    )])
}

fn tool_name(value: &SqlValue) -> Option<String> {
    let bytes = match value {
        SqlValue::Text(text) => text.as_bytes(),
        SqlValue::Blob(bytes) => bytes,
        _ => return None,
    };
    let record: Value = serde_json::from_slice(bytes).ok()?;
    record
        .get("toolCallName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_db(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE chat_session (session_id TEXT PRIMARY KEY, session_title TEXT, \
             project_uri TEXT, project_id TEXT, gmt_create INTEGER, gmt_modified INTEGER, \
             parent_session_id TEXT DEFAULT '');
             CREATE TABLE chat_message (session_id TEXT, role TEXT, tool_result TEXT, \
             gmt_create INTEGER);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chat_session VALUES ('s1','配置工具链','/p/explore','/p/explore',1700000000000,1700000005000,'')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chat_session VALUES ('child','sub','/p','/p',1700000000000,1700000000000,'s1')",
            [],
        )
        .unwrap();
        let tool = serde_json::json!({"toolCallName": "search_file"}).to_string();
        conn.execute(
            "INSERT INTO chat_message VALUES ('s1','user','',1700000000000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chat_message VALUES ('s1','tool',?1,1700000001000)",
            rusqlite::params![tool],
        )
        .unwrap();
        conn
    }

    #[test]
    fn qoder_scan_lists_top_level_sessions_only() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("local.db");
        write_db(&db);
        let sessions = scan(&db, "qoder");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider_id, "qoder");
        assert_eq!(sessions[0].session_id, "s1");
        assert_eq!(sessions[0].title.as_deref(), Some("配置工具链"));
        assert_eq!(sessions[0].project_dir.as_deref(), Some("/p/explore"));
        assert_eq!(sessions[0].last_active_at, Some(1700000005000));
    }

    #[test]
    fn qoder_reader_reports_encrypted_body_and_tools() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("local.db");
        write_db(&db);
        let source = format!("sqlite:{}:s1", db.display());
        let messages = load_sqlite(&source, "Qoder").unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("加密"));
        assert!(messages[0].content.contains("配置工具链"));
        assert!(messages[0].content.contains("search_file"));
    }

    #[test]
    fn qoder_parse_source_handles_windows_path() {
        let (db, id) = parse_sqlite_source("sqlite:C:\\Q\\local.db:s1").unwrap();
        assert_eq!(db, PathBuf::from("C:\\Q\\local.db"));
        assert_eq!(id, "s1");
    }
}