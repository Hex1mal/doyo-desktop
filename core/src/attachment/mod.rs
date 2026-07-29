use crate::db::Database;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub node_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub file_path: String,
    pub created_at: String,
}

pub struct AttachmentService {
    db: Arc<Database>,
    base_path: PathBuf,
}

impl AttachmentService {
    pub fn new(db: Arc<Database>, base_path: PathBuf) -> Self {
        Self { db, base_path }
    }

    pub fn add_attachment(
        &self,
        node_id: &str,
        filename: &str,
        mime_type: &str,
        data: &[u8],
    ) -> Result<Attachment> {
        validate_storage_component(node_id, "node id")?;
        validate_storage_component(filename, "attachment filename")?;
        let id = Uuid::now_v7().to_string();
        let base_path = canonical_base(&self.base_path)?;
        let dir = base_path.join(node_id);
        std::fs::create_dir_all(&dir)?;
        let file_path = dir.join(filename);
        ensure_inside(&base_path, &file_path)?;
        std::fs::write(&file_path, data)?;

        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO attachments (id, node_id, filename, mime_type, size_bytes, file_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &id,
                node_id,
                filename,
                mime_type,
                data.len() as i64,
                &file_path.to_string_lossy().to_string()
            ],
        )?;

        Ok(Attachment {
            id,
            node_id: node_id.to_string(),
            filename: filename.to_string(),
            mime_type: mime_type.to_string(),
            size_bytes: data.len() as i64,
            file_path: file_path.to_string_lossy().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn get_attachments(&self, node_id: &str) -> Result<Vec<Attachment>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, node_id, filename, mime_type, size_bytes, file_path, created_at FROM attachments WHERE node_id = ?1"
        )?;
        let attachments = stmt
            .query_map(rusqlite::params![node_id], |row| {
                Ok(Attachment {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    filename: row.get(2)?,
                    mime_type: row.get(3)?,
                    size_bytes: row.get(4)?,
                    file_path: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(attachments)
    }

    pub fn delete_attachment(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        let file_path: String = conn.query_row(
            "SELECT file_path FROM attachments WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )?;
        let base_path = canonical_base(&self.base_path)?;
        let path = PathBuf::from(&file_path);
        ensure_inside(&base_path, &path)?;
        let _ = std::fs::remove_file(path);
        conn.execute(
            "DELETE FROM attachments WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }
}

fn canonical_base(path: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(path)?;
    Ok(path.canonicalize()?)
}

fn validate_storage_component(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::Validation(format!("{label} is required")));
    }
    let path = Path::new(value);
    if path.is_absolute() || path.components().count() != 1 {
        return Err(Error::Validation(format!(
            "{label} must be a filename, not a path"
        )));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Validation(format!(
            "{label} contains invalid path components"
        )));
    }
    Ok(())
}

fn ensure_inside(base_path: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| Error::Validation("Attachment path has no parent".into()))?;
    let parent = parent.canonicalize()?;
    if !parent.starts_with(base_path) {
        return Err(Error::Validation(
            "Attachment path escapes the attachment directory".into(),
        ));
    }
    Ok(())
}
