//! File inventory and repository analysis context.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::error::Result;
use crate::git::GitMeta;

/// A discovered file in the repository.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path relative to repository root, using `/` separators.
    pub relative: String,
    /// Absolute path on disk.
    pub absolute: PathBuf,
    /// File size in bytes.
    pub size: u64,
}

/// Inventory of tracked/visible files in the repository.
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    /// Relative path (as stored) → entry.
    files: BTreeMap<String, FileEntry>,
    /// Lowercased relative path → canonical relative path.
    lower_index: HashMap<String, String>,
    /// Directory prefixes present (relative, `/`-separated, no trailing slash).
    dirs: BTreeSet<String>,
}

impl Inventory {
    /// Walk `root`, respecting `.gitignore`, and collect file metadata.
    pub fn scan(root: &Path) -> Result<Self> {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false);
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);
        builder.parents(true);

        let mut files = BTreeMap::new();
        let mut lower_index = HashMap::new();
        let mut dirs = BTreeSet::new();

        for entry in builder.build().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path == root {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let relative = normalize_rel(rel);
            if relative.is_empty() {
                continue;
            }

            // Skip .git contents entirely
            if relative == ".git" || relative.starts_with(".git/") {
                continue;
            }

            if entry.file_type().is_some_and(|t| t.is_dir()) {
                dirs.insert(relative);
                continue;
            }

            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let absolute = path.to_path_buf();
            lower_index.insert(relative.to_ascii_lowercase(), relative.clone());

            // Record parent directories
            if let Some(parent) = Path::new(&relative).parent() {
                let p = normalize_rel(parent);
                if !p.is_empty() {
                    dirs.insert(p);
                }
            }

            files.insert(
                relative.clone(),
                FileEntry {
                    relative,
                    absolute,
                    size,
                },
            );
        }

        Ok(Self {
            files,
            lower_index,
            dirs,
        })
    }

    /// Number of inventoried files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Iterate all file entries.
    pub fn iter(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.values()
    }

    /// Case-insensitive file lookup by relative path.
    pub fn get(&self, relative: &str) -> Option<&FileEntry> {
        let key = relative.replace('\\', "/");
        self.files.get(&key).or_else(|| {
            self.lower_index
                .get(&key.to_ascii_lowercase())
                .and_then(|canon| self.files.get(canon))
        })
    }

    /// Whether a file exists (case-insensitive).
    pub fn has_file(&self, relative: &str) -> bool {
        self.get(relative).is_some()
    }

    /// Whether a directory prefix exists.
    pub fn has_dir(&self, relative: &str) -> bool {
        let key = relative.trim_matches('/').replace('\\', "/");
        self.dirs.contains(&key) || self.files.keys().any(|f| f.starts_with(&format!("{key}/")))
    }

    /// Find files whose relative path matches a predicate.
    pub fn find_matching<F>(&self, mut pred: F) -> Vec<&FileEntry>
    where
        F: FnMut(&str) -> bool,
    {
        self.files.values().filter(|e| pred(&e.relative)).collect()
    }

    /// Find files matching any of the given basename patterns (case-insensitive).
    pub fn find_by_basenames(&self, names: &[&str]) -> Vec<&FileEntry> {
        let lower: Vec<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
        self.files
            .values()
            .filter(|e| {
                Path::new(&e.relative)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| lower.iter().any(|l| n.eq_ignore_ascii_case(l)))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Find files under a directory prefix matching a path substring (case-insensitive).
    pub fn find_path_contains(&self, needle: &str) -> Vec<&FileEntry> {
        let needle = needle.to_ascii_lowercase();
        self.files
            .values()
            .filter(|e| e.relative.to_ascii_lowercase().contains(&needle))
            .collect()
    }
}

/// Shared analysis context available to all rules.
#[derive(Debug)]
pub struct RepoContext {
    /// Canonical repository root.
    pub root: PathBuf,
    /// File inventory.
    pub inventory: Inventory,
    /// Git metadata snapshot.
    pub git: GitMeta,
    /// Lazily cached file contents (relative path → text).
    content_cache: std::sync::Mutex<HashMap<String, Option<String>>>,
}

impl RepoContext {
    pub fn new(root: PathBuf, inventory: Inventory, git: GitMeta) -> Self {
        Self {
            root,
            inventory,
            git,
            content_cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn has_file(&self, relative: &str) -> bool {
        self.inventory.has_file(relative)
    }

    pub fn has_dir(&self, relative: &str) -> bool {
        self.inventory.has_dir(relative)
    }

    /// Read a text file (cached). Returns `None` if missing or non-UTF8 / too large.
    pub fn read_text(&self, relative: &str) -> Option<String> {
        let entry = self.inventory.get(relative)?;
        let key = entry.relative.clone();

        {
            let cache = self.content_cache.lock().ok()?;
            if let Some(cached) = cache.get(&key) {
                return cached.clone();
            }
        }

        // Cap reads at 1 MiB for hygiene scanning
        let content = if entry.size > 1_048_576 {
            None
        } else {
            std::fs::read_to_string(&entry.absolute).ok()
        };

        if let Ok(mut cache) = self.content_cache.lock() {
            cache.insert(key, content.clone());
        }
        content
    }

    /// First existing file among candidates (case-insensitive), returning relative path.
    pub fn first_existing<'a>(&self, candidates: &[&'a str]) -> Option<&'a str> {
        candidates.iter().copied().find(|c| self.has_file(c))
    }

    /// Detect whether any path matches a glob-like suffix/prefix pattern set.
    pub fn any_path_matches(&self, needles: &[&str]) -> Vec<String> {
        let mut hits = Vec::new();
        for needle in needles {
            let n = needle.to_ascii_lowercase();
            for f in self.inventory.iter() {
                let rel = f.relative.to_ascii_lowercase();
                if rel == n || rel.ends_with(&format!("/{n}")) || rel.contains(&n) {
                    hits.push(f.relative.clone());
                }
            }
        }
        hits.sort();
        hits.dedup();
        hits
    }

    /// Parallel helper: evaluate many path existence checks.
    pub fn existing_of(&self, candidates: &[&str]) -> Vec<String> {
        candidates
            .par_iter()
            .filter(|c| self.has_file(c))
            .map(|c| (*c).to_string())
            .collect()
    }

    /// Detect tooling and ecosystem signals from the inventory.
    pub fn detect_signals(&self) -> ToolSignals {
        ToolSignals::detect(&self.inventory)
    }
}

