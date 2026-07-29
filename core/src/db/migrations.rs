use super::Database;

pub fn run_migrations(db: &Database) -> crate::error::Result<()> {
    let conn = db.conn.lock().unwrap();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            description TEXT
        );"
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        run_migration_001(&conn)?;
    }
    if current_version < 2 {
        run_migration_002(&conn)?;
    }
    if current_version < 3 {
        run_migration_003(&conn)?;
    }
    if current_version < 4 {
        run_migration_004(&conn)?;
    }
    if current_version < 5 {
        run_migration_005(&conn)?;
    }
    if current_version < 6 {
        run_migration_006(&conn)?;
    }

    Ok(())
}

fn run_migration_001(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/001_initial.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (1, 'Initial schema')",
        [],
    )?;
    Ok(())
}

fn run_migration_002(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/002_time_blocks.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (2, 'Add calendar time blocks')",
        [],
    )?;
    Ok(())
}

fn run_migration_003(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/003_focus_sessions.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (3, 'Add focus sessions')",
        [],
    )?;
    Ok(())
}

fn run_migration_004(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/004_productivity_entities.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (4, 'Add saved filters, habits, habit logs, and countdowns')",
        [],
    )?;
    Ok(())
}

fn run_migration_005(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/005_habit_days.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (5, 'Add habit weekly days column')",
        [],
    )?;
    Ok(())
}

fn run_migration_006(conn: &rusqlite::Connection) -> crate::error::Result<()> {
    conn.execute_batch(include_str!("migrations/006_focus_workflow.sql"))?;
    conn.execute(
        "INSERT INTO schema_version (version, description) VALUES (6, 'Add focus workflow metadata')",
        [],
    )?;
    Ok(())
}
