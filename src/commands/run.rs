use std::{collections::HashSet, process::Command};

use anyhow::{Context, Result, bail};

use crate::{config::RuntimeContext, package::record::load_record};

pub async fn execute(runtime: &RuntimeContext, package_name: &str, args: &[String]) -> Result<()> {
    let package_dir = runtime.paths.package_dir(package_name);
    if !package_dir.exists() {
        bail!("package '{package_name}' is not installed");
    }

    if runtime.config.auto_update_on_run {
        if let Some(record) = load_record(&package_dir)? {
            let mut visited = HashSet::new();
            crate::commands::sync::sync_package_internal(
                runtime,
                &record.repo_spec(),
                record.version.as_deref(),
                Some(&record.package_name),
                record.global,
                &mut visited,
            )
            .await?;
        }
    }

    let binary_path = if let Some(record) = load_record(&package_dir)? {
        package_dir.join(record.binary_rel_path)
    } else if cfg!(windows) {
        package_dir.join(format!("{package_name}.exe"))
    } else {
        package_dir.join(package_name)
    };

    if !binary_path.exists() {
        bail!(
            "package binary for '{package_name}' not found at {}",
            binary_path.display()
        );
    }

    let status = Command::new(&binary_path)
        .args(args)
        .status()
        .with_context(|| format!("failed to launch {}", binary_path.display()))?;
    if !status.success() {
        bail!(
            "package '{}' exited with status {:?}",
            package_name,
            status.code()
        );
    }
    Ok(())
}
