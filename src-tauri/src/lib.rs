use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;
use doyo_core::backup::BackupService;
use doyo_core::countdown::{Countdown, CountdownService, CreateCountdownInput, UpdateCountdownInput};
use doyo_core::db::Database;
use doyo_core::habit::{CreateHabitInput, Habit, HabitLog, HabitService, HabitSummary, UpdateHabitInput, UpsertHabitLogInput};
use doyo_core::node::model::*;
use doyo_core::node::service::NodeService;
use doyo_core::focus::{FocusService, FocusSession, FocusSummary, StartFocusInput, StopFocusInput};
use doyo_core::saved_filter::{CreateSavedFilterInput, SavedFilter, SavedFilterService, UpdateSavedFilterInput};
use doyo_core::settings::SettingsRepository;
use doyo_core::tag::{Tag, TagRepository, TagService};
use doyo_core::time_block::{CreateTimeBlockInput, TimeBlock, TimeBlockService, UpdateTimeBlockInput};

pub struct AppState {
    pub node_service: std::sync::Mutex<NodeService>,
    pub db: Arc<Database>,
    pub db_path: std::path::PathBuf,
    pub backup_dir: std::path::PathBuf,
}

const OLD_APP_DIR_NAME: &str = "com.todoapp.desktop";
const OLD_DB_NAME: &str = "todoapp.db";
const NEW_DB_NAME: &str = "doyo.db";

fn validate_sqlite_database(path: &Path) -> Result<i64, String> {
    let db = Database::open(path).map_err(|e| format!("failed to open database {}: {e}", path.display()))?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("failed integrity check for {}: {e}", path.display()))?;
    if integrity != "ok" {
        return Err(format!("database integrity check failed for {}: {integrity}", path.display()));
    }
    conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0))
        .map_err(|e| format!("failed to read schema_version for {}: {e}", path.display()))
}

fn copy_file_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|e| format!("failed to copy {} to {}: {e}", source.display(), destination.display()))
}

