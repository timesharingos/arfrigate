use clap::{Parser, Subcommand};

#[derive(PartialEq, Eq, Parser)]
#[command(version = "0.1.3", about = "filter directories by gitignore")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(PartialEq, Eq, Subcommand)]
pub enum Commands {
    Filter { dirs: Vec<String> },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
