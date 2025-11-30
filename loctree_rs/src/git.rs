//! Git operations for temporal awareness
//!
//! This module provides native git operations using libgit2 (git2 crate).
//! It enables loctree to analyze repository history and compare snapshots
//! across different commits.

use git2::{DiffOptions, Oid, Repository};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use time::{OffsetDateTime, format_description};

/// Error type for git operations
#[derive(Debug)]
pub enum GitError {
    /// Not a git repository
    NotARepository(String),
    /// Failed to resolve reference (branch, tag, commit)
    RefNotFound(String),
    /// Git operation failed
    OperationFailed(String),
    /// IO error
    IoError(std::io::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository(path) => {
                write!(f, "not a git repository: {}", path)
            }
            GitError::RefNotFound(reference) => {
                write!(f, "reference not found: {}", reference)
            }
            GitError::OperationFailed(msg) => {
                write!(f, "git operation failed: {}", msg)
            }
            GitError::IoError(e) => {
                write!(f, "IO error: {}", e)
            }
        }
    }
}

impl std::error::Error for GitError {}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::OperationFailed(e.message().to_string())
    }
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::IoError(e)
    }
}

/// Information about a commit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Full commit hash
    pub hash: String,
    /// Short commit hash (7 chars)
    pub short_hash: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp (ISO 8601)
    pub date: String,
    /// Unix timestamp
    pub timestamp: i64,
    /// Commit message (first line)
    pub message: String,
    /// Full commit message
    pub message_full: String,
}

/// Wrapper around a git repository
pub struct GitRepo {
    repo: Repository,
    path: PathBuf,
}

impl GitRepo {
    /// Discover a git repository from the given path
    /// Searches upward from the path to find .git directory
    pub fn discover(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::discover(path)
            .map_err(|_| GitError::NotARepository(path.display().to_string()))?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::NotARepository("bare repository".to_string()))?;

        Ok(Self {
            path: workdir.to_path_buf(),
            repo,
        })
    }

    /// Get the repository root path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the current HEAD commit hash
    pub fn head_commit(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Resolve a reference (branch, tag, commit hash, HEAD~n) to a commit hash
    pub fn resolve_ref(&self, reference: &str) -> Result<String, GitError> {
        // Try to parse as OID first (commit hash)
        if let Ok(oid) = Oid::from_str(reference)
            && self.repo.find_commit(oid).is_ok()
        {
            return Ok(oid.to_string());
        }

        // Try to resolve as a reference
        let obj = self
            .repo
            .revparse_single(reference)
            .map_err(|_| GitError::RefNotFound(reference.to_string()))?;

        let commit = obj.peel_to_commit().map_err(|_| {
            GitError::RefNotFound(format!("{} does not point to a commit", reference))
        })?;

        Ok(commit.id().to_string())
    }

    /// Get commit information for a given reference
    pub fn get_commit_info(&self, reference: &str) -> Result<CommitInfo, GitError> {
        let oid_str = self.resolve_ref(reference)?;
        let oid = Oid::from_str(&oid_str)?;
        let commit = self.repo.find_commit(oid)?;

        let author = commit.author();
        let time = commit.time();

        // Format timestamp
        let timestamp = time.seconds();
        let datetime =
            OffsetDateTime::from_unix_timestamp(timestamp).unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z")
            .unwrap_or_default();
        let date = datetime.format(&format).unwrap_or_default();

        let message_full = commit.message().unwrap_or("").to_string();
        let message = message_full.lines().next().unwrap_or("").to_string();

        Ok(CommitInfo {
            hash: oid_str.clone(),
            short_hash: oid_str.chars().take(7).collect(),
            author: author.name().unwrap_or("Unknown").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            date,
            timestamp,
            message,
            message_full,
        })
    }

    /// Get commit log for a file or the entire repository
    pub fn log(&self, file_path: Option<&Path>, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();

        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }

            let oid = oid_result?;
            let commit = self.repo.find_commit(oid)?;

            // If file_path is specified, check if the commit touches that file
            if let Some(path) = file_path
                && !self.commit_touches_file(&commit, path)?
            {
                continue;
            }

            let author = commit.author();
            let time = commit.time();
            let timestamp = time.seconds();
            let datetime = OffsetDateTime::from_unix_timestamp(timestamp)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            let format =
                format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z")
                    .unwrap_or_default();
            let date = datetime.format(&format).unwrap_or_default();

            let message_full = commit.message().unwrap_or("").to_string();
            let message = message_full.lines().next().unwrap_or("").to_string();

            commits.push(CommitInfo {
                hash: oid.to_string(),
                short_hash: oid.to_string().chars().take(7).collect(),
                author: author.name().unwrap_or("Unknown").to_string(),
                author_email: author.email().unwrap_or("").to_string(),
                date,
                timestamp,
                message,
                message_full,
            });
        }

        Ok(commits)
    }

    /// Check if a commit modifies a specific file
    fn commit_touches_file(
        &self,
        commit: &git2::Commit,
        file_path: &Path,
    ) -> Result<bool, GitError> {
        let tree = commit.tree()?;

        // Get parent tree (if exists)
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        opts.pathspec(file_path);

        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

        Ok(diff.deltas().count() > 0)
    }

    /// Get the list of files changed between two commits
    pub fn changed_files(&self, from: &str, to: &str) -> Result<Vec<ChangedFile>, GitError> {
        let from_oid = Oid::from_str(&self.resolve_ref(from)?)?;
        let to_oid = Oid::from_str(&self.resolve_ref(to)?)?;

        let from_commit = self.repo.find_commit(from_oid)?;
        let to_commit = self.repo.find_commit(to_oid)?;

        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

        let mut files = Vec::new();

        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => ChangeStatus::Added,
                git2::Delta::Deleted => ChangeStatus::Deleted,
                git2::Delta::Modified => ChangeStatus::Modified,
                git2::Delta::Renamed => ChangeStatus::Renamed,
                git2::Delta::Copied => ChangeStatus::Copied,
                _ => ChangeStatus::Modified,
            };

            let old_path = delta.old_file().path().map(|p| p.to_path_buf());
            let new_path = delta.new_file().path().map(|p| p.to_path_buf());

            files.push(ChangedFile {
                old_path,
                new_path,
                status,
            });
        }

        Ok(files)
    }

    /// Get file content at a specific commit
    pub fn file_content_at(&self, reference: &str, file_path: &Path) -> Result<String, GitError> {
        let oid_str = self.resolve_ref(reference)?;
        let oid = Oid::from_str(&oid_str)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let entry = tree.get_path(file_path).map_err(|_| {
            GitError::OperationFailed(format!(
                "file '{}' not found at commit {}",
                file_path.display(),
                &oid_str[..7]
            ))
        })?;

        let blob = self.repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content())
            .map_err(|_| GitError::OperationFailed("file is not valid UTF-8".to_string()))?;

        Ok(content.to_string())
    }

    /// List all files in the repository at a specific commit
    pub fn list_files_at(&self, reference: &str) -> Result<Vec<PathBuf>, GitError> {
        let oid_str = self.resolve_ref(reference)?;
        let oid = Oid::from_str(&oid_str)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let mut files = Vec::new();
        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let path = if dir.is_empty() {
                    PathBuf::from(entry.name().unwrap_or(""))
                } else {
                    PathBuf::from(dir).join(entry.name().unwrap_or(""))
                };
                files.push(path);
            }
            git2::TreeWalkResult::Ok
        })?;

        Ok(files)
    }
}

