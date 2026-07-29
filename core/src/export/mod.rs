use crate::db::Database;
use crate::error::Result;
use rusqlite::types::ValueRef;
use rusqlite::{params, params_from_iter, Connection};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

const EXPORT_FORMAT: &str = "io.github.hex1mal.doyo.transfer";
const EXPORT_VERSION: u32 = 1;

pub struct ExportService {
    db: Arc<Database>,
}

impl ExportService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn export_json(&self, root_id: Option<&str>) -> Result<String> {
        let conn = self.db.conn.lock().unwrap();
        let node_ids = selected_node_ids(&conn, root_id)?;
        let nodes = rows_for_ids(&conn, "nodes", "id", &node_ids)?;
        let tag_ids = related_tag_ids(&conn, &node_ids)?;

        let envelope = json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "exportedAt": chrono::Utc::now().to_rfc3339(),
            "scope": {
                "rootId": root_id,
                "kind": if root_id.is_some() { "subtree" } else { "full" },
                "note": "JSON transfer data is not a byte-for-byte backup. Use SQLite backup/restore for exact database restore."
            },
            "data": {
                "nodes": nodes,
                "tags": rows_for_ids(&conn, "tags", "id", &tag_ids)?,
                "nodeTags": rows_for_node_ids(&conn, "node_tags", &node_ids)?,
                "timeBlocks": related_time_blocks(&conn, &node_ids, root_id.is_none())?,
                "habits": if root_id.is_none() { rows_all(&conn, "habits")? } else { vec![] },
                "habitLogs": if root_id.is_none() { rows_all(&conn, "habit_logs")? } else { vec![] },
                "countdowns": if root_id.is_none() { rows_all(&conn, "countdowns")? } else { vec![] },
                "focusSessions": related_focus_sessions(&conn, &node_ids, root_id.is_none())?,
            }
        });

        Ok(serde_json::to_string_pretty(&envelope)?)
    }

    pub fn export_markdown(&self, root_id: Option<&str>, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let conn = self.db.conn.lock().unwrap();
        let node_ids = selected_node_ids(&conn, root_id)?;
        let nodes = rows_for_ids(&conn, "nodes", "id", &node_ids)?;
        let id_set: HashSet<String> = node_ids.into_iter().collect();
        let mut by_parent: HashMap<Option<String>, Vec<Value>> = HashMap::new();

        for node in nodes {
            let parent_id = string_field(&node, "parent_id");
            let visible_parent = parent_id.filter(|id| id_set.contains(id));
            by_parent.entry(visible_parent).or_default().push(node);
        }
        for children in by_parent.values_mut() {
            children.sort_by(|a, b| {
                number_field(a, "position")
                    .partial_cmp(&number_field(b, "position"))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| string_field(a, "title").cmp(&string_field(b, "title")))
            });
        }

        write_markdown_children(output_dir, &by_parent, None)?;
        Ok(())
    }
}

fn selected_node_ids(conn: &Connection, root_id: Option<&str>) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    match root_id {
        Some(id) => {
            let mut stmt = conn.prepare(
                "WITH RECURSIVE subtree(id, depth) AS (
                    SELECT id, 0 FROM nodes WHERE id = ?1
                    UNION ALL
                    SELECT n.id, s.depth + 1 FROM nodes n JOIN subtree s ON n.parent_id = s.id
                 )
                 SELECT id FROM subtree ORDER BY depth",
            )?;
            for row in stmt.query_map(params![id], |row| row.get(0))? {
                ids.push(row?);
            }
        }
        None => {
            let mut stmt =
                conn.prepare("SELECT id FROM nodes ORDER BY parent_id IS NOT NULL, position")?;
            for row in stmt.query_map([], |row| row.get(0))? {
                ids.push(row?);
            }
        }
    }
    Ok(ids)
}

fn related_tag_ids(conn: &Connection, node_ids: &[String]) -> Result<Vec<String>> {
    if node_ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT DISTINCT tag_id FROM node_tags WHERE node_id IN ({})",
        placeholders(node_ids.len())
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(node_ids), |row| row.get(0))?;
    Ok(rows.collect::<std::result::Result<Vec<String>, _>>()?)
}

fn related_time_blocks(
    conn: &Connection,
    node_ids: &[String],
    include_standalone: bool,
) -> Result<Vec<Value>> {
    if node_ids.is_empty() {
        return if include_standalone {
            query_json(
                conn,
                "SELECT * FROM time_blocks WHERE task_id IS NULL ORDER BY start_time",
                [],
            )
        } else {
            Ok(vec![])
        };
    }
    let sql = if include_standalone {
        format!(
            "SELECT * FROM time_blocks WHERE task_id IS NULL OR task_id IN ({}) ORDER BY start_time",
            placeholders(node_ids.len())
        )
    } else {
        format!(
            "SELECT * FROM time_blocks WHERE task_id IN ({}) ORDER BY start_time",
            placeholders(node_ids.len())
        )
    };
    query_json_dynamic(conn, &sql, node_ids)
}

