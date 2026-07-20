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

        /// Also write an LLM remediation prompt (`{repo}-repository-hygiene-prompt.md`)
        /// that instructs a model to add missing artifacts and close gaps.
        #[arg(long)]
        llm_prompt: bool,
    },

    /// Analyze and emit only an LLM remediation prompt (stdout + file).
    Prompt {
        /// Local repository path or remote Git URI.
        #[arg(default_value = ".")]
        target: String,

        /// Write prompt to this path (file) or directory.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Write the file only; do not print the prompt to stdout.
        #[arg(long)]
        quiet: bool,
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
                llm_prompt,
            } => Ok(Config {
                target,
                format: OutputFormat::parse(&format)?,
                output,
                llm_prompt,
                prompt_only: false,
                prompt_stdout: false,
                ..Config::default()
            }),
            Commands::Prompt {
                target,
                output,
                quiet,
            } => Ok(Config {
                target,
                format: OutputFormat::Markdown,
                output,
                llm_prompt: true,
                prompt_only: true,
                prompt_stdout: !quiet,
                ..Config::default()
            }),
        }
    }
}
