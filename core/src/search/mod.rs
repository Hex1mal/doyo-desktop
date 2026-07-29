use crate::db::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node_id: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

pub struct SearchService {
    db: Arc<Database>,
}

impl SearchService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let conn = self.db.conn.lock().unwrap();
        let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
        let sql = "SELECT n.id, n.title, snippet(nodes_fts, 1, '<mark>', '</mark>', '...', 40) AS snippet, rank
                   FROM nodes_fts JOIN nodes n ON nodes_fts.node_id = n.id
                   WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL ORDER BY rank LIMIT 50";
        let mut stmt = conn.prepare(sql)?;
        let results = stmt
            .query_map(rusqlite::params![fts_query], |row| {
                Ok(SearchResult {
                    node_id: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    rank: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(results)
    }
}
