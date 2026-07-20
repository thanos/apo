//! Hygiene rule engine.

mod collaboration;
mod delivery;
mod documentation;
mod local_dev;
mod security;
mod testing;

use crate::discovery::RepoContext;
use crate::evidence::Finding;

/// A hygiene rule that produces observational evidence.
pub trait Rule: Send + Sync {
    /// Stable rule id, e.g. `documentation.readme`.
    fn id(&self) -> &'static str;

    /// Evaluate the rule against the repository context.
    fn evaluate(&self, ctx: &RepoContext) -> Finding;
}

/// Run all built-in rules (in parallel) and return findings.
pub fn evaluate_all(ctx: &RepoContext) -> Vec<Finding> {
    use rayon::prelude::*;

    let rules = all_rules();
    let mut findings: Vec<Finding> = rules.par_iter().map(|rule| rule.evaluate(ctx)).collect();

    findings.sort_by(|a, b| a.rule.cmp(&b.rule));
    findings
}

/// Construct the v0.1 built-in rule set.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    rules.extend(documentation::rules());
    rules.extend(local_dev::rules());
    rules.extend(testing::rules());
    rules.extend(security::rules());
    rules.extend(delivery::rules());
    rules.extend(collaboration::rules());
    rules
}

/// Shared helpers for path-based rules.
pub(crate) mod helpers {
    use crate::discovery::RepoContext;
    use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};

    /// Evaluate presence of any of the candidate files.
    pub fn file_presence(
        ctx: &RepoContext,
        rule: &str,
        category: Category,
        candidates: &[&str],
        present_summary: &str,
        missing_summary: &str,
        remediation: &str,
    ) -> Finding {
        if let Some(path) = ctx.first_existing(candidates) {
            Finding::builder(rule, category)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary(present_summary)
                .push_evidence(EvidenceItem::path(path))
                .build()
        } else {
            Finding::builder(rule, category)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary(missing_summary)
                .remediation(remediation)
                .build()
        }
    }

    /// Evaluate presence with optional content keyword enrichment.
    #[allow(clippy::too_many_arguments)]
    pub fn file_presence_with_keywords(
        ctx: &RepoContext,
        rule: &str,
        category: Category,
        candidates: &[&str],
        keywords: &[&str],
        enforced_summary: &str,
        present_summary: &str,
        missing_summary: &str,
        remediation: &str,
    ) -> Finding {
        let Some(path) = ctx.first_existing(candidates) else {
            return Finding::builder(rule, category)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary(missing_summary)
                .remediation(remediation)
                .build();
        };

        let content = ctx.read_text(path).unwrap_or_default();
        let lower = content.to_ascii_lowercase();
        let hits: Vec<&str> = keywords
            .iter()
            .copied()
            .filter(|k| lower.contains(&k.to_ascii_lowercase()))
            .collect();

        if !hits.is_empty() {
            Finding::builder(rule, category)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary(enforced_summary)
                .push_evidence(EvidenceItem::path_detail(
                    path,
                    format!("matched keywords: {}", hits.join(", ")),
                ))
                .build()
        } else {
            Finding::builder(rule, category)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary(present_summary)
                .push_evidence(EvidenceItem::path(path))
                .build()
        }
    }

    /// Search CI workflow bodies for any of the needles.
    pub fn ci_mentions(ctx: &RepoContext, needles: &[&str]) -> Vec<EvidenceItem> {
        let signals = ctx.detect_signals();
        let mut items = Vec::new();
        for path in &signals.ci_workflow_paths {
            let Some(content) = ctx.read_text(path) else {
                continue;
            };
            let lower = content.to_ascii_lowercase();
            let matched: Vec<&str> = needles
                .iter()
                .copied()
                .filter(|n| lower.contains(&n.to_ascii_lowercase()))
                .collect();
            if !matched.is_empty() {
                items.push(EvidenceItem::path_detail(
                    path,
                    format!("mentions: {}", matched.join(", ")),
                ));
            }
        }
        items
    }

    /// Find config files by basename list.
    pub fn find_configs(ctx: &RepoContext, names: &[&str]) -> Vec<String> {
        ctx.inventory
            .find_by_basenames(names)
            .into_iter()
            .map(|e| e.relative.clone())
            .collect()
    }
}
