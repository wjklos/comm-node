//! Async event loop wiring the filesystem watcher to the message router.
//!
//! The orchestrator owns the runtime lifecycle: it watches all outbox
//! directories, routes messages through the router, and handles
//! graceful shutdown on ctrl-c.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::artifact::FsArtifactStore;
use crate::config::ProjectConfig;
use crate::event::FileEventLog;
use crate::router::Router;
use crate::types::DomainId;
use crate::watcher::OutboxWatcher;

/// The main orchestrator that wires watcher -> router -> event log.
pub struct Orchestrator {
    router: Arc<Router>,
    watcher: OutboxWatcher,
}

impl Orchestrator {
    /// Build an orchestrator from a project config and state directory.
    ///
    /// Creates the artifact store, event log, router, and watcher.
    /// Ensures the state directory exists.
    pub fn new(config: &ProjectConfig, state_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&state_dir)
            .with_context(|| format!("creating state dir: {}", state_dir.display()))?;

        // Build domain -> .orchestrator/ path map.
        let domains: HashMap<DomainId, PathBuf> = config
            .domains
            .iter()
            .map(|(id, dc)| (id.clone(), dc.path.join(".orchestrator")))
            .collect();

        // Build domain -> root path map for the artifact store.
        let artifact_roots: HashMap<String, PathBuf> = config
            .domains
            .iter()
            .map(|(id, dc)| (id.as_str().to_owned(), dc.path.clone()))
            .collect();

        let artifact_store = Arc::new(FsArtifactStore::new(artifact_roots));
        let event_log = Arc::new(FileEventLog::new(state_dir.join("event.log")));
        let router = Arc::new(Router::new(domains.clone(), artifact_store, event_log));

        // Collect all outbox directories.
        let outbox_dirs: Vec<PathBuf> = domains.values().map(|d| d.join("outbox")).collect();

        let watcher = OutboxWatcher::new(outbox_dirs).context("creating outbox watcher")?;

        Ok(Self { router, watcher })
    }

    /// Run the async event loop until ctrl-c.
    pub async fn run(mut self) -> Result<()> {
        tracing::info!("comm-node started, watching outboxes");

        loop {
            tokio::select! {
                Some(path) = self.watcher.events.recv() => {
                    let router = self.router.clone();
                    // Route inline — no need to spawn since messages are sequential.
                    if let Err(e) = router.route(&path).await {
                        tracing::error!(
                            path = %path.display(),
                            error = %e,
                            "failed to route message"
                        );
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("received ctrl-c, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }
}
