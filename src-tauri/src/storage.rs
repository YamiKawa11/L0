use rusqlite::{params, Connection, Result};
use tauri::Manager;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub content: String,
    pub char_count: i32,
}

pub struct Storage {
    db_path: PathBuf,
}

impl Storage {
    pub fn new(app_handle: &tauri::AppHandle) -> Self {
        let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
        std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
        let db_path = app_dir.join("glazgod.db");
        
        let conn = Connection::open(&db_path).expect("Failed to open database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                ended_at TEXT NOT NULL,
                content TEXT NOT NULL,
                char_count INTEGER NOT NULL
            )",
            [],
        ).expect("Failed to create sessions table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).expect("Failed to create settings table");

        Storage { db_path }
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    pub fn save_session(&self, session: &Session) -> Result<i64> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO sessions (created_at, ended_at, content, char_count) 
             VALUES (?1, ?2, ?3, ?4)",
            params![
                session.created_at.to_rfc3339(),
                session.ended_at.to_rfc3339(),
                session.content,
                session.char_count,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_sessions(&self, search: Option<String>) -> Result<Vec<Session>> {
        let conn = self.get_connection()?;
        let mut query = "SELECT id, created_at, ended_at, content, char_count FROM sessions".to_string();
        
        if let Some(s) = search {
            if !s.is_empty() {
                query.push_str(&format!(" WHERE content LIKE '%{}%'", s.replace("'", "''")));
            }
        }
        query.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            let created_at: String = row.get(1)?;
            let ended_at: String = row.get(2)?;
            Ok(Session {
                id: Some(row.get(0)?),
                created_at: DateTime::parse_from_rfc3339(&created_at).unwrap().with_timezone(&Utc),
                ended_at: DateTime::parse_from_rfc3339(&ended_at).unwrap().with_timezone(&Utc),
                content: row.get(3)?,
                char_count: row.get(4)?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in rows {
            sessions.push(session?);
        }
        Ok(sessions)
    }

    pub fn delete_sessions(&self, ids: Vec<i64>) -> Result<()> {
        let conn = self.get_connection()?;
        let ids_str = ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",");
        let query = format!("DELETE FROM sessions WHERE id IN ({})", ids_str);
        conn.execute(&query, [])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str, default: &str) -> String {
        let conn = self.get_connection().expect("DB connection failed");
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1").unwrap();
        let res = stmt.query_row(params![key], |row| row.get(0));
        res.unwrap_or_else(|_| default.to_string())
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}
