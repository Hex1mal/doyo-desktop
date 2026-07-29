use doyo_core::attachment::AttachmentService;
use doyo_core::backup::BackupService;
use doyo_core::countdown::{
    CountdownMode, CountdownService, CreateCountdownInput, UpdateCountdownInput,
};
use doyo_core::db::run_migrations;
use doyo_core::db::Database;
use doyo_core::export::ExportService;
use doyo_core::focus::{
    FocusMethod, FocusService, FocusState, PomodoroPhase, StartFocusInput, StopFocusInput,
};
use doyo_core::habit::{
    CreateHabitInput, HabitFrequency, HabitLogStatus, HabitService, UpdateHabitInput,
    UpsertHabitLogInput,
};
use doyo_core::import::ImportService;
use doyo_core::node::model::*;
use doyo_core::node::service::NodeService;
use doyo_core::saved_filter::{CreateSavedFilterInput, SavedFilterService, UpdateSavedFilterInput};
use doyo_core::settings::SettingsRepository;
use doyo_core::tag::{TagRepository, TagService};
use doyo_core::time_block::{CreateTimeBlockInput, TimeBlockService, UpdateTimeBlockInput};
use std::sync::Arc;

fn setup_db() -> Arc<Database> {
    let db = Arc::new(Database::open_in_memory().expect("Failed to create in-memory DB"));
    run_migrations(&db).expect("Failed to run migrations");
    db
}

fn setup() -> NodeService {
    NodeService::new(setup_db())
}

fn create_node(
    service: &mut NodeService,
    parent_id: Option<String>,
    node_type: NodeType,
    title: &str,
) -> Node {
    service
        .create(CreateNodeInput {
            parent_id,
            node_type,
            title: title.into(),
            body: String::new(),
            properties: NodeProperties::default(),
            position: None,
        })
        .unwrap()
}

fn workspace(service: &mut NodeService, title: &str) -> Node {
    create_node(service, None, NodeType::Workspace, title)
}

fn group(service: &mut NodeService, parent_id: &str, title: &str) -> Node {
    create_node(service, Some(parent_id.into()), NodeType::Group, title)
}

fn task(service: &mut NodeService, parent_id: &str, title: &str) -> Node {
    create_node(service, Some(parent_id.into()), NodeType::Task, title)
}

fn task_with_properties(
    service: &mut NodeService,
    parent_id: &str,
    title: &str,
    properties: NodeProperties,
) -> Node {
    service
        .create(CreateNodeInput {
            parent_id: Some(parent_id.into()),
            node_type: NodeType::Task,
            title: title.into(),
            body: String::new(),
            properties,
            position: None,
        })
        .unwrap()
}

#[test]
fn test_create_workspace() {
    let mut service = setup();
    let ws = workspace(&mut service, "Personal");
    assert_eq!(ws.node_type, NodeType::Workspace);
    assert!(ws.parent_id.is_none());
}

#[test]
fn test_reject_invalid_root_group_and_task() {
    let mut service = setup();

    let root_group = service.create(CreateNodeInput {
        parent_id: None,
        node_type: NodeType::Group,
        title: "Invalid group".into(),
        body: String::new(),
        properties: NodeProperties::default(),
        position: None,
    });
    assert!(root_group.is_err());

    let root_task = service.create(CreateNodeInput {
        parent_id: None,
        node_type: NodeType::Task,
        title: "Invalid task".into(),
        body: String::new(),
        properties: NodeProperties::default(),
        position: None,
    });
    assert!(root_task.is_err());
}

#[test]
fn test_reject_invalid_child_types() {
    let mut service = setup();
    let ws = workspace(&mut service, "Polyglot");
    let task_node = task(&mut service, &ws.id, "Study");

    let group_under_task = service.create(CreateNodeInput {
        parent_id: Some(task_node.id.clone()),
        node_type: NodeType::Group,
        title: "Invalid subgroup".into(),
        body: String::new(),
        properties: NodeProperties::default(),
        position: None,
    });
    assert!(group_under_task.is_err());

    let nested_workspace = service.create(CreateNodeInput {
        parent_id: Some(ws.id.clone()),
        node_type: NodeType::Workspace,
        title: "Invalid workspace".into(),
        body: String::new(),
        properties: NodeProperties::default(),
        position: None,
    });
    assert!(nested_workspace.is_err());
}

