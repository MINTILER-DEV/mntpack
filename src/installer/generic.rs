use anyhow::{Result, bail};

use super::driver::{
    DriverRuntime, InstallContext, InstallDriver, InstallResult, manifest_bin, run_shell_command,
};

pub struct GenericDriver;

impl InstallDriver for GenericDriver {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn detect(&self, _repo_path: &std::path::Path) -> bool {
        true
    }

    fn install(&self, ctx: &InstallContext, _runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        let Some(manifest) = &ctx.manifest else {
            bail!("generic installs require mntpack.json");
        };
        let Some(build_command) = &manifest.build else {
            bail!("generic installs require a 'build' command in mntpack.json");
        };

        run_shell_command(build_command, &ctx.repo_path)?;
        Ok(InstallResult {
            binary_path: manifest_bin(ctx)?,
        })
    }
}
