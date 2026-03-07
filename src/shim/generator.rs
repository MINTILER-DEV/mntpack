use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

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

pub fn ensure_bin_on_path(runtime: &RuntimeContext) -> Result<bool> {
    let bin_dir = runtime.paths.bin.clone();
    let current = env::var_os("PATH").unwrap_or_default();
    if path_contains(&current, &bin_dir) {
        return Ok(false);
    }

    let mut entries: Vec<PathBuf> = env::split_paths(&current).collect();
    entries.push(bin_dir.clone());
    let joined = env::join_paths(entries).context("failed to rebuild PATH variable")?;
    unsafe {
        env::set_var("PATH", &joined);
    }

    if cfg!(windows) {
        persist_windows_user_path(&bin_dir)?;
        let _ = refresh_windows_environment();
    } else {
        persist_bashrc_path(&bin_dir)?;
        let _ = source_bashrc();
    }

    Ok(true)
}

fn path_contains(path_value: &std::ffi::OsStr, needle: &Path) -> bool {
    env::split_paths(path_value).any(|entry| path_eq(&entry, needle))
}

fn path_eq(a: &Path, b: &Path) -> bool {
    let left = a
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    let right = b
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn persist_windows_user_path(bin_dir: &Path) -> Result<()> {
    let bin = bin_dir.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$target='{bin}';\
         $existing=[Environment]::GetEnvironmentVariable('Path','User');\
         $parts=@();\
         if ($existing) {{ $parts=$existing -split ';' }};\
         $exists=$false;\
         foreach ($p in $parts) {{\
           if ($p.TrimEnd('\\') -ieq $target.TrimEnd('\\')) {{ $exists=$true; break }}\
         }};\
         if (-not $exists) {{\
           $newPath = if ([string]::IsNullOrWhiteSpace($existing)) {{ $target }} else {{ \"$existing;$target\" }};\
           [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')\
         }}"
    );

    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .context("failed to persist user PATH on Windows")?;
    if !status.success() {
        anyhow::bail!("failed to persist PATH using powershell");
    }
    Ok(())
}

fn refresh_windows_environment() -> Result<()> {
    let script = r#"
$sig='[DllImport("user32.dll",SetLastError=true,CharSet=CharSet.Auto)] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);'
Add-Type -MemberDefinition $sig -Name NativeMethods -Namespace Win32 -ErrorAction SilentlyContinue | Out-Null
[UIntPtr]$out=[UIntPtr]::Zero
[Win32.NativeMethods]::SendMessageTimeout([IntPtr]0xffff,0x1A,[UIntPtr]::Zero,'Environment',2,5000,[ref]$out) | Out-Null
"#;
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .status()
        .context("failed to refresh windows environment")?;
    if !status.success() {
        anyhow::bail!("failed to refresh windows environment");
    }
    Ok(())
}

fn persist_bashrc_path(bin_dir: &Path) -> Result<()> {
    let home = dirs::home_dir().context("unable to locate home directory")?;
    let bashrc = home.join(".bashrc");
    let bin = bin_dir.to_string_lossy().replace('"', "\\\"");
    let line = format!("export PATH=\"{bin}:$PATH\"");
    let existing = if bashrc.exists() {
        fs::read_to_string(&bashrc)
            .with_context(|| format!("failed to read {}", bashrc.display()))?
    } else {
        String::new()
    };

    if existing.contains(".mntpack/bin") {
        return Ok(());
    }

    let mut new_content = existing;
    if !new_content.ends_with('\n') && !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str("# Added by mntpack\n");
    new_content.push_str(&line);
    new_content.push('\n');
    fs::write(&bashrc, new_content)
        .with_context(|| format!("failed to write {}", bashrc.display()))?;
    Ok(())
}

fn source_bashrc() -> Result<()> {
    let status = Command::new("bash")
        .args(["-lc", "source ~/.bashrc >/dev/null 2>&1 || true"])
        .status()
        .context("failed to source ~/.bashrc")?;
    if !status.success() {
        anyhow::bail!("failed to source ~/.bashrc");
    }
    Ok(())
}
