use anyhow::{bail, Result};

use crate::config::RuntimeContext;

pub fn execute(_runtime: &RuntimeContext, _package_name: &str, _args: &[String]) -> Result<()> {
    bail!("run command not implemented yet")
}
