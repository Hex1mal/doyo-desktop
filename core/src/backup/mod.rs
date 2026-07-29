use crate::error::{Error, Result};
use std::path::{Component, Path, PathBuf};

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

    pub fn restore_backup(&self, backup_name: &str) -> Result<()> {
        let backup_path = self.validated_backup_path(backup_name)?;
        if !backup_path.exists() {
            return Err(crate::error::Error::NotFound(format!(
                "Backup not found: {}",
                backup_name
            )));
        }
        std::fs::copy(&backup_path, &self.db_path)?;
        Ok(())
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

    fn prune_old_backups(&self) -> Result<()> {
        let mut backups = self.list_backups()?;
        while backups.len() > self.max_backups {
            if let Some(oldest) = backups.pop() {
                if let Ok(path) = self.validated_backup_path(&oldest) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
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
