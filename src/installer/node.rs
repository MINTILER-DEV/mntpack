use anyhow::{bail, Result};

use super::driver::{DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct NodeDriver;

impl InstallDriver for NodeDriver {
    fn name(&self) -> &'static str {
        "node"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("package.json").exists()
    }

    fn install(&self, _ctx: &InstallContext, _runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        bail!("node driver not implemented yet")
    }
}