#[test]
fn test_required_polyglot_hierarchy_and_persistence_queries() {
    let mut service = setup();

    let polyglot = workspace(&mut service, "Polyglot");
    let english = group(&mut service, &polyglot.id, "English");
    let grammar = group(&mut service, &english.id, "Grammar");
    let tenses = group(&mut service, &grammar.id, "Tenses");
    let study = task(&mut service, &tenses.id, "Study present perfect");
    let read = task(&mut service, &study.id, "Read lesson");
    let write = task(&mut service, &read.id, "Write examples");

    let full_tree = service.get_full_tree(None).unwrap();
    assert_eq!(full_tree.len(), 7);

    let ancestors = service.get_ancestors(&write.id).unwrap();
    let path: Vec<String> = ancestors
        .iter()
        .map(|node| node.title.clone())
        .chain(std::iter::once(write.title.clone()))
        .collect();
    assert_eq!(
        path,
        vec![
            "Polyglot",
            "English",
            "Grammar",
            "Tenses",
            "Study present perfect",
            "Read lesson",
            "Write examples",
        ]
    );

    assert_eq!(
        service.get_children(Some(&polyglot.id)).unwrap()[0].id,
        english.id
    );
    assert_eq!(
        service.get_children(Some(&english.id)).unwrap()[0].id,
        grammar.id
    );
    assert_eq!(
        service.get_children(Some(&grammar.id)).unwrap()[0].id,
        tenses.id
    );
    assert_eq!(
        service.get_children(Some(&tenses.id)).unwrap()[0].id,
        study.id
    );
    assert_eq!(
        service.get_children(Some(&study.id)).unwrap()[0].id,
        read.id
    );
}

#[test]
fn test_cycle_prevention() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let parent = group(&mut service, &ws.id, "Parent");
    let child = group(&mut service, &parent.id, "Child");

    let result = service.move_node(&parent.id, Some(&child.id), 0.0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        doyo_core::Error::CycleDetected
    ));
}

#[test]
fn test_soft_delete_cascades_to_children() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let parent = group(&mut service, &ws.id, "Parent");
    let child = task(&mut service, &parent.id, "Child");

    service.delete(&parent.id, false).unwrap();

    assert!(service.get(&parent.id).is_err());
    assert!(service.get(&child.id).is_err());
}

#[test]
fn test_trash_restore_permanent_delete_and_empty_trash() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let parent = group(&mut service, &ws.id, "Parent");
    let child_group = group(&mut service, &parent.id, "Child group");
    let child_task = task(&mut service, &child_group.id, "Child task");
    let subtask = task(&mut service, &child_task.id, "Subtask");

    service.delete(&parent.id, false).unwrap();
    let deleted = service.get_deleted_nodes().unwrap();
    let deleted_ids: Vec<String> = deleted.iter().map(|node| node.id.clone()).collect();
    assert!(deleted_ids.contains(&parent.id));
    assert!(deleted_ids.contains(&child_group.id));
    assert!(deleted_ids.contains(&child_task.id));
    assert!(deleted_ids.contains(&subtask.id));

    service.restore(&parent.id, None).unwrap();
    assert!(service.get(&parent.id).unwrap().deleted_at.is_none());
    assert!(service.get(&subtask.id).unwrap().deleted_at.is_none());

    service.delete(&parent.id, false).unwrap();
    service.delete(&parent.id, true).unwrap();
    assert!(service.get_deleted_nodes().unwrap().is_empty());
    assert!(service.get(&parent.id).is_err());

    let another = group(&mut service, &ws.id, "Another");
    let another_task = task(&mut service, &another.id, "Another task");
    service.delete(&another.id, false).unwrap();
    let removed = service.empty_trash().unwrap();
    assert_eq!(removed, 2);
    assert!(service.get(&another_task.id).is_err());
}

#[test]
fn test_restore_requires_active_parent_or_valid_destination() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let destination = group(&mut service, &ws.id, "Destination");
    let parent = group(&mut service, &ws.id, "Parent");
    let child = task(&mut service, &parent.id, "Child");

    service.delete(&parent.id, false).unwrap();
    assert!(service.restore(&child.id, None).is_err());

    let restored = service.restore(&child.id, Some(&destination.id)).unwrap();
    assert_eq!(restored.parent_id, Some(destination.id));
    assert!(service.get(&child.id).unwrap().deleted_at.is_none());
}

