//! Helpers for naming report artifacts from a repository identity.

use std::path::Path;

use crate::report::Report;
use crate::source::looks_like_remote;

/// Sanitize a repository name for use in filenames.
pub fn sanitize_repo_name(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches('/').trim_end_matches(".git");
    let cleaned: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('-').trim_matches('_').to_string();
    if cleaned.is_empty() {
        "repository".into()
    } else {
        cleaned
    }
}

/// Derive a short repo name from a path or URI label.
pub fn repo_name_from_label(label: &str, checkout_path: Option<&str>) -> String {
    let raw = if looks_like_remote(label) {
        basename_from_remote(label)
    } else {
        let from_label = Path::new(label)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty() && *n != "." && *n != "..");
        let from_checkout = checkout_path
            .map(Path::new)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty());
        from_label
            .or(from_checkout)
            .unwrap_or("repository")
            .to_string()
    };
    sanitize_repo_name(&raw)
}

fn basename_from_remote(uri: &str) -> String {
    let trimmed = uri.trim_end_matches('/').trim_end_matches(".git");
    trimmed
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("repository")
        .trim()
        .to_string()
}

impl Report {
    /// Filename prefix for hygiene artifacts, e.g. `apo` → `apo-repository-hygiene.md`.
    pub fn artifact_prefix(&self) -> String {
        if let Some(uri) = &self.source_uri {
            return repo_name_from_label(uri, None);
        }
        // Prefer absolute checkout path so targets like `.` resolve to the real dirname.
        if let Some(path) = &self.checkout_path {
            return repo_name_from_label(path, None);
        }
        repo_name_from_label(&self.repository, None)
    }

    /// Default markdown report filename.
    pub fn markdown_filename(&self) -> String {
        format!("{}-repository-hygiene.md", self.artifact_prefix())
    }

    /// Default JSON report filename.
    pub fn json_filename(&self) -> String {
        format!("{}-repository-hygiene.json", self.artifact_prefix())
    }

    /// Default LLM remediation prompt filename.
    pub fn prompt_filename(&self) -> String {
        format!("{}-repository-hygiene-prompt.md", self.artifact_prefix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_path_and_uri() {
        assert_eq!(repo_name_from_label("/Users/thanos/work/apo", None), "apo");
        assert_eq!(
            repo_name_from_label("https://github.com/thanos/ex_arrow", None),
            "ex_arrow"
        );
        assert_eq!(
            repo_name_from_label("git@github.com:thanos/ex_arrow.git", None),
            "ex_arrow"
        );
        assert_eq!(sanitize_repo_name("my repo!"), "my-repo");
    }
}
