use anyhow::{bail, Result};

use super::driver::{DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct RustDriver;

impl InstallDriver for RustDriver {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("Cargo.toml").exists()
    }

    fn install(&self, _ctx: &InstallContext, _runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        bail!("rust driver not implemented yet")
    }
}
