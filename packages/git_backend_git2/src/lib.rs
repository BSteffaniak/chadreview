#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

//! git2 (libgit2) implementation of the `GitBackend` trait.
//!
//! This crate provides a production-ready git backend using the `git2` crate,
//! which wraps the `libgit2` C library.

use std::path::{Path, PathBuf};

use chadreview_git_backend::{GitBackend, GitRepository};
use chadreview_git_backend_models::{
    CommitInfo, DiffResult, DiffStatus, FileDiff, GitBackendError, RefType, ResolvedRef,
    WorkingTreeDiffOptions,
};
use git2::{DiffOptions, Repository, StatusOptions};

/// git2-based implementation of `GitBackend`.
#[derive(Debug, Clone, Default)]
pub struct Git2Backend;

impl Git2Backend {
    /// Create a new git2 backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl GitBackend for Git2Backend {
    fn open(&self, path: &Path) -> Result<Box<dyn GitRepository>, GitBackendError> {
        let repo = Repository::open(path).map_err(|e| GitBackendError::RepoNotFound {
            path: format!("{}: {e}", path.display()),
        })?;
        Ok(Box::new(Git2Repository::new(repo)))
    }

    fn discover(&self, path: &Path) -> Result<Box<dyn GitRepository>, GitBackendError> {
        let repo = Repository::discover(path).map_err(|e| GitBackendError::NotARepository {
            path: format!("{}: {e}", path.display()),
        })?;
        Ok(Box::new(Git2Repository::new(repo)))
    }
}

/// git2-based implementation of `GitRepository`.
struct Git2Repository {
    repo: Repository,
    workdir: Option<PathBuf>,
}

impl Git2Repository {
    fn new(repo: Repository) -> Self {
        let workdir = repo.workdir().map(Path::to_path_buf);
        Self { repo, workdir }
    }

    fn resolve_to_commit(&self, spec: &str) -> Result<git2::Commit<'_>, GitBackendError> {
        let obj = self
            .repo
            .revparse_single(spec)
            .map_err(|_| GitBackendError::RefNotFound {
                ref_name: spec.to_string(),
            })?;

        obj.peel_to_commit()
            .map_err(|_| GitBackendError::RefNotFound {
                ref_name: spec.to_string(),
            })
    }

    fn commit_to_info(commit: &git2::Commit<'_>) -> CommitInfo {
        let sha = commit.id().to_string();
        let short_sha = sha[..sha.len().min(7)].to_string();
        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();
        let parent_shas = commit.parent_ids().map(|id| id.to_string()).collect();

        CommitInfo {
            sha,
            short_sha,
            message,
            summary,
            author_name,
            author_email,
            timestamp,
            parent_shas,
        }
    }

    fn diff_to_result(diff: &git2::Diff<'_>) -> DiffResult {
        let mut files = Vec::new();

        for (delta_idx, delta) in diff.deltas().enumerate() {
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());

            let status = match delta.status() {
                git2::Delta::Added => DiffStatus::Added,
                git2::Delta::Deleted => DiffStatus::Deleted,
                git2::Delta::Renamed => DiffStatus::Renamed,
                git2::Delta::Copied => DiffStatus::Copied,
                git2::Delta::Untracked | git2::Delta::Ignored => DiffStatus::Untracked,
                git2::Delta::Modified
                | git2::Delta::Unmodified
                | git2::Delta::Typechange
                | git2::Delta::Unreadable
                | git2::Delta::Conflicted => DiffStatus::Modified,
            };

            let binary = delta.flags().is_binary();

            // Get patch text
            let patch = if binary {
                None
            } else {
                Self::get_patch_text(diff, delta_idx)
            };

            files.push(FileDiff {
                old_path,
                new_path,
                status,
                patch,
                binary,
            });
        }

        DiffResult { files }
    }

    fn get_patch_text(diff: &git2::Diff<'_>, delta_idx: usize) -> Option<String> {
        let mut patch_text = String::new();

        let result = diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            // Check if this is for our delta
            let current_idx = diff.deltas().position(|d| {
                d.old_file().id() == delta.old_file().id()
                    && d.new_file().id() == delta.new_file().id()
            });

            if current_idx != Some(delta_idx) {
                return true;
            }

            let origin = line.origin();
            match origin {
                '+' | '-' | ' ' => {
                    patch_text.push(origin);
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        patch_text.push_str(content);
                    }
                }
                'H' => {
                    // Hunk header
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        patch_text.push_str(content);
                    }
                }
                _ => {}
            }
            true
        });

        if result.is_err() || patch_text.is_empty() {
            return None;
        }

        Some(patch_text)
    }
}

