use anyhow::Result;
use chadreview_pr_models::{Comment, CreateComment, DiffFile, PullRequest};

#[async_trait::async_trait]
pub trait GitProvider: Send + Sync {
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest>;

    async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>>;

    async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>>;

    async fn create_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment: CreateComment,
    ) -> Result<Comment>;

    async fn update_comment(&self, comment_id: u64, body: String) -> Result<Comment>;

    async fn delete_comment(&self, comment_id: u64) -> Result<()>;

    fn provider_name(&self) -> &str;

    fn supports_drafts(&self) -> bool {
        false
    }

    fn supports_line_comments(&self) -> bool {
        true
    }
}
