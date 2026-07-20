//! Local development control rules.

use crate::discovery::RepoContext;
use crate::evidence::{Category, Confidence, EvidenceItem, Finding, Status};
use crate::rules::Rule;
use crate::rules::helpers;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Gitignore),
        Box::new(Editorconfig),
        Box::new(Formatter),
        Box::new(Linter),
        Box::new(TypeChecker),
        Box::new(PreCommitHooks),
        Box::new(DevContainer),
    ]
}

struct Gitignore;
impl Rule for Gitignore {
    fn id(&self) -> &'static str {
        "local_development.gitignore"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::LocalDevelopment,
            &[".gitignore"],
            ".gitignore detected.",
            "No .gitignore detected.",
            "Add a .gitignore appropriate for this project's artifacts.",
        )
    }
}

struct Editorconfig;
impl Rule for Editorconfig {
    fn id(&self) -> &'static str {
        "local_development.editorconfig"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        helpers::file_presence(
            ctx,
            self.id(),
            Category::LocalDevelopment,
            &[".editorconfig"],
            ".editorconfig detected.",
            "No .editorconfig detected.",
            "Add .editorconfig for consistent editor defaults.",
        )
    }
}

struct Formatter;
impl Rule for Formatter {
    fn id(&self) -> &'static str {
        "local_development.formatter"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = helpers::find_configs(
            ctx,
            &[
                "rustfmt.toml",
                ".rustfmt.toml",
                ".prettierrc",
                ".prettierrc.js",
                ".prettierrc.cjs",
                ".prettierrc.json",
                ".prettierrc.yaml",
                ".prettierrc.yml",
                "prettier.config.js",
                "prettier.config.cjs",
                ".clang-format",
                "pyproject.toml",
            ],
        );

