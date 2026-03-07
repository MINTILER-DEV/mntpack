use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{config::RuntimeContext, package::manifest::Manifest};

#[derive(Debug, Clone)]
pub struct InstallContext {
    pub package_name: String,
    pub repo_path: PathBuf,
    pub package_dir: PathBuf,
    pub manifest: Option<Manifest>,
}

pub struct DriverRuntime<'a> {
    pub runtime: &'a RuntimeContext,
}

#[derive(Debug, Clone)]
pub struct InstallResult {
    pub binary_path: PathBuf,
}

pub trait InstallDriver: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect(&self, repo_path: &Path) -> bool;
    fn install(&self, ctx: &InstallContext, runtime: &DriverRuntime<'_>) -> Result<InstallResult>;
}
