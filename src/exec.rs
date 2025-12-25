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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_run_success() {
        // Test with a simple command that should succeed
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "exit 0"]); // Command that exits with success code 0
        let result = run(&mut cmd);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_failure() {
        // Test with a command that should fail
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "exit 1"]); // Command that exits with failure code 1
        let result = run(&mut cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_with_output() {
        // Test with a command that produces output
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo 'test output'"]);
        let result = run(&mut cmd);
        // This should succeed even though it produces output
        assert!(result.is_ok());
    }
}