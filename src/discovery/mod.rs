//! Repository discovery and file inventory.

mod inventory;

pub use inventory::{FileEntry, Inventory, RepoContext};

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::git;

/// Discover a repository at `root` and build analysis context.
pub fn discover(root: &Path, commit_sample_limit: usize) -> Result<RepoContext> {
    let root = root.canonicalize().map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", root.display()),
        ))
    })?;

    if !root.is_dir() {
        return Err(Error::NotADirectory(root));
    }

    let git_dir = find_git_dir(&root)?;
    let inventory = Inventory::scan(&root)?;
    let git_meta = git::inspect(&git_dir, &root, commit_sample_limit)?;

    Ok(RepoContext::new(root, inventory, git_meta))
}

fn find_git_dir(root: &Path) -> Result<PathBuf> {
    let git = root.join(".git");
    if git.is_dir() {
        return Ok(git);
    }
    // Worktree / gitfile support
    if git.is_file() {
        let contents = std::fs::read_to_string(&git)?;
        if let Some(rest) = contents.strip_prefix("gitdir: ") {
            let path = rest.trim();
            let resolved = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                root.join(path)
            };
            if resolved.is_dir() {
                return Ok(resolved);
            }
        }
    }
    Err(Error::NotAGitRepository(root.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn rejects_non_git_directory() {
        let dir = tempdir().unwrap();
        let err = discover(dir.path(), 10).unwrap_err();
        assert!(matches!(err, Error::NotAGitRepository(_)));
    }

    #[test]
    fn discovers_simple_repo() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join("README.md"), "# hi\n").unwrap();
        let ctx = discover(dir.path(), 10).unwrap();
        assert!(ctx.has_file("README.md"));
    }
}
