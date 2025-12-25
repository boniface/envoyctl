use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name="envoyctl", version, about="Manage Envoy config via fragments -> generated YAML")]
pub struct Cli {
    /// Root config directory (contains common/, domains/, upstreams/, policies/)
    #[arg(long, default_value = "config")]
    pub config_dir: PathBuf,

    /// Output directory for generated config
    #[arg(long, default_value = "out")]
    pub out_dir: PathBuf,

    /// Where to install the generated Envoy config (apply)
    #[arg(long, default_value = "/etc/envoy/envoy.yaml")]
    pub install_path: PathBuf,

    /// Envoy binary name/path (native validation mode)
    #[arg(long)]
    pub envoy_bin: Option<String>,

    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a starter workspace you can edit
    Init {
        /// Directory to create (default: ./envoy-work)
        #[arg(long, default_value = "envoy-work")]
        dir: PathBuf,
    },
    Build,
    Validate,
    Apply,
}
