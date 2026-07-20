//! LLM remediation prompt generation from hygiene findings.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::evidence::{Category, Status};
use crate::report::Report;

/// Render a paste-ready prompt telling an LLM how to remediate hygiene gaps.
pub fn render_llm_prompt(report: &Report) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# Repository hygiene remediation task");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "You are an expert software engineer improving **repository engineering hygiene**."
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "APO (Engineering Evidence Platform) analyzed this repository and found missing or weak controls."
    );
    let _ = writeln!(
        out,
        "Your job is to **update the repository** so those gaps are addressed with real, minimal, high-quality artifacts and configuration — not aspirational docs."
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Repository");
    let _ = writeln!(out);
    let _ = writeln!(out, "- **Identity:** `{}`", report.repository);
    if let Some(uri) = &report.source_uri {
        let _ = writeln!(out, "- **Source URI:** `{uri}`");
        let _ = writeln!(
            out,
            "- **Important:** Analysis used a temporary clone. Apply all changes to the real working tree / open PR for `{uri}`."
        );
    }
    if let Some(path) = &report.checkout_path
        && report.source_uri.is_some()
    {
        let _ = writeln!(out, "- **Analysis checkout (ephemeral):** `{path}`");
    }
    let _ = writeln!(out, "- **APO version:** {}", report.apo_version);
    let _ = writeln!(out, "- **Analyzer:** {}", report.analyzer);
    let _ = writeln!(out, "- **Generated:** {}", report.generated_at);
    if let Some(score) = report.policy.overall_score {
        let _ = writeln!(
            out,
            "- **Current overall score:** {score:.1}/100 (rubric `{}`)",
            report.policy.rubric
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Objectives");
    let _ = writeln!(out);
    let _ = writeln!(out, "1. Close the **gap findings** listed below (Missing / Partial / Unknown).");
    let _ = writeln!(
        out,
        "2. Prefer the smallest change set that creates **observable evidence** APO can detect (files, CI steps, configs)."
    );
    let _ = writeln!(
        out,
        "3. Match the repository’s existing language, tooling, and style; detect the ecosystem from files already present."
    );
    let _ = writeln!(
        out,
        "4. Do **not** claim controls you did not add. Do **not** invent security reviews, owners, or history."
    );
    let _ = writeln!(
        out,
        "5. For `Unknown` items that require hosting-platform settings (branch protection, required checks), document the exact clicks/API/settings-as-code needed; implement settings-as-code when the repo already uses it."
    );
    let _ = writeln!(
        out,
        "6. After changes, summarize what you added/updated and which APO rule ids each change targets."
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Constraints");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Keep documentation accurate and short; avoid filler.");
    let _ = writeln!(out, "- Do not remove existing working controls.");
    let _ = writeln!(out, "- Do not commit secrets, tokens, or private keys.");
    let _ = writeln!(
        out,
        "- If a gap is truly not applicable (e.g. no deployable artifact for release automation), say so and skip rather than adding fake workflows."
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Scoring rubric (priority hint)");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Category | Weight | Score | Notes |");
    let _ = writeln!(out, "|----------|-------:|------:|-------|");
    for c in &report.policy.categories {
        let score = c
            .score
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "n/a".into());
        let _ = writeln!(
            out,
            "| {} | {:.0}% | {score} | {} |",
            c.name,
            c.weight * 100.0,
            c.note
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Prioritize higher-weight categories with low scores when sequencing work."
    );
    let _ = writeln!(out);

    // Satisfied controls — context so the LLM doesn't redo them
    let satisfied: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(f.status, Status::Enforced | Status::Present))
        .collect();

    let _ = writeln!(out, "## Already satisfied (do not redo)");
    let _ = writeln!(out);
    if satisfied.is_empty() {
        let _ = writeln!(out, "_None — treat the repository as greenfield for hygiene controls._");
    } else {
        for f in &satisfied {
            let paths = evidence_paths(f);
            if paths.is_empty() {
                let _ = writeln!(
                    out,
                    "- `{}` — {:?} — {}",
                    f.rule, f.status, f.summary
                );
            } else {
                let _ = writeln!(
                    out,
                    "- `{}` — {:?} — {} (evidence: {})",
                    f.rule,
                    f.status,
                    f.summary,
                    paths.join(", ")
                );
            }
        }
    }
    let _ = writeln!(out);

    let gaps: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.status.is_gap())
        .collect();

    let _ = writeln!(out, "## Gaps to remediate");
    let _ = writeln!(out);
    if gaps.is_empty() {
        let _ = writeln!(
            out,
            "_No gap findings. Optionally suggest polish improvements only if clearly valuable._"
        );
    } else {
        let _ = writeln!(
            out,
            "Work through these in category order. For each gap: create or update the concrete artifacts, then tick it off."
        );
        let _ = writeln!(out);

        for cat in Category::all() {
            let cat_gaps: Vec<_> = gaps.iter().filter(|f| f.category == *cat).collect();
            if cat_gaps.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "### {} ({:.0}% — {})",
                cat.display_name(),
                cat.weight() * 100.0,
                cat.rubric_note()
            );
            let _ = writeln!(out);

            for (i, f) in cat_gaps.into_iter().enumerate() {
                let _ = writeln!(out, "#### {}. `{}` — {:?}", i + 1, f.rule, f.status);
                let _ = writeln!(out);
                let _ = writeln!(out, "- **Summary:** {}", f.summary);
                let _ = writeln!(out, "- **Confidence:** {:?}", f.confidence);
                if let Some(r) = &f.remediation {
                    let _ = writeln!(out, "- **Suggested remediation:** {r}");
                }
                if !f.evidence.is_empty() {
                    let _ = writeln!(out, "- **Evidence / notes:**");
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
                let _ = writeln!(
                    out,
                    "- **Your task:** Implement changes that would make APO report this rule as `Present` or `Enforced` (unless genuinely Not Applicable)."
                );
                let _ = writeln!(out);
            }
        }
    }

    let _ = writeln!(out, "## Deliverable format");
    let _ = writeln!(out);
    let _ = writeln!(out, "1. Make the file/config/CI edits in the repository.");
    let _ = writeln!(
        out,
        "2. Reply with a short changelog mapping each touched path → APO rule id(s)."
    );
    let _ = writeln!(
        out,
        "3. Call out any gaps you could not close locally (platform-only settings) with exact follow-up steps."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "---");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "_This prompt was generated by APO from observational evidence. Treat findings as facts to act on, not as opinions to debate._"
    );

    out
}

