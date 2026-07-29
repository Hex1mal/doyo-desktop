use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::model::NodeType;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

const EXPORT_FORMAT: &str = "io.github.hex1mal.doyo.transfer";
const SUPPORTED_VERSION: u64 = 1;

pub struct ImportService {
    db: Arc<Database>,
}

impl ImportService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn import_json(&self, json: &str, parent_id: Option<&str>) -> Result<Vec<String>> {
        let document: Value = serde_json::from_str(json)?;
        let data = normalize_transfer_data(&document)?;
        let mut conn = self.db.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute_batch("PRAGMA foreign_keys = ON;")?;

        validate_destination_parent(&tx, parent_id)?;

        let mut tag_id_map = import_tags(&tx, data.get("tags"))?;
        let node_id_map = import_nodes(&tx, data.get("nodes"), parent_id)?;
        import_node_tags(&tx, data.get("nodeTags"), &node_id_map, &mut tag_id_map)?;
        import_time_blocks(&tx, data.get("timeBlocks"), &node_id_map)?;
        let habit_id_map = import_habits(&tx, data.get("habits"))?;
        import_habit_logs(&tx, data.get("habitLogs"), &habit_id_map)?;
        import_countdowns(&tx, data.get("countdowns"))?;
        import_focus_sessions(&tx, data.get("focusSessions"), &node_id_map)?;

        let imported = node_id_map.values().cloned().collect::<Vec<_>>();
        tx.commit()?;
        Ok(imported)
    }
}

fn normalize_transfer_data(document: &Value) -> Result<Value> {
    if let Some(nodes) = document.as_array() {
        return Ok(serde_json::json!({
            "nodes": nodes,
            "tags": [],
            "nodeTags": [],
            "timeBlocks": [],
            "habits": [],
            "habitLogs": [],
            "countdowns": [],
            "focusSessions": []
        }));
    }

    let format = document
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if format != EXPORT_FORMAT {
        return Err(Error::Validation(format!(
            "Unsupported JSON export format: expected {EXPORT_FORMAT}"
        )));
    }
    let version = document.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version != SUPPORTED_VERSION {
        return Err(Error::Validation(format!(
            "Unsupported JSON export version: {version}"
        )));
    }
    document
        .get("data")
        .cloned()
        .ok_or_else(|| Error::Validation("JSON export is missing data".into()))
}

fn validate_destination_parent(conn: &Connection, parent_id: Option<&str>) -> Result<()> {
    if let Some(parent_id) = parent_id {
        conn.query_row(
            "SELECT id FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
            params![parent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| {
            Error::Validation(format!("Import destination parent not found: {parent_id}"))
        })?;
    }
    Ok(())
}

fn import_tags(conn: &Connection, tags: Option<&Value>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for tag in array_or_empty(tags)? {
        let old_id = required_string(tag, "id")?;
        let name = required_string(tag, "name")?;
        let color = optional_string(tag, "color");
        let created_at = optional_string(tag, "created_at")
            .or_else(|| optional_string(tag, "createdAt"))
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![&name],
                |row| row.get(0),
            )
            .optional()?;
        let new_id = if let Some(existing) = existing {
            existing
        } else {
            let id = Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![&id, &name, &color, &created_at],
            )?;
            id
        };
        map.insert(old_id, new_id);
    }
    Ok(map)
}

