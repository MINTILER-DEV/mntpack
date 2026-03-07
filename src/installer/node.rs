use anyhow::Result;

use super::driver::{manifest_bin, run_command, DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct NodeDriver;

impl InstallDriver for NodeDriver {
    fn name(&self) -> &'static str {
        "node"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("package.json").exists()
    }

    fn install(&self, ctx: &InstallContext, runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        run_command(&runtime.runtime.config.paths.npm, &["install"], &ctx.repo_path)?;

        Ok(InstallResult {
            binary_path: manifest_bin(ctx)?,
        })
    }
}
