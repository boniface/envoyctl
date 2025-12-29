use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AdminSpec {
    #[serde(default = "default_admin_address")]
    pub address: String,
    #[serde(default = "default_admin_port")]
    pub port: u16,
}
fn default_admin_address() -> String {
    "0.0.0.0".into()
}
fn default_admin_port() -> u16 {
    9901
}

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
fn default_route_timeout() -> String {
    "60s".into()
}
fn default_http_upstream() -> String {
    "cilium_http".into()
}
fn default_tls_passthrough_upstream() -> String {
    "cilium_tls".into()
}

#[derive(Debug, Deserialize)]
pub struct AccessLogSpec {
    /// Log output type: "stdout" or "file" (reserved for future use)
    #[serde(default = "default_log_type")]
    #[allow(dead_code)]
    pub r#type: String,
    #[serde(default = "default_log_path")]
    pub path: String,
}
fn default_log_type() -> String {
    "stdout".into()
}
fn default_log_path() -> String {
    "/dev/stdout".into()
}

#[derive(Debug, Deserialize)]
pub struct RuntimeSpec {
    pub validate: ValidateSpec,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListenersSpec {
    #[serde(default)]
    pub internal_http_listeners: Vec<InternalHttpListenerSpec>,
}

#[derive(Debug, Deserialize)]
pub struct InternalHttpListenerSpec {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub stat_prefix: String,
    pub domains: Vec<String>,
    pub to_upstream: String,
    pub timeout: Option<String>,
    #[serde(default)]
    pub request_headers_to_add: Vec<HeaderValueOption>,
}

#[derive(Debug, Deserialize)]
pub struct HeaderValueOption {
    pub header: HeaderValue,
    pub append_action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeaderValue {
    pub key: String,
    pub value: String,
}

/// Validation mode for checking Envoy configuration
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ValidateSpec {
    /// Validate using docker exec on a running container
    /// Command: docker exec -it <container> envoy --mode validate -c <config_path>
    #[serde(rename = "docker_exec")]
    DockerExec {
        /// Name of the running Envoy container
        container: String,
        /// Path to config inside the container (default: /etc/envoy/envoy.yaml)
        #[serde(default = "default_container_config_path")]
        config_path: String,
    },

    /// Validate on baremetal using sudo
    /// Command: sudo -u envoy envoy --mode validate -c <config_path>
    #[serde(rename = "native")]
    Native {
        /// User to run envoy as (default: envoy)
        #[serde(default = "default_envoy_user")]
        user: String,
        /// Path to envoy binary (default: envoy)
        #[serde(default = "default_envoy_bin")]
        bin: String,
        /// Path to config file (default: /etc/envoy/envoy.yaml)
        #[serde(default = "default_native_config_path")]
        config_path: String,
    },

    /// Validate using docker run with a fresh container (for testing)
    /// Command: docker run --rm -v <config>:/cfg.yaml:ro <image> envoy --mode validate -c /cfg.yaml
    #[serde(rename = "docker_image")]
    DockerImage { image: String },
}

fn default_container_config_path() -> String {
    "/etc/envoy/envoy.yaml".into()
}
fn default_envoy_user() -> String {
    "envoy".into()
}
fn default_envoy_bin() -> String {
    "envoy".into()
}
fn default_native_config_path() -> String {
    "/etc/envoy/envoy.yaml".into()
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
fn default_connect_timeout() -> String {
    "5s".into()
}
fn default_cluster_type() -> String {
    "STRICT_DNS".into()
}
fn default_lb_policy() -> String {
    "ROUND_ROBIN".into()
}

#[derive(Debug, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Default)]
pub struct DomainSpec {
    #[serde(default)]
    pub domain: String,

    /// Supported: "terminate_https_443" or "passthrough_https_443"
    #[serde(default = "default_mode")]
    pub mode: String,

    #[serde(default)]
    pub tls: Option<TlsSpec>,
    #[serde(default)]
    pub routes: Vec<RouteSpec>,
    #[serde(default)]
    pub http_connection_manager: Option<HttpConnectionManagerSpec>,

    /// Override normalize_path for this domain (default: true, but set false for S3 signing)
    #[serde(default)]
    pub normalize_path: Option<bool>,

    /// Override merge_slashes for this domain (default: true, but set false for S3 signing)
    #[serde(default)]
    pub merge_slashes: Option<bool>,

    /// AWS Request Signing configuration for upstream requests
    #[serde(default)]
    pub aws_signing: Option<AwsSigningSpec>,
}
fn default_mode() -> String {
    "terminate_https_443".into()
}

#[derive(Debug, Deserialize)]
pub struct TlsSpec {
    pub cert_chain: String,
    pub private_key: String,
}

/// AWS Request Signing configuration for upstream requests
#[derive(Debug, Deserialize)]
pub struct AwsSigningSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_s3_service")]
    pub service_name: String,
    #[serde(default = "default_garage_region")]
    pub region: String,
    #[serde(default = "default_unsigned_payload")]
    pub use_unsigned_payload: bool,
    /// Use only environment variables for credentials (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// This prevents Envoy from trying IMDS/instance profile which fails on non-AWS environments
    #[serde(default = "default_use_env_credentials")]
    pub use_env_credentials: bool,
}

fn default_s3_service() -> String {
    "s3".into()
}
fn default_garage_region() -> String {
    "garage".into()
}
fn default_unsigned_payload() -> bool {
    true
}
fn default_use_env_credentials() -> bool {
    true  // Default to true since most non-AWS deployments need this
}

#[derive(Debug, Deserialize, Default)]
pub struct RouteSpec {
    #[serde(rename = "match", default)]
    pub m: MatchSpec,

