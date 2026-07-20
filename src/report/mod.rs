//! Report generation (JSON + Markdown).

mod json;
mod markdown;

pub use json::{to_string as json_to_string, write_json};
pub use markdown::write_markdown;

use serde::{Deserialize, Serialize};

use crate::config::OutputFormat;
use crate::discovery::RepoContext;
use crate::error::Result;
use crate::evidence::Finding;
use crate::policy::PolicyResult;
use crate::source::Workspace;

use std::path::{Path, PathBuf};

/// Complete analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Schema / tool version.
    pub apo_version: String,
    /// Analyzer name.
    pub analyzer: String,
    /// User-facing repository identity (remote URI or local path).
    pub repository: String,
    /// Local checkout path when different from `repository` (remote clones).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_path: Option<String>,
    /// Original remote URI when analysis cloned a remote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// ISO-8601 generation timestamp.
    pub generated_at: String,
    /// Executive summary text.
    pub executive_summary: String,
    /// Policy scores.
    pub policy: PolicyResult,
    /// All findings.
    pub findings: Vec<Finding>,
    /// Missing / gap controls (rule ids).
    pub missing_controls: Vec<String>,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

impl Report {
    /// Build a report from context, findings, policy, and workspace identity.
    pub fn build(
        ctx: &RepoContext,
        findings: Vec<Finding>,
        policy: PolicyResult,
        workspace: &Workspace,
    ) -> Self {
        let overall = policy
            .overall_score
            .map(|s| format!("{s:.0}/100"))
            .unwrap_or_else(|| "n/a".into());

        let enforced = findings
            .iter()
            .filter(|f| f.status == crate::evidence::Status::Enforced)
            .count();
        let present = findings
            .iter()
            .filter(|f| f.status == crate::evidence::Status::Present)
            .count();
        let gaps = policy.gaps.len();

        let executive_summary = format!(
            "Repository hygiene analysis of `{}` scored {overall}. \
             Observed {enforced} enforced, {present} present, and {gaps} gap signal(s) across {} rules. \
             Findings are observational evidence only; scores are derived by policy weights.",
            workspace.label,
            findings.len(),
        );

        let missing_controls = policy.gaps.clone();
        let recommendations = policy.recommendations.clone();

        let checkout_path = if workspace.is_remote() {
            Some(ctx.root.display().to_string())
        } else {
            None
        };

        Self {
            apo_version: env!("CARGO_PKG_VERSION").to_string(),
            analyzer: "repository-hygiene".into(),
            repository: workspace.label.clone(),
            checkout_path,
            source_uri: workspace.source_uri.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            executive_summary,
            policy,
            findings,
            missing_controls,
            recommendations,
        }
    }
}

/// Resolve output paths for the requested format.
pub fn resolve_outputs(
    format: OutputFormat,
    output: Option<&Path>,
    cwd: &Path,
) -> Result<Vec<(OutputFormat, PathBuf)>> {
    let default_md = cwd.join("repository-hygiene.md");
    let default_json = cwd.join("repository-hygiene.json");

    match (format, output) {
        (OutputFormat::Markdown, None) => Ok(vec![(OutputFormat::Markdown, default_md)]),
        (OutputFormat::Json, None) => Ok(vec![(OutputFormat::Json, default_json)]),
        (OutputFormat::Both, None) => Ok(vec![
            (OutputFormat::Markdown, default_md),
            (OutputFormat::Json, default_json),
        ]),
        (OutputFormat::Markdown, Some(p)) => {
            if p.is_dir() || p.extension().is_none() {
                Ok(vec![(OutputFormat::Markdown, p.join("repository-hygiene.md"))])
            } else {
                Ok(vec![(OutputFormat::Markdown, p.to_path_buf())])
            }
        }
        (OutputFormat::Json, Some(p)) => {
            if p.is_dir() || (p.extension().is_none() && !p.to_string_lossy().ends_with(".json")) {
                Ok(vec![(OutputFormat::Json, p.join("repository-hygiene.json"))])
            } else {
                Ok(vec![(OutputFormat::Json, p.to_path_buf())])
            }
        }
        (OutputFormat::Both, Some(p)) => {
            if p.is_file()
                || p
                    .extension()
                    .is_some_and(|e| e == "md" || e == "json" || e == "markdown")
            {
                // Treat as directory parent or stem base
                let parent = p.parent().unwrap_or(cwd);
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("repository-hygiene");
                Ok(vec![
                    (OutputFormat::Markdown, parent.join(format!("{stem}.md"))),
                    (OutputFormat::Json, parent.join(format!("{stem}.json"))),
                ])
            } else {
                Ok(vec![
                    (OutputFormat::Markdown, p.join("repository-hygiene.md")),
                    (OutputFormat::Json, p.join("repository-hygiene.json")),
                ])
            }
        }
    }
}

/// Write report artifacts according to format/output config.
pub fn write_report(
    report: &Report,
    format: OutputFormat,
    output: Option<&Path>,
    default_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let targets = resolve_outputs(format, output, default_dir)?;
    let mut written = Vec::new();
    for (fmt, path) in targets {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match fmt {
            OutputFormat::Markdown => write_markdown(report, &path)?,
            OutputFormat::Json => write_json(report, &path)?,
            OutputFormat::Both => unreachable!("resolved to concrete formats"),
        }
        written.push(path);
    }
    Ok(written)
}
