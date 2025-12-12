//! Git backend and repository traits.
//!
//! These traits abstract over git implementations for testability and flexibility.

use std::path::Path;

use chadreview_git_backend_models::{
    CommitInfo, DiffResult, GitBackendError, ResolvedRef, WorkingTreeDiffOptions,
};

/// Factory trait for opening git repositories.
///
/// This is the main abstraction point for testing - mock implementations
/// can provide deterministic repository state.
pub trait GitBackend: Send + Sync {
    /// Open a repository at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the repository root (containing `.git`).
    ///
    /// # Errors
    ///
    /// Returns `GitBackendError::RepoNotFound` if no repository exists at the path.
    fn open(&self, path: &Path) -> Result<Box<dyn GitRepository>, GitBackendError>;

    /// Discover a repository by walking up from the given path.
    ///
    /// This mimics `git rev-parse --git-dir` behavior, searching for a `.git`
    /// directory in the given path and its parents.
    ///
    /// # Arguments
    ///
    /// * `path` - Starting path to search from.
    ///
    /// # Errors
    ///
    /// Returns `GitBackendError::NotARepository` if no repository is found.
    fn discover(&self, path: &Path) -> Result<Box<dyn GitRepository>, GitBackendError>;
}

/// Operations on an opened git repository.
///
/// This trait defines all the git operations needed for diff viewing.
///
/// Note: This trait only requires `Send`, not `Sync`, because `git2::Repository`
/// is not thread-safe. Operations should be performed on a single thread or
/// protected by external synchronization.
pub trait GitRepository: Send {
    // === Reference Resolution ===

    /// Resolve a ref name (branch, tag, HEAD, sha) to commit info.
    ///
    /// # Arguments
    ///
    /// * `ref_name` - Reference to resolve (e.g., "main", "HEAD", "abc123").
    ///
    /// # Errors
    ///
    /// Returns `GitBackendError::RefNotFound` if the reference doesn't exist.
    fn resolve_ref(&self, ref_name: &str) -> Result<ResolvedRef, GitBackendError>;

    /// Get the merge base of two commits.
    ///
    /// This finds the best common ancestor of two commits, used for three-dot diffs.
    ///
    /// # Arguments
    ///
    /// * `commit1` - First commit SHA or ref.
    /// * `commit2` - Second commit SHA or ref.
    ///
    /// # Errors
    ///
    /// Returns an error if either commit doesn't exist or has no common ancestor.
    fn merge_base(&self, commit1: &str, commit2: &str) -> Result<String, GitBackendError>;

    // === Commit Information ===

    /// Get commit information for a SHA.
    ///
    /// # Arguments
    ///
    /// * `sha` - The commit SHA (full or partial).
    ///
    /// # Errors
    ///
    /// Returns `GitBackendError::CommitNotFound` if the commit doesn't exist.
    fn get_commit(&self, sha: &str) -> Result<CommitInfo, GitBackendError>;

    /// List commits in a range (base..head), newest first.
    ///
    /// # Arguments
    ///
    /// * `base` - Base commit SHA (exclusive).
    /// * `head` - Head commit SHA (inclusive).
    ///
    /// # Errors
    ///
    /// Returns an error if either commit doesn't exist.
    fn list_commits(&self, base: &str, head: &str) -> Result<Vec<CommitInfo>, GitBackendError>;

    // === Diff Operations ===

    /// Compute diff between two commits.
    ///
    /// # Arguments
    ///
    /// * `old_sha` - The "old" commit SHA.
    /// * `new_sha` - The "new" commit SHA.
    ///
    /// # Errors
    ///
    /// Returns an error if either commit doesn't exist.
    fn diff_commits(&self, old_sha: &str, new_sha: &str) -> Result<DiffResult, GitBackendError>;

    /// Diff a single commit against its parent(s).
    ///
    /// For merge commits, this diffs against the first parent.
    ///
    /// # Arguments
    ///
    /// * `sha` - The commit SHA.
    ///
    /// # Errors
    ///
    /// Returns an error if the commit doesn't exist.
    fn diff_commit(&self, sha: &str) -> Result<DiffResult, GitBackendError>;

    /// Diff working tree against a reference.
    ///
    /// # Arguments
    ///
    /// * `against` - Reference to diff against (e.g., "HEAD").
    /// * `options` - Options controlling what to include in the diff.
    ///
    /// # Errors
    ///
    /// Returns an error if the reference doesn't exist.
    fn diff_working_tree(
        &self,
        against: &str,
        options: WorkingTreeDiffOptions,
    ) -> Result<DiffResult, GitBackendError>;

    // === Repository Information ===

    /// Get the current HEAD SHA.
    ///
    /// # Errors
    ///
    /// Returns an error if HEAD is unborn (empty repository).
    fn head(&self) -> Result<String, GitBackendError>;

    /// Get the repository working directory.
    ///
    /// Returns `None` for bare repositories.
    fn workdir(&self) -> Option<&Path>;

    /// Check if the working tree has uncommitted changes.
    ///
    /// This includes staged, unstaged, and untracked files.
    ///
    /// # Errors
    ///
    /// Returns an error if the status cannot be determined.
    fn is_dirty(&self) -> Result<bool, GitBackendError>;
}
