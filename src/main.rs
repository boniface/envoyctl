mod apply;
mod cli;
mod exec;
mod generate;
mod init;
mod load;
mod model;
mod validate;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.cmd {
        Command::Init { dir } => init::cmd_init(&cli, dir),
        Command::Build => apply::cmd_build(&cli),
        Command::Validate => apply::cmd_validate(&cli),
    }
}
