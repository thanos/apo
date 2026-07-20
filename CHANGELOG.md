# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-20

First public release of **APO** — Engineering Evidence Platform — with the Repository Hygiene analyzer.

### Added

- Single-crate Rust 2024 CLI/library `apo` for local and remote Git repository hygiene analysis
- 35 observational rules across six weighted rubric categories (`apo-hygiene-v0.1`):
  - Documentation & Onboarding (20%)
  - Development Hygiene (15%)
  - Quality Assurance (20%)
  - Security & Supply Chain (15%)
  - Automation & Delivery (15%)
  - Project Management & Collaboration (15%)
- Markdown and JSON reports with scores, findings, gaps, recommendations, and evidence appendix
- Remote Git URI support via shallow clone to a temporary checkout
- `apo prompt` / `analyze --llm-prompt` LLM remediation prompts for closing hygiene gaps
- Repo-prefixed default artifacts (`{repo}-repository-hygiene.md|.json|-prompt.md`)
- CI (fmt, clippy, multi-OS test, coverage, audit/`cargo deny`, publish dry-run)
- Release workflow: multi-platform binaries + GitHub Release + crates.io publish on `v*` tags
- Dependabot + scheduled dependency workflow

### Notes

- Branch protection / required status checks cannot be verified from a local clone alone (reported as `Unknown` unless policy-as-code is present)
- MSRV: Rust **1.85** (edition 2024)
- Uses `gix` 0.85 (`revision` + `sha1`) for Git history sampling

[0.1.0]: https://github.com/thanos/apo/releases/tag/v0.1.0
