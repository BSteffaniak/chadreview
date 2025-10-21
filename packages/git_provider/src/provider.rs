use anyhow::Result;
use chadreview_pr_models::{Comment, CreateComment, DiffFile, PullRequest};

/// Abstract trait for git hosting provider implementations.
///
/// Defines the interface for fetching pull request data, diffs, comments,
/// and performing comment CRUD operations across different git hosting platforms
/// (GitHub, GitLab, Bitbucket, etc.).
#[async_trait::async_trait]
pub trait GitProvider: Send + Sync {
    /// Fetches pull request metadata and details.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    ///
    /// # Returns
    /// Complete pull request information including author, state, branches, labels, etc.
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest>;

    /// Fetches the diff for a pull request.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    ///
    /// # Returns
    /// List of changed files with their hunks and line-by-line diffs.
    async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>>;

    /// Fetches all comments for a pull request.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    ///
    /// # Returns
    /// List of comments including general PR comments, file-level, and line-level comments
    /// with their nested reply threads.
    async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>>;

    /// Creates a new comment on a pull request.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    /// * `comment` - Comment creation request with body and type
    ///
    /// # Returns
    /// The created comment with server-assigned ID and timestamps.
    async fn create_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment: CreateComment,
    ) -> Result<Comment>;

    /// Updates an existing comment body.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    /// * `comment_id` - Comment ID to update
    /// * `body` - New comment body text
    ///
    /// # Returns
    /// The updated comment with new body and updated timestamp.
    async fn update_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment_id: u64,
        body: String,
    ) -> Result<Comment>;

    /// Deletes a comment.
    ///
    /// # Arguments
    /// * `owner` - Repository owner username or organization
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    /// * `comment_id` - Comment ID to delete
    async fn delete_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment_id: u64,
    ) -> Result<()>;

    /// Returns the provider name identifier.
    ///
    /// # Returns
    /// Provider name string (e.g., "github", "gitlab", "bitbucket")
    fn provider_name(&self) -> &'static str;

    /// Indicates whether the provider supports draft pull requests.
    ///
    /// # Returns
    /// `true` if draft PRs are supported, `false` otherwise. Defaults to `false`.
    fn supports_drafts(&self) -> bool {
        false
    }

    /// Indicates whether the provider supports line-level comments.
    ///
    /// # Returns
    /// `true` if line-level comments are supported, `false` otherwise. Defaults to `true`.
    fn supports_line_comments(&self) -> bool {
        true
    }
}
