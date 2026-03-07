use anyhow::Result;

use super::driver::{manifest_bin, run_command, DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct PythonDriver;

impl InstallDriver for PythonDriver {
    fn name(&self) -> &'static str {
        "python"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("requirements.txt").exists() || repo_path.join("pyproject.toml").exists()
    }

    fn install(&self, ctx: &InstallContext, runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        let requirements = ctx.repo_path.join("requirements.txt");
        if requirements.exists() {
            run_command(
                &runtime.runtime.config.paths.pip,
                &["install", "-r", "requirements.txt"],
                &ctx.repo_path,
            )?;
        }

        Ok(InstallResult {
            binary_path: manifest_bin(ctx)?,
        })
    }
}