impl GitRepository for Git2Repository {
    fn resolve_ref(&self, ref_name: &str) -> Result<ResolvedRef, GitBackendError> {
        let commit = self.resolve_to_commit(ref_name)?;
        let sha = commit.id().to_string();

        // Try to determine ref type
        let ref_type = if ref_name == "HEAD" {
            RefType::Head
        } else if self
            .repo
            .find_branch(ref_name, git2::BranchType::Local)
            .is_ok()
        {
            RefType::Branch
        } else if self
            .repo
            .find_branch(ref_name, git2::BranchType::Remote)
            .is_ok()
        {
            RefType::Remote
        } else if self
            .repo
            .find_reference(&format!("refs/tags/{ref_name}"))
            .is_ok()
        {
            RefType::Tag
        } else {
            RefType::Commit
        };

        Ok(ResolvedRef {
            sha,
            name: ref_name.to_string(),
            ref_type,
        })
    }

    fn merge_base(&self, commit1: &str, commit2: &str) -> Result<String, GitBackendError> {
        let oid1 = self.resolve_to_commit(commit1)?.id();
        let oid2 = self.resolve_to_commit(commit2)?.id();

        let merge_base =
            self.repo
                .merge_base(oid1, oid2)
                .map_err(|e| GitBackendError::GitError {
                    message: format!("Failed to find merge base: {e}"),
                })?;

        Ok(merge_base.to_string())
    }

    fn get_commit(&self, sha: &str) -> Result<CommitInfo, GitBackendError> {
        let commit = self.resolve_to_commit(sha)?;
        Ok(Self::commit_to_info(&commit))
    }

    fn list_commits(&self, base: &str, head: &str) -> Result<Vec<CommitInfo>, GitBackendError> {
        let base_oid = self.resolve_to_commit(base)?.id();
        let head_oid = self.resolve_to_commit(head)?.id();

        let mut revwalk = self.repo.revwalk().map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        revwalk
            .push(head_oid)
            .map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;

        revwalk
            .hide(base_oid)
            .map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;

        let mut commits = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result.map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;

            let commit =
                self.repo
                    .find_commit(oid)
                    .map_err(|e| GitBackendError::CommitNotFound {
                        sha: format!("{oid}: {e}"),
                    })?;

            commits.push(Self::commit_to_info(&commit));
        }

