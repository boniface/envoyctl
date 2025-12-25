use crate::model::*;
use anyhow::{Context, Result};
use std::{fs, path::{Path, PathBuf}};
use walkdir::WalkDir;

pub struct Loaded {
    pub admin: AdminSpec,
    pub defaults: DefaultsSpec,
    pub access_log: AccessLogSpec,
    pub runtime: RuntimeSpec,
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
    let upstreams = read_dir_yaml::<UpstreamSpec>(&config_dir.join("upstreams"))?;

    Ok(Loaded { admin, defaults, access_log, runtime, domains, upstreams, policies })
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
        if !e.file_type().is_file() { continue; }
        let p = e.path();
        if !matches!(p.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")) { continue; }
        let s = std::fs::read_to_string(p)?;
        let v = serde_yaml::from_str(&s).with_context(|| format!("parse {}", p.display()))?;
        out.push(v);
    }
    Ok(out)
}
