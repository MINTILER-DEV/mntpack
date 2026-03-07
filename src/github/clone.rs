use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use git2::{
    AnnotatedCommit, AutotagOption, FetchOptions, Oid, RemoteCallbacks, Repository,
    build::CheckoutBuilder,
};

use crate::package::resolver::ResolvedRepo;

pub fn sync_repo(resolved: &ResolvedRepo, repo_dir: &Path, version: Option<&str>) -> Result<()> {
    if repo_dir.exists() {
        if let Err(err) = pull_repo(repo_dir) {
            eprintln!("pull failed for {}: {err}. recloning...", resolved.key);
            fs::remove_dir_all(repo_dir)
                .with_context(|| format!("failed to remove repo dir {}", repo_dir.display()))?;
            clone_repo(&resolved.clone_url, repo_dir)?;
        }
    } else {
        clone_repo(&resolved.clone_url, repo_dir)?;
    }

    if let Some(reference) = version {
        checkout_version(repo_dir, reference)?;
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

fn pull_repo(repo_dir: &Path) -> Result<()> {
    let repo = Repository::open(repo_dir)
        .with_context(|| format!("failed to open repository {}", repo_dir.display()))?;

    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, _username, _allowed| git2::Cred::default());
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo.download_tags(AutotagOption::All);

    let mut remote = repo
        .find_remote("origin")
        .context("failed to access origin remote")?;
    remote
        .fetch(&["refs/heads/*:refs/remotes/origin/*"], Some(&mut fo), None)
        .context("failed to fetch origin")?;

    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .context("failed to locate FETCH_HEAD")?;
    let fetch_commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .context("failed to interpret FETCH_HEAD commit")?;

    let analysis = repo
        .merge_analysis(&[&fetch_commit])
        .context("failed to compute merge analysis")?
        .0;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        fast_forward(&repo, &fetch_commit)?;
        return Ok(());
    }

    bail!("repository requires non fast-forward merge")
}

fn fast_forward(repo: &Repository, fetch_commit: &AnnotatedCommit<'_>) -> Result<()> {
    let head = repo.head().context("failed to resolve repository HEAD")?;
    let branch_ref = head
        .name()
        .context("HEAD does not resolve to a branch")?
        .to_string();

    let mut target_ref = repo
        .find_reference(&branch_ref)
        .with_context(|| format!("failed to find branch reference {branch_ref}"))?;
    target_ref
        .set_target(fetch_commit.id(), "Fast-Forward")
        .context("failed to fast-forward branch")?;

    repo.set_head(&branch_ref)
        .with_context(|| format!("failed to set HEAD to {branch_ref}"))?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))
        .context("failed to checkout updated HEAD")?;
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
