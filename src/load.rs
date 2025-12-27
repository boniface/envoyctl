use crate::model::*;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub struct Loaded {
    pub admin: AdminSpec,
    pub defaults: DefaultsSpec,
    pub access_log: AccessLogSpec,
    pub validate: ValidateSpec,
    pub domains: Vec<DomainSpec>,
    pub upstreams: Vec<UpstreamSpec>,
    pub policies: PoliciesSpec,
}

pub fn load_all(config_dir: &Path) -> Result<Loaded> {
    let admin: AdminSpec = read_yaml(config_dir.join("common/admin.yaml"))?;
    let defaults: DefaultsSpec = read_yaml(config_dir.join("common/defaults.yaml"))?;
    let access_log: AccessLogSpec = read_yaml(config_dir.join("common/access_log.yaml"))?;
    let runtime: RuntimeSpec = read_yaml(config_dir.join("common/runtime.yaml"))?;
    let policies: PoliciesSpec = read_yaml(config_dir.join("policies/ratelimits.yaml"))?;

    let domains = read_dir_yaml::<DomainSpec>(&config_dir.join("domains"))?;

    // Load upstreams from upstreams/ directory (strict - will error on parse failures)
    let mut upstreams = read_dir_yaml::<UpstreamSpec>(&config_dir.join("upstreams"))?;
    // Also try to load upstreams from common/ (lenient - skips files that don't match)
    let common_upstreams = try_read_dir_yaml::<UpstreamSpec>(&config_dir.join("common"));
    upstreams.extend(common_upstreams);

    Ok(Loaded {
        admin,
        defaults,
        access_log,
        validate: runtime.validate, // Extract validate from runtime
        domains,
        upstreams,
        policies,
    })
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T> {
    let s = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let v = serde_yaml::from_str(&s).with_context(|| format!("parse {}", path.display()))?;
    Ok(v)
}

fn read_dir_yaml<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for e in WalkDir::new(dir).min_depth(1).max_depth(1) {
        let e = e?;
        if !e.file_type().is_file() {
            continue;
        }
        let p = e.path();
        if !matches!(p.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")) {
            continue;
        }
        let s = std::fs::read_to_string(p)?;
        let v = serde_yaml::from_str(&s).with_context(|| format!("parse {}", p.display()))?;
        out.push(v);
    }
    Ok(out)
}

