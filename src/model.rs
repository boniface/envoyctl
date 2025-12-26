use serde::Deserialize;
use indexmap::IndexMap;

#[derive(Debug, Deserialize)]
pub struct AdminSpec {
    #[serde(default = "default_admin_address")]
    pub address: String,
    #[serde(default = "default_admin_port")]
    pub port: u16,
}
fn default_admin_address() -> String { "0.0.0.0".into() }
fn default_admin_port() -> u16 { 9901 }

#[derive(Debug, Deserialize)]
pub struct DefaultsSpec {
    #[serde(default = "default_route_timeout")]
    pub route_timeout: String,

    /// Upstream name used by :80 listener (HTTP)
    #[serde(default = "default_http_upstream")]
    pub http_default_upstream: String,

    /// Upstream name used by :443 default passthrough chain (TCP proxy)
    #[serde(default = "default_tls_passthrough_upstream")]
    pub tls_passthrough_upstream: String,
}
fn default_route_timeout() -> String { "60s".into() }
fn default_http_upstream() -> String { "cilium_http".into() }
fn default_tls_passthrough_upstream() -> String { "cilium_tls".into() }

#[derive(Debug, Deserialize)]
pub struct AccessLogSpec {
    #[serde(default = "default_log_type")]
    pub r#type: String, // "stdout" (initial draft)
    #[serde(default = "default_log_path")]
    pub path: String,
}
fn default_log_type() -> String { "stdout".into() }
fn default_log_path() -> String { "/dev/stdout".into() }

#[derive(Debug, Deserialize)]
pub struct RuntimeSpec {
    pub validate: ValidateSpec,
}

#[derive(Debug, Deserialize)]
#[serde(tag="type")]
pub enum ValidateSpec {
    #[serde(rename="native")]
    Native {},
    #[serde(rename="docker_image")]
    DockerImage { image: String },
}

#[derive(Debug, Deserialize)]
pub struct UpstreamSpec {
    pub name: String,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: String,
    #[serde(default = "default_cluster_type")]
    pub r#type: String,
    #[serde(default = "default_lb_policy")]
    pub lb_policy: String,
    pub endpoints: Vec<Endpoint>,
    /// If true, add `http2_protocol_options: {}` (needed for h2c backends like Zitadel)
    #[serde(default)]
    pub http2: bool,
}
fn default_connect_timeout() -> String { "5s".into() }
fn default_cluster_type() -> String { "STRICT_DNS".into() }
fn default_lb_policy() -> String { "ROUND_ROBIN".into() }

#[derive(Debug, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DomainSpec {
    pub domain: String,

    /// Supported: "terminate_https_443" or "passthrough_https_443"
    #[serde(default = "default_mode")]
    pub mode: String,

    pub tls: Option<TlsSpec>,
    pub routes: Vec<RouteSpec>,
}
fn default_mode() -> String { "terminate_https_443".into() }

#[derive(Debug, Deserialize)]
pub struct TlsSpec {
    pub cert_chain: String,
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteSpec {
    #[serde(rename="match")]
    pub m: MatchSpec,
    pub to_upstream: String,
    pub timeout: Option<String>,
    pub per_filter_config: Option<PerFilterConfigRef>,
}

#[derive(Debug, Deserialize)]
pub struct PerFilterConfigRef {
    pub local_ratelimit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MatchSpec {
    Prefix(String),
    Path(String),
}

#[derive(Debug, Deserialize)]
pub struct PoliciesSpec {
    #[serde(default)]
    pub local_ratelimits: IndexMap<String, TokenBucket>,
}

#[derive(Debug, Deserialize)]
pub struct TokenBucket {
    pub max_tokens: u32,
    pub tokens_per_fill: u32,
    pub fill_interval: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn test_deserialize_admin_spec() {
        let yaml = r#"
address: "127.0.0.1"
port: 9000
"#;
        let admin: AdminSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(admin.address, "127.0.0.1");
        assert_eq!(admin.port, 9000);
    }

    #[test]
    fn test_deserialize_admin_spec_with_defaults() {
        let yaml = "{}";
        let admin: AdminSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(admin.address, "0.0.0.0"); // default
        assert_eq!(admin.port, 9901); // default
    }

    #[test]
    fn test_deserialize_defaults_spec() {
        let yaml = r#"
route_timeout: "30s"
http_default_upstream: "my_http"
tls_passthrough_upstream: "my_tls"
"#;
        let defaults: DefaultsSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(defaults.route_timeout, "30s");
        assert_eq!(defaults.http_default_upstream, "my_http");
        assert_eq!(defaults.tls_passthrough_upstream, "my_tls");
    }

