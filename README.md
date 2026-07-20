# APO

**APO** (from Greek *apothiki* — "storehouse") is an **Engineering Evidence Platform**.

This repository ships the first analyzer: **Repository Hygiene**.

`apo` scans a local Git repository (or a remote Git URI) and collects **objective engineering hygiene evidence** — documentation, local development controls, testing gates, security/supply-chain signals, delivery automation, and collaboration practices.

Rules emit evidence only. A separate policy layer turns those observations into category and overall scores. APO does not grade “code quality” or run your tests; it reports whether engineering *controls* are observable in the repository.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Local checkout
apo analyze .
apo analyze . --format json
apo analyze . --output report.md
apo analyze . --format both --output ./out

# Remote Git URI (shallow-cloned to a temp dir, then cleaned up)
apo analyze https://github.com/thanos/ex_arrow
apo analyze git@github.com:thanos/ex_arrow.git --format both

# LLM remediation prompt (paste into Cursor/ChatGPT/etc. to close gaps)
apo analyze . --llm-prompt
apo prompt .
apo prompt https://github.com/thanos/ex_arrow --output ./out
```

Default artifacts:

- Local targets: written next to the analyzed repository
- Remote URIs: written to the current working directory

Files:

- `{repo}-repository-hygiene.md`
- `{repo}-repository-hygiene.json` (when `--format json` or `both`)
- `{repo}-repository-hygiene-prompt.md` (when `--llm-prompt` or `apo prompt`)

`{repo}` is the repository directory name (local) or the remote repo basename (e.g. `ex_arrow` from `https://github.com/thanos/ex_arrow`).

### LLM remediation prompt

`apo prompt` / `--llm-prompt` generates a paste-ready instructions file for an LLM coding agent. It includes:

- Repository identity and current weighted score
- Rubric priorities
- Controls already satisfied (do not redo)
- Enumerated gaps (`Missing` / `Partial` / `Unknown`) with remediations and evidence
- Constraints and a required changelog deliverable mapping files → APO rule ids

Example:

```bash
apo prompt . > /tmp/fix-hygiene.md   # also writes {repo}-repository-hygiene-prompt.md
# then paste into your LLM agent against the repo checkout
```

## Pipeline

```text
Repository → Discovery → Hygiene Rules → Evidence → Policy/Scoring → Markdown + JSON
```

1. **Discovery** — walk the tree (respecting `.gitignore`), index files, detect ecosystem signals (`Cargo.toml`, `package.json`, CI workflows, etc.), and sample recent Git history.
2. **Rules** — each rule inspects paths, file contents, CI workflow text, and/or commit metadata and emits one finding.
3. **Policy** — maps finding statuses to numeric weights, averages them per category, then applies the weighted rubric for the overall score.
4. **Report** — writes Markdown and/or JSON with summary, scores, findings, gaps, and recommendations.

---

## What gets measured

APO v0.1 runs **35 rules** across **6 categories**. Every finding includes:

| Field | Meaning |
|-------|---------|
| `rule` | Stable id, e.g. `documentation.readme` |
| `category` | One of the six hygiene categories |
| `status` | Observational outcome (see below) |
| `confidence` | `High` / `Medium` / `Low` how sure the observation is |
| `summary` | Short description of what was observed |
| `evidence` | Paths and/or detail strings backing the finding |
| `remediation` | Optional guidance when the control looks weak or missing |

### Status values

| Status | Meaning | Policy weight |
|--------|---------|---------------|
| `Enforced` | Control present and appears actively applied (e.g. keywords in docs, CI steps) | 100 |
| `Present` | Control artifact exists | 80 |
| `Partial` | Weak or incomplete signal | 45 |
| `Unknown` | Cannot decide from a local clone alone (or insufficient Git history) | 25 |
| `Missing` | No evidence found | 0 |
| `NotApplicable` | Control does not apply (excluded from averages) | — |

**Category score** = average status weight of that category’s findings (excluding `NotApplicable`).

**Overall score** = weighted average of category scores using the v0.1 rubric below. If a category has no scorable findings, its weight is redistributed proportionally among the remaining categories.

### Scoring rubric (`apo-hygiene-v0.1`)

| Category | Weight | Notes |
|----------|-------:|-------|
| Documentation & Onboarding | 20% | Foundation |
| Development Hygiene | 15% | Daily developer experience |
| Quality Assurance | 20% | Code health |
| Security & Supply Chain | 15% | Risk reduction |
| Automation & Delivery | 15% | Velocity + reliability |
| Project Management & Collaboration | 15% | Process maturity |

Gaps (`Missing`, `Partial`, `Unknown`) appear under **Missing controls** and feed **Recommendations** when a remediation string is set.

---

## Report contents

Both Markdown and JSON include:

| Section | Contents |
|---------|----------|
| Header / metadata | APO version, analyzer name (`repository-hygiene`), repository label, optional `source_uri` / `checkout_path`, timestamp |
| Executive summary | Overall score and counts of enforced / present / gap findings |
| Category scores | Per-category score plus counts of each status |
| Findings | All 35 rule results with evidence |
| Missing controls | Rule ids with gap status |
| Recommendations | Remediation text derived from gap findings |
| Evidence appendix (Markdown) | Compact table of rule → status → evidence paths |

---

## Rules catalog

### 1. Documentation & Onboarding (20% — Foundation)

