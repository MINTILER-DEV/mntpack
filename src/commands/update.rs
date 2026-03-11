use std::collections::HashSet;

use anyhow::Result;

use crate::{
    config::RuntimeContext,
    package::lockfile::{regenerate_from_installed, save_to_cwd},
    package::record::{find_record_by_package_name, load_all_records},
    ui::progress::ProgressBar,
};

pub async fn execute(runtime: &RuntimeContext, package: Option<&str>) -> Result<()> {
    if let Some(package_name) = package {
        let mut progress = ProgressBar::new("update", 2);
        if let Some(record) = find_record_by_package_name(&runtime.paths.packages, package_name)? {
            let mut visited = HashSet::new();
            crate::commands::sync::sync_package_internal(
                runtime,
                &record.repo_spec(),
                record.version.as_deref(),
                None,
                Some(&record.package_name),
                record.global,
                &mut visited,
                false,
            )
            .await?;
            progress.advance(format!("synced {}", record.package_name));
            println!("updated {}", record.package_name);
            let lock = regenerate_from_installed(runtime)?;
            save_to_cwd(&lock)?;
            progress.finish("lockfile regenerated");
            return Ok(());
        }

        crate::commands::sync::execute(runtime, package_name, None, None, None, false).await?;
        progress.advance(format!("synced {package_name}"));
        let lock = regenerate_from_installed(runtime)?;
        save_to_cwd(&lock)?;
        progress.finish("lockfile regenerated");
        return Ok(());
    }

    let records = load_all_records(&runtime.paths.packages)?;
    if records.is_empty() {
        println!("no installed packages to update");
        return Ok(());
    }

    let mut progress = ProgressBar::new("update", records.len());
    let mut visited = HashSet::new();
    for record in &records {
        crate::commands::sync::sync_package_internal(
            runtime,
            &record.repo_spec(),
            record.version.as_deref(),
            None,
            Some(&record.package_name),
            record.global,
            &mut visited,
            false,
        )
        .await?;
        progress.advance(record.package_name.clone());
    }

    let lock = regenerate_from_installed(runtime)?;
    save_to_cwd(&lock)?;
    progress.finish(format!("{} package(s)", records.len()));
    println!("updated {} package(s)", records.len());
    Ok(())
}