fn related_focus_sessions(
    conn: &Connection,
    node_ids: &[String],
    include_standalone: bool,
) -> Result<Vec<Value>> {
    if node_ids.is_empty() {
        return if include_standalone {
            query_json(
                conn,
                "SELECT * FROM focus_sessions WHERE task_id IS NULL ORDER BY started_at",
                [],
            )
        } else {
            Ok(vec![])
        };
    }
    let sql = if include_standalone {
        format!(
            "SELECT * FROM focus_sessions WHERE task_id IS NULL OR task_id IN ({}) ORDER BY started_at",
            placeholders(node_ids.len())
        )
    } else {
        format!(
            "SELECT * FROM focus_sessions WHERE task_id IN ({}) ORDER BY started_at",
            placeholders(node_ids.len())
        )
    };
    query_json_dynamic(conn, &sql, node_ids)
}

fn rows_all(conn: &Connection, table: &str) -> Result<Vec<Value>> {
    query_json(conn, &format!("SELECT * FROM {table}"), [])
}

fn rows_for_node_ids(conn: &Connection, table: &str, node_ids: &[String]) -> Result<Vec<Value>> {
    rows_for_ids(conn, table, "node_id", node_ids)
}

fn rows_for_ids(
    conn: &Connection,
    table: &str,
    column: &str,
    ids: &[String],
) -> Result<Vec<Value>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT * FROM {table} WHERE {column} IN ({})",
        placeholders(ids.len())
    );
    query_json_dynamic(conn, &sql, ids)
}

fn query_json_dynamic(conn: &Connection, sql: &str, values: &[String]) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params_from_iter(values), row_to_json)?;
    Ok(rows.collect::<std::result::Result<Vec<Value>, _>>()?)
}

fn query_json<P: rusqlite::Params>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, row_to_json)?;
    Ok(rows.collect::<std::result::Result<Vec<Value>, _>>()?)
}

fn row_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let mut object = Map::new();
    let row_ref = row.as_ref();
    for index in 0..row_ref.column_count() {
        let name = row_ref.column_name(index)?.to_string();
        object.insert(name, sqlite_value_to_json(row.get_ref(index)?));
    }
    Ok(Value::Object(object))
}

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => json!(value),
        ValueRef::Real(value) => json!(value),
        ValueRef::Text(value) => {
            let text = String::from_utf8_lossy(value).to_string();
            match serde_json::from_str::<Value>(&text) {
                Ok(parsed) if text.trim_start().starts_with(['{', '[']) => parsed,
                _ => Value::String(text),
            }
        }
        ValueRef::Blob(value) => Value::String(format!("<{} bytes>", value.len())),
    }
}

fn placeholders(len: usize) -> String {
    std::iter::repeat_n("?", len).collect::<Vec<_>>().join(",")
}

fn write_markdown_children(
    output_dir: &Path,
    by_parent: &HashMap<Option<String>, Vec<Value>>,
    parent_id: Option<String>,
) -> Result<()> {
    let Some(children) = by_parent.get(&parent_id) else {
        return Ok(());
    };
    for node in children {
        let id = string_field(node, "id").unwrap_or_default();
        let title = string_field(node, "title").unwrap_or_else(|| "Untitled".to_string());
        let node_type = string_field(node, "type").unwrap_or_else(|| "Node".to_string());
        let body = string_field(node, "body").unwrap_or_default();
        let prefix = short_id(&id);
        let filename = format!("{prefix}-{}.md", sanitize_title(&title));
        let file_path = output_dir.join(filename);
        let content = format!("# {title}\n\nType: `{node_type}`\nID: `{id}`\n\n{body}\n");
        std::fs::write(file_path, content)?;

        if by_parent.contains_key(&Some(id.clone())) {
            let dir = output_dir.join(format!("{prefix}-{}", sanitize_title(&title)));
            std::fs::create_dir_all(&dir)?;
            write_markdown_children(&dir, by_parent, Some(id))?;
        }
    }
    Ok(())
}

fn sanitize_title(title: &str) -> String {
    let mut sanitized = String::new();
    for ch in title.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ' ') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    let collapsed = sanitized.split_whitespace().collect::<Vec<_>>().join("-");
    if collapsed.is_empty() {
        "untitled".to_string()
    } else {
        collapsed.chars().take(80).collect()
    }
}

fn short_id(id: &str) -> String {
    let compact = id
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    if compact.len() <= 14 {
        compact
    } else {
        format!("{}{}", &compact[..8], &compact[compact.len() - 6..])
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field)?.as_str().map(ToString::to_string)
}

fn number_field(value: &Value, field: &str) -> f64 {
    value.get(field).and_then(Value::as_f64).unwrap_or(0.0)
}
