use crate::model::*;
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

pub fn validate_model(domains: &[DomainSpec], upstreams: &[UpstreamSpec], policies: &PoliciesSpec, defaults: &DefaultsSpec) -> Result<()> {
    let mut seen_domains = HashSet::new();
    for d in domains {
        if !seen_domains.insert(d.domain.as_str()) {
            bail!("duplicate domain: {}", d.domain);
        }

        if d.mode == "terminate_https_443" && d.tls.is_none() {
            bail!("domain {} mode terminate_https_443 requires tls block", d.domain);
        }
        if d.mode != "terminate_https_443" && d.mode != "passthrough_https_443" {
            bail!("domain {} has unsupported mode: {}", d.domain, d.mode);
        }
    }

    let upstream_map: HashMap<_, _> = upstreams.iter().map(|u| (u.name.as_str(), u)).collect();

    if !upstream_map.contains_key(defaults.http_default_upstream.as_str()) {
        bail!("defaults.http_default_upstream '{}' does not exist in upstreams/", defaults.http_default_upstream);
    }
    if !upstream_map.contains_key(defaults.tls_passthrough_upstream.as_str()) {
        bail!("defaults.tls_passthrough_upstream '{}' does not exist in upstreams/", defaults.tls_passthrough_upstream);
    }

    for u in upstreams {
        if u.endpoints.is_empty() {
            bail!("upstream {} has no endpoints", u.name);
        }
    }

    for d in domains {
        for r in &d.routes {
            if !upstream_map.contains_key(r.to_upstream.as_str()) {
                bail!("domain {} route references unknown upstream {}", d.domain, r.to_upstream);
            }
            if let Some(pfc) = &r.per_filter_config {
                if let Some(key) = &pfc.local_ratelimit {
                    if !policies.local_ratelimits.contains_key(key) {
                        bail!("domain {} route references unknown local_ratelimit policy {}", d.domain, key);
                    }
                }
            }
        }
    }

    Ok(())
}