#[test]
fn test_normalized_tags_and_legacy_sync_are_non_destructive() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let tag_service = TagService::new(TagRepository::new(db.clone()));
    let ws = workspace(&mut service, "Work");
    let properties = NodeProperties {
        custom: Some(serde_json::json!({ "tags": [" Study ", "English"] })),
        ..Default::default()
    };
    let tagged = task_with_properties(&mut service, &ws.id, "Tagged task", properties);

    let synced = tag_service.sync_legacy_custom_tags().unwrap();
    assert_eq!(synced, 2);
    let tags = tag_service.get_tags_for_node(&tagged.id).unwrap();
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|tag| tag.name == "Study"));

    let duplicate = tag_service.create_tag(" study ", None);
    assert!(duplicate.is_err());

    let important = tag_service
        .create_tag("Important", Some("#EF4444"))
        .unwrap();
    tag_service.assign_tag(&tagged.id, &important.id).unwrap();
    let renamed = tag_service
        .rename_tag(&important.id, "Important Work", Some("#F59E0B"))
        .unwrap();
    assert_eq!(renamed.name, "Important Work");
    assert_eq!(renamed.color.as_deref(), Some("#F59E0B"));

    let tasks = tag_service.query_tasks_by_tag(&renamed.id).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, tagged.id);

    let still_legacy = service.get(&tagged.id).unwrap();
    assert_eq!(
        still_legacy
            .properties
            .custom
            .as_ref()
            .and_then(|custom| custom.get("tags"))
            .and_then(|tags| tags.as_array())
            .map(|tags| tags.len()),
        Some(2)
    );
}

#[test]
fn test_time_block_crud_validation_and_linked_task_delete_behavior() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let blocks = TimeBlockService::new(db.clone());
    let ws = workspace(&mut service, "Calendar");
    let linked = task(&mut service, &ws.id, "Timed task");
    let start = chrono::Utc::now();
    let end = start + chrono::Duration::hours(1);

    let block = blocks
        .create(CreateTimeBlockInput {
            task_id: Some(linked.id.clone()),
            title: "Plan writing".into(),
            start_time: start,
            end_time: end,
            all_day: false,
            notes: "Draft".into(),
        })
        .unwrap();
    assert_eq!(block.task_id, Some(linked.id.clone()));

    let listed = blocks
        .list_between(
            start - chrono::Duration::hours(1),
            end + chrono::Duration::hours(1),
        )
        .unwrap();
    assert_eq!(listed.len(), 1);

    let moved = blocks
        .update(
            &block.id,
            UpdateTimeBlockInput {
                start_time: Some(start + chrono::Duration::days(1)),
                end_time: Some(end + chrono::Duration::days(1)),
                notes: Some("Moved".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(moved.notes, "Moved");

    let invalid = blocks.create(CreateTimeBlockInput {
        task_id: Some(linked.id.clone()),
        title: "Invalid".into(),
        start_time: end,
        end_time: start,
        all_day: false,
        notes: String::new(),
    });
    assert!(invalid.is_err());

    let bad_link = blocks.create(CreateTimeBlockInput {
        task_id: Some(ws.id.clone()),
        title: "Bad link".into(),
        start_time: start,
        end_time: end,
        all_day: false,
        notes: String::new(),
    });
    assert!(bad_link.is_err());

    service.delete(&linked.id, true).unwrap();
    let unlinked = blocks.get(&block.id).unwrap();
    assert!(unlinked.task_id.is_none());

    blocks.delete(&block.id).unwrap();
    assert!(blocks.get(&block.id).is_err());
}

#[test]
fn test_focus_pomodoro_cycle_duplicate_rejection_and_summary() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let focus = FocusService::new(db.clone());
    let ws = workspace(&mut service, "Focus");
    let linked = task(&mut service, &ws.id, "Write draft");

    let started = focus
        .start(StartFocusInput {
            method: FocusMethod::Pomodoro,
            task_id: Some(linked.id.clone()),
            planned_seconds: 1,
            pomodoro_phase: Some(PomodoroPhase::Focus),
            pomodoro_cycle: 1,
            note: "Start".into(),
        })
        .unwrap();
    assert_eq!(started.state, FocusState::Running);
    assert_eq!(started.task_id, Some(linked.id.clone()));

    let duplicate = focus.start(StartFocusInput {
        method: FocusMethod::Pomodoro,
        task_id: None,
        planned_seconds: 1,
        pomodoro_phase: Some(PomodoroPhase::ShortBreak),
        pomodoro_cycle: 1,
        note: String::new(),
    });
    assert!(duplicate.is_err());

    let completed = focus
        .stop(
            &started.id,
            StopFocusInput {
                completed: true,
                note: Some("Done".into()),
            },
        )
        .unwrap();
    assert_eq!(completed.state, FocusState::Completed);
    assert_eq!(completed.note, "Done");
    assert!(focus.get_active().unwrap().is_none());

    let summary = focus.summary().unwrap();
    assert_eq!(summary.pomodoro_count, 1);
    assert!(summary.total_seconds >= 0);
}

#[test]
fn test_focus_stopwatch_pause_resume_and_task_delete_preserves_history() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let focus = FocusService::new(db.clone());
    let ws = workspace(&mut service, "Focus");
    let linked = task(&mut service, &ws.id, "Timed task");

    let started = focus
        .start(StartFocusInput {
            method: FocusMethod::Stopwatch,
            task_id: Some(linked.id.clone()),
            planned_seconds: 0,
            pomodoro_phase: None,
            pomodoro_cycle: 1,
            note: "Stopwatch".into(),
        })
        .unwrap();
    let paused = focus.pause(&started.id).unwrap();
    assert_eq!(paused.state, FocusState::Paused);
    assert_eq!(paused.interruptions, 1);

    let resumed = focus.resume(&started.id).unwrap();
    assert_eq!(resumed.state, FocusState::Running);

    let stopped = focus
        .stop(
            &started.id,
            StopFocusInput {
                completed: true,
                note: None,
            },
        )
        .unwrap();
    assert_eq!(stopped.state, FocusState::Completed);
    assert_eq!(stopped.task_title, "Timed task");

    service.delete(&linked.id, true).unwrap();
    let historical = focus.get(&started.id).unwrap();
    assert!(historical.task_id.is_none());
    assert_eq!(historical.task_title, "Timed task");
}

