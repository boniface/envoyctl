use crate::cli::Cli;
use anyhow::{Context, Result};
use fs_extra::dir::{copy as copy_dir, CopyOptions};
use std::path::{Path, PathBuf};

pub fn cmd_init(_cli: &Cli, dir: &Path) -> Result<()> {
    // templates/workspace is packaged to /usr/share/envoyctl/templates/workspace
    // For local dev, we support multiple locations
    let mut candidates = vec![
        // System install location
        PathBuf::from("/usr/share/envoyctl/templates/workspace"),
        // Running from project root (cargo run from project dir)
        PathBuf::from("templates/workspace"),
    ];

    // Also check relative to the executable location (for development)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // When running via cargo run, executable is in target/debug/
            // So templates would be at ../../templates/workspace
            candidates.push(exe_dir.join("../../templates/workspace"));
            // Also try sibling directory
            candidates.push(exe_dir.join("../templates/workspace"));
        }
    }

    // Try CARGO_MANIFEST_DIR for development (set by cargo during build)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(PathBuf::from(manifest_dir).join("templates/workspace"));
    }

    let src = candidates.iter()
        .map(|p| p.canonicalize().unwrap_or(p.clone()))
        .find(|p| p.exists())
        .context(format!(
            "could not find templates/workspace. Searched:\n  {}",
            candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n  ")
        ))?;

    if dir.exists() {
        anyhow::bail!("target dir already exists: {}", dir.display());
    }

    std::fs::create_dir_all(&dir)?;

    // Copy contents of workspace folder, not the folder itself
    let mut opts = CopyOptions::new();
    opts.overwrite = false;
    opts.content_only = true;  // Copy only contents, not the source folder

    copy_dir(&src, &dir, &opts).with_context(|| format!("copy {} -> {}", src.display(), dir.display()))?;

    println!("Workspace created at {}", dir.display());
    println!("Next:\n  cd {}\n  envoyctl validate --config-dir ./config --out-dir ./out\n", dir.display());
    Ok(())
}
