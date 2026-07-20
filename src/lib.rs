//! APO — Engineering Evidence Platform.
//!
//! Repository Hygiene is the first analyzer: it collects observational evidence
//! about engineering controls in a local Git repository and derives policy scores.

#![forbid(unsafe_code)]

pub mod cli;
pub mod config;
pub mod discovery;
pub mod error;
pub mod evidence;
pub mod git;
pub mod policy;
pub mod report;
pub mod rules;
pub mod source;

pub use config::{Config, OutputFormat};
pub use error::{Error, Result};
pub use report::Report;
pub use source::Workspace;

use tracing::info;

/// Analyze a repository and produce a hygiene report.
///
/// `target` may be a local path or a remote Git URI. Remote URIs are shallow-cloned
/// into a temporary directory that is removed when this function returns.
pub fn analyze(config: &Config) -> Result<Report> {
    let (report, _workspace) = analyze_with_workspace(config)?;
    Ok(report)
}

/// Analyze and retain the workspace until the caller drops it.
///
/// Useful when the caller still needs the checkout path (e.g. writing reports
/// beside a local repo, or inspecting a remote clone before cleanup).
pub fn analyze_with_workspace(config: &Config) -> Result<(Report, Workspace)> {
    info!(target = %config.target, "resolving repository");
    let workspace = source::resolve(&config.target, config.commit_sample_limit)?;

    info!(
        path = %workspace.path.display(),
        label = %workspace.label,
        remote = workspace.is_remote(),
        "discovering repository"
    );
    let ctx = discovery::discover(&workspace.path, config.commit_sample_limit)?;

    info!(files = ctx.inventory.len(), "evaluating hygiene rules");
    let findings = rules::evaluate_all(&ctx);

    info!(count = findings.len(), "computing policy scores");
    let policy = policy::evaluate(&findings);

    let report = Report::build(&ctx, findings, policy, &workspace);
    Ok((report, workspace))
}

/// Analyze and write reports to disk. Returns the report and written paths.
pub fn analyze_and_write(config: &Config) -> Result<(Report, Vec<std::path::PathBuf>)> {
    let (report, workspace) = analyze_with_workspace(config)?;

    let cwd = std::env::current_dir()?;
    let default_dir = config
        .output
        .as_ref()
        .and_then(|p| {
            if p.is_dir() {
                Some(p.as_path())
            } else {
                p.parent()
            }
        })
        .unwrap_or(if workspace.is_remote() {
            cwd.as_path()
        } else {
            workspace.path.as_path()
        });

    let write_dir = if config.output.is_none() {
        if workspace.is_remote() {
            cwd.as_path()
        } else {
            workspace.path.as_path()
        }
    } else {
        default_dir
    };

    let written = report::write_report(&report, config.format, config.output.as_deref(), write_dir)?;
    // Keep workspace alive until writes finish (remote temp clone).
    drop(workspace);
    Ok((report, written))
}
