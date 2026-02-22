use clap::{Parser, Subcommand};

#[derive(Debug, PartialEq, Eq, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum Commands {
    Filter { dirs: Vec<String> },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
