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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_model_success() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert".to_string(),
                    private_key: "/path/to/key".to_string(),
                }),
                routes: vec![
                    RouteSpec {
                        m: MatchSpec::Prefix("/api".to_string()),
                        to_upstream: "api_backend".to_string(),
                        timeout: Some("30s".to_string()),
                        per_filter_config: None,
                    }
                ],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "api_backend".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 8080,
                }],
            },
            UpstreamSpec {
                name: "cilium_http".to_string(),  // default http upstream
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),  // default tls upstream
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_model_duplicate_domain() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert".to_string(),
                    private_key: "/path/to/key".to_string(),
                }),
                routes: vec![],
            },
            DomainSpec {
                domain: "example.com".to_string(),  // duplicate
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert2".to_string(),
                    private_key: "/path/to/key2".to_string(),
                }),
                routes: vec![],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate domain"));
    }

    #[test]
    fn test_validate_model_terminate_without_tls() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),  // requires TLS
                tls: None,  // but no TLS provided
                routes: vec![],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires tls block"));
    }

    #[test]
    fn test_validate_model_unsupported_mode() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "unsupported_mode".to_string(),  // not supported
                tls: None,
                routes: vec![],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has unsupported mode"));
    }

    #[test]
    fn test_validate_model_missing_default_upstream() {
        let domains = vec![];

        let upstreams = vec![
            UpstreamSpec {
                name: "some_upstream".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "missing_upstream".to_string(),  // doesn't exist
            tls_passthrough_upstream: "cilium_tls".to_string(),  // also missing
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("http_default_upstream") || error_msg.contains("tls_passthrough_upstream"));
    }

    #[test]
    fn test_validate_model_upstream_with_no_endpoints() {
        let domains = vec![];

        let upstreams = vec![
            UpstreamSpec {
                name: "empty_upstream".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![],  // no endpoints
            },
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has no endpoints"));
    }

    #[test]
    fn test_validate_model_route_with_unknown_upstream() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert".to_string(),
                    private_key: "/path/to/key".to_string(),
                }),
                routes: vec![
                    RouteSpec {
                        m: MatchSpec::Prefix("/api".to_string()),
                        to_upstream: "unknown_backend".to_string(),  // doesn't exist
                        timeout: Some("30s".to_string()),
                        per_filter_config: None,
                    }
                ],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("route references unknown upstream"));
    }

    #[test]
    fn test_validate_model_route_with_unknown_rate_limit_policy() {
        let domains = vec![
            DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert".to_string(),
                    private_key: "/path/to/key".to_string(),
                }),
                routes: vec![
                    RouteSpec {
                        m: MatchSpec::Prefix("/api".to_string()),
                        to_upstream: "api_backend".to_string(),
                        timeout: Some("30s".to_string()),
                        per_filter_config: Some(PerFilterConfigRef {
                            local_ratelimit: Some("unknown_policy".to_string()),  // doesn't exist
                        }),
                    }
                ],
            }
        ];

        let upstreams = vec![
            UpstreamSpec {
                name: "api_backend".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 8080,
                }],
            },
            UpstreamSpec {
                name: "cilium_http".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 80,
                }],
            },
            UpstreamSpec {
                name: "cilium_tls".to_string(),
                connect_timeout: "5s".to_string(),
                r#type: "STRICT_DNS".to_string(),
                lb_policy: "ROUND_ROBIN".to_string(),
                http2: false,
                endpoints: vec![Endpoint {
                    address: "127.0.0.1".to_string(),
                    port: 443,
                }],
            },
        ];

        let policies = PoliciesSpec {
            local_ratelimits: Default::default(),  // empty - no policies
        };

        let defaults = DefaultsSpec {
            route_timeout: "60s".to_string(),
            http_default_upstream: "cilium_http".to_string(),
            tls_passthrough_upstream: "cilium_tls".to_string(),
        };

        let result = validate_model(&domains, &upstreams, &policies, &defaults);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("route references unknown local_ratelimit policy"));
    }
}