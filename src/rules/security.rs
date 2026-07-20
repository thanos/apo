//! Security & supply chain rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::helpers;
use crate::rules::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(DependencyScanning),
        Box::new(SecretScanning),
        Box::new(SecurityPolicy),
        Box::new(DependencyUpdates),
    ]
}

struct DependencyScanning;
impl Rule for DependencyScanning {
    fn id(&self) -> &'static str {
        "security.dependency_scanning"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = helpers::find_configs(
            ctx,
            &[
                "deny.toml",
                ".snyk",
                "nancy.toml",
                "audit.toml",
            ],
        );
        let mut items = helpers::ci_mentions(
            ctx,
            &[
                "cargo audit",
                "cargo deny",
                "npm audit",
                "pnpm audit",
                "yarn audit",
                "pip-audit",
                "safety",
                "snyk",
                "dependabot",
                "osv-scanner",
                "govulncheck",
                "nancy",
                "trivy",
                "grype",
            ],
        );
        for h in hits.drain(..) {
            items.push(EvidenceItem::path(h));
        }

        // GitHub Dependabot security updates often live with dependency updates —
        // still count scanning intent if code scanning workflows exist
        let codeql = ctx
            .inventory
            .find_path_contains("codeql")
            .into_iter()
            .map(|e| EvidenceItem::path(e.relative.clone()));
        items.extend(codeql);

        if items.is_empty() {
            Finding::builder(self.id(), Category::Security)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No dependency scanning configuration detected.")
                .remediation("Add cargo audit / npm audit / snyk / osv-scanner (or similar) in CI.")
                .build()
        } else {
            Finding::builder(self.id(), Category::Security)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary("Dependency scanning signals detected.")
                .evidence(items)
                .build()
        }
    }
}

struct SecretScanning;
impl Rule for SecretScanning {
    fn id(&self) -> &'static str {
        "security.secret_scanning"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = helpers::find_configs(
            ctx,
            &[
                ".gitleaks.toml",
                "gitleaks.toml",
                ".secretlintrc",
                ".secretlintrc.json",
                ".trufflehog.yml",
                "detect-secrets",
            ],
        );
        // detect-secrets baseline often named .secrets.baseline
        if ctx.has_file(".secrets.baseline") {
            hits.push(".secrets.baseline".into());
        }

        let mut items = helpers::ci_mentions(
            ctx,
            &[
                "gitleaks",
                "trufflehog",
                "detect-secrets",
                "secretless",
                "git-secrets",
                "secret scanning",
            ],
        );
        for h in hits {
            items.push(EvidenceItem::path(h));
        }

        if items.is_empty() {
            Finding::builder(self.id(), Category::Security)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No secret scanning configuration detected.")
                .remediation("Add gitleaks/trufflehog/detect-secrets to CI or pre-commit.")
                .build()
        } else {
            Finding::builder(self.id(), Category::Security)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary("Secret scanning signals detected.")
                .evidence(items)
                .build()
        }
    }
}

struct SecurityPolicy;
impl Rule for SecurityPolicy {
    fn id(&self) -> &'static str {
        "security.policy"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        // Overlaps documentation.security but focuses on security category scoring.
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Security,
            &[
                "SECURITY.md",
                "SECURITY.rst",
                "SECURITY",
                ".github/SECURITY.md",
                "docs/SECURITY.md",
            ],
            "Security policy document detected.",
            "No security policy document detected.",
            "Add SECURITY.md with vulnerability reporting instructions.",
        )
    }
}

struct DependencyUpdates;
impl Rule for DependencyUpdates {
    fn id(&self) -> &'static str {
        "security.dependency_updates"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = Vec::new();
        for name in [
            ".github/dependabot.yml",
            ".github/dependabot.yaml",
            "dependabot.yml",
            "renovate.json",
            "renovate.json5",
            ".renovaterc",
            ".renovaterc.json",
            "renovate-config.json",
        ] {
            if ctx.has_file(name) {
                hits.push(name.to_string());
            }
        }
        hits.extend(
            ctx.inventory
                .find_matching(|p| {
                    let l = p.to_ascii_lowercase();
                    l.contains("dependabot") || l.contains("renovate")
                })
                .into_iter()
                .map(|e| e.relative.clone()),
        );
        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Security)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No automated dependency update configuration detected.")
                .remediation("Add Dependabot or Renovate configuration.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Security)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Automated dependency update configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}
