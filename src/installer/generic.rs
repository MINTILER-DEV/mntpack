use anyhow::{bail, Result};

use super::driver::{DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct GenericDriver;

impl InstallDriver for GenericDriver {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn detect(&self, _repo_path: &std::path::Path) -> bool {
        true
    }

    fn install(&self, _ctx: &InstallContext, _runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        bail!("generic driver not implemented yet")
    }
}
