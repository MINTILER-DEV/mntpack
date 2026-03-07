use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::driver::{manifest_bin, run_command, DriverRuntime, InstallContext, InstallDriver, InstallResult};

pub struct RustDriver;

impl InstallDriver for RustDriver {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn detect(&self, repo_path: &std::path::Path) -> bool {
        repo_path.join("Cargo.toml").exists()
    }

    fn install(&self, ctx: &InstallContext, runtime: &DriverRuntime<'_>) -> Result<InstallResult> {
        run_command(
            &runtime.runtime.config.paths.cargo,
            &["build", "--release"],
            &ctx.repo_path,
        )?;

        if ctx.manifest.as_ref().and_then(|m| m.bin.as_ref()).is_some() {
            return Ok(InstallResult {
                binary_path: manifest_bin(ctx)?,
            });
        }

        let binary = infer_rust_binary(ctx)?;
        Ok(InstallResult { binary_path: binary })
    }
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
}

fn infer_rust_binary(ctx: &InstallContext) -> Result<PathBuf> {
    let release_dir = ctx.repo_path.join("target").join("release");
    if !release_dir.exists() {
        anyhow::bail!(
            "cargo build completed but release output was not found at {}",
            release_dir.display()
        );
    }

    let expected = expected_binary_name(ctx)?;
    if let Some(name) = expected {
        let candidate = release_dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(&release_dir)
        .with_context(|| format!("failed to read {}", release_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".d")
            || file_name.ends_with(".pdb")
            || file_name.ends_with(".rlib")
            || file_name.ends_with(".rmeta")
        {
            continue;
        }
        if cfg!(windows) && !file_name.ends_with(".exe") {
            continue;
        }
        if !cfg!(windows) && path.extension().is_some() {
            continue;
        }
        candidates.push(path);
    }

    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    anyhow::bail!(
        "unable to infer rust binary; define 'bin' in mntpack.json for package '{}'",
        ctx.package_name
    )
}

fn expected_binary_name(ctx: &InstallContext) -> Result<Option<String>> {
    let cargo_toml = ctx.repo_path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("failed to read {}", cargo_toml.display()))?;
    let parsed = toml::from_str::<CargoToml>(&content)
        .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;
    let Some(name) = parsed.package.and_then(|p| p.name) else {
        return Ok(None);
    };
    let mut executable_name = name.replace('-', "_");
    if cfg!(windows) {
        executable_name.push_str(".exe");
    }
    Ok(Some(executable_name))
}
