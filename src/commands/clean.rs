use std::{collections::HashSet, fs};

use anyhow::{Context, Result};

use crate::{config::RuntimeContext, package::record::load_all_records};

pub fn execute(runtime: &RuntimeContext, clean_unused_repos: bool) -> Result<()> {
    if runtime.paths.cache.exists() {
        fs::remove_dir_all(&runtime.paths.cache)
            .with_context(|| format!("failed to remove {}", runtime.paths.cache.display()))?;
    }
    fs::create_dir_all(&runtime.paths.cache)
        .with_context(|| format!("failed to create {}", runtime.paths.cache.display()))?;
    fs::create_dir_all(&runtime.paths.cache_git)
        .with_context(|| format!("failed to create {}", runtime.paths.cache_git.display()))?;
    fs::create_dir_all(&runtime.paths.cache_exec)
        .with_context(|| format!("failed to create {}", runtime.paths.cache_exec.display()))?;

    println!("cleared cache at {}", runtime.paths.cache.display());

    if clean_unused_repos {
        clean_repos(runtime)?;
    }

    Ok(())
}

fn clean_repos(runtime: &RuntimeContext) -> Result<()> {
    let records = load_all_records(&runtime.paths.packages)?;
    let used: HashSet<String> = records
        .iter()
        .map(|record| crate::config::repo_key(&record.owner, &record.repo))
        .collect();
    let used_legacy: HashSet<String> = records
        .iter()
        .map(|record| crate::config::repo_key_legacy(&record.owner, &record.repo))
        .collect();

    if !runtime.paths.repos.exists() {
        return Ok(());
    }

    let mut removed = 0usize;
    for owner_entry in fs::read_dir(&runtime.paths.repos)
        .with_context(|| format!("failed to read {}", runtime.paths.repos.display()))?
    {
        let owner_entry = owner_entry?;
        if !owner_entry.file_type()?.is_dir() {
            continue;
        }

        let owner_name = owner_entry.file_name().to_string_lossy().to_string();
        let owner_path = owner_entry.path();
        if owner_name.contains("__") {
            if !used.contains(&owner_name) && !used_legacy.contains(&owner_name) {
                fs::remove_dir_all(&owner_path)
                    .with_context(|| format!("failed to remove {}", owner_path.display()))?;
                removed += 1;
            }
            continue;
        }

        for repo_entry in fs::read_dir(&owner_path)
            .with_context(|| format!("failed to read {}", owner_path.display()))?
        {
            let repo_entry = repo_entry?;
            if !repo_entry.file_type()?.is_dir() {
                continue;
            }
            let repo_name = repo_entry.file_name().to_string_lossy().to_string();
            let key = format!("{owner_name}/{repo_name}");
            if used.contains(&key) {
                continue;
            }
            fs::remove_dir_all(repo_entry.path())
                .with_context(|| format!("failed to remove {}", repo_entry.path().display()))?;
            removed += 1;
        }

        let owner_empty = fs::read_dir(&owner_path)
            .with_context(|| format!("failed to read {}", owner_path.display()))?
            .next()
            .is_none();
        if owner_empty {
            fs::remove_dir_all(&owner_path)
                .with_context(|| format!("failed to remove {}", owner_path.display()))?;
        }
    }

    println!("removed {removed} unused repo clone(s)");
    Ok(())
}
