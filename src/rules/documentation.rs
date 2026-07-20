//! Documentation & ownership rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::helpers;
use crate::rules::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Readme),
        Box::new(Contributing),
        Box::new(SecurityDoc),
        Box::new(License),
        Box::new(Architecture),
        Box::new(Adrs),
        Box::new(Runbooks),
        Box::new(Codeowners),
        Box::new(IssueTemplates),
        Box::new(PrTemplates),
    ]
}

struct Readme;
impl Rule for Readme {
    fn id(&self) -> &'static str {
        "documentation.readme"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence_with_keywords(
            ctx,
            self.id(),
            Category::Documentation,
            &["README.md", "README.rst", "README.txt", "README", "Readme.md"],
            &["quickstart", "getting started", "install", "usage", "how to"],
            "README with quickstart/usage guidance detected.",
            "README present.",
            "No README file detected.",
            "Add a README.md describing purpose, setup, and usage.",
        )
    }
}

struct Contributing;
impl Rule for Contributing {
    fn id(&self) -> &'static str {
        "documentation.contributing"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Documentation,
            &[
                "CONTRIBUTING.md",
                "CONTRIBUTING.rst",
                "CONTRIBUTING",
                ".github/CONTRIBUTING.md",
                "docs/CONTRIBUTING.md",
            ],
            "CONTRIBUTING guide detected.",
            "No CONTRIBUTING guide detected.",
            "Add CONTRIBUTING.md with contribution workflow and standards.",
        )
    }
}

struct SecurityDoc;
impl Rule for SecurityDoc {
    fn id(&self) -> &'static str {
        "documentation.security"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Documentation,
            &[
                "SECURITY.md",
                "SECURITY.rst",
                "SECURITY",
                ".github/SECURITY.md",
                "docs/SECURITY.md",
            ],
            "SECURITY policy document detected.",
            "No SECURITY policy document detected.",
            "Add SECURITY.md describing how to report vulnerabilities.",
        )
    }
}

struct License;
impl Rule for License {
    fn id(&self) -> &'static str {
        "documentation.license"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Documentation,
            &[
                "LICENSE",
                "LICENSE.md",
                "LICENSE.txt",
                "COPYING",
                "COPYING.md",
                "LICENCE",
                "LICENCE.md",
            ],
            "License file detected.",
            "No license file detected.",
            "Add a LICENSE file clarifying reuse terms.",
        )
    }
}

struct Architecture;
impl Rule for Architecture {
    fn id(&self) -> &'static str {
        "documentation.architecture"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let candidates = [
            "ARCHITECTURE.md",
            "docs/ARCHITECTURE.md",
            "docs/architecture.md",
            "docs/architecture/README.md",
            "DESIGN.md",
            "docs/DESIGN.md",
        ];
        let mut hits = ctx.existing_of(&candidates);
        hits.extend(
            ctx.inventory
                .find_path_contains("architecture")
                .into_iter()
                .filter(|e| {
                    let l = e.relative.to_ascii_lowercase();
                    l.ends_with(".md") || l.ends_with(".rst") || l.ends_with(".adoc")
                })
                .map(|e| e.relative.clone()),
        );
        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Documentation)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No architecture documentation detected.")
                .remediation("Add ARCHITECTURE.md or docs describing system design.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Documentation)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Architecture documentation detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct Adrs;
impl Rule for Adrs {
    fn id(&self) -> &'static str {
        "documentation.adrs"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let dirs = ["docs/adr", "docs/adrs", "adr", "adrs", "doc/adr", "architecture/decisions"];
        let mut hits = Vec::new();
        for d in dirs {
            if ctx.has_dir(d) {
                hits.push(d.to_string());
            }
        }
        let files = ctx.inventory.find_path_contains("/adr");
        for f in files {
            let l = f.relative.to_ascii_lowercase();
            if l.contains("adr") && (l.ends_with(".md") || l.ends_with(".rst")) {
                hits.push(f.relative.clone());
            }
        }
        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Documentation)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No Architecture Decision Records (ADRs) detected.")
                .remediation("Add an adr/ or docs/adr/ directory for decision records.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Documentation)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Architecture Decision Records detected.");
            for h in hits.into_iter().take(10) {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct Runbooks;
impl Rule for Runbooks {
    fn id(&self) -> &'static str {
        "documentation.runbooks"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits: Vec<String> = ctx
            .inventory
            .find_path_contains("runbook")
            .into_iter()
            .map(|e| e.relative.clone())
            .collect();
        for d in ["docs/runbooks", "runbooks", "ops/runbooks"] {
            if ctx.has_dir(d) {
                hits.push(d.to_string());
            }
        }
        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Documentation)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No runbooks detected.")
                .remediation("Add operational runbooks under docs/runbooks/.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Documentation)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Runbook documentation detected.");
            for h in hits.into_iter().take(10) {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct Codeowners;
impl Rule for Codeowners {
    fn id(&self) -> &'static str {
        "documentation.codeowners"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Documentation,
            &[
                "CODEOWNERS",
                ".github/CODEOWNERS",
                "docs/CODEOWNERS",
                ".gitlab/CODEOWNERS",
            ],
            "CODEOWNERS file detected.",
            "No CODEOWNERS file detected.",
            "Add CODEOWNERS to define review ownership.",
        )
    }
}

struct IssueTemplates;
impl Rule for IssueTemplates {
    fn id(&self) -> &'static str {
        "documentation.issue_templates"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = Vec::new();
        if ctx.has_dir(".github/ISSUE_TEMPLATE") || ctx.has_dir(".github/issue_template") {
            hits.push(".github/ISSUE_TEMPLATE".to_string());
        }
        for name in [
            ".github/ISSUE_TEMPLATE.md",
            ".github/issue_template.md",
            "ISSUE_TEMPLATE.md",
        ] {
            if ctx.has_file(name) {
                hits.push(name.to_string());
            }
        }
        // GitHub YAML issue forms
        hits.extend(
            ctx.inventory
                .find_matching(|p| {
                    let l = p.to_ascii_lowercase();
                    l.starts_with(".github/issue_template/")
                        && (l.ends_with(".md") || l.ends_with(".yml") || l.ends_with(".yaml"))
                })
                .into_iter()
                .map(|e| e.relative.clone()),
        );

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Documentation)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No issue templates detected.")
                .remediation("Add issue templates under .github/ISSUE_TEMPLATE/.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Documentation)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Issue templates detected.");
            for h in hits.into_iter().take(10) {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct PrTemplates;
impl Rule for PrTemplates {
    fn id(&self) -> &'static str {
        "documentation.pr_templates"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::Documentation,
            &[
                ".github/pull_request_template.md",
                ".github/PULL_REQUEST_TEMPLATE.md",
                "docs/pull_request_template.md",
                "PULL_REQUEST_TEMPLATE.md",
                ".github/PULL_REQUEST_TEMPLATE/pull_request_template.md",
            ],
            "Pull request template detected.",
            "No pull request template detected.",
            "Add .github/pull_request_template.md.",
        )
    }
}