    /// Upstream cluster to route to (mutually exclusive with direct_response)
    #[serde(default)]
    pub to_upstream: Option<String>,

    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub per_filter_config: Option<PerFilterConfigRef>,

    /// Rewrite the path prefix before forwarding upstream
    #[serde(default)]
    pub prefix_rewrite: Option<String>,

    /// Return a direct response instead of routing to upstream
    #[serde(default)]
    pub direct_response: Option<DirectResponseSpec>,
}

#[derive(Debug, Deserialize)]
pub struct DirectResponseSpec {
    pub status: u16,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PerFilterConfigRef {
    pub local_ratelimit: Option<String>,
}

/// Match specification for routes
/// Supports: { prefix: "/api/" } or { path: "/health" }
/// Can also include header matchers
#[derive(Debug, Deserialize, Clone, Default)]
pub struct MatchSpec {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    /// Header matchers for the route
    #[serde(default)]
    pub headers: Vec<HeaderMatcher>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HeaderMatcher {
    pub name: String,
    pub exact_match: Option<String>,
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
    pub stat_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HttpConnectionManagerSpec {
    pub stat_prefix: Option<String>,
    pub normalize_path: Option<bool>,
    pub merge_slashes: Option<bool>,
    pub use_remote_address: Option<bool>,
    pub xff_num_trusted_hops: Option<u32>,
    pub stream_idle_timeout: Option<String>,
    pub local_ratelimit_stat_prefix: Option<String>,
    #[serde(default)]
    pub extra_http_filters: Vec<HttpFilterSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpFilterSpec {
    GrpcWeb,
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
  - match:
      prefix: "/api"
    to_upstream: "api_backend"
    timeout: "30s"
"#;
        let domain: DomainSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(domain.domain, "example.com");
        assert_eq!(domain.mode, "terminate_https_443");
        assert!(domain.tls.is_some());
        assert_eq!(domain.routes.len(), 1);
        assert_eq!(domain.routes[0].to_upstream, Some("api_backend".to_string()));
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
match: { prefix: "/api" }
to_upstream: "api_backend"
timeout: "30s"
per_filter_config:
  local_ratelimit: "strict"
"#;
        let route: RouteSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(route.to_upstream, Some("api_backend".to_string()));
        assert_eq!(route.timeout, Some("30s".to_string()));
        assert!(route.per_filter_config.is_some());
        assert_eq!(
            route.per_filter_config.as_ref().unwrap().local_ratelimit,
            Some("strict".to_string())
        );
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
