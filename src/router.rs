//! Message routing between agent domains.
//!
//! Parses YAML frontmatter from outbox messages, validates fields,
//! routes artifacts, handles completion signals, and delivers
//! messages to the target domain's inbox.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::artifact::ArtifactStore;
use crate::event::{Event, EventLog};
use crate::types::DomainId;

/// A parsed inter-agent message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub from: DomainId,
    pub to: DomainId,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// The markdown body after the frontmatter.
    #[serde(skip)]
    pub body: String,
}

/// Routes messages between domain inboxes.
pub struct Router {
    /// Base paths for each domain's `.orchestrator/` directory.
    domains: HashMap<DomainId, PathBuf>,
    /// Artifact store for cross-domain work products.
    artifact_store: Arc<dyn ArtifactStore>,
    /// Event log for routing audit trail.
    event_log: Arc<dyn EventLog>,
}

impl Router {
    pub fn new(
        domains: HashMap<DomainId, PathBuf>,
        artifact_store: Arc<dyn ArtifactStore>,
        event_log: Arc<dyn EventLog>,
    ) -> Self {
        Self {
            domains,
            artifact_store,
            event_log,
        }
    }

    /// Parse a message file and route it to the target domain's inbox.
    ///
    /// Full flow:
    /// 1. Parse message
    /// 2. Resolve source domain from path
    /// 3. Validate `from` field matches source domain
    /// 4. Validate target domain exists
    /// 5. Check for completion signal -> call `bd close`
    /// 6. Route artifacts if present
    /// 7. Copy message to target inbox, remove from source outbox
    /// 8. Log routing event
    pub async fn route(&self, message_path: &Path) -> Result<()> {
        let raw_content = std::fs::read(message_path)
            .with_context(|| format!("reading message: {}", message_path.display()))?;
        let size_bytes = raw_content.len();

        let message = Self::parse(message_path)?;

        // Resolve which domain's outbox this file lives in.
        let source_domain = self.resolve_source_domain(message_path)?;

        // Validate `from` field matches the actual source domain.
        self.validate_from(&message, &source_domain)?;

        // Validate target domain exists.
        if !self.domains.contains_key(&message.to) {
            bail!("unknown target domain: {}", message.to);
        }

        // Check for completion signal and call bd close (warn on failure).
        let filename = message_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        if let Some(task_id) = Self::parse_completion_signal(filename) {
            if let Err(e) = Self::close_beads_task(&task_id).await {
                tracing::warn!(task_id = %task_id, error = %e, "bd close failed (continuing)");
            }
        }

        // Route artifacts if present (failure = route failure).
        if !message.artifacts.is_empty() {
            self.route_artifacts(&message)?;
        }

        // Copy message to target inbox.
        let target_dir = &self.domains[&message.to];
        let inbox = target_dir.join("inbox");
        let dest = inbox.join(
            message_path
                .file_name()
                .context("message path has no filename")?,
        );

        std::fs::copy(message_path, &dest)?;
        std::fs::remove_file(message_path)?;

        tracing::info!(
            from = %message.from,
            to = %message.to,
            msg_type = %message.msg_type,
            size_bytes,
            "routed message"
        );

        // Log routing event.
        self.log_routing_event(&message, size_bytes);

        Ok(())
    }

    /// Reverse-lookup which domain's outbox a file lives in.
    fn resolve_source_domain(&self, path: &Path) -> Result<DomainId> {
        for (domain_id, orch_dir) in &self.domains {
            let outbox = orch_dir.join("outbox");
            if path.starts_with(&outbox) {
                return Ok(domain_id.clone());
            }
        }
        bail!(
            "cannot resolve source domain for path: {}",
            path.display()
        );
    }

    /// Validate that the `from` field in the message matches the actual source domain.
    fn validate_from(&self, message: &Message, actual_source: &DomainId) -> Result<()> {
        if &message.from != actual_source {
            bail!(
                "message `from: {}` does not match source domain `{}`",
                message.from,
                actual_source
            );
        }
        Ok(())
    }

    /// Route artifacts referenced in the message from source to target domain.
    fn route_artifacts(&self, message: &Message) -> Result<()> {
        for artifact_name in &message.artifacts {
            let content = self
                .artifact_store
                .retrieve(message.from.as_str(), artifact_name)
                .with_context(|| {
                    format!(
                        "retrieving artifact `{}` from domain `{}`",
                        artifact_name, message.from
                    )
                })?;

            self.artifact_store
                .store(message.to.as_str(), artifact_name, &content)
                .with_context(|| {
                    format!(
                        "storing artifact `{}` to domain `{}`",
                        artifact_name, message.to
                    )
                })?;

            tracing::info!(
                artifact = %artifact_name,
                from = %message.from,
                to = %message.to,
                "routed artifact"
            );
        }
        Ok(())
    }

    /// Detect completion signal filenames like `completion-bd-XXX.md`.
    /// Returns the task ID (e.g. `bd-XXX`) if matched.
    fn parse_completion_signal(filename: &str) -> Option<String> {
        let stem = filename.strip_suffix(".md")?;
        let task_id = stem.strip_prefix("completion-")?;
        if task_id.is_empty() {
            return None;
        }
        Some(task_id.to_string())
    }

    /// Shell out to `bd close <task-id>` to mark a beads task as complete.
    async fn close_beads_task(task_id: &str) -> Result<()> {
        let output = tokio::process::Command::new("bd")
            .args(["close", task_id])
            .output()
            .await
            .context("failed to execute `bd close`")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("bd close {} failed: {}", task_id, stderr.trim());
        }

        tracing::info!(task_id = %task_id, "closed beads task");
        Ok(())
    }

    /// Write a routing event to the event log.
    fn log_routing_event(&self, message: &Message, size_bytes: usize) {
        let event = Event {
            timestamp: chrono::Utc::now(),
            kind: "message_routed".to_string(),
            payload: serde_json::json!({
                "from": message.from.as_str(),
                "to": message.to.as_str(),
                "type": message.msg_type,
                "task": message.task,
                "priority": message.priority,
                "artifacts": message.artifacts,
                "size_bytes": size_bytes,
            }),
        };

        if let Err(e) = self.event_log.log(&event) {
            tracing::error!(error = %e, "failed to log routing event");
        }
    }

    /// Parse a message file into a Message struct.
    fn parse(path: &Path) -> Result<Message> {
        let content = std::fs::read_to_string(path)?;

        // Split YAML frontmatter from body.
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            bail!("message missing YAML frontmatter: {}", path.display());
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim().to_string();

        let mut message: Message = serde_yaml::from_str(frontmatter)?;
        message.body = body;

        Ok(message)
    }
}
