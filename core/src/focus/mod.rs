use crate::db::Database;
use crate::error::{Error, Result};
use crate::node::model::NodeType;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusMethod {
    Pomodoro,
    Stopwatch,
    Flowtime,
}

impl FocusMethod {
    fn as_str(&self) -> &'static str {
        match self {
            FocusMethod::Pomodoro => "pomodoro",
            FocusMethod::Stopwatch => "stopwatch",
            FocusMethod::Flowtime => "stopwatch",
        }
    }

    fn workflow_str(&self) -> Option<&'static str> {
        match self {
            FocusMethod::Flowtime => Some("flowtime"),
            _ => None,
        }
    }

    fn from_parts(value: &str, workflow: Option<&str>) -> Self {
        if workflow == Some("flowtime") {
            return FocusMethod::Flowtime;
        }
        match value {
            "pomodoro" => FocusMethod::Pomodoro,
            "stopwatch" => FocusMethod::Stopwatch,
            _ => FocusMethod::Stopwatch,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusState {
    Running,
    Paused,
    Completed,
    Stopped,
}

impl FocusState {
    fn from_str(value: &str) -> Self {
        match value {
            "running" => FocusState::Running,
            "paused" => FocusState::Paused,
            "completed" => FocusState::Completed,
            "stopped" => FocusState::Stopped,
            _ => FocusState::Stopped,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PomodoroPhase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl PomodoroPhase {
    fn as_str(&self) -> &'static str {
        match self {
            PomodoroPhase::Focus => "focus",
            PomodoroPhase::ShortBreak => "short_break",
            PomodoroPhase::LongBreak => "long_break",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "focus" => Some(PomodoroPhase::Focus),
            "short_break" => Some(PomodoroPhase::ShortBreak),
            "long_break" => Some(PomodoroPhase::LongBreak),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: String,
    pub task_id: Option<String>,
    pub task_title: String,
    pub method: FocusMethod,
    pub state: FocusState,
    pub pomodoro_phase: Option<PomodoroPhase>,
    pub pomodoro_cycle: i32,
    pub planned_seconds: i64,
    pub accumulated_seconds: i64,
    pub elapsed_seconds: i64,
    pub duration_seconds: i64,
    pub interruptions: i32,
    pub note: String,
    pub started_at: DateTime<Utc>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartFocusInput {
    pub method: FocusMethod,
    pub task_id: Option<String>,
    #[serde(default)]
    pub planned_seconds: i64,
    pub pomodoro_phase: Option<PomodoroPhase>,
    #[serde(default = "default_cycle")]
    pub pomodoro_cycle: i32,
    #[serde(default)]
    pub note: String,
}

fn default_cycle() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StopFocusInput {
    #[serde(default)]
    pub completed: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FocusSummary {
    pub today_seconds: i64,
    pub total_seconds: i64,
    pub pomodoro_count: i64,
    pub stopwatch_seconds: i64,
    pub flowtime_seconds: i64,
}

pub struct FocusService {
    db: Arc<Database>,
}

impl FocusService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn start(&self, input: StartFocusInput) -> Result<FocusSession> {
        self.validate_start(&input)?;
        let (task_id, task_title) = self.task_snapshot(input.task_id.as_deref())?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        {
            let conn = self.db.conn.lock().unwrap();
            if self.active_exists_locked(&conn)? {
                return Err(Error::Validation("A focus timer is already active".into()));
            }
            conn.execute(
                "INSERT INTO focus_sessions
                 (id, task_id, task_title, method, state, pomodoro_phase, pomodoro_cycle,
                  planned_seconds, accumulated_seconds, duration_seconds, interruptions, focus_workflow,
                  note, started_at, last_started_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?6, ?7, 0, 0, 0, ?8, ?9, ?10, ?10, ?10, ?10)",
                params![
                    &id,
                    task_id.as_deref(),
                    &task_title,
                    input.method.as_str(),
                    input.pomodoro_phase.as_ref().map(|phase| phase.as_str()),
                    input.pomodoro_cycle.max(1),
                    input.planned_seconds,
                    input.method.workflow_str(),
                    input.note,
                    now.to_rfc3339(),
                ],
            )?;
        }
        self.get(&id)
    }

    pub fn get_active(&self) -> Result<Option<FocusSession>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM focus_sessions
             WHERE state IN ('running', 'paused')
             ORDER BY started_at DESC
             LIMIT 1",
        )?;
        stmt.query_row([], map_focus_session).optional().map_err(Into::into)
    }

    pub fn list_recent(&self, limit: i64) -> Result<Vec<FocusSession>> {
        let conn = self.db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM focus_sessions
             WHERE state IN ('completed', 'stopped')
             ORDER BY datetime(started_at) DESC
             LIMIT ?1",
        )?;
        let sessions = stmt
            .query_map(params![limit.clamp(1, 500)], map_focus_session)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    pub fn summary(&self) -> Result<FocusSummary> {
        let conn = self.db.conn.lock().unwrap();
        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let mut summary = FocusSummary::default();
        let mut stmt = conn.prepare(
            "SELECT method, focus_workflow, duration_seconds, started_at
             FROM focus_sessions
             WHERE state = 'completed'",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (method, workflow, seconds, started_at) = row?;
            summary.total_seconds += seconds;
            if started_at.starts_with(&today) {
                summary.today_seconds += seconds;
            }
            if method == "pomodoro" {
                summary.pomodoro_count += 1;
            }
            if workflow.as_deref() == Some("flowtime") {
                summary.flowtime_seconds += seconds;
            } else if method == "stopwatch" {
                summary.stopwatch_seconds += seconds;
            }
        }
        Ok(summary)
    }

    pub fn pause(&self, id: &str) -> Result<FocusSession> {
        let current = self.get(id)?;
        if current.state != FocusState::Running {
            return Err(Error::Validation("Only a running focus timer can be paused".into()));
        }
        let now = Utc::now();
        let elapsed = elapsed_since(current.last_started_at, now);
        let next_accumulated = current.accumulated_seconds + elapsed;
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE focus_sessions
                 SET state = 'paused',
                     accumulated_seconds = ?1,
                     duration_seconds = ?1,
                     interruptions = interruptions + 1,
                     last_started_at = NULL,
                     updated_at = ?2
                 WHERE id = ?3",
                params![next_accumulated, now.to_rfc3339(), id],
            )?;
        }
        self.get(id)
    }

    pub fn resume(&self, id: &str) -> Result<FocusSession> {
        let current = self.get(id)?;
        if current.state != FocusState::Paused {
            return Err(Error::Validation("Only a paused focus timer can be resumed".into()));
        }
        let now = Utc::now();
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE focus_sessions
                 SET state = 'running',
                     last_started_at = ?1,
                     updated_at = ?1
                 WHERE id = ?2",
                params![now.to_rfc3339(), id],
            )?;
        }
        self.get(id)
    }

    pub fn stop(&self, id: &str, input: StopFocusInput) -> Result<FocusSession> {
        let current = self.get(id)?;
        if current.state != FocusState::Running && current.state != FocusState::Paused {
            return Err(Error::Validation("Only an active focus timer can be stopped".into()));
        }
        let now = Utc::now();
        let elapsed = if current.state == FocusState::Running {
            elapsed_since(current.last_started_at, now)
        } else {
            0
        };
        let duration = (current.accumulated_seconds + elapsed).max(0);
        let state = if input.completed { "completed" } else { "stopped" };
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute(
                "UPDATE focus_sessions
                 SET state = ?1,
                     accumulated_seconds = ?2,
                     duration_seconds = ?2,
                     note = COALESCE(?3, note),
                     last_started_at = NULL,
                     ended_at = ?4,
                     updated_at = ?4
                 WHERE id = ?5",
                params![state, duration, input.note, now.to_rfc3339(), id],
            )?;
        }
        self.get(id)
    }

    pub fn get(&self, id: &str) -> Result<FocusSession> {
        let conn = self.db.conn.lock().unwrap();
        conn.query_row("SELECT * FROM focus_sessions WHERE id = ?1", params![id], map_focus_session)
            .map_err(|_| Error::NotFound(format!("Focus session not found: {}", id)))
    }

    fn validate_start(&self, input: &StartFocusInput) -> Result<()> {
        if input.method == FocusMethod::Pomodoro {
            if input.planned_seconds < 1 || input.planned_seconds > 24 * 60 * 60 {
                return Err(Error::Validation(
                    "Pomodoro planned duration must be between 1 second and 24 hours".into(),
                ));
            }
            if input.pomodoro_phase.is_none() {
                return Err(Error::Validation("Pomodoro phase is required".into()));
            }
        }
        if (input.method == FocusMethod::Stopwatch || input.method == FocusMethod::Flowtime)
            && input.planned_seconds < 0
        {
            return Err(Error::Validation("Stopwatch duration cannot be negative".into()));
        }
        Ok(())
    }

    fn task_snapshot(&self, task_id: Option<&str>) -> Result<(Option<String>, String)> {
        let Some(task_id) = task_id else {
            return Ok((None, String::new()));
        };
        let conn = self.db.conn.lock().unwrap();
        let row: Option<(String, String, Option<String>)> = conn
            .query_row(
                "SELECT type, title, deleted_at FROM nodes WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((node_type, title, deleted_at)) = row else {
            return Err(Error::Validation("Linked focus task does not exist".into()));
        };
        if NodeType::from_str(&node_type) != Some(NodeType::Task) || deleted_at.is_some() {
            return Err(Error::Validation("Focus session can only link to an active task".into()));
        }
        Ok((Some(task_id.to_string()), title))
    }

    fn active_exists_locked(&self, conn: &rusqlite::Connection) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE state IN ('running', 'paused')",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

fn elapsed_since(last_started_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
    last_started_at
        .map(|started| (now - started).num_seconds().max(0))
        .unwrap_or(0)
}

fn parse_dt(value: String) -> DateTime<Utc> {
    value.parse::<DateTime<Utc>>().unwrap_or_default()
}

fn parse_dt_opt(value: Option<String>) -> Option<DateTime<Utc>> {
    value.and_then(|raw| raw.parse::<DateTime<Utc>>().ok())
}

fn map_focus_session(row: &rusqlite::Row) -> rusqlite::Result<FocusSession> {
    let state = FocusState::from_str(&row.get::<_, String>("state")?);
    let method_raw = row.get::<_, String>("method")?;
    let workflow = row.get::<_, Option<String>>("focus_workflow")?;
    let accumulated = row.get::<_, i64>("accumulated_seconds")?;
    let last_started_at = parse_dt_opt(row.get::<_, Option<String>>("last_started_at")?);
    let elapsed_seconds = if state == FocusState::Running {
        accumulated + elapsed_since(last_started_at, Utc::now())
    } else {
        accumulated
    };
    Ok(FocusSession {
        id: row.get("id")?,
        task_id: row.get("task_id")?,
        task_title: row.get("task_title")?,
        method: FocusMethod::from_parts(&method_raw, workflow.as_deref()),
        state,
        pomodoro_phase: row
            .get::<_, Option<String>>("pomodoro_phase")?
            .and_then(|value| PomodoroPhase::from_str(&value)),
        pomodoro_cycle: row.get("pomodoro_cycle")?,
        planned_seconds: row.get("planned_seconds")?,
        accumulated_seconds: accumulated,
        elapsed_seconds,
        duration_seconds: row.get("duration_seconds")?,
        interruptions: row.get("interruptions")?,
        note: row.get("note")?,
        started_at: parse_dt(row.get("started_at")?),
        last_started_at,
        ended_at: parse_dt_opt(row.get("ended_at")?),
        created_at: parse_dt(row.get("created_at")?),
        updated_at: parse_dt(row.get("updated_at")?),
    })
}
