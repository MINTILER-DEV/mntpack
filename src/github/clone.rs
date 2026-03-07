use std::path::Path;

use anyhow::{bail, Result};

use crate::package::resolver::ResolvedRepo;

pub fn sync_repo(_resolved: &ResolvedRepo, _repo_dir: &Path, _version: Option<&str>) -> Result<()> {
    bail!("github sync not implemented yet")
}