/// Try to read YAML files from a directory, silently skipping files that don't match the target type.
/// Useful for loading upstreams from common/ where other config files also exist.
fn try_read_dir_yaml<T: serde::de::DeserializeOwned>(dir: &Path) -> Vec<T> {
    let mut out = Vec::new();
    if !dir.exists() {
        return out;
    }
    for e in WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .flatten()
    {
        if !e.file_type().is_file() {
            continue;
        }
        let p = e.path();
        if !matches!(p.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")) {
            continue;
        }
        if let Ok(s) = std::fs::read_to_string(p) {
            if let Ok(v) = serde_yaml::from_str(&s) {
                out.push(v);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_yaml_success() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_file = temp_dir.path().join("test.yaml");

        let yaml_content = r#"
address: "127.0.0.1"
port: 9000
"#;
        fs::write(&yaml_file, yaml_content).unwrap();

        let result: AdminSpec = read_yaml(yaml_file).unwrap();
        assert_eq!(result.address, "127.0.0.1");
        assert_eq!(result.port, 9000);
    }

    #[test]
    fn test_read_yaml_file_not_found() {
        let non_existent_file = PathBuf::from("/non/existent/file.yaml");
        let result: Result<AdminSpec, _> = read_yaml(non_existent_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_dir_yaml_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_dir = temp_dir.path().join("yaml_dir");
        fs::create_dir(&yaml_dir).unwrap();

        let result: Result<Vec<AdminSpec>, _> = read_dir_yaml(&yaml_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_read_dir_yaml_with_yaml_files() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_dir = temp_dir.path().join("yaml_dir");
        fs::create_dir(&yaml_dir).unwrap();

        // Create test YAML files
        fs::write(
            yaml_dir.join("file1.yaml"),
            "address: \"127.0.0.1\"\nport: 9000",
        )
        .unwrap();
        fs::write(
            yaml_dir.join("file2.yaml"),
            "address: \"192.168.1.1\"\nport: 8080",
        )
        .unwrap();

        let result: Result<Vec<AdminSpec>, _> = read_dir_yaml(&yaml_dir);
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert_eq!(specs.len(), 2);

        // Check that we can access the data
        assert!(specs
            .iter()
            .any(|s| s.address == "127.0.0.1" && s.port == 9000));
        assert!(specs
            .iter()
            .any(|s| s.address == "192.168.1.1" && s.port == 8080));
    }

    #[test]
    fn test_read_dir_yaml_with_mixed_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_dir = temp_dir.path().join("yaml_dir");
        fs::create_dir(&yaml_dir).unwrap();

        // Create YAML and non-YAML files
        fs::write(
            yaml_dir.join("file1.yaml"),
            "address: \"127.0.0.1\"\nport: 9000",
        )
        .unwrap();
        fs::write(
            yaml_dir.join("file2.yml"),
            "address: \"192.168.1.1\"\nport: 8080",
        )
        .unwrap();
        fs::write(yaml_dir.join("file3.txt"), "this should be ignored").unwrap();
        fs::write(yaml_dir.join("file4.json"), "this should be ignored too").unwrap();

        let result: Result<Vec<AdminSpec>, _> = read_dir_yaml(&yaml_dir);
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert_eq!(specs.len(), 2); // Only .yaml and .yml files should be processed

        // Check that we can access the data
        assert!(specs
            .iter()
            .any(|s| s.address == "127.0.0.1" && s.port == 9000));
        assert!(specs
            .iter()
            .any(|s| s.address == "192.168.1.1" && s.port == 8080));
    }

    #[test]
    fn test_load_all_with_valid_structure() {
        // Create a temporary directory structure that mimics the expected config layout
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("config");
        fs::create_dir_all(config_dir.join("common")).unwrap();
        fs::create_dir_all(config_dir.join("domains")).unwrap();
        fs::create_dir_all(config_dir.join("upstreams")).unwrap();
        fs::create_dir_all(config_dir.join("policies")).unwrap();

        // Write common config files
        fs::write(
            config_dir.join("common/admin.yaml"),
            "address: \"0.0.0.0\"\nport: 9901",
        )
        .unwrap();
        fs::write(
            config_dir.join("common/defaults.yaml"),
            "route_timeout: \"60s\"",
        )
        .unwrap();
        fs::write(
            config_dir.join("common/access_log.yaml"),
            "type: \"stdout\"\npath: \"/dev/stdout\"",
        )
        .unwrap();
        fs::write(
            config_dir.join("common/runtime.yaml"),
            r#"validate:
  type: "native"
  user: "envoy"
  bin: "envoy"
  config_path: "/etc/envoy/envoy.yaml"
"#,
        )
        .unwrap();

        // Write policies file
        fs::write(config_dir.join("policies/ratelimits.yaml"), "").unwrap();

        let result = load_all(&config_dir);
        assert!(result.is_ok());

        let loaded = result.unwrap();
        assert_eq!(loaded.admin.address, "0.0.0.0");
        assert_eq!(loaded.admin.port, 9901);
        assert_eq!(loaded.defaults.route_timeout, "60s");
        assert_eq!(loaded.access_log.r#type, "stdout");
        assert_eq!(loaded.access_log.path, "/dev/stdout");
        assert_eq!(loaded.domains.len(), 0); // No domain files
        assert_eq!(loaded.upstreams.len(), 0); // No upstream files
    }

    #[test]
    fn test_load_all_missing_common_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("config");
        // Don't create the common directory

        let result = load_all(&config_dir);
        assert!(result.is_err()); // Should fail because common files are missing
    }
}
