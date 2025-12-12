//! Local diff provider implementation.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use chadreview_diff::parse_unified_diff;
use chadreview_git_backend::{GitBackend, GitRepository, WorkingTreeDiffOptions};
use chadreview_git_backend_models::{DiffResult, DiffStatus};
use chadreview_local_diff_models::{DiffSpec, LocalDiffInfo, MultiCommitMode};
use chadreview_pr_models::{DiffFile, FileStatus};
use chadreview_syntax::SyntaxHighlighter;

/// Provider for local git diff operations.
///
/// This provider uses a `GitBackend` implementation to extract diffs from
/// local git repositories.
pub struct LocalDiffProvider<B: GitBackend> {
    backend: Arc<B>,
    repo_path: PathBuf,
}

impl<B: GitBackend> LocalDiffProvider<B> {
    /// Create a new provider for a specific repository path.
    ///
    /// # Arguments
    ///
    /// * `backend` - The git backend implementation to use.
    /// * `repo_path` - Path to the repository root.
    #[must_use]
    pub const fn new(backend: Arc<B>, repo_path: PathBuf) -> Self {
        Self { backend, repo_path }
    }

    /// Create a provider by discovering a repository from the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if no repository is found or if the repository is bare.
    pub fn from_cwd(backend: Arc<B>) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Self::from_path(backend, &cwd)
    }

    /// Create a provider by discovering a repository from a given path.
    ///
    /// # Arguments
    ///
    /// * `backend` - The git backend implementation to use.
    /// * `path` - Path to search from (can be anywhere in the repository).
    ///
    /// # Errors
    ///
    /// Returns an error if no repository is found or if the repository is bare.
    pub fn from_path(backend: Arc<B>, path: &Path) -> Result<Self> {
        let repo = backend.discover(path)?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?
            .to_path_buf();
        Ok(Self { backend, repo_path })
    }

    /// Get the repository path.
    #[must_use]
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Get the underlying backend.
    #[must_use]
    pub const fn backend(&self) -> &Arc<B> {
        &self.backend
    }

    fn open_repo(&self) -> Result<Box<dyn GitRepository>> {
        self.backend
            .open(&self.repo_path)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Get diff files for a given specification.
    ///
    /// # Arguments
    ///
    /// * `spec` - The diff specification describing what to diff.
    ///
    /// # Errors
    ///
    /// Returns an error if the diff cannot be computed.
    pub fn get_diff(&self, spec: &DiffSpec) -> Result<Vec<DiffFile>> {
        let repo = self.open_repo()?;
        let highlighter = SyntaxHighlighter::new();

        let diff_result = match spec {
            DiffSpec::Range {
                base,
                head,
                three_dot,
            } => {
                let base_sha = if *three_dot {
                    let base_resolved = repo.resolve_ref(base)?.sha;
                    let head_resolved = repo.resolve_ref(head)?.sha;
                    repo.merge_base(&base_resolved, &head_resolved)?
                } else {
                    repo.resolve_ref(base)?.sha
                };
                let head_sha = repo.resolve_ref(head)?.sha;
                repo.diff_commits(&base_sha, &head_sha)?
            }

            DiffSpec::WorkingTree {
                against,
                staged_only,
                include_untracked,
            } => {
                let options = WorkingTreeDiffOptions {
                    staged_only: *staged_only,
                    include_untracked: *include_untracked,
                    include_ignored: false,
                };
                repo.diff_working_tree(against, options)?
            }

            DiffSpec::Commit { sha } => repo.diff_commit(sha)?,

            DiffSpec::Commits { shas, mode } => match mode {
                MultiCommitMode::Separate => {
                    // Concatenate all commit diffs
                    let mut all_files = Vec::new();
                    for sha in shas {
                        let diff = repo.diff_commit(sha)?;
                        all_files.extend(diff.files);
                    }
                    DiffResult { files: all_files }
                }
                MultiCommitMode::Squashed => {
                    // Diff from first commit's parent to last commit
                    if shas.is_empty() {
                        return Ok(Vec::new());
                    }
                    let first = repo.get_commit(&shas[0])?;
                    let last_sha = &shas[shas.len() - 1];

                    if let Some(parent) = first.parent_shas.first() {
                        repo.diff_commits(parent, last_sha)?
                    } else {
                        // First commit has no parent - diff against empty tree
                        repo.diff_commit(&shas[0])?
                    }
                }
            },
        };

        // Convert to DiffFile with syntax highlighting
        Self::convert_and_highlight(diff_result, &highlighter)
    }

    /// Get metadata about a diff.
    ///
    /// # Arguments
    ///
    /// * `spec` - The diff specification describing what to diff.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata cannot be computed.
    pub fn get_diff_info(&self, spec: &DiffSpec) -> Result<LocalDiffInfo> {
        let repo = self.open_repo()?;

        let commits = match spec {
            DiffSpec::Range {
                base,
                head,
                three_dot,
            } => {
                let base_sha = if *three_dot {
                    let base_resolved = repo.resolve_ref(base)?.sha;
                    let head_resolved = repo.resolve_ref(head)?.sha;
                    repo.merge_base(&base_resolved, &head_resolved)?
                } else {
                    repo.resolve_ref(base)?.sha
                };
                let head_sha = repo.resolve_ref(head)?.sha;
                repo.list_commits(&base_sha, &head_sha)?
            }
            DiffSpec::Commit { sha } => {
                vec![repo.get_commit(sha)?]
            }
            DiffSpec::Commits { shas, .. } => shas
                .iter()
                .map(|sha| repo.get_commit(sha))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow::anyhow!("{e}"))?,
            DiffSpec::WorkingTree { .. } => vec![],
        };

        let diffs = self.get_diff(spec)?;
        let is_dirty = repo.is_dirty()?;

        Ok(LocalDiffInfo {
            repo_path: self.repo_path.display().to_string(),
            spec: spec.clone(),
            description: spec.description(),
            commits,
            base_ref: match spec {
                DiffSpec::Range { base, .. } => Some(base.clone()),
                DiffSpec::WorkingTree { against, .. } => Some(against.clone()),
                _ => None,
            },
            head_ref: match spec {
                DiffSpec::Range { head, .. } => Some(head.clone()),
                _ => None,
            },
            total_additions: diffs.iter().map(|f| f.additions).sum(),
            total_deletions: diffs.iter().map(|f| f.deletions).sum(),
            files_changed: diffs.len(),
            is_dirty,
        })
    }

    fn convert_and_highlight(
        result: DiffResult,
        highlighter: &SyntaxHighlighter,
    ) -> Result<Vec<DiffFile>> {
        let mut files = Vec::new();

        for file_diff in result.files {
            let filename = file_diff
                .new_path
                .clone()
                .or_else(|| file_diff.old_path.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let status = match file_diff.status {
                DiffStatus::Added | DiffStatus::Untracked => FileStatus::Added,
                DiffStatus::Deleted => FileStatus::Deleted,
                DiffStatus::Modified => FileStatus::Modified,
                DiffStatus::Renamed | DiffStatus::Copied => FileStatus::Renamed,
            };

            if file_diff.binary || file_diff.patch.is_none() {
                // Binary file - no diff content
                files.push(DiffFile {
                    filename,
                    status,
                    additions: 0,
                    deletions: 0,
                    hunks: vec![],
                });
                continue;
            }

            let patch = file_diff.patch.unwrap();
            let (additions, deletions) = chadreview_diff::parser::count_additions_deletions(&patch);

            let diff_file =
                parse_unified_diff(&filename, status, additions, deletions, &patch, highlighter)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

            files.push(diff_file);
        }

        Ok(files)
    }
}

impl<B: GitBackend> Clone for LocalDiffProvider<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            repo_path: self.repo_path.clone(),
        }
    }
}
