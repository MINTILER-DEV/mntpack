use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::{
    config::RuntimeContext,
    github::clone::sync_repo,
    github::release::try_download_release_binary_from_tags,
    package::{
        resolver::resolve_repo,
        store::{first_file_in_dir, normalize_hash, sha256_file},
    },
};

pub fn enabled(runtime: &RuntimeContext) -> bool {
    runtime.config.binary_cache.enabled
}

pub fn configured(runtime: &RuntimeContext) -> bool {
    enabled(runtime) && effective_cache_repo(runtime).is_some()
}

pub fn try_download_cached_binary(
    runtime: &RuntimeContext,
    package_repo_name: &str,
    hash: &str,
) -> Result<Option<PathBuf>> {
    let Some(cache_checkout) = ensure_cache_checkout(runtime)? else {
        return Ok(None);
    };
    let hash = normalize_hash(hash);
    let package_dir = cache_checkout.join(package_repo_name).join(&hash);
    if !package_dir.exists() {
        return Ok(None);
    }
    let Some(source) = first_file_in_dir(&package_dir) else {
        return Ok(None);
    };

    let actual = sha256_file(&source)?;
    if normalize_hash(&actual) != hash {
        bail!(
            "binary cache hash mismatch for {}: expected {}, got {}",
            source.display(),
            hash,
            actual
        );
    }

    let out_dir = runtime.paths.cache.join("binary-cache-download");
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("binary");
    let destination = out_dir.join(file_name);
    fs::copy(&source, &destination).with_context(|| {
        format!(
            "failed to copy cached binary {} -> {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(Some(destination))
}

pub async fn try_download_cached_release_binary(
    runtime: &RuntimeContext,
    package_repo_name: &str,
    requested_version: Option<&str>,
    commit: Option<&str>,
) -> Result<Option<PathBuf>> {
    if !enabled(runtime) {
        return Ok(None);
    }

    let Some(cache_repo_spec) = effective_cache_repo(runtime) else {
        return Ok(None);
    };
    let cache_repo = resolve_repo(&cache_repo_spec, &runtime.config.default_owner)?;
    let tags = build_release_tag_candidates(package_repo_name, requested_version, commit);
    if tags.is_empty() {
        return Ok(None);
    }

    try_download_release_binary_from_tags(runtime, &cache_repo, &tags).await
}

pub fn upload_binary_to_cache(
    runtime: &RuntimeContext,
    package_repo_name: &str,
    hash: &str,
    binary_path: &Path,
) -> Result<()> {
    let Some(cache_checkout) = ensure_cache_checkout(runtime)? else {
        bail!("binary cache is not configured");
    };
    let hash = normalize_hash(hash);
    let target_dir = cache_checkout.join(package_repo_name).join(&hash);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;

    let file_name = binary_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("binary");
    let target_file = target_dir.join(file_name);
    if !target_file.exists() {
        fs::copy(binary_path, &target_file).with_context(|| {
            format!(
                "failed to copy {} -> {}",
                binary_path.display(),
                target_file.display()
            )
        })?;
    }

    run_git(
        runtime,
        &cache_checkout,
        &["add", "."],
        "failed to stage binary cache updates",
    )?;

    let diff_status = Command::new(&runtime.config.paths.git)
        .arg("-C")
        .arg(&cache_checkout)
        .args(["diff", "--cached", "--quiet"])
        .status()
        .with_context(|| {
            format!(
                "failed to check staged changes in binary cache {}",
                cache_checkout.display()
            )
        })?;
    if diff_status.success() {
        return Ok(());
    }

    run_git(
        runtime,
        &cache_checkout,
        &[
            "commit",
            "-m",
            &format!("mntpack prebuild {} {}", package_repo_name, hash),
        ],
        "failed to commit binary cache update",
    )?;
    run_git(
        runtime,
        &cache_checkout,
        &["push", "origin", "HEAD"],
        "failed to push binary cache update",
    )?;

    Ok(())
}

fn ensure_cache_checkout(runtime: &RuntimeContext) -> Result<Option<PathBuf>> {
    if !configured(runtime) {
        return Ok(None);
    }
    let Some(repo_spec) = effective_cache_repo(runtime) else {
        return Ok(None);
    };
    let resolved = resolve_repo(&repo_spec, &runtime.config.default_owner)?;
    let checkout = runtime
        .paths
        .cache
        .join("binary-cache")
        .join(&resolved.owner)
        .join(&resolved.repo);
    sync_repo(
        &resolved,
        &checkout,
        &runtime.paths.cache_git,
        &runtime.config.paths.git,
        None,
    )?;
    Ok(Some(checkout))
}

fn effective_cache_repo(runtime: &RuntimeContext) -> Option<String> {
    let from_binary_cache = runtime
        .config
        .binary_cache
        .repo
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    if from_binary_cache.is_some() {
        return from_binary_cache;
    }
    if runtime.config.sync_dispatch.repo.trim().is_empty() {
        None
    } else {
        Some(runtime.config.sync_dispatch.repo.trim().to_string())
    }
}

fn build_release_tag_candidates(
    package_repo_name: &str,
    requested_version: Option<&str>,
    commit: Option<&str>,
) -> Vec<String> {
    let key_dash = package_repo_name.replace('/', "-");
    let key_dunder = package_repo_name.replace('/', "__");
    let key_sanitized = sanitize_tag_component(package_repo_name);
    let mut suffixes = BTreeSet::new();
    for value in [requested_version, commit]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        suffixes.insert(value.to_string());
        suffixes.insert(sanitize_tag_component(value));
    }

    let mut tags = BTreeSet::new();
    for suffix in suffixes {
        tags.insert(format!("{key_dash}-{suffix}"));
        tags.insert(format!("{key_dunder}-{suffix}"));
        tags.insert(format!("{key_sanitized}-{suffix}"));
    }
    tags.into_iter().collect()
}

fn sanitize_tag_component(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn run_git(runtime: &RuntimeContext, checkout: &Path, args: &[&str], context: &str) -> Result<()> {
    let status = Command::new(&runtime.config.paths.git)
        .arg("-C")
        .arg(checkout)
        .args(args)
        .status()
        .with_context(|| format!("{context} in {}", checkout.display()))?;
    if !status.success() {
        bail!(
            "{context}: git {} exited with status {:?}",
            args.join(" "),
            status.code()
        );
    }
    Ok(())
}
