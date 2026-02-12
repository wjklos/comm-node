//! Shared types used across the comm-node.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies a domain (e.g. "backend", "frontend").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainId(pub String);

impl DomainId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DomainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a routed message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Idle,
    Working,
    Blocked,
    Complete,
}

/// Agent status written to `.orchestrator/status.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub domain: String,
    pub status: AgentState,
    pub current_task: Option<String>,
    pub last_heartbeat: DateTime<Utc>,
    pub artifacts_produced: Vec<String>,
    pub blocked_on: Option<String>,
}

impl AgentStatus {
    /// Create a new idle status for the given domain.
    pub fn new_idle(domain: &DomainId) -> Self {
        Self {
            domain: domain.as_str().to_owned(),
            status: AgentState::Idle,
            current_task: None,
            last_heartbeat: Utc::now(),
            artifacts_produced: Vec::new(),
            blocked_on: None,
        }
    }
}
