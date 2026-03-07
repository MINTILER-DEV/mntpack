use anyhow::{bail, Result};

use crate::config::{normalize_repo_url, package_name_from_repo, repo_key};

#[derive(Debug, Clone)]
pub struct ResolvedRepo {
    pub owner: String,
    pub repo: String,
    pub clone_url: String,
    pub key: String,
    pub package_name: String,
}

pub fn resolve_repo(input: &str, default_owner: &str) -> Result<ResolvedRepo> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("repository cannot be empty");
    }

    if trimmed.contains("://") {
        return resolve_url(trimmed);
    }

    if trimmed.contains('/') {
        let mut parts = trimmed.splitn(2, '/');
        let owner = parts.next().unwrap_or_default().trim();
        let repo = parts.next().unwrap_or_default().trim().trim_end_matches(".git");
        if owner.is_empty() || repo.is_empty() {
            bail!("invalid repository shorthand: {trimmed}");
        }
        return Ok(from_owner_repo(owner, repo));
    }

    Ok(from_owner_repo(default_owner, trimmed.trim_end_matches(".git")))
}

fn resolve_url(url: &str) -> Result<ResolvedRepo> {
    let normalized = normalize_repo_url(url.trim_end_matches('/'));
    let marker = "github.com/";
    let Some(idx) = normalized.find(marker) else {
        bail!("only github repositories are currently supported");
    };
    let suffix = &normalized[idx + marker.len()..];
    let mut parts = suffix.trim_end_matches(".git").split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if owner.is_empty() || repo.is_empty() {
        bail!("unable to parse owner/repo from url: {url}");
    }
    Ok(from_owner_repo(owner, repo))
}

fn from_owner_repo(owner: &str, repo: &str) -> ResolvedRepo {
    let owner = owner.to_string();
    let repo = repo.to_string();
    let clone_url = format!("https://github.com/{owner}/{repo}.git");
    let key = repo_key(&owner, &repo);
    let package_name = package_name_from_repo(&owner, &repo);

    ResolvedRepo {
        owner,
        repo,
        clone_url,
        key,
        package_name,
    }
}
