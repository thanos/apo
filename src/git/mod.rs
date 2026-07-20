//! Git metadata inspection via `gix`.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

/// Snapshot of repository Git metadata used by hygiene rules.
#[derive(Debug, Clone, Default)]
pub struct GitMeta {
    /// Whether a `.git` directory was found and opened.
    pub available: bool,
    /// HEAD commit id (hex), if any.
    pub head_id: Option<String>,
    /// Current branch name, if on a branch.
    pub branch: Option<String>,
    /// Number of commits sampled from HEAD history.
    pub commit_count_sampled: usize,
    /// Age of the most recent commit in days, if known.
    pub days_since_last_commit: Option<u64>,
    /// Fraction of sampled commits whose subjects match conventional commits.
    pub conventional_commit_ratio: Option<f64>,
    /// Fraction of sampled commits that reference an issue id (`#123` or `JIRA-1`).
    pub issue_link_ratio: Option<f64>,
    /// Sample of recent commit subjects (for evidence appendix).
    pub recent_subjects: Vec<String>,
    /// Error message if git inspection partially failed.
    pub note: Option<String>,
}

/// Inspect git metadata for `repo_root` using `git_dir`.
pub fn inspect(git_dir: &Path, repo_root: &Path, sample_limit: usize) -> Result<GitMeta> {
    let mut meta = GitMeta {
        available: true,
        ..GitMeta::default()
    };

    let repo = match gix::open(repo_root) {
        Ok(r) => r,
        Err(e) => match gix::open(git_dir.parent().unwrap_or(repo_root)) {
            Ok(r) => r,
            Err(_) => {
                meta.note = Some(format!("unable to open git repository: {e}"));
                return Ok(meta);
            }
        },
    };

    match repo.head() {
        Ok(head) => {
            if let Some(name) = head.referent_name() {
                let s = name.as_bstr().to_string();
                meta.branch = s
                    .strip_prefix("refs/heads/")
                    .map(str::to_string)
                    .or(Some(s));
            }
            match head.id() {
                Some(id) => {
                    let oid = id.detach();
                    meta.head_id = Some(oid.to_string());
                    enrich_history(&repo, oid, sample_limit, &mut meta);
                }
                None => {
                    meta.note = Some("repository has no commits yet".into());
                }
            }
        }
        Err(e) => {
            meta.note = Some(format!("unable to read HEAD: {e}"));
        }
    }

    Ok(meta)
}

fn enrich_history(
    repo: &gix::Repository,
    head: gix::ObjectId,
    sample_limit: usize,
    meta: &mut GitMeta,
) {
    let Ok(commit) = repo.find_commit(head) else {
        meta.note = Some("unable to load HEAD commit".into());
        return;
    };

    let mut ids = Vec::new();
    match repo.rev_walk([head]).all() {
        Ok(walk) => {
            for item in walk.take(sample_limit) {
                match item {
                    Ok(info) => ids.push(info.id),
                    Err(_) => break,
                }
            }
        }
        Err(_) => {
            ids.push(head);
        }
    }

    if ids.is_empty() {
        ids.push(head);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;

    if let Ok(time) = commit.time() {
        let ts = time.seconds;
        if ts > 0 && now >= ts {
            meta.days_since_last_commit = Some(((now - ts) as u64) / 86_400);
        }
    }

    let conventional_re = regex::Regex::new(
        r"(?i)^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?(!)?:",
    )
    .ok();
    let issue_re = regex::Regex::new(r"(?i)(#[0-9]+|[A-Z][A-Z0-9]+-\d+)").ok();

    let mut conventional = 0usize;
    let mut linked = 0usize;
    let mut subjects = Vec::new();

    for id in &ids {
        let Ok(c) = repo.find_commit(*id) else {
            continue;
        };
        let Ok(msg) = c.message_raw() else {
            continue;
        };
        let msg = msg.to_string();
        let subject = msg.lines().next().unwrap_or("").trim().to_string();
        if subjects.len() < 10 && !subject.is_empty() {
            subjects.push(subject.clone());
        }
        if let Some(re) = &conventional_re
            && re.is_match(&subject)
        {
            conventional += 1;
        }
        if let Some(re) = &issue_re
            && re.is_match(&msg)
        {
            linked += 1;
        }
    }

    let n = ids.len();
    meta.commit_count_sampled = n;
    meta.recent_subjects = subjects;
    if n > 0 {
        meta.conventional_commit_ratio = Some(conventional as f64 / n as f64);
        meta.issue_link_ratio = Some(linked as f64 / n as f64);
    }
}

/// Map gix errors into APO errors when a hard failure is required.
#[allow(dead_code)]
pub fn map_err(err: impl std::fmt::Display) -> Error {
    Error::Git(err.to_string())
}