#[test]
fn test_focus_flowtime_is_tracked_separately_from_stopwatch() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let focus = FocusService::new(db.clone());
    let ws = workspace(&mut service, "Focus");
    let linked = task(&mut service, &ws.id, "Deep work");

    let started = focus
        .start(StartFocusInput {
            method: FocusMethod::Flowtime,
            task_id: Some(linked.id.clone()),
            planned_seconds: 0,
            pomodoro_phase: None,
            pomodoro_cycle: 1,
            note: "Flexible session".into(),
        })
        .unwrap();
    assert_eq!(started.method, FocusMethod::Flowtime);

    let completed = focus
        .stop(
            &started.id,
            StopFocusInput {
                completed: true,
                note: None,
            },
        )
        .unwrap();
    assert_eq!(completed.method, FocusMethod::Flowtime);

    let summary = focus.summary().unwrap();
    assert!(summary.flowtime_seconds >= 0);
    assert_eq!(summary.stopwatch_seconds, 0);
}

#[test]
fn test_saved_filters_are_validated_and_persisted() {
    let db = setup_db();
    let service = SavedFilterService::new(db.clone());

    let created = service
        .create(CreateSavedFilterInput {
            name: "High priority study".into(),
            definition: serde_json::json!({
                "completion": "active",
                "priority": 1,
                "tagIds": ["tag-study"]
            }),
        })
        .unwrap();
    assert_eq!(created.name, "High priority study");
    assert_eq!(service.list().unwrap().len(), 1);

    let updated = service
        .update(
            &created.id,
            UpdateSavedFilterInput {
                name: Some("Study P1".into()),
                definition: Some(serde_json::json!({ "priority": 1 })),
                position: Some(10.0),
            },
        )
        .unwrap();
    assert_eq!(updated.name, "Study P1");
    assert_eq!(updated.definition["priority"], 1);

    assert!(service
        .create(CreateSavedFilterInput {
            name: " ".into(),
            definition: serde_json::json!({}),
        })
        .is_err());
    assert!(service
        .create(CreateSavedFilterInput {
            name: "Bad".into(),
            definition: serde_json::json!(["not", "object"]),
        })
        .is_err());

    service.delete(&created.id).unwrap();
    assert!(service.list().unwrap().is_empty());
}

