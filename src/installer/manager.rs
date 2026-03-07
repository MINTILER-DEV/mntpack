use std::{fs, path::PathBuf};

use anyhow::{bail, Context, Result};

use super::{
    driver::{DriverRuntime, InstallContext, InstallDriver, InstallResult},
    generic::GenericDriver,
    node::NodeDriver,
    python::PythonDriver,
    rust::RustDriver,
};

pub struct InstallerManager {
    drivers: Vec<Box<dyn InstallDriver>>,
}

impl InstallerManager {
    pub fn new() -> Self {
        Self {
            drivers: vec![
                Box::new(RustDriver),
                Box::new(PythonDriver),
                Box::new(NodeDriver),
                Box::new(GenericDriver),
            ],
        }
    }

    pub fn install(&self, ctx: &InstallContext, runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        fs::create_dir_all(&ctx.package_dir).with_context(|| {
            format!("failed to create package directory {}", ctx.package_dir.display())
        })?;

        for driver in &self.drivers {
            if driver.detect(&ctx.repo_path) {
                let result = driver.install(ctx, runtime)?;
                let destination = normalized_binary_destination(&ctx.package_dir, &ctx.package_name);
                copy_binary(&result.binary_path, &destination)?;
                return Ok(InstallResult {
                    binary_path: destination,
                });
            }
        }

        bail!("no install driver matched repository")
    }
}

fn normalized_binary_destination(package_dir: &std::path::Path, package_name: &str) -> PathBuf {
    if cfg!(windows) {
        package_dir.join(format!("{package_name}.exe"))
    } else {
        package_dir.join(package_name)
    }
}

fn copy_binary(source: &std::path::Path, destination: &std::path::Path) -> Result<()> {
    if !source.exists() {
        bail!("binary not found at {}", source.display());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy binary {} -> {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}
