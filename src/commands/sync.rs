use std::{collections::HashSet, path::Path};

use anyhow::Result;
use async_recursion::async_recursion;

use crate::{
    config::RuntimeContext,
    github::{clone::sync_repo, release::try_download_release_binary},
    installer::{
        driver::{DriverRuntime, InstallContext, run_shell_command},
        manager::{InstallerManager, materialize_binary},
    },
    package::{
        manifest::Manifest,
        record::{PackageRecord, load_record, save_record},
        resolver::resolve_repo,
    },
    shim::generator::{create_shim, ensure_bin_on_path},
};

pub async fn execute(
    runtime: &RuntimeContext,
    repo_input: &str,
    version: Option<&str>,
    global: bool,
) -> Result<()> {
    let mut visited = HashSet::new();
    let record = sync_package_internal(runtime, repo_input, version, global, &mut visited).await?;
    println!("synced {} ({})", record.package_name, record.repo_spec());
    Ok(())
}

#[async_recursion]
pub async fn sync_package_internal(
    runtime: &RuntimeContext,
    repo_input: &str,
    version: Option<&str>,
    global: bool,
    visited: &mut HashSet<String>,
) -> Result<PackageRecord> {
    let resolved = resolve_repo(repo_input, &runtime.config.default_owner)?;
    let repo_dir = runtime.paths.repo_dir(&resolved.key);
    let package_dir = runtime.paths.package_dir(&resolved.package_name);

    if visited.contains(&resolved.key) {
        if let Some(record) = load_record(&package_dir)? {
            return Ok(record);
        }
    } else {
        visited.insert(resolved.key.clone());
    }

    sync_repo(&resolved, &repo_dir, version)?;
    let manifest = Manifest::load(&repo_dir)?;

    if let Some(manifest) = &manifest {
        for dependency in &manifest.dependencies {
            sync_package_internal(runtime, dependency, None, false, visited).await?;
        }
    }

    if let Some(script) = manifest.as_ref().and_then(|m| m.preinstall.as_deref()) {
        run_script(script, &repo_dir)?;
    }

    let runtime_driver = DriverRuntime { runtime };
    let installer_ctx = InstallContext {
        package_name: resolved.package_name.clone(),
        repo_path: repo_dir.clone(),
        package_dir: package_dir.clone(),
        manifest: manifest.clone(),
    };

    let installed_binary = if let Some(manifest) = &manifest {
        if let Some(release_binary) =
            try_download_release_binary(runtime, &resolved, manifest).await?
        {
            materialize_binary(&release_binary, &package_dir, &resolved.package_name)?
        } else {
            InstallerManager::new()
                .install(&installer_ctx, &runtime_driver)?
                .binary_path
        }
    } else {
        InstallerManager::new()
            .install(&installer_ctx, &runtime_driver)?
            .binary_path
    };

    if let Some(script) = manifest.as_ref().and_then(|m| m.postinstall.as_deref()) {
        run_script(script, &repo_dir)?;
    }

    if global {
        create_shim(runtime, &resolved.package_name, &installed_binary)?;
        if ensure_bin_on_path(runtime)? {
            println!(
                "added '{}' to PATH for global shims",
                runtime.paths.bin.display()
            );
        }
    }

    let binary_rel_path = installed_binary
        .strip_prefix(&package_dir)
        .unwrap_or(&installed_binary)
        .to_string_lossy()
        .to_string();

    let record = PackageRecord {
        package_name: resolved.package_name.clone(),
        owner: resolved.owner.clone(),
        repo: resolved.repo.clone(),
        version: version.map(|v| v.to_string()),
        binary_rel_path,
        global,
    };
    save_record(&package_dir, &record)?;

    Ok(record)
}

fn run_script(script: &str, repo_dir: &Path) -> Result<()> {
    run_shell_command(script, repo_dir)
}
