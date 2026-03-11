use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use git2::{
    AutotagOption, FetchOptions, Oid, RemoteCallbacks, Repository, ResetType,
    build::CheckoutBuilder,
};

use crate::package::resolver::ResolvedRepo;

pub fn sync_repo(
    resolved: &ResolvedRepo,
    repo_dir: &Path,
    cache_git_dir: &Path,
    git_bin: &str,
    version: Option<&str>,
) -> Result<()> {
    let mirror_dir = ensure_bare_mirror(resolved, cache_git_dir)?;
    if !repo_dir.exists() {
        add_worktree(git_bin, &mirror_dir, repo_dir)?;
    } else if !is_linked_worktree(repo_dir) {
        eprintln!(
            "detected legacy full clone at {}. migrating to mirror-backed worktree...",
            repo_dir.display()
        );
        remove_checkout(git_bin, &mirror_dir, repo_dir)?;
        add_worktree(git_bin, &mirror_dir, repo_dir)?;
    } else {
        if let Err(err) = fetch_repo(repo_dir) {
            eprintln!(
                "detected broken linked worktree at {}: {err}. recreating checkout...",
                repo_dir.display()
            );
            remove_checkout(git_bin, &mirror_dir, repo_dir)?;
            add_worktree(git_bin, &mirror_dir, repo_dir)?;
        }
    }

    let operation = if let Some(reference) = version {
        checkout_version(repo_dir, reference)
    } else {
        sync_default_branch(repo_dir)
    };

    if let Err(err) = operation {
        eprintln!("sync failed for {}: {err}. recloning...", resolved.key);
        remove_checkout(git_bin, &mirror_dir, repo_dir)?;
        add_worktree(git_bin, &mirror_dir, repo_dir)?;
        if let Some(reference) = version {
            checkout_version(repo_dir, reference)?;
        } else {
            sync_default_branch(repo_dir)?;
        }
    }

    Ok(())
}

fn ensure_bare_mirror(resolved: &ResolvedRepo, cache_git_dir: &Path) -> Result<std::path::PathBuf> {
    fs::create_dir_all(cache_git_dir)
        .with_context(|| format!("failed to create {}", cache_git_dir.display()))?;
    let mirror_dir = cache_git_dir.join(format!("{}.git", resolved.key));

    if !mirror_dir.exists() {
        clone_bare_repo(&resolved.clone_url, &mirror_dir)?;
        return Ok(mirror_dir);
    }

    let mirror = Repository::open_bare(&mirror_dir)
        .with_context(|| format!("failed to open mirror {}", mirror_dir.display()))?;
    fetch_all(&mirror)?;
    Ok(mirror_dir)
}

fn clone_bare_repo(clone_url: &str, repo_dir: &Path) -> Result<()> {
    if let Some(parent) = repo_dir.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.clone(clone_url, repo_dir).with_context(|| {
        format!(
            "failed to create mirror {clone_url} into {}",
            repo_dir.display()
        )
    })?;
    Ok(())
}

fn add_worktree(git_bin: &str, mirror_dir: &Path, repo_dir: &Path) -> Result<()> {
    if let Some(parent) = repo_dir.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    if repo_dir.exists() {
        remove_checkout(git_bin, mirror_dir, repo_dir)?;
    }

    let mirror_repo = Repository::open_bare(mirror_dir)
        .with_context(|| format!("failed to open mirror {}", mirror_dir.display()))?;
    let default_branch = resolve_default_branch(&mirror_repo)?;
    let git_dir_arg = format!("--git-dir={}", mirror_dir.display());
    let repo_dir_str = repo_dir.to_string_lossy().to_string();
    let status = Command::new(git_bin)
        .args([
            git_dir_arg.as_str(),
            "worktree",
            "add",
            "--force",
            repo_dir_str.as_str(),
            default_branch.as_str(),
        ])
        .status()
        .with_context(|| {
            format!(
                "failed to run '{} worktree add' for {}",
                git_bin,
                repo_dir.display()
            )
        })?;
    if !status.success() {
        bail!(
            "failed to add git worktree at {} (exit code {:?})",
            repo_dir.display(),
            status.code()
        );
    }
    Ok(())
}

