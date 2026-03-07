use std::collections::HashSet;

use anyhow::Result;

use crate::{config::RuntimeContext, package::record::load_all_records};

pub async fn execute(runtime: &RuntimeContext) -> Result<()> {
    let records = load_all_records(&runtime.paths.packages)?;
    if records.is_empty() {
        println!("no installed packages to update");
        return Ok(());
    }

    let mut visited = HashSet::new();
    for record in &records {
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

    println!("updated {} package(s)", records.len());
    Ok(())
}
