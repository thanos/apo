//! Runtime configuration for analysis.

use serde::{Deserialize, Serialize};

/// Output format for reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Markdown report (default).
    #[default]
    Markdown,
    /// JSON report.
    Json,
    /// Emit both Markdown and JSON.
    Both,
}

impl OutputFormat {
    /// Parse from CLI string.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "both" => Ok(Self::Both),
            other => Err(format!(
                "unknown format '{other}'; expected markdown, json, or both"
            )),
        }
    }
}

/// Analysis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Local path or remote Git URI to analyze.
    pub target: String,
    /// Desired output format.
    pub format: OutputFormat,
    /// Optional explicit output path (file or directory).
    pub output: Option<std::path::PathBuf>,
    /// Maximum commits to inspect for maintenance signals (also clone depth for remotes).
    pub commit_sample_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target: ".".into(),
            format: OutputFormat::Markdown,
            output: None,
            commit_sample_limit: 100,
        }
    }
}
