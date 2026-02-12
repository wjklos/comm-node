//! TOML configuration types for the comm-node.
//!
//! The project config defines domains, their filesystem scopes,
//! and orchestrator settings. Loaded from `comm-node.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::DomainId;

/// Top-level project configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Domains managed by this comm-node instance.
    pub domains: HashMap<DomainId, DomainConfig>,
}

/// Configuration for a single domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Root path of the domain's working directory.
    pub path: PathBuf,

    /// Glob patterns defining the domain's file scope for boundary enforcement.
    #[serde(default)]
    pub scope: Vec<String>,

    /// Human-readable description of this domain's responsibility.
    #[serde(default)]
    pub description: String,
}

/// Load a project config from a TOML file.
pub fn load(path: &Path) -> Result<ProjectConfig> {
    let contents = std::fs::read_to_string(path)?;
    let config: ProjectConfig = toml::from_str(&contents)?;
    Ok(config)
}
