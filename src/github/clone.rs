use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use git2::{
    AutotagOption, FetchOptions, Oid, RemoteCallbacks, Repository, ResetType,
    build::CheckoutBuilder,
};

use crate::package::resolver::ResolvedRepo;

pub fn sync_repo(resolved: &ResolvedRepo, repo_dir: &Path, version: Option<&str>) -> Result<()> {
    if !repo_dir.exists() {
        clone_repo(&resolved.clone_url, repo_dir)?;
    }

    let operation = if let Some(reference) = version {
        checkout_version(repo_dir, reference)
    } else {
        sync_default_branch(repo_dir)
    };

    if let Err(err) = operation {
        eprintln!("sync failed for {}: {err}. recloning...", resolved.key);
        fs::remove_dir_all(repo_dir)
            .with_context(|| format!("failed to remove repo dir {}", repo_dir.display()))?;
        clone_repo(&resolved.clone_url, repo_dir)?;
        if let Some(reference) = version {
            checkout_version(repo_dir, reference)?;
        } else {
            sync_default_branch(repo_dir)?;
        }
    }

    Ok(())
}

fn clone_repo(clone_url: &str, repo_dir: &Path) -> Result<()> {
    if let Some(parent) = repo_dir.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Repository::clone(clone_url, repo_dir)
        .with_context(|| format!("failed to clone {clone_url} into {}", repo_dir.display()))?;
    Ok(())
}

fn sync_default_branch(repo_dir: &Path) -> Result<()> {
    let repo = Repository::open(repo_dir)
        .with_context(|| format!("failed to open repository {}", repo_dir.display()))?;
    fetch_all(&repo)?;

    let default_branch = resolve_default_branch(&repo)?;
    let local_ref = format!("refs/heads/{default_branch}");
    let remote_ref = format!("refs/remotes/origin/{default_branch}");
    let remote_oid = repo
        .refname_to_id(&remote_ref)
        .with_context(|| format!("failed to resolve {remote_ref}"))?;
    let commit = repo.find_commit(remote_oid)?;

    if let Ok(mut local) = repo.find_reference(&local_ref) {
        local
            .set_target(remote_oid, "Sync to origin default branch")
            .with_context(|| format!("failed to update {local_ref}"))?;
    } else {
        repo.reference(
            &local_ref,
            remote_oid,
            true,
            "Create local branch from origin",
        )
        .with_context(|| format!("failed to create {local_ref}"))?;
    }

    repo.set_head(&local_ref)
        .with_context(|| format!("failed to set HEAD to {local_ref}"))?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))
        .context("failed to checkout synced branch")?;
    repo.reset(commit.as_object(), ResetType::Hard, None)
        .context("failed to hard reset local branch to remote")?;
    Ok(())
}

pub fn checkout_version(repo_dir: &Path, version: &str) -> Result<()> {
    let repo = Repository::open(repo_dir)
        .with_context(|| format!("failed to open repository {}", repo_dir.display()))?;

    fetch_all(&repo)?;
    let (object, reference) = resolve_version(&repo, version)?;

    repo.checkout_tree(&object, Some(CheckoutBuilder::default().force()))
        .with_context(|| format!("failed to checkout {version}"))?;

    if let Some(reference) = reference {
        if let Some(name) = reference.name() {
            if name.starts_with("refs/remotes/origin/") {
                repo.set_head_detached(object.id())
                    .context("failed to detach HEAD")?;
            } else {
                repo.set_head(name)
                    .with_context(|| format!("failed to set HEAD to {name}"))?;
            }
        } else {
            repo.set_head_detached(object.id())
                .context("failed to detach HEAD")?;
        }
    } else {
        repo.set_head_detached(object.id())
            .context("failed to detach HEAD")?;
    }

    Ok(())
}

fn fetch_all(repo: &Repository) -> Result<()> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, _username, _allowed| git2::Cred::default());
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo.download_tags(AutotagOption::All);

    let mut remote = repo
        .find_remote("origin")
        .context("failed to access origin remote")?;
    remote
        .fetch(
            &[
                "refs/heads/*:refs/remotes/origin/*",
                "refs/tags/*:refs/tags/*",
            ],
            Some(&mut fo),
            None,
        )
        .context("failed to fetch refs from origin")?;
    Ok(())
}

fn resolve_default_branch(repo: &Repository) -> Result<String> {
    if let Ok(head) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Some(target) = head.symbolic_target() {
            let branch = target
                .trim_start_matches("refs/remotes/origin/")
                .to_string();
            if !branch.is_empty() {
                return Ok(branch);
            }
        }
    }

    let remote = repo
        .find_remote("origin")
        .context("failed to access origin remote")?;
    let default = remote
        .default_branch()
        .context("failed to resolve origin default branch")?;
    let default = default
        .as_str()
        .context("origin default branch is not valid utf-8")?;
    let branch = default.trim_start_matches("refs/heads/").to_string();
    if branch.is_empty() {
        bail!("origin default branch is empty");
    }
    Ok(branch)
}

fn resolve_version<'repo>(
    repo: &'repo Repository,
    version: &str,
) -> Result<(git2::Object<'repo>, Option<git2::Reference<'repo>>)> {
    if let Ok(parsed) = repo.revparse_ext(version) {
        return Ok(parsed);
    }

    for candidate in [
        format!("refs/tags/{version}"),
        format!("refs/heads/{version}"),
        format!("refs/remotes/origin/{version}"),
    ] {
        if let Ok(parsed) = repo.revparse_ext(&candidate) {
            return Ok(parsed);
        }
    }

    if let Ok(oid) = Oid::from_str(version) {
        let object = repo
            .find_object(oid, None)
            .with_context(|| format!("failed to find commit {version}"))?;
        return Ok((object, None));
    }

    bail!("unable to resolve version/commit '{version}'")
}
