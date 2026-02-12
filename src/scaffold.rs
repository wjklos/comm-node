//! Directory scaffolder for `comm-node init`.
//!
//! Creates the `.orchestrator/` directory structure in each domain,
//! writes `registry.json` for peer discovery, and `PROTOCOL.md`
//! with communication rules.

use anyhow::Result;

use crate::config::ProjectConfig;

/// Scaffold the `.orchestrator/` directories for all configured domains.
///
/// Creates:
/// - `.orchestrator/artifacts/`
/// - `.orchestrator/inbox/`
/// - `.orchestrator/outbox/`
/// - `.orchestrator/registry.json`
/// - `.orchestrator/PROTOCOL.md`
/// - `.orchestrator/status.json` (template)
pub fn scaffold(config: &ProjectConfig) -> Result<()> {
    for (domain_id, domain_config) in &config.domains {
        let orch_dir = domain_config.path.join(".orchestrator");

        std::fs::create_dir_all(orch_dir.join("artifacts"))?;
        std::fs::create_dir_all(orch_dir.join("inbox"))?;
        std::fs::create_dir_all(orch_dir.join("outbox"))?;

        tracing::info!(domain = %domain_id, path = %orch_dir.display(), "scaffolded .orchestrator/");
    }

    write_registry(config)?;
    write_protocol(config)?;

    Ok(())
}

/// Write `registry.json` to each domain's `.orchestrator/` directory.
fn write_registry(config: &ProjectConfig) -> Result<()> {
    let registry: Vec<_> = config
        .domains
        .iter()
        .map(|(id, dc)| {
            serde_json::json!({
                "domain": id.as_str(),
                "description": dc.description,
            })
        })
        .collect();

    let registry_json = serde_json::to_string_pretty(&registry)?;

    for domain_config in config.domains.values() {
        let path = domain_config.path.join(".orchestrator/registry.json");
        std::fs::write(&path, &registry_json)?;
    }

    Ok(())
}

/// Write `PROTOCOL.md` to each domain's `.orchestrator/` directory.
fn write_protocol(config: &ProjectConfig) -> Result<()> {
    let protocol = include_str!("protocol_template.md");

    for domain_config in config.domains.values() {
        let path = domain_config.path.join(".orchestrator/PROTOCOL.md");
        std::fs::write(&path, protocol)?;
    }

    Ok(())
}
