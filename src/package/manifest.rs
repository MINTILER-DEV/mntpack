use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const MANIFEST_FILE: &str = "mntpack.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseAssetConfig {
    pub file: String,
    pub bin: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub preinstall: Option<String>,
    pub postinstall: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub build: Option<String>,
    pub bin: Option<String>,
    #[serde(default)]
    pub release: HashMap<String, ReleaseAssetConfig>,
}

impl Manifest {
    pub fn load(repo_path: &Path) -> Result<Option<Self>> {
        let file = repo_path.join(MANIFEST_FILE);
        if !file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let manifest = serde_json::from_str::<Self>(&content)
            .with_context(|| format!("failed to parse {}", file.display()))?;
        Ok(Some(manifest))
    }
}