fn import_nodes(
    conn: &Connection,
    nodes: Option<&Value>,
    destination_parent: Option<&str>,
) -> Result<HashMap<String, String>> {
    let nodes = array_or_empty(nodes)?;
    let original_ids = nodes
        .iter()
        .filter_map(|node| required_string(node, "id").ok())
        .collect::<HashSet<_>>();
    let mut imported = HashMap::new();
    let mut pending = nodes.iter().collect::<Vec<_>>();

    while !pending.is_empty() {
        let before = pending.len();
        let mut next_pending = Vec::new();
        for node in pending {
            let old_id = required_string(node, "id")?;
            if imported.contains_key(&old_id) {
                continue;
            }
            let old_parent =
                optional_string(node, "parent_id").or_else(|| optional_string(node, "parentId"));
            let mapped_parent = match old_parent.as_deref() {
                Some(parent) if imported.contains_key(parent) => imported.get(parent).cloned(),
                Some(parent) if original_ids.contains(parent) => {
                    next_pending.push(node);
                    continue;
                }
                Some(_) | None => destination_parent.map(ToString::to_string),
            };

            let node_type = required_string(node, "type")?;
            let parsed_type = NodeType::parse(&node_type).ok_or_else(|| {
                Error::Validation(format!("Unsupported node type in import: {node_type}"))
            })?;
            if destination_parent.is_some()
                && old_parent.is_none()
                && parsed_type == NodeType::Workspace
            {
                return Err(Error::Validation(
                    "Cannot import a Workspace under another node. Import this JSON at the root instead.".into(),
                ));
            }

            let new_id = Uuid::now_v7().to_string();
            let properties =
                normalize_json_text(node.get("properties")).unwrap_or_else(|| "{}".to_string());
            conn.execute(
                "INSERT INTO nodes
                 (id, parent_id, position, type, title, body, properties, is_collapsed,
                  is_completed, completed_at, deleted_at, version, clock, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    &new_id,
                    &mapped_parent,
                    number_field(node, "position", 0.0),
                    &node_type,
                    text_field(node, "title", "Untitled"),
                    text_field(node, "body", ""),
                    properties,
                    bool_field(node, "is_collapsed", "isCollapsed") as i32,
                    bool_field(node, "is_completed", "isCompleted") as i32,
                    optional_string(node, "completed_at")
                        .or_else(|| optional_string(node, "completedAt")),
                    optional_string(node, "deleted_at")
                        .or_else(|| optional_string(node, "deletedAt")),
                    integer_field(node, "version", 1),
                    text_field(node, "clock", ""),
                    text_field_any(
                        node,
                        &["created_at", "createdAt"],
                        &chrono::Utc::now().to_rfc3339()
                    ),
                    text_field_any(
                        node,
                        &["updated_at", "updatedAt"],
                        &chrono::Utc::now().to_rfc3339()
                    ),
                ],
            )?;
            imported.insert(old_id, new_id);
        }
        if next_pending.len() == before {
            return Err(Error::Validation(
                "Import contains a circular or unresolved node parent relationship".into(),
            ));
        }
        pending = next_pending;
    }

    Ok(imported)
}

fn import_node_tags(
    conn: &Connection,
    node_tags: Option<&Value>,
    node_id_map: &HashMap<String, String>,
    tag_id_map: &mut HashMap<String, String>,
) -> Result<()> {
    for relation in array_or_empty(node_tags)? {
        let old_node_id = required_string(relation, "node_id")
            .or_else(|_| required_string(relation, "nodeId"))?;
        let old_tag_id =
            required_string(relation, "tag_id").or_else(|_| required_string(relation, "tagId"))?;
        let Some(new_node_id) = node_id_map.get(&old_node_id) else {
            return Err(Error::Validation(format!(
                "Tag relation references missing node: {old_node_id}"
            )));
        };
        let Some(new_tag_id) = tag_id_map.get(&old_tag_id).cloned() else {
            return Err(Error::Validation(format!(
                "Tag relation references missing tag: {old_tag_id}"
            )));
        };
        conn.execute(
            "INSERT OR IGNORE INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
            params![new_node_id, new_tag_id],
        )?;
    }
    Ok(())
}

