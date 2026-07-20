//! Change management & collaboration rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::helpers;
use crate::rules::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(CommitConvention),
        Box::new(IssueLinkage),
        Box::new(CodeownersCollab),
        Box::new(ReviewConfiguration),
        Box::new(MaintenanceActivity),
    ]
}

struct CommitConvention;
impl Rule for CommitConvention {
    fn id(&self) -> &'static str {
        "collaboration.commit_convention"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut items = Vec::new();

        for name in [
            "COMMIT_CONVENTION.md",
            "docs/COMMIT_CONVENTION.md",
            ".commitlintrc",
            ".commitlintrc.js",
            ".commitlintrc.cjs",
            ".commitlintrc.json",
            ".commitlintrc.yml",
            "commitlint.config.js",
            "commitlint.config.cjs",
            "commitlint.config.mjs",
        ] {
            if ctx.has_file(name) {
                items.push(EvidenceItem::path(name));
            }
        }

        if let Some(pkg) = ctx.read_text("package.json")
            && pkg.to_ascii_lowercase().contains("commitlint")
        {
            items.push(EvidenceItem::path_detail(
                "package.json",
                "commitlint dependency/config",
            ));
        }

        if let Some(ratio) = ctx.git.conventional_commit_ratio {
            items.push(EvidenceItem::detail(format!(
                "{:.0}% of {} sampled commits match Conventional Commits subject pattern",
                ratio * 100.0,
                ctx.git.commit_count_sampled
            )));

            if ratio >= 0.7 {
                return Finding::builder(self.id(), Category::Collaboration)
                    .status(Status::Enforced)
                    .confidence(Confidence::High)
                    .summary("Commit messages largely follow Conventional Commits.")
                    .evidence(items)
                    .build();
            }
            if ratio >= 0.3 || !items.is_empty() {
                return Finding::builder(self.id(), Category::Collaboration)
                    .status(Status::Partial)
                    .confidence(Confidence::Medium)
                    .summary("Partial Conventional Commits adoption observed.")
                    .evidence(items)
                    .remediation("Adopt Conventional Commits and optionally enforce via commitlint.")
                    .build();
            }
        }

        if !items.is_empty() {
            return Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Commit convention configuration detected.")
                .evidence(items)
                .build();
        }

        Finding::builder(self.id(), Category::Collaboration)
            .status(Status::Missing)
            .confidence(Confidence::Medium)
            .summary("No commit message convention detected.")
            .remediation("Document and optionally enforce a commit message convention.")
            .build()
    }
}

struct IssueLinkage;
impl Rule for IssueLinkage {
    fn id(&self) -> &'static str {
        "collaboration.issue_linkage"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut items = Vec::new();

        // PR template mentioning issues
        for name in [
            ".github/pull_request_template.md",
            ".github/PULL_REQUEST_TEMPLATE.md",
        ] {
            if let Some(content) = ctx.read_text(name) {
                let l = content.to_ascii_lowercase();
                if l.contains("fixes #")
                    || l.contains("closes #")
                    || l.contains("issue")
                    || l.contains("ticket")
                {
                    items.push(EvidenceItem::path_detail(name, "mentions issue linkage"));
                }
            }
        }

        if let Some(ratio) = ctx.git.issue_link_ratio {
            items.push(EvidenceItem::detail(format!(
                "{:.0}% of {} sampled commits reference an issue id",
                ratio * 100.0,
                ctx.git.commit_count_sampled
            )));

            if ratio >= 0.4 {
                return Finding::builder(self.id(), Category::Collaboration)
                    .status(Status::Enforced)
                    .confidence(Confidence::Medium)
                    .summary("Issue linkage commonly present in commit history.")
                    .evidence(items)
                    .build();
            }
            if ratio > 0.0 || !items.is_empty() {
                return Finding::builder(self.id(), Category::Collaboration)
                    .status(Status::Partial)
                    .confidence(Confidence::Medium)
                    .summary("Some issue linkage observed.")
                    .evidence(items)
                    .build();
            }
        }

