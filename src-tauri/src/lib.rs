use doyo_core::backup::BackupService;
use doyo_core::countdown::{
    Countdown, CountdownService, CreateCountdownInput, UpdateCountdownInput,
};
use doyo_core::db::Database;
use doyo_core::focus::{FocusService, FocusSession, FocusSummary, StartFocusInput, StopFocusInput};
use doyo_core::habit::{
    CreateHabitInput, Habit, HabitLog, HabitService, HabitSummary, UpdateHabitInput,
    UpsertHabitLogInput,
};
use doyo_core::node::model::*;
use doyo_core::node::service::NodeService;
use doyo_core::saved_filter::{
    CreateSavedFilterInput, SavedFilter, SavedFilterService, UpdateSavedFilterInput,
};
use doyo_core::settings::SettingsRepository;
use doyo_core::tag::{Tag, TagRepository, TagService};
use doyo_core::time_block::{
    CreateTimeBlockInput, TimeBlock, TimeBlockService, UpdateTimeBlockInput,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;

pub struct AppState {
    pub node_service: std::sync::Mutex<NodeService>,
    pub db: Arc<Database>,
    pub db_path: std::path::PathBuf,
    pub backup_dir: std::path::PathBuf,
    pub migration_backup_dir: std::path::PathBuf,
    pub startup: std::sync::Mutex<StartupReport>,
}

/// Directory holding databases that could not be opened. Never deleted
/// automatically: a database we failed to read is still the user's data.
const QUARANTINE_DIR_NAME: &str = "unopenable";
const MIGRATION_BACKUP_DIR_NAME: &str = "migration-backups";

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupStatus {
    /// The database opened and migrated normally.
    Ok,
    /// The database could not be used; it was set aside and a new one started.
    Recovered,
    /// No usable database directory at all; running from memory, nothing persists.
    Ephemeral,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupReport {
    pub status: StartupStatus,
    /// One-line summary suitable for showing to the user.
    pub summary: Option<String>,
    /// The underlying error, for users who want to know what actually happened.
    pub detail: Option<String>,
    /// Where the unusable database was preserved.
    pub quarantined_path: Option<String>,
    /// Databases that pass validation and could be restored right now.
    pub recovery_candidates: Vec<RecoveryCandidate>,
}

impl StartupReport {
    fn healthy() -> Self {
        Self {
            status: StartupStatus::Ok,
            summary: None,
            detail: None,
            quarantined_path: None,
            recovery_candidates: Vec::new(),
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidate {
    pub name: String,
    /// Which directory it came from: `backup` or `migrationBackup`.
    pub source: String,
    pub schema_version: i64,
    pub size_bytes: u64,
    /// RFC3339 modification time, when the filesystem reports one.
    pub modified_at: Option<String>,
}

const LEGACY_TODOAPP_APP_DIR_NAME: &str = "com.todoapp.desktop";
const LEGACY_TODOAPP_DB_NAME: &str = "todoapp.db";
const LEGACY_DOYO_APP_DIR_NAME: &str = "io.github.sembee.doyo";
const LEGACY_DOYO_DB_NAME: &str = "doyo.db";
const NEW_DB_NAME: &str = "doyo.db";

struct LegacyDataSource {
    app_dir_name: &'static str,
    db_name: &'static str,
    label: &'static str,
}

const LEGACY_DATA_SOURCES: [LegacyDataSource; 2] = [
    LegacyDataSource {
        app_dir_name: LEGACY_DOYO_APP_DIR_NAME,
        db_name: LEGACY_DOYO_DB_NAME,
        label: "legacy-doyo",
    },
    LegacyDataSource {
        app_dir_name: LEGACY_TODOAPP_APP_DIR_NAME,
        db_name: LEGACY_TODOAPP_DB_NAME,
        label: "legacy-todoapp",
    },
];

fn validate_sqlite_database(path: &Path) -> Result<i64, String> {
    let db = Database::open(path)
        .map_err(|e| format!("failed to open database {}: {e}", path.display()))?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("failed integrity check for {}: {e}", path.display()))?;
    if integrity != "ok" {
        return Err(format!(
            "database integrity check failed for {}: {integrity}",
            path.display()
        ));
    }
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("failed to read schema_version for {}: {e}", path.display()))
}

fn copy_file_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(source, destination).map(|_| ()).map_err(|e| {
        format!(
            "failed to copy {} to {}: {e}",
            source.display(),
            destination.display()
        )
    })
}

fn copy_dir_contents_if_missing(
    source_dir: &Path,
    destination_dir: &Path,
    source_db_name: &str,
) -> Result<(), String> {
    if !source_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(destination_dir).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(source_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with(source_db_name) {
            continue;
        }
        let destination_path = destination_dir.join(file_name);
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            copy_dir_contents_if_missing(&source_path, &destination_path, source_db_name)?;
        } else if metadata.is_file() {
            copy_file_if_missing(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn create_migration_safety_backup(
    source_db: &Path,
    new_app_dir: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let backup_dir = new_app_dir.join("migration-backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
    let backup_path = backup_dir.join(format!(
        "pre-doyo-migration-{label}-{stamp}-{}.db",
        uuid::Uuid::now_v7()
    ));
    std::fs::copy(source_db, &backup_path)
        .map(|_| backup_path)
        .map_err(|e| {
            format!(
                "failed to create migration safety backup from {}: {e}",
                source_db.display()
            )
        })
}

/// Snapshot the database before a schema upgrade so a bad migration is
/// recoverable. Returns `None` when the schema is already current, or when there
/// is no database yet to protect.
///
/// The WAL is truncated into the main file first: a plain file copy of a WAL-mode
/// database would otherwise miss everything not yet checkpointed.
fn backup_before_schema_upgrade(db: &Database, db_path: &Path, app_dir: &Path) -> Option<PathBuf> {
    if !db_path.exists() {
        return None;
    }
    let conn = db.conn.lock().ok()?;
    let current: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    // 0 means a brand-new database; there is nothing worth protecting.
    if current == 0 || current >= doyo_core::db::LATEST_SCHEMA_VERSION {
        return None;
    }
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    drop(conn);

    match create_migration_safety_backup(db_path, app_dir, &format!("schema-v{current}")) {
        Ok(path) => {
            eprintln!(
                "doyo: created pre-migration backup at {} before upgrading schema v{current} -> v{}",
                path.display(),
                doyo_core::db::LATEST_SCHEMA_VERSION
            );
            Some(path)
        }
        Err(e) => {
            eprintln!("doyo: could not create pre-migration backup: {e}");
            None
        }
    }
}

/// Open the database and bring its schema up to date.
fn open_and_migrate(db_path: &Path, app_dir: &Path) -> Result<Arc<Database>, String> {
    let db = Arc::new(
        Database::open(db_path)
            .map_err(|e| format!("could not open {}: {e}", db_path.display()))?,
    );
    {
        let conn = db
            .conn
            .lock()
            .map_err(|e| format!("database lock poisoned: {e}"))?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("integrity check failed: {e}"))?;
        if integrity != "ok" {
            return Err(format!("database is corrupted: {integrity}"));
        }
    }
    backup_before_schema_upgrade(&db, db_path, app_dir);
    doyo_core::db::run_migrations(&db).map_err(|e| format!("could not migrate schema: {e}"))?;
    Ok(db)
}

/// Move an unusable database aside, preserving its WAL sidecars with it.
///
/// Renamed rather than deleted, always: a database we could not read may still
/// be recoverable by hand, and it is the user's data either way.
fn quarantine_database(db_path: &Path, app_dir: &Path) -> Result<PathBuf, String> {
    let quarantine_dir = app_dir.join(QUARANTINE_DIR_NAME);
    std::fs::create_dir_all(&quarantine_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
    let target = quarantine_dir.join(format!("doyo-unopenable-{stamp}.db"));

    std::fs::rename(db_path, &target)
        .map_err(|e| format!("could not set aside {}: {e}", db_path.display()))?;

    // Keep the sidecars with the file they belong to; they may hold the only
    // copy of recent transactions.
    if let Some(file_name) = db_path.file_name() {
        for suffix in ["-wal", "-shm"] {
            let mut sidecar_name = file_name.to_os_string();
            sidecar_name.push(suffix);
            let sidecar = db_path.with_file_name(&sidecar_name);
            if sidecar.exists() {
                let mut target_name = target.file_name().unwrap().to_os_string();
                target_name.push(suffix);
                let _ = std::fs::rename(&sidecar, target.with_file_name(target_name));
            }
        }
    }
    Ok(target)
}

/// List databases in `dir` that would actually pass a restore, newest first.
fn collect_recovery_candidates(dir: &Path, source: &str) -> Vec<RecoveryCandidate> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut candidates: Vec<(std::time::SystemTime, RecoveryCandidate)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        // Only offer files that will genuinely restore; a candidate that fails
        // at the moment of truth is worse than not listing it.
        let Ok(schema_version) = doyo_core::backup::validate_restorable_database(&path) else {
            continue;
        };
        let metadata = entry.metadata().ok();
        let modified = metadata.as_ref().and_then(|m| m.modified().ok());
        candidates.push((
            modified.unwrap_or(std::time::UNIX_EPOCH),
            RecoveryCandidate {
                name: entry.file_name().to_string_lossy().to_string(),
                source: source.to_string(),
                schema_version,
                size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified_at: modified
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()),
            },
        ));
    }
    candidates.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    candidates.into_iter().map(|(_, c)| c).collect()
}

fn all_recovery_candidates(app_dir: &Path) -> Vec<RecoveryCandidate> {
    let mut all = collect_recovery_candidates(&app_dir.join("backups"), "backup");
    all.extend(collect_recovery_candidates(
        &app_dir.join(MIGRATION_BACKUP_DIR_NAME),
        "migrationBackup",
    ));
    all
}

/// Bring up a database, recovering rather than crashing when that is not possible.
///
/// Startup must always reach a running window. A panic here leaves the user with
/// no application and no explanation, which is the worst outcome for a database
/// problem that is usually recoverable from an existing backup.
fn initialize_database(app_dir: &Path, db_path: &Path) -> (Arc<Database>, StartupReport) {
    match open_and_migrate(db_path, app_dir) {
        Ok(db) => (db, StartupReport::healthy()),
        Err(reason) => {
            eprintln!("doyo: database could not be opened: {reason}");

            let quarantined = if db_path.exists() {
                match quarantine_database(db_path, app_dir) {
                    Ok(path) => Some(path),
                    Err(e) => {
                        eprintln!("doyo: could not quarantine the database: {e}");
                        None
                    }
                }
            } else {
                None
            };

            // With the bad file out of the way, a fresh database should open.
            match open_and_migrate(db_path, app_dir) {
                Ok(db) => {
                    let report = StartupReport {
                        status: StartupStatus::Recovered,
                        summary: Some(
                            "Doyo could not open your database, so it started with an empty one."
                                .into(),
                        ),
                        detail: Some(reason),
                        quarantined_path: quarantined
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        recovery_candidates: all_recovery_candidates(app_dir),
                    };
                    (db, report)
                }
                Err(second) => {
                    eprintln!("doyo: could not start a fresh database either: {second}");
                    // Last resort: run from memory so the window still opens and
                    // can explain the problem. Nothing written here persists.
                    let db = Arc::new(
                        Database::open_in_memory().expect("in-memory database is always available"),
                    );
                    let _ = doyo_core::db::run_migrations(&db);
                    let report = StartupReport {
                        status: StartupStatus::Ephemeral,
                        summary: Some(
                            "Doyo cannot write to its data folder. Changes will not be saved."
                                .into(),
                        ),
                        detail: Some(format!("{reason}; {second}")),
                        quarantined_path: quarantined
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        recovery_candidates: all_recovery_candidates(app_dir),
                    };
                    (db, report)
                }
            }
        }
    }
}

fn migrate_legacy_doyo_data(new_app_dir: &Path) -> Result<(), String> {
    let new_db_path = new_app_dir.join(NEW_DB_NAME);
    if new_db_path.exists() {
        validate_sqlite_database(&new_db_path)?;
        return Ok(());
    }

    let Some(data_parent) = new_app_dir.parent() else {
        return Ok(());
    };

    let Some(source) = LEGACY_DATA_SOURCES.iter().find(|source| {
        data_parent
            .join(source.app_dir_name)
            .join(source.db_name)
            .exists()
    }) else {
        return Ok(());
    };
    let old_app_dir = data_parent.join(source.app_dir_name);
    let old_db_path = old_app_dir.join(source.db_name);

    let old_schema_version = validate_sqlite_database(&old_db_path)?;
    std::fs::create_dir_all(new_app_dir).map_err(|e| e.to_string())?;
    create_migration_safety_backup(&old_db_path, new_app_dir, source.label)?;

    copy_dir_contents_if_missing(
        &old_app_dir.join("backups"),
        &new_app_dir.join("backups"),
        source.db_name,
    )?;
    copy_dir_contents_if_missing(
        &old_app_dir.join("localstorage"),
        &new_app_dir.join("localstorage"),
        source.db_name,
    )?;
    copy_file_if_missing(&old_db_path, &new_db_path)?;

    for suffix in ["-wal", "-shm"] {
        let old_sidecar = old_app_dir.join(format!("{}{suffix}", source.db_name));
        if old_sidecar.exists() {
            copy_file_if_missing(
                &old_sidecar,
                &new_app_dir.join(format!("{NEW_DB_NAME}{suffix}")),
            )?;
        }
    }

    let new_schema_version = validate_sqlite_database(&new_db_path)?;
    if new_schema_version != old_schema_version {
        return Err(format!(
            "legacy data migration schema mismatch: old version {old_schema_version}, new version {new_schema_version}"
        ));
    }

    Ok(())
}

#[tauri::command]
fn node_get(state: tauri::State<AppState>, id: String) -> Result<Node, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_create(
    state: tauri::State<AppState>,
    parent_id: Option<String>,
    node_type: String,
    title: String,
    body: Option<String>,
) -> Result<Node, String> {
    let nt = NodeType::parse(&node_type).unwrap_or(NodeType::Task);
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    let input = CreateNodeInput {
        parent_id,
        node_type: nt,
        title,
        body: body.unwrap_or_default(),
        properties: NodeProperties::default(),
        position: None,
    };
    service.create(input).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_update(
    state: tauri::State<AppState>,
    id: String,
    changes: UpdateNodeInput,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.update(&id, changes).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_replace_properties(
    state: tauri::State<AppState>,
    id: String,
    properties: NodeProperties,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .replace_properties(&id, properties)
        .map_err(|e| e.to_string())
}

/// Change named property keys against the stored value.
///
/// Preferred over `node_replace_properties` for single-field edits: the UI sends
/// only what it is changing, so a stale client snapshot cannot ride along and
/// clobber a key another view updated.
#[tauri::command]
fn node_patch_properties(
    state: tauri::State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .patch_properties(&id, patch)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_delete(state: tauri::State<AppState>, id: String, permanent: bool) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.delete(&id, permanent).map_err(|e| e.to_string())
}

#[tauri::command]
fn trash_get_nodes(state: tauri::State<AppState>) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_deleted_nodes().map_err(|e| e.to_string())
}

#[tauri::command]
fn trash_restore(
    state: tauri::State<AppState>,
    id: String,
    destination_parent_id: Option<String>,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .restore(&id, destination_parent_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn trash_empty(state: tauri::State<AppState>) -> Result<u32, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
fn node_duplicate(state: tauri::State<AppState>, id: String) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.duplicate(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_move(
    state: tauri::State<AppState>,
    id: String,
    new_parent_id: Option<String>,
    position: f64,
) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .move_node(&id, new_parent_id.as_deref(), position)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_move_ordered(
    state: tauri::State<AppState>,
    id: String,
    new_parent_id: Option<String>,
    target_index: usize,
) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .move_node_ordered(&id, new_parent_id.as_deref(), target_index)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_reorder(
    state: tauri::State<AppState>,
    parent_id: String,
    child_ids: Vec<String>,
) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .reorder_children(&parent_id, &child_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_reorder_root(state: tauri::State<AppState>, child_ids: Vec<String>) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .reorder_root_children(&child_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_children(
    state: tauri::State<AppState>,
    parent_id: Option<String>,
) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .get_children(parent_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_ancestors(state: tauri::State<AppState>, id: String) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_ancestors(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_full(
    state: tauri::State<AppState>,
    root_id: Option<String>,
) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .get_full_tree(root_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_due_date(
    state: tauri::State<AppState>,
    id: String,
    due_date: Option<String>,
) -> Result<Node, String> {
    let dt = due_date.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    });
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.set_due_date(&id, dt).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_priority(
    state: tauri::State<AppState>,
    id: String,
    priority: i32,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .set_priority(&id, priority)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_toggle_complete(state: tauri::State<AppState>, id: String) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.toggle_complete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_completion(
    state: tauri::State<AppState>,
    id: String,
    completed: bool,
    cascade: bool,
) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .set_completion(&id, completed, cascade)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn node_incomplete_descendant_count(
    state: tauri::State<AppState>,
    id: String,
) -> Result<u32, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service
        .incomplete_task_descendant_count(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_query(
    state: tauri::State<AppState>,
    query: String,
    filters: SearchFilters,
) -> Result<Vec<SearchResult>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.search(&query, filters).map_err(|e| e.to_string())
}

#[tauri::command]
fn quick_find(state: tauri::State<AppState>, query: String) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.quick_find(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_today_tasks(state: tauri::State<AppState>) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_today_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_node_count(state: tauri::State<AppState>) -> Result<u32, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_node_count().map_err(|e| e.to_string())
}

#[tauri::command]
fn undo(state: tauri::State<AppState>) -> Result<String, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.undo().map_err(|e| e.to_string())
}

#[tauri::command]
fn redo(state: tauri::State<AppState>) -> Result<String, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.redo().map_err(|e| e.to_string())
}

#[tauri::command]
fn export_json(state: tauri::State<AppState>, root_id: Option<String>) -> Result<String, String> {
    use doyo_core::export::ExportService;
    let export_svc = ExportService::new(state.db.clone());
    export_svc
        .export_json(root_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn export_markdown(
    state: tauri::State<AppState>,
    root_id: Option<String>,
    output_dir: String,
) -> Result<(), String> {
    use doyo_core::export::ExportService;
    let export_svc = ExportService::new(state.db.clone());
    export_svc
        .export_markdown(root_id.as_deref(), std::path::Path::new(&output_dir))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn import_json(
    state: tauri::State<AppState>,
    json: String,
    parent_id: Option<String>,
) -> Result<Vec<String>, String> {
    use doyo_core::import::ImportService;
    let import_svc = ImportService::new(state.db.clone());
    import_svc
        .import_json(&json, parent_id.as_deref())
        .map_err(|e| e.to_string())
}

fn tag_service(state: &tauri::State<AppState>) -> TagService {
    TagService::new(TagRepository::new(state.db.clone()))
}

#[tauri::command]
fn tag_list(state: tauri::State<AppState>) -> Result<Vec<Tag>, String> {
    tag_service(&state).list_all().map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_create(
    state: tauri::State<AppState>,
    name: String,
    color: Option<String>,
) -> Result<Tag, String> {
    tag_service(&state)
        .create_tag(&name, color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_rename(
    state: tauri::State<AppState>,
    id: String,
    name: String,
    color: Option<String>,
) -> Result<Tag, String> {
    tag_service(&state)
        .rename_tag(&id, &name, color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    tag_service(&state)
        .delete_tag(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_assign(
    state: tauri::State<AppState>,
    node_id: String,
    tag_id: String,
) -> Result<(), String> {
    tag_service(&state)
        .assign_tag(&node_id, &tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_remove(
    state: tauri::State<AppState>,
    node_id: String,
    tag_id: String,
) -> Result<(), String> {
    tag_service(&state)
        .remove_tag_id(&node_id, &tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_get_for_node(state: tauri::State<AppState>, node_id: String) -> Result<Vec<Tag>, String> {
    tag_service(&state)
        .get_tags_for_node(&node_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_query_tasks(state: tauri::State<AppState>, tag_id: String) -> Result<Vec<Node>, String> {
    tag_service(&state)
        .query_tasks_by_tag(&tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_sync_legacy(state: tauri::State<AppState>) -> Result<u32, String> {
    tag_service(&state)
        .sync_legacy_custom_tags()
        .map_err(|e| e.to_string())
}

fn time_block_service(state: &tauri::State<AppState>) -> TimeBlockService {
    TimeBlockService::new(state.db.clone())
}

#[tauri::command]
fn time_block_list(
    state: tauri::State<AppState>,
    start: String,
    end: String,
) -> Result<Vec<TimeBlock>, String> {
    let start = chrono::DateTime::parse_from_rfc3339(&start)
        .map_err(|e| e.to_string())?
        .with_timezone(&chrono::Utc);
    let end = chrono::DateTime::parse_from_rfc3339(&end)
        .map_err(|e| e.to_string())?
        .with_timezone(&chrono::Utc);
    time_block_service(&state)
        .list_between(start, end)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn time_block_create(
    state: tauri::State<AppState>,
    input: CreateTimeBlockInput,
) -> Result<TimeBlock, String> {
    time_block_service(&state)
        .create(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn time_block_update(
    state: tauri::State<AppState>,
    id: String,
    input: UpdateTimeBlockInput,
) -> Result<TimeBlock, String> {
    time_block_service(&state)
        .update(&id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn time_block_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    time_block_service(&state)
        .delete(&id)
        .map_err(|e| e.to_string())
}

fn focus_service(state: &tauri::State<AppState>) -> FocusService {
    FocusService::new(state.db.clone())
}

#[tauri::command]
fn focus_start(
    state: tauri::State<AppState>,
    input: StartFocusInput,
) -> Result<FocusSession, String> {
    focus_service(&state)
        .start(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_get_active(state: tauri::State<AppState>) -> Result<Option<FocusSession>, String> {
    focus_service(&state)
        .get_active()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_pause(state: tauri::State<AppState>, id: String) -> Result<FocusSession, String> {
    focus_service(&state).pause(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_resume(state: tauri::State<AppState>, id: String) -> Result<FocusSession, String> {
    focus_service(&state).resume(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_stop(
    state: tauri::State<AppState>,
    id: String,
    input: StopFocusInput,
) -> Result<FocusSession, String> {
    focus_service(&state)
        .stop(&id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_list(state: tauri::State<AppState>, limit: i64) -> Result<Vec<FocusSession>, String> {
    focus_service(&state)
        .list_recent(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_summary(state: tauri::State<AppState>) -> Result<FocusSummary, String> {
    focus_service(&state).summary().map_err(|e| e.to_string())
}

fn saved_filter_service(state: &tauri::State<AppState>) -> SavedFilterService {
    SavedFilterService::new(state.db.clone())
}

#[tauri::command]
fn saved_filter_list(state: tauri::State<AppState>) -> Result<Vec<SavedFilter>, String> {
    saved_filter_service(&state)
        .list()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_filter_create(
    state: tauri::State<AppState>,
    input: CreateSavedFilterInput,
) -> Result<SavedFilter, String> {
    saved_filter_service(&state)
        .create(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_filter_update(
    state: tauri::State<AppState>,
    id: String,
    input: UpdateSavedFilterInput,
) -> Result<SavedFilter, String> {
    saved_filter_service(&state)
        .update(&id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_filter_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    saved_filter_service(&state)
        .delete(&id)
        .map_err(|e| e.to_string())
}

fn habit_service(state: &tauri::State<AppState>) -> HabitService {
    HabitService::new(state.db.clone())
}

#[tauri::command]
fn habit_list(state: tauri::State<AppState>, include_archived: bool) -> Result<Vec<Habit>, String> {
    habit_service(&state)
        .list(include_archived)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_create(state: tauri::State<AppState>, input: CreateHabitInput) -> Result<Habit, String> {
    habit_service(&state)
        .create(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_update(
    state: tauri::State<AppState>,
    id: String,
    input: UpdateHabitInput,
) -> Result<Habit, String> {
    habit_service(&state)
        .update(&id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_archive(
    state: tauri::State<AppState>,
    id: String,
    archived: bool,
) -> Result<Habit, String> {
    habit_service(&state)
        .archive(&id, archived)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    habit_service(&state).delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_log_upsert(
    state: tauri::State<AppState>,
    input: UpsertHabitLogInput,
) -> Result<HabitLog, String> {
    habit_service(&state)
        .upsert_log(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_log_delete(
    state: tauri::State<AppState>,
    habit_id: String,
    log_date: String,
) -> Result<(), String> {
    let log_date =
        chrono::NaiveDate::parse_from_str(&log_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    habit_service(&state)
        .delete_log(&habit_id, log_date)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_log_list(
    state: tauri::State<AppState>,
    from: String,
    to: String,
) -> Result<Vec<HabitLog>, String> {
    let from = chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let to = chrono::NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|e| e.to_string())?;
    habit_service(&state)
        .list_logs(from, to)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn habit_summary(
    state: tauri::State<AppState>,
    from: String,
    to: String,
) -> Result<HabitSummary, String> {
    let from = chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let to = chrono::NaiveDate::parse_from_str(&to, "%Y-%m-%d").map_err(|e| e.to_string())?;
    habit_service(&state)
        .summary(from, to)
        .map_err(|e| e.to_string())
}

fn countdown_service(state: &tauri::State<AppState>) -> CountdownService {
    CountdownService::new(state.db.clone())
}

#[tauri::command]
fn countdown_list(
    state: tauri::State<AppState>,
    include_archived: bool,
) -> Result<Vec<Countdown>, String> {
    countdown_service(&state)
        .list(include_archived)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn countdown_create(
    state: tauri::State<AppState>,
    input: CreateCountdownInput,
) -> Result<Countdown, String> {
    countdown_service(&state)
        .create(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn countdown_update(
    state: tauri::State<AppState>,
    id: String,
    input: UpdateCountdownInput,
) -> Result<Countdown, String> {
    countdown_service(&state)
        .update(&id, input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn countdown_archive(
    state: tauri::State<AppState>,
    id: String,
    archived: bool,
) -> Result<Countdown, String> {
    countdown_service(&state)
        .archive(&id, archived)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn countdown_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    countdown_service(&state)
        .delete(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn countdown_reorder(
    state: tauri::State<AppState>,
    ids: Vec<String>,
) -> Result<Vec<Countdown>, String> {
    countdown_service(&state)
        .reorder(&ids)
        .map_err(|e| e.to_string())
}

fn settings_repo(state: &tauri::State<AppState>) -> SettingsRepository {
    SettingsRepository::new(state.db.clone())
}

#[tauri::command]
fn settings_get(
    state: tauri::State<AppState>,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    settings_repo(&state)
        .get::<serde_json::Value>(&key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn settings_set(
    state: tauri::State<AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    settings_repo(&state)
        .set(&key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn settings_delete(state: tauri::State<AppState>, key: String) -> Result<(), String> {
    settings_repo(&state)
        .delete(&key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn settings_list(
    state: tauri::State<AppState>,
    prefix: Option<String>,
) -> Result<Vec<(String, serde_json::Value)>, String> {
    settings_repo(&state)
        .list(prefix.as_deref())
        .map_err(|e| e.to_string())
}

fn backup_service(state: &tauri::State<AppState>) -> BackupService {
    BackupService::new(state.db_path.clone(), state.backup_dir.clone(), 20)
}

/// Flush the WAL into the main database file so a file-level copy sees every
/// committed change.
///
/// `wal_checkpoint` reports busy/incomplete through its result row rather than an
/// error, so the row is inspected: swallowing it would let a partial checkpoint
/// produce a silently incomplete backup.
fn checkpoint_database(state: &tauri::State<AppState>) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let (busy, _log, _checkpointed): (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("failed to checkpoint database: {e}"))?;
    if busy != 0 {
        return Err(
            "Database is busy and could not be fully checkpointed. Close other Doyo activity and try again.".into(),
        );
    }
    Ok(())
}

#[tauri::command]
fn backup_create(state: tauri::State<AppState>) -> Result<String, String> {
    checkpoint_database(&state)?;
    backup_service(&state)
        .create_backup()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn backup_list(state: tauri::State<AppState>) -> Result<Vec<String>, String> {
    backup_service(&state)
        .list_backups()
        .map_err(|e| e.to_string())
}

/// Make the file at `db_path` the live database for this running session.
///
/// Restore replaces the database file underneath handles that were cloned at
/// startup. Repointing the shared connection here means the restored data is
/// active immediately, instead of the user being told to restart and left
/// looking at data that is no longer on disk.
fn activate_database(state: &tauri::State<AppState>) -> Result<(), String> {
    state
        .db
        .reopen(&state.db_path)
        .map_err(|e| format!("restored database could not be opened: {e}"))?;

    // A backup may predate the current schema, so bring it up to date before
    // anything queries it.
    doyo_core::db::run_migrations(&state.db)
        .map_err(|e| format!("restored database could not be migrated: {e}"))?;

    // Undo history describes rows in the database we just replaced.
    state
        .node_service
        .lock()
        .map_err(|e| e.to_string())?
        .reset_history();

    // Whatever was wrong at startup no longer describes the live database.
    if let Ok(mut startup) = state.startup.lock() {
        *startup = StartupReport::healthy();
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    /// Pre-restore snapshot filename; restoring it undoes this restore.
    pub snapshot_name: Option<String>,
    /// True when the restored database is already live and no restart is needed.
    pub activated: bool,
    /// Set only when the data was restored but could not be made live.
    pub activation_error: Option<String>,
}

/// Run a restore and make the result live, keeping the session usable either way.
fn perform_restore(
    state: &tauri::State<AppState>,
    resolve: impl FnOnce(&BackupService) -> doyo_core::error::Result<Option<PathBuf>>,
) -> Result<RestoreOutcome, String> {
    checkpoint_database(state)?;

    // Close the file before it is replaced so SQLite finishes its WAL cleanup
    // against the database it actually opened.
    state.db.detach().map_err(|e| e.to_string())?;

    let service = backup_service(state);
    let restored = resolve(&service);

    let snapshot = match restored {
        Ok(snapshot) => snapshot,
        Err(e) => {
            // The live file was left untouched, so put the session back as it was.
            let _ = activate_database(state);
            return Err(e.to_string());
        }
    };

    let activation = activate_database(state);
    Ok(RestoreOutcome {
        snapshot_name: snapshot.map(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string_lossy().to_string())
        }),
        activated: activation.is_ok(),
        activation_error: activation.err(),
    })
}

#[tauri::command]
fn backup_restore(
    state: tauri::State<AppState>,
    backup_name: String,
) -> Result<RestoreOutcome, String> {
    perform_restore(&state, |service| service.restore_backup(&backup_name))
}

#[tauri::command]
fn startup_report(state: tauri::State<AppState>) -> Result<StartupReport, String> {
    state
        .startup
        .lock()
        .map(|report| report.clone())
        .map_err(|e| e.to_string())
}

/// Restore one of the candidates offered by the startup recovery screen.
///
/// `source` selects the directory, and the filename is validated against it, so
/// a caller cannot reach a file outside the two recovery directories.
#[tauri::command]
fn recovery_restore(
    state: tauri::State<AppState>,
    name: String,
    source: String,
) -> Result<RestoreOutcome, String> {
    let dir = match source.as_str() {
        "backup" => state.backup_dir.clone(),
        "migrationBackup" => state.migration_backup_dir.clone(),
        other => return Err(format!("Unknown recovery source: {other}")),
    };

    // Resolve inside the chosen directory and prove the result stayed there.
    let candidate = std::path::Path::new(&name);
    if candidate.components().count() != 1 {
        return Err("Recovery filename must not contain path separators".into());
    }
    let resolved = dir
        .join(candidate)
        .canonicalize()
        .map_err(|e| format!("Recovery file not found: {e}"))?;
    let canonical_dir = dir
        .canonicalize()
        .map_err(|e| format!("Recovery directory not found: {e}"))?;
    if !resolved.starts_with(&canonical_dir) {
        return Err("Recovery path escapes its directory".into());
    }

    perform_restore(&state, move |service| service.restore_from(&resolved))
}

/// Re-list recovery candidates on demand, for the recovery screen's refresh.
#[tauri::command]
fn recovery_candidates(state: tauri::State<AppState>) -> Result<Vec<RecoveryCandidate>, String> {
    let app_dir = state
        .db_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "database path has no parent directory".to_string())?;
    Ok(all_recovery_candidates(&app_dir))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Nothing in here may panic. A database problem must still produce a
            // window that can explain itself and offer recovery.
            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("io.github.hex1mal.doyo"));
            if let Err(e) = std::fs::create_dir_all(&app_dir) {
                eprintln!("doyo: could not create app data dir: {e}");
            }

            // A legacy-import failure must not block startup; the fresh database
            // still works, and the old data is left untouched where it is.
            let legacy_error = migrate_legacy_doyo_data(&app_dir).err();
            if let Some(ref e) = legacy_error {
                eprintln!("doyo: could not import legacy data: {e}");
            }

            let db_path = app_dir.join(NEW_DB_NAME);
            let (db, mut startup) = initialize_database(&app_dir, &db_path);
            if let Some(e) = legacy_error {
                startup.detail = Some(match startup.detail {
                    Some(existing) => format!("{existing}; legacy import failed: {e}"),
                    None => format!("legacy import failed: {e}"),
                });
            }

            let node_service = NodeService::new(db.clone());
            app.manage(AppState {
                node_service: std::sync::Mutex::new(node_service),
                db,
                db_path,
                backup_dir: app_dir.join("backups"),
                migration_backup_dir: app_dir.join(MIGRATION_BACKUP_DIR_NAME),
                startup: std::sync::Mutex::new(startup),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            node_get,
            node_create,
            node_update,
            node_replace_properties,
            node_patch_properties,
            node_delete,
            trash_get_nodes,
            trash_restore,
            trash_empty,
            node_duplicate,
            node_move,
            node_move_ordered,
            node_reorder,
            node_reorder_root,
            tree_get_children,
            tree_get_ancestors,
            tree_get_full,
            node_set_due_date,
            node_set_priority,
            node_toggle_complete,
            node_set_completion,
            node_incomplete_descendant_count,
            search_query,
            quick_find,
            get_today_tasks,
            get_node_count,
            undo,
            redo,
            export_json,
            export_markdown,
            import_json,
            tag_list,
            tag_create,
            tag_rename,
            tag_delete,
            tag_assign,
            tag_remove,
            tag_get_for_node,
            tag_query_tasks,
            tag_sync_legacy,
            time_block_list,
            time_block_create,
            time_block_update,
            time_block_delete,
            focus_start,
            focus_get_active,
            focus_pause,
            focus_resume,
            focus_stop,
            focus_list,
            focus_summary,
            saved_filter_list,
            saved_filter_create,
            saved_filter_update,
            saved_filter_delete,
            habit_list,
            habit_create,
            habit_update,
            habit_archive,
            habit_delete,
            habit_log_upsert,
            habit_log_delete,
            habit_log_list,
            habit_summary,
            countdown_list,
            countdown_create,
            countdown_update,
            countdown_archive,
            countdown_delete,
            countdown_reorder,
            settings_get,
            settings_set,
            settings_delete,
            settings_list,
            backup_create,
            backup_list,
            backup_restore,
            startup_report,
            recovery_restore,
            recovery_candidates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
