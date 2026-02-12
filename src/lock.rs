//! Advisory file lock manager.
//!
//! In-memory HashMap tracking which files are locked by which domain.
//! Snapshots to disk every 30s for crash recovery.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::DomainId;

/// A single advisory file lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLock {
    pub path: PathBuf,
    pub holder: DomainId,
    pub acquired_at: DateTime<Utc>,
}

/// Manages advisory file locks across domains.
pub struct LockManager {
    locks: HashMap<PathBuf, FileLock>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: HashMap::new(),
        }
    }

    /// Acquire an advisory lock on a file path for a domain.
    /// Returns `Err` if the file is already locked by another domain.
    pub fn acquire(&mut self, path: PathBuf, holder: DomainId) -> Result<()> {
        if let Some(existing) = self.locks.get(&path) {
            if existing.holder != holder {
                anyhow::bail!(
                    "file {} already locked by {}",
                    path.display(),
                    existing.holder
                );
            }
            return Ok(());
        }

        self.locks.insert(
            path.clone(),
            FileLock {
                path,
                holder,
                acquired_at: Utc::now(),
            },
        );

        Ok(())
    }

    /// Release a lock on a file path.
    pub fn release(&mut self, path: &Path, holder: &DomainId) -> Result<()> {
        if let Some(existing) = self.locks.get(path) {
            if &existing.holder != holder {
                anyhow::bail!(
                    "cannot release lock on {} -- held by {}, not {}",
                    path.display(),
                    existing.holder,
                    holder
                );
            }
        }

        self.locks.remove(path);
        Ok(())
    }

    /// Snapshot current locks to a JSON file for crash recovery.
    pub fn snapshot(&self, path: &Path) -> Result<()> {
        let locks: Vec<&FileLock> = self.locks.values().collect();
        let json = serde_json::to_string_pretty(&locks)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Restore locks from a snapshot file.
    pub fn restore(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let locks: Vec<FileLock> = serde_json::from_str(&json)?;
        let map = locks.into_iter().map(|l| (l.path.clone(), l)).collect();
        Ok(Self { locks: map })
    }

    /// List all currently held locks.
    pub fn list(&self) -> Vec<&FileLock> {
        self.locks.values().collect()
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}
