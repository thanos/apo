//! Markdown report writer.

use std::fmt::Write as _;
use std::path::Path;

use crate::error::Result;
use crate::evidence::Status;
use crate::report::Report;

pub fn write_markdown(report: &Report, path: &Path) -> Result<()> {
    std::fs::write(path, render_markdown(report))?;
    Ok(())
}

pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# Repository Hygiene Report");
    let _ = writeln!(out);
    let _ = writeln!(out, "- **Analyzer:** {}", report.analyzer);
    let _ = writeln!(out, "- **APO version:** {}", report.apo_version);
    let _ = writeln!(out, "- **Repository:** `{}`", report.repository);
    if let Some(uri) = &report.source_uri {
        let _ = writeln!(out, "- **Source URI:** `{uri}`");
    }
    if let Some(path) = &report.checkout_path {
        let _ = writeln!(out, "- **Checkout path:** `{path}`");
    }
    let _ = writeln!(out, "- **Generated:** {}", report.generated_at);
    if let Some(score) = report.policy.overall_score {
        let _ = writeln!(out, "- **Overall score:** {score:.1}/100");
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Executive summary");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", report.executive_summary);
    let _ = writeln!(out);

    let _ = writeln!(out, "## Category scores");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Category | Weight | Score | Contribution | Enforced | Present | Partial | Missing | Unknown |"
    );
    let _ = writeln!(
        out,
        "|----------|-------:|------:|-------------:|---------:|--------:|--------:|--------:|--------:|"
    );
    for c in &report.policy.categories {
        let score = c
            .score
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "n/a".into());
        let contrib = c
            .weighted_contribution
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "—".into());
        let _ = writeln!(
            out,
            "| {} | {:.0}% | {} | {} | {} | {} | {} | {} | {} |",
            c.name,
            c.weight * 100.0,
            score,
            contrib,
            c.enforced,
            c.present,
            c.partial,
            c.missing,
            c.unknown
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "_Rubric `{}`: weighted overall from category scores. Notes: {}._",
        report.policy.rubric,
        report
            .policy
            .categories
            .iter()
            .map(|c| format!("{} — {}", c.name, c.note))
            .collect::<Vec<_>>()
            .join("; ")
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out);
    for f in &report.findings {
        let _ = writeln!(out, "### `{}`", f.rule);
        let _ = writeln!(out);
        let _ = writeln!(out, "- **Category:** {}", f.category.display_name());
        let _ = writeln!(out, "- **Status:** {:?}", f.status);
        let _ = writeln!(out, "- **Confidence:** {:?}", f.confidence);
        let _ = writeln!(out, "- **Summary:** {}", f.summary);
        if let Some(r) = &f.remediation {
            let _ = writeln!(out, "- **Remediation:** {r}");
        }
        if !f.evidence.is_empty() {
            let _ = writeln!(out, "- **Evidence:**");
            for e in &f.evidence {
                match (&e.path, &e.detail) {
                    (Some(p), Some(d)) => {
                        let _ = writeln!(out, "  - `{p}` — {d}");
                    }
                    (Some(p), None) => {
                        let _ = writeln!(out, "  - `{p}`");
                    }
                    (None, Some(d)) => {
                        let _ = writeln!(out, "  - {d}");
                    }
                    (None, None) => {}
                }
            }
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Missing controls");
    let _ = writeln!(out);
    if report.missing_controls.is_empty() {
        let _ = writeln!(out, "No gap signals recorded.");
    } else {
        for g in &report.missing_controls {
            let _ = writeln!(out, "- `{g}`");
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Recommendations");
    let _ = writeln!(out);
    if report.recommendations.is_empty() {
        let _ = writeln!(out, "No recommendations.");
    } else {
        for r in &report.recommendations {
            let _ = writeln!(out, "- {r}");
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Evidence appendix");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Rule | Status | Paths |");
    let _ = writeln!(out, "|------|--------|-------|");
    for f in &report.findings {
        let paths: Vec<_> = f
            .evidence
            .iter()
            .filter_map(|e| e.path.as_deref())
            .collect();
        let path_str = if paths.is_empty() {
            "—".into()
        } else {
            paths.join(", ")
        };
        let status = match f.status {
            Status::Enforced => "Enforced",
            Status::Present => "Present",
            Status::Partial => "Partial",
            Status::Missing => "Missing",
            Status::NotApplicable => "NotApplicable",
            Status::Unknown => "Unknown",
        };
        let _ = writeln!(out, "| `{}` | {status} | {path_str} |", f.rule);
    }
    let _ = writeln!(out);

    out
}
