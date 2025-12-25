use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub fn run(cmd: &mut Command) -> Result<()> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().context("failed to run command")?;
    if !status.success() {
        anyhow::bail!("command failed with exit code: {}", status);
    }
    Ok(())
}
