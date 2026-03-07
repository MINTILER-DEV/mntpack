use anyhow::{bail, Result};

use crate::config::RuntimeContext;

pub async fn execute(_runtime: &RuntimeContext) -> Result<()> {
    bail!("update command not implemented yet")
}
