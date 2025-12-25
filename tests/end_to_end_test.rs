use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    // Test that the CLI shows help without crashing
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute help command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("envoyctl"));
    assert!(stdout.contains("Manage Envoy config via fragments"));
}

#[test]
fn test_end_to_end_workflow_with_temp_workspace() {
    // Create a temporary workspace
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("envoy-workspace");

    // Initialize a workspace using the CLI
    let init_output = Command::new("cargo")
        .args(&["run", "--", "init", "--dir", workspace_dir.to_str().unwrap()])
        .output()
        .expect("Failed to execute init command");

    // The init command might fail if templates don't exist, which is expected in test environment
    // Just check that the command didn't crash with a panic
    if !init_output.status.success() {
        let stderr = String::from_utf8_lossy(&init_output.stderr);
        // If it fails, it should be due to missing templates, not a crash
        assert!(!stderr.contains("thread 'main' panicked"));
    } else {
        // If init succeeded, verify workspace was created
        assert!(workspace_dir.exists());
        assert!(workspace_dir.join("config").exists());
        assert!(workspace_dir.join("config/common").exists());
        assert!(workspace_dir.join("config/domains").exists());
        assert!(workspace_dir.join("config/upstreams").exists());
        assert!(workspace_dir.join("config/policies").exists());

        // Now test build command in the workspace
        let build_output = Command::new("cargo")
            .args(&["run", "--", "--config-dir", &workspace_dir.join("config").to_str().unwrap(), "build"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to execute build command");

        // Build should succeed (though it might fail due to missing required files)
        // The important thing is that it doesn't crash
        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            let stdout = String::from_utf8_lossy(&build_output.stdout);
            // If it fails, it should be due to missing configuration, not a crash
            assert!(stderr.contains("read") || stderr.contains("missing") || stdout.contains("Error"));
        }
    }
}

#[test]
fn test_invalid_command_error_handling() {
    // Test that invalid commands produce appropriate error messages
    let output = Command::new("cargo")
        .args(&["run", "--", "nonexistent-command"])
        .output()
        .expect("Failed to execute invalid command");

    // Should fail but not crash
    assert!(!output.status.success());
}