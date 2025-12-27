//! Execute external commands (docker, envoy, etc.)

use anyhow::{bail, Result};
use std::process::Command;

/// Run a command and check for success
pub fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        bail!("command failed with exit code: {:?}", status.code());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_success() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        assert!(run(&mut cmd).is_ok());
    }

    #[test]
    fn test_run_failure() {
        let mut cmd = Command::new("false");
        assert!(run(&mut cmd).is_err());
    }
}