        if !items.is_empty() {
            return Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Present)
                .confidence(Confidence::Medium)
                .summary("Issue linkage guidance detected in templates.")
                .evidence(items)
                .build();
        }

        Finding::builder(self.id(), Category::Collaboration)
            .status(Status::Missing)
            .confidence(Confidence::Low)
            .summary("No issue linkage convention detected in commits or templates.")
            .remediation("Reference issue IDs in commits/PRs (e.g. Fixes #123).")
            .build()
    }
}

struct CodeownersCollab;
impl Rule for CodeownersCollab {
    fn id(&self) -> &'static str {
        "collaboration.codeowners"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Collaboration,
            &[
                "CODEOWNERS",
                ".github/CODEOWNERS",
                "docs/CODEOWNERS",
                ".gitlab/CODEOWNERS",
            ],
            "CODEOWNERS file detected.",
            "No CODEOWNERS file detected.",
            "Add CODEOWNERS to route reviews to responsible owners.",
        )
    }
}

struct ReviewConfiguration;
impl Rule for ReviewConfiguration {
    fn id(&self) -> &'static str {
        "collaboration.review_configuration"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut items = Vec::new();

        for name in [
            ".github/pull_request_template.md",
            ".github/PULL_REQUEST_TEMPLATE.md",
            "CODEOWNERS",
            ".github/CODEOWNERS",
            ".github/settings.yml",
        ] {
            if ctx.has_file(name) {
                items.push(EvidenceItem::path(name));
            }
        }

        // Reviewers file / auto-assign
        items.extend(
            ctx.inventory
                .find_matching(|p| {
                    let l = p.to_ascii_lowercase();
                    l.contains("auto_assign") || l.contains("reviewers")
                })
                .into_iter()
                .map(|e| EvidenceItem::path(e.relative.clone())),
        );

        if items.is_empty() {
            Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No review configuration artifacts detected.")
                .remediation("Add PR templates, CODEOWNERS, and/or required review settings.")
                .build()
        } else {
            Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Review-oriented configuration artifacts detected.")
                .evidence(items)
                .build()
        }
    }
}

struct MaintenanceActivity;
impl Rule for MaintenanceActivity {
    fn id(&self) -> &'static str {
        "collaboration.maintenance_activity"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        if !ctx.git.available {
            return Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Unknown)
                .confidence(Confidence::Low)
                .summary("Git metadata unavailable; maintenance activity not assessed.")
                .build();
        }

        let Some(days) = ctx.git.days_since_last_commit else {
            return Finding::builder(self.id(), Category::Collaboration)
                .status(Status::Unknown)
                .confidence(Confidence::Low)
                .summary("Unable to determine last commit age.")
                .evidence(
                    ctx.git
                        .note
                        .as_ref()
                        .map(|n| vec![EvidenceItem::detail(n.clone())])
                        .unwrap_or_default(),
                )
                .build();
        };

        let mut items = vec![EvidenceItem::detail(format!(
            "last commit approximately {days} day(s) ago; sampled {} commit(s)",
            ctx.git.commit_count_sampled
        ))];
        for s in ctx.git.recent_subjects.iter().take(3) {
            items.push(EvidenceItem::detail(format!("recent subject: {s}")));
        }

        let (status, summary) = if days <= 30 {
            (
                Status::Enforced,
                "Recent maintenance activity detected (commit within 30 days).",
            )
        } else if days <= 90 {
            (
                Status::Present,
                "Maintenance activity within the last 90 days.",
            )
        } else if days <= 180 {
            (
                Status::Partial,
                "Last commit is older than 90 days.",
            )
        } else {
            (
                Status::Missing,
                "No recent maintenance activity (last commit older than 180 days).",
            )
        };

        let mut b = Finding::builder(self.id(), Category::Collaboration)
            .status(status)
            .confidence(Confidence::High)
            .summary(summary)
            .evidence(items);

        if status.is_gap() {
            b = b.remediation("Resume regular maintenance commits or archive the repository.");
        }
        b.build()
    }
}
