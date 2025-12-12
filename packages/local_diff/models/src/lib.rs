#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! Local diff models for `ChadReview`.
//!
//! This crate defines the data types for specifying and describing local git diffs.

use std::collections::BTreeMap;

use chadreview_git_backend_models::CommitInfo;
use serde::{Deserialize, Serialize};

/// Specification of what to diff - supports all diff types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffSpec {
    /// Compare two refs/commits: `base..head`
    Range {
        /// Base reference (branch, tag, or commit SHA).
        base: String,
        /// Head reference (branch, tag, or commit SHA).
        head: String,
        /// If true, use merge-base semantics (three-dot: `base...head`).
        /// This shows changes on head since it diverged from base.
        three_dot: bool,
    },

    /// Working tree changes.
    WorkingTree {
        /// Compare against this ref (default: "HEAD").
        against: String,
        /// Only staged changes (`git diff --cached`).
        staged_only: bool,
        /// Include untracked files in the diff.
        include_untracked: bool,
    },

    /// Single commit (shows commit vs its parent).
    Commit {
        /// The commit SHA.
        sha: String,
    },

    /// Multiple specific commits (combined/concatenated diff view).
    /// Useful for reviewing cherry-picked commits or non-contiguous history.
    Commits {
        /// Commits in order to be displayed.
        shas: Vec<String>,
        /// How to combine the commits.
        mode: MultiCommitMode,
    },
}

/// How to combine multiple commits in a diff view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MultiCommitMode {
    /// Show each commit's diff separately (default).
    #[default]
    Separate,
    /// Squash all commits into a single combined diff.
    Squashed,
}

/// Errors when parsing a diff specification.
#[derive(Debug, thiserror::Error)]
pub enum DiffSpecError {
    /// A required parameter is missing.
    #[error("Missing required parameter: {0}")]
    MissingParam(&'static str),

    /// A parameter has an invalid value.
    #[error("Invalid parameter: {0}")]
    InvalidParam(&'static str),

    /// Conflicting parameters were provided.
    #[error("Conflicting parameters: {0}")]
    ConflictingParams(String),
}

impl Default for DiffSpec {
    fn default() -> Self {
        Self::WorkingTree {
            against: "HEAD".to_string(),
            staged_only: false,
            include_untracked: true,
        }
    }
}

impl DiffSpec {
    /// Parse a `DiffSpec` from query parameters.
    ///
    /// Priority order:
    /// 1. `commits=sha1,sha2,sha3` (multiple specific commits)
    /// 2. `commit=sha` (single commit)
    /// 3. `base=X&head=Y` (range)
    /// 4. Otherwise: working tree against HEAD (default)
    ///
    /// # Errors
    ///
    /// Returns an error if parameters are invalid or conflicting.
    pub fn from_query(params: &BTreeMap<String, String>) -> Result<Self, DiffSpecError> {
        // Multiple specific commits
        if let Some(commits) = params.get("commits") {
            let shas: Vec<String> = commits
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if shas.is_empty() {
                return Err(DiffSpecError::InvalidParam("commits cannot be empty"));
            }

            let mode = params
                .get("mode")
                .map(|m| match m.as_str() {
                    "squashed" => MultiCommitMode::Squashed,
                    _ => MultiCommitMode::Separate,
                })
                .unwrap_or_default();

            return Ok(Self::Commits { shas, mode });
        }

        // Single commit
        if let Some(sha) = params.get("commit") {
            if sha.is_empty() {
                return Err(DiffSpecError::InvalidParam("commit cannot be empty"));
            }
            return Ok(Self::Commit { sha: sha.clone() });
        }

        // Range diff
        if let (Some(base), Some(head)) = (params.get("base"), params.get("head")) {
            if base.is_empty() {
                return Err(DiffSpecError::InvalidParam("base cannot be empty"));
            }
            if head.is_empty() {
                return Err(DiffSpecError::InvalidParam("head cannot be empty"));
            }

            let three_dot = params
                .get("three_dot")
                .or_else(|| params.get("merge_base"))
                .is_some_and(|v| v == "true" || v == "1");

            return Ok(Self::Range {
                base: base.clone(),
                head: head.clone(),
                three_dot,
            });
        }

        // Check for partial range (error case)
        if params.contains_key("base") && !params.contains_key("head") {
            return Err(DiffSpecError::MissingParam(
                "head (required when base is specified)",
            ));
        }
        if params.contains_key("head") && !params.contains_key("base") {
            return Err(DiffSpecError::MissingParam(
                "base (required when head is specified)",
            ));
        }

        // Default to working tree
        let against = params
            .get("against")
            .cloned()
            .unwrap_or_else(|| "HEAD".to_string());

        let staged_only = params
            .get("staged")
            .is_some_and(|v| v == "true" || v == "1");

        let include_untracked = params
            .get("untracked")
            .is_none_or(|v| v != "false" && v != "0");

        Ok(Self::WorkingTree {
            against,
            staged_only,
            include_untracked,
        })
    }

    /// Human-readable description of this diff spec.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Range {
                base,
                head,
                three_dot,
            } => {
                if *three_dot {
                    format!("{base}...{head}")
                } else {
                    format!("{base}..{head}")
                }
            }
            Self::WorkingTree {
                against,
                staged_only,
                ..
            } => {
                if *staged_only {
                    format!("Staged changes (vs {against})")
                } else {
                    format!("Working tree changes (vs {against})")
                }
            }
            Self::Commit { sha } => {
                format!("Commit {}", &sha[..sha.len().min(7)])
            }
            Self::Commits { shas, mode } => {
                let count = shas.len();
                match mode {
                    MultiCommitMode::Separate => format!("{count} commits"),
                    MultiCommitMode::Squashed => format!("{count} commits (squashed)"),
                }
            }
        }
    }

    /// Convert to query string format.
    #[must_use]
    pub fn to_query_string(&self) -> String {
        match self {
            Self::Range {
                base,
                head,
                three_dot,
            } => {
                let mut s = format!("base={base}&head={head}");
                if *three_dot {
                    s.push_str("&three_dot=true");
                }
                s
            }
            Self::WorkingTree {
                against,
                staged_only,
                include_untracked,
            } => {
                let mut s = format!("against={against}");
                if *staged_only {
                    s.push_str("&staged=true");
                }
                if !*include_untracked {
                    s.push_str("&untracked=false");
                }
                s
            }
            Self::Commit { sha } => format!("commit={sha}"),
            Self::Commits { shas, mode } => {
                let mut s = format!("commits={}", shas.join(","));
                if *mode == MultiCommitMode::Squashed {
                    s.push_str("&mode=squashed");
                }
                s
            }
        }
    }
}

