use anyhow::{bail, Result};

use crate::config::RuntimeContext;

pub async fn execute(
    _runtime: &RuntimeContext,
    _repo_input: &str,
    _version: Option<&str>,
    _global: bool,
) -> Result<()> {
    bail!("sync command not implemented yet")
}
