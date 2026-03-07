use std::path::PathBuf;

use anyhow::Result;

use crate::{config::RuntimeContext, package::manifest::Manifest, package::resolver::ResolvedRepo};

pub async fn try_download_release_binary(
    _runtime: &RuntimeContext,
    _resolved: &ResolvedRepo,
    _manifest: &Manifest,
) -> Result<Option<PathBuf>> {
    Ok(None)
}
