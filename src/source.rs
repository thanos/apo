//! Resolve local paths and remote Git URIs into an analyzable workspace.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use tracing::info;

use crate::error::{Error, Result};

/// A resolved repository ready for analysis.
#[derive(Debug)]
pub struct Workspace {
    /// Local checkout path.
    pub path: PathBuf,
    /// User-facing repository identity (URI or local path).
    pub label: String,
    /// Original remote URI when the workspace was cloned.
    pub source_uri: Option<String>,
    /// Temporary directory holding a remote clone (cleaned up on drop).
    _temp: Option<TempDir>,
}

impl Workspace {
    /// Whether this workspace was cloned from a remote URI.
    pub fn is_remote(&self) -> bool {
        self.source_uri.is_some()
    }
}

/// Resolve `target` (local path or remote Git URI) into a [`Workspace`].
pub fn resolve(target: &str, clone_depth: usize) -> Result<Workspace> {
    let target = target.trim();
    if target.is_empty() {
        return Err(Error::Config("empty repository target".into()));
    }

    if looks_like_remote(target) {
        let uri = normalize_remote(target);
        return clone_remote(&uri, clone_depth);
    }

    let path = PathBuf::from(target);
    if !path.exists() {
        // Ambiguous: looks local but missing — if it resembles a host/path, hint.
        if target.contains('/') && !target.starts_with('.') && !target.starts_with('/') {
            return Err(Error::Config(format!(
                "path does not exist: {target} (did you mean a remote URI? try https://{target})"
            )));
        }
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("path does not exist: {target}"),
        )));
    }

    Ok(Workspace {
        path,
        label: target.to_string(),
        source_uri: None,
        _temp: None,
    })
}

/// Detect whether `target` is a remote Git URI rather than a local path.
pub fn looks_like_remote(target: &str) -> bool {
    let t = target.trim();
    if t.is_empty() {
        return false;
    }

    let lower = t.to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("git://")
        || lower.starts_with("ssh://")
        || lower.starts_with("git@")
        || lower.starts_with("file://")
    {
        return true;
    }

    // host:path SCP-like form (git@ omitted), e.g. github.com:org/repo.git
    if let Some((host, rest)) = t.split_once(':')
        && !host.contains('/')
        && !host.is_empty()
        && rest.contains('/')
        && !Path::new(t).exists()
    {
        // Exclude Windows drive letters (C:\...)
        if host.len() == 1 && host.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            return false;
        }
        return true;
    }

    false
}

fn normalize_remote(target: &str) -> String {
    let t = target.trim();
    if t.starts_with("git@") || t.contains("://") || looks_like_scp(t) {
        return t.to_string();
    }
    t.to_string()
}

fn looks_like_scp(t: &str) -> bool {
    t.split_once(':')
        .is_some_and(|(host, rest)| !host.contains('/') && rest.contains('/'))
}

fn clone_remote(uri: &str, depth: usize) -> Result<Workspace> {
    let depth = depth.max(1);
    let temp = TempDir::new().map_err(Error::Io)?;
    let dest = temp.path().join(repo_dirname(uri));

    info!(%uri, depth, dest = %dest.display(), "cloning remote repository");

    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            &depth.to_string(),
            "--quiet",
            uri,
            dest.to_str()
                .ok_or_else(|| Error::Config("clone destination path is not valid UTF-8".into()))?,
        ])
        .output()
        .map_err(|e| {
            Error::Git(format!(
                "failed to run git clone (is git installed and on PATH?): {e}"
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = [stderr.trim(), stdout.trim()]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or("git clone failed");
        return Err(Error::Git(format!("git clone {uri}: {detail}")));
    }

    if !dest.join(".git").exists() {
        return Err(Error::Git(format!(
            "git clone completed but no .git found at {}",
            dest.display()
        )));
    }

    Ok(Workspace {
        path: dest,
        label: uri.to_string(),
        source_uri: Some(uri.to_string()),
        _temp: Some(temp),
    })
}

fn repo_dirname(uri: &str) -> String {
    let trimmed = uri.trim_end_matches('/').trim_end_matches(".git");
    let name = trimmed.rsplit(['/', ':']).next().unwrap_or("repo").trim();
    if name.is_empty() {
        "repo".into()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_https_and_ssh() {
        assert!(looks_like_remote("https://github.com/thanos/ex_arrow"));
        assert!(looks_like_remote("http://example.com/r.git"));
        assert!(looks_like_remote("git@github.com:thanos/ex_arrow.git"));
        assert!(looks_like_remote(
            "ssh://git@github.com/thanos/ex_arrow.git"
        ));
        assert!(looks_like_remote("git://github.com/thanos/ex_arrow.git"));
        assert!(looks_like_remote("file:///tmp/foo.git"));
    }

    #[test]
    fn rejects_local_paths() {
        assert!(!looks_like_remote("."));
        assert!(!looks_like_remote("./repo"));
        assert!(!looks_like_remote("../repo"));
        assert!(!looks_like_remote("/tmp/repo"));
        assert!(!looks_like_remote("C:\\Users\\repo"));
    }

    #[test]
    fn dirname_from_uri() {
        assert_eq!(
            repo_dirname("https://github.com/thanos/ex_arrow"),
            "ex_arrow"
        );
        assert_eq!(
            repo_dirname("git@github.com:thanos/ex_arrow.git"),
            "ex_arrow"
        );
    }
}
