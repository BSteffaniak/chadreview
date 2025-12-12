#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Git backend models for `ChadReview`.
//!
//! This crate defines the data types returned by git backend operations,
//! abstracting over the specific git implementation (git2, CLI, etc.).

use serde::{Deserialize, Serialize};

/// Result of a diff operation containing all changed files.
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// List of files with changes.
    pub files: Vec<FileDiff>,
}

/// A single file's diff information.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Path in the old tree (None for added files).
    pub old_path: Option<String>,
    /// Path in the new tree (None for deleted files).
    pub new_path: Option<String>,
    /// Status of the file change.
    pub status: DiffStatus,
    /// Unified diff patch text (None for binary files).
    pub patch: Option<String>,
    /// Whether this is a binary file.
    pub binary: bool,
}

/// Status of a file in a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffStatus {
    /// File was added.
    Added,
    /// File was deleted.
    Deleted,
    /// File was modified.
    Modified,
    /// File was renamed.
    Renamed,
    /// File was copied.
    Copied,
    /// File is untracked (working tree only).
    Untracked,
}

/// Git commit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Full SHA of the commit.
    pub sha: String,
    /// Short SHA (first 7 characters).
    pub short_sha: String,
    /// Commit message (full).
    pub message: String,
    /// First line of the commit message.
    pub summary: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Unix timestamp of the commit.
    pub timestamp: i64,
    /// Parent commit SHAs.
    pub parent_shas: Vec<String>,
}

/// Result of resolving a git reference.
#[derive(Debug, Clone)]
pub struct ResolvedRef {
    /// The resolved commit SHA.
    pub sha: String,
    /// The original ref name.
    pub name: String,
    /// Type of the reference.
    pub ref_type: RefType,
}

/// Type of a git reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    /// Local branch.
    Branch,
    /// Tag.
    Tag,
    /// Direct commit SHA.
    Commit,
    /// HEAD reference.
    Head,
    /// Remote tracking branch.
    Remote,
}

/// Errors from git backend operations.
#[derive(Debug, thiserror::Error)]
pub enum GitBackendError {
    /// Repository not found at the specified path.
    #[error("Repository not found at {path}")]
    RepoNotFound {
        /// The path that was searched.
        path: String,
    },

    /// Reference (branch, tag, commit) not found.
    #[error("Ref not found: {ref_name}")]
    RefNotFound {
        /// The reference name that wasn't found.
        ref_name: String,
    },

    /// Commit not found.
    #[error("Commit not found: {sha}")]
    CommitNotFound {
        /// The SHA that wasn't found.
        sha: String,
    },

    /// Path is not a git repository.
    #[error("Not a git repository: {path}")]
    NotARepository {
        /// The path that isn't a repository.
        path: String,
    },

    /// Invalid diff specification.
    #[error("Invalid diff specification: {message}")]
    InvalidDiffSpec {
        /// Description of what's invalid.
        message: String,
    },

    /// General git operation error.
    #[error("Git operation failed: {message}")]
    GitError {
        /// Error message from the underlying git implementation.
        message: String,
    },

    /// I/O error.
    #[error("I/O error: {message}")]
    IoError {
        /// Error message.
        message: String,
    },
}

impl From<std::io::Error> for GitBackendError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            message: err.to_string(),
        }
    }
}

/// Options for working tree diff operations.
#[derive(Debug, Clone, Default)]
pub struct WorkingTreeDiffOptions {
    /// Only include staged changes (git diff --cached).
    pub staged_only: bool,
    /// Include untracked files.
    pub include_untracked: bool,
    /// Include ignored files.
    pub include_ignored: bool,
}
