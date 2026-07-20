//! Command-line interface.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{Config, OutputFormat};

/// APO — Engineering Evidence Platform.
#[derive(Debug, Parser)]
#[command(
    name = "apo",
    version,
    about = "APO — Engineering Evidence Platform. Collect objective repository hygiene evidence.",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Analyze a local path or remote Git URI for hygiene evidence.
    Analyze {
        /// Local repository path or remote Git URI
        /// (https://…, git@…, ssh://…, file://…).
        #[arg(default_value = ".")]
        target: String,

        /// Output format: markdown, json, or both.
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Write report to this path (file) or directory.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

impl Cli {
    /// Convert CLI args into a [`Config`].
    pub fn into_config(self) -> Result<Config, String> {
        match self.command {
            Commands::Analyze {
                target,
                format,
                output,
            } => Ok(Config {
                target,
                format: OutputFormat::parse(&format)?,
                output,
                ..Config::default()
            }),
        }
    }
}
