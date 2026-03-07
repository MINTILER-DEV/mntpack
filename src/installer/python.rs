use anyhow::{bail, Result};

use super::driver::{DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct PythonDriver;

impl InstallDriver for PythonDriver {
    fn name(&self) -> &'static str {
        "python"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("requirements.txt").exists() || repo_path.join("pyproject.toml").exists()
    }

    fn install(&self, _ctx: &InstallContext, _runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        bail!("python driver not implemented yet")
    }
}