        // package.json scripts / prettier dep
        if let Some(pkg) = ctx.read_text("package.json") {
            let l = pkg.to_ascii_lowercase();
            if l.contains("\"prettier\"") || l.contains("\"format\"") {
                hits.push("package.json".into());
            }
        }
        if let Some(cargo) = ctx.read_text("Cargo.toml")
            && cargo.to_ascii_lowercase().contains("rustfmt")
        {
            hits.push("Cargo.toml".into());
        }
        // ruff / black in pyproject
        if let Some(py) = ctx.read_text("pyproject.toml") {
            let l = py.to_ascii_lowercase();
            if (l.contains("[tool.black]")
                || l.contains("[tool.ruff")
                || l.contains("[tool.isort]"))
                && !hits.iter().any(|h| h == "pyproject.toml")
            {
                hits.push("pyproject.toml".into());
            }
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No formatter configuration detected.")
                .remediation("Add formatter config (e.g. rustfmt.toml, .prettierrc, black/ruff).")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Formatter configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct Linter;
impl Rule for Linter {
    fn id(&self) -> &'static str {
        "local_development.linter"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = helpers::find_configs(
            ctx,
            &[
                "clippy.toml",
                ".clippy.toml",
                ".eslintrc",
                ".eslintrc.js",
                ".eslintrc.cjs",
                ".eslintrc.json",
                ".eslintrc.yml",
                "eslint.config.js",
                "eslint.config.mjs",
                "eslint.config.cjs",
                ".golangci.yml",
                ".golangci.yaml",
                "ruff.toml",
                ".flake8",
                "setup.cfg",
                "pylintrc",
                ".pylintrc",
                "tslint.json",
            ],
        );

        if let Some(pkg) = ctx.read_text("package.json") {
            let l = pkg.to_ascii_lowercase();
            if l.contains("eslint") || l.contains("\"lint\"") {
                hits.push("package.json".into());
            }
        }
        if let Some(py) = ctx.read_text("pyproject.toml") {
            let l = py.to_ascii_lowercase();
            if l.contains("[tool.ruff") || l.contains("[tool.pylint") || l.contains("flake8") {
                hits.push("pyproject.toml".into());
            }
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No linter configuration detected.")
                .remediation("Add linter config (eslint, clippy, ruff, golangci-lint, etc.).")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Linter configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct TypeChecker;
impl Rule for TypeChecker {
    fn id(&self) -> &'static str {
        "local_development.type_checker"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let signals = ctx.detect_signals();
        let mut hits = helpers::find_configs(
            ctx,
            &[
                "tsconfig.json",
                "jsconfig.json",
                "mypy.ini",
                ".mypy.ini",
                "pyrightconfig.json",
            ],
        );

        if let Some(py) = ctx.read_text("pyproject.toml") {
            let l = py.to_ascii_lowercase();
            if l.contains("[tool.mypy]") || l.contains("[tool.pyright]") || l.contains("mypy") {
                hits.push("pyproject.toml".into());
            }
        }

        // Rust/Go/Java are typed by default — mark Present when those ecosystems dominate.
        if signals.has_cargo {
            hits.push("Cargo.toml".into());
        }
        if signals.has_go_mod {
            hits.push("go.mod".into());
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No type checker configuration detected.")
                .remediation("Add tsconfig/mypy/pyright or adopt a typed language toolchain.")
                .build()
        } else {
            let summary = if signals.has_cargo || signals.has_go_mod {
                "Typed toolchain or type-checker configuration detected."
            } else {
                "Type checker configuration detected."
            };
            let mut b = Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary(summary);
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct PreCommitHooks;
impl Rule for PreCommitHooks {
    fn id(&self) -> &'static str {
        "local_development.pre_commit_hooks"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = Vec::new();
        for name in [
            ".pre-commit-config.yaml",
            ".pre-commit-config.yml",
            ".husky/pre-commit",
            "lefthook.yml",
            "lefthook.yaml",
            ".lefthook.yml",
        ] {
            if ctx.has_file(name) {
                hits.push(name.to_string());
            }
        }
        if ctx.has_dir(".husky") {
            hits.push(".husky".into());
        }
        if let Some(pkg) = ctx.read_text("package.json") {
            let l = pkg.to_ascii_lowercase();
            if l.contains("husky") || l.contains("lint-staged") || l.contains("simple-git-hooks") {
                hits.push("package.json".into());
            }
        }

        hits.sort();
        hits.dedup();

        if hits.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Missing)
                .confidence(Confidence::High)
                .summary("No pre-commit hook configuration detected.")
                .remediation("Add pre-commit, husky, or lefthook hook configuration.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Pre-commit hook configuration detected.");
            for h in hits {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}

struct DevContainer;
impl Rule for DevContainer {
    fn id(&self) -> &'static str {
        "local_development.dev_environment"
    }
    fn evaluate(&self, ctx: &RepoContext) -> Finding {
        let mut hits = Vec::new();
        for name in [
            ".devcontainer/devcontainer.json",
            "devcontainer.json",
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
            "Makefile",
            "justfile",
            "Justfile",
            "scripts/setup.sh",
            "script/setup",
            "bin/setup",
            "setup.sh",
            "CONTRIBUTING.md",
        ] {
            if ctx.has_file(name) {
                hits.push(name.to_string());
            }
        }
        if ctx.has_dir(".devcontainer") {
            hits.push(".devcontainer".into());
        }

        // Prefer stronger signals over CONTRIBUTING alone
        let strong: Vec<_> = hits
            .iter()
            .filter(|h| !h.eq_ignore_ascii_case("CONTRIBUTING.md"))
            .cloned()
            .collect();

        if strong.is_empty() && hits.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Missing)
                .confidence(Confidence::Medium)
                .summary("No dev container or setup scripts detected.")
                .remediation(
                    "Add .devcontainer, Dockerfile, or setup scripts for local onboarding.",
                )
                .build()
        } else if strong.is_empty() {
            Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Partial)
                .confidence(Confidence::Low)
                .summary("Only contribution docs found; no explicit setup automation detected.")
                .push_evidence(EvidenceItem::path("CONTRIBUTING.md"))
                .remediation("Add .devcontainer or setup scripts for reproducible local setup.")
                .build()
        } else {
            let mut b = Finding::builder(self.id(), Category::LocalDevelopment)
                .status(Status::Present)
                .confidence(Confidence::High)
                .summary("Dev environment / setup automation detected.");
            for h in strong {
                b = b.push_evidence(EvidenceItem::path(h));
            }
            b.build()
        }
    }
}
