use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const APP_DIR: &str = ".mntpack";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPaths {
    pub git: String,
    pub python: String,
    pub pip: String,
    pub node: String,
    pub npm: String,
    pub cargo: String,
}

impl Default for ToolPaths {
    fn default() -> Self {
        Self {
            git: "git".to_string(),
            python: "python".to_string(),
            pip: "pip".to_string(),
            node: "node".to_string(),
            npm: "npm".to_string(),
            cargo: "cargo".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_owner")]
    pub default_owner: String,
    #[serde(default)]
    pub paths: ToolPaths,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_owner: default_owner(),
            paths: ToolPaths::default(),
        }
    }
}

fn default_owner() -> String {
    "MINTILER-DEV".to_string()
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub repos: PathBuf,
    pub packages: PathBuf,
    pub cache: PathBuf,
    pub bin: PathBuf,
}

impl AppPaths {
    pub fn package_dir(&self, package_name: &str) -> PathBuf {
        self.packages.join(package_name)
    }

    pub fn repo_dir(&self, repo_key: &str) -> PathBuf {
        self.repos.join(repo_key)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub config: Config,
    pub paths: AppPaths,
}

impl RuntimeContext {
    pub fn load_or_init() -> Result<Self> {
        let home = dirs::home_dir().context("unable to locate user home directory")?;
        let root = home.join(APP_DIR);
        let config_path = root.join(CONFIG_FILE);
        let repos = root.join("repos");
        let packages = root.join("packages");
        let cache = root.join("cache");
        let bin = root.join("bin");

        for dir in [&root, &repos, &packages, &cache, &bin] {
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;
        }

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            serde_json::from_str::<Config>(&content)
                .with_context(|| format!("failed to parse {}", config_path.display()))?
        } else {
            let cfg = Config::default();
            let serialized = serde_json::to_string_pretty(&cfg)?;
            fs::write(&config_path, serialized)
                .with_context(|| format!("failed to write {}", config_path.display()))?;
            cfg
        };

        Ok(Self {
            config,
            paths: AppPaths {
                root,
                config: config_path,
                repos,
                packages,
                cache,
                bin,
            },
        })
    }
}

pub fn package_name_from_repo(owner: &str, repo: &str) -> String {
    if owner.eq_ignore_ascii_case("MINTILER-DEV") {
        repo.to_string()
    } else {
        format!("{owner}-{repo}")
    }
}

pub fn repo_key(owner: &str, repo: &str) -> String {
    format!("{owner}__{repo}")
}

pub fn normalize_repo_url(url: &str) -> String {
    if url.ends_with(".git") {
        url.to_string()
    } else {
        format!("{url}.git")
    }
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}
