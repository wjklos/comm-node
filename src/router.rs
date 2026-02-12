//! Message routing between agent domains.
//!
//! Parses YAML frontmatter from outbox messages, validates fields,
//! and delivers messages to the target domain's inbox.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
}

impl Router {
    pub fn new(domains: HashMap<DomainId, PathBuf>) -> Self {
        Self { domains }
    }

    /// Parse a message file and route it to the target domain's inbox.
    pub fn route(&self, message_path: &Path) -> Result<()> {
        let message = Self::parse(message_path)?;

        let target_dir = self
            .domains
            .get(&message.to)
            .context(format!("unknown target domain: {}", message.to))?;

        let inbox = target_dir.join("inbox");
        let filename = message_path
            .file_name()
            .context("message path has no filename")?;
        let dest = inbox.join(filename);

        std::fs::copy(message_path, &dest)?;
        std::fs::remove_file(message_path)?;

        tracing::info!(
            from = %message.from,
            to = %message.to,
            msg_type = %message.msg_type,
            "routed message"
        );

        Ok(())
    }

    /// Parse a message file into a Message struct.
    fn parse(path: &Path) -> Result<Message> {
        let content = std::fs::read_to_string(path)?;

        // Split YAML frontmatter from body.
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            anyhow::bail!("message missing YAML frontmatter: {}", path.display());
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim().to_string();

        let mut message: Message = serde_yaml::from_str(frontmatter)?;
        message.body = body;

        Ok(message)
    }
}