/// Metadata about a local diff view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDiffInfo {
    /// Repository root path.
    pub repo_path: String,
    /// What's being diffed.
    pub spec: DiffSpec,
    /// Human-readable description.
    pub description: String,
    /// Commits involved (empty for working tree diffs).
    pub commits: Vec<CommitInfo>,
    /// Base ref (if applicable).
    pub base_ref: Option<String>,
    /// Head ref (if applicable).
    pub head_ref: Option<String>,
    /// Total additions across all files.
    pub total_additions: u64,
    /// Total deletions across all files.
    pub total_deletions: u64,
    /// Number of files changed.
    pub files_changed: usize,
    /// Whether working tree is dirty (has uncommitted changes).
    pub is_dirty: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn test_parse_default() {
        let params = BTreeMap::new();
        let spec = DiffSpec::from_query(&params).unwrap();
        assert_eq!(
            spec,
            DiffSpec::WorkingTree {
                against: "HEAD".to_string(),
                staged_only: false,
                include_untracked: true,
            }
        );
    }

    #[test]
    fn test_parse_working_tree_staged() {
        let params = make_params(&[("staged", "true")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert_eq!(
            spec,
            DiffSpec::WorkingTree {
                against: "HEAD".to_string(),
                staged_only: true,
                include_untracked: true,
            }
        );
    }

    #[test]
    fn test_parse_range() {
        let params = make_params(&[("base", "main"), ("head", "feature")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert_eq!(
            spec,
            DiffSpec::Range {
                base: "main".to_string(),
                head: "feature".to_string(),
                three_dot: false,
            }
        );
    }

    #[test]
    fn test_parse_range_three_dot() {
        let params = make_params(&[("base", "main"), ("head", "feature"), ("three_dot", "true")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert!(matches!(
            spec,
            DiffSpec::Range {
                three_dot: true,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_single_commit() {
        let params = make_params(&[("commit", "abc123")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert_eq!(
            spec,
            DiffSpec::Commit {
                sha: "abc123".to_string()
            }
        );
    }

    #[test]
    fn test_parse_multiple_commits() {
        let params = make_params(&[("commits", "abc,def,ghi")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert_eq!(
            spec,
            DiffSpec::Commits {
                shas: vec!["abc".to_string(), "def".to_string(), "ghi".to_string()],
                mode: MultiCommitMode::Separate,
            }
        );
    }

    #[test]
    fn test_parse_multiple_commits_squashed() {
        let params = make_params(&[("commits", "abc,def"), ("mode", "squashed")]);
        let spec = DiffSpec::from_query(&params).unwrap();
        assert!(matches!(
            spec,
            DiffSpec::Commits {
                mode: MultiCommitMode::Squashed,
                ..
            }
        ));
    }

    #[test]
    fn test_description() {
        assert_eq!(
            DiffSpec::Range {
                base: "main".to_string(),
                head: "feature".to_string(),
                three_dot: false
            }
            .description(),
            "main..feature"
        );

        assert_eq!(
            DiffSpec::Range {
                base: "main".to_string(),
                head: "feature".to_string(),
                three_dot: true
            }
            .description(),
            "main...feature"
        );

        assert_eq!(
            DiffSpec::WorkingTree {
                against: "HEAD".to_string(),
                staged_only: true,
                include_untracked: true
            }
            .description(),
            "Staged changes (vs HEAD)"
        );

        assert_eq!(
            DiffSpec::Commit {
                sha: "abc123def456".to_string()
            }
            .description(),
            "Commit abc123d"
        );
    }

    #[test]
    fn test_to_query_string() {
        let spec = DiffSpec::Range {
            base: "main".to_string(),
            head: "feature".to_string(),
            three_dot: true,
        };
        assert_eq!(
            spec.to_query_string(),
            "base=main&head=feature&three_dot=true"
        );
    }

    #[test]
    fn test_error_partial_range() {
        let params = make_params(&[("base", "main")]);
        let result = DiffSpec::from_query(&params);
        assert!(result.is_err());
    }
}