/// Observable tooling / ecosystem signals (not opinions).
#[derive(Debug, Clone, Default)]
pub struct ToolSignals {
    pub has_cargo: bool,
    pub has_package_json: bool,
    pub has_pyproject: bool,
    pub has_requirements_txt: bool,
    pub has_go_mod: bool,
    pub has_gemfile: bool,
    pub has_maven: bool,
    pub has_gradle: bool,
    pub has_dotnet: bool,
    pub ci_workflow_paths: Vec<String>,
    pub pre_commit_config: bool,
    pub editorconfig: bool,
    pub gitignore: bool,
}

impl ToolSignals {
    fn detect(inv: &Inventory) -> Self {
        let ci = inv
            .find_matching(|p| {
                let l = p.to_ascii_lowercase();
                l.starts_with(".github/workflows/") && (l.ends_with(".yml") || l.ends_with(".yaml"))
            })
            .into_iter()
            .map(|e| e.relative.clone())
            .collect();

        Self {
            has_cargo: inv.has_file("Cargo.toml"),
            has_package_json: inv.has_file("package.json"),
            has_pyproject: inv.has_file("pyproject.toml") || inv.has_file("setup.py"),
            has_requirements_txt: inv.has_file("requirements.txt"),
            has_go_mod: inv.has_file("go.mod"),
            has_gemfile: inv.has_file("Gemfile"),
            has_maven: inv.has_file("pom.xml"),
            has_gradle: inv.has_file("build.gradle")
                || inv.has_file("build.gradle.kts")
                || inv.has_file("settings.gradle")
                || inv.has_file("settings.gradle.kts"),
            has_dotnet: !inv
                .find_matching(|p| p.ends_with(".csproj") || p.ends_with(".sln"))
                .is_empty(),
            ci_workflow_paths: ci,
            pre_commit_config: inv.has_file(".pre-commit-config.yaml")
                || inv.has_file(".pre-commit-config.yml"),
            editorconfig: inv.has_file(".editorconfig"),
            gitignore: inv.has_file(".gitignore"),
        }
    }
}

fn normalize_rel(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn inventory_finds_readme() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "x").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main(){}").unwrap();
        let inv = Inventory::scan(dir.path()).unwrap();
        assert!(inv.has_file("README.md"));
        assert!(inv.has_file("readme.md"));
        assert!(inv.has_dir("src"));
        assert_eq!(inv.len(), 2);
    }
}
