//! Testing & quality gate rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::helpers;
use crate::rules::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(TestFramework),
        Box::new(CoverageConfig),
        Box::new(CoverageEnforcement),
        Box::new(StaticAnalysisCi),
        Box::new(TypeCheckingCi),
    ]
}

struct TestFramework;
impl Rule for TestFramework {
    fn id(&self) -> &'static str {
        "testing.framework"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        let mut hits = Vec::new();

        // Directory conventions
        for d in ["tests", "test", "spec", "__tests__", "src/test"] {
            if ctx.has_dir(d) {
                hits.push(format!("{d}/"));
            }
        }

        // Test file patterns
        let test_files = ctx.inventory.find_matching(|p| {
            let l = p.to_ascii_lowercase();
            l.contains("/test_")
                || l.contains("_test.")
                || l.contains(".test.")
                || l.contains(".spec.")
                || l.ends_with("_test.go")
                || l.ends_with("_test.rs")
                || l.ends_with("_spec.rb")
                || l.contains("/__tests__/")
        });
        for f in test_files.into_iter().take(15) {
            hits.push(f.relative.clone());
        }

        if let Some(pkg) = ctx.read_text("package.json") {
            let l = pkg.to_ascii_lowercase();
            if l.contains("jest")
                || l.contains("vitest")
                || l.contains("mocha")
                || l.contains("ava")
                || l.contains("\"test\"")
            {
                hits.push("package.json".into());
            }
        }
        if signals.has_cargo {
            hits.push("Cargo.toml".into());
        }
        if let Some(py) = ctx.read_text("pyproject.toml") {
            let l = py.to_ascii_lowercase();
            if l.contains("pytest") || l.contains("unittest") || l.contains("[tool.pytest") {
                hits.push("pyproject.toml".into());
            }
        }
        if ctx.has_file("pytest.ini") || ctx.has_file("tox.ini") {
            if ctx.has_file("pytest.ini") {
                hits.push("pytest.ini".into());
            }
            if ctx.has_file("tox.ini") {
                hits.push("tox.ini".into());
            }
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No test framework or test files detected.")
                .remediation("Add a test suite and framework configuration.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Testing)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Test framework or test files detected.");
            for h in hits.into_iter().take(12) {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct CoverageConfig;
impl Rule for CoverageConfig {
    fn id(&self) -> &'static str {
        "testing.coverage_config"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = helpers::find_configs(
            ctx,
            &[
                "codecov.yml",
                "codecov.yaml",
                ".codecov.yml",
                "coverage.xml",
                ".coveragerc",
                "lcov.info",
            ],
        );
        if let Some(pkg) = ctx.read_text("package.json") {
            let l = pkg.to_ascii_lowercase();
            if l.contains("nyc") || l.contains("c8") || l.contains("istanbul") || l.contains("coverage") {
                hits.push("package.json".into());
            }
        }
        if let Some(py) = ctx.read_text("pyproject.toml") {
            let l = py.to_ascii_lowercase();
            if l.contains("coverage") || l.contains("pytest-cov") {
                hits.push("pyproject.toml".into());
            }
        }
        // CI mentions
        let ci = helpers::ci_mentions(
            ctx,
            &["coverage", "codecov", "coveralls", "tarpaulin", "llvm-cov", "c8", "pytest-cov"],
        );
        for item in &ci {
            if let Some(p) = &item.path {
                hits.push(p.clone());
            }
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No coverage configuration detected.")
                .remediation("Add coverage tooling config (codecov, tarpaulin, c8, pytest-cov).")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::Testing)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Coverage configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct CoverageEnforcement;
impl Rule for CoverageEnforcement {
    fn id(&self) -> &'static str {
        "testing.coverage_enforcement"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut items = helpers::ci_mentions(
            ctx,
            &[
                "coverage",
                "--fail-under",
                "codecov",
                "coveralls",
                "tarpaulin",
                "llvm-cov",
                "diff-cover",
            ],
        );

        // codecov / coveralls config often implies upload+gate intent
        for name in ["codecov.yml", "codecov.yaml", ".codecov.yml"] {
            if let Some(content) = ctx.read_text(name) {
                let l = content.to_ascii_lowercase();
                if l.contains("target") || l.contains("threshold") || l.contains("require") {
                    items.push(EvidenceItem::path_detail(name, "threshold/target configured"));
                } else {
                    items.push(EvidenceItem::path(name));
                }
            }
        }

        if items.is_empty() {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No coverage enforcement signal detected in CI or config.")
                .remediation("Fail builds when coverage drops below a configured threshold.")
                .build()
        } else {
            let enforced = items.iter().any(|i| {
                i.detail
                    .as_deref()
                    .is_some_and(|d| d.contains("threshold") || d.contains("fail-under") || d.contains("mentions"))
            });
            Finding::builder(self.id(), Category::Testing)
                .status(if enforced {
                    Status::Enforced
                } else {
                    Status::Partial
                })
                .confidence(Confidence::Medium)
                .summary(if enforced {
                    "Coverage enforcement signals detected."
                } else {
                    "Coverage tooling present; enforcement threshold unclear."
                })
                .evidence(items)
                .build()
        }
    }
}

struct StaticAnalysisCi;
impl Rule for StaticAnalysisCi {
    fn id(&self) -> &'static str {
        "testing.static_analysis_ci"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let items = helpers::ci_mentions(
            ctx,
            &[
                "clippy",
                "eslint",
                "golangci-lint",
                "ruff",
                "flake8",
                "pylint",
                "semgrep",
                "codeql",
                "sonar",
                "shellcheck",
                "rubocop",
            ],
        );

        if items.is_empty() {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No static analysis steps detected in CI workflows.")
                .remediation("Run linters/static analysis in CI (clippy, eslint, ruff, etc.).")
                .build()
        } else {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary("Static analysis steps detected in CI workflows.")
                .evidence(items)
                .build()
        }
    }
}

struct TypeCheckingCi;
impl Rule for TypeCheckingCi {
    fn id(&self) -> &'static str {
        "testing.type_checking_ci"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        let mut items = helpers::ci_mentions(
            ctx,
            &[
                "tsc",
                "typescript",
                "mypy",
                "pyright",
                "cargo check",
                "cargo build",
                "go build",
                "go vet",
            ],
        );

        // Strongly typed ecosystems compiling in CI count as type checking
        if signals.has_cargo {
            let cargo_ci = helpers::ci_mentions(ctx, &["cargo"]);
            items.extend(cargo_ci);
        }

        items.sort_by(|a, b| a.path.cmp(&b.path));
        items.dedup();

        if items.is_empty() {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No type-checking steps detected in CI workflows.")
                .remediation("Add tsc/mypy/cargo check (or equivalent) to CI.")
                .build()
        } else {
            Finding::builder(self.id(), Category::Testing)
                .status(Status::Enforced)
                .confidence(Confidence::High)
                .summary("Type-checking / compile checks detected in CI workflows.")
                .evidence(items)
                .build()
        }
    }
}