    #[test]
    fn test_deserialize_defaults_spec_with_defaults() {
        let yaml = "{}";
        let defaults: DefaultsSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(defaults.route_timeout, "60s"); // default
        assert_eq!(defaults.http_default_upstream, "cilium_http"); // default
        assert_eq!(defaults.tls_passthrough_upstream, "cilium_tls"); // default
    }

    #[test]
    fn test_deserialize_upstream_spec() {
        let yaml = r#"
name: "my_upstream"
connect_timeout: "10s"
type: "STATIC"
lb_policy: "ROUND_ROBIN"
http2: true
endpoints:
  - address: "127.0.0.1"
    port: 8080
  - address: "127.0.0.1"
    port: 8081
"#;
        let upstream: UpstreamSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(upstream.name, "my_upstream");
        assert_eq!(upstream.connect_timeout, "10s");
        assert_eq!(upstream.r#type, "STATIC");
        assert_eq!(upstream.lb_policy, "ROUND_ROBIN");
        assert!(upstream.http2);
        assert_eq!(upstream.endpoints.len(), 2);
        assert_eq!(upstream.endpoints[0].address, "127.0.0.1");
        assert_eq!(upstream.endpoints[0].port, 8080);
        assert_eq!(upstream.endpoints[1].address, "127.0.0.1");
        assert_eq!(upstream.endpoints[1].port, 8081);
    }

    #[test]
    fn test_deserialize_upstream_spec_with_defaults() {
        let yaml = r#"
name: "my_upstream"
endpoints:
  - address: "127.0.0.1"
    port: 8080
"#;
        let upstream: UpstreamSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(upstream.connect_timeout, "5s"); // default
        assert_eq!(upstream.r#type, "STRICT_DNS"); // default
        assert_eq!(upstream.lb_policy, "ROUND_ROBIN"); // default
        assert!(!upstream.http2); // default
    }

    #[test]
    fn test_deserialize_domain_spec() {
        let yaml = r#"
domain: "example.com"
mode: "terminate_https_443"
tls:
  cert_chain: "/path/to/cert"
  private_key: "/path/to/key"
routes:
  - match: "/api"
    to_upstream: "api_backend"
    timeout: "30s"
"#;
        let domain: DomainSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(domain.domain, "example.com");
        assert_eq!(domain.mode, "terminate_https_443");
        assert!(domain.tls.is_some());
        assert_eq!(domain.routes.len(), 1);
        assert_eq!(domain.routes[0].to_upstream, "api_backend");
        assert_eq!(domain.routes[0].timeout, Some("30s".to_string()));
    }

    #[test]
    fn test_deserialize_domain_spec_with_defaults() {
        let yaml = r#"
domain: "example.com"
routes: []
"#;
        let domain: DomainSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(domain.mode, "terminate_https_443"); // default
        assert!(domain.tls.is_none());
        assert_eq!(domain.routes.len(), 0);
    }

    #[test]
    fn test_deserialize_route_spec_with_per_filter_config() {
        let yaml = r#"
match: "/api"
to_upstream: "api_backend"
timeout: "30s"
per_filter_config:
  local_ratelimit: "strict"
"#;
        let route: RouteSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(route.to_upstream, "api_backend");
        assert_eq!(route.timeout, Some("30s".to_string()));
        assert!(route.per_filter_config.is_some());
        assert_eq!(route.per_filter_config.as_ref().unwrap().local_ratelimit, Some("strict".to_string()));
    }

    #[test]
    fn test_deserialize_policies_spec() {
        let yaml = r#"
local_ratelimits:
  strict:
    max_tokens: 100
    tokens_per_fill: 100
    fill_interval: "1s"
  moderate:
    max_tokens: 50
    tokens_per_fill: 50
    fill_interval: "1s"
"#;
        let policies: PoliciesSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(policies.local_ratelimits.len(), 2);
        assert!(policies.local_ratelimits.contains_key("strict"));
        assert!(policies.local_ratelimits.contains_key("moderate"));

        let strict = policies.local_ratelimits.get("strict").unwrap();
        assert_eq!(strict.max_tokens, 100);
        assert_eq!(strict.tokens_per_fill, 100);
        assert_eq!(strict.fill_interval, "1s");
    }

    #[test]
    fn test_deserialize_policies_spec_empty() {
        let yaml = "{}";
        let policies: PoliciesSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(policies.local_ratelimits.len(), 0);
    }
}