fn import_time_blocks(
    conn: &Connection,
    rows: Option<&Value>,
    node_id_map: &HashMap<String, String>,
) -> Result<()> {
    for row in array_or_empty(rows)? {
        let old_task_id =
            optional_string(row, "task_id").or_else(|| optional_string(row, "taskId"));
        let new_task_id = match old_task_id {
            Some(old_id) => Some(node_id_map.get(&old_id).cloned().ok_or_else(|| {
                Error::Validation(format!("Time block references missing task: {old_id}"))
            })?),
            None => None,
        };
        conn.execute(
            "INSERT INTO time_blocks
             (id, task_id, title, start_time, end_time, all_day, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                Uuid::now_v7().to_string(),
                new_task_id,
                text_field(row, "title", ""),
                text_field_any(row, &["start_time", "startTime"], ""),
                text_field_any(row, &["end_time", "endTime"], ""),
                bool_field(row, "all_day", "allDay") as i32,
                text_field(row, "notes", ""),
                text_field_any(
                    row,
                    &["created_at", "createdAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
                text_field_any(
                    row,
                    &["updated_at", "updatedAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
            ],
        )?;
    }
    Ok(())
}

fn import_habits(conn: &Connection, rows: Option<&Value>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for row in array_or_empty(rows)? {
        let old_id = required_string(row, "id")?;
        let new_id = Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO habits
             (id, title, icon, color, frequency, goal, goal_unit, start_date, reminder_time,
              archived, position, created_at, updated_at, days)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &new_id,
                text_field(row, "title", "Untitled habit"),
                text_field(row, "icon", ""),
                optional_string(row, "color"),
                text_field(row, "frequency", "daily"),
                number_field(row, "goal", 1.0),
                text_field(row, "goal_unit", "count"),
                text_field_any(row, &["start_date", "startDate"], ""),
                optional_string(row, "reminder_time")
                    .or_else(|| optional_string(row, "reminderTime")),
                bool_field(row, "archived", "archived") as i32,
                number_field(row, "position", 0.0),
                text_field_any(
                    row,
                    &["created_at", "createdAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
                text_field_any(
                    row,
                    &["updated_at", "updatedAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
                normalize_json_text(row.get("days")).unwrap_or_else(|| "[]".to_string()),
            ],
        )?;
        map.insert(old_id, new_id);
    }
    Ok(map)
}

fn import_habit_logs(
    conn: &Connection,
    rows: Option<&Value>,
    habit_id_map: &HashMap<String, String>,
) -> Result<()> {
    for row in array_or_empty(rows)? {
        let old_habit_id =
            required_string(row, "habit_id").or_else(|_| required_string(row, "habitId"))?;
        let new_habit_id = habit_id_map.get(&old_habit_id).ok_or_else(|| {
            Error::Validation(format!(
                "Habit log references missing habit: {old_habit_id}"
            ))
        })?;
        conn.execute(
            "INSERT INTO habit_logs
             (id, habit_id, log_date, status, value, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::now_v7().to_string(),
                new_habit_id,
                text_field_any(row, &["log_date", "logDate"], ""),
                text_field(row, "status", "completed"),
                number_field(row, "value", 1.0),
                text_field(row, "note", ""),
                text_field_any(
                    row,
                    &["created_at", "createdAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
                text_field_any(
                    row,
                    &["updated_at", "updatedAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
            ],
        )?;
    }
    Ok(())
}

fn import_countdowns(conn: &Connection, rows: Option<&Value>) -> Result<()> {
    for row in array_or_empty(rows)? {
        conn.execute(
            "INSERT INTO countdowns
             (id, title, target_date, mode, icon, color, recurrence, reminder_at,
              archived, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                Uuid::now_v7().to_string(),
                text_field(row, "title", "Untitled countdown"),
                text_field_any(row, &["target_date", "targetDate"], ""),
                text_field(row, "mode", "countdown"),
                text_field(row, "icon", ""),
                optional_string(row, "color"),
                optional_string(row, "recurrence"),
                optional_string(row, "reminder_at").or_else(|| optional_string(row, "reminderAt")),
                bool_field(row, "archived", "archived") as i32,
                number_field(row, "position", 0.0),
                text_field_any(
                    row,
                    &["created_at", "createdAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
                text_field_any(
                    row,
                    &["updated_at", "updatedAt"],
                    &chrono::Utc::now().to_rfc3339()
                ),
            ],
        )?;
    }
    Ok(())
}

fn import_focus_sessions(
    conn: &Connection,
    rows: Option<&Value>,
    node_id_map: &HashMap<String, String>,
) -> Result<()> {
    for row in array_or_empty(rows)? {
        let old_task_id =
            optional_string(row, "task_id").or_else(|| optional_string(row, "taskId"));
        let new_task_id = match old_task_id {
            Some(old_id) => node_id_map.get(&old_id).cloned(),
            None => None,
        };
        conn.execute(
            "INSERT INTO focus_sessions
             (id, task_id, task_title, method, state, pomodoro_phase, pomodoro_cycle,
              planned_seconds, accumulated_seconds, duration_seconds, interruptions,
              note, started_at, last_started_at, ended_at, created_at, updated_at, focus_workflow)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                Uuid::now_v7().to_string(),
                new_task_id,
                text_field(row, "task_title", ""),
                text_field(row, "method", "stopwatch"),
                text_field(row, "state", "stopped"),
                optional_string(row, "pomodoro_phase").or_else(|| optional_string(row, "pomodoroPhase")),
                integer_field(row, "pomodoro_cycle", 1),
                integer_field(row, "planned_seconds", 0),
                integer_field(row, "accumulated_seconds", 0),
                integer_field(row, "duration_seconds", 0),
                integer_field(row, "interruptions", 0),
                text_field(row, "note", ""),
                text_field_any(row, &["started_at", "startedAt"], &chrono::Utc::now().to_rfc3339()),
                optional_string(row, "last_started_at").or_else(|| optional_string(row, "lastStartedAt")),
                optional_string(row, "ended_at").or_else(|| optional_string(row, "endedAt")),
                text_field_any(row, &["created_at", "createdAt"], &chrono::Utc::now().to_rfc3339()),
                text_field_any(row, &["updated_at", "updatedAt"], &chrono::Utc::now().to_rfc3339()),
                optional_string(row, "focus_workflow").or_else(|| optional_string(row, "focusWorkflow")),
            ],
        )?;
    }
    Ok(())
}

fn array_or_empty(value: Option<&Value>) -> Result<&[Value]> {
    match value {
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(Error::Validation(
            "Expected an array in JSON export data".into(),
        )),
        None => Ok(&[]),
    }
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    optional_string(value, field)
        .ok_or_else(|| Error::Validation(format!("Missing required field: {field}")))
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn text_field(value: &Value, field: &str, fallback: &str) -> String {
    optional_string(value, field).unwrap_or_else(|| fallback.to_string())
}

fn text_field_any(value: &Value, fields: &[&str], fallback: &str) -> String {
    fields
        .iter()
        .find_map(|field| optional_string(value, field))
        .unwrap_or_else(|| fallback.to_string())
}

fn number_field(value: &Value, field: &str, fallback: f64) -> f64 {
    value.get(field).and_then(Value::as_f64).unwrap_or(fallback)
}

fn integer_field(value: &Value, field: &str, fallback: i64) -> i64 {
    value.get(field).and_then(Value::as_i64).unwrap_or(fallback)
}

fn bool_field(value: &Value, snake: &str, camel: &str) -> bool {
    value
        .get(snake)
        .or_else(|| value.get(camel))
        .and_then(|value| match value {
            Value::Bool(value) => Some(*value),
            Value::Number(value) => Some(value.as_i64().unwrap_or(0) != 0),
            _ => None,
        })
        .unwrap_or(false)
}

fn normalize_json_text(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) => Some(value.clone()),
        Some(value) => Some(value.to_string()),
        None => None,
    }
}
