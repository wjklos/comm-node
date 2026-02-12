//! Filesystem watcher for agent outbox directories.
//!
//! Uses the `notify` crate (inotify on Linux, FSEvents on macOS)
//! to detect new messages written by agents, then feeds them
//! to the router for processing.

use std::path::PathBuf;

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// Watches outbox directories for new messages from agents.
pub struct OutboxWatcher {
    _watcher: RecommendedWatcher,
    pub events: mpsc::Receiver<PathBuf>,
}

impl OutboxWatcher {
    /// Create a new watcher that monitors the given outbox directories.
    pub fn new(outbox_dirs: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.blocking_send(path);
                }
            }
        })?;

        for dir in &outbox_dirs {
            watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            events: rx,
        })
    }
}
