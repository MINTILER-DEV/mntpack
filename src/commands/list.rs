use anyhow::{bail, Result};

use crate::config::RuntimeContext;

pub fn execute(_runtime: &RuntimeContext) -> Result<()> {
    bail!("list command not implemented yet")
}