fn remove_checkout(git_bin: &str, mirror_dir: &Path, repo_dir: &Path) -> Result<()> {
    if repo_dir.exists()
        && is_linked_worktree(repo_dir)
        && worktree_is_registered(git_bin, mirror_dir, repo_dir)?
    {
        let git_dir_arg = format!("--git-dir={}", mirror_dir.display());
        let repo_dir_str = repo_dir.to_string_lossy().to_string();
        let remove_output = Command::new(git_bin)
            .args([
                git_dir_arg.as_str(),
                "worktree",
                "remove",
                "--force",
                repo_dir_str.as_str(),
            ])
            .output();
        if let Ok(output) = remove_output {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !stderr.is_empty() {
                    eprintln!(
                        "warning: failed to unregister worktree '{}': {}",
                        repo_dir.display(),
                        stderr
                    );
                }
            }
        }
    }

    if repo_dir.exists() {
        fs::remove_dir_all(repo_dir)
            .with_context(|| format!("failed to remove repo dir {}", repo_dir.display()))?;
    }

    let git_dir_arg = format!("--git-dir={}", mirror_dir.display());
    let _ = Command::new(git_bin)
        .args([git_dir_arg.as_str(), "worktree", "prune", "--expire", "now"])
        .output();
    Ok(())
}

fn worktree_is_registered(git_bin: &str, mirror_dir: &Path, repo_dir: &Path) -> Result<bool> {
    let git_dir_arg = format!("--git-dir={}", mirror_dir.display());
    let output = Command::new(git_bin)
        .args([git_dir_arg.as_str(), "worktree", "list", "--porcelain"])
        .output()
        .with_context(|| {
            format!(
                "failed to list worktrees for mirror {}",
                mirror_dir.display()
            )
        })?;
    if !output.status.success() {
        return Ok(false);
    }

    let target = normalize_path_for_compare(repo_dir);
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            let current = normalize_path_for_compare(Path::new(path));
            if current == target {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn normalize_path_for_compare(path: &Path) -> String {
    let candidate: PathBuf = path
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path.to_string_lossy().to_string()));
    let mut normalized = candidate.to_string_lossy().replace('/', "\\");
    normalized = normalized.trim_end_matches('\\').to_string();
    if cfg!(windows) {
        normalized = normalized.to_ascii_lowercase();
    }
    normalized
}

fn is_linked_worktree(repo_dir: &Path) -> bool {
    repo_dir.join(".git").is_file()
}

fn sync_default_branch(repo_dir: &Path) -> Result<()> {
    let repo = open_repo(repo_dir)?;
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
    let repo = open_repo(repo_dir)?;

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

pub fn fetch_repo(repo_dir: &Path) -> Result<()> {
    let repo = open_repo(repo_dir)?;
    fetch_all(&repo)
}

pub fn head_commit(repo_dir: &Path) -> Result<String> {
    let repo = open_repo(repo_dir)?;
    let oid = repo
        .head()
        .context("failed to read HEAD")?
        .target()
        .context("failed to resolve HEAD target")?;
    Ok(oid.to_string())
}

pub fn head_commit_short(repo_dir: &Path) -> Result<String> {
    let commit = head_commit(repo_dir)?;
    Ok(short_commit(&commit))
}

pub fn default_remote_commit(repo_dir: &Path) -> Result<String> {
    let repo = open_repo(repo_dir)?;
    fetch_all(&repo)?;
    let branch = resolve_default_branch(&repo)?;
    let remote_ref = format!("refs/remotes/origin/{branch}");
    let oid = repo
        .refname_to_id(&remote_ref)
        .with_context(|| format!("failed to resolve {remote_ref}"))?;
    Ok(oid.to_string())
}

pub fn default_remote_commit_short(repo_dir: &Path) -> Result<String> {
    let commit = default_remote_commit(repo_dir)?;
    Ok(short_commit(&commit))
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

fn open_repo(repo_dir: &Path) -> Result<Repository> {
    Repository::open(repo_dir)
        .with_context(|| format!("failed to open repository {}", repo_dir.display()))
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

fn short_commit(commit: &str) -> String {
    commit.chars().take(7).collect()
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
