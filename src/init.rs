use crate::cli::Cli;
use anyhow::{Context, Result};
use fs_extra::dir::{copy as copy_dir, CopyOptions};
use std::path::PathBuf;

pub fn cmd_init(_cli: &Cli, dir: PathBuf) -> Result<()> {
    // templates/workspace is packaged to /usr/share/envoyctl/templates/workspace
    // For local dev, we also support running from repo root where templates/ exists.
    let candidates = [
        PathBuf::from("/usr/share/envoyctl/templates/workspace"),
        PathBuf::from("templates/workspace"),
    ];

    let src = candidates.iter().find(|p| p.exists())
        .cloned()
        .context("could not find templates/workspace (neither /usr/share/... nor ./templates/...)")?;

    if dir.exists() {
        anyhow::bail!("target dir already exists: {}", dir.display());
    }

    std::fs::create_dir_all(&dir)?;

    let mut opts = CopyOptions::new();
    opts.copy_inside = true;
    opts.overwrite = false;

    copy_dir(&src, &dir, &opts).with_context(|| format!("copy {} -> {}", src.display(), dir.display()))?;

    println!("Workspace created at {}", dir.display());
    println!("Next:\n  cd {}\n  envoyctl validate --config-dir ./config --out-dir ./out\n", dir.display());
    Ok(())
}