| Rule id | What APO looks for |
|---------|-------------------|
| `documentation.readme` | `README.md` (or common variants). **Enforced** if content mentions quickstart/install/usage-style keywords. |
| `documentation.contributing` | `CONTRIBUTING.md` (root, `.github/`, or `docs/`). |
| `documentation.security` | `SECURITY.md` (root, `.github/`, or `docs/`). |
| `documentation.license` | `LICENSE` / `LICENCE` / `COPYING` (and common extensions). |
| `documentation.architecture` | `ARCHITECTURE.md`, `DESIGN.md`, or docs paths containing “architecture”. |
| `documentation.adrs` | ADR directories (`docs/adr`, `adr`, …) or ADR markdown files. |
| `documentation.runbooks` | Paths/dirs containing “runbook”. |
| `documentation.codeowners` | `CODEOWNERS` (root, `.github/`, `docs/`, `.gitlab/`). |
| `documentation.issue_templates` | `.github/ISSUE_TEMPLATE/` or issue template files. |
| `documentation.pr_templates` | Pull request template files (e.g. `.github/pull_request_template.md`). |

### 2. Development Hygiene (15% — Daily developer experience)

| Rule id | What APO looks for |
|---------|-------------------|
| `local_development.gitignore` | `.gitignore` |
| `local_development.editorconfig` | `.editorconfig` |
| `local_development.formatter` | Formatter configs (`rustfmt.toml`, Prettier, Black/Ruff in `pyproject.toml`, `.clang-format`, etc.) or format-related `package.json` signals. |
| `local_development.linter` | Linter configs (Clippy, ESLint, Ruff, golangci-lint, Flake8, Pylint, …) or lint scripts in `package.json`. |
| `local_development.type_checker` | `tsconfig.json` / MyPy / Pyright, or a strongly typed toolchain (`Cargo.toml`, `go.mod`). |
| `local_development.pre_commit_hooks` | `.pre-commit-config.yaml`, Husky, Lefthook, or related `package.json` deps. |
| `local_development.dev_environment` | Dev containers, Docker/Compose, Makefile/Justfile, or setup scripts. **Partial** if only CONTRIBUTING exists without setup automation. |

### 3. Quality Assurance (20% — Code health)

| Rule id | What APO looks for |
|---------|-------------------|
| `testing.framework` | Test dirs (`tests/`, `spec/`, `__tests__/`, …), test file naming patterns, or framework config (`package.json` test runners, pytest, Cargo tests). |
| `testing.coverage_config` | Coverage configs (`codecov.yml`, `.coveragerc`, …), coverage tooling in manifests, or coverage mentions in CI. |
| `testing.coverage_enforcement` | CI/config signals that coverage is gated (thresholds, `--fail-under`, codecov targets). **Partial** if tooling exists but enforcement is unclear. |
| `testing.static_analysis_ci` | Linters/SAST invoked in CI (Clippy, ESLint, Ruff, CodeQL, Semgrep, Sonar, …). |
| `testing.type_checking_ci` | Typecheck/compile steps in CI (`tsc`, MyPy, `cargo check`/`build`, `go build`/`vet`, …). |

### 4. Security & Supply Chain (15% — Risk reduction)

| Rule id | What APO looks for |
|---------|-------------------|
| `security.dependency_scanning` | Audit/deny configs or CI steps (`cargo audit`/`deny`, `npm audit`, Snyk, OSV, Trivy, CodeQL paths, …). |
| `security.secret_scanning` | Gitleaks / TruffleHog / detect-secrets configs or CI mentions; `.secrets.baseline`. |
| `security.policy` | Security policy document (`SECURITY.md`, etc.) — scored in the security category. |
| `security.dependency_updates` | Dependabot or Renovate configuration. |

### 5. Automation & Delivery (15% — Velocity + reliability)

| Rule id | What APO looks for |
|---------|-------------------|
| `delivery.ci_workflows` | CI configs: `.github/workflows/*.yml`, GitLab CI, CircleCI, Azure Pipelines, Jenkinsfile, Travis, Buildkite, … |
| `delivery.release_workflows` | Release/publish workflows (by name or body: release, publish, goreleaser, `cargo`/`npm` publish, …) or release-tooling configs. |
| `delivery.branch_protection` | Policy-as-code hints (e.g. `.github/settings.yml`). **Unknown** when nothing is checked in — platform branch protection cannot be verified from a local clone. |
| `delivery.required_status_checks` | CI presence plus optional settings-as-code for required checks. **Unknown** if workflows exist but platform enforcement cannot be verified locally. |

### 6. Project Management & Collaboration (15% — Process maturity)

| Rule id | What APO looks for |
|---------|-------------------|
| `collaboration.commit_convention` | Commitlint / convention docs, and/or Conventional Commits patterns in sampled commit subjects (≥70% → **Enforced**, some adoption → **Partial**). |
| `collaboration.issue_linkage` | Issue refs in commit messages (`#123`, `PROJ-1`) and/or PR templates that ask for issue linkage. |
| `collaboration.codeowners` | Same CODEOWNERS paths as documentation, scored under collaboration. |
| `collaboration.review_configuration` | PR templates, CODEOWNERS, settings-as-code, or auto-assign/reviewer configs. |
| `collaboration.maintenance_activity` | Age of latest commit from Git history: ≤30 days **Enforced**, ≤90 **Present**, ≤180 **Partial**, older **Missing**. **Unknown** if the repo has no commits or Git metadata is unavailable. |

---

## Git history signals

For collaboration rules, APO samples up to `commit_sample_limit` commits from `HEAD` (default **100**; also used as shallow-clone `--depth` for remote URIs) and records:

- Days since last commit
- Share of subjects matching Conventional Commits
- Share of commits referencing an issue id
- A small sample of recent commit subjects (as evidence detail)

---

## Design

- One crate, one binary (`apo`)
- Rules produce evidence; policy computes scores
- Detect tools rather than hard-code a single ecosystem
- Local-clone limits: branch protection and required checks on the hosting platform are reported as `Unknown` unless policy-as-code is checked in
- No AI required for v0.1

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- analyze . --format both
```

## License

MIT
