//! Event logging trait and filesystem implementation.
//!
//! Events are the comm-node's internal audit trail: messages routed,
//! locks acquired, agents registered, etc. Append-only log,
//! queryable with grep/jq in the filesystem implementation.

use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single logged event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: DateTime<Utc>,
    pub kind: String,
    pub payload: serde_json::Value,
}

/// Trait for event logging backends.
pub trait EventLog: Send + Sync {
    /// Append an event to the log.
    fn log(&self, event: &Event) -> Result<()>;

    /// Query events by kind, returning matches in chronological order.
    fn query(&self, kind: &str) -> Result<Vec<Event>>;
}

/// Append-only file-based event log (TSV: timestamp + JSON).
pub struct FileEventLog {
    path: PathBuf,
}

impl FileEventLog {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl EventLog for FileEventLog {
    fn log(&self, event: &Event) -> Result<()> {
        use std::io::Write;
        let json = serde_json::to_string(&event.payload)?;
        let line = format!("{}\t{}\t{}\n", event.timestamp.to_rfc3339(), event.kind, json);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        Ok(())
    }

    fn query(&self, kind: &str) -> Result<Vec<Event>> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        let mut events = Vec::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() == 3 && parts[1] == kind {
                let timestamp = parts[0].parse::<DateTime<Utc>>()?;
                let payload: serde_json::Value = serde_json::from_str(parts[2])?;
                events.push(Event {
                    timestamp,
                    kind: parts[1].to_string(),
                    payload,
                });
            }
        }

        Ok(events)
    }
}
