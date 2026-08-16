use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

/// Pragmas every on-disk connection needs. Kept in one place so a reopened
/// connection is configured exactly like the original.
const FILE_PRAGMAS: &str = "PRAGMA journal_mode = WAL;
     PRAGMA foreign_keys = ON;
     PRAGMA busy_timeout = 5000;";

impl Database {
    pub fn open(path: &Path) -> crate::error::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(FILE_PRAGMAS)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> crate::error::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Point this handle at `path`, replacing the connection underneath.
    ///
    /// Restore swaps the database file on disk, and every service holds an
    /// `Arc<Database>` cloned at startup. Swapping the `Connection` inside the
    /// existing mutex lets those handles keep working against the new file
    /// without rebuilding application state or asking the user to restart.
    ///
    /// The new connection is only installed once it is open and configured, so a
    /// failure leaves the previous connection intact and usable.
    pub fn reopen(&self, path: &Path) -> crate::error::Result<()> {
        let replacement = Connection::open(path)?;
        replacement.execute_batch(FILE_PRAGMAS)?;
        let mut guard = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = replacement;
        Ok(())
    }

    /// Release the database file, parking this handle on a scratch in-memory
    /// connection.
    ///
    /// Call before replacing the file on disk. Closing the file connection first
    /// means SQLite finishes its own WAL cleanup against the database it was
    /// actually opened on, instead of against whatever file later occupies that
    /// path.
    pub fn detach(&self) -> crate::error::Result<()> {
        let scratch = Connection::open_in_memory()?;
        let mut guard = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = scratch;
        Ok(())
    }
}
