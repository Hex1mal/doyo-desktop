use crate::db::LATEST_SCHEMA_VERSION;
use crate::error::{Error, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Component, Path, PathBuf};

/// Sidecar files SQLite keeps alongside a WAL-mode database. They describe the
/// *previous* database and must never survive a restore, or SQLite may replay
/// them over the freshly restored file.
const SIDECAR_SUFFIXES: [&str; 2] = ["-wal", "-shm"];

/// Filename prefix for the snapshot taken immediately before a restore.
pub const PRE_RESTORE_PREFIX: &str = "doyo-pre-restore-";

/// Pre-restore snapshots are pruned against their own budget so routine
/// backups can never evict them.
const MAX_PRE_RESTORE_SNAPSHOTS: usize = 5;

pub struct BackupService {
    db_path: PathBuf,
    backup_dir: PathBuf,
    max_backups: usize,
}

impl BackupService {
    pub fn new(db_path: PathBuf, backup_dir: PathBuf, max_backups: usize) -> Self {
        Self {
            db_path,
            backup_dir,
            max_backups,
        }
    }

    pub fn create_backup(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.backup_dir)?;
        let now = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
        let suffix = uuid::Uuid::now_v7();
        let backup_name = format!("doyo-backup-{}-{}.db", now, suffix);
        let backup_path = self.backup_dir.join(&backup_name);
        std::fs::copy(&self.db_path, &backup_path)?;
        self.prune_old_backups()?;
        Ok(backup_path)
    }

    /// Restore a backup over the live database.
    ///
    /// The live database is the user's only copy, so this is deliberately
    /// defensive and ordered so that the destructive step happens last:
    ///   1. validate the backup is a genuine, intact, compatible Doyo database
    ///   2. snapshot the current database so the restore is reversible
    ///   3. stage the backup beside the live file and swap it in atomically
    ///   4. drop the stale `-wal`/`-shm` sidecars of the replaced database
    ///
    /// Returns the path of the pre-restore snapshot, when one was taken.
    pub fn restore_backup(&self, backup_name: &str) -> Result<Option<PathBuf>> {
        let backup_path = self.validated_backup_path(backup_name)?;
        if !backup_path.exists() {
            return Err(Error::NotFound(format!(
                "Backup not found: {}",
                backup_name
            )));
        }

        // 1. Never overwrite live data with a file we have not verified.
        validate_restorable_database(&backup_path)?;

        // 2. Keep an escape hatch before touching anything.
        let safety_copy = self.create_pre_restore_snapshot()?;

        // 3. Stage then swap, so an interrupted copy cannot truncate the live DB.
        let staged = self.db_path.with_extension("restore-staging");
        let _ = std::fs::remove_file(&staged);
        if let Err(e) = std::fs::copy(&backup_path, &staged) {
            let _ = std::fs::remove_file(&staged);
            return Err(e.into());
        }
        if let Err(e) = std::fs::rename(&staged, &self.db_path) {
            let _ = std::fs::remove_file(&staged);
            return Err(e.into());
        }

        // 4. The old sidecars belong to the database we just replaced.
        self.remove_stale_sidecars();

        Ok(safety_copy)
    }

    /// Snapshot the live database immediately before a restore. Named distinctly
    /// from routine backups so it is obvious what it is during recovery.
    fn create_pre_restore_snapshot(&self) -> Result<Option<PathBuf>> {
        if !self.db_path.exists() {
            return Ok(None);
        }
        std::fs::create_dir_all(&self.backup_dir)?;
        let now = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
        let suffix = uuid::Uuid::now_v7();
        let path = self
            .backup_dir
            .join(format!("{PRE_RESTORE_PREFIX}{}-{}.db", now, suffix));
        std::fs::copy(&self.db_path, &path)?;
        Ok(Some(path))
    }

    fn remove_stale_sidecars(&self) {
        let Some(file_name) = self.db_path.file_name().map(|n| n.to_os_string()) else {
            return;
        };
        for suffix in SIDECAR_SUFFIXES {
            let mut sidecar = file_name.clone();
            sidecar.push(suffix);
            let _ = std::fs::remove_file(self.db_path.with_file_name(&sidecar));
        }
    }

    pub fn list_backups(&self) -> Result<Vec<String>> {
        if !self.backup_dir.exists() {
            return Ok(vec![]);
        }
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().ends_with(".db") {
                backups.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        backups.sort();
        backups.reverse();
        Ok(backups)
    }

    /// Prune routine backups and pre-restore snapshots against separate budgets.
    ///
    /// They are kept apart on purpose: a burst of routine backups must never be
    /// able to evict the snapshot that makes a bad restore reversible.
    fn prune_old_backups(&self) -> Result<()> {
        let all = self.list_backups()?;
        let (snapshots, routine): (Vec<String>, Vec<String>) = all
            .into_iter()
            .partition(|name| name.starts_with(PRE_RESTORE_PREFIX));
        self.prune_to_limit(routine, self.max_backups);
        self.prune_to_limit(snapshots, MAX_PRE_RESTORE_SNAPSHOTS);
        Ok(())
    }

    /// `names` arrives newest-first, so the tail is the oldest.
    fn prune_to_limit(&self, mut names: Vec<String>, limit: usize) {
        while names.len() > limit {
            if let Some(oldest) = names.pop() {
                if let Ok(path) = self.validated_backup_path(&oldest) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    fn validated_backup_path(&self, backup_name: &str) -> Result<PathBuf> {
        validate_backup_name(backup_name)?;
        let backup_dir = ensure_canonical_dir(&self.backup_dir)?;
        let backup_path = backup_dir.join(backup_name);
        let canonical = if backup_path.exists() {
            backup_path.canonicalize()?
        } else {
            backup_path
        };
        if !canonical.starts_with(&backup_dir) {
            return Err(Error::Validation(
                "Backup path escapes the backup directory".into(),
            ));
        }
        Ok(canonical)
    }
}

/// Verify a file is a genuine, intact Doyo database that this build can open.
///
/// Opened read-only on purpose: a validation pass must never write to (or set the
/// journal mode of) the candidate file.
pub fn validate_restorable_database(path: &Path) -> Result<i64> {
    let conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| {
            Error::Validation(format!(
                "Backup is not a readable SQLite database ({}): {e}",
                display_name(path)
            ))
        })?;

    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| {
            Error::Validation(format!(
                "Backup failed its integrity check ({}): {e}",
                display_name(path)
            ))
        })?;
    if integrity != "ok" {
        return Err(Error::Validation(format!(
            "Backup is corrupted ({}): {integrity}",
            display_name(path)
        )));
    }

    let has_nodes: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'nodes'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);
    if !has_nodes {
        return Err(Error::Validation(format!(
            "Backup is not a Doyo database ({}): no nodes table",
            display_name(path)
        )));
    }

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .map_err(|e| {
            Error::Validation(format!(
                "Backup is not a Doyo database ({}): {e}",
                display_name(path)
            ))
        })?;
    if version < 1 {
        return Err(Error::Validation(format!(
            "Backup has no applied schema version ({})",
            display_name(path)
        )));
    }
    if version > LATEST_SCHEMA_VERSION as i64 {
        return Err(Error::Validation(format!(
            "Backup was written by a newer version of Doyo (schema v{version}, this build supports v{LATEST_SCHEMA_VERSION}). Upgrade Doyo before restoring it."
        )));
    }

    Ok(version)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn validate_backup_name(backup_name: &str) -> Result<()> {
    if backup_name.trim().is_empty() {
        return Err(Error::Validation("Backup filename is required".into()));
    }
    let path = Path::new(backup_name);
    if path.is_absolute() {
        return Err(Error::Validation(
            "Backup filename must not be absolute".into(),
        ));
    }
    if path.components().count() != 1 {
        return Err(Error::Validation(
            "Backup filename must not contain path separators".into(),
        ));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Validation(
            "Backup filename contains invalid path components".into(),
        ));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("db") {
        return Err(Error::Validation(
            "Backup filename must end with .db".into(),
        ));
    }
    Ok(())
}

fn ensure_canonical_dir(path: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(path)?;
    Ok(path.canonicalize()?)
}
