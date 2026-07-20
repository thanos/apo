//! Automation & delivery rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::Rule;
use crate::rules::helpers;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(CiWorkflows),
        Box::new(ReleaseWorkflows),
        Box::new(BranchProtection),
        Box::new(RequiredStatusChecks),
    ]
}

struct CiWorkflows;
impl Rule for CiWorkflows {
    fn id(&self) -> &'static str {
        "delivery.ci_workflows"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        let mut hits = signals.ci_workflow_paths.clone();

        // Other CI systems
        for name in [
            ".gitlab-ci.yml",
            ".circleci/config.yml",
            "azure-pipelines.yml",
            "bitbucket-pipelines.yml",
            "Jenkinsfile",
            ".travis.yml",
            "buildkite.yml",
        ] {
            if ctx.has_file(name) {
                hits.push(name.to_string());
            }
        }
        if ctx.has_dir(".circleci") {
            hits.push(".circleci".into());
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Delivery)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No CI workflow configuration detected.")
                .remediation("Add CI workflows under .github/workflows/ or equivalent.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Delivery)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("CI workflow configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct ReleaseWorkflows;
impl Rule for ReleaseWorkflows {
    fn id(&self) -> &'static str {
        "delivery.release_workflows"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        let mut items = Vec::new();

        for path in &signals.ci_workflow_paths {
            let name = path.to_ascii_lowercase();
            let content = ctx.read_text(path).unwrap_or_default().to_ascii_lowercase();
            let name_hit =
                name.contains("release") || name.contains("publish") || name.contains("deploy");
            let body_hit = content.contains("release")
                || content.contains("softprops/action-gh-release")
                || content.contains("cargo publish")
                || content.contains("npm publish")
                || content.contains("pypi")
                || content.contains("goreleaser");
            if name_hit || body_hit {
                items.push(EvidenceItem::path_detail(
                    path,
                    if name_hit {
                        "filename suggests release/publish"
                    } else {
                        "workflow body mentions release/publish"
                    },
                ));
            }
        }

        for name in [
            "goreleaser.yml",
            ".goreleaser.yml",
            ".goreleaser.yaml",
            "release-please-config.json",
        ] {
            if ctx.has_file(name) {
                items.push(EvidenceItem::path(name));
            }
        }

        if items.is_empty() {
            Finding::builder(self.id(), Category::Delivery)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No release/publish workflow detected.")
                .remediation("Add a release workflow or release automation config.")
                .build()
        } else {
            Finding::builder(self.id(), Category::Delivery)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Release/publish automation detected.")
                .evidence(items)
                .build()
        }
    }
}

struct BranchProtection;
impl Rule for BranchProtection {
    fn id(&self) -> &'static str {
        "delivery.branch_protection"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        // Local git cannot observe GitHub branch protection settings.
        // Look for checked-in policy-as-code hints only.
        let mut items = Vec::new();
        for name in [
            ".github/settings.yml",
            ".github/branch-protection.yml",
            "branch-protection.md",
        ] {
            if ctx.has_file(name) {
                items.push(EvidenceItem::path(name));
            }
        }

        // Probot settings / github-as-code
        items.extend(
            ctx.inventory
                .find_path_contains("branch-protection")
                .into_iter()
                .map(|e| EvidenceItem::path(e.relative.clone())),
        );

        let rulesets =
            helpers::ci_mentions(ctx, &["branch protection", "ruleset", "required reviewers"]);
        items.extend(rulesets);

        if items.is_empty() {
            Finding::builder(self.id(), Category::Delivery)
                .status(Status::Unknown)
                .confidence(Confidence::Low)
                .summary(
                    "Branch protection cannot be verified from a local clone alone; no policy-as-code file detected.",
                )
                .push_evidence(EvidenceItem::detail(
                    "GitHub/GitLab branch protection APIs are out of scope for v0.1 local analysis.",
                ))
                .remediation(
                    "Enable branch protection on the default branch, or check in .github/settings.yml.",
                )
                .build()
        } else {
            Finding::builder(self.id(), Category::Delivery)
                .status(Status::Partial)
                .confidence(Confidence::Medium)
                .summary("Branch protection policy-as-code signals detected (platform enforcement not verified).")
                .evidence(items)
                .build()
        }
    }
}

struct RequiredStatusChecks;
impl Rule for RequiredStatusChecks {
    fn id(&self) -> &'static str {
        "delivery.required_status_checks"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        if signals.ci_workflow_paths.is_empty()
            && !ctx.has_file(".gitlab-ci.yml")
            && !ctx.has_file(".circleci/config.yml")
        {
            return Finding::builder(self.id(), Category::Delivery)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No CI workflows found from which status checks could be required.")
                .remediation(
                    "Add CI workflows, then require them as status checks on the default branch.",
                )
                .build();
        }

        // Presence of CI is evidence that checks *can* be required; actual enforcement needs API.
        let mut items: Vec<_> = signals
            .ci_workflow_paths
            .iter()
            .map(|p| {
                EvidenceItem::path_detail(
                    p,
                    "CI workflow present (required-check enforcement not verified locally)",
                )
            })
            .collect();

        if let Some(content) = ctx.read_text(".github/settings.yml") {
            let lower = content.to_ascii_lowercase();
            if lower.contains("status_checks") || lower.contains("contexts") {
                items.push(EvidenceItem::path_detail(
                    ".github/settings.yml",
                    "mentions status_checks/contexts",
                ));
                return Finding::builder(self.id(), Category::Delivery)
                    .status(Status::Present)
                    .confidence(Confidence::Medium)
                    .summary("Status check configuration hinted in settings-as-code.")
                    .evidence(items)
                    .build();
            }
        }

        Finding::builder(self.id(), Category::Delivery)
            .status(Status::Unknown)
            .confidence(Confidence::Low)
            .summary(
                "CI workflows exist, but whether they are required status checks cannot be verified locally.",
            )
            .evidence(items)
            .remediation("Configure required status checks on the default branch in the hosting platform.")
            .build()
    }
}
