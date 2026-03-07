use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::config::RuntimeContext;

pub fn create_shim(runtime: &RuntimeContext, package_name: &str, binary_path: &Path) -> Result<()> {
    if cfg!(windows) {
        let shim_path = runtime.paths.bin.join(format!("{package_name}.cmd"));
        let content = format!("@echo off\r\n\"{}\" %*\r\n", binary_path.display());
        fs::write(&shim_path, content)
            .with_context(|| format!("failed to write shim {}", shim_path.display()))?;
        return Ok(());
    }

    let shim_path = runtime.paths.bin.join(package_name);
    let content = format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", binary_path.display());
    fs::write(&shim_path, content)
        .with_context(|| format!("failed to write shim {}", shim_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shim_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shim_path, perms)?;
    }

    Ok(())
}
