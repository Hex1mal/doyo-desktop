use crate::db::Database;
use crate::error::Result;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

pub struct SettingsRepository {
    db: Arc<Database>,
}

impl SettingsRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let conn = self.db.conn.lock().unwrap();
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .ok();
        match result {
            Some(val) => Ok(Some(serde_json::from_str(&val)?)),
            None => Ok(None),
        }
    }

    pub fn set<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let val = serde_json::to_string(value)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            rusqlite::params![key, &val, &now],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            rusqlite::params![key],
        )?;
        Ok(())
    }

    pub fn list(&self, prefix: Option<&str>) -> Result<Vec<(String, Value)>> {
        let conn = self.db.conn.lock().unwrap();
        let rows = if let Some(prefix) = prefix {
            let mut stmt =
                conn.prepare("SELECT key, value FROM settings WHERE key LIKE ?1 ORDER BY key")?;
            let rows = stmt
                .query_map(rusqlite::params![format!("{}%", prefix)], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        Ok(rows
            .into_iter()
            .map(|(key, raw)| {
                let value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                (key, value)
            })
            .collect())
    }
}