#[test]
fn test_habits_logs_archive_delete_and_summary() {
    let db = setup_db();
    let habits = HabitService::new(db.clone());
    let start = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();

    let habit = habits
        .create(CreateHabitInput {
            title: "Read".into(),
            icon: "book".into(),
            color: Some("#2563EB".into()),
            frequency: HabitFrequency::Daily,
            days: vec![],
            goal: 1.0,
            goal_unit: "session".into(),
            start_date: start,
            reminder_time: Some("08:00".into()),
        })
        .unwrap();

    let log1 = habits
        .upsert_log(UpsertHabitLogInput {
            habit_id: habit.id.clone(),
            log_date: start,
            status: HabitLogStatus::Completed,
            value: 1.0,
            note: String::new(),
        })
        .unwrap();
    let log2 = habits
        .upsert_log(UpsertHabitLogInput {
            habit_id: habit.id.clone(),
            log_date: start + chrono::Duration::days(1),
            status: HabitLogStatus::Completed,
            value: 1.0,
            note: "Done".into(),
        })
        .unwrap();
    assert_ne!(log1.id, log2.id);

    let summary = habits
        .summary(start, start + chrono::Duration::days(1))
        .unwrap();
    assert_eq!(summary.active_count, 1);
    assert_eq!(summary.best_streak, 2);
    assert!(summary.completion_rate > 0.9);

    let updated = habits
        .update(
            &habit.id,
            UpdateHabitInput {
                title: Some("Read deeply".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.title, "Read deeply");

    habits.archive(&habit.id, true).unwrap();
    assert!(habits.list(false).unwrap().is_empty());
    assert_eq!(habits.list(true).unwrap().len(), 1);

    habits.delete(&habit.id).unwrap();
    assert!(habits.list(true).unwrap().is_empty());
    assert!(habits
        .list_logs(start, start + chrono::Duration::days(2))
        .unwrap()
        .is_empty());
}

#[test]
fn test_countdowns_create_update_reorder_archive_delete() {
    let db = setup_db();
    let countdowns = CountdownService::new(db.clone());
    let now = chrono::Utc::now();
    let first = countdowns
        .create(CreateCountdownInput {
            title: "Launch".into(),
            target_date: now + chrono::Duration::days(30),
            mode: CountdownMode::Countdown,
            icon: "rocket".into(),
            color: Some("#10B981".into()),
            recurrence: None,
            reminder_at: None,
        })
        .unwrap();
    let second = countdowns
        .create(CreateCountdownInput {
            title: "Since start".into(),
            target_date: now - chrono::Duration::days(1),
            mode: CountdownMode::Countup,
            icon: "flag".into(),
            color: None,
            recurrence: Some("yearly".into()),
            reminder_at: None,
        })
        .unwrap();

    let updated = countdowns
        .update(
            &first.id,
            UpdateCountdownInput {
                title: Some("Launch day".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.title, "Launch day");

    let reordered = countdowns
        .reorder(&[second.id.clone(), first.id.clone()])
        .unwrap();
    assert_eq!(reordered[0].id, second.id);

    countdowns.archive(&second.id, true).unwrap();
    assert_eq!(countdowns.list(false).unwrap().len(), 1);
    countdowns.delete(&second.id).unwrap();
    assert_eq!(countdowns.list(true).unwrap().len(), 1);
}

#[test]
fn test_settings_repository_lists_prefixed_values() {
    let db = setup_db();
    let settings = SettingsRepository::new(db.clone());
    settings
        .set("ui.theme", &serde_json::json!("dark"))
        .unwrap();
    settings
        .set(
            "ui.preferences",
            &serde_json::json!({ "sidebarWidth": 320 }),
        )
        .unwrap();
    settings
        .set("other.value", &serde_json::json!(true))
        .unwrap();

    let ui_values = settings.list(Some("ui.")).unwrap();
    assert_eq!(ui_values.len(), 2);
    assert!(ui_values.iter().any(|(key, _)| key == "ui.theme"));
    assert_eq!(
        settings.get::<serde_json::Value>("ui.theme").unwrap(),
        Some(serde_json::json!("dark"))
    );

    settings.delete("ui.theme").unwrap();
    assert!(settings
        .get::<serde_json::Value>("ui.theme")
        .unwrap()
        .is_none());
}

#[test]
fn test_backup_service_create_prune_and_restore() {
    let root = std::env::temp_dir().join(format!("doyo-backup-test-{}", uuid::Uuid::now_v7()));
    let backup_dir = root.join("backups");
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("doyo.db");
    std::fs::write(&db_path, b"original").unwrap();

    let backups = BackupService::new(db_path.clone(), backup_dir.clone(), 2);
    let first_path = backups.create_backup().unwrap();
    assert!(first_path.exists());

    std::fs::write(&db_path, b"changed").unwrap();
    let second_path = backups.create_backup().unwrap();
    assert!(second_path.exists());

    std::fs::write(&db_path, b"changed again").unwrap();
    backups
        .restore_backup(&first_path.file_name().unwrap().to_string_lossy())
        .unwrap();
    assert_eq!(std::fs::read(&db_path).unwrap(), b"original");

    std::fs::write(&db_path, b"third").unwrap();
    backups.create_backup().unwrap();
    assert!(backups.list_backups().unwrap().len() <= 2);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_backup_restore_rejects_path_traversal() {
    let root = std::env::temp_dir().join(format!("doyo-backup-traversal-{}", uuid::Uuid::now_v7()));
    let backup_dir = root.join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let db_path = root.join("doyo.db");
    std::fs::write(&db_path, b"original").unwrap();
    std::fs::write(backup_dir.join("safe.db"), b"backup").unwrap();

    let backups = BackupService::new(db_path.clone(), backup_dir, 10);
    assert!(backups.restore_backup("../safe.db").is_err());
    assert!(backups.restore_backup("/tmp/safe.db").is_err());
    assert!(backups.restore_backup("nested/safe.db").is_err());
    backups.restore_backup("safe.db").unwrap();
    assert_eq!(std::fs::read(&db_path).unwrap(), b"backup");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_attachment_service_rejects_path_traversal() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let ws = workspace(&mut service, "Files");
    let attachment_root =
        std::env::temp_dir().join(format!("doyo-attachment-test-{}", uuid::Uuid::now_v7()));
    let attachments = AttachmentService::new(db, attachment_root.clone());

    assert!(attachments
        .add_attachment(&ws.id, "../escape.txt", "text/plain", b"no")
        .is_err());
    assert!(attachments
        .add_attachment("../node", "safe.txt", "text/plain", b"no")
        .is_err());

    let added = attachments
        .add_attachment(&ws.id, "safe.txt", "text/plain", b"ok")
        .unwrap();
    assert!(std::path::Path::new(&added.file_path).exists());
    attachments.delete_attachment(&added.id).unwrap();
    assert!(!std::path::Path::new(&added.file_path).exists());

    std::fs::remove_dir_all(attachment_root).unwrap();
}

#[test]
fn test_json_export_import_nested_hierarchy_round_trip() {
    let source_db = setup_db();
    let mut source = NodeService::new(source_db.clone());
    let ws = workspace(&mut source, "Polyglot");
    let english = group(&mut source, &ws.id, "English");
    let grammar = group(&mut source, &english.id, "Grammar");
    let tenses = group(&mut source, &grammar.id, "Tenses");
    let task_node = task(&mut source, &tenses.id, "Study present perfect");
    task(&mut source, &task_node.id, "Write examples");
    source.set_completion(&task_node.id, true, false).unwrap();

    let tags = TagService::new(TagRepository::new(source_db.clone()));
    let study = tags.create_tag("Study", Some("#22c55e")).unwrap();
    tags.assign_tag(&task_node.id, &study.id).unwrap();

    let time_blocks = TimeBlockService::new(source_db.clone());
    time_blocks
        .create(CreateTimeBlockInput {
            task_id: Some(task_node.id.clone()),
            title: "Practice".into(),
            start_time: chrono::DateTime::parse_from_rfc3339("2026-07-29T08:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            end_time: chrono::DateTime::parse_from_rfc3339("2026-07-29T09:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            all_day: false,
            notes: "Imported with task".into(),
        })
        .unwrap();

    let exported = ExportService::new(source_db).export_json(None).unwrap();
    let document: serde_json::Value = serde_json::from_str(&exported).unwrap();
    assert_eq!(document["format"], "io.github.hex1mal.doyo.transfer");
    assert_eq!(document["version"], 1);

    let target_db = setup_db();
    let imported = ImportService::new(target_db.clone())
        .import_json(&exported, None)
        .unwrap();
    assert_eq!(imported.len(), 6);

    let target = NodeService::new(target_db.clone());
    let roots = target.get_children(None).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].title, "Polyglot");
    let imported_tree = target.get_full_tree(None).unwrap();
    let titles = imported_tree
        .iter()
        .map(|node| node.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        titles,
        vec![
            "Polyglot",
            "English",
            "Grammar",
            "Tenses",
            "Study present perfect",
            "Write examples"
        ]
    );
    assert!(imported_tree
        .iter()
        .any(|node| node.title == "Study present perfect" && node.is_completed));

    let target_tags = TagService::new(TagRepository::new(target_db.clone()));
    let tag_names = target_tags
        .get_tag_names_for_node(
            &imported_tree
                .iter()
                .find(|node| node.title == "Study present perfect")
                .unwrap()
                .id,
        )
        .unwrap();
    assert_eq!(tag_names, vec!["Study"]);

    let target_blocks = TimeBlockService::new(target_db)
        .list_between(
            chrono::DateTime::parse_from_rfc3339("2026-07-29T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            chrono::DateTime::parse_from_rfc3339("2026-07-30T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        )
        .unwrap();
    assert_eq!(target_blocks.len(), 1);
    assert_eq!(target_blocks[0].title, "Practice");

    assert!(ImportService::new(setup_db())
        .import_json("{\"format\":\"bad\"}", None)
        .is_err());
}

#[test]
fn test_markdown_export_preserves_duplicate_and_unsafe_titles() {
    let db = setup_db();
    let mut service = NodeService::new(db.clone());
    let ws = workspace(&mut service, "Export");
    task(&mut service, &ws.id, "Read/Notes");
    task(&mut service, &ws.id, "Read/Notes");

    let output_dir =
        std::env::temp_dir().join(format!("doyo-markdown-export-{}", uuid::Uuid::now_v7()));
    ExportService::new(db)
        .export_markdown(None, &output_dir)
        .unwrap();
    let files = std::fs::read_dir(&output_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let markdown_files = files
        .iter()
        .filter(|name| name.ends_with(".md"))
        .collect::<Vec<_>>();
    assert!(!markdown_files.is_empty());
    assert!(files
        .iter()
        .all(|name| !name.contains('/') && !name.contains('\\')));

    let mut all_markdown = Vec::new();
    collect_markdown_files(&output_dir, &mut all_markdown);
    let duplicate_exports = all_markdown
        .iter()
        .filter(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().contains("Read_Notes"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    assert_eq!(duplicate_exports.len(), 2);
    assert_ne!(duplicate_exports[0], duplicate_exports[1]);

    std::fs::remove_dir_all(output_dir).unwrap();
}

fn collect_markdown_files(dir: &std::path::Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_markdown_files(&path, output);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            output.push(path);
        }
    }
}

#[test]
fn test_update_toggle_priority_due_duplicate_and_undo() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let node = task(&mut service, &ws.id, "Original");

    let updated = service
        .update(
            &node.id,
            UpdateNodeInput {
                title: Some("Updated title".into()),
                body: Some("# Hello\n\nWorld".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.title, "Updated title");
    assert_eq!(updated.body, "# Hello\n\nWorld");

    let completed = service.toggle_complete(&node.id).unwrap();
    assert!(completed.is_completed);
    let uncompleted = service.toggle_complete(&node.id).unwrap();
    assert!(!uncompleted.is_completed);

    let priority = service.set_priority(&node.id, 1).unwrap();
    assert_eq!(priority.properties.priority, Some(1));

    let future = chrono::Utc::now() + chrono::Duration::days(7);
    let dated = service.set_due_date(&node.id, Some(future)).unwrap();
    assert!(dated.properties.due_date.is_some());

    let dup = service.duplicate(&node.id).unwrap();
    assert!(dup.title.contains("Copy"));
    assert_ne!(dup.id, node.id);

    assert!(service.can_undo());
    service.undo().unwrap();
}

#[test]
fn test_move_and_tree_queries() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let group1 = group(&mut service, &ws.id, "Group 1");
    let group2 = group(&mut service, &ws.id, "Group 2");
    let movable = task(&mut service, &group1.id, "Movable");

    service
        .move_node(&movable.id, Some(&group2.id), 500.0)
        .unwrap();

    let children = service.get_children(Some(&group2.id)).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, movable.id);

    let descendants = service.get_descendants(&ws.id).unwrap();
    assert_eq!(descendants.len(), 3);
}

#[test]
fn test_valid_contextual_moves_preserve_descendants() {
    let mut service = setup();
    let ws1 = workspace(&mut service, "Polyglot");
    let ws2 = workspace(&mut service, "Archive");
    let english = group(&mut service, &ws1.id, "English");
    let grammar = group(&mut service, &english.id, "Grammar");
    let tenses = group(&mut service, &grammar.id, "Tenses");
    let study = task(&mut service, &tenses.id, "Study present perfect");
    let read = task(&mut service, &study.id, "Read lesson");
    let holder = group(&mut service, &ws1.id, "Language Notes");
    let other_task = task(&mut service, &ws2.id, "Parent task");

    service
        .move_node(&english.id, Some(&ws2.id), 100.0)
        .unwrap();
    assert_eq!(
        service.get(&english.id).unwrap().parent_id,
        Some(ws2.id.clone())
    );
    assert_eq!(
        service.get(&grammar.id).unwrap().parent_id,
        Some(english.id.clone())
    );

    service
        .move_node(&english.id, Some(&ws1.id), 200.0)
        .unwrap();
    assert_eq!(
        service.get(&english.id).unwrap().parent_id,
        Some(ws1.id.clone())
    );

    service
        .move_node(&grammar.id, Some(&holder.id), 300.0)
        .unwrap();
    assert_eq!(
        service.get(&grammar.id).unwrap().parent_id,
        Some(holder.id.clone())
    );
    assert_eq!(
        service.get(&tenses.id).unwrap().parent_id,
        Some(grammar.id.clone())
    );

    service
        .move_node(&study.id, Some(&other_task.id), 400.0)
        .unwrap();
    assert_eq!(
        service.get(&study.id).unwrap().parent_id,
        Some(other_task.id.clone())
    );
    assert_eq!(
        service.get(&read.id).unwrap().parent_id,
        Some(study.id.clone())
    );

    service
        .move_node(&study.id, Some(&tenses.id), 500.0)
        .unwrap();
    assert_eq!(
        service.get(&study.id).unwrap().parent_id,
        Some(tenses.id.clone())
    );
}

#[test]
fn test_invalid_moves_and_cycle_prevention_are_authoritative() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let other_ws = workspace(&mut service, "Other");
    let group_node = group(&mut service, &ws.id, "Group");
    let child_group = group(&mut service, &group_node.id, "Child group");
    let task_node = task(&mut service, &group_node.id, "Task");
    let subtask_node = task(&mut service, &task_node.id, "Subtask");

    assert!(service.move_node(&other_ws.id, Some(&ws.id), 0.0).is_err());
    assert!(service
        .move_node(&group_node.id, Some(&task_node.id), 0.0)
        .is_err());
    assert!(service
        .move_node(&group_node.id, Some(&child_group.id), 0.0)
        .is_err());
    assert!(service
        .move_node(&task_node.id, Some(&subtask_node.id), 0.0)
        .is_err());
}

#[test]
fn test_recursive_completion_individual_and_cascade() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let root = task(&mut service, &ws.id, "Root task");
    let mut ids = vec![root.id.clone()];
    let mut parent_id = root.id.clone();

    for i in 1..=6 {
        let child = task(&mut service, &parent_id, &format!("Subtask {}", i));
        parent_id = child.id.clone();
        ids.push(child.id);
    }

    assert_eq!(
        service.incomplete_task_descendant_count(&root.id).unwrap(),
        6
    );

    let completed_root = service.set_completion(&root.id, true, false).unwrap();
    assert!(completed_root.is_completed);
    assert!(completed_root.completed_at.is_some());
    assert_eq!(
        service.incomplete_task_descendant_count(&root.id).unwrap(),
        6
    );
    for id in ids.iter().skip(1) {
        let child = service.get(id).unwrap();
        assert!(!child.is_completed);
        assert!(child.completed_at.is_none());
    }

    let reopened_root = service.set_completion(&root.id, false, false).unwrap();
    assert!(!reopened_root.is_completed);
    assert!(reopened_root.completed_at.is_none());

    service.set_completion(&root.id, true, true).unwrap();
    for id in &ids {
        let node = service.get(id).unwrap();
        assert!(node.is_completed, "{} should be completed", node.title);
        assert!(
            node.completed_at.is_some(),
            "{} should have completed_at",
            node.title
        );
    }

    let reopened_again = service.set_completion(&root.id, false, false).unwrap();
    assert!(!reopened_again.is_completed);
    for id in ids.iter().skip(1) {
        assert!(service.get(id).unwrap().is_completed);
    }
}

#[test]
fn test_deep_recursive_task_nesting() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    let mut parent_id = ws.id;

    for i in 0..50 {
        let node = task(&mut service, &parent_id, &format!("Level {}", i));
        parent_id = node.id;
    }

    let ancestors = service.get_ancestors(&parent_id).unwrap();
    assert_eq!(ancestors.len(), 50);
}

#[test]
fn test_search_quick_find_and_count() {
    let mut service = setup();
    let ws = workspace(&mut service, "Work");
    task(&mut service, &ws.id, "Fix authentication bug");
    task(&mut service, &ws.id, "Write documentation");

    let results = service
        .search("authentication", SearchFilters::default())
        .unwrap();
    assert!(!results.is_empty());

    let quick = service.quick_find("auth").unwrap();
    assert!(!quick.is_empty());

    let count = service.get_node_count().unwrap();
    assert_eq!(count, 3);
}
