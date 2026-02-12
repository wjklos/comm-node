//! Filesystem watcher for agent outbox directories.
//!
//! Uses the `notify` crate (inotify on Linux, FSEvents on macOS)
//! to detect new messages written by agents, then feeds them
//! to the router for processing.

use std::path::PathBuf;

use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// Watches outbox directories for new messages from agents.
pub struct OutboxWatcher {
    _watcher: RecommendedWatcher,
    pub events: mpsc::Receiver<PathBuf>,
}

impl OutboxWatcher {
    /// Create a new watcher that monitors the given outbox directories.
    ///
    /// Only forwards `Create` events for `.md` files — ignores modify,
    /// remove, access, directories, and temp files.
    pub fn new(outbox_dirs: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) => {
                    // Only react to file creation events.
                    if !matches!(event.kind, EventKind::Create(_)) {
                        return;
                    }

                    for path in event.paths {
                        // Only forward .md files.
                        let is_md = path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

                        if is_md {
                            if let Err(e) = tx.blocking_send(path) {
                                tracing::error!(error = %e, "failed to send watcher event");
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "filesystem watch error");
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
