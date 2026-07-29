use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::model::Node;
use crate::node::repository::map_node;
use crate::tag::model::Tag;
use std::sync::Arc;
use uuid::Uuid;

pub struct TagRepository {
    db: Arc<Database>,
}

impl TagRepository {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }

    fn clean_name(name: &str) -> Result<String> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err(Error::Validation("Tag name is required".into()));
        }
        Ok(clean.to_string())
    }

    pub fn create(&self, name: &str, color: Option<&str>) -> Result<Tag> {
        let name = Self::clean_name(name)?;
        let conn = self.db.conn.lock().unwrap();
        let id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&id, &name, &color, &now],
        )?;
        Ok(Tag {
            id,
            name,
            color: color.map(|v| v.to_string()),
            created_at: now,
        })
    }

    pub fn get_or_create(&self, name: &str) -> Result<Tag> {
        let clean = Self::clean_name(name)?;
        match self.find_by_name(&clean) {
            Ok(tag) => Ok(tag),
            Err(_) => self.create(&clean, None),
        }
    }

    pub fn get(&self, id: &str) -> Result<Tag> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM tags WHERE id = ?1", rusqlite::params![id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(|_| Error::NotFound(format!("Tag not found: {}", id)))
    }

    pub fn find_by_name(&self, name: &str) -> Result<Tag> {
        let clean = Self::clean_name(name)?;
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM tags WHERE name = ?1 COLLATE NOCASE", rusqlite::params![clean], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(|_| Error::NotFound(format!("Tag not found: {}", clean)))
    }

    pub fn list_all(&self) -> Result<Vec<Tag>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tags ORDER BY name")?;
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(tags)
    }

    pub fn rename(&self, id: &str, name: &str, color: Option<&str>) -> Result<Tag> {
        let name = Self::clean_name(name)?;
        let conn = self.db.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE AND id != ?2",
                rusqlite::params![&name, id],
                |row| row.get(0),
            )
            .ok();
        if existing.is_some() {
            return Err(Error::Validation(format!("Tag already exists: {}", name)));
        }
        conn.execute(
            "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
            rusqlite::params![&name, &color, id],
        )?;
        conn.query_row("SELECT * FROM tags WHERE id = ?1", rusqlite::params![id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(|_| Error::NotFound(format!("Tag not found: {}", id)))
    }

    pub fn add_to_node(&self, node_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![node_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_from_node(&self, node_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM node_tags WHERE node_id = ?1 AND tag_id = ?2",
            rusqlite::params![node_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_tags_for_node(&self, node_id: &str) -> Result<Vec<Tag>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.* FROM tags t JOIN node_tags nt ON t.id = nt.tag_id WHERE nt.node_id = ?1 ORDER BY t.name"
        )?;
        let tags = stmt.query_map(rusqlite::params![node_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(tags)
    }

    pub fn get_tag_names_for_node(&self, node_id: &str) -> Result<Vec<String>> {
        Ok(self
            .get_tags_for_node(node_id)?
            .into_iter()
            .map(|tag| tag.name)
            .collect())
    }

    pub fn query_tasks_by_tag(&self, tag_id: &str) -> Result<Vec<Node>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT n.* FROM nodes n
             JOIN node_tags nt ON nt.node_id = n.id
             WHERE nt.tag_id = ?1
               AND n.type = 'Task'
               AND n.deleted_at IS NULL
             ORDER BY n.position",
        )?;
        let nodes = stmt
            .query_map(rusqlite::params![tag_id], |row| Ok(map_node(row)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn sync_legacy_custom_tags(&self) -> Result<u32> {
        let conn = self.db.conn.lock().unwrap();
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, json_extract(properties, '$.custom.tags')
                 FROM nodes
                 WHERE type = 'Task'
                   AND deleted_at IS NULL
                   AND json_type(properties, '$.custom.tags') = 'array'",
            )?;
            let rows = stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };

        let mut assignments = 0;
        for (node_id, raw_tags) in rows {
            let parsed: Vec<String> = serde_json::from_str(&raw_tags).unwrap_or_default();
            for tag in parsed {
                let clean = tag.trim();
                if clean.is_empty() {
                    continue;
                }
                let now = chrono::Utc::now().to_rfc3339();
                let id = Uuid::now_v7().to_string();
                conn.execute(
                    "INSERT OR IGNORE INTO tags (id, name, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![&id, clean, &now],
                )?;
                let tag_id: String = conn.query_row(
                    "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                    rusqlite::params![clean],
                    |row| row.get(0),
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
                    rusqlite::params![&node_id, &tag_id],
                )?;
                assignments += 1;
            }
        }
        Ok(assignments)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn.lock().unwrap();
        conn.execute("DELETE FROM tags WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }
}
