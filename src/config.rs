//! TOML configuration types for the comm-node.
//!
//! The project config defines domains, their filesystem scopes,
//! and orchestrator settings. Loaded from `comm-node.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
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

impl ProjectConfig {
    /// Validate the configuration, bailing on fatal errors and warning on issues.
    pub fn validate(&self) -> Result<()> {
        if self.domains.is_empty() {
            bail!("config has no domains defined");
        }

        for (id, dc) in &self.domains {
            if id.as_str().is_empty() {
                bail!("domain ID must not be empty");
            }

            if !dc.path.exists() {
                tracing::warn!(domain = %id, path = %dc.path.display(), "domain path does not exist");
            }
        }

        // Check for overlapping scope patterns across domains.
        let domains: Vec<_> = self.domains.iter().collect();
        for i in 0..domains.len() {
            for j in (i + 1)..domains.len() {
                let (id_a, dc_a) = domains[i];
                let (id_b, dc_b) = domains[j];
                for scope_a in &dc_a.scope {
                    for scope_b in &dc_b.scope {
                        if scope_a == scope_b {
                            tracing::warn!(
                                domain_a = %id_a,
                                domain_b = %id_b,
                                scope = %scope_a,
                                "overlapping scope pattern across domains"
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Load a project config from a TOML file.
pub fn load(path: &Path) -> Result<ProjectConfig> {
    let contents = std::fs::read_to_string(path)?;
    let config: ProjectConfig = toml::from_str(&contents)?;
    config.validate()?;
    Ok(config)
}
