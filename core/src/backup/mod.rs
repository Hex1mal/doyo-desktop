use crate::error::Result;
use std::path::PathBuf;

pub struct BackupService {
    db_path: PathBuf,
    backup_dir: PathBuf,
    max_backups: usize,
}

impl BackupService {
    pub fn new(db_path: PathBuf, backup_dir: PathBuf, max_backups: usize) -> Self {
        Self { db_path, backup_dir, max_backups }
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
        let backup_path = self.backup_dir.join(backup_name);
        if !backup_path.exists() {
            return Err(crate::error::Error::NotFound(format!("Backup not found: {}", backup_name)));
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
                let _ = std::fs::remove_file(self.backup_dir.join(&oldest));
            }
        }
        Ok(())
    }
}
