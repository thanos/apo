//! CLI smoke tests.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::tempdir;

use std::fs;
use std::process::Command;

fn init_tiny_repo(root: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(root)
        .status()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "apo@example.com"])
        .current_dir(root)
        .status()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "APO Test"])
        .current_dir(root)
        .status()
        .unwrap();
    fs::write(root.join("README.md"), "# tiny\n").unwrap();
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(root)
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: init"])
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "APO Test")
        .env("GIT_AUTHOR_EMAIL", "apo@example.com")
        .env("GIT_COMMITTER_NAME", "APO Test")
        .env("GIT_COMMITTER_EMAIL", "apo@example.com")
        .status()
        .unwrap();
}

#[test]
fn analyze_json_writes_and_prints() {
    let dir = tempdir().unwrap();
    init_tiny_repo(dir.path());

    cargo_bin_cmd!("apo")
        .args([
            "analyze",
            dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--output",
            dir.path().join("report.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("repository-hygiene"))
        .stdout(predicate::str::contains("documentation.readme"));

    let written = fs::read_to_string(dir.path().join("report.json")).unwrap();
    assert!(written.contains("\"analyzer\": \"repository-hygiene\""));
}