fn evidence_paths(f: &crate::evidence::Finding) -> Vec<&str> {
    f.evidence
        .iter()
        .filter_map(|e| e.path.as_deref())
        .collect()
}

/// Write the LLM remediation prompt to `path`.
pub fn write_llm_prompt(report: &Report, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render_llm_prompt(report))?;
    Ok(())
}

/// Resolve where to write the LLM prompt given `--output` and default directory.
pub fn resolve_prompt_path(
    report: &Report,
    output: Option<&Path>,
    default_dir: &Path,
) -> PathBuf {
    let default_name = report.prompt_filename();
    match output {
        None => default_dir.join(&default_name),
        Some(p) if p.is_dir() || p.extension().is_none() => p.join(&default_name),
        Some(p) if p.extension().is_some_and(|e| e == "md" || e == "markdown") => {
            // If user passed report.md, write sibling *-prompt.md; if already *prompt*, use as-is.
            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("repository-hygiene");
            if name.contains("prompt") {
                p.to_path_buf()
            } else {
                p.with_file_name(format!("{name}-prompt.md"))
            }
        }
        Some(p) => {
            // json or other file → sibling prompt in same directory
            let parent = p.parent().unwrap_or(default_dir);
            parent.join(default_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
    use crate::policy;

    fn sample_report() -> Report {
        let findings = vec![
            Finding::builder("documentation.readme", Category::Documentation)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary("README present.")
                .push_evidence(EvidenceItem::path("README.md"))
                .build(),
            Finding::builder("documentation.license", Category::Documentation)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No license file detected.")
                .remediation("Add a LICENSE file clarifying reuse terms.")
                .build(),
            Finding::builder("security.dependency_updates", Category::Security)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No Dependabot.")
                .remediation("Add Dependabot or Renovate configuration.")
                .build(),
        ];
        let pol = policy::evaluate(&findings);
        Report {
            apo_version: "0.1.0".into(),
            analyzer: "repository-hygiene".into(),
            repository: "/tmp/demo".into(),
            checkout_path: None,
            source_uri: None,
            generated_at: "2026-01-01T00:00:00Z".into(),
            executive_summary: "test".into(),
            policy: pol,
            missing_controls: vec![
                "documentation.license".into(),
                "security.dependency_updates".into(),
            ],
            recommendations: vec![],
            findings,
        }
    }

    #[test]
    fn prompt_includes_gaps_and_satisfied() {
        let text = render_llm_prompt(&sample_report());
        assert!(text.contains("Repository hygiene remediation task"));
        assert!(text.contains("documentation.license"));
        assert!(text.contains("security.dependency_updates"));
        assert!(text.contains("Already satisfied"));
        assert!(text.contains("documentation.readme"));
        assert!(text.contains("Add a LICENSE file"));
    }
}