/// Status of a changed file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
}

/// Information about a changed file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangedFile {
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
    pub status: ChangeStatus,
}

impl ChangedFile {
    /// Get the effective path (new_path for added/modified, old_path for deleted)
    pub fn path(&self) -> Option<&Path> {
        self.new_path.as_deref().or(self.old_path.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, GitRepo) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create initial file and commit
        std::fs::write(path.join("main.ts"), "export function main() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();

        let repo = GitRepo::discover(path).unwrap();
        (temp_dir, repo)
    }

    #[test]
    fn test_discover_git_repo() {
        let (temp_dir, repo) = create_test_repo();
        // Canonicalize paths to handle macOS /private/var vs /var symlink
        let expected = temp_dir.path().canonicalize().unwrap();
        let actual = repo.path().canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_discover_non_git_dir_fails() {
        let temp_dir = TempDir::new().unwrap();
        let result = GitRepo::discover(temp_dir.path());
        assert!(matches!(result, Err(GitError::NotARepository(_))));
    }

    #[test]
    fn test_head_commit() {
        let (_temp_dir, repo) = create_test_repo();
        let head = repo.head_commit().unwrap();
        assert_eq!(head.len(), 40); // SHA-1 hash length
    }

    #[test]
    fn test_resolve_head() {
        let (_temp_dir, repo) = create_test_repo();
        let head = repo.resolve_ref("HEAD").unwrap();
        assert_eq!(head.len(), 40);
    }

    #[test]
    fn test_get_commit_info() {
        let (_temp_dir, repo) = create_test_repo();
        let info = repo.get_commit_info("HEAD").unwrap();
        assert_eq!(info.author, "Test User");
        assert_eq!(info.message, "Initial commit");
        assert_eq!(info.short_hash.len(), 7);
    }

    #[test]
    fn test_log() {
        let (_temp_dir, repo) = create_test_repo();
        let commits = repo.log(None, 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "Initial commit");
    }

    #[test]
    fn test_file_content_at() {
        let (_temp_dir, repo) = create_test_repo();
        let content = repo.file_content_at("HEAD", Path::new("main.ts")).unwrap();
        assert_eq!(content, "export function main() {}");
    }

    #[test]
    fn test_list_files_at() {
        let (_temp_dir, repo) = create_test_repo();
        let files = repo.list_files_at("HEAD").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("main.ts"));
    }

    #[test]
    fn test_changed_files() {
        let (temp_dir, repo) = create_test_repo();
        let path = temp_dir.path();

        // Make another commit with a new file
        std::fs::write(path.join("utils.ts"), "export function add() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add utils"])
            .current_dir(path)
            .output()
            .unwrap();

        let changes = repo.changed_files("HEAD~1", "HEAD").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, ChangeStatus::Added);
        assert_eq!(changes[0].new_path, Some(PathBuf::from("utils.ts")));
    }
}