fn copy_dir_contents_if_missing(source_dir: &Path, destination_dir: &Path) -> Result<(), String> {
    if !source_dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(destination_dir).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(source_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with(OLD_DB_NAME) {
            continue;
        }
        let destination_path = destination_dir.join(file_name);
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            copy_dir_contents_if_missing(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            copy_file_if_missing(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn create_migration_safety_backup(source_db: &Path, new_app_dir: &Path) -> Result<PathBuf, String> {
    let backup_dir = new_app_dir.join("migration-backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
    let backup_path = backup_dir.join(format!("pre-doyo-migration-{stamp}-{}.db", uuid::Uuid::now_v7()));
    std::fs::copy(source_db, &backup_path)
        .map(|_| backup_path)
        .map_err(|e| format!("failed to create migration safety backup from {}: {e}", source_db.display()))
}

fn migrate_legacy_todoapp_data(new_app_dir: &Path) -> Result<(), String> {
    let new_db_path = new_app_dir.join(NEW_DB_NAME);
    if new_db_path.exists() {
        validate_sqlite_database(&new_db_path)?;
        return Ok(());
    }

    let Some(data_parent) = new_app_dir.parent() else {
        return Ok(());
    };
    let old_app_dir = data_parent.join(OLD_APP_DIR_NAME);
    let old_db_path = old_app_dir.join(OLD_DB_NAME);
    if !old_db_path.exists() {
        return Ok(());
    }

    let old_schema_version = validate_sqlite_database(&old_db_path)?;
    std::fs::create_dir_all(new_app_dir).map_err(|e| e.to_string())?;
    create_migration_safety_backup(&old_db_path, new_app_dir)?;

    copy_dir_contents_if_missing(&old_app_dir.join("backups"), &new_app_dir.join("backups"))?;
    copy_dir_contents_if_missing(&old_app_dir.join("localstorage"), &new_app_dir.join("localstorage"))?;
    copy_file_if_missing(&old_db_path, &new_db_path)?;

    for suffix in ["-wal", "-shm"] {
        let old_sidecar = old_app_dir.join(format!("{OLD_DB_NAME}{suffix}"));
        if old_sidecar.exists() {
            copy_file_if_missing(&old_sidecar, &new_app_dir.join(format!("{NEW_DB_NAME}{suffix}")))?;
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
    let nt = NodeType::from_str(&node_type).unwrap_or(NodeType::Task);
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
fn node_update(state: tauri::State<AppState>, id: String, changes: UpdateNodeInput) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.update(&id, changes).map_err(|e| e.to_string())
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
fn node_move(state: tauri::State<AppState>, id: String, new_parent_id: Option<String>, position: f64) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.move_node(&id, new_parent_id.as_deref(), position).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_reorder(state: tauri::State<AppState>, parent_id: String, child_ids: Vec<String>) -> Result<(), String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.reorder_children(&parent_id, &child_ids).map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_children(state: tauri::State<AppState>, parent_id: Option<String>) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_children(parent_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_ancestors(state: tauri::State<AppState>, id: String) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_ancestors(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn tree_get_full(state: tauri::State<AppState>, root_id: Option<String>) -> Result<Vec<Node>, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.get_full_tree(root_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_due_date(state: tauri::State<AppState>, id: String, due_date: Option<String>) -> Result<Node, String> {
    let dt = due_date.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&chrono::Utc)));
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.set_due_date(&id, dt).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_priority(state: tauri::State<AppState>, id: String, priority: i32) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.set_priority(&id, priority).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_toggle_complete(state: tauri::State<AppState>, id: String) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.toggle_complete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_set_completion(state: tauri::State<AppState>, id: String, completed: bool, cascade: bool) -> Result<Node, String> {
    let mut service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.set_completion(&id, completed, cascade).map_err(|e| e.to_string())
}

#[tauri::command]
fn node_incomplete_descendant_count(state: tauri::State<AppState>, id: String) -> Result<u32, String> {
    let service = state.node_service.lock().map_err(|e| e.to_string())?;
    service.incomplete_task_descendant_count(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_query(state: tauri::State<AppState>, query: String, filters: SearchFilters) -> Result<Vec<SearchResult>, String> {
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
    export_svc.export_json(root_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_markdown(state: tauri::State<AppState>, root_id: Option<String>, output_dir: String) -> Result<(), String> {
    use doyo_core::export::ExportService;
    let export_svc = ExportService::new(state.db.clone());
    export_svc.export_markdown(root_id.as_deref(), std::path::Path::new(&output_dir)).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_json(state: tauri::State<AppState>, json: String, parent_id: Option<String>) -> Result<Vec<String>, String> {
    use doyo_core::import::ImportService;
    let import_svc = ImportService::new(state.db.clone());
    import_svc.import_json(&json, parent_id.as_deref()).map_err(|e| e.to_string())
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
    tag_service(&state).delete_tag(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_assign(state: tauri::State<AppState>, node_id: String, tag_id: String) -> Result<(), String> {
    tag_service(&state)
        .assign_tag(&node_id, &tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn tag_remove(state: tauri::State<AppState>, node_id: String, tag_id: String) -> Result<(), String> {
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
    focus_service(&state).start(input).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_get_active(state: tauri::State<AppState>) -> Result<Option<FocusSession>, String> {
    focus_service(&state).get_active().map_err(|e| e.to_string())
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
    focus_service(&state).stop(&id, input).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_list(state: tauri::State<AppState>, limit: i64) -> Result<Vec<FocusSession>, String> {
    focus_service(&state).list_recent(limit).map_err(|e| e.to_string())
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
    saved_filter_service(&state).list().map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_filter_create(
    state: tauri::State<AppState>,
    input: CreateSavedFilterInput,
) -> Result<SavedFilter, String> {
    saved_filter_service(&state).create(input).map_err(|e| e.to_string())
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
    saved_filter_service(&state).delete(&id).map_err(|e| e.to_string())
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
    habit_service(&state).create(input).map_err(|e| e.to_string())
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
fn habit_archive(state: tauri::State<AppState>, id: String, archived: bool) -> Result<Habit, String> {
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
    let log_date = chrono::NaiveDate::parse_from_str(&log_date, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
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
    countdown_service(&state).delete(&id).map_err(|e| e.to_string())
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
    settings_repo(&state).delete(&key).map_err(|e| e.to_string())
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

fn checkpoint_database(state: &tauri::State<AppState>) -> Result<(), String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|e| e.to_string())
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
    backup_service(&state).list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
fn backup_restore(state: tauri::State<AppState>, backup_name: String) -> Result<(), String> {
    checkpoint_database(&state)?;
    backup_service(&state)
        .restore_backup(&backup_name)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");
            migrate_legacy_todoapp_data(&app_dir).expect("failed to migrate legacy TodoApp data");
            let db_path = app_dir.join(NEW_DB_NAME);
            let backup_dir = app_dir.join("backups");
            let db = Arc::new(Database::open(&db_path).expect("failed to open database"));
            doyo_core::db::run_migrations(&db).expect("failed to run migrations");
            let node_service = NodeService::new(db.clone());
            app.manage(AppState {
                node_service: std::sync::Mutex::new(node_service),
                db,
                db_path,
                backup_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            node_get,
            node_create,
            node_update,
            node_delete,
            trash_get_nodes,
            trash_restore,
            trash_empty,
            node_duplicate,
            node_move,
            node_reorder,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
