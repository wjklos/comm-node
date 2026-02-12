//! Artifact storage trait and filesystem implementation.
//!
//! Artifacts are the cross-domain work products (API contracts, type defs,
//! specs) that flow between agents. The filesystem implementation stores
//! them in each domain's `.orchestrator/artifacts/` directory.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

/// Trait for artifact storage backends.
pub trait ArtifactStore: Send + Sync {
    /// Store an artifact, making it available to the target domain.
    fn store(&self, domain: &str, name: &str, content: &[u8]) -> Result<()>;

    /// Retrieve an artifact by domain and name.
    fn retrieve(&self, domain: &str, name: &str) -> Result<Vec<u8>>;

    /// List all artifacts available in a domain.
    fn list(&self, domain: &str) -> Result<Vec<String>>;
}

/// Filesystem-backed artifact store.
///
/// Artifacts are stored under `<domain_root>/.orchestrator/artifacts/<name>`.
pub struct FsArtifactStore {
    /// Map of domain name -> domain root path.
    roots: HashMap<String, PathBuf>,
}

impl FsArtifactStore {
    pub fn new(roots: HashMap<String, PathBuf>) -> Self {
        Self { roots }
    }

    fn artifacts_dir(&self, domain: &str) -> Result<PathBuf> {
        let root = self
            .roots
            .get(domain)
            .ok_or_else(|| anyhow::anyhow!("unknown domain: {}", domain))?;
        Ok(root.join(".orchestrator/artifacts"))
    }
}

impl ArtifactStore for FsArtifactStore {
    fn store(&self, domain: &str, name: &str, content: &[u8]) -> Result<()> {
        let dir = self.artifacts_dir(domain)?;
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(name), content)?;
        Ok(())
    }

    fn retrieve(&self, domain: &str, name: &str) -> Result<Vec<u8>> {
        let dir = self.artifacts_dir(domain)?;
        let data = std::fs::read(dir.join(name))?;
        Ok(data)
    }

    fn list(&self, domain: &str) -> Result<Vec<String>> {
        let dir = self.artifacts_dir(domain)?;
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut names = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }
}
