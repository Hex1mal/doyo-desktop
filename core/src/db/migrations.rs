use super::Database;

/// Highest schema version this build knows how to produce and read.
/// Restore refuses databases newer than this rather than silently mangling them.
pub const LATEST_SCHEMA_VERSION: i32 = 6;

pub fn run_migrations(db: &Database) -> crate::error::Result<()> {
    let mut conn = db.conn.lock().unwrap();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            description TEXT
        );",
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for migration in MIGRATIONS {
        if current_version >= migration.version {
            continue;
        }
        // Each migration is all-or-nothing. Without this a failure part-way
        // through leaves a half-applied schema that no later run can recover
        // from, because the non-idempotent statements would be replayed.
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_version (version, description) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.description],
        )?;
        tx.commit()?;
    }

    Ok(())
}

struct Migration {
    version: i32,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: [Migration; LATEST_SCHEMA_VERSION as usize] = [
    Migration {
        version: 1,
        description: "Initial schema",
        sql: include_str!("migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        description: "Add calendar time blocks",
        sql: include_str!("migrations/002_time_blocks.sql"),
    },
    Migration {
        version: 3,
        description: "Add focus sessions",
        sql: include_str!("migrations/003_focus_sessions.sql"),
    },
    Migration {
        version: 4,
        description: "Add saved filters, habits, habit logs, and countdowns",
        sql: include_str!("migrations/004_productivity_entities.sql"),
    },
    Migration {
        version: 5,
        description: "Add habit weekly days column",
        sql: include_str!("migrations/005_habit_days.sql"),
    },
    Migration {
        version: 6,
        description: "Add focus workflow metadata",
        sql: include_str!("migrations/006_focus_workflow.sql"),
    },
];
