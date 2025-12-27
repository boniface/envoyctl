use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "envoyctl",
    version,
    about = "Manage Envoy config via fragments -> generated YAML"
)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parse_build() {
        let args = vec!["envoyctl", "build"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.config_dir, std::path::PathBuf::from("config"));
        assert_eq!(cli.out_dir, std::path::PathBuf::from("out"));
        assert_eq!(
            cli.install_path,
            std::path::PathBuf::from("/etc/envoy/envoy.yaml")
        );
        match cli.cmd {
            Command::Build => {} // Expected
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_parse_validate() {
        let args = vec!["envoyctl", "validate"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.cmd {
            Command::Validate => {} // Expected
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_cli_parse_with_custom_paths() {
        let args = vec![
            "envoyctl",
            "--config-dir",
            "/custom/config",
            "--out-dir",
            "/custom/out",
            "--install-path",
            "/custom/install.yaml",
            "build",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.config_dir, std::path::PathBuf::from("/custom/config"));
        assert_eq!(cli.out_dir, std::path::PathBuf::from("/custom/out"));
        assert_eq!(
            cli.install_path,
            std::path::PathBuf::from("/custom/install.yaml")
        );
        match cli.cmd {
            Command::Build => {} // Expected
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_parse_with_envoy_bin() {
        let args = vec![
            "envoyctl",
            "--envoy-bin",
            "/usr/local/bin/envoy",
            "validate",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.envoy_bin, Some("/usr/local/bin/envoy".to_string()));
        match cli.cmd {
            Command::Validate => {} // Expected
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_cli_parse_init() {
        let args = vec!["envoyctl", "init"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.cmd {
            Command::Init { dir } => {
                assert_eq!(dir, std::path::PathBuf::from("envoy-work")); // Default value
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parse_init_with_custom_dir() {
        let args = vec!["envoyctl", "init", "--dir", "/custom/dir"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.cmd {
            Command::Init { dir } => {
                assert_eq!(dir, std::path::PathBuf::from("/custom/dir"));
            }
            _ => panic!("Expected Init command"),
        }
    }
}
