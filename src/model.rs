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
    pub restart: RestartSpec,
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
#[serde(tag="type")]
pub enum RestartSpec {
    #[serde(rename="docker_restart")]
    DockerRestart { container: String },
    #[serde(rename="docker_compose")]
    DockerCompose { service: String, file: Option<String> },
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
#[serde(rename_all="snake_case")]
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
