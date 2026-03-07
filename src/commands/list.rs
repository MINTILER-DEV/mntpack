use anyhow::Result;

use crate::{config::RuntimeContext, package::record::load_all_records};

pub fn execute(runtime: &RuntimeContext) -> Result<()> {
    let records = load_all_records(&runtime.paths.packages)?;
    if records.is_empty() {
        println!("no packages installed");
        return Ok(());
    }

    for record in records {
        let version = record.version.as_deref().unwrap_or("latest");
        let scope = if record.global { "global" } else { "local" };
        println!(
            "{}\t{}\t{}\t{}",
            record.package_name,
            version,
            scope,
            record.repo_spec()
        );
    }

    Ok(())
}