        Ok(commits)
    }

    fn diff_commits(&self, old_sha: &str, new_sha: &str) -> Result<DiffResult, GitBackendError> {
        let old_commit = self.resolve_to_commit(old_sha)?;
        let new_commit = self.resolve_to_commit(new_sha)?;

        let old_tree = old_commit.tree().map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        let new_tree = new_commit.tree().map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
            .map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;

        Ok(Self::diff_to_result(&diff))
    }

    fn diff_commit(&self, sha: &str) -> Result<DiffResult, GitBackendError> {
        let commit = self.resolve_to_commit(sha)?;
        let tree = commit.tree().map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        let parent_tree = if commit.parent_count() > 0 {
            let parent = commit.parent(0).map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;
            Some(parent.tree().map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?)
        } else {
            None
        };

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .map_err(|e| GitBackendError::GitError {
                message: e.to_string(),
            })?;

        Ok(Self::diff_to_result(&diff))
    }

    fn diff_working_tree(
        &self,
        against: &str,
        options: WorkingTreeDiffOptions,
    ) -> Result<DiffResult, GitBackendError> {
        let commit = self.resolve_to_commit(against)?;
        let tree = commit.tree().map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        let mut diff_opts = DiffOptions::new();
        if options.include_untracked {
            diff_opts.include_untracked(true);
            diff_opts.show_untracked_content(true);
            diff_opts.recurse_untracked_dirs(true);
        }
        if options.include_ignored {
            diff_opts.include_ignored(true);
        }

        let diff = if options.staged_only {
            self.repo
                .diff_tree_to_index(Some(&tree), None, Some(&mut diff_opts))
        } else {
            self.repo
                .diff_tree_to_workdir_with_index(Some(&tree), Some(&mut diff_opts))
        }
        .map_err(|e| GitBackendError::GitError {
            message: e.to_string(),
        })?;

        Ok(Self::diff_to_result(&diff))
    }

    fn head(&self) -> Result<String, GitBackendError> {
        let head = self.repo.head().map_err(|e| GitBackendError::GitError {
            message: format!("Failed to get HEAD: {e}"),
        })?;

        let commit = head
            .peel_to_commit()
            .map_err(|e| GitBackendError::GitError {
                message: format!("HEAD is not a commit: {e}"),
            })?;

        Ok(commit.id().to_string())
    }

    fn workdir(&self) -> Option<&Path> {
        self.workdir.as_deref()
    }

    fn is_dirty(&self) -> Result<bool, GitBackendError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);

        let statuses =
            self.repo
                .statuses(Some(&mut opts))
                .map_err(|e| GitBackendError::GitError {
                    message: format!("Failed to get status: {e}"),
                })?;

        Ok(!statuses.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Configure user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        (dir, repo)
    }

    fn create_commit(repo: &Repository, message: &str, files: &[(&str, &str)]) -> git2::Oid {
        let mut index = repo.index().unwrap();

        for (path, content) in files {
            let full_path = repo.workdir().unwrap().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }

        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let sig = repo.signature().unwrap();

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    #[test]
    fn test_open_and_discover() {
        let (dir, _repo) = create_test_repo();
        let backend = Git2Backend::new();

        // Test open
        let result = backend.open(dir.path());
        assert!(result.is_ok());

        // Test discover from subdirectory
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let result = backend.discover(&subdir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_diff_commits() {
        let (dir, repo) = create_test_repo();

        // Create initial commit
        create_commit(&repo, "Initial commit", &[("file.txt", "Hello")]);

        let first_sha = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();

        // Create second commit
        create_commit(&repo, "Second commit", &[("file.txt", "Hello World")]);

        let second_sha = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();

        let backend = Git2Backend::new();
        let git_repo = backend.open(dir.path()).unwrap();

        let result = git_repo.diff_commits(&first_sha, &second_sha);
        assert!(result.is_ok());

        let diff = result.unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].new_path, Some("file.txt".to_string()));
        assert_eq!(diff.files[0].status, DiffStatus::Modified);
    }

    #[test]
    fn test_list_commits() {
        let (dir, repo) = create_test_repo();

        create_commit(&repo, "First", &[("a.txt", "a")]);
        let first_sha = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();

        create_commit(&repo, "Second", &[("b.txt", "b")]);
        create_commit(&repo, "Third", &[("c.txt", "c")]);

        let backend = Git2Backend::new();
        let git_repo = backend.open(dir.path()).unwrap();

        let commits = git_repo.list_commits(&first_sha, "HEAD").unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Third");
        assert_eq!(commits[1].summary, "Second");
    }

    #[test]
    fn test_is_dirty() {
        let (dir, repo) = create_test_repo();
        create_commit(&repo, "Initial", &[("file.txt", "content")]);

        let backend = Git2Backend::new();
        let git_repo = backend.open(dir.path()).unwrap();

        // Clean state
        assert!(!git_repo.is_dirty().unwrap());

        // Dirty state
        fs::write(dir.path().join("file.txt"), "modified").unwrap();
        assert!(git_repo.is_dirty().unwrap());
    }
}